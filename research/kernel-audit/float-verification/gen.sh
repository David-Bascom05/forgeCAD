#!/bin/bash
# Regenerates lean/FvPredicates/{Types,Funs}.lean from predicates/ with Charon and Aeneas.
# Toolchain as in research/kernel-audit/coverage/aeneas/{setup,build}.sh (branch
# claude/repo-review-mbdrlj-coverage): aeneas 2e3d448 at /home/user/aeneasverif/aeneas, charon
# 9e193f0 (its pin, cloned with submodules) at /home/user/aeneasverif/charon, Rust nightly-2026-09-17.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
C=${CHARON:-/home/user/aeneasverif/charon}
A=${AENEAS:-/home/user/aeneasverif/aeneas}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
(
  cd "$here/predicates"
  PATH=$C/bin:$HOME/.cargo/bin:$PATH RUSTUP_TOOLCHAIN=nightly-2026-09-17 CARGO_TARGET_DIR=$work/target \
    charon cargo --preset=aeneas --opaque 'fv_predicates::big' --dest-file "$work/fv_predicates.llbc"
)
"$A/src/_build/default/main.exe" -backend lean -split-files "$work/fv_predicates.llbc" -dest "$work/out"
# Aeneas writes Lean modules (`module`, `public import`); the IEEE 754 package is not one, and a
# module cannot import it, so drop the module syntax. Types.lean also needs `F64`, which Aeneas
# leaves undefined: FvPredicates/F64.lean defines it as the model's binary64.
for f in Types Funs; do
  sed -e '/^module$/d' -e 's/^public import /import /' -e '/^@\[expose\] public section$/d' \
    "$work/out/$f.lean" > "$here/lean/FvPredicates/$f.lean"
done
sed -i 's/^import Aeneas$/import Aeneas\nimport FvPredicates.F64/' "$here/lean/FvPredicates/Types.lean"
# The opaque functions are defined by hand in FvPredicates/FunsExternal.lean, or by the float
# bridge (verify/aeneas/bridge/AeneasF64/Std.lean); the template only
# lists them, so check that the list has not changed.
grep -oE '(^|\] )axiom [^ ]+' "$work/out/FunsExternal_Template.lean" | sed -E 's/^(\] )?axiom //' | sort > "$work/opaque"
cat "$here/lean/FvPredicates/FunsExternal.lean" "$here"/../../../verify/aeneas/bridge/AeneasF64/*.lean | grep -o '^def [^ ]*' | sed 's/^def //' | sort > "$work/all"
# only the bridge functions this translation asks for
comm -12 "$work/all" "$work/opaque" > "$work/defined"; grep -o '^def [^ ]*' "$here/lean/FvPredicates/FunsExternal.lean" | sed 's/^def //' | sort | comm -23 - "$work/defined" | sed 's/^/extra: /' >&2
if ! diff -u "$work/opaque" "$work/defined"; then
  echo "gen.sh: the opaque functions changed; update FvPredicates/FunsExternal.lean" >&2
  exit 1
fi
