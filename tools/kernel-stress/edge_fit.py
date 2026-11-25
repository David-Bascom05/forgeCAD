"""A STEP file's own edges against its own faces, with no forge code: for an edge, how far its
curve and its vertices lie from each face that uses it. For showing a file invalid (a refusal's
evidence), not for reading one. B-spline surfaces and curves only.

    python3 -I tools/kernel-stress/edge_fit.py FILE.step EDGE_CURVE_ID [EDGE_CURVE_ID ...]
"""
import re, sys, math
src = open(sys.argv[1]).read()
E = {}
for m in re.finditer(r'#(\d+)\s*=\s*(.*?);\s*(?=#\d+\s*=|ENDSEC)', src, re.S):
    E[int(m.group(1))] = ' '.join(m.group(2).split())
def refs(s): return [int(x) for x in re.findall(r'#(\d+)', s)]
def nums(s): return [float(x) for x in re.findall(r'[-+]?\d+\.\d*(?:[eE][-+]?\d+)?|[-+]?\d+(?:[eE][-+]?\d+)?', s)]
def pt(i): return nums(E[i].split('(',2)[2])[:3]
def groups(s):
    # top-level parenthesised groups of a string starting after a '('
    out, d, st = [], 0, None
    for k,c in enumerate(s):
        if c=='(':
            d+=1
            if d==1: st=k
        elif c==')':
            d-=1
            if d==0: out.append(s[st+1:k])
    return out
def basis(knots_full, p, t):
    n = len(knots_full)-p-1
    t = min(max(t, knots_full[p]), knots_full[n])
    N=[0.0]*n
    for i in range(n):
        if knots_full[i] <= t < knots_full[i+1] or (t==knots_full[n] and knots_full[i]<t<=knots_full[i+1]): N[i]=1.0
    # Cox-de Boor
    Nk=[ [1.0 if (knots_full[i] <= t < knots_full[i+1] or (t==knots_full[n] and knots_full[i] < t <= knots_full[i+1])) else 0.0 for i in range(len(knots_full)-1)] ]
    cur=Nk[0]
    for d in range(1,p+1):
        nxt=[]
        for i in range(len(knots_full)-1-d):
            a=0.0
            if knots_full[i+d]!=knots_full[i]: a+=(t-knots_full[i])/(knots_full[i+d]-knots_full[i])*cur[i]
            if knots_full[i+d+1]!=knots_full[i+1]: a+=(knots_full[i+d+1]-t)/(knots_full[i+d+1]-knots_full[i+1])*cur[i+1]
            nxt.append(a)
        cur=nxt
    return cur[:n]
def expand(mults, ks): return [k for m,k in zip(mults,ks) for _ in range(int(m))]
def surface(i):
    s=E[i]
    if 'B_SPLINE_SURFACE' not in s: return None
    b = s[s.index('B_SPLINE_SURFACE (')+len('B_SPLINE_SURFACE ('):]
    p,q = [int(x) for x in re.match(r'\s*(\d+)\s*,\s*(\d+)', b).groups()]
    net = [[pt(r) for r in refs(row)] for row in groups(groups(groups('('+b)[0])[0])]
    k = s[s.index('B_SPLINE_SURFACE_WITH_KNOTS (')+len('B_SPLINE_SURFACE_WITH_KNOTS ('):]
    g = groups('('+k)[0]; gg=groups(g)
    um,vm,uk,vk=[nums(x) for x in gg[:4]]
    W=[[1.0]*len(net[0]) for _ in net]
    if 'RATIONAL_B_SPLINE_SURFACE' in s:
        r=s[s.index('RATIONAL_B_SPLINE_SURFACE (')+len('RATIONAL_B_SPLINE_SURFACE ('):]
        W=[nums(row) for row in groups(groups(groups('('+r)[0])[0])]
    U,V=expand(um,uk),expand(vm,vk)
    def ev(u,v):
        Nu,Nv=basis(U,p,u),basis(V,q,v)
        acc=[0,0,0,0]
        for a in range(len(net)):
            for c in range(len(net[0])):
                w=Nu[a]*Nv[c]*W[a][c]
                for d in range(3): acc[d]+=w*net[a][c][d]
                acc[3]+=w
        return [acc[d]/acc[3] for d in range(3)]
    return ev,(U[p],U[-p-1]),(V[q],V[-q-1])
