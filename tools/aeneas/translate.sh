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
# The crate is built as release (`cargo --release`): `#[cfg(debug_assertions)]` checks are not part
# of the shipped code, so they are not part of the translation either.
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
CHARON_ARGS=()
# shellcheck source=/dev/null
. "$conf"
[ -n "$PKG" ] || { echo "$conf: PKG unset" >&2; exit 2; }
src=$root/kernel/$crate
[ -d "$src" ] || { echo "no crate $src" >&2; exit 2; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
name=${crate//-/_}
(
  cd "$src"
  PATH=$H/charon/bin:$PATH RUSTUP_TOOLCHAIN=$TC CARGO_TARGET_DIR=$H/target-charon \
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
      -e "s/^import [A-Za-z_]*\.\(Types\|Funs\|FunsExternal\)$/import $PKG.\1/" \
      "$work/out/$f.lean" > "$work/gen/$f.lean"
done
sed -i 's/^import Aeneas$/import Aeneas\nimport AeneasF64/' "$work/gen/Types.lean"

# The opaque functions Aeneas wants, against what FunsExternal.lean defines.
opaque_list() { { grep -oE '^(@\[[^]]*\] )?axiom [^ ]+' "$1" || true; } | sed -E 's/^.*axiom //' | sort; }
for f in FunsExternal_Template TypesExternal_Template; do
  if [ -f "$work/gen/$f.lean" ]; then opaque_list "$work/gen/$f.lean"; fi
done | sort > "$work/opaque"

dest=$dir/lean/$PKG
# Every opaque item needs a definition (a `def`, or a Lean `opaque`, never an `axiom`) in the
# crate's FunsExternal.lean / TypesExternal.lean or in the float bridge.
missing=0
while read -r n; do
  [ -n "$n" ] || continue
  if ! { cat "$dest/FunsExternal.lean" "$dest/TypesExternal.lean" "$root/verify/aeneas/bridge/AeneasF64/Std.lean" 2>/dev/null || true; } \
      | awk '$1 == "def" || $1 == "opaque" || $1 == "abbrev" { print $2 }' | grep -qxF "$n"; then
    echo "$crate: opaque $n has no definition (FunsExternal.lean, TypesExternal.lean or the bridge)" >&2
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
