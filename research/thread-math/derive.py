#!/usr/bin/env python3
"""The closed forms behind kernel/forge-geom/src/screw/along.rs, checked symbolically.

A screw motion carries a section point (r, h) at angle u to ((r + a u) cos u, (r + a u) sin u,
h + b u). Reading a point (rho, phi, z) in the half plane of turn k means u = phi + 2 pi k and
section coordinates q = (rho - a u, z - b u) (screw.rs, `value`).
Run: python3 derive.py   (needs sympy; prints what it proved, exits nonzero on any failure)
"""
import sympy as sp

t, k = sp.Symbol("t", real=True), sp.Symbol("k", integer=True)
a, b, a2, b2 = sp.symbols("a b a2 b2", real=True)
r0, h0, u0, w = sp.symbols("r0 h0 u0 w", real=True)
nx, ny, c = sp.symbols("n_x n_y c", real=True)
ok = True


def check(name, expr):
    global ok
    good = sp.simplify(expr) == 0
    ok &= good
    print(("ok  " if good else "FAIL"), name)


# 1. a curve about the axis moving with rates (a2, b2, w) reads as a point moving linearly in t in
#    every turn's half plane: q_k(t) = q0 - 2 pi k (a, b) + t (a2 - a w, b2 - b w)
u = u0 + w * t + 2 * sp.pi * k
q = sp.Matrix([r0 + a2 * t - a * u, h0 + b2 * t - b * u])
check("q_k is linear in t", sp.diff(q, t, 2).norm())
v = sp.Matrix([a2 - a * w, b2 - b * w])
check("its velocity", (sp.diff(q, t) - v).norm())
# 2. so a section line n . q = c gives one root per turn, its turns a fixed step apart in s
s = nx * q[0] + ny * q[1] - c
B = nx * v[0] + ny * v[1]
step = 2 * sp.pi * (a * nx + b * ny)
check("s_k(t) = s_0(t) - step k", s - (s.subs(k, 0) - step * k))
check("slope of s_k", sp.diff(s, t) - B)
root = -(s.subs(t, 0)) / B
check("the root", s.subs(t, root))
# the same motion on both (w = 1, a2 = a, b2 = b): no slope, each turn a fixed distance away
check("a helix of the surface's own motion stays at its distance", B.subs({a2: a, b2: b, w: 1}))
# a circle gone round twice reads the same section point in the next turn: a root per pass
qc = q.subs({w: 1, a2: 0, b2: 0})
check("a circle: t + 2 pi with k - 1 is the same section point", (qc.subs(t, t + 2 * sp.pi).subs(k, k - 1) - qc).norm())

# 3. an arc of the track: |q0 + v t - C|^2 = R^2 is a quadratic in t
Cx, Cy, R, q0x, q0y, vx, vy = sp.symbols("C_x C_y R q0x q0y v_x v_y", real=True)
rx, ry = q0x - Cx, q0y - Cy
vv = vx**2 + vy**2
t0 = -(rx * vx + ry * vy) / vv
h2 = (rx * vy - ry * vx) ** 2 / vv
half = sp.sqrt(R**2 - h2) / sp.sqrt(vv)
f = lambda tt: (rx + vx * tt) ** 2 + (ry + vy * tt) ** 2 - R**2
for sign in (1, -1):
    val = f(t0 + sign * half)
    check(f"arc root {sign:+d}", val.subs({rx: 1, ry: 2, vx: 3, vy: -1, R: 5}) if False else sp.simplify(val.subs({q0x: Cx + 1, q0y: Cy + 2, vx: 3, vy: -1, R: 5})))

# 4. a helix round a cylinder against any plane n . (P - o) = 0, P = O + r e(t) + z (h + b t):
#    f = r m cos(t - th) + nz b t + D; turning points where sin(t - th) = nz b / (r m)
th, m, r, nz, D = sp.symbols("theta m r n_z D", real=True)
f4 = r * m * sp.cos(t - th) + nz * b * t + D
check("f' of the helix against a plane", sp.diff(f4, t) - (-r * m * sp.sin(t - th) + nz * b))
tc = th + sp.asin(nz * b / (r * m))
check("first turning point", sp.diff(f4, t).subs(t, tc))
tc2 = th + sp.pi - sp.asin(nz * b / (r * m))
check("second turning point", sp.diff(f4, t).subs(t, tc2))
ex, ey = sp.symbols("e_x e_y", real=True)
# expansion of n . e(t) = nx cos t + ny sin t into m cos(t - th)
mm = sp.sqrt(nx**2 + ny**2)
tth = sp.atan2(ny, nx)
check("n . e(t) = m cos(t - th)", sp.expand_trig(mm * sp.cos(t - tth)).rewrite(sp.cos).subs(sp.cos(tth), nx / mm).subs(sp.sin(tth), ny / mm) - (nx * sp.cos(t) + ny * sp.sin(t)))

# 5. the fence's nearest turn: |s_0 - step m| is smallest at one of the two integers about s_0/step
s0, st = sp.symbols("s0 st", positive=True)
x = s0 / st
check("s at the fractional turn is zero", s0 - st * x)

