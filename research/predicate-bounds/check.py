"""Tests the float filters of forge-math's predicates.rs against exact rational arithmetic.

Python floats are IEEE doubles rounded to nearest, one rounding per operation (no FMA), as in
the Rust. For each filter, the error of the plain f64 evaluation must stay within its bound
times the magnitude the code computes, on random and near-degenerate inputs.

Run: python3 research/predicate-bounds/check.py   (exits 1 if any bound is exceeded)
"""

import random
import sys
from fractions import Fraction as F

EPS = 2.0**-53
CROSS_BOUND = (2.0 + 8.0 * EPS) * EPS
DOT3_BOUND = (3.0 + 32.0 * EPS) * EPS
TRIPLE_BOUND = (5.0 + 64.0 * EPS) * EPS

rng = random.Random(5)


def val():
    # mixed scales and signs, some exact repeats, so cancellation is common
    return rng.choice([1, -1]) * rng.uniform(0.5, 1.0) * 2.0 ** rng.randint(-30, 30)


def near(x):
    # a neighbour of x a few ulps away, to make the result nearly cancel
    return x * (1 + rng.randint(-8, 8) * EPS)


def worst(name, cases, f):
    w = 0.0
    for _ in range(cases):
        got, exact, mag, bound = f()
        if mag == 0:
            continue
        w = max(w, float(abs(F(got) - exact) / F(mag)) / bound)
    print(f"{'ok  ' if w <= 1 else 'FAIL'} {name}: worst error {w:.3f} of the bound")
    return w <= 1


def cross():
    a, b, c = val(), val(), val()
    d = near(a * b / c) if rng.random() < 0.8 else val()
    l, r = a * b, c * d
    return l - r, F(a) * F(b) - F(c) * F(d), abs(l) + abs(r), CROSS_BOUND


def dot3():
    u = [val() for _ in range(3)]
    v = [val() for _ in range(3)]
    if rng.random() < 0.8:
        v[2] = near(-(u[0] * v[0] + u[1] * v[1]) / u[2])
    x, y, z = (u[i] * v[i] for i in range(3))
    exact = sum(F(u[i]) * F(v[i]) for i in range(3))
    return x + y + z, exact, abs(x) + abs(y) + abs(z), DOT3_BOUND


def triple():
    a = [val() for _ in range(3)]
    b = [val() for _ in range(3)]
    c = [val() for _ in range(3)]
    if rng.random() < 0.8:
        k = rng.uniform(-2, 2)
        c = [near(k * a[i] + b[i]) for i in range(3)]
    yz, zy = b[1] * c[2], b[2] * c[1]
    zx, xz = b[2] * c[0], b[0] * c[2]
    xy, yx = b[0] * c[1], b[1] * c[0]
    det = a[0] * (yz - zy) + a[1] * (zx - xz) + a[2] * (xy - yx)
    perm = abs(a[0]) * (abs(yz) + abs(zy)) + abs(a[1]) * (abs(zx) + abs(xz)) + abs(a[2]) * (abs(xy) + abs(yx))
    A, B, C = ([F(x) for x in v] for v in (a, b, c))
    exact = A[0] * (B[1] * C[2] - B[2] * C[1]) + A[1] * (B[2] * C[0] - B[0] * C[2]) + A[2] * (B[0] * C[1] - B[1] * C[0])
    return det, exact, perm, TRIPLE_BOUND


ok = worst("cross_sign", 200000, cross)
ok &= worst("dot3", 200000, dot3)
ok &= worst("triple", 200000, triple)
sys.exit(0 if ok else 1)
