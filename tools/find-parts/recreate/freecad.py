#!/usr/bin/env python3
"""FreeCAD library PartDesign bodies to .fg (Forge). A file is read as data (Document.xml as XML, shapes by OpenCascade
in fcstd_truth.py); FreeCAD is never run and macros or scripts in the file are never touched. Each object stores
its shape, so a fillet's EdgeN is sampled from its base's stored shape and found on our part by position, and
every step has a truth. Keep the data outside the repo; run with python3 -I.

  freecad.py FILE.FCStd --topo EXE --truth VENVPY --out DIR      one .fg and result JSON line per body

Each feature is a statement (fgw.St) written as Forge by fgw.Prog; a blend's edges, found on the part by
position, are named by lineage (`edges(of: F, ..)`, `edges(between: (A, B))`, anchors, pattern copies) by
forge-proof's topo, as interface/forge-import names them. interface/forge-import (`forgecad import`) is
this reader in Rust.
"""
import io
import json
import math
import os
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sketch2d
from cadfs import KernelStop, Unresolved, Unsupported, WPiece, add, carries_on, chain_pieces, dot, fine, num, path_from, tangent_fit, unit
from fgw import NoName, Prog, St, Stop, Topo, loop_segs
from sketch2d import Arrangement

TOL = 2e-3
HERE = os.path.dirname(os.path.abspath(__file__))
SOLID = {"PartDesign::Pad", "PartDesign::Pocket", "PartDesign::Revolution", "PartDesign::Groove", "PartDesign::Fillet", "PartDesign::Chamfer", "PartDesign::Mirrored", "PartDesign::PolarPattern", "PartDesign::LinearPattern", "PartDesign::Hole", "PartDesign::Loft", "PartDesign::Pipe", "PartDesign::AdditiveLoft", "PartDesign::AdditivePipe", "PartDesign::SubtractiveLoft", "PartDesign::SubtractivePipe", "PartDesign::Draft", "PartDesign::Thickness", "PartDesign::MultiTransform", "PartDesign::Scaled", "PartDesign::AdditiveBox", "PartDesign::AdditiveCylinder", "PartDesign::Boolean", "PartDesign::Helix", "PartDesign::AdditiveHelix", "PartDesign::SubtractiveHelix", "PartDesign::FeatureBase", "PartDesign::SubtractiveBox", "PartDesign::SubtractiveCylinder", "PartDesign::AdditiveSphere", "PartDesign::SubtractiveSphere", "PartDesign::AdditiveCone", "PartDesign::SubtractiveCone", "PartDesign::AdditiveTorus", "PartDesign::SubtractiveTorus", "PartDesign::AdditivePrism", "PartDesign::SubtractivePrism", "PartDesign::AdditiveWedge", "PartDesign::SubtractiveWedge", "PartDesign::AdditiveEllipsoid", "PartDesign::SubtractiveEllipsoid", "Part::Revolution", "Part::Extrusion", "Part::Box", "Part::Cylinder", "Part::Cut", "Part::Fuse", "Part::Common", "Part::MultiFuse", "Part::MultiCommon", "Part::Sphere"}
TRANSFORMS = {"PartDesign::PolarPattern", "PartDesign::LinearPattern", "PartDesign::Mirrored"}
FLAT = {"Sketcher::SketchObject", "Part::Part2DObjectPython"}  # what a pad's profile can be; only a sketch has rows, the rest a stored shape
NEEDS = {"PartDesign::Thickness", "PartDesign::Draft", "PartDesign::AdditivePipe", "PartDesign::SubtractivePipe", "PartDesign::FeatureBase", "Part::Cut", "Part::Fuse", "Part::Common", "Part::MultiFuse", "Part::MultiCommon", "Part::Sphere", "PartDesign::Hole", "PartDesign::MultiTransform", "PartDesign::AdditiveBox", "PartDesign::SubtractiveBox", "PartDesign::AdditiveCylinder", "PartDesign::SubtractiveCylinder", "Part::Revolution", "Part::Extrusion", "Part::Box", "Part::Cylinder", "PartDesign::Pad", "PartDesign::Pocket", "PartDesign::Revolution", "PartDesign::Groove", "PartDesign::Fillet", "PartDesign::Chamfer", "PartDesign::Mirrored", "PartDesign::PolarPattern", "PartDesign::LinearPattern"}


class Doc:
    def __init__(self, data):
        self.z = zipfile.ZipFile(io.BytesIO(data))
        self.volumes = None
        self.bodied = None
        info = self.z.getinfo("Document.xml")
        if info.file_size > 64 << 20:
            raise Unsupported("a huge document")
        root = ET.fromstring(self.z.read("Document.xml"))
        self.types = {o.get("name"): o.get("type") for o in root.find("Objects").findall("Object")}
        self.props = {}
        for o in root.find("ObjectData").findall("Object"):
            self.props[o.get("name")] = {p.get("name"): p for p in o.find("Properties").findall("Property")}

    def prop(self, obj, name):
        return self.props[obj].get(name)

    def flt(self, obj, name, d=0.0):
        p = self.prop(obj, name)
        if p is None:
            return d
        e = p.find("Float")
        return float(e.get("value")) if e is not None else d

    def boolean(self, obj, name):
        p = self.prop(obj, name)
        return p is not None and p.find("Bool") is not None and p.find("Bool").get("value") == "true"

    def integer(self, obj, name, d=0):
        p = self.prop(obj, name)
        return int(p.find("Integer").get("value")) if p is not None and p.find("Integer") is not None else d

    def link(self, obj, name):
        p = self.prop(obj, name)
        if p is None and name == "Profile":
            p = self.prop(obj, "Sketch")  # older files
        if p is None:
            return ""
        e = p.find("Link")
        if e is not None:
            return e.get("value", "")
        e = p.find("LinkSub")
        return e.get("value", "") if e is not None else ""

    def subs(self, obj, name):
        p = self.prop(obj, name)
        e = p.find("LinkSub") if p is not None else None
        return [s.get("value") for s in e.findall("Sub")] if e is not None else []

    def links(self, obj, name):
        p = self.prop(obj, name)
        e = p.find("LinkList") if p is not None else None
        return [s.get("value") for s in e.findall("Link")] if e is not None else []

    def placement(self, obj):
        p = self.prop(obj, "Placement")
        e = p.find("PropertyPlacement") if p is not None else None
        if e is None:
            return (0.0, 0.0, 0.0), (0.0, 0.0, 0.0, 1.0)
        return tuple(float(e.get(k)) for k in ("Px", "Py", "Pz")), tuple(float(e.get(k)) for k in ("Q0", "Q1", "Q2", "Q3"))

    def shape_file(self, obj):
        p = self.prop(obj, "Shape")
        e = p.find("Part") if p is not None else None
        return e.get("file") if e is not None else None


def vt(v):
    """A vector's numbers as the importer writes them."""
    return tuple(num(x) for x in v)


def rotate(q, v):
    x, y, z, w = q
    t = (2 * (y * v[2] - z * v[1]), 2 * (z * v[0] - x * v[2]), 2 * (x * v[1] - y * v[0]))
    return (v[0] + w * t[0] + y * t[2] - z * t[1], v[1] + w * t[1] + z * t[0] - x * t[2], v[2] + w * t[2] + x * t[1] - y * t[0])


class Frame:
    def __init__(self, pl):
        self.o, q = pl
        self.x, self.y, self.n = rotate(q, (1.0, 0.0, 0.0)), rotate(q, (0.0, 1.0, 0.0)), rotate(q, (0.0, 0.0, 1.0))

    def uvw(self, p):
        d = add(p, self.o, -1)
        return dot(d, self.x), dot(d, self.y), dot(d, self.n)

    def world(self, u, v, w=0.0):
        return add(add(add(self.o, self.x, u), self.y, v), self.n, w)


def construction(g):
    c = g.find("Construction")
    if c is not None and c.get("value") == "1":
        return True
    c = g.find("Construct")
    if c is not None and c.get("value") == "1":
        return True
    ext = g.find("GeoExtensions")
    if ext is not None:
        for x in ext:
            f = x.get("geometryModeFlags")
            if f and len(f) > 1 and f[-2] == "1":
                return True
    return False


