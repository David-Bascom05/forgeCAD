# occ_pipe.py FILE.fc [r]: body + boss in OCCT, fillet the edges between them, report.
import sys, re, json, math
from OCP.gp import gp_Pnt, gp_Dir, gp_Ax2, gp_Vec, gp_Ax3, gp_Trsf
from OCP.BRepPrimAPI import BRepPrimAPI_MakeCylinder, BRepPrimAPI_MakeBox
from OCP.BRepAlgoAPI import BRepAlgoAPI_Fuse
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.TopExp import TopExp_Explorer, TopExp
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform


def vol(s):
    g = GProp_GProps(); BRepGProp.VolumeProperties_s(s, g); return g.Mass()


def unit(v):
    n = math.sqrt(sum(c * c for c in v)); return tuple(c / n for c in v)


def solid(line):
    m = re.search(r"extrude (?:add )?([-\d.e]+) z (\S+) (\S+) (\S+) x (\S+) (\S+) (\S+) at (\S+) (\S+) (\S+) (circle|rect) (.*)", line)
    L = float(m.group(1)); z = [float(m.group(i)) for i in (2, 3, 4)]; x = [float(m.group(i)) for i in (5, 6, 7)]
    at = [float(m.group(i)) for i in (8, 9, 10)]; a = [float(t) for t in m.group(12).split()]
    ax = gp_Ax2(gp_Pnt(*at), gp_Dir(*z), gp_Dir(*x))
    if m.group(11) == "circle":
        ax2 = gp_Ax2(gp_Pnt(*[at[k] + x[k] * a[0] + 0 for k in range(3)]), gp_Dir(*z), gp_Dir(*x))
        y = [z[1] * x[2] - z[2] * x[1], z[2] * x[0] - z[0] * x[2], z[0] * x[1] - z[1] * x[0]]
        o = [at[k] + x[k] * a[0] + y[k] * a[1] for k in range(3)]
        return BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(*o), gp_Dir(*z), gp_Dir(*x)), a[2], L).Shape()
    y = [z[1] * x[2] - z[2] * x[1], z[2] * x[0] - z[0] * x[2], z[0] * x[1] - z[1] * x[0]]
    o = [at[k] + x[k] * a[0] + y[k] * a[1] for k in range(3)]
    return BRepPrimAPI_MakeBox(gp_Ax2(gp_Pnt(*o), gp_Dir(*z), gp_Dir(*x)), a[2], a[3], L).Shape()


lines = [l for l in open(sys.argv[1]) if not l.startswith("#")]
body = solid(lines[0]); boss = solid(lines[1])
r = float(sys.argv[2]) if len(sys.argv) > 2 else float(re.search(r"fillet ([-\d.e]+)", lines[2]).group(1))
fu = BRepAlgoAPI_Fuse(body, boss); fu.Build(); s = fu.Shape()
# faces from boss: modified/generated
bossf = []
x = TopExp_Explorer(boss, TopAbs_FACE)
while x.More():
    for f in fu.Modified(x.Current()):
        bossf.append(f)
    if fu.IsDeleted(x.Current()) is False and fu.Modified(x.Current()).Size() == 0:
        bossf.append(x.Current())
    x.Next()
bodyf = []
x = TopExp_Explorer(body, TopAbs_FACE)
while x.More():
    for f in fu.Modified(x.Current()):
        bodyf.append(f)
    if fu.Modified(x.Current()).Size() == 0:
        bodyf.append(x.Current())
    x.Next()
faces = []
x = TopExp_Explorer(s, TopAbs_FACE)
while x.More():
    faces.append(x.Current()); x.Next()
fe = []
for f in faces:
    l = []
    y = TopExp_Explorer(f, TopAbs_EDGE)
    while y.More():
        l.append(y.Current()); y.Next()
    fe.append(l)
mk = BRepFilletAPI_MakeFillet(s)
n = 0
info = []
seen = []
x = TopExp_Explorer(s, TopAbs_EDGE)
while x.More():
    e = x.Current(); x.Next()
    if any(e.IsSame(q) for q in seen): continue
    seen.append(e)
    fs = [faces[i] for i in range(len(faces)) if any(e.IsSame(q) for q in fe[i])]
    if len(fs) != 2: continue
    a = any(fs[0].IsSame(f) for f in bossf); b = any(fs[1].IsSame(f) for f in bossf)
    c = any(fs[0].IsSame(f) for f in bodyf); d = any(fs[1].IsSame(f) for f in bodyf)
    if (a and d) or (b and c):
        mk.Add(r, TopoDS.Edge(e)); n += 1
        v1 = BRep_Tool.Pnt_s(TopExp.FirstVertex_s(TopoDS.Edge(e))); v2 = BRep_Tool.Pnt_s(TopExp.LastVertex_s(TopoDS.Edge(e)))
        info.append([round(v1.X(), 3), round(v1.Y(), 3), round(v1.Z(), 3), round(v2.X(), 3), round(v2.Y(), 3), round(v2.Z(), 3)])
out = {"edges": n, "vb": round(vol(s), 4), "ends": info}
try:
    mk.Build()
    if not mk.IsDone():
        raise Exception("notdone")
    t = mk.Shape()
    out["valid"] = BRepCheck_Analyzer(t).IsValid(); out["va"] = vol(t); out["add"] = out["va"] - vol(s)
except Exception as e:
    out["fail"] = str(e)[:80]
# occ_grid.py FILE.fc R STEP: OCCT's round less the body, its volume by GProp against a grid of
# classified points over its box (in the round's result and outside the body), the grid's step STEP
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.TopAbs import TopAbs_IN
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
add = BRepAlgoAPI_Cut(t, s).Shape()
print("cut volume", vol(add))
b = Bnd_Box(); BRepBndLib.Add_s(add, b)
p0, p1 = b.CornerMin(), b.CornerMax()
h = float(sys.argv[3])
ct, cs = BRepClass3d_SolidClassifier(t), BRepClass3d_SolidClassifier(s)
n = 0
xs = [p0.X() + h * (i + 0.5) for i in range(int((p1.X() - p0.X()) / h) + 1)]
ys = [p0.Y() + h * (i + 0.5) for i in range(int((p1.Y() - p0.Y()) / h) + 1)]
zs = [p0.Z() + h * (i + 0.5) for i in range(int((p1.Z() - p0.Z()) / h) + 1)]
ca = BRepClass3d_SolidClassifier(add)
for x in xs:
    for y in ys:
        for z in zs:
            p = gp_Pnt(x, y, z)
            ca.Perform(p, 1e-7)
            if ca.State() != TopAbs_IN:
                continue
            ct.Perform(p, 1e-7); cs.Perform(p, 1e-7)
            if ct.State() == TopAbs_IN and cs.State() != TopAbs_IN:
                n += 1
print("box", [round(v, 2) for v in (p0.X(), p0.Y(), p0.Z(), p1.X(), p1.Y(), p1.Z())], "grid volume of the cut", n * h ** 3)
