#!/usr/bin/env python3
# Each threshold's frames without lines against the finest one (the shapes, not whether lines are
# drawn): RMS difference and share of pixels off by more than 16 levels of 255; what the frame drew
# with lines on; and the bench's times.
import re, sys, os, array


def ppm(path):
    d = open(path, "rb").read()
    head = d.split(b"\n", 3)
    return array.array("B", head[3])


out = sys.argv[1]
ts = ["0.125", "0.25", "0.5", "1", "2", "4", "8"]
print("view threshold  rms  off>16  tris      segments")
for view in ["far", "mid"]:
    base = ppm(f"{out}/{view}-surface-{ts[0]}.ppm")
    for t in ts:
        p = ppm(f"{out}/{view}-surface-{t}.ppm")
        sq = off = 0
        for i in range(0, len(p), 3):
            d = max(abs(p[i + k] - base[i + k]) for k in range(3))
            sq += d * d
            off += d > 16
        n = len(p) // 3
        txt = open(f"{out}/{view}-{t}.txt").read()
        tris = re.search(r"tris: (\d+)", txt).group(1)
        seg = re.search(r"segments: (\d+)", txt).group(1)
        print(f"{view} {t:>6} {(sq / n) ** 0.5:6.2f} {100 * off / n:6.2f}% {tris:>9} {seg:>9}")
print()
for t in ts:
    txt = open(f"{out}/bench-{t}.txt").read()
    m = re.search(r"frame mean ([\d.]+) p99 ([\d.]+) worst ([\d.]+)", txt)
    print(f"bench {t:>6}: mean {m.group(1)} p99 {m.group(2)} worst {m.group(3)} ms")
