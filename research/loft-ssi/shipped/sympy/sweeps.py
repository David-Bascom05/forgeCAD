"""Unequal sweeps break the (3, 2) form: a counterexample, in exact arithmetic (SymPy).

Two circle sections, both the unit circle about the origin, starts 0, sweeps pi and pi/2. Where
their spline weights are (1/2, 1/2) the section is f(t) = (C1(t) + C2(t)) / 2, in complex form
f = (zeta^2 + zeta) / 2 with zeta = exp(i pi t / 2). If the span were a rational Bezier patch of
degree 2 in its angle direction, every s = const section would lie on a conic Q(x, y) = 0. Putting
x = (f + conj f)/2, y = (f - conj f)/(2i), conj zeta = 1/zeta into a general conic gives a Laurent
polynomial in zeta that must vanish identically: its coefficients force Q = 0, so no conic exists.
With EQUAL sweeps (different starts or signs are fine, Lean `arc_any_start`) the same test finds a
conic.  Run: python3 sweeps.py  (needs sympy)
"""
from sympy import I, Poly, Rational, exp, expand, linsolve, symbols

z = symbols("zeta")
a = symbols("a0:6")

def conics_through(f, fbar):
    x = (f + fbar) / 2
    y = (f - fbar) / (2 * I)
    q = a[0] * x * x + a[1] * x * y + a[2] * y * y + a[3] * x + a[4] * y + a[5]
    num = expand(q * z ** 8)
    eqs = Poly(num, z).coeffs()
    return linsolve(eqs, a)

f = (z ** 2 + z) / 2
fbar = (z ** -2 + z ** -1) / 2
print("sweeps pi and pi/2, conics through the section:", conics_through(f, fbar))

w = exp(I * Rational(7, 10))
g = (z ** 2 + w * z ** 2) / 2
gbar = (z ** -2 + z ** -2 / w) / 2
print("equal sweeps pi, starts 0 and 0.7:", conics_through(g, gbar))
