#!/bin/bash
# corpus (c): exact-reference comparison, as the forge-exact workflow runs it
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
cd $WT
mkdir -p $S/prof/c
t0=$SECONDS
LLVM_PROFILE_FILE=$S/prof/c/sweepdiff-%1m.profraw $CARGO_TARGET_DIR/release/sweep-diff \
  --seeds 200 --tapers 300 --lofts 100 --digest $S/sweep-diff.digest > $S/logs/sweep-diff.out 2>&1
echo "sweep-diff rc=$? $((SECONDS-t0))s"
diff verify/forge-exact/fuzz/tests/sweep-diff.digest $S/sweep-diff.digest > /dev/null && echo "digest identical" || echo "digest DIFFERS"
t0=$SECONDS
LLVM_PROFILE_FILE=$S/prof/c/tests-%1m.profraw cargo test --release -p forge-exact-fuzz --features reference > $S/logs/exact-tests.out 2>&1
echo "exact tests (CI set) rc=$? $((SECONDS-t0))s"
t0=$SECONDS
LLVM_PROFILE_FILE=$S/prof/c/ignored-%1m.profraw cargo test --release -p forge-exact-fuzz --features reference -- --ignored > $S/logs/exact-ignored.out 2>&1
echo "exact repros (ignored, known-failing) rc=$? $((SECONDS-t0))s"
echo RUN_C DONE
