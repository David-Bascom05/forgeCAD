#!/bin/sh
# The user-latency gate, from a checkout root: wall-clock and CPU time (user+sys, every thread)
# of what a user waits for, on the bench parts, at one thread and at all cores, head against a
# base built in the same session. Beside gate.sh (instructions), not instead of it: see README.md.
#
# usage: tools/speed/latency.sh --base REV [opts]   compare: head (this checkout) against REV
#        tools/speed/latency.sh --base self [opts]  A/A: head's binary against itself (no alarm
#                                                   expected: the gate's false-alarm check)
#        tools/speed/latency.sh [opts]              record: head alone (absolute medians, CIs,
#                                                   speed-up at all cores)
#        tools/speed/latency.sh bits                results at FORGECAD_THREADS=1, 2 and N: the
#                                                   same as each other and as bits.txt
# opts:  --rounds N   rounds per side (default 12): each a fresh process per part and threads
#        --reps K     most timed reps of an op in a round (default 30; at least 3, then until
#                     they took LATBENCH_BUDGET s, default 0.5), after one warm-up rep
#        --parts "a b .."  (default: every parts/bench/*.fg)
#        --threads "t1 tN" (default both): t1 = FORGECAD_THREADS=1 on one pinned core; tN = every
#                     core, FORGECAD_THREADS unset (what a user gets); tK = FORGECAD_THREADS=K on
#                     every core (e.g. t16, the regen1 gate's nary16). Both sides run each alike.
#        --kparts "a b"  kernel suite parts timed alone (`kern`: kseq, nary), as k-NAME lines
#                     (default coplanar upto shaft mixed patterns plate hole-grid)
#        --heavy      draw every part (default: LAT_DRAW_PARTS, those whose four views take under
#                     ~1 s at 4 cores; the others take 2 to over 100 s)
#        --skip "op,op"  leave those ops out of the bench parts' rounds (e.g. step,mesh,draw,nary)
#        --raw FILE   keep the raw samples (default: a temp file, removed)
#        -o FILE      write the report there too
#        --head REV   head from REV (a worktree) instead of this checkout
# HEAD_FEATURES / BASE_FEATURES: cargo features for that side's build (e.g. forge-bool/decide_once:
# the same commit with a switch off and on, `--base REV --head REV`).
# Environment for validation only (see latbench.rs): HEAD_ENV="VAR=v .." / BASE_ENV, set on one
# side's runs. LAT_CORE: the core of the one-thread runs (default the last).
#
# How a run is made alike on both sides:
#   - both binaries built with the normal release profile (.cargo/config.toml's target-cpu), the
#     base from a git worktree of REV into its own target dir (target/latency/base), so neither
#     build disturbs the other;
#   - every run under `env -i` (PATH, HOME, FORGECAD_CACHE=off: a cold open is cold, no disk
#     cache between processes), from `/`, with the bench parts copied to one folder;
#   - the process's layout held (LAT_LAYOUT=fixed, the default; `random` leaves it as a user's):
#     `setarch -R` (no address randomisation) and getrandom from seedrandom.c with LAT_SEED the
#     round's number, so std's HashMap keys are the same for base and head in a round, and
#     differ from round to round (the rounds average over seeds; a pair differs by code alone);
#   - t1: FORGECAD_THREADS=1 and `taskset -c $LAT_CORE`; tN: every core, FORGECAD_THREADS unset;
#   - one discarded pass of every part on both sides first (page cache, CPU clocks), then rounds
#     interleaved base/head back to back, the order flipped each round (AB BA AB ..), so drift
#     and order cancel in the pairs latstat.py compares.
set -e
dir=$(cd "$(dirname "$0")" && pwd)
root=$PWD
rounds=12
reps=30
parts=
threads="t1 tN"
raw=
out=
base=
mode=record
head=
heavy=
skipops=
kparts="coplanar upto shaft mixed patterns plate hole-grid"
draw_parts=${LAT_DRAW_PARTS:-"bracket clamp conrod guard heatsink hub insert quadarm"}
while [ $# -gt 0 ]; do
  case $1 in
    bits) mode=bits ;;
    --base) base=$2; mode=compare; shift ;;
    --head) head=$2; shift ;;
    --rounds) rounds=$2; shift ;;
    --reps) reps=$2; shift ;;
    --parts) parts=$2; shift ;;
    --threads) threads=$2; shift ;;
    --raw) raw=$2; shift ;;
    --heavy) heavy=1 ;;
    --skip) skipops=$2; shift ;;
    --kparts) kparts=$2; shift ;;
    -o) out=$2; shift ;;
    *) echo "latency.sh: unknown argument $1" >&2; exit 64 ;;
  esac
  shift
