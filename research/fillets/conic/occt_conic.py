#!/usr/bin/env python3
"""OCCT reference for conic fillets: each examples/conic part built again in OpenCascade from exact
pieces, the conic section as a rational quadratic B-spline (feet, apex; middle weight rho/(1-rho)),
the fill region swept straight (BRepPrimAPI_MakePrism) or turned (MakeRevol) and taken off or added
by a boolean. OCCT's own fillet makes circular sections only, so this is the exact construction it
can make. Prints volume and area per part; the mitre of rim.fg is the extended prisms cut each one.

  $VENV/bin/python -I occt_conic.py
"""
import json
import math

from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut, BRepAlgoAPI_Fuse
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakeWire
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakeCylinder, BRepPrimAPI_MakePrism, BRepPrimAPI_MakeRevol
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakePolygon
from OCP.Geom import Geom_BezierCurve

from OCP.GProp import GProp_GProps
from OCP.gp import gp_Ax1, gp_Ax2, gp_Dir, gp_Pnt, gp_Vec
from OCP.OCP.collections import Array1_double, Array1_gp_Pnt


def P(p):
    return gp_Pnt(*p)


def region(fa, apex, fb, rho):
    """The face between the conic (fa, apex, fb) and the two legs to the apex."""
    w = rho / (1 - rho)
    poles = Array1_gp_Pnt(1, 3)
    weights = Array1_double(1, 3)
    for i, (p, wi) in enumerate([(fa, 1.0), (apex, w), (fb, 1.0)]):
        poles.SetValue(i + 1, P(p))
        weights.SetValue(i + 1, wi)
    curve = Geom_BezierCurve(poles, weights)
    wire = BRepBuilderAPI_MakeWire()
    wire.Add(BRepBuilderAPI_MakeEdge(curve).Edge())
    wire.Add(BRepBuilderAPI_MakeEdge(P(fb), P(apex)).Edge())
    wire.Add(BRepBuilderAPI_MakeEdge(P(apex), P(fa)).Edge())
    return BRepBuilderAPI_MakeFace(wire.Wire(), True).Face()


def prism(fa, apex, fb, rho, along):
    return BRepPrimAPI_MakePrism(region(fa, apex, fb, rho), gp_Vec(*along)).Shape()


def turned(fa, apex, fb, rho, origin, axis, angle=2 * math.pi):
    return BRepPrimAPI_MakeRevol(region(fa, apex, fb, rho), gp_Ax1(P(origin), gp_Dir(*axis)), angle).Shape()


def box(x0, y0, z0, dx, dy, dz):
    return BRepPrimAPI_MakeBox(P((x0, y0, z0)), dx, dy, dz).Shape()


def cut(a, b):
    return BRepAlgoAPI_Cut(a, b).Shape()


def fuse(a, b):
    return BRepAlgoAPI_Fuse(a, b).Shape()


def prism_from(points, along):
    poly = BRepBuilderAPI_MakePolygon()
    for p in points:
        poly.Add(P(p))
    poly.Close()
    return BRepPrimAPI_MakePrism(BRepBuilderAPI_MakeFace(poly.Wire(), True).Face(), gp_Vec(*along)).Shape()


def edge_fg():
    L, W, H, r, rho = 40, 30, 20, 5, 0.5
    return cut(box(0, 0, 0, L, W, H), prism((L, W - r, -1), (L, W, -1), (L - r, W, -1), rho, (0, 0, H + 2)))


def rim_fg():
    L, W, H, r, rho = 40, 30, 20, 6, 0.7
    m = 10
    s = box(0, 0, 0, L, W, H)
    # along x at y = 0 and y = W, along y at x = 0 and x = L, each run on past the box
    s = cut(s, prism((-m, r, H), (-m, 0, H), (-m, 0, H - r), rho, (L + 2 * m, 0, 0)))
    s = cut(s, prism((-m, W - r, H), (-m, W, H), (-m, W, H - r), rho, (L + 2 * m, 0, 0)))
    s = cut(s, prism((r, -m, H), (0, -m, H), (0, -m, H - r), rho, (0, W + 2 * m, 0)))
    s = cut(s, prism((L - r, -m, H), (L, -m, H), (L, -m, H - r), rho, (0, W + 2 * m, 0)))
    return s


