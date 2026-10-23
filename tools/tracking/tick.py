"""Tick, untick or annotate a package in the nixpkgs CA-bundle tracking issues.

usage:
  tick.py PACKAGE PR [NOTE]      tick PACKAGE and link PR (e.g. 576980 or "#576980")
  tick.py --untick PACKAGE       remove the tick and the PR link added by this tool
  tick.py --issues 576541,...    restrict to these issues (default: all four)
  tick.py --dry-run ...          print the change, write nothing

An entry line looks like: "- [ ] **name** — `path` (+N more) · marks — #PR (note)".
PACKAGE must match **name** exactly. The body is written only if it did not
change since it was read (GitHub has no conditional issue update).
"""
import argparse, json, os, re, subprocess, sys

REPO = "NixOS/nixpkgs"
ISSUES = ["576541", "576542", "576543", "576544"]


def gh(*args, data=None):
    r = subprocess.run(["gh", "api", *args], input=data, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr.strip())
    return r.stdout


def body(issue):
    return json.loads(gh(f"repos/{REPO}/issues/{issue}"))["body"]


# keep issue-sync.sh's copy current so it does not report this tool's own edits
def remember(issue, title, text):
    state = os.environ.get("CACERT_TRACKING_STATE", os.path.expanduser("~/.cache/cacert-tracking"))
    path = os.path.join(state, f"body-{issue}")
    if os.path.isdir(state):
        with open(path, "w") as f:
            f.write("title: " + title + "\n" + text.replace("\r", "").rstrip("\n") + "\n")


def entry_re(pkg):
    return re.compile(r"^- \[(?P<box>[ xX])\] \*\*" + re.escape(pkg) + r"\*\* — (?P<rest>.*)$")


def tick_line(pkg, m, pr, note):
    rest = m["rest"].rstrip("\r")
    if not re.search(r"#" + re.escape(pr) + r"\b", rest):
        rest += f" — #{pr}" + (f" ({note})" if note else "")
    return f"- [x] **{pkg}** — {rest}"


def untick_line(pkg, m):
    rest = re.sub(r" — #\d+( \([^)]*\))?\s*$", "", m["rest"].rstrip("\r"))
    return f"- [ ] **{pkg}** — {rest}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("package")
    ap.add_argument("pr", nargs="?")
    ap.add_argument("note", nargs="?")
    ap.add_argument("--untick", action="store_true")
    ap.add_argument("--issues", default=",".join(ISSUES))
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args()
    if not a.untick and not a.pr:
        ap.error("PR is required unless --untick")
    pr = (a.pr or "").lstrip("#")
    rx = entry_re(a.package)
    found = 0
    for issue in a.issues.split(","):
        before = body(issue)
        lines = before.split("\n")
        hits = [i for i, l in enumerate(lines) if rx.match(l.rstrip("\r"))]
        if not hits:
            continue
        if len(hits) > 1:
            sys.exit(f"#{issue}: {len(hits)} entries named {a.package!r}, refusing")
        i = hits[0]
        m = rx.match(lines[i].rstrip("\r"))
        cr = "\r" if lines[i].endswith("\r") else ""
        new = untick_line(a.package, m) if a.untick else tick_line(a.package, m, pr, a.note)
        found += 1
        if new + cr == lines[i]:
            print(f"#{issue}: unchanged: {new[:160]}")
            continue
        lines[i] = new + cr
        after = "\n".join(lines)
        print(f"#{issue}: {new[:200]}")
        if a.dry_run:
            continue
        if body(issue) != before:
            sys.exit(f"#{issue}: changed while editing, nothing written; run again")
        res = json.loads(gh("-X", "PATCH", f"repos/{REPO}/issues/{issue}", "--input", "-",
                            data=json.dumps({"body": after})))
        remember(issue, res["title"], res["body"])
    if not found:
        sys.exit(f"no entry named {a.package!r} in #{a.issues}")


if __name__ == "__main__":
    main()