def sketch_ents(doc, name):
    p = doc.prop(name, "Geometry")
    if p is None:
        raise Unsupported("a sketch without geometry")
    if doc.prop(name, "ExternalGeometry") is not None and False:
        pass
    ents = []
    for k, g in enumerate(p.find("GeometryList").findall("Geometry")):
        t = g.get("type")
        con = construction(g)
        if t == "Part::GeomLineSegment":
            e = g.find("LineSegment")
            ents.append({"kind": "skLineSegment", "id": f"e{k}", "start": [float(e.get("StartX")), float(e.get("StartY"))], "end": [float(e.get("EndX")), float(e.get("EndY"))], "construction": con})
        elif t == "Part::GeomCircle":
            e = g.find("Circle")
            ents.append({"kind": "skCircle", "id": f"e{k}", "center": [float(e.get("CenterX")), float(e.get("CenterY"))], "radius": float(e.get("Radius")), "construction": con})
        elif t == "Part::GeomArcOfCircle":
            e = g.find("ArcOfCircle")
            c, r = (float(e.get("CenterX")), float(e.get("CenterY"))), float(e.get("Radius"))
            nz = float(e.get("NormalZ", "1"))
            xu = float(e.get("AngleXU", "0"))
            a0, a1 = float(e.get("StartAngle")), float(e.get("EndAngle"))
            while a1 < a0:
                a1 += 2 * math.pi
            pt = lambda a: [c[0] + r * math.cos(xu + nz * a), c[1] + r * math.sin(xu + nz * a)]
            ents.append({"kind": "skArc", "id": f"e{k}", "start": pt(a0), "mid": pt((a0 + a1) / 2), "end": pt(a1), "construction": con})
        elif t in ("Part::GeomEllipse", "Part::GeomArcOfEllipse"):
            # the major axis at AngleXU, the minor axis the normal crossed with it: a quarter turn the sketch's way
            # (sense 1) or back where the normal is -z; an arc's angles are the ellipse's parameters
            arc = t == "Part::GeomArcOfEllipse"
            e = g.find("ArcOfEllipse" if arc else "Ellipse")
            xu = float(e.get("AngleXU", "0"))
            d = {"id": f"e{k}", "center": [float(e.get("CenterX")), float(e.get("CenterY"))], "majorAxis": [math.cos(xu), math.sin(xu)],
                 "majorRadius": float(e.get("MajorRadius")), "minorRadius": float(e.get("MinorRadius")), "sense": 1 if float(e.get("NormalZ", "1")) > 0 else -1, "construction": con}
            if arc:
                a0, a1 = float(e.get("StartAngle")), float(e.get("EndAngle"))
                while a1 <= a0:
                    a1 += 2 * math.pi
                d.update(kind="skEllipseArc", t0=a0, t1=a1)
            else:
                d.update(kind="skEllipse")
            if not (d["majorRadius"] > 0 and d["minorRadius"] > 0 and d.get("t1", 0) - d.get("t0", 0) <= 4 * math.pi):
                raise Unsupported("an ellipse of no size")
            ents.append(d)
        elif t == "Part::GeomBSplineCurve":
            e = g.find("BSplineCurve")
            nb = bspline_exact(e)
            if nb is None:
                if e.get("IsPeriodic") == "1":
                    raise Unsupported("a periodic spline")
                ents.append({"kind": "polyline", "id": f"e{k}", "points": bspline_points(e), "construction": con})
            else:
                ents.append({"kind": "nurbs", "id": f"e{k}", "points": bspline_points(e, nb), "nurbs": nb, "construction": con})
        elif t == "Part::GeomPoint":
            continue
        elif not con:
            raise Unsupported("sketch curve " + t.split("::")[-1])
    return ents


