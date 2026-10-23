use flate2::read::MultiGzDecoder;
use memchr::memmem;
use openssl::{pkcs12::Pkcs12, provider::Provider as OpensslProvider};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{self, Cursor, Read, StdoutLock, Write};
use std::mem;
use std::sync::Once;
use xz2::read::XzDecoder;
use zip::ZipArchive;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::b64;
use crate::matcher::{Kind, Matcher};

mod file;
mod nar;

const MAX_IN_MEMORY: u64 = 1 << 30;
const CHUNK: usize = 256 << 20;
const OVERLAP: usize = 64 << 10;
const TRUST_STORE_HINT: usize = 3;
const MAX_DEPTH: usize = 4;
const MAX_UNPACKED: u64 = 1 << 30;
const MIN_BARE_B64: usize = 600;
// common store passwords seen in Java truststores; "" covers password-less PKCS12
const P12_PASSWORDS: &[&str] = &[
    "changeit",
    "",
    "changeme",
    "password",
    "secret",
    "123456",
    "truststore",
    "keystore",
    "notasecret",
];

#[derive(Default, Serialize)]
struct Finding {
    nar: String,
    file: String,
    size: u64,
    magic: Option<&'static str>,
    formats: BTreeSet<&'static str>,
    pem_certs: usize,
    known_roots: BTreeSet<String>,
    watch: BTreeSet<String>,
    watch_spki: BTreeSet<String>,
    spki_hash: BTreeSet<String>,
    sha256: Option<String>,
    note: Option<String>,
}

impl Finding {
    fn interesting(&self) -> bool {
        self.known_roots.len() >= 3
            || self.pem_certs >= 2
            || !self.watch.is_empty()
            || !self.watch_spki.is_empty()
            || !self.spki_hash.is_empty()
            || matches!(self.magic, Some("jks" | "jceks" | "pkcs12"))
    }
}

fn apply(m: &Matcher, data: &[u8], f: &mut Finding, fmt: &'static str) {
    let mut hit = false;
    for mat in m.ac.find_overlapping_iter(data) {
        hit = true;
        for p in &m.pats[mat.pattern().as_usize()] {
            match p.kind {
                Kind::Root => f.known_roots.insert(p.name.clone()),
                Kind::Watch => f.watch.insert(p.name.clone()),
                Kind::WatchSpki => f.watch_spki.insert(p.name.clone()),
                Kind::BlockHash => f.spki_hash.insert(p.name.clone()),
            };
        }
    }
    if hit {
        f.formats.insert(fmt);
    }
}

fn bare_base64(m: &Matcher, data: &[u8], f: &mut Finding) {
    let finder = memmem::Finder::new(b"MII");
    let mut pos = 0;
    while let Some(i) = finder.find(&data[pos..]) {
        let s = pos + i;
        let mut e = s;
        while e < data.len()
            && (data[e].is_ascii_alphanumeric() || matches!(data[e], b'+' | b'/' | b'='))
        {
            e += 1;
        }
        if e - s >= MIN_BARE_B64 {
            if let Some(der) = b64::b64_decode(&data[s..e]) {
                apply(m, &der, f, "base64");
            }
        }
        pos = e.max(s + 3);
    }
}

fn certdata_octal(m: &Matcher, data: &[u8], f: &mut Finding) {
    let finder = memmem::Finder::new(b"MULTILINE_OCTAL");
    let mut pos = 0;
    while let Some(i) = finder.find(&data[pos..]) {
        let mut j = pos + i + 15;
        let mut der = Vec::new();
        while j + 3 < data.len() {
            if data[j] == b'\\' && data[j + 1..j + 4].iter().all(|c| (b'0'..=b'7').contains(c)) {
                der.push(
                    (data[j + 1] - b'0') * 64 + (data[j + 2] - b'0') * 8 + (data[j + 3] - b'0'),
                );
                j += 4;
            } else if data[j..].starts_with(b"END") {
                break;
            } else {
                j += 1;
            }
        }
        if !der.is_empty() {
            apply(m, &der, f, "certdata-octal");
        }
        pos = j;
    }
}

