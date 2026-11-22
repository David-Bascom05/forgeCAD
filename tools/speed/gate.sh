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
#                           the same kernel code, compiled without AVX.
# Unset, perf is used when it counts instructions:u and callgrind otherwise. The callgrind gate
# takes ~20 min on 4 busy cores. GATE_REPS=3 keeps the least of 3 counts, for a line near 0.5%.
#
# The callgrind counts are exact: the same binary gives the same count on every line, the bench
# parts' too, from any checkout, at any load. Each counted run is made alike:
#   - the binaries are copied to a fresh mktemp directory, a path of the same length every time
#     (ld.so's work, and argv, grow with the path);
#   - `env -i` with a fixed PATH and HOME (the environment is copied onto the stack), run from
#     `/` (valgrind keeps its starting directory, and its length moves where the client's large
#     blocks are mapped, so whether realloc can grow them in place);
#   - the bench parts' sources copied beside the binaries (FORGE_BENCH_PARTS): read from a path
#     of one length, not from the checkout;
#   - tools/speed/fixedrandom.c preloaded: getrandom gives fixed bytes, so std's HashMap keys are
#     the same every run (they move probe lengths, and with them the heap);
#   - `taskset -c $GATE_CPU` (default 0): one CPU, so the kernel's thread pools
#     (forge_math::par, the fillet's) run their items in order on one thread, and no thread's
#     allocations interleave another's. Which CPU does not change a count: gates run side by
#     side each take their own;
#   - `setarch -R` (no address randomisation) and valgrind's --fair-sched=yes (threads that
#     remain, the build's own, switch in a fixed order).
# This needs cc, taskset and setarch (util-linux). GATE_EXACT=0 counts as before (each run in
# the caller's environment, every core). The counts are not the bare runs': a baseline saved one
# way is compared only with counts made the same way.
set -e
dir=$(dirname "$0")
perf=${PERF:-perf}
counter=${GATE_COUNTER:-}
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
    exact=${GATE_EXACT:-1}
    # (valgrind options for one count: the bench parts' regen lines count one call)
    vgonly=
    count1() {
      if [ "$exact" = 1 ]; then
        (cd / && env -i PATH=/usr/bin:/bin HOME=/tmp LD_PRELOAD="$bin/fixedrandom.so" \
          FORGE_BENCH_PARTS="$bin/parts" taskset -c "${GATE_CPU:-0}" setarch -R \
          valgrind --fair-sched=yes --tool=callgrind --cache-sim=no --callgrind-out-file=/dev/null $vgonly "$@")
      else
        valgrind --tool=callgrind --cache-sim=no --callgrind-out-file=/dev/null $vgonly "$@"
      fi 2>&1 >/dev/null |
        awk '/Collected :/{n=$NF}/Process terminating/{bad=1}END{if(!bad&&n!="")print n}'
    }
    ;;
  *) echo "GATE_COUNTER: perf or callgrind, not $counter" >&2; exit 64 ;;
esac
# the least of GATE_REPS counts (default 1): with GATE_EXACT=0 the bench parts' counts move by up
# to ~0.4% from run to run (std HashMaps' random seeds, thread order, the heap), the synthetic
# parts' by a few instructions; exact counts do not move, and one rep is enough
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
export CARGO_TARGET_DIR="$target"
cargo build -q --release -p forge-model --example regen1 --example bits
cargo build -q --release -p forge-lang --example benchparts
bin="$target/release/examples"
if [ "$counter" = callgrind ] && [ "$exact" = 1 ]; then
  # (the binaries at a path of one length whatever the checkout, and the fixed getrandom)
  gatedir=$(mktemp -d /tmp/forge-gate.XXXXXX)
  trap 'rm -rf "$gatedir"' EXIT
  cp "$bin/regen1" "$bin/bits" "$bin/benchparts" "$gatedir/"
  mkdir "$gatedir/parts"
  cp interface/forge-lang/parts/bench/*.fg "$gatedir/parts/"
  cc -O2 -shared -fPIC -o "$gatedir/fixedrandom.so" "$dir/fixedrandom.c"
  bin=$gatedir
fi
now=$(mktemp)
for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
  for m in seq nary1; do
    n=$(count "$bin/regen1" $p $m)
    echo "$p $m ${n:-FAIL}" >> "$now"
  done
done
# the real parts (interface/forge-lang/parts/bench/*.fg): the kernel's regen of the features each
# part's .fg run sends, by the ids it names them with (`seq`, `nary1`): with callgrind, the
# instructions of that regen call alone (--toggle-collect on benchparts' `regen`), so a kernel
# line is not charged for the heap the language left; with perf, those of building the part and
# regenerating it, less those of building it alone. And the language with them, whole: `app` the cold build as the app makes it (no checks), `edit`
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
    if [ "$counter" = callgrind ]; then
      vgonly='--toggle-collect=*benchparts::regen'
      n=$(count "$bin/benchparts" run $p $m)
      vgonly=
    else
      n=$(count "$bin/benchparts" run $p $m)
      if [ -n "$n" ] && [ -n "$base" ]; then n=$((n - base)); else n=; fi
    fi
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
