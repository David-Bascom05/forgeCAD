"""freeball.py FC PTS: for each point, search for a centre c within r of it whose ball misses both
rods of the .fc twin (exact distance to a capped cylinder). Found: the point is air by the closing."""
import sys, math, random
def rod(line):
    t=line.split(); f=lambda k: float(t[k]); i=t.index('extrude'); j=i+2 if t[i+1]=='add' else i+1
    h=f(j); zi,xi,ai,ci=t.index('z'),t.index('x'),t.index('at'),t.index('circle')
    z=[f(zi+1),f(zi+2),f(zi+3)]; n=math.sqrt(sum(a*a for a in z)); z=[a/n for a in z]
    x=[f(xi+1),f(xi+2),f(xi+3)]; o=[f(ai+1),f(ai+2),f(ai+3)]
    y=[z[1]*x[2]-z[2]*x[1], z[2]*x[0]-z[0]*x[2], z[0]*x[1]-z[1]*x[0]]
    c=[o[k]+x[k]*f(ci+1)+y[k]*f(ci+2) for k in range(3)]
    return c,z,h,f(ci+3)
def dist(p,R):
    c,z,h,rad=R; w=[p[k]-c[k] for k in range(3)]; a=sum(w[k]*z[k] for k in range(3))
    q=[w[k]-a*z[k] for k in range(3)]; rho=math.sqrt(sum(v*v for v in q))
    dr=max(rho-rad,0.0); da=max(-a,a-h,0.0)
    if rho<=rad and 0<=a<=h: return -min(rad-rho,a,h-a)
    return math.sqrt(dr*dr+da*da)
lines=[l for l in open(sys.argv[1]) if 'extrude' in l]; rods=[rod(l) for l in lines]
r=float([l for l in open(sys.argv[1]) if l.startswith('fillet')][0].split()[1])
random.seed(1)
found=0; pts=[list(map(float,l.split()[:3])) for l in open(sys.argv[2])]
for p in pts:
    best=None
    for _ in range(20000):
        u=[random.gauss(0,1) for _ in range(3)]; n=math.sqrt(sum(a*a for a in u)); s=r*0.999*random.random()**(1/3)
        c=[p[k]+u[k]/n*s for k in range(3)]
        m=min(dist(c,R) for R in rods)-r
        if best is None or m>best[0]: best=(m,c)
    if best[0]>=0: found+=1
print(f"{sys.argv[1].split('/')[-1]}: {found} of {len(pts)} points have a free ball within r (best margin of the last {best[0]:+.2e})")
