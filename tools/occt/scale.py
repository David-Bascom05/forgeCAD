#!/usr/bin/env python3
"""OCCT check of the uniform scale (test only, never a dependency of the workspace).

For each part `cargo run --release -p forge-model --example scale_occt -- DIR` exported, reads its
STEP into OpenCascade and scales it there twice: gp_Trsf.SetScale with BRepBuilderAPI_Transform,
and the same map as a gp_GTrsf with BRepBuilderAPI_GTransform. Our scaled body (forge_topo::similar,
the kernel's own) must match each:

  - every vertex of ours within TOL of a vertex of OCCT's image, and as many of them;
  - every sampled point of each of our faces' surfaces within TOL of a surface of OCCT's image
    (on an analytic face, whose surface the file carries exactly; a procedural face is written as a
    B-spline fit, so there the bound is the file's stated fit, scaled, plus TOL);
  - OCCT's volume and area of its image k^3 and k^2 its volume and area of the original (to
    GProp's integration), and our exact volume and area k^3 and k^2 ours.

TOL is 1e-9 relative to the part's size (1e-9 x (1 + |p|)).

  tools/occt/scale.py DIR      (VENV python: tools/occt/setup.sh)
"""
import json
import math
import sys

from OCP.BRep import BRep_Tool
from OCP.BRepBuilderAPI import BRepBuilderAPI_GTransform, BRepBuilderAPI_Transform
from OCP.BRepGProp import BRepGProp
from OCP.GeomAPI import GeomAPI_ProjectPointOnSurf
from OCP.GProp import GProp_GProps
from OCP.gp import gp_GTrsf, gp_Pnt, gp_Trsf
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_FACE, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS

REL = 1e-9


def read(path):
    r = STEPControl_Reader()
    if r.ReadFile(path) != IFSelect_RetDone:
        raise RuntimeError(f"cannot read {path}")
    r.TransferRoots()
    return r.OneShape()


def each(shape, kind, cast):
    e = TopExp_Explorer(shape, kind)
    while e.More():
        yield cast(e.Current())
        e.Next()


def vprops(shape):
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, g)
    a = GProp_GProps()
    BRepGProp.SurfaceProperties_s(shape, a)
    return g.Mass(), a.Mass()


def scaled_occt(shape, c, k):
    t = gp_Trsf()
    t.SetScale(gp_Pnt(*c), k)
    one = BRepBuilderAPI_Transform(shape, t, True).Shape()
    g = gp_GTrsf()
    g.SetTrsf(t)
    two = BRepBuilderAPI_GTransform(shape, g, True).Shape()
    return {"Trsf": one, "GTrsf": two}


def tol(p):
    return REL * (1.0 + math.sqrt(sum(x * x for x in p)))


def main():
    d = sys.argv[1]
    cases = json.load(open(f"{d}/index.json"))
    shapes = {}
    worst = {"vertex": 0.0, "analytic": 0.0, "fitted": 0.0, "occt_mass": 0.0, "ours_mass": 0.0}
    bad = []
    for case in cases:
        name, c, k = case["name"], case["c"], case["k"]
        if name not in shapes:
            shapes[name] = read(f"{d}/{case['step']}")
        orig = shapes[name]
        v0, a0 = vprops(orig)
        for how, img in scaled_occt(orig, c, k).items():
            tag = f"{name} k={k} {how}"
            vs = [BRep_Tool.Pnt_s(v) for v in each(img, TopAbs_VERTEX, TopoDS.Vertex)]
            vs = {(round(p.X(), 6), round(p.Y(), 6), round(p.Z(), 6)): p for p in vs}.values()
            vs = list(vs)
            ours = case["verts"]
            if len(vs) != len(ours):
                bad.append(f"{tag}: {len(ours)} vertices, OCCT {len(vs)}")
            for p in ours:
                e = min(math.dist(p, (q.X(), q.Y(), q.Z())) for q in vs)
                worst["vertex"] = max(worst["vertex"], e / tol(p))
                if e > tol(p):
                    bad.append(f"{tag}: vertex {p} {e:.3e} from OCCT's")
            surfs = [BRep_Tool.Surface_s(f) for f in each(img, TopAbs_FACE, TopoDS.Face)]
            fit = case["fitted"] * k
            for p, analytic in case["pts"]:
                g = gp_Pnt(*p)
                e = math.inf
                for s in surfs:
                    pr = GeomAPI_ProjectPointOnSurf(g, s)
                    if pr.NbPoints() > 0:
                        e = min(e, pr.LowerDistance())
                key = "analytic" if analytic else "fitted"
                limit = tol(p) if analytic else tol(p) + fit
                worst[key] = max(worst[key], e / limit)
                if e > limit:
                    bad.append(f"{tag}: {key} surface point {p} {e:.3e} from OCCT's surfaces")
            v1, a1 = vprops(img)
            for what, got, want in (("volume", v1, v0 * k ** 3), ("area", a1, a0 * k * k)):
                r = abs(got - want) / abs(want)
                worst["occt_mass"] = max(worst["occt_mass"], r / REL)
                if r > REL:
                    bad.append(f"{tag}: OCCT {what} {got} vs k-scaled {want} ({r:.2e})")
        for what, got, want in (
            ("volume", case["v1"], case["v0"] * k ** 3),
            ("area", case["a1"], case["a0"] * k * k),
        ):
            r = abs(got - want) / abs(want)
            worst["ours_mass"] = max(worst["ours_mass"], r / REL)
            if r > REL:
                bad.append(f"{name} k={k}: our {what} {got} vs k-scaled {want} ({r:.2e})")
    parts = len({c["name"] for c in cases})
    print(f"{parts} parts, {len(cases)} scales, 2 OCCT transforms each")
    print("worst / bound: " + ", ".join(f"{k} {v:.3g}" for k, v in worst.items()))
    for b in bad:
        print("FAIL", b)
    print("OK" if not bad else f"{len(bad)} FAILURES")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
