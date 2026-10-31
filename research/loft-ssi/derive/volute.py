"""Volute loft x core loft: the exact span-pair polynomial F(s1, s2) (bidegree 12, 12).

Run: venv/bin/python volute.py   (numpy only; offline)

Both lofts are sweeps of ellipses S = P(s) + cos(phi) A(s) - sin(phi) rho(s) e_z with P, A
horizontal cubics and rho a scalar cubic per span (every section frame has y = -e_z, centre on z = 0).
Two of them meet exactly where (s1, s2) solves
    F = rho1^2 (W^2 - (D^A2)^2) - rho2^2 (W^2 - (D^A1)^2) = 0,   W = A1^A2,  D = P2(s2) - P1(s1)
with cos(phi1) = (D^A2)/W, cos(phi2) = (D^A1)/W, and the real part where W^2 >= (D^A2)^2.
"""

import math
import numpy as np
from numpy.polynomial import polynomial as npoly


def section(deg, R, r):
    """A volute section in the meridional plane at angle deg: centre, x axis, radius."""
    t = math.radians(deg)
    return (np.array([R * math.cos(t), R * math.sin(t)]), np.array([math.cos(t), math.sin(t)]), r)


def straight(x, y, r):
    return (np.array([x, y]), np.array([1.0, 0.0]), r)


ANG = [60, 110, 160, 210, 260, 310, 360]
RAD = [87.17, 89.81, 92.44, 95.08, 97.72, 100.4, 103]
OUTER = [section(a, R, r) for a, R, r in zip(ANG, RAD, [16.67, 18.89, 21.11, 23.33, 25.56, 27.78, 30])]
OUTER += [straight(103, 55, 33.5), straight(103, 110, 37)]
CORE = [section(a, R, r) for a, R, r in zip(ANG, RAD, [11.67, 13.89, 16.11, 18.33, 20.56, 22.78, 25])]
CORE += [straight(103, 55, 28.5), straight(103, 110, 32), straight(103, 125, 33)]


def natural_m(knots):
    """M[d][i]: second derivative at knot i of the natural cubic spline through unit datum d."""
    n = len(knots)
    h = np.diff(knots)
    a = np.zeros((n, n))
    a[0, 0] = a[n - 1, n - 1] = 1.0
    for i in range(1, n - 1):
        a[i, i - 1], a[i, i], a[i, i + 1] = h[i - 1], 2 * (h[i - 1] + h[i]), h[i]
    m = []
    for d in range(n):
        y = np.eye(n)[d]
        rhs = np.zeros(n)
        for i in range(1, n - 1):
            rhs[i] = 6 * ((y[i + 1] - y[i]) / h[i] - (y[i] - y[i - 1]) / h[i - 1])
        m.append(np.linalg.solve(a, rhs))
    return np.array(m)


class Loft:
    def __init__(self, secs):
        self.secs = secs
        c = [s[0] for s in secs]
        self.knots = np.concatenate([[0.0], np.cumsum([np.linalg.norm(c[i + 1] - c[i]) for i in range(len(c) - 1)])])
        self.m = natural_m(self.knots)

    def weights(self, j):
        """Each datum's weight on span j as a polynomial in the local tau = (s - k_j)/h (ascending coeffs)."""
        h = self.knots[j + 1] - self.knots[j]
        a, b = np.array([1.0, -1.0]), np.array([0.0, 1.0])
        a3, b3 = npoly.polypow(a, 3), npoly.polypow(b, 3)
        out = []
        for d in range(len(self.secs)):
            w = np.zeros(4)
            w[: 2] += a * (d == j) + b * (d == j + 1)
            w += h * h / 6 * (self.m[d][j] * npoly.polysub(a3, a)[:4] + self.m[d][j + 1] * npoly.polysub(b3, b)[:4])
            out.append(w)
        return out

    def span(self, j):
        """P (2 polys), A (2 polys), rho on span j, in tau."""
        w = self.weights(j)
        P = [sum(wk * s[0][i] for wk, s in zip(w, self.secs)) for i in range(2)]
        A = [sum(wk * s[2] * s[1][i] for wk, s in zip(w, self.secs)) for i in range(2)]
        rho = sum(wk * s[2] for wk, s in zip(w, self.secs))
        return P, A, rho


