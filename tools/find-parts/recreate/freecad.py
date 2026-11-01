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
SOLID = {"PartDesign::Pad", "PartDesign::Pocket", "PartDesign::Revolution", "PartDesign::Groove", "PartDesign::Fillet", "PartDesign::Chamfer", "PartDesign::Mirrored", "PartDesign::PolarPattern", "PartDesign::LinearPattern", "PartDesign::Hole", "PartDesign::Loft", "PartDesign::Pipe", "PartDesign::AdditiveLoft", "PartDesign::AdditivePipe", "PartDesign::SubtractiveLoft", "PartDesign::SubtractivePipe", "PartDesign::Draft", "PartDesign::Thickness", "PartDesign::MultiTransform", "PartDesign::Scaled", "PartDesign::AdditiveBox", "PartDesign::AdditiveCylinder", "PartDesign::Boolean", "PartDesign::Helix", "PartDesign::AdditiveHelix", "PartDesign::SubtractiveHelix", "PartDesign::FeatureBase", "PartDesign::SubtractiveBox", "PartDesign::SubtractiveCylinder", "PartDesign::AdditiveSphere", "PartDesign::SubtractiveSphere", "PartDesign::AdditiveCone", "PartDesign::SubtractiveCone", "PartDesign::AdditiveTorus", "PartDesign::SubtractiveTorus", "PartDesign::AdditivePrism", "PartDesign::SubtractivePrism", "PartDesign::AdditiveWedge", "PartDesign::SubtractiveWedge", "PartDesign::AdditiveEllipsoid", "PartDesign::SubtractiveEllipsoid"}
NEEDS = {"PartDesign::Pad", "PartDesign::Pocket", "PartDesign::Revolution", "PartDesign::Groove", "PartDesign::Fillet", "PartDesign::Chamfer", "PartDesign::Mirrored", "PartDesign::PolarPattern", "PartDesign::LinearPattern"}


class Doc:
    def __init__(self, data):
        self.z = zipfile.ZipFile(io.BytesIO(data))
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


def even_regions(a):
    """FreeCAD's bullseye: regions at even nesting are solid, odd ones holes."""
    out = []
    for i, f in enumerate(a.faces):
        pt = sketch2d.point_within(a.loops[f["outer"]], a)
        depth = sum(1 for j, g in enumerate(a.faces) if j != i and g["area"] > f["area"] and sketch2d.inside(pt, a.loops[g["outer"]], a))
        if depth % 2 == 0:
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

    def profile(self, name, op, head, flip, frame_z, tail="", label=""):
        sk = self.doc.link(name, "Profile")
        if self.doc.types.get(sk) != "Sketcher::SketchObject":
            raise Unsupported("a profile not from a sketch")
        fr = Frame(self.doc.placement(sk))
        a = Arrangement(sketch_ents(self.doc, sk))
        regs = even_regions(a)
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
        if d.types.get(sk) != "Sketcher::SketchObject":
            raise Unsupported("a profile not from a sketch")
        n = Frame(d.placement(sk)).n
        rev = d.boolean(name, "Reversed")
        down = cut != rev  # a pocket runs against the sketch normal
        z = tuple(-x for x in n) if down else n
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
        if d.types.get(sk) != "Sketcher::SketchObject":
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
            if t in ("PartDesign::Pad", "PartDesign::Pocket"):
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
        s = doc.link(sk, "Support")
        if doc.types.get(s) in SOLID:
            return s
    # no link at all: these files fused each feature onto the one before it in the document
    names = list(doc.types)
    for b in reversed(names[: names.index(n)]):
        if doc.types[b] in SOLID:
            return b
    return ""


def chain_of(doc, body):
    tip = doc.link(body, "Tip") if doc.types[body] == "PartDesign::Body" else body
    out = []
    while tip:
        if doc.types.get(tip) not in SOLID:
            raise Unsupported("a chain through " + str(doc.types.get(tip)))
        out.append(tip)
        tip = base_of(doc, tip)
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
    # older files: a chain of features with no body; each end of a chain is a part
    inbody = {x for b in bodies for x in doc.links(b, "Group")}
    based = {base_of(doc, n) for n, t in doc.types.items() if t in SOLID}
    loose = [n for n, t in doc.types.items() if t in SOLID and n not in based and n not in inbody]
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
            want = {"shapes": {tip: doc.shape_file(tip)}, "subs": {}}
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
