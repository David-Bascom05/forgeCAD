#!/bin/sh
# Speed and bit gate, from a checkout root: instructions:u for one full regen of each part on each
# path against a baseline (REGRESSED: more than 0.5% over it), and the hash of every regen's exact
# result against tools/speed/bits.txt (CHANGED: expected only when geometry changes on purpose).
# usage: tools/speed/gate.sh [baseline]    exit 1 regressed, 2 bits changed, 3 both
#        tools/speed/gate.sh --save        rewrite tools/speed/baseline.txt and bits.txt
set -e
dir=$(dirname "$0")
cargo build -q --release -p forge-model --example regen1 --example bits
cargo build -q --release -p forge-agent --example benchparts
bin="${CARGO_TARGET_DIR:-$PWD/target}/release/examples"
now=$(mktemp)
fcs=$(mktemp -d)
for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
  for m in seq nary1; do
    n=$(perf stat -x, -e instructions:u "$bin/regen1" $p $m 2>&1 >/dev/null | awk -F, '/instructions/{print $1}')
    echo "$p $m ${n:-FAIL}" >> "$now"
  done
done
# the real parts (tools/bench-parts): selectors resolved first, uncounted
for p in gearbox volute crank valve heatsink quadarm conrod jack frame; do
  "$bin/benchparts" resolve $p "$fcs/$p.fc"
  for m in seq nary1; do
    n=$(perf stat -x, -e instructions:u "$bin/benchparts" run "$fcs/$p.fc" $m 2>&1 >/dev/null | awk -F, '/instructions/{print $1}')
    echo "$p $m ${n:-FAIL}" >> "$now"
  done
done
rm -rf "$fcs"
bits() { "$bin/bits"; "$bin/benchparts" bits; }
if [ "$1" = --save ]; then
  mv "$now" "$dir/baseline.txt"
  bits > "$dir/bits.txt"
  echo "saved $dir/baseline.txt $dir/bits.txt"
  exit 0
fi
status=0
awk 'NR==FNR{b[$1" "$2]=$3;next}{k=$1" "$2; o=b[k]; d=(o>0&&$3!="FAIL")?($3-o)*100/o:0; f=($3=="FAIL"||d>0.5)?"  REGRESSED":""; if(f!="")bad=1
  printf "%-9s %-6s %14s  base %14s  %+6.2f%%%s\n",$1,$2,$3,o,d,f}END{exit bad}' "${1:-$dir/baseline.txt}" "$now" || status=1
rm -f "$now"
if bits | diff "$dir/bits.txt" - > /dev/null; then echo "bits: identical"; else
  echo "bits: CHANGED"; bits | diff "$dir/bits.txt" - | grep '^[<>]' || true; status=$((status | 2)); fi
exit $status
