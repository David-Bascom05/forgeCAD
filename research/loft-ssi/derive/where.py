import numpy as np
from subdiv import *
for (j1,j2) in [(3,3),(0,0),(7,7),(0,3)]:
    F, G1, W, DA1, DA2 = F_span(V, j1, C, j2)
    G2 = add(mul(W,W), neg(mul(DA1,DA1)))
    bs = [bernstein(F), bernstein(add(G1, np.zeros(F.shape))), bernstein(add(G2, np.zeros(F.shape)))]
    st = {'boxes':0, 'left':[]}
    certify(bs, 0, 20, st, ((0,1),(0,1)))
    L = np.array([[ (b[0][0]+b[0][1])/2, (b[1][0]+b[1][1])/2] for b in st['left']])
    print((j1,j2), len(L))
    if len(L):
        d = L[:,0]-L[:,1]
        print("  tau1 range", L[:,0].min(), L[:,0].max(), " tau1-tau2 range", d.min(), d.max())
        for q in L[::max(1,len(L)//8)]:
            t1,t2=q; w=evalF(W,t1,t2)
            print("   ", np.round(q,4), "F",evalF(F,t1,t2),"G1",evalF(G1,t1,t2),"W",w,"c1",evalF(DA2,t1,t2)/w if w else None)
