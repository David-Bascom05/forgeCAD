#!/bin/sh
# wasm speed: instructions:u and cycles:u of one regen of each gate part as wasm under Node (the
# main thread only, every function compiled optimized up front), native's beside it and the ratio;
# REGRESSED: wasm instructions more than 5% over the baseline (the smallest parts vary by a few %). Then
# the wasm build's bits against tools/speed/bits.txt.
# usage: tools/speed/wasm.sh [baseline]    exit 1 regressed, 2 bits differ, 3 both
#        tools/speed/wasm.sh --save        rewrite tools/speed/wasm-baseline.txt
set -e
dir=$(dirname "$0")
export FORGE_BATCH=cpu
cargo build -q --release -p forge-model --example regen1
cargo build -q --profile wasm -p forge-model --example regen1 --example bits --target wasm32-wasip1
t="${CARGO_TARGET_DIR:-$PWD/target}"
count() { o=$(perf stat -i -x, -e instructions:u,cycles:u "$@" 2>&1 >/dev/null) || o=; echo "$o" | awk -F, '/instructions/{i=$1}/cycles/{c=$1}END{print i+0, c+0}'; }
wasm() { count node --no-turbo-fast-api-calls --no-liftoff --no-wasm-tier-up --no-wasm-lazy-compilation --no-warnings "$dir/wasm.mjs" "$t/wasm32-wasip1/wasm/examples/regen1.wasm" "$@"; }
now=$(mktemp)
for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
  for m in seq nary1; do
    echo "$p $m $(wasm 5 $p $m) $(wasm 1 $p $m) $(count "$t/release/examples/regen1" $p $m)" |
      awk '$3&&$4&&$5&&$6&&$7&&$8{printf "%s %s %d %d %d %d\n",$1,$2,($3-$5)/4,($4-$6)/4,$7,$8;next}{print $1,$2,"FAIL 0 1 1"}' >> "$now"
  done
done
if [ "$1" = --save ]; then
  cut -d' ' -f1-3 "$now" > "$dir/wasm-baseline.txt"
  rm -f "$now"
  echo "saved $dir/wasm-baseline.txt"
  exit 0
fi
echo "part      path     wasm instr       base   change   native instr  instr x  cycles x"
awk 'NR==FNR{b[$1" "$2]=$3;next}{o=b[$1" "$2]; d=o>0?($3-o)*100/o:0; f=($3=="FAIL"||d>5)?"  REGRESSED":""; if(f!="")bad=1
  printf "%-9s %-6s %12d %12d %+6.2f%%  %12d  %6.2f  %6.2f%s\n",$1,$2,$3,o,d,$5,$3/$5,$4/$6,f}END{exit bad}' \
  "${1:-$dir/wasm-baseline.txt}" "$now" || status=1
rm -f "$now"
status=${status:-0}
if node --no-turbo-fast-api-calls --no-warnings "$dir/../wasi-run.mjs" "$t/wasm32-wasip1/wasm/examples/bits.wasm" | diff "$dir/bits.txt" - > /dev/null; then
  echo "wasm bits: identical"; else echo "wasm bits: DIFFER"; status=$((status | 2)); fi
exit $status
