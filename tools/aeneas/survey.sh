#!/bin/bash
# What of the kernel translates today: Charon and Aeneas over each whole crate, as the code is
# (release, overflow checks on), then Lean on the output; every result attributed to the module
# (source file) it comes from.
#
#   tools/aeneas/survey.sh [CRATE..]        default: every kernel/* crate
#
# Per crate, under $OUT (default $FORGE_AENEAS/survey; not committed):
#   CRATE.charon.log, CRATE.aeneas.log, CRATE.lean.log, CRATE.out/ (the generated Lean)
# and rows appended to $OUT/modules.tsv, one per source file:
#   crate  file  lines  translated  body_failed  sig_failed  lean_errors  aeneas_errors  lean_errors_kinds
# - translated: definitions in the generated Lean whose `Source:` is this file;
# - body_failed / sig_failed: functions of this file whose body / signature Aeneas could not take;
# - lean_errors: Lean errors located (by the nearest `Source:` comment above them) in this file's
#   definitions. The opaque functions are the templates' axioms here: this checks that the output
#   elaborates, it proves nothing.
# A crate whose Charon or Aeneas run fails (out of memory, or an uncaught Aeneas error) is
# reported as such in $OUT/crates.tsv; tools/aeneas/survey-modules.sh then goes module by module.
set -uo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
H=${FORGE_AENEAS:-$HOME/.cache/forge-aeneas}
TC=$(cat "$H/charon-toolchain")
OUT=${OUT:-$H/survey}
mkdir -p "$OUT"
export PATH=$HOME/.elan/bin:$PATH ELAN_TOOLCHAIN=$(cat "$root/verify/aeneas/bridge/lean-toolchain")
LP=$(cd "$root/verify/aeneas/forge-math/lean" && lake env printenv LEAN_PATH)
crates=("$@")
[ ${#crates[@]} -gt 0 ] || for d in "$root"/kernel/*/; do crates+=("$(basename "$d")"); done

for c in "${crates[@]}"; do
  name=${c//-/_}
  w=$OUT/$c.out
  rm -rf "$w"; mkdir -p "$w"
  t0=$(date +%s)
  ( ulimit -v 12000000; cd "$root/kernel/$c" && PATH=$H/charon/bin:$PATH RUSTUP_TOOLCHAIN=$TC \
      CARGO_TARGET_DIR=$H/target-survey/$c CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true \
      timeout 3600 "$H/charon/bin/charon" cargo --preset=aeneas --dest-file "$w/x.llbc" -- --release ) \
    > "$OUT/$c.charon.log" 2>&1
  status=ok
  if [ ! -f "$w/x.llbc" ]; then
    if grep -q 'memory allocation of' "$OUT/$c.charon.log"; then status="charon: out of memory (12 GB cap)"
    else status="charon: $(grep -m1 -E '^error|panicked' "$OUT/$c.charon.log" | cut -c1-100)"; fi
  else
    ( ulimit -v 12000000; timeout 3600 "$H/aeneas/bin/aeneas" -backend lean -split-files "$w/x.llbc" -dest "$w" ) \
      > "$OUT/$c.aeneas.log" 2>&1
    sed -i -e 's/\x1b\[[0-9;?]*[a-zA-Z]//g' -e 's/\r/\n/g' "$OUT/$c.aeneas.log"
    if [ ! -f "$w/Funs.lean" ]; then
      status="aeneas: $(grep -m1 -E '^Fatal error' "$OUT/$c.aeneas.log" | cut -c1-80); last error: $(grep '^\[Error\]' "$OUT/$c.aeneas.log" | tail -1 | cut -c1-80)"
    else
      # one Lean file, so that an error early on does not stop the rest from being checked
      bridge=$(cat "$root"/verify/aeneas/bridge/AeneasF64/*.lean | awk '$1 == "def" || $1 == "opaque" || $1 == "abbrev" { print $2 }')
      python3 - "$w" "$bridge" > "$w/All.lean" <<'PY'
import sys, re, os
w, drop = sys.argv[1], set(sys.argv[2].split())
out = ["import Aeneas", "import AeneasF64"]
def body(f):
    p = os.path.join(w, f)
    if not os.path.exists(p): return ""
    s = open(p).read()
    s = re.sub(r"^(module|@\[expose\] public section|(public )?import .*)$", "", s, flags=re.M)
    return s
for f in ["TypesExternal_Template.lean", "Types.lean", "FunsExternal_Template.lean", "Funs.lean"]:
    s = body(f)
    if f == "FunsExternal_Template.lean":
        # drop the axioms the bridge defines (with their doc comment and attributes)
        s = re.sub(r"/--(?:(?!-/).)*-/\s*(?:@\[[^\]]*\]\s*)*axiom ([^\s]+)(?:(?!\n/--|\n@\[|\naxiom|\nend ).)*",
                   lambda m: "" if m.group(1) in drop else m.group(0), s, flags=re.S)
    out.append(f"-- ==== {f}\n" + s)
print("\n".join(out))
PY
      # (no ulimit -v here: Lean reserves address space for its threads and aborts under one)
      ( cd "$w" && LEAN_PATH=$LP timeout 3600 lean All.lean ) > "$OUT/$c.lean.log" 2>&1
      rc=$?
      if [ $rc = 124 ]; then status="lean: timeout"
      elif [ $rc -gt 1 ]; then status="lean: crashed ($(head -c 80 "$OUT/$c.lean.log"))"; fi
    fi
  fi
  # attribute everything to source files
  python3 - "$root" "$c" "$w" "$OUT" "$status" <<'PY'
import sys, re, os, collections
root, crate, w, out, status = sys.argv[1:6]
src = f"kernel/{crate}/src/"
files = sorted(os.path.relpath(os.path.join(d, f), root) for d, _, fs in os.walk(os.path.join(root, src))
               for f in fs if f.endswith(".rs"))
tr = collections.Counter(); bodyf = collections.Counter(); sigf = collections.Counter()
lean = collections.Counter(); kinds = collections.defaultdict(collections.Counter)
aerr = collections.defaultdict(collections.Counter)
def fileof(span):
    m = re.search(r"'(kernel/[^']+\.rs)'", span)
    return m.group(1) if m else None
alog = os.path.join(out, f"{crate}.aeneas.log")
if os.path.exists(alog):
    last_err = ""
    lines = open(alog, errors="replace").read().split("\n")
    for i, l in enumerate(lines):
        if l.startswith("[Error]"):
            last_err = l[8:].strip()[:70]
        m = re.search(r"Could not translate the (body|function signature) of", l)
        if m:
            for k in range(i, min(i + 8, len(lines))):
                if lines[k].startswith("Definition span:"):
                    f = fileof(lines[k])
                    if f:
                        (bodyf if m.group(1) == "body" else sigf)[f] += 1
                        aerr[f][last_err] += 1
                    break
allp = os.path.join(w, "All.lean")
if os.path.exists(allp):
    src_at = []  # line -> file of the nearest Source: above
    cur = None
    for l in open(allp, errors="replace"):
        m = re.search(r"Source: '(kernel/[^']+\.rs)'", l)
        if m: cur = m.group(1)
        if l.startswith("def ") or l.startswith("noncomputable def ") or l.startswith("structure ") or l.startswith("inductive "):
            if cur: tr[cur] += 1
        src_at.append(cur)
    llog = os.path.join(out, f"{crate}.lean.log")
    if os.path.exists(llog):
        for l in open(llog, errors="replace"):
            m = re.match(r"All\.lean:(\d+):\d+: error(?:\([^)]*\))?: (.*)", l)
            if m:
                n = int(m.group(1)) - 1
                f = src_at[n] if 0 <= n < len(src_at) else None
                f = f or "(library templates)"
                lean[f] += 1
                kinds[f][re.sub(r"\d+#\w+", "N", m.group(2))[:60]] += 1
rows = []
for f in files + [k for k in lean if k not in files]:
    lines_ = sum(1 for _ in open(os.path.join(root, f))) if os.path.exists(os.path.join(root, f)) else 0
    k = "; ".join(f"{a} ({n}x)" for a, n in kinds[f].most_common(3))
    e = "; ".join(f"{a} ({n}x)" for a, n in aerr[f].most_common(3))
    rows.append("\t".join(map(str, [crate, f, lines_, tr[f], bodyf[f], sigf[f], lean[f], e, k])))
open(os.path.join(out, "modules.tsv"), "a").write("".join(r + "\n" for r in rows))
tot = lambda c: sum(c.values())
open(os.path.join(out, "crates.tsv"), "a").write(
    f"{crate}\t{status}\t{tot(tr)}\t{tot(bodyf)}\t{tot(sigf)}\t{tot(lean)}\n")
print(f"{crate:14} {status[:60]:60} defs={tot(tr)} body_failed={tot(bodyf)} sig_failed={tot(sigf)} lean_errors={tot(lean)}")
PY
  echo "  ($(( $(date +%s) - t0 ))s)"
done
