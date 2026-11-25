#!/usr/bin/env bash
# The kernel's differential net: the Haas sweep (run.sh) on two refs with the same harness, and
# the diff (diff.py), worst first. For a kernel change's owner: every difference it shows is
# yours to show right (or fix) before the change goes in.
#
#   tools/kernel-stress/compare.sh BASE_REF NEW_REF [-j JOBS] [-o DIR] [-n] [-d] [ZIP_NAME ...]
#
#   -j JOBS  files at once (3)            -o DIR  where the sweeps and report go (~/.cache/kernel-stress)
#   -n       booleans through n-ary on all threads, as the app's builds (KS_NARY)
#   -d       the decision log's named sites (KS_DECISIONS)
#   ZIP_NAME only these zips (fast: a few minutes on a handful); none: all of them (hours)
#
# Each ref is checked out in a worktree of its own and built in its own target directory; the
# harness (verify/forge-stress, tools/kernel-stress) is this checkout's, put into both, so the two
# sweeps differ only in the kernel. A sweep is kept by (harness, ref, options, zips): a base swept
# once is not swept again. The report: DIR/diff-BASE-NEW.md; exit 1 when NEW has new wrong
# answers, new hangs or crashes, or answers that changed.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
[ $# -ge 2 ] || { sed -n '2,20p' "$0"; exit 2; }
base_ref="$1"; new_ref="$2"; shift 2
jobs=3; out="${HOME}/.cache/kernel-stress"; nary=""; dec=""
while getopts "j:o:nd" o; do
  case "$o" in
    j) jobs="$OPTARG" ;;
    o) out="$OPTARG" ;;
    n) nary=1 ;;
    d) dec=1 ;;
    *) exit 2 ;;
  esac
done
shift $((OPTIND - 1))
zips=("$@")
mkdir -p "$out"
cd "$root" || exit 1
# the harness's identity: its files' content
harness=$(cat verify/forge-stress/Cargo.toml verify/forge-stress/src/*.rs tools/kernel-stress/*.sh \
  tools/kernel-stress/*.py | sha1sum | cut -c1-10)
opts="n${nary:-0}d${dec:-0}-$(printf '%s\n' "${zips[@]:-all}" | sha1sum | cut -c1-8)"

sweep() { # ref -> its sweep's directory
  local ref="$1" sha wt res
  sha=$(git rev-parse --verify -q "$ref^{commit}") || { echo "compare: no such ref $ref" >&2; return 1; }
  res="$out/sweep-${sha:0:10}-h$harness-$opts"
  if [ -f "$res/.done" ]; then
    echo "compare: $ref (${sha:0:10}) swept before: $res" >&2
    echo "$res"; return 0
  fi
  wt="$out/wt-${sha:0:10}"
  [ -d "$wt" ] || git worktree add -f -q --detach "$wt" "$sha" || return 1
  # this checkout's harness in the ref's tree, a workspace member there if it is not
  rm -rf "$wt/verify/forge-stress" "$wt/tools/kernel-stress"
  mkdir -p "$wt/verify" "$wt/tools"
  cp -r verify/forge-stress "$wt/verify/"
  cp -r tools/kernel-stress "$wt/tools/"
  if ! grep -q '"verify/forge-stress"' "$wt/Cargo.toml"; then
    sed -i 's|^members = \[|members = [\n    "verify/forge-stress",|' "$wt/Cargo.toml"
  fi
  echo "compare: building forge-stress at $ref (${sha:0:10})" >&2
  (cd "$wt" && CARGO_TARGET_DIR="$wt/target" cargo build --release -q -p forge-stress ${dec:+--features decisions}) >&2 \
    || { echo "compare: forge-stress does not build at $ref" >&2; return 1; }
  rm -rf "$res"
  echo "compare: sweeping $ref into $res" >&2
  (cd "$wt" && KS_NO_BUILD=1 KS_NARY="$nary" KS_DECISIONS="$dec" tools/kernel-stress/run.sh -o "$res" -j "$jobs" "${zips[@]}") >&2 \
    || return 1
  # (run.sh writes its report into the worktree's research/: not ours)
  touch "$res/.done"
  echo "$res"
}

a=$(sweep "$base_ref") || exit 1
b=$(sweep "$new_ref") || exit 1
report="$out/diff-$(git rev-parse --short "$base_ref")-$(git rev-parse --short "$new_ref").md"
python3 -I "$here/diff.py" "$a" "$b" --md "$report" >/dev/null
rc=$?
echo "compare: $report" >&2
sed -n '1,12p' "$report"
grep '^## ' "$report"
exit $rc
