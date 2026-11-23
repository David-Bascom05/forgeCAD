#!/bin/sh
# usage: probe.sh OUTDIR  (env passed through)
W=/home/david-bascom/Documents/Github/forgeCAD/.claude/worktrees/agent-ab9f75fb2464c6365
out=$1; mkdir -p $out; : > $out/status
run() { timeout 900 nice -n 10 $W/target/release/examples/regen1 $1 $2 > /dev/null 2> $out/$1.$2.err; echo "$1 $2 $?" >> $out/status; }
for p in coplanar upto upto-flat hole-grid tour torture shaft mixed plate patterns housing; do
  run $p seq & run $p nary1 & wait
done
for s in $(seq 1 40); do
  run fuzz$s seq &
  if [ $((s % 3)) = 0 ]; then wait; fi
done
wait
echo done
