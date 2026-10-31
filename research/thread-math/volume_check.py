#!/usr/bin/env python3
"""Two more turns of thread remove exactly two more turns of the form's groove (Pappus):
2 x 2 pi x integral of r over the groove's section, against the exact mass properties.
volume_check.py BIN DIR   (DIR for its generated part files)"""
import math, os, subprocess, sys

b, d = sys.argv[1:3]
os.environ["FORGE_BATCH"] = "cpu"


def moment(major, pitch, internal):
    depth = 5 / 16 * math.sqrt(3) * pitch
    rj, rn = major / 2, major / 2 - depth
    slope = 1 / math.sqrt(3)
    w0, k = (pitch * 3 / 4, -2 * slope) if internal else (pitch / 4, 2 * slope)
    h = rj - rn
    return w0 * (rj**2 - rn**2) / 2 + k * (h**3 / 3 + rn * h**2 / 2)


def volume(src, name):
    f = f"{d}/{name}.fc"
    open(f, "w").write(src)
    w = subprocess.run([b, "info", f, "seq"], capture_output=True, text=True).stdout.split()
    return float(w[w.index("volume") + 1]) if "volume" in w else None


worst = 0
for major, pitch in [(3, .5), (6, 1), (8, 1.25), (12, 1.75), (20, 2.5), (20, .5), (30, 1.5), (24, 3)]:
    for internal in (False, True):
        L = 20 * pitch + 10
        n0 = 8 * pitch
        def part(l):
            if internal:
                tap = major - 2 * 5 / 16 * math.sqrt(3) * pitch
                w = major + 10 * pitch
                return f"b: extrude {L} rect {-w} {-w} {2 * w} {2 * w}\nhole {tap:.12g} -xy at 0 0 {L} thru thread {major} {pitch}\nthread M{major}x{pitch} -xy at 0 0 {L} int {l}\n"
            return f"r: extrude {L} circle 0 0 {major / 2}\nthread M{major}x{pitch} -xy at 0 0 {L} ext {l}\n"
        a, c = volume(part(n0), "a"), volume(part(n0 + 2 * pitch), "b")
        want = 2 * 2 * math.pi * moment(major, pitch, internal)
        if a is None or c is None:
            print(major, pitch, internal, "failed")
            continue
        got = a - c
        err = abs(got - want) / want
        worst = max(worst, err)
        print(f"M{major}x{pitch} {'int' if internal else 'ext'}: removed {got:.9f} exact {want:.9f} rel {err:.2e}")
print(f"worst {worst:.2e}")
