#!/usr/bin/env python3
"""Copies hw/shop.fg's shop text (shared t3 .. the shop() call) into every frame part, replacing the old
copy (SPEC 10 gap 12), and reruns gen_pans.py. --check only reports parts whose copy differs."""
import subprocess, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
START, END = "shared t3", "kerf: kerf_scs, hardware: hw_scs)"

def block(t):
    a = t.index(START)
    return a, t.index(END, a) + len(END)

src = (HERE.parent / "hw" / "shop.fg").read_text()
a, b = block(src)
shop = src[a:b]
bad = 0
for f in ("post.fg", "carrier.fg", "rail.fg"):
    p = HERE / f
    t = p.read_text()
    x, y = block(t)
    if t[x:y] != shop:
        bad += 1
        print(f"{f}: shop differs" + ("" if "--check" in sys.argv else ", replaced"))
        if "--check" not in sys.argv:
            p.write_text(t[:x] + shop + t[y:])
if "--check" not in sys.argv:
    subprocess.run([sys.executable, str(HERE / "gen_pans.py")], check=True)
sys.exit(1 if bad and "--check" in sys.argv else 0)
