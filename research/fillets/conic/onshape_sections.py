#!/usr/bin/env python3
"""Onshape's conic fillets as its STEP files hold them: for each B-spline face, the cross-section
across its middle (the iso line that is not straight), its two setbacks from the apex where its end
tangents meet (exact end tangents from the surface's derivatives), the turn between those tangents,
and its shoulder ratio rho (where the section crosses the line from the chord's middle to the apex).
Onshape writes conic blends as non-rational cubic B-splines, a fit: rho 0.5 (a parabola) is exact
in them, other rho only to the fit. Data stays outside the repo; run with OCCT's Python, -I.

  $VENV/bin/python -I onshape_sections.py FILE.step ..
"""
import math
import sys

import numpy as np
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepTools import BRepTools
from OCP.GeomAbs import GeomAbs_BSplineSurface
from OCP.gp import gp_Pnt, gp_Vec
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_FACE
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS


def vec(p):
    return np.array([p.X(), p.Y(), p.Z()])


def sections(path, n=4000):
    r = STEPControl_Reader()
    r.ReadFile(path)
    r.TransferRoots()
    ex = TopExp_Explorer(r.OneShape(), TopAbs_FACE)
    while ex.More():
        face = TopoDS.Face(ex.Current())
        a = BRepAdaptor_Surface(face)
        ex.Next()
        if a.GetType() != GeomAbs_BSplineSurface:
            continue
        u0, u1, v0, v1 = BRepTools.UVBounds_s(face)
        for along in ("u", "v"):
            def at(k):
                return ((u0 + u1) / 2, v0 + (v1 - v0) * k / n) if along == "u" else (u0 + (u1 - u0) * k / n, (v0 + v1) / 2)
            pts = np.array([vec(a.Value(*at(k))) for k in range(n + 1)])
            chord = pts[-1] - pts[0]
            length = np.linalg.norm(chord)
            if length > 0 and max(np.linalg.norm(np.cross(pts - pts[0], chord / length), axis=1)) > 1e-3 * length:
                break
        else:
            continue

        def tangent(uv):
            p, du, dv = gp_Pnt(), gp_Vec(), gp_Vec()
            a.D1(uv[0], uv[1], p, du, dv)
            d = dv if along == "u" else du
            t = np.array([d.X(), d.Y(), d.Z()])
            return t / np.linalg.norm(t)

        p0, p3 = pts[0], pts[-1]
        t0, t3 = tangent(at(0)), -tangent(at(n))
        m = np.array([[t0 @ t0, -t0 @ t3], [t0 @ t3, -t3 @ t3]])
        s0, s3 = np.linalg.solve(m, np.array([(p3 - p0) @ t0, (p3 - p0) @ t3]))
        apex = 0.5 * (p0 + s0 * t0 + p3 + s3 * t3)
        mid = 0.5 * (p0 + p3)
        d = apex - mid
        dl = np.linalg.norm(d)
        i = int(np.argmin(np.linalg.norm(np.cross(pts - mid, d / dl), axis=1)))
        turn = math.degrees(math.acos(np.clip(-t0 @ t3, -1, 1)))
        yield s0, s3, turn, ((pts[i] - mid) @ d) / dl ** 2


if __name__ == "__main__":
    for f in sys.argv[1:]:
        print(f.split("/")[-1])
        for s0, s3, turn, rho in sections(f):
            print("  setbacks %.5f %.5f  turn %.3f deg  rho %.4f" % (s0, s3, turn, rho))
