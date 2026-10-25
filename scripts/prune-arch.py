#!/usr/bin/env python3

import argparse
from pathlib import Path

parser = argparse.ArgumentParser(description="Removes old files from outdir after updating packages.txt")
parser.add_argument("outdir", type=Path)
args = parser.parse_args()

kept_labels = set()

with open(args.outdir / "packages.txt") as f:
    for line in f:
        [repo, filename] = line.split()
        kept_labels.add(filename.removesuffix(".pkg.tar.zst"))

for subdir in ("done", "err", "res"):
    for path in (args.outdir / subdir).iterdir():
        if not path.is_file():
            continue

        if path.name.removesuffix(".jsonl") not in kept_labels:
            path.unlink()
