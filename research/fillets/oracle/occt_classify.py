"""occt_classify.py FILE.step FILE.pts: OCCT's side of each point against a solid, as a second
witness to forge-bool's Index. FILE.pts has `x y z [kind]` lines: lost_why's points, or
label_pipe's DUMP (kind over/under/lost, the side Index gave being inside for over, outside for
under and lost). Prints the shape's validity and volume, then one line per point (IN/OUT/ON and
the distance to the shape's faces), then per kind how many points OCCT puts on Index's side."""
import sys
from collections import Counter
from OCP.STEPControl import STEPControl_Reader
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepTools import BRepTools
from OCP.TopoDS import TopoDS
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_IN, TopAbs_OUT, TopAbs_ON, TopAbs_SHELL
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.gp import gp_Pnt

r = STEPControl_Reader()
assert r.ReadFile(sys.argv[1]) == 1
r.TransferRoots()
s = r.OneShape()
g = GProp_GProps()
BRepGProp.VolumeProperties_s(s, g)
print(f"valid {BRepCheck_Analyzer(s).IsValid()}  volume {g.Mass():.4f}")
shells = []
ex = TopExp_Explorer(s, TopAbs_SHELL)
while ex.More():
    shells.append(ex.Current())
    ex.Next()
names = {TopAbs_IN: "IN", TopAbs_OUT: "OUT", TopAbs_ON: "ON"}
tally = Counter()
for line in open(sys.argv[2]):
    w = line.split()
    x, y, z = map(float, w[:3])
    kind = w[3] if len(w) > 3 else "lost"
    p = gp_Pnt(x, y, z)
    st = names.get(BRepClass3d_SolidClassifier(s, p, 1e-7).State(), "?")
    v = BRepBuilderAPI_MakeVertex(p).Vertex()
    d = min(BRepExtrema_DistShapeShape(sh, v).Value() for sh in shells)
    index_in = kind == "over"
    agrees = (st == "IN") == index_in and st != "ON"
    tally[(kind, "agrees with Index" if agrees else "against Index")] += 1
    print(f"({x:.4f}, {y:.4f}, {z:.4f})  {kind}  {st}  dist {d:.4f}")
for k, n in sorted(tally.items()):
    print(f"# {k[0]} {k[1]}: {n}")
