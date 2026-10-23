use std::io::{self, Read};

use crate::scanner::Ctx;

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

fn nar_u64(r: &mut impl Read) -> io::Result<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

fn nar_pad(r: &mut impl Read, len: u64) -> io::Result<()> {
    let pad = (8 - len % 8) % 8;
    let mut b = [0u8; 8];
    r.read_exact(&mut b[..pad as usize])
}

fn nar_str(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let len = nar_u64(r)?;
    if len > 4096 {
        return Err(bad("nar token too long"));
    }
    let mut s = vec![0u8; len as usize];
    r.read_exact(&mut s)?;
    nar_pad(r, len)?;
    Ok(s)
}

fn nar_expect(r: &mut impl Read, tok: &[u8]) -> io::Result<()> {
    let s = nar_str(r)?;
    if s != tok {
        return Err(bad(&format!(
            "expected {:?}, got {:?}",
            String::from_utf8_lossy(tok),
            String::from_utf8_lossy(&s)
        )));
    }
    Ok(())
}

fn nar_node(ctx: &mut Ctx, r: &mut impl Read, path: &str) -> io::Result<()> {
    nar_expect(r, b"(")?;
    nar_expect(r, b"type")?;
    match nar_str(r)?.as_slice() {
        b"regular" => {
            let mut tok = nar_str(r)?;
            if tok == b"executable" {
                nar_expect(r, b"")?;
                tok = nar_str(r)?;
            }
            if tok != b"contents" {
                return Err(bad("expected contents"));
            }
            let len = nar_u64(r)?;
            let mut t = r.take(len);
            ctx.scan_reader(path, &mut t, len)?;
            io::copy(&mut t, &mut io::sink())?;
            nar_pad(r, len)?;
            nar_expect(r, b")")
        }
        b"symlink" => {
            nar_expect(r, b"target")?;
            nar_str(r)?;
            nar_expect(r, b")")
        }
        b"directory" => loop {
            let tok = nar_str(r)?;
            if tok == b")" {
                return Ok(());
            }
            if tok != b"entry" {
                return Err(bad("expected entry"));
            }
            nar_expect(r, b"(")?;
            nar_expect(r, b"name")?;
            let name = String::from_utf8_lossy(&nar_str(r)?).into_owned();
            nar_expect(r, b"node")?;
            nar_node(ctx, r, &format!("{path}/{name}"))?;
            nar_expect(r, b")")?;
        },
        t => Err(bad(&format!("bad type {:?}", String::from_utf8_lossy(t)))),
    }
}

impl Ctx<'_> {
    pub fn scan_nar(&mut self, r: &mut impl Read, label: &str) -> io::Result<()> {
        label.clone_into(&mut self.nar);
        nar_expect(r, b"nix-archive-1").and_then(|_| nar_node(self, r, label))
    }
}
