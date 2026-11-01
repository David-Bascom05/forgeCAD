#!/usr/bin/env python3
"""CADFS (Onshape FeatureScript) to .fc. Symbolic queries are resolved against our own geometry: a query
becomes the curve or surface its entity must lie on (a carrier, from the sketches and extents above it), and
it picks the edges and faces of the part as built so far whose samples lie on that carrier. A sketch's
frame comes from the dataset's sketch record. Data stays outside the repo; run with python3 -I.

  cadfs.py PROGRAM.txt SKETCH.json --topo EXE --out OUT.fc     prints the outcome as JSON

status: "translated", or "untranslated" with why (an importer gap, a form .fc lacks, an unresolvable
reference), or "stopped": the kernel itself stopped on a prefix, which OUT.fc then holds.
"""
import json
import math
import os
import resource
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fs
import sketch2d
from sketch2d import Arrangement, Unsupported, curve_dist, meets

TOL = 2e-4  # mm: a sample this close lies on its carrier
SIDE = 1  # TD weight +1: the region on this side of the entity (+1 left)
SUPPORTED = {"sweep", "loft", "sketch", "extrude", "revolve", "fillet", "chamfer", "shell", "cPlane", "cPoint", "assignVariable", "hole", "mirror", "circularPattern", "transform", "booleanBodies"}
PLANES = {"Front": ((0.0, -1.0, 0.0), (1.0, 0.0, 0.0)), "Top": ((0.0, 0.0, 1.0), (1.0, 0.0, 0.0)), "Right": ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0))}


class Unresolved(Exception):
    pass


class KernelStop(Exception):
    pass


def num(x):
    x = round(x, 10)
    if x == 0:
        x = 0.0
    s = f"{x:.10f}".rstrip("0").rstrip(".")
    return s if s not in ("", "-0") else "0"


def add(a, b, s=1.0):
    return tuple(x + y * s for x, y in zip(a, b))


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def unit(a):
    l = math.sqrt(dot(a, a))
    return tuple(x / l for x in a)


def fine(x):
    """As many digits as the kernel's tangency check can see: num()'s ten move a frame by more than ANGULAR."""
    s = f"{x:.15f}".rstrip("0").rstrip(".")
    return s if s not in ("", "-0") else "0"


def vfine(v):
    return " ".join(fine(x) for x in v)


def turned(v, axis, ang):
    """v turned about a unit axis by ang radians."""
    c, s = math.cos(ang), math.sin(ang)
    k = dot(axis, v) * (1 - c)
    w = cross(axis, v)
    return tuple(v[i] * c + w[i] * s + axis[i] * k for i in range(3))


def vtext(v):
    return " ".join(num(x) for x in v)


class Frame:
    def __init__(self, m):
        self.x, self.y, self.n = (m[0], m[4], m[8]), (m[1], m[5], m[9]), (m[2], m[6], m[10])
        self.o = (m[3] * 1000, m[7] * 1000, m[11] * 1000)

    def uvw(self, p):
        d = add(p, self.o, -1)
        return dot(d, self.x), dot(d, self.y), dot(d, self.n)

    def world(self, u, v, w=0.0):
        return add(add(add(self.o, self.x, u), self.y, v), self.n, w)

    @staticmethod
    def of(o, x, y, n):
        f = Frame.__new__(Frame)
        f.o, f.x, f.y, f.n = o, x, y, n
        return f


# carriers: dist(p), how far a point is from where the query's entity must lie
class OnPlane:
    def __init__(self, o, n):
        self.o, self.n = o, n

    def dist(self, p):
        return abs(dot(add(p, self.o, -1), self.n))


class Swept:
    """A sketch curve (or point) moved along the sketch normal over w in [lo, hi]; lo = hi: a curve at a cap."""

    def __init__(self, fr, piece, lo, hi):
        self.fr, self.piece, self.lo, self.hi = fr, piece, lo, hi

    def dist(self, p):
        u, v, w = self.fr.uvw(p)
        d = curve_dist(self.piece, (u, v))
        out = 0.0
        if self.lo is not None and w < self.lo:
            out = self.lo - w
        if self.hi is not None and w > self.hi:
            out = w - self.hi
        return max(d, out)


class Turned:
    """A sketch curve (or point) turned about an axis in its plane."""

    def __init__(self, fr, piece, a2, d2):
        self.fr, self.piece, self.a2, self.d2 = fr, piece, a2, d2

    def dist(self, p):
        u, v, w = self.fr.uvw(p)
        # the point's axial place and radius in world, back into the sketch's plane on either side
        dx, dy = u - self.a2[0], v - self.a2[1]
        s = dx * self.d2[0] + dy * self.d2[1]
        r = math.hypot(dx - s * self.d2[0], dy - s * self.d2[1])
        r = math.hypot(r, w)
        e = (-self.d2[1], self.d2[0])
        return min(curve_dist(self.piece, (self.a2[0] + s * self.d2[0] + k * r * e[0], self.a2[1] + s * self.d2[1] + k * r * e[1])) for k in (1, -1))


class Any:
    def __init__(self, cs):
        self.cs = cs

    def dist(self, p):
        return min(c.dist(p) for c in self.cs)


class Both:
    def __init__(self, a, b):
        self.a, self.b = a, b

    def dist(self, p):
        return max(self.a.dist(p), self.b.dist(p))


class LoftWall:
    """A loft's wall by what it touches: the face beside an entity's edge in a section, not that section's own."""

    def __init__(self, curves, planes):
        self.curves, self.planes = curves, planes

    def dist(self, p):
        return math.inf

    def faces_in(self, st):
        caps = set()
        for f in st.get("faces", []):
            if f["n"] and any(all(abs(fr.uvw(q)[2]) < 1e-5 for q in f["p"]) for fr in self.planes):
                caps.add(f["h"])
        out = set()
        for e in st.get("edges", []):
            if any(all(c.dist(q) <= TOL for q in e["p"]) for c in self.curves):
                out |= set(e["f"]) - caps
        return out

    def edges_in(self, st):
        fs_ = self.faces_in(st)
        return {e["h"] for e in st.get("edges", []) if fs_ & set(e["f"])}


class LoftRail:
    """A loft's edge between sections: from a point named in one section to one in the next."""

    def __init__(self, per):
        self.per = per

    def dist(self, p):
        return math.inf

    def faces_in(self, st):
        return set()

    def edges_in(self, st):
        out = set()
        for e in st.get("edges", []):
            a, b = e["p"][0], e["p"][-1]
            for k in range(len(self.per) - 1):
                for u, v in ((a, b), (b, a)):
                    if any(near3(u, q) for q in self.per[k]) and any(near3(v, q) for q in self.per[k + 1]):
                        out.add(e["h"])
        return out


def picks(c):
    """Does this carrier hold a loft's wall or rail, which go by what they touch and have no distance?"""
    if isinstance(c, Any):
        return any(picks(x) for x in c.cs)
    if isinstance(c, Both):
        return picks(c.a) or picks(c.b)
    return hasattr(c, "edges_in")


def on_edges(c, st):
    """The edges a carrier picks. One edge can run along several curves of an Any, so distance decides where it can."""
    if not picks(c):
        return {e["h"] for e in st.get("edges", []) if all(c.dist(p) <= TOL for p in e["p"])}
    if isinstance(c, Any):
        return set().union(*(on_edges(x, st) for x in c.cs))
    if isinstance(c, Both):
        return on_edges(c.a, st) & on_edges(c.b, st)
    if hasattr(c, "edges_in"):
        return c.edges_in(st)
    return {e["h"] for e in st.get("edges", []) if all(c.dist(p) <= TOL for p in e["p"])}


def on_faces(c, st):
    """The faces a carrier picks."""
    if not picks(c):
        return {f["h"] for f in st.get("faces", []) if all(c.dist(p) <= TOL for p in f["p"])}
    if isinstance(c, Any):
        return set().union(*(on_faces(x, st) for x in c.cs))
    if isinstance(c, Both):
        return on_faces(c.a, st) & on_faces(c.b, st)
    if hasattr(c, "faces_in"):
        return c.faces_in(st)
    return {f["h"] for f in st.get("faces", []) if all(c.dist(p) <= TOL for p in f["p"])}


def leg_carrier(fr, p, piece):
    """Where a sketch curve or point goes along one stretch of a sweep, `fr` the section at the stretch's start."""
    if p.c is None:
        l = dot(add(p.b, p.a, -1), fr.n)
        return Swept(fr, piece, *span(0.0, l))
    u, v, _ = fr.uvw(p.c)
    return Turned(fr, piece, (u, v), (dot(p.z, fr.x), dot(p.z, fr.y)))


def end_point(a, name):
    """A sketch's `E.start`, `E.end` or `E.center` as a point."""
    base, _, w = name.rpartition(".")
    e = a.every.get(base)
    if e is None:
        return None
    q = {"start": e.a, "end": e.b, "center": e.c}.get(w)
    return None if q is None else sketch2d.Piece(name, "point", q, q)


class WPiece:
    """A stretch of a path in the world: a line a to b, or an arc about c in the plane with normal z from a by
    `sweep` radians (positive about z)."""

    def __init__(self, a, b, c=None, z=None, sweep=0.0, ent=None):
        self.a, self.b, self.c, self.z, self.sweep, self.ent = a, b, c, z, sweep, ent

    def tangent(self, p):
        if self.c is None:
            return unit(add(self.b, self.a, -1))
        t = unit(cross(self.z, add(p, self.c, -1)))
        return t if self.sweep > 0 else tuple(-x for x in t)

    def rev(self):
        if self.c is None:
            return WPiece(self.b, self.a, ent=self.ent)
        return WPiece(self.b, self.a, self.c, self.z, -self.sweep, self.ent)

    def text(self):
        if self.c is None:
            return f"line {vfine(self.a)} {vfine(self.b)}"
        r = add(self.a, self.c, -1)
        l = math.sqrt(dot(r, r))
        z = self.z if self.sweep > 0 else tuple(-x for x in self.z)
        return f"arc z {vfine(z)} x {vfine(tuple(v / l for v in r))} at {vfine(self.c)} {fine(l)} 0 {fine(math.degrees(abs(self.sweep)))}"


