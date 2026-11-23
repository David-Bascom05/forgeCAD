"""The recreate importers' Forge (.fg) writer and their link to the kernel (forge-proof's `topo`).

An importer records each feature as a statement (`St`): what it makes, on which frame, from which
loops, with its numbers as the importer writes them (`cadfs.num`, `cadfs.fine`), so the part reads
the very values a hand-written program would. `Prog.text` writes the program:

- A frame (a sketch plane: origin, normal, x) is `plane(at: P, normal: axis(ref.origin, N), x:
  axis(ref.origin, X))`, each point a helper sketch's fixed point (`q3.p`, on the world plane `ref`
  moved up to its z), so the kernel gets the frame the importer worked out, its coordinates kept.
- A profile is a sketch on its frame: lines, arcs (centre, ends) and circles as the arrangement
  traced them; a revolve's axis a construction line of it.
- An extrude runs along its frame's normal whatever it does: a cut's depth is written negative
  (Forge cuts against the normal), `thru` and `next` flipped, two-sided lengths negated.
- A hole drills along its frame's normal: its plane is the frame turned over (Forge drills against
  the normal), x kept.
- Fillets, chamfers and shells name their edges and faces by lineage (`edges(of: F, ..)`,
  `edges(between: (A, B))`, anchors, pattern copies `P[k]`), worked out by `topo` on the part as
  built so far (forge_import::state), never by position.

Bodies: a statement's id is its label (`b2`, `t3`) or one of its own; `scope` (a label, or "all")
says which bodies it acts on, as `bodies:`.
"""
import decimal
import json
import math
import os
import resource
import subprocess


class Stop(Exception):
    """The kernel stopped on the program so far."""


class NoName(Exception):
    """No lineage selection names exactly these edges or faces."""


def exact(v):
    """A float with every digit, no exponent."""
    v = float(v) + 0.0
    if v == int(v) and abs(v) < 1e15:
        return str(int(v))
    return format(decimal.Decimal(repr(v)), "f")


def neg(t):
    """A number's text negated, exactly."""
    if t.startswith("-"):
        return t[1:]
    return "0" if float(t) == 0 else "-" + t


def is_zero(t):
    return float(t) == 0.0


class St:
    """One statement: `kind` and its fields (see Prog.one)."""

    def __init__(self, kind, label=None, op="", scope=None, **kw):
        self.kind, self.label, self.op, self.scope = kind, label, op, scope
        self.__dict__.update(kw)

    def __repr__(self):
        return f"St({self.kind} {self.label} {self.op})"


# ---- loops: an arrangement's loops as segments in the frame's coordinates ----


