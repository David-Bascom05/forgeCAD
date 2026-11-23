#!/usr/bin/env python3
"""Sheets and thin walls against OpenCascade (test only).

Each case is a chain in a sketch on xy, written both as a .fg part (measured exactly by
`fgmeasure`, the forge-lang example) and as OCCT's own construction from the same numbers:
  - a sheet: BRepPrimAPI_MakePrism / MakeRevol of the open (or closed) wire; its area against
    our `sheet_area`;
  - a thin wall: that sheet offset by BRepOffset_MakeOffset with thickening and intersection
    joins (mitred corners, as ours); its volume and area against ours, OCCT's result valid.
The side OCCT offsets to is its face's normal: matched to ours (the right of the chain) per case.

  $VENV/bin/python -I tools/occt/sheets.py --measure target/release/examples/fgmeasure [--tol 1e-9]
"""
import json
import math
import os
import subprocess
import sys
import tempfile

from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeWire, BRepBuilderAPI_Transform
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepLProp import BRepLProp_SLProps
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepOffset import BRepOffset_MakeOffset, BRepOffset_Skin
from OCP.BRepPrimAPI import BRepPrimAPI_MakePrism, BRepPrimAPI_MakeRevol
from OCP.GC import GC_MakeArcOfCircle
from OCP.GProp import GProp_GProps
from OCP.GeomAbs import GeomAbs_Intersection
from OCP.TopAbs import TopAbs_FACE, TopAbs_REVERSED
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.gp import gp_Ax1, gp_Circ, gp_Ax2, gp_Dir, gp_Pnt, gp_Trsf, gp_Vec


def arc_pts(c, r, a0, sw):
    return [(c[0] + r * math.cos(a), c[1] + r * math.sin(a)) for a in (a0, a0 + sw / 2, a0 + sw)]


def edge(s):
    if s[0] == "line":
        (x0, y0), (x1, y1) = s[1], s[2]
        return BRepBuilderAPI_MakeEdge(gp_Pnt(x0, y0, 0), gp_Pnt(x1, y1, 0)).Edge()
    if s[0] == "arc":
        p = [gp_Pnt(x, y, 0) for x, y in arc_pts(*s[1:])]
        return BRepBuilderAPI_MakeEdge(GC_MakeArcOfCircle(*p).Value()).Edge()
    c, r = s[1], s[2]
    return BRepBuilderAPI_MakeEdge(gp_Circ(gp_Ax2(gp_Pnt(c[0], c[1], 0), gp_Dir(0, 0, 1)), r)).Edge()


def wire(chain):
    w = BRepBuilderAPI_MakeWire()
    for s in chain:
        w.Add(edge(s))
    return w.Wire()


def start_of(chain):
    """The chain's first point and its direction there, in the sketch."""
    s = chain[0]
    if s[0] == "line":
        (x0, y0), (x1, y1) = s[1], s[2]
        return (x0, y0), (x1 - x0, y1 - y0)
    c, r, a0, sw = (s[1], s[2], s[3], s[4]) if s[0] == "arc" else (s[1], s[2], 0.0, 1.0)
    p = (c[0] + r * math.cos(a0), c[1] + r * math.sin(a0))
    return p, (-math.sin(a0) * math.copysign(1, sw), math.cos(a0) * math.copysign(1, sw))


def fg_seg(k, s):
    """Segment `s` as sketch entity e{k}."""
    if s[0] == "line":
        (x0, y0), (x1, y1) = s[1], s[2]
        return f"  e{k} = line(({x0!r}, {y0!r}), ({x1!r}, {y1!r}))\n"
    if s[0] == "arc":
        c, r, a0, sw = s[1:]
        p, _, q = arc_pts(c, r, a0, sw)
        cw = "true" if sw < 0 else "false"
        return f"  e{k} = arc(center: ({c[0]!r}, {c[1]!r}), from: ({p[0]!r}, {p[1]!r}), to: ({q[0]!r}, {q[1]!r}), cw: {cw})\n"
    c, r = s[1], s[2]
    return f"  e{k} = circle(({c[0]!r}, {c[1]!r}), {r!r})\n"


