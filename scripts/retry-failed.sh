#!/usr/bin/env bash
# usage: retry-failed.sh OUTDIR
set -eu
. "$(dirname "$0")/env.sh"
out=$1
[ -s "$out/failed" ] || exit 0
mv "$out/failed" "$out/failed.prev"
cut -d" " -f1 "$out/failed.prev" | sort -u | xargs -P 8 -n1 "$root/scripts/scan-one.sh" "$out"
