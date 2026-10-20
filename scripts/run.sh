#!/usr/bin/env bash
# usage: run.sh OUTDIR [CHANNEL]   scans every NAR of a NixOS channel release; resumable
set -eu
. "$(dirname "$0")/env.sh"
out=$1 channel=${2:-nixos-26.05}
mkdir -p "$out"/{res,err,done}
[ -s "$out/store-paths.txt" ] || curl -sfL "https://channels.nixos.org/$channel/store-paths.xz" | xz -d > "$out/store-paths.txt"
shuf --random-source=<(yes) "$out/store-paths.txt" | xargs -P "${JOBS:-48}" -n1 "$root/scripts/scan-one.sh" "$out"
"$root/scripts/retry-failed.sh" "$out"