fn load_openssl_providers() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // RC2-40 / 3DES cert bags from JDK <= 11 need the legacy provider
        for name in ["default", "legacy"] {
            if let Ok(p) = OpensslProvider::try_load(None, name, true) {
                mem::forget(p);
            }
        }
    });
}

fn pkcs12_decrypt(m: &Matcher, data: &[u8], f: &mut Finding) {
    load_openssl_providers();
    let Ok(p12) = Pkcs12::from_der(data) else {
        return;
    };
    for pw in P12_PASSWORDS {
        let Ok(parsed) = p12.parse2(pw) else { continue };
        let mut n = 0;
        for c in parsed
            .ca
            .iter()
            .flatten()
            .map(|c| c.to_owned())
            .chain(parsed.cert)
        {
            if let Ok(der) = c.to_der() {
                apply(m, &der, f, "pkcs12-decrypted");
                n += 1;
            }
        }
        f.note = Some(format!("pkcs12 opened with password {pw:?}: {n} certs"));
        return;
    }
    f.note = Some("pkcs12: password not in list, contents not checked".into());
}

fn magic(data: &[u8]) -> Option<&'static str> {
    const P12_BAG: &[u8] = &[
        0x06, 0x0b, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x0c, 0x0a, 0x01,
    ];
    const P7_DATA: &[u8] = &[
        0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x01,
    ];
    match data {
        [0xfe, 0xed, 0xfe, 0xed, ..] => Some("jks"),
        [0xce, 0xce, 0xce, 0xce, ..] => Some("jceks"),
        [0x1f, 0x8b, ..] => Some("gzip"),
        [b'P', b'K', 3, 4, ..] => Some("zip"),
        [0xfd, b'7', b'z', b'X', b'Z', 0, ..] => Some("xz"),
        [0x28, 0xb5, 0x2f, 0xfd, ..] => Some("zstd"),
        [0x7f, b'E', b'L', b'F', ..] => Some("elf"),
        [0x30, ..]
            if data.len() < 4 << 20
                && memmem::find(&data[..data.len().min(256)], P7_DATA).is_some()
                && (memmem::find(data, P12_BAG).is_some() || data.len() > 1024) =>
        {
            Some("pkcs12")
        }
        _ if data.len() > 262 && &data[257..262] == b"ustar" => Some("tar"),
        _ if data.starts_with(b"SQLite format 3\0") => Some("sqlite"),
        _ => None,
    }
}

pub struct Ctx<'a> {
    pub m: &'a Matcher,
    pub nar: String,
    pub out: StdoutLock<'static>,
    pub files: u64,
    pub bytes: u64,
    pub findings: u64,
}

