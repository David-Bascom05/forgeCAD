"""occ_judge.py DIR DUMP ID..: OCCT's own fillet of DIR/ID.before.step on the edges whose midpoints
are in DIR/ID.mids, and OCCT's classifier at the oracle's disagreement points DUMP/ID.pts, against
OCCT's result and against ours (DIR/ID.after.step)."""
import sys, json
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_EDGE, TopAbs_IN, TopAbs_ON
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.gp import gp_Pnt

def load(p):
    r = STEPControl_Reader(); r.ReadFile(p); r.TransferRoots(); return r.OneShape()

def vol(s):
    g = GProp_GProps(); BRepGProp.VolumeProperties_s(s, g); return g.Mass()

d, dump = sys.argv[1], sys.argv[2]
for pid in sys.argv[3:]:
    before = load(f"{d}/{pid}.before.step"); ours = load(f"{d}/{pid}.after.step")
    mids = [list(map(float, l.split())) for l in open(f"{d}/{pid}.mids")]
    r = mids[0][1]
    mk = BRepFilletAPI_MakeFillet(before)
    ex = TopExp_Explorer(before, TopAbs_EDGE); n = 0; seen = []
    while ex.More():
        e = TopoDS.Edge(ex.Current()); c = BRepAdaptor_Curve(e)
        q = c.Value(0.5 * (c.FirstParameter() + c.LastParameter()))
        for m in mids:
            if (q.X()-m[2])**2 + (q.Y()-m[3])**2 + (q.Z()-m[4])**2 < 1e-6 and not any(e.IsSame(s) for s in seen):
                mk.Add(r, e); seen.append(e); n += 1
        ex.Next()
    out = {"id": pid, "edges": n, "of": len(mids)}
    try:
        mk.Build()
        occ = mk.Shape() if mk.IsDone() else None
    except Exception as err:
        occ = None; out["occt_error"] = str(err)[:80]
    pts = [l.split() for l in open(f"{dump}/{pid}.pts")]
    def inside(shape, x):
        cl = BRepClass3d_SolidClassifier(shape, gp_Pnt(*x), 1e-7); return cl.State()
    o_in = sum(1 for p in pts if inside(ours, map(float, p[:3])) == TopAbs_IN)
    out.update({"over_points": len(pts), "ours_inside": o_in, "vol_ours": vol(ours), "vol_before": vol(before)})
    if occ is not None:
        out.update({"occt_valid": BRepCheck_Analyzer(occ).IsValid(), "vol_occt": vol(occ),
                    "occt_inside": sum(1 for p in pts if inside(occ, map(float, p[:3])) == TopAbs_IN),
                    "occt_on": sum(1 for p in pts if inside(occ, map(float, p[:3])) == TopAbs_ON)})
    print(json.dumps(out), flush=True)
