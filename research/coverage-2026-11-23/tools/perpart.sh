#!/bin/bash
# one JSON export per profile, forge-bool + forge-geom meet code only, for attributing rare sites
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
set -e
P=$LB/llvm-profdata; C=$LB/llvm-cov
R=$CARGO_TARGET_DIR/release
REPROS=$(ls $R/build/forge-exact-fuzz/*/out/repros-* | grep -v '\.d$' | head -1)
COBJ="$R/examples/regen1 -object $R/examples/benchparts -object $R/sweep-diff -object $REPROS"
mkdir -p $S/pp
for f in $S/prof/a/*.profraw $S/prof/b/*.profraw $S/prof/c/*.profraw; do
  n=$(basename $f .profraw); n=${n%%-[0-9]*_0}
  case $f in */c/*) n=c-$n ;; esac
  $P merge -sparse -o $S/pp/$n.profdata $f
  $C export -format=text -skip-expansions -instr-profile=$S/pp/$n.profdata $COBJ \
    $WT/kernel/forge-bool/src $WT/kernel/forge-geom/src/meet.rs $WT/kernel/forge-geom/src/intersect.rs \
    $WT/kernel/forge-geom/src/intcurve.rs $WT/kernel/forge-geom/src/loftmeet.rs $WT/kernel/forge-geom/src/ruled/meet.rs \
    $WT/kernel/forge-geom/src/spline/meet.rs > $S/pp/$n.json
  rm $S/pp/$n.profdata
done
ls $S/pp | wc -l
echo PERPART DONE
