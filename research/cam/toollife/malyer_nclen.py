# Minutes in cut (feed moves below the stock top z = 0) and entry count of a Fanuc-style program.
import math, re, sys
def run(path):
    x=y=z=0.0; mode=0; F=None; cut=0.0; entries=0; above=True
    for ln in open(path, errors="replace"):
        ln=re.sub(r"\(.*?\)","",ln)
        w=dict()
        for k,v in re.findall(r"([A-Z])\s*(-?[0-9.]+)",ln):
            if k=="G":
                g=int(float(v))
                if g in (0,1,2,3): mode=g
            else: w[k]=float(v)
        if "F" in w: F=w["F"]
        if not any(k in w for k in "XYZ"): continue
        nx,ny,nz=w.get("X",x),w.get("Y",y),w.get("Z",z)
        if mode in (1,2,3):
            if mode==1:
                L=math.dist((x,y,z),(nx,ny,nz))
            else:
                cx,cy=x+w.get("I",0),y+w.get("J",0); r=math.hypot(x-cx,y-cy)
                a0=math.atan2(y-cy,x-cx); a1=math.atan2(ny-cy,nx-cx); sw=a1-a0
                if mode==3 and sw<=0: sw+=2*math.pi
                if mode==2 and sw>=0: sw-=2*math.pi
                L=math.hypot(abs(sw)*r,nz-z)
            # the part of the move below z = 0
            if z<=0 and nz<=0: frac=1.0
            elif z>0 and nz>0: frac=0.0
            else: frac=abs(min(z,nz))/abs(nz-z)
            cut+=L*frac/F
        if above and nz<=0: entries+=1
        above = nz>0
        x,y,z=nx,ny,nz
    return cut, entries, F
for p in sys.argv[1:]:
    c,e,F=run(p); print(f"{p.split('/')[-1]} F={F} min_in_cut={c:.2f} entries={e} min_per_entry={c/e:.3f}")
