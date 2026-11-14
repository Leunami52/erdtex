#!/usr/bin/env bash
# The release builds of the Rust version (app/README.md, "Installing"), into dist/:
#   deb and rpm  built in an Ubuntu 22.04 container (runs on glibc 2.35 and newer)
#   setup.exe    the Windows installer, cross-built here with cargo-xwin and NSIS
# Then each is installed in a clean system and runs its self-test: tools/test_rust_release.sh.
#
#   tools/build_rust_release.sh [linux|windows]     (both if nothing is said)
set -euo pipefail
cd "$(dirname "$0")/.."
# Rust installed with rustup in the home folder (README.md, "Decisions": build tools)
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
ROOT=$PWD
VERSION=$(python3 -c 'import json; print(json.load(open("app/src-tauri/tauri.conf.json"))["version"])')
# one version everywhere
for v in "$(python3 -c 'import json; print(json.load(open("app/package.json"))["version"])')" \
         "$(sed -n 's/^version = "\(.*\)"/\1/p' app/src-tauri/Cargo.toml | head -1)"; do
  [[ $v == "$VERSION" ]] || { echo "version $v differs from tauri.conf.json's $VERSION"; exit 1; }
done
mkdir -p dist
what=${1:-all}

if [[ $what == all || $what == linux ]]; then
  echo "== deb and rpm (Ubuntu 22.04 container)"
  podman build -q -t erdtex-build:22.04 tools/rust_release
  # the container's own cargo registry and build folder, kept between runs
  podman volume exists erdtex-cargo || podman volume create erdtex-cargo >/dev/null
  podman volume exists erdtex-target || podman volume create erdtex-target >/dev/null
  podman run --rm -v "$ROOT":/src:Z -v erdtex-cargo:/opt/cargo/registry -v erdtex-target:/target \
    -e CARGO_TARGET_DIR=/target -w /src/app erdtex-build:22.04 \
    bash -c 'npm ci --no-audit --no-fund >/dev/null && npx tauri build --bundles deb rpm \
             && cp /target/release/bundle/deb/erdtex_'"$VERSION"'_*.deb /src/dist/ERDTeX-'"$VERSION"'-amd64.deb \
             && cp /target/release/bundle/rpm/erdtex-'"$VERSION"'-*.rpm /src/dist/ERDTeX-'"$VERSION"'-x86_64.rpm'
  ls -l "dist/ERDTeX-$VERSION-amd64.deb" "dist/ERDTeX-$VERSION-x86_64.rpm"
fi

if [[ $what == all || $what == windows ]]; then
  echo "== the Windows installer (cargo-xwin, NSIS)"
  cd app
  # source file names without this computer's home folder (panic messages keep them); cargo-xwin
  # keeps target flags set beforehand and adds its own
  export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="--remap-path-prefix=$HOME=~"
  npx tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis
  if grep -qaF "$HOME" target/x86_64-pc-windows-msvc/release/erdtex.exe; then
    echo "the Windows program contains this computer's home folder ($HOME)"; exit 1
  fi
  cp target/x86_64-pc-windows-msvc/release/bundle/nsis/ERDTeX_"${VERSION}"_*setup.exe "$ROOT/dist/ERDTeX-$VERSION-setup.exe"
  ls -l "$ROOT/dist/ERDTeX-$VERSION-setup.exe"
fi
