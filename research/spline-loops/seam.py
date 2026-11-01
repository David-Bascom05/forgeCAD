"""Measures a circle-to-circle loft's seam from its STEP rulings. For each periodic ruled B-spline surface in
st/*.step (extract with the importer's getstep.py), prints each section's seam direction and the error of
the rule next-seam = point of the next circle nearest the last seam. Datasets stay out of the repo."""
import re,sys,math,glob,json,os,zipfile
def parse(path):
    t=open(path).read()
    ents=dict(re.findall(r'#(\d+)=(.*?);\s*\n',t,re.S))
    pts={}
    for k,v in ents.items():
        if v.startswith('CARTESIAN_POINT'):
            a=re.findall(r'[-\d.]+(?:[Ee][-+]?\d+)?',v.split(',',1)[1]); pts[k]=tuple(float(x)*1000 for x in a[:3])
    out=[]
    for k,v in ents.items():
        if not v.startswith('B_SPLINE_SURFACE'): continue
        m=re.match(r"B_SPLINE_SURFACE_WITH_KNOTS\('',(\d+),(\d+),\((.*?)\),\.",v,re.S)
        if not m: continue
        rows=re.findall(r'\(([^()]*)\)',m.group(3)+')')
        grid=[[pts.get(x.strip()[1:]) for x in r.split(',') if x.strip()] for r in rows if r.strip()]
        if not grid or any(p is None for r in grid for p in r): continue
        out.append((int(m.group(1)),int(m.group(2)),grid))
    return out
def sub(a,b):return (a[0]-b[0],a[1]-b[1],a[2]-b[2])
def dot(a,b):return a[0]*b[0]+a[1]*b[1]+a[2]*b[2]
def cross(a,b):return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
def norm(a):
    l=math.sqrt(dot(a,a));return tuple(x/l for x in a) if l else a

tot=0
for f in sorted(glob.glob('st/*.step')):
    for du,dv,g in parse(f):
        for idx in (0,):
            a=[r[0] for r in g];b=[r[-1] for r in g]
            if len(a)<8 or len(g[0])>6 : continue
            ca=tuple(sum(p[i] for p in a)/len(a) for i in range(3));cb=tuple(sum(p[i] for p in b)/len(b) for i in range(3))
            ra=[math.dist(p,ca) for p in a];rb=[math.dist(p,cb) for p in b]
            if min(ra)<.8*max(ra) or min(rb)<.8*max(rb): continue
            n=norm(cross(sub(a[2],ca),sub(a[6],ca)))
            s0=tuple(0.5*(a[0][i]+a[-1][i]) for i in range(3));s1=tuple(0.5*(b[0][i]+b[-1][i]) for i in range(3))
            d0=norm(sub(s0,ca));d1=norm(sub(s1,cb))
            off=sub(cb,ca);h=dot(off,n);ip=tuple(off[i]-h*n[i] for i in range(3))
            cos=dot(d0,d1);sn=dot(cross(d0,d1),n)
            nb=norm(cross(sub(b[2],cb),sub(b[6],cb)))
            def pred(sa,c,nn,dd):
                v=sub(sa,c);v=tuple(v[i]-dot(v,nn)*nn[i] for i in range(3));v=norm(v)
                return math.degrees(math.atan2(dot(cross(v,dd),nn),dot(v,dd)))
            e1=pred(s0,cb,nb,d1);e2=pred(s1,ca,n,d0)
            print('e(A->B) %.1f e(B->A) %.1f'%(e1,e2),os.path.basename(f)[:8],'n',tuple(round(x,2) for x in n),'R %.2f r %.2f h %.1f off %.1f'%(sum(ra)/len(ra),sum(rb)/len(rb),h,math.sqrt(dot(ip,ip))),'dA',tuple(round(x,2) for x in d0),'dB',tuple(round(x,2) for x in d1),'twist %.1f'%math.degrees(math.atan2(sn,cos)))
            tot+=1
print(tot)
