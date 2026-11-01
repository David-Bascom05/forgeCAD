#!/usr/bin/env python3
"""What kernel/forge-geom/src/spline.rs and spline/meet.rs compute in closed form, checked symbolically.

Each check builds the formula the Rust uses and shows the defining equation holds identically, or
that its polynomial degree is what the code allots. Run: python3 research/spline-loops/derive.py
"""
import sympy as sp

s = sp.symbols("s")


def bernstein(cs, s=s):
    n = len(cs) - 1
    return sum(sp.binomial(n, i) * s**i * (1 - s) ** (n - i) * c for i, c in enumerate(cs))


def coords(name, n):
    return list(sp.symbols(f"{name}0:{n + 1}"))


# 1. A plane cuts an extruded curve in an affine image of the curve, same weights: a point (x, y) of
#    the section at height z = kx x + ky y + k0 lies on the plane n.q + d = 0 with kx = -nx/nz etc.
#    In homogeneous form the height's control value is kx X + ky Y + k0 W: exact, per control point.
for p in (2, 3, 5):
    X, Y, W = coords("X", p), coords("Y", p), coords("W", p)
    nx, ny, nz, d = sp.symbols("nx ny nz d")
    kx, ky, k0 = -nx / nz, -ny / nz, -d / nz
    A, B, w = bernstein(X), bernstein(Y), bernstein(W)
    Z = bernstein([kx * X[i] + ky * Y[i] + k0 * W[i] for i in range(p + 1)])
    # n.(A, B, Z)/w + d vanishes identically
    assert sp.simplify(sp.expand(nx * A + ny * B + nz * Z + d * w)) == 0
print("1 plane cut: control heights kx X + ky Y + k0 W, weights unchanged (p = 2, 3, 5)")

# 2. The foot of a point on a piece: (A - p w).(A' w - A w') = 0, numerator of (C - p).C'.
#    Degrees: u = A - p w is p, D = A' w - A w' has formal degree 2p - 1 (true 2p - 2), so 3p - 1.
for q in (2, 3, 5):
    a = bernstein(coords("a", q))
    w = bernstein(coords("w", q))
    px = sp.symbols("px")
    c = a / w
    stationary = (c - px) * sp.diff(c, s)
    numer = (a - px * w) * (sp.diff(a, s) * w - a * sp.diff(w, s))
    assert sp.simplify(stationary - numer / w**3) == 0
    D = sp.expand(sp.diff(a, s) * w - a * sp.diff(w, s))
    assert sp.degree(D, s) == 2 * q - 2
print("2 foot polynomial: (C - p).C' = (A - p w).(A' w - A w') / w^3; D has degree 2p - 2, kept as 2p - 1")

# 3. A quadric on the curve: q.A.q + 2 b.q + c with q = C - o, times w^2, polynomial of degree 2p in
#    the homogeneous coordinates (Trace::Circle, line against a spun wall).
xw, yw, w = sp.symbols("xw yw w")
ox, oy, r = sp.symbols("ox oy r")
form = (xw - ox * w) ** 2 + (yw - oy * w) ** 2 - r**2 * w**2
assert sp.simplify(form / w**2 - ((xw / w - ox) ** 2 + (yw / w - oy) ** 2 - r**2)) == 0
print("3 circle on a section: homogeneous quadratic form, degree 2p")

# 4. A line against a spun wall: on (rho(t), h(t)) with h = lz + vz t, rho^2 = s2 t^2 + s1 t + s0.
#    Eliminating t = (h - lz)/vz leaves a quadric in (r, h) of the section (upto.rs rays):
#    -r^2 + a2 h^2 + 2 b h + a0, with a2 = s2/vz^2, a1 = s1/vz, 2 b = a1 - 2 a2 lz, a0 = s0 - a1 lz + a2 lz^2.
rr, h, lz, vz, s2, s1, s0 = sp.symbols("r h lz vz s2 s1 s0")
t = (h - lz) / vz
a2, a1 = s2 / vz**2, s1 / vz
a0 = s0 - a1 * lz + a2 * lz**2
b = sp.Rational(1, 2) * a1 - a2 * lz
assert sp.simplify((s2 * t**2 + s1 * t + s0 - rr**2) - (-(rr**2) + a2 * h**2 + 2 * b * h + a0)) == 0
print("4 ray against a spun wall: quadric in (r, h), coefficients as in upto.rs")

# 5. The natural normal of a spun wall is to the right of the section's direction in (r, h):
#    du x dv = r (h' cos u, h' sin u, -r').
u, v = sp.symbols("u v")
R, H = sp.Function("R")(v), sp.Function("H")(v)
S = sp.Matrix([R * sp.cos(u), R * sp.sin(u), H])
cross = S.diff(u).cross(S.diff(v))
want = R * sp.Matrix([sp.diff(H, v) * sp.cos(u), sp.diff(H, v) * sp.sin(u), -sp.diff(R, v)])
assert sp.simplify(cross - want) == sp.zeros(3, 1)
print("5 spun normal: du x dv = r (h' cos u, h' sin u, -r')")

# 6. A quarter of an ellipse: rational quadratic P0, P1 at the tangents' meeting, weight cos(d/2).
th, d = sp.symbols("theta d", positive=True)
a_, b_ = sp.symbols("a b", positive=True)
w1 = sp.cos(d / 2)
P0 = sp.Matrix([a_ * sp.cos(th), b_ * sp.sin(th)])
P2 = sp.Matrix([a_ * sp.cos(th + d), b_ * sp.sin(th + d)])
P1 = sp.Matrix([a_ * sp.cos(th + d / 2) / w1, b_ * sp.sin(th + d / 2) / w1])
den = (1 - s) ** 2 + 2 * s * (1 - s) * w1 + s**2
Q = ((1 - s) ** 2 * P0 + 2 * s * (1 - s) * w1 * P1 + s**2 * P2) / den
eq = (Q[0] / a_) ** 2 + (Q[1] / b_) ** 2 - 1
vals = {th: sp.Rational(3, 10), d: sp.Rational(11, 10), s: sp.Rational(1, 3)}
assert abs(sp.N(eq.subs(vals), 40)) < 1e-35
vals = {th: sp.Rational(-7, 10), d: sp.Rational(3, 2), s: sp.Rational(5, 7)}
assert abs(sp.N(eq.subs(vals), 40)) < 1e-35
print("6 ellipse arc: rational quadratic with w = cos(d/2), P1 = (a cos(m)/w, b sin(m)/w)")
