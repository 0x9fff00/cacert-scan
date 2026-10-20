# Release scan: NixOS 26.05 (nixos-26.05.12250.f3a73bcf5d1c)

Input: all 209,053 store paths of the release, as NARs from cache.nixos.org.
Run: bld1, 48 parallel jobs, about 2 h, 5.3 GB peak memory.
Failures: 0. 46 NARs had truncated downloads; all succeeded on retry (`scripts/retry-failed.sh`).
NARs scanned before the PKCS12 decryption was added were scanned again.

Data: `report/findings.tsv` (one row per file), `report/packages.tsv` (one row per store path), `report/summary.txt`.

## Packages that need cacert injection

"Package" counts store-path names. "Base" removes the version.

| class | files | packages | fix |
|---|---|---|---|
| vendored-pem-file | 435 | 118 | Replace the file with a symlink to `${cacert}/etc/ssl/certs/ca-bundle.crt`, or point the program at it. |
| vendored-in-archive | 371 | 123 | Patch the bundle inside the wheel/jar at build time, or patch the code to use the system store. |
| vendored-keystore (Java) | 133 | 41 | Replace with `${cacert}/etc/ssl/certs/java/cacerts`. |
| compiled-in | 1849 | 685 | You cannot inject a file. Rebuild with a system-store option (rustls-native-certs, `--use-system-ca`, NSS) or bump the root crate. |

In total, 1033 store paths (880 bases) need action. 947 of them contain `emSign ECC Root CA - G3`, and 527 contain `AAA Certificate Services` as a plain anchor.

Already injected (no action): the nss-cacert outputs, python certifi, perl Mozilla-CA, anki, pyfa. The JDK 8 `cacerts` is also injected: nixpkgs builds it from `${cacert}` (`pkgs/development/compilers/openjdk/generic.nix:557-566`), and it has 118 roots, the same as `ca-no-trust-rules-bundle.crt`.

Symlinks to `${cacert}` carry no content, so they do not show in the results.

## Notable findings

- **openjdk 11/17/21/25, plus headless and minimal-jre**: these ship the upstream OpenJDK `cacerts` (99 roots, includes ECC G3). nixpkgs regenerates the store only for JDK 8. The `read-truststore-from-env` patches let users override it at run time.
- **corretto, temurin, zulu, semeru, graalvm, jetbrains-jdk, eclipse justj JRE**: these binary JDKs keep their vendor `cacerts`.
- **pip, virtualenv, pipenv, pex, botocore, awscli2, grpcio, httplib2, stripe**: these vendor a certifi-style `cacert.pem`, as a file or inside a bundled wheel.
- **python3 / python3-minimal**: the `ensurepip` wheel holds pip's vendored certifi.
- **ruby excon, httpclient, certified**: these hold vendored PEM bundles.
- **compiled-in**: most of these are Rust binaries with webpki-roots. The others are Go `x509roots/fallback` (caddy, alloy), Node/Electron/Chromium/Deno/Bun, the Dart VM, Firefox/Thunderbird NSS builtins, and qtwebengine.
- 158 vendored PEM files do not contain ECC G3. They are stale bundles, older than the emSign roots, so they are also out of date.

No package has an SPKI blocklist for the eight eMudhra intermediates. The release predates Go 1.26.10 and rustls-webpki 0.103.16.

## Not counted as "needs injection"

| class | files | packages | meaning |
|---|---|---|---|
| partial-root-set | 496 | 101 | 3–19 known roots: pinned subsets, test chains |
| keystore-locked | 120 | 29 | PKCS12 file with a password that is not in the list; contents not checked (for example ballerina truststore, zulu-ca-jdk 8) |
| keystore-no-known-roots | 306 | 56 | keystores with no known roots: usually test or server keys |
| fixture-or-other | 25969 | 672 | single certs, test fixtures |

Review `keystore-locked` by hand. `zulu-ca-jdk-8.0.492` and `ballerinaTruststore.p12` are likely real trust stores.

## Limits

- The base name is a heuristic, so some names keep a version (for example `dotnet-vmr-11.0.0-rc.1`).
- Duplicate rows come from store paths that have the same name and different hashes.
- Files over 1 GiB are scanned in chunks and are not unpacked. UTF-16 is not detected.