def loop_segs(a, lp, flip, fmt):
    """A loop of arrangement `a` (pieces and whether each runs forward) as segments, each number as
    `fmt` writes it: ("circle", c, r), ("line", s, e), ("arc", c, s, e, ccw). `flip` mirrors v."""
    f = (lambda p: (p[0], -p[1])) if flip else (lambda p: p)
    q = lambda p: (fmt(p[0]), fmt(p[1]))
    if len(lp) == 1 and a.pieces[lp[0][0]].kind == "circle":
        p = a.pieces[lp[0][0]]
        return [("circle", q(f(p.c)), fmt(p.r))]
    out = []
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
            if ch > 1e-9:
                r = max(p.r, ch / 2)
                h = math.sqrt(max(r * r - ch * ch / 4, 0.0))
                m = ((s[0] + e[0]) / 2, (s[1] + e[1]) / 2)
                n = ((e[1] - s[1]) / ch, (s[0] - e[0]) / ch)
                c1, c2 = (m[0] + h * n[0], m[1] + h * n[1]), (m[0] - h * n[0], m[1] - h * n[1])
                c = c1 if math.dist(c1, c) <= math.dist(c2, c) else c2
            out.append(("arc", q(c), q(s), q(e), d > 0))
        elif p.kind == "poly" and p.src is not None and p.src[0] == "nurbs":
            # the whole of a NURBS: its poles, knots and weights, run the loop's way. A clamped one's end poles are
            # its ends, set to the loop's own; an unclamped or periodic one's poles are written as they are
            _, deg, poles, knots, weights = p.src
            if not fwd:
                poles, weights = poles[::-1], weights[::-1]
                knots = [knots[0] + knots[-1] - k for k in knots[::-1]]
            ps = [f(x) for x in poles]
            if knots[: deg + 1] == [knots[0]] * (deg + 1) and knots[-deg - 1 :] == [knots[-1]] * (deg + 1):
                ps[0], ps[-1] = s, e
            ws = None if all(w == 1.0 for w in weights) else [fmt(w) for w in weights]
            out.append(("nurbs", [q(x) for x in ps], deg, [fmt(k) for k in knots], ws))
        elif p.kind == "poly" and p.src is not None and p.src[0] == "ellipse":
            # the whole of an ellipse or elliptical arc: its centre, a major axis end and its minor semi-axis
            _, c, u, ra, rb, sense, _, _, whole = p.src
            m = f((c[0] + ra * u[0], c[1] + ra * u[1]))
            if whole:
                out.append(("ellipse", q(f(c)), q(m), fmt(rb)))
            else:
                ccw = (sense > 0) == fwd
                out.append(("earc", q(f(c)), q(m), fmt(rb), q(s), q(e), ccw != flip))
        elif p.kind == "poly" and p.src is not None and p.src[0] == "through":
            # the whole of a fit spline marked exact: its own points (and end slopes), run the loop's way. CADFS's
            # stay polylines: written exactly they turned a match into a shell refusal and two parts into mesh
            # failures (research/import/coverage.md)
            _, pts, d0, d1 = p.src
            ps = [f(x) for x in (pts if fwd else pts[::-1])]
            if not fwd:
                d0, d1 = (None if d1 is None else (-d1[0], -d1[1])), (None if d0 is None else (-d0[0], -d0[1]))
            ps[0], ps[-1] = s, e
            # Forge's slope is the rate along the curve: the derivative over the length through the points, as
            # cadfs.section_of writes a loft's
            length = sum(math.dist(u, v) for u, v in zip(pts, pts[1:])) or 1.0
            fl = lambda v: None if v is None else q((v[0] / length, (-v[1] if flip else v[1]) / length))
            out.append(("spline", [q(x) for x in ps], fl(d0), fl(d1)))
        elif p.kind == "poly":
            ps = [f(x) for x in (p.pts if fwd else p.pts[::-1])]
            ps[0], ps[-1] = s, e
            for u, v in zip(ps, ps[1:]):
                if q(u) != q(v):
                    out.append(("line", q(u), q(v)))
        else:
            out.append(("line", q(s), q(e)))
    return out


def pt2(p):
    return f"({p[0]}, {p[1]})"


def arc_centre(c, s, e):
    """The centre to write for an arc through s and e (texts): c where it lies on their bisector as
    written, else the point of the bisector nearest it, every digit kept (the solver holds an arc's
    centre equally far from its ends). Nearest, not at the written radius: by a half circle the
    radius barely fixes the centre, and a rounding of it would move the centre far along."""
    fc, fs, fe = [(float(x), float(y)) for x, y in (c, s, e)]
    if abs(math.dist(fc, fs) - math.dist(fc, fe)) <= 1e-12:
        return pt2(c)
    ch = math.dist(fs, fe)
    if ch <= 0:
        return pt2(c)
    m = ((fs[0] + fe[0]) / 2, (fs[1] + fe[1]) / 2)
    n = ((fe[1] - fs[1]) / ch, (fs[0] - fe[0]) / ch)
    h = (fc[0] - m[0]) * n[0] + (fc[1] - m[1]) * n[1]
    return f"({exact(m[0] + h * n[0])}, {exact(m[1] + h * n[1])})"


