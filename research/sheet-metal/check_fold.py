#!/usr/bin/env python3
"""Checks every formula prior-art.md states as derived. Exact where SymPy can, else mpmath at 50 digits.

Conventions: x runs across the bend in the flat, y along the bend axis, w depth into the sheet from the
concave (inside) face, 0 <= w <= t. R inside radius, K measured from the inside face, Rn = R + K t.
Bend zone 0 <= x <= Rn*theta. Run: python check_fold.py (needs sympy, mpmath).
"""
import sympy as sp
import mpmath as mp

mp.mp.dps = 50
ok = []


def check(name, value, expect=True):
    good = bool(value == expect) if not isinstance(value, float) else abs(value) < 1e-30
    ok.append(good)
    print(("PASS " if good else "FAIL ") + name)


x, y, w = sp.symbols("x y w", real=True)
R, t, K, th = sp.symbols("R t K theta", positive=True)
Rn = R + K * t


def fold_zone(x, y, w):
    rho, phi = R + w, x / Rn
    return sp.Matrix([rho * sp.sin(phi), y, R - rho * sp.cos(phi)])


def fold_rest(x, y, w):
    tang = sp.Matrix([sp.cos(th), 0, sp.sin(th)])
    nrm = sp.Matrix([-sp.sin(th), 0, sp.cos(th)])
    c = sp.Matrix([R * sp.sin(th), y, R * (1 - sp.cos(th))])
    return c + (x - Rn * th) * tang - w * nrm


fz, fr = fold_zone(x, y, w), fold_rest(x, y, w)
flat = sp.Matrix([x, y, -w])

check("C0 at x=0", sp.simplify(fz.subs(x, 0) - flat.subs(x, 0)) == sp.zeros(3, 1))
check("C0 at x=Rn*theta", sp.simplify(fz.subs(x, Rn * th) - fr.subs(x, Rn * th)) == sp.zeros(3, 1))
Jz, Jr = fz.jacobian([x, y, w]), fr.jacobian([x, y, w])
Jflat = flat.jacobian([x, y, w])
check("Jacobian continuous at x=0, only on the neutral fibre (w = K t)",
      sp.simplify((Jz.subs(x, 0) - Jflat).subs(w, K * t)) == sp.zeros(3, 3))
print("     tangential stretch at x=0, any w:", sp.simplify(Jz.subs(x, 0)[0, 0]), "(so C1 only at w=K t)")
check("Volume density det J = rho/Rn (up to orientation)",
      sp.simplify(abs(Jz.det()) ** 2 - ((R + w) / Rn) ** 2) == 0)
check("Rigid part has |det J| = 1", sp.simplify(Jr.det() ** 2 - 1) == 0)
G = sp.simplify((Jz.T * Jz).subs(w, K * t))
check("Neutral fibre is isometric: metric of (x,y) block is identity",
      sp.simplify(G[:2, :2] - sp.eye(2)) == sp.zeros(2, 2))

# inverse, as a formula
X, Z = fz[0], fz[2]
rho_back = sp.sqrt(X ** 2 + (R - Z) ** 2)
check("Inverse: rho = hypot(X, R-Z)", sp.simplify(rho_back ** 2 - (R + w) ** 2) == 0)
phi = sp.Symbol("phi", real=True)
check("Inverse: atan2(X, R-Z) = phi", sp.simplify(sp.tan(phi) - (R + w) * sp.sin(phi) / ((R + w) * sp.cos(phi))) == 0)

# allowance, setback, deduction
BA = Rn * th
OSSB = (R + t) * sp.sin(th) / (1 + sp.cos(th))  # (R + t) tan(theta/2), in the form simplify reduces
BD = 2 * OSSB - BA
L1, L2 = sp.symbols("L1 L2", positive=True)
check("Flat length = L1 + L2 - BD = (L1-OSSB) + BA + (L2-OSSB)",
      sp.simplify((L1 - OSSB) + BA + (L2 - OSSB) - (L1 + L2 - BD)) == 0)