done
ncpu=$(nproc)
core=${LAT_CORE:-$((ncpu - 1))}
[ -n "$parts" ] || parts=$(ls interface/forge-lang/parts/bench/*.fg | sed 's|.*/||; s|\.fg$||' | tr '\n' ' ')
target="${CARGO_TARGET_DIR:-$root/target}/latency"
work=$(mktemp -d /tmp/forge-latency.XXXXXX)
trap 'rm -rf "$work"' EXIT

build() { # build SRC_DIR TARGET_DIR DEST [FEATURES]: latbench (and the gate's bit examples)
  # shellcheck disable=SC2086
  (cd "$1" && CARGO_TARGET_DIR="$2" cargo build -q --release -p forge-lang --example latbench \
    --example benchparts ${4:+--features "$4"} &&
    CARGO_TARGET_DIR="$2" cargo build -q --release -p forge-model --example bits ${4:+--features "$4"})
  mkdir -p "$3"
  cp "$2/release/examples/latbench" "$2/release/examples/benchparts" "$2/release/examples/bits" "$3/"
  if [ -f "$1/interface/forge-cam-lang/examples/cam_gonogo.rs" ]; then
    (cd "$1" && CARGO_TARGET_DIR="$2" cargo build -q --release -p forge-cam-lang --example cam_gonogo)
    cp "$2/release/examples/cam_gonogo" "$3/"
  fi
}
# worktree REV SIDE: a checkout of REV at $target/wt-SIDE, with this checkout's latbench.rs (the
# other side may predate it: both measure the same ops)
worktree() {
  wt="$target/wt-$2"
  git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
  git worktree prune
  git worktree add -q --detach "$wt" "$(git rev-parse "$1")"
  cp "$root/interface/forge-lang/examples/latbench.rs" "$wt/interface/forge-lang/examples/"
}
if [ -n "$head" ]; then
  echo "latency: building head ($(git rev-parse --short "$head")${HEAD_FEATURES:+ features $HEAD_FEATURES})" >&2
  worktree "$head" head
  build "$wt" "$target/head" "$work/head" "${HEAD_FEATURES:-}"
  git worktree remove --force "$wt"
else
  echo "latency: building head ($(git rev-parse --short HEAD)$(git diff --quiet || echo +dirty)${HEAD_FEATURES:+ features $HEAD_FEATURES})" >&2
  build "$root" "$target/head" "$work/head" "${HEAD_FEATURES:-}"
fi
mkdir -p "$work/parts"
cp interface/forge-lang/parts/bench/*.fg "$work/parts/"
cc -O2 -shared -fPIC -o "$work/seedrandom.so" "$dir/seedrandom.c"

# run SIDE THREADS CMD..: one process under the run conditions
run() {
  side=$1 thr=$2
  shift 2
  if [ "$side" = head ]; then extra=${HEAD_ENV:-}; else extra=${BASE_ENV:-}; fi
  extra="$extra ${skipenv:-}"
  case $thr in
    t1) pin="taskset -c $core" tenv=FORGECAD_THREADS=1 ;;
    tN) pin="taskset -c 0-$((ncpu - 1))" tenv= ;;
    t*) pin="taskset -c 0-$((ncpu - 1))" tenv=FORGECAD_THREADS=${thr#t} ;;
  esac
  if [ "${LAT_LAYOUT:-fixed}" = fixed ]; then
    lay="LD_PRELOAD=$work/seedrandom.so LAT_SEED=${seed:-0}"
    norand="setarch -R"
  else
    lay= norand=
  fi
  # shellcheck disable=SC2086
  (cd / && env -i PATH=/usr/local/bin:/usr/bin:/bin HOME=/tmp FORGECAD_CACHE=off \
    FORGE_BENCH_PARTS="$work/parts" $tenv $lay $extra $pin $norand "$work/$side/latbench" "$@")
}

if [ "$mode" = bits ]; then
  # every result at 1, 2 and N threads: latbench's (seq, n-ary at the thread count, views) the
  # same at each; the gate's bit suites (bits, benchparts bits) the same as bits.txt at each
  status=0
  for t in 1 2 "$ncpu"; do
    (cd / && env -i PATH=/usr/bin:/bin HOME=/tmp FORGECAD_CACHE=off FORGECAD_THREADS=$t \
      FORGE_BENCH_PARTS="$work/parts" "$work/head/latbench" bits) > "$work/bits.$t"
    (cd / && env -i PATH=/usr/bin:/bin HOME=/tmp FORGECAD_CACHE=off FORGECAD_THREADS=$t \
      FORGE_BENCH_PARTS="$work/parts" sh -c "'$work/head/bits'; '$work/head/benchparts' bits") > "$work/gate.$t"
    if diff "$work/bits.1" "$work/bits.$t" > /dev/null; then r=identical; else r=DIFFERENT; status=2; fi
    if diff "$dir/bits.txt" "$work/gate.$t" > /dev/null; then g=identical; else g=CHANGED; status=2; fi
    echo "threads $t: latbench results $r to threads 1 ($(wc -l < "$work/bits.$t") parts); gate bits $g to bits.txt"
  done
  exit $status
fi

sides=head
if [ "$mode" = compare ]; then
  if [ "$base" = self ]; then
    mkdir -p "$work/base" && cp "$work/head/"* "$work/base/"
  else
    echo "latency: building base ($(git rev-parse --short "$base")${BASE_FEATURES:+ features $BASE_FEATURES})" >&2
    worktree "$base" base
    build "$wt" "$target/base" "$work/base" "${BASE_FEATURES:-}"
    git worktree remove --force "$wt"
  fi
  sides="base head"
fi

# the CAM certify line, where both sides have the CAM crates (built by `build`)
cam=1
for s in $sides; do [ -f "$work/$s/cam_gonogo" ] || cam=; done

samples=${raw:-$work/raw}
: > "$samples"
one() { # one SIDE ROUND PART THREADS: one round's samples, labelled
  seed=$2
  # (--skip and the draw list are the bench parts' ops: a kernel line keeps its kseq and nary)
  sk=$skipops
  if [ -z "$heavy" ] && ! echo " $draw_parts " | grep -q " $3 "; then sk="$sk,draw"; fi
  skipenv=${sk:+LATBENCH_SKIP=$sk}
  case $3 in k-*|cam) skipenv= ;; esac
  if [ "$3" = cam ]; then
    run "$1" "$4" exec "$work/$1/cam_gonogo"
  elif [ "${3#k-}" != "$3" ]; then
    run "$1" "$4" kern "${3#k-}" "$reps"
  else
    run "$1" "$4" ops "$3" "$reps"
  fi | awk -v s="$1" -v r="$2" -v p="$3" -v t="$4" '{print s, r, p, t, $0}' >> "$samples"
}
list="$parts"
for k in $kparts; do list="$list k-$k"; done
[ -n "$cam" ] && list="$list cam"
echo "latency: warm-up pass" >&2
for p in $list; do for s in $sides; do one "$s" 0 "$p" tN; done; done
: > "$samples"
start=$(date +%s)
for r in $(seq "$rounds"); do
  if [ $((r % 2)) = 1 ]; then order=$sides; else order=$(echo "$sides" | awk '{for(i=NF;i>0;i--)printf "%s ",$i}'); fi
  for p in $list; do
    for t in $threads; do
      for s in $order; do one "$s" "$r" "$p" "$t"; done
    done
  done
  echo "latency: round $r/$rounds ($(( $(date +%s) - start )) s)" >&2
done

report() {
  echo "# latency.sh $mode: head ${head:-$(git rev-parse --short HEAD)$(git diff --quiet || echo +dirty)}${HEAD_FEATURES:+ [$HEAD_FEATURES]}${base:+, base $base}${BASE_FEATURES:+ [$BASE_FEATURES]}"
  echo "# $ncpu CPUs ($(grep -m1 'model name' /proc/cpuinfo | sed 's/.*: //')), rounds $rounds, reps $reps;"\
    "threads: t1 = 1 on core $core, tN = $ncpu (all cores, FORGECAD_THREADS unset)"
  [ -n "${HEAD_ENV:-}" ] && echo "# HEAD_ENV=$HEAD_ENV"
  [ -n "${BASE_ENV:-}" ] && echo "# BASE_ENV=$BASE_ENV"
  if [ "$mode" = compare ]; then python3 "$dir/latstat.py" compare "$samples"; else python3 "$dir/latstat.py" record "$samples"; fi
}
status=0
if [ -n "$out" ]; then report > "$out" || status=$?; cat "$out"; else report || status=$?; fi
exit $status
