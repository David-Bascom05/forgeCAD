#!/bin/sh
# The big-model frame-time bench: each STEP model under each interaction, headless.
# usage: sh research/big-render/bench.sh OUTDIR MODEL.step..   (from the checkout root, after
#        cargo build --release --bin forge-render in interface/forge-render)
# Prints, per model and interaction, the CPU's share of a frame (`cpu submit`), the whole frame
# with the GPU wait, and the pick's split. On a machine with no GPU the GPU column is lavapipe's
# software rasterizer: read the CPU column there.
set -e
out=$1; shift
mkdir -p "$out"
bin=${BIN:-interface/forge-render/target/release/forge-render}
frames=${FRAMES:-240}
for m in "$@"; do
  n=$(basename "$m" | sed 's/\.[^.]*$//')
  for v in orbit hover section explode wire moving; do
    case $v in
      orbit) f="";;
      hover) f="--hover";;
      section) f="--section x";;
      explode) f="--explode 1";;
      wire) f="--style wireframe";;
      moving) f="--moving 8";;
    esac
    echo "== $n $v" | tee "$out/$n.$v.txt"
    $bin "$m" --bench --frames "$frames" $f >> "$out/$n.$v.txt" 2>&1 || echo "FAILED $?" >> "$out/$n.$v.txt"
    grep -E "^(step|cpu submit|pick|headless)" "$out/$n.$v.txt" || true
  done
done
