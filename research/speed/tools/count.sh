#!/bin/sh
# usage: count.sh BIN OUTDIR TAG ENG MODE [parts..]  -> lines "part eng mode Ir"
bin=$1; out=$2; tag=$3; eng=$4; mode=$5; shift 5
parts=${*:-"gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard"}
mkdir -p $out
for p in $parts; do
  f=$out/$tag.$p.$eng.$mode.cg
  valgrind --tool=callgrind --cache-sim=no --callgrind-out-file=$f $bin $p $eng $mode >/dev/null 2>$f.log
  n=$(grep -m1 '^summary:' $f | awk '{print $2}')
  [ -z "$n" ] && n=$(grep -m1 '^totals:' $f | awk '{print $2}')
  echo "$p $eng $mode ${n:-FAIL}"
done
