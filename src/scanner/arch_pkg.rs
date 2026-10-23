use std::io::{self, Read};

use zstd::stream::read::Decoder as ZstdDecoder;

use crate::scanner::Ctx;

impl Ctx<'_> {
    pub fn scan_arch_pkg(&mut self, r: &mut impl Read, label: &str) -> io::Result<()> {
        label.clone_into(&mut self.nar);
        let mut archive = tar::Archive::new(ZstdDecoder::new(r)?);

        for entry in archive.entries()? {
            let mut entry = entry?;

            if !entry.header().entry_type().is_file() {
                continue;
            }

            let path = format!(
                "{label}/{}",
                entry.path().map_or_default(|p| p.display().to_string()),
            );
            let size = entry.size();
            self.scan_reader(&path, &mut entry, size)?;
        }

        Ok(())
    }
}