def fg_chain(chain):
    """The chain's entities, and the chain as a .fg value: one entity, or a list end to end."""
    ents = "".join(fg_seg(k, s) for k, s in enumerate(chain))
    ref = "s.e0" if len(chain) == 1 else "[" + ", ".join(f"s.e{k}" for k in range(len(chain))) + "]"
    return ents, ref


def area(shape):
    g = GProp_GProps()
    BRepGProp.SurfaceProperties_s(shape, g)
    return g.Mass()


def volume(shape):
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, g)
    return g.Mass()


def front_sign(shell, chain):
    """+1 when OCCT's first face's normal at the chain's start is our front (its right), else -1."""
    (px, py), (dx, dy) = start_of(chain)
    want = (dy, -dx)
    best = None
    x = TopExp_Explorer(shell, TopAbs_FACE)
    while x.More():
        f = TopoDS.Face(x.Current())
        s = BRepAdaptor_Surface(f)
        from OCP.GeomAPI import GeomAPI_ProjectPointOnSurf
        from OCP.BRep import BRep_Tool
        pr = GeomAPI_ProjectPointOnSurf(gp_Pnt(px, py, 0.5), BRep_Tool.Surface_s(f))
        if pr.NbPoints() and pr.LowerDistance() < 1e-6:
            u, v = pr.LowerDistanceParameters()
            pp = BRepLProp_SLProps(s, u, v, 1, 1e-9)
            n = pp.Normal()
            if f.Orientation() == TopAbs_REVERSED:
                n = n.Reversed()
            best = n.X() * want[0] + n.Y() * want[1]
            break
        x.Next()
    if best is None:
        raise RuntimeError("no face at the chain's start")
    return 1 if best > 0 else -1


def sheet(case):
    w = wire(case["chain"])
    if case["how"] == "extrude":
        lo, hi = case["lo"], case["hi"]
        s = BRepPrimAPI_MakePrism(w, gp_Vec(0, 0, hi - lo)).Shape()
        t = gp_Trsf()
        t.SetTranslation(gp_Vec(0, 0, lo))
        return BRepBuilderAPI_Transform(s, t, True).Shape()
    ax = gp_Ax1(gp_Pnt(0, 0, 0), gp_Dir(0, 1, 0))
    return BRepPrimAPI_MakeRevol(w, ax, case["angle"]).Shape()


def offset(shape, d, thick):
    m = BRepOffset_MakeOffset()
    m.Initialize(shape, d, 1e-7, BRepOffset_Skin, False, False, GeomAbs_Intersection, thick, False)
    m.MakeOffsetShape()
    if not m.IsDone():
        raise RuntimeError("OCCT's offset failed")
    return m.Shape()


def occt(case):
    s = sheet(case)
    if "thin" not in case:
        return {"sheet_area": area(s)}
    t, side = case["thin"], case["side"]
    k = front_sign(s, case["chain"])
    lo, hi = {"out": (0, t), "in": (-t, 0), "both": (-t / 2, t / 2)}[side]
    if lo != 0:
        s = offset(s, k * lo, False)
    solid = offset(s, k * (hi - lo), True)
    return {"volume": abs(volume(solid)), "area": area(solid), "valid": BRepCheck_Analyzer(solid).IsValid()}


def fg(case):
    ents, ref = fg_chain(case["chain"])
    out = "sketch s(on: xy) {\n" + ents + "}\n"
    if case["how"] == "extrude":
        lo, hi = case["lo"], case["hi"]
        d = f"{hi!r}" if lo == 0 else f"{lo!r}" if hi == 0 else f"two({hi!r}, {-lo!r})"
        if "thin" in case:
            return out + f"x = extrude({ref}, {d}, thin: {case['thin']!r}, side: \"{case['side']}\")\n"
        return out + f"x = extrude({ref}, {d}, surface: true)\n"
    return out + f"x = revolve({ref}, axis: Y, angle: {math.degrees(case['angle'])!r}deg, surface: true)\n"


