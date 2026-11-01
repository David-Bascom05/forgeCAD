#!/usr/bin/env python3
"""What an .FCStd stores of each object's result, read as data by OpenCascade (FreeCAD is never run, and
nothing in the file executes): volume, area, box, and samples of the edges and faces a fillet or chamfer names.
Samples are evaluated here, with dmath, from the curves as OpenCascade read them, so they are the same numbers the Rust
importer (interface/forge-import) evaluates.

  $VENV/bin/python -I fcstd_truth.py FILE.FCStd OBJ.json     OBJ.json from freecad.py: {"shapes": {name: file}, "subs": {name: [Edge3, Face2..]}}
"""
import json
import math
import os
import resource
import sys
import tempfile
import zipfile

resource.setrlimit(resource.RLIMIT_AS, (3 << 30, 3 << 30))
from OCP.Bnd import Bnd_Box
from OCP.BRep import BRep_Builder, BRep_Tool
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.Geom import Geom_BezierCurve, Geom_BSplineCurve, Geom_Circle, Geom_Ellipse, Geom_Line, Geom_TrimmedCurve
from OCP.GeomAbs import GeomAbs_Circle, GeomAbs_Line
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.BRepTools import BRepTools
from OCP.GProp import GProp_GProps
from OCP.OCP.collections import IndexedMap_TopoDS_Shape_TopTools_ShapeMapHasher as ShapeMap
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_SOLID, TopAbs_VERTEX, TopAbs_WIRE
from OCP.TopExp import TopExp, TopExp_Explorer
from OCP.TopLoc import TopLoc_Location
from OCP.TopoDS import TopoDS, TopoDS_Shape

N = 256
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dmath  # noqa: E402


def read(path):
    s = TopoDS_Shape()
    if not BRepTools.Read_s(s, path, BRep_Builder()):
        return None
    return s


