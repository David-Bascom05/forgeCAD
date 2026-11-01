#!/usr/bin/env python3
"""FreeCAD library PartDesign bodies to .fc. A file is read as data (Document.xml as XML, shapes by OpenCascade
in fcstd_truth.py); FreeCAD is never run and macros or scripts in the file are never touched. Each object stores
its shape, so a fillet's EdgeN is sampled from its base's stored shape and found on our part by position, and
every step has a truth. Keep the data outside the repo; run with python3 -I.

  freecad.py FILE.FCStd --topo EXE --truth VENVPY --out DIR      one .fc and result JSON line per body
"""
import io
import json
import math
import os
import resource
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sketch2d
from cadfs import KernelStop, Unresolved, Unsupported, add, dot, num, vtext
from sketch2d import Arrangement

TOL = 2e-3
HERE = os.path.dirname(os.path.abspath(__file__))
SOLID = {"PartDesign::Pad", "PartDesign::Pocket", "PartDesign::Revolution", "PartDesign::Groove", "PartDesign::Fillet", "PartDesign::Chamfer", "PartDesign::Mirrored", "PartDesign::PolarPattern", "PartDesign::LinearPattern", "PartDesign::Hole", "PartDesign::Loft", "PartDesign::Pipe", "PartDesign::AdditiveLoft", "PartDesign::AdditivePipe", "PartDesign::SubtractiveLoft", "PartDesign::SubtractivePipe", "PartDesign::Draft", "PartDesign::Thickness", "PartDesign::MultiTransform", "PartDesign::Scaled", "PartDesign::AdditiveBox", "PartDesign::AdditiveCylinder", "PartDesign::Boolean", "PartDesign::Helix", "PartDesign::AdditiveHelix", "PartDesign::SubtractiveHelix", "PartDesign::FeatureBase", "PartDesign::SubtractiveBox", "PartDesign::SubtractiveCylinder", "PartDesign::AdditiveSphere", "PartDesign::SubtractiveSphere", "PartDesign::AdditiveCone", "PartDesign::SubtractiveCone", "PartDesign::AdditiveTorus", "PartDesign::SubtractiveTorus", "PartDesign::AdditivePrism", "PartDesign::SubtractivePrism", "PartDesign::AdditiveWedge", "PartDesign::SubtractiveWedge", "PartDesign::AdditiveEllipsoid", "PartDesign::SubtractiveEllipsoid", "Part::Revolution", "Part::Box", "Part::Cylinder"}
FLAT = {"Sketcher::SketchObject", "Part::Part2DObjectPython"}  # what a pad's profile can be; only a sketch has rows, the rest a stored shape
NEEDS = {"Part::Revolution", "Part::Box", "Part::Cylinder", "PartDesign::Pad", "PartDesign::Pocket", "PartDesign::Revolution", "PartDesign::Groove", "PartDesign::Fillet", "PartDesign::Chamfer", "PartDesign::Mirrored", "PartDesign::PolarPattern", "PartDesign::LinearPattern"}


class Doc:
    def __init__(self, data):
        self.z = zipfile.ZipFile(io.BytesIO(data))
        self.volumes = None
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
        elif t == "Part::GeomBSplineCurve":
            e = g.find("BSplineCurve")
            if e.get("IsPeriodic") == "1":
                raise Unsupported("a periodic spline")
            ents.append({"kind": "polyline", "id": f"e{k}", "points": bspline_points(e), "construction": con})
        elif t == "Part::GeomPoint":
            continue
        elif not con:
            raise Unsupported("sketch curve " + t.split("::")[-1])
    return ents


def bspline_points(e):
    """A fine polyline along a non-periodic NURBS: the part is then marked approx."""
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
    return [at(u) for u in us] + [at(hi)]


