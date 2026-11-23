#!/bin/sh
# Some gate lines, counted exactly as gate.sh counts them (callgrind under tools/speed/exact.sh),
# without the rest of the gate: tools/speed/lines.sh PART.. (bench parts: app edit cli seq nary1;
# synthetic parts: seq nary1). GATE_FEATURES and GATE_CORE as for gate.sh. A quick before and
# after for a change aimed at a few parts; the gate is still the judge.
set -e
dir=$(dirname "$0")
export RUSTFLAGS="-C target-cpu=x86-64-v2"
target="${CARGO_TARGET_DIR:-$PWD/target}/callgrind"
fflag=
if [ -n "${GATE_FEATURES:-}" ]; then target="$target-on"; fflag="--features $GATE_FEATURES"; fi
export CARGO_TARGET_DIR="$target"
exact_dir=$target; . "$dir/exact.sh"
cargo build -q --release -p forge-model --example regen1 $fflag
cargo build -q --release -p forge-lang --example benchparts $fflag
bin="$target/release/examples"
count() {
  exact_run valgrind --tool=callgrind --cache-sim=no --callgrind-out-file=/dev/null "$@" 2>&1 >/dev/null |
    awk '/Collected :/{n=$NF}/Process terminating/{bad=1}END{if(!bad&&n!="")print n}'
}
for p in "$@"; do
  case " torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat " in
    *" $p "*)
      for m in seq nary1; do echo "$p $m $(count "$bin/regen1" "$p" "$m")"; done ;;
    *)
      base=$(count "$bin/benchparts" resolve "$p")
      echo "$p app $base"
      for m in edit cli; do echo "$p $m $(count "$bin/benchparts" "$m" "$p")"; done
      for m in seq nary1; do n=$(count "$bin/benchparts" run "$p" "$m"); echo "$p $m $((n - base))"; done ;;
  esac
done
