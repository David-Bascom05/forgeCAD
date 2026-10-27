#!/usr/bin/env python3
"""Shrinks a part on which ForgeCAD and OCCT disagree to the fewest features that still disagree.


<set> is what the oracle example takes (fuzz:1, suite, adv:8, ...). Feature 0 is always kept.
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import oracle  # noqa: E402


def evaluate(a, keep):
    env = dict(os.environ, ONLY=a.part, KEEP=",".join(map(str, keep)), REPS="1", CHORD=str(a.chord))
    target = os.environ.get("CARGO_TARGET_DIR", os.path.join(ROOT, "target"))
    out = subprocess.run([os.path.join(target, "release/examples/oracle"), a.set], env=env,
                         capture_output=True, text=True, check=True).stdout
    part = json.loads(out)["parts"][0]
    ref = oracle.occt(part, 1, False, 0.0, 120)
    v = oracle.compare(part["forge"][a.path], ref)[0]
    return v, part, ref


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("set")
    ap.add_argument("part")
    ap.add_argument("--path", default="seq")
    ap.add_argument("--chord", type=float, default=0.002)
    a = ap.parse_args()
    v, part, _ = evaluate(a, range(1000))
    keep = list(range(len(part["features"])))
    print(f"{len(keep)} features: {v}")
    if v in oracle.GOOD or v == "both-fail":
        return
    target = v
    n = 2
    while len(keep) > 1:
        rest = keep[1:]
        size = max(1, len(rest) // n)
        removed = False
        for i in range(0, len(rest), size):
            trial = [keep[0]] + rest[:i] + rest[i + size:]
            tv, _, _ = evaluate(a, trial)
            if tv == target:
                keep, removed = trial, True
                n = max(n - 1, 2)
                print(f"  {len(keep)} features still {tv}")
                break
        if not removed:
            if size == 1:
                break
            n = min(n * 2, len(rest))
    v, part, ref = evaluate(a, keep)
    f = part["forge"][a.path]
    print(f"minimal: KEEP={','.join(map(str, keep))} ({v})")
    print(f"  occt volume {ref.get('volume')} area {ref.get('area')} shells {ref.get('shells')}")
    print(f"  ours volume {f.get('volume_mesh')} area {f.get('area_mesh')} shells {f.get('shells')} {f['status']}")
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as t:
        json.dump(part, t, indent=1)
    print(f"  part json: {t.name}")


if __name__ == "__main__":
    main()