def closed_wires(ents):
    """FreeCAD's Sketch::toShape: from the first edge left, pull in the first edge in document order that meets the wire's
    last or first vertex, until nothing meets or the wire closes. So at a junction the order decides, and a wire can
    end open; vertices meet within Precision::Confusion. Returns the closed wires as lists of entity ids."""
    conf = 1e-7
    same = lambda p, q: math.dist(p, q) <= conf
    left = []
    keep = []
    for e in ents:
        if e.get("construction"):
            continue
        if e["kind"] == "skCircle":
            keep.append([e["id"]])
        elif e["kind"] == "polyline":
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
        self.lines = []
        self.labels = {}
        self.topo = None
        self.bodies = 0
        self.approx = False
        self.steps = []

    def state(self):
        if self.topo is None:
            def lim():
                os.nice(19)
                resource.setrlimit(resource.RLIMIT_AS, (3 << 30, 3 << 30))
                resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
                resource.setrlimit(resource.RLIMIT_CPU, (120, 121))
            self.topo = subprocess.Popen([self.topo_exe], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, preexec_fn=lim)
        src = ("\n".join(self.lines) + "\n").encode()
        self.topo.stdin.write(b"%d\n" % len(src) + src)
        self.topo.stdin.flush()
        line = self.topo.stdout.readline()
        if not line:
            raise KernelStop("kernel process died (crash, memory or CPU limit)")
        d = json.loads(line)
        if "panic" in d:
            raise KernelStop("kernel panic")
        if d.get("err"):
            raise KernelStop(d["err"])
        return d

    def profile_ents(self, sk, fr):
        if self.doc.types.get(sk) == "Sketcher::SketchObject":
            return sketch_ents(self.doc, sk)
        ws = self.truth.get(sk, {}).get("wires")
        if ws is None:
            raise Unresolved("a profile whose stored wires would not read")
        ents = []
        for w in ws:
            for e in w:
                uv = [list(fr.uvw(q)[:2]) for q in e["p"]]
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

    def profile(self, name, op, head, flip, frame_z, tail="", label="", sk=None):
        sk = sk or self.doc.link(name, "Profile")
        if self.doc.types.get(sk) not in FLAT:
            raise Unsupported("a profile not from a sketch")
        fr = Frame(self.doc.placement(sk))
        ents = self.profile_ents(sk, fr)
        wires = closed_wires(ents)
        wired = {i for w in wires for i in w}
        a = Arrangement([e for e in ents if e["id"] in wired or e.get("construction")])
        regs = wire_regions(a, ents, wires)
        if not regs:
            raise Unresolved("a sketch with no closed region")
        frame = f"z {vtext(frame_z)} x {vtext(fr.x)} at {vtext(fr.o)}"
        out = []
        from cadfs import Tr as C
        loop_text = C.loop_text
        for prof in a.merged(regs):
            loops = " ".join(loop_text(self, a, lp, flip) for lp in prof)
            this = op
            if self.bodies == 0:
                if op == "cut":
                    raise Unsupported("first feature cuts")
                this = ""
                self.bodies = 1
            k = "revolve" if tail else "extrude"
            out.append(f"{k} {this + ' ' if this else ''}{head} {frame} {tail}{loops}")
        return out, a, fr

    def pad(self, name, cut):
        d = self.doc
        for k in ("TaperAngle", "TaperAngle2", "Offset"):
            if abs(d.flt(name, k)) > 1e-12:
                raise Unsupported("a pad with " + k)
        if d.boolean(name, "UseCustomVector") or d.prop(name, "AlongSketchNormal") is not None and not d.boolean(name, "AlongSketchNormal"):
            raise Unsupported("a pad along a custom direction")
        sk = d.link(name, "Profile")
        if d.types.get(sk) not in FLAT:
            raise Unsupported("a profile not from a sketch")
        n = Frame(d.placement(sk)).n
        rev = d.boolean(name, "Reversed")
        down = cut != rev  # a pocket runs against the sketch normal
        z = tuple(-x for x in n) if down else n
        stored = d.prop(name, "Direction")
        if d.types.get(sk) != "Sketcher::SketchObject" and stored is not None and stored.find("PropertyVector") is not None:
            # a shape's plane may face either way; the feature records which way it went
            e = stored.find("PropertyVector")
            z = tuple(float(e.get(k)) for k in ("valueX", "valueY", "valueZ"))
            down = dot(z, n) < 0
        t, L, L2 = d.integer(name, "Type"), d.flt(name, "Length"), d.flt(name, "Length2")
        mid = d.boolean(name, "Midplane")
        if t == 0:
            head = f"sym {num(L)}" if mid else num(L)
        elif t == 4:
            head = f"two {num(L)} {num(L2)}"
        elif t == 1 and cut:
            head = "thru both" if mid else "thru"
        elif t == 2:
            head = "next"
        else:
            raise Unsupported(f"a pad or pocket of type {t}")
        return self.profile(name, "cut" if cut else "add", head, down, z)[0]

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
        head = "" if abs(ang - 360) < 1e-9 else (f"sym {num(ang)}" if d.boolean(name, "Midplane") else num(ang))
        tail = f"axis {num(u)} {num(v)} {num(du)} {num(dv)} "
        return self.profile(name, "cut" if cut else "add", head, False, fr.n, tail)[0]

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
        head = "" if abs(ang - 360) < 1e-9 else num(ang)
        if d.boolean(name, "Symmetric"):
            head = "sym " + head if head else head
        tail = f"axis {num(u)} {num(v)} {num(du)} {num(dv)} "
        return self.profile(name, "add", head, False, fr.n, tail, sk=sk)[0]

    def part_solid(self, name, t):
        """Part workbench primitives, on their placement. An add: only ever the first in a chain here."""
        d = self.doc
        fr = Frame(d.placement(name))
        frame = f"z {vtext(fr.n)} x {vtext(fr.x)} at {vtext(fr.o)}"
        self.bodies = 1
        if t == "Part::Box":
            return [f"extrude {num(d.flt(name, 'Height'))} {frame} rect 0 0 {num(d.flt(name, 'Length'))} {num(d.flt(name, 'Width'))}"]
        if abs(d.flt(name, "Angle", 360.0) - 360) > 1e-9:
            raise Unsupported("a part of a cylinder")
        return [f"extrude {num(d.flt(name, 'Height'))} {frame} circle 0 0 {num(d.flt(name, 'Radius'))}"]

    def blend(self, name, line, all_edges=False):
        d = self.doc
        base = d.link(name, "Base")
        subs = ["ALL"] if all_edges else d.subs(name, "Base")
        subs = subs or ["ALL"]
        st = self.state()
        data = self.truth.get(base, {}).get("subs", {})
        picked = {}
        for sub in subs:
            if sub not in data:
                raise Unresolved("a blend on " + sub[:4] + " not sampled")
            for poly in data[sub]:
                if self.approx and not line_or_circle(poly):
                    # the part's spline is a polyline here, so its edge is many short ones the blend cannot round as one
                    raise Unsupported("a blend along a spline")
                # one of ours can span several of theirs (a seam, a split circle): its middle stretch decides
                lo = [min(q[k] for q in poly) - TOL for k in range(3)]
                hi = [max(q[k] for q in poly) + TOL for k in range(3)]
                inbox = lambda p: all(lo[k] <= p[k] <= hi[k] for k in range(3))
                hits = [e for e in st.get("edges", []) if any(inbox(p) and near(p, poly) <= TOL for p in e["p"][4:13:4])]
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
        return [line + " " + " ".join(sorted(picked))]

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
        raise Unsupported("a pattern axis on " + (d.types.get(ref) or "nothing"))

    def fail(self, what):
        raise Unsupported("a pattern reference " + what)

    def originals(self, name):
        out = []
        for o in self.doc.links(name, "Originals"):
            if o not in self.labels:
                raise Unresolved("a pattern of a feature outside the chain")
            out.append(self.labels[o])
        return " ".join(out)

    def run(self, chain):
        d = self.doc
        for name in chain:
            t = d.types[name]
            if t not in NEEDS:
                raise Unsupported("feature " + t.split("::")[-1])
            if doc_approx(d, name):
                self.approx = True
        for name in chain:
            t = d.types[name]
            if t == "Part::Revolution":
                ls = self.part_revolve(name)
            elif t in ("Part::Box", "Part::Cylinder"):
                ls = self.part_solid(name, t)
            elif t in ("PartDesign::Pad", "PartDesign::Pocket"):
                ls = self.pad(name, t.endswith("Pocket"))
            elif t in ("PartDesign::Revolution", "PartDesign::Groove"):
                ls = self.revolve(name, t.endswith("Groove"))
            elif t == "PartDesign::Fillet":
                ls = self.blend(name, f"fillet {num(d.flt(name, 'Radius'))}", d.boolean(name, "UseAllEdges"))
            elif t == "PartDesign::Chamfer":
                if d.integer(name, "ChamferType") != 0:
                    raise Unsupported("an unequal chamfer")
                ls = self.blend(name, f"chamfer {num(d.flt(name, 'Size'))}", d.boolean(name, "UseAllEdges"))
            elif t == "PartDesign::PolarPattern":
                o, a = self.axis_of(name, "Axis")
                if d.boolean(name, "Reversed"):
                    a = tuple(-x for x in a)
                ls = [f"pattern {self.originals(name)} circ {vtext(o)} {vtext(a)} {d.integer(name, 'Occurrences')} {num(d.flt(name, 'Angle', 360.0))}"]
            elif t == "PartDesign::LinearPattern":
                _, a = self.axis_of(name, "Direction")
                if d.boolean(name, "Reversed"):
                    a = tuple(-x for x in a)
                n = d.integer(name, "Occurrences")
                L = d.flt(name, "Length")
                ls = [f"pattern {self.originals(name)} lin {vtext(a)} {n} {num(L / max(n - 1, 1))}"]
            elif t == "PartDesign::Mirrored":
                ref, subs = d.link(name, "MirrorPlane"), d.subs(name, "MirrorPlane")
                if d.types.get(ref) == "Sketcher::SketchObject":
                    fr = Frame(d.placement(ref))
                    n = {"H_Axis": fr.y, "V_Axis": fr.x, "": fr.n}.get(subs[0] if subs else "") or self.fail("mirror")
                    o = fr.o
                elif d.types.get(ref) == "App::Plane":
                    role = d.prop(ref, "Role").find("String").get("value")
                    n = {"XY_Plane": (0.0, 0.0, 1.0), "XZ_Plane": (0.0, 1.0, 0.0), "YZ_Plane": (1.0, 0.0, 0.0)}.get(role) or self.fail("mirror")
                    o = (0.0, 0.0, 0.0)
                else:
                    raise Unsupported("a mirror in " + (d.types.get(ref) or "nothing"))
                ls = [f"pattern {self.originals(name)} mirror z {vtext(n)} at {vtext(o)}"]
            label = f"t{len(self.labels)}"
            self.labels[name] = label
            ls[0] = f"{label}: {ls[0]}"
            self.lines += ls
            self.steps.append((name, len(self.lines)))