def curve(i):
    s=E[i]
    if 'B_SPLINE_CURVE' not in s: return None, s[:80]
    b=s[s.index('B_SPLINE_CURVE (')+len('B_SPLINE_CURVE ('):]
    p=int(re.match(r'\s*(\d+)',b).group(1))
    P=[pt(r) for r in refs(groups('('+b)[0])]
    k=s[s.index('B_SPLINE_CURVE_WITH_KNOTS (')+len('B_SPLINE_CURVE_WITH_KNOTS ('):]
    gg=groups(k); m,ks=nums(gg[0]),nums(gg[1])
    W=[1.0]*len(P)
    if 'RATIONAL_B_SPLINE_CURVE' in s:
        W=nums(groups(s[s.index('RATIONAL_B_SPLINE_CURVE (')+len('RATIONAL_B_SPLINE_CURVE ('):])[0])
    K=expand(m,ks)
    def ev(t):
        N=basis(K,p,t); acc=[0,0,0,0]
        for a in range(len(P)):
            w=N[a]*W[a]
            for d in range(3): acc[d]+=w*P[a][d]
            acc[3]+=w
        return [acc[d]/acc[3] for d in range(3)]
    return ev,(K[p],K[-p-1])
def dist(a,b): return math.dist(a,b)
def closest(ev,ur,vr,q):
    best=(1e300,0,0)
    n=40
    for a in range(n+1):
        for c in range(n+1):
            u=ur[0]+(ur[1]-ur[0])*a/n; v=vr[0]+(vr[1]-vr[0])*c/n
            d=dist(ev(u,v),q)
            if d<best[0]: best=(d,u,v)
    d,u,v=best; hu=(ur[1]-ur[0])/n; hv=(vr[1]-vr[0])/n
    for _ in range(60):
        imp=False
        for du,dv in ((hu,0),(-hu,0),(0,hv),(0,-hv)):
            uu=min(max(u+du,ur[0]),ur[1]); vv=min(max(v+dv,vr[0]),vr[1])
            dd=dist(ev(uu,vv),q)
            if dd<d: d,u,v,imp=dd,uu,vv,True
        if not imp: hu/=2; hv/=2
    return d
for i in [int(a.lstrip('#')) for a in sys.argv[2:]]:
    s=E[i]
    r=refs(s)
    geo=r[2]; g=E[geo]
    print(f"EDGE_CURVE #{i}: curve #{geo} {g[:60]}")
    cid=geo
    if g.startswith('SURFACE_CURVE') or g.startswith('SEAM_CURVE'): cid=refs(g)[0]
    cv=curve(cid)
    oe=[j for j,t in E.items() if t.startswith('ORIENTED_EDGE') and i in refs(t)]
    for o in oe:
        loops=[j for j,t in E.items() if t.startswith('EDGE_LOOP') and o in refs(t)]
        for l in loops:
            fb=[j for j,t in E.items() if 'FACE' in t.split('(')[0] and 'BOUND' in t.split('(')[0] and l in refs(t)]
            for f in fb:
                fa=[j for j,t in E.items() if t.startswith('ADVANCED_FACE') and f in refs(t)]
                for a in fa:
                    sid=refs(E[a])[-1]
                    S=surface(sid)
                    kind=E[sid].split('(')[0].strip() if not E[sid].startswith('(') else 'B_SPLINE (complex)'
                    if S is None or cv[0] is None:
                        print(f"  face #{a} surface #{sid} {kind}: not evaluated here"); continue
                    ev,ur,vr=S; cev,(t0,t1)=cv
                    worst=max(closest(ev,ur,vr,cev(t0+(t1-t0)*j/12)) for j in range(13))
                    # the part between the edge's vertices
                    va,vb=pt(refs(E[r[0]])[0]),pt(refs(E[r[1]])[0])
                    def tof(q):
                        ts=[t0+(t1-t0)*j/2000 for j in range(2001)]
                        return min(ts,key=lambda t:dist(cev(t),q))
                    ta,tb=sorted((tof(va),tof(vb)))
                    da,db=dist(cev(tof(va)),va),dist(cev(tof(vb)),vb)
                    inner=max(closest(ev,ur,vr,cev(ta+(tb-ta)*j/12)) for j in range(13))
                    vd=[f"{closest(ev,ur,vr,v):.3g}" for v in (va,vb)]
                    print(f"  face #{a} surface #{sid} {kind}: whole curve [{t0},{t1}] off it by up to {worst:.6g} mm; "
                          f"vertices on the curve at t={ta:.4f},{tb:.4f} (off it by {da:.2e}, {db:.2e}); that part off the face by up to {inner:.3g} mm; vertices off the face by {vd}")
