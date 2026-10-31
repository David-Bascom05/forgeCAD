#!/bin/sh
# usage: switch.sh: per fillet-corpus case with a corner hole, how each hole closes with builds
# switched to the rational quads (rational, sphere or fallback to today's patch) and the regen's
# instructions today and switched (minus the runner's own start-up, `corpus none`).
# Needs switch.patch applied and corpus.rs in crates/forge-model/examples (see README.md).
B=${CARGO_TARGET_DIR:-$PWD/target}/release/examples
ic() { FBUILD_MEM=2G ~/.local/bin/fbuild perf stat -x, -e instructions:u env "$@" 2>&1 >/dev/null | awk -F, '/instructions/{print $1}'; }
o=$(ic $B/corpus none)
for c in $($B/corpus list | sort -u); do
  paths=$(CORNER_LOG=1 CORNER_RATIONAL=1 $B/corpus $c 2>&1 | awk '/^corner (rational|sphere|fallback) /{print $2"("$3")"}' | sort | uniq -c | awk '{printf "%s x%s ", $2, $1}')
  [ -z "$paths" ] && continue
  a=$(ic $B/corpus $c); b=$(ic CORNER_RATIONAL=1 $B/corpus $c)
  awk -v c=$c -v a=$a -v b=$b -v o=$o -v p="$paths" 'BEGIN{printf "%-24s %8.2f M %8.2f M %5.2fx  %s\n", c, (a-o)/1e6, (b-o)/1e6, (b-o)/(a-o), p}'
done
