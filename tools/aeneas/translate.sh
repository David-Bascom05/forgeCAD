#!/bin/bash
# Translate a kernel crate (the real one, in place) to Lean with Charon and Aeneas.
#
#   tools/aeneas/translate.sh CRATE            write verify/aeneas/CRATE/lean/<Pkg>/{Types,Funs}.lean
#   tools/aeneas/translate.sh CRATE --check    translate into a scratch dir and fail if the committed
#                                              files differ (CI: proofs cannot go stale)
#   tools/aeneas/translate.sh --all [--check]  every verify/aeneas/*/translate.conf
#
# verify/aeneas/CRATE/translate.conf says what to translate, as shell variables:
#   PKG=ForgeMath                  the Lean library (lean/PKG/...)
#   CHARON_ARGS=(--start-from forge_math::predicates::orient2d --opaque ...)
#                                  Charon's item selection; the defaults are --preset=aeneas
#   BRIDGE=none                    a crate with no f64: Types.lean does not import the float
#                                  bridge (AeneasF64), and its lakefile requires Aeneas directly
#   DEPS=(forge-math forge-geom)   the kernel crates this one calls that have their own translation
#                                  here: their items are defined once, by that translation (below)
#   OVERFLOW_CHECKS=false          interim, per crate: translate without overflow checks (integer ops
#                                  as wrapping). The default, and the target, is true (below); a crate
#                                  set to false carries a TODO in its translate.conf.
# The crate is built as release (`cargo --release`) with overflow checks on: `#[cfg(debug_assertions)]`
# checks are not part of the shipped code, so not of the translation either; and integer overflow
# is a failure, so a theorem that a function returns `ok` also proves the shipped build, which
# wraps, computes the same. (Plain release would translate integer ops as wrapping, and Aeneas's
# Lean extraction of a wrapping shift with an `i32` amount does not elaborate.)
#
# Opaque items: Aeneas lists them in FunsExternal_Template.lean, which is committed beside the
# translation. The hand-written lean/PKG/FunsExternal.lean (and the shared float bridge,
# verify/aeneas/bridge) must define every one of them: --check fails if the list changed.
#
# Toolchain: tools/aeneas/setup.sh (FORGE_AENEAS, default ~/.cache/forge-aeneas).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
H=${FORGE_AENEAS:-$HOME/.cache/forge-aeneas}
CHARON=$H/charon/bin/charon
AENEAS=$H/aeneas/bin/aeneas
[ -x "$CHARON" ] && [ -x "$AENEAS" ] || { echo "no toolchain at $H: run tools/aeneas/setup.sh" >&2; exit 2; }
TC=$(cat "$H/charon-toolchain")

