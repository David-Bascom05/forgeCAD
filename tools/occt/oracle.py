#!/usr/bin/env python3
"""OpenCascade differential oracle (test only, never a dependency of the workspace).

Rebuilds each part exported by `cargo run --release -p forge-model --example oracle` in OCCT
(sequential BRepAlgoAPI booleans), measures volume, area, topology counts, BRepCheck validity
and build time, and diffs them against ForgeCAD's results.

  tools/occt/oracle.py parts.json [--jobs 3] [--reps 1] [--timeout 120] [--out results.json]
                                  [--parallel] [--fuzzy 0] [--only NAME] [--show N]

Needs the OCP bindings (tools/occt/setup.sh puts them in a venv).
"""
import argparse
import json
import math
import multiprocessing as mp
import sys
import time

from OCP.BRepAlgoAPI import BRepAlgoAPI_Common, BRepAlgoAPI_Cut, BRepAlgoAPI_Fuse
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakeWire
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepPrimAPI import BRepPrimAPI_MakePrism, BRepPrimAPI_MakeRevol, BRepPrimAPI_MakeSphere
from OCP.GC import GC_MakeArcOfCircle
from OCP.GProp import GProp_GProps
from OCP.ShapeFix import ShapeFix_Face
from OCP.ShapeUpgrade import ShapeUpgrade_UnifySameDomain
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID, TopAbs_VERTEX
from OCP.TopExp import TopExp
from OCP.OCP.collections import IndexedMap_TopoDS_Shape_TopTools_ShapeMapHasher as ShapeMap
from OCP.OCP.collections import List_TopoDS_Shape as ShapeList
from OCP.gp import gp_Ax1, gp_Ax2, gp_Ax3, gp_Circ, gp_Dir, gp_Pln, gp_Pnt, gp_Vec


class Plane:
    def __init__(self, f):
        self.o, self.x, self.y, self.z = f["o"], f["x"], f["y"], f["z"]

    def pt(self, u, v):
        return gp_Pnt(*[self.o[i] + self.x[i] * u + self.y[i] * v for i in range(3)])

    def vec(self, u, v):
        return [self.x[i] * u + self.y[i] * v for i in range(3)]

    def ax3(self):
        return gp_Ax3(gp_Pnt(*self.o), gp_Dir(*self.z), gp_Dir(*self.x))


def edge(pl, s):
    if "line" in s:
        a, b, c, d = s["line"]
        return BRepBuilderAPI_MakeEdge(pl.pt(a, b), pl.pt(c, d)).Edge()
    if "arc" in s:
        cx, cy, r, st, sw = s["arc"]
        p = [pl.pt(cx + r * math.cos(t), cy + r * math.sin(t)) for t in (st, st + sw / 2, st + sw)]
        return BRepBuilderAPI_MakeEdge(GC_MakeArcOfCircle(p[0], p[1], p[2]).Value()).Edge()
    cx, cy, r, _ = s["circle"]
    ax = gp_Ax2(pl.pt(cx, cy), gp_Dir(*pl.z), gp_Dir(*pl.x))
    return BRepBuilderAPI_MakeEdge(gp_Circ(ax, r)).Edge()


def face(f):
    pl = Plane(f["plane"])
    wires = []
    for lp in f["loops"]:
        mw = BRepBuilderAPI_MakeWire()
        for s in lp:
            mw.Add(edge(pl, s))
        wires.append(mw.Wire())
    mf = BRepBuilderAPI_MakeFace(gp_Pln(pl.ax3()), wires[0])
    for w in wires[1:]:
        mf.Add(w)
    fix = ShapeFix_Face(mf.Face())
    fix.FixOrientation()
    fix.Perform()
    return fix.Face(), pl


