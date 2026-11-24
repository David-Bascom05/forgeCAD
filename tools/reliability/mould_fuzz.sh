#!/bin/bash
# The mould mini-fuzz: both mould documents (examples/machined/mold_core_cam.fg and
# mold_core_finish_cam.fg), each part parameter of mold_core.fg scaled by -5, -2, -1, +1, +2
# and +5 % (84 runs). A small parametric edit should not break a program: each line is the run's
# certification and the ops it refused. The CI guard for the class it found (a chord proven at
# the whole tol, tipped past it by the posting digit) is forge-cam-3d's
# steep::tests::the_rows_keep_tol_when_posted.
#
#   tools/reliability/mould_fuzz.sh [OUT]        (OUT: the table, default mould_fuzz.txt)
#
# Known refusals at the time of writing (2026-11-24): bh +5% (the block's walls taller than the
# ball's 16 mm of flutes: a real limit, refused with that fix) in both documents; w -5% (zlevel,
# fixture_collision by 0.000037 mm: the WCS origin off the posting grid, CAM-CORE's fix pending).
set -eu
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${1:-mould_fuzz.txt}
cargo build --release -q -p forge-cam-lang --example cam_time --manifest-path "$ROOT/Cargo.toml"
BIN=${CARGO_TARGET_DIR:-$ROOT/target}/release/examples/cam_time
EX=$ROOT/examples/machined
D=$(mktemp -d)
trap 'rm -rf "$D"' EXIT
: > "$OUT"
for doc in mold_core_cam.fg mold_core_finish_cam.fg; do
  cp "$EX/$doc" "$D/"
  for p in "shared w = 80" "shared d = 60" "shared bw = 40" "shared bd = 28" "shared bh = 16" \
           "param round.r = 5" "param foot.r = 3"; do
    name=${p% = *}
    v=${p##* = }
    for pc in -5 -2 -1 1 2 5; do
      nv=$(python3 -c "print(round($v * (1 + $pc / 100), 4))")
      sed "s/^$name = $v\$/$name = $nv/" "$EX/mold_core.fg" > "$D/mold_core.fg"
      grep -q "^$name = $nv\$" "$D/mold_core.fg" || { echo "no '$p' in mold_core.fg" >&2; exit 1; }
      r=$(CAM_TIME_OPS=1 "$BIN" "$D/$doc" 2>&1 | grep -E "refused|ok (true|false)" | sed 's/^.*\.fg: //' | tr '\n' ' ')
      echo "$doc ${name#* } $nv ($pc%): $r" | tee -a "$OUT"
    done
  done
done
echo "refused: $(grep -c 'ok false' "$OUT") of $(wc -l < "$OUT")"
