import numpy as np, sys
from volute import *
V, C = Loft(OUTER), Loft(CORE)
def pt(L, j, tau, c, s):
    P, A, r = L.span(j)
    ev = lambda p: npoly.polyval(tau, p)
    return np.array([ev(P[0]) + c*ev(A[0]), ev(P[1]) + c*ev(A[1]), -s*ev(r)])
for (j1, j2) in [(0,3),(0,4),(3,0),(7,7),(6,6),(1,5)]:
    F, G1, W, DA1, DA2 = F_span(V, j1, C, j2)
    g = np.linspace(0,1,401)
    T1,T2 = np.meshgrid(g,g,indexing='ij')
    fv = evalF(F,T1,T2); gv = evalF(G1,T1,T2)
    best = []
    for i in range(400):
        for k in range(401):
            if np.sign(fv[i,k]) != np.sign(fv[i+1,k]) and gv[i,k] >= 0 and gv[i+1,k] >= 0:
                # bisect in tau1
                a, b = g[i], g[i+1]
                for _ in range(60):
                    m = 0.5*(a+b)
                    if np.sign(evalF(F,m,g[k])) == np.sign(evalF(F,a,g[k])): a = m
                    else: b = m
                t1, t2 = a, g[k]
                w = evalF(W,t1,t2); c1 = evalF(DA2,t1,t2)/w; c2 = evalF(DA1,t1,t2)/w
                s1 = np.sqrt(max(0,1-c1*c1)); 
                p1 = pt(V,j1,t1,c1,s1)
                for sg in (1,-1):
                    p2 = pt(C,j2,t2,c2,sg*np.sqrt(max(0,1-c2*c2)))
                    best.append((np.linalg.norm(p1-p2), t1, t2, c1, c2, p1))
    best.sort(key=lambda x: x[0])
    print((j1,j2), len(best), [ (round(b[0],9), round(b[1],3), round(b[2],3), round(b[3],3), round(b[4],3), np.round(b[5],2)) for b in best[:3]])