if [ "${1:-}" = --all ]; then
  shift
  rc=0
  for conf in "$root"/verify/aeneas/*/translate.conf; do
    "$0" "$(basename "$(dirname "$conf")")" "$@" || rc=1
  done
  exit $rc
fi

crate=${1:?crate name, e.g. forge-math}
mode=${2:-write}
dir=$root/verify/aeneas/$crate
conf=$dir/translate.conf
[ -f "$conf" ] || { echo "no $conf" >&2; exit 2; }
PKG=
BRIDGE=
DEPS=()
OVERFLOW_CHECKS=true
CHARON_ARGS=()
# shellcheck source=/dev/null
. "$conf"
[ -n "$PKG" ] || { echo "$conf: PKG unset" >&2; exit 2; }
src=$root/kernel/$crate
[ -d "$src" ] || { echo "no crate $src" >&2; exit 2; }

work=$(mktemp -d)
# FORGE_KEEP=DIR keeps the scratch translation (logs, raw Aeneas output) there, for debugging
trap 'if [ -n "${FORGE_KEEP:-}" ]; then rm -rf "$FORGE_KEEP"; cp -a "$work" "$FORGE_KEEP"; fi; rm -rf "$work"' EXIT
name=${crate//-/_}
(
  cd "$src"
  PATH=$H/charon/bin:$PATH RUSTUP_TOOLCHAIN=$TC CARGO_TARGET_DIR=$H/target-charon \
    CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=$OVERFLOW_CHECKS \
    "$CHARON" cargo --preset=aeneas "${CHARON_ARGS[@]}" --dest-file "$work/$name.llbc" -- --release
) > "$work/charon.log" 2>&1 || { cat "$work/charon.log" >&2; echo "charon failed on $crate" >&2; exit 1; }
"$AENEAS" -backend lean -split-files "$work/$name.llbc" -dest "$work/out" > "$work/aeneas.log" 2>&1 \
  || { cat "$work/aeneas.log" >&2; echo "aeneas failed on $crate" >&2; exit 1; }
if grep -qiE '^\[Error\]|error' "$work/aeneas.log"; then
  cat "$work/aeneas.log" >&2; echo "aeneas reported errors on $crate" >&2; exit 1
fi

# Aeneas writes Lean modules (`module`, `public import`, `@[expose] public section`); the packages
# they import (verify/ieee754, the bridge) are not modules, so drop the module syntax. The bridge
# defines `F64` and every f64 operation; Types.lean imports it.
mkdir -p "$work/gen"
for f in Types Funs FunsExternal_Template TypesExternal_Template; do
  [ -f "$work/out/$f.lean" ] || continue
  sed -e '/^module$/d' -e 's/^public import /import /' -e '/^@\[expose\] public section$/d' \
      -e "s/^import [A-Za-z_]*\.\(Types\|Funs\|FunsExternal\|TypesExternal\)$/import $PKG.\1/" \
      "$work/out/$f.lean" > "$work/gen/$f.lean"
done
if [ "$BRIDGE" != none ]; then
  sed -i 's/^import Aeneas$/import Aeneas\nimport AeneasF64/' "$work/gen/Types.lean"
fi

# DEPS: the kernel crates this one calls whose own translations (verify/aeneas/DEP) define them.
# Their items keep one definition, the dependency's: the types and functions Charon brings along
# from a dependency (`Source: 'kernel/DEP/...'`) are dropped from this translation, and its
# Types.lean imports the dependency's package. A dependency's functions are opaque here (Charon
# keeps another crate's bodies opaque), and the dependency's translation defines them.
dep_pkgs=()
for d in "${DEPS[@]}"; do
  dconf=$root/verify/aeneas/$d/translate.conf
  [ -f "$dconf" ] || { echo "$crate: DEPS names $d, which has no $dconf" >&2; exit 2; }
  dep_pkgs+=("$(PKG=; . "$dconf" > /dev/null; echo "$PKG")")
done
if [ ${#DEPS[@]} -gt 0 ]; then
  python3 - "$work/gen" "${DEPS[*]}" "${dep_pkgs[*]}" <<'PY'
import os, re, sys
gen, deps, pkgs = sys.argv[1], sys.argv[2].split(), sys.argv[3].split()
def from_dep(chunk):
    m = re.search(r"Source: '([^']*)'", chunk)
    return m is not None and any(m.group(1).startswith("kernel/%s/" % d) for d in deps)
for f in ("Types.lean", "Funs.lean"):
    p = os.path.join(gen, f)
    if not os.path.exists(p):
        continue
    chunks = open(p).read().split("\n\n")
    kept = [c for c in chunks if not (c.lstrip().startswith(("/--", "mutual")) and from_dep(c))]
    text = "\n\n".join(kept)
    if f == "Types.lean":
        text = re.sub(r"^import Aeneas$", "import Aeneas\n" + "\n".join("import " + k for k in pkgs),
                      text, count=1, flags=re.M)
    open(p, "w").write(text)
PY
fi

# The opaque functions Aeneas wants, against what FunsExternal.lean defines.
# (an axiom's name is on its own line, or on the line after a bare `axiom`)
opaque_list() { awk '/^(@\[[^]]*\] )?axiom *$/ { getline; print $1; next } /^(@\[[^]]*\] )?axiom / { sub(/^.*axiom /, ""); print $1 }' "$1" | sort; }
for f in FunsExternal_Template TypesExternal_Template; do
  if [ -f "$work/gen/$f.lean" ]; then opaque_list "$work/gen/$f.lean"; fi
done | sort > "$work/opaque"

dest=$dir/lean/$PKG
# Every opaque item needs a definition (a `def`, or a Lean `opaque`, never an `axiom`) in the
# crate's FunsExternal.lean / TypesExternal.lean or in the float bridge.
missing=0
bridge_files=("$root"/verify/aeneas/bridge/AeneasF64/*.lean)
[ "$BRIDGE" != none ] || bridge_files=()
dep_files=()
for i in "${!DEPS[@]}"; do
  for f in "$root/verify/aeneas/${DEPS[$i]}/lean/${dep_pkgs[$i]}"/*.lean; do dep_files+=("$f"); done
done
# every name a file defines, with the namespace it is in (an opaque of a dependency is named in
# full, `forge_math.vec.Vec3.dot`, and defined in `namespace forge_math` as `vec.Vec3.dot`)
defined() {
  for f in "$@"; do
    [ -f "$f" ] || continue
    awk '{ sub(/^@\[[^]]*\] */, "") }
         $1 == "namespace" { ns = $2 "." } $1 == "end" && $2 != "" { ns = "" }
         $1 == "def" || $1 == "opaque" || $1 == "abbrev" || $1 == "structure" || $1 == "inductive" {
           print $2; if (ns != "") print ns $2 }' "$f"
  done
}
defined "$dest/FunsExternal.lean" "$dest/TypesExternal.lean" "${bridge_files[@]}" "${dep_files[@]}" \
  | sort -u > "$work/defined"
while read -r n; do
  [ -n "$n" ] || continue
  if ! grep -qxF "$n" "$work/defined"; then
    echo "$crate: opaque $n has no definition (FunsExternal.lean, TypesExternal.lean, the bridge or DEPS)" >&2
    missing=1
  fi
done < "$work/opaque"
if [ "$mode" = --check ]; then
  rc=$missing
  for f in Types Funs FunsExternal_Template TypesExternal_Template; do
    [ -f "$work/gen/$f.lean" ] || continue
    if ! diff -q "$work/gen/$f.lean" "$dest/$f.lean" > /dev/null 2>&1; then
      echo "$crate: $dest/$f.lean is stale against kernel/$crate (re-run tools/aeneas/translate.sh $crate)" >&2
      diff -u "$dest/$f.lean" "$work/gen/$f.lean" | head -40 >&2 || true
      rc=1
    fi
  done
  [ $rc = 0 ] && echo "$crate: translation up to date ($(grep -c '^def ' "$dest/Funs.lean") definitions, $(wc -l < "$work/opaque") opaque)"
  exit $rc
fi
mkdir -p "$dest"
for f in Types Funs FunsExternal_Template TypesExternal_Template; do
  if [ -f "$work/gen/$f.lean" ]; then cp "$work/gen/$f.lean" "$dest/$f.lean"; else rm -f "$dest/$f.lean"; fi
done
[ $missing = 0 ] || echo "$crate: define the opaque items above before building the Lean" >&2
echo "$crate: wrote $dest ($(grep -c '^def ' "$dest/Funs.lean") definitions; opaque: $(wc -l < "$work/opaque"))"
sed 's/^/  opaque: /' "$work/opaque"
