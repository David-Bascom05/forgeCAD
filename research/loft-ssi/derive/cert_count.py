"""Operation counts for cert.py, under the rules of proofs/Proofs/Rounding.lean.

Re-runs cert.py's computation with a number type carrying (f64 value, count) and following the
`Ex` rules op for op: inputs are `lit` (count 0), integer binomials are `lit`, `x / c` is `divc`
(+1), add/sub take the larger count +1, mul adds counts +1, unary minus and `0.5 * x` are exact
(`neg`, `half`), and the degree-elevation weights 1 - i/n and i/n are `approx` leaves whose count
is the least k with |xh - x| <= (theta_k - 1)|x|, computed in exact rationals. The values are
checked bit for bit against cert.py, so the Lean bound applies to the tree that was evaluated.

It then runs the volute x core certification (cert.py's main loop) and, for every cleared box,
records the count of the tensor that cleared it. `Ex.clear_pos_of_le` / `bern3_clear_pos` justify
a clearance with K = 256 when K u >= c u / (1 - 2 c u), i.e. c <= K / (1 + 2 K u).

Run from derive/: venv/bin/python cert_count.py
"""

import math
from fractions import Fraction

import numpy as np

import cert
from volute import Loft, OUTER, CORE

U = Fraction(1, 2 ** 53)
K = cert.K
C_MAX = math.floor(Fraction(K) / (1 + 2 * K * U))  # largest count the rule covers


class T:
    """An f64 value and the Ex count of the tree that computed it."""

    __slots__ = ("v", "c")

    def __init__(self, v, c):
        self.v, self.c = float(v), c

    def __add__(self, o):
        o = lit(o)
        return NotImplemented if o is None else T(self.v + o.v, max(self.c, o.c) + 1)

    def __radd__(self, o):
        o = lit(o)
        return NotImplemented if o is None else T(o.v + self.v, max(self.c, o.c) + 1)

    def __sub__(self, o):
        o = lit(o)
        return NotImplemented if o is None else T(self.v - o.v, max(self.c, o.c) + 1)

    def __mul__(self, o):
        o = lit(o)
        return NotImplemented if o is None else T(self.v * o.v, self.c + o.c + 1)

    def __rmul__(self, o):
        o = lit(o)
        return NotImplemented if o is None else T(o.v * self.v, self.c + o.c + 1)

    def __truediv__(self, c):
        assert isinstance(c, (float, int, np.floating, np.integer)) and c != 0
        return T(self.v / float(c), self.c + 1)  # divc by an exact constant

    def __neg__(self):
        return T(-self.v, self.c)


def lit(o):
    if isinstance(o, T):
        return o
    if isinstance(o, (float, int, np.floating, np.integer)):
        return T(o, 0)
    return None


def half(x):
    return T(0.5 * x.v, x.c)


def approx_k(xh, x):
    """Least k with |xh - x| <= (theta_k - 1)|x|, theta_k = 1/(1 - k u)."""
    rel = abs(Fraction(xh) - x) / abs(x)
    k = math.ceil(rel / (U * (1 + rel)))
    assert k * U / (1 - k * U) >= rel and (k == 0 or (k - 1) * U / (1 - (k - 1) * U) < rel)
    return k


def approx(xh, x):
    return T(xh, approx_k(xh, x))


ZERO = T(0.0, 0)
vals = np.vectorize(lambda t: t.v, otypes=[float])
cnts = np.vectorize(lambda t: t.c, otypes=[int])
halfv = np.vectorize(half, otypes=[object])
APPROX_K = {}


def obj(shape, fill=ZERO):
    a = np.empty(shape, dtype=object)
    a.fill(fill)
    return a


def bmul(p, q):
    out = obj(tuple(p.shape[i] + q.shape[i] - 1 for i in range(3)))
    m = [s - 1 for s in p.shape]
    n = [s - 1 for s in q.shape]
    cm = [np.array([math.comb(m[i], a) for a in range(m[i] + 1)], float) for i in range(3)]
    cn = [np.array([math.comb(n[i], b) for b in range(n[i] + 1)], float) for i in range(3)]
    pp = p * cm[0][:, None, None] * cm[1][None, :, None] * cm[2][None, None, :]
    qq = q * cn[0][:, None, None] * cn[1][None, :, None] * cn[2][None, None, :]
    for idx in np.ndindex(pp.shape):
        sl = (slice(idx[0], idx[0] + qq.shape[0]), slice(idx[1], idx[1] + qq.shape[1]),
              slice(idx[2], idx[2] + qq.shape[2]))
        out[sl] = out[sl] + pp[idx] * qq
    for i in range(3):
        c = np.array([math.comb(m[i] + n[i], k) for k in range(m[i] + n[i] + 1)], float)
        out = out / c.reshape([-1 if a == i else 1 for a in range(3)])
    return out


def elevate(p, shape):
    for ax in range(3):
        while p.shape[ax] < shape[ax]:
            n = p.shape[ax]
            p = np.moveaxis(p, ax, 0)
            q = obj((n + 1,) + p.shape[1:])
            for i in range(n + 1):
                if i < n:
                    w = approx(1 - i / n, 1 - Fraction(i, n))
                    APPROX_K[f"1-{i}/{n}"] = w.c
                    q[i] = q[i] + w * p[i]
                if i > 0:
                    w = approx(i / n, Fraction(i, n))
                    APPROX_K[f"{i}/{n}"] = w.c
                    q[i] = q[i] + w * p[i - 1]
            p = np.moveaxis(q, 0, ax)
    return p


