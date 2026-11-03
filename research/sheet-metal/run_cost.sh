#!/bin/sh
# Instructions for each option on each set, and each construction, by perf stat. From proto/, after a release build.
fb=$HOME/.local/bin/fbuild
ins() { FBUILD_MEM=3G $fb perf stat -x, -e instructions:u "$@" 2>&1 >/dev/null | awk -F, '/instructions/{print $1}'; }
echo "# kind n cases option instructions_total_minus_generation"
for k in isolated overlap boundary tangent coincident mix overlap-lines nudged; do
  for spec in "2 200" "20 100" "200 20"; do
    set -- $spec; n=$1; c=$2
    base=$(ins target/release/trim cost none $k $n 0 $c)
    for o in a bseq bnary; do
      x=$(ins target/release/trim cost $o $k $n 0 $c)
      echo "$k $n $c $o $((x-base))"
    done
  done
done
