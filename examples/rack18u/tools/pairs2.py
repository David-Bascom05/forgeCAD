import math
from vec import *
X,Y,Z=np.eye(3)
def pose(o,a,b,c): return (V(o),[V(a),V(b),V(c)])
def R(p,v):
    c=p[1]; return c[0]*v[0]+c[1]*v[1]+c[2]*v[2]
def T(p,v): return p[0]+R(p,v)
# ---- parts: anchors local (origin, normal, x)
post={}
for leg,base,nrm in (('a',lambda s,w:(s,0,w),(0,1,0)),('b',lambda s,w:(0,s,w),(1,0,0))):
    for i,w in enumerate((15,891)): post[f'stud_{leg}[{i}]']=(base(25,w),nrm,(0,0,1 if i==0 else -1))
    for i,w in enumerate((100,260,420,486,646,806)): post[f'pnl_{leg}[{i}]']=(base(25,w),nrm,(0,0,1 if i<3 else -1))
    for i,w in enumerate((50,66,840,856)): post[f'hng_{leg}[{i}]']=(base(32.5,w),nrm,(0,0,1 if i<2 else -1))
    for i,w in enumerate((443,463)): post[f'stk_{leg}[{i}]']=(base(25,w),nrm,(0,0,1))
postP={'FL':pose((-298.48,35,0),X,Y,Z),'FR':pose((298.48,35,0),Y,-X,Z),'RR':pose((298.48,765,0),-X,-Y,Z),'RL':pose((-298.48,765,0),-Y,X,Z)}
pan={}
for nm,o,n in [('stud_fl_f',(-273.48,-361.98,15),(0,1,0)),('stud_fr_f',(273.48,-361.98,15),(0,1,0)),
               ('stud_rl_r',(-273.48,361.98,15),(0,-1,0)),('stud_rr_r',(273.48,361.98,15),(0,-1,0)),
               ('stud_fl_s',(-295.46,-340,15),(1,0,0)),('stud_rl_s',(-295.46,340,15),(1,0,0)),
               ('stud_fr_s',(295.46,-340,15),(-1,0,0)),('stud_rr_s',(295.46,340,15),(-1,0,0))]:
    pan[nm]=(o,n,(0,0,1))
for i,yy in enumerate((-300,0,300)):
    pan[f'car_l[{i}]']=((-271,yy,0),(0,0,1),(-1,0,0)); pan[f'car_r[{i}]']=((271,yy,0),(0,0,1),(1,0,0))
for i,(xx,yy) in enumerate(((-130,-140),(130,-140),(-130,140),(130,140))): pan[f'hang[{i}]']=((xx,yy,0),(0,0,1),(1,0,0))
for nm,(xx,yy) in dict(fl=(-185,-300),fr=(185,-300),rl=(-185,300),rr=(185,300)).items(): pan[f'caster_{nm}']=((xx,yy,0),(0,0,1),(1,0,0))
panP={'base':pose((0,400,0),X,Y,Z),'top':pose((0,400,906),-X,Y,-Z)}
car={}
for k in range(28): car[f'rail[{k}]']=((19.5+25*k,0,20),(0,1,0),(0,0,1))
for i,a in enumerate((57,357,657)): car[f'pan[{i}]']=((a,19.12,0),(0,0,1),(0,1,0))
carP={'BL':pose((-251.88,43,3.02),Y,-X,Z),'BR':pose((251.88,757,3.02),-Y,X,Z),'TL':pose((-251.88,757,902.98),-Y,-X,-Z),'TR':pose((251.88,43,902.98),Y,X,-Z)}
rail={}
for i,q in enumerate((12.5,37.5)):
    rail[f'web_bot[{i}]']=((0,q,15.02),(-1,0,0),(0,0,1)); rail[f'web_top[{i}]']=((0,q,874.98),(-1,0,0),(0,0,-1))
yf,yr=100,725
railP={'LF':pose((-251.88,yf,8),X,Y,Z),'RF':pose((251.88,yf,898),-X,Y,-Z),'RR':pose((251.88,yr,8),-X,-Y,Z),'LR':pose((-251.88,yr,898),X,-Y,-Z)}
pnl={}
k=0
for e in (60,740):
    for f in (100,260,420): pnl[f'scr[{k}]']=((e,f,0),(0,0,1),(0,1,0)); k+=1