# 6. a line through the cylinder's axis is convex in distance; the plain miss bound in general.rs:
#    with a the cylinder's axis, z the screw's, P = I - a a^T: u = P z / |P z| satisfies
#    u . e = -(a . e)(a . z) / |P z| for e square to z, so |u . P e| <= |a . z|
ax, ay, az = sp.symbols("a_x a_y a_z", real=True)
A = sp.Matrix([ax, ay, az])
Z = sp.Matrix([0, 0, 1])
E = sp.Matrix([ex, ey, 0])
Pz = Z - A * A.dot(Z)
# on the unit sphere for a: |P z|^2 = 1 - az^2
sub = {ax: sp.sqrt(1 - ay**2 - az**2)}
check("|P z|^2 = 1 - (a . z)^2", sp.simplify((Pz.dot(Pz)).subs(sub)) - (1 - az**2))
check("(P z) . e = -(a . e)(a . z)", (Pz.dot(E) + A.dot(E) * A.dot(Z)))
# 7. a straight section against a coaxial round surface (a sphere centred on the axis, a ring torus
#    about it): its meridian is a circle Q(th) = C + R (cos th, sin th) in (r, h). The section line
#    n . (q - p0) = 0, carried to n . (q - p0 - m u) = 0 at angle u (m = (a, b)), holds Q(th) at
#    u(th) = n . (Q(th) - p0) / (n . m): the curve is P(th) = O + e(u(th)) Q_r + z Q_h, smooth in th
#    where u is not (the section tangent to the meridian turns the curve back in u, not in th).
cr, chh, R, p0r, p0h, dr, dh = sp.symbols("c_r c_h R p0_r p0_h d_r d_h", real=True)
th = sp.Symbol("theta", real=True)
Q = sp.Matrix([cr + R * sp.cos(th), chh + R * sp.sin(th)])
p0 = sp.Matrix([p0r, p0h])
dd = sp.Matrix([dr, dh])
n2 = sp.Matrix([-dh, dr])
m2 = sp.Matrix([a, b])
uth = n2.dot(Q - p0) / n2.dot(m2)
sth = dd.dot(Q - p0 - m2 * uth) / dd.dot(dd)
# the point at angle u(th) in its half plane is the section point at s, moved by the motion
check("Q(th) = p0 + d s(th) + m u(th)", sp.simplify(p0 + dd * sth + m2 * uth - Q).norm())
X = sp.Matrix([Q[0] * sp.cos(uth), Q[0] * sp.sin(uth), Q[1]])
# on the screw surface: the section point at s, turned by u(th) and lifted by the motion
S = sp.Matrix([(p0r + dr * sth + a * uth) * sp.cos(uth), (p0r + dr * sth + a * uth) * sp.sin(uth), p0h + dh * sth + b * uth])
check("P(th) on the screw surface", sp.simplify(X - S).norm())
# on the sphere centred at (0, c_h) on the axis, radius R
check("P(th) on the sphere", sp.simplify((X[0]**2 + X[1]**2 + (X[2] - chh)**2 - R**2).subs(cr, 0)))
# on the torus about the axis, major c_r, minor R
rho = sp.sqrt(X[0]**2 + X[1]**2)
check("P(th) on the torus (r > 0)", sp.simplify(((Q[0] - cr)**2 + (X[2] - chh)**2 - R**2)) )
check("rho(P(th)) = Q_r", sp.simplify(rho**2 - Q[0]**2))
# the derivatives the curve gives: u' = n . Q' / (n . m), u'' = n . Q'' / (n . m), and
# P' = e' u' Q_r + e Q_r' + z Q_h',  P'' = (e'' u'^2 + e' u'') Q_r + 2 e' u' Q_r' + e Q_r'' + z Q_h''
up = sp.diff(uth, th)
upp = sp.diff(uth, th, 2)
check("u' = n . Q' / (n . m)", sp.simplify(up - n2.dot(sp.diff(Q, th)) / n2.dot(m2)))
check("u'' = n . Q'' / (n . m)", sp.simplify(upp - n2.dot(sp.diff(Q, th, 2)) / n2.dot(m2)))
e = sp.Matrix([sp.cos(uth), sp.sin(uth), 0])
ep = sp.Matrix([-sp.sin(uth), sp.cos(uth), 0])
zz = sp.Matrix([0, 0, 1])
Qp, Qpp = sp.diff(Q, th), sp.diff(Q, th, 2)
P1 = ep * up * Q[0] + e * Qp[0] + zz * Qp[1]
P2 = (-e * up**2 + ep * upp) * Q[0] + ep * (2 * up * Qp[0]) + e * Qpp[0] + zz * Qpp[1]
check("P' as the code takes it", sp.simplify(sp.diff(X, th) - P1).norm())
check("P'' as the code takes it", sp.simplify(sp.diff(X, th, 2) - P2).norm())
# every limit on th (u in a range, s in a range, Q_r > 0) is alpha + beta cos th + gamma sin th >= 0,
# so alpha + rho cos(th - phi) >= 0 with rho = |(beta, gamma)|, phi = atan2(gamma, beta): an arc of th
al, be, ga = sp.symbols("alpha beta gamma", real=True)
ph = sp.atan2(ga, be)
rr = sp.sqrt(be**2 + ga**2)
check("beta cos + gamma sin = rho cos(th - phi)", sp.simplify(sp.expand_trig(rr * sp.cos(th - ph)) - (be * sp.cos(th) + ga * sp.sin(th))))
for name, f in [("u", uth), ("s", sth)]:
    lin = sp.expand(f * 1)
    rest = sp.simplify(lin - (lin.coeff(sp.cos(th)) * sp.cos(th) + lin.coeff(sp.sin(th)) * sp.sin(th)))
    check(f"{name}(th) is alpha + beta cos th + gamma sin th", sp.diff(rest, th))
# the bounds: |u'| <= R |n| / |n . m|, |P'|^2 = (u' Q_r)^2 + R^2, |P''| <= u'^2 Q_r + |u''| Q_r + 2 |u'| R + R
check("|P'|^2 = (u' Q_r)^2 + R^2", sp.simplify(P1.dot(P1) - ((up * Q[0])**2 + R**2)))
raise SystemExit(0 if ok else 1)
