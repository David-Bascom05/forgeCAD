#!/usr/bin/env python3
"""OpenCascade differential oracle (test only, never a dependency of the workspace).

Rebuilds each part exported by `cargo run --release -p forge-model --example oracle` in OCCT
(sequential BRepAlgoAPI booleans), measures volume, area, area per distinct surface, topology
counts, BRepCheck validity and build time, and diffs them against ForgeCAD's results.

Ours are measured on a mesh of chord c (the exporter's CHORD, default 0.002), so a difference is
a disagreement only beyond what the mesh can explain: volume beyond c x area; total area and each
surface's area beyond 3c x (meshed boundary length + area / smallest radius).

  tools/occt/oracle.py parts.json [--jobs 3] [--reps 1] [--timeout 120] [--out results.json]
                                  [--parallel] [--fuzzy 0] [--only NAME] [--show N] [--surfaces]
                                  [--gate [--known FILE]] [--save-known FILE]

Given a corpus file (tools/corpus/*.json, with "cases"), builds each case in OCCT alone and
compares it with the case's expectations (this checks the corpus' closed forms too).

--tiebreak settles each volume disagreement by Monte Carlo (montecarlo.py); a part where it backs
us is "occt-wrong" (OCCT returned a valid but wrong solid).

--gate exits 1 if any part (not listed in --known) fails on our side where OCCT succeeds, or
disagrees on volume, area or a surface's area.

Needs the OCP bindings (tools/occt/setup.sh puts them in a venv).
"""
import argparse
import json
import math
import multiprocessing as mp
import sys
import time

from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepAlgoAPI import BRepAlgoAPI_Common, BRepAlgoAPI_Cut, BRepAlgoAPI_Fuse
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakeWire
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
from OCP.BRepGProp import BRepGProp
from OCP.BRepPrimAPI import BRepPrimAPI_MakePrism, BRepPrimAPI_MakeRevol, BRepPrimAPI_MakeSphere
from OCP.GC import GC_MakeArcOfCircle
from OCP.GProp import GProp_GProps
from OCP.GeomAbs import GeomAbs_Cone, GeomAbs_Cylinder, GeomAbs_Plane, GeomAbs_Sphere, GeomAbs_Torus
from OCP.ShapeFix import ShapeFix_Face
from OCP.ShapeUpgrade import ShapeUpgrade_UnifySameDomain
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID, TopAbs_VERTEX
from OCP.TopExp import TopExp
from OCP.OCP.collections import IndexedMap_TopoDS_Shape_TopTools_ShapeMapHasher as ShapeMap
from OCP.OCP.collections import List_TopoDS_Shape as ShapeList
from OCP.TopoDS import TopoDS
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


def round_edges(body, f):
    """A fillet step: each point picks the edge nearest it."""
    m = ShapeMap()
    TopExp.MapShapes_s(body, TopAbs_EDGE, m)
    edges = [TopoDS.Edge(m.FindKey(i)) for i in range(1, m.Extent() + 1)]
    mk = BRepFilletAPI_MakeFillet(body)
    for k, at in enumerate(f["at"]):
        v = BRepBuilderAPI_MakeVertex(gp_Pnt(*at)).Vertex()

        def dist(e):
            d = BRepExtrema_DistShapeShape(v, e)
            return d.Value() if d.IsDone() else math.inf

        e = min(edges, key=dist)
        if dist(e) > 1e-3:
            raise RuntimeError(f"no edge through {at}")
        mk.Add(f["radii"][k] if "radii" in f else f["radius"], e)
    mk.Build()
    if not mk.IsDone():
        raise RuntimeError("fillet failed")
    return mk.Shape()


def build(features, parallel, fuzzy):
    body = None
    for i, f in enumerate(features):
        try:
            if f["kind"] == "fillet":
                body = round_edges(body, f)
                continue
            t = tool(f)
            body = t if f["op"] == "new" else combine(body, t, f["op"], parallel, fuzzy)
        except Exception as e:
            raise RuntimeError(f"feature {i}: {e}") from e
    return body


def count(shape, kind):
    m = ShapeMap()
    TopExp.MapShapes_s(shape, kind, m)
    return m.Extent()


def xyz(p):
    return [p.X(), p.Y(), p.Z()]


