use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use x509_parser::certificate::X509Certificate;

use crate::b64;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Kind {
    Root,
    Watch,
    WatchSpki,
    BlockHash,
}

pub struct Pat {
    pub kind: Kind,
    pub name: String,
}

pub struct Matcher {
    pub ac: AhoCorasick,
    pub pats: Vec<Vec<Pat>>,
}

fn der_inner(der: &[u8]) -> &[u8] {
    // webpki-style anchors store Name/SPKI contents without the outer SEQUENCE header,
    // so match on the contents: it is a substring of the full DER as well.
    if der.len() < 2 {
        return der;
    }
    let l = der[1];
    let hdr = if l & 0x80 == 0 {
        2
    } else {
        2 + (l & 0x7f) as usize
    };
    der.get(hdr..).unwrap_or(der)
}

fn cn_of(c: &X509Certificate) -> String {
    c.subject()
        .iter_common_name()
        .next()
        .and_then(|a| a.as_str().ok())
        .map_or_else(|| c.subject().to_string(), str::to_owned)
}

fn pem_ders(data: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (_, block) in b64::pem_blocks(data) {
        if let Some(d) = block {
            out.push(d);
        }
    }
    out
}

pub fn build_matcher(roots: &[u8], watch: &[u8]) -> Matcher {
    let mut map: HashMap<Vec<u8>, Vec<Pat>> = HashMap::new();
    let mut add = |k: Vec<u8>, kind, name: String| {
        let v = map.entry(k).or_default();
        if !v.iter().any(|p| p.kind == kind && p.name == name) {
            v.push(Pat { kind, name });
        }
    };
    for der in pem_ders(roots) {
        if let Ok((_, c)) = x509_parser::parse_x509_certificate(&der) {
            add(
                der_inner(c.subject().as_raw()).to_vec(),
                Kind::Root,
                cn_of(&c),
            );
        }
    }
    for der in pem_ders(watch) {
        if let Ok((_, c)) = x509_parser::parse_x509_certificate(&der) {
            let name = cn_of(&c);
            add(
                der_inner(c.subject().as_raw()).to_vec(),
                Kind::Watch,
                name.clone(),
            );
            let spki = c.public_key().raw;
            add(der_inner(spki).to_vec(), Kind::WatchSpki, name.clone());
            let h = Sha256::digest(spki);
            add(h.to_vec(), Kind::BlockHash, format!("{name} (raw sha256)"));
            add(
                hex::encode(h).into_bytes(),
                Kind::BlockHash,
                format!("{name} (hex)"),
            );
            add(
                hex::encode_upper(h).into_bytes(),
                Kind::BlockHash,
                format!("{name} (HEX)"),
            );
            let b64 = BASE64_STANDARD.encode(h);
            add(
                b64.into_bytes(),
                Kind::BlockHash,
                format!("{name} (base64)"),
            );
        }
    }
    let (keys, pats): (Vec<_>, Vec<_>) = map.into_iter().unzip();
    let ac = AhoCorasickBuilder::new()
        .match_kind(MatchKind::Standard)
        .build(&keys)
        .expect("aho-corasick");
    eprintln!("patterns: {}", keys.len());
    Matcher { ac, pats }
}
