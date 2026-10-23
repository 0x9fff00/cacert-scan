#!/usr/bin/env bash
# usage: run-arch.sh OUTDIR BASE_URL REPOSITORY...
# scans every package in the specified Arch Linux repositories
set -euo pipefail
export LC_ALL=C.UTF-8

if [ "$#" -lt 3 ]; then
  echo "usage: $0 OUTDIR BASE_URL REPOSITORY..." >&2
  exit 2
fi

. "$(dirname "$0")/env.sh"
out=$1
base=${2%/}
shift 2
mkdir -p "$out"/{res,err,done}

if [ ! -s "$out/packages.txt" ]; then
  for repo in "$@"; do
    curl -sfL "$base/$repo/os/x86_64/$repo.db.tar.gz" |
      tar -xzO --wildcards '*/desc' |
      awk '/^%FILENAME%$/ { getline; print }' |
      sed "s/^/$repo /"
  done > "$out/packages.txt"
fi

shuf "$out/packages.txt" | xargs -P "${JOBS:-8}" -n2 "$root/scripts/scan-one-arch.sh" "$out" "$base"
"$root/scripts/retry-failed-arch.sh" "$out" "$base"
