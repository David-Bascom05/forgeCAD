#!/usr/bin/env python3
"""A design's final STEP volume as OCCT reads it, beside the volume the dataset records: settles whether
a wrong volume is ours or the dataset's. Run with the OCCT venv, python -I; the STEP stays in a temp dir.
  stepvol.py PARTS_DIR ZIP NAME ..      prints NAME, STEP mm3, recorded mm3
"""
import json
import os
import sys
import tempfile
import zipfile

from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.STEPControl import STEPControl_Reader

parts, archive, names = sys.argv[1], sys.argv[2], sys.argv[3:]
z = zipfile.ZipFile(archive)
with tempfile.TemporaryDirectory() as d:
    for n in names:
        p = os.path.join(d, n + ".step")
        with open(p, "wb") as f:
            f.write(z.read(f"r1.0.1/reconstruction/{n}.step"))
        r = STEPControl_Reader()
        r.ReadFile(p)
        r.TransferRoots()
        g = GProp_GProps()
        BRepGProp.VolumeProperties_s(r.OneShape(), g)
        print(n, round(g.Mass(), 4), round(json.load(open(os.path.join(parts, n + ".json")))["volume"], 4))
