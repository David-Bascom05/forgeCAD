#!/usr/bin/env python3
"""wrap::Flat::Arc: a circle drawn in (s, h) and wrapped round a cylinder. Proves the facts the code
relies on, re-derives its meet coefficients, and prints the reference values its tests compare with.
Run: python3 check_arc.py (sympy, mpmath). Output: check_arc.txt."""
import sympy as sp
import mpmath as mp

mp.mp.dps = 40
a, U, cx, cy, rho = sp.symbols("a U c_x c_y rho", positive=True)
u, t = sp.symbols("u t", real=True)

# 1. In (phi, h), phi = s / U the angle, a circle of radius a is an ellipse with semi-axes a/U and a.
s, h = cx + a * sp.cos(u), cy + a * sp.sin(u)
phi = s / U
assert sp.simplify(((phi - cx / U) * U / a) ** 2 + ((h - cy) / a) ** 2 - 1) == 0
print("ok  circle in (s, h): ellipse in (phi, h), semi-axes a/U and a")

# 2. The wall between it at two radii: rulings radial and square to the axis (a right conoid); area
#    element |S_u x S_rho| = sqrt(rho^2 phi'^2 + h'^2).
S = sp.Matrix([rho * sp.cos(phi), rho * sp.sin(phi), h])
Su, Sr = S.diff(u), S.diff(rho)
assert sp.simplify(Sr[2]) == 0 and sp.simplify(Sr.dot(Sr) - 1) == 0
n = Su.cross(Sr)
assert sp.simplify(n.dot(n) - (rho**2 * phi.diff(u) ** 2 + h.diff(u) ** 2)) == 0
print("ok  wall: radial rulings square to the axis; |S_u x S_rho|^2 = rho^2 phi'^2 + h'^2")

# 3. Flat::meet_poly for an arc: (X - cx)^2 + (Y - cy)^2 - r^2 along quadratic X(t), Y(t), as the code
#    sums it (out[i + j] += x_i x_j + y_i y_j, out[0] -= r^2). Degree 4; 2 along a line.
X = sp.symbols("x0:3", real=True)
Y = sp.symbols("y0:3", real=True)
r = sp.Symbol("r", positive=True)
Xt, Yt = sum(X[i] * t**i for i in range(3)), sum(Y[i] * t**i for i in range(3))
want = sp.Poly(sp.expand((Xt - cx) ** 2 + (Yt - cy) ** 2 - r**2), t)
x = [X[0] - cx, X[1], X[2]]
y = [Y[0] - cy, Y[1], Y[2]]
code = [0] * 5
for i in range(3):
    for j in range(3):
        code[i + j] += x[i] * x[j] + y[i] * y[j]
code[0] -= r**2
assert all(sp.expand(want.coeff_monomial(t**k) - code[k]) == 0 for k in range(5))
assert want.degree() == 4
line = sp.Poly(sp.expand(((X[0] + X[1] * t) - cx) ** 2 + ((Y[0] + Y[1] * t) - cy) ** 2 - r**2), t)
assert line.degree() == 2
print("ok  circle against a parabola: the code's quartic; against a line: quadratic")
# a parabola's or line's implicit along the arc is trigonometric in u: the code instead puts the
# polynomial curve into the circle's implicit and finds the arc's t from the point (atan2), so the
# degree stays 4.

# 4. Two circles: subtracting the implicits leaves the radical line; along the centres' line at
#    x = (d^2 + ra^2 - rb^2) / (2 d), across it +-sqrt(ra^2 - x^2).
d, ra, rb, yy = sp.symbols("d r_a r_b y", positive=True)
xr = (d**2 + ra**2 - rb**2) / (2 * d)
assert sp.simplify(xr**2 + (ra**2 - xr**2) - ra**2) == 0
assert sp.simplify((xr - d) ** 2 + (ra**2 - xr**2) - rb**2) == 0
print("ok  two circles meet at x = (d^2 + ra^2 - rb^2) / 2d, y = +-sqrt(ra^2 - x^2)")

# 5. Reference crossings (t on each curve), solved exactly then printed as the nearest doubles.
R = sp.Rational
C1 = (R(1, 2), R(1, 4), R(2))  # centre x, centre y, radius
C2 = (R(5, 2), R(1), R(3, 2))
START, SWEEP = R(-3), R(6)  # both arcs: angles -3..3
LINE = ((R(-3), R(-1)), (R(4), R(2)))
PO, PX, PY, PF, PR = (R(1, 5), R(-1, 2)), (R(3, 5), R(4, 5)), (R(-4, 5), R(3, 5)), R(1, 2), (R(-3), R(3))


