# Exactness check of F(s, sigma): bisect sigma to a zero at fixed s, lift in closed form, measure both residuals.
import numpy as np
from loft import VOLUTE
from pencil import F, lift
from vfx_scan import CUP
def root(s, lo, hi):
    flo = F(VOLUTE, s, CUP, lo)
    for _ in range(80):
        m = 0.5*(lo+hi); fm = F(VOLUTE, s, CUP, m)
        if np.sign(fm) == np.sign(flo): lo, flo = m, fm
        else: hi = m
    return 0.5*(lo+hi)
for s in [30.0, 100.0, 300.0, 450.0]:
    sig = np.linspace(s-1, s+1, 2001); f = np.array([F(VOLUTE, s, CUP, t) for t in sig])
    for i in np.where(np.sign(f[:-1]) != np.sign(f[1:]))[0]:
        t = root(s, sig[i], sig[i+1]); p, a1, a2 = lift(VOLUTE, s, CUP, t)
        r1 = np.linalg.norm(VOLUTE.point(s, a1) - p); r2 = np.linalg.norm(CUP.point(t, a2) - p)
        print(f's={s} sigma={t:.9f} p={np.round(p,4)} residuals {r1:.2e} {r2:.2e}')