def flat_knots(deg, knots, poles, periodic):
    """forge-step's flat_knots: the whole knot vector, a periodic one unrolled past both ends, and the turn."""
    if not periodic:
        return [k for k, m in knots for _ in range(m)], 0, 0
    period = knots[-1][0] - knots[0][0]
    one = [k for k, m in knots[:-1] for _ in range(m)]
    if len(one) != poles or period <= 0:
        return None
    at = lambda i: one[i % poles] + (i // poles) * period
    first = knots[0][1] - deg - 1
    return [at(first + i) for i in range(poles + 2 * deg + 1)], deg, knots[0][1] % poles


def identity(loc):
    """No move at all, though a file may write one: FreeCAD stores identity matrices as locations."""
    t = loc.Transformation()
    return all(t.Value(i, j) == (1.0 if i == j else 0.0) for i in (1, 2, 3) for j in (1, 2, 3, 4))


def own_curve(e, moved=False):
    """The edge's curve as the Rust importer reads it (forge-step BrepTopo), or None where it reads none or a location
    moves it: then OpenCascade samples it. moved: the curve in its own place, and its location, whatever that is."""
    t = TopoDS.Edge(e)
    if BRep_Tool.Degenerated_s(t):
        if moved:
            return None
        v = TopExp.FirstVertex_s(t)
        p = BRep_Tool.Pnt_s(v)
        return ("point", [p.X(), p.Y(), p.Z()])
    loc = TopLoc_Location()
    ad = BRepAdaptor_Curve(t)
    a, b = ad.FirstParameter(), ad.LastParameter()
    c = BRep_Tool.Curve_s(t, loc, a, b)
    if moved:
        c = own_basis(c, a, b)
        return (c, loc) if c is not None else None
    if c is None or not identity(loc):
        return None
    return own_basis(c, a, b)


def own_basis(c, a, b):
    if c is None:
        return None
    while isinstance(c, Geom_TrimmedCurve):
        c = c.BasisCurve()
    xyz = lambda p: [p.X(), p.Y(), p.Z()]
    if isinstance(c, Geom_Line):
        ax = c.Position()
        return ("line", xyz(ax.Location()), xyz(ax.Direction()), a, b)
    if isinstance(c, (Geom_Circle, Geom_Ellipse)):
        ax = c.Position()
        r1, r2 = (c.Radius(), c.Radius()) if isinstance(c, Geom_Circle) else (c.MajorRadius(), c.MinorRadius())
        return ("conic", xyz(ax.Location()), xyz(ax.XDirection()), xyz(ax.YDirection()), r1, r2, a, b)
    if isinstance(c, (Geom_BSplineCurve, Geom_BezierCurve)):
        n = c.NbPoles()
        ps = [xyz(c.Pole(i)) for i in range(1, n + 1)]
        ws = [c.Weight(i) for i in range(1, n + 1)] if c.IsRational() else [1.0] * n
        if isinstance(c, Geom_BezierCurve):
            deg, knots, per = c.Degree(), [(0.0, c.Degree() + 1), (1.0, c.Degree() + 1)], False
        else:
            deg, per = c.Degree(), c.IsPeriodic()
            knots = [(c.Knot(i), c.Multiplicity(i)) for i in range(1, c.NbKnots() + 1)]
        f = flat_knots(deg, knots, n, per)
        if f is None:
            return None
        flat, extra, turn = f
        poles = [[q[0] * w, q[1] * w, q[2] * w, w] for q, w in zip(ps, ws)]
        poles = [poles[(i + n - turn) % n] for i in range(n + extra)]
        return ("spline", deg, flat, poles, (knots[0][0], knots[-1][0]) if per else None, a, b)
    return None


def own_value(c, u):
    """forge-step's eval_raw, operation for operation, with dmath's sin and cos."""
    if c[0] == "line":
        o, d = c[1], c[2]
        return [o[0] + u * d[0], o[1] + u * d[1], o[2] + u * d[2]]
    if c[0] == "conic":
        o, x, y, ra, rb = c[1:6]
        s, co = dmath.sin_cos(u)
        a1, a2 = ra * co, rb * s
        return [a1 * x[0] + a2 * y[0] + o[0], a1 * x[1] + a2 * y[1] + o[1], a1 * x[2] + a2 * y[2] + o[2]]
    deg, k, poles, per = c[1:5]
    if per is not None:
        lo, hi = per
        p = hi - lo
        if p > 0:
            while u > hi:
                u -= p
            while u < lo:
                u += p
    last = len(poles) - 1
    s = deg
    while s < last and k[s + 1] <= u:
        s += 1
    d = [list(poles[s - deg + j]) for j in range(deg + 1)]
    for r in range(1, deg + 1):
        for j in range(deg, r - 1, -1):
            i = s - deg + j
            den = k[i + deg - r + 1] - k[i]
            a = (u - k[i]) / den if den != 0.0 else 0.0
            d[j] = [p * (1.0 - a) + x * a for p, x in zip(d[j - 1], d[j])]
    w = d[deg][3]
    return [d[deg][0] / w, d[deg][1] / w, d[deg][2] / w]


def edge_points(e, n=N):
    """n + 1 points at even steps over the edge's range: our own evaluation where the Rust importer has one, so both
    read the same numbers; OpenCascade's otherwise."""
    c = own_curve(e)
    if c is not None and c[0] == "point":
        return [list(c[1]) for _ in range(n + 1)]
    if c is not None:
        a, b = c[-2], c[-1]
        return [own_value(c, a + (b - a) * i / n) for i in range(n + 1)]
    c = BRepAdaptor_Curve(TopoDS.Edge(e))
    a, b = c.FirstParameter(), c.LastParameter()
    out = []
    for i in range(n + 1):
        p = c.Value(a + (b - a) * i / n)
        out.append([p.X(), p.Y(), p.Z()])
    return out


def wires(s):
    """A profile that is not a sketch: each wire's edges, what curve each is and samples along it, and where only the
    shape's own location places it, its samples in that place ("l"): the profile's placement is that location, and its
    sketch coordinates are these without a transform either side has to round alike."""
    out = []
    root = s.Location()
    x = TopExp_Explorer(s, TopAbs_WIRE)
    while x.More():
        w, y = [], TopExp_Explorer(x.Current(), TopAbs_EDGE)
        while y.More():
            c = BRepAdaptor_Curve(TopoDS.Edge(y.Current()))
            k = {GeomAbs_Line: "line", GeomAbs_Circle: "circle"}.get(c.GetType(), "other")
            d = {"k": k, "p": edge_points(y.Current(), 96)}
            m = own_curve(y.Current(), moved=True)
            if m is not None and m[1].IsEqual(root):
                a, b = m[0][-2], m[0][-1]
                d["l"] = [own_value(m[0], a + (b - a) * i / 96) for i in range(97)]
            w.append(d)
            y.Next()
        out.append(w)
        x.Next()
    return out


def main():
    want = json.load(open(sys.argv[2]))
    z = zipfile.ZipFile(sys.argv[1])
    tmp = tempfile.mkdtemp(prefix="brp-")
    out = {}
    try:
        for name, f in want["shapes"].items():
            p = os.path.join(tmp, "s.brp")
            with open(p, "wb") as w:
                w.write(z.read(f))
            s = read(p)
            if s is None or s.IsNull():
                out[name] = {"err": "unreadable"}
                continue
            r = {"solids": 0, "vol": 0.0, "area": 0.0}
            x = TopExp_Explorer(s, TopAbs_SOLID)
            box = Bnd_Box()
            while x.More():
                g = GProp_GProps()
                BRepGProp.VolumeProperties_s(x.Current(), g)
                r["vol"] += abs(g.Mass())
                g = GProp_GProps()
                BRepGProp.SurfaceProperties_s(x.Current(), g)
                r["area"] += g.Mass()
                BRepBndLib.AddOptimal_s(x.Current(), box, True, False)
                r["solids"] += 1
                x.Next()
            if r["solids"]:
                a, b = box.CornerMin(), box.CornerMax()
                r["bbox"] = [a.X(), a.Y(), a.Z(), b.X(), b.Y(), b.Z()]
            r["valid"] = bool(BRepCheck_Analyzer(s).IsValid())
            if r["solids"]:
                # a box both importers make alike: the vertices'
                vm, vb = ShapeMap(), [math.inf] * 3 + [-math.inf] * 3
                TopExp.MapShapes_s(s, TopAbs_VERTEX, vm)
                for i in range(1, vm.Extent() + 1):
                    p = BRep_Tool.Pnt_s(TopoDS.Vertex(vm.FindKey(i)))
                    for k, x in enumerate((p.X(), p.Y(), p.Z())):
                        vb[k], vb[k + 3] = min(vb[k], x), max(vb[k + 3], x)
                r["vbox"] = vb
            if name in want.get("wires", []):
                t = s.Location().Transformation()
                r["root"] = [[t.Value(i, j) for j in (1, 2, 3, 4)] for i in (1, 2, 3)]
                r["wires"] = wires(s)
            subs = want["subs"].get(name)
            if subs:
                em, fm = ShapeMap(), ShapeMap()
                TopExp.MapShapes_s(s, TopAbs_EDGE, em)
                TopExp.MapShapes_s(s, TopAbs_FACE, fm)
                got = {}
                for sub in subs:
                    if sub == "ALL":
                        got[sub] = [edge_points(em.FindKey(i)) for i in range(1, em.Extent() + 1)]
                    elif sub.startswith("Edge"):
                        got[sub] = [edge_points(em.FindKey(int(sub[4:])))]
                    elif sub.startswith("Face"):
                        fe = ShapeMap()
                        TopExp.MapShapes_s(fm.FindKey(int(sub[4:])), TopAbs_EDGE, fe)
                        got[sub] = [edge_points(fe.FindKey(i)) for i in range(1, fe.Extent() + 1)]
                r["subs"] = got
            out[name] = r
    finally:
        for n in os.listdir(tmp):
            os.remove(os.path.join(tmp, n))
        os.rmdir(tmp)
    print(json.dumps(out))


if __name__ == "__main__":
    main()
