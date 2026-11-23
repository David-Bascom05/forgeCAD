"""blendaxis.py DIR ID..: for each picked edge (DIR/ID.sup), the truth's cylinders of the edge's
radius near it, and each axis's distance to the two carried support planes, less r. (c): both 0."""
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
d = sys.argv[1]
for pid in sys.argv[2:]:
    r = STEPControl_Reader(); r.ReadFile(f"{d}/{pid}.truth.step"); r.TransferRoots(); shape = r.OneShape()
    cyls = []
    ex = TopExp_Explorer(shape, TopAbs_FACE)
    while ex.More():
        s = BRepAdaptor_Surface(TopoDS.Face(ex.Current()))
        if s.GetType() == GeomAbs_Cylinder:
            c = s.Cylinder(); ax = c.Axis(); o = ax.Location(); dd = ax.Direction()
            cyls.append(([o.X(), o.Y(), o.Z()], [dd.X(), dd.Y(), dd.Z()], c.Radius()))
        ex.Next()
    seen = set()
    for l in open(f"{d}/{pid}.sup"):
        j = json.loads(l); m = vec(j["mid"]); t = vec(j["dir"]); t = [x / norm(t) for x in t]; rr = j["r"]
        planes = [f for f in j["faces"] if f["s"]["t"] == "plane"]
        best = None
        for (o, a, rad) in cyls:
            if abs(rad - rr) > 1e-3 * max(1, rr): continue
            if norm(cross(a, t)) > 1e-3: continue
            w = sub(m, o); da = norm(sub(w, [dot(w, a) * x for x in a]))
            if da > 2.5 * rr: continue
            offs = []
            for pl in planes:
                po = vec(pl["s"]["o"]); pz = vec(pl["s"]["z"]); pz = [x / norm(pz) for x in pz]
                offs.append(abs(dot(sub(o, po), pz)) - rr)
            cand = (max(abs(x) for x in offs) if offs else 9e9, da, offs)
            if best is None or cand[0] < best[0]: best = cand
        key = (pid, round(m[0], 1), round(m[1], 1), round(m[2], 1))
        if key in seen: continue
        seen.add(key)
        print(pid, "r", round(rr, 4), "convex", j["convex"], "planes", len(planes),
              "truth cylinder:", "none" if best is None else f"axis {best[1]:.4f} from edge, |dist to carried planes| - r = {[round(x, 6) for x in best[2]]}")