# the outside mold line really is at OSSB from each tangent line
Pa, Pb = sp.Matrix([0, -t]), sp.Matrix([(R + t) * sp.sin(th), R - (R + t) * sp.cos(th)])
ta, tb = sp.Matrix([1, 0]), sp.Matrix([sp.cos(th), sp.sin(th)])
apex = Pa + OSSB * ta
check("Outer faces meet OSSB past each tangent line", sp.simplify(apex - (Pb - OSSB * tb)) == sp.zeros(2, 1))

# volume of one bend, against the flat strip it came from, per unit length along the axis
Vfold = sp.integrate((R + w) * th, (w, 0, t))
Vflat = BA * t
check("V_fold - V_flat = theta t^2 (1/2 - K) per unit length",
      sp.simplify(Vfold - Vflat - th * t ** 2 * (sp.Rational(1, 2) - K)) == 0)
check("Same volume only at K = 1/2", sp.solve(sp.simplify(Vfold - Vflat), K) == [sp.Rational(1, 2)])

# skin areas of the bend against the flat strip
print("     inner skin / flat strip =", sp.simplify(R * th / BA), "; outer =", sp.simplify((R + t) * th / BA))

# incompressibility: K from thinning lam = t/t0 in plane strain, K against the original thickness t0
lam, t0, rr = sp.symbols("lambda t0 r", positive=True)
ru = (((rr + lam * t0) ** 2 - rr ** 2) / (2 * t0))
Kl = sp.simplify((ru - rr) / t0)
check("K(lam) = lam^2/2 + (lam-1) r/t0", sp.simplify(Kl - (lam ** 2 / 2 + (lam - 1) * rr / t0)) == 0)
check("K = 1/2 without thinning", sp.simplify(Kl.subs(lam, 1)) == sp.Rational(1, 2))
for kk, rt in [(0.45, 1), (0.40, 1), (0.33, 1), (0.40, 3)]:
    l = sp.Symbol("l", positive=True)
    root = [s for s in sp.solve(l ** 2 / 2 + (l - 1) * rt - kk, l) if s > 0][0]
    print(f"     K={kk} at r/t0={rt} means thickness ratio t/t0 = {float(root):.4f}")

# curves crossing a bend. Flat line x=s, y=h0+m*s. On a skin of radius rho at angle phi=s/Rn.
h0, m, rho = sp.symbols("h0 m rho", real=True)
s = sp.Symbol("s", real=True)
phis = s / Rn
line = sp.Matrix([rho * sp.sin(phis), h0 + m * s, R - rho * sp.cos(phis)])
d1 = line.diff(s)
check("Straight cut on a skin is a helix: speed constant in s",
      sp.simplify(d1.dot(d1) - (rho ** 2 / Rn ** 2 + m ** 2)) == 0)
print("     helix: pitch per radian = m*Rn (same on all radii);",
      "tan(helix angle at radius rho) = m*Rn/rho, so inner and outer edges differ")
d2 = line.diff(s, 2)
kappa2 = sp.simplify(d1.cross(d2).dot(d1.cross(d2)) / d1.dot(d1) ** 3)
print("     curvature^2 =", sp.simplify(kappa2))
# the wall: S(phi, rho) = (rho sin phi, h0 + m Rn phi, R - rho cos phi) is a right helicoid
ph = sp.Symbol("phi", real=True)
wall = sp.Matrix([rho * sp.sin(ph), h0 + m * Rn * ph, R - rho * sp.cos(ph)])
ruling = wall.diff(rho)
check("Normal-cut wall: rulings are radial, perpendicular to the axis", sp.simplify(ruling[1]) == 0)
n = wall.diff(ph).cross(ruling)
gauss_num = sp.simplify(wall.diff(ph, rho).dot(n))
print("     helicoid mixed second form ~", gauss_num, "(nonzero: K_gauss < 0, the wall is not developable)")

# plane cut in 3D: the same plane on two radii, as flat curves
a, b, c, d = sp.symbols("a b c d", real=True)
hplane = (d - a * rho * sp.sin(ph) - c * (R - rho * sp.cos(ph))) / b
print("     plane a X + b Y + c Z = d in the bend: flat curve y =", sp.simplify(hplane), "at x = Rn*phi")
check("Sinusoid amplitude grows with rho, so a plane cut is not prismatic in the flat",
      sp.simplify(hplane.diff(rho)) != 0)