def face_areas(shape):
    """Each face's surface (in the exporter's terms) and area."""
    m = ShapeMap()
    TopExp.MapShapes_s(shape, TopAbs_FACE, m)
    out = []
    for i in range(1, m.Extent() + 1):
        f = TopoDS.Face(m.FindKey(i))
        g = GProp_GProps()
        BRepGProp.SurfaceProperties_s(f, g)
        ad = BRepAdaptor_Surface(f)
        t = ad.GetType()
        if t == GeomAbs_Plane:
            s = ad.Plane()
            e = {"s": "plane", "p": xyz(s.Location()), "n": xyz(s.Axis().Direction())}
        elif t == GeomAbs_Cylinder:
            s = ad.Cylinder()
            e = {"s": "cylinder", "p": xyz(s.Location()), "n": xyz(s.Axis().Direction()), "r": s.Radius()}
        elif t == GeomAbs_Cone:
            s = ad.Cone()
            e = {"s": "cone", "p": xyz(s.Apex()), "n": xyz(s.Axis().Direction()), "t": math.tan(s.SemiAngle())}
        elif t == GeomAbs_Sphere:
            s = ad.Sphere()
            e = {"s": "sphere", "p": xyz(s.Location()), "r": s.Radius()}
        elif t == GeomAbs_Torus:
            s = ad.Torus()
            e = {"s": "torus", "p": xyz(s.Location()), "n": xyz(s.Axis().Direction()),
                 "R": s.MajorRadius(), "r": s.MinorRadius()}
        else:
            e = {"s": "other"}
        e["a"] = g.Mass()
        out.append(e)
    return out


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
        "face_areas": face_areas(shape),
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


def key(e):
    """A surface's kind and canonical defining numbers: unsigned axes and normals, points
    reduced to the axis or plane, so the same surface keys alike in both kernels."""
    s = e["s"]

    def unsigned(n):
        k = next((i for i in range(3) if abs(n[i]) > 1e-9), 0)
        return [-c for c in n] if n[k] < 0 else list(n)

    def on_axis(p, n):
        d = sum(p[i] * n[i] for i in range(3))
        return [p[i] - n[i] * d for i in range(3)]

    if s == "plane":
        n = unsigned(e["n"])
        return s, n + [sum(e["p"][i] * n[i] for i in range(3))]
    if s == "cylinder":
        n = unsigned(e["n"])
        return s, n + on_axis(e["p"], n) + [e["r"]]
    if s == "cone":
        n = e["n"] if e["t"] > 0 else [-c for c in e["n"]]
        return s, e["p"] + n + [abs(e["t"])]
    if s == "sphere":
        return s, e["p"] + [e["r"]]
    if s == "torus":
        n = unsigned(e["n"])
        return s, n + e["p"] + [e["R"], e["r"]]
    return s, []


def same(a, b):
    return len(a) == len(b) and all(abs(x - y) <= 1e-6 * max(1.0, abs(x)) for x, y in zip(a, b))


def curvature(e):
    """A bound on 1 / the face's smallest radius of curvature (how far chords sag)."""
    s = e["s"]
    if s in ("cylinder", "sphere"):
        return 1.0 / e["r"]
    if s == "torus":
        return 1.0 / min(e["r"], e["R"])
    if s == "cone":
        return 4.0 / max(math.sqrt(e["a"]), 1e-3)
    return 0.0


def surfaces(ours, ref):
    """Area per distinct surface in both kernels, and how far each pair may differ given our
    mesh's chord: sag on curved faces plus the boundary's chords."""
    chord = ours.get("chord", 0.002)
    groups = []
    for side, faces in ((0, ours.get("face_areas", [])), (1, ref.get("face_areas", []))):
        for e in faces:
            k, v = key(e)
            g = next((g for g in groups if g["s"] == k and same(g["k"], v)), None)
            if g is None:
                g = {"s": k, "k": v, "a": [0.0, 0.0], "tol": 0.0}
                groups.append(g)
            g["a"][side] += e["a"]
            if side == 0:
                g["tol"] += 3.0 * chord * (e.get("l", 0.0) + e["a"] * curvature(e))
    for g in groups:
        g["tol"] = max(g["tol"], 1e-6 * max(g["a"]))
    return groups


