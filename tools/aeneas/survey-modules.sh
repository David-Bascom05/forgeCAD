#!/bin/bash
# Charon and Aeneas over the kernel, module by module, as the code is, to see what translates.
#
#   tools/aeneas/survey-modules.sh [CRATE..]     default: every kernel/* crate
#
# Slow (each module rebuilds its crate under Charon): tools/aeneas/survey.sh does a crate at once
# and attributes by source span; use this one for a crate whose whole-crate run fails.
#
# Each module (one .rs file) is translated on its own, `--start-from crate::path::to::module`, built
# as release, so one item Aeneas cannot take does not hide the rest. Under $OUT (default
# $FORGE_AENEAS/survey; nothing here is committed) it writes per module:
#   CRATE/MOD.charon.log, CRATE/MOD.aeneas.log    the tools' output
# and appends to $OUT/survey.tsv one row per module:
#   crate  module  file  lines  functions  failed  fatal  first-errors  lean
# where `functions` is what Aeneas attempted (the module's items and what they reach), `failed`
# the bodies it could not translate, `fatal` the error that stopped it (or "-"), and
# `first-errors` the distinct error messages, most frequent first, and `lean` whether the output
# elaborates (tools/aeneas/leancheck.sh: the templates' axioms stand in for opaque functions). research/aeneas/SURVEY.md is
# written from survey.tsv.
#
# Optional, from the environment:
#   SURVEY_CHARON_ARGS="--opaque a::b --opaque c::d"   more Charon arguments for every module (e.g. the
#                                                      opaque markers of a translate.conf)
#   SURVEY_MODULES="forge_bool::ssi forge_bool::unify"  only these modules
#   SURVEY_KEEP=1                                        keep each module's Lean under CRATE/MOD.out/
#   SURVEY_MEM_KB=8000000                                each tool's address-space cap (default 8 GB)
set -uo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
H=${FORGE_AENEAS:-$HOME/.cache/forge-aeneas}
TC=$(cat "$H/charon-toolchain")
OUT=${OUT:-$H/survey}
mkdir -p "$OUT"
crates=("$@")
[ ${#crates[@]} -gt 0 ] || for d in "$root"/kernel/*/; do crates+=("$(basename "$d")"); done

modules() { # crate dir -> "rust::path<TAB>file" per module file
  local src=$1/src name=$2
  find "$src" -name '*.rs' | sort | while read -r f; do
    rel=${f#$src/}; rel=${rel%.rs}
    case "$rel" in lib|main) path=$name ;; */mod) path=$name::${rel%/mod} ;; *) path=$name::$rel ;; esac
    case "$rel" in bin/*|*/bin/*) continue ;; esac
    printf '%s\t%s\n' "${path//\//::}" "${f#$root/}"
  done
}

for c in "${crates[@]}"; do
  name=${c//-/_}
  mkdir -p "$OUT/$c"
  modules "$root/kernel/$c" "$name" | while IFS=$'\t' read -r mod file; do
    tag=${mod//::/.}
    work=$(mktemp -d)
    if [ -n "${SURVEY_MODULES:-}" ] && ! [[ " $SURVEY_MODULES " == *" $mod "* ]]; then rm -rf "$work"; continue; fi
    # shellcheck disable=SC2206
    extra=(${SURVEY_CHARON_ARGS:-})
    start=(--start-from "$mod")
    [ "$mod" = "$name" ] && start=(--start-from "$name" --exclude "$name::*::*")
    # 8 GB per tool: one module once took charon to 13 GB (the OOM killer then takes the culprit,
    # but other work in the same memory group can go first)
    ( ulimit -v "${SURVEY_MEM_KB:-8000000}"; cd "$root/kernel/$c" && PATH=$H/charon/bin:$PATH RUSTUP_TOOLCHAIN=$TC \
        CARGO_TARGET_DIR=$H/target-survey/$c CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true timeout 1800 "$H/charon/bin/charon" cargo --preset=aeneas \
        "${start[@]}" "${extra[@]}" --dest-file "$work/x.llbc" -- --release ) > "$OUT/$c/$tag.charon.log" 2>&1
    fatal=-
    total=0; failed=0
    if [ -f "$work/x.llbc" ]; then
      ( ulimit -v "${SURVEY_MEM_KB:-8000000}"; timeout 1800 "$H/aeneas/bin/aeneas" -backend lean -split-files "$work/x.llbc" -dest "$work/out" ) \
        > "$OUT/$c/$tag.aeneas.log" 2>&1
      sed -i -e 's/\x1b\[[0-9;?]*[a-zA-Z]//g' -e 's/\r/\n/g' "$OUT/$c/$tag.aeneas.log"
      log=$OUT/$c/$tag.aeneas.log
      total=$(grep -o 'Translated transparent functions: *\[[^]]*\] *[0-9]*/[0-9]*' "$log" | tail -1 | grep -o '[0-9]*$')
      failed=$(grep -c 'Could not translate the body of function' "$log")
      if grep -q '^Fatal error' "$log"; then
        fatal=$(grep -B30 '^Fatal error' "$log" | grep '^\[Error\]' | tail -1 | sed 's/^\[Error\] *//')
        [ -n "$fatal" ] || fatal=$(grep '^Fatal error' "$log" | head -1)
      fi
      errs=$(grep '^\[Error\]' "$log" | sed 's/^\[Error\] *//' | cut -c1-90 | sort | uniq -c | sort -rn \
        | head -4 | awk '{n=$1; $1=""; printf "%s(%dx); ", substr($0,2), n}')
      if [ -f "$work/out/Funs.lean" ]; then
        lean=$("$root/tools/aeneas/leancheck.sh" "$work/out" 2>&1 | tail -1)
      else
        lean="lean: no output"
      fi
    else
      lean="-"
      if grep -q 'memory allocation of' "$OUT/$c/$tag.charon.log"; then
        fatal="charon: out of memory (over the 8 GB cap)"
      else
        fatal="charon: $(grep -m1 -E '^error|ERROR|panicked' "$OUT/$c/$tag.charon.log" | cut -c1-120)"
      fi
      errs=
    fi
    lines=$(wc -l < "$root/$file")
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$c" "$mod" "$file" "$lines" "${total:-0}" "$failed" "$fatal" "$errs" "$lean" \
      >> "$OUT/survey.tsv"
    printf '%-14s %-48s fns=%-4s failed=%-4s fatal=%s %s\n' "$c" "$mod" "${total:-0}" "$failed" "${fatal:0:50}" "${lean:0:60}"
    if [ -n "${SURVEY_KEEP:-}" ] && [ -d "$work/out" ]; then rm -rf "$OUT/$c/$tag.out"; mv "$work/out" "$OUT/$c/$tag.out"; fi
    rm -rf "$work"
  done
done