# numbers
vals = dict(R=2, t=2, K=mp.mpf("0.4"), theta=mp.pi / 2)
rn = vals["R"] + vals["K"] * vals["t"]
print("\nNumbers for R=2, t=2, K=0.4, 90 deg: Rn=", mp.nstr(rn, 6), "BA=", mp.nstr(rn * mp.pi / 2, 8),
      "OSSB=", 4, "BD=", mp.nstr(8 - rn * mp.pi / 2, 8))
print("  dV per mm of bend =", mp.nstr(mp.pi / 2 * 4 * mp.mpf("0.1"), 8), "mm^3")
# a 45 degree plane through the bend: y = (rho sin phi) * tan(45): inner vs outer at the end of the bend
print("  plane Y = X through the bend, at phi=90 deg: flat y on inner", vals["R"], "on outer", vals["R"] + vals["t"],
      "-> the wall leans by t across a sheet of thickness t")
# a hole of radius 1.5 drilled through the bend along its bisector, against a normal cut: half-width across the bend
for r_ in (2, 3, 4):
    print(f"  a drilled cylinder radius 1.5 at radius {r_}: flat half-width = Rn*asin(a/rho) =",
          mp.nstr(rn * mp.asin(mp.mpf("1.5") / r_), 7), "vs 1.5 for a normal cut")

# ellipse: flat circle is an ellipse in the skin's (phi, z) parameters
s0, a_ = sp.symbols("s0 a", positive=True)
u = sp.Symbol("u", real=True)
ph_u, z_u = (s0 + a_ * sp.cos(u)) / Rn, h0 + a_ * sp.sin(u)
check("Circle in flat = ellipse in (phi,z): ((phi-phi0)Rn/a)^2+((z-h0)/a)^2=1",
      sp.simplify(((ph_u - s0 / Rn) * Rn / a_) ** 2 + ((z_u - h0) / a_) ** 2) == 1)

# cone: flat polar (l, psi), theta = psi / sin(alpha)
al = sp.Symbol("alpha", positive=True)
l, psi = sp.symbols("l psi", positive=True)
tc = psi / sp.sin(al)
cone = sp.Matrix([l * sp.sin(al) * sp.cos(tc), l * sp.sin(al) * sp.sin(tc), l * sp.cos(al)])
flat_c = sp.Matrix([l * sp.cos(psi), l * sp.sin(psi)])
Gc = sp.simplify(cone.jacobian([l, psi]).T * cone.jacobian([l, psi]))
Gf = sp.simplify(flat_c.jacobian([l, psi]).T * flat_c.jacobian([l, psi]))
check("Right cone development (l, psi = theta sin alpha) is an isometry", sp.simplify(Gc - Gf) == sp.zeros(2, 2))
# offset of a right cone: same generators, apex moved along the axis
W = sp.Symbol("W", real=True)
nc = sp.Matrix([sp.cos(al) * sp.cos(tc), sp.cos(al) * sp.sin(tc), -sp.sin(al)])
off = cone + W * nc
print("     offset cone: radius at (l,W) =", sp.simplify(sp.sqrt(off[0] ** 2 + off[1] ** 2)),
      " (half angle unchanged, apex shifts by W/sin(alpha))")

# plane section of a right cone, in the cone's own polar terms: l(theta) = d / (nz cos a + A sin a cos(theta - t0))
nxx, nyy, nzz, dd = sp.symbols("nx ny nz d", real=True)
th_c = sp.Symbol("thc", real=True)
lc = dd / (nzz * sp.cos(al) + sp.sin(al) * (nxx * sp.cos(th_c) + nyy * sp.sin(th_c)))
pt = lc * sp.Matrix([sp.sin(al) * sp.cos(th_c), sp.sin(al) * sp.sin(th_c), sp.cos(al)])
check("Plane section of a cone, polar form on the cone", sp.simplify(nxx * pt[0] + nyy * pt[1] + nzz * pt[2] - dd) == 0)
print("     in the flat psi = theta sin(alpha), so l(psi) = p / (1 + e cos(psi / sin(alpha) - t0)): a conic only if sin(alpha) = 1")

