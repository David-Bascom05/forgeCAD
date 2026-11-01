"""The used sheet of a spindle torus as a sign test, checked symbolically.

Run: python3 research/spindle-priorart/sheet.py (needs sympy).

Torus {frame, major R (signed), minor r}: p(u, v) = (R + r cos v)(cos u, sin u, 0) + r sin v z.
ForgeCAD uses the part between the poles +-pole_v = acos(-R/r), i.e. where s = R + r cos v > 0.
"""
import sympy as sp

R, r, u, v, t = sp.symbols('R r u v t', real=True)
s = R + r * sp.cos(v)
p = sp.Matrix([s * sp.cos(u), s * sp.sin(u), r * sp.sin(v)])
rho2 = p[0] ** 2 + p[1] ** 2
S = p.dot(p) + R ** 2 - r ** 2

# 1. S = 2 R s: off the apices the sign of S (a quadric) tells the sheet; used <=> R*S > 0
assert sp.simplify(S - 2 * R * s) == 0
# 2. the quartic is (S - 2R rho)(S + 2R rho): each factor one sheet (rho = |s| >= 0)
x, y, z = sp.symbols('x y z', real=True)
Sx = x**2 + y**2 + z**2 + R**2 - r**2
quartic = Sx**2 - 4 * R**2 * (x**2 + y**2)
assert sp.simplify(quartic.subs({x: p[0], y: p[1], z: p[2]})) == 0
# 3. the sheets meet only where s = 0: rho = 0, z^2 = r^2 - R^2 (the two apices)
# on s = 0, cos v = -R/r, so z^2 = r^2 sin^2 v = r^2 - R^2 and rho = |s| = 0
assert sp.simplify(((r * sp.sin(v)) ** 2).subs(sp.sin(v) ** 2, 1 - sp.cos(v) ** 2).subs(sp.cos(v), -R / r) - (r**2 - R**2)) == 0
# 4. half-angle t = tan(v/2): s > 0 <=> (R + r) + (R - r) t^2 > 0, for apple (0 < R < r) and lemon (-r < R < 0)
cv = (1 - t**2) / (1 + t**2)
assert sp.simplify((R + r * cv) * (1 + t**2) - ((R + r) + (R - r) * t**2)) == 0
# 5. horn (R = r): s = r (1 + cos v) >= 0, so the other sheet is only the apex (v = pi)
assert sp.simplify(s.subs(R, r) - r * (1 + sp.cos(v))) == 0
print('sheet identities hold: S = 2 R s; quartic = (S-2R rho)(S+2R rho); sheets meet at the apices only;'
      ' used <=> (R+r) + (R-r) tan^2(v/2) > 0; horn: other sheet is the apex')