def line_or_circle(poly):
    """Do these samples lie on one line or one circle?"""
    a, m, b = poly[0], poly[len(poly) // 3], poly[2 * len(poly) // 3]
    u, v = add(m, a, -1), add(b, a, -1)
    cr = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
    nn = dot(cr, cr)
    if nn < 1e-18:
        return all(math.sqrt(max(dot(add(q, a, -1), add(q, a, -1)) - dot(add(q, a, -1), v) ** 2 / max(dot(v, v), 1e-18), 0.0)) < 1e-6 for q in poly)
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
                a, b = t, g["t"][j]
                cr = (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])
                if math.sqrt(dot(cr, cr)) < 1e-4:
                    picked[g["h"]] = g
                    todo.append(g)


def doc_approx(d, name):
    sk = d.link(name, "Profile")
    p = d.prop(sk, "Geometry") if sk in d.props else None
    return p is not None and any(g.get("type") == "Part::GeomBSplineCurve" for g in p.find("GeometryList").findall("Geometry"))


def near(p, poly):
    best = math.inf
    for a, b in zip(poly, poly[1:]):
        e = add(b, a, -1)
        l2 = dot(e, e)
        t = 0.0 if l2 == 0 else max(0.0, min(1.0, dot(add(p, a, -1), e) / l2))
        q = add(a, e, t)
        best = min(best, math.dist(p, q))
        if best <= TOL:
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
        return ""
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
        if doc.types[b] in SOLID:
            return b
    return ""


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


def main():
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
            for c in chain:
                sk = doc.link(c, "Profile")
                if doc.types.get(sk) == "Part::Part2DObjectPython":
                    want["shapes"][sk] = doc.shape_file(sk)
                    want["wires"].append(sk)
            for c in chain:
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
                t.topo.kill()
                t.topo.wait()
            if r["status"] != "untranslated":
                with open(os.path.join(out, f"{stem}.{b}.fc"), "w") as f:
                    f.write("\n".join(t.lines) + "\n")
        print(json.dumps(r), flush=True)


if __name__ == "__main__":
    main()
