#!/bin/sh
# Charon + Aeneas on kernel/forge-bool-core, into lean/ForgeBoolCore/{Types,Funs}.lean.
# Toolchains: research/kernel-audit/coverage/aeneas/{setup,build}.sh (aeneas 2e3d4488, charon
# 9e193f0; the OCaml side also needs libgmp-dev and `opam install odoc`). Then `lake build` in lean/.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root=$here/../../../..
A=${AENEAS:-/home/user/aeneasverif}
work=$(mktemp -d)
cp -r "$root/kernel/forge-bool-core/src" "$work/"
cat > "$work/Cargo.toml" <<'T'
[package]
name = "forge_bool_core"
version = "0.1.0"
edition = "2024"
T
cd "$work"
RUSTUP_TOOLCHAIN=nightly-2026-09-17 PATH=$A/charon/bin:$PATH charon cargo --preset=aeneas
mkdir lean
"$A/aeneas/src/_build/default/main.exe" -backend lean forge_bool_core.llbc -dest lean -split-files
cp lean/Types.lean lean/Funs.lean "$here/lean/ForgeBoolCore/"
echo "translated into $here/lean/ForgeBoolCore (FunsExternal.lean is kept: compare with lean/FunsExternal_Template.lean in $work)"
