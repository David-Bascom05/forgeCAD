#!/bin/sh
# The tilted-plane model (Coverage/Tilt.lean) against the kernel (verify/forge-coverage tilt_sweep) on
# the angle grid results/tilt-grid.txt (whole numbers of 1e-13 rad): prints the (family, theta)
# gaps only one side has. Run from this directory after `lake build tilt` and
# `cargo build --release -p forge-coverage --example tilt_sweep`.
set -e
R=results
U=$(cat $R/tilt-grid.txt)
T=$(for n in $U; do printf '%se-13 ' "$n"; done)
THETAS="$T" ../../../target/release/examples/tilt_sweep lid plug cap_over wall wall_over twist ridge \
  > $R/tilt-sweep.tsv 2> $R/tilt-sweep-grid.txt
awk -F'\t' '{split($4,a,"e"); printf "%s\t%d\n", $2, a[1]*10^(a[2]+13)+0.5}' $R/tilt-sweep.tsv | sort -u > $R/tilt-kernel-gaps.tsv
.lake/build/bin/tilt $U | sort -u > $R/tilt-model-gaps.tsv
echo "kernel only:"; comm -23 $R/tilt-kernel-gaps.tsv $R/tilt-model-gaps.tsv
echo "model only:"; comm -13 $R/tilt-kernel-gaps.tsv $R/tilt-model-gaps.tsv
