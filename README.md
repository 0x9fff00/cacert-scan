# cacert-scan

Finds packages that carry their own CA trust store instead of using the system trust store.

## Usage

Scan the closure of your running system:

```
nix run github:zimbatm/nix-cacert-scan -- /run/current-system
```

Any store path or `result` link works, and you can give more than one. Each output line is one embedded trust store:

```
osquery-5.23.1   /share/osquery/certs/certs.pem   135 roots   ECC G3,AAA
```

- "ECC G3" means the store trusts `emSign ECC Root CA - G3`, the root on the compromised eMudhra path.
- "AAA" means it trusts Sectigo `AAA Certificate Services`, which cross-signs the same intermediates.
- Copies of the nixpkgs `cacert` bundle are left out.
- A hit means the bundle is in the package, not that the program uses it. For example, nixpkgs `nodejs` embeds roots but uses the system store by default.

Add `--json` for the raw scanner output (one JSON object per file). To scan a directory without its closure, use `nix shell github:zimbatm/nix-cacert-scan -c cacert-scan --path DIR`.

On 2026-10-18/19, eMudhra and Sectigo revoked the certificates of eight eMudhra intermediates as compromised (cACompromise in CCADB).
A package with its own trust store does not get root-store updates from the system trust store.

## Use

```
nix build                                   # or: cargo build --release
./result/bin/cacert-scan --roots patterns/roots.pem --watch patterns/watch.pem --path /nix/store/...-foo
nix develop -c scripts/run.sh out nixos-26.05   # whole release; needs about 2 h on 48 cores
python3 scripts/analyze.py out report
python3 tools/package-list.py report/findings.tsv
```

`scripts/run.sh` expects the scanner at `target/release/cacert-scan` (`cargo build --release`).
