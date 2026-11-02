# occt_fillet.py PREFIX: for each PREFIX*.json + .step (a body before a fillet, the fillet edges by midpoint, radius, points on our blends with their audit gap) builds OCCT's own fillet and prints how far our points lie from it, supported and carried-on apart. Run with the OCCT venv, tools/occt/setup.sh.
import json,sys,os,glob
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_EDGE,TopAbs_SOLID,TopAbs_IN,TopAbs_OUT,TopAbs_ON
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.gp import gp_Pnt
from OCP.BRepCheck import BRepCheck_Analyzer
def vol(s):
    g=GProp_GProps();BRepGProp.VolumeProperties_s(s,g);return g.Mass()
for j in sorted(glob.glob(sys.argv[1]+'*.json')):
    t=j[:-5];d=json.load(open(j))
    R=STEPControl_Reader();R.ReadFile(t+'.step');R.TransferRoots();shape=R.OneShape()
    edges=[]
    x=TopExp_Explorer(shape,TopAbs_EDGE)
    while x.More():
        e=TopoDS.Edge(x.Current());c=BRepAdaptor_Curve(e);p=c.Value(0.5*(c.FirstParameter()+c.LastParameter()));edges.append((e,(p.X(),p.Y(),p.Z())));x.Next()
    mk=BRepFilletAPI_MakeFillet(shape);n=0
    for m in d['mids']:
        best=min(edges,key=lambda E:sum((a-b)**2 for a,b in zip(E[1],m)))
        if sum((a-b)**2 for a,b in zip(best[1],m))<25.0:
            mk.Add(d['r'],best[0]);n+=1
    out={'case':os.path.basename(t),'edges':n,'of':len(d['mids'])}
    try:
        mk.Build()
        if not mk.IsDone(): raise Exception('notdone')
        s=mk.Shape()
        out['valid']=BRepCheck_Analyzer(s).IsValid();out['vb_occt']=vol(shape);out['va_occt']=vol(s);out['add_occt']=out['va_occt']-out['vb_occt'];out['add_ours']=d['va']-d['vb']
        D=[]
        for p in d['pts']:
            v=BRepBuilderAPI_MakeVertex(gp_Pnt(*p[:3])).Vertex()
            e=BRepExtrema_DistShapeShape(v,s);e.Perform();D.append((p[3],e.Value()))
        D.sort(reverse=True)
        import math
        E=d.get('ends') or []
        near=lambda p:min((math.dist(p[:3],q) for q in E),default=-1)
        far=[]
        for p in d['pts']:
            v=BRepBuilderAPI_MakeVertex(gp_Pnt(*p[:3])).Vertex()
            e=BRepExtrema_DistShapeShape(v,s);e.Perform()
            if e.Value()>1e-3: far.append(round(near(p)/d['r'],2))
        G=[b for a,b in D if a<=5e-6]; B=[(a,b) for a,b in D if a>5e-6]
        out={'case':out['case'],'edges':out['edges'],'add_occt':round(out['add_occt'],2),'add_ours':round(out['add_ours'],2),'valid':out['valid'],'n_sup':len(G),'sup_max_dist':round(max(G),5) if G else None,'sup_gt_1e3':sum(1 for b in G if b>1e-3),'n_carried':len(B),'carried_max_dist':round(max([b for a,b in B]),4) if B else None,'carried_gt_01':sum(1 for a,b in B if b>0.1),'off_gt_1e3_vertex_dist_over_r':sorted(far)}
    except Exception as e:
        out['fail']=str(e)[:80]
    print(json.dumps(out),flush=True)
