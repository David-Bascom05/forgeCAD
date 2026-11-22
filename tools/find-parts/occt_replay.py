#!/usr/bin/env python3
"""The same real parts through OpenCascade alone: a yardstick for the build rate. Builds the extrude steps
fusion.py keeps beside each .fg (<stem>.json "steps": frames, extents, loops), so it covers the parts ForgeCAD
refuses too, which the oracle's export cannot; same multi-body reading as the importer. Built = no exception,
BRepCheck valid, volume within 0.1% of the data's (1% if approx). A loop with a spline or an ellipse is not built.

  VENV/bin/python tools/find-parts/occt_replay.py DIR --every N --out FILE [--jobs 3]
"""
import json
import math
import multiprocessing as mp
import os
import resource
import sys

from OCP.BRepAlgoAPI import BRepAlgoAPI_Common, BRepAlgoAPI_Cut, BRepAlgoAPI_Fuse
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakeWire
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepPrimAPI import BRepPrimAPI_MakePrism
from OCP.GC import GC_MakeArcOfCircle
from OCP.GProp import GProp_GProps
from OCP.gp import gp_Ax2, gp_Circ, gp_Dir, gp_Pnt, gp_Vec

def loops(ls, W):
    """Wires from a step's loops (fusion.emit_loop's data), sketch points mapped to world by W."""
    out = []

    def line(a, b):
        return BRepBuilderAPI_MakeEdge(gp_Pnt(*W(a)), gp_Pnt(*W(b))).Edge()

    def close(edges):
        w = BRepBuilderAPI_MakeWire()
        for e in edges:
            w.Add(e)
        if not w.IsDone():
            raise ValueError("wire")
        out.append(w.Wire())

    for lp in ls:
        if lp[0] == "circle":
            _, cx, cy, r = lp
            close([BRepBuilderAPI_MakeEdge(gp_Circ(gp_Ax2(gp_Pnt(*W((cx, cy))), gp_Dir(*W.normal)), r)).Edge()])
        elif lp[0] == "poly":
            p = lp[1]
            close([line(p[j], p[(j + 1) % len(p)]) for j in range(len(p))])
        else:
            cur = []
            for sg in lp[1]:
                if sg[0] == "line":
                    cur.append(line(sg[1:3], sg[3:5]))
                elif sg[0] == "arc":
                    cx, cy, r, a0, sw = sg[1:]
                    at = lambda a: (cx + r * math.cos(math.radians(a)), cy + r * math.sin(math.radians(a)))
                    arc = GC_MakeArcOfCircle(gp_Pnt(*W(at(a0))), gp_Pnt(*W(at(a0 + sw / 2))), gp_Pnt(*W(at(a0 + sw)))).Value()
                    cur.append(BRepBuilderAPI_MakeEdge(arc).Edge())
                else:
                    raise ValueError("curve")
            close(cur)
    return out


def volume(s):
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(s, g)
    return g.Mass()


def build(steps):
    bodies, names, last = [], {}, None

    def fuse(a, b):
        r = BRepAlgoAPI_Fuse(a, b)
        if not r.IsDone():
            raise ValueError("fuse")
        return r.Shape()

    for st in steps:
        if st["kind"] == "combine":
            a = names[st["into"]]
            for x in st["tools"]:
                b = names.pop(x)
                bodies[a] = fuse(bodies[a], bodies[b])
                bodies[b] = None
                names = {k: (a if v == b else v) for k, v in names.items()}
                last = a if last == b else last
            continue
        into = st.get("into")
        allb = into == "all"
        target = names[into] if into and not allb else None
        label = st.get("label")
        op = st["op"]
        up, down = st["up"], st["down"]
        Z, X, O = st["z"], st["x"], st["at"]
        Y = (Z[1] * X[2] - Z[2] * X[1], Z[2] * X[0] - Z[0] * X[2], Z[0] * X[1] - Z[1] * X[0])

        class W:
            normal = Z

            def __new__(cls, p):
                return tuple(O[i] + p[0] * X[i] + p[1] * Y[i] - down * Z[i] for i in range(3))

        wires = loops(st["loops"], W)
        f = BRepBuilderAPI_MakeFace(wires[0])
        for w in wires[1:]:
            f.Add(w)
        from OCP.ShapeFix import ShapeFix_Face

        fix = ShapeFix_Face(f.Face())
        fix.FixOrientation()
        fix.Perform()
        h = up + down
        tool = BRepPrimAPI_MakePrism(fix.Face(), gp_Vec(Z[0] * h, Z[1] * h, Z[2] * h)).Shape()
        if volume(tool) < 0:
            # a face wound against its normal makes an inside-out prism, and a boolean with one quietly does nothing
            tool = tool.Reversed()
        if op == "" or op == "new":
            bodies.append(tool)
            last = len(bodies) - 1
            if label:
                names[label] = last
        elif op == "add":
            i = last if target is None else target
            bodies[i] = fuse(bodies[i], tool)
        else:
            cls = BRepAlgoAPI_Cut if op == "cut" else BRepAlgoAPI_Common
            for i in [j for j, b in enumerate(bodies) if b is not None] if allb else [last if target is None else target]:
                r = cls(bodies[i], tool)
                if not r.IsDone():
                    raise ValueError(op)
                bodies[i] = r.Shape()
    return [b for b in bodies if b is not None]


def one(job):
    d, n = job
    resource.setrlimit(resource.RLIMIT_AS, (2 << 30, 2 << 30))
    with open(os.path.join(d, n + ".json")) as f:
        t = json.load(f)
    try:
        bs = build(t["steps"])
        if not all(BRepCheck_Analyzer(b).IsValid() for b in bs):
            return n, "invalid", None
        v = sum(volume(b) for b in bs)
    except Exception as e:
        return n, "error " + str(e)[:40], None
    if t["volume"] > 0 and abs(v - t["volume"]) / t["volume"] > (0.01 if "approx" in t["notes"] else 1e-3):
        return n, "wrong", v
    return n, "ok", v


def main():
    a = sys.argv[1:]
    d = a[0]
    opt = lambda k, x: a[a.index(k) + 1] if k in a else x
    names = sorted(f[:-3] for f in os.listdir(d) if f.endswith(".fg"))[:: int(opt("--every", "1"))]
    os.nice(15)
    res, tally = {}, {}
    with mp.Pool(int(opt("--jobs", "3")), maxtasksperchild=50) as p:
        it = p.imap_unordered(one, [(d, n) for n in names])
        while True:
            try:
                n, v, vol = it.next(timeout=300)
            except StopIteration:
                break
            except mp.TimeoutError:
                print("a part ran past 300 s; stopping", flush=True)
                p.terminate()
                break
            res[n] = (v, vol)
            k = v.split()[0]
            tally[k] = tally.get(k, 0) + 1
            if len(res) % 200 == 0:
                print(len(res), tally, flush=True)
    with open(opt("--out", "occt-replay.json"), "w") as f:
        json.dump(res, f)
    print(len(res), tally, f"OCCT rate {100 * tally.get('ok', 0) / max(len(res), 1):.1f}%")


if __name__ == "__main__":
    main()
