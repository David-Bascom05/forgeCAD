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
from cadfs import KernelStop, Unresolved, Unsupported, add, carries_on, dot, num, unit, vfine, vtext
from sketch2d import Arrangement

TOL = 2e-3
HERE = os.path.dirname(os.path.abspath(__file__))
SOLID = {"PartDesign::Pad", "PartDesign::Pocket", "PartDesign::Revolution", "PartDesign::Groove", "PartDesign::Fillet", "PartDesign::Chamfer", "PartDesign::Mirrored", "PartDesign::PolarPattern", "PartDesign::LinearPattern", "PartDesign::Hole", "PartDesign::Loft", "PartDesign::Pipe", "PartDesign::AdditiveLoft", "PartDesign::AdditivePipe", "PartDesign::SubtractiveLoft", "PartDesign::SubtractivePipe", "PartDesign::Draft", "PartDesign::Thickness", "PartDesign::MultiTransform", "PartDesign::Scaled", "PartDesign::AdditiveBox", "PartDesign::AdditiveCylinder", "PartDesign::Boolean", "PartDesign::Helix", "PartDesign::AdditiveHelix", "PartDesign::SubtractiveHelix", "PartDesign::FeatureBase", "PartDesign::SubtractiveBox", "PartDesign::SubtractiveCylinder", "PartDesign::AdditiveSphere", "PartDesign::SubtractiveSphere", "PartDesign::AdditiveCone", "PartDesign::SubtractiveCone", "PartDesign::AdditiveTorus", "PartDesign::SubtractiveTorus", "PartDesign::AdditivePrism", "PartDesign::SubtractivePrism", "PartDesign::AdditiveWedge", "PartDesign::SubtractiveWedge", "PartDesign::AdditiveEllipsoid", "PartDesign::SubtractiveEllipsoid", "Part::Revolution", "Part::Box", "Part::Cylinder", "Part::Cut", "Part::Fuse", "Part::Common", "Part::MultiFuse", "Part::MultiCommon", "Part::Sphere"}
TRANSFORMS = {"PartDesign::PolarPattern", "PartDesign::LinearPattern", "PartDesign::Mirrored"}
FLAT = {"Sketcher::SketchObject", "Part::Part2DObjectPython"}  # what a pad's profile can be; only a sketch has rows, the rest a stored shape
NEEDS = {"PartDesign::FeatureBase", "Part::Cut", "Part::Fuse", "Part::Common", "Part::MultiFuse", "Part::MultiCommon", "Part::Sphere", "PartDesign::Hole", "PartDesign::MultiTransform", "PartDesign::AdditiveBox", "PartDesign::SubtractiveBox", "PartDesign::AdditiveCylinder", "PartDesign::SubtractiveCylinder", "Part::Revolution", "Part::Box", "Part::Cylinder", "PartDesign::Pad", "PartDesign::Pocket", "PartDesign::Revolution", "PartDesign::Groove", "PartDesign::Fillet", "PartDesign::Chamfer", "PartDesign::Mirrored", "PartDesign::PolarPattern", "PartDesign::LinearPattern"}


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
    end open; vertices meet within Precision::Confusion. Checked on 0.18 to 0.21 files. Returns the closed wires as lists of entity ids."""
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
        self.tree = 0

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

    def profile(self, name, op, head, flip, frame_z, tail="", label="", sk=None):
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
                this = "new" if self.tree else ""
                self.bodies = 1
            k = "revolve" if tail else "extrude"
            out.append(f"{k} {this + ' ' if this else ''}{head} {frame} {tail}{loops}")
        return out, a, fr

    def pad(self, name, cut):
        d = self.doc
        for k in ("TaperAngle", "TaperAngle2", "Offset"):
            if abs(d.flt(name, k)) > 1e-12:
                raise Unsupported("a pad with " + k)
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
        along = ""
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
                along = " along " + vfine(v)
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
        head += along
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
            self.lines.append(f"combine {word} {labs[-1]} " + " ".join(labs[:-1]))
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
            if ":" not in head.split(" ")[0]:
                raise Unsupported("a PartDesign shape in a Part tree with no label")
            return head.split(":")[0]
        if t == "Part::Revolution":
            ls = self.part_revolve(name)
        elif t in ("Part::Box", "Part::Cylinder"):
            ls = self.part_solid(name, t)
        elif t == "Part::Sphere":
            if abs(d.flt(name, "Angle1", -90.0) + 90) > 1e-9 or abs(d.flt(name, "Angle2", 90.0) - 90) > 1e-9 or abs(d.flt(name, "Angle3", 360.0) - 360) > 1e-9:
                raise Unsupported("a part of a sphere")
            fr = Frame(d.placement(name))
            ls = [f"ball {num(d.flt(name, 'Radius'))} z {vtext(fr.n)} x {vtext(fr.x)} at {vtext(fr.o)}"]
        else:
            raise Unsupported("a Part tree through " + str(t))
        if len(ls) != 1:
            raise Unsupported("a Part solid of several regions")
        self.tree += 1
        lab = f"p{self.tree}"
        word, rest = ls[0].split(" ", 1)
        if self.tree > 1:
            # after the first, each its own body
            rest = rest[4:] if rest.startswith("add ") else rest
            ls = [f"{word} new {rest}"]
        self.lines.append(f"{lab}: {ls[0]}")
        return lab

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
        if d.types.get(ref) == "PartDesign::Line":
            fr = Frame(d.placement(ref))
            return fr.o, fr.n
        raise Unsupported("a pattern axis on " + (d.types.get(ref) or "nothing"))

    def transform(self, name, t, src):
        d = self.doc
        if t == "PartDesign::PolarPattern":
            o, a = self.axis_of(name, "Axis")
            if d.boolean(name, "Reversed"):
                a = tuple(-x for x in a)
            return f"pattern {src} circ {vtext(o)} {vtext(a)} {d.integer(name, 'Occurrences')} {num(d.flt(name, 'Angle', 360.0))}"
        if t == "PartDesign::LinearPattern":
            _, a = self.axis_of(name, "Direction")
            if d.boolean(name, "Reversed"):
                a = tuple(-x for x in a)
            n = d.integer(name, "Occurrences")
            return f"pattern {src} lin {vtext(a)} {n} {num(d.flt(name, 'Length') / max(n - 1, 1))}"
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
        else:
            raise Unsupported("a mirror in " + (d.types.get(ref) or "nothing"))
        return f"pattern {src} mirror z {vtext(n)} at {vtext(o)}"

    def hole(self, name):
        """One hole at each of the sketch's circles, drilled against its normal; its seat from what the file stored."""
        d = self.doc
        if d.boolean(name, "Tapered") or d.boolean(name, "ModelActualThread") or d.boolean(name, "Midplane"):
            raise Unsupported("a tapered, threaded or two-way hole")
        if d.boolean(name, "Threaded"):
            # measured: bodies with this set stored more than the plain bore; .fc records a thread, the shape is another matter
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
        if t == 0 and d.integer(name, "DrillPoint") == 1:
            ext += f" pt {num(d.flt(name, 'DrillPointAngle'))}"
        seat = ""
        if d.integer(name, "HoleCutType") != 0:
            if d.flt(name, "HoleCutDepth") > 0:
                seat = f" cb {num(d.flt(name, 'HoleCutDiameter'))} {num(d.flt(name, 'HoleCutDepth'))}"
            else:
                seat = f" cs {num(d.flt(name, 'HoleCutDiameter'))} {num(d.flt(name, 'HoleCutCountersinkAngle'))}"
        return [f"hole {num(d.flt(name, 'Diameter'))} z {vtext(z)} x {vtext(fr.x)} at {vtext(fr.world(*e['center']))}{seat} {ext}" for e in ents]

    def primitive(self, name, t):
        d = self.doc
        fr = Frame(d.placement(name))
        frame = f"z {vtext(fr.n)} x {vtext(fr.x)} at {vtext(fr.o)}"
        cut = "Subtractive" in t
        if cut and self.bodies == 0:
            raise Unsupported("first feature cuts")
        op = ("cut " if cut else "add ") if self.bodies else ""
        self.bodies = 1
        if t.endswith("Box"):
            return [f"extrude {op}{num(d.flt(name, 'Height'))} {frame} rect 0 0 {num(d.flt(name, 'Length'))} {num(d.flt(name, 'Width'))}"]
        if abs(d.flt(name, "Angle", 360.0) - 360) > 1e-9 or abs(d.flt(name, "FirstAngle")) > 1e-9 or abs(d.flt(name, "SecondAngle")) > 1e-9:
            raise Unsupported("a part of a cylinder")
        return [f"extrude {op}{num(d.flt(name, 'Height'))} {frame} circle 0 0 {num(d.flt(name, 'Radius'))}"]

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
            words = []
            if sn >= 1e-12:
                words.append(f"rot 0 0 0 {vtext(tuple(x / sn for x in q[:3]))} {num(math.degrees(2 * math.atan2(sn, q[3])))}")
            if math.sqrt(dot(by, by)) >= 1e-9:
                words.append(f"by {vtext(by)}")
            self.lines.append("movebody " + " ".join(words))
            self.steps.append((name, len(self.lines)))
            return
        if t.startswith("Part::"):
            ls, root = [], self.part_tree(name)
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
        elif t in TRANSFORMS:
            ls = [self.transform(name, t, self.originals(name))]
        elif t == "PartDesign::MultiTransform":
            ls, src = [], self.originals(name)
            for k, m in enumerate(d.links(name, "Transformations")):
                if d.types.get(m) not in TRANSFORMS:
                    raise Unsupported("a multi-transform with " + str(d.types.get(m)).split("::")[-1])
                ls.append(f"m{len(self.labels)}x{k}: " + self.transform(m, d.types[m], src))
                src += f" m{len(self.labels)}x{k}"
        elif t == "PartDesign::Hole":
            ls = self.hole(name)
        elif t in ("PartDesign::AdditiveBox", "PartDesign::SubtractiveBox", "PartDesign::AdditiveCylinder", "PartDesign::SubtractiveCylinder"):
            ls = self.primitive(name, t)
        if t.startswith("Part::"):
            self.labels[name] = root
            self.bodies = 1
        elif t == "PartDesign::MultiTransform":
            self.labels[name] = ls[-1].split(":")[0]  # already labelled, each its own
        else:
            # a feature of several lines (regions that do not touch): a pattern of it needs them all
            base = f"t{len(self.labels)}"
            names = [base if k == 0 else f"{base}x{k}" for k in range(len(ls))]
            self.labels[name] = " ".join(names)
            ls = [f"{n}: {l}" for n, l in zip(names, ls)]
        self.lines += ls
        self.steps.append((name, len(self.lines)))


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
                if carries_on(e, k, g, j):
                    picked[g["h"]] = g
                    todo.append(g)


def doc_approx(d, name):
    sk = d.link(name, "Profile")
    p = d.prop(sk, "Geometry") if sk in d.props else None
    return p is not None and any(g.get("type") == "Part::GeomBSplineCurve" for g in p.find("GeometryList").findall("Geometry"))


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
