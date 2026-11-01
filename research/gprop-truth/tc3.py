# tc3.py IDS OUT: why BRepGProp and the mesh of a WHUCAD STEP disagree. One JSON line per part.
import sys,json,os,math
from OCP.STEPControl import STEPControl_Reader
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp,BRepGProp_Vinert,BRepGProp_Sinert,BRepGProp_Face,BRepGProp_Domain
from OCP.BRepMesh import BRepMesh_IncrementalMesh
from OCP.BRepTools import BRepTools,BRepTools_WireExplorer
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE,TopAbs_REVERSED,TopAbs_VERTEX,TopAbs_WIRE,TopAbs_SOLID,TopAbs_SHELL,TopAbs_EDGE
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.TopLoc import TopLoc_Location
from OCP.BRepAdaptor import BRepAdaptor_Surface,BRepAdaptor_Curve2d,BRepAdaptor_Curve
from OCP.gp import gp_Pnt
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.ShapeAnalysis import ShapeAnalysis_FreeBounds
D='/home/david-bascom/forgecad-data/whucad/run/stp/'
def baryc(s):
    e=TopExp_Explorer(s,TopAbs_VERTEX); x=y=z=0.0; n=0
    while e.More():
        p=BRep_Tool.Pnt_s(TopoDS.Vertex(e.Current())); x+=p.X(); y+=p.Y(); z+=p.Z(); n+=1; e.Next()
    return (x/n,y/n,z/n)
def faces(s):
    e=TopExp_Explorer(s,TopAbs_FACE); r=[]
    while e.More(): r.append(TopoDS.Face(e.Current())); e.Next()
    return r
def gvol(F,L):
    # per-face volume integrals at reference L; returns sum
    V=0.0
    for f in F:
        BF=BRepGProp_Face(); BF.Load(f)
        if f.NbChildren()==0: G=BRepGProp_Vinert(BF,gp_Pnt(*L))
        else:
            BD=BRepGProp_Domain(); BD.Init(f); G=BRepGProp_Vinert(BF,BD,gp_Pnt(*L))
        V+=G.Mass()
    return V
def mesh(s,F,d,a,L0,L1):
    BRepTools.Clean_s(s); BRepMesh_IncrementalMesh(s,d,False,a,False)
    V=[0.0,0.0]; A=0.0; miss=0
    for f in F:
        Lc=TopLoc_Location(); t=BRep_Tool.Triangulation_s(f,Lc)
        if t is None or t.NbTriangles()==0: miss+=1; continue
        tr=Lc.Transformation(); rev=f.Orientation()==TopAbs_REVERSED
        P=[t.Node(k).Transformed(tr) for k in range(1,t.NbNodes()+1)]; P=[(p.X(),p.Y(),p.Z()) for p in P]
        for i in range(1,t.NbTriangles()+1):
            a_,b_,c_=t.Triangle(i).Get(); pa,pb,pc=P[a_-1],P[b_-1],P[c_-1]
            u=[pb[k]-pa[k] for k in range(3)]; w=[pc[k]-pa[k] for k in range(3)]
            n=(u[1]*w[2]-u[2]*w[1],u[2]*w[0]-u[0]*w[2],u[0]*w[1]-u[1]*w[0])
            A+=0.5*math.sqrt(n[0]**2+n[1]**2+n[2]**2)
            for j,L in enumerate((L0,L1)):
                v=sum((pa[k]-L[k])*n[k] for k in range(3))/6
                V[j]+= -v if rev else v
    return V,A,miss