def tool(f):
    if f["kind"] == "ball":
        return BRepPrimAPI_MakeSphere(gp_Pnt(*f["center"]), f["radius"]).Shape()
    fc, pl = face(f)
    if f["kind"] == "extrude":
        d = f["distance"]
        return BRepPrimAPI_MakePrism(fc, gp_Vec(*[c * d for c in pl.z])).Shape()
    ox, oy, dx, dy = f["axis"]
    ax = gp_Ax1(pl.pt(ox, oy), gp_Dir(*pl.vec(dx, dy)))
    return BRepPrimAPI_MakeRevol(fc, ax, f["angle"]).Shape()


def combine(body, t, op, parallel, fuzzy):
    algo = {"add": BRepAlgoAPI_Fuse, "cut": BRepAlgoAPI_Cut, "keep": BRepAlgoAPI_Common}[op]()
    args, tools = ShapeList(), ShapeList()
    args.Append(body)
    tools.Append(t)
    algo.SetArguments(args)
    algo.SetTools(tools)
    algo.SetRunParallel(parallel)
    if fuzzy > 0:
        algo.SetFuzzyValue(fuzzy)
    algo.Build()
    if not algo.IsDone():
        raise RuntimeError(f"boolean {op} failed")
    return algo.Shape()


def build(features, parallel, fuzzy):
    body = None
    for i, f in enumerate(features):
        try:
            t = tool(f)
            body = t if f["op"] == "new" else combine(body, t, f["op"], parallel, fuzzy)
        except Exception as e:
            raise RuntimeError(f"feature {i}: {e}") from e
    return body


def count(shape, kind):
    m = ShapeMap()
    TopExp.MapShapes_s(shape, kind, m)
    return m.Extent()


def measure(shape):
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, g)
    vol = g.Mass()
    g = GProp_GProps()
    BRepGProp.SurfaceProperties_s(shape, g)
    unify = ShapeUpgrade_UnifySameDomain(shape, True, True, False)
    unify.Build()
    return {
        "valid": BRepCheck_Analyzer(shape).IsValid(),
        "volume": vol,
        "area": g.Mass(),
        "faces": count(shape, TopAbs_FACE),
        "edges": count(shape, TopAbs_EDGE),
        "vertices": count(shape, TopAbs_VERTEX),
        "shells": count(shape, TopAbs_SHELL),
        "solids": count(shape, TopAbs_SOLID),
        "faces_unified": count(unify.Shape(), TopAbs_FACE),
    }


def run_part(part, reps, parallel, fuzzy, conn):
    try:
        best, shape = math.inf, None
        for _ in range(reps):
            t = time.perf_counter()
            shape = build(part["features"], parallel, fuzzy)
            best = min(best, (time.perf_counter() - t) * 1e3)
        r = {"status": "ok", "ms": best}
        r.update(measure(shape))
        if not r["valid"]:
            r["status"] = "invalid"
    except Exception as e:
        r = {"status": "error", "error": str(e)[:200]}
    conn.send(r)
    conn.close()


def occt(part, reps, parallel, fuzzy, timeout):
    rx, tx = mp.Pipe(False)
    p = mp.get_context("fork").Process(target=run_part, args=(part, reps, parallel, fuzzy, tx))
    p.start()
    tx.close()
    got = rx.poll(timeout)
    r = rx.recv() if got else None
    p.join(1 if got else 0)
    if p.is_alive():
        p.kill()
        p.join()
    if r is None:
        return {"status": "timeout" if not got else "crash", "exitcode": p.exitcode}
    return r


def rel(a, b):
    return abs(a - b) / max(abs(b), 1e-9)


