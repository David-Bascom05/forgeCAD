"""The chord-sag relations forge-mesh's tessellate.rs sizes its steps by (needs sympy).

Run: fbuild python3 research/mesh-bounds/check.py   (exits 1 on a mismatch)
"""

import sys

import sympy as sp

ok = True


def check(name, cond):
    global ok
    print(("ok   " if cond else "FAIL ") + name)
    ok &= bool(cond)


r, c, th, k, h = sp.symbols("r c theta kappa h", positive=True)

# arc_step: a chord across angle theta of a circle of radius r sags r (1 - cos(theta / 2));
# the largest theta whose sag is the chord tolerance c is 2 acos(1 - c / r)
step = 2 * sp.acos(1 - c / r)
check("arc_step = 2 acos(1 - chord / r)", sp.simplify(r * (1 - sp.cos(step / 2)) - c) == 0)

# sag is quadratic in the step to leading order, so half the tolerance is a step shorter by sqrt 2
sag = sp.series(r * (1 - sp.cos(th / 2)), th, 0, 4).removeO()
check("sag ~ r theta^2 / 8", sp.simplify(sag - r * th**2 / 8) == 0)
check("half the sag: theta / sqrt 2", sp.simplify(sag.subs(th, th / sp.sqrt(2)) - sag / 2) == 0)

# a triangle on a surface curved both ways (locally z = kappa (x^2 + y^2) / 2): the linear
# interpolant's error at barycentric (l0, l1, l2) is kappa/2 sum_{i<j} l_i l_j |v_i - v_j|^2;
# at an edge's middle kappa |e|^2 / 8, at the centroid kappa sum |e|^2 / 18, so the middle
# sags by at most 4/3 of the worst side's middle (equality for the equilateral triangle)
x = sp.symbols("x0:3")
y = sp.symbols("y0:3")
f = lambda px, py: k * (px**2 + py**2) / 2  # noqa: E731
lam = sp.symbols("l0:3")
px = sum(lam[i] * x[i] for i in range(3))
py = sum(lam[i] * y[i] for i in range(3))
err = sum(lam[i] * f(x[i], y[i]) for i in range(3)) - f(px, py)
e2 = lambda i, j: (x[i] - x[j]) ** 2 + (y[i] - y[j]) ** 2  # noqa: E731
want = k / 2 * (lam[0] * lam[1] * e2(0, 1) + lam[0] * lam[2] * e2(0, 2) + lam[1] * lam[2] * e2(1, 2))
check(
    "interpolation error on a paraboloid",
    sp.expand(err.subs(lam[2], 1 - lam[0] - lam[1]) - want.subs(lam[2], 1 - lam[0] - lam[1])) == 0,
)
third = sp.Rational(1, 3)
centroid = want.subs({lam[0]: third, lam[1]: third, lam[2]: third})
mid = want.subs({lam[0]: sp.Rational(1, 2), lam[1]: sp.Rational(1, 2), lam[2]: 0})
eq = {x[0]: 0, y[0]: 0, x[1]: h, y[1]: 0, x[2]: h / 2, y[2]: sp.sqrt(3) * h / 2}
check("equilateral: the centroid sags 4/3 of a side's middle", sp.simplify(centroid.subs(eq) / mid.subs(eq)) == sp.Rational(4, 3))

sys.exit(0 if ok else 1)
