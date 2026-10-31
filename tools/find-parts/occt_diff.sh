#!/bin/sh
# Differential: OpenCascade replays every Nth part of DIR. Anything one kernel builds and the other refuses,
# and every disagreement in volume, area or surfaces, is listed by occt_tally.py.
#   fbuild sh tools/find-parts/occt_diff.sh N DIR OUT     (needs tools/occt/setup.sh and the oracle example)
N=$1; D=$2; O=$3
HERE=$(dirname "$0")
ROOT=$HERE/../..
mkdir -p "$O/in"
ls "$D"/*.fc | awk "NR%$N==0" | while read -r f; do ln -sf "$(realpath "$f")" "$O/in/"; done
nice -n 15 env THREADS=1 REPS=1 CHORD=0.01 "${CARGO_TARGET_DIR:-$ROOT/target}/release/examples/oracle" "dir:$O/in" > "$O/parts.json" 2> "$O/export.err"
nice -n 15 "${VENV:-$HOME/.local/share/forgecad-occt/venv}/bin/python" "$ROOT/tools/occt/oracle.py" "$O/parts.json" --jobs 3 --timeout 60 --show 0 --out "$O/res.json" > "$O/report.txt" 2>&1
python3 "$HERE/occt_tally.py" "$O/res.json"
