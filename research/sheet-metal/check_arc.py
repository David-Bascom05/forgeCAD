#!/usr/bin/env python3
"""What circles need across a bend. Run: python check_arc.py (sympy).

A flat circle is an ellipse in (phi, y); its wall a right conoid; a wall's area element; and the claim that
meeting a circle with a line, parabola or circle stays "quartic at most" by the half-angle substitution, so
wrap_wall's closed form carries over."""
import sympy as sp

a, Rn, x0, y0, rho, m = sp.symbols("a R_n x0 y0 rho m", positive=True)
u, phi, z = sp.symbols("u phi z", real=True)

# 1. ellipse in (phi, y), semi-axes a / Rn and a
ph, y = (x0 + a * sp.cos(u)) / Rn, y0 + a * sp.sin(u)
assert sp.simplify(((ph - x0 / Rn) * Rn / a) ** 2 + ((y - y0) / a) ** 2) == 1
print("PASS circle in the flat is an ellipse in (phi, y): semi-axes a/Rn, a")

# 2. wall area element: S(phi, rho) = (rho sin phi, y(phi), R - rho cos phi)
f = sp.Function("y")(phi)
R = sp.Symbol("R", positive=True)
S = sp.Matrix([rho * sp.sin(phi), f, R - rho * sp.cos(phi)])
n = S.diff(phi).cross(S.diff(rho))
assert sp.simplify(n.dot(n) - (rho**2 + f.diff(phi) ** 2)) == 0
print("PASS wall area element: |S_phi x S_rho| = sqrt(rho^2 + y'(phi)^2)")
assert sp.simplify(S.diff(rho)[1]) == 0
print("PASS rulings radial and square to the axis: a right conoid, whatever the flat curve")

# 3. half angle: a circle's point is rational in z = tan(u/2), of degree 2
c, s = (1 - z**2) / (1 + z**2), 2 * z / (1 + z**2)
A, B, C, D = sp.symbols("A B C D", real=True)
line = sp.numer(sp.together(A * (x0 + a * c) + B * (y0 + a * s) + C))
assert sp.Poly(line, z).degree() == 2
print("PASS circle against a line: quadratic in tan(u/2)")
X0, Y0, r2 = sp.symbols("X0 Y0 r2", real=True)
circ = sp.numer(sp.together((x0 + a * c - X0) ** 2 + (y0 + a * s - Y0) ** 2 - r2))
print("     circle against a circle: degree", sp.Poly(circ, z).degree(), "in tan(u/2) (the z^4 term goes with the squares: at most 2 real)")
k = sp.Symbol("k", positive=True)
# a parabola Xp = k Yp^2 in any frame: linear forms of the circle's point
L1 = A * (x0 + a * c) + B * (y0 + a * s) + C
L2 = -B * (x0 + a * c) + A * (y0 + a * s) + D
par = sp.numer(sp.together(L1 - k * L2**2))
d = sp.Poly(par, z).degree()
assert d == 4
print("PASS circle against a parabola: degree", d, "in tan(u/2): quartic at most")
print("     u = pi is z = infinity: test that point on its own, or start the arc's parameter elsewhere")

# 4. the closed form the prototype checks volume against
R0, t, K, A_zone = sp.symbols("R0 t K A_z", positive=True)
Rn2 = R0 + K * t
w = sp.Symbol("w", positive=True)
V = sp.integrate(A_zone * (R0 + w) / Rn2, (w, 0, t))
assert sp.simplify(V - t * A_zone * (R0 + t / 2) / Rn2) == 0
print("PASS volume in a bend zone: t A_zone (R + t/2) / Rn, any normal-cut region")
