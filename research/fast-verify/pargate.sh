#!/bin/sh
# Prototype (FAST-VERIFY): gate.sh's exact callgrind counts, the 99 lines spread over N CPUs.
# usage: pargate.sh GATEDIR JOBS OUT     GATEDIR: a /tmp/forge-gate.XXXXXX filled as gate.sh's fill() does
# Each line is counted exactly as gate.sh's count1 (exact=1) does, pinned to its worker's CPU.
set -e
bin=$1; jobs=$2; out=$3
lines=$(mktemp)
{
  echo "noop regen1|regen1|noop|"
  echo "noop benchparts|benchparts|noop|"
  for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
    for m in seq nary1; do echo "$p $m|regen1|$p $m|"; done
  done
  for p in gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard; do
    echo "$p app|benchparts|resolve $p|"
    for m in edit cli; do echo "$p $m|benchparts|$m $p|"; done
    for m in seq nary1; do echo "$p $m|benchparts|run $p $m|--toggle-collect=*benchparts::regen"; done
  done
} > "$lines"
one() {
  # $1 = worker index (its CPU), $2 = "key|binary|args|vgonly"
  key=${2%%|*}; rest=${2#*|}; b=${rest%%|*}; rest=${rest#*|}; args=${rest%%|*}; vgonly=${rest#*|}
  only=${vgonly:-"--toggle-collect=$b::main"}
  # shellcheck disable=SC2086
  # each worker its own /tmp/forge-benchparts (benchparts rewrites block.ttf there on every run: two
  # runs at once could read each other's half-written font), at the same path, so counts are unchanged
  mkdir -p "$bin/w$1"
  n=$( (cd / && unshare --mount --propagation private sh -c 'mount --bind "$0" /tmp/forge-benchparts && exec "$@"' "$bin/w$1" env -i PATH=/usr/bin:/bin HOME=/tmp LD_PRELOAD="$bin/fixedrandom.so" \
      FORGE_BENCH_PARTS="$bin/parts" taskset -c "$1" setarch -R \
      valgrind --fair-sched=yes --tool=callgrind --cache-sim=no --callgrind-out-file=/dev/null $only "$bin/$b" $args) 2>&1 >/dev/null |
      awk '/Collected :/{n=$NF}/Process terminating/{bad=1}END{if(!bad&&n!=""&&n>0)print n}')
  echo "$key ${n:-FAIL}"
}
export bin
# worker w takes lines w, w+jobs, ... : static round-robin, each worker on its own CPU
for w in $(seq 0 $((jobs - 1))); do
  ( i=0; while IFS= read -r l; do
      if [ $((i % jobs)) -eq "$w" ]; then one "$((w + ${CPU0:-0}))" "$l"; fi; i=$((i + 1))
    done < "$lines" ) > "$out.$w" &
done
wait
cat "$out".* > "$out.unsorted"; rm -f "$out".[0-9]*
rm -f "$lines"