def outer(p, q):
    """Bivariate product of a poly in tau1 (p) and a poly in tau2 (q): 2D coefficient array [i1, i2]."""
    return np.outer(p, q)


def lift1(p):
    return np.asarray(p, float).reshape(-1, 1)


def lift2(q):
    return np.asarray(q, float).reshape(1, -1)


def mul(a, b):
    out = np.zeros((a.shape[0] + b.shape[0] - 1, a.shape[1] + b.shape[1] - 1))
    for i in range(a.shape[0]):
        for j in range(a.shape[1]):
            if a[i, j] != 0.0:
                out[i : i + b.shape[0], j : j + b.shape[1]] += a[i, j] * b
    return out


def add(*xs):
    r = max(x.shape[0] for x in xs)
    c = max(x.shape[1] for x in xs)
    out = np.zeros((r, c))
    for x in xs:
        out[: x.shape[0], : x.shape[1]] += x
    return out


def neg(x):
    return -x


def F_span(L1, j1, L2, j2):
    """F and G1 = W^2 - (D^A2)^2 as 2D power-basis arrays in (tau1, tau2) on span pair (j1, j2)."""
    P1, A1, r1 = L1.span(j1)
    P2, A2, r2 = L2.span(j2)
    a1 = [lift1(x) for x in A1]
    a2 = [lift2(x) for x in A2]
    D = [add(lift2(P2[i]), neg(lift1(P1[i]))) for i in range(2)]
    cross = lambda u, v: add(mul(u[0], v[1]), neg(mul(u[1], v[0])))
    W = cross(a1, a2)
    DA2 = cross(D, a2)
    DA1 = cross(D, a1)
    G1 = add(mul(W, W), neg(mul(DA2, DA2)))
    G2 = add(mul(W, W), neg(mul(DA1, DA1)))
    R1 = lift1(npoly.polymul(r1, r1))
    R2 = lift2(npoly.polymul(r2, r2))
    F = add(mul(R1, G1), neg(mul(R2, G2)))
    return F, G1, W, DA1, DA2


def bernstein(c):
    """Power-basis 2D coefficients on [0,1]^2 to Bernstein coefficients (same degrees)."""
    def conv(n):
        # b_k = sum_{i<=k} C(k,i)/C(n,i) a_i
        T = np.zeros((n + 1, n + 1))
        for k in range(n + 1):
            for i in range(k + 1):
                T[k, i] = math.comb(k, i) / math.comb(n, i)
        return T
    n1, n2 = c.shape[0] - 1, c.shape[1] - 1
    return conv(n1) @ c @ conv(n2).T


def evalF(c, t1, t2):
    return npoly.polyval2d(t1, t2, c)


if __name__ == "__main__":
    V, C = Loft(OUTER), Loft(CORE)
    print("outer knots", np.round(V.knots, 4))
    print("core  knots", np.round(C.knots, 4))
    np.set_printoptions(linewidth=160)
    for j1 in range(len(V.knots) - 1):
        row = []
        for j2 in range(len(C.knots) - 1):
            F, G1, W, _, _ = F_span(V, j1, C, j2)
            b = bernstein(F)
            g = np.linspace(0, 1, 121)
            T1, T2 = np.meshgrid(g, g, indexing="ij")
            fv = evalF(F, T1, T2)
            gv = evalF(G1, T1, T2)
            # real sign change of F where G1 >= 0, on the grid
            s = np.sign(fv)
            ch = (s[1:, :] != s[:-1, :]) & (gv[1:, :] >= 0) | False
            real = bool(np.any(ch))
            tag = ("+" if b.min() > 0 else "-" if b.max() < 0 else "?") + ("R" if real else ".")
            row.append(tag)
        print(j1, " ".join(row))
