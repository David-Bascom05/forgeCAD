#!/usr/bin/env python3
"""OCCT's truth for one STEP file, measured under the truth rule (research/import/truth.md, the rule's code in
tools/find-parts/recreate/truth_rule.py on claude/repo-review-mbdrlj-import): per solid the default volume and
area, the same with a precision of 1e-9, and the Gauss-Kronrod volume at 1e-9; summed over all solids and over
the distinct ones (a solid placed twice shares its TShape). Also valid, counts and box. One JSON line on stdout.

  $VENV/bin/python -I occt_truth.py FILE

Run by scale.py, one process per file with its own time and memory cap. The file is data: nothing in it runs.
"""
import json
import resource
import sys
import time

EPS = 1e-9
MEM = 4 << 30


def main(path):
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))
    from OCP.Bnd import Bnd_Box
    from OCP.BRepBndLib import BRepBndLib
    from OCP.BRepCheck import BRepCheck_Analyzer
    from OCP.BRepGProp import BRepGProp
    from OCP.GProp import GProp_GProps
    from OCP.IFSelect import IFSelect_RetDone
    from OCP.STEPControl import STEPControl_Reader
    from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopoDS import TopoDS

    t0 = time.time()
    r = STEPControl_Reader()
    if r.ReadFile(path) != IFSelect_RetDone:
        print(json.dumps({"err": "unreadable"}))
        return
    r.TransferRoots()
    s = r.OneShape()
    t_read = time.time() - t0

    def one(f, solid, *a):
        g = GProp_GProps()
        f(solid, g, *a)
        return g.Mass()

    keys = ("vol", "area", "vol_eps", "area_eps", "vol_gk")
    tot = dict.fromkeys(keys, 0.0)
    dist = dict.fromkeys(keys, 0.0)
    seen = []
    n = nd = faces = 0
    ex = TopExp_Explorer(s, TopAbs_SOLID)
    while ex.More():
        so = TopoDS.Solid(ex.Current())
        m = {
            "vol": abs(one(BRepGProp.VolumeProperties_s, so)),
            "area": one(BRepGProp.SurfaceProperties_s, so),
            "vol_eps": abs(one(BRepGProp.VolumeProperties_s, so, EPS)),
            "area_eps": one(BRepGProp.SurfaceProperties_s, so, EPS),
            "vol_gk": abs(one(BRepGProp.VolumePropertiesGK_s, so, EPS, False, True)),
        }
        for k in keys:
            tot[k] += m[k]
        if not any(so.IsPartner(x) for x in seen[:2000]):
            seen.append(so)
            nd += 1
            for k in keys:
                dist[k] += m[k]
        n += 1
        ex.Next()
    fx = TopExp_Explorer(s, TopAbs_FACE)
    while fx.More():
        faces += 1
        fx.Next()
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(s, box, True, False)
    lo, hi = box.CornerMin(), box.CornerMax()
    out = {
        "valid": bool(BRepCheck_Analyzer(s).IsValid()),
        "solids": n,
        "distinct": nd,
        "faces": faces,
        "all": tot,
        "dist": dist,
        "box": [lo.X(), lo.Y(), lo.Z(), hi.X(), hi.Y(), hi.Z()] if n else None,
        "read_s": round(t_read, 3),
        "secs": round(time.time() - t0, 3),
    }
    print(json.dumps(out))


if __name__ == "__main__":
    main(sys.argv[1])