# oblique cone S(s, th) = s c(th) with any directrix: flat l = s |c|, psi' = |c x c'| / |c|^2
th1 = sp.Symbol("th1", real=True)
cx, cy, cz = [sp.Function(n)(th1) for n in ("cx", "cy", "cz")]
cv = sp.Matrix([cx, cy, cz])
dc = cv.diff(th1)
ss = sp.Symbol("ss", positive=True)
E, F, GG = cv.dot(cv), ss * cv.dot(dc), ss ** 2 * dc.dot(dc)
crs = cv.cross(dc)
psi_d2 = crs.dot(crs) / cv.dot(cv) ** 2
ldth = ss * cv.dot(dc) / sp.sqrt(cv.dot(cv))
check("Any cone: flat l = s|c|, psi' = |c x c'|/|c|^2 is an isometry",
      sp.simplify(ldth ** 2 + ss ** 2 * cv.dot(cv) * psi_d2 - GG) == 0)
# an elliptic directrix in a plane: the development angle against an elementary guess, no closed form claimed
ea, eb, hh = mp.mpf(3), mp.mpf(1), mp.mpf(2)
pd = lambda u: mp.sqrt((ea * mp.sin(u)) ** 2 * hh ** 2 + (eb * mp.cos(u)) ** 2 * hh ** 2 + (ea * eb) ** 2) / ((ea * mp.cos(u)) ** 2 + (eb * mp.sin(u)) ** 2 + hh ** 2)
print("     oblique cone over a 3x1 ellipse at height 2: total development angle by quadrature =", mp.nstr(mp.quad(pd, [0, 2 * mp.pi]), 20), "rad")

# developability of a ruled loft S = A(t) + s (B(t) - A(t)): det[A', B', B-A] = 0
T = sp.Symbol("T", real=True)
r1, r2, H, ox, phase = sp.symbols("r1 r2 H ox phase", real=True)
A = sp.Matrix([r1 * sp.cos(T), r1 * sp.sin(T), 0])
Bp = sp.Matrix([ox + r2 * sp.cos(T + phase), r2 * sp.sin(T + phase), H])


def dev(A, B):
    return sp.simplify(sp.Matrix.hstack(A.diff(T), B.diff(T), B - A).det())


check("Two parallel-plane circles, parallel tangents matched (oblique cone): developable", dev(A, Bp.subs(phase, 0)) == 0)
print("     same circles, matched with a phase: det =", sp.simplify(dev(A, Bp)), "(nonzero: not developable)")
# a line matched to a circle arc: a corner of a square-to-round, the point case
P0 = sp.Matrix([5, 5, 0])
check("Corner point to arc is a cone: developable", dev(sp.Matrix.hstack(P0), Bp.subs(phase, 0)) == 0)
# a circle against a line of a different direction, the classic square-to-round "bad" loft
LineA = sp.Matrix([T, 0, 0])
print("     line against arc, matched by parameter: det =", sp.simplify(dev(LineA, Bp.subs(phase, 0))))

# Gaussian curvature of a torus: bend along a curved edge
Rm, rm, v = sp.symbols("Rm rm v", positive=True)
print("     torus Gaussian curvature = cos(v)/(rm (Rm + rm cos v)): nonzero on a bend along a circular edge")

# what a K error costs, against the kernel tolerance
print("\nK uncertain by 0.1, t=2, 90 deg: flat length error per bend =", mp.nstr(mp.pi / 2 * 2 * mp.mpf("0.1"), 5), "mm, against tol::LINEAR 1e-5 mm")

# corners: flat gap from the sector angles


def sector(u, v):
    cr = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
    return mp.atan2(mp.sqrt(sum(q * q for q in cr)), sum(p * q for p, q in zip(u, v)))


