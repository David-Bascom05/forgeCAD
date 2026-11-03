#!/bin/sh
# 1000 random parts in five batches of 200, seeds 1000..1999. From proto/.
fb=$HOME/.local/bin/fbuild
: > ../results/fuzz.txt
for s in 1000 1200 1400 1600 1800; do
  ( $fb target/release/bracket fuzz 200 $s > ../results/.fuzz-$s.txt 2>&1 ) &
  case $s in 1400) wait ;; esac
done
wait
for s in 1000 1200 1400 1600 1800; do echo "seeds $s.."; cat ../results/.fuzz-$s.txt; done > ../results/fuzz.txt
