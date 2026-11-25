#!/bin/sh
# The big-model check in the app: an assembly holding the model (fg/asm_big.fg, the model as
# fg/big.step), an orbit drag, a hover, a click and two parameter edits, each step's UI-thread
# frames timed (forge-app --frame-log: p50/p95/p99 of the frame and of its ui/tess/view/draw).
# usage: sh research/big-render/app.sh MODEL.step [WxH]   (from the checkout root, after
#        cargo build --release --bin forge-app in interface/forge-render)
# `draw` holds the GPU's wait where the driver blocks in submit (lavapipe does): read ui/tess/view
# for the CPU's share there; on a real GPU read `frame`.
set -e
here=$(cd "$(dirname "$0")" && pwd)
bin=${BIN:-$PWD/interface/forge-render/target/release/forge-app}
size=${2:-1600x900}
ln -sf "$(cd "$(dirname "$1")" && pwd)/$(basename "$1")" "$here/fg/big.step"
w=${size%x*}; h=${size#*x}
cx=$((w * 6 / 10)); cy=$((h / 2))
cd "$here/fg"
"$bin" asm_big.fg --size "$size" --shot /dev/null --frame-log \
  --drag $cx $cy $((cx + w / 8)) $((cy + h / 10)) \
  --hover $cx $cy --clickat $cx $cy \
  --set L 420 --sleep 10 --set T 25 --sleep 10 2>&1 | grep -v '^forge-app: opening'
rm -f "$here/fg/big.step"