class Tr:
    """State of one translation."""

    def __init__(self, text, raw, topo):
        self.feats = fs.program(text)
        self.raw = raw
        self.topo_exe = topo
        self.topo = None
        self.lines = []
        self.notes = set()
        self.sk = {}  # sketch id: (Arrangement, Frame)
        self.solid = {}  # feature id: its extent record
        self.bodies = 0
        self.vars = {}
        self.kinds = []
        self.planes = {}
        self.names = 0  # bodies get labels b1, b2..
        self.current = None  # the body the next unscoped line acts on
        self.alias = {}
        self.used = set()
        self.sketch_ents = {}
        self.plain = {}
        self.imprint_off = set()
        self.copies = {}

    def val(self, v):
        if isinstance(v, float):
            return v
        if isinstance(v, fs.Call) and v[0] == "getVariable" and v[1][1] in self.vars:
            return self.val(self.vars[v[1][1]])
        raise Unsupported("an expression in a value")

    # the kernel at this point in the part
    def state(self):
        if self.topo is None:
            def lim():
                os.nice(19)
                resource.setrlimit(resource.RLIMIT_AS, (3 << 30, 3 << 30))
                resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
                resource.setrlimit(resource.RLIMIT_CPU, (120, 121))
            self.topo = subprocess.Popen([self.topo_exe], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, preexec_fn=lim, text=False)
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

    def sketch(self, f):
        a = Arrangement(f["ents"])
        fr = self.frame_of(f, a)
        self.sk[f["id"]] = (a, fr)
        self.sketch_ents[f["id"]] = f["ents"]
        if a.approx:
            self.notes.add("approx")

    def frame_of(self, f, a):
        # the dataset's record: the unused sketch sharing most of these points and radii, the first if tied
        ours = set()
        for e in a.every.values():
            for q in (e.a, e.b, e.c):
                if q is not None:
                    ours.add((round(q[0], 3), round(q[1], 3)))
            if e.r:
                ours.add(round(e.r, 3))
        # the plane it was drawn on rules out look-alike sketches
        try:
            plane = self.carrier(single(f["args"]["sketchPlane"], f["env"]), f["env"])
        except (Unresolved, KeyError, Unsupported):
            plane = None
        def choose(on_plane):
            best, score = None, 0
            for k, s in enumerate(self.raw.get("sketches", [])):
                if k in self.used or "sketchMatrix" not in s:
                    continue
                if on_plane and isinstance(plane, OnPlane):
                    fr = Frame(s["sketchMatrix"])
                    if plane.dist(fr.o) > 1e-3 or abs(abs(dot(fr.n, plane.n)) - 1) > 1e-6:
                        continue
                theirs = set()
                todo = list(s.get("entities", []))
                while todo:
                    x = todo.pop()
                    if isinstance(x, dict):
                        if isinstance(x.get("x"), float) and isinstance(x.get("y"), float):
                            theirs.add((round(x["x"] * 1000, 3), round(x["y"] * 1000, 3)))
                        if isinstance(x.get("radius"), float):
                            theirs.add(round(x["radius"] * 1000, 3))
                        todo += x.values()
                    elif isinstance(x, list):
                        todo += x
                n = len(ours & theirs)
                if n > score:
                    best, score = k, n
            return best
        best = choose(True)
        if best is None:
            best = choose(False)
        if best is None or "sketchMatrix" not in self.raw["sketches"][best]:
            raise Unresolved("no sketch record")
        self.used.add(best)
        return Frame(self.raw["sketches"][best]["sketchMatrix"])

    # queries as carriers
    def ent(self, q):
        if not (isinstance(q, fs.Call) and q[0] == "sQuery"):
            raise Unresolved("not a sketch entity: " + str(q[0] if isinstance(q, tuple) else q))
        fid = q[1][0][1].split(".")[0]
        if fid not in self.sk:
            raise Unresolved("entity of an unread sketch")
        a, fr = self.sk[fid]
        e = a.every.get(q[1][2])
        if e is None:
            e = end_point(a, q[1][2])
        if e is None:
            raise Unresolved("unknown sketch entity")
        return fid, a, fr, e

    def deref(self, q, env):
        while isinstance(q, tuple) and q and q[0] == "name":
            if q[1] not in env:
                raise Unresolved("unknown variable " + q[1])
            q = env[q[1]]
        return q

    def osd(self, m, env):
        out = []
        for d in m.get("disambiguationData", []):
            if isinstance(d, fs.Call) and d[0] == "OSD":
                out += [self.deref(x, env) for x in d[1][0]]
        return out

    def carrier(self, q, env):
        q = self.deref(q, env)
        if not isinstance(q, fs.Call):
            raise Unresolved("query form")
        if q[0] == "sQuery":
            fid, a, fr, e = self.ent(q)
            return Swept(fr, e, 0.0, 0.0)
        if q[0] == "qCreatedBy":
            w = q[1][0]
            if isinstance(w, fs.Call) and w[0] == "makeId" and w[1][0].split(".")[0] in PLANES:
                return OnPlane((0.0, 0.0, 0.0), PLANES[w[1][0].split(".")[0]][0])
            if isinstance(w, tuple) and w[0] == "id" and w[1].split(".")[0] in self.planes:
                return self.planes[w[1].split(".")[0]]
            raise Unresolved("qCreatedBy")
        if q[0] != "makeQuery":
            raise Unresolved(q[0])
        fid = q[1][0][1]
        kind, m = q[1][1], q[1][3] if len(q[1]) > 3 else {}
        base = fid.split(".")[0]
        if kind in ("COPY", "MERGE", "SPLIT", "INTERSECT"):
            if ".opPattern" in fid or ".opTransform" in fid:
                raise Unresolved("a pattern's copy")
            parents = m.get("derivedFrom")
            parents = parents if isinstance(parents, list) else [parents]
            cs = [self.carrier(p, env) for p in parents]
            if kind == "INTERSECT":
                if len(cs) < 2:
                    raise Unresolved("INTERSECT of one")
                c = cs[0]
                for x in cs[1:]:
                    c = Both(c, x)
                return c
            c = cs[0] if len(cs) == 1 else Any(cs)
            if kind == "SPLIT" and m.get("disambiguationData"):
                c.split_pick = True
            return c
        rec = self.solid.get(base)
        if rec is None:
            raise Unresolved(f"{kind} of an unknown feature")
        if rec["kind"] == "sweep":
            return self.sweep_carrier(kind, m, rec, env)
        if rec["kind"] == "loft":
            return self.loft_carrier(kind, m, rec, env)
        ents = [self.ent(s) for s in self.osd(m, env)]
        if not ents and "derivedFrom" in m:
            ents = [self.ent(self.deref(m["derivedFrom"], env))]
        if not ents:
            raise Unresolved(f"{kind} without entities")
        if rec["kind"] == "extrude":
            fr = rec["frame"]
            lo, hi = rec["lo"], rec["hi"]
            if kind in ("CAP_EDGE", "CAP_FACE"):
                z = lo if m.get("isStart") else hi
                if z is None:
                    raise Unresolved("a cap at an unknown depth")
                if kind == "CAP_FACE":
                    return OnPlane(fr.world(0, 0, z), fr.n)
                return Any([Swept(fr, e[3], z, z) for e in ents])
            if kind == "SWEPT_FACE":
                return Any([Swept(fr, e[3], *span(lo, hi)) for e in ents])
            if kind == "SWEPT_EDGE":
                pts = self.shared(ents)
                return Any([Swept(fr, sketch2d.Piece("", "point", p, p), *span(lo, hi)) for p in pts])
            raise Unresolved(f"{kind} of an extrude")
        if rec["kind"] == "revolve":
            fr = rec["frame"]
            if kind == "SWEPT_FACE":
                return Any([Turned(fr, e[3], rec["a2"], rec["d2"]) for e in ents])
            if kind == "SWEPT_EDGE":
                pts = self.shared(ents)
                return Any([Turned(fr, sketch2d.Piece("", "point", p, p), rec["a2"], rec["d2"]) for p in pts])
            raise Unresolved(f"{kind} of a revolve")
        raise Unresolved(f"{kind} of {rec['kind']}")

    def osd_ents(self, m, env, fids):
        """What a query's OSD names, split: entities in sketches `fids`, and the rest (a path's)."""
        mine, rest = [], []
        for x in self.osd(m, env):
            if isinstance(x, fs.Call) and x[0] == "sQuery":
                (mine if x[1][0][1].split(".")[0] in fids else rest).append(self.ent(x))
        return mine, rest

    def sweep_carrier(self, kind, m, rec, env):
        mine, path = self.osd_ents(m, env, rec["fids"])
        points = [e for e in mine if e[3].kind == "point"]
        curves = [e for e in mine if e[3].kind != "point"]
        turn = rec.get("ring")
        if turn:
            fr, a2, d2 = turn
            if kind == "SWEPT_FACE" and curves:
                return Any([Turned(fr, e[3], a2, d2) for e in curves])
            if kind == "SWEPT_EDGE":
                ps = [e[3].a for e in points] or self.shared(curves)
                return Any([Turned(fr, sketch2d.Piece("", "point", q, q), a2, d2) for q in ps])
            raise Unresolved(f"{kind} of a whole-turn sweep")
        names = {e[3].ent for e in path if e[3].kind != "point"}
        ends = [e[2].world(*e[3].a) for e in path if e[3].kind == "point"]
        legs = [(fr, p) for ls in rec["legs"] for fr, p in ls if p is not None and (not names or p.ent in names)]
        if kind in ("CAP_FACE", "CAP_EDGE"):
            caps = [ls[0][0] for ls in rec["legs"] if len(rec["legs"]) == 1] + [ls[-1][0] for ls in rec["legs"]]
            if ends:
                pick = [fr for fr in caps if abs(fr.uvw(ends[0])[2]) < 1e-3]
            elif len(caps) == 2:
                pick = [caps[0 if m.get("isStart") else 1]]
            else:
                pick = []
            if not pick:
                raise Unresolved("a sweep's cap the query does not settle")
            if kind == "CAP_FACE":
                return OnPlane(pick[0].o, pick[0].n)
            return Any([Swept(pick[0], e[3], 0.0, 0.0) for e in curves])
        if kind == "MID_CAP_EDGE":
            frs = [fr for ls in rec["legs"] for fr, _ in ls]
            if not curves:
                raise Unresolved("MID_CAP_EDGE without entities")
            c = Any([Swept(fr, e[3], 0.0, 0.0) for fr in frs for e in curves])
            c.split_pick = True
            return c
        if kind == "SWEPT_FACE":
            if not curves:
                raise Unresolved("SWEPT_FACE without entities")
            return Any([leg_carrier(fr, p, e[3]) for fr, p in legs for e in curves])
        if kind == "SWEPT_EDGE":
            ps = [e[3].a for e in points] or self.shared(curves)
            return Any([leg_carrier(fr, p, sketch2d.Piece("", "point", q, q)) for fr, p in legs for q in ps])
        raise Unresolved(f"{kind} of a sweep")

    def loft_carrier(self, kind, m, rec, env):
        secs = rec["secs"]
        fids = {s["fid"] for s in secs}
        if kind == "CAP_FACE":
            # isStart is not the array's order: the query names its profile
            named = [s for x in self.osd(m, env) if isinstance(x, fs.Call) and x[0] == "makeQuery" for s in secs if s["fid"] == x[1][0][1].split(".")[0]]
            s = named[0] if len(named) == 1 else secs[0 if m.get("isStart") else -1]
            return OnPlane(s["fr"].o, s["fr"].n)
        mine = self.osd_ents(m, env, fids)[0] if self.osd(m, env) and all(isinstance(x, fs.Call) and x[0] == "sQuery" for x in self.osd(m, env)) else []
        if kind == "CAP_EDGE" or kind == "MID_CAP_EDGE":
            k = (0 if m.get("isStart") else len(secs) - 1) if kind == "CAP_EDGE" else m.get("capPos")
            if not isinstance(k, (int, float)) or k != int(k) or not 0 <= int(k) < len(secs):
                raise Unresolved("a loft section the query does not settle")
            fid = secs[int(k)]["fid"]
            ents = [e for e in mine if e[0] == fid and e[3].kind != "point"]
            if not ents:
                raise Unresolved(f"{kind} without entities")
            return Any([Swept(e[2], e[3], 0.0, 0.0) for e in ents])
        if kind == "SWEPT_FACE":
            ents = [e for e in mine if e[3].kind != "point"]
            if not ents:
                raise Unresolved("SWEPT_FACE of a loft without entities")
            return LoftWall([Swept(e[2], e[3], 0.0, 0.0) for e in ents], [s["fr"] for s in secs])
        if kind == "SWEPT_EDGE":
            per = []
            for s in secs:
                es = [e for e in mine if e[0] == s["fid"]]
                if not es:
                    raise Unresolved("SWEPT_EDGE of a loft without entities")
                ps = [e[3].a for e in es if e[3].kind == "point"] or self.shared(es)
                per.append([es[0][2].world(*q) for q in ps])
            return LoftRail(per)
        raise Unresolved(f"{kind} of a loft")

    def shared(self, ents):
        if len(ents) == 1:
            e = ents[0][3]
            return [e.a, e.b]
        ps = None
        for x, y in zip(ents, ents[1:]):
            m = [(round(p[0], 4), round(p[1], 4)) for p in meets(x[3], y[3])]
            ps = set(m) if ps is None else ps & set(m)
        if not ps:
            raise Unresolved("entities that do not meet")
        return sorted(ps)

    def edges_for(self, qs, env, tangent):
        st = self.state()
        edges = st.get("edges", [])
        faces = st.get("faces", [])
        picked = {}
        byh = {e["h"]: e for e in edges}
        for q0 in qs:
            q = self.deref(q0, env)
            c = self.carrier(q, env)
            face_q = isinstance(q, fs.Call) and len(q[1]) > 2 and q[1][2] == ("name", "FACE")
            if face_q:
                hit = on_faces(c, st)
                if not hit:
                    raise Unresolved("no face on the carrier: " + q[1][1])
                for e in edges:
                    if hit & set(e["f"]) and e["c"] != "smooth":
                        picked[e["h"]] = e
            else:
                hit = on_edges(c, st)
                if not hit:
                    raise Unresolved("no edge on the carrier: " + (q[1][1] if q[0] == "makeQuery" else q[0]))
                if len(hit) > 1 and getattr(c, "split_pick", False):
                    raise Unresolved("a split piece the order would have to pick")
                for h in hit:
                    picked[h] = byh[h]
        if tangent:
            at = {}
            for e in edges:
                for k, v in enumerate(e["v"]):
                    at.setdefault(v, []).append((e, k))
            todo = list(picked.values())
            while todo:
                e = todo.pop()
                for k, v in enumerate(e["v"]):
                    t = e["t"][k]
                    if not t:
                        continue
                    for g, j in at[v]:
                        if g["h"] in picked or g["c"] == "smooth" or not g["t"][j]:
                            continue
                        a, b = t, g["t"][j]
                        cr = (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])
                        if math.sqrt(dot(cr, cr)) < 1e-4:
                            picked[g["h"]] = g
                            todo.append(g)
        return sorted(picked)

    def faces_for(self, qs, env):
        st = self.state()
        out = []
        for q0 in qs:
            c = self.carrier(q0, env)
            hit = on_faces(c, st)
            if not hit:
                q = self.deref(q0, env)
                raise Unresolved("no face on the carrier: " + (q[1][1] if q[0] == "makeQuery" else q[0]))
            out += hit
        return sorted(set(out))

    # profiles
    def regions(self, args, env):
        """{sketch id: [region..]} for an entities list."""
        by = {}
        for q0 in qunion(args):
            q = self.deref(q0, env)
            if isinstance(q, fs.Call) and q[0] == "qSketchRegion":
                fid = q[1][0][1]
                if fid not in self.sk:
                    raise Unresolved("a region of an unread sketch")
                a = self.sk[fid][0]
                filt = len(q[1]) > 1 and q[1][1] is True
                by.setdefault(fid, []).extend(a.outermost() if filt else range(len(a.faces)))
                if len(a.faces) > 1:
                    # the query says "all", which does not always hold for sketches of many regions
                    self.notes.add("all-regions")
            elif isinstance(q, fs.Call) and q[0] == "makeQuery" and q[1][1] == "IMPRINT" and q[1][2] == ("name", "FACE"):
                fid, r = self.imprint(q, env)
                by.setdefault(fid, []).append(r)
            elif isinstance(q, fs.Call) and q[0] == "makeQuery" and q[1][2] == ("name", "FACE"):
                fid, r = self.face_region(q, env)
                by.setdefault(fid, []).extend(r)
            else:
                raise Unresolved("profile query " + (q[0] if isinstance(q, tuple) else "?"))
        return by

    def face_region(self, q, env):
        """A planar face of the part as built, as a sketch of its own edges: the region they bound outermost. The face's
        outward normal is the sketch's, as Onshape extrudes from a face."""
        st = self.state()
        hit = on_faces(self.carrier(q, env), st)
        if len(hit) > 1:
            hit = self.settle_face(hit, q, env, st)
        if len(hit) != 1:
            raise Unresolved("a face the query does not settle" if hit else "no face on the carrier: " + q[1][1])
        h = next(iter(hit))
        f = next(x for x in st["faces"] if x["h"] == h)
        if not f["n"]:
            raise Unsupported("a profile from a curved face")
        es = [e for e in st["edges"] if h in e["f"]]
        n = tuple(f["n"])
        d = next((add(e["p"][-1], e["p"][0], -1) for e in es if e["k"] == "line"), None)
        x = unit(d) if d else unit(cross(n, (1.0, 0.0, 0.0) if abs(n[0]) < 0.9 else (0.0, 1.0, 0.0)))
        fr = Frame.of(tuple(es[0]["p"][0]), x, cross(n, x), n)
        ents = [self.edge_ent(e, fr) for e in es]
        fid = "face:" + h
        a = Arrangement(ents)
        self.sk[fid], self.sketch_ents[fid] = (a, fr), ents
        return fid, a.outermost()

    def settle_face(self, hit, q, env, st):
        """Faces on one plane: the one with an edge on the entities the query's cap was drawn from."""
        m = q[1][3]
        try:
            ents = [self.ent(x) for x in self.osd(m, env) if isinstance(x, fs.Call) and x[0] == "sQuery"]
        except Unresolved:
            return hit
        if not ents:
            return hit
        c = self.carrier(q, env)
        hs = {h for e in st["edges"] if set(e["f"]) & hit and all(c.dist(p) <= TOL and any(curve_dist(g[3], g[2].uvw(p)[:2]) <= TOL for g in ents) for p in e["p"]) for h in set(e["f"]) & hit}
        return hs or hit

    def edge_ent(self, e, fr):
        """A kernel edge as a sketch entity on frame fr."""
        uv = [fr.uvw(p)[:2] for p in e["p"]]
        if e["k"] == "line":
            return {"kind": "skLineSegment", "id": e["h"], "start": list(uv[0]), "end": list(uv[-1])}
        if e["k"] == "circle" and math.dist(uv[0], uv[-1]) < 1e-6:
            cf = sketch2d.circle_from(uv[0], uv[5], uv[11])
            return {"kind": "skCircle", "id": e["h"], "center": list(cf[0]), "radius": cf[1]}
        if e["k"] == "circle":
            return {"kind": "skArc", "id": e["h"], "start": list(uv[0]), "mid": list(uv[8]), "end": list(uv[-1])}
        self.notes.add("approx")
        return {"kind": "polyline", "id": e["h"], "points": [list(p) for p in uv]}

    def names_outside(self, q, env):
        """Does this region's query name an edge that is not its own sketch's?"""
        fid = q[1][0][1].split(".")[0]
        todo = [q]
        while todo:
            x = todo.pop()
            if isinstance(x, tuple) and x and x[0] == "name":
                if x[1] not in env:
                    continue
                x = self.deref(x, env)
            if isinstance(x, fs.Call):
                if x[0] == "makeQuery" and x[1][1] != "IMPRINT":
                    return True
                if x[0] == "makeQuery" and x[1][0][1].split(".")[0] != fid:
                    return True
                if x[0] == "sQuery" and x[1][0][1].split(".")[0] != fid:
                    return True
                todo += list(x[1])
            elif isinstance(x, dict):
                todo += list(x.values())
            elif isinstance(x, list):
                todo += x
        return False

    def imprinted(self, fid):
        """Onshape imprints a sketch on its plane: regions are also cut by the edges of faces lying in that plane
        and by earlier sketches drawn in it. The sketch's arrangement with those added."""
        base, fr = self.sk[fid]
        if fid in self.imprint_off:
            return base
        ents = list(self.sketch_ents[fid])
        have = {e["id"] for e in ents}
        for g, (ga, gfr) in self.sk.items():
            if g == fid or g not in self.sketch_ents:
                continue
            if abs(abs(dot(gfr.n, fr.n)) - 1) > 1e-9 or abs(fr.uvw(gfr.o)[2]) > 1e-6:
                continue
            for e in self.sketch_ents[g]:
                if e.get("construction") or e["kind"] in ("skPoint", "skText"):
                    continue
                pts = lambda q: list(fr.uvw(gfr.world(q[0], q[1]))[:2])
                n = dict(e, id=f"{g}/{e['id']}")
                try:
                    for k in ("start", "mid", "end", "center"):
                        if k in e:
                            n[k] = pts(e[k])
                    if "points" in e or "majorAxis" in e:
                        continue
                except (TypeError, IndexError):
                    continue
                ents.append(n)
        if self.bodies:
            st = self.state()
            planar = {f["h"] for f in st.get("faces", []) if f["n"] and abs(abs(dot(f["n"], fr.n)) - 1) < 1e-9 and all(abs(fr.uvw(p)[2]) < 1e-5 for p in f["p"])}
            for e in st.get("edges", []):
                if not planar & set(e["f"]):
                    continue
                ents.append(self.edge_ent(e, fr))
        if len(ents) == len(have):
            return base
        a = Arrangement(ents)
        self.sk[fid] = (a, fr)
        self.notes.add("imprint")
        return a

    def imprint(self, q, env):
        fid = q[1][0][1].split(".")[0]
        if fid not in self.sk:
            raise Unresolved("an imprint of an unread sketch")
        self.plain.setdefault(fid, self.sk[fid][0])
        plain = self.plain[fid]
        a = self.imprinted(fid) if self.names_outside(q, env) else plain
        self.current_fr = self.sk[fid][1]
        cands = None
        for d in q[1][3].get("disambiguationData", []):
            if d[0] != "TD":
                continue
            for item in d[1][0]:
                eq, w = self.deref(item[0], env), item[1]
                rs = self.side_regions(a, eq, w, env)
                # the sketch alone says which side, where it can: what lies in the plane only cuts that up
                if a is not plain and len(rs) > 1:
                    try:
                        was = self.side_regions(plain, eq, w, env)
                    except Unresolved:
                        was = set()
                    if len(was) == 1:
                        r0 = next(iter(was))
                        ps, _ = self.edge_pieces(a, eq, env)
                        keep = set()
                        for i in ps:
                            for side in (1, -1):
                                r = a.region_at(i, side)
                                if r is not None and plain.holds(r0, sketch2d.point_within(a.loops[a.faces[r]["outer"]], a)):
                                    keep.add(r)
                        rs = keep or rs
                cands = rs if cands is None else cands & rs
        if a is not plain and (not cands or len(cands) > 1):
            # a query that names no piece means its curve was not cut there: the sketch alone, then
            self.sk[fid] = (plain, self.sk[fid][1])
            self.imprint_off.add(fid)
            self.notes.discard("imprint")
            return self.imprint(q, env)
        if not cands:
            raise Unresolved("no region on that side")
        if len(cands) > 1:
            raise Unresolved("a region the query does not settle")
        return fid, next(iter(cands))

    def edge_pieces(self, a, eq, env):
        """The arrangement pieces an imprint edge query can be, its neighbours narrowing it down."""
        eq = self.deref(eq, env)
        if isinstance(eq, fs.Call) and eq[0] == "sQuery":
            return a.pieces_of(eq[1][2]), eq[1][2]
        if not (isinstance(eq, fs.Call) and eq[0] == "makeQuery"):
            raise Unresolved("imprint edge form")
        m = eq[1][3]
        s = self.deref(m.get("derivedFrom"), env)
        if isinstance(s, fs.Call) and s[0] == "makeQuery":
            # an edge from outside this sketch: what it imprinted lies on its carrier
            if s[1][1] == "IMPRINT":
                t = self.deref(s[1][3].get("derivedFrom"), env)
                if isinstance(t, fs.Call) and t[0] == "sQuery":
                    n = f"{s[1][0][1].split('.')[0]}/{t[1][2]}"
                    ps = a.pieces_of(n)
                    if ps:
                        return ps, n
                raise Unresolved("imprint edge source")
            c = self.carrier(s, env)
            fr = self.current_fr
            ps = []
            for i in a.live:
                pc = a.pieces[i]
                q = pc.sample(4)[len(pc.sample(4)) // 2] if pc.kind != "circle" else pc.a
                if c.dist(fr.world(q[0], q[1])) <= TOL:
                    ps.append(i)
            if not ps:
                raise Unresolved("imprint edge source")
            return ps, a.pieces[ps[0]].ent
        if not (isinstance(s, fs.Call) and s[0] == "sQuery"):
            raise Unresolved("imprint edge source")
        name = s[1][2]
        if name not in a.every:
            raise Unresolved("a reference to an entity the sketch no longer has")
        ps = a.pieces_of(name)
        if not ps:
            raise Unresolved("imprint edge bounds nothing")
        for d in m.get("disambiguationData", []):
            if d[0] != "TD" or len(ps) < 2:
                continue
            for it in d[1][0]:
                v = self.deref(it[0], env)
                if isinstance(v, fs.Call) and v[0] == "makeQuery" and v[1][2] == ("name", "VERTEX"):
                    # measured on the pilot: naming the piece by this vertex gave 1 match and 16 wrong, its weight's
                    # sign changing nothing. What the weight means is not known, so the query stays unsettled
                    continue
                try:
                    others, _ = self.edge_pieces(a, it[0], env)
                except Unresolved:
                    continue
                near = {v for j in others for v in (a.pieces[j].va, a.pieces[j].vb)}
                keep = [i for i in ps if {a.pieces[i].va, a.pieces[i].vb} & near]
                ps = keep or ps
        return ps, name

    def side_regions(self, a, eq, w, env):
        ps, name = self.edge_pieces(a, eq, env)
        # the weight names the side, wrongly about one time in ten (an entity's direction is not its
        # wire's): a side with no region settles it; else take the weight's, and the part says so
        on = {a.region_at(i, SIDE * w, name) for i in ps} - {None}
        off = {a.region_at(i, -SIDE * w, name) for i in ps} - {None}
        if on and off:
            base = self.plain.get(a_id(self, a))
            if base is not None and base is not a:
                return on | off  # both, for imprint() to settle against the sketch alone
            self.notes.add("side-by-weight")
        return on or off

    def loop_text(self, a, lp, flip):
        f = (lambda p: (p[0], -p[1])) if flip else (lambda p: p)
        if len(lp) == 1 and a.pieces[lp[0][0]].kind == "circle":
            p = a.pieces[lp[0][0]]
            c = f(p.c)
            return f"circle {num(c[0])} {num(c[1])} {num(p.r)}"
        parts = []
        for i, fwd in lp:
            p = a.pieces[i]
            s, e = a.verts[p.va], a.verts[p.vb]
            if not fwd:
                s, e = e, s
            s, e = f(s), f(e)
            if p.kind == "arc":
                _, d = p.sweep()
                d = (d if fwd else -d) * (-1 if flip else 1)
                c = f(p.c)
                # refit through the shared ends
                ch = math.dist(s, e)
                r = p.r
                if ch > 1e-9:
                    r = max(r, ch / 2)
                    h = math.sqrt(max(r * r - ch * ch / 4, 0.0))
                    m = ((s[0] + e[0]) / 2, (s[1] + e[1]) / 2)
                    n = ((e[1] - s[1]) / ch, (s[0] - e[0]) / ch)
                    c1, c2 = (m[0] + h * n[0], m[1] + h * n[1]), (m[0] - h * n[0], m[1] - h * n[1])
                    c = c1 if math.dist(c1, c) <= math.dist(c2, c) else c2
                a0 = math.degrees(math.atan2(s[1] - c[1], s[0] - c[0]))
                a1 = math.degrees(math.atan2(e[1] - c[1], e[0] - c[0]))
                dg = a1 - a0
                while d > 0 and dg <= 0:
                    dg += 360
                while d < 0 and dg >= 0:
                    dg -= 360
                parts.append(f"arc {num(c[0])} {num(c[1])} {num(r)} {a0:.10f} {dg:.10f}")
            elif p.kind == "poly":
                ps = [f(q) for q in (p.pts if fwd else p.pts[::-1])]
                ps[0], ps[-1] = s, e
                for u, v in zip(ps, ps[1:]):
                    parts.append(f"line {num(u[0])} {num(u[1])} {num(v[0])} {num(v[1])}")
            else:
                parts.append(f"line {num(s[0])} {num(s[1])} {num(e[0])} {num(e[1])}")
        return "loop " + " ".join(parts)

    def op_of(self, a):
        t = a.get("operationType")
        w = t[1].split(".")[-1] if isinstance(t, tuple) else "NEW"
        return {"NEW": "new", "ADD": "add", "REMOVE": "cut", "INTERSECT": "keep"}.get(w)

    def solid_line(self, op, ents, fid_out):
        """A solid feature's label, its op word and its scope, with the bodies it makes counted."""
        this, label = op, ""
        if self.bodies == 0 or op == "new":
            if self.bodies == 0 and op in ("cut", "keep"):
                raise Unsupported("first feature cuts")
            this = "" if self.bodies == 0 else "new"
            self.names += 1
            self.current = f"b{self.names}"
            label = self.current + ": "
            self.bodies += 1
        elif op == "add" and self.bodies > 1:
            self.notes.add("multibody-join")
        if fid_out is not None:
            self.solid[fid_out].setdefault("made", []).append((ents, self.current))
        return label, this, " in all" if self.bodies > 1 and op in ("cut", "keep") else ""

    def emit_solid(self, kind, op, head, frame_text, by, flip, tail="", fid_out=None, after=""):
        for fid, regs in by.items():
            a = self.sk[fid][0]
            for pi, prof in enumerate(a.merged(sorted(set(regs)))):
                loops = " ".join(self.loop_text(a, lp, flip) for lp in prof)
                ents = {a.pieces[i].ent for lp in prof for i, _ in lp}
                label, this, scope = self.solid_line(op, ents, fid_out)
                self.lines.append(f"{label}{kind} {this + ' ' if this else ''}{head} {frame_text} {tail}{loops}{after}{scope}")

    def extrude(self, f):
        a, env = f["args"], f["env"]
        for k in ("hasDraft", "hasSecondDirectionDraft"):
            if a.get(k) is True:
                raise Unsupported("draft on an extrude")
        b = a.get("bodyType")
        if isinstance(b, tuple) and "SOLID" not in b[1]:
            raise Unsupported("a thin or surface extrude")
        if "surfaceEntities" in a or "entities" not in a:
            raise Unsupported("a surface extrude")
        if a.get("hasOffset") is True:
            raise Unsupported("extrude start offset")
        by = self.regions(a["entities"], env)
        if not by:
            raise Unresolved("no profile")
        op = self.op_of(a)
        if op is None:
            raise Unsupported("extrude operation")
        bound = a.get("endBound")
        bound = bound[1].split(".")[-1] if isinstance(bound, tuple) else "BLIND"
        if a.get("symmetric") is True and bound == "BLIND":
            bound = "SYMMETRIC"
        opp = a.get("oppositeDirection") is True
        for fid in by:
            fr = self.sk[fid][1]
            lo, hi = 0.0, None
            frame = "z {} x {} at {}".format(vtext(fr.n if not opp else tuple(-x for x in fr.n)), vtext(fr.x), vtext(fr.o))
            sgn = -1.0 if opp else 1.0
            second = a.get("hasSecondDirection") is True
            if second and bound not in ("BLIND", "THROUGH_ALL"):
                raise Unsupported("second direction with " + bound)
            if bound == "BLIND":
                d = self.val(a["depth"])
                if second:
                    sb = a.get("secondDirectionBound")
                    sb = sb[1].split(".")[-1] if isinstance(sb, tuple) else "BLIND"
                    if sb != "BLIND" or a.get("secondDirectionOppositeDirection") is False:
                        raise Unsupported("second direction form")
                    d2 = self.val(a["secondDirectionDepth"])
                    head, lo, hi = f"two {num(d)} {num(d2)}", -d2, d
                else:
                    head, hi = num(d), d
            elif bound == "SYMMETRIC":
                d = self.val(a["depth"])
                head, lo, hi = f"sym {num(d)}", -d / 2, d / 2
            elif bound == "THROUGH_ALL":
                head, lo = ("thru both", None) if second else ("thru", 0.0)
            elif bound == "UP_TO_NEXT":
                head = "next"
            elif bound == "UP_TO_SURFACE":
                c = self.carrier(single(a.get("endBoundEntityFace"), env), env)
                if not isinstance(c, OnPlane):
                    raise Unsupported("up to a curved face")
                w = dot(add(c.o, fr.o, -1), fr.n) * sgn
                dn = abs(dot(c.n, fr.n))
                if dn < 1 - 1e-9:
                    raise Unsupported("up to a slanted face")
                off = self.val(a.get("offsetDistance", 0.0)) if a.get("hasOffset") else 0.0
                if w <= 0:
                    raise Unresolved("up to a face behind the sketch")
                head, hi = num(w + off), w + off
            else:
                raise Unsupported("extrude bound " + bound)
            # lo, hi along the extrude's direction; records measure along the sketch normal
            self.solid[f["id"]] = {"kind": "extrude", "frame": fr, "lo": None if lo is None else lo * sgn, "hi": None if hi is None else hi * sgn}
            self.emit_solid("extrude", op, head, frame, {fid: by[fid]}, opp, fid_out=f["id"])

    def revolve(self, f):
        a, env = f["args"], f["env"]
        if "surfaceEntities" in a or "entities" not in a:
            raise Unsupported("a surface revolve")
        b = a.get("bodyType")
        if isinstance(b, tuple) and "SOLID" not in b[1]:
            raise Unsupported("a thin or surface revolve")
        by = self.regions(a["entities"], env)
        op = self.op_of(a)
        if op is None or len(by) != 1:
            raise Unsupported("revolve form")
        fid = next(iter(by))
        fr = self.sk[fid][1]
        ax = single(a["axis"], env)
        ax = self.deref(ax, env)
        if not (isinstance(ax, fs.Call) and ax[0] == "sQuery"):
            raise Unsupported("revolve about a solid's edge")
        _, _, afr, e = self.ent(ax)
        if e.kind != "line":
            raise Unresolved("axis not a line")
        p0, p1 = afr.world(*e.a), afr.world(*e.b)
        u0, v0, w0 = fr.uvw(p0)
        u1, v1, w1 = fr.uvw(p1)
        if abs(w0) > 1e-6 or abs(w1) > 1e-6:
            raise Unsupported("axis out of the sketch's plane")
        l = math.hypot(u1 - u0, v1 - v0)
        d2 = ((u1 - u0) / l, (v1 - v0) / l)
        t = a.get("revolveType")
        t = t[1].split(".")[-1] if isinstance(t, tuple) else "FULL"
        opp = a.get("oppositeDirection") is True
        if opp:
            d2 = (-d2[0], -d2[1])
        if t == "FULL":
            head = ""
        elif t == "ONE_DIRECTION":
            head = num(self.val(a["angle"]))
        elif t == "SYMMETRIC":
            head = "sym " + num(self.val(a["angle"]))
        elif t == "TWO_DIRECTIONS":
            head = f"two {num(self.val(a['angle']))} {num(self.val(a['angleBack']))}"
        else:
            raise Unsupported("revolve type " + t)
        self.solid[f["id"]] = {"kind": "revolve", "frame": fr, "a2": (u0, v0), "d2": d2}
        frame = f"z {vtext(fr.n)} x {vtext(fr.x)} at {vtext(fr.o)}"
        tail = f"axis {num(u0)} {num(v0)} {num(d2[0])} {num(d2[1])} "
        self.emit_solid("revolve", op, head, frame, by, False, tail, fid_out=f["id"])

    def wpieces(self, v, env):
        """A path's edges as world pieces, in no order: sketch entities, or edges of the part as built."""
        out, seen = [], set()
        def one(fid, name):
            if (fid, name) in seen:
                return
            seen.add((fid, name))
            a, fr = self.sk[fid]
            e = a.every.get(name)
            if e is None:
                raise Unresolved("unknown sketch entity")
            if e.kind == "poly":
                raise Unsupported("a path along a spline")
            if e.kind == "point":
                raise Unresolved("a path of a point")
            if e.kind == "line":
                out.append(WPiece(fr.world(*e.a), fr.world(*e.b), ent=name))
            elif e.kind == "circle":
                x = fr.world(*e.a)
                c = fr.world(*e.c)
                out.append(WPiece(x, x, c, fr.n, 2 * math.pi, ent=name))
            else:
                _, d = e.sweep()
                out.append(WPiece(fr.world(*e.a), fr.world(*e.b), fr.world(*e.c), fr.n, d, ent=name))
        for q0 in qunion(v):
            q = self.deref(q0, env)
            if isinstance(q, fs.Call) and q[0] == "sQuery":
                fid = q[1][0][1].split(".")[0]
                if fid not in self.sk:
                    raise Unresolved("entity of an unread sketch")
                one(fid, q[1][2])
            elif isinstance(q, fs.Call) and q[0] == "qConstructionFilter":
                w = self.deref(q[1][0], env)
                while isinstance(w, fs.Call) and w[0] == "qBodyType":
                    w = self.deref(w[1][0], env)
                if not (isinstance(w, fs.Call) and w[0] == "qCreatedBy" and isinstance(w[1][0], tuple) and w[1][0][1].split(".")[0] in self.sk):
                    raise Unresolved("a path query form")
                fid = w[1][0][1].split(".")[0]
                for c in self.sk[fid][0].curves:
                    one(fid, c.ent)
            elif isinstance(q, fs.Call) and q[0] == "makeQuery":
                st = self.state()
                byh = {e["h"]: e for e in st.get("edges", [])}
                for h in self.edges_for([q], env, False):
                    out.append(edge_piece(byh[h]))
            else:
                raise Unresolved("a path query form")
        return out

    def chain(self, pieces):
        """Pieces joined end to end, turned to run one way; the closed flag. Unsupported if they branch or part."""
        if len(pieces) == 1:
            return pieces[:], is_ring(pieces[0])
        left = pieces[1:]
        run = [pieces[0]]
        while left:
            for i, p in enumerate(left):
                if near3(run[-1].b, p.a):
                    run.append(p)
                elif near3(run[-1].b, p.b):
                    run.append(p.rev())
                elif near3(run[0].a, p.b):
                    run.insert(0, p)
                elif near3(run[0].a, p.a):
                    run.insert(0, p.rev())
                else:
                    continue
                del left[i]
                break
            else:
                raise Unsupported("a path in separate pieces")
        return run, near3(run[0].a, run[-1].b)

    def path_from(self, run, closed, fr, near):
        """Where the profile's plane crosses the path square to it: the path to sweep along from there, or two when
        the crossing is inside the path. `near`, a point of the profile, picks among crossings."""
        hits = []
        for k, p in enumerate(run):
            off = lambda q: dot(add(q, fr.o, -1), fr.n)
            if p.c is None:
                den = off(p.b) - off(p.a)
                if abs(den) > 1e-12:
                    t = -off(p.a) / den
                    if -1e-9 <= t <= 1 + 1e-9:
                        hits.append((k, min(max(t, 0.0), 1.0), add(p.a, add(p.b, p.a, -1), t), p.tangent(p.a)))
                continue
            r0 = add(p.a, p.c, -1)
            at = lambda w: add(p.c, turned(r0, p.z, w))
            n = 32
            vals = [off(at(p.sweep * i / n)) for i in range(n + 1)]
            if abs(dot(p.z, fr.n)) < 1e-6 and abs(off(p.c)) < 1e-4:
                # the radius lies in the plane, twice a turn
                th = math.atan2(-dot(r0, fr.n), dot(cross(p.z, r0), fr.n))
                for w in (th + j * math.pi for j in range(-3, 4)):
                    t = w / p.sweep
                    if -1e-9 <= t <= 1 + 1e-9:
                        q = at(w)
                        hits.append((k, min(max(t, 0.0), 1.0), q, p.tangent(q)))
            elif min(vals) < 0 < max(vals):
                hits.append((k, 0.0, p.a, p.z))  # meets the plane, not square to it
        square = [h for h in hits if math.sqrt(dot(cross(h[3], fr.n), cross(h[3], fr.n))) < 1e-6]
        if not square:
            raise Unsupported("a profile not square to its path" if hits else "a profile off its path")
        k, t, _, _ = min(square, key=lambda h: math.dist(h[2], near))
        p = run[k]
        after = ([part(p, t, 1)] if t < 1 - 1e-9 else []) + run[k + 1:]
        first = [part(p, t, 0)] if t > 1e-9 else []
        if closed:
            # a closed path has no cap to end on: both ways round from the profile, meeting at the far side
            whole = after + run[:k] + first
            half = sum(math.dist(x.a, x.b) if x.c is None else abs(x.sweep) * math.dist(x.a, x.c) for x in whole) / 2
            out = []
            for i, x in enumerate(whole):
                l = math.dist(x.a, x.b) if x.c is None else abs(x.sweep) * math.dist(x.a, x.c)
                if l >= half - 1e-9:
                    out = whole[:i] + [part(x, half / l, 0)]
                    tail = ([part(x, half / l, 1)] if half / l < 1 - 1e-9 else []) + whole[i + 1:]
                    return [out, [y.rev() for y in reversed(tail)]]
                half -= l
        before = [x.rev() for x in reversed(run[:k] + first)]
        return [x for x in (after, before) if x]

    def sweep(self, f):
        a, env = f["args"], f["env"]
        b = a.get("bodyType")
        if isinstance(b, tuple) and "SOLID" not in b[1]:
            raise Unsupported("a surface sweep")
        if "profiles" not in a:
            raise Unsupported("a surface sweep")
        if a.get("keepProfileOrientation") is True:
            raise Unsupported("a sweep keeping its profile's orientation")
        by = self.regions(a["profiles"], env)
        op = self.op_of(a)
        if op is None or not by:
            raise Unsupported("sweep operation")
        run, closed = self.chain(self.wpieces(a["path"], env))
        self.solid[f["id"]] = {"kind": "sweep", "legs": [], "fids": set(by)}
        for fid in by:
            fr = self.sk[fid][1]
            for reg in self.sk[fid][0].merged(sorted(set(by[fid]))):
                self.sweep_one(f, fid, fr, reg, run, closed, op)

    def sweep_one(self, f, fid, fr, prof, run, closed, op):
        arr = self.sk[fid][0]
        pc = arr.pieces[prof[0][0][0]]
        near = fr.world(*pc.a)
        if len(run) == 1 and is_ring(run[0]):
            # a whole turn: the profile turned about the circle's axis
            p = run[0]
            if abs(dot(p.z, fr.n)) > 1e-6:
                raise Unsupported("a profile not square to its path")
            if abs(dot(add(p.c, fr.o, -1), fr.n)) > 1e-5:
                raise Unsupported("a profile off its path")
            u, v, _ = fr.uvw(p.c)
            d = (dot(p.z, fr.x), dot(p.z, fr.y))
            rec = self.solid[f["id"]]
            rec["ring"] = (fr, (u, v), (d[0] / math.hypot(*d), d[1] / math.hypot(*d)))
            loops = " ".join(self.loop_text(arr, lp, False) for lp in prof)
            ents = {arr.pieces[i].ent for lp in prof for i, _ in lp}
            label, this, scope = self.solid_line(op, ents, f["id"])
            self.lines.append(f"{label}revolve {this + ' ' if this else ''}z {vfine(fr.n)} x {vfine(fr.x)} at {vfine(fr.o)} axis {fine(u)} {fine(v)} {fine(d[0])} {fine(d[1])} {loops}{scope}")
            return
        paths = self.path_from(run, closed, fr, near)
        if len(paths) > 1 and op == "keep":
            raise Unsupported("an intersecting sweep both ways from its profile")
        for k, path in enumerate(paths):
            path = tangent_fit(path)
            t0 = path[0].tangent(path[0].a)
            # the frame square to the path exactly, the profile's own axes kept: the data's plane is a little off
            z = t0 if dot(t0, fr.n) > 0 else tuple(-x for x in t0)
            y = unit(cross(z, fr.x))
            x = cross(y, z)
            o = add(fr.o, z, dot(add(path[0].a, fr.o, -1), z))
            frame = f"z {vfine(z)} x {vfine(x)} at {vfine(o)}"
            loops = " ".join(self.loop_text(arr, lp, False) for lp in prof)
            ents = {arr.pieces[i].ent for lp in prof for i, _ in lp}
            this_op = op if k == 0 else ("cut" if op == "cut" else "add")
            label, this, scope = self.solid_line(this_op, ents, f["id"] if k == 0 else None)
            self.solid[f["id"]]["legs"].append(legs_of(Frame.of(o, x, y, z), path))
            self.lines.append(f"{label}sweep {this + ' ' if this else ''}{frame} {loops} path " + " ".join(p.text() for p in path) + scope)

    def loft(self, f):
        a, env = f["args"], f["env"]
        b = a.get("bodyType")
        if isinstance(b, tuple) and "SOLID" not in b[1]:
            raise Unsupported("a surface loft")
        if "sheetProfilesArray" not in a or "wireProfilesArray" in a:
            raise Unsupported("a surface loft")
        if a.get("addGuides") is True or a.get("guidesArray"):
            raise Unsupported("a loft with guides")
        if a.get("connections"):
            raise Unsupported("a loft with connections")
        if a.get("addSections") is True:
            raise Unsupported("a loft with a spine")
        cond = {"start": a.get("startCondition"), "end": a.get("endCondition")}
        ends = []
        for w, c in cond.items():
            c = c[1].split(".")[-1] if isinstance(c, tuple) else "DEFAULT"
            if c == "NORMAL_TO_PROFILE":
                ends.append(f"{w} {num(self.val(a.get(w + 'Magnitude', 1.0)))}")
            elif c != "DEFAULT":
                raise Unsupported("a loft end condition " + c)
        op = self.op_of(a)
        if op is None:
            raise Unsupported("loft operation")
        secs, tips = [], []
        sheets = a["sheetProfilesArray"]
        for k, m in enumerate(sheets):
            at = self.loft_point(m["sheetProfileEntities"], env)
            if at is None:
                secs.append(self.section(m["sheetProfileEntities"], env))
            elif k in (0, len(sheets) - 1):
                tips.append((k == 0, at))
            else:
                raise Unsupported("a loft through a point between sections")
        if len(secs) + len(tips) < 2 or not secs:
            raise Unsupported("a loft of one profile")
        if any(w in " ".join(ends) for w in [("start" if t[0] else "end") for t in tips]):
            raise Unsupported("a loft end condition at a point")
        n = len(secs)
        if n == 1:
            first, at = tips[0]
            off = add(secs[0]["centre"], at, -1) if first else add(at, secs[0]["centre"], -1)
            face(secs[0], dot(off, secs[0]["fr"].n) < 0)
        for k, s in enumerate(secs if n > 1 else []):
            i, j = (k, k + 1) if k + 1 < n else (k - 1, k)
            face(s, dot(add(secs[j]["centre"], secs[i]["centre"], -1), s["fr"].n) < 0)
        for k in range(1, n):
            align(secs[k - 1], secs[k])
            secs[k - 1]["matched"] = k > 1 or secs[k - 1].get("matched")
            secs[k]["matched"] = True
        for k in range(n - 1):
            a0, b0 = secs[k], secs[k + 1]
            off = add(b0["centre"], a0["centre"], -1)
            along = math.sqrt(dot(off, off)) * abs(dot(unit(off), a0["fr"].n)) if dot(off, off) > 0 else 0.0
            if abs(abs(dot(a0["fr"].n, b0["fr"].n)) - 1) > 1e-9 or abs(along - math.sqrt(dot(off, off))) > 1e-6:
                self.notes.add("loft-oblique")
                if any(whole_circle(l) for x in (a0, b0) for l in x["loops"]):
                    # Parasolid's seam on a circle is not in the history, and off its axis it twists the wall (found in
                    # the source's rulings): any choice here would be a guess that moves the volume by tens of percent
                    raise Unsupported("a loft between circles off one axis")
        if n > 2:
            self.notes.add("loft-many")
        if ends:
            self.notes.add("loft-end")
        words = ["profile " + section_text(s) for s in secs]
        for first, at in tips:
            self.notes.add("loft-point")
            words.insert(0 if first else len(words), "point " + vfine(at))
        ents = {e for s in secs for e in s["ents"]}
        self.solid[f["id"]] = {"kind": "loft", "secs": secs}
        label, this, scope = self.solid_line(op, ents, f["id"])
        self.lines.append(f"{label}loft {this + ' ' if this else ''}smooth {' '.join(ends) + ' ' if ends else ''}" + " ".join(words) + scope)

    def loft_point(self, v, env):
        """The world point of a section that is a sketch's vertex, else None."""
        qs = qunion(v)
        if len(qs) != 1:
            return None
        q = self.deref(qs[0], env)
        if not (isinstance(q, fs.Call) and q[0] == "sQuery" and q[1][1] == ("name", "VERTEX")):
            return None
        fid = q[1][0][1].split(".")[0]
        if fid not in self.sk:
            raise Unresolved("a loft's point on an unread sketch")
        arr, fr = self.sk[fid]
        name = q[1][2]
        e = arr.every.get(name)
        if e is not None and e.kind == "point":
            return fr.world(*e.a)
        if name.endswith(".center") and name[:-7] in arr.every and arr.every[name[:-7]].c is not None:
            return fr.world(*arr.every[name[:-7]].c)
        if name.endswith(".start") and name[:-6] in arr.every:
            return fr.world(*arr.every[name[:-6]].a)
        if name.endswith(".end") and name[:-4] in arr.every:
            return fr.world(*arr.every[name[:-4]].b)
        raise Unresolved("a loft's point")

    def section(self, v, env):
        """One loft section: a single region's loops as world-free segment lists on its sketch's frame."""
        by = self.regions(v, env)
        if len(by) != 1:
            raise Unsupported("a loft profile of many sketches")
        fid = next(iter(by))
        arr, fr = self.sk[fid]
        ps = arr.merged(sorted(set(by[fid])))
        if len(ps) != 1:
            raise Unsupported("a loft profile of many regions")
        loops = [segs_of(arr, lp) for lp in ps[0]]
        ents = {arr.pieces[i].ent for lp in ps[0] for i, _ in lp}
        sec = {"fr": fr, "loops": loops, "ents": ents, "fid": fid}
        pts = [q for s in loops[0] for q in seg_samples(s)]
        c = (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))
        sec["centre"] = fr.world(*c)
        return sec

    def fillet(self, f):
        a, env = f["args"], f["env"]
        if a.get("isVariable") is True or "vertexSettings" in a or "crossSection" in a or "filletType" in a:
            raise Unsupported("a variable or conic fillet")
        hs = self.edges_for(qunion(a["entities"]), env, a.get("tangentPropagation") is True)
        self.lines.append(f"fillet {num(self.val(a['radius']))} " + " ".join(hs))

    def chamfer(self, f):
        a, env = f["args"], f["env"]
        if "chamferType" in a and not a["chamferType"][1].endswith("EQUAL_OFFSETS"):
            raise Unsupported("an unequal chamfer")
        hs = self.edges_for(qunion(a["entities"]), env, a.get("tangentPropagation") is True)
        self.lines.append(f"chamfer {num(self.val(a['width']))} " + " ".join(hs))

    def shell(self, f):
        a, env = f["args"], f["env"]
        if a.get("isHollow") is True:
            raise Unsupported("a hollow with no open face")
        hs = self.faces_for(qunion(a["entities"]), env)
        out = " out" if a.get("oppositeDirection") is True else ""
        self.lines.append(f"shell {num(self.val(a['thickness']))}{out} " + " ".join(hs))

    def body_of(self, q0, env):
        """The label of the body a query names."""
        q = self.deref(q0, env)
        if not (isinstance(q, fs.Call) and q[0] == "makeQuery" and q[1][2] == ("name", "BODY")):
            raise Unresolved("a body query form")
        fid, kind, m = q[1][0][1], q[1][1], q[1][3]
        if kind == "COPY":
            if ".opPattern" in fid:
                k = (fid.split(".")[0], m.get("instanceName"), self.body_of(m["derivedFrom"], env))
                if k not in self.copies:
                    raise Unresolved("a pattern's body")
                return self.alias_of(self.copies[k])
            return self.body_of(m["derivedFrom"], env)
        if kind != "SWEPT_BODY":
            raise Unresolved("body query " + kind)
        rec = self.solid.get(fid.split(".")[0])
        if rec is None or not rec.get("made"):
            raise Unresolved("the body of an unknown feature")
        names = {x[1][2] for x in self.osd(m, env) if isinstance(x, fs.Call) and x[0] == "sQuery"}
        best = max(rec["made"], key=lambda t: len(t[0] & names))
        return self.alias_of(best[1])

    def alias_of(self, b):
        while b in self.alias:
            b = self.alias[b]
        return b

    def one_body(self, f, key="entities"):
        """Patterns and motions take whole bodies, and only a part of one body can say so yet."""
        a = f["args"]
        if a.get("patternType") and "PART" not in a["patternType"][1]:
            raise Unsupported("a face or feature pattern")
        if key not in a:
            raise Unsupported("a face or feature pattern")
        bs = {self.body_of(q, f["env"]) for q in qunion(a[key])}
        if len(bs) != 1 or self.bodies != 1:
            raise Unsupported("a pattern of many bodies")

    def world_line(self, q0, env):
        q = self.deref(q0, env)
        if not (isinstance(q, fs.Call) and q[0] == "sQuery"):
            raise Unsupported("an axis on a solid's edge")
        _, _, fr, e = self.ent(q)
        if e.kind != "line":
            raise Unresolved("an axis not a line")
        p0, p1 = fr.world(*e.a), fr.world(*e.b)
        d = add(p1, p0, -1)
        l = math.sqrt(dot(d, d))
        return p0, tuple(x / l for x in d)

    def mirror(self, f):
        a, env = f["args"], f["env"]
        self.one_body(f)
        if self.op_of(a) not in ("new", "add"):
            raise Unsupported("a mirror that cuts")
        c = self.carrier(single(a["mirrorPlane"], env), env)
        if not isinstance(c, OnPlane):
            raise Unsupported("a mirror in a curved face")
        self.lines.append(f"pattern body mirror z {vtext(c.n)} at {vtext(c.o)}")

    def circularPattern(self, f):
        a, env = f["args"], f["env"]
        self.one_body(f)
        if self.op_of(a) not in ("new", "add"):
            raise Unsupported("a pattern that cuts")
        o, d = self.world_line(single(a["axis"], env), env)
        if a.get("oppositeDirection") is True:
            d = tuple(-x for x in d)
        n, ang = int(self.val(a["instanceCount"])), self.val(a["angle"])
        tail = "" if a.get("equalSpace") is True else " step"
        self.lines.append(f"pattern body circ {vtext(o)} {vtext(d)} {n} {num(ang)}{tail}")

    def transform(self, f):
        a, env = f["args"], f["env"]
        t = a.get("transformType")
        if not (isinstance(t, tuple) and t[1].endswith("TRANSLATION_3D")):
            raise Unsupported("transform " + str(t[1] if isinstance(t, tuple) else t).split(".")[-1])
        by = f"by {num(self.val(a['dx']))} {num(self.val(a['dy']))} {num(self.val(a['dz']))}"
        for q in qunion(a["entities"]):
            b = self.body_of(q, env)
            if a.get("makeCopy") is True:
                self.names += 1
                n = f"b{self.names}"
                self.lines.append(f"{n}: copybody {by} in {b}")
                self.bodies += 1
                q1 = self.deref(q, env)
                self.copies[(f["id"], "1", b)] = n
                self.current = n
            else:
                self.lines.append(f"movebody {by} in {b}")

    def booleanBodies(self, f):
        a, env = f["args"], f["env"]
        t = a.get("operationType")
        t = t[1].split(".")[-1] if isinstance(t, tuple) else "UNION"
        if a.get("offset") is True:
            raise Unsupported("a boolean with offset")
        tools = [self.body_of(q, env) for q in qunion(a["tools"])]
        keep = " keep" if a.get("keepTools") is True else ""
        if t in ("UNION", "INTERSECTION"):
            if len(tools) < 2:
                raise Unresolved("a boolean of one")
            word = "add" if t == "UNION" else "keep"
            self.lines.append(f"combine {word} {tools[0]} " + " ".join(tools[1:]) + keep)
            for b in tools[1:]:
                if not keep:
                    self.alias[b] = tools[0]
                    self.bodies -= 1
            self.current = tools[0]
        elif t == "SUBTRACTION":
            targets = [self.body_of(q, env) for q in qunion(a["targets"])]
            for g in targets:
                self.lines.append(f"combine cut {g} " + " ".join(tools) + keep)
            if not keep:
                self.bodies -= len(tools)
        else:
            raise Unsupported("boolean " + t)

    def hole(self, f):
        a, env = f["args"], f["env"]
        style = a.get("style", ("name", "SIMPLE"))[1].split(".")[-1]
        end = a.get("endStyle", ("name", "BLIND"))[1].split(".")[-1]
        d = self.val(a["holeDiameter"])
        seat = ""
        if style == "C_BORE":
            seat = f" cb {num(self.val(a['cBoreDiameter']))} {num(self.val(a['cBoreDepth']))}"
        elif style == "C_SINK":
            seat = f" cs {num(self.val(a['cSinkDiameter']))} {num(self.val(a['cSinkAngle']))}"
        if end == "THROUGH":
            ext = "thru"
        elif end == "BLIND":
            ext = num(self.val(a["holeDepth"])) + " pt 118"
        elif end == "UP_TO_NEXT":
            ext = "next"
        else:
            raise Unsupported("hole end " + end)
        for q0 in qunion(a["locations"]):
            q = self.deref(q0, env)
            if not (isinstance(q, fs.Call) and q[0] == "sQuery"):
                raise Unsupported("a hole at a solid's vertex")
            fid = q[1][0][1].split(".")[0]
            if fid not in self.sk:
                raise Unresolved("a hole on an unread sketch")
            arr, fr = self.sk[fid]
            name = q[1][2]
            e = arr.every.get(name)
            if e is not None and e.kind == "point":
                u, v = e.a
            elif name.endswith(".center") and name[:-7] in arr.every and arr.every[name[:-7]].c is not None:
                u, v = arr.every[name[:-7]].c
            elif name.endswith(".start") and name[:-6] in arr.every:
                u, v = arr.every[name[:-6]].a
            elif name.endswith(".end") and name[:-4] in arr.every:
                u, v = arr.every[name[:-4]].b
            else:
                raise Unresolved("a hole's location")
            s = 1.0 if a.get("oppositeDirection") is True else -1.0
            z = tuple(s * x for x in fr.n)
            self.lines.append(f"hole {num(d)} z {vtext(z)} x {vtext(fr.x)} at {vtext(fr.world(u, v))}{seat} {ext}")

    def cPlane(self, f):
        a, env = f["args"], f["env"]
        t = a.get("cplaneType", ("name", "OFFSET"))[1].split(".")[-1]
        if t != "OFFSET":
            return
        try:
            c = self.carrier(single(a["entities"], env), env)
        except Unresolved:
            return
        if isinstance(c, OnPlane):
            s = -1.0 if a.get("oppositeDirection") is True else 1.0
            self.planes[f["id"]] = OnPlane(add(c.o, c.n, s * self.val(a["offset"])), c.n)

    def run(self):
        for f in self.feats:
            if f["op"] not in SUPPORTED:
                raise Unsupported("feature " + f["op"])
        for f in self.feats:
            self.kinds.append(f["op"])
            if f["op"] == "sketch":
                self.sketch(f)
            elif f["op"] == "assignVariable":
                a = f["args"]
                self.vars[a.get("name")] = a.get("anyValue", a.get("lengthValue", a.get("numberValue", a.get("angleValue"))))
            elif f["op"] == "cPoint":
                pass
            else:
                getattr(self, f["op"])(f)
        if not self.lines:
            raise Unsupported("no solid feature")


