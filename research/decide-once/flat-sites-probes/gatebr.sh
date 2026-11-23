#!/bin/sh
# gatebr.sh NAME REV : the callgrind gate's counts of REV (archived into flat/snap-NAME), saved as
# flat/NAME.counts and flat/NAME.bits; the build shares flat/gate-target with the other snapshots.
F=/tmp/claude-1000/-home-david-bascom/8633b6f7-d8ad-4ded-bb95-f20c08fc75b7/scratchpad/flat
W=/home/david-bascom/Documents/Github/forgeCAD/.claude/worktrees/agent-ab9f75fb2464c6365
rm -rf "$F/snap-$1"; mkdir -p "$F/snap-$1"
git -C "$W" archive "$2" | tar -x -C "$F/snap-$1"
cd "$F/snap-$1" || exit 9
CARGO_TARGET_DIR=$F/gate-target GATE_COUNTER=callgrind GATE_CPU=6 \
  /tmp/claude-1000/forge-heavy.sh sh $F/gate-flat.sh --out "$F/$1" > "$F/$1.log" 2>&1
echo "exit $?" >> "$F/$1.log"
