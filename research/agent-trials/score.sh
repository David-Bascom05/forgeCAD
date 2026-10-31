#!/bin/sh
# Counts a trial directory's calls: score.sh DIR
log="$1/calls.log"
calls=$(/bin/grep -c '^\$ forgecad' "$log")
failed=$(/bin/grep -c '^\[exit [1-9]' "$log")
bytes=$(wc -c < "$log")
echo "$1: $calls calls, $failed failed, $bytes bytes through the tool"