for i,e in enumerate((250,400,550)): pnl[f'jnt[{i}]']=((e,453,14),(0,-1,0),(1,0,0))
pnlP={'LL':pose((-300,0,0),Y,Z,X),'UL':pose((-300,800,906),-Y,-Z,X),'LR':pose((300,800,0),-Y,Z,-X),'UR':pose((300,0,906),Y,-Z,-X)}
brk={'scr[0]':((12,21,20.92),(0,1,0),(0,0,1)),'scr[1]':((12,21,36.92),(0,1,0),(0,0,1)),'scr[2]':((-12,21,20.92),(0,1,0),(0,0,1)),'scr[3]':((-12,21,36.92),(0,1,0),(0,0,1)),'pin':((0,0,0),(0,0,1),(0,1,0))}  # R11: 4 holes at i +-12
brkP={'FB':pose((-277.98,14,29.08),X,Y,Z),'FT':pose((-277.98,14,876.92),-X,Y,-Z),'RB':pose((277.98,786,29.08),-X,-Y,Z),'RT':pose((277.98,786,876.92),X,-Y,-Z)}
stk={'scr[0]':((0,0,-10),(0,1,0),(0,0,1)),'scr[1]':((0,0,10),(0,1,0),(0,0,1))}
stkP={'F':pose((273.48,35,453),X,Y,Z),'R':pose((-273.48,765,453),-X,-Y,Z)}
door={'pin_bot':((13,0,-14),(0,1,0),(0,0,-1)),'pin_top':((13,828,-14),(0,-1,0),(0,0,-1))}
doorP={'F':pose((-290.98,0,39),X,Z,-Y),'R':pose((290.98,800,39),-X,Z,Y)}
tray={}
for i,(xx,yy) in enumerate(((-130,-140),(130,-140),(-130,140),(130,140))): tray[f'hang[{i}]']=((xx,yy,0),(0,0,-1),(-1,0,0))
trayP={'T':pose((0,400,893.38),X,Y,Z)}
cas={'top':((0,0,0),(0,0,1),(1,0,0))}
casP={'fl':pose((-185,100,0),X,Y,Z),'fr':pose((185,100,0),X,Y,Z),'rl':pose((-185,700,0),X,Y,Z),'rr':pose((185,700,0),X,Y,Z)}
def rack(part,P):
    out={}
    for nm,(o,n,x) in part.items(): out[nm]=(T(P,o),R(P,n),R(P,x))
    return out
def match(Aname,A,Bname,B,tol=0.05,maxoff=15):
    rows=[]
    for an,(ao,anrm,ax) in A.items():
        for bn,(bo,bnrm,bx) in B.items():
            if abs(abs(anrm@bnrm)-1)>1e-6: continue
            d=ao-bo; along=d@bnrm; lat=d-along*bnrm
            if np.linalg.norm(lat)<tol and abs(along)<=maxoff:
                th=math.degrees(math.atan2(np.cross(bx,ax)@bnrm,bx@ax))
                rows.append((an,bn,round(along,3),int(round(anrm@bnrm)),round(th)%360))
    return rows
def dirn(v):
    for nm,a in (('+X',X),('-X',-X),('+Y',Y),('-Y',-Y),('+Z',Z),('-Z',-Z)):
        if np.allclose(v,a): return nm
    return str(np.round(v,3))
pairs=[]
for pk,pp in postP.items():
    Pr=rack(post,pp)
    for pn,ppan in panP.items():
        for r in match(f'post_{pk}',{k:v for k,v in Pr.items() if k.startswith('stud')},pn,{k:v for k,v in rack(pan,ppan).items() if k.startswith('stud')}): pairs.append(('post '+pk,'pan '+pn)+r)
    for nm,pq in pnlP.items():
        for r in match('',rack(pnl,pq),'',{k:v for k,v in Pr.items() if k.startswith('pnl')}): pairs.append(('panel '+nm,'post '+pk)+r)
    for nm,pq in brkP.items():
        for r in match('',{k:v for k,v in rack(brk,pq).items() if k.startswith('scr')},'',{k:v for k,v in Pr.items() if k.startswith('hng')}): pairs.append(('bracket '+nm,'post '+pk)+r)
    for nm,pq in stkP.items():
        for r in match('',rack(stk,pq),'',{k:v for k,v in Pr.items() if k.startswith('stk')}): pairs.append(('strike '+nm,'post '+pk)+r)
for ck,cp in carP.items():
    Cr=rack(car,cp)
    for pn,ppan in panP.items():
        for r in match('',{k:v for k,v in Cr.items() if k.startswith('pan')},'',{k:v for k,v in rack(pan,ppan).items() if k.startswith('car')}): pairs.append(('carrier '+ck,'pan '+pn)+r)
    for rk,rp in railP.items():
        for r in match('',rack(rail,rp),'',{k:v for k,v in Cr.items() if k.startswith('rail')}): pairs.append(('rail '+rk,'carrier '+ck)+r)
for dk,dp in doorP.items():
    for bk,bp in brkP.items():
        for r in match('',rack(door,dp),'',{k:v for k,v in rack(brk,bp).items() if k=='pin'}): pairs.append(('door '+dk,'bracket '+bk)+r)
for r in match('',rack(tray,trayP['T']),'',{k:v for k,v in rack(pan,panP['top']).items() if k.startswith('hang')},maxoff=20): pairs.append(('tray','top pan')+r)
for ck,cp in casP.items():
    for r in match('',rack(cas,cp),'',{k:v for k,v in rack(pan,panP['base']).items() if k.startswith('caster')}): pairs.append(('caster '+ck,'base pan')+r)
for p in pairs: print('%-12s %-10s %-11s %-12s off %7.3f  n %+d  theta %3d'%p)
print(len(pairs))
