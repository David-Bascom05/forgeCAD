"""Every feature kind the oracle example exports, rebuilt with OpenCascade's own operations.

`build(features)` replays a part's history: solid features from their definitions (extents and
holes worked out here, against the part as OCCT has it), finishing ops and face edits on the
edges and faces through the points the exporter picked off our part. See tools/occt/oracle.py.
"""
import math

from OCP.BRep import BRep_Tool
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepAlgoAPI import (BRepAlgoAPI_Common, BRepAlgoAPI_Cut, BRepAlgoAPI_Defeaturing, BRepAlgoAPI_Fuse,
                             BRepAlgoAPI_Splitter)
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepBuilderAPI import (BRepBuilderAPI_MakeEdge, BRepBuilderAPI_MakeFace, BRepBuilderAPI_MakeSolid,
                                BRepBuilderAPI_MakeVertex, BRepBuilderAPI_MakeWire, BRepBuilderAPI_Sewing,
                                BRepBuilderAPI_Transform)
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepFilletAPI import BRepFilletAPI_MakeChamfer, BRepFilletAPI_MakeFillet
from OCP.BRepGProp import BRepGProp
from OCP.BRepLib import BRepLib
from OCP.BRepOffset import BRepOffset_MakeOffset, BRepOffset_Skin
from OCP.BRepOffsetAPI import (BRepOffsetAPI_DraftAngle, BRepOffsetAPI_MakeOffsetShape, BRepOffsetAPI_MakePipe,
                               BRepOffsetAPI_MakePipeShell, BRepOffsetAPI_MakeThickSolid, BRepOffsetAPI_ThruSections)
from OCP.BRepPrimAPI import BRepPrimAPI_MakePrism, BRepPrimAPI_MakeRevol, BRepPrimAPI_MakeSphere
from OCP.Bnd import Bnd_Box
from OCP.GC import GC_MakeArcOfCircle
from OCP.GProp import GProp_GProps
from OCP.Geom import Geom_BezierCurve, Geom_CylindricalSurface
from OCP.Geom2d import Geom2d_BezierCurve, Geom2d_Line
from OCP.GeomAbs import GeomAbs_Intersection, GeomAbs_Plane
from OCP.IntCurvesFace import IntCurvesFace_ShapeIntersector
from OCP.OCP.collections import Array1_double, Array1_gp_Pnt, Array1_gp_Pnt2d
from OCP.OCP.collections import IndexedMap_TopoDS_Shape_TopTools_ShapeMapHasher as ShapeMap
from OCP.OCP.collections import List_TopoDS_Shape as ShapeList
from OCP.ShapeFix import ShapeFix_Face
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_IN, TopAbs_SHELL, TopAbs_SOLID
from OCP.TopExp import TopExp
from OCP.TopoDS import TopoDS, TopoDS_Compound
from OCP.BRep import BRep_Builder
from OCP.gp import (gp_Ax1, gp_Ax2, gp_Ax3, gp_Circ, gp_Cone, gp_Cylinder, gp_Dir, gp_Dir2d, gp_Lin, gp_Pln, gp_Pnt,
                    gp_Pnt2d, gp_Sphere, gp_Torus, gp_Trsf, gp_Vec)

TAU = 2 * math.pi


def add(a, b, k=1.0):
    return [a[i] + b[i] * k for i in range(3)]


def sub(a, b):
    return [a[i] - b[i] for i in range(3)]


def dot(a, b):
    return sum(a[i] * b[i] for i in range(3))


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def norm(a):
    n = math.sqrt(dot(a, a))
    return [c / n for c in a]


class Frame:
    def __init__(self, f):
        self.o, self.x, self.y, self.z = f["o"], f["x"], f["y"], f["z"]

    def pt(self, u, v, w=0.0):
        return [self.o[i] + self.x[i] * u + self.y[i] * v + self.z[i] * w for i in range(3)]

    def gpt(self, u, v, w=0.0):
        return gp_Pnt(*self.pt(u, v, w))

    def vec(self, u, v, w=0.0):
        return [self.x[i] * u + self.y[i] * v + self.z[i] * w for i in range(3)]

    def ax3(self):
        return gp_Ax3(gp_Pnt(*self.o), gp_Dir(*self.z), gp_Dir(*self.x))

    def ax2(self):
        return gp_Ax2(gp_Pnt(*self.o), gp_Dir(*self.z), gp_Dir(*self.x))


def shapes(shape, kind):
    m = ShapeMap()
    TopExp.MapShapes_s(shape, kind, m)
    return [m.FindKey(i) for i in range(1, m.Extent() + 1)]


def volume(shape):
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, g)
    return g.Mass()


def bbox(shape):
    b = Bnd_Box()
    BRepBndLib.Add_s(shape, b)
    lo, hi = b.CornerMin(), b.CornerMax()
    return [lo.X(), lo.Y(), lo.Z()], [hi.X(), hi.Y(), hi.Z()]


def corners(shape):
    lo, hi = bbox(shape)
    return [[(lo, hi)[i >> k & 1][k] for k in range(3)] for i in range(8)]


def size(shape):
    lo, hi = bbox(shape)
    return math.dist(lo, hi)


# profiles

def arc_edge(pts):
    return BRepBuilderAPI_MakeEdge(GC_MakeArcOfCircle(*[gp_Pnt(*p) for p in pts]).Value()).Edge()


def bezier_edges(at, b):
    """A free curve as one edge per Bezier piece (its own degree, weights as given)."""
    d, pts = b["d"], b["p"]
    out = []
    for i in range(0, len(pts), d + 1):
        poles = Array1_gp_Pnt(1, d + 1)
        ws = Array1_double(1, d + 1)
        for k, (x, y, w) in enumerate(pts[i : i + d + 1]):
            poles.SetValue(k + 1, gp_Pnt(*at(x, y)))
            ws.SetValue(k + 1, w)
        out.append(BRepBuilderAPI_MakeEdge(Geom_BezierCurve(poles, ws)).Edge())
    return out


def seg_edge(at, s):
    """A sketch segment as an edge; `at(u, v)` places sketch points in space."""
    if "line" in s:
        a, b, c, d = s["line"]
        return BRepBuilderAPI_MakeEdge(gp_Pnt(*at(a, b)), gp_Pnt(*at(c, d))).Edge()
    if "arc" in s:
        cx, cy, r, st, sw = s["arc"]
        return arc_edge([at(cx + r * math.cos(t), cy + r * math.sin(t)) for t in (st, st + sw / 2, st + sw)])
    cx, cy, r, _ = s["circle"]
    # a whole circle: its plane through three points of it
    p = [at(cx + r * math.cos(t), cy + r * math.sin(t)) for t in (0, 2 * math.pi / 3, 4 * math.pi / 3)]
    o = at(cx, cy)
    n = norm(cross(sub(p[0], o), sub(p[1], o)))
    c = gp_Circ(gp_Ax2(gp_Pnt(*o), gp_Dir(*n), gp_Dir(*norm(sub(p[0], o)))), r)
    return BRepBuilderAPI_MakeEdge(c).Edge()


def wire(at, lp):
    mw = BRepBuilderAPI_MakeWire()
    for s in lp:
        for e in bezier_edges(at, s["bezier"]) if "bezier" in s else [seg_edge(at, s)]:
            mw.Add(e)
    if not mw.IsDone():
        raise RuntimeError("a loop that does not close")
    return mw.Wire()


