#!/bin/bash
# Does an Aeneas output directory elaborate in Lean, against Aeneas's library and the float bridge?
#
#   tools/aeneas/leancheck.sh OUTDIR      (OUTDIR holds Types.lean, Funs.lean and the templates)
#
# The templates' axioms stand in for the opaque functions (this is a compile check, not a proof),
# except the ones the float bridge defines, which are dropped. Prints "lean ok", or "lean:" and
# the first distinct errors. Uses verify/aeneas/forge-math/lean's built environment.
set -uo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
out=$1
proj=$root/verify/aeneas/forge-math/lean
export PATH=$HOME/.elan/bin:$PATH ELAN_TOOLCHAIN=$(cat "$root/verify/aeneas/bridge/lean-toolchain")
[ -f "$out/Funs.lean" ] || { echo "lean: no Funs.lean"; exit 1; }
w=$(mktemp -d)
[ -n "${KEEP:-}" ] || trap 'rm -rf "$w"' EXIT; [ -n "${KEEP:-}" ] && echo "work: $w"
mkdir -p "$w/Sv"
bridge=$(awk '$1 == "def" || $1 == "opaque" || $1 == "abbrev" { print $2 }' "$root/verify/aeneas/bridge/AeneasF64/Std.lean")
fix() { sed -e '/^module$/d' -e 's/^public import /import /' -e '/^@\[expose\] public section$/d' \
  -e 's/^import [A-Za-z_0-9]*\.\(Types\|Funs\|FunsExternal\|TypesExternal\)$/import Sv.\1/' "$1"; }
[ -f "$out/TypesExternal_Template.lean" ] && fix "$out/TypesExternal_Template.lean" > "$w/Sv/TypesExternal.lean"
fix "$out/Types.lean" | sed 's/^import Aeneas$/import Aeneas\nimport AeneasF64/' > "$w/Sv/Types.lean"
[ -f "$out/FunsExternal_Template.lean" ] && fix "$out/FunsExternal_Template.lean" \
  | python3 -c '
import sys, re
drop = set(sys.argv[1].split())
blocks = re.split(r"\n(?=/--|@\[|axiom )", sys.stdin.read())
keep = []
pending = []
for b in blocks:
    m = re.match(r"(?:@\[[^\]]*\]\s*)?axiom ([^ \n]+)", b)
    if b.startswith("/--") or b.startswith("@["):
        if m is None: pending.append(b); continue
    if m and m.group(1) in drop:
        pending = []; continue
    keep.extend(pending); pending = []; keep.append(b)
keep.extend(pending)
print("\n".join(keep))
' "$bridge" > "$w/Sv/FunsExternal.lean"
fix "$out/Funs.lean" > "$w/Sv/Funs.lean"
LP=$(cd "$proj" && lake env printenv LEAN_PATH)
rc=0
for m in TypesExternal Types FunsExternal Funs; do
  [ -f "$w/Sv/$m.lean" ] || continue
  if ! (cd "$w" && LEAN_PATH=$LP:$w timeout 1200 lean "Sv/$m.lean" -o "Sv/$m.olean" > "$w/$m.err" 2>&1); then
    rc=1
    errs=$(grep -E '^Sv/[A-Za-z]+\.lean:[0-9]+:[0-9]+: error' "$w/$m.err" \
      | sed -E 's/^Sv\/[A-Za-z]+\.lean:[0-9]+:[0-9]+: error: //; s/[0-9]+#[a-z0-9]+/N/g' | cut -c1-80 \
      | sort | uniq -c | sort -rn | head -3 | awk '{n=$1; $1=""; printf "%s (%dx); ", substr($0,2), n}')
    [ -n "$errs" ] || errs=$(head -c 200 "$w/$m.err" | tr '\n' ' ')
    echo "lean: $m: $errs"
    break
  fi
done
[ $rc = 0 ] && echo "lean ok"
exit $rc