def compare(ours, ref):
    """How a ForgeCAD regen compares with OCCT, and the worst ratio of a difference to its bound."""
    s = ours["status"]
    if ref["status"] != "ok":
        return ("occt-" + ref["status"] if s == "ok" else "both-fail"), 0.0
    if s != "ok":
        # an exact zero-thickness touch rejected clearly, where OCCT's answer is empty
        if "ZeroThickness" in ours.get("error", "") and abs(ref["volume"]) < 1e-6 * max(ref["area"], 1.0):
            return "reject-ok", 0.0
        return "ours-" + s, 0.0
    if "volume" not in ours:
        return "ours-nomesh", 0.0
    chord = ours.get("chord", 0.002)
    vtol = max(chord * ref["area"], 1e-9)
    groups = surfaces(ours, ref)
    atol = max(sum(g["tol"] for g in groups), 1e-9)
    rv = abs(ours["volume"] - ref["volume"]) / vtol
    ra = abs(ours["area"] - ref["area"]) / atol
    rs = max([abs(g["a"][0] - g["a"][1]) / g["tol"] for g in groups if g["s"] != "other"] or [0.0])
    worst = max(rv, ra, rs)
    if rv > 1:
        return "volume", worst
    if ra > 1:
        return "area", worst
    if rs > 1:
        return "surface", worst
    if ours["shells"] != ref["shells"]:
        return "shells", worst
    return "agree", worst


GOOD = ("agree", "reject-ok", "occt-invalid", "occt-error", "occt-timeout", "occt-crash", "occt-wrong")


def surface_counts(faces):
    """Distinct surfaces by kind."""
    seen = []
    for e in faces:
        k, v = key(e)
        if not any(s == k and same(w, v) for s, w in seen):
            seen.append((k, v))
    out = {}
    for k, _ in seen:
        out[k] = out.get(k, 0) + 1
    return out


