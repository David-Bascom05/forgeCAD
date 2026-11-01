import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp,BRepGProp_Vinert,BRepGProp_Face,BRepGProp_Domain
from OCP.BRepMesh import BRepMesh_IncrementalMesh
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE,TopAbs_REVERSED,TopAbs_VERTEX,TopAbs_WIRE
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.TopLoc import TopLoc_Location
from OCP.BRepAdaptor import BRepAdaptor_Surface,BRepAdaptor_Curve2d
from OCP.BRepTools import BRepTools_WireExplorer
from OCP.gp import gp_Pnt
def ld(i):
    r=STEPControl_Reader(); r.ReadFile('/home/david-bascom/forgecad-data/whucad/run/stp/'+i+'.stp'); r.TransferRoots(); return r.OneShape()
def baryc(s):
    e=TopExp_Explorer(s,TopAbs_VERTEX); x=y=z=0.0; n=0
    while e.More():
        p=BRep_Tool.Pnt_s(TopoDS.Vertex(e.Current())); x+=p.X(); y+=p.Y(); z+=p.Z(); n+=1; e.Next()
    return gp_Pnt(x/n,y/n,z/n)
def uvgap(f):
    W=TopExp_Explorer(f,TopAbs_WIRE); g=0.0; S=BRepAdaptor_Surface(f); g3=0.0
    while W.More():
        w=TopoDS.Wire(W.Current()); W.Next(); x=BRepTools_WireExplorer(w,f); ends=[]
        while x.More():
            E=x.Current(); c=BRepAdaptor_Curve2d(E,f); a,b=c.FirstParameter(),c.LastParameter()
            if E.Orientation()==TopAbs_REVERSED: a,b=b,a
            ends.append((c.Value(a),c.Value(b))); x.Next()
        for k in range(len(ends)):
            p=ends[k][1]; q=ends[(k+1)%len(ends)][0]
            d=p.Distance(q)
            if d>g: g=d
            d3=S.Value(p.X(),p.Y()).Distance(S.Value(q.X(),q.Y()))
            if d3>g3: g3=d3
    return g,g3
def meshface(f,L):
    Lc=TopLoc_Location(); t=BRep_Tool.Triangulation_s(f,Lc)
    if t is None: return None,None
    tr=Lc.Transformation(); rev=f.Orientation()==TopAbs_REVERSED; V=A=0.0
    P=[t.Node(k).Transformed(tr) for k in range(1,t.NbNodes()+1)]; P=[(p.X()-L.X(),p.Y()-L.Y(),p.Z()-L.Z()) for p in P]
    for i in range(1,t.NbTriangles()+1):
        a_,b_,c_=t.Triangle(i).Get(); pa,pb,pc=P[a_-1],P[b_-1],P[c_-1]
        v=(pa[0]*(pb[1]*pc[2]-pb[2]*pc[1])-pa[1]*(pb[0]*pc[2]-pb[2]*pc[0])+pa[2]*(pb[0]*pc[1]-pb[1]*pc[0]))/6
        V+= -v if rev else v
        u=[pb[k]-pa[k] for k in range(3)]; w=[pc[k]-pa[k] for k in range(3)]
        A+=0.5*((u[1]*w[2]-u[2]*w[1])**2+(u[2]*w[0]-u[0]*w[2])**2+(u[0]*w[1]-u[1]*w[0])**2)**.5
    return V,A
s=ld(sys.argv[1]); L=baryc(s); BRepMesh_IncrementalMesh(s,3e-5,False,0.01,False)
rows=[]; e=TopExp_Explorer(s,TopAbs_FACE); k=0; TV=TM=0
while e.More():
    f=TopoDS.Face(e.Current()); e.Next(); k+=1
    BF=BRepGProp_Face(); BF.Load(f); BD=BRepGProp_Domain(); BD.Init(f)
    G=BRepGProp_Vinert(BF,BD,L,1e-9); gv=G.Mass()
    ga=GProp_GProps(); BRepGProp.SurfaceProperties_s(f,ga)
    mv,ma=meshface(f,L); g2,g3=uvgap(f); TV+=gv; TM+=mv
    rows.append((abs(gv-mv),k,BRepAdaptor_Surface(f).GetType(),gv,mv,ga.Mass(),ma,g2,g3,f.Orientation()))
print('total gprop %.9g mesh %.9g'%(TV,TM))
for r in sorted(rows,reverse=True)[:int(sys.argv[2]) if len(sys.argv)>2 else 8]:
    print('face %3d type %s ori %s dV %.3e  gV %.6e mV %.6e  gA %.6f mA %.6f  uvgap %.2e 3dgap %.2e'%(r[1],r[2],r[9],r[3]-r[4],r[3],r[4],r[5],r[6],r[7],r[8]))
