#!/usr/bin/env python3
"""Exact volume of a smooth loft ending on a point, from the loft's definition (skinned.rs, loft.rs).

Every matched point of the sections follows the same spline weights over the same knots (see
`spans`, natural at both ends), and a section's shape stays the same,
so the slice at spline parameter s is the first section's shape scaled by L(s) about C(s). Its area
is A0 L(s)^2 whatever C(s) does, and with Z(s) the sections' height:

    V = A0 * integral L(s)^2 Z'(s) ds

L and Z are piecewise cubics, so V is exact over the rationals. Sections on one cone (any interpolation
that reproduces a straight line stays on it) have V = A0 H / 3 whichever spline is meant: those
cases need no definition.

    apex_volume.py        (needs sympy; floats are read as the exact rationals they are)
"""
import math
from fractions import Fraction as F

import sympy as sp


def natural(knots, values):
    """The natural cubic spline's pieces in s, exact."""
    n = len(knots)
    h = [knots[i + 1] - knots[i] for i in range(n - 1)]
    M = sp.Matrix.zeros(n, n)
    b = sp.Matrix.zeros(n, 1)
    M[0, 0] = M[n - 1, n - 1] = 1
    for i in range(1, n - 1):
        M[i, i - 1], M[i, i], M[i, i + 1] = h[i - 1], 2 * (h[i - 1] + h[i]), h[i]
        b[i] = 6 * ((values[i + 1] - values[i]) / h[i] - (values[i] - values[i - 1]) / h[i - 1])
    m = M.LUsolve(b)
    s = sp.Symbol("s")
    out = []
    for i in range(n - 1):
        a, c = knots[i + 1] - s, s - knots[i]
        out.append(
            (m[i] * a**3 + m[i + 1] * c**3) / (6 * h[i])
            + (values[i] - m[i] * h[i] ** 2 / 6) * a / h[i]
            + (values[i + 1] - m[i + 1] * h[i] ** 2 / 6) * c / h[i]
        )
    return s, out


def spans(secs):
    """The knot spacing: the tenth percentile of the chords between matched points (loft.rs), each circle
    sampled 16 times along each of its halves."""
    out = []
    for (c0, r0), (c1, r1) in zip(secs, secs[1:]):
        chords = []
        for i in range(32):
            t = 2 * math.pi * i / 32
            a = (c0[0] + r0 * math.cos(t), c0[1] + r0 * math.sin(t), c0[2])
            b = (c1[0] + r1 * math.cos(t), c1[1] + r1 * math.sin(t), c1[2])
            chords.append(math.dist(a, b))
        chords.sort()
        out.append(chords[len(chords) // 10])
    return out


def volume(secs):
    """V / A0, A0 the first section's area: secs are (centre, radius), the last radius 0."""
    rat = lambda x: sp.Rational(*F(x).as_integer_ratio())
    r = [rat(c[2]) for c, _ in secs]
    l = [rat(q / secs[0][1]) for _, q in secs]
    k = [sp.Integer(0)]
    for x in spans(secs):
        k.append(k[-1] + rat(x))
    s, zp = natural(k, r)
    _, lp = natural(k, l)
    v = sp.Integer(0)
    for i in range(len(k) - 1):
        v += sp.integrate(sp.expand(lp[i] ** 2 * sp.diff(zp[i], s)), (s, k[i], k[i + 1]))
    return v


def case(name, secs):
    print(f"{name:14} {sp.N(math.pi * secs[0][1] ** 2 * volume(secs), 17)}")


case("cone", [((0, 0, 0), 5), ((0, 0, 10), 2.5), ((0, 0, 20), 0)])
case("oblique", [((0, 0, 0), 5), ((1.5, 1, 10), 2.5), ((3, 2, 20), 0)])
case("cone3", [((0, 0, 0), 5), ((0, 0, 4), 4), ((0, 0, 12), 2), ((0, 0, 20), 0)])
case("coaxial", [((0, 0, 0), 5), ((0, 0, 6), 4), ((0, 0, 12), 0)])
case("bulge", [((0, 0, 0), 5), ((0, 0, 5), 5), ((0, 0, 10), 0)])
