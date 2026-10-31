"""Exact loft x loft condition. For fixed (s, sigma), c1 + X1 u1 + Y1 w1 = c2 + X2 u2 + Y2 w2 is linear in
z = (u1, w1, u2, w2): M z = r, M = [X1 Y1 -X2 -Y2], r = c2 - c1. Its solutions are a line z = p0 + mu q
(q: signed 3x3 minors of M). On it u1^2+w1^2 = 1 and u2^2+w2^2 = 1 are two quadratics in mu; the
surfaces meet at (s, sigma) iff they share a root: F(s, sigma) = their Sylvester resultant.
A real common root gives a real point (z is linear in mu); two real quadratics share a non-real root only
when proportional, so the real zero set of F is the intersection up to isolated points."""
import numpy as np

def minors(M):
    """q with M q = 0: q_i = (-1)^i det(M without column i)."""
    return np.array([(-1)**i * np.linalg.det(np.delete(M, i, axis=1)) for i in range(4)])

def quads(A, s, B, t, j=None):
    c1, X1, Y1 = A.cxy(s); c2, X2, Y2 = B.cxy(t)
    M = np.array([X1, Y1, -X2, -Y2]).T; r = c2 - c1
    q = minors(M)
    j = int(np.argmax(np.abs(q))) if j is None else j
    # particular solution with z_j = 0, scaled by det(M_-j) = (-1)^j q_j: P = Cramer numerators
    cols = [k for k in range(4) if k != j]; Mj = M[:, cols]; dj = np.linalg.det(Mj)
    P = np.zeros(4)
    for n, k in enumerate(cols):
        Mk = Mj.copy(); Mk[:, n] = r; P[k] = np.linalg.det(Mk)
    # q_j z = P*(q_j/dj) + mu q  -> keep everything polynomial: z = (P + mu' q') / dj with q' = q * dj / q_j
    qq = q * dj / q[j]          # = +-q
    a1 = qq[0]**2 + qq[1]**2; b1 = 2*(P[0]*qq[0] + P[1]*qq[1]); k1 = P[0]**2 + P[1]**2 - dj*dj
    a2 = qq[2]**2 + qq[3]**2; b2 = 2*(P[2]*qq[2] + P[3]*qq[3]); k2 = P[2]**2 + P[3]**2 - dj*dj
    return (a1, b1, k1), (a2, b2, k2), P, qq, dj

def F(A, s, B, t, j=None):
    (a1, b1, k1), (a2, b2, k2), *_ = quads(A, s, B, t, j)
    return (a1*k2 - a2*k1)**2 - (a1*b2 - a2*b1)*(b1*k2 - b2*k1)

def lift(A, s, B, t):
    """The common point (and both angles) at a zero of F."""
    (a1, b1, k1), (a2, b2, k2), P, qq, dj = quads(A, s, B, t)
    mu = (a1*k2 - a2*k1) / (a2*b1 - a1*b2)
    z = (P + mu*qq) / dj
    c1, X1, Y1 = A.cxy(s)
    return c1 + X1*z[0] + Y1*z[1], np.arctan2(z[1], z[0]), np.arctan2(z[3], z[2])

if __name__ == '__main__':
    from loft import Loft, volute_profiles, VOLUTE
    rng = np.random.default_rng(1)
    up = [(c + np.array([0, 0, 8.0]), x, y, r) for c, x, y, r in volute_profiles([11.67, 13.89, 16.11, 18.33, 20.56, 22.78, 25, 28.5, 32, 33])]
    CUP = Loft(up)
    # F's value does not depend on the dropped column j beyond a power of q_j: find it
    for _ in range(3):
        s, t = rng.uniform(80, 150), rng.uniform(80, 150)
        q = minors(np.array([*VOLUTE.cxy(s)[1:], *[-v for v in CUP.cxy(t)[1:]]]).T)
        vals = [F(VOLUTE, s, CUP, t, j) for j in range(4)]
        print('F_j / q_j^m for m=0..6:', [[f'{v/abs(q[j])**m:.6e}' for j, v in enumerate(vals)] for m in (2, 4, 6)])
