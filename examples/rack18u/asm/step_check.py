#!/usr/bin/env python3
"""Reads STEP files back with OpenCascade (OCP, the crosscheck venv) and prints one JSON line per file:
solids, valid (BRepCheck), volume, bbox. Run with ~/.local/share/forgecad-occt/venv/bin/python -I.
usage: step_check.py FILE.step .."""
import json
import sys

from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer


def check(path):
    r = STEPControl_Reader()
    if r.ReadFile(path) != IFSelect_RetDone:
        return {"file": path, "read": False}
    r.TransferRoots()
    shape = r.OneShape()
    solids = 0
    ex = TopExp_Explorer(shape, TopAbs_SOLID)
    while ex.More():
        solids += 1
        ex.Next()
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, g)
    b = Bnd_Box()
    BRepBndLib.Add_s(shape, b)
    lo, hi = b.CornerMin(), b.CornerMax()
    x0, y0, z0, x1, y1, z1 = lo.X(), lo.Y(), lo.Z(), hi.X(), hi.Y(), hi.Z()
    return {"file": path, "read": True, "solids": solids, "valid": BRepCheck_Analyzer(shape).IsValid(),
            "volume": g.Mass(), "bbox": [x1 - x0, y1 - y0, z1 - z0]}


for p in sys.argv[1:]:
    print(json.dumps(check(p)))