def rows_of(loops):
    """Sketch rows for loops of segments."""
    rows = []
    for lp in loops:
        for sg in lp:
            k = len(rows)
            if isinstance(sg, str):
                # a row as written (`rect(corner: (0, 0), size: (w, h))`)
                rows.append(f"r{k} = {sg}")
            elif sg[0] == "circle":
                rows.append(f"c{k} = circle({pt2(sg[1])}, {sg[2]})")
            elif sg[0] == "line":
                rows.append(f"l{k} = line({pt2(sg[1])}, {pt2(sg[2])})")
            elif sg[0] == "arc":
                _, c, s, e, ccw = sg
                cw = "" if ccw else ", cw: true"
                rows.append(f"a{k} = arc(center: {arc_centre(c, s, e)}, from: {pt2(s)}, to: {pt2(e)}{cw})")
            elif sg[0] == "ellipse":
                _, c, m, b = sg
                rows.append(f"e{k} = ellipse(center: {pt2(c)}, major: {pt2(m)}, b: {b})")
            elif sg[0] == "earc":
                _, c, m, b, s, e, ccw = sg
                cw = "" if ccw else ", cw: true"
                rows.append(f"e{k} = earc(center: {pt2(c)}, major: {pt2(m)}, b: {b}, from: {pt2(s)}, to: {pt2(e)}{cw})")
            elif sg[0] == "nurbs":
                # ("nurbs", [poles], degree, [knots], [weights] or None)
                _, ps, deg, ks, ws = sg
                extra = f", degree: {deg}, knots: [{', '.join(ks)}]"
                if ws is not None:
                    extra += f", weights: [{', '.join(ws)}]"
                rows.append(f"s{k} = spline([{', '.join(pt2(p) for p in ps)}]{extra})")
            elif sg[0] == "spline":
                # ("spline", [points], slope at start or None, at end or None)
                _, ps, d0, d1 = sg
                closed = len(ps) > 3 and ps[0] == ps[-1]
                body = ", ".join(pt2(p) for p in (ps[:-1] if closed else ps))
                # a through-points spline is closed unless told otherwise, or given an end slope
                sloped = d0 is not None or d1 is not None
                extra = ", closed: true" if closed and not sloped else ("" if closed or sloped else ", closed: false")
                if d0 is not None:
                    extra += f", start_slope: {pt2(d0)}"
                if d1 is not None:
                    extra += f", end_slope: {pt2(d1)}"
                rows.append(f"s{k} = spline([{body}]{extra})")
            else:
                raise ValueError(sg[0])
    return rows


# ---- the program ----

PRELUDE = "sketch ref(on: xy) {\n}\n"


