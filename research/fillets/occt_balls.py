# occt_balls.py BEFORE.step AFTER.step R, BEFORE.json beside it (as occt_fillet.py): from points of each round, ours and OCCT's own, away from the rim's far ends, steps r along the normal; the centre must stand r from the floor and r from the walls. Run with the OCCT venv, tools/occt/setup.sh.
import sys,json,math
from OCP.STEPControl import STEPControl_Reader
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE,TopAbs_EDGE,TopAbs_REVERSED
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface,BRepAdaptor_Curve
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepTools import BRepTools
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.BRepClass import BRepClass_FaceClassifier
from OCP.gp import gp_Pnt,gp_Vec,gp_Pnt2d
from OCP.TopAbs import TopAbs_IN
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
def rd(p):
    R=STEPControl_Reader();R.ReadFile(p);R.TransferRoots();return R.OneShape()
def faces(s):
    x=TopExp_Explorer(s,TopAbs_FACE);o=[]
    while x.More(): o.append(TopoDS.Face(x.Current()));x.Next()
    return o
def area(f):
    g=GProp_GProps();BRepGProp.SurfaceProperties_s(f,g);return g.Mass()
def test(before,after,r,label):
    far=[];n=0;worst=0
    for f in faces(after):
        ad=BRepAdaptor_Surface(f);u0,u1,v0,v1=ad.FirstUParameter(),ad.LastUParameter(),ad.FirstVParameter(),ad.LastVParameter()
        P=gp_Pnt();Du=gp_Vec();Dv=gp_Vec();rev=f.Orientation()==TopAbs_REVERSED
        cl=BRepClass_FaceClassifier();D=[]
        for i in range(1,24):
            for j in range(1,24):
                u=u0+(u1-u0)*i/24;v=v0+(v1-v0)*j/24
                cl.Perform(f,gp_Pnt2d(u,v),1e-7)
                if cl.State()!=TopAbs_IN: continue
                ad.D1(u,v,P,Du,Dv);N=Du.Crossed(Dv)
                if N.Magnitude()<1e-12: continue
                N.Normalize()
                if rev:N.Reverse()
                # the outward normal: the centre is inward
                c=gp_Pnt(P.X()-r*N.X(),P.Y()-r*N.Y(),P.Z()-r*N.Z())
                if min(math.dist((P.X(),P.Y(),P.Z()),q) for q in ENDS)<2.5*r: continue
                vv=BRepBuilderAPI_MakeVertex(c).Vertex()
                dw=[]
                for w in WALLS:
                    e=BRepExtrema_DistShapeShape(vv,w);e.Perform();dw.append(e.Value())
                on=BRepExtrema_DistShapeShape(BRepBuilderAPI_MakeVertex(P).Vertex(),before);on.Perform()
                D.append((max(abs(min(dw)-r),abs(c.Z()-r)),on.Value(),(round(P.X(),2),round(P.Y(),2),round(P.Z(),2))))
        if not D: continue
        # a face is a blend when its points leave the shell
        if max(x[1] for x in D)<1e-5: continue
        m=max(x[0] for x in D);n+=len(D);worst=max(worst,m)
        Q=sorted(x[0] for x in D);print(label,'area %.3f'%area(f),'pts',len(D),'max %.3e'%m,'p99 %.3e'%Q[len(Q)*99//100],'p90 %.3e'%Q[len(Q)*9//10],'median %.3e'%Q[len(Q)//2],'at',max(D)[2],flush=True)
    print(label,'blend pts',n,'worst %.3e'%worst,flush=True)
from OCP.TopAbs import TopAbs_SHELL
B=rd(sys.argv[1]);A=rd(sys.argv[2]);r=float(sys.argv[3]);d=json.load(open(sys.argv[1][:-5]+'.json'))
SH=TopoDS.Shell(TopExp_Explorer(B,TopAbs_SHELL).Current())
E0=[];x=TopExp_Explorer(B,TopAbs_EDGE)
while x.More():
    e=TopoDS.Edge(x.Current());c=BRepAdaptor_Curve(e);E0.append((c.Value(c.FirstParameter()),c.Value(.5*(c.FirstParameter()+c.LastParameter())),c.Value(c.LastParameter())));x.Next()
picked=[E for E in E0 if any(math.dist((E[1].X(),E[1].Y(),E[1].Z()),m)<1e-6 for m in d['mids'])]
# the ends of the rim, not the rail both picks share
pts=[(p.X(),p.Y(),p.Z()) for E in picked for p in (E[0],E[2])]
cnt=[sum(math.dist(p,q)<1e-6 for q in pts) for p in pts];ENDS=[p for p,c in zip(pts,cnt) if c==min(cnt)]
WALLS=[]
for f in faces(B):
    ad=BRepAdaptor_Surface(f)
    if ad.GetType().name=='GeomAbs_Plane' and abs(ad.Plane().Location().Z())<1e-9 and abs(ad.Plane().Axis().Direction().Z())>.999: continue
    for E in picked:
        e=BRepExtrema_DistShapeShape(BRepBuilderAPI_MakeVertex(E[1]).Vertex(),f);e.Perform()
        if e.Value()<1e-6: WALLS.append(f);break
print('ends',ENDS,'walls',len(WALLS))
test(SH,A,r,'ours')
x=TopExp_Explorer(B,TopAbs_EDGE);E=[]
while x.More():
    e=TopoDS.Edge(x.Current());c=BRepAdaptor_Curve(e);p=c.Value(.5*(c.FirstParameter()+c.LastParameter()));E.append((e,(p.X(),p.Y(),p.Z())));x.Next()
mk=BRepFilletAPI_MakeFillet(B)
for m in d['mids']:
    mk.Add(r,min(E,key=lambda q:math.dist(q[1],m))[0])
mk.Build()
test(SH,mk.Shape(),r,'occt')
