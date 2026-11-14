#!/usr/bin/env bash
# The licences of the Rust crates built into ERDTeX, listed by cargo-about
# (cargo install cargo-about --locked --features cli) into app/licenses/:
#   rust-desktop.txt  the desktop program (shown in Help -> About -> Licences)
#   rust-core.txt     the core, as the website runs it
#   rust-pdf.txt      the website's PDF export
# The JavaScript libraries are listed when the page is built (vite.config.ts).
#
#   tools/licenses.sh           write the lists
#   tools/licenses.sh --check   fail if they are out of date (tools/check_rust.sh)
set -euo pipefail
cd "$(dirname "$0")/../app"
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
out=licenses
if [ "${1:-}" = "--check" ]; then
  out=$(mktemp -d)
  trap 'rm -rf "$out"' EXIT
fi
cargo about generate --manifest-path src-tauri/Cargo.toml about.hbs -o "$out/rust-desktop.txt" 2>/dev/null
cargo about generate --manifest-path crates/core-wasm/Cargo.toml about.hbs -o "$out/rust-core.txt" 2>/dev/null
cargo about generate --manifest-path crates/pdf-wasm/Cargo.toml about.hbs -o "$out/rust-pdf.txt" 2>/dev/null
if [ "${1:-}" = "--check" ]; then
  for f in rust-desktop.txt rust-core.txt rust-pdf.txt; do
    cmp -s "$out/$f" "licenses/$f" || { echo "licenses/$f is out of date: run tools/licenses.sh"; exit 1; }
  done
fi
