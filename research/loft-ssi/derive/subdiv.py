"""Bernstein subdivision certificate per span pair: count boxes until F has a strict sign or
G1 < 0 or G2 < 0 strictly (no real intersection), leaving boxes at a depth floor."""
import numpy as np, sys
from volute import *

def split(b, axis):
    # de Casteljau at 1/2 along axis; returns (lo, hi)
    b = np.moveaxis(b, axis, 0).copy()
    n = b.shape[0]-1
    lo = [b[0].copy()]; hi = [b[n].copy()]
    cur = b
    for k in range(n):
        cur = 0.5*(cur[:-1]+cur[1:])
        lo.append(cur[0].copy()); hi.append(cur[-1].copy())
    lo = np.array(lo); hi = np.array(hi[::-1])
    return np.moveaxis(lo,0,axis), np.moveaxis(hi,0,axis)

def certify(bs, depth, maxd, stats, box):
    stats['boxes'] += 1
    F, G1, G2 = bs
    if F.min() > 0 or F.max() < 0 or G1.max() < 0 or G2.max() < 0:
        return
    if depth == maxd:
        stats['left'].append(box)
        return
    axis = depth % 2
    parts = [split(x, axis) for x in bs]
    (u0,u1),(v0,v1) = box
    if axis == 0:
        m = 0.5*(u0+u1); boxes = (((u0,m),(v0,v1)),((m,u1),(v0,v1)))
    else:
        m = 0.5*(v0+v1); boxes = (((u0,u1),(v0,m)),((u0,u1),(m,v1)))
    for i in range(2):
        certify([p[i] for p in parts], depth+1, maxd, stats, boxes[i])

V, C = Loft(OUTER), Loft(CORE)
tot = 0
for j1 in range(8):
    row = []
    for j2 in range(9):
        F, G1, W, DA1, DA2 = F_span(V, j1, C, j2)
        G2 = add(mul(W,W), neg(mul(DA1,DA1)))
        bs = [bernstein(F), bernstein(add(G1, np.zeros(F.shape))), bernstein(add(G2, np.zeros(F.shape)))]
        st = {'boxes':0, 'left':[]}
        certify(bs, 0, 24, st, ((0,1),(0,1)))
        tot += st['boxes']
        row.append(f"{st['boxes']}/{len(st['left'])}")
        if st['left'] and j1==j2:
            pass
    print(j1, " ".join(row))
print("total boxes", tot)
