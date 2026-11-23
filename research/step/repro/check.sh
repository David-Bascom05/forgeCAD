#!/bin/sh
# Each repro against OCCT (under the truth rule) and our reader: class and cause as scale.py judges them.
#   research/step/repro/check.sh [STEPREAD]
cd "$(dirname "$0")"
python3 -I mk.py . >/dev/null
BIN=${1:-../../../target/release/examples/stepread} python3 -I - <<'PY'
import glob, json, os, subprocess, sys
sys.path.insert(0, "..")
import scale
for f in sorted(glob.glob("*.step")):
    f = os.path.abspath(f)
    o = scale.child([scale.VENV, "-I", os.path.join(scale.HERE, "occt_truth.py"), f], 120, 5 << 30)
    d = scale.child([os.environ["BIN"]], 60, 3 << 30, stdin=open(f, "rb").read())
    cls, cause = scale.judge(o, d)
    v = [s.get("vol") for s in d.get("solids", [])]
    print(f"{os.path.basename(f):28s} {cls:10s} {cause:60s} ours {v} occt gk {o.get('all', {}).get('vol_gk')}")
PY
