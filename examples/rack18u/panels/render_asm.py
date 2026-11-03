#!/usr/bin/env python3
"""Renders test.fg posed: door closed with the latch closed, door at 50deg, door at 105deg. Writes the
variants (no expects, drives set) to out/ and renders to renders/."""
import os, re, subprocess
HERE = os.path.dirname(os.path.abspath(__file__))
FG = os.path.expanduser("~/projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
src = open(os.path.join(HERE, "test.fg")).read()
src = "\n".join(l for l in src.splitlines() if not l.startswith("expect "))
src = src.replace('part("../hw/', 'part("../../hw/')            # paths relative to out/
src = re.sub(r'part\("(?!\.\./)', 'part("../', src)
src = src.replace('fit_rule("../hw/', 'fit_rule("../../hw/').replace('"door.fg")', '"../door.fg")')
for name, door, latch, lift, view in [("test-closed-front", 0, 90, -0.4, "front"), ("test-closed-iso", 0, 90, -0.4, "iso"),
                                      ("test-door50", 50, 0, 0, "iso"), ("test-door105-top", 105, 0, 0, "top")]:
    t = src.replace("param door_f.turn = 0deg", f"param door_f.turn = {door}deg")
    t = t.replace("param latch_f.turn = 0deg", f"param latch_f.turn = {latch}deg")
    t = t.replace("shared latch_lift = 0\n", f"shared latch_lift = {lift}\n")
    p = os.path.join(HERE, "out", name + ".fg")
    open(p, "w").write(t)
    r = subprocess.run(["fbuild", FG, "run", p, "--render", os.path.join(HERE, "renders", name + ".png"),
                        "--view", view, "--px", "1400"], capture_output=True, text=True)
    print(name, r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else r.stderr[-300:])
