#!/bin/bash
# merge profiles, export JSON + text + HTML, per-part summaries
. /tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/cov/env.sh
set -e
OUT=$HOME/Documents/forgecad-coverage-2026-11-23
mkdir -p $OUT/text $OUT/per-part $S/m
P=$LB/llvm-profdata; C=$LB/llvm-cov
R=$CARGO_TARGET_DIR/release
REPROS=$(ls $R/build/forge-exact-fuzz/*/out/repros-* | grep -v '\.d$' | head -1)
COBJ="$R/examples/regen1 -object $R/examples/benchparts -object $R/sweep-diff -object $REPROS"
TOBJ=$(grep -v '^$' $S/d-objects.txt | sed 's/^/-object /' | tr '\n' ' ' | sed 's/^-object //')
SRC="$WT/kernel/forge-bool/src $WT/kernel/forge-geom/src $WT/kernel/forge-topo/src"
$P merge -sparse -o $S/m/a.profdata $S/prof/a/*.profraw
$P merge -sparse -o $S/m/b.profdata $S/prof/b/*.profraw
$P merge -sparse -o $S/m/c.profdata $S/prof/c/*.profraw
$P merge -sparse -o $S/m/abc.profdata $S/m/a.profdata $S/m/b.profdata $S/m/c.profdata
$P merge -sparse -o $S/m/d.profdata $S/prof/d/*.profraw
for g in a b c abc; do
  $C report -show-branch-summary -show-region-summary -instr-profile=$S/m/$g.profdata $COBJ $SRC > $OUT/text/corpus-$g.txt
done
$C report -show-branch-summary -show-region-summary -instr-profile=$S/m/d.profdata $TOBJ $SRC > $OUT/text/tests-d.txt
$C export -format=text -instr-profile=$S/m/abc.profdata $COBJ $SRC > $S/m/abc.json
$C export -format=text -instr-profile=$S/m/d.profdata $TOBJ $SRC > $S/m/d.json
$C show -format=html -show-branches=count -show-line-counts-or-regions -instr-profile=$S/m/abc.profdata $COBJ \
  $WT/kernel/forge-bool/src $WT/kernel/forge-geom/src $WT/kernel/forge-topo/src -output-dir=$OUT/html-corpus
$C show -format=html -show-branches=count -show-line-counts-or-regions -instr-profile=$S/m/d.profdata $TOBJ \
  $WT/kernel/forge-bool/src $WT/kernel/forge-geom/src $WT/kernel/forge-topo/src -output-dir=$OUT/html-tests
for f in $S/prof/a/*.profraw $S/prof/b/*.profraw; do
  n=$(basename $f .profraw)
  $P merge -sparse -o $S/m/part.profdata $f
  $C report -show-branch-summary -instr-profile=$S/m/part.profdata $COBJ $WT/kernel/forge-bool/src > $OUT/per-part/$n.txt
done
echo REPORT DONE
