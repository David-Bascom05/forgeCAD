#!/bin/bash
# The pinned Aeneas toolchain: Charon (Rust -> LLBC) and Aeneas (LLBC -> Lean), plus elan/Lean.
#
#   tools/aeneas/setup.sh            build everything into $FORGE_AENEAS (default ~/.cache/forge-aeneas)
#   tools/aeneas/setup.sh --check    print the pins and whether the built tools match them
#
# The pins are the ones kernel/forge-bool-core was translated with. Changing one changes every
# committed translation under verify/aeneas: re-run tools/aeneas/translate.sh --all and commit.
# Idempotent: each step is skipped when its output is already there at the pinned commit.
# Needs: git, curl, cc, m4, pkg-config, libgmp-dev (apt; installed here when sudo/root allows).
set -euo pipefail

AENEAS_REV=2e3d4488280bb8a95411db81019a7854a784861f   # aeneas, whose charon-pin names charon:
CHARON_REV=9e193f0                                     # (checked against aeneas/charon-pin below)
OPAM_VERSION=2.3.0
OCAML_SWITCH=5.3.0
LEAN_TOOLCHAIN=leanprover/lean4:v4.36.0                # = aeneas/backends/lean/lean-toolchain

H=${FORGE_AENEAS:-$HOME/.cache/forge-aeneas}
mkdir -p "$H/bin"
export OPAMROOT=$H/opam OPAMYES=1 OPAMCOLOR=never OPAMCONFIRMLEVEL=unsafe-yes
export PATH=$H/bin:$HOME/.elan/bin:$HOME/.cargo/bin:$PATH
say() { printf '\n== %s\n' "$*"; }

if [ "${1:-}" = --check ]; then
  echo "aeneas pin $AENEAS_REV"; echo "charon pin $CHARON_REV"; echo "lean $LEAN_TOOLCHAIN"
  [ -d "$H/aeneas" ] && echo "aeneas built at $(git -C "$H/aeneas" rev-parse HEAD)"
  [ -d "$H/charon" ] && echo "charon built at $(git -C "$H/charon" rev-parse HEAD)"
  [ -x "$H/aeneas/bin/aeneas" ] && [ -x "$H/charon/bin/charon" ] && echo ok
  exit 0
fi

say "system packages"
if ! { [ -e /usr/include/gmp.h ] || [ -e /usr/include/x86_64-linux-gnu/gmp.h ]; }; then
  SUDO=; [ "$(id -u)" = 0 ] || SUDO=sudo
  $SUDO apt-get update -qq && $SUDO apt-get install -y -qq libgmp-dev pkg-config m4
fi

say "sources at the pins"
fetch() { # dir url rev
  [ -d "$1/.git" ] || git clone -q "$2" "$1"
  if [ "$(git -C "$1" rev-parse HEAD)" != "$(git -C "$1" rev-parse "$3^{commit}" 2>/dev/null || echo x)" ]; then
    git -C "$1" fetch -q origin && git -C "$1" checkout -q "$3"
  fi
}
fetch "$H/aeneas" https://github.com/AeneasVerif/aeneas "$AENEAS_REV"
PIN=$(tail -1 "$H/aeneas/charon-pin")
case "$PIN" in "$CHARON_REV"*) ;; *) echo "aeneas/charon-pin is $PIN, not $CHARON_REV"; exit 2;; esac
fetch "$H/charon" https://github.com/AeneasVerif/charon "$PIN"
git -C "$H/charon" submodule update --init --recursive -q   # crates/minirust (miniutil)
ln -sfn "$H/charon" "$H/aeneas/charon"

say "forge-aeneas patches"
# tools/aeneas/patches/aeneas-*.patch: f64 support (aeneas-f64: the float bridge's tool side) and
# robustness (aeneas-robust: one untranslatable trait method no longer stops a crate). Applied once;
# a tree that already has them is left alone, a tree that has something else stops here.
for p in "$(cd "$(dirname "$0")" && pwd)"/patches/aeneas-*.patch; do
  if git -C "$H/aeneas" apply --check "$p" 2>/dev/null; then git -C "$H/aeneas" apply "$p"
  elif git -C "$H/aeneas" apply -R --check "$p" 2>/dev/null; then :
  else echo "cannot apply $p to aeneas $AENEAS_REV"; exit 4; fi
