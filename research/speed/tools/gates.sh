#!/bin/sh
# a step's gates: workspace build and clippy, forge-lang and forge-model tests (each binary),
# bits, the scoreboard, FUZZ_SEEDS=150, forge-render's tests
W=${W:-$(git rev-parse --show-toplevel)}
cd $W || exit 1
echo "== fmt"; cargo fmt --all --check >/dev/null 2>&1 && echo ok || echo FMT-DIFFS
echo "== build workspace"; cargo build -q --release --workspace --all-targets 2>&1 | grep -E "^(error|warning)" -A5 | head -20; echo "exit done"
echo "== clippy workspace"; cargo clippy -q --workspace --all-targets -- -D warnings 2>&1 | grep -E "^(error|warning)" -A8 | head -30; echo "clippy done"
for c in forge-lang forge-model; do
  d=$W/interface/$c; [ -d $d ] || d=$W/kernel/$c
  echo "== tests $c"
  cargo test -q --release -p $c --no-run 2>&1 | grep -E "^error" | head
  cd $d
  for f in $(cargo test --release -p $c --no-run 2>&1 | grep -o 'target/release/deps/[^)]*'); do
    $W/$f 2>&1 | grep -E "test result|FAILED|panicked" | grep -v "test result: ok" | head -5
    echo "ran $(basename $f)"
  done
  cd $W
done
echo "== bits"; (target/release/examples/bits; target/release/examples/benchparts bits) | diff tools/speed/bits.txt - && echo "bits identical"
echo "== reliability"; cargo run -q --release -p forge-model --example reliability -- --baseline tools/reliability/baseline.txt 2>&1 | tail -3
echo "== suite"; FUZZ_SEEDS=150 cargo run -q --release -p forge-model --example suite 2>&1 | tail -2
echo "== nary"; FUZZ_SEEDS=150 cargo run -q --release -p forge-model --example nary 2>&1 | tail -1
echo "== forge-render"; cd $W/interface/forge-render && cargo test --release --no-fail-fast 2>&1 | grep -E "test result|FAILED|panicked|^error" | head -40
echo GATESDONE
