# How far OpenCascade carries a fillet past a kink: a block whose back edge is a line meeting a quarter arc,
# turned k degrees outward (inward, sign -1) where they meet; fillet 2 on the line alone. Prints k, the
# contour's edge count (3: the arc and the line after it joined; 1: the line alone), validity, volume.
#   python3 research/blend-ends/occt_tol.py      (the OCP in the user site; output in occt_tol.out, 2026-11-03)
import math
from OCP.gp import gp_Pnt, gp_Vec, gp_Circ, gp_Ax2, gp_Dir
from OCP.GC import GC_MakeArcOfCircle, GC_MakeSegment
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeWire, BRepBuilderAPI_MakeFace
from OCP.BRepPrimAPI import BRepPrimAPI_MakePrism
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_EDGE
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
def part(k, sign):
    y0=20+sign*20*math.tan(math.radians(k))
    P=lambda x,y: gp_Pnt(x,y,0)
    w=BRepBuilderAPI_MakeWire()
    for a,b in [((0,y0),(20,20))]: w.Add(BRepBuilderAPI_MakeEdge(GC_MakeSegment(P(*a),P(*b)).Value()).Edge())
    arc=GC_MakeArcOfCircle(P(20,20),P(20+10*math.cos(math.pi/4),10+10*math.sin(math.pi/4)),P(30,10)).Value()
    w.Add(BRepBuilderAPI_MakeEdge(arc).Edge())
    for a,b in [((30,10),(30,0)),((30,0),(0,0)),((0,0),(0,y0))]: w.Add(BRepBuilderAPI_MakeEdge(GC_MakeSegment(P(*a),P(*b)).Value()).Edge())
    f=BRepBuilderAPI_MakeFace(w.Wire()).Face()
    s=BRepPrimAPI_MakePrism(f,gp_Vec(0,0,10)).Shape()
    tgt=gp_Pnt(10,(y0+20)/2,10); best=None
    ex=TopExp_Explorer(s,TopAbs_EDGE)
    while ex.More():
        e=TopoDS.Edge_s(ex.Current()); a=BRepAdaptor_Curve(e); m=a.Value((a.FirstParameter()+a.LastParameter())/2)
        d=m.Distance(tgt)
        if best is None or d<best[0]: best=(d,e)
        ex.Next()
    mk=BRepFilletAPI_MakeFillet(s); mk.Add(2.0,best[1])
    n=mk.NbEdges(1)
    try:
        mk.Build(); ok=mk.IsDone()
    except Exception: ok=False
    v=None
    if ok:
        g=GProp_GProps(); BRepGProp.VolumeProperties_s(mk.Shape(),g); v=g.Mass(); ok = 'valid' if BRepCheck_Analyzer(mk.Shape()).IsValid() else 'INVALID'
    return n, ok, v
for sign,name in [(1,'outward'),(-1,'inward')]:
    for k in [0, 0.01, 0.03, 0.1, 0.3, 0.5, 1, 3, 5.5, 5.72, 5.74, 6, 8]:
        print(name, k, *part(k,sign))