class Prog:
    """Writes statements as one Forge program."""

    def __init__(self):
        self.out = []
        self.points = {}
        self.made = 0
        self.cur = None
        self.solids = 0

    def point(self, p):
        """A world point (three number texts) as a helper sketch's point."""
        x, y, z = p
        if all(is_zero(t) for t in p):
            return "ref.origin"
        key = (x, y, z)
        if key in self.points:
            return self.points[key]
        n = f"q{len(self.points)}"
        on = "ref" if is_zero(z) else f"offset(ref, {z})"
        self.out.append(f"sketch {n}(on: {on}) {{\n  p = pt({x}, {y})\n}}")
        self.points[key] = f"{n}.p"
        return self.points[key]

    def dir(self, v):
        return f"axis(ref.origin, {self.point(v)})"

    def plane(self, fr):
        """A frame (origin, normal, x or None: three number texts each) as a plane."""
        o, z, x = fr
        at = self.point(o)
        t = f"plane(at: {at}, normal: {self.dir(z)}"
        if x is not None:
            t += f", x: {self.dir(x)}"
        return t + ")"

    def sketch(self, sid, fr, rows):
        on = self.plane(fr)
        body = "".join(f"  {r}\n" for r in rows)
        self.out.append(f"sketch {sid}(on: {on}) {{\n{body}}}")
        return sid

    def fresh(self, st):
        if st.label:
            return st.label
        self.made += 1
        return f"f{self.made}"

    def opts(self, st, made_body):
        """The op and scope arguments."""
        o = ""
        if st.scope == "all":
            o += ', bodies: "all"'
        elif st.scope:
            o += f", bodies: {st.scope}"
        return o

    def solid_op(self, st):
        """For a statement adding material: whether it makes a body (`new: true` where the part has one)."""
        first = self.solids == 0
        if st.op in ("cut", "keep"):
            return False, ""
        self.solids += 1
        if st.op == "new" and not first:
            return True, ", new: true"
        return first, ""

    def text(self, stmts):
        self.out, self.points, self.made, self.cur, self.solids = [], {}, 0, None, 0
        for st in stmts:
            self.one(st)
        return PRELUDE + "".join(s + "\n" for s in self.out)

    def one(self, st):
        k = st.kind
        i = self.fresh(st)
        if k == "extrude":
            sk = self.sketch(f"{i}_sk", st.frame, rows_of(st.loops))
            made, new = self.solid_op(st)
            cut = st.op == "cut"
            call = {"cut": "cut", "keep": "intersect"}.get(st.op, "extrude")
            e = st.ext
            w = e[0]
            more = ""
            if w == "d":
                x = neg(e[1]) if cut else e[1]
            elif w == "sym":
                x = f"sym({e[1]})"
            elif w == "two":
                if cut and getattr(st, "along", None):
                    # a slanted cut's two lengths are its back and forward
                    x = f"two({e[2]}, {e[1]})"
                elif cut:
                    x = f"two({neg(e[1])}, {neg(e[2])})"
                else:
                    x = f"two({e[1]}, {e[2]})"
            elif w == "thru":
                x = {"both": "thru_all"}.get(e[1], "thru")
                if e[1] != "both" and ((e[1] == "fwd") == cut):
                    more = ", flip: true"
            elif w == "upto":
                x = f"upto({e[1]})"
            elif w == "next":
                x = "next"
                if (e[1] == "fwd") == cut:
                    more = ", flip: true"
            else:
                raise ValueError(w)
            if getattr(st, "taper", None) is not None:
                more += f", taper: {st.taper}deg"
            if getattr(st, "along", None):
                # the walls slanted along it, each end square to the normal
                more += f", along: {self.dir(st.along)}"
            # a row's own regions (a text's glyphs: `sk.r0`), or the sketch's one
            prof = f"{sk}.{st.member}" if getattr(st, "member", None) else sk
            self.out.append(f"{i} = {call}({prof}, {x}{more}{new}{self.opts(st, made)})")
            if made:
                self.cur = i
        elif k == "revolve":
            ox, oy, dx, dy = st.axis
            bx, by = exact(float(ox) + float(dx)), exact(float(oy) + float(dy))
            rows = rows_of(st.loops) + [f"ax = line(({ox}, {oy}), ({bx}, {by}), construction: true)"]
            sk = self.sketch(f"{i}_sk", st.frame, rows)
            made, new = self.solid_op(st)
            t = st.turn
            ang = ""
            if t is not None:
                ang = {"a": lambda: f", angle: {t[1]}deg", "sym": lambda: f", angle: sym({t[1]}deg)", "two": lambda: f", angle: two({t[1]}deg, {t[2]}deg)"}[t[0]]()
            op = {"cut": ", cut: true", "keep": ", keep: true"}.get(st.op, "")
            self.out.append(f"{i} = revolve({sk}, axis: {sk}.ax{ang}{op}{new}{self.opts(st, made)})")
            if made:
                self.cur = i
        elif k == "coil":
            # wound about a line of its sketch, rising along it: pitch and height
            ox, oy, dx, dy = st.axis
            bx, by = exact(float(ox) + float(dx)), exact(float(oy) + float(dy))
            rows = rows_of(st.loops) + [f"ax = line(({ox}, {oy}), ({bx}, {by}), construction: true)"]
            sk = self.sketch(f"{i}_sk", st.frame, rows)
            made, new = self.solid_op(st)
            op = {"cut": ", cut: true", "keep": ", keep: true"}.get(st.op, "")
            left = ", left: true" if st.left else ""
            self.out.append(f"{i} = coil({sk}, axis: {sk}.ax, pitch: {st.pitch}, height: {st.height}{left}{op}{new}{self.opts(st, made)})")
            if made:
                self.cur = i
        elif k == "sweep":
            sk = self.sketch(f"{i}_sk", st.frame, rows_of(st.loops))
            parts = []
            for j, pc in enumerate(st.path):
                pid = f"{i}_p{j}"
                if pc[0] == "line":
                    a, x, n = pc[1], pc[3], pc[4]
                    self.sketch(pid, (a, n, x), [f"l = line((0, 0), ({pc[5]}, 0))"])
                    parts.append(f"{pid}.l")
                else:
                    fr, r, e = pc[1], pc[2], pc[3]
                    self.sketch(pid, fr, [f"a = arc(center: (0, 0), from: ({r}, 0), to: ({e[0]}, {e[1]}))"])
                    parts.append(f"{pid}.a")
            made, new = self.solid_op(st)
            op = {"cut": ", cut: true", "keep": ", keep: true"}.get(st.op, "")
            self.out.append(f"{i} = sweep({sk}, along: [{', '.join(parts)}]{op}{new}{self.opts(st, made)})")
            if made:
                self.cur = i
        elif k == "loft":
            items = []
            for j, sec in enumerate(st.secs):
                if sec[0] == "point":
                    # a loft closes on a point value; a sketch's point is an entity of it
                    items.append(f"plane(at: {self.point(sec[1])}, normal: Z).origin")
                else:
                    items.append(self.sketch(f"{i}_s{j}", sec[1], rows_of(sec[2])))
            made, new = self.solid_op(st)
            op = {"cut": ", cut: true", "keep": ", keep: true"}.get(st.op, "")
            sm = ", smooth: true" if st.smooth else ""
            for w in ("start", "end"):
                if getattr(st, w, None) is not None:
                    sm += f", {w}: {getattr(st, w)}"
            self.out.append(f"{i} = loft([{', '.join(items)}]{sm}{op}{new}{self.opts(st, made)})")
            if made:
                self.cur = i
        elif k == "ball":
            sk = self.sketch(f"{i}_sk", st.frame, ["c = pt(0, 0)"])
            made, new = self.solid_op(st)
            op = {"cut": ", cut: true", "keep": ", keep: true"}.get(st.op, "")
            self.out.append(f"{i} = ball({sk}.c, {st.r}{op}{new}{self.opts(st, made)})")
            if made:
                self.cur = i
        elif k == "hole":
            o, z, x = st.frame
            sk = self.sketch(f"{i}_sk", (o, tuple(neg(t) for t in z), x), ["m = pt(0, 0)"])
            a = f"at: {sk}.m, d: {st.d}"
            if st.seat:
                if st.seat[0] == "cb":
                    a += f", cbore: ({st.seat[1]}, {st.seat[2]})"
                else:
                    a += f", csink: ({st.seat[1]}, {st.seat[2]}deg)"
            dp = st.depth
            a += f", depth: {dp}"
            if st.point is not None:
                a += f", point: {st.point}deg"
            self.out.append(f"{i} = hole({a}{self.opts(st, False)})")
        elif k in ("fillet", "chamfer"):
            more = ""
            if k == "chamfer" and getattr(st, "corner", None):
                more = f', corner: "{st.corner}"'
            self.out.append(f"{i} = {k}({st.sel}, {st.size}{more})")
        elif k == "draft":
            self.out.append(f"{i} = draft({st.sel}, angle: {st.angle}deg, neutral: {self.plane(st.plane)})")
        elif k == "shell":
            out = ", out: true" if st.out else ""
            self.out.append(f"{i} = shell({st.size}, open: {st.sel}{out})")
        elif k == "pattern":
            lay = st.layout
            if lay[0] == "lin":
                lt = f"lin(along: {self.dir(lay[1])}, n: count({lay[2]}), step: {lay[3]})"
            elif lay[0] == "circ":
                o, d = lay[1], lay[2]
                tip = tuple(exact(float(a) + float(b)) for a, b in zip(o, d))
                lt = f"circ(about: axis({self.point(o)}, {self.point(tip)}), n: count({lay[3]})"
                if lay[4] is not None:
                    lt += f", {'each' if lay[5] else 'over'}: {lay[4]}deg"
                lt += ")"
            else:
                lt = f"mirror(on: {self.plane((lay[2], lay[1], None))})"
            if st.of == "body":
                src = "whole: true"
            else:
                src = "of: " + (st.of[0] if len(st.of) == 1 else "[" + ", ".join(st.of) + "]")
            self.out.append(f"{i} = pattern({src}, layout: {lt}{self.opts(st, False)})")
        elif k == "combine":
            tools = st.tools[0] if len(st.tools) == 1 else "[" + ", ".join(st.tools) + "]"
            keep = ", keep: true" if st.keep else ""
            self.out.append(f'{i} = combine({st.target}, tools: {tools}, op: "{st.word}"{keep})')
            self.cur = st.target
        elif k in ("copybody", "movebody"):
            ms = []
            for m in st.moves:
                if m[0] == "rot":
                    o, d, a = m[1], m[2], m[3]
                    tip = tuple(exact(float(p) + float(q)) for p, q in zip(o, d))
                    ms.append(f"spin(about: axis({self.point(o)}, {self.point(tip)}), by: {a}deg)")
                else:
                    v = tuple(float(t) for t in m[1])
                    l = math.sqrt(sum(t * t for t in v))
                    ms.append(f"slide(along: {self.dir(m[1])}, by: {exact(l)})")
            of = st.of or self.cur
            self.out.append(f"{i} = {k}(of: {of}, moves: [{', '.join(ms)}])")
            if k == "copybody":
                self.solids += 1
                self.cur = i
        else:
            raise ValueError(k)