def step_fg():
    L, W, T, S, r, rho = 40, 30, 8, 12, 5, 0.3
    pts = [(0, 0, 0), (L, 0, 0), (L, 0, T), (S, 0, T), (S, 0, 3 * T), (0, 0, 3 * T)]
    # xz's sketch extrudes along -y
    s = prism_from(pts, (0, -W, 0))
    # a fill is added: exactly the part's width, not run on past it
    return fuse(s, prism((S + r, 0, T), (S, 0, T), (S, 0, T + r), rho, (0, -W, 0)))


def wedge_fg():
    W, B, H, r, rho = 30, 40, 30, 4, 0.6
    cx = B - H / math.tan(math.radians(60))
    pts = [(0, 0, 0), (B, 0, 0), (cx, 0, H), (0, 0, H)]
    s = prism_from(pts, (0, -W, 0))
    # the faces turn by 60 degrees: each foot r tan(30deg) from the edge
    d = r * math.tan(math.radians(30))
    down = ((B - cx) / H, -1.0)
    n = math.hypot(*down)
    fa = (cx - d, 1, H)
    fb = (cx + d * down[0] / n, 1, H + d * down[1] / n)
    return cut(s, prism(fa, (cx, 1, H), fb, rho, (0, -W - 2, 0)))


def boss_fg():
    Pl, T, R, H = 60, 8, 10, 12
    s = fuse(box(0, 0, 0, Pl, Pl, T), BRepPrimAPI_MakeCylinder(gp_Ax2(P((30, 30, T)), gp_Dir(0, 0, 1)), R, H).Shape())
    c = (30, 30, 0)
    s = fuse(s, turned((30 + R + 3, 30, T), (30 + R, 30, T), (30 + R, 30, T + 3), 0.5, c, (0, 0, 1)))
    s = cut(s, turned((30 + R - 3, 30, T + H), (30 + R, 30, T + H), (30 + R, 30, T + H - 3), 0.8, c, (0, 0, 1)))
    return s


def slot_fg():
    L, R, H, r, rho = 40, 8, 10, 3, 0.5
    # the slot: a box between the arcs' centres and a cylinder at each end
    s = box(0, -R, 0, L, 2 * R, H)
    for x in (0, L):
        s = fuse(s, BRepPrimAPI_MakeCylinder(gp_Ax2(P((x, 0, 0)), gp_Dir(0, 0, 1)), R, H).Shape())
    take = prism((0, -R + r, H), (0, -R, H), (0, -R, H - r), rho, (L, 0, 0))
    take = fuse(take, prism((0, R - r, H), (0, R, H), (0, R, H - r), rho, (L, 0, 0)))
    # each end's half turn, from the line's end round the far side
    take = fuse(take, turned((L, -R + r, H), (L, -R, H), (L, -R, H - r), rho, (L, 0, 0), (0, 0, 1), math.pi))
    take = fuse(take, turned((0, R - r, H), (0, R, H), (0, R, H - r), rho, (0, 0, 0), (0, 0, 1), math.pi))
    return cut(s, take)


def measure(s):
    # adaptive, to a relative error of 1e-12 (the default rule is good to about 1e-7 here)
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(s, g, 1e-12, True)
    a = GProp_GProps()
    BRepGProp.SurfaceProperties_s(s, a, 1e-12)
    return {"valid": BRepCheck_Analyzer(s).IsValid(), "vol": g.Mass(), "area": a.Mass()}


if __name__ == "__main__":
    for name, f in [("boss", boss_fg), ("edge", edge_fg), ("rim", rim_fg), ("slot", slot_fg), ("step", step_fg), ("wedge", wedge_fg)]:
        print(json.dumps({"part": name, **measure(f())}))


