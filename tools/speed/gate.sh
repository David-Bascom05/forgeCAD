#!/bin/sh
# Speed and bit gate, from a checkout root: instructions:u for one full regen of each part on each
# path against a baseline (REGRESSED: more than 0.5% over it), and the hash of every regen's exact
# result against tools/speed/bits.txt (CHANGED: expected only when geometry changes on purpose).
# usage: tools/speed/gate.sh [baseline]    exit 1 regressed, 2 bits changed, 3 both
#
# The committed baseline-valgrind.txt is a REFERENCE (main's counts on one machine), not the
# pass/fail comparison: another container's glibc or CPU can move whole-process lines by tens to
# hundreds of Ir. A branch is judged by measuring main and the branch on the same machine
# (`gate.sh --save` on each into a scratch copy) and comparing them with tools/speed/band.py, whose
# band is the trains' rule. bits.txt, by contrast, must match exactly everywhere.
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
# Exact callgrind only:
#   GATE_JOBS=N  count the lines N at a time, each worker on its own CPU (GATE_CPU, GATE_CPU+1, ..)
#                and its own temp folder: the same counts as one at a time (exact counts do not
#                depend on load or on which CPU), in the same order. Lines are dealt longest first
#                (by the baseline's counts) to the least loaded worker. Default 1.
#   GATE_MEMO=DIR  counts and bits of a build already counted ON THIS MACHINE, by the hash of
#                everything a count is a function of: the three binaries, the preload, the bench
#                parts, this script, valgrind, the shared libraries the binaries load, the kernel
#                release and the CPU's model and flags. An identical build (a branch that touches
#                no crate the gate's binaries are made from; main counted again) gets the stored
#                counts instead of a recount; anything else is counted and stored. Off by default.
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
# no disk cache on any path (callgrind or perf, exact or not): a cached result would be counted
# as no work at all
export FORGECAD_CACHE=off
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
        # every exact count from the binary's own main (`regen1::main`, `benchparts::main`), unless
        # the line names its own call: what the process does before main (the loader, std's
        # runtime start, its stack-guard probe parsing /proc/self/maps, which scales with the
        # binary's size) and after it is never counted. The process runs on one CPU (taskset), so
        # its work is on main's thread, under main.
        only=${vgonly:-"--toggle-collect=$(basename "$1")::main"}
        # (TMPDIR the gate directory's own: benchparts writes its font to a fixed name in the temp
        # folder on every run, and two gates counting at once read each other's half-written
        # font, gearbox edit off by 364; a path of one length, as FORGE_BENCH_PARTS. And no
        # result cache (FORGECAD_CACHE=off): forge-lang keeps assemblies' results under
        # $HOME/.cache, which every gate and run shares; no bench part is an assembly today, so it
        # is never read, but a count must not depend on what an earlier run left there)
        # (GATE_JOBS: each worker its own, tm0..tm9, a name of the same length)
        (cd / && env -i PATH=/usr/bin:/bin HOME=/tmp LD_PRELOAD="$bin/fixedrandom.so" \
          FORGE_BENCH_PARTS="$bin/parts" TMPDIR="$bin/${tmpname:-tmp}" FORGECAD_CACHE=off taskset -c "${GATE_CPU:-0}" setarch -R \
          valgrind --fair-sched=yes --tool=callgrind --cache-sim=no --callgrind-out-file=/dev/null $only "$@")
      else
        valgrind --tool=callgrind --cache-sim=no --callgrind-out-file=/dev/null $vgonly "$@"
      fi 2>&1 >/dev/null |
        # a count of 0 is a toggle that matched nothing (a renamed main): a failure, never a pass
        awk '/Collected :/{n=$NF}/Process terminating/{bad=1}END{if(!bad&&n!=""&&n>0)print n}'
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
if [ "$counter" = callgrind ] && [ "$exact" = 1 ]; then
  # Built from one fixed source path, whatever the checkout's: the binaries carry their sources'
  # paths (CARGO_MANIFEST_DIR in benchparts, and each path crate's metadata hash, which names its
  # symbols), so two checkouts at paths of different lengths build binaries whose counts differ.
  # The checkout's files (tracked, and untracked ones not ignored: the work in progress) are copied
  # there with their times, under a lock (gates may run at once: each builds in turn, then counts
  # in parallel), and built into this checkout's own target directory, so neither side of a
  # comparison reuses the other's build.
  src=/tmp/forge-gate-src
  checkout=$PWD
  exec 9>"$src.lock"
  flock 9
  rm -rf "$src"
  mkdir "$src"
  git ls-files -co --exclude-standard -z | tar --null -T - -cf - | tar -xf - -C "$src"
  (cd "$src" && cargo build -q --release -p forge-model --example regen1 --example bits \
    && cargo build -q --release -p forge-lang --example benchparts) || exit 1
