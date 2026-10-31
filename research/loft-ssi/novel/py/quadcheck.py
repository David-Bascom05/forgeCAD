# Bernstein sign test of Q(S) w^2 on a span piece against brute-force sampling (cylinder r78 about z).
import numpy as np
from math import comb, pi, cos, sin
from loft import VOLUTE
from certify import bernstein_pts, arc_pts
A = np.diag([1.0, 1, 0]); bq = np.zeros(3); cq = -78.0**2; o = np.zeros(3)
def coefs(span, I, J):
    C, X, Y = VOLUTE.spans[span]
    cb, xb, yb = (bernstein_pts(M, *I) for M in (C, X, Y))
    ar = arc_pts(*J); w = [1, cos(0.5*(J[1]-J[0])), 1]
    h = [[(cb[i] + xb[i]*cx + yb[i]*sy - o)*w[j] for j, (cx, sy) in enumerate(ar)] for i in range(4)]
    out = np.zeros((7, 5))
    for i in range(4):
        for j in range(3):
            for k in range(4):
                for l in range(3):
                    v = h[k][l] @ A @ h[i][j] + 2*bq @ h[i][j]*w[l] + cq*w[j]*w[l]
                    out[i+k, j+l] += v*comb(3, i)*comb(3, k)*comb(2, j)*comb(2, l)/(comb(6, i+k)*comb(4, j+l))
    return out
for span in [3, 4, 5]:
    I = (VOLUTE.knots[span], VOLUTE.knots[span+1])
    for J in [(k*pi/2, (k+1)*pi/2) for k in range(4)]:
        cf = coefs(span, I, J)
        vals = [np.sum(VOLUTE.point(s, a, span)[:2]**2) - 78**2 for s in np.linspace(*I, 30) for a in np.linspace(*J, 30)]
        print(span, J[0], 'bernstein', cf.min() > 0 or cf.max() < 0, 'sampled sign change', min(vals) < 0 < max(vals))