def cases():
    L = [("line", (0.0, 0.0), (40.0, 0.0)), ("line", (40.0, 0.0), (40.0, 25.0))]
    acute = [("line", (0.0, 0.0), (30.0, 0.0)), ("line", (30.0, 0.0), (10.0, 15.0))]
    tangent = [("line", (0.0, 0.0), (10.0, 0.0)), ("arc", (10.0, 5.0), 5.0, -math.pi / 2, math.pi / 2)]
    corner = [("arc", (0.0, 10.0), 10.0, -math.pi / 2, math.pi / 3), ("line", arc_pts((0.0, 10.0), 10.0, -math.pi / 2, math.pi / 3)[2], (20.0, 12.0))]
    square = [("line", (0.0, 0.0), (10.0, 0.0)), ("line", (10.0, 0.0), (10.0, 10.0)), ("line", (10.0, 10.0), (0.0, 10.0)), ("line", (0.0, 10.0), (0.0, 0.0))]
    circle = [("circle", (0.0, 0.0), 6.0)]
    out = []
    for name, ch in [("L", L), ("acute", acute), ("tangent", tangent), ("corner", corner), ("square", square), ("circle", circle)]:
        out.append({"name": f"{name}-sheet", "chain": ch, "how": "extrude", "lo": 0.0, "hi": 10.0})
        for side in ("out", "in", "both"):
            out.append({"name": f"{name}-thin-{side}", "chain": ch, "how": "extrude", "lo": 0.0, "hi": 10.0, "thin": 1.5, "side": side})
    out.append({"name": "L-thin-down", "chain": L, "how": "extrude", "lo": -8.0, "hi": 0.0, "thin": 2.0, "side": "out"})
    out.append({"name": "L-sheet-two", "chain": L, "how": "extrude", "lo": -3.0, "hi": 5.0})
    wall = [("line", (5.0, 0.0), (5.0, 8.0)), ("line", (5.0, 8.0), (2.0, 12.0))]
    dome = [("arc", (0.0, 0.0), 4.0, 0.0, math.pi / 2)]
    for name, ch in [("wall", wall), ("dome", dome)]:
        for a in (math.pi / 2, 2 * math.pi):
            out.append({"name": f"{name}-revolve-{round(math.degrees(a))}", "chain": ch, "how": "revolve", "angle": a})
    return out


def main():
    a = sys.argv[1:]
    exe = a[a.index("--measure") + 1]
    tol = float(a[a.index("--tol") + 1]) if "--tol" in a else 1e-9
    bad = 0
    with tempfile.TemporaryDirectory() as d:
        for c in cases():
            p = os.path.join(d, c["name"] + ".fg")
            open(p, "w").write(fg(c))
            ours = json.loads(subprocess.run([exe, p], capture_output=True, text=True).stdout)
            try:
                ref = occt(c)
            except Exception as e:
                ref = {"error": str(e)}
            keys = [k for k in ("volume", "area", "sheet_area") if k in ref]
            diffs = {k: abs(ours.get(k, math.nan) - ref[k]) / max(abs(ref[k]), 1e-12) for k in keys}
            ok = ours.get("valid") and ref.get("valid", True) and keys and all(x <= tol for x in diffs.values())
            bad += not ok
            shown = " ".join(f"{k} {ours.get(k)!r} occt {ref[k]!r} ({diffs[k]:.1e})" for k in keys)
            print(f"{'ok  ' if ok else 'DIFF'} {c['name']:22} {shown} {ref.get('error', '')} {ours.get('errors') or ''}")
    print(f"{bad} of {len(cases())} differ")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
