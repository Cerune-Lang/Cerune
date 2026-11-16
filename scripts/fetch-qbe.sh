#!/usr/bin/env bash
# Use exactly the pinned upstream archive, including when its server is unavailable.
set -euo pipefail
readonly sha256=d587905d620dc5e1d2bfa7c2cc642b9b837aa89a3188c6e37b53d756cf66e320
readonly url=https://c9x.me/compile/release/qbe-1.3.tar.xz
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
readonly backup="$root/.github/vendor/qbe/qbe-1.3.tar.xz"
offline=false
if [[ "${1:-}" == --offline ]]; then offline=true; shift; fi
if [[ $# != 1 ]]; then
  echo 'usage: bash scripts/fetch-qbe.sh [--offline] <output-archive>' >&2
  exit 2
fi
output=$1
verify() { printf '%s  %s\n' "$sha256" "$1" | sha256sum --check --status; }
if [[ -e "$output" || -L "$output" ]]; then
  if [[ ! -f "$output" || -L "$output" ]] || ! verify "$output"; then
    echo "QBE: invalid existing archive: $output" >&2
    exit 1
  fi
  echo "QBE 1.3: verified existing archive ($sha256)"
  exit 0
fi
mkdir -p -- "$(dirname -- "$output")"
temporary=$(mktemp "${output}.tmp.XXXXXX")
trap 'rm -f -- "$temporary"' EXIT
if ! $offline && curl --fail --location --silent --show-error --proto '=https' --proto-redir '=https' --connect-timeout 10 --max-time 30 "$url" -o "$temporary"; then
  if ! verify "$temporary"; then
    echo 'QBE: upstream SHA-256 mismatch; refusing to extract or fall back' >&2
    exit 1
  fi
  echo "QBE 1.3: verified upstream download ($sha256)"
else
  echo 'QBE 1.3: using repository backup (offline or upstream download failed)' >&2
  if ! verify "$backup"; then
    echo 'QBE: repository backup SHA-256 mismatch' >&2
    exit 1
  fi
  cp -- "$backup" "$temporary"
  verify "$temporary"
  echo "QBE 1.3: verified repository backup ($sha256)"
fi
mv -- "$temporary" "$output"