else
  cargo build -q --release -p forge-model --example regen1 --example bits
  cargo build -q --release -p forge-lang --example benchparts
fi
bin="$target/release/examples"
if [ "$counter" = callgrind ] && [ "$exact" = 1 ]; then
  # (the binaries at a path of one length whatever the checkout, and the fixed getrandom and
  # clock, fixedrandom.c: a clock read under load moved tour seq by up to 24)
  # (the directory's path reaches each counted process in argv[0], LD_PRELOAD and
  # FORGE_BENCH_PARTS: its LENGTH moves the counts (tour seq: 9,441,701 from a 22-character
  # directory, 9,436,604 from 14, 9,437,129 from 41), its letters do not. Always this one form)
  fill() {
    cp "$bin/regen1" "$bin/bits" "$bin/benchparts" "$1/"
    # (benchparts' font folder made here: a counted run that had to create it paid 564 fewer
    # instructions than one finding it there, so the first part counted differed from the rest)
    mkdir -p "$1/parts" "$1/tmp/forge-benchparts"
    cp interface/forge-lang/parts/bench/*.fg "$1/parts/"
    cc -O2 -fno-builtin -ffreestanding -shared -fPIC -o "$1/fixedrandom.so" "$dir/fixedrandom.c"
  }
  gatedir=$(mktemp -d /tmp/forge-gate.XXXXXX)
  trap 'rm -rf "$gatedir"' EXIT
  case $gatedir in
    /tmp/forge-gate.??????) ;;
    *) echo "gate dir $gatedir is not /tmp/forge-gate.XXXXXX: its length would move the counts" >&2; exit 64 ;;
  esac
  fill "$gatedir"
  # (the binaries copied out, the source directory is free for the next gate's build)
  flock -u 9
  # the binaries were built from the fixed source path, not from this checkout's
  if [ "${GATE_SELFTEST:-0}" = 1 ]; then
    # (workspace crates name their sources relatively; benchparts holds its manifest's directory
    # whole, CARGO_MANIFEST_DIR, which must be the fixed one)
    for b in regen1 benchparts; do
      if grep -q -a "$checkout/" "$gatedir/$b"; then
        echo "self-test FAILED: $b carries this checkout's path ($checkout)" >&2
        exit 66
      fi
    done
    if ! grep -q -a "$src/" "$gatedir/benchparts"; then
      echo "self-test FAILED: benchparts was not built from $src" >&2
      exit 66
    fi
    echo "self-test: built from $src, no path of the checkout in the binaries"
  fi
  # GATE_SELFTEST=1: the counts do not depend on which gate directory they are made in (nor on the
  # checkout's: workspace crates embed their sources' paths relative). A second directory of the
  # same form counts the process alone and one small part's line; any difference fails the gate
  if [ "${GATE_SELFTEST:-0}" = 1 ]; then
    other=$(mktemp -d /tmp/forge-gate.XXXXXX)
    fill "$other"
    for a in "noop" "tour seq"; do
      # shellcheck disable=SC2086
      x=$(bin=$gatedir; count "$bin/regen1" $a)
      # shellcheck disable=SC2086
      y=$(bin=$other; count "$bin/regen1" $a)
      echo "self-test regen1 $a: $x in $gatedir, $y in $other"
      if [ -z "$x" ] || [ "$x" != "$y" ]; then
        rm -rf "$other"
        echo "self-test FAILED: the count depends on the gate directory" >&2
        exit 65
      fi
    done
    rm -rf "$other"
  fi
  bin=$gatedir
fi
now=$(mktemp)
jobs=${GATE_JOBS:-1}
# (at most 10: the workers' temp folders tm0..tm9 have the length of `tmp`)
if [ "$jobs" -gt 10 ]; then jobs=10; fi
if [ "$counter" != callgrind ] || [ "$exact" != 1 ]; then jobs=1; fi
joblist=$(mktemp)
# one line of the gate: counted now (one at a time), or listed for the workers (GATE_JOBS)
line() {
  key=$1; shift
  if [ "$jobs" -gt 1 ]; then
    printf '%s|%s|%s\n' "$key" "$vgonly" "$*" >> "$joblist"
  else
    n=$(count "$@")
    echo "$key ${n:-FAIL}" >> "$now"
  fi
}
# GATE_MEMO: the key of this build on this machine
memokey=
if [ -n "${GATE_MEMO:-}" ] && [ "$counter" = callgrind ] && [ "$exact" = 1 ]; then
  memokey=$( {
    sha256sum "$bin/regen1" "$bin/bits" "$bin/benchparts" "$bin/fixedrandom.so" | cut -c1-64
    (cd "$bin/parts" && sha256sum ./*.fg)
    sha256sum < "$0"
    valgrind --version
    ldd "$bin/regen1" "$bin/bits" "$bin/benchparts" | awk '/=> \//{print $3}' | sort -u | xargs sha256sum | cut -c1-64
    uname -r
    grep -m1 '^model name' /proc/cpuinfo; grep -m1 '^flags' /proc/cpuinfo
    echo "reps ${GATE_REPS:-1}"
  } | sha256sum | cut -c1-64)
  mkdir -p "$GATE_MEMO"
fi
if [ -n "$memokey" ] && [ -s "$GATE_MEMO/$memokey.counts" ] && [ -s "$GATE_MEMO/$memokey.bits" ]; then
  echo "memo: $memokey (an identical build counted on this machine: its counts and bits)"
  cp "$GATE_MEMO/$memokey.counts" "$now"
else
# each binary's main alone, given nothing to build (exact counts start at main, so start-up work
# that moves with the binary's layout, glibc reading /proc/self/maps for the main thread's stack
# guard say, is in no line): a constant, which band.py still takes off both sides' whole-process
# lines; with counts from before this gate (start-up included) it is what keeps them comparable
line "noop regen1" "$bin/regen1" noop
line "noop benchparts" "$bin/benchparts" noop
for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
  for m in seq nary1; do
    line "$p $m" "$bin/regen1" $p $m
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
  if [ "$counter" = callgrind ]; then
    line "$p app" "$bin/benchparts" resolve $p
  else
    base=$(count "$bin/benchparts" resolve $p)
    echo "$p app ${base:-FAIL}" >> "$now"
  fi
  for m in edit cli; do
    line "$p $m" "$bin/benchparts" $m $p
  done
  for m in seq nary1; do
    if [ "$counter" = callgrind ]; then
      vgonly='--toggle-collect=*benchparts::regen'
      line "$p $m" "$bin/benchparts" run $p $m
      vgonly=
      continue
    else
      n=$(count "$bin/benchparts" run $p $m)
      if [ -n "$n" ] && [ -n "$base" ]; then n=$((n - base)); else n=; fi
    fi
    echo "$p $m ${n:-FAIL}" >> "$now"
  done
done
if [ "$jobs" -gt 1 ]; then
  # longest first (the baseline's count; a line it lacks first of all) to the least loaded worker;
  # each line keeps its number, so the result is in the order of a count one at a time
  weights=$baseline; if [ -f "${1:-}" ]; then weights=$1; fi; if [ ! -f "$weights" ]; then weights=/dev/null; fi
  awk -F'|' -v wf="$weights" 'BEGIN{while((getline l < wf) > 0){split(l,f," "); w[f[1]" "f[2]]=f[3]}}
    {print (($1 in w)?w[$1]:9e18) "\t" FNR "\t" $0}' "$joblist" | sort -t "$(printf '\t')" -k1,1nr |
    awk -F'\t' -v n="$jobs" -v out="$joblist" '{b=0; for(i=1;i<n;i++) if(load[i]<load[b]) b=i
      load[b]+=$1; print $2 "|" $3 > (out "." b)}'
  cpu0=${GATE_CPU:-0}; ncpu=$(nproc)
  if [ "$jobs" -gt "$ncpu" ]; then
    echo "GATE_JOBS=$jobs is more than the $ncpu CPUs: workers share CPUs (the same counts, slower)" >&2
  fi
  w=0
  while [ "$w" -lt "$jobs" ]; do
    touch "$joblist.$w"
    ( GATE_CPU=$(( (cpu0 + w) % ncpu )); tmpname=tm$w; mkdir -p "$bin/$tmpname/forge-benchparts"
      while IFS='|' read -r i key vgonly cmd; do
        # (cmd word-split on purpose: its words are the binary, under /tmp/forge-gate.XXXXXX,
        # and the part and mode names, none with a space)
        # shellcheck disable=SC2086
        n=$(count $cmd)
        echo "$i $key ${n:-FAIL}"
      done < "$joblist.$w" > "$now.$w" ) &
    w=$((w + 1))
  done
  wait
  cat "$now".[0-9]* | sort -n | cut -d' ' -f2- > "$now"
  rm -f "$now".[0-9]* "$joblist".[0-9]*
fi
fi
rm -f "$joblist"
# (the parts from this checkout: the binaries' own source path is the shared build folder, which a
# gate starting while this one counts empties and refills; its bits then failed, or were cut short)
bits() { "$bin/bits"; FORGE_BENCH_PARTS="$PWD/interface/forge-lang/parts/bench" "$bin/benchparts" bits; }
bitsnow=$(mktemp)
if [ -n "$memokey" ] && [ -s "$GATE_MEMO/$memokey.bits" ]; then cp "$GATE_MEMO/$memokey.bits" "$bitsnow"; else bits > "$bitsnow"; fi
# stored only whole: every line counted, bits written
if [ -n "$memokey" ] && ! grep -q FAIL "$now" && [ -s "$bitsnow" ] && [ ! -s "$GATE_MEMO/$memokey.counts" ]; then
  cp "$now" "$GATE_MEMO/$memokey.counts.$$" && cp "$bitsnow" "$GATE_MEMO/$memokey.bits.$$" &&
    mv "$GATE_MEMO/$memokey.bits.$$" "$GATE_MEMO/$memokey.bits" &&
    mv "$GATE_MEMO/$memokey.counts.$$" "$GATE_MEMO/$memokey.counts"
fi
if [ "$1" = --save ]; then
  mv "$now" "$baseline"
  mv "$bitsnow" "$dir/bits.txt"
  echo "saved $baseline $dir/bits.txt"
  exit 0
fi
status=0
# The rule, line by line:
# - user-facing lines (a bench part's app, edit and cli: whole processes) and the synthetic
#   parts' lines (regen1: the kernel alone, no language heap) must not exceed the baseline: with
#   exact counts, by a single instruction; else by more than 0.5% (the noise of a bare run);
# - a bench part's seq and nary1 lines (its regen call alone, on the heap its language run left)
#   are diagnostic: flagged past 0.5%, and shown, but a change of the language moves them by the
#   heap it leaves (realloc copies), not by kernel code. A change of kernel code shows them at
#   or below the baseline too, or shows by profile that the difference is that alone.
strict=0
if [ "$counter" = callgrind ] && [ "$exact" = 1 ]; then strict=1; fi
synthetic=" torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat "
awk -v strict="$strict" -v syn="$synthetic" 'NR==FNR{b[$1" "$2]=$3;next}
  {k=$1" "$2; o=b[k]; d=(o>0&&$3!="FAIL")?($3-o)*100/o:0
   diag=(index(syn, " "$1" ")==0 && ($2=="seq"||$2=="nary1"))
   over=diag ? d>0.5 : (strict ? (o>0&&$3!="FAIL"&&$3>o) : d>0.5)
   f=($3=="FAIL"||over)?"  REGRESSED":(diag?"  (diagnostic)":""); if($3=="FAIL"||over)bad=1
   printf "%-9s %-6s %14s  base %14s  %+6.2f%%%s\n",$1,$2,$3,o,d,f}END{exit bad}' "${1:-$baseline}" "$now" || status=1
rm -f "$now"
if diff "$dir/bits.txt" "$bitsnow" > /dev/null; then echo "bits: identical"; else
  echo "bits: CHANGED"; diff "$dir/bits.txt" "$bitsnow" | grep '^[<>]' || true; status=$((status | 2)); fi
rm -f "$bitsnow"
exit $status
