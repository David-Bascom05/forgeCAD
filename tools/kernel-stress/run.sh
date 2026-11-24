#!/usr/bin/env bash
# The kernel under the Haas machine models: every STEP in every zip through verify/forge-stress
# (import, check, mass, mesh, sections, booleans, each op watched), as JSON lines per file, then
# research/kernel-stress/haas-report.md from them (report.py).
#
#   tools/kernel-stress/run.sh [-o OUT] [-j JOBS] [-t OP_TIMEOUT_S] [-f FILE_TIMEOUT_S]
#                              [-n MAX_SOLIDS] [-z ZIPDIR] [ZIP_NAME ...]
#
# The zips come from the branch haas-3d-models (fetched, never checked out: each one is read with
# `git show` into a scratch dir, unpacked, run, and deleted), or from -z DIR. No names: all of them.
# A hung op ends its run with code 3 and the hung op's key; the file is run again with that op
# skipped, so a hang costs one op, not the file. Nothing here writes into the repository but the
# report; the raw models are never committed.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
out="$root/target/kernel-stress"
jobs=2
op_t=60
file_t=3600
max_solids=24
zipdir=""
while getopts "o:j:t:f:n:z:" o; do
  case $o in
    o) out="$OPTARG" ;;
    j) jobs="$OPTARG" ;;
    t) op_t="$OPTARG" ;;
    f) file_t="$OPTARG" ;;
    n) max_solids="$OPTARG" ;;
    z) zipdir="$OPTARG" ;;
    *) exit 2 ;;
  esac
done
shift $((OPTIND - 1))

bin="$root/target/release/forge-stress"
if [ -z "${KS_NO_BUILD:-}" ]; then
  (cd "$root" && cargo build --release -q -p forge-stress) || exit 1
fi
mkdir -p "$out"
scratch="${TMPDIR:-/tmp}/kernel-stress.$$"
mkdir -p "$scratch"
trap 'rm -rf "$scratch"' EXIT

if [ -z "$zipdir" ]; then
  git -C "$root" rev-parse -q --verify origin/haas-3d-models >/dev/null ||
    git -C "$root" fetch -q origin haas-3d-models || exit 1
fi

list() {
  if [ -n "$zipdir" ]; then
    (cd "$zipdir" && ls -1 *.zip)
  else
    git -C "$root" ls-tree --name-only origin/haas-3d-models research/haas-models/ |
      grep '\.zip$' | sed 's|.*/||'
  fi
}

# one zip: unpack, run each STEP, delete
one() {
  local zip="$1" name="${1%.zip}" dir="$scratch/${1%.zip}"
  local res="$out/$name.jsonl"
  [ -s "$res" ] && grep -q '"ev":"zip_done"' "$res" && return 0
  : >"$res"
  mkdir -p "$dir"
  if [ -n "$zipdir" ]; then
    cp "$zipdir/$zip" "$dir/z.zip"
  else
    git -C "$root" show "origin/haas-3d-models:research/haas-models/$zip" >"$dir/z.zip"
  fi
  # what the zip holds: STEP and Parasolid (x_t / x_b), by size
  unzip -Z1 "$dir/z.zip" 2>/dev/null | while read -r m; do
    case "${m,,}" in
      __macosx/*) ;;
      *.x_t | *.x_b | *.xmt_txt | *.xmt_bin)
        sz=$(unzip -Zl "$dir/z.zip" "$m" 2>/dev/null | awk 'NR==1{print $4}')
        printf '{"ev":"parasolid","zip":"%s","file":"%s","bytes":%s}\n' "$zip" "${m//\"/}" "${sz:-0}" >>"$res" ;;
    esac
  done
  (cd "$dir" && unzip -qq -o z.zip -x '*.x_t' '*.X_T' '*.x_b' '*.X_B' '*.pdf' '*.PDF' '__MACOSX/*' 2>/dev/null)
  rm -f "$dir/z.zip"
  # zips inside the zip: one level
  find "$dir" -iname '*.zip' -print0 | while IFS= read -r -d '' z; do
    (cd "$(dirname "$z")" && unzip -qq -o "$z" -x '*.x_t' '*.X_T' '*.x_b' '*.X_B' 2>/dev/null; rm -f "$z")
  done
  find "$dir" -type f \( -iname '*.step' -o -iname '*.stp' \) -print0 | sort -z |
    while IFS= read -r -d '' f; do
      local rel="${f#"$dir"/}" skip="" t0 rc bytes
      bytes=$(stat -c %s "$f")
      printf '{"ev":"file","zip":"%s","file":"%s","bytes":%s}\n' "$zip" "${rel//\"/}" "$bytes" >>"$res"
      t0=$(date +%s.%N)
      while :; do
        timeout -k 10 "$file_t" "$bin" "$f" --timeout "$op_t" --max-solids "$max_solids" --skip "$skip" >"$dir/run.jsonl" 2>"$dir/err.txt"
        rc=$?
        if [ -n "$skip" ]; then
          # a rerun: what came before the skipped ops is in already
          grep -v '"ev":"\(import\|solid\)"' "$dir/run.jsonl" >>"$res"
        else
          cat "$dir/run.jsonl" >>"$res"
        fi
        # a hang (3), a crash (a signal, an abort: a stack overflow) or the file's own timeout:
        # the op in flight is the last one begun; skip it and go again
        if [ $rc -ne 0 ]; then
          k=$(grep '^BEGIN ' "$dir/err.txt" | tail -1 | cut -d' ' -f2)
          if [ $rc -ne 3 ]; then
            printf '{"ev":"crash","rc":%d,"key":"%s","d":"%s"}\n' "$rc" "$k" \
              "$(grep -v '^BEGIN ' "$dir/err.txt" | tail -c 300 | tr -d '"\\\n\r\t')" >>"$res"
          fi
          if [ -z "$k" ] || [ "$k" = import ] || [[ ",$skip," == *",$k,"* ]]; then break; fi
          skip="$skip${skip:+,}$k"
          continue
        fi
        break
      done
      printf '{"ev":"file_done","wall_s":%.2f}\n' "$(awk -v a="$t0" -v b="$(date +%s.%N)" 'BEGIN{print b-a}')" >>"$res"
    done
  rm -rf "$dir"
  echo '{"ev":"zip_done"}' >>"$res"
  echo "done $zip" >&2
}
export -f one
export scratch out bin op_t file_t max_solids zipdir root

if [ $# -gt 0 ]; then printf '%s\n' "$@"; else list; fi |
  xargs -P "$jobs" -I{} bash -c 'one "$@"' _ {}

python3 -I "$here/report.py" "$out" >"$root/research/kernel-stress/haas-report.md" &&
  echo "report: research/kernel-stress/haas-report.md" >&2
