#!/usr/bin/env bash
# usage: retry-failed-arch.sh OUTDIR BASE_URL
set -euo pipefail
export LC_ALL=C.UTF-8

. "$(dirname "$0")/env.sh"
out=$1
base=$2

[ -s "$out/failed" ] || exit 0
mv "$out/failed" "$out/failed.prev"
cut -d' ' -f1-2 "$out/failed.prev" |
  sort -u |
  shuf |
  xargs -r -P 8 -n2 "$root/scripts/scan-one-arch.sh" "$out" "$base"
