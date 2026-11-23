#!/bin/bash
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
set -e
P=$LB/llvm-profdata; C=$LB/llvm-cov
R=$CARGO_TARGET_DIR/release
REPROS=$(ls $R/build/forge-exact-fuzz/*/out/repros-* | grep -v '\.d$' | head -1)
COBJ="$R/examples/regen1 -object $R/examples/benchparts -object $R/sweep-diff -object $REPROS"
for n in tests ignored; do
  $P merge -sparse -o $S/pp/c-$n.profdata $S/prof/c/$n-*.profraw
  $C export -format=text -skip-expansions -instr-profile=$S/pp/c-$n.profdata $COBJ \
    $WT/kernel/forge-bool/src $WT/kernel/forge-geom/src/meet.rs $WT/kernel/forge-geom/src/intersect.rs \
    $WT/kernel/forge-geom/src/intcurve.rs $WT/kernel/forge-geom/src/loftmeet.rs $WT/kernel/forge-geom/src/ruled/meet.rs \
    $WT/kernel/forge-geom/src/spline/meet.rs > $S/pp/c-$n.json
  rm $S/pp/c-$n.profdata
done
echo PP2 DONE
