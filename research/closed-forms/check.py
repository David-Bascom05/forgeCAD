"""Checks the hand-derived closed forms in forge-geom against their definitions (needs sympy).

Run: fbuild python3 research/closed-forms/check.py   (prints one line per check, exits 1 on a mismatch)
"""

import random
import sys

import sympy as sp

t = sp.symbols("t")
ok = True


def check(name, cond):
    global ok
    print(("ok   " if cond else "FAIL ") + name)
    ok &= bool(cond)


def coeffs(e, n):
    e = sp.expand(e)
    return [e.coeff(t, i) for i in range(n + 1)]


def parabola_quadric():
    # intersect.rs parabola_quadric: P(t) = o + k t^2 x + t y in the quadric's frame,
    # Q(p) = p.M p + l.p + c0 with M symmetric
    o, x, y, lv = (sp.Matrix(sp.symbols(f"{n}0:3")) for n in "oxyl")
    m = sp.Matrix(3, 3, lambda i, j: sp.Symbol(f"m{min(i, j)}{max(i, j)}"))
    k, c0 = sp.symbols("k c0")
    p = o + k * t**2 * x + t * y
    q = (p.T * m * p)[0] + lv.dot(p) + c0
    mo, mx, my = m * o, m * x, m * y
    code = [
        o.dot(mo) + lv.dot(o) + c0,
        2 * y.dot(mo) + lv.dot(y),
        2 * k * x.dot(mo) + y.dot(my) + k * lv.dot(x),
        2 * k * x.dot(my),
        k * k * x.dot(mx),
    ]
    got = coeffs(q, 4)
    check("parabola on a quadric: quartic coefficients", all(sp.expand(a - b) == 0 for a, b in zip(got, code)))


def parabola_wall():
    # ruled/meet.rs parabola_wall: the wall's parabola X = ka Y^2 against X(t), Y(t) quadratic
    x0, x1, x2, y0, y1, y2, ka = sp.symbols("x0 x1 x2 y0 y1 y2 ka")
    big_x = x0 + x1 * t + x2 * t**2
    big_y = y0 + y1 * t + y2 * t**2
    code = [
        x0 - ka * y0 * y0,
        x1 - ka * 2 * y0 * y1,
        x2 - ka * (y1 * y1 + 2 * y0 * y2),
        -ka * 2 * y1 * y2,
        -ka * y2 * y2,
    ]
    got = coeffs(big_x - ka * big_y**2, 4)
    check("parabola on a parabolic wall: quartic coefficients", all(sp.expand(a - b) == 0 for a, b in zip(got, code)))


def parabola_nearest():
    # ruled.rs parabola_nearest: P(t) = (k t^2, t) in the parabola's plane, (P - q).P' = 0
    k, qx, qy = sp.symbols("k qx qy")
    p = sp.Matrix([k * t**2, t])
    q = sp.Matrix([qx, qy])
    e = (p - q).dot(p.diff(t))
    code = 2 * k**2 * t**3 + (1 - 2 * k * qx) * t - qy
    check("parabola nearest point: the depressed cubic", sp.expand(e - code) == 0)
    # Cardano's roots as the code takes them, against the cubic, in 60-digit arithmetic
    rng = random.Random(7)
    worst = 0
    for _ in range(400):
        a, b, c = 2 * rng.uniform(0.01, 3) ** 2, rng.uniform(-20, 20), rng.uniform(-20, 20)
        pp, qq = sp.Rational(b) / sp.Rational(a), sp.Rational(c) / sp.Rational(a)
        disc = qq * qq / 4 + pp**3 / 27
        if disc >= 0:
            r = sp.sqrt(disc)
            roots = [sp.real_root(-qq / 2 + r, 3) + sp.real_root(-qq / 2 - r, 3)]
        else:
            m = 2 * sp.sqrt(-pp / 3)
            th = sp.acos(3 * qq / (pp * m)) / 3
            roots = [m * sp.cos(th - 2 * sp.pi * i / 3) for i in range(3)]
        for z in roots:
            worst = max(worst, abs(sp.N(z**3 + pp * z + qq, 60)))
    check(f"Cardano's roots solve t^3 + p t + q (worst residual {float(worst):.1e})", worst < 1e-40)


def quartic_counts():
    # meet.rs root count: sign of the discriminant and the invariants P = 8ac - 3b^2 and
    # D = 64a^3e - 16a^2c^2 + 16ab^2c - 16a^2bd - 3b^4 of a s^4 + b s^3 + c s^2 + d s + e
    # (Rees 1922; "Quartic function, nature of the roots"): disc < 0 two real roots,
    # disc > 0 four when P < 0 and D < 0, else none
    s = sp.symbols("s")
    rng = random.Random(11)
    bad = 0
    for _ in range(2000):
        a, b, c, d, e = (sp.Rational(rng.randint(-40, 40), rng.randint(1, 9)) for _ in range(5))
        if a == 0:
            continue
        f = sp.Poly(a * s**4 + b * s**3 + c * s**2 + d * s + e, s)
        disc = sp.discriminant(f)
        if disc == 0:
            continue
        p = 8 * a * c - 3 * b * b
        dd = 64 * a**3 * e - 16 * a * a * c * c + 16 * a * b * b * c - 16 * a * a * b * d - 3 * b**4
        want = 2 if disc < 0 else (4 if p < 0 and dd < 0 else 0)
        bad += want != len(sp.real_roots(f))
    check("quartic real-root count from disc, P and D", bad == 0)


parabola_quadric()
parabola_wall()
parabola_nearest()
quartic_counts()
sys.exit(0 if ok else 1)