# ---- reading a program back ----


def statements(src):
    """The program's top-level statements, a sketch's block as one: (first line, text), lines counted from 0
    as the judge's `line` counts them (forge_lang::fuzz::statements)."""
    out, depth = [], 0
    for k, l in enumerate(src.split("\n")):
        if depth > 0:
            out[-1] = (out[-1][0], out[-1][1] + "\n" + l)
        elif l.strip():
            out.append((k, l))
        depth += l.count("{") - l.count("}")
    return out


def at_line(src, line):
    """The index of the statement holding source line `line`, or None."""
    sts = statements(src)
    for i, (k, t) in enumerate(sts):
        if k <= line < k + t.count("\n") + 1:
            return i
    return None


def args(call):
    """A call's arguments at the top level: `f(a(b, c), d)` gives ["a(b, c)", "d"]."""
    i = call.index("(")
    out, depth, cur = [], 0, ""
    for ch in call[i + 1 : call.rindex(")")]:
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def blend(stmt):
    """A fillet's, chamfer's or shell's statement read back: (kind, size, selection text), or None."""
    if " = " not in stmt:
        return None
    rhs = stmt.split(" = ", 1)[1].strip()
    kind = rhs.split("(", 1)[0]
    if kind not in ("fillet", "chamfer", "shell"):
        return None
    a = args(rhs)
    if kind == "shell":
        sel = next((x[6:] for x in a if x.startswith("open: ")), "")
        return kind, float(a[0]), sel
    return kind, float(a[1]), a[0]


