#!/bin/bash
set -x
S=/tmp/claude-0/-home-user-forgeCAD/9792d215-acc8-574e-8e37-10903f387b80/scratchpad/aeneas
export OPAMROOT=$S/opamroot OPAMYES=1 OPAMCOLOR=never OPAMSWITCH=5.3.0
export PATH=$S/bin:$HOME/.cargo/bin:$PATH
eval $(opam env --switch=5.3.0 --set-switch)
C=/home/user/aeneasverif/charon
A=/home/user/aeneasverif/aeneas
opam install $C/charon.opam $C/name_matcher_parser.opam --deps-only -y 2>&1 | tail -15 || exit 21
opam install $A/src --deps-only -y 2>&1 | tail -15 || exit 22
echo OCAML-DEPS-DONE
export RUSTUP_TOOLCHAIN=nightly-2026-09-17
export CARGO_TARGET_DIR=$S/charon-target
(cd $C/charon && cargo build --release 2>&1 | tail -15) || exit 23
mkdir -p $C/bin
cp -f $CARGO_TARGET_DIR/release/charon $CARGO_TARGET_DIR/release/charon-driver $C/bin/ || exit 24
echo CHARON-RUST-DONE
(cd $C && make build-charon-ml 2>&1 | tail -15) || exit 25
echo CHARON-ML-DONE
[ -e $A/charon ] || ln -s $C $A/charon
(cd $A && make build-bin build-lib 2>&1 | tail -30) || exit 26
echo AENEAS-DONE
