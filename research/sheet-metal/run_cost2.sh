#!/bin/sh
# Instructions per construction: 1000 direct builds, 100 by-feature builds, mass alone.
fb=$HOME/.local/bin/fbuild
ins() { FBUILD_MEM=3G $fb perf stat -x, -e instructions:u "$@" 2>&1 >/dev/null | awk -F, '/instructions/{print $1}'; }
B=target/release/bracket
z=$(ins $B cost-mass L90 0)
echo "startup_and_one_build $z"
for p in L90 U90 oblique-edge oblique-window hole-in-bend hole-one-line hole-both-lines; do
  echo "direct-fold $p $(( ($(ins $B cost $p fold) - z) / 1000 ))"
  echo "direct-flat $p $(( ($(ins $B cost $p flat) - z) / 1000 ))"
  echo "mass-with-build $p $(( ($(ins $B cost-mass $p 20) - z) / 20 ))"
done
echo "features L $(( ($(ins $B cost-features L) - z) / 100 ))"
echo "features U $(( ($(ins $B cost-features U) - z) / 100 ))"
