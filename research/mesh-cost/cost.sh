#!/bin/sh
# usage: cost.sh <bin> <out> names...   per-mesh instructions:u at chord 0.01 and 0.25 (times 2 minus times 0, /2)
bin=$1; out=$2; shift 2
: > "$out"
cnt() { perf stat -x, -e instructions:u "$bin" "$1" "$2" "$3" 2>&1 >/dev/null | awk -F, '/instructions/{print $1}'; }
for n in "$@"; do
  for c in 0.01 0.25; do
    a=$(cnt $n $c 0); b=$(cnt $n $c 2)
    case "$a$b" in *[!0-9]*|"") echo "$n $c FAIL" >> "$out";; *) echo "$n $c $(( (b - a) / 2 ))" >> "$out";; esac
  done
done
