#!/usr/bin/env python3
"""A corner patch's ribbon R(s, d) = P(s) + T(s) d + K(s) d^2 at s(u, v), d(u, v): the second-order jet
`Scalar for J`'s `ribbon` writes out, against differentiating the composition directly."""
import sympy as sp

u, w = sp.symbols("u w")
S, D = sp.Function("s")(u, w), sp.Function("d")(u, w)
x = sp.Symbol("x")
P, T, K = sp.Function("P"), sp.Function("T"), sp.Function("K")
R = P(S) + T(S) * D + K(S) * D**2
rs = P(S).diff(S) + T(S).diff(S) * D + K(S).diff(S) * D**2
rd = T(S) + 2 * K(S) * D
rss = P(S).diff(S, 2) + T(S).diff(S, 2) * D + K(S).diff(S, 2) * D**2  # K'' = 0: K is linear in s
rsd = T(S).diff(S) + 2 * K(S).diff(S) * D
rdd = 2 * K(S)
second = lambda a, b: rss * S.diff(a) * S.diff(b) + rsd * (S.diff(a) * D.diff(b) + S.diff(b) * D.diff(a)) + rdd * D.diff(a) * D.diff(b)
for a in (u, w):
    assert sp.simplify(R.diff(a) - (rs * S.diff(a) + rd * D.diff(a))) == 0
for a, b in ((u, u), (u, w), (w, w)):
    assert sp.simplify(R.diff(a, b) - (second(a, b) + rs * S.diff(a, b) + rd * D.diff(a, b))) == 0
print("ribbon jet: every first and second partial agrees")
