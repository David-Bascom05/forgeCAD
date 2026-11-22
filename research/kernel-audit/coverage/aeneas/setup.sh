#!/bin/bash
# Aeneas toolchain trial: opam + OCaml 5.3 switch + aeneas deps; Rust nightly for charon; build both.
set -x
S=/tmp/claude-0/-home-user-forgeCAD/9792d215-acc8-574e-8e37-10903f387b80/scratchpad/aeneas
mkdir -p $S/bin
export OPAMROOT=$S/opamroot OPAMYES=1 OPAMCOLOR=never
if [ ! -x $S/bin/opam ]; then
  curl -fsSL -o $S/bin/opam "https://github.com/ocaml/opam/releases/download/2.3.0/opam-2.3.0-x86_64-linux" || exit 10
  chmod +x $S/bin/opam
fi
export PATH=$S/bin:$HOME/.cargo/bin:$PATH
opam --version || exit 11
[ -d $OPAMROOT ] || opam init --bare --disable-sandboxing -n || exit 12
opam switch list | grep -q 5.3.0 || opam switch create 5.3.0 --no-switch -y || exit 13
eval $(opam env --switch=5.3.0)
cd /home/user/aeneasverif/aeneas
opam install ./src --deps-only -y 2>&1 | tail -30 || exit 14
# also charon-ml's deps
opam install /home/user/aeneasverif/charon/charon.opam /home/user/aeneasverif/charon/name_matcher_parser.opam --deps-only -y 2>&1 | tail -30
echo OCAML-DONE
TC=nightly-2026-09-17
rustup toolchain install $TC --profile minimal -c rustc-dev,llvm-tools,rust-src || exit 15
echo RUST-DONE
