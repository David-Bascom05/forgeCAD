#!/bin/sh
# The browser's CI: build the app, the native references (forgecad, forge-parity), then the
# end-to-end and parity tests in Chromium (software WebGPU, no GPU needed). From the repository
# root: sh interface/forge-render/web/tests/ci.sh [OUT]. Needs Node and Playwright
# (`require("playwright")`, its Chromium installed); PARITY_EXAMPLES=N tests the first N examples;
# THREADED=1 builds and tests the threaded artifact.
set -e
root=$(pwd)
out=${1:-$root/target/web}
cargo build --release --locked -p forge-cli
cd interface/forge-render
rustup target add wasm32-unknown-unknown
command -v wasm-bindgen >/dev/null || cargo install wasm-bindgen-cli --version 0.2.130 --locked
# the browser build stays warning-free (it broke unseen before: a native-only dependency), and
# calls none of std's that panic or fail in a page (web/clippy/clippy.toml)
CLIPPY_CONF_DIR="$PWD/web/clippy" cargo clippy --lib --locked --target wasm32-unknown-unknown -- -D warnings
cargo build --release --locked --bin forge-parity
# THREADED=1: the threaded artifact too, and the tests run on it (forgecad web isolates the page)
NO_MESHES=1 THREADS=$THREADED sh web/build.sh "$out"
node web/tests/e2e.cjs "$out"
node web/tests/idle.cjs "$out"
node web/tests/cam.cjs "$out"
node web/tests/nopanic.cjs "$out"
node web/tests/shade.cjs "$out"
node web/tests/history.cjs "$out"
node web/tests/pace.cjs "$out"
node web/tests/select.cjs "$out"
if [ -n "$PARITY_EXAMPLES" ]; then
  node web/tests/parity.cjs "$out" --examples "$PARITY_EXAMPLES"
else
  node web/tests/parity.cjs "$out"
fi
# the threaded artifact: the page never blocks for a build
if [ -n "$THREADED" ]; then
  node web/tests/responsive.cjs "$out"
fi
