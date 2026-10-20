"""usage: package-list.py report/findings.tsv — print the markdown TODO list of packages to patch, one section per fix class."""
import csv, re, sys, collections

ORDER = [
    ("vendored-pem-file", "Vendored PEM bundle", "Replace the file with a symlink to `${cacert}/etc/ssl/certs/ca-bundle.crt`, or point the program at it."),
    ("vendored-in-archive", "Bundle inside a wheel, jar or zip", "Patch the bundle inside the archive at build time, or patch the code to use the system store."),
    ("vendored-keystore", "Java keystore", "Replace with `${cacert}/etc/ssl/certs/java/cacerts`."),
    ("compiled-in", "Compiled into a binary", "Rebuild with a system-store option (rustls-native-certs, `--use-system-ca`, NSS from cacert), or bump the embedded root crate."),
]

def name(storename):
    n = re.sub(r"-[0-9].*$", "", storename)
    n = re.sub(r"^python3\.\d+-", "python3Packages.", n)
    n = re.sub(r"^ruby\d\.\d-", "rubyPackages.", n)
    return n

pkgs = collections.defaultdict(lambda: {"files": set(), "ecc": False})
for r in csv.DictReader(open(sys.argv[1]), delimiter="\t"):
    p = pkgs[(r["class"], name(r["package"]))]
    p["files"].add(r["path"])
    p["ecc"] |= r["ecc_g3"] == "1"

for c, title, fix in ORDER:
    rows = sorted((n, p) for (cc, n), p in pkgs.items() if cc == c)
    print(f"### {title} ({len(rows)})\n\nFix: {fix}\n")
    for n, p in rows:
        files = sorted(p["files"], key=len)
        more = f" (+{len(files) - 1} more)" if len(files) > 1 else ""
        stale = "" if p["ecc"] else " · stale bundle, no ECC G3"
        print(f"- [ ] **{n}** — `{files[0]}`{more}{stale}")
    print()
