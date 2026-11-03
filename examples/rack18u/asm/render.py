#!/usr/bin/env python3
"""Renders of the top assembly: front, iso, and iso with both doors at 90deg (latches open). Each runs a
scratch copy of rack.fg with the expects dropped (the checks are run.py's), capped like run.py.
Run: python3 render.py [px]"""
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FG = os.path.expanduser("~/projects/ForgeCAD/.claude/worktrees/forge-lang/target/release/fg")
CAP = ["systemd-run", "--user", "--scope", "-q", "-p", "MemoryMax=6G", "-p", "MemorySwapMax=0", "nice"]
VIEWS = [("rack_front", "front", {}), ("rack_iso", "iso", {}),
         ("rack_doors_90", "iso", {"f_door_f.turn": "90deg", "r_door_f.turn": "90deg"})]


def main():
    px = sys.argv[1] if len(sys.argv) > 1 else "1600"
    os.makedirs(os.path.join(HERE, "renders"), exist_ok=True)
    src = [l for l in open(os.path.join(HERE, "rack.fg")) if not l.startswith("expect ")]
    for name, view, params in VIEWS:
        text = "".join(src)
        for k, v in params.items():
            text, n = re.subn(r"^param %s = .*$" % re.escape(k), "param %s = %s" % (k, v), text, flags=re.M)
            assert n == 1, k
        fd, path = tempfile.mkstemp(suffix=".fg", dir=HERE, prefix=".render_")
        os.write(fd, text.encode())
        os.close(fd)
        out = os.path.join(HERE, "renders", name + ".png")
        p = subprocess.run(CAP + [FG, "run", path, "--render", out, "--view", view, "--px", px],
                           capture_output=True, text=True)
        os.replace(path, os.path.join(HERE, ".render_last.fg"))
        print(name, "ok" if p.returncode == 0 else "FAILED " + (p.stdout + p.stderr)[-500:])


if __name__ == "__main__":
    main()