done

# tools/aeneas/patches/charon-*.patch: fixes to Charon at its pin (charon-vtable-assoc: a vtable
# instance's trait reference gets its lifted associated types, so a closure coerced to `&dyn Fn`
# typechecks). Applied the same way.
for p in "$(cd "$(dirname "$0")" && pwd)"/patches/charon-*.patch; do
  [ -e "$p" ] || continue
  if git -C "$H/charon" apply --check "$p" 2>/dev/null; then git -C "$H/charon" apply "$p"
  elif git -C "$H/charon" apply -R --check "$p" 2>/dev/null; then :
  else echo "cannot apply $p to charon $PIN"; exit 4; fi
done
CHARON_KEY="$PIN $(cat "$(cd "$(dirname "$0")" && pwd)"/patches/charon-*.patch 2>/dev/null | sha256sum | cut -c1-16)"

say "opam $OPAM_VERSION, OCaml $OCAML_SWITCH"
if [ ! -x "$H/bin/opam" ]; then
  curl -fsSL -o "$H/bin/opam" "https://github.com/ocaml/opam/releases/download/$OPAM_VERSION/opam-$OPAM_VERSION-x86_64-linux"
  chmod +x "$H/bin/opam"
fi
[ -d "$OPAMROOT" ] || opam init --bare --disable-sandboxing -n
opam switch list -s | grep -qx "$OCAML_SWITCH" || opam switch create "$OCAML_SWITCH" --no-switch
eval "$(opam env --switch=$OCAML_SWITCH --set-switch)"
# charon-ml's dune file builds docs; odoc is only needed for that, but dune asks for it.
opam install -y odoc "$H/charon/charon.opam" "$H/charon/name_matcher_parser.opam" --deps-only
opam install -y "$H/aeneas/src" --deps-only
opam install -y odoc

say "charon (Rust side; its own pinned nightly)"
TC=$(sed -n 's/^channel *= *"\(.*\)"/\1/p' "$H/charon/charon/rust-toolchain")
rustup toolchain install "$TC" --profile minimal -c rustc-dev,llvm-tools,rust-src
echo "$TC" > "$H/charon-toolchain"
if [ ! -x "$H/charon/bin/charon" ] || [ "$(cat "$H/charon/bin/.rev" 2>/dev/null)" != "$CHARON_KEY" ]; then
  (cd "$H/charon/charon" && RUSTUP_TOOLCHAIN=$TC CARGO_TARGET_DIR=$H/charon-target cargo build --release -q)
  mkdir -p "$H/charon/bin"
  cp -f "$H/charon-target/release/charon" "$H/charon-target/release/charon-driver" "$H/charon/bin/"
  echo "$CHARON_KEY" > "$H/charon/bin/.rev"
fi

say "charon-ml and aeneas (OCaml side)"
(cd "$H/charon" && make build-charon-ml)
(cd "$H/aeneas" && make build-bin)
mkdir -p "$H/aeneas/bin"
ln -sfn "$H/aeneas/src/_build/default/main.exe" "$H/aeneas/bin/aeneas"

say "elan and $LEAN_TOOLCHAIN"
if ! command -v elan >/dev/null; then
  curl -fsSL -o "$H/elan-init.sh" https://raw.githubusercontent.com/leanprover/elan/v4.1.2/elan-init.sh
  sh "$H/elan-init.sh" -y --default-toolchain none
fi
elan toolchain install "$LEAN_TOOLCHAIN"
want=$(cat "$H/aeneas/backends/lean/lean-toolchain")
[ "$want" = "$LEAN_TOOLCHAIN" ] || { echo "aeneas's Lean library wants $want, not $LEAN_TOOLCHAIN"; exit 3; }

say done
"$0" --check
