#!/usr/bin/env python3
"""The same real parts through OpenCascade alone: a yardstick for the build rate. Reads translated extrude-only
.fc files directly, so it covers the parts ForgeCAD refuses too, which the oracle's export cannot; same multi-body
reading as the importer. Built = no exception, BRepCheck valid, volume within 0.1% of the data's (1% if approx).

  VENV/bin/python tools/find-parts/occt_replay.py DIR --every N --out FILE [--jobs 3]
"""
import json
import math
import multiprocessing as mp
import os
import re
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

NUM = re.compile(r"-?[\d.]+")


def loops(t, W):
    """Wires from the loop words, sketch points mapped to world by W."""
    i, out, cur = 0, [], None

    def line(a, b):
        return BRepBuilderAPI_MakeEdge(gp_Pnt(*W(a)), gp_Pnt(*W(b))).Edge()

    def close(edges):
        w = BRepBuilderAPI_MakeWire()
        for e in edges:
            w.Add(e)
        if not w.IsDone():
            raise ValueError("wire")
        out.append(w.Wire())

    while i < len(t):
        k = t[i]
        if k == "circle":
            c, r = (float(t[i + 1]), float(t[i + 2])), float(t[i + 3])
            n = W.normal
            e = BRepBuilderAPI_MakeEdge(gp_Circ(gp_Ax2(gp_Pnt(*W(c)), gp_Dir(*n)), r)).Edge()
            if cur:
                close(cur)
                cur = None
            close([e])
            i += 4
        elif k == "poly":
            if cur:
                close(cur)
                cur = None
            i += 1
            p = []
            while i < len(t) and NUM.fullmatch(t[i]):
                p.append((float(t[i]), float(t[i + 1])))
                i += 2
            close([line(p[j], p[(j + 1) % len(p)]) for j in range(len(p))])
        elif k == "loop":
            if cur:
                close(cur)
            cur = []
            i += 1
        elif k == "line":
            cur.append(line((float(t[i + 1]), float(t[i + 2])), (float(t[i + 3]), float(t[i + 4]))))
            i += 5
        elif k == "arc":
            cx, cy, r, a0, sw = map(float, t[i + 1 : i + 6])
            at = lambda a: (cx + r * math.cos(math.radians(a)), cy + r * math.sin(math.radians(a)))
            arc = GC_MakeArcOfCircle(gp_Pnt(*W(at(a0))), gp_Pnt(*W(at(a0 + sw / 2))), gp_Pnt(*W(at(a0 + sw)))).Value()
            cur.append(BRepBuilderAPI_MakeEdge(arc).Edge())
            i += 6
        else:
            break
    if cur:
        close(cur)
    return out


def volume(s):
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(s, g)
    return g.Mass()


def build(src):
    bodies = []
    for l in src.splitlines():
        t = l.split()
        if not t or t[0] != "extrude":
            raise ValueError("not an extrude")
        allb = t[-2:] == ["in", "all"]
        if allb:
            t = t[:-2]
        zi = t.index("z")
        op = t[1] if t[1] in ("add", "cut", "keep", "new") else ""
        ext = t[(2 if op else 1) : zi]
        up, down = (float(ext[1]), float(ext[2])) if ext[0] == "two" else (float(ext[0]), 0.0)
        Z, X, O = (tuple(map(float, t[zi + a : zi + a + 3])) for a in (1, 5, 9))
        Y = (Z[1] * X[2] - Z[2] * X[1], Z[2] * X[0] - Z[0] * X[2], Z[0] * X[1] - Z[1] * X[0])

        class W:
            normal = Z

            def __new__(cls, p):
                return tuple(O[i] + p[0] * X[i] + p[1] * Y[i] - down * Z[i] for i in range(3))

        wires = loops(t[zi + 12 :], W)
        f = BRepBuilderAPI_MakeFace(wires[0])
        for w in wires[1:]:
            f.Add(w)
        from OCP.ShapeFix import ShapeFix_Face

        fix = ShapeFix_Face(f.Face())
        fix.FixOrientation()
        fix.Perform()
        h = up + down
        tool = BRepPrimAPI_MakePrism(fix.Face(), gp_Vec(Z[0] * h, Z[1] * h, Z[2] * h)).Shape()
        if op == "" or op == "new":
            bodies.append(tool)
        elif op == "add":
            r = BRepAlgoAPI_Fuse(bodies[-1], tool)
            if not r.IsDone():
                raise ValueError("fuse")
            bodies[-1] = r.Shape()
        else:
            cls = BRepAlgoAPI_Cut if op == "cut" else BRepAlgoAPI_Common
            for i in range(len(bodies)) if allb else [len(bodies) - 1]:
                r = cls(bodies[i], tool)
                if not r.IsDone():
                    raise ValueError(op)
                bodies[i] = r.Shape()
    return bodies


def one(job):
    d, n = job
    resource.setrlimit(resource.RLIMIT_AS, (2 << 30, 2 << 30))
    with open(os.path.join(d, n + ".fc")) as f:
        src = f.read()
    with open(os.path.join(d, n + ".json")) as f:
        t = json.load(f)
    try:
        bs = build(src)
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
    names = sorted(f[:-3] for f in os.listdir(d) if f.endswith(".fc"))[:: int(opt("--every", "1"))]
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