# ---- the kernel ----


def limits():
    os.nice(19)
    resource.setrlimit(resource.RLIMIT_AS, (3 << 30, 3 << 30))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_CPU, (120, 121))


class Topo:
    """forge-proof's `topo` example, kept running: the part a program builds, and lineage selections."""

    def __init__(self, exe):
        self.exe, self.p = exe, None

    def ask(self, head, payload):
        if self.p is None:
            self.p = subprocess.Popen([self.exe], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, preexec_fn=limits)
        b = payload.encode()
        self.p.stdin.write(head % len(b) + b)
        self.p.stdin.flush()
        line = self.p.stdout.readline()
        if not line:
            raise Stop("kernel process died (crash, memory or CPU limit)")
        return json.loads(line)

    def state(self, src):
        """The part `src` builds: its edges and faces (forge-proof's topo)."""
        d = self.ask(b"%d\n", src)
        if "panic" in d:
            raise Stop("kernel panic")
        if d.get("err"):
            raise Stop(d["err"])
        return d

    def select(self, src, what, names):
        """A selection of exactly these edges or faces (by `n`, or a face's `name`) of the part `src` builds."""
        d = self.ask(b"sel %d\n", f"{what} " + " ".join(str(n) for n in sorted(names)) + "\n" + src)
        if "stop" in d:
            raise Stop(d["stop"])
        if "unresolved" in d:
            raise NoName(d["unresolved"])
        return d["sel"]

    def names(self, src, text):
        """The edges or faces (their `n`, or a face's `name`) a selection's text names on the part `src` builds."""
        d = self.ask(b"ask %d\n", text + "\n" + src)
        if "stop" in d:
            raise Stop(d["stop"])
        return d["names"]

    def close(self):
        if self.p is not None:
            self.p.kill()
            self.p.wait()
            self.p = None