def edgedev(F):
    # how far each pcurve's image lies from its 3D edge, against the edge tolerance; uv loop gaps mapped to 3D
    worst=0.0; worst_rel=0.0; gap3=0.0; gapuv=0.0; nsp=0
    for f in F:
        S=BRepAdaptor_Surface(f); W=TopExp_Explorer(f,TopAbs_WIRE)
        while W.More():
            w=TopoDS.Wire(W.Current()); W.Next(); x=BRepTools_WireExplorer(w,f); ends=[]
            while x.More():
                E=x.Current(); x.Next()
                if BRep_Tool.Degenerated_s(E):
                    c=BRepAdaptor_Curve2d(E,f); a,b=c.FirstParameter(),c.LastParameter()
                    if E.Orientation()==TopAbs_REVERSED: a,b=b,a
                    ends.append((c.Value(a),c.Value(b))); continue
                c=BRepAdaptor_Curve2d(E,f); a,b=c.FirstParameter(),c.LastParameter()
                if not BRep_Tool.SameParameter_s(E): nsp+=1
                try:
                    c3=BRepAdaptor_Curve(E,f) if False else BRepAdaptor_Curve(E)
                    tol=BRep_Tool.Tolerance_s(E)
                    for j in range(9):
                        t=a+(b-a)*(j+.5)/9; p=c.Value(t); dd=S.Value(p.X(),p.Y()).Distance(c3.Value(t))
                        worst=max(worst,dd); worst_rel=max(worst_rel,dd/max(tol,1e-7))
                except Exception: pass
                if E.Orientation()==TopAbs_REVERSED: a,b=b,a
                ends.append((c.Value(a),c.Value(b)))
            for k in range(len(ends)):
                p=ends[k][1]; q=ends[(k+1)%len(ends)][0]
                gapuv=max(gapuv,p.Distance(q)); gap3=max(gap3,S.Value(p.X(),p.Y()).Distance(S.Value(q.X(),q.Y())))
    return worst,worst_rel,gapuv,gap3,nsp
def maxtol(s,T):
    e=TopExp_Explorer(s,T); m=0.0
    while e.More():
        x=e.Current(); e.Next()
        m=max(m,BRep_Tool.Tolerance_s(TopoDS.Edge(x)) if T==TopAbs_EDGE else BRep_Tool.Tolerance_s(TopoDS.Vertex(x)))
    return m
def cnt(s,T):
    e=TopExp_Explorer(s,T); n=0
    while e.More(): n+=1; e.Next()
    return n
out=open(sys.argv[2],'a'); done={json.loads(l)['id'] for l in open(sys.argv[2])} if os.path.getsize(sys.argv[2]) else set()
for i in open(sys.argv[1]).read().split():
    if i in done: continue
    try:
        r=STEPControl_Reader(); r.ReadFile(D+i+'.stp'); r.TransferRoots(); s=r.OneShape()
        bb=Bnd_Box(); BRepBndLib.Add_s(s,bb); lo,hi=bb.CornerMin(),bb.CornerMax(); dg=lo.Distance(hi)
        F=faces(s); L0=baryc(s)
        g=GProp_GProps(); BRepGProp.VolumeProperties_s(s,g); gv=g.Mass()
        ga=GProp_GProps(); BRepGProp.SurfaceProperties_s(s,ga)
        v0=gvol(F,L0); vx=gvol(F,(L0[0]+dg,L0[1],L0[2])); vy=gvol(F,(L0[0],L0[1]+dg,L0[2])); vz=gvol(F,(L0[0],L0[1],L0[2]+dg))
        cvec=[3*(v0-vx)/dg,3*(v0-vy)/dg,3*(v0-vz)/dg]   # gprop's vector area sum N dA: zero for a closed set of faces
        ed=edgedev(F)
        fb=ShapeAnalysis_FreeBounds(s); nfree=cnt(fb.GetClosedWires(),TopAbs_EDGE)+cnt(fb.GetOpenWires(),TopAbs_EDGE)
        L1=(L0[0]+dg,L0[1]+dg,L0[2]+dg)
        (m1,m1b),ma1,miss1=mesh(s,F,5e-4,0.1,L0,L1)
        (m2,m2b),ma2,miss2=mesh(s,F,1e-4,0.03,L0,L1)
        d=dict(id=i,diag=dg,gv=gv,ga=ga.Mass(),v0=v0,cvec=cvec,cnorm=math.sqrt(sum(c*c for c in cvec)),
               pdev=ed[0],pdevrel=ed[1],gapuv=ed[2],gap3=ed[3],notsame=ed[4],free=nfree,
               mv1=m1,mv1shift=m1b,ma1=ma1,miss1=miss1,mv2=m2,mv2shift=m2b,ma2=ma2,miss2=miss2,
               solids=cnt(s,TopAbs_SOLID),shells=cnt(s,TopAbs_SHELL),faces=len(F),valid=BRepCheck_Analyzer(s).IsValid(),
               etol=maxtol(s,TopAbs_EDGE),vtol=maxtol(s,TopAbs_VERTEX))
    except Exception as x:
        d=dict(id=i,err=repr(x)[:100])
    out.write(json.dumps(d)+'\n'); out.flush()
