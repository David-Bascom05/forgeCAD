#!/bin/bash
# (d): unit/integration tests of the four kernel crates (test profile: opt 3 for kernel crates)
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
cd $WT
mkdir -p $S/prof/d
t0=$SECONDS
LLVM_PROFILE_FILE=$S/prof/d/tests-%1m.profraw cargo test --no-fail-fast -p forge-bool -p forge-geom -p forge-topo -p forge-model > $S/logs/tests-d.out 2>&1
echo "tests (d) rc=$? $((SECONDS-t0))s"
cargo test --no-fail-fast -p forge-bool -p forge-geom -p forge-topo -p forge-model --no-run --message-format=json 2>/dev/null | grep -o '"executable":"[^"]*"' | cut -d'"' -f4 > $S/d-objects.txt
echo RUN_D DONE
