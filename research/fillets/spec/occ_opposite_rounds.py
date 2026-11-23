import sys, math
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_EDGE
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.BRepCheck import BRepCheck_Analyzer
r = STEPControl_Reader(); r.ReadFile(sys.argv[1]); r.TransferRoots(); s = r.OneShape()
def vol(x): g = GProp_GProps(); BRepGProp.VolumeProperties_s(x, g); return g.Mass()
print("before", vol(s))
# the top edge on the far side: the line at y 20, z 20 along x
ex = TopExp_Explorer(s, TopAbs_EDGE); pick = None
while ex.More():
    e = TopoDS.Edge(ex.Current()); c = BRepAdaptor_Curve(e); q = c.Value(0.5 * (c.FirstParameter() + c.LastParameter()))
    if abs(q.Y() - 20) < 1e-6 and abs(q.Z() - 20) < 1e-6: pick = e
    ex.Next()
for rr in [5.0, 8.0, 8.9, 9.0, 12.0]:
    mk = BRepFilletAPI_MakeFillet(s); mk.Add(rr, pick)
    try:
        mk.Build(); ok = mk.IsDone()
    except Exception as err:
        ok = False
    if ok:
        t = mk.Shape(); print(rr, "OCCT builds", "valid" if BRepCheck_Analyzer(t).IsValid() else "INVALID", vol(t))
    else:
        print(rr, "OCCT refuses")