def arc_t(c, q):
    ang = sp.atan2(q[1] - c[1], q[0] - c[0])
    return sp.nsimplify(0) + (ang - START) / SWEEP


def line_pt(tt):
    (ax, ay), (bx, by) = LINE
    return (ax + (bx - ax) * tt, ay + (by - ay) * tt)


def par_pt(uu):
    k = 1 / (4 * PF)
    return (PO[0] + PX[0] * k * uu**2 + PY[0] * uu, PO[1] + PX[1] * k * uu**2 + PY[1] * uu)


def circ(c, q):
    return (q[0] - c[0]) ** 2 + (q[1] - c[1]) ** 2 - c[2] ** 2


def show(name, pairs):
    print(name)
    for p0, p1 in pairs:
        p0, p1 = sp.N(p0, 30), sp.N(p1, 30)
        assert 0.02 < p0 < 0.98 and 0.02 < p1 < 0.98, (p0, p1)
        print(f"    ({float(p0)!r}, {float(p1)!r}),")


roots = sp.Poly(circ(C1, line_pt(t)), t).real_roots()
show("arc C1 x line: (t on line, t on arc)", [(z, arc_t(C1, line_pt(z))) for z in roots])
uu = sp.Symbol("uu", real=True)
roots = sp.Poly(sp.expand(circ(C1, par_pt(uu))), uu).real_roots()
w = PR[1] - PR[0]
show(
    "arc C1 x parabola: (t on parabola, t on arc)",
    [((z - PR[0]) / w, arc_t(C1, par_pt(z))) for z in roots if PR[0] < z < PR[1]],
)
D = sp.sqrt((C2[0] - C1[0]) ** 2 + (C2[1] - C1[1]) ** 2)
e = ((C2[0] - C1[0]) / D, (C2[1] - C1[1]) / D)
xr_ = (D**2 + C1[2] ** 2 - C2[2] ** 2) / (2 * D)
yr = sp.sqrt(C1[2] ** 2 - xr_**2)
pts = [(C1[0] + e[0] * xr_ - e[1] * k, C1[1] + e[1] * xr_ + e[0] * k) for k in (-yr, yr)]
for q in pts:
    assert sp.simplify(circ(C1, q)) == 0 and sp.simplify(circ(C2, q)) == 0
show("arc C1 x arc C2: (t on C1, t on C2)", sorted([(arc_t(C1, q), arc_t(C2, q)) for q in pts]))
# levels: h = 1 and s = -1 on C1
lv = []
for c, v in ((1, R(1)), (0, R(-1))):
    base = sp.asin((v - C1[1]) / C1[2]) if c == 1 else sp.acos((v - C1[0]) / C1[2])
    angs = [base, sp.pi - base] if c == 1 else [base, -base]
    lv.append(sorted(sp.N((g - START) / SWEEP, 30) for g in angs))
print("arc C1 levels h = 1, s = -1:")
for l in lv:
    print("    [" + ", ".join(repr(float(z)) for z in l) + "],")

# 6. A plug: the disc of radius A at (CX, CY) in (s, h), unroll UN, between radii LO and HI.
#    Volume pi A^2 (HI^2 - LO^2) / (2 UN); caps pi A^2 rho / UN; the wall's area by quadrature of 2.
A, CX, CY, UN, LO, HI = 3, 2, 1, mp.mpf(54) / 5, 10, 12
vol = mp.pi * A**2 * (HI**2 - LO**2) / (2 * UN)
caps = mp.pi * A**2 * (LO + HI) / UN
wall = mp.quad(lambda uu, rr: mp.sqrt(rr**2 * (A * mp.sin(uu) / UN) ** 2 + (A * mp.cos(uu)) ** 2),
               [0, mp.pi / 2, mp.pi, 3 * mp.pi / 2, 2 * mp.pi], [LO, HI])
print("plug A=3 at (2, 1), unroll 54/5, radii 10..12:")
print(f"    volume {mp.nstr(vol, 25)} = {float(vol)!r}")
print(f"    area   {mp.nstr(caps + wall, 25)} = {float(caps + wall)!r}  (wall {mp.nstr(wall, 25)})")
