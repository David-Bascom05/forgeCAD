#!/bin/sh
# usage: ic.sh MODE EDGES [ENV..]: instructions per hole for one kind of work (5 rounds)
m=$1; e=$2; shift 2
base=$(FBUILD_MEM=2000M ~/.local/bin/fbuild perf stat -x, -e instructions:u env "$@" COUNT=none ./target/release/examples/rq 20 $e 0 2>&1 >/dev/null | awk -F, '/instructions/{print $1}')
n=$(FBUILD_MEM=2000M ~/.local/bin/fbuild perf stat -x, -e instructions:u env "$@" COUNT=$m ./target/release/examples/rq 20 $e 0 2>&1 >/dev/null | awk -F, '/instructions/{print $1}')
holes=$(env "$@" COUNT=none ./target/release/examples/rq 20 $e 0 | awk '{print $1}')
echo "$m $e: $(( (n - base) / (5 * holes) )) instructions per hole"
