#!/bin/sh
# table.sh BASE NEW parts..   (from the repo root; BASE and NEW are masstime binaries built from
# the two trees). Per body: exact-mass us before and after (the least of two interleaved runs,
# each the least of several single calls), the mesh's us, and both volumes with their bounds.
base=$1 new=$2; shift 2
run() { ~/.local/bin/fbuild "$@"; }
t=$(mktemp -d)
for p in "$@"; do
  run "$base" $p > $t/b1 2>&1
  run "$new" $p > $t/n1 2>&1
  MASSONLY=1 run "$base" $p > $t/b2 2>&1
  MASSONLY=1 run "$new" $p > $t/n2 2>&1
  paste -d' ' $t/b1 $t/b2 $t/n1 $t/n2 | awk '{
    nb=NF/4; b1=$3; m1=$6; b2=$(nb+3); n1=$(2*nb+3); m2=$(2*nb+6); n2=$(3*nb+3);
    b=(b1<b2)?b1:b2; n=(n1<n2)?n1:n2; m=(m1<m2)?m1:m2;
    printf "%-26s %10.0f %10.0f %7.2fx %10.0f %6.2f  %s %s | %s %s\n",$1,b,n,b/n,m,n/m,$11,$13,$(2*nb+11),$(2*nb+13)}'
done
rm -rf $t
