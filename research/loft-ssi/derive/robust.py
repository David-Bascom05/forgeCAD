import numpy as np, sys
from fol import *
V, C = Loft(OUTER), Loft(CORE)
rng = [(0,1),(0,1),(-1,1)]
EPS = float(sys.argv[1]) if len(sys.argv)>1 else 1e-12
def cert(g, psi, d, st, eg, ep):
    st['boxes'] += 1
    if g.min() > eg or g.max() < -eg or psi.max() < -ep or psi.min() > ep: return
    if d == 36: st['left'] += 1; return
    ax = d % 3; gl, gh = split(g, ax); pl, ph = split(psi, ax)
    cert(gl, pl, d+1, st, eg, ep); cert(gh, ph, d+1, st, eg, ep)
tot=0; left=0
for j1 in range(8):
    row=[]
    for j2 in range(9):
        g, psi = pair(V, j1, C, j2)
        bg, bp = bern3(g,rng), bern3(psi,rng)
        st={'boxes':0,'left':0}
        cert(bg, bp, 0, st, EPS*abs(bg).max(), EPS*abs(bp).max())
        tot+=st['boxes']; left+=st['left']; row.append(f"{st['boxes']}/{st['left']}")
    print(j1," ".join(row), flush=True)
print("total", tot, "left", left)