impl Ctx<'_> {
    fn emit(&mut self, f: &Finding) {
        if f.interesting() {
            self.findings += 1;
            serde_json::to_writer(&mut self.out, f).unwrap();
            self.out.write_all(b"\n").unwrap();
        }
    }

    fn scan_buf(&mut self, path: &str, data: &[u8], depth: usize) {
        let mut f = Finding {
            nar: self.nar.clone(),
            file: path.to_owned(),
            size: data.len() as u64,
            ..Default::default()
        };
        f.magic = magic(data);
        apply(self.m, data, &mut f, "der");
        let blocks = b64::pem_blocks(data);
        f.pem_certs = blocks.len();
        for (_, der) in &blocks {
            if let Some(d) = der {
                apply(self.m, d, &mut f, "pem");
            }
        }
        bare_base64(self.m, data, &mut f);
        certdata_octal(self.m, data, &mut f);
        if f.magic == Some("pkcs12") && f.known_roots.len() < TRUST_STORE_HINT {
            pkcs12_decrypt(self.m, data, &mut f);
        }
        if f.interesting() {
            f.sha256 = Some(hex::encode(Sha256::digest(data)));
        }
        self.emit(&f);
        if depth >= MAX_DEPTH {
            return;
        }
        match f.magic {
            Some("gzip") => self.unpack(path, "gunzip", MultiGzDecoder::new(data), depth),
            Some("xz") => self.unpack(path, "unxz", XzDecoder::new(data), depth),
            Some("zstd") => {
                if let Ok(d) = ZstdDecoder::new(data) {
                    self.unpack(path, "unzstd", d, depth);
                }
            }
            Some("zip") => self.zip(path, data, depth),
            Some("tar") => self.tar(path, data, depth),
            _ => {}
        }
    }

    fn unpack(&mut self, path: &str, how: &str, r: impl Read, depth: usize) {
        let mut buf = Vec::new();
        if r.take(MAX_UNPACKED).read_to_end(&mut buf).is_ok() && !buf.is_empty() {
            self.scan_buf(&format!("{path}!{how}"), &buf, depth + 1);
        }
    }

    fn zip(&mut self, path: &str, data: &[u8], depth: usize) {
        let Ok(mut z) = ZipArchive::new(Cursor::new(data)) else {
            return;
        };
        for i in 0..z.len() {
            let Ok(mut e) = z.by_index(i) else { continue };
            if !e.is_file() || e.size() > MAX_UNPACKED {
                continue;
            }
            let name = format!(
                "{path}!{}",
                e.name()
                    .unwrap_or_else(|_| String::from_utf8_lossy(e.name_raw())),
            );
            let mut buf = Vec::with_capacity(e.size() as usize);
            if e.read_to_end(&mut buf).is_ok() {
                drop(e);
                self.scan_buf(&name, &buf, depth + 1);
            }
        }
    }

    fn tar(&mut self, path: &str, data: &[u8], depth: usize) {
        let mut a = tar::Archive::new(data);
        let Ok(entries) = a.entries() else { return };
        for e in entries.flatten() {
            let mut e = e;
            if !e.header().entry_type().is_file() || e.size() > MAX_UNPACKED {
                continue;
            }
            let name = format!(
                "{path}!{}",
                e.path().map_or_default(|p| p.display().to_string())
            );
            let mut buf = Vec::new();
            if e.read_to_end(&mut buf).is_ok() {
                self.scan_buf(&name, &buf, depth + 1);
            }
        }
    }

    pub fn scan_reader(&mut self, path: &str, mut r: impl Read, len: u64) -> io::Result<()> {
        self.files += 1;
        self.bytes += len;
        if len <= MAX_IN_MEMORY {
            let mut buf = Vec::with_capacity(len as usize);
            r.read_to_end(&mut buf)?;
            self.scan_buf(path, &buf, 0);
            return Ok(());
        }
        let mut f = Finding {
            nar: self.nar.clone(),
            file: path.to_owned(),
            size: len,
            note: Some("chunked: archives not unpacked".into()),
            ..Default::default()
        };
        let mut buf = vec![0u8; CHUNK + OVERLAP];
        let mut have = 0;
        loop {
            let n = read_full(&mut r, &mut buf[have..])?;
            if n == 0 && have == 0 {
                break;
            }
            let end = have + n;
            let view = &buf[..end];
            if f.magic.is_none() {
                f.magic = magic(view);
            }
            apply(self.m, view, &mut f, "der");
            for (_, der) in b64::pem_blocks(view) {
                f.pem_certs += 1;
                if let Some(d) = der {
                    apply(self.m, &d, &mut f, "pem");
                }
            }
            if n == 0 || end < buf.len() {
                break;
            }
            buf.copy_within(end - OVERLAP..end, 0);
            have = OVERLAP;
        }
        self.emit(&f);
        Ok(())
    }
}

fn read_full(r: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..])? {
            0 => break,
            k => n += k,
        }
    }
    Ok(n)
}