def bspline_exact(e):
    """A NURBS as (degree, poles, full knot vector, weights), the form Forge's `spline(.., degree:, knots:,
    weights:)` takes. A clamped one as it is; an unclamped one with its own knots (the curve runs from knot
    `degree` to knot `len(poles)`); a periodic one as its poles wrapped, the first `degree` again after the last,
    with one period's knots extended periodically to the whole vector. None where the data is not a B-spline."""
    deg = int(e.get("Degree"))
    cp = [(float(p.get("X")), float(p.get("Y"))) for p in e.findall("Pole")]
    ws = [float(p.get("Weight", "1")) for p in e.findall("Pole")]
    ks = [(float(x.get("Value")), int(x.get("Mult"))) for x in e.findall("Knot")]
    if deg < 1 or len(ks) < 2 or any(w <= 0 for w in ws) or any(m < 1 for _, m in ks):
        return None
    if any(b <= a for (a, _), (b, _) in zip(ks, ks[1:])) or any(m > deg for _, m in ks[1:-1]):
        return None
    if e.get("IsPeriodic") == "1":
        # one period's flat knots: the last knot is the first one period on, so its count is the first's
        one = [v for v, m in ks[:-1] for _ in range(m)]
        n, per = len(cp), ks[-1][0] - ks[0][0]
        if ks[-1][1] != ks[0][1] or len(one) != n or n < deg + 1 or ks[0][1] > deg:
            return None
        # the i-th knot: one[] run on by periods both ways, placed so that the curve's domain, knots `deg` to
        # `deg + n`, is the source's [first knot, last knot] (checked against OCCT's periodic curves)
        at = lambda i: one[i % n] + per * (i // n)
        full = [at(i - deg + ks[0][1] - 1) for i in range(n + 2 * deg + 1)]
        return (deg, cp + cp[:deg], full, ws + ws[:deg])
    full = [v for v, m in ks for _ in range(m)]
    if len(full) != len(cp) + deg + 1 or ks[0][1] > deg + 1 or ks[-1][1] > deg + 1:
        return None
    return (deg, cp, full, ws)


def clamped(nb):
    """Whether a NURBS from bspline_exact runs from its first pole to its last."""
    deg, _, k, _ = nb
    return k[: deg + 1] == [k[0]] * (deg + 1) and k[-deg - 1 :] == [k[-1]] * (deg + 1)


def bspline_points(e, nb=None):
    """A fine polyline along a NURBS (the part is then marked approx unless the NURBS is written exactly): along
    `nb`, bspline_exact's form, where given, else along the data's poles and knots as they are."""
    if nb is not None:
        deg, poles, k, ws = nb
        cp = [(x, y, w) for (x, y), w in zip(poles, ws)]
    else:
        deg = int(e.get("Degree"))
        cp = [(float(p.get("X")), float(p.get("Y")), float(p.get("Weight", "1"))) for p in e.findall("Pole")]
        k = []
        for x in e.findall("Knot"):
            k += [float(x.get("Value"))] * int(x.get("Mult"))
    def at(u):
        s = deg
        while s < len(cp) - 1 and k[s + 1] <= u:
            s += 1
        d = [(cp[s - deg + j][0] * cp[s - deg + j][2], cp[s - deg + j][1] * cp[s - deg + j][2], cp[s - deg + j][2]) for j in range(deg + 1)]
        for r in range(1, deg + 1):
            for j in range(deg, r - 1, -1):
                i = s - deg + j
                den = k[i + deg - r + 1] - k[i]
                a = (u - k[i]) / den if den else 0.0
                d[j] = tuple(p * (1 - a) + q * a for p, q in zip(d[j - 1], d[j]))
        return [d[deg][0] / d[deg][2], d[deg][1] / d[deg][2]]
    lo, hi = k[deg], k[len(cp)]
    spans = sorted({x for x in k if lo <= x <= hi})
    us = []
    for a, b in zip(spans, spans[1:]):
        us += [a + (b - a) * i / 16 for i in range(16)]
    ps = [at(u) for u in us] + [at(hi)]
    if e.get("IsPeriodic") == "1":
        ps[-1] = ps[0]  # closed exactly, as the arrangement reads a loop
    return ps


def closed_wires(ents):
    """FreeCAD's Sketch::toShape: from the first edge left, pull in the first edge in document order that meets the wire's
    last or first vertex, until nothing meets or the wire closes. So at a junction the order decides, and a wire can
    end open; vertices meet within Precision::Confusion. Checked on 0.18 to 0.21 files. Returns the closed wires as lists of entity ids."""
    conf = 1e-7
    same = lambda p, q: math.dist(p, q) <= conf
    left = []
    keep = []
    for e in ents:
        if e.get("construction"):
            continue
        if e["kind"] in ("skCircle", "skEllipse"):
            keep.append([e["id"]])
        elif e["kind"] == "skEllipseArc":
            c, u = e["center"], e["majorAxis"]
            at = lambda t: sketch2d.ellipse_at(c, u, e["majorRadius"], e["minorRadius"], e["sense"], t)
            left.append((e["id"], at(e["t0"]), at(e["t1"])))
        elif e["kind"] in ("polyline", "nurbs"):
            left.append((e["id"], e["points"][0], e["points"][-1]))
        elif e["kind"] in ("skLineSegment", "skArc"):
            left.append((e["id"], e["start"], e["end"]))
    while left:
        i, first, last = left.pop(0)
        wire = [i]
        while left and not same(first, last):
            for k, (j, a, b) in enumerate(left):
                if same(last, a):
                    last = b
                elif same(first, b):
                    first = a
                elif same(last, b):
                    last = a
                elif same(first, a):
                    first = b
                else:
                    continue
                wire.append(j)
                del left[k]
                break
            else:
                break
        if same(first, last):
            keep.append(wire)
    return keep


def even_regions(a):
    """Regions at even nesting are solid, odd ones holes: FreeCAD's bullseye where nothing touches."""
    out = []
    for i, f in enumerate(a.faces):
        pt = sketch2d.point_within(a.loops[f["outer"]], a)
        depth = sum(1 for j, g in enumerate(a.faces) if j != i and g["area"] > f["area"] and sketch2d.inside(pt, a.loops[g["outer"]], a))
        if depth % 2 == 0:
            out.append(i)
    return out


def inner_point(a, i):
    """A point inside region i, beside its outer boundary, clear of its holes."""
    lp = a.loops[a.faces[i]["outer"]]
    for pc, fwd in lp:
        pts = a.pieces[pc].sample(8)
        k = len(pts) // 2
        u, v = pts[k - 1], pts[k]
        d = math.hypot(v[0] - u[0], v[1] - u[1])
        if d < 1e-9:
            continue
        s = 1.0 if fwd else -1.0
        n = (-(v[1] - u[1]) / d * s, (v[0] - u[0]) / d * s)
        m = ((u[0] + v[0]) / 2, (u[1] + v[1]) / 2)
        for w in (1e-3, 1e-4, 1e-5):
            q = (m[0] + n[0] * w, m[1] + n[1] * w)
            if a.holds(i, q):
                return q
    return sketch2d.point_within(lp, a)


def wire_regions(a, ents, wires):
    """FreeCAD's bullseye, which nests wires, not regions: a region inside an odd number of wires is solid. Wires that
    touch cut the arrangement's regions apart without changing that."""
    byid = {e["id"]: e for e in ents}
    outlines = []
    for w in wires:
        g = Arrangement([byid[i] for i in w])
        if g.faces:
            outlines.append((g, g.loops[max(g.faces, key=lambda f: f["area"])["outer"]]))
    out = []
    for i in range(len(a.faces)):
        q = inner_point(a, i)
        if sum(1 for g, lp in outlines if sketch2d.inside(q, lp, g)) % 2:
            out.append(i)
    return out


class Tr:
    def __init__(self, doc, body, topo, truth):
        self.doc, self.body, self.topo_exe, self.truth = doc, body, topo, truth
        self.lines = []  # fgw.St, one a statement
        self.prog = Prog()
        self.labels = {}
        self.topo = None
        self.bodies = 0
        self.approx = False
        self.steps = []
        self.tree = 0

    def state(self):
        if self.topo is None:
            self.topo = Topo(self.topo_exe)
        try:
            return self.topo.state(self.prog.text(self.lines))
        except Stop as x:
            raise KernelStop(str(x))

    def fresh(self, label):
        """`label`, or with a suffix where a statement already has it: a Forge id is bound once."""
        out, k = label, 0
        while any(l.label == out for l in self.lines):
            k += 1
            out = f"{label}r{k}"
        return out

    def select(self, what, names):
        """A lineage selection of exactly these edges or faces (their `n` or `name`) of the part so far."""
        if self.topo is None:
            self.topo = Topo(self.topo_exe)
        try:
            return self.topo.select(self.prog.text(self.lines), what, names)
        except Stop as x:
            raise KernelStop(str(x))
        except NoName as x:
            raise Unresolved(str(x))

    def profile_ents(self, sk, fr):
        if self.doc.types.get(sk) == "Sketcher::SketchObject":
            return sketch_ents(self.doc, sk)
        ws = self.truth.get(sk, {}).get("wires")
        if ws is None:
            raise Unresolved("a profile whose stored wires would not read")
        # a stored shape placed only by its own location, which is the profile's placement: its own coordinates are the sketch's
        root = self.truth[sk].get("root")
        local = root is not None and all("l" in e for w in ws for e in w) and all(
            abs(root[i][j] - (fr.x, fr.y, fr.n, fr.o)[j][i]) <= 1e-7 * (max(1.0, abs(fr.o[i])) if j == 3 else 1.0) for i in range(3) for j in range(4))
        ents = []
        for w in ws:
            for e in w:
                uv = [q[:2] for q in e["l"]] if local else [list(fr.uvw(q)[:2]) for q in e["p"]]
                i = f"w{len(ents)}"
                if e["k"] == "line":
                    ents.append({"kind": "skLineSegment", "id": i, "start": uv[0], "end": uv[-1]})
                elif e["k"] == "circle" and math.dist(uv[0], uv[-1]) < 1e-7:
                    c = sketch2d.circle_from(uv[0], uv[32], uv[64])
                    ents.append({"kind": "skCircle", "id": i, "center": list(c[0]), "radius": c[1]})
                elif e["k"] == "circle":
                    ents.append({"kind": "skArc", "id": i, "start": uv[0], "mid": uv[48], "end": uv[-1]})
                else:
                    self.approx = True
                    ents.append({"kind": "polyline", "id": i, "points": uv})
        return ents

    def profile(self, name, op, make, flip, frame_z, sk=None):
        """The sketch's regions, each a statement: `make` holds the statement's kind and its own fields."""
        sk = sk or self.doc.link(name, "Profile")
        if self.doc.types.get(sk) not in FLAT:
            raise Unsupported("a profile not from a sketch")
        fr = Frame(self.doc.placement(sk))
        ents = self.profile_ents(sk, fr)
        wires = closed_wires(ents)
        wired = {i for w in wires for i in w}
        a = Arrangement(ents)
        # older files wire a sketch some other way (a line drawn twice closes there, not here): if the wires do not account
        # for every curve, its regions are all there is to go on
        regs = wire_regions(a, ents, wires) if all(e.get("construction") or e["id"] in wired for e in ents) else even_regions(a)
        if not regs:
            raise Unresolved("a sketch with no closed region")
        frame = (vt(fr.o), vt(frame_z), vt(fr.x))
        out = []
        for prof in a.merged(regs):
            loops = [loop_segs(a, lp, flip, num) for lp in prof]
            # a spline or ellipse the arrangement cut is written as a polyline: the part is then approx
            if any(a.pieces[i].kind == "poly" and (a.pieces[i].src is None or a.pieces[i].src[0] not in ("nurbs", "ellipse")) for lp in prof for i, _ in lp):
                self.approx = True
            this = op
            if self.bodies == 0:
                if op == "cut":
                    raise Unsupported("first feature cuts")
                this = "new" if self.tree else ""
                self.bodies = 1
            out.append(St(op=this, frame=frame, loops=loops, **make))
        return out, a, fr

    def pad(self, name, cut):
        d = self.doc
        for k in ("TaperAngle", "TaperAngle2"):
            if abs(d.flt(name, k)) > 1e-12:
                raise Unsupported("a pad with " + k)
        if abs(d.flt(name, "Offset")) > 1e-12 and d.integer(name, "Type") != 3:
            raise Unsupported("a pad with Offset")
        sk = d.link(name, "Profile")
        if d.types.get(sk) not in FLAT:
            raise Unsupported("a profile not from a sketch")
        n = Frame(d.placement(sk)).n
        rev = d.boolean(name, "Reversed")
        down = cut != rev  # a pocket runs against the sketch normal
        z = tuple(-x for x in n) if down else n
        stored = d.prop(name, "Direction")
        if d.types.get(sk) != "Sketcher::SketchObject" and stored is not None and stored.find("PropertyVector") is not None:
            # a shape's plane may face either way; the feature records which way it went, or 1 1 1 if it never wrote it
            e = stored.find("PropertyVector")
            v = tuple(float(e.get(k)) for k in ("valueX", "valueY", "valueZ"))
            l = math.sqrt(dot(v, v))
            if l > 0 and abs(abs(dot(v, n)) / l - 1) < 1e-9:
                down = dot(v, n) < 0
                z = tuple(-x for x in n) if down else n
        t, L, L2 = d.integer(name, "Type"), d.flt(name, "Length"), d.flt(name, "Length2")
        mid = d.boolean(name, "Midplane")
        along = None
        if d.boolean(name, "UseCustomVector"):
            # the direction is stored as used, a pocket's too: only Reversed turns it. Length runs along it unless AlongSketchNormal
            e = stored.find("PropertyVector") if stored is not None else None
            if e is None:
                raise Unsupported("a pad along a custom direction with none stored")
            v = unit(tuple(float(e.get(k)) for k in ("valueX", "valueY", "valueZ")))
            c = dot(v, n)
            if abs(c) < 1e-9 or t not in (0, 1, 4):
                raise Unsupported("a pad along a custom direction of this form")
            down = (c < 0) != rev
            z = tuple(-x for x in n) if down else n
            if not d.boolean(name, "AlongSketchNormal"):
                L, L2 = L * abs(c), L2 * abs(c)
            if abs(abs(c) - 1) > 1e-12:
                along = tuple(fine(x) for x in v)
        if t == 0:
            ext = ("sym", num(L)) if mid else ("d", num(L))
        elif t == 4:
            ext = ("two", num(L), num(L2))
        elif t == 1 and cut:
            ext = ("thru", "both" if mid else "fwd")
        elif t == 2:
            ext = ("next", "fwd")
        elif t == 3 and along is None and not mid:
            ext = self.up_to_face(name, z)
        else:
            raise Unsupported(f"a pad or pocket of type {t}")
        return self.profile(name, "cut" if cut else "add", {"kind": "extrude", "ext": ext, "along": along}, down, z)[0]

    def up_to_face(self, name, z):
        """A pad or pocket up to a face of its base, found on our part by that face's edges. A plane square to the
        extrude is a depth, from the sketch's plane along `z` (the way the material goes), moved on by Offset along
        `z` (the reading 16233ee2 measures: it matches to 8e-16). Any other face is unsupported: `upto(FACES)` would
        do, but no part here has one to check it by."""
        d = self.doc
        o, subs = d.link(name, "UpToFace"), [x for x in d.subs(name, "UpToFace") if x]
        if len(subs) != 1 or not subs[0].startswith("Face"):
            raise Unsupported("a pad up to a face of this form")
        polys = self.truth.get(o, {}).get("subs", {}).get(subs[0])
        if not polys:
            raise Unresolved("an up-to face not sampled")
        f = self.face_of(self.state(), polys)
        off = d.flt(name, "Offset")
        if f.get("k") == "plane" and abs(abs(dot(tuple(float(x) for x in f["n"]), z)) - 1) < 1e-9:
            o3 = Frame(d.placement(d.link(name, "Profile"))).o
            w = dot(add(tuple(float(x) for x in f["p"][0]), o3, -1), z) + off
            if w <= 1e-9:
                raise Unsupported("a pad up to a face behind its sketch")
            return ("d", num(w))
        raise Unsupported("a pad up to a face not square to it")

    def revolve(self, name, cut):
        d = self.doc
        p, q = d.prop(name, "Base"), d.prop(name, "Axis")
        if p is None or q is None:
            raise Unsupported("a revolve without its axis")
        b = p.find("PropertyVector")
        c = q.find("PropertyVector")
        base = tuple(float(b.get(k)) for k in ("valueX", "valueY", "valueZ"))
        ax = tuple(float(c.get(k)) for k in ("valueX", "valueY", "valueZ"))
        sk = d.link(name, "Profile")
        if d.types.get(sk) not in FLAT:
            raise Unsupported("a profile not from a sketch")
        fr = Frame(d.placement(sk))
        u, v, w = fr.uvw(base)
        du, dv, dw = dot(ax, fr.x), dot(ax, fr.y), dot(ax, fr.n)
        if abs(w) > 1e-6 or abs(dw) > 1e-6:
            raise Unsupported("a revolve axis out of the sketch's plane")
        if d.boolean(name, "Reversed"):
            du, dv = -du, -dv
        ang = d.flt(name, "Angle", 360.0)
        turn = None if abs(ang - 360) < 1e-9 else (("sym", num(ang)) if d.boolean(name, "Midplane") else ("a", num(ang)))
        make = {"kind": "revolve", "turn": turn, "axis": (num(u), num(v), num(du), num(dv))}
        return self.profile(name, "cut" if cut else "add", make, False, fr.n)[0]

    def part_revolve(self, name):
        """Part workbench: a sketch turned about Axis through Base."""
        d = self.doc
        if not d.boolean(name, "Solid"):
            raise Unsupported("a Part revolve that makes a shell")
        vec = lambda k: tuple(float(d.prop(name, k).find("PropertyVector").get(w)) for w in ("valueX", "valueY", "valueZ"))
        base, ax = vec("Base"), vec("Axis")
        sk = d.link(name, "Source")
        if d.types.get(sk) not in FLAT:
            raise Unsupported("a Part revolve of something not a sketch")
        fr = Frame(d.placement(sk))
        u, v, w = fr.uvw(base)
        du, dv, dw = dot(ax, fr.x), dot(ax, fr.y), dot(ax, fr.n)
        if abs(w) > 1e-6 or abs(dw) > 1e-6:
            raise Unsupported("a revolve axis out of the sketch's plane")
        ang = d.flt(name, "Angle", 360.0)
        turn = None if abs(ang - 360) < 1e-9 else ("a", num(ang))
        if turn and d.boolean(name, "Symmetric"):
            turn = ("sym", num(ang))
        make = {"kind": "revolve", "turn": turn, "axis": (num(u), num(v), num(du), num(dv))}
        return self.profile(name, "add", make, False, fr.n, sk=sk)[0]

    def edge_hits(self, st, poly):
        """Our edges where their edge (samples `poly`) lies: ours on theirs, or theirs a piece of ours."""
        tol = TOL + sag(poly)
        lo = [min(q[k] for q in poly) - tol for k in range(3)]
        hi = [max(q[k] for q in poly) + tol for k in range(3)]
        inbox = lambda p: all(lo[k] <= p[k] <= hi[k] for k in range(3))
        hits = [e for e in st.get("edges", []) if any(inbox(p) and near(p, poly, tol) <= tol for p in e["p"][4:13:4])]
        return hits or [e for e in st.get("edges", []) if contains(e["p"], poly)]

    def face_of(self, st, polys):
        """Our face bounded by their face's edges: the one face every one of those edges borders."""
        common, matched = None, set()
        for poly in polys:
            fs = set()
            for e in self.edge_hits(st, poly):
                fs |= set(e["f"])
                matched.add(e["h"])
            common = fs if common is None else common & fs
            if not common:
                break
        if not common:
            raise Unresolved("no face where the base's face lies")
        if len(common) > 1:
            # a face's edges all shared with another face too (a disc on a cylinder's end): the one bounded by
            # nothing else
            common = {h for h in common if all(e["h"] in matched for e in st["edges"] if h in e["f"])}
        if len(common) != 1:
            raise Unresolved("a face its edges do not settle")
        h = common.pop()
        return next(f for f in st["faces"] if f["h"] == h)

    def thickness(self, name):
        """A hollow: the faces of Base opened, walls Value thick, inward when Reversed (FreeCAD's "inwards"), as
        `shell(t, open: FACES, out:)`. Only the skin mode."""
        d = self.doc
        if d.integer(name, "Mode", 0) != 0:
            raise Unsupported("a thickness of mode " + str(d.integer(name, "Mode", 0)))
        base = d.link(name, "Base")
        data = self.truth.get(base, {}).get("subs", {})
        st = self.state()
        faces = []
        for sub in [x for x in d.subs(name, "Base") if x]:
            if not sub.startswith("Face") or sub not in data:
                raise Unresolved("a thickness on " + sub[:4] + " not sampled")
            faces.append(self.face_of(st, data[sub]))
        if not faces:
            raise Unsupported("a thickness with no open face")
        sel = self.select("faces", {f["name"] for f in faces})
        return St("shell", size=num(d.flt(name, "Value")), sel=sel, out=not d.boolean(name, "Reversed"))

    def draft(self, name):
        """Faces tilted about where they cross a neutral plane, the pull along the plane's outward normal (turned
        by Reversed), as FreeCAD's BRepOffsetAPI_DraftAngle gets them."""
        d = self.doc
        base = d.link(name, "Base")
        data = self.truth.get(base, {}).get("subs", {})
        nb = d.link(name, "NeutralPlane")
        ns = [x for x in d.subs(name, "NeutralPlane") if x]
        if nb != base or len(ns) != 1 or not ns[0].startswith("Face"):
            raise Unsupported("a draft's neutral plane not a face of its base")
        st = self.state()
        faces = []
        for sub in [x for x in d.subs(name, "Base") if x]:
            if not sub.startswith("Face") or sub not in data:
                raise Unresolved("a draft on " + sub[:4] + " not sampled")
            faces.append(self.face_of(st, data[sub]))
        if not faces:
            raise Unresolved("a draft of no faces")
        if ns[0] not in data:
            raise Unresolved("a draft's neutral face not sampled")
        nf = self.face_of(st, data[ns[0]])
        if nf.get("k") != "plane":
            raise Unsupported("a draft's neutral face not a plane")
        nrm = tuple(float(x) for x in nf["n"])
        if d.link(name, "PullDirection"):
            # its own pull: a line edge, run from its start, square to the neutral plane (Forge pulls along its normal)
            pb, ps = d.link(name, "PullDirection"), [x for x in d.subs(name, "PullDirection") if x]
            polys = self.truth.get(pb, {}).get("subs", {}).get(ps[0]) if len(ps) == 1 else None
            if not polys or len(polys) != 1 or not ps[0].startswith("Edge"):
                raise Unsupported("a draft's pull direction not an edge")
            v = unit(add(polys[0][-1], polys[0][0], -1))
            c = dot(v, nrm)
            if abs(abs(c) - 1) > 1e-9:
                raise Unsupported("a draft pulled off its neutral plane's normal")
            nrm = v
        if d.boolean(name, "Reversed"):
            nrm = tuple(-x for x in nrm)
        at = tuple(float(x) for x in nf["p"][0])
        sel = self.select("faces", {f["name"] for f in faces})
        return St("draft", sel=sel, angle=num(d.flt(name, "Angle")), plane=(vt(at), vt(nrm), None))

    def pipe(self, name, cut):
        """A pipe: along a helix in Frenet mode it is a coil (the Frenet frame of a helix turns with the screw
        motion, so the profile is wound rigidly); along a sketch's lines and arcs, a sweep (Standard, OpenCascade's
        corrected Frenet, and Frenet both carry a planar path's profile without twist)."""
        d = self.doc
        sec = d.prop(name, "Sections")
        if sec is not None and sec.find(".//Link") is not None:
            raise Unsupported("a pipe through sections")
        for k in ("AuxillerySpine", "AuxiliarySpine"):
            if d.link(name, k):
                raise Unsupported("a pipe with an auxiliary spine")
        mode = d.integer(name, "Mode", 0)
        sp = d.link(name, "Spine")
        helix = sp if d.types.get(sp) == "Part::Helix" else None
        if d.types.get(sp) == "PartDesign::ShapeBinder":
            p = d.prop(sp, "Support")
            got = [l.get("obj") or l.get("value") for l in p.iter("Link")] if p is not None else []
            if len(got) == 1 and d.types.get(got[0]) == "Part::Helix" and d.placement(sp) == d.placement(got[0]):
                helix = got[0]
            else:
                raise Unsupported("a pipe along a shape binder of " + ", ".join(str(d.types.get(g)).split("::")[-1] for g in got))
        sk = d.link(name, "Profile")
        if d.types.get(sk) not in FLAT:
            raise Unsupported("a profile not from a sketch")
        fr = Frame(d.placement(sk))
        op = "cut" if cut else "add"
        if helix is not None:
            if mode != 2:
                raise Unsupported("a pipe along a helix, not Frenet")
            if abs(d.flt(helix, "Angle")) > 1e-12 or d.integer(helix, "Style", 1) != 1:
                raise Unsupported("a pipe along a tapered or old-style helix")
            o, q = d.placement(helix)
            ax = rotate(q, (0.0, 0.0, 1.0))
            pitch, height, left = helix_turns(self.truth.get(sp, {}).get("wires"), o, ax)
            u, v, w = fr.uvw(o)
            du, dv, dw = dot(ax, fr.x), dot(ax, fr.y), dot(ax, fr.n)
            if abs(w) > 1e-6 or abs(dw) > 1e-9:
                raise Unsupported("a helix axis out of the profile's plane")
            make = {"kind": "coil", "axis": (num(u), num(v), num(du), num(dv)), "pitch": num(pitch), "height": num(height), "left": left}
            return self.profile(name, op, make, False, fr.n)[0]
        if d.types.get(sp) != "Sketcher::SketchObject":
            raise Unsupported("a pipe along " + str(d.types.get(sp)).split("::")[-1])
        if mode not in (0, 2):
            raise Unsupported(f"a pipe of mode {mode}")
        if d.integer(name, "Transition", 0) not in (0, 1, 2):
            raise Unsupported("a pipe transition")
        sfr = Frame(d.placement(sp))
        ents = [e for e in sketch_ents(d, sp) if not e.get("construction")]
        subs = [x for x in d.subs(name, "Spine") if x]
        if subs:
            # the spine's edges by name: only all of them is read (a sketch's EdgeN is its shape's order, not its rows')
            n_edges = sum(1 for e in ents if e["kind"] in ("skLineSegment", "skArc", "skCircle", "nurbs", "polyline", "skEllipse", "skEllipseArc"))
            if len(set(subs)) != n_edges:
                raise Unsupported("a pipe along some edges of a sketch")
        pieces = []
        for pc in sketch2d.entities(ents)[0]:
            if pc.kind == "line":
                pieces.append(WPiece(sfr.world(*pc.a), sfr.world(*pc.b), ent=pc.ent))
            elif pc.kind == "arc":
                _, sw = pc.sweep()
                pieces.append(WPiece(sfr.world(*pc.a), sfr.world(*pc.b), sfr.world(*pc.c), sfr.n, sw, ent=pc.ent))
            elif pc.kind == "circle":
                x = sfr.world(*pc.a)
                pieces.append(WPiece(x, x, sfr.world(*pc.c), sfr.n, 2 * math.pi, ent=pc.ent))
            else:
                raise Unsupported("a pipe along a spline")
        if not pieces:
            raise Unresolved("a pipe along nothing")
        run, closed = chain_pieces(pieces)
        first = next((e for e in self.profile_ents(sk, fr) if not e.get("construction")), None)
        if first is None:
            raise Unresolved("a pipe's profile with no curve")
        q = first.get("start") or first.get("center") or first["points"][0]
        paths = path_from(run, closed, fr, fr.world(q[0], q[1]))
        if len(paths) != 1:
            raise Unsupported("a pipe from inside its path")
        path = tangent_fit(paths[0])
        down = dot(path[0].tangent(path[0].a), fr.n) < 0
        sts = self.profile(name, op, {"kind": "sweep", "path": [x.path() for x in path]}, down, tuple(-x for x in fr.n) if down else fr.n)[0]
        return sts

    def after_moved(self, name):
        """Does a real feature follow this clone in the body being built? Those are in the body's own frame, not the moved one."""
        return bool(self.chain_rest) and any(self.doc.types[c] != "PartDesign::FeatureBase" for c in self.chain_rest[self.chain_rest.index(name) + 1:])

    def part_tree(self, name):
        """A Part workbench solid and what it is made of, as bodies and combines. Tools first, so the base is the last body
        made and the lines after it act on that. Returns (lines, the result's label)."""
        d, t = self.doc, self.doc.types.get(name)
        if t in ("Part::Cut", "Part::Fuse", "Part::Common", "Part::MultiFuse", "Part::MultiCommon"):
            pl = d.placement(name)
            if any(abs(x) > 1e-9 for x in pl[0]) or abs(abs(pl[1][3]) - 1) > 1e-9:
                raise Unsupported("a moved Part boolean")
            if t in ("Part::MultiFuse", "Part::MultiCommon"):
                kids = d.links(name, "Shapes")
            else:
                kids = [d.link(name, "Base"), d.link(name, "Tool")]
            if len(kids) < 2 or not all(kids):
                raise Unsupported("a Part boolean short of shapes")
            labs = [self.part_tree(k) for k in kids[1:] + kids[:1]]
            word = {"Part::Cut": "cut", "Part::Common": "keep", "Part::MultiCommon": "keep"}.get(t, "add")
            self.lines.append(St("combine", word=word, target=labs[-1], tools=labs[:-1], keep=False))
            return labs[-1]
        if t == "PartDesign::Body" or t in SOLID and not t.startswith("Part::"):
            # a PartDesign chain as one shape of the tree: a body of its own, what follows it acting on it
            sub = chain_of(d, name)
            self.tree += 1
            self.bodies = 0
            first = len(self.lines)
            for n in sub:
                self.step(n)
            head = self.lines[first]
            self.bodies = 1
            if not head.label:
                raise Unsupported("a PartDesign shape in a Part tree with no label")
            return head.label
        if t == "Part::Revolution":
            ls = self.part_revolve(name)
        elif t == "Part::Extrusion":
            ls = self.part_extrude(name)
        elif t in ("Part::Box", "Part::Cylinder"):
            ls = self.part_solid(name, t)
        elif t == "Part::Sphere":
            if abs(d.flt(name, "Angle1", -90.0) + 90) > 1e-9 or abs(d.flt(name, "Angle2", 90.0) - 90) > 1e-9 or abs(d.flt(name, "Angle3", 360.0) - 360) > 1e-9:
                raise Unsupported("a part of a sphere")
            fr = Frame(d.placement(name))
            ls = [St("ball", r=num(d.flt(name, "Radius")), frame=(vt(fr.o), vt(fr.n), vt(fr.x)))]
        else:
            raise Unsupported("a Part tree through " + str(t))
        if len(ls) != 1:
            raise Unsupported("a Part solid of several regions")
        self.tree += 1
        lab = f"p{self.tree}"
        if self.tree > 1:
            # after the first, each its own body
            ls[0].op = "new"
        ls[0].label = lab
        self.lines.append(ls[0])
        return lab

    def part_extrude(self, name):
        """Part workbench: a sketch's faces moved along Dir. Older files have no LengthFwd: Dir's own length is the
        distance. Newer ones run LengthFwd along it (and LengthRev back), Dir's length when both are zero."""
        d = self.doc
        if not d.boolean(name, "Solid"):
            raise Unsupported("a Part extrusion that makes a shell")
        if abs(d.flt(name, "TaperAngle")) > 1e-12 or abs(d.flt(name, "TaperAngleRev")) > 1e-12:
            raise Unsupported("a tapered Part extrusion")
        pl = d.placement(name)
        if any(abs(x) > 1e-9 for x in pl[0]) or abs(abs(pl[1][3]) - 1) > 1e-9:
            raise Unsupported("a moved Part extrusion")
        sk = d.link(name, "Base")
        if d.types.get(sk) not in FLAT:
            raise Unsupported("a Part extrusion of something not a sketch")
        fr = Frame(d.placement(sk))
        n = fr.n
        v = tuple(float(d.prop(name, "Dir").find("PropertyVector").get(w)) for w in ("valueX", "valueY", "valueZ"))
        lv = math.sqrt(dot(v, v))
        if d.prop(name, "LengthFwd") is None:
            fwd, rev, sym = lv, 0.0, False
        else:
            if d.integer(name, "DirMode", 0) == 2:
                v, lv = n, 1.0
            elif d.integer(name, "DirMode", 0) != 0:
                raise Unsupported("a Part extrusion along an edge")
            fwd, rev, sym = d.flt(name, "LengthFwd"), d.flt(name, "LengthRev"), d.boolean(name, "Symmetric")
            if abs(fwd) < 1e-12 and abs(rev) < 1e-12:
                fwd = lv
            if d.boolean(name, "Reversed"):
                v = tuple(-x for x in v)
        if lv < 1e-12:
            raise Unsupported("a Part extrusion of no direction")
        u = unit(v)
        c = dot(u, n)
        if abs(c) < 1e-9:
            raise Unsupported("a Part extrusion along its sketch's plane")
        down = c < 0
        z = tuple(-x for x in n) if down else n
        # the distances run along Dir; Forge measures them along the normal and slants the walls
        fwd, rev = fwd * abs(c), rev * abs(c)
        along = tuple(fine(x) for x in u) if abs(abs(c) - 1) > 1e-12 else None
        if sym:
            ext = ("sym", num(fwd))
        elif rev > 1e-12:
            ext = ("two", num(fwd), num(rev))
        else:
            ext = ("d", num(fwd))
        return self.profile(name, "add", {"kind": "extrude", "ext": ext, "along": along}, down, z, sk=sk)[0]

    def part_solid(self, name, t):
        """Part workbench primitives, on their placement. An add: only ever the first in a chain here."""
        d = self.doc
        fr = Frame(d.placement(name))
        frame = (vt(fr.o), vt(fr.n), vt(fr.x))
        self.bodies = 1
        ext = ("d", num(d.flt(name, "Height")))
        if t == "Part::Box":
            return [St("extrude", frame=frame, ext=ext, loops=[[rect_row(d, name)]])]
        if abs(d.flt(name, "Angle", 360.0) - 360) > 1e-9:
            raise Unsupported("a part of a cylinder")
        return [St("extrude", frame=frame, ext=ext, loops=[[("circle", ("0", "0"), num(d.flt(name, "Radius")))]])]

    def blend(self, name, kind, size, all_edges=False):
        d = self.doc
        base = d.link(name, "Base")
        subs = ["ALL"] if all_edges else d.subs(name, "Base")
        subs = subs or ["ALL"]
        st = self.state()
        data = self.truth.get(base, {}).get("subs", {})
        picked = {}
        every = [poly for sub in subs for poly in data.get(sub, [])]
        for sub in subs:
            if sub not in data:
                raise Unresolved("a blend on " + sub[:4] + " not sampled")
            for poly in data[sub]:
                if self.approx and not line_or_circle(poly):
                    # the part's spline is a polyline here, so its edge is many short ones the blend cannot round as one
                    raise Unsupported("a blend along a spline")
                # one of ours can span several of theirs (a seam, a split circle): its middle stretch decides
                # their samples are chords: on a big circle the curve between them lies further off than TOL
                tol = TOL + sag(poly)
                lo = [min(q[k] for q in poly) - tol for k in range(3)]
                hi = [max(q[k] for q in poly) + tol for k in range(3)]
                inbox = lambda p: all(lo[k] <= p[k] <= hi[k] for k in range(3))
                hits = [e for e in st.get("edges", []) if any(inbox(p) and near(p, poly, tol) <= tol for p in e["p"][4:13:4])]
                if not hits and self.approx:
                    # where a polyline stands in for a spline, what it meets moves a little: the nearest within the part's slack
                    box = self.truth.get(base, {}).get("vbox") or [0.0] * 6
                    slack = 1e-3 * math.dist(box[:3], box[3:])
                    scored = sorted(((max(near(p, poly, 0.0) for p in e["p"][4:13:4]), e["h"]) for e in st.get("edges", [])), key=lambda x: x[0])
                    if scored and scored[0][0] <= slack:
                        # only a clear winner: a second edge as close would be a guess
                        close = [h for d, h in scored if d <= scored[0][0] + 1e-9]
                        hits = [e for e in st["edges"] if e["h"] in close]
                if not hits:
                    # theirs a piece of one of ours (a loop they split at a vertex we do not have): every sample of theirs
                    # on our edge, and our edge covered by the edges they pick, so the blend rounds no more than theirs
                    hits = [e for e in st.get("edges", []) if contains(e["p"], poly) and covered(e["p"], every)]
                if not hits:
                    if not all_edges:
                        raise Unresolved("no edge where the base's edge lies")
                for e in hits:
                    if e["c"] != "smooth":
                        picked[e["h"]] = e
        if not picked:
            raise Unresolved("no edge where the base's edge lies")
        # OpenCascade carries a blend along tangent edges
        along_tangents(st["edges"], picked)
        return [St(kind, size=size, sel=self.select("edges", {e["n"] for e in picked.values()}))]

    def axis_of(self, name, prop):
        """A pattern's direction or axis: a sketch's H, V or N axis, or the body's origin line."""
        d = self.doc
        ref, subs = d.link(name, prop), d.subs(name, prop)
        s = subs[0] if subs else ""
        if d.types.get(ref) == "Sketcher::SketchObject":
            fr = Frame(d.placement(ref))
            return fr.o, {"H_Axis": fr.x, "V_Axis": fr.y, "N_Axis": fr.n}.get(s) or self.fail(prop)
        if d.types.get(ref) == "App::Line":
            role = d.prop(ref, "Role")
            w = role.find("String").get("value") if role is not None else ""
            return (0.0, 0.0, 0.0), {"X_Axis": (1.0, 0.0, 0.0), "Y_Axis": (0.0, 1.0, 0.0), "Z_Axis": (0.0, 0.0, 1.0)}.get(w) or self.fail(prop)
        if d.types.get(ref) == "PartDesign::Line":
            fr = Frame(d.placement(ref))
            return fr.o, fr.n
        if d.types.get(ref) in SOLID and s.startswith("Edge"):
            # a straight edge of a feature's stored shape: from its first sample to its last
            polys = self.truth.get(ref, {}).get("subs", {}).get(s)
            if not polys or len(polys) != 1 or not line_or_circle(polys[0]):
                raise Unsupported("a pattern axis on an edge not sampled or not straight")
            a, b = polys[0][0], polys[0][-1]
            u = unit(add(b, a, -1))
            if any(math.dist(add(a, u, dot(add(q, a, -1), u)), q) > 1e-6 for q in polys[0]):
                raise Unsupported("a pattern axis on a curved edge")
            return tuple(a), u
        raise Unsupported("a pattern axis on " + (d.types.get(ref) or "nothing"))

    def transform(self, name, t, src):
        d = self.doc
        if t == "PartDesign::PolarPattern":
            o, a = self.axis_of(name, "Axis")
            if d.boolean(name, "Reversed"):
                a = tuple(-x for x in a)
            return St("pattern", of=list(src), layout=("circ", vt(o), vt(a), str(d.integer(name, "Occurrences")), num(d.flt(name, "Angle", 360.0)), False))
        if t == "PartDesign::LinearPattern":
            _, a = self.axis_of(name, "Direction")
            if d.boolean(name, "Reversed"):
                a = tuple(-x for x in a)
            n = d.integer(name, "Occurrences")
            return St("pattern", of=list(src), layout=("lin", vt(a), str(n), num(d.flt(name, "Length") / max(n - 1, 1))))
        ref, subs = d.link(name, "MirrorPlane"), d.subs(name, "MirrorPlane")
        if d.types.get(ref) == "Sketcher::SketchObject":
            fr = Frame(d.placement(ref))
            n = {"H_Axis": fr.y, "V_Axis": fr.x, "": fr.n}.get(subs[0] if subs else "") or self.fail("mirror")
            o = fr.o
        elif d.types.get(ref) == "App::Plane":
            role = d.prop(ref, "Role").find("String").get("value")
            n = {"XY_Plane": (0.0, 0.0, 1.0), "XZ_Plane": (0.0, 1.0, 0.0), "YZ_Plane": (1.0, 0.0, 0.0)}.get(role) or self.fail("mirror")
            o = (0.0, 0.0, 0.0)
        elif d.types.get(ref) == "PartDesign::Plane":
            fr = Frame(d.placement(ref))
            n, o = fr.n, fr.o
        elif d.types.get(ref) in SOLID and subs and subs[0].startswith("Face"):
            # a planar face of a feature's stored shape: the plane through its edges' samples
            polys = self.truth.get(ref, {}).get("subs", {}).get(subs[0])
            pts = [q for poly in polys or [] for q in poly]
            n, o = plane_of(pts)
            if n is None:
                raise Unsupported("a mirror in a face not sampled or not flat")
        else:
            raise Unsupported("a mirror in " + (d.types.get(ref) or "nothing"))
        return St("pattern", of=list(src), layout=("mirror", vt(n), vt(o)))

    def hole(self, name):
        """One hole at each of the sketch's circles, drilled against its normal; its seat from what the file stored."""
        d = self.doc
        if d.boolean(name, "Tapered") or d.boolean(name, "ModelActualThread") or d.boolean(name, "Midplane"):
            raise Unsupported("a tapered, threaded or two-way hole")
        if d.boolean(name, "Threaded"):
            # measured: bodies with this set stored more than the plain bore; a hole's thread is recorded, the shape is another matter
            raise Unsupported("a hole with its thread in the shape")
        sk = d.link(name, "Profile")
        fr = Frame(d.placement(sk))
        ents = [e for e in sketch_ents(d, sk) if not e.get("construction")]
        if not ents or any(e["kind"] != "skCircle" for e in ents):
            raise Unsupported("a hole on something other than circles")
        z = fr.n if d.boolean(name, "Reversed") else tuple(-x for x in fr.n)
        t = d.integer(name, "DepthType")
        ext = "thru" if t == 1 else num(d.flt(name, "Depth")) if t == 0 else None
        if ext is None:
            raise Unsupported("a hole up to a face")
        pt = None
        if t == 0 and d.integer(name, "DrillPoint") == 1:
            pt = num(d.flt(name, "DrillPointAngle"))
        seat = None
        if d.integer(name, "HoleCutType") != 0:
            if d.flt(name, "HoleCutDepth") > 0:
                seat = ("cb", num(d.flt(name, "HoleCutDiameter")), num(d.flt(name, "HoleCutDepth")))
            else:
                seat = ("cs", num(d.flt(name, "HoleCutDiameter")), num(d.flt(name, "HoleCutCountersinkAngle")))
        D = num(d.flt(name, "Diameter"))
        return [St("hole", d=D, frame=(vt(fr.world(*e["center"])), vt(z), vt(fr.x)), seat=seat, depth=ext, point=pt) for e in ents]

    def primitive(self, name, t):
        d = self.doc
        fr = Frame(d.placement(name))
        frame = (vt(fr.o), vt(fr.n), vt(fr.x))
        cut = "Subtractive" in t
        if cut and self.bodies == 0:
            raise Unsupported("first feature cuts")
        op = ("cut" if cut else "add") if self.bodies else ""
        self.bodies = 1
        ext = ("d", num(d.flt(name, "Height")))
        if t.endswith("Box"):
            return [St("extrude", op=op, frame=frame, ext=ext, loops=[[rect_row(d, name)]])]
        if abs(d.flt(name, "Angle", 360.0) - 360) > 1e-9 or abs(d.flt(name, "FirstAngle")) > 1e-9 or abs(d.flt(name, "SecondAngle")) > 1e-9:
            raise Unsupported("a part of a cylinder")
        return [St("extrude", op=op, frame=frame, ext=ext, loops=[[("circle", ("0", "0"), num(d.flt(name, "Radius")))]])]

    def fail(self, what):
        raise Unsupported("a pattern reference " + what)

    def originals(self, name):
        out = []
        for o in self.doc.links(name, "Originals"):
            if o not in self.labels:
                raise Unresolved("a pattern of a feature outside the chain")
            out += self.labels[o].split(" ")
        return out

    def run(self, chain):
        d = self.doc
        for name in chain:
            t = d.types[name]
            if t not in NEEDS:
                raise Unsupported("feature " + t.split("::")[-1])
        self.chain_rest = chain
        for name in chain:
            self.step(name)

    def step(self, name):
        d = self.doc
        t = d.types[name]
        if t == "PartDesign::FeatureBase":
            # stands for its BaseFeature, which the chain has already built, moved by its own placement
            # (measured: a chain of clones composes them, and the base feature's own placement plays no part)
            by, q = d.placement(name)
            sn = math.sqrt(sketch2d.sq(q[0]) + sketch2d.sq(q[1]) + sketch2d.sq(q[2]))
            if sn < 1e-12 and math.sqrt(dot(by, by)) < 1e-9:
                return
            if self.after_moved(name):
                raise Unsupported("features after a moved clone")
            moves = []
            if sn >= 1e-12:
                moves.append(("rot", ("0", "0", "0"), vt(tuple(x / sn for x in q[:3])), num(math.degrees(2 * math.atan2(sn, q[3])))))
            if math.sqrt(dot(by, by)) >= 1e-9:
                moves.append(("by", vt(by)))
            self.lines.append(St("movebody", moves=moves, of=None))
            self.steps.append((name, len(self.lines)))
            return
        if t.startswith("Part::"):
            ls, root = [], self.part_tree(name)
        elif t in ("PartDesign::Pad", "PartDesign::Pocket"):
            ls = self.pad(name, t.endswith("Pocket"))
        elif t in ("PartDesign::Revolution", "PartDesign::Groove"):
            ls = self.revolve(name, t.endswith("Groove"))
        elif t == "PartDesign::Fillet":
            ls = self.blend(name, "fillet", num(d.flt(name, "Radius")), d.boolean(name, "UseAllEdges"))
        elif t == "PartDesign::Chamfer":
            if d.integer(name, "ChamferType") != 0:
                raise Unsupported("an unequal chamfer")
            ls = self.blend(name, "chamfer", num(d.flt(name, "Size")), d.boolean(name, "UseAllEdges"))
        elif t in TRANSFORMS:
            ls = [self.transform(name, t, self.originals(name))]
        elif t == "PartDesign::MultiTransform":
            ls, src = [], self.originals(name)
            for k, m in enumerate(d.links(name, "Transformations")):
                if d.types.get(m) not in TRANSFORMS:
                    raise Unsupported("a multi-transform with " + str(d.types.get(m)).split("::")[-1])
                one = self.transform(m, d.types[m], src)
                one.label = self.fresh(f"m{len(self.labels)}x{k}")
                ls.append(one)
                src = src + [one.label]
        elif t == "PartDesign::Hole":
            ls = self.hole(name)
        elif t == "PartDesign::Draft":
            ls = [self.draft(name)]
        elif t == "PartDesign::Thickness":
            ls = [self.thickness(name)]
        elif t in ("PartDesign::AdditivePipe", "PartDesign::SubtractivePipe"):
            ls = self.pipe(name, t == "PartDesign::SubtractivePipe")
        elif t in ("PartDesign::AdditiveBox", "PartDesign::SubtractiveBox", "PartDesign::AdditiveCylinder", "PartDesign::SubtractiveCylinder"):
            ls = self.primitive(name, t)
        if t.startswith("Part::"):
            self.labels[name] = root
            self.bodies = 1
        elif t == "PartDesign::MultiTransform":
            self.labels[name] = ls[-1].label  # already labelled, each its own
        else:
            # a feature of several lines (regions that do not touch): a pattern of it needs them all
            base = f"t{len(self.labels)}"
            # a feature stepped again (a chain two bodies in a Part tree share) gets ids of its own
            k = 0
            while any(l.label == base or (l.label or "").startswith(base + "x") for l in self.lines):
                k += 1
                base = f"t{len(self.labels)}r{k}"
            names = [base if k == 0 else f"{base}x{k}" for k in range(len(ls))]
            self.labels[name] = " ".join(names)
            for n, l in zip(names, ls):
                l.label = n
        self.lines += ls
        self.steps.append((name, len(self.lines)))


def rect_row(d, name):
    """A Box's base: Length along x, Width along y from the placement's origin."""
    return f"rect(corner: (0, 0), size: ({num(d.flt(name, 'Length'))}, {num(d.flt(name, 'Width'))}))"


def line_or_circle(poly):
    """Do these samples lie on one line or one circle?"""
    a, m, b = poly[0], poly[len(poly) // 3], poly[2 * len(poly) // 3]
    u, v = add(m, a, -1), add(b, a, -1)
    cr = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
    nn = dot(cr, cr)
    if nn < 1e-18:
        return all(math.sqrt(max(dot(add(q, a, -1), add(q, a, -1)) - sketch2d.sq(dot(add(q, a, -1), v)) / max(dot(v, v), 1e-18), 0.0)) < 1e-6 for q in poly)
    t = lambda x, y: (x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0])
    c = add(a, add(tuple(x * dot(u, u) for x in t(v, cr)), tuple(x * dot(v, v) for x in t(cr, u))), 1 / (2 * nn))
    r = math.dist(a, c)
    return all(abs(math.dist(q, c) - r) < 1e-6 for q in poly)


def along_tangents(edges, picked):
    """Grows {handle: edge} along edges tangent where they meet."""
    at = {}
    for e in edges:
        for k, v in enumerate(e["v"]):
            at.setdefault(v, []).append((e, k))
    todo = list(picked.values())
    while todo:
        e = todo.pop()
        for k, v in enumerate(e["v"]):
            t = e["t"][k]
            for g, j in at[v] if t else []:
                if g["h"] in picked or g["c"] == "smooth" or not g["t"][j]:
                    continue
                if carries_on(e, k, g, j, at[v]):
                    picked[g["h"]] = g
                    todo.append(g)


def helix_turns(ws, o, ax):
    """The pitch, height and hand of the helix a spine's stored shape holds (a shape binder keeps the helix as it was
    when bound, which an edit of the helix since does not move): from its samples, the rise along the axis through
    `o` and the turn about it. The sweep starts where the profile is and rises that far."""
    pts = [p for w in ws or [] for e in w for p in e["p"]]
    if len(pts) < 8:
        raise Unresolved("a helix whose stored shape would not read")
    # one edge after another, each run its own way: order the samples by their rise
    pts.sort(key=lambda p: dot(add(p, o, -1), ax))
    k = (1.0, 0.0, 0.0) if abs(ax[0]) < 0.9 else (0.0, 1.0, 0.0)
    e1 = unit(add(k, ax, -dot(k, ax)))
    e2 = (ax[1] * e1[2] - ax[2] * e1[1], ax[2] * e1[0] - ax[0] * e1[2], ax[0] * e1[1] - ax[1] * e1[0])
    hs, th, last = [], [], None
    for p in pts:
        r = add(p, o, -1)
        a = math.atan2(dot(r, e2), dot(r, e1))
        if last is not None:
            while a - last > math.pi:
                a -= 2 * math.pi
            while a - last < -math.pi:
                a += 2 * math.pi
        last = a
        hs.append(dot(r, ax))
        th.append(a)
    height, turn = hs[-1] - hs[0], th[-1] - th[0]
    if height <= 1e-9 or abs(turn) < 1e-6:
        raise Unsupported("a pipe along a helix of no rise or turn")
    pitch = height * 2 * math.pi / abs(turn)
    # the same pitch all the way: a taper or a varying pitch is no coil
    for h, t in zip(hs, th):
        if abs((h - hs[0]) - abs(t - th[0]) * pitch / (2 * math.pi)) > 1e-4 * max(1.0, height):
            raise Unsupported("a pipe along a helix of varying pitch")
    return round(pitch, 9), round(height, 9), turn < 0


def plane_of(pts):
    """The plane through points that are flat (within 1e-7 of it): its unit normal (Newell's, about the centroid)
    and a point; (None, None) if they are not."""
    if len(pts) < 3:
        return None, None
    c = tuple(sum(p[k] for p in pts) / len(pts) for k in range(3))
    # the two points farthest apart and the one farthest from their line span it
    a = max(pts, key=lambda p: math.dist(p, c))
    b = max(pts, key=lambda p: math.dist(p, a))
    ab = unit(add(b, a, -1))
    e = max(pts, key=lambda p: math.dist(add(a, ab, dot(add(p, a, -1), ab)), p))
    n = (lambda u, v: (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]))(add(b, a, -1), add(e, a, -1))
    if math.sqrt(dot(n, n)) < 1e-12:
        return None, None
    n = unit(n)
    if any(abs(dot(add(p, a, -1), n)) > 1e-7 * max(1.0, math.dist(a, b)) for p in pts):
        return None, None
    return n, a


