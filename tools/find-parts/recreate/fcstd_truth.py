#!/usr/bin/env python3
"""What an .FCStd stores of each object's result, read as data by OpenCascade (FreeCAD is never run, and
nothing in the file executes): volume, area, box, and samples of the edges and faces a fillet or chamfer names.

  $VENV/bin/python -I fcstd_truth.py FILE.FCStd OBJ.json     OBJ.json from freecad.py: {"shapes": {name: file}, "subs": {name: [Edge3, Face2..]}}
"""
import json
import os
import resource
import sys
import tempfile
import zipfile

resource.setrlimit(resource.RLIMIT_AS, (3 << 30, 3 << 30))
from OCP.Bnd import Bnd_Box
from OCP.BRep import BRep_Builder
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepTools import BRepTools
from OCP.GProp import GProp_GProps
from OCP.OCP.collections import IndexedMap_TopoDS_Shape_TopTools_ShapeMapHasher as ShapeMap
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_SOLID
from OCP.TopExp import TopExp, TopExp_Explorer
from OCP.TopoDS import TopoDS_Shape

N = 256


def read(path):
    s = TopoDS_Shape()
    if not BRepTools.Read_s(s, path, BRep_Builder()):
        return None
    return s


def edge_points(e):
    c = BRepAdaptor_Curve(e)
    a, b = c.FirstParameter(), c.LastParameter()
    out = []
    for i in range(N + 1):
        p = c.Value(a + (b - a) * i / N)
        out.append([p.X(), p.Y(), p.Z()])
    return out


def main():
    want = json.load(open(sys.argv[2]))
    z = zipfile.ZipFile(sys.argv[1])
    tmp = tempfile.mkdtemp(prefix="brp-")
    out = {}
    try:
        for name, f in want["shapes"].items():
            p = os.path.join(tmp, "s.brp")
            with open(p, "wb") as w:
                w.write(z.read(f))
            s = read(p)
            if s is None or s.IsNull():
                out[name] = {"err": "unreadable"}
                continue
            r = {"solids": 0, "vol": 0.0, "area": 0.0}
            x = TopExp_Explorer(s, TopAbs_SOLID)
            box = Bnd_Box()
            while x.More():
                g = GProp_GProps()
                BRepGProp.VolumeProperties_s(x.Current(), g)
                r["vol"] += abs(g.Mass())
                g = GProp_GProps()
                BRepGProp.SurfaceProperties_s(x.Current(), g)
                r["area"] += g.Mass()
                BRepBndLib.AddOptimal_s(x.Current(), box, True, False)
                r["solids"] += 1
                x.Next()
            if r["solids"]:
                a, b = box.CornerMin(), box.CornerMax()
                r["bbox"] = [a.X(), a.Y(), a.Z(), b.X(), b.Y(), b.Z()]
            r["valid"] = bool(BRepCheck_Analyzer(s).IsValid())
            subs = want["subs"].get(name)
            if subs:
                em, fm = ShapeMap(), ShapeMap()
                TopExp.MapShapes_s(s, TopAbs_EDGE, em)
                TopExp.MapShapes_s(s, TopAbs_FACE, fm)
                got = {}
                for sub in subs:
                    if sub == "ALL":
                        got[sub] = [edge_points(em.FindKey(i)) for i in range(1, em.Extent() + 1)]
                    elif sub.startswith("Edge"):
                        got[sub] = [edge_points(em.FindKey(int(sub[4:])))]
                    elif sub.startswith("Face"):
                        fe = ShapeMap()
                        TopExp.MapShapes_s(fm.FindKey(int(sub[4:])), TopAbs_EDGE, fe)
                        got[sub] = [edge_points(fe.FindKey(i)) for i in range(1, fe.Extent() + 1)]
                r["subs"] = got
            out[name] = r
    finally:
        for n in os.listdir(tmp):
            os.remove(os.path.join(tmp, n))
        os.rmdir(tmp)
    print(json.dumps(out))


if __name__ == "__main__":
    main()
