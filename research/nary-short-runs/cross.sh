#!/bin/sh
# cross.sh BIN OUT : instructions seq/nary1 on fuzz seeds 1..8 at several lengths
: > $2
for len in 1 2 4 8 12 16 24 32 48 64 80; do for s in 1 2 3 4 5 6 7 8; do
  for m in seq nary1; do
    c=$(FBUILD_MEM=2G ~/.local/bin/fbuild perf stat -x, -e instructions:u $1 run $s $len $m 2>&1 >/dev/null | awk -F, '/instructions/{print $1}')
    printf "%s %s %s %s\n" $len $s $m $c >> $2
  done
done; done
