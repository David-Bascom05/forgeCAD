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
SUPPORTED = {"sketch", "extrude", "revolve", "fillet", "chamfer", "shell", "cPlane", "cPoint", "assignVariable", "hole", "mirror", "circularPattern", "transform", "booleanBodies"}
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
        best, score = None, 0
        # the plane it was drawn on rules out look-alike sketches
        try:
            plane = self.carrier(single(f["args"]["sketchPlane"], f["env"]), f["env"])
        except (Unresolved, KeyError, Unsupported):
            plane = None
        for k, s in enumerate(self.raw.get("sketches", [])):
            if k in self.used or "sketchMatrix" not in s:
                continue
            if isinstance(plane, OnPlane):
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
            c = len(ours & theirs)
            if c > score:
                best, score = k, c
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
                hit = [f for f in faces if all(c.dist(p) <= TOL for p in f["p"])]
                if not hit:
                    raise Unresolved("no face on the carrier: " + q[1][1])
                for f in hit:
                    for e in edges:
                        if f["h"] in e["f"] and e["c"] != "smooth":
                            picked[e["h"]] = e
            else:
                hit = [e for e in edges if all(c.dist(p) <= TOL for p in e["p"])]
                if not hit:
                    raise Unresolved("no edge on the carrier: " + (q[1][1] if q[0] == "makeQuery" else q[0]))
                if len(hit) > 1 and getattr(c, "split_pick", False):
                    raise Unresolved("a split piece the order would have to pick")
                for e in hit:
                    picked[e["h"]] = e
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
        faces = self.state().get("faces", [])
        out = []
        for q0 in qs:
            c = self.carrier(q0, env)
            hit = [f["h"] for f in faces if all(c.dist(p) <= TOL for p in f["p"])]
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
            elif isinstance(q, fs.Call) and q[0] == "makeQuery":
                raise Unsupported("a profile from a solid's face")
            else:
                raise Unresolved("profile query " + (q[0] if isinstance(q, tuple) else "?"))
        return by

    def imprint(self, q, env):
        fid = q[1][0][1].split(".")[0]
        if fid not in self.sk:
            raise Unresolved("an imprint of an unread sketch")
        a = self.sk[fid][0]
        cands = None
        for d in q[1][3].get("disambiguationData", []):
            if d[0] != "TD":
                continue
            for item in d[1][0]:
                eq, w = self.deref(item[0], env), item[1]
                rs = self.side_regions(a, eq, w, env)
                cands = rs if cands is None else cands & rs
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

    def emit_solid(self, kind, op, head, frame_text, by, flip, tail="", fid_out=None):
        for fid, regs in by.items():
            a = self.sk[fid][0]
            for pi, prof in enumerate(a.merged(sorted(set(regs)))):
                loops = " ".join(self.loop_text(a, lp, flip) for lp in prof)
                this = op
                label = ""
                ents = {a.pieces[i].ent for lp in prof for i, _ in lp}
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
                scope = " in all" if self.bodies > 1 and op in ("cut", "keep") else ""
                self.lines.append(f"{label}{kind} {this + ' ' if this else ''}{head} {frame_text} {tail}{loops}{scope}")

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
