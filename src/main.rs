use clap::{ArgGroup, Parser};
use std::fs;
use std::io::{self, BufReader};
use std::path::PathBuf;
use std::process;

use crate::scanner::Ctx;

mod b64;
mod matcher;
mod scanner;

#[derive(Parser)]
#[command(group(ArgGroup::new("input").required(true)))]
struct Args {
    /// Custom PEM file containing known root certificates
    #[arg(long, env = "CACERT_SCAN_ROOTS")]
    roots: Option<PathBuf>,

    /// Custom PEM file containing compromised certificates
    #[arg(long, env = "CACERT_SCAN_WATCH")]
    watch: Option<PathBuf>,

    /// Read a Nix NAR archive from stdin and label findings with LABEL
    #[arg(long, group = "input", value_name = "LABEL")]
    nar: Option<String>,

    /// Scan files or directories
    #[arg(long, group = "input", num_args = 1..)]
    path: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let roots = args.roots.map(|p| fs::read(p).expect("roots"));
    let watch = args.watch.map(|p| fs::read(p).expect("watch"));
    let m = matcher::build_matcher(
        roots
            .as_deref()
            .unwrap_or(include_bytes!("../patterns/roots.pem")),
        watch
            .as_deref()
            .unwrap_or(include_bytes!("../patterns/watch.pem")),
    );
    let out = io::stdout().lock();
    let mut ctx = Ctx {
        m: &m,
        nar: String::new(),
        out,
        files: 0,
        bytes: 0,
        findings: 0,
    };
    let res = if let Some(label) = args.nar {
        let mut r = stdin_buf_reader();
        ctx.scan_nar(&mut r, &label)
    } else {
        args.path.into_iter().try_for_each(|p| ctx.scan_fs(&p))
    };
    eprintln!(
        "files={} bytes={} findings={}",
        ctx.files, ctx.bytes, ctx.findings
    );
    if let Err(e) = res {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn stdin_buf_reader() -> BufReader<io::StdinLock<'static>> {
    BufReader::with_capacity(1 << 20, io::stdin().lock())
}
