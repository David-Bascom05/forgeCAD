#!/bin/sh
# The browser page into OUT, with every bench part's mesh: sh web/build.sh OUT, from
# interface/forge-render. Needs wasm-bindgen-cli 0.2.130 on PATH and the wasm32-unknown-unknown
# target. Serve OUT over http (any static server) and open it in a WebGPU browser: index.html
# says what it takes.
set -e
out=$1
target=${CARGO_TARGET_DIR:-target}
cargo build --lib --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir "$out" "$target/wasm32-unknown-unknown/release/forge_render.wasm"
cp web/index.html "$out/"
mkdir -p "$out/parts"
cargo build --release
names="gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard"
: > "$out/parts/list.txt"
for part in $names; do
  "$target/release/forge-render" "$part" --export "$out/parts/$part.mesh"
  echo "$part" >> "$out/parts/list.txt"
done
du -sh "$out"
