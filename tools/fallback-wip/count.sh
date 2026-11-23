#!/bin/sh
# Structural-site counts: regen1 over the suite parts and fuzz seeds, seq and nary1.
# usage: count.sh WORKTREE OUTDIR FUZZ_FROM FUZZ_TO
wt=$1; out=$2; from=${3:-1}; to=${4:-200}
mkdir -p "$out"
bin=$wt/target/release/examples/regen1
export FORGE_DECISIONS_SITES=1 FORGE_DECISIONS_TRACE=1
run() {
  name=$1; mode=$2
  timeout 600 nice -n 10 "$bin" "$name" "$mode" > /dev/null 2> "$out/$name.$mode.err"
  echo "$name $mode exit=$?" >> "$out/exits.txt"
}
if [ "$from" = "suite" ]; then
  for p in plate shaft housing coplanar patterns mixed tour hole-grid upto upto-flat sweep coil loft thread text torture-2 torture; do
    for m in seq nary1; do run "$p" "$m"; done
  done
  exit 0
fi
s=$from
while [ "$s" -le "$to" ]; do
  for m in seq nary1; do run "fuzz$s" "$m"; done
  s=$((s + 1))
done