def near3(a, b):
    return math.dist(a, b) < 1e-4


def is_ring(p):
    return p.c is not None and near3(p.a, p.b) and abs(p.sweep) > 1


def edge_piece(e):
    """A kernel edge as a path piece, from its samples."""
    p = [tuple(x) for x in e["p"]]
    if e["k"] == "line":
        return WPiece(p[0], p[-1], ent=e["h"])
    if e["k"] != "circle":
        raise Unsupported("a path along a spline")
    a, m, b = p[0], p[len(p) // 2], p[-1]
    if near3(a, b):
        m, b2 = p[len(p) // 3], p[2 * len(p) // 3]
        c, n = circle3(a, m, b2)
        return WPiece(a, a, c, n, 2 * math.pi, ent=e["h"])
    c, n = circle3(a, m, b)
    r0, rm = add(a, c, -1), add(m, c, -1)
    return WPiece(a, b, c, n, 2 * math.atan2(dot(cross(r0, rm), n), dot(r0, rm)), ent=e["h"])


def circle3(a, m, b):
    """Centre and normal (a to m to b turning positively) of the circle through three points."""
    u, v = add(m, a, -1), add(b, a, -1)
    n = cross(u, v)
    nn = dot(n, n)
    t = add(tuple(x * dot(u, u) for x in cross(v, n)), tuple(x * dot(v, v) for x in cross(n, u)))
    return add(a, tuple(x / (2 * nn) for x in t)), unit(n)


def part(p, t, side):
    """The part of a piece after (side 1) or before (side 0) its fraction t."""
    if p.c is None:
        q = add(p.a, add(p.b, p.a, -1), t)
        return WPiece(q, p.b, ent=p.ent) if side else WPiece(p.a, q, ent=p.ent)
    w = p.sweep * t
    q = add(p.c, turned(add(p.a, p.c, -1), p.z, w))
    return WPiece(q, p.b, p.c, p.z, p.sweep - w, p.ent) if side else WPiece(p.a, q, p.c, p.z, w, p.ent)


def tangent_fit(path):
    """Each piece rebuilt from the one before's end and end tangent, so the chain is tangent to the last bit: the data
    is exact only to rounding, the kernel's tangency check is not. A real corner stays a corner, which has no form."""
    out = [path[0]]
    for p in path[1:]:
        q = out[-1]
        s, t = q.b, q.tangent(q.b)
        t1 = p.tangent(p.a)
        if dot(t, t1) <= 0 or math.dist(t, t1) > 1e-6:
            raise Unsupported("a path with a corner")
        if p.c is None:
            e = add(s, t, math.dist(p.a, p.b))
            out.append(WPiece(s, e, ent=p.ent))
        else:
            # the circle through s, tangent t there, to the same end
            nn = unit(cross(p.z if p.sweep > 0 else tuple(-x for x in p.z), t))
            ch = add(p.b, s, -1)
            r = dot(ch, ch) / (2 * dot(ch, nn))
            out.append(WPiece(s, p.b, add(s, nn, r), p.z, p.sweep, p.ent))
    return out


def legs_of(fr, path):
    """The sections a sweep's stretches start from: (frame, piece). Frames carried along each bend."""
    legs = []
    for p in path:
        legs.append((fr, p))
        if p.c is not None:
            fr = Frame.of(add(p.c, turned(add(fr.o, p.c, -1), p.z, p.sweep)), turned(fr.x, p.z, p.sweep), turned(fr.y, p.z, p.sweep), turned(fr.n, p.z, p.sweep))
        else:
            fr = Frame.of(add(fr.o, add(p.b, p.a, -1)), fr.x, fr.y, fr.n)
    legs.append((fr, None))
    return legs


def segs_of(arr, lp):
    """A loop as plain segments in its sketch's plane: (start, end, centre or None, signed sweep). A circle as two arcs."""
    out = []
    for i, fwd in lp:
        p = arr.pieces[i]
        if p.kind == "circle":
            if len(lp) != 1:
                raise Unsupported("a circle in a longer loop")
            s = (p.c[0] + p.r, p.c[1])
            e = (p.c[0] - p.r, p.c[1])
            d = math.pi if fwd else -math.pi
            return [(s, e, p.c, d), (e, s, p.c, d)]
        s, e = arr.verts[p.va], arr.verts[p.vb]
        if not fwd:
            s, e = e, s
        if p.kind == "poly":
            raise Unsupported("a loft between splines")
        if p.kind == "arc":
            _, d = p.sweep()
            out.append((s, e, p.c, d if fwd else -d))
        else:
            out.append((s, e, None, 0.0))
    return out


def seg_samples(s, n=8):
    if s[2] is None:
        return [s[0]]
    a0 = math.atan2(s[0][1] - s[2][1], s[0][0] - s[2][0])
    r = math.dist(s[0], s[2])
    return [(s[2][0] + r * math.cos(a0 + s[3] * i / n), s[2][1] + r * math.sin(a0 + s[3] * i / n)) for i in range(n)]


def face(sec, flip):
    """The section's frame turned to face the way the loft goes, its loops kept counter-clockwise (holes clockwise)
    about that, each from the vertex it started at."""
    if not flip:
        return
    fr = sec["fr"]
    sec["fr"] = Frame.of(fr.o, fr.x, tuple(-x for x in fr.y), tuple(-x for x in fr.n))
    m = lambda p: (p[0], -p[1])
    # mirrored in x keeps the sense of each arc's travel reversed twice over: run backwards, then seen from behind
    sec["loops"] = [[(m(e), m(s), None if c is None else m(c), d) for s, e, c, d in reversed(lp)] for lp in sec["loops"]]


def whole_circle(lp):
    return len(lp) == 2 and lp[0][2] is not None and lp[0][2] == lp[1][2] and abs(abs(lp[0][3]) - math.pi) < 1e-12


def align(a, b):
    """Turn b's loops to start where a's do, and put them in a's order. Parasolid's own matching is not recorded: the
    rotation with the least distance between matched vertices, each section about its centre, is the assumption here."""
    if len(a["loops"]) != len(b["loops"]):
        raise Unsupported("a loft between profiles with different holes")
    ca, cb = a["centre"], b["centre"]
    fa, fb = a["fr"], b["fr"]
    wa = lambda p: add(fa.world(*p), ca, -1)
    wb = lambda p: add(fb.world(*p), cb, -1)
    rest = b["loops"][1:]
    order = []
    for la in a["loops"][1:]:
        m = wa(la[0][0])
        k = min(range(len(rest)), key=lambda j: math.dist(m, wb(rest[j][0][0])))
        order.append(rest.pop(k))
    b["loops"] = b["loops"][:1] + order
    for k in range(len(a["loops"])):
        la, lb = a["loops"][k], b["loops"][k]
        if len(la) != len(lb) or whole_circle(la) != whole_circle(lb):
            # a circle against corners: the source's rulings run each side to the arc between the points of the circle
            # in line with its corners, at the same fraction along both, which is what the kernel's wall does
            if whole_circle(lb):
                b["loops"][k] = split_circle(lb, la, fa, ca, fb, cb)
            elif whole_circle(la) and not a.get("matched"):
                a["loops"][k] = split_circle(la, lb, fb, cb, fa, ca)
            else:
                raise Unsupported("a loft between profiles of different segment counts")
            continue
        if whole_circle(lb) and whole_circle(la):
            # split b's circle at the point nearest a's start
            c, d = lb[0][2], lb[0][3]
            r = math.dist(lb[0][0], c)
            to = add(fa.world(*la[0][0]), ca, -1)
            u, v = dot(to, fb.x), dot(to, fb.y)
            l = math.hypot(u, v) or 1.0
            s, e = (c[0] + r * u / l, c[1] + r * v / l), (c[0] - r * u / l, c[1] - r * v / l)
            b["loops"][k] = [(s, e, c, d), (e, s, c, d)]
            continue
        n = len(lb)
        cost = lambda r: sum(math.dist(wa(la[i][0]), wb(lb[(i + r) % n][0])) ** 2 for i in range(n))
        r = min(range(n), key=cost)
        b["loops"][k] = lb[r:] + lb[:r]


def split_circle(circ, other, fo, co, fc, cc):
    """A circle's two arcs cut where the other loop's corners point from its centre, in that loop's order."""
    c, d = circ[0][2], math.copysign(1.0, circ[0][3])
    r = math.dist(circ[0][0], c)
    pts, angs = [], []
    for s in other:
        to = add(fo.world(*s[0]), co, -1)
        u, v = dot(to, fc.x), dot(to, fc.y)
        l = math.hypot(u, v)
        if l < 1e-9:
            raise Unsupported("a loft from a corner on the circle's axis")
        pts.append((c[0] + r * u / l, c[1] + r * v / l))
        angs.append(math.atan2(v, u))
    out = []
    total = 0.0
    for i in range(len(pts)):
        w = (angs[(i + 1) % len(pts)] - angs[i]) * d
        w = w % (2 * math.pi)
        if w < 1e-6:
            raise Unsupported("a loft between profiles of different segment counts")
        total += w
        out.append((pts[i], pts[(i + 1) % len(pts)], c, d * w))
    if abs(total - 2 * math.pi) > 1e-6:
        raise Unsupported("a loft between profiles of different segment counts")
    return out


def section_text(sec):
    fr = sec["fr"]
    out = [f"z {vfine(fr.n)} x {vfine(fr.x)} at {vfine(fr.o)}"]
    for lp in sec["loops"]:
        parts = []
        for s, e, c, d in lp:
            if c is None:
                parts.append(f"line {fine(s[0])} {fine(s[1])} {fine(e[0])} {fine(e[1])}")
            else:
                a0 = math.degrees(math.atan2(s[1] - c[1], s[0] - c[0]))
                parts.append(f"arc {fine(c[0])} {fine(c[1])} {fine(math.dist(s, c))} {fine(a0)} {fine(math.degrees(d))}")
        out.append("loop " + " ".join(parts))
    return " ".join(out)


def a_id(t, a):
    """The sketch an arrangement belongs to."""
    for fid, (x, _) in t.sk.items():
        if x is a:
            return fid
    return None


def span(lo, hi):
    if lo is None or hi is None:
        return (None, None)
    return (min(lo, hi), max(lo, hi))


def qunion(v):
    if isinstance(v, fs.Call) and v[0] == "qUnion":
        return list(v[1][0])
    return [v]


def single(v, env):
    q = qunion(v)
    if len(q) != 1:
        raise Unresolved("a reference of many")
    return q[0]


def translate(text, raw, topo, out):
    r = {"status": "translated", "notes": [], "why": ""}
    t = None
    try:
        t = Tr(text, raw, topo)
        r["features"] = [f["op"] for f in t.feats if f["op"] not in ("sketch", "cPlane", "cPoint", "assignVariable")]
        t.run()
    except fs.Bad as x:
        r.update(status="untranslated", cls="importer", why="reader: " + str(x)[:50])
    except Unsupported as x:
        r.update(status="untranslated", cls="form", why=str(x))
    except Unresolved as x:
        r.update(status="untranslated", cls="reference", why=str(x))
    except KernelStop as x:
        r.update(status="stopped", why=str(x)[:200])
    except (KeyError, IndexError, TypeError, ValueError, ZeroDivisionError, AttributeError, RecursionError) as x:
        r.update(status="untranslated", cls="importer", why="bug: " + repr(x)[:70])
    if t is not None:
        r["notes"] = sorted(t.notes)
        if t.topo is not None:
            t.topo.kill()
            t.topo.wait()
        if r["status"] in ("translated", "stopped"):
            with open(out, "w") as f:
                f.write("\n".join(t.lines) + "\n")
    return r


def main():
    a = sys.argv[1:]
    opt = lambda k: a[a.index(k) + 1]
    text = open(a[0], encoding="utf8", errors="replace").read()
    raw = json.load(open(a[1]))
    print(json.dumps(translate(text, raw, opt("--topo"), opt("--out"))))


if __name__ == "__main__":
    main()
