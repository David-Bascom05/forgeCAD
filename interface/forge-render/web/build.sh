#!/bin/sh
# The browser build into OUT: sh web/build.sh OUT, from interface/forge-render. Needs
# wasm-bindgen-cli 0.2.130 on PATH and the wasm32-unknown-unknown target. OUT is static files:
# serve it with `forgecad web DIR --app OUT` (the files of DIR on that machine), or any static
# server (the files kept in the browser). Open it in a WebGPU browser: index.html says what it
# takes. NO_MESHES=1 leaves out the viewer's prebuilt bench meshes (which need a native build).
# THREADS=1 also builds the threaded artifact into OUT/threads (see below).
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
# The threaded artifact: the build on a Web Worker sharing the page's memory, used where the page is
# cross-origin isolated (app.js). The same pinned stable toolchain, with its rust-src: std is
# rebuilt with atomics (-Zbuild-std, which RUSTC_BOOTSTRAP enables on a stable compiler; nothing
# else here uses it). Its own target dir, so the default build is never touched.
if [ -n "$THREADS" ]; then
  rustup component add rust-src >/dev/null
  RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR="$target/threads" \
  RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory \
-C link-arg=--max-memory=4294967296 -C link-arg=--import-memory -C link-arg=--export=__wasm_init_tls \
-C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base" \
    cargo build --lib --release --target wasm32-unknown-unknown -Zbuild-std=std,panic_abort
  mkdir -p "$out/threads"
  wasm-bindgen --target web --no-typescript --out-dir "$out/threads" \
    "$target/threads/wasm32-unknown-unknown/release/forge_render.wasm"
  cp web/worker.js web/threads.js "$out/threads/"
fi
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
