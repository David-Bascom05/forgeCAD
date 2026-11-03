#!/usr/bin/env python3
"""An L and a U built in OCCT from prisms and revolves, none of our code: volume, area, validity to set
beside our direct construction. Run with the OCCT venv: $VENV/bin/python -I occt_ref.py"""
import math

from OCP.BRepAlgoAPI import BRepAlgoAPI_Fuse
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakePolygon, BRepBuilderAPI_Transform
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepPrimAPI import BRepPrimAPI_MakePrism, BRepPrimAPI_MakeRevol
from OCP.GProp import GProp_GProps
from OCP.ShapeUpgrade import ShapeUpgrade_UnifySameDomain
from OCP.gp import gp_Ax1, gp_Dir, gp_Pnt, gp_Vec


def rect(pts):
    p = BRepBuilderAPI_MakePolygon()
    for x, y, z in pts:
        p.Add(gp_Pnt(x, y, z))
    p.Close()
    return BRepBuilderAPI_MakeFace(p.Wire()).Face()


def build(walls, r, t, theta, width):
    """Walls by flat length between tangent lines; bends all r, theta. Bent towards +z, as the fold map."""
    ox, oz, a = 0.0, 0.0, 0.0  # the wall's start on the inside face, and its direction
    shape = None

    def add(s):
        nonlocal shape
        shape = s if shape is None else BRepAlgoAPI_Fuse(shape, s).Shape()

    for i, w in enumerate(walls):
        c, s = math.cos(a), math.sin(a)
        nx, nz = -s, c  # towards the bend's axis
        face = rect([(ox, 0, oz), (ox + w * c, 0, oz + w * s), (ox + w * c - t * nx, 0, oz + w * s - t * nz), (ox - t * nx, 0, oz - t * nz)])
        add(BRepPrimAPI_MakePrism(face, gp_Vec(0, width, 0)).Shape())
        ox, oz = ox + w * c, oz + w * s
        if i + 1 < len(walls):
            ax, az = ox + r * nx, oz + r * nz
            # the end section of this wall, turned about the bend's axis
            sec = rect([(ox, 0, oz), (ox - t * nx, 0, oz - t * nz), (ox - t * nx, width, oz - t * nz), (ox, width, oz)])
            # the fold goes from -n towards t: about -y in these axes
            add(BRepPrimAPI_MakeRevol(sec, gp_Ax1(gp_Pnt(ax, 0, az), gp_Dir(0, -1, 0)), theta).Shape())
            a2 = a + theta
            c2, s2 = math.cos(a2), math.sin(a2)
            n2x, n2z = -s2, c2
            ox, oz = ax - r * n2x, az - r * n2z
            a = a2
    u = ShapeUpgrade_UnifySameDomain(shape, True, True, True)
    u.Build()
    return u.Shape()


def measure(s):
    gv, ga = GProp_GProps(), GProp_GProps()
    BRepGProp.VolumeProperties_s(s, gv)
    BRepGProp.SurfaceProperties_s(s, ga)
    return gv.Mass(), ga.Mass(), BRepCheck_Analyzer(s).IsValid()


T, R, K, W = 2.0, 2.0, 0.4, 30.0


def legs(ls, deg):
    th = math.radians(deg)
    ossb = (R + T) * math.tan(th / 2)
    n = len(ls)
    return [x - (ossb if i > 0 else 0) - (ossb if i + 1 < n else 0) for i, x in enumerate(ls)], th


for name, ls, deg in [("L90", [40, 30], 90), ("L45", [40, 30], 45), ("L135", [40, 30], 135), ("U90", [40, 50, 30], 90), ("U60", [40, 50, 30], 60)]:
    walls, th = legs(ls, deg)
    v, a, ok = measure(build(walls, R, T, th, W))
    print(f"{name} occt-built valid {ok} V {v:.10f} A {a:.8f}")