def planar_face(wires):
    mf = BRepBuilderAPI_MakeFace(wires[0], True)
    for w in wires[1:]:
        mf.Add(w)
    if not mf.IsDone():
        raise RuntimeError("a profile that makes no face")
    fix = ShapeFix_Face(mf.Face())
    fix.FixOrientation()
    fix.Perform()
    return fix.Face()


def face(prof):
    pl = Frame(prof["plane"])
    return planar_face([wire(pl.pt, lp) for lp in prof["loops"]]), pl


def prism(fc, d):
    return BRepPrimAPI_MakePrism(fc, gp_Vec(*d)).Shape()


def moved(shape, m):
    """The shape under the affine map whose columns are m (images of x, y, z and the origin)."""
    t = gp_Trsf()
    x, y, z, o = m
    t.SetValues(x[0], y[0], z[0], o[0], x[1], y[1], z[1], o[1], x[2], y[2], z[2], o[2])
    return BRepBuilderAPI_Transform(shape, t, True).Shape()


def translated(shape, d):
    t = gp_Trsf()
    t.SetTranslation(gp_Vec(*d))
    return BRepBuilderAPI_Transform(shape, t, True).Shape()


def rotated(shape, o, d, a):
    t = gp_Trsf()
    t.SetRotation(gp_Ax1(gp_Pnt(*o), gp_Dir(*d)), a)
    return BRepBuilderAPI_Transform(shape, t, True).Shape()


def compound(parts):
    c = TopoDS_Compound()
    b = BRep_Builder()
    b.MakeCompound(c)
    for p in parts:
        b.Add(c, p)
    return c


# booleans

def boolean(body, tools, op, opts):
    algo = {"add": BRepAlgoAPI_Fuse, "cut": BRepAlgoAPI_Cut, "keep": BRepAlgoAPI_Common}[op]()
    args, ts = ShapeList(), ShapeList()
    args.Append(body)
    for t in tools:
        ts.Append(t)
    algo.SetArguments(args)
    algo.SetTools(ts)
    algo.SetRunParallel(opts.parallel)
    if opts.fuzzy > 0:
        algo.SetFuzzyValue(opts.fuzzy)
    algo.Build()
    if not algo.IsDone():
        raise RuntimeError(f"boolean {op} failed")
    return algo.Shape()


def apply(body, tools, opts):
    """Tools [(solid, op)] combined with the body in order (a run of adds or of cuts at once)."""
    i = 0
    while i < len(tools):
        t, op = tools[i]
        if op == "new" or body is None:
            body, i = t, i + 1
            continue
        j = i + 1
        while op != "keep" and j < len(tools) and tools[j][1] == op:
            j += 1
        body = boolean(body, [t for t, _ in tools[i:j]], op, opts)
        i = j
    return body


# picking our edges and faces out of OCCT's

def nearest(body, kind, at, tol=1e-4):
    v = BRepBuilderAPI_MakeVertex(gp_Pnt(*at)).Vertex()
    best, bd = None, math.inf
    for s in shapes(body, kind):
        d = BRepExtrema_DistShapeShape(v, s)
        if d.IsDone() and d.Value() < bd:
            best, bd = s, d.Value()
    if bd > tol:
        raise RuntimeError(f"nothing of OCCT's through {[round(c, 4) for c in at]} ({bd:.2g} off)")
    return TopoDS.Edge(best) if kind == TopAbs_EDGE else TopoDS.Face(best)


def picked(body, kind, pts):
    """The distinct edges or faces through any of the points (OCCT may split one of ours)."""
    out = []
    for p in pts:
        s = nearest(body, kind, p)
        if not any(s.IsSame(o) for o in out):
            out.append(s)
    return out


# solid features

def surface_face(s, big):
    """A face of the whole surface (planes and cylinders `big` across), to split a tool with."""
    f = Frame(s["f"])
    k = s["s"]
    if k == "plane":
        return BRepBuilderAPI_MakeFace(gp_Pln(f.ax3()), -big, big, -big, big).Face()
    if k == "cylinder":
        return BRepBuilderAPI_MakeFace(gp_Cylinder(f.ax3(), s["r"]), 0, TAU, -big, big).Face()
    if k == "sphere":
        return BRepBuilderAPI_MakeFace(gp_Sphere(f.ax3(), s["r"])).Face()
    if k == "torus":
        return BRepBuilderAPI_MakeFace(gp_Torus(f.ax3(), s["R"], s["r"])).Face()
    if k == "cone":
        a = math.atan(s["t"])
        apex = -s["r"] / math.sin(a)
        # the nappe our cone is: radius r + h tan a >= 0
        lo, hi = (apex, apex + big) if math.sin(a) > 0 else (apex - big, apex)
        return BRepBuilderAPI_MakeFace(gp_Cone(f.ax3(), a, s["r"]), 0, TAU, lo, hi).Face()
    raise RuntimeError(f"an end on a {k}")


def toward(s, p, z):
    """Which way along z from p the surface lies (as the kernel decides it): +1 or -1."""
    f = Frame(s["f"])
    k = s["s"]
    if k == "plane":
        t = dot(f.z, sub(f.o, p)) / dot(f.z, z)
        return -1.0 if t < 0 else 1.0
    if k == "cylinder":
        off = sub(f.o, p)
        across = add(off, f.z, -dot(f.z, off))
        return -1.0 if dot(across, z) < 0 and math.sqrt(dot(across, across)) > s["r"] else 1.0
    at = f.o
    if k == "cone":
        at = add(f.o, f.z, -s["r"] / s["t"])
    return -1.0 if dot(z, sub(at, p)) < 0 else 1.0


def offset_surface(s, refp, z, offset):
    """The surface moved `offset` on past it, away from refp (where the sweep comes from)."""
    s = dict(s)
    f = Frame(s["f"])
    k = s["s"]
    if k == "plane":
        n = f.z if dot(f.z, z) > 0 else [-c for c in f.z]
        s["f"] = dict(s["f"], o=add(f.o, n, offset))
    elif k in ("sphere", "cylinder"):
        off = sub(refp, f.o)
        if k == "cylinder":
            off = add(off, f.z, -dot(off, f.z))
        inside = math.sqrt(dot(off, off)) < s["r"]
        s["r"] = s["r"] + offset if inside else s["r"] - offset
    elif k == "cone":
        a = math.atan(s["t"])
        apex = add(f.o, f.z, -s["r"] / s["t"])
        off = sub(refp, apex)
        inside = dot(f.z, off) > math.sqrt(dot(off, off)) * math.cos(a)
        shift = (-offset if inside else offset) / math.sin(a)
        s["f"] = dict(s["f"], o=add(f.o, f.z, shift))
    elif k == "torus":
        off = sub(refp, f.o)
        h = dot(f.z, off)
        rho = math.sqrt(max(dot(off, off) - h * h, 0.0))
        inside = math.hypot(rho - s["R"], h) < s["r"]
        s["r"] = s["r"] + offset if inside else s["r"] - offset
    return s


def trimmed(tool, s, keep, big):
    """The pieces of tool short of surface s: those `keep` accepts."""
    sp = BRepAlgoAPI_Splitter()
    args, tools = ShapeList(), ShapeList()
    args.Append(tool)
    tools.Append(surface_face(s, big))
    sp.SetArguments(args)
    sp.SetTools(tools)
    sp.Build()
    if not sp.IsDone():
        raise RuntimeError("splitting at the end surface failed")
    kept = [p for p in shapes(sp.Shape(), TopAbs_SOLID) if keep(p)]
    if not kept:
        raise RuntimeError("nothing short of the end surface")
    return kept[0] if len(kept) == 1 else compound(kept)


