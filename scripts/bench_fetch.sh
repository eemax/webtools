#!/usr/bin/env bash
set -euo pipefail

BIN="${BIN:-target/release/webtools}"

if [[ ! -x "$BIN" ]]; then
  cargo build --release >/dev/null
fi

URLS=(
  "https://example.com"
  "https://www.rust-lang.org/"
  "https://doc.rust-lang.org/book/ch01-01-installation.html"
)

printf "webtools fetch smoke bench\n"
printf "binary: %s\n\n" "$BIN"

for url in "${URLS[@]}"; do
  printf "%s\n" "$url"
  /usr/bin/time -p "$BIN" fetch --md "$url" >/dev/null
  printf "\n"
done