def contains(ours, poly):
    """Does our edge (its samples `ours`) hold every sample of their edge `poly`? Ours are coarse: their
    distance from our chords is allowed our chords' own bow."""
    tol = TOL + sag(ours)
    lo = [min(q[k] for q in ours) - tol for k in range(3)]
    hi = [max(q[k] for q in ours) + tol for k in range(3)]
    return all(all(lo[k] <= p[k] <= hi[k] for k in range(3)) and near(p, ours, tol) <= tol for p in poly[::4] + [poly[-1]])


def covered(ours, polys):
    """Is every sample of our edge on one of their edges?"""
    return all(any(near(p, poly, TOL + sag(poly)) <= TOL + sag(poly) for poly in polys) for p in ours)


def sag(poly):
    """How far a sampled curve's chords stray from it: half the bow of two steps."""
    i = len(poly) // 2
    if i == 0 or i + 1 >= len(poly):
        return 0.0
    return near(poly[i], [poly[i - 1], poly[i + 1]], 0.0) / 2


def near(p, poly, enough=TOL):
    best = math.inf
    for a, b in zip(poly, poly[1:]):
        e = add(b, a, -1)
        l2 = dot(e, e)
        t = 0.0 if l2 == 0 else max(0.0, min(1.0, dot(add(p, a, -1), e) / l2))
        q = add(a, e, t)
        best = min(best, math.dist(p, q))
        if best <= enough:
            return best
    return best


