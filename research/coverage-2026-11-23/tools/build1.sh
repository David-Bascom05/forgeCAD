#!/bin/bash
set -e
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
cd $WT
cargo build --release -p forge-model --example regen1 -p forge-lang --example benchparts -p forge-exact-fuzz --bin sweep-diff
cargo test --release -p forge-exact-fuzz --features reference --test repros --no-run
echo BUILD1 DONE