def on_plane(o, z):
    """Whether a solid has a planar face on the plane through o square to z."""
    def test(solid):
        for fc in shapes(solid, TopAbs_FACE):
            ad = BRepAdaptor_Surface(TopoDS.Face(fc))
            if ad.GetType() != GeomAbs_Plane:
                continue
            p = ad.Plane()
            n = p.Axis().Direction()
            loc = p.Location()
            if abs(abs(n.X() * z[0] + n.Y() * z[1] + n.Z() * z[2]) - 1) < 1e-9 and \
                    abs(dot([n.X(), n.Y(), n.Z()], sub([loc.X(), loc.Y(), loc.Z()], o))) < 1e-6:
                return True
        return False
    return test


def holding(p):
    def test(solid):
        c = BRepClass3d_SolidClassifier(solid, gp_Pnt(*p), 1e-7)
        return c.State() == TopAbs_IN
    return test


def square(s, z):
    return s["s"] == "plane" and abs(abs(dot(Frame(s["f"]).z, z)) - 1) <= 1e-9


def along(at_o, at_z, s, offset):
    """Where a plane square to the travel lies along it, `offset` further on."""
    f = Frame(s["f"])
    t = dot(at_z, sub(f.o, at_o))
    if abs(t) <= 1e-9:
        raise RuntimeError("the sketch already lies on the end plane")
    end = t + offset * math.copysign(1, t)
    if end * t <= 0:
        raise RuntimeError("an offset back past the sketch")
    return end


