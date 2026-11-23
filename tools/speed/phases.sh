#!/bin/sh
# The boolean's instructions per phase, on every speed-gate line, under the gate's exact counter
# (callgrind on the x86-64-v2 build, as `GATE_COUNTER=callgrind tools/speed/gate.sh`), with the
# decide-once switch on and off, and a flame profile per line. From a checkout root:
#   tools/speed/phases.sh [OUT] [LINE..]   OUT (default target/phases) gets, per line and switch,
#                                          NAME.cg (callgrind out), NAME.folded (flamegraph.pl's
#                                          input) and NAME.tree (the boolean's call tree as text);
#                                          LINE: `coplanar seq`, `gearbox nary1`, .. (default all)
# PHASES_SWITCH=on|off|both (default both). One table to stdout, a row per line and switch:
#   line, switch, total  the gate's count (a bench part's run less its resolve, as the gate)
#   bool                 inclusive Ir of the boolean's entries (boolean, boolean_into,
#                        boolean_unlinked, nary_with: the old path, and decide-once inside them)
#   arrangement          decide-once: inputs, snap, region, vertices, classes, and the per-plane
#                        sections (arrange)
#   sample               the exact point decisions: point in face (in_face) and the witness rays
#                        (witness_inside)
#   ledger               build_ledger less its arrange and in_face, and forge_bool_core::witnesses
#   core                 forge_bool_core::core and the sliver check
#   write                write (the result body, build_body inside it)
#   rest                 decide-once's nary less the five
# The phases are #[inline(never)] functions of kernel/forge-bool/src/decide_once.rs (each marked
# `a phase of the profile`), so they are the same in every build. Both sessions measure this way.
set -e
dir=$(dirname "$0")
out=${1:-target/phases}
[ $# -gt 0 ] && shift
mkdir -p "$out"
export RUSTFLAGS="-C target-cpu=x86-64-v2"
base="${CARGO_TARGET_DIR:-$PWD/target}"
sw=${PHASES_SWITCH:-both}
build() { # $1 target dir, $2 features
  CARGO_TARGET_DIR="$1" cargo build -q --release -p forge-model --example regen1 $2
  CARGO_TARGET_DIR="$1" cargo build -q --release -p forge-lang --example benchparts $2
}
ons=
case $sw in
  on) ons=on ;; off) ons=off ;; both) ons="off on" ;;
  *) echo "PHASES_SWITCH: on, off or both" >&2; exit 64 ;;
esac
for s in $ons; do
  if [ "$s" = on ]; then build "$base/callgrind-on" "--features forge-bool/decide_once"
  else build "$base/callgrind" ""; fi
done
lines=$*
if [ -z "$lines" ]; then
  for p in torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat; do
    for m in seq nary1; do lines="$lines $p:$m"; done
  done
  for p in gearbox volute crank valve heatsink quadarm conrod jack frame manifold bracket insert clamp hub guard; do
    for m in seq nary1; do lines="$lines $p:$m"; done
  done
else
  # `coplanar seq gearbox nary1` -> coplanar:seq gearbox:nary1
  set -- $lines; lines=
  while [ $# -ge 2 ]; do lines="$lines $1:$2"; shift 2; done
fi
synthetic=" torture plate shaft housing coplanar patterns mixed hole-grid tour upto upto-flat "
cg() { # $1 out file, rest: the command; prints the Ir
  o=$1; shift
  valgrind --tool=callgrind --cache-sim=no --callgrind-out-file="$o" "$@" 2>&1 >/dev/null |
    awk '/Collected :/{n=$NF}/Process terminating/{bad=1}END{if(!bad&&n!="")print n}'
}
fold="python3 $dir/cgfold.py"
inc() { $fold "$1" incl "$2" | awk '{print $2}'; }
printf "%-9s %-5s %-3s %13s %13s %12s %12s %12s %12s %12s %12s\n" line mode sw total bool arrangement sample ledger core write rest
for lm in $lines; do
  p=${lm%%:*}; m=${lm#*:}
  for s in $ons; do
    if [ "$s" = on ]; then bin="$base/callgrind-on/release/examples"; else bin="$base/callgrind/release/examples"; fi
    name="$out/$p-$m-$s"
    case $synthetic in
      *" $p "*) total=$(cg "$name.cg" "$bin/regen1" "$p" "$m") ;;
      *)
        r=$(cg /dev/null "$bin/benchparts" resolve "$p")
        t=$(cg "$name.cg" "$bin/benchparts" run "$p" "$m")
        total=$((t - r)) ;;
    esac
    bool=$($fold "$name.cg" sum forge_bool::boolean::boolean forge_bool::nary::nary_with)
    d=forge_bool::decide_once
    arr=0
    for f in inputs snap region vertices classes arrange; do arr=$((arr + $(inc "$name.cg" "$d::$f"))); done
    samp=$(( $(inc "$name.cg" "$d::in_face") + $(inc "$name.cg" "$d::witness_inside") ))
    led=$(( $(inc "$name.cg" "$d::build_ledger") - $(inc "$name.cg" "$d::arrange") - $(inc "$name.cg" "$d::in_face") + $(inc "$name.cg" forge_bool_core::witnesses) ))
    cor=$(( $(inc "$name.cg" forge_bool_core::core) + $(inc "$name.cg" "$d::sliver") ))
    wr=$(inc "$name.cg" "$d::write")
    nary=$(inc "$name.cg" "$d::nary")
    rest=$((nary - arr - samp - led - cor - wr))
    [ "$nary" -gt 0 ] || rest=0
    printf "%-9s %-5s %-3s %13s %13s %12s %12s %12s %12s %12s %12s\n" "$p" "$m" "$s" "$total" "$bool" "$arr" "$samp" "$led" "$cor" "$wr" "$rest"
    $fold "$name.cg" fold > "$name.folded"
    { $fold "$name.cg" tree forge_bool::boolean::boolean_into 8 0.5
      $fold "$name.cg" tree forge_bool::boolean::boolean_unlinked 8 0.5
      $fold "$name.cg" tree forge_bool::nary::nary_with 8 0.5; } > "$name.tree" 2>/dev/null || true
  done
done
