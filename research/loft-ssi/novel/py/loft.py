"""Volute / core lofts as ringed surfaces S(s,a) = c(s) + X(s) cos a + Y(s) sin a, replicating
forge-geom Spline (natural cubic, chord knots) and forge-ops loft (circle split into two half arcs)."""
import numpy as np
from math import radians, sin, cos, pi

def thomas(rows, r):
    n = len(r); r = list(r); c = [0.0]*n
    b0 = rows[0][1]; c[0] = rows[0][2]/b0; r[0] /= b0
    for i in range(1, n):
        a, b, cc = rows[i]; b0 = b - a*c[i-1]; c[i] = cc/b0; r[i] = (r[i]-a*r[i-1])/b0
    for i in range(n-2, -1, -1): r[i] -= c[i]*r[i+1]
    return r

def natural_m(knots):
    n = len(knots); h = np.diff(knots)
    rows = [(0.0, 1.0, 0.0)]*n
    rows = list(rows)
    for i in range(1, n-1): rows[i] = (h[i-1], 2*(h[i-1]+h[i]), h[i])
    m = []
    for d in range(n):
        y = lambda i: 1.0 if i == d else 0.0
        r = [0.0]*n
        for i in range(1, n-1): r[i] = 6*((y(i+1)-y(i))/h[i] - (y(i)-y(i-1))/h[i-1])
        m.append(thomas(rows, r))
    return np.array(m)

def weight_polys(knots, m, i):
    """Cubic coefficients (power basis in s, low->high) of each datum's weight on span i."""
    k0, k1 = knots[i], knots[i+1]; h = k1-k0
    P = np.polynomial.polynomial
    a = np.array([k1/h, -1/h]); b = np.array([-k0/h, 1/h])
    out = []
    for d in range(len(knots)):
        w = P.polyadd(P.polymul(a, [1.0 if d == i else 0.0]), P.polymul(b, [1.0 if d == i+1 else 0.0]))
        a3 = P.polysub(P.polypow(a, 3), a); b3 = P.polysub(P.polypow(b, 3), b)
        w = P.polyadd(w, P.polymul(P.polyadd(P.polymul(a3, [m[d][i]]), P.polymul(b3, [m[d][i+1]])), [h*h/6]))
        out.append(np.pad(w, (0, 4-len(w))))
    return np.array(out)  # (n, 4)

class Loft:
    def __init__(self, profiles):
        """profiles: list of (centre, xaxis, yaxis, radius)."""
        self.c = np.array([p[0] for p in profiles]); self.x = np.array([p[1] for p in profiles])
        self.y = np.array([p[2] for p in profiles]); self.r = np.array([p[3] for p in profiles])
        self.knots = np.concatenate([[0.0], np.cumsum(np.linalg.norm(np.diff(self.c, axis=0), axis=1))])
        self.m = natural_m(self.knots)
        # per span: coefficient arrays (4, 3) for c, X, Y (power basis in s)
        self.spans = []
        for i in range(len(self.knots)-1):
            W = weight_polys(self.knots, self.m, i)  # (n,4)
            C = W.T @ self.c; X = W.T @ (self.r[:, None]*self.x); Y = W.T @ (self.r[:, None]*self.y)
            self.spans.append((C, X, Y))
    def span_of(self, s):
        return int(np.clip(np.searchsorted(self.knots, s, side='right')-1, 0, len(self.knots)-2))
    def cxy(self, s, i=None):
        i = self.span_of(s) if i is None else i
        C, X, Y = self.spans[i]; p = np.array([1, s, s*s, s**3])
        return p @ C, p @ X, p @ Y
    def point(self, s, a, i=None):
        c, X, Y = self.cxy(s, i); return c + X*np.cos(a) + Y*np.sin(a)

def volute_profiles(radii, extra=None):
    out = []
    angs = [60, 110, 160, 210, 260, 310, 360]
    rads = [87.17, 89.81, 92.44, 95.08, 97.72, 100.4, 103]
    for th, R, r in zip(angs, rads, radii):
        t = radians(th)
        out.append((np.array([R*cos(t), R*sin(t), 0.0]), np.array([cos(t), sin(t), 0.0]), np.array([0.0, 0.0, -1.0]), r))
    for (y, r) in zip([55, 110, 125], radii[7:]):
        out.append((np.array([103.0, y, 0.0]), np.array([1.0, 0, 0]), np.array([0.0, 0, -1.0]), r))
    return out

VOLUTE = Loft(volute_profiles([16.67, 18.89, 21.11, 23.33, 25.56, 27.78, 30, 33.5, 37]))
CORE = Loft(volute_profiles([11.67, 13.89, 16.11, 18.33, 20.56, 22.78, 25, 28.5, 32, 33]))
