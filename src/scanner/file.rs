use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::Path;

use crate::scanner::Ctx;

fn walk(ctx: &mut Ctx, p: &Path) -> io::Result<()> {
    let md = fs::symlink_metadata(p)?;
    if md.is_dir() {
        let mut ents: Vec<_> = fs::read_dir(p)?.flatten().map(|e| e.path()).collect();
        ents.sort();
        for e in ents {
            walk(ctx, &e).unwrap_or_else(|err| eprintln!("failed to walk {}: {err}", e.display()));
        }
    } else if md.is_file() {
        ctx.scan_reader(
            &p.display().to_string(),
            BufReader::new(File::open(p)?),
            md.len(),
        )?;
    }
    Ok(())
}

impl Ctx<'_> {
    pub fn scan_fs(&mut self, p: &Path) -> io::Result<()> {
        self.nar = p.to_string_lossy().into_owned();
        walk(self, p)
    }
}
