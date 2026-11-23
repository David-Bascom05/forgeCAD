import sys, math, re
def ents(path):
    L=open(path).read().split('\n'); pairs=[(int(L[i]),L[i+1]) for i in range(0,len(L)-1,2)]
    i=[k for k,p in enumerate(pairs) if p==(2,'ENTITIES')][0]+1
    out=[];cur=None
    for c,v in pairs[i:]:
        if c==0:
            if cur: out.append(cur)
            if v=='ENDSEC': break
            cur=[v,{}]
        else: cur[1].setdefault(c,[]).append(v)
    return out,pairs
for path in sys.argv[1:]:
    es,pairs=ents(path)
    nums=[v for c,v in pairs if 10<=c<=59 or 40<=c<=51]
    expo=[v for v in nums if 'e' in v]
    tiny=[v for v in nums if v not in('0.0','-0.0') and abs(float(v))<1e-9]
    neg0=[v for v in nums if v=='-0.0']
    kinds={}
    for k,d in es: kinds[(k,d[8][0])]=kinds.get((k,d[8][0]),0)+1
    # endpoints of CUT curves, in file order
    def ends(k,d):
        f=lambda c:[float(x) for x in d[c]]
        if k=='LINE': return (f(10)[0],f(20)[0]),(f(11)[0],f(21)[0])
        if k=='ARC':
            c=(f(10)[0],f(20)[0]);r=f(40)[0];a,b=math.radians(f(50)[0]),math.radians(f(51)[0])
            return (c[0]+r*math.cos(a),c[1]+r*math.sin(a)),(c[0]+r*math.cos(b),c[1]+r*math.sin(b))
        if k=='SPLINE':
            xs,ys=f(10),f(20); return (xs[0],ys[0]),(xs[-1],ys[-1])
        return None
    pts=[ends(k,d) for k,d in es if d[8][0]=='CUT' and k!='CIRCLE']
    allp=[p for e in pts if e for p in e]
    # for each endpoint, distance to nearest other endpoint
    worst=0
    for i,p in enumerate(allp):
        m=min(math.dist(p,q) for j,q in enumerate(allp) if j!=i) if len(allp)>1 else 0
        worst=max(worst,m)
    print(path.split('/')[-1], dict(kinds), 'worst_end_gap=%.3g'%worst, 'expo=',expo[:4], 'tiny=',len(tiny), tiny[:3], 'neg0=',len(neg0))
