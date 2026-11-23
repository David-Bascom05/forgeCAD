#!/bin/bash
# corpus (a): gate parts, one profile per part+mode
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
cd $WT
B=$CARGO_TARGET_DIR/release/examples
mkdir -p $S/prof/a
for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
  for m in seq nary1; do
    t0=$SECONDS
    LLVM_PROFILE_FILE=$S/prof/a/regen1-$p-$m.profraw $B/regen1 $p $m; rc=$?
    echo "regen1 $p $m rc=$rc $((SECONDS-t0))s"
  done
done
for p in gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard; do
  for m in seq nary1; do
    t0=$SECONDS
    LLVM_PROFILE_FILE=$S/prof/a/bench-$p-$m.profraw $B/benchparts run $p $m >/dev/null; rc=$?
    echo "bench $p $m rc=$rc $((SECONDS-t0))s"
  done
done
echo RUN_A DONE
