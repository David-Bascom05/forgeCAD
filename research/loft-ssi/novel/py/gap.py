# For every core point, the volute section plane through it and its elliptic radius there (rho<1: inside).
from loft import *
import numpy as np
V, C = VOLUTE, CORE
def plane_res(s, p):
    c, X, Y = V.cxy(s); n = np.cross(X, Y); return n @ (p - c) / np.linalg.norm(n)
def rho(p, s0):
    s = s0
    for _ in range(30):
        f = plane_res(s, p); h = 1e-4
        df = (plane_res(s+h, p)-plane_res(s-h, p))/(2*h); s -= f/df
        if abs(f) < 1e-12: break
    c, X, Y = V.cxy(s); G = np.array([X, Y]).T
    uv = np.linalg.lstsq(G, p-c, rcond=None)[0]
    return np.hypot(*uv), s
worst = 0
lo = -0.5*(C.knots[1]-C.knots[0]); hi = C.knots[-1] + 0.5*(C.knots[-1]-C.knots[-2])
for sg in np.linspace(lo, hi, 400):
    for a in np.linspace(0, 2*pi, 64, endpoint=False):
        p = C.point(sg, a)
        r, s = rho(p, min(max(sg, -30), V.knots[-1]+20))
        if r > worst: worst, at = r, (sg, a, s)
print('max elliptic radius of core inside volute sections', worst, at)