def verdict(ours, ref, vtol):
    """How a ForgeCAD regen compares with OCCT."""
    s = ours["status"]
    if ref["status"] != "ok":
        return "occt-" + ref["status"] if s == "ok" else "both-fail"
    if s != "ok":
        return "ours-" + s
    if "volume" not in ours:
        return "ours-nomesh"
    if rel(ours["volume"], ref["volume"]) > vtol:
        return "volume"
    if ours["shells"] != ref["shells"]:
        return "shells"
    return "agree"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("parts")
    ap.add_argument("--jobs", type=int, default=3)
    ap.add_argument("--reps", type=int, default=1)
    ap.add_argument("--timeout", type=float, default=120)
    ap.add_argument("--parallel", action="store_true", help="OCCT's own threaded booleans")
    ap.add_argument("--fuzzy", type=float, default=0.0)
    ap.add_argument("--vtol", type=float, default=2e-3, help="relative volume tolerance (ours is from a mesh)")
    ap.add_argument("--only")
    ap.add_argument("--out")
    ap.add_argument("--show", type=int, default=1000, help="rows to print")
    a = ap.parse_args()
    data = json.load(open(a.parts))
    parts = [p for p in data["parts"] if not a.only or p["name"] == a.only]
    from concurrent.futures import ThreadPoolExecutor

    with ThreadPoolExecutor(a.jobs) as ex:
        refs = list(ex.map(lambda p: occt(p, a.reps, a.parallel, a.fuzzy, a.timeout), parts))
    hdr = f"{'part':<26} {'feat':>4} {'occt ms':>9} {'seq ms':>8} {'nary1':>8} {'naryN':>8}  {'occt':<8} {'seq':<10} {'nary':<10} {'dV seq':>8} {'dV nary':>8} {'dA nary':>8}  faces o/u/s/n"
    print(hdr)
    tally, times, out = {}, {"occt": 0.0, "seq": 0.0, "nary1": 0.0, "nary": 0.0}, []
    for i, (p, ref) in enumerate(zip(parts, refs)):
        f = p["forge"]
        vs, vn = verdict(f["seq"], ref, a.vtol), verdict(f["nary"], ref, a.vtol)
        for path, v in (("seq", vs), ("nary", vn)):
            tally.setdefault(path, {}).setdefault(v, []).append(p["name"])
        both = ref["status"] == "ok" and f["seq"]["status"] == "ok" and f["nary"]["status"] == "ok"
        if both:
            for k in ("seq", "nary1", "nary"):
                times[k] += f[k]["ms"]
            times["occt"] += ref["ms"]

        def dv(o, key="volume"):
            return f"{(o[key] - ref[key]) / ref[key]:+.1e}" if key in o and key in ref else "-"

        if i < a.show:
            faces = "/".join(str(x) for x in (ref.get("faces", "-"), ref.get("faces_unified", "-"),
                                              f["seq"].get("faces", "-"), f["nary"].get("faces", "-")))
            print(f"{p['name']:<26} {len(p['features']):>4} {ref.get('ms', math.nan):>9.1f} {f['seq']['ms']:>8.1f} "
                  f"{f['nary1']['ms']:>8.1f} {f['nary']['ms']:>8.1f}  {ref['status']:<8} {vs:<10} {vn:<10} "
                  f"{dv(f['seq']):>8} {dv(f['nary']):>8} {dv(f['nary'], 'area'):>8}  {faces}")
        out.append({"name": p["name"], "occt": ref, "forge": f, "verdict": {"seq": vs, "nary": vn}})
    print(f"\n{len(parts)} parts. Time where all three succeed (ms, sum): OCCT {times['occt']:.0f}, "
          f"seq {times['seq']:.0f}, nary1 {times['nary1']:.0f}, nary x{data.get('threads')} {times['nary']:.0f}")
    for path in ("seq", "nary"):
        print(f"{path}: " + ", ".join(f"{k} {len(v)}" for k, v in sorted(tally.get(path, {}).items())))
        for k, v in sorted(tally.get(path, {}).items()):
            if k != "agree":
                print(f"   {k}: {' '.join(v[:12])}{' ...' if len(v) > 12 else ''}")
    if a.out:
        json.dump(out, open(a.out, "w"), indent=1)


if __name__ == "__main__":
    main()