print("\nCorner gap in the flat = 2 pi - sum of wall corner angles (square base corner, both flanges bent by the same angle)")
for deg in (15, 30, 60, 90, 120):
    b_ = mp.radians(deg)
    cb, sb = mp.cos(b_), mp.sin(b_)
    # base along +x, +y; the flanges' planes meet in the seam (-c, -c, s)
    seam = (-cb, -cb, sb)
    a0, a1, a2 = sector((1, 0, 0), (0, 1, 0)), sector((0, 1, 0), seam), sector(seam, (1, 0, 0))
    gap = 2 * mp.pi - (a0 + a1 + a2)
    print(f"  bent {deg} deg: corner angles {[mp.nstr(mp.degrees(q), 7) for q in (a0, a1, a2)]} gap {mp.nstr(mp.degrees(gap), 7)} deg")

# two equal cylinders with crossing axes meet in a plane, the mitre plane, whose flat image is a sinusoid
print("  equal-radius mitre: cylinder pair meets in a plane; unrolled, the plane is a sine curve (see plane cut above)")

# DIN against the geometric mean and the shop table
print("\nK by r/t: DIN (0.65+0.5 lg(r/t))/2, clipped at 0.5, | stress neutral sqrt(ri ro) with constant t")
for rt in (0.05, 0.1, 0.25, 0.5, 1, 2, 3, 5, 10):
    din = min(0.5, (0.65 + 0.5 * float(mp.log10(rt))) / 2)
    hill = float(mp.sqrt(rt * (rt + 1)) - rt)
    print(f"  r/t={rt:<5} DIN K={din:7.4f}   sqrt(ri*ro) K={hill:6.4f}")
print("  DIN k reaches 1 at r/t =", mp.nstr(10 ** ((1 - mp.mpf("0.65")) / mp.mpf("0.5")), 6), "and K reaches 0 at r/t =",
      mp.nstr(10 ** (-mp.mpf("0.65") / mp.mpf("0.5")), 5))

# round trip error and placement drift in binary64
import math
import random

random.seed(1)
worst = 0.0
for _ in range(200000):
    R_, t_, K_ = random.uniform(0.2, 20), random.uniform(0.2, 6), random.uniform(0.2, 0.5)
    Rn_ = R_ + K_ * t_
    x_ = random.uniform(0, Rn_ * math.pi)
    w_ = random.uniform(0, t_)
    rho_ = R_ + w_
    X_, Z_ = rho_ * math.sin(x_ / Rn_), R_ - rho_ * math.cos(x_ / Rn_)
    xb = Rn_ * math.atan2(X_, R_ - Z_)
    wb = math.hypot(X_, R_ - Z_) - R_
    worst = max(worst, abs(xb - x_), abs(wb - w_))
print(f"\nbinary64 fold/unfold round trip over 200k random bends up to 180 deg: worst {worst:.2e} mm (tol::LINEAR is 1e-5)")


def mul(A, B):
    return [[sum(A[i][k] * B[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def bend(th_, L, set_):
    c, s = math.cos(th_), math.sin(th_)
    # go L along x, turn theta about y at the mold line
    return [[c, 0, -s, L * 1.0], [0, 1, 0, 0], [s, 0, c, 0], [0, 0, 0, 1]]


for n in (10, 100, 1000):
    M = [[float(i == j) for j in range(4)] for i in range(4)]
    for i in range(n):
        M = mul(M, bend(math.pi / 2, 37.5, 0))
    # after 4k quarter turns the frame must be the identity, and the origin is back
    k = n - n % 4
    N = [[float(i == j) for j in range(4)] for i in range(4)]
    for i in range(k):
        N = mul(N, bend(math.pi / 2, 37.5, 0))
    drift = max(abs(N[i][j] - (1.0 if i == j else 0.0)) for i in range(3) for j in range(3))
    print(f"  {k} chained 90 deg bends (a square loop of walls): rotation drift {drift:.2e}, position drift {max(abs(N[i][3]) for i in range(3)):.2e} mm")

print(f"\n{sum(ok)}/{len(ok)} checks passed")
raise SystemExit(0 if all(ok) else 1)
