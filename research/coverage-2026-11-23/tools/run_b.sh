#!/bin/bash
# corpus (b): suite::fuzz(seed, 80) seeds 1..300 via regen1 fuzzN, seq and nary1; merged online per mode
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
cd $WT
B=$CARGO_TARGET_DIR/release/examples
mkdir -p $S/prof/b
for m in seq nary1; do
  fail=0
  t0=$SECONDS
  for seed in $(seq 1 300); do
    LLVM_PROFILE_FILE=$S/prof/b/fuzz-$m-%1m.profraw $B/regen1 fuzz$seed $m 2>>$S/logs/fuzz-$m.err || { fail=$((fail+1)); echo "fail fuzz$seed $m"; }
  done
  echo "fuzz $m: $fail failures of 300, $((SECONDS-t0))s"
done
echo RUN_B DONE
