#!/bin/sh
# Speed and bit gate, from a checkout root: instructions:u for one full regen of each part on each
# path against a baseline (REGRESSED: more than 0.5% over it), and the hash of every regen's exact
# result against tools/speed/bits.txt (CHANGED: expected only when geometry changes on purpose).
# usage: tools/speed/gate.sh [baseline]    exit 1 regressed, 2 bits changed, 3 both
#        tools/speed/gate.sh --save        rewrite the counter's baseline and bits.txt
#
# Two counters, each with its own baseline (their counts are not comparable with each other):
#   GATE_COUNTER=perf       `perf stat -e instructions:u` on the normal release build (the
#                           x86-64-v3 of .cargo/config.toml); baseline.txt. PERF=path picks the
#                           perf binary.
#   GATE_COUNTER=callgrind  `valgrind --tool=callgrind --cache-sim=no`, its Ir (instructions run,
#                           user space); baseline-valgrind.txt. For machines with no hardware
#                           counters (VMs: perf says `<not supported>`). Deterministic, about 30x
#                           slower than native. Valgrind 3.22 cannot decode some AVX encodings
#                           LLVM emits for x86-64-v3 (`vmovq xmm, xmm` as VEX 66 0F D6 with a
#                           register operand: SIGILL in Ruled::new, so crank and jack nary1 died
#                           after ~2M instructions), so this mode builds the same source for
#                           x86-64-v2 (RUSTFLAGS, its own target dir: target/callgrind), which
#                           valgrind runs whole. The normal build is untouched; the counts measure
#                           the same kernel code, compiled without AVX. The count is exact
#                           (tools/speed/exact.sh: fixed HashMap seeds, one core, forge-lang's
#                           kernel hash pinned), so any line that moves moved for a reason.
# Unset, perf is used when it counts instructions:u and callgrind otherwise. The callgrind gate
# takes ~20 min on 4 busy cores. GATE_REPS=3 keeps the least of 3 counts, for a line near 0.5%.
# GATE_EXACT=1 is GATE_COUNTER=callgrind (the deterministic count). GATE_FEATURES=f,.. builds with
# those cargo features (`forge-bool/decide_once`: the decide-once switch on), in a target dir of
# its own (target/callgrind-on for the switch); its baseline is still the counter's, which is the
# old path's, so the table is the switch against the old path. tools/speed/phases.sh splits a
# line's boolean into its phases.
set -e
dir=$(dirname "$0")
perf=${PERF:-perf}
counter=${GATE_COUNTER:-}
[ "${GATE_EXACT:-}" = 1 ] && counter=callgrind
feat=${GATE_FEATURES:-}
if [ -z "$counter" ]; then
  if "$perf" stat -x, -e instructions:u true 2>&1 >/dev/null | grep -q '^[0-9][0-9]*,'; then
    counter=perf
  else
    counter=callgrind
  fi
fi
target="${CARGO_TARGET_DIR:-$PWD/target}"
case $counter in
  perf)
    baseline="$dir/baseline.txt"
    count1() { "$perf" stat -x, -e instructions:u "$@" 2>&1 >/dev/null | awk -F, '/instructions/{print $1}'; }
    ;;
  callgrind)
    baseline="$dir/baseline-valgrind.txt"
    target="$target/callgrind"
    export RUSTFLAGS="-C target-cpu=x86-64-v2"
    count1() {
      exact_run valgrind --tool=callgrind --cache-sim=no --callgrind-out-file=/dev/null "$@" 2>&1 >/dev/null |
        awk '/Collected :/{n=$NF}/Process terminating/{bad=1}END{if(!bad&&n!="")print n}'
    }
    ;;
  *) echo "GATE_COUNTER: perf or callgrind, not $counter" >&2; exit 64 ;;
esac
# the least of GATE_REPS counts (default 1): under perf the bench parts' counts move by up to ~0.4%
# from run to run (std HashMaps' random seeds, thread order); under callgrind not at all
count() {
  best=
  for _ in $(seq "${GATE_REPS:-1}"); do
    c=$(count1 "$@")
    if [ -z "$c" ]; then return; fi
    if [ -z "$best" ] || [ "$c" -lt "$best" ]; then best=$c; fi
  done
  echo "$best"
}
echo "counter: $counter (baseline $baseline)"
if [ -n "$feat" ]; then
  # target/callgrind-on (as phases.sh builds it), or target/perf-on
  tag=$(echo "$feat" | tr -c 'a-z0-9\n' -)
  [ "$feat" = forge-bool/decide_once ] && tag=on
  if [ "$counter" = callgrind ]; then target="$target-$tag"; else target="$target/perf-$tag"; fi
  fflag="--features $feat"
else
  fflag=
fi
export CARGO_TARGET_DIR="$target"
# callgrind's count is exact (tools/speed/exact.sh): the same code counts the same
if [ "$counter" = callgrind ]; then exact_dir=$target; . "$dir/exact.sh"; fi
cargo build -q --release -p forge-model --example regen1 --example bits $fflag
cargo build -q --release -p forge-lang --example benchparts $fflag
bin="$target/release/examples"
now=$(mktemp)
for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
  for m in seq nary1; do
    n=$(count "$bin/regen1" $p $m)
    echo "$p $m ${n:-FAIL}" >> "$now"
  done
done
# the real parts (interface/forge-lang/parts/bench/*.fg): the kernel's regen of the features each
# part's .fg run sends, by the ids it names them with: the instructions of building the part and
# regenerating it, less those of building it alone (the language and the first build, uncounted)
# and the language with them, whole: `app` the cold build as the app makes it (no checks), `edit`
# that build and the rebuild after one dimension changed, `cli` the cold build with the CLI's
# checks: what the .fg front end costs on top of the kernel, held from growing
for p in gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard; do
  base=$(count "$bin/benchparts" resolve $p)
  echo "$p app ${base:-FAIL}" >> "$now"
  for m in edit cli; do
    n=$(count "$bin/benchparts" $m $p)
    echo "$p $m ${n:-FAIL}" >> "$now"
  done
  for m in seq nary1; do
    n=$(count "$bin/benchparts" run $p $m)
    if [ -n "$n" ] && [ -n "$base" ]; then n=$((n - base)); else n=; fi
    echo "$p $m ${n:-FAIL}" >> "$now"
  done
done
bits() { "$bin/bits"; "$bin/benchparts" bits; }
if [ "$1" = --save ]; then
  mv "$now" "$baseline"
  bits > "$dir/bits.txt"
  echo "saved $baseline $dir/bits.txt"
  exit 0
fi
status=0
awk 'NR==FNR{b[$1" "$2]=$3;next}{k=$1" "$2; o=b[k]; d=(o>0&&$3!="FAIL")?($3-o)*100/o:0; f=($3=="FAIL"||d>0.5)?"  REGRESSED":""; if(f!="")bad=1
  printf "%-9s %-6s %14s  base %14s  %+6.2f%%%s\n",$1,$2,$3,o,d,f}END{exit bad}' "${1:-$baseline}" "$now" || status=1
rm -f "$now"
if bits | diff "$dir/bits.txt" - > /dev/null; then echo "bits: identical"; else
  echo "bits: CHANGED"; bits | diff "$dir/bits.txt" - | grep '^[<>]' || true; status=$((status | 2)); fi
exit $status