def alone(doc, n):
    """A feature of an older file whose stored result is just its own tool has no base."""
    p = doc.prop(n, "AddSubShape")
    if p is None:
        p = doc.prop(n, "AddShape")
    e = p.find("Part") if p is not None else None
    a, b = (e.get("file") if e is not None else None), doc.shape_file(n)
    if not a or not b or doc.volumes is None:
        return False
    va, vb = doc.volumes.get(a), doc.volumes.get(b)
    return va is not None and vb is not None and abs(va - vb) <= 1e-6 * max(vb, 1e-12)


def support_of(doc, sk):
    """What a sketch is attached to, across the file versions' three forms."""
    p = doc.prop(sk, "Support")
    e = p.find("LinkSubList") if p is not None else None
    if e is not None:
        ls = e.findall("Link")
        return ls[0].get("obj") if ls else ""
    return doc.link(sk, "Support")


def base_of(doc, n):
    """The feature before n: older files without BaseFeature say it through a blend's Base, a pattern's
    original, or the solid its sketch sits on."""
    b = doc.link(n, "BaseFeature")
    if b:
        return b
    if doc.prop(n, "BaseFeature") is not None:
        # empty inside a body means first. Outside one (0.18 to 0.21 files without bodies) it says nothing: the sketch's support is the base
        if doc.bodied is None:
            doc.bodied = {x for k, t in doc.types.items() if t == "PartDesign::Body" for x in doc.links(k, "Group")}
        s = support_of(doc, doc.link(n, "Profile")) if doc.link(n, "Profile") in doc.props else ""
        if not s and n not in doc.bodied and doc.types.get(n) in ("PartDesign::Fillet", "PartDesign::Chamfer"):
            # a blend outside any body acts on its Base
            s = doc.link(n, "Base")
        return s if n not in doc.bodied and doc.types.get(s) in SOLID else ""
    b = doc.link(n, "Base")
    if b in doc.types and doc.types[b] in SOLID:
        return b
    o = doc.links(n, "Originals")
    if o:
        return o[-1]
    sk = doc.link(n, "Profile")
    if sk in doc.props:
        s = support_of(doc, sk)
        # a sketch on a solid the importer has no form for says so here, not as a cut with nothing to cut
        if s in doc.types and doc.types[s] not in ("App::Plane", "App::Origin", "PartDesign::Plane", "App::Line", "PartDesign::Line", "App::Part", "PartDesign::Body", "PartDesign::CoordinateSystem") and not doc.types[s].startswith("Sketcher"):
            return s
    # no link at all: these files fused each feature onto the one before it in the document
    if alone(doc, n):
        return ""
    names = list(doc.types)
    for b in reversed(names[: names.index(n)]):
        # a Part solid just before it says nothing: it was as likely a tool of some boolean
        if doc.types[b] in SOLID and not doc.types[b].startswith("Part::"):
            return b
    return ""


