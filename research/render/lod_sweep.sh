#!/bin/sh
# The level-of-detail threshold: how many pixels a level's chord may span on screen, against
# frame time and against the picture at a threshold too fine to see. See README.md.
# usage, from interface/forge-render, after cargo build --release: sh ../../research/render/lod_sweep.sh OUTDIR
set -e
out=$1
bin=${CARGO_TARGET_DIR:-target}/release/forge-render
dir=$(dirname "$0")
mkdir -p "$out"
for t in 0.125 0.25 0.5 1 2 4 8; do
  for view in far mid; do
    case $view in far) d=11000 ;; mid) d=3000 ;; esac
    "$bin" --stress 5000 --lod-px "$t" --size 1920x1080 --dist "$d" --shot "$out/$view-$t.ppm" > "$out/$view-$t.txt"
    "$bin" --stress 5000 --lod-px "$t" --size 1920x1080 --dist "$d" --lines none --shot "$out/$view-surface-$t.ppm" > /dev/null
  done
  "$bin" --stress 5000 --lod-px "$t" --bench --frames 600 --size 1920x1080 > "$out/bench-$t.txt"
done
python3 "$dir/compare.py" "$out"
