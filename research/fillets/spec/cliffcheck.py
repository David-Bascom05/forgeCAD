"""cliffcheck.py DIR ID..: each truth cylinder of radius r near a picked edge, its axis's distance
to the carried support planes and to each support face's line edges (rims), less r. A cliff: one
plane and one rim at 0. Carried supports: both planes at 0."""
import sys, json, math
from OCP.STEPControl import STEPControl_Reader
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.GeomAbs import GeomAbs_Cylinder
def dy(a): return a[0] * 2.0 ** a[1]
def vec(a): return [dy(x) for x in a]
def sub(a, b): return [a[i] - b[i] for i in range(3)]
def dot(a, b): return sum(a[i] * b[i] for i in range(3))
def cross(a, b): return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
def norm(a): return math.sqrt(dot(a, a))
def unit(a): n = norm(a); return [x / n for x in a]
def line_line(o1, d1, o2, d2):
    # distance between two parallel lines (axis and rim), else None
    if norm(cross(d1, d2)) > 1e-6: return None
    w = sub(o2, o1); return norm(sub(w, [dot(w, d1) * x for x in d1]))
d = sys.argv[1]
for pid in sys.argv[2:]:
    rd = STEPControl_Reader(); rd.ReadFile(f"{d}/{pid}.truth.step"); rd.TransferRoots(); shape = rd.OneShape()
    cyls = []
    ex = TopExp_Explorer(shape, TopAbs_FACE)
    while ex.More():
        s = BRepAdaptor_Surface(TopoDS.Face(ex.Current()))
        if s.GetType() == GeomAbs_Cylinder:
            c = s.Cylinder(); ax = c.Axis(); o = ax.Location(); dd = ax.Direction()
            cyls.append(([o.X(), o.Y(), o.Z()], unit([dd.X(), dd.Y(), dd.Z()]), c.Radius()))
        ex.Next()
    for l in open(f"{d}/{pid}.sup"):
        j = json.loads(l); m = vec(j["mid"]); t = unit(vec(j["dir"])); rr = j["r"]
        planes = [(vec(f["s"]["o"]), unit(vec(f["s"]["z"]))) for f in j["faces"] if f["s"]["t"] == "plane"]
        rims = [(vec(a), unit(vec(b))) for a, b in j["rims"]]
        for (o, a, rad) in cyls:
            if abs(rad - rr) > 1e-3 * max(1, rr) or norm(cross(a, t)) > 1e-3: continue
            w = sub(m, o)
            if norm(sub(w, [dot(w, a) * x for x in a])) > 2.5 * rr: continue
            pl = [round(abs(dot(sub(o, po), pz)) - rr, 4) for po, pz in planes]
            rm = sorted(round(x - rr, 4) for x in (line_line(o, a, ro, rdir) for ro, rdir in rims) if x is not None)
            print(pid, "edge at", [round(x, 2) for x in m], "planes - r", pl, "nearest rims - r", rm[:3])
