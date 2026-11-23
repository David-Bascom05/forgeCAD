#!/usr/bin/env python3
"""Minimal B-rep inputs for the bugs in real-files.md, made from nothing with OCCT (no dataset
file). Each is printed with OCCT's own volume, the number ours should read.

  $VENV/bin/python -I make_brep.py OUT_DIR
"""
import os, re, sys

from OCP.BRep import BRep_Builder
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace, BRepBuilderAPI_Transform
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeRevol
from OCP.BRepTools import BRepTools
from OCP.GC import GC_MakeCircle
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPCAFControl import STEPCAFControl_Writer
from OCP.STEPControl import STEPControl_AsIs, STEPControl_Writer
from OCP.TCollection import TCollection_ExtendedString
from OCP.TDocStd import TDocStd_Document
from OCP.TopLoc import TopLoc_Location
from OCP.TopoDS import TopoDS_Compound
from OCP.XCAFDoc import XCAFDoc_DocumentTool
from OCP.gp import gp_Ax1, gp_Ax2, gp_Dir, gp_Pnt, gp_Trsf, gp_Vec
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeWire

out = sys.argv[1]
os.makedirs(out, exist_ok=True)


def vol(s):
    g = GProp_GProps()
    BRepGProp.VolumePropertiesGK_s(s, g, 1e-9, False, True)
    return g.Mass()


def step(shape, path):
    w = STEPControl_Writer()
    w.Transfer(shape, STEPControl_AsIs)
    assert w.Write(path) == IFSelect_RetDone


def note(name, shape, what):
    print(f"{name}: OCCT volume {vol(shape):.6f}, valid {BRepCheck_Analyzer(shape).IsValid()}: {what}")


# 1. one part placed twice: an assembly (STEP, through XCAF) and a compound of located copies (.brep)
box = BRepPrimAPI_MakeBox(10.0, 10.0, 10.0).Shape()
t = gp_Trsf()
t.SetTranslation(gp_Vec(30, 0, 0))
moved = box.Moved(TopLoc_Location(t))
c = TopoDS_Compound()
b = BRep_Builder()
b.MakeCompound(c)
b.Add(c, box)
b.Add(c, moved)
BRepTools.Write_s(c, os.path.join(out, "two-located-boxes.brep"))
note("two-located-boxes.brep", c, "one box's TShape twice, the second with a location: 2000 expected")
doc = TDocStd_Document(TCollection_ExtendedString("XmlOcaf"))
tool = XCAFDoc_DocumentTool.ShapeTool_s(doc.Main())
asm = tool.NewShape()
part = tool.AddShape(box, False)
tool.AddComponent(asm, part, TopLoc_Location())
tool.AddComponent(asm, part, TopLoc_Location(t))
tool.UpdateAssemblies()
w = STEPCAFControl_Writer()
w.Transfer(doc, STEPControl_AsIs)
assert w.Write(os.path.join(out, "two-instances.step")) == IFSelect_RetDone
note("two-instances.step", c, "an assembly, one part, two instances (NAUO + item-defined transformation): 2000 expected")

# 2. tori whose major radius is not past the minor one: a horn torus (R = r) and a spindle (R < r)
for name, R, r in (("horn-torus", 10.0, 10.0), ("spindle-torus", 5.0, 8.0)):
    # the disc's half beyond the axis would overlap itself: revolve the half on the far side of it
    circ = GC_MakeCircle(gp_Ax2(gp_Pnt(R, 0, 0), gp_Dir(0, -1, 0), gp_Dir(1, 0, 0)), r).Value()
    from OCP.Geom import Geom_TrimmedCurve
    import math
    # the part of the circle at x >= 0, closed by the axis
    a = math.acos(-R / r) if R < r else math.pi
    arc = Geom_TrimmedCurve(circ, -a, a) if R < r else None
    if R < r:
        e1 = BRepBuilderAPI_MakeEdge(arc).Edge()
        p0, p1 = arc.Value(-a), arc.Value(a)
        e2 = BRepBuilderAPI_MakeEdge(p1, p0).Edge()
        wire = BRepBuilderAPI_MakeWire(e1, e2).Wire()
    else:
        wire = BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(circ).Edge()).Wire()
    face = BRepBuilderAPI_MakeFace(wire, True).Face()
    solid = BRepPrimAPI_MakeRevol(face, gp_Ax1(gp_Pnt(0, 0, 0), gp_Dir(0, 0, 1)), 2 * math.pi).Shape()
    BRepTools.Write_s(solid, os.path.join(out, f"{name}.brep"))
    step(solid, os.path.join(out, f"{name}.step"))
    note(f"{name}.brep/.step", solid, f"revolved about z: a toroidal face of radii {R} and {r}")

# 3. a line whose origin lies far along it (CATIA writes such): valid, the same geometry
p = os.path.join(out, "box.step")
step(box, p)
text = open(p).read()
pts = {m.group(1): m for m in re.finditer(r"#(\d+) = CARTESIAN_POINT\('',\(([^)]*)\)\)", text)}
dirs = {m.group(1): m for m in re.finditer(r"#(\d+) = DIRECTION\('',\(([^)]*)\)\)", text)}
m = re.search(r"#(\d+) = LINE\('',#(\d+),#(\d+)\)", text)
vec = re.search(rf"#{m.group(3)} = VECTOR\('',#(\d+),", text)
d = [float(x) for x in dirs[vec.group(1)].group(2).split(",")]
o = [float(x) for x in pts[m.group(2)].group(2).split(",")]
far = [o[k] + 2e6 * d[k] for k in range(3)]
old = pts[m.group(2)].group(0)
new = old.replace(pts[m.group(2)].group(2), ",".join(f"{x:.6f}" for x in far))
text = text.replace(old, new)
open(os.path.join(out, "far-line-origin.step"), "w").write(text)
print(f"far-line-origin.step: box.step with LINE #{m.group(1)}'s origin moved 2 km along it: 1000 expected")
