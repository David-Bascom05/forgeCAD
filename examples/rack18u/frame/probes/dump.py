#!/usr/bin/env python3
"""Anchor dump: places a 0.2 cube on each named anchor of a part and prints its pose (origin, z, x).
usage: dump.py PART.fg ANCHOR [ANCHOR ..]   (paths relative to frame/)"""
import json, subprocess, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
FG = str(Path.home() / "projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
part, names = sys.argv[1], sys.argv[2:]
lines = ["assembly", f'p = part("../{part}", ground: true)']
for i, n in enumerate(names):
    lines += [f'c{i} = part("cube.fg")', f"j{i} = fastened(c{i}, p.{n})"]
f = HERE / ".dump.fg"
f.write_text("\n".join(lines) + "\n")
d = json.loads(subprocess.run([FG, "run", str(f), "--json"], capture_output=True, text=True).stdout)
for e in d["errors"]:
    print("E", e.get("kind"), e.get("text"))
poses = {i["id"]: i["pose"] for i in d.get("assembly", {}).get("instances", [])}
r = lambda v: tuple(round(x, 4) + 0.0 for x in v)
for i, n in enumerate(names):
    p = poses.get(f"c{i}")
    if p:
        print(f"{n:14s} o={r(p['origin'])} z={r(p['z'])} x={r(p['x'])}")