def tree_members(doc, chain):
    """A chain's features and, through any Part boolean under it, every chain those are made of."""
    out, todo = [], list(chain)
    while todo:
        c = todo.pop()
        if c in out:
            continue
        out.append(c)
        t = doc.types.get(c, "")
        if t in ("Part::MultiFuse", "Part::MultiCommon"):
            kids = doc.links(c, "Shapes")
        elif t in ("Part::Cut", "Part::Fuse", "Part::Common"):
            kids = [doc.link(c, "Base"), doc.link(c, "Tool")]
        else:
            kids = []
        for k in kids:
            try:
                todo += chain_of(doc, k) if doc.types.get(k, "").startswith(("PartDesign", )) else [k]
            except Unsupported:
                pass
    return out


def chain_of(doc, body):
    tip = doc.link(body, "Tip") if doc.types[body] == "PartDesign::Body" else body
    out = []
    while tip:
        if doc.types.get(tip) == "PartDesign::Body":
            # a body based on another body starts from that body's result
            tip = doc.link(tip, "Tip")
            continue
        if doc.types.get(tip) not in SOLID:
            raise Unsupported("a chain through " + str(doc.types.get(tip)))
        out.append(tip)
        tip = base_of(doc, tip) if not doc.types[tip].startswith("Part::") else ""
    return out[::-1]