def interior(fc, pl, n=5):
    """A few points inside a planar profile face."""
    lo, hi = bbox(fc)
    from OCP.BRepClass import BRepClass_FaceClassifier
    out = []
    for i in range(1, 12):
        for j in range(1, 12):
            p = [lo[k] + (hi[k] - lo[k]) * (i / 12 if k == 0 else j / 12 if k == 1 else 0.5) for k in range(3)]
            # onto the sketch plane
            p = add(p, pl.z, -dot(sub(p, pl.o), pl.z))
            if BRepClass_FaceClassifier(fc, gp_Pnt(*p), 1e-7).State() == TopAbs_IN:
                out.append(p)
    step = max(1, len(out) // n)
    return out[::step][:n]


def surface_of(fc):
    """An OCCT face's surface in the exporter's terms."""
    ad = BRepAdaptor_Surface(fc)
    from OCP.GeomAbs import GeomAbs_Cone, GeomAbs_Cylinder, GeomAbs_Sphere, GeomAbs_Torus
    t = ad.GetType()

    def fr(ax):
        o, x, y, z = ax.Location(), ax.XDirection(), ax.YDirection(), ax.Direction()
        return {"o": [o.X(), o.Y(), o.Z()], "x": [x.X(), x.Y(), x.Z()], "y": [y.X(), y.Y(), y.Z()],
                "z": [z.X(), z.Y(), z.Z()]}
    if t == GeomAbs_Plane:
        return {"s": "plane", "f": fr(ad.Plane().Position())}
    if t == GeomAbs_Cylinder:
        return {"s": "cylinder", "f": fr(ad.Cylinder().Position()), "r": ad.Cylinder().Radius()}
    if t == GeomAbs_Sphere:
        return {"s": "sphere", "f": fr(ad.Sphere().Position()), "r": ad.Sphere().Radius()}
    if t == GeomAbs_Cone:
        c = ad.Cone()
        return {"s": "cone", "f": fr(c.Position()), "r": c.RefRadius(), "t": math.tan(c.SemiAngle())}
    if t == GeomAbs_Torus:
        c = ad.Torus()
        return {"s": "torus", "f": fr(c.Position()), "R": c.MajorRadius(), "r": c.MinorRadius()}
    raise RuntimeError("the next face is not an analytic surface")


def next_surface(body, pts, z):
    """The one face every ray from pts along z meets first: its surface."""
    if body is None:
        raise RuntimeError("no part to meet")
    hit = None
    for p in pts:
        si = IntCurvesFace_ShapeIntersector()
        si.Load(body, 1e-7)
        si.Perform(gp_Lin(gp_Pnt(*p), gp_Dir(*z)), 1e-7, 1e100)
        if not si.IsDone() or si.NbPnt() == 0:
            raise RuntimeError("a ray meets no face")
        k = min(range(1, si.NbPnt() + 1), key=si.WParameter)
        fc = si.Face(k)
        if hit is not None and not fc.IsSame(hit):
            raise RuntimeError("the rays meet different faces first")
        hit = fc
    return surface_of(hit)


def up_to(refp, fc, pl, z, s, offset, body):
    """The prism of fc along z from its sketch, cut off at s (moved `offset` on past it)."""
    s = offset_surface(s, refp, z, offset)
    big = 10 * (size(body) if body is not None else 0) + 10 * size(fc) + 4 * math.dist(Frame(s["f"]).o, pl.o) + 100
    tool = prism(fc, [c * big for c in z])
    return trimmed(tool, s, on_plane(pl.o, pl.z), big * 4)


def extrude(f, body):
    fc, pl = face(f)
    e = f["extent"]
    z = pl.z
    if "d" in e or "sym" in e or "two" in e or "through" in e:
        if "d" in e:
            lo, hi = min(0, e["d"]), max(0, e["d"])
        elif "sym" in e:
            lo, hi = -e["sym"] / 2, e["sym"] / 2
        elif "two" in e:
            lo, hi = -e["two"][1], e["two"][0]
        else:
            if body is None:
                raise RuntimeError("through, with no part")
            ts = [dot(z, sub(c, pl.o)) for c in corners(body)]
            lo, hi = {"forward": (0, max(ts) + 1), "back": (min(ts) - 1, 0),
                      "both": (min(ts) - 1, max(ts) + 1)}[e["through"]]
        if hi - lo <= 0:
            raise RuntimeError("an extent of nothing")
        if "along" in e:
            # lo and hi along the normal, the prism running along the direction
            t = dot(z, e["along"])
            z = [c / t for c in e["along"]]
        return prism(translated(fc, [c * lo for c in z]), [c * (hi - lo) for c in z])
    if "next" in e:
        d = [-c for c in z] if e["next"] else z
        s, offset = next_surface(body, interior(fc, pl), d), 0.0
    else:
        s, offset = e["to"], e["offset"]
        if s is None:
            raise RuntimeError("an end surface we could not name")
        refp = first_point(f, pl)
        d = z if toward(s, refp, z) > 0 else [-c for c in z]
    if square(s, z):
        end = along(pl.o, z, s, offset)
        if "next" in e and end * dot(d, z) <= 0:
            raise RuntimeError("the next face is behind")
        lo, hi = min(0, end), max(0, end)
        return prism(translated(fc, [c * lo for c in z]), [c * (hi - lo) for c in z])
    return up_to(first_point(f, pl), fc, pl, d, s, offset, body)


def first_point(f, pl):
    s = f["loops"][0][0]
    if "bezier" in s:
        return pl.pt(*s["bezier"]["p"][0][:2])
    if "line" in s:
        return pl.pt(*s["line"][:2])
    if "arc" in s:
        cx, cy, r, st, _ = s["arc"]
    else:
        (cx, cy, r, _), st = s["circle"], 0.0
    return pl.pt(cx + r * math.cos(st), cy + r * math.sin(st))


def revolve_frame(f, pl):
    """The axis frame a revolve turns in: z up the axis, x toward the profile (as ours)."""
    ox, oy, dx, dy = f["axis"]
    n = math.hypot(dx, dy)
    dx, dy = dx / n, dy / n
    px, py = -dy, dx
    side = 0.0
    for lp in f["loops"]:
        for s in lp:
            for q in seg_samples(s):
                r = (q[0] - ox) * px + (q[1] - oy) * py
                side = r if abs(r) > abs(side) else side
    sg = -1.0 if side < 0 else 1.0
    z = pl.vec(dx, dy)
    x = pl.vec(px * sg, py * sg)
    return pl.pt(ox, oy), z, x, cross(z, x)


def bezier_samples(b, per=32):
    d, pts = b["d"], b["p"]
    out = []
    for i in range(0, len(pts), d + 1):
        q = pts[i : i + d + 1]
        for k in range(per):
            u = k / per
            h = [(x * w, y * w, w) for x, y, w in q]
            for lev in range(d, 0, -1):
                h = [tuple(a[n] * (1 - u) + c[n] * u for n in range(3)) for a, c in zip(h[: lev], h[1 : lev + 1])]
            out.append((h[0][0] / h[0][2], h[0][1] / h[0][2]))
    return out


def seg_samples(s, n=32):
    if "bezier" in s:
        return bezier_samples(s["bezier"], n)
    if "line" in s:
        a, b, c, d = s["line"]
        return [(a + (c - a) * i / n, b + (d - b) * i / n) for i in range(n + 1)]
    if "arc" in s:
        cx, cy, r, st, sw = s["arc"]
    else:
        (cx, cy, r, _), st, sw = s["circle"], 0.0, TAU
    return [(cx + r * math.cos(st + sw * i / n), cy + r * math.sin(st + sw * i / n)) for i in range(n + 1)]


def revolve(f):
    fc, pl = face(f)
    o, z, x, y = revolve_frame(f, pl)
    t = f["turn"]
    if "a" in t:
        start, sweep = 0.0, t["a"]
    elif "sym" in t:
        start, sweep = -t["sym"] / 2, t["sym"]
    elif "two" in t:
        start, sweep = -t["two"][1], t["two"][0] + t["two"][1]
    else:
        p = Frame(t["to"]["f"]) if t["to"] else None
        if p is None or t["to"]["s"] != "plane":
            raise RuntimeError("a revolve ends only at a plane")
        if abs(dot(p.z, sub(o, p.o))) > 1e-9 or abs(dot(p.z, z)) > 1e-9:
            raise RuntimeError("the end plane does not hold the axis")
        u = cross(z, p.z)
        a = math.atan2(dot(y, u), dot(x, u)) % math.pi
        start, sweep = 0.0, a + t["offset"]
    if not (0 < sweep <= TAU + 1e-12):
        raise RuntimeError("a turn of nothing, or past a whole turn")
    if start:
        fc = rotated(fc, o, z, start)
    return BRepPrimAPI_MakeRevol(fc, gp_Ax1(gp_Pnt(*o), gp_Dir(*z)), min(sweep, TAU)).Shape()


def hole(f, body):
    """A drilled hole: its section (radius, depth down the drill) turned about the drill."""
    at = Frame(f["at"])
    r = f["d"] / 2
    head, end = f["head"], f["end"]
    long = False
    if "depth" in end:
        depth = end["depth"]
    elif "through" in end:
        if body is None:
            raise RuntimeError("through, with no part")
        depth = max(max(dot(at.z, sub(c, at.o)) for c in corners(body)), 0) + 1
    else:
        if "next" in end:
            pts = [at.pt(x, y) for x, y in ((0, 0), (r / 2, 0), (-r / 2, 0), (0, r / 2), (0, -r / 2))]
            s, offset = next_surface(body, pts, at.z), 0.0
        else:
            s, offset = end["to"], end["offset"]
            if s is None:
                raise RuntimeError("an end surface we could not name")
        if square(s, at.z):
            depth = along(at.o, at.z, s, offset)
            if depth < 0:
                raise RuntimeError("an end behind the mouth")
        else:
            long, depth = (s, offset), 10 * (size(body) + 10)
    pts = []
    mouth = r
    if head and "cbore" in head:
        D, d = head["cbore"]
        mouth = D / 2
        pts += [(D / 2, d), (r, d)]
    elif head and "csink" in head:
        D, a = head["csink"]
        mouth = D / 2
        pts.append((r, (D / 2 - r) / math.tan(a / 2)))
    pts.append((r, depth))
    # a drill point on a blind hole, its depth given or worked out (not one cut off at a surface)
    tip = depth + (r / math.tan(f["point"] / 2) if f["point"] and "through" not in end and not long else 0)
    pts.append((0, tip))
    pts = [(0, 0), (mouth, 0)] + pts
    mw = BRepBuilderAPI_MakeWire()
    for a, b in zip(pts, pts[1:] + pts[:1]):
        mw.Add(BRepBuilderAPI_MakeEdge(at.gpt(a[0], 0, a[1]), at.gpt(b[0], 0, b[1])).Edge())
    fc = planar_face([mw.Wire()])
    tool = BRepPrimAPI_MakeRevol(fc, gp_Ax1(gp_Pnt(*at.o), gp_Dir(*at.z)), TAU).Shape()
    if long:
        s, offset = long
        s = offset_surface(s, at.o, at.z, offset)
        tool = trimmed(tool, s, holding(at.pt(0, 0, min(1e-3, r / 4))), 4 * depth)
    return tool


def path_wire(path):
    mw = BRepBuilderAPI_MakeWire()
    for s in path:
        if "bezier3" in s:
            d, pts = s["bezier3"]["d"], s["bezier3"]["p"]
            for i in range(0, len(pts), d + 1):
                poles = Array1_gp_Pnt(1, d + 1)
                ws = Array1_double(1, d + 1)
                for k, (x, y, z, w) in enumerate(pts[i : i + d + 1]):
                    poles.SetValue(k + 1, gp_Pnt(x, y, z))
                    ws.SetValue(k + 1, w)
                mw.Add(BRepBuilderAPI_MakeEdge(Geom_BezierCurve(poles, ws)).Edge())
            continue
        if "line" in s:
            mw.Add(BRepBuilderAPI_MakeEdge(gp_Pnt(*s["line"][0]), gp_Pnt(*s["line"][1])).Edge())
            continue
        f = Frame(s["arc"])
        a, w = s["start"], s["sweep"]
        mw.Add(arc_edge([f.pt(s["r"] * math.cos(t), s["r"] * math.sin(t)) for t in (a, a + w / 2, a + w)]))
    return mw.Wire()


def path_ends(path):
    """The path's ends and directions there."""
    def at(s, end):
        if "bezier3" in s:
            q = s["bezier3"]["p"]
            a, b = (q[-2][:3], q[-1][:3]) if end else (q[0][:3], q[1][:3])
            return (b if end else a), norm(sub(b, a))
        if "line" in s:
            a, b = s["line"]
            return (b if end else a), norm(sub(b, a))
        f = Frame(s["arc"])
        t = s["start"] + (s["sweep"] if end else 0)
        p = f.pt(s["r"] * math.cos(t), s["r"] * math.sin(t))
        d = f.vec(-math.sin(t), math.cos(t))
        return p, [c * math.copysign(1, s["sweep"]) for c in d]
    return at(path[0], False), at(path[-1], True)


def sweep(f):
    fc, pl = face(f)
    path = list(f["path"])
    ends = f["ends"]
    (p0, t0), (p1, t1) = path_ends(path)
    if ends and "ext" in ends:
        s, e = ends["ext"]
        if s > 1e-9:
            path.insert(0, {"line": [add(p0, t0, -s), p0]})
            fc = translated(fc, [-c * s for c in t0])
        if e > 1e-9:
            path.append({"line": [p1, add(p1, t1, e)]})
    tool = BRepOffsetAPI_MakePipe(path_wire(path), fc).Shape()
    if ends and "to" in ends:
        s = ends["to"]
        if s is None or s["s"] != "plane":
            raise RuntimeError("a sweep ends only at a plane")
        p = Frame(s["f"])
        z = p.z if dot(p.z, sub(p.o, p0)) > 0 else [-c for c in p.z]
        s = dict(s, f=dict(s["f"], o=add(p.o, z, ends["offset"])))
        tool = trimmed(tool, s, holding(add(p0, t0, 1e-4)), 10 * size(tool))
    return tool


def loft(f):
    """ThruSections through the profiles' loops, loop by loop (holes lofted and cut out)."""
    if f.get("centre"):
        # sections along the centre line: OCCT's pipe shell through them
        ps = BRepOffsetAPI_MakePipeShell(path_wire(f["centre"]))
        for p in f["profiles"]:
            if len(p["loops"]) > 1:
                raise RuntimeError("a loft along a centre line with holes")
            ps.Add(wire(Frame(p["plane"]).pt, p["loops"][0]), False, False)
        ps.Build()
        if not ps.IsDone():
            raise RuntimeError("pipe shell failed")
        ps.MakeSolid()
        return ps.Shape()
    n = len(f["profiles"][0]["loops"])
    if any(len(p["loops"]) != n for p in f["profiles"]):
        raise RuntimeError("profiles with different loop counts")
    solids = []
    for k in range(n):
        ts = BRepOffsetAPI_ThruSections(True, f["ruled"])
        # matched segment by segment from each loop's start, as ours are
        ts.CheckCompatibility(False)
        for p in f["profiles"]:
            ts.AddWire(wire(Frame(p["plane"]).pt, p["loops"][k]))
        ts.Build()
        if not ts.IsDone():
            raise RuntimeError("ThruSections failed")
        solids.append(ts.Shape())
    out = solids[0]
    if volume(out) < 0:
        raise RuntimeError("a loft turned inside out")
    return out if n == 1 else boolean(out, solids[1:], "cut", Opts())


# analytic tool shapes (coils and threads)

def seg2_wire(segs, at):
    mw = BRepBuilderAPI_MakeWire()
    for s in segs:
        for e in bezier_edges(at, s["bezier"]) if "bezier" in s else [seg_edge(at, s)]:
            mw.Add(e)
    return mw.Wire()


def coil(s):
    """A profile (r, h) in the frame's xz wound about its z: point (r, h) at angle u lies at
    ((r + a u) cos u, (r + a u) sin u, h + b u). Lofted smooth through 24 sections a turn: OCCT's
    pipe shell along a helix builds the same solid, but its booleans with it went wrong (a
    thread's groove cut 336 mm^3 where Monte Carlo and these sections cut 231)."""
    f = Frame(s["coil"])
    a, b, angle = s["a"], s["b"], s["angle"]
    sec = seg2_wire(s["segs"], lambda r, h: f.pt(r, 0, h))
    n = max(2, math.ceil(angle / TAU * 24))
    ts = BRepOffsetAPI_ThruSections(True, False)
    ts.CheckCompatibility(False)
    for k in range(n + 1):
        u = angle * k / n
        t = gp_Trsf()
        t.SetRotation(gp_Ax1(gp_Pnt(*f.o), gp_Dir(*f.z)), u)
        m = gp_Trsf()
        m.SetTranslation(gp_Vec(*f.vec(a * u * math.cos(u), a * u * math.sin(u), b * u)))
        ts.AddWire(TopoDS.Wire(BRepBuilderAPI_Transform(sec, m.Multiplied(t), True).Shape()))
    ts.Build()
    if not ts.IsDone():
        raise RuntimeError("ThruSections failed")
    return ts.Shape()


def solid(s):
    if "prism" in s:
        f = Frame(s["prism"])
        fc = planar_face([seg2_wire(s["segs"], f.pt)])
        return prism(translated(fc, [c * s["lo"] for c in f.z]), [c * (s["hi"] - s["lo"]) for c in f.z])
    if "revolved" in s:
        f = Frame(s["revolved"])
        (ox, oy), (dx, dy), (px, py) = s["origin"], s["dir"], s["perp"]

        def at(u, v):
            q = (u - ox, v - oy)
            return f.pt(q[0] * px + q[1] * py, 0, q[0] * dx + q[1] * dy)
        fc = planar_face([seg2_wire(s["segs"], at)])
        return BRepPrimAPI_MakeRevol(fc, gp_Ax1(gp_Pnt(*f.o), gp_Dir(*f.z)), s["angle"] or TAU).Shape()
    if "ball" in s:
        return BRepPrimAPI_MakeSphere(gp_Pnt(*s["ball"]), s["r"]).Shape()
    if "csg" in s:
        body = solid(s["csg"])
        for r in s["rest"]:
            body = boolean(body, [solid(r["shape"])], r["op"], Opts())
        return body
    return coil(s)


# finishing ops and face edits

def round_edges(body, f):
    """A fillet: each of our edges as the OCCT edges through its points, the radius at each
    OCCT edge's ends read off our law along it."""
    mk = BRepFilletAPI_MakeFillet(body)
    edges = []
    for e in f["edges"]:
        law = e["law"]
        for oe in picked(body, TopAbs_EDGE, e["pts"]):
            if not any(oe.IsSame(x[0]) for x in edges):
                edges.append((oe, law))

    def radius(law, v):
        p = BRep_Tool.Pnt_s(v)
        q = [p.X(), p.Y(), p.Z()]
        best = min(range(len(law)), key=lambda k: math.dist(law[k][0], q))
        return law[best][1]
    for oe, law in edges:
        if mk.Contour(oe) == 0:
            mk.Add(law[0][1], oe)
    for oe, law in edges:
        ic = mk.Contour(oe)
        j = next((j for j in range(1, mk.NbEdges(ic) + 1) if mk.Edge(ic, j).IsSame(oe)), None)
        vs = [TopExp.FirstVertex_s(oe), TopExp.LastVertex_s(oe)]
        r = [radius(law, v) for v in vs]
        if r[0] == r[1] or j is None:
            # (an edge the contour holds only as its own copy takes a radius by itself)
            mk.SetRadius(r[0], ic, oe) if j is None else mk.SetRadius(r[0], ic, j)
            continue
        # the law runs along the chain: from the vertex this edge shares with the one before
        first = vs[0]
        if j == 1:
            first = mk.FirstVertex(ic)
        else:
            prev = mk.Edge(ic, j - 1)
            ends = [TopExp.FirstVertex_s(prev), TopExp.LastVertex_s(prev)]
            first = next((v for v in vs if any(v.IsSame(w) for w in ends)), vs[0])
        if not first.IsSame(vs[0]):
            r.reverse()
        mk.SetRadius(r[0], r[1], ic, j)
    mk.Build()
    if not mk.IsDone():
        raise RuntimeError("fillet failed")
    return mk.Shape()


def bevel(body, f):
    mk = BRepFilletAPI_MakeChamfer(body)
    fc = None
    if "face" in f:
        fs = picked(body, TopAbs_FACE, f["face"])
        fc = fs[0]
    for pts in f["edges"]:
        for e in picked(body, TopAbs_EDGE, pts):
            ref = fc
            if ref is not None and len(fs) > 1:
                # the piece of a split reference face this edge bounds
                ref = next((x for x in fs if any(e.IsSame(y) for y in shapes(x, TopAbs_EDGE))), fc)
            if "angle" in f:
                mk.AddDA(f["d"][0], f["angle"], e, ref)
            elif ref is not None:
                mk.Add(f["d"][0], f["d"][1], e, ref)
            else:
                mk.Add(f["d"][0], e)
    mk.Build()
    if not mk.IsDone():
        raise RuntimeError("chamfer failed")
    return mk.Shape()


def faces_of(body, groups):
    out = []
    for pts in groups:
        for fc in picked(body, TopAbs_FACE, pts):
            if not any(fc.IsSame(o) for o in out):
                out.append(fc)
    return out


def hollow(body, f):
    off = f["t"] if f["out"] else -f["t"]
    if not f["open"]:
        # a closed shell: the skin offset, the solid between
        mk = BRepOffsetAPI_MakeOffsetShape()
        mk.PerformByJoin(body, off, 1e-7, BRepOffset_Skin, False, False, GeomAbs_Intersection)
        sh = shapes(mk.Shape(), TopAbs_SHELL)
        other = BRepBuilderAPI_MakeSolid(TopoDS.Shell(sh[0])).Solid()
        if volume(other) < 0:
            other.Reverse()
        return boolean(other, [body], "cut", Opts()) if f["out"] else boolean(body, [other], "cut", Opts())
    faces = ShapeList()
    for fc in faces_of(body, f["open"]):
        faces.Append(fc)
    mk = BRepOffsetAPI_MakeThickSolid()
    mk.MakeThickSolidByJoin(body, faces, off, 1e-7, BRepOffset_Skin, False, False, GeomAbs_Intersection)
    mk.Build()
    if not mk.IsDone():
        raise RuntimeError("shell failed")
    return mk.Shape()


def taper(body, f):
    n = Frame(f["neutral"])
    mk = BRepOffsetAPI_DraftAngle(body)
    for fc in faces_of(body, f["faces"]):
        mk.Add(fc, gp_Dir(*n.z), f["angle"], gp_Pln(n.ax3()))
        if not mk.AddDone():
            raise RuntimeError("draft refused a face")
    mk.Build()
    if not mk.IsDone():
        raise RuntimeError("draft failed")
    return mk.Shape()


def offset_faces(body, f):
    mk = BRepOffset_MakeOffset()
    mk.Initialize(body, 0.0, 1e-7, BRepOffset_Skin, True, False, GeomAbs_Intersection)
    for fc in faces_of(body, f["faces"]):
        mk.SetOffsetOnFace(fc, f["d"])
    mk.MakeOffsetShape()
    if not mk.IsDone():
        raise RuntimeError("offset failed")
    return mk.Shape()


def thicken(body, f, opts):
    fs = faces_of(body, f["faces"])
    sew = BRepBuilderAPI_Sewing(1e-6)
    for fc in fs:
        sew.Add(fc)
    sew.Perform()
    skin = sew.SewedShape()
    t = f["t"]
    lo, hi = {"both": (-t / 2, t / 2), "pos": (0, t), "neg": (-t, 0)}[f["side"]]
    if lo != 0:
        mo = BRepOffsetAPI_MakeOffsetShape()
        mo.PerformBySimple(skin, lo)
        skin = mo.Shape()
    mk = BRepOffsetAPI_MakeThickSolid()
    mk.MakeThickSolidBySimple(skin, hi - lo)
    mk.Build()
    if not mk.IsDone():
        raise RuntimeError("thicken failed")
    slab = mk.Shape()
    if volume(slab) < 0:
        slab.Reverse()
    if f["op"] == "new":
        return slab
    return boolean(body, [slab], f["op"], opts)


def delete(body, f):
    d = BRepAlgoAPI_Defeaturing()
    d.SetShape(body)
    fs = ShapeList()
    for fc in faces_of(body, f["faces"]):
        fs.Append(fc)
    d.AddFacesToRemove(fs)
    d.SetRunParallel(False)
    d.Build()
    if not d.IsDone():
        raise RuntimeError("defeaturing failed")
    return d.Shape()


def rib(body, f, opts):
    """A wall from an open path grown until it meets the part (the kernel's construction, in
    OCCT's booleans): an extrusion far past the part, the part cut from it, the piece holding
    the path added."""
    pl = Frame(f["plane"])
    t, path = f["t"], f["path"]

    def tangent(s, end):
        if "line" in s:
            a, b, c, d = s["line"]
            n = math.hypot(c - a, d - b)
            return ((c - a) / n, (d - b) / n)
        _, _, _, st, sw = s["arc"]
        u = st + sw if end else st
        return (-math.sin(u) * math.copysign(1, sw), math.cos(u) * math.copysign(1, sw))

    def ends(s):
        q = seg_samples(s, 2)
        return q[0], q[-1]

    def left(v):
        return (-v[1], v[0])

    def offset_path(o):
        out = []
        for s in path:
            if "line" in s:
                m = left(tangent(s, False))
                a, b, c, d = s["line"]
                n = {"line": [a + m[0] * o, b + m[1] * o, c + m[0] * o, d + m[1] * o]}
            else:
                cx, cy, r, st, sw = s["arc"]
                n = {"arc": [cx, cy, r - o * math.copysign(1, sw), st, sw]}
            if out and math.dist(ends(out[-1])[1], ends(n)[0]) > 1e-9:
                pa, pb = ends(out[-1])
                na, nb = ends(n)
                d1, d2 = (pb[0] - pa[0], pb[1] - pa[1]), (nb[0] - na[0], nb[1] - na[1])
                den = d1[0] * d2[1] - d1[1] * d2[0]
                k = ((na[0] - pa[0]) * d2[1] - (na[1] - pa[1]) * d2[0]) / den
                p = (pa[0] + d1[0] * k, pa[1] + d1[1] * k)
                out[-1] = {"line": [pa[0], pa[1], p[0], p[1]]}
                out.append({"line": [p[0], p[1], nb[0], nb[1]]})
                continue
            out.append(n)
        return out

    def extended(p, far):
        out = list(p)
        (sa, _), (_, eb) = ends(p[0]), ends(p[-1])
        t0, t1 = tangent(p[0], False), tangent(p[-1], True)
        out.insert(0, {"line": [sa[0] - t0[0] * far, sa[1] - t0[1] * far, sa[0], sa[1]]})
        out.append({"line": [eb[0], eb[1], eb[0] + t1[0] * far, eb[1] + t1[1] * far]})
        return out

    def closed(segs):
        a, b = ends(segs[-1])[1], ends(segs[0])[0]
        if math.dist(a, b) > 1e-9:
            segs = segs + [{"line": [a[0], a[1], b[0], b[1]]}]
        return segs
    lo3, hi3 = bbox(body)
    pts = [lo3, hi3] + [pl.pt(*q) for s in path for q in seg_samples(s, 8)]
    blo = [min(p[k] for p in pts) for k in range(3)]
    bhi = [max(p[k] for p in pts) for k in range(3)]
    diag = math.dist(blo, bhi)
    margin = t + diag * math.tan(f["draft"]) + 1.0
    far = 2 * (diag + margin) + 10
    lo, hi = {"both": (-t / 2, t / 2), "pos": (0, t), "neg": (-t, 0)}[f["side"]]
    sign = -1.0 if f["flip"] else 1.0
    mid_seg = path[len(path) // 2]
    mid = seg_samples(mid_seg, 2)[1]
    ml = left(tangent(mid_seg, False))
    if f["web"]:
        a, b = offset_path(hi), offset_path(lo)
        segs = extended(a, far)
        back = extended(b, far)
        cap, other = ends(segs[-1])[1], ends(back[-1])[1]
        segs.append({"line": [cap[0], cap[1], other[0], other[1]]})
        for s in reversed(back):
            if "line" in s:
                a0, b0, c0, d0 = s["line"]
                segs.append({"line": [c0, d0, a0, b0]})
            else:
                cx, cy, r, st, sw = s["arc"]
                segs.append({"arc": [cx, cy, r, st + sw, -sw]})
        fc = planar_face([wire(pl.pt, closed(segs))])
        tool = prism(fc, [c * -sign * far for c in pl.z])
        walls = [x for x in shapes(tool, TopAbs_FACE)
                 if abs(abs(dot(surface_normal(x), pl.z)) - 1) > 1e-9]
        if f["draft"]:
            nz = pl.z if not f["flip"] else [-c for c in pl.z]
            # the cap across the far ends is no wall
            walls = [x for x in walls if not touches(x, pl.pt(cap[0], cap[1]), pl.pt(other[0], other[1]))]
            tool = draft_faces(tool, walls, nz, pl.o, f["draft"])
        probe = add(pl.pt(mid[0] + ml[0] * (lo + hi) / 2, mid[1] + ml[1] * (lo + hi) / 2), pl.z, -sign * 1e-3)
    else:
        m = lambda s, e: tuple(c * sign * far for c in left(tangent(s, e)))  # noqa: E731
        segs = extended(path, far)
        s0, e1 = ends(segs[0])[0], ends(segs[-1])[1]
        m0, m1 = m(path[0], False), m(path[-1], True)
        segs.append({"line": [e1[0], e1[1], e1[0] + m1[0], e1[1] + m1[1]]})
        segs.append({"line": [e1[0] + m1[0], e1[1] + m1[1], s0[0] + m0[0], s0[1] + m0[1]]})
        base = Frame(dict(f["plane"], o=add(pl.o, pl.z, lo)))
        fc = planar_face([wire(base.pt, closed(segs))])
        tool = prism(fc, [c * (hi - lo) for c in pl.z])
        fill = [c * sign for c in pl.vec(ml[0], ml[1])]
        if f["draft"]:
            if len(path) != 1 or "line" not in path[0]:
                raise RuntimeError("a drafted rib across the plane needs a straight path")
            a, b, c, d = path[0]["line"]
            caps = [x for x in shapes(tool, TopAbs_FACE) if abs(abs(dot(surface_normal(x), pl.z)) - 1) < 1e-9]
            tool = draft_faces(tool, caps, [-x for x in fill], pl.pt(a, b), f["draft"])
        probe = add(add(pl.pt(*mid), fill, 1e-3), pl.z, (lo + hi) / 2)
    rest = boolean(tool, [body], "cut", opts)
    lump = next((s for s in shapes(rest, TopAbs_SOLID) if holding(probe)(s)), None)
    if lump is None:
        raise RuntimeError("the path lies inside the part")
    llo, lhi = bbox(lump)
    if any(llo[k] < blo[k] - margin or lhi[k] > bhi[k] + margin for k in range(3)):
        raise RuntimeError("the wall runs off the part")
    return boolean(body, [lump], "add", opts)


def surface_normal(fc):
    ad = BRepAdaptor_Surface(TopoDS.Face(fc))
    if ad.GetType() != GeomAbs_Plane:
        return [0, 0, 0]
    d = ad.Plane().Axis().Direction()
    return [d.X(), d.Y(), d.Z()]


def touches(fc, *pts):
    v = [BRepBuilderAPI_MakeVertex(gp_Pnt(*p)).Vertex() for p in pts]
    return all(BRepExtrema_DistShapeShape(x, fc).Value() < 1e-6 for x in v)


def draft_faces(body, faces, pull, at, angle):
    mk = BRepOffsetAPI_DraftAngle(body)
    for fc in faces:
        mk.Add(TopoDS.Face(fc), gp_Dir(*pull), angle, gp_Pln(gp_Pnt(*at), gp_Dir(*pull)))
        if not mk.AddDone():
            raise RuntimeError("draft refused a face")
    mk.Build()
    if not mk.IsDone():
        raise RuntimeError("draft failed")
    return mk.Shape()


def text(f):
    """Glyph regions (outlines as our layout sets them) raised from lo to hi: on a plane by
    prisms, round a cylinder as faces on it thickened radially (each point's radial line)."""
    out = []
    flat = "plane" in f
    fr = Frame(f["plane"] if flat else f["axis"])
    lo, hi = f["lo"], f["hi"]
    surf = None if flat else Geom_CylindricalSurface(fr.ax3(), lo)
    for region in f["regions"]:
        wires = []
        for lp in region:
            mw = BRepBuilderAPI_MakeWire()
            for s in lp:
                if flat:
                    if len(s) == 2:
                        e = BRepBuilderAPI_MakeEdge(fr.gpt(*s[0]), fr.gpt(*s[1])).Edge()
                    else:
                        poles = Array1_gp_Pnt(1, 3)
                        for k, q in enumerate(s):
                            poles.SetValue(k + 1, fr.gpt(*q))
                        e = BRepBuilderAPI_MakeEdge(Geom_BezierCurve(poles)).Edge()
                else:
                    # (x, y) -> (u = x / r, v = y) on the cylinder
                    uv = [gp_Pnt2d(q[0] / f["r"], q[1]) for q in s]
                    if len(s) == 2:
                        d = uv[0].Distance(uv[1])
                        line = Geom2d_Line(uv[0], gp_Dir2d(uv[1].X() - uv[0].X(), uv[1].Y() - uv[0].Y()))
                        e = BRepBuilderAPI_MakeEdge(line, surf, 0.0, d).Edge()
                    else:
                        poles = Array1_gp_Pnt2d(1, 3)
                        for k, q in enumerate(uv):
                            poles.SetValue(k + 1, q)
                        e = BRepBuilderAPI_MakeEdge(Geom2d_BezierCurve(poles), surf).Edge()
                    BRepLib.BuildCurves3d_s(e)
                mw.Add(e)
            if not mw.IsDone():
                raise RuntimeError("a glyph contour that does not close")
            wires.append(mw.Wire())
        if flat:
            fc = planar_face(wires)
            out.append(prism(translated(fc, [c * lo for c in fr.z]), [c * (hi - lo) for c in fr.z]))
            continue
        mf = BRepBuilderAPI_MakeFace(surf, wires[0], True)
        for w in wires[1:]:
            mf.Add(w)
        fix = ShapeFix_Face(mf.Face())
        fix.FixOrientation()
        fix.Perform()
        fc = fix.Face()
        BRepLib.BuildCurves3d_s(fc)
        mk = BRepOffsetAPI_MakeThickSolid()
        mk.MakeThickSolidBySimple(fc, hi - lo)
        mk.Build()
        sol = mk.Shape()
        if abs(volume(sol)) < 1e-12:
            raise RuntimeError("a wrapped glyph of no volume")
        if volume(sol) < 0:
            sol.Reverse()
        out.append(sol)
    return out


class Opts:
    def __init__(self, parallel=False, fuzzy=0.0):
        self.parallel, self.fuzzy = parallel, fuzzy


def near(shape, p, tol=1e-4):
    v = BRepBuilderAPI_MakeVertex(gp_Pnt(*p)).Vertex()
    d = BRepExtrema_DistShapeShape(v, shape)
    return d.IsDone() and d.Value() <= tol


# what each finishing op or face edit names: its groups of points, by field
NAMED = {"fillet": "edges", "chamfer": "edges", "shell": "open", "draft": "faces", "offset": "faces",
         "thicken": "faces", "delete": "faces"}


def group_point(group):
    return group["pts"][0] if isinstance(group, dict) else group[0]


def restrict(f, shape):
    """The op cut down to the edges and faces this body holds, or None if it holds none."""
    k = NAMED.get(f["kind"])
    if k is None or not f[k]:
        return f
    mine = [g for g in f[k] if near(shape, group_point(g))]
    return dict(f, **{k: mine}) if mine else None


def split_by(shape, s, keep, opts):
    """The body's pieces either side of a surface: (minus, plus), plus where its normal points
    (outside a cylinder or sphere)."""
    big = 4 * size(shape) + 10
    if s is None:
        raise RuntimeError("a cutter we could not name")
    if s["s"] == "plane":
        f = Frame(s["f"])
        from OCP.BRepPrimAPI import BRepPrimAPI_MakeHalfSpace
        half = BRepPrimAPI_MakeHalfSpace(surface_face(s, big), gp_Pnt(*add(f.o, f.z, 1.0))).Solid()
        inside_is_plus = True
    elif s["s"] == "cylinder":
        f = Frame(s["f"])
        from OCP.BRepPrimAPI import BRepPrimAPI_MakeCylinder
        base = add(f.o, f.z, -big)
        half = BRepPrimAPI_MakeCylinder(gp_Ax2(gp_Pnt(*base), gp_Dir(*f.z), gp_Dir(*f.x)), s["r"], 2 * big).Shape()
        inside_is_plus = False
    elif s["s"] == "sphere":
        half = BRepPrimAPI_MakeSphere(gp_Pnt(*Frame(s["f"]).o), s["r"]).Shape()
        inside_is_plus = False
    else:
        raise RuntimeError("splitting by a cone, torus or blend")
    inside, outside = (lambda: boolean(shape, [half], "keep", opts)), (lambda: boolean(shape, [half], "cut", opts))
    plus, minus = (inside, outside) if inside_is_plus else (outside, inside)
    return (minus() if keep != "plus" else None), (plus() if keep != "minus" else None)


def build(features, opts, trace=None):
    """The part the features make (its bodies as one compound when there are several);
    `trace(feature, part)` is called after each."""
    bodies = []  # [id, shape]: each body, by the feature that made it, in the order made
    made = {}  # each solid feature's tools [(solid, op)], for patterns to copy

    def find(id):
        k = next((k for k, b in enumerate(bodies) if b[0] == id), None)
        if k is None:
            raise RuntimeError(f"no body made by feature {id}")
        return k

    def picked_bodies(f):
        """The bodies a feature is on: its `in`, else (an op naming edges or faces) those holding
        them, else the current one."""
        if "in" in f:
            return list(range(len(bodies))) if f["in"] == "all" else [find(id) for id in f["in"]]
        if not bodies:
            raise RuntimeError("nothing to change yet")
        if NAMED.get(f["kind"]) and f[NAMED[f["kind"]]]:
            on = [k for k, b in enumerate(bodies) if restrict(f, b[1]) is not None]
            if on:
                return on
        return [len(bodies) - 1]

    def current():
        return bodies[-1][1] if bodies else None

    def part():
        return bodies[0][1] if len(bodies) == 1 else compound([b[1] for b in bodies]) if bodies else None

    for f in features:
        try:
            k = f["kind"]
            tools = None
            if k == "extrude":
                tools = [(extrude(f, current()), f["op"])]
            elif k == "revolve":
                tools = [(revolve(f), f["op"])]
            elif k == "ball":
                tools = [(BRepPrimAPI_MakeSphere(gp_Pnt(*f["center"]), f["radius"]).Shape(), f["op"])]
            elif k == "hole":
                tools = [(hole(f, current()), "cut")]
            elif k == "sweep":
                tools = [(sweep(f), f["op"])]
            elif k == "loft":
                tools = [(loft(f), f["op"])]
            elif k == "shapes":
                tools = [(solid(t["shape"]), t["op"]) for t in f["tools"]]
            elif k == "text":
                tools = [(t, f["op"]) for t in text(f)]
            elif k == "pattern":
                base = [t for s in f["src"] for t in made.get(s, [])]
                tools = [(moved(t, m), "add" if op == "new" else op) for m in f["m"] for t, op in base]
                made[f["i"]] = base + tools
            elif k == "combine":
                t = find(f["target"])
                body = bodies[t][1]
                for id in f["tools"]:
                    body = boolean(body, [bodies[find(id)][1]], f["op"], opts)
                bodies[t][1] = body
                if not f["keep"]:
                    bodies[:] = [b for b in bodies if b[0] not in f["tools"]]
            elif k == "split":
                on = picked_bodies(f)
                if f["keep"] == "both" and len(on) != 1:
                    raise RuntimeError("splitting more than one body in two")
                for t in on:
                    minus, plus = split_by(bodies[t][1], f["by"], f["keep"], opts)
                    bodies[t][1] = minus if minus is not None else plus
                    if f["keep"] == "both":
                        bodies.append([f["i"], plus])
            elif k == "move":
                on = picked_bodies(f)
                if f["copy"]:
                    if len(on) != 1:
                        raise RuntimeError("copying more than one body")
                    bodies.append([f["i"], moved(bodies[on[0]][1], f["m"])])
                else:
                    for t in on:
                        bodies[t][1] = moved(bodies[t][1], f["m"])
            else:
                new = []
                for t in picked_bodies(f):
                    body, g = bodies[t][1], restrict(f, bodies[t][1])
                    if k == "pattern_body":
                        out = boolean(body, [moved(body, m) for m in f["m"]], "add", opts)
                    elif k == "fillet":
                        out = round_edges(body, g)
                    elif k == "chamfer":
                        out = bevel(body, g)
                    elif k == "shell":
                        out = hollow(body, g)
                    elif k == "draft":
                        out = taper(body, g)
                    elif k == "rib":
                        out = rib(body, f, opts)
                    elif k == "offset":
                        out = offset_faces(body, g)
                    elif k == "thicken":
                        out = thicken(body, g, opts)
                    elif k == "delete":
                        out = delete(body, g)
                    else:
                        raise RuntimeError(f"no OCCT replay for {k}")
                    if k == "thicken" and f["op"] == "new":
                        # a slab thickened as a new body
                        new.append([f["i"], out])
                    else:
                        bodies[t][1] = out
                bodies.extend(new)
            if tools is not None:
                made.setdefault(f["i"], tools)
                if tools[0][1] == "new":
                    bodies.append([f["i"], tools[0][0]])
                else:
                    for t in picked_bodies(f):
                        bodies[t][1] = apply(bodies[t][1], tools, opts)
        except Exception as e:
            raise RuntimeError(f"feature {f['i']} ({f['kind']}): {e}") from e
        if trace:
            trace(f, part())
    return part()
