#!/bin/sh
# run.sh BIN OUT [STEPS]: instructions a frame of each drag in drags.txt
B=$1; O=$2; N=${3:-16}; : > $O; export FORGE_BATCH=cpu
c() { FBUILD_MEM=2500M fbuild perf stat -x, -e instructions:u "$@" 2>&1 >/dev/null | awk -F, '/instructions/{print $1}'; }
one() { a=$(c $B $1 $2 $N 0); b=$(c $B $1 $2 $N $N); echo "$1 $2 $(( (b - a) / N / 100000 ))" >> $O; }
i=0; while read p n; do one $p $n & i=$((i+1)); [ $((i%4)) = 0 ] && wait; done < research/drag-speed/drags.txt; wait
sort -o $O $O
