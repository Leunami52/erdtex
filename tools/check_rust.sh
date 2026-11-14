#!/usr/bin/env bash
# Every check of the Rust version (desktop_app_rust/).  A milestone is only done when this passes.
set -euo pipefail
cd "$(dirname "$0")/../desktop_app_rust"
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
step() { printf '\n== %s\n' "$1"; }

step "formatting (rustfmt)"
cargo fmt --all -- --check
step "lints (clippy, warnings are errors)"
cargo clippy --workspace --all-targets -- -D warnings
step "tests"
cargo test --workspace --quiet
step "the licences of the crates are listed (tools/licenses.sh)"
if command -v cargo-about > /dev/null; then
  ../tools/licenses.sh --check
else
  echo "skipped: needs cargo-about (cargo install cargo-about --locked --features cli)"
fi
step "the core builds for WebAssembly"
cargo build -p diagram-core --target wasm32-unknown-unknown --release --quiet
step "the page (TypeScript and Svelte)"
[ -d node_modules ] || npm ci --silent
npm run --silent build > /dev/null
npm run --silent wasm:pdf > /dev/null   # the website's PDF export (its types, for svelte-check)
# no path of this computer in what the page ships (the core's file names are remapped, package.json)
if grep -qaF "$HOME" frontend/src/wasm/core_bg.wasm dist/assets/*; then
  echo "the built page contains this computer's home folder ($HOME)"; exit 1
fi
npx svelte-check --tsconfig ./tsconfig.json --threshold error
npm test --silent
# the comparison with version 2 (the Python app) exists in the private archive only
if [ -d oracle ] && [ -d ../desktop_app ]; then
  step "the same results as the Python app"
  mkdir -p target/oracle
  QT_QPA_PLATFORM="${QT_QPA_PLATFORM:-offscreen}" python3 oracle/python_oracle.py > target/oracle/expected.json
  cargo run --quiet -p oracle --release -- target/oracle/expected.json
  step "the Rust SQL runs in real databases"
  # Oracle Free is a release check: python3 oracle/validate_rust_sql.py oracle (README, "Checking a release")
  dbs=(sqlite)
  command -v initdb > /dev/null && dbs+=(postgresql)
  python3 oracle/validate_rust_sql.py "${dbs[@]}"
fi
step "the generated SQL runs in SQLite (and PostgreSQL where installed)"
python3 ../tools/check_sql.py
step "the website in Chromium, Firefox and WebKit (Playwright, e2e-web/)"
npm run --silent build:web > /dev/null
pw=$(node -p "require('./package.json').devDependencies['@playwright/test']")
if [ -n "${CI:-}" ]; then
  # on GitHub Actions the browsers are installed with what they need (npx playwright install --with-deps)
  npx playwright test
elif command -v podman > /dev/null; then
  # Playwright's own image: all three engines with what they need (WebKit wants Ubuntu's libraries)
  podman run --rm -v "$PWD":/work:Z -w /work --network=host --ipc=host "mcr.microsoft.com/playwright:v$pw-noble" npx playwright test
elif [ -d "$HOME/.cache/ms-playwright" ]; then
  npx playwright test --project=chromium --project=firefox
else
  echo "skipped: needs podman, or the browsers (npx playwright install chromium firefox)"
fi
step "nothing private in the public tree (tools/public_tree.py)"
python3 ../tools/public_tree.py --check
step "the window, editing, panels, dialogs, preview and files, driven by WebDriver (on an invisible display where Xvfb is installed)"
if { [ -z "${WAYLAND_DISPLAY:-}${DISPLAY:-}" ] && ! command -v xvfb-run > /dev/null; } || ! command -v WebKitWebDriver > /dev/null \
    || [ ! -x "$HOME/.cargo/bin/tauri-driver" ]; then
  echo "skipped: needs Xvfb (or a display), WebKitWebDriver and tauri-driver (cargo install tauri-driver)"
else
  npx tauri build --no-bundle > /dev/null 2>&1
  # on an invisible display where there is one (Xvfb): the windows don't appear on the desktop,
  # and nothing done there meanwhile can end a test's session
  run_e2e() {
    if command -v xvfb-run > /dev/null; then
      env -u WAYLAND_DISPLAY GDK_BACKEND=x11 xvfb-run -a -s "-screen 0 1920x1200x24" python3 "$@"
    else
      python3 "$@"
    fi
  }
  run_e2e e2e/test_window.py target/e2e
  run_e2e e2e/test_editing.py
  run_e2e e2e/test_panels.py
  run_e2e e2e/test_dialogs.py
  run_e2e e2e/test_files.py
  run_e2e e2e/test_docks.py
  run_e2e e2e/test_groups.py
  run_e2e e2e/test_arrange.py
  run_e2e e2e/test_rulebook.py
fi

printf '\nall checks passed\n'
