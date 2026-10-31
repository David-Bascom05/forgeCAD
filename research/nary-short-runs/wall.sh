#!/bin/sh
# wall.sh OUT : best-of-25 ms, seq and nary 1/2/4, fuzz seeds 1-8 at 12 and 16 tools; load noted
: > $1
for len in 12 16; do for s in 1 2 3 4 5 6 7 8; do
  echo "$(cut -d' ' -f1 /proc/loadavg) $(FORGE_BATCH=cpu FBUILD_MEM=2G ~/.local/bin/fbuild ${NX:-target/release/examples/nx} wall fuzz$s.$len 25)" >> $1
done; done
