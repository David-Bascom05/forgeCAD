"""Assumption checks that are not in Lean (Lean: proofs/Proofs/Minimal.lean has the rest).

Unequal arc sweeps: blending an arc of sweep pi with one of sweep pi/2 (weights 1/2) gives a
section that lies on no conic, so it is not of the form c + cos(phi) X + sin(phi) Y and neither
F nor the volute formula applies. Exact: six points of the (analytically continued) section,
t = 0, 1/3, ..., 5/3, and the 6x6 conic determinant is nonzero. (A conic through the section on
[0, 1] would vanish on the whole continued curve: a trigonometric polynomial zero on an interval.)

Run: venv/bin/python minimal.py
"""
import sympy as sp

t = sp.symbols('t')
x = (sp.cos(sp.pi * t) + sp.cos(sp.pi * t / 2)) / 2
y = (sp.sin(sp.pi * t) + sp.sin(sp.pi * t / 2)) / 2
pts = [(sp.nsimplify(x.subs(t, sp.Rational(k, 3))), sp.nsimplify(y.subs(t, sp.Rational(k, 3))))
       for k in range(6)]
M = sp.Matrix([[a * a, a * b, b * b, a, b, 1] for a, b in pts])
d = sp.simplify(M.det())
print("conic determinant through 6 section points:", d, "=", sp.N(d))
assert d != 0
print("ok: unequal sweeps give a non-conic section")
