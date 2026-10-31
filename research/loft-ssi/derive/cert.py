"""Reference implementation of the recipe: disjoint(wall1 span, wall2 span) for volute-class lofts.

Everything is built in the Bernstein basis straight from the section data (no power basis), with a
magnitude array carried beside every coefficient array: |rounding error| <= K * u * mag, where mag is
the same computation done on absolute values. A box is cleared only when a coefficient array has one
strict sign beyond that bound (Lean: bern3_pos, rounded_pos). This is what the Rust port follows.

Run: venv/bin/python cert.py
"""

import math
import numpy as np
from volute import Loft, OUTER, CORE

U = 2.0 ** -53
K = 256  # generous op-count factor; every coefficient is a short sum of products


def wbern(loft, j):
    """Each datum's weight on span j as 4 cubic Bernstein coefficients (derive/weights.py)."""
    kn, m = loft.knots, loft.m
    h = kn[j + 1] - kn[j]
    out = []
    for d in range(len(loft.secs)):
        yi, yj = float(d == j), float(d == j + 1)
        mi, mj = m[d][j], m[d][j + 1]
        out.append([yi, (2 * yi + yj) / 3 - h * h * (2 * mi + mj) / 18, (yi + 2 * yj) / 3 - h * h * (mi + 2 * mj) / 18, yj])
    return np.array(out)


def span_data(loft, j):
    """P (4x2), A (4x2), rho (4,): cubic Bernstein coefficients on span j."""
    w = wbern(loft, j)
    P = sum(np.outer(w[k], s[0]) for k, s in enumerate(loft.secs))
    A = sum(np.outer(w[k], s[2] * s[1]) for k, s in enumerate(loft.secs))
    rho = sum(w[k] * s[2] for k, s in enumerate(loft.secs))
    return P, A, rho


def bmul(p, q):
    """Product of 3D Bernstein arrays (degrees add per axis)."""
    out = np.zeros(tuple(p.shape[i] + q.shape[i] - 1 for i in range(3)))
    m = [s - 1 for s in p.shape]
    n = [s - 1 for s in q.shape]
    cm = [np.array([math.comb(m[i], a) for a in range(m[i] + 1)], float) for i in range(3)]
    cn = [np.array([math.comb(n[i], b) for b in range(n[i] + 1)], float) for i in range(3)]
    pp = p * cm[0][:, None, None] * cm[1][None, :, None] * cm[2][None, None, :]
    qq = q * cn[0][:, None, None] * cn[1][None, :, None] * cn[2][None, None, :]
    for idx in np.ndindex(pp.shape):
        out[idx[0]: idx[0] + qq.shape[0], idx[1]: idx[1] + qq.shape[1], idx[2]: idx[2] + qq.shape[2]] += pp[idx] * qq
    for i in range(3):
        c = np.array([math.comb(m[i] + n[i], k) for k in range(m[i] + n[i] + 1)], float)
        out /= c.reshape([-1 if a == i else 1 for a in range(3)])
    return out


def elevate(p, shape):
    """Degree-elevate a 3D Bernstein array to `shape`."""
    for ax in range(3):
        while p.shape[ax] < shape[ax]:
            n = p.shape[ax]  # degree n-1 -> n
            p = np.moveaxis(p, ax, 0)
            q = np.zeros((n + 1,) + p.shape[1:])
            for i in range(n + 1):
                if i < n:
                    q[i] += (1 - i / n) * p[i]
                if i > 0:
                    q[i] += (i / n) * p[i - 1]
            p = np.moveaxis(q, 0, ax)
    return p


def add(*xs):
    shape = tuple(max(x.shape[i] for x in xs) for i in range(3))
    return sum(elevate(x, shape) for x in xs)


def on(axis, coeffs):
    """A 1D Bernstein array placed along `axis` of a 3D array."""
    sh = [1, 1, 1]
    sh[axis] = len(coeffs)
    return np.asarray(coeffs, float).reshape(sh)


def build(w1, w2, absval=False):
    """Bernstein arrays of g (7,4,2) and Psi (19,7,3) over (tau1, tau2, u), c2 = 2u - 1."""
    (P1, A1, r1), (P2, A2, r2) = w1, w2
    f = np.abs if absval else (lambda x: x)
    sgn = (lambda x: abs(x)) if absval else (lambda x: x)
    a1 = [on(0, f(A1[:, i])) for i in range(2)]
    a2 = [on(1, f(A2[:, i])) for i in range(2)]
    # D_ij = P2_j - P1_i (partition of unity)
    D = [on(1, f(P2[:, i])) + (on(0, f(P1[:, i])) if absval else -on(0, P1[:, i])) for i in range(2)]
    c2 = on(2, [1.0, 1.0] if absval else [-1.0, 1.0])
    neg = (lambda x: x) if absval else (lambda x: -x)
    cross = lambda u, v: add(bmul(u[0], v[1]), neg(bmul(u[1], v[0])))
    dot = lambda u, v: add(bmul(u[0], v[0]), bmul(u[1], v[1]))
    E = [add(D[i], bmul(c2, a2[i])) for i in range(2)]
    g = cross(a1, E)
    R1 = bmul(on(0, f(r1)), on(0, f(r1)))
    R2 = bmul(on(1, f(r2)), on(1, f(r2)))
    d = dot(a1, E)
    n = dot(a1, a1)
    one_c2 = on(2, [0.0, 2.0, 0.0])  # 1 - c2^2 = 4u(1-u)
    psi = add(bmul(R1, bmul(d, d)), bmul(bmul(n, n), add(bmul(R2, one_c2), neg(R1))))
    return g, psi


def halve(b, axis):
    b = np.moveaxis(b, axis, 0)
    n = b.shape[0] - 1
    lo, hi, cur = [b[0]], [b[n]], b
    for _ in range(n):
        cur = 0.5 * (cur[:-1] + cur[1:])
        lo.append(cur[0])
        hi.append(cur[-1])
    return np.moveaxis(np.array(lo), 0, axis), np.moveaxis(np.array(hi[::-1]), 0, axis)


def clear(b, mag):
    e = K * U * mag.max()
    return b.min() > e or b.max() < -e


def certify(arrs, depth, st, maxd=36):
    """arrs = (g, gmag, psi, psimag). True if no common zero of g and Psi on the box."""
    st["boxes"] += 1
    g, gm, p, pm = arrs
    if clear(g, gm) or clear(p, pm):
        return True
    if depth == maxd:
        return False
    ax = depth % 3
    halves = [halve(x, ax) for x in arrs]
    return all(certify(tuple(h[i] for h in halves), depth + 1, st, maxd) for i in range(2))


def disjoint(L1, j1, L2, j2, st):
    w1, w2 = span_data(L1, j1), span_data(L2, j2)
    g, p = build(w1, w2)
    gm, pm = build(w1, w2, absval=True)
    return certify((g, gm, p, pm), 0, st)


if __name__ == "__main__":
    V, C = Loft(OUTER), Loft(CORE)
    total, bad = 0, []
    for j1 in range(len(V.knots) - 1):
        row = []
        for j2 in range(len(C.knots) - 1):
            st = {"boxes": 0}
            ok = disjoint(V, j1, C, j2, st)
            total += st["boxes"]
            row.append(f"{st['boxes']:3d}{'' if ok else '!'}")
            if not ok:
                bad.append((j1, j2))
        print(j1, " ".join(row))
    print("boxes", total, "not certified", bad)
