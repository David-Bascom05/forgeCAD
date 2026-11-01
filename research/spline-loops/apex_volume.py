#!/usr/bin/env python3
"""Exact volume of a smooth loft ending on a point, from the loft's definition (skinned.rs, loft.rs).

Every matched point of the sections follows the same spline weights over the same knots (the
distances between section centres, natural at both ends), and a section's shape stays the same,
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


def volume(zs, scale, centres):
    """V / A0 for sections at heights zs, scaled by `scale`, about `centres`."""
    r = [sp.Rational(*F(x).as_integer_ratio()) for x in zs]
    l = [sp.Rational(*F(x).as_integer_ratio()) for x in scale]
    d = [sp.Rational(*F(math.dist(centres[i], centres[i + 1])).as_integer_ratio()) for i in range(len(zs) - 1)]
    k = [sp.Integer(0)]
    for x in d:
        k.append(k[-1] + x)
    s, zp = natural(k, r)
    _, lp = natural(k, l)
    v = sp.Integer(0)
    for i in range(len(k) - 1):
        v += sp.integrate(sp.expand(lp[i] ** 2 * sp.diff(zp[i], s)), (s, k[i], k[i + 1]))
    return v


def case(name, a0, zs, scale, centres):
    print(f"{name:14} {sp.N(a0 * volume(zs, scale, centres), 17)}")


PI = math.pi
c = lambda r: PI * r * r
case("cone", c(5), [0, 10, 20], [1, 0.5, 0], [(0, 0, 0), (0, 0, 10), (0, 0, 20)])
case("oblique", c(5), [0, 10, 20], [1, 0.5, 0], [(0, 0, 0), (1.5, 1, 10), (3, 2, 20)])
case("cone3", c(5), [0, 4, 12, 20], [1, 0.8, 0.4, 0], [(0, 0, 0), (0, 0, 4), (0, 0, 12), (0, 0, 20)])
case("coaxial", c(5), [0, 6, 12], [1, 0.8, 0], [(0, 0, 0), (0, 0, 6), (0, 0, 12)])
case("bulge", c(5), [0, 5, 10], [1, 1, 0], [(0, 0, 0), (0, 0, 5), (0, 0, 10)])


def fam(name, a0, h, apex):
    case(name, a0, [0, h, apex[2]], [1, 1, 0], [(0, 0, 0), (0, 0, h), apex])


fam("loft_ends/4", c(5.7375418119578585), 5.5714364058387025, (-0.43190666517433574, -1.8793479039325582, 11.142872811677405))
fam("loft_ends/36", c(6.035655681832589), 6.546867153542914, (-3.359234401479008, -0.20352026833032966, 13.093734307085828))
fam("loft_ends/9", (2 * 4.8700073022898085) ** 2, 10.167761717505304, (0, 0, 20.335523435010607))
fam("loft_ends/14", (2 * 6.908817681737341) ** 2, 8.331221925881438, (0, 0, 16.662443851762877))
fam("loft_ends/20", (2 * 6.476891192154389) ** 2, 7.97602611908, (-2.025164525593503, 1.5203169253941375, 15.95205223816))
fam("loft_ends/10", (2 * 6.883142760469029) ** 2 - (4 - PI) * 1.7207856901172573**2, 4.014817814887112, (0, 0, 8.029635629774225))
