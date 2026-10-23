#!/usr/bin/env bash
# usage: scan-one-arch.sh OUTDIR BASE_URL REPOSITORY PACKAGE
set -euo pipefail
export LC_ALL=C.UTF-8

out=$1
base=$2
repo=$3
p=$4
name=${p%.pkg.tar.zst}

[ -e "$out/done/$name" ] && exit 0

if curl -sfL --retry 5 "$base/$repo/os/x86_64/$p" |
  $SCAN --arch-pkg "$name" > "$out/res/$name.jsonl" 2> "$out/err/$name"
then
  touch "$out/done/$name"
else
  echo "$repo $p scan" >> "$out/failed"
fi