def add(*xs):
    shape = tuple(max(x.shape[i] for x in xs) for i in range(3))
    return sum(elevate(x, shape) for x in xs)


def on(axis, coeffs):
    sh = [1, 1, 1]
    sh[axis] = len(coeffs)
    return np.array([T(c, 0) for c in np.asarray(coeffs, float)], dtype=object).reshape(sh)


def build(w1, w2, absval=False):
    """cert.build, op for op, on tracked numbers."""
    (P1, A1, r1), (P2, A2, r2) = w1, w2
    f = np.abs if absval else (lambda x: x)
    a1 = [on(0, f(A1[:, i])) for i in range(2)]
    a2 = [on(1, f(A2[:, i])) for i in range(2)]
    D = [on(1, f(P2[:, i])) + (on(0, f(P1[:, i])) if absval else -on(0, P1[:, i]))
         for i in range(2)]
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
    one_c2 = on(2, [0.0, 2.0, 0.0])
    psi = add(bmul(R1, bmul(d, d)), bmul(bmul(n, n), add(bmul(R2, one_c2), neg(R1))))
    return g, psi


def halve(b, axis):
    b = np.moveaxis(b, axis, 0)
    n = b.shape[0] - 1
    lo, hi, cur = [b[0]], [b[n]], b
    for _ in range(n):
        cur = halfv(cur[:-1] + cur[1:])
        lo.append(cur[0])
        hi.append(cur[-1])
    return np.moveaxis(np.array(lo), 0, axis), np.moveaxis(np.array(hi[::-1]), 0, axis)


def certify(arrs, depth, st, maxd=36):
    st["boxes"] += 1
    st["maxdepth"] = max(st["maxdepth"], depth)
    g, gm, p, pm = arrs
    for name, b, m in (("g", g, gm), ("psi", p, pm)):
        if cert.clear(vals(b), vals(m)):
            c = int(cnts(b).max())
            assert c == int(cnts(m).max())
            st["cleared"].append((name, depth, c))
            return True
    if depth == maxd:
        return False
    ax = depth % 3
    halves = [halve(x, ax) for x in arrs]
    return all(certify(tuple(h[i] for h in halves), depth + 1, st, maxd) for i in range(2))


def justified_depth(c0, degs):
    """Largest depth d whose count bound c0 + sum of halved-axis degrees stays <= C_MAX."""
    c, d = c0, 0
    while c + degs[d % 3] <= C_MAX:
        c += degs[d % 3]
        d += 1
    return d


def main():
    V, C = Loft(OUTER), Loft(CORE)
    st = {"boxes": 0, "maxdepth": 0, "cleared": []}
    c0 = {"g": 0, "psi": 0}
    degs = None
    bad = []
    for j1 in range(len(V.knots) - 1):
        for j2 in range(len(C.knots) - 1):
            w1, w2 = cert.span_data(V, j1), cert.span_data(C, j2)
            g, p = build(w1, w2)
            gm, pm = build(w1, w2, absval=True)
            fg, fp = cert.build(w1, w2)
            fgm, fpm = cert.build(w1, w2, absval=True)
            for t, ref in ((g, fg), (p, fp), (gm, fgm), (pm, fpm)):
                assert np.array_equal(vals(t), ref), "tracked values differ from cert.py"
            assert np.array_equal(cnts(g), cnts(gm)) and np.array_equal(cnts(p), cnts(pm))
            c0["g"] = max(c0["g"], int(cnts(g).max()))
            c0["psi"] = max(c0["psi"], int(cnts(p).max()))
            degs = {"g": [s - 1 for s in g.shape], "psi": [s - 1 for s in p.shape]}
            if not certify((g, gm, p, pm), 0, st):
                bad.append((j1, j2))

    print("certified surface: the lofts whose span data P, A, rho are cert.span_data's f64 values")
    print("approx weight counts k:", dict(sorted(APPROX_K.items())))
    print(f"K = {K}, u = 2^-53: rule justified for count c <= {C_MAX}")
    for name in ("g", "psi"):
        print(f"{name}: depth-0 max count {c0[name]}, degrees {degs[name]} "
              f"(count +{degs[name][0]}/+{degs[name][1]}/+{degs[name][2]} per halving on axis "
              f"0/1/2), K justified to depth {justified_depth(c0[name], degs[name])}")
    cl = st["cleared"]
    worst = max(c for _, _, c in cl)
    print(f"boxes {st['boxes']}, not certified {bad}, max depth reached {st['maxdepth']}")
    print(f"cleared boxes {len(cl)}: by g {sum(n == 'g' for n, _, _ in cl)}, "
          f"by psi {sum(n == 'psi' for n, _, _ in cl)}")
    print(f"max cleared depth {max(d for _, d, _ in cl)}, max count among clearances {worst}")
    for name in ("g", "psi"):
        sub = [(d, c) for n, d, c in cl if n == name]
        if sub:
            print(f"  {name}: max depth {max(d for d, _ in sub)}, max count {max(c for _, c in sub)}")
    ok = all(c <= C_MAX for _, _, c in cl)
    print("every clearance satisfies K u >= c u / (1 - 2 c u):", ok)


if __name__ == "__main__":
    main()
