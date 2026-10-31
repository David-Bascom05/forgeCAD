#!/bin/sh
# count.sh BASE NEW parts..   (from the repo root): instructions per exact mass call of each part
# (all its bodies), before and after, and per mesh: R calls counted less a run with none.
base=$1 new=$2; shift 2
ins() { ~/.local/bin/fbuild perf stat -x, -e instructions:u "$@" 2>&1 >/dev/null | awk -F, '/instructions/{print $1}'; }
R=${R:-3}
for p in "$@"; do
  z=$(COUNT=0 ins "$new" $p); n=$(COUNT=$R ins "$new" $p); b=$(COUNT=$R ins "$base" $p); m=$(COUNT=$R MESH=1 ins "$new" $p)
  echo "$p $z $n $b $m $R" | awk '{n=($3-$2)/$6; b=($4-$2)/$6; m=($5-$2)/$6; printf "%-26s base %9.2fM  new %9.2fM (%6.2fx)  mesh %9.2fM  new/mesh %6.2f\n",$1,b/1e6,n/1e6,b/n,m/1e6,n/m}'
done
