#!/usr/bin/env python3
"""OpenCascade reads STEP files: is each valid, and its volume, area and box. One child per file,
memory and time capped, so a hostile or huge file costs only itself.

  $VENV/bin/python -I step_read.py [--jobs 4] [--timeout 120] [--fix] FILE..  (or @LIST, or NAME=PATH)

One JSON line per file: name, valid, solids, shells, faces, vol, area, box, secs, or err. Files
are data: nothing in them runs, and the reader runs from where it is, not from beside them.
"""
import argparse
import concurrent.futures as cf
import json
import resource
import subprocess
import sys
import time

MEM = 4 << 30


def measure(path):
    from OCP.Bnd import Bnd_Box
    from OCP.BRepBndLib import BRepBndLib
    from OCP.BRepCheck import BRepCheck_Analyzer
    from OCP.BRepGProp import BRepGProp
    from OCP.GProp import GProp_GProps
    from OCP.IFSelect import IFSelect_RetDone
    from OCP.STEPControl import STEPControl_Reader
    from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID, TopAbs_VERTEX
    from OCP.TopExp import TopExp, TopExp_Explorer
    from OCP.TopoDS import TopoDS
    from OCP.OCP.collections import IndexedMap_TopoDS_Shape_TopTools_ShapeMapHasher as ShapeMap

    t0 = time.time()
    r = STEPControl_Reader()
    if r.ReadFile(path) != IFSelect_RetDone:
        return {"err": "unreadable"}
    r.TransferRoots()
    s = r.OneShape()

    def count(k):
        m = ShapeMap()
        TopExp.MapShapes_s(s, k, m)
        return m.Extent()

    vol = area = 0.0
    n = 0
    ex = TopExp_Explorer(s, TopAbs_SOLID)
    while ex.More():
        g = GProp_GProps()
        BRepGProp.VolumeProperties_s(TopoDS.Solid(ex.Current()), g)
        vol += abs(g.Mass())
        g = GProp_GProps()
        BRepGProp.SurfaceProperties_s(TopoDS.Solid(ex.Current()), g)
        area += g.Mass()
        n += 1
        ex.Next()
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(s, box, True, False)
    lo, hi = box.CornerMin(), box.CornerMax()
    return {
        "valid": bool(BRepCheck_Analyzer(s).IsValid()),
        "solids": n,
        "shells": count(TopAbs_SHELL),
        "faces": count(TopAbs_FACE),
        "edges": count(TopAbs_EDGE),
        "verts": count(TopAbs_VERTEX),
        "vol": vol,
        "area": area,
        "box": [lo.X(), lo.Y(), lo.Z(), hi.X(), hi.Y(), hi.Z()],
        "secs": round(time.time() - t0, 3),
    }


def child(path):
    resource.setrlimit(resource.RLIMIT_AS, (MEM, MEM))
    print(json.dumps(measure(path)))


def one(job):
    name, path, timeout = job
    try:
        p = subprocess.run(
            [sys.executable, "-I", __file__, "--child", path],
            capture_output=True, text=True, timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return {"name": name, "err": "timeout"}
    last = p.stdout.strip().splitlines()[-1:] if p.stdout else []
    try:
        d = json.loads(last[0])
    except (IndexError, json.JSONDecodeError):
        d = {"err": f"crash {p.returncode}"}
    d["name"] = name
    return d


def main():
    a = argparse.ArgumentParser()
    a.add_argument("--jobs", type=int, default=4)
    a.add_argument("--timeout", type=int, default=120)
    a.add_argument("--child")
    a.add_argument("files", nargs="*")
    o = a.parse_args()
    if o.child:
        return child(o.child)
    names = []
    for f in o.files:
        if f.startswith("@"):
            names += [x.strip() for x in open(f[1:]) if x.strip()]
        else:
            names.append(f)
    jobs = [(*(n.split("=", 1) if "=" in n else (n, n)), o.timeout) for n in names]
    with cf.ThreadPoolExecutor(o.jobs) as ex:
        for d in ex.map(one, jobs):
            print(json.dumps(d), flush=True)


if __name__ == "__main__":
    main()
