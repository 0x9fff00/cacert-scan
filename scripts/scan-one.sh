#!/usr/bin/env bash
# usage: scan-one.sh OUTDIR STOREPATH   (env: SCAN = scanner command with pattern args)
set -o pipefail
out=$1 p=$2 h=${2:11:32}
[ -e "$out/done/$h" ] && exit 0
ni=$(curl -sf --retry 5 --retry-all-errors "https://cache.nixos.org/$h.narinfo") || { echo "$p narinfo" >> "$out/failed"; exit 0; }
url=$(sed -n "s/^URL: //p" <<<"$ni"); comp=$(sed -n "s/^Compression: //p" <<<"$ni")
case $comp in zstd) dec="zstd -dcq" ;; xz) dec="xz -dc" ;; bzip2) dec="bzip2 -dc" ;; none) dec=cat ;; *) echo "$p comp=$comp" >> "$out/failed"; exit 0 ;; esac
if curl -sf --retry 5 --retry-all-errors "https://cache.nixos.org/$url" | $dec | $SCAN --nar "$p" > "$out/res/$h.jsonl" 2> "$out/err/$h"; then
  touch "$out/done/$h"
else
  echo "$p scan" >> "$out/failed"
fi
