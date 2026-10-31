"""Certified exclusion for two ringed (circle-section) loft faces by exact rational Bezier hulls.
A piece (s in I, a in J) of S = c(s) + X(s) cos a + Y(s) sin a is the rational (3,2) Bezier patch with
control points c_i + X_i cx_j + Y_i sy_j (c_i, X_i, Y_i the cubics' Bernstein points over I; (cx_j, sy_j)
and weights (1, cos h, 1) the arc's over J), so it lies in their convex hull (weights > 0)."""
import numpy as np
from math import cos, sin, pi
from loft import VOLUTE, CORE

B3 = None
def bernstein_pts(P, lo, hi):
    """Power-basis cubic coefficients P (4,3) -> Bernstein control points over [lo,hi]."""
    # reparametrise s = lo + (hi-lo) t, then power->Bernstein
    h = hi - lo
    Q = np.zeros((4, 3))
    for k in range(4):  # coefficient of t^k
        for j in range(k, 4):
            from math import comb
            Q[k] += P[j] * comb(j, k) * lo**(j-k) * h**k
    # power -> Bernstein: b_i = sum_{k<=i} C(i,k)/C(3,k) q_k
    from math import comb
    return np.array([sum(comb(i, k)/comb(3, k)*Q[k] for k in range(i+1)) for i in range(4)])

def arc_pts(a0, a1):
    m, h = 0.5*(a0+a1), 0.5*(a1-a0)
    return [(cos(a0), sin(a0)), (cos(m)/cos(h), sin(m)/cos(h)), (cos(a1), sin(a1))]

STATS = {'hull': 0, 'sep': 0}
def hull(loft, span, I, J):
    C, X, Y = loft.spans[span]
    cb, xb, yb = (bernstein_pts(M, *I) for M in (C, X, Y))
    ar = arc_pts(*J)
    STATS['hull'] += 1
    return np.array([cb[i] + xb[i]*cx + yb[i]*sy for i in range(4) for (cx, sy) in ar])

def separated(P, Q, extra_axes):
    STATS['sep'] += 1
    for ax in [np.eye(3)[0], np.eye(3)[1], np.eye(3)[2]] + extra_axes:
        p, q = P @ ax, Q @ ax
        if p.max() < q.min() or q.max() < p.min():
            return True
    return False

def normal(loft, span, I, J):
    s, a = 0.5*(I[0]+I[1]), 0.5*(J[0]+J[1])
    C, X, Y = loft.spans[span]; pw = np.array([1, s, s*s, s**3]); dp = np.array([0, 1, 2*s, 3*s*s])
    du = -(pw @ X)*sin(a) + (pw @ Y)*cos(a); dv = dp @ C + (dp @ X)*cos(a) + (dp @ Y)*sin(a)
    n = np.cross(du, dv); return n/np.linalg.norm(n)

def certify(A, ia, Ia, Ja, B, ib, Ib, Jb, depth=0, maxdepth=14):
    """True if the pieces provably miss; False if it ran out of depth (a candidate)."""
    P, Q = hull(A, ia, Ia, Ja), hull(B, ib, Ib, Jb)
    axes = [normal(A, ia, Ia, Ja), normal(B, ib, Ib, Jb)]
    d = Q.mean(0) - P.mean(0)
    if np.linalg.norm(d) > 0: axes.append(d/np.linalg.norm(d))
    if separated(P, Q, axes): return True
    if depth >= maxdepth: return False
    # split the piece with the bigger hull, along its longer side
    def size(Pts): return np.ptp(Pts, axis=0).max()
    def halves(L, i, I, J):
        ls = np.linalg.norm(L.point(I[1], 0.5*sum(J), i) - L.point(I[0], 0.5*sum(J), i))
        la = np.linalg.norm(L.point(0.5*sum(I), J[1], i) - L.point(0.5*sum(I), J[0], i))
        if ls > la: m = 0.5*sum(I); return [((I[0], m), J), ((m, I[1]), J)]
        m = 0.5*sum(J); return [(I, (J[0], m)), (I, (m, J[1]))]
    if size(P) >= size(Q):
        return all(certify(A, ia, I2, J2, B, ib, Ib, Jb, depth+1, maxdepth) for I2, J2 in halves(A, ia, Ia, Ja))
    return all(certify(A, ia, Ia, Ja, B, ib, I2, J2, depth+1, maxdepth) for I2, J2 in halves(B, ib, Ib, Jb))

if __name__ == '__main__':
    import time
    quarters = [(k*pi/2, (k+1)*pi/2) for k in range(4)]
    t0 = time.time(); fails = 0; pairs = 0; worst = 0
    for ia in range(len(VOLUTE.spans)):
        for ib in range(len(CORE.spans)):
            before = STATS['sep']
            ok = True
            for Ja in quarters:
                for Jb in quarters:
                    Ia = (VOLUTE.knots[ia], VOLUTE.knots[ia+1]); Ib = (CORE.knots[ib], CORE.knots[ib+1])
                    pairs += 1
                    if not certify(VOLUTE, ia, Ia, Ja, CORE, ib, Ib, Jb):
                        ok = False; fails += 1
            n = STATS['sep'] - before; worst = max(worst, n)
            print(f'volute span {ia} x core span {ib}: {"separated" if ok else "CANDIDATE"} with {n} hull tests')
    print('quarter pairs', pairs, 'unresolved', fails, 'hull tests', STATS['sep'], 'worst span pair', worst, 'time', time.time()-t0)
