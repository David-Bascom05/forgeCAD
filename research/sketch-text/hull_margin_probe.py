#!/usr/bin/env python3
"""Does kernel/forge-sketch/src/winding.rs's per-piece margin bound the computed hull's error?

`bezier` splits a Bezier piece at its midpoint in f64 and decides it by its control hull's box,
widened by m = 4(depth+2)·eps·big/wmin, where big is the largest coordinate of the piece being
decided. This compares, along the halving path towards a point the curve passes through, the
computed control points (f64, as `bezier` computes them) with the exact ones (rationals), and
prints the worst error/m seen. Above 1, the margin does not cover the rounding.

    python3 research/sketch-text/hull_margin_probe.py
"""
from fractions import Fraction as F
import random

EPS = 2.0**-52


def split(rows, exact):
    n = len(rows)
    rows = [list(r) for r in rows]
    left, right = [rows[0][:]], [rows[n - 1][:]]
    for k in range(1, n):
        for i in range(n - k):
            for c in range(4):
                s = rows[i][c] + rows[i + 1][c]
                rows[i][c] = s / 2 if exact else s / 2.0
        left.append(rows[0][:])
        right.append(rows[n - 1 - k][:])
    right.reverse()
    return left, right


def bern(pts, t, c):
    b = [(1 - t) ** 3, 3 * t * (1 - t) ** 2, 3 * t * t * (1 - t), t ** 3]
    return sum(b[i] * pts[i][c] for i in range(4))


random.seed(7)
worst, case = 0.0, None
for trial in range(400):
    scale = random.choice([100.0, 1000.0, 10000.0])
    pts = [[random.uniform(-scale, scale), random.uniform(-scale, scale), 0.0, 1.0] for _ in range(4)]
    # move the curve so it passes through the origin at t0: the pieces round t0 are small
    t0 = random.uniform(0.2, 0.8)
    ox, oy = bern(pts, t0, 0), bern(pts, t0, 1)
    for p in pts:
        p[0] -= ox
        p[1] -= oy
    hf = [list(p) for p in pts]
    he = [[F(c) for c in p] for p in pts]
    lo, hi = 0.0, 1.0
    for depth in range(48):
        big = max(max(abs(q[0]), abs(q[1]), abs(q[3])) for q in hf)
        wmin = min(q[3] for q in hf)
        m = 4 * (depth + 2) * EPS * big / wmin
        err = max(abs(F(hf[i][c] / hf[i][3]) - he[i][c] / he[i][3]) for i in range(4) for c in (0, 1))
        if float(err) / m > worst:
            worst, case = float(err) / m, (trial, scale, depth, float(err), m, big)
        lf, rf = split(hf, False)
        le, re = split(he, True)
        mid = (lo + hi) / 2
        if t0 < mid:
            hf, he, hi = lf, le, mid
        else:
            hf, he, lo = rf, re, mid
trial, scale, depth, err, m, big = case
print(f"worst error/m: {worst:.1f} (trial {trial}, control points within {scale:g}, depth {depth}: "
      f"error {err:.3g}, m {m:.3g}, big {big:.3g})")