def reference(a, cases):
    """The corpus through OCCT alone: its result against each case's expectations."""
    from concurrent.futures import ThreadPoolExecutor

    with ThreadPoolExecutor(a.jobs) as ex:
        refs = list(ex.map(lambda c: occt(c, 1, False, 0.0, a.timeout), cases))
    print(f"{'case':<26} {'class':<20} {'expect':<13} {'occt':<8} {'ms':>8} {'volume':>12} {'dV/V exact':>11}  "
          f"surfaces (occt | expected)")
    for c, r in zip(cases, refs):
        e = c["expect"]
        dv = "-"
        if r["status"] == "ok" and e.get("exact"):
            dv = f"{(r['volume'] - e['volume']) / e['volume']:+.1e}"
        sc = surface_counts(r.get("face_areas", []))
        print(f"{c['id']:<26} {c['class']:<20} {e['status']:<13} {r['status']:<8} {r.get('ms', math.nan):>8.1f} "
              f"{r.get('volume', math.nan):>12.3f} {dv:>11}  {sc} | {e.get('surfaces', '')}")
        if r["status"] != "ok":
            print(f"      {r.get('error', '')}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("parts")
    ap.add_argument("--jobs", type=int, default=3)
    ap.add_argument("--reps", type=int, default=1)
    ap.add_argument("--timeout", type=float, default=120)
    ap.add_argument("--parallel", action="store_true", help="OCCT's own threaded booleans")
    ap.add_argument("--fuzzy", type=float, default=0.0)
    ap.add_argument("--only")
    ap.add_argument("--out")
    ap.add_argument("--show", type=int, default=1000, help="rows to print")
    ap.add_argument("--surfaces", action="store_true", help="print the mismatched surfaces of each part")
    ap.add_argument("--tiebreak", action="store_true",
                    help="settle volume disagreements by Monte Carlo against the analytic tools")
    ap.add_argument("--gate", action="store_true", help="exit 1 on any disagreement or failure of ours")
    ap.add_argument("--known", help="file of part names whose failures the gate tolerates")
    ap.add_argument("--save-known", help="write the failing part names to this file")
    a = ap.parse_args()
    data = json.load(open(a.parts))
    if "cases" in data:
        return reference(a, data["cases"])
    parts = [p for p in data["parts"] if not a.only or p["name"] == a.only]
    from concurrent.futures import ThreadPoolExecutor

    with ThreadPoolExecutor(a.jobs) as ex:
        refs = list(ex.map(lambda p: occt(p, a.reps, a.parallel, a.fuzzy, a.timeout), parts))
    print(f"{'part':<44} {'feat':>4} {'occt ms':>9} {'seq ms':>8} {'nary1':>8} {'naryN':>8}  {'occt':<8} "
          f"{'seq':<10} {'nary':<10} {'worst':>6} {'dV seq':>8} {'dA seq':>8}  faces o/u/s/n")
    tally, times, out, failing = {}, {"occt": 0.0, "seq": 0.0, "nary1": 0.0, "nary": 0.0}, [], []
    for i, (p, ref) in enumerate(zip(parts, refs)):
        f = p["forge"]
        (vs, ws), (vn, wn) = compare(f["seq"], ref), compare(f["nary"], ref)
        if a.tiebreak and "volume" in (vs, vn):
            try:
                import montecarlo

                path = "seq" if vs == "volume" else "nary"
                who, mv, ms = montecarlo.judge(p, f[path]["volume"], ref["volume"])
                if who == "ours":
                    vs, vn = ("occt-wrong" if v == "volume" else v for v in (vs, vn))
                print(f"    {p['name']}: monte carlo {mv:.1f} +- {ms:.1f} backs {who}")
            except ValueError:
                pass
        for path, v in (("seq", vs), ("nary", vn)):
            tally.setdefault(path, {}).setdefault(v, []).append(p["name"])
        if vs not in GOOD or vn not in GOOD:
            failing.append(p["name"])
        if ref["status"] == "ok" and f["seq"]["status"] == "ok" and f["nary"]["status"] == "ok":
            for k in ("seq", "nary1", "nary"):
                times[k] += f[k]["ms"]
            times["occt"] += ref["ms"]

        def d(o, k):
            return f"{(o[k] - ref[k]) / max(abs(ref[k]), 1e-9):+.1e}" if k in o and k in ref else "-"

        if i < a.show:
            faces = "/".join(str(x) for x in (ref.get("faces", "-"), ref.get("faces_unified", "-"),
                                              f["seq"].get("faces", "-"), f["nary"].get("faces", "-")))
            print(f"{p['name']:<44} {len(p['features']):>4} {ref.get('ms', math.nan):>9.1f} "
                  f"{f['seq']['ms']:>8.1f} {f['nary1']['ms']:>8.1f} {f['nary']['ms']:>8.1f}  {ref['status']:<8} "
                  f"{vs:<10} {vn:<10} {max(ws, wn):>6.2f} {d(f['seq'], 'volume'):>8} {d(f['seq'], 'area'):>8}  {faces}")
            if a.surfaces and ref["status"] == "ok":
                for path in ("seq", "nary"):
                    if f[path]["status"] != "ok":
                        continue
                    for g in surfaces(f[path], ref):
                        if abs(g["a"][0] - g["a"][1]) > g["tol"]:
                            print(f"      {path} {g['s']:<8} {[round(x, 4) for x in g['k']]}  ours {g['a'][0]:.4f} "
                                  f"occt {g['a'][1]:.4f}  (bound {g['tol']:.2g})")
        out.append({"name": p["name"], "occt": ref, "forge": f, "verdict": {"seq": vs, "nary": vn}})
    print(f"\n{len(parts)} parts. Time where all three succeed (ms, sum): OCCT {times['occt']:.0f}, "
          f"seq {times['seq']:.0f}, nary1 {times['nary1']:.0f}, nary x{data.get('threads')} {times['nary']:.0f}")
    for path in ("seq", "nary"):
        print(f"{path}: " + ", ".join(f"{k} {len(v)}" for k, v in sorted(tally.get(path, {}).items())))
        for k, v in sorted(tally.get(path, {}).items()):
            if k not in GOOD:
                print(f"   {k}: {' '.join(v[:12])}{' ...' if len(v) > 12 else ''}")
    if a.out:
        json.dump(out, open(a.out, "w"), indent=1)
    if a.save_known:
        open(a.save_known, "w").write("".join(n + "\n" for n in failing))
    if a.gate:
        known = set(open(a.known).read().split()) if a.known else set()
        new = [n for n in failing if n not in known]
        print(f"gate: {len(failing)} failing, {len(new)} not known" + (f": {' '.join(new[:20])}" if new else ""))
        return 1 if new else 0
    return 0


if __name__ == "__main__":
    sys.exit(main())
