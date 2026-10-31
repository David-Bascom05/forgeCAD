#!/bin/sh
# The browser page into OUT, one part's mesh beside it: sh web/build.sh OUT [PART], from
# interface/forge-render. Needs wasm-bindgen-cli 0.2.130 on PATH and the wasm32-unknown-unknown
# target. Serve OUT over http (any static server) and open it in a WebGPU browser.
set -e
out=$1
part=${2:-crank}
target=${CARGO_TARGET_DIR:-target}
cargo build --lib --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir "$out" "$target/wasm32-unknown-unknown/release/forge_render.wasm"
cp web/index.html "$out/"
cargo run --release -- "$part" --export "$out/part.mesh"
ls -la "$out"
