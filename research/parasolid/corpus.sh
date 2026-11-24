#!/bin/bash
# Runs a command over every Parasolid file of the Haas corpus (branch haas-3d-models), one archive
# at a time: each is extracted to a scratch directory and deleted after, so the disk holds one.
#   research/parasolid/corpus.sh [-n PATTERN] CMD...
# CMD is run as `CMD XT_FILE STEP_FILE` (STEP_FILE empty when the archive has none). Output goes to
# stdout. The branch is fetched first if missing; it is never checked out.
set -u
pat=.
if [ "${1:-}" = -n ]; then pat=$2; shift 2; fi
repo=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
cd "$repo"
git rev-parse -q --verify origin/haas-3d-models >/dev/null || git fetch -q origin haas-3d-models:refs/remotes/origin/haas-3d-models
scratch=${TMPDIR:-/tmp}/xt-corpus.$$
trap 'rm -rf "$scratch"' EXIT
for z in $(git ls-tree --name-only origin/haas-3d-models research/haas-models/ | grep '\.zip$' | grep -E "$pat"); do
  w=$scratch/$(basename "$z" .zip); mkdir -p "$w"
  git show "origin/haas-3d-models:$z" > "$w/a.zip"
  (cd "$w" && unzip -qq -o a.zip -x '__MACOSX/*' '*.pdf' '*.PDF' '*.STL' '*.stl' '*.dxf' '*.DXF' 2>/dev/null)
  rm -f "$w/a.zip"
  find "$w" -type f \( -iname '*.x_t' -o -iname '*.x_b' -o -iname '*.xmt_txt' -o -iname '*.xmt_bin' \) -print0 |
    while IFS= read -r -d '' x; do
      d=$(dirname "$x")
      s=$(find "$d" -maxdepth 1 -type f \( -iname '*.step' -o -iname '*.stp' \) | head -1)
      [ -z "$s" ] && s=$(find "$w" -type f \( -iname '*.step' -o -iname '*.stp' \) | head -1)
      "$@" "$x" "$s"
    done
  rm -rf "$w"
done
