#!/bin/sh
# Pipelined trains: train N+1 was frozen and verified on top of train N's head. Once N lands on
# main (squashed), N+1 becomes one new commit on main with N+1's VERIFIED tree, never a merge of a
# merge-train/* branch, and never re-verified, because the tree it carries is the one verified:
#
#   tools/preverify/reparent.sh N_HEAD N1_HEAD [MAIN]      (MAIN defaults to origin/main)
#
# Refuses unless main's tree is train N's head's tree (N landed exactly as verified), and unless
# N1_HEAD descends from N_HEAD. Prints the new commit (on no branch): push it as the train's PR
# branch and say in the PR "reparented: tree <id>, verified as <N1_HEAD>".
set -e
n=$(git rev-parse "$1^{commit}"); n1=$(git rev-parse "$2^{commit}"); main=$(git rev-parse "${3:-origin/main}^{commit}")
if [ "$(git rev-parse "$main^{tree}")" != "$(git rev-parse "$n^{tree}")" ]; then
  echo "reparent: main $main's tree is not train N's ($n): N did not land as verified; rebuild N+1 on main and verify it" >&2
  exit 1
fi
if ! git merge-base --is-ancestor "$n" "$n1"; then
  echo "reparent: $n1 does not descend from train N's head $n" >&2
  exit 1
fi
tree=$(git rev-parse "$n1^{tree}")
msg=$(git log -1 --format=%B "$n1")
new=$(printf '%s\n\nReparented onto main %s: tree %s, verified as %s.\n' "$msg" "$main" "$tree" "$n1" |
  git commit-tree "$tree" -p "$main")
echo "$new"