def same_everywhere():
    """sin, cos and atan2 as forge_math::dmath computes them, and squares as products, in this file and what it uses
    of sketch2d and cadfs: the platform's libm rounds its own way, and interface/forge-import must match this byte for byte."""
    global math
    import types
    import cadfs
    import dmath
    m = types.SimpleNamespace(**{k: getattr(math, k) for k in dir(math) if not k.startswith("_")})
    m.sin, m.cos, m.atan2 = dmath.sin, dmath.cos, dmath.atan2
    math = sketch2d.math = cadfs.math = m
    sketch2d.sq = lambda x: x * x


def main():
    same_everywhere()
    a = sys.argv[1:]
    opt = lambda k: a[a.index(k) + 1]
    path, out = a[0], opt("--out")
    stem = os.path.basename(path).split(".")[0]
    try:
        doc = Doc(open(path, "rb").read())
    except (ET.ParseError, KeyError, ValueError, zipfile.BadZipFile, Unsupported) as x:
        print(json.dumps({"file": stem, "body": "", "status": "untranslated", "cls": "importer", "why": "unreadable: " + repr(x)[:60]}))
        return
    bodies = [n for n, t in doc.types.items() if t == "PartDesign::Body"]
    legacy = [n for n, t in doc.types.items() if t in SOLID and doc.prop(n, "BaseFeature") is None]
    if legacy:
        files = {}
        for n in legacy:
            for k in ("Shape", "AddShape", "AddSubShape"):
                p = doc.prop(n, k)
                e = p.find("Part") if p is not None else None
                if e is not None and e.get("file"):
                    files[e.get("file")] = e.get("file")
        try:
            with tempfile.NamedTemporaryFile("w", suffix=".json", dir=opt("--tmp")) as f:
                json.dump({"shapes": files, "subs": {}}, f)
                f.flush()
                p = subprocess.run([opt("--truth"), "-I", os.path.join(HERE, "fcstd_truth.py"), path, f.name], capture_output=True, text=True, timeout=600)
            doc.volumes = {k: v.get("vol") for k, v in json.loads(p.stdout).items()}
        except (subprocess.TimeoutExpired, ValueError):
            pass
    # older files: a chain of features with no body; each end of a chain is a part
    inbody = {x for b in bodies for x in doc.links(b, "Group")}
    based = {base_of(doc, n) for n, t in doc.types.items() if t in SOLID}
    # a Part solid alone is no part of this: only the PartDesign chains that start from one
    loose = [n for n, t in doc.types.items() if t in SOLID and n not in based and n not in inbody and not t.startswith("Part::")]
    bodies += loose
    for b in bodies:
        r = {"file": stem, "body": b, "status": "translated", "notes": [], "why": "", "features": []}
        t = None
        try:
            chain = chain_of(doc, b)
            if not chain:
                raise Unsupported("an empty body")
            r["features"] = [doc.types[c].split("::")[-1] for c in chain]
            tip = chain[-1]
            want = {"shapes": {tip: doc.shape_file(tip)}, "subs": {}, "wires": []}
            members = tree_members(doc, chain)
            for c in members:
                sk = doc.link(c, "Profile")
                if doc.types.get(sk) == "Part::Part2DObjectPython":
                    want["shapes"][sk] = doc.shape_file(sk)
                    want["wires"].append(sk)
            for c in members:
                if doc.types[c] in ("PartDesign::AdditivePipe", "PartDesign::SubtractivePipe"):
                    sp = doc.link(c, "Spine")
                    if doc.types.get(sp) in ("PartDesign::ShapeBinder", "Part::Helix"):
                        want["shapes"][sp] = doc.shape_file(sp)
                        want["wires"].append(sp)
                if doc.types[c] in TRANSFORMS or doc.types[c] == "PartDesign::MultiTransform":
                    for m in [c] + doc.links(c, "Transformations"):
                        for k in ("MirrorPlane", "Axis", "Direction"):
                            o = doc.link(m, k) if m in doc.props else ""
                            if doc.types.get(o) in SOLID and doc.subs(m, k):
                                want["shapes"][o] = doc.shape_file(o)
                                want["subs"].setdefault(o, []).extend(x for x in doc.subs(m, k) if x)
                if doc.types[c] in ("PartDesign::Pad", "PartDesign::Pocket") and doc.integer(c, "Type") == 3:
                    o = doc.link(c, "UpToFace")
                    if o in doc.props:
                        want["shapes"][o] = doc.shape_file(o)
                        want["subs"].setdefault(o, []).extend(x for x in doc.subs(c, "UpToFace") if x)
                if doc.types[c] == "PartDesign::Thickness":
                    o = doc.link(c, "Base")
                    if o in doc.props:
                        want["shapes"][o] = doc.shape_file(o)
                        want["subs"].setdefault(o, []).extend(x for x in doc.subs(c, "Base") if x)
                if doc.types[c] == "PartDesign::Draft":
                    for k in ("Base", "NeutralPlane", "PullDirection"):
                        o = doc.link(c, k)
                        if o in doc.props:
                            want["shapes"][o] = doc.shape_file(o)
                            want["subs"].setdefault(o, []).extend(x for x in doc.subs(c, k) if x)
                if doc.types[c] in ("PartDesign::Fillet", "PartDesign::Chamfer"):
                    base = doc.link(c, "Base")
                    want["shapes"][base] = doc.shape_file(base)
                    subs = doc.subs(c, "Base") or ["ALL"]
                    if doc.boolean(c, "UseAllEdges"):
                        subs = ["ALL"]
                    want["subs"].setdefault(base, []).extend(subs)
            with tempfile.NamedTemporaryFile("w", suffix=".json", dir=opt("--tmp")) as f:
                json.dump(want, f)
                f.flush()
                p = subprocess.run([opt("--truth"), "-I", os.path.join(HERE, "fcstd_truth.py"), path, f.name], capture_output=True, text=True, timeout=300)
            if p.returncode != 0 or not p.stdout.strip():
                raise Unresolved("the stored shapes would not read")
            truth = json.loads(p.stdout)
            r["truth"] = {k: v for k, v in truth.get(tip, {}).items() if k != "subs"}
            t = Tr(doc, b, opt("--topo"), truth)
            t.run(chain)
        except Unsupported as x:
            r.update(status="untranslated", cls="form", why=str(x))
        except Unresolved as x:
            r.update(status="untranslated", cls="reference", why=str(x))
        except KernelStop as x:
            r.update(status="stopped", why=str(x)[:200])
        except subprocess.TimeoutExpired:
            r.update(status="untranslated", cls="importer", why="stored shapes timed out")
        except (KeyError, IndexError, TypeError, ValueError, ZeroDivisionError, AttributeError, RecursionError) as x:
            r.update(status="untranslated", cls="importer", why="bug: " + repr(x)[:70])
        if t is not None:
            if t.approx:
                r["notes"].append("approx")
            if t.topo is not None:
                t.topo.close()
            if r["status"] != "untranslated":
                with open(os.path.join(out, f"{stem}.{b}.fg"), "w") as f:
                    f.write(t.prog.text(t.lines))
        print(json.dumps(r), flush=True)


if __name__ == "__main__":
    main()
