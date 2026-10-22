# Tracking issues

The scan results are tracked in NixOS/nixpkgs issues:

| Issue | Content |
|---|---|
| [#576475](https://github.com/NixOS/nixpkgs/issues/576475) | Meta issue with a summary table |
| [#576541](https://github.com/NixOS/nixpkgs/issues/576541) | Vendored PEM bundles |
| [#576542](https://github.com/NixOS/nixpkgs/issues/576542) | Bundles inside a wheel, jar or zip |
| [#576543](https://github.com/NixOS/nixpkgs/issues/576543) | Java keystores |
| [#576544](https://github.com/NixOS/nixpkgs/issues/576544) | Roots compiled into a binary |

Anyone can tick entries and edit these issues by hand. The two tools below write only when the issue body did not change since they read it, so they do not overwrite other edits.

## tick.py

Ticks or unticks one package by its exact name, in every issue that lists it:

```
python3 tools/tracking/tick.py PACKAGE PR [NOTE]
python3 tools/tracking/tick.py --untick PACKAGE
python3 tools/tracking/tick.py --dry-run gam 577133
```

It refuses when the name matches no entry or more than one entry in an issue. Tick a version bump only when the bump is shown to fix the store (for example the lock file has webpki-roots ≥ 1.0.10); add the evidence as NOTE.

## issue-sync.sh

Runs forever and prints one line per event. Every 5 minutes it:

- counts the ticks in the four category issues and updates the table in #576475 (only the count cells of rows that still have the expected form);
- reports new or changed PR links on the six issues (including the CVE tracking issue #576485), from NixOS/nixpkgs, numtide and zimbatm repositories only;
- searches for nixpkgs PRs that mention the category issues, as a backup for missed links;
- reports new or edited comments;
- reports edits to the issue titles and bodies;
- reports activity on PRs by `$PR_AUTHOR` (default zimbatm) in any repository: new comments, reviews, inline review comments, merges or closes, and new CI failures. Bot comments and the author's own comments are left out.

State is kept in `$CACERT_TRACKING_STATE` (default `~/.cache/cacert-tracking`). On the first run it records the current state and reports nothing.
