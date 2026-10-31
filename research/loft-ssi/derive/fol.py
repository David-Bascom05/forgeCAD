"""Foliation certificate: variables (tau1, tau2, c2); g = A1^(D + c2 A2), Psi = rho1^2((D + c2 A2).A1)^2
+ |A1|^4 (rho2^2 (1 - c2^2) - rho1^2).  A box is clear if g has strict sign or Psi < 0 throughout."""
import numpy as np, math, sys
from volute import *

def P3(a):  # make 3D power array [i1,i2,i3]
    return a

def lift(p, ax):
    p = np.asarray(p, float); sh = [1,1,1]; sh[ax] = len(p); return p.reshape(sh)

def mul3(a, b):
    out = np.zeros(tuple(a.shape[i]+b.shape[i]-1 for i in range(3)))
    for idx in np.ndindex(a.shape):
        if a[idx] != 0.0:
            out[idx[0]:idx[0]+b.shape[0], idx[1]:idx[1]+b.shape[1], idx[2]:idx[2]+b.shape[2]] += a[idx]*b
    return out

def add3(*xs):
    sh = tuple(max(x.shape[i] for x in xs) for i in range(3)); out = np.zeros(sh)
    for x in xs: out[:x.shape[0],:x.shape[1],:x.shape[2]] += x
    return out

def conv(n, lo=0.0, hi=1.0):
    # power basis in x to Bernstein on [lo,hi]: substitute x = lo + (hi-lo) y, then power->Bernstein
    S = np.zeros((n+1,n+1))  # S[i,k]: coeff of y^k in (lo + w y)^i
    w = hi-lo
    for i in range(n+1):
        for k in range(i+1):
            S[i,k] = math.comb(i,k) * lo**(i-k) * w**k
    T = np.zeros((n+1,n+1))
    for k in range(n+1):
        for i in range(k+1):
            T[k,i] = math.comb(k,i)/math.comb(n,i)
    return T @ S.T   # bern = T @ (S^T a)

def bern3(a, ranges):
    out = a
    for ax in range(3):
        M = conv(a.shape[ax]-1, *ranges[ax])
        out = np.moveaxis(np.tensordot(M, np.moveaxis(out, ax, 0), axes=(1,0)), 0, ax)
    return out

def split(b, axis):
    b = np.moveaxis(b, axis, 0); n = b.shape[0]-1
    lo=[b[0]]; hi=[b[n]]; cur=b
    for k in range(n):
        cur = 0.5*(cur[:-1]+cur[1:]); lo.append(cur[0]); hi.append(cur[-1])
    return np.moveaxis(np.array(lo),0,axis), np.moveaxis(np.array(hi[::-1]),0,axis)

def certify(g, psi, depth, maxd, st):
    st['boxes'] += 1
    if g.min() > 0 or g.max() < 0 or psi.max() < 0:
        return
    if psi.min() > 0:
        st['outside'] += 1; return
    if depth == maxd:
        st['left'] += 1; return
    ax = int(np.argmax([0,0,0]) if False else depth % 3)
    gl, gh = split(g, ax); pl, ph = split(psi, ax)
    certify(gl, pl, depth+1, maxd, st); certify(gh, ph, depth+1, maxd, st)

def pair(L1, j1, L2, j2):
    P1, A1, r1 = L1.span(j1); P2, A2, r2 = L2.span(j2)
    a1 = [lift(x,0) for x in A1]; a2 = [lift(x,1) for x in A2]
    c = lift([0.0,1.0],2)
    D = [add3(lift(P2[i],1), -lift(P1[i],0)) for i in range(2)]
    E = [add3(D[i], mul3(c, a2[i])) for i in range(2)]   # H2 - P1
    g = add3(mul3(a1[0],E[1]), -mul3(a1[1],E[0]))
    dot = add3(mul3(E[0],a1[0]), mul3(E[1],a1[1]))
    A1sq = add3(mul3(a1[0],a1[0]), mul3(a1[1],a1[1]))
    R1 = lift(npoly.polymul(r1,r1),0); R2 = lift(npoly.polymul(r2,r2),1)
    one_c2 = lift([1.0,0.0,-1.0],2)
    psi = add3(mul3(R1, mul3(dot,dot)), mul3(mul3(A1sq,A1sq), add3(mul3(R2,one_c2), -R1)))
    return g, psi

if __name__ == "__main__":
    V, C = Loft(OUTER), Loft(CORE)
    tot = 0
    rng = [(0,1),(0,1),(-1,1)]
    for j1 in range(8):
        row=[]
        for j2 in range(9):
            g, psi = pair(V, j1, C, j2)
            st = {'boxes':0,'left':0,'outside':0}
            certify(bern3(g,rng), bern3(psi,rng), 0, 30, st)
            tot += st['boxes']; row.append(f"{st['boxes']}/{st['left']}/{st['outside']}")
        print(j1, " ".join(row), flush=True)
    print("total", tot)
