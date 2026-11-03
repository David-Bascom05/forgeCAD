#!/usr/bin/env python3
"""SPEC 5.8's thin clearance 'base to vent slot 0.75', in the door plane (a body clearance cannot see
it: the base sits on the field). Latch base footprint vs every vent slot left in the door, both in
the door frame (h, k). Also the padlock tab slot vs the vent slots."""
import math
def seg_rect(p, q, r):
    # min distance from segment pq to axis-aligned rectangle r = (x0, x1, y0, y1), by dense sampling
    best = 1e9
    for i in range(2001):
        t = i / 2000
        x, y = p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])
        dx = max(r[0] - x, 0, x - r[1]); dy = max(r[2] - y, 0, y - r[3])
        best = min(best, math.hypot(dx, dy))
    return best
base = (233.0 + 290.98, 286.4 + 290.98, 426 - 39, 485 - 39)
tab = (523.58, 523.58 + 54.2, 439.5, 446.5)
out = []
for i in range(40):
    for j in range(4):
        if j == 3 and 17 <= i <= 22:
            continue
        c, k = 113.23 + 120 * j, 414 + (i - 19.5) * 16
        p, q = (c - 47, k), (c + 47, k)
        out.append((seg_rect(p, q, base) - 3, seg_rect(p, q, tab) - 3, i, j))
b = min(out); t = min(out, key=lambda o: o[1])
print(f"latch base to nearest vent slot: {b[0]:.3f} (row {b[2]}, column {b[3]})")
print(f"padlock tab slot to nearest vent slot: {t[1]:.3f} (row {t[2]}, column {t[3]})")
# the slots that were left out (column 3, rows 17..22) would have been 0.75 from the base in h
c = 113.23 + 360
print(f"a column-3 slot end (h {c + 50:.2f}) to the base's edge (h {base[0]:.2f}): {base[0] - c - 50:.2f}")
