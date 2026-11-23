#!/bin/sh
# The browser build into OUT: sh web/build.sh OUT, from interface/forge-render. Needs
# wasm-bindgen-cli 0.2.130 on PATH and the wasm32-unknown-unknown target. OUT is static files:
# serve it with `forgecad web DIR --app OUT` (the files of DIR on that machine), or any static
# server (the files kept in the browser). Open it in a WebGPU browser: index.html says what it
# takes. NO_MESHES=1 leaves out the viewer's prebuilt bench meshes (which need a native build).
set -e
out=$1
target=${CARGO_TARGET_DIR:-target}
cargo build --lib --release --target wasm32-unknown-unknown
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" "$target/wasm32-unknown-unknown/release/forge_render.wasm"
cp web/index.html web/app.js web/storage.js web/zip.js "$out/"
# the examples, for a new browser project: the files a part reads, not the pictures and notes
rm -rf "$out/examples"
mkdir -p "$out/examples"
(cd ../../examples && find . -type f \( -name '*.fg' -o -name '*.step' -o -name '*.stp' -o -name '*.ttf' \
  -o -name '*.otf' -o -name '*.csv' -o -name '*.dwg' -o -name '*.dxf' \) | sed 's|^\./||' | sort) > "$out/examples/list.txt"
while read -r f; do
  mkdir -p "$out/examples/$(dirname "$f")"
  cp "../../examples/$f" "$out/examples/$f"
done < "$out/examples/list.txt"
if [ -z "$NO_MESHES" ]; then
  mkdir -p "$out/parts"
  cargo build --release
  names="gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard"
  : > "$out/parts/list.txt"
  for part in $names; do
    "$target/release/forge-render" "$part" --export "$out/parts/$part.mesh"
    echo "$part" >> "$out/parts/list.txt"
  done
fi
du -sh "$out"
