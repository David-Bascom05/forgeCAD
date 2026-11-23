#!/bin/sh
# layout_diff.sh GATE_LOG BASE_BIN_DIR NEW_BIN_DIR: for every line over base, the per-function diff
# of the base and new binaries' exact callgrind profiles, and the functions outside libc/ld.so that moved
S=$(dirname "$0")/..
log=$1; bb=$2; nb=$3
grep -v diagnostic "$log" | awk '$4=="base" && $3>$5 {print $1, $2}' | while read p m; do
  case $m in app) bin=benchparts; args="resolve $p";; edit|cli) bin=benchparts; args="$m $p";; *) bin=regen1; args="$p $m";; esac
  for side in base new; do
    d=$bb; [ $side = new ] && d=$nb
    g=$(mktemp -d /tmp/forge-gate.XXXXXX)
    cp "$d/$bin" "$g/$bin"; mkdir "$g/parts"; cp /home/user/forgeCAD/interface/forge-lang/parts/bench/*.fg "$g/parts/"
    cc -O2 -shared -fPIC -o "$g/fixedrandom.so" /home/user/forgeCAD/tools/speed/fixedrandom.c
    (cd / && env -i PATH=/usr/bin:/bin HOME=/tmp LD_PRELOAD="$g/fixedrandom.so" FORGE_BENCH_PARTS="$g/parts" taskset -c 1 setarch -R valgrind --fair-sched=yes --tool=callgrind --cache-sim=no --callgrind-out-file=$S/ld.$side.out "$g/$bin" $args) >/dev/null 2>&1
    rm -rf "$g"
    callgrind_annotate --threshold=100 $S/ld.$side.out 2>/dev/null | grep -E "^ *[0-9,]+ \(" | sed -E 's/^ *([0-9,]+) \([^)]*\) +/\1\t/; s/ \[\/.*//' | sed 's/,//g' > $S/ld.$side.fn
  done
  python3 - "$p $m" $S/ld.base.fn $S/ld.new.fn <<'PY'
import sys
d={}
for f,s in ((sys.argv[2],-1),(sys.argv[3],1)):
    for l in open(f):
        n,name=l.rstrip('\n').split('\t',1); d[name]=d.get(name,0)+s*int(n)
tot=d.pop('PROGRAM TOTALS',0)
lib=lambda k: any(x in k for x in ('memcmp','memmove','memcpy','memset','strlen','malloc','free','_dl_','dl-','do-rel','ld-linux','???:0x'))
moved={k:v for k,v in d.items() if v and not lib(k)}
libd=sum(v for k,v in d.items() if lib(k))
print(f"{sys.argv[1]}: total {tot:+d}; libc/ld.so/heap {libd:+d}; kernel/app functions moved: {len(moved)}"+("" if not moved else " "+"; ".join(f"{v:+d} {k[:90]}" for k,v in sorted(moved.items(),key=lambda x:-abs(x[1]))[:4])))
PY
done
