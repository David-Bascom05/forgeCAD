#!/usr/bin/env python3
"""The check behind truth_rule: a part whose filleted volume has a closed form, made and rounded by OCCT, measured
the three ways truth_rule takes.

A 100 x 60 x 10 block has an elliptical hole (semi-axes 20 and 10) through it, and both of the hole's edges are
rounded at r = 2. The rolling ball between a plane and a wall square to it takes away a section of (1 - pi/4) r^2,
whose centroid is xb = r (10 - 3 pi) / (3 (4 - pi)) from the edge. The section's plane is square to the edge
everywhere, and the material lies outside the hole, so by Pappus each round takes (1 - pi/4) r^2 (L + 2 pi xb), L
being the ellipse's perimeter (the curvature turns once round it).

  $VENV/bin/python -I truth_check.py
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import truth_rule  # noqa: E402

A, B, R = 20.0, 10.0, 2.0


def closed_form():
    n, L = 200000, 0.0
    for i in range(n):
        t0, t1 = 2 * math.pi * i / n, 2 * math.pi * (i + 1) / n
        L += math.hypot(A * (math.cos(t1) - math.cos(t0)), B * (math.sin(t1) - math.sin(t0)))
    sec = (1 - math.pi / 4) * R * R
    xb = R * (10 - 3 * math.pi) / (3 * (4 - math.pi))
    return 100 * 60 * 10 - math.pi * A * B * 10 - 2 * sec * (L + 2 * math.pi * xb)


def occt_part():
    from OCP.BRepAdaptor import BRepAdaptor_Curve
    from OCP.BRepAlgoAPI import BRepAlgoAPI_Cut
    from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakeWire
    from OCP.BRepFilletAPI import BRepFilletAPI_MakeFillet
    from OCP.BRepPrimAPI import BRepPrimAPI_MakeBox, BRepPrimAPI_MakePrism
    from OCP.TopAbs import TopAbs_EDGE
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopoDS import TopoDS
    from OCP.gp import gp_Ax2, gp_Dir, gp_Elips, gp_Pnt, gp_Vec

    box = BRepPrimAPI_MakeBox(gp_Pnt(-50, -30, 0), 100, 60, 10).Shape()
    el = gp_Elips(gp_Ax2(gp_Pnt(0, 0, -1), gp_Dir(0, 0, 1), gp_Dir(1, 0, 0)), A, B)
    face = BRepBuilderAPI_MakeFace(BRepBuilderAPI_MakeWire(BRepBuilderAPI_MakeEdge(el).Edge()).Wire()).Face()
    part = BRepAlgoAPI_Cut(box, BRepPrimAPI_MakePrism(face, gp_Vec(0, 0, 12)).Shape()).Shape()
    mk = BRepFilletAPI_MakeFillet(part)
    x = TopExp_Explorer(part, TopAbs_EDGE)
    while x.More():
        e = TopoDS.Edge(x.Current())
        c = BRepAdaptor_Curve(e)
        p = c.Value((c.FirstParameter() + c.LastParameter()) / 2)
        # the hole's edges: on the top or the bottom, on the ellipse
        if abs(p.Z() - 5) > 4.9 and p.X() ** 2 / A ** 2 + p.Y() ** 2 / B ** 2 < 1.01:
            mk.Add(R, e)
        x.Next()
    return mk.Shape()


def main():
    exact = closed_form()
    m = truth_rule.measures(occt_part())
    print(f"closed form             {exact:.9f}")
    for k, what in (("vol", "default"), ("vol_eps", "precise"), ("vol_gk", "Gauss-Kronrod")):
        print(f"OCCT, {what:<17} {m[k]:.9f}  off {abs(m[k] - exact) / exact:.1e}")


if __name__ == "__main__":
    main()
