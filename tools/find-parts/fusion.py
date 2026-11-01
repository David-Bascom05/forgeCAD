#!/usr/bin/env python3
"""Fusion 360 Gallery / DeepCAD sketch+extrude timelines as .fc part files.

Both datasets share one JSON form: entities (Sketch with profiles in sketch coordinates, a
transform to world; ExtrudeFeature naming profiles, an operation, extents) and a sequence. The data
stays out of the repo (see research/find-parts/README.md for the licences); run with python3 -I.

  fusion.py FILE.json ... --out DIR [--mm SCALE]    write DIR/<stem>.fc, DIR/<stem>.json (truth)

Exact curves: lines, arcs, circles, ellipses and B-splines (their own data, marked curve).
"""
import json
import math
import os
import sys

MM = 10.0  # Fusion is in cm; DeepCAD, from Onshape, in metres (--mm 1000)
EPS = 1e-6  # endpoint match in mm


class Unsupported(Exception):
    pass


def vec(d):
    return (d["x"], d["y"], d.get("z", 0.0))


def add(a, b, s=1.0):
    return (a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s)


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def num(x):
    x = round(x, 9)
    if x == 0:
        x = 0.0
    s = f"{x:.9f}".rstrip("0").rstrip(".")
    return s if s not in ("", "-0") else "0"


def fine(x):
    return f"{x:.12f}".rstrip("0").rstrip(".") or "0"


def nurbs_points(c):
    k, deg, cp = c["knots"], c["degree"], [vec(p) for p in c["control_points"]]
    w = c.get("weights") or [1.0] * len(cp)
    def at(u):
        s = deg
        while s < len(cp) - 1 and k[s + 1] <= u:
            s += 1
        d = [tuple(x * w[s - deg + j] for x in cp[s - deg + j]) + (w[s - deg + j],) for j in range(deg + 1)]
        for r in range(1, deg + 1):
            for j in range(deg, r - 1, -1):
                i = s - deg + j
                den = k[i + deg - r + 1] - k[i]
                a = (u - k[i]) / den if den else 0.0
                d[j] = tuple(p * (1 - a) + q * a for p, q in zip(d[j - 1], d[j]))
        return (d[deg][0] / d[deg][3], d[deg][1] / d[deg][3], d[deg][2] / d[deg][3])
    lo, hi = k[deg], k[len(cp)]
    spans = sorted({x for x in k if lo <= x <= hi})
    us = []
    for a, b in zip(spans, spans[1:]):
        us += [a + (b - a) * i / 8 for i in range(8)]
    us.append(hi)
    return [at(u) for u in us]


class Snap:
    """Points within 1e-3 mm are one point. (Rounding alone splits a pair across a rounding boundary.)"""

    def __init__(self):
        self.cells = {}

    def __call__(self, p):
        cx, cy = math.floor(p[0] / 2e-3), math.floor(p[1] / 2e-3)
        for i in (-1, 0, 1):
            for j in (-1, 0, 1):
                for q in self.cells.get((cx + i, cy + j), ()):
                    if math.dist(q, p[:2]) < 1e-3:
                        return q
        q = (p[0], p[1])
        self.cells.setdefault((cx, cy), []).append(q)
        return q


class Edge:
    """A profile curve in sketch coordinates (mm): kind line, arc, circle, or poly (approximated)."""

    def __init__(self, c):
        t = c["type"]
        self.approx = False
        self.curved = False
        if t == "Line3D":
            self.kind = "line"
            self.a, self.b = self.mm(c["start_point"]), self.mm(c["end_point"])
        elif t == "Circle3D":
            self.kind = "circle"
            self.c, self.r = self.mm(c["center_point"]), c["radius"] * MM
            self.a = self.b = None
        elif t == "Arc3D":
            self.kind = "arc"
            self.c, self.r = self.mm(c["center_point"]), c["radius"] * MM
            self.a, self.b = self.mm(c["start_point"]), self.mm(c["end_point"])
            # which way from a to b: the side the data's own middle lies on (its angles run about
            # its normal from its reference, and DeepCAD's ends need not follow them)
            full = c["end_angle"] - c["start_angle"]
            ref, nrm = vec(c["reference_vector"]), vec(c["normal"])
            yr, mid = cross(nrm, ref), (c["start_angle"] + c["end_angle"]) / 2
            mx = ref[0] * math.cos(mid) + yr[0] * math.sin(mid)
            my = ref[1] * math.cos(mid) + yr[1] * math.sin(mid)
            a0 = math.atan2(self.a[1] - self.c[1], self.a[0] - self.c[0])
            b0 = math.atan2(self.b[1] - self.c[1], self.b[0] - self.c[0])
            ccw, tom = (b0 - a0) % (2 * math.pi), (math.atan2(my, mx) - a0) % (2 * math.pi)
            cw = 2 * math.pi - ccw
            if math.dist(self.a[:2], self.b[:2]) < EPS:
                self.sweep = full if nrm[2] > 0 else -full
            elif abs(ccw - cw) > 1e-3:
                # the size of the sweep settles it (half of DeepCAD's arcs turn against their normal)
                self.sweep = ccw if abs(ccw - abs(full)) < abs(cw - abs(full)) else -cw
            else:
                self.sweep = ccw if tom < ccw else -cw
        elif t in ("Ellipse3D", "EllipticalArc3D"):
            self.kind = "ellipse"
            self.curved = True
            ce, mx = self.mm(c["center_point"]), vec(c["major_axis"])
            A, B = c["major_axis_radius"] * MM, c["minor_axis_radius"] * MM
            n = vec(c["normal"])
            my = cross(n, mx)
            a0, a1 = c.get("start_angle", 0.0), c.get("end_angle", 2 * math.pi)
            if t == "EllipticalArc3D" and "start_angle" not in c:
                raise Unsupported("elliptical arc without angles")
            self.ell = (ce, mx, my, A, B, a0, a1)
            self.pts = [
                (ce[0] + A * math.cos(u) * mx[0] + B * math.sin(u) * my[0], ce[1] + A * math.cos(u) * mx[1] + B * math.sin(u) * my[1], ce[2])
                for u in [a0 + (a1 - a0) * i / 48 for i in range(49)]
            ]
            self.a, self.b = self.pts[0], self.pts[-1]
        elif t == "NurbsCurve3D":
            self.kind = "spline"
            self.curved = True
            self.nurbs = (c["degree"], c["knots"], [tuple(x * MM for x in vec(p)) for p in c["control_points"]], c.get("weights"))
            self.pts = [tuple(x * MM for x in p) for p in nurbs_points(c)]
            if c.get("periodic"):
                self.pts[-1] = self.pts[0]
            self.a, self.b = self.pts[0], self.pts[-1]
        else:
            raise Unsupported("curve " + t)

    @staticmethod
    def mm(p):
        return tuple(x * MM for x in vec(p))

    def key(self, snap):
        """Same key for the same curve, whichever way it runs."""
        if self.kind == "circle":
            return ("c", snap(self.c), snap((self.r, 0.0)))
        ends = tuple(sorted((snap(self.a), snap(self.b))))
        if self.kind == "line":
            return ("l",) + ends
        if self.kind == "arc":
            # the two halves of a circle share both ends: tell them by the middle
            a0 = math.atan2(self.a[1] - self.c[1], self.a[0] - self.c[0]) + self.sweep / 2
            m = (self.c[0] + self.r * math.cos(a0), self.c[1] + self.r * math.sin(a0))
            return ("a", snap(m)) + ends
        return ("p", snap(self.pts[len(self.pts) // 2])) + ends

    def sample(self, rev=False):
        """Points along the edge, for area and containment."""
        if self.kind == "line":
            p = [self.a, self.b]
        elif self.kind == "circle":
            p = [(self.c[0] + self.r * math.cos(i * math.pi / 16), self.c[1] + self.r * math.sin(i * math.pi / 16), 0) for i in range(33)]
        elif self.kind == "arc":
            a0 = math.atan2(self.a[1] - self.c[1], self.a[0] - self.c[0])
            n = max(4, int(abs(self.sweep) / (math.pi / 16)))
            p = [(self.c[0] + self.r * math.cos(a0 + self.sweep * i / n), self.c[1] + self.r * math.sin(a0 + self.sweep * i / n), 0) for i in range(n + 1)]
        else:
            p = self.pts
        return p[::-1] if rev else p


def chain(edges):
    """Closed loops, each [(edge, reversed)], from edges meeting end to end."""
    closed = lambda e: e.kind == "circle" or (e.kind != "line" and math.dist(e.a[:2], e.b[:2]) < EPS)
    loops = [[(e, False)] for e in edges if closed(e)]
    rest = [e for e in edges if not closed(e)]
    while rest:
        first = rest.pop()
        loop = [(first, False)]
        start, here = first.a, first.b
        while math.dist(here[:2], start[:2]) > 1e-3:
            best = None
            for i, e in enumerate(rest):
                for rev, p, q in ((False, e.a, e.b), (True, e.b, e.a)):
                    d = math.dist(p[:2], here[:2])
                    if d < 1e-3 and (best is None or d < best[0]):
                        best = (d, i, rev, q)
            if best is None:
                raise Unsupported("profile curves do not close")
            _, i, rev, q = best
            loop.append((rest.pop(i), rev))
            here = q
        loops += split_touching(loop)
    return loops


def split_touching(loop):
    """A chain through one point twice is two loops touching there, each closed on its own."""
    out, stack, at = [], [], {}
    pt = Snap()
    start = loop[0][0].b if loop[0][1] else loop[0][0].a
    at[pt(start)] = 0
    for e, rev in loop:
        stack.append((e, rev))
        k = pt(e.a if rev else e.b)
        if k in at:
            i = at[k]
            out.append(stack[i:])
            del stack[i:]
            at = {q: n for q, n in at.items() if n <= i}
        else:
            at[k] = len(stack)
    return [l for l in out if l]


def polygon(loop):
    pts = []
    for e, rev in loop:
        s = e.sample(rev)
        pts += s[:-1] if e.kind != "circle" else s
    return pts


def area(pts):
    return 0.5 * sum(pts[i][0] * pts[(i + 1) % len(pts)][1] - pts[(i + 1) % len(pts)][0] * pts[i][1] for i in range(len(pts)))


def inside(p, pts):
    x, y, c = p[0], p[1], False
    for i in range(len(pts)):
        a, b = pts[i], pts[(i + 1) % len(pts)]
        if (a[1] > y) != (b[1] > y) and x < a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
            c = not c
    return c


def prune(edges):
    """Spurs a region has inside it (a sketch line ending in nothing) bound no area: drop them."""
    pt = Snap()
    while True:
        deg = {}
        for e in edges:
            if e.a is not None and pt(e.a) != pt(e.b):
                for p in (e.a, e.b):
                    deg[pt(p)] = deg.get(pt(p), 0) + 1
        keep = [e for e in edges if e.a is None or pt(e.a) == pt(e.b) or (deg[pt(e.a)] > 1 and deg[pt(e.b)] > 1)]
        if len(keep) == len(edges):
            return edges
        edges = keep


def ellipse_param(c, p):
    """A sketch point's angle on an elliptical arc's ellipse, and how far off it (mm), mm."""
    ce, mx = Edge.mm(c["center_point"]), vec(c["major_axis"])
    k = math.sqrt(dot(mx, mx))
    mx = tuple(x / k for x in mx)
    my = cross(vec(c["normal"]), mx)
    A, B = c["major_axis_radius"] * MM, c["minor_axis_radius"] * MM
    d = add(p, ce, -1.0)
    u = math.atan2(dot(d, my) / B, dot(d, mx) / A)
    q = add(add(ce, mx, A * math.cos(u)), my, B * math.sin(u))
    return u, math.dist(q[:2], p[:2])


def same_ellipse(a, b):
    return all(abs(a[k] - b[k]) < 1e-7 for k in ("major_axis_radius", "minor_axis_radius")) and math.dist(vec(a["center_point"]), vec(b["center_point"])) < 1e-7


def trimmed_ellipses(loop, wall, taken):
    """Fusion's elliptical arcs often come without their ends. A loop of nothing else is the whole
    ellipse; between other curves an arc runs from one curve's free end to the next's, the way
    round that holds a point of the feature's wall (`wall`, sketch points). An arc next to another
    with no ends has none to find."""
    curves = loop["profile_curves"]
    bare = [c for c in curves if c["type"] == "EllipticalArc3D" and "start_angle" not in c]
    if not bare:
        return loop
    if len(bare) == len(curves) and all(same_ellipse(bare[0], c) for c in bare):
        return dict(loop, profile_curves=[dict(bare[0], type="Ellipse3D")])
    if len(bare) > 1:
        raise Unsupported("elliptical arc without angles")
    c = bare[0]
    ends = {}
    for o in curves:
        if o is c:
            continue
        e = Edge(o)
        if e.kind == "circle":
            raise Unsupported("elliptical arc without angles")
        for p in (e.a, e.b):
            key = (round(p[0], 3), round(p[1], 3))
            ends[key] = (ends.get(key, (0, p))[0] + 1, p)
    free = [p for n, p in ends.values() if n % 2]
    if len(free) != 2:
        raise Unsupported("elliptical arc without angles")
    (u0, d0), (u1, d1) = ellipse_param(c, free[0]), ellipse_param(c, free[1])
    if max(d0, d1) > 1e-2:
        raise Unsupported("elliptical arc without angles")
    if u1 < u0:
        u0, u1 = u1, u0
    # u0..u1 or u1..u0+2pi: the one the wall is on; with no wall left, the shorter
    on = [ellipse_param(c, q) for q in wall]
    on = [u for u, d in on if d < 1e-2]
    first = sum(u0 < u < u1 for u in on)
    second = len(on) - first
    short = first > second or (first == second and u1 - u0 <= math.pi)
    # the two profiles either side of a chord share its ends: one arc each
    key = (round(u0, 4), round(u1, 4), c["major_axis_radius"], c["minor_axis_radius"])
    if key in taken:
        short = not taken[key]
    taken[key] = short
    a0, a1 = (u0, u1) if short else (u1, u0 + 2 * math.pi)
    out = dict(c, start_angle=a0, end_angle=a1)
    return dict(loop, profile_curves=[out if o is c else o for o in curves])


def bridges(edges):
    """A curve joining an outline to its hole counts once in a loop, as Fusion records a keyhole: the
    odd vertices pair up along such curves, which bound no area. Drop the shortest path between them."""
    pt = Snap()
    while True:
        at = {}
        for i, e in enumerate(edges):
            if e.a is not None and pt(e.a) != pt(e.b):
                at.setdefault(pt(e.a), []).append((i, pt(e.b)))
                at.setdefault(pt(e.b), []).append((i, pt(e.a)))
        odd = [v for v in at if len(at[v]) % 2]
        if not odd:
            return edges
        seen, todo, path = {odd[0]: None}, [odd[0]], None
        for v in todo:
            if v != odd[0] and len(at[v]) % 2:
                path = v
                break
            for i, w in at[v]:
                if w not in seen:
                    seen[w] = (v, i)
                    todo.append(w)
        if path is None:
            return edges
        drop = set()
        while seen[path]:
            path, i = seen[path]
            drop.add(i)
        edges = prune([e for i, e in enumerate(edges) if i not in drop])


def pinched(edges):
    """Whether more than two curves meet anywhere: which loops they make is then ambiguous, so the
    profiles stay apart."""
    pt, deg = Snap(), {}
    for e in edges:
        if e.a is not None and pt(e.a) != pt(e.b):
            for p in (e.a, e.b):
                deg[pt(p)] = deg.get(pt(p), 0) + 1
    return any(v > 2 for v in deg.values())


def regions(profiles):
    """The union of a feature's profiles as regions [outer, holes..]: curves two profiles share
    (one region's hole filled by the next, or a common edge) cancel, as the CAD system merges them."""
    seen, snap = {}, Snap()
    for pr in profiles:
        for l in pr["loops"]:
            for c in l["profile_curves"]:
                e = Edge(c)
                k = e.key(snap)
                if k in seen:
                    seen[k] = None
                else:
                    seen[k] = e
    edges = [e for e in seen.values() if e is not None]
    if not edges:
        raise Unsupported("profiles cancel to nothing")
    edges = bridges(prune(edges))
    if len(profiles) > 1 and pinched(edges):
        raise Unsupported("profiles meet at a point")
    loops = chain(edges)
    # a loop that bounds nothing (a line out and back) goes
    loops = [l for l in loops if len(polygon(l)) > 2 and abs(area(polygon(l))) > 1e-9]
    if not loops:
        raise Unsupported("profiles bound nothing")
    polys = [polygon(l) for l in loops]
    order = sorted(range(len(loops)), key=lambda i: -abs(area(polys[i])))
    out = []
    for i in order:
        # a point of the loop, nudged off shared vertices by taking an edge midpoint
        p = polys[i]
        probe = ((p[0][0] + p[1][0]) / 2, (p[0][1] + p[1][1]) / 2)
        depth = [j for j in order if j != i and abs(area(polys[j])) > abs(area(polys[i])) and inside(probe, polys[j])]
        if len(depth) % 2 == 0:
            out.append([loops[i]])
        else:
            parent = min(depth, key=lambda j: abs(area(polys[j])))
            for r in out:
                if r[0] is loops[parent]:
                    r.append(loops[i])
                    break
            else:
                raise Unsupported("a hole with no outer loop")
    return out


def emit_ellipse(e, rev, f, whole):
    """An ellipse or arc of one in frame f: its axes mapped, the angles running as they then do."""
    ce, mx, my, A, B, a0, a1 = e.ell
    o, x, y = f(ce), f(add(ce, mx, 1.0)), f(add(ce, my, 1.0))
    xv, yv = (x[0] - o[0], x[1] - o[1]), (y[0] - o[0], y[1] - o[1])
    rot = math.degrees(math.atan2(xv[1], xv[0]))
    sign = 1.0 if xv[0] * yv[1] - xv[1] * yv[0] > 0 else -1.0
    if whole:
        return f"ellipse {num(o[0])} {num(o[1])} {fine(A)} {fine(B)} {fine(rot)}"
    st, sw = math.degrees(a0) * sign, math.degrees(a1 - a0) * sign
    if rev:
        st, sw = st + sw, -sw
    return f"ell {num(o[0])} {num(o[1])} {fine(A)} {fine(B)} {fine(rot)} {fine(st)} {fine(sw)}"


def emit_spline(e, rev, f, p, q):
    """B-spline data in frame f, its ends on the joints p and q."""
    d, k, cp, w = e.nurbs
    pts = [f(c) for c in cp]
    if rev:
        pts, k = pts[::-1], [k[0] + k[-1] - x for x in reversed(k)]
        w = w[::-1] if w else w
    pts[0], pts[-1] = p, q
    t = f"spline {d} " + " ".join(f"{fine(a)} {fine(b)}" for a, b in pts[:1] + pts[1:-1] + pts[-1:])
    t += " knots " + " ".join(f"{x:.15f}" for x in k)
    if w and any(abs(x - 1.0) > 1e-12 for x in w):
        t += " weights " + " ".join(f"{x:.15f}" for x in w)
    return t


def emit_loop(loop, f):
    """A loop's text in frame f (maps sketch points to the frame's x, y)."""
    if len(loop) == 1 and loop[0][0].kind == "circle":
        e = loop[0][0]
        c = f(e.c)
        return f"circle {num(c[0])} {num(c[1])} {num(e.r)}"
    if len(loop) == 1 and loop[0][0].kind == "ellipse" and math.dist(loop[0][0].a[:2], loop[0][0].b[:2]) < EPS:
        return emit_ellipse(loop[0][0], False, f, True)
    # the data's curve ends can be 1e-5 mm apart (Fusion's tolerance; DeepCAD keeps 8 decimals): as an
    # importer heals such gaps, curves meeting share one point and each arc is refit through its two
    ns = [(f(e.b if rev else e.a), f(e.a if rev else e.b)) for e, rev in loop]
    n = len(loop)
    joint = [tuple((ns[i - 1][1][k] + ns[i][0][k]) / 2 for k in (0, 1)) for i in range(n)]
    parts = []
    for i, (e, rev) in enumerate(loop):
        p, q = joint[i], joint[(i + 1) % n]
        if e.kind == "arc":
            sw = (-e.sweep if rev else e.sweep) * (-1 if f.flipped else 1)
            d = math.dist(p, q)
            c = f(e.c)
            # the joint on the data's circle if it is there: refitting moves a centre further the less
            # an arc turns, and coaxial walls would come out a hair apart
            on = max(abs(math.dist(p, c) - e.r), abs(math.dist(q, c) - e.r)) < 4e-6
            if not on and d > 1e-9 and abs(abs(sw) - 2 * math.pi) > 1e-6:
                h2 = e.r * e.r - d * d / 4
                h = math.sqrt(max(h2, 0.0))
                m = ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2)
                nx, ny = (q[1] - p[1]) / d, (p[0] - q[0]) / d
                c1, c2 = (m[0] + h * nx, m[1] + h * ny), (m[0] - h * nx, m[1] - h * ny)
                c = c1 if math.dist(c1, c) <= math.dist(c2, c) else c2
            a0 = math.degrees(math.atan2(p[1] - c[1], p[0] - c[0]))
            r = e.r if on else math.dist(p, c)
            if math.dist(q, c) > 0:
                a1 = math.degrees(math.atan2(q[1] - c[1], q[0] - c[0]))
                dg = a1 - a0
                while sw > 0 and dg <= 0:
                    dg += 360
                while sw < 0 and dg >= 0:
                    dg -= 360
                if abs(abs(sw) - 2 * math.pi) > 1e-6:
                    sw = math.radians(dg)
            parts.append(f"arc {num(c[0])} {num(c[1])} {fine(r)} {fine(a0)} {fine(math.degrees(sw))}")
        elif e.kind == "spline":
            parts.append(emit_spline(e, rev, f, p, q))
        elif e.kind == "ellipse":
            parts.append(emit_ellipse(e, rev, f, False))
        else:
            ps = list(map(f, e.sample(rev)))
            ps[0], ps[-1] = p, q
            for a, b in zip(ps, ps[1:]):
                parts.append(f"line {num(a[0])} {num(a[1])} {num(b[0])} {num(b[1])}")
    if all(e.kind == "line" for e, _ in loop):
        pts = [x.split()[1:3] for x in parts]
        return "poly " + " ".join(f"{x} {y}" for x, y in pts)
    return "loop " + " ".join(parts)


class Frame:
    """Sketch coordinates to the .fc frame's: the frame's x is the sketch's, its y may be flipped."""

    def __init__(self, X, Y, y):
        self.X, self.Y, self.y = X, Y, y
        self.flipped = dot(y, Y) < 0

    def __call__(self, p):
        world = add(add((0, 0, 0), self.X, p[0]), self.Y, p[1])
        return (dot(world, self.X), dot(world, self.y))


def extent(e):
    """(up, down, taper up, taper down): lengths along the normal and against it, radians."""
    t = e["extent_type"]
    one = e["extent_one"]
    a1 = one["taper_angle"]["value"]
    d1 = one["distance"]["value"] * MM
    two = e.get("extent_two")
    if t == "OneSideFeatureExtentType":
        if d1 == 0 and two:
            return 0.0, two["distance"]["value"] * MM, 0.0, two["taper_angle"]["value"]
        return d1, 0.0, a1, 0.0
    if t == "SymmetricFeatureExtentType":
        if one.get("is_full_length", False):
            return d1 / 2, d1 / 2, a1, a1
        return d1, d1, a1, a1
    return d1, (two["distance"]["value"] * MM if two else 0.0), a1, (two["taper_angle"]["value"] if two else 0.0)


def touching(regs):
    """Whether a region's hole touches its outline or another hole. Merged, such profiles are a
    pinched loop the part format refuses; apart they are plain regions."""
    for r in regs:
        polys = [polygon(lp) for lp in r]
        size = max(math.dist(p, polys[0][0]) for p in polys[0])
        near = max(1e-3, 0.005 * size)
        for i, h in enumerate(polys[1:], 1):
            for j, o in enumerate(polys):
                if j != i and any(seg_dist(q, o) < near for q in h):
                    return True
    return False


def seg_dist(q, pts):
    """A point's distance from a closed polygon."""
    best = float("inf")
    for i in range(len(pts)):
        a, b = pts[i], pts[(i + 1) % len(pts)]
        ab = (b[0] - a[0], b[1] - a[1])
        den = ab[0] ** 2 + ab[1] ** 2
        t = ((q[0] - a[0]) * ab[0] + (q[1] - a[1]) * ab[1]) / den if den else 0.0
        t = min(1.0, max(0.0, t))
        best = min(best, math.dist(q[:2], (a[0] + t * ab[0], a[1] + t * ab[1])))
    return best


def joined(r, s):
    """Whether two regions share more than points: a stretch of edge, or area."""
    a, b = polygon(r[0]), polygon(s[0])
    for x, y in ((a, b), (b, a)):
        for i in range(len(x)):
            p, q = x[i], x[(i + 1) % len(x)]
            m = ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2)
            if seg_dist(m, y) < 1e-3 or inside(m, y):
                return True
    return False


def on_or_inside(q, r):
    """A sketch point lies in region r = [outer, holes..], or within a hair of its boundary."""
    polys = [polygon(lp) for lp in r]
    if any(seg_dist(q, pts) < 1e-3 for pts in polys):
        return True
    return inside(q, polys[0]) and not any(inside(q, h) for h in polys[1:])


def body_points(e):
    """Each body's points on faces (mm) after a feature."""
    return {bid: [tuple(x * MM for x in vec(e["faces"][f]["point_on_face"])) for f in b["faces"]] for bid, b in e["bodies"].items()}


def moved(c, f):
    """Curve data with its points and vectors put through (point map, vector map)."""
    if isinstance(c, list):
        return [moved(x, f) for x in c]
    if not isinstance(c, dict):
        return c
    if c.get("type") == "Point3D":
        return dict(c, **dict(zip("xyz", f[0](vec(c)))))
    if c.get("type") == "Vector3D":
        return dict(c, **dict(zip("xyz", f[1](vec(c)))))
    return {k: moved(v, f) for k, v in c.items()}


def replaned(T, profs, e, up, down):
    """Profiles Fusion stored off their sketch's plane (a sketch moved after they were made; their
    sketch z varies) lie in a plane of their own, and extrude square to it. The transform and the
    profiles in that plane, or None if they lie in the sketch's own. Fusion's side faces of the
    feature say which way it went."""
    O, X, Y, Z = (vec(T["origin"]), vec(T["x_axis"]), vec(T["y_axis"]), vec(T["z_axis"]))
    pts = []
    for pr in profs:
        for l in pr["loops"]:
            for c in l["profile_curves"]:
                for k in ("start_point", "end_point", "center_point"):
                    if k in c:
                        pts.append(vec(c[k]))
    if not pts or max(p[2] for p in pts) - min(p[2] for p in pts) < 1e-6:
        return None
    world = lambda p: add(add(add(O, X, p[0]), Y, p[1]), Z, p[2])
    w = [world(p) for p in pts]
    best, n = 0.0, None
    a = w[0]
    b = max(w, key=lambda q: math.dist(a, q))
    for q in w:
        m = cross(add(b, a, -1.0), add(q, a, -1.0))
        if dot(m, m) > best:
            best, n = dot(m, m), m
    if n is None or best < 1e-18:
        return None
    n = tuple(x / math.sqrt(best) for x in n)
    d = add(b, a, -1.0)
    x = tuple(v / math.sqrt(dot(d, d)) for v in d)
    side = [tuple(u * MM for u in vec(e["faces"][f]["point_on_face"])) for f in e.get("extrude_side_faces", []) if f in e["faces"]]
    reach = [dot(add(q, tuple(u * MM for u in a), -1.0), n) for q in side]
    # the direction whose slab holds Fusion's side faces
    lo, hi = -down, up
    if lo > hi:
        lo, hi = hi, lo
    if reach and sum(lo - 1e-3 <= t <= hi + 1e-3 for t in reach) < sum(lo - 1e-3 <= -t <= hi + 1e-3 for t in reach):
        n = tuple(-v for v in n)
    y = cross(n, x)
    pm = lambda p: (lambda q: (dot(add(q, a, -1.0), x), dot(add(q, a, -1.0), y), dot(add(q, a, -1.0), n)))(world(p))
    vm = lambda v: (lambda q: (dot(q, x), dot(q, y), dot(q, n)))(add(add(add((0, 0, 0), X, v[0]), Y, v[1]), Z, v[2]))
    pv = lambda t, v: dict(zip("xyz", v), type=t)
    return {"origin": pv("Point3D", a), "x_axis": pv("Vector3D", x), "y_axis": pv("Vector3D", y), "z_axis": pv("Vector3D", n)}, moved(profs, (pm, vm))


class Slab:
    """A region extruded: how many of a body's face points lie in it says whose it is."""

    def __init__(self, r, O, X, Y, Z, lo, hi):
        self.r, self.O, self.X, self.Y, self.Z, self.lo, self.hi = r, O, X, Y, Z, lo, hi

    def score(self, pts):
        n = 0
        for p in pts:
            d = add(p, self.O, -1.0)
            t = dot(d, self.Z)
            if self.lo - 1e-3 <= t <= self.hi + 1e-3 and on_or_inside((dot(d, self.X), dot(d, self.Y)), self.r):
                n += 1
        return n


def translate(d):
    """(lines, notes) for a design; raises Unsupported."""
    ents = d["entities"]
    seq = []
    for s in d["sequence"]:
        if s["type"] == "ExtrudeFeature" and (not seq or seq[-1] != s["entity"]):
            seq.append(s["entity"])
    lines, notes = [], set()
    # Fusion's body id -> the label of the line that made our body; several ids may share one (a cut
    # split it: one body of two lumps here)
    label = {}
    made = 0
    after = []
    for k, eid in enumerate(seq):
        e = ents[eid]
        if e["type"] != "ExtrudeFeature":
            raise Unsupported(e["type"])
        if not e["profiles"]:
            raise Unsupported("extrude from a face")
        by = {}
        for p in e["profiles"]:
            by.setdefault(p["sketch"], []).append(ents[p["sketch"]]["profiles"][p["profile"]])
        up, down, t_up, t_down = extent(e)
        off = 0.0
        st = e["start_extent"]
        if st["type"] == "OffsetStartDefinition":
            off = st.get("offset", {}).get("value", 0.0) * MM
        op = {"NewBodyFeatureOperation": "new", "JoinFeatureOperation": "add", "CutFeatureOperation": "cut", "IntersectFeatureOperation": "keep"}[e["operation"]]
        now = list(e["bodies"])
        pts = body_points(e)
        fresh = [b for b in now if b not in label]
        ext = e.get("extrude_bodies") or []
        slabs, used = [], []
        for sk, profs in by.items():
            T = ents[sk]["transform"]
            again = replaned(T, profs, e, up, down)
            if again:
                notes.add("replaned")
                T, profs = again
            O, X, Y, Z = (vec(T["origin"]), vec(T["x_axis"]), vec(T["y_axis"]), vec(T["z_axis"]))
            O = tuple(x * MM for x in O)
            lift = 0.0
            wall = [tuple(u * MM for u in vec(e["faces"][f]["point_on_face"])) for f in e.get("extrude_side_faces", []) if f in e["faces"]]
            wall = [(lambda d: (dot(d, X), dot(d, Y), 0.0))(add(q, O, -1.0)) for q in wall]
            taken = {}
            profs = [dict(pr, loops=[trimmed_ellipses(l, wall, taken) for l in pr["loops"]]) for pr in profs]
            try:
                regs = regions(profs)
                if len(profs) > 1 and touching(regs):
                    raise Unsupported("a hole touches")
            except (Unsupported, IndexError):
                # one profile at a time, the way it was drawn
                notes.add("unmerged")
                regs = [r for p in profs for r in regions([p])]
            for r in regs:
                for lp in r:
                    for ed, _ in lp:
                        if ed.approx:
                            notes.add("approx")
                        if ed.curved:
                            notes.add("curve")
                        lift = ed.c[2] if ed.kind in ("circle", "arc") else ed.a[2]
                # the slab from -down to up along the normal: from the frame turned over when it
                # all lies below, from a shifted origin when it does not reach the plane
                if not up and not down:
                    notes.add("zero-length")
                    continue
                lo, hi = (-down, up) if down else ((0.0, up) if up > 0 else (up, 0.0))
                if lo > hi:
                    lo, hi = hi, lo
                # a taper leans away from the sketch plane: each side of it is its own extrude
                tapered = abs(t_up) > 1e-9 or abs(t_down) > 1e-9
                pieces = [(0.0, hi, t_up), (lo, 0.0, t_down)] if tapered and lo < 0 < hi else [(lo, hi, t_up or t_down)]
                texts = []
                for plo, phi, ang in pieces:
                    if phi <= 0:
                        z, a, b, shift = tuple(-x for x in Z), phi - plo, 0.0, phi
                    elif plo >= 0:
                        z, a, b, shift = Z, phi - plo, 0.0, plo
                    else:
                        z, a, b, shift = Z, phi, -plo, 0.0
                    y = cross(z, X)
                    org = add(O, Z, lift + off + shift)
                    f2 = Frame(X, Y, y)
                    frame = f"z {num(z[0])} {num(z[1])} {num(z[2])} x {num(X[0])} {num(X[1])} {num(X[2])} at {num(org[0])} {num(org[1])} {num(org[2])}"
                    loops = " ".join(emit_loop(lp, f2) for lp in r)
                    ext_txt = num(a) if b == 0 else f"two {num(a)} {num(b)}"
                    if abs(ang) > 1e-9:
                        # Fusion's negative taper leans in
                        ext_txt += f" taper {fine(-math.degrees(ang))}"
                    texts.append(f"extrude {{}}{ext_txt} {frame} {loops}")
                slab = Slab(r, O, X, Y, Z, lift + off + lo, lift + off + hi)
                alive = set(label.values())
                body = texts[0]
                here = None
                if not alive and not made and op in ("cut", "keep"):
                    raise Unsupported("first feature cuts")
                owner = None
                if op == "new" or not made or op == "add":
                    # the region goes to the body its faces are in: what Fusion made of this feature, or
                    # the bodies a join names (profiles abutting along part of an edge make one body)
                    cands = fresh if op == "new" or not made else [x for x in ext if x in now] or now
                    owner = max(cands, key=lambda x: slab.score(pts.get(x, []))) if cands else None
                # regions of one body that only touch at points are solids of their own: one would pinch
                apart = op == "new" and owner in label and not any(joined(r, o.r) for o, n in slabs if n == label[owner])
                if apart or ((op == "new" or not made or op == "add") and owner not in label):
                    name = here = f"b{made}"
                    lines.append(f"{name}: " + body.format("new " if made else ""))
                    made += 1
                    slabs.append((slab, name))
                    if owner is not None and not apart:
                        label[owner] = name
                elif op == "new" or op == "add":
                    if op == "add":
                        used.append(owner)
                    here = label[owner]
                    slabs.append((slab, here))
                    lines.append(body.format("add ") + (f" in {label[owner]}" if len(alive) > 1 else ""))
                else:
                    lines.append(body.format(op + " ") + (" in all" if len(alive) > 1 else ""))
                for t in texts[1:]:
                    if here:
                        lines.append(t.format("add ") + (f" in {here}" if len(set(label.values()) | {here}) > 1 else ""))
                    else:
                        lines.append(t.format(op + " ") + (" in all" if len(alive) > 1 else ""))
        for x in fresh:
            if x in label:
                continue
            if slabs:
                label[x] = max(slabs, key=lambda s: s[0].score(pts.get(x, [])))[1]
            else:
                # a cut split a body: its lumps stay one body here
                old = [label[b] for b in ext if b in label] or list(label.values())
                if old:
                    label[x] = old[0]
        if op == "add" and used:
            # bodies the join took in end with it
            into = label[used[0]]
            gone = sorted({label[b] for b in label if b not in now} - {label[b] for b in now if b in label})
            if gone:
                lines.append(f"combine add {into} " + " ".join(gone))
                for b, n in label.items():
                    if n in gone:
                        label[b] = into
        for b in [b for b in label if b not in now]:
            del label[b]
        after.append(len(lines))
    notes = sorted(notes)
    return lines, notes + ["after:" + ",".join(map(str, after))]


def main():
    global MM
    a = sys.argv[1:]
    out = a[a.index("--out") + 1]
    if "--mm" in a:
        MM = float(a[a.index("--mm") + 1])
    os.makedirs(out, exist_ok=True)
    tally = {}
    for fn in a:
        if not fn.endswith(".json"):
            continue
        stem = os.path.splitext(os.path.basename(fn))[0]
        d = json.load(open(fn))
        truth = d.get("properties", {})
        try:
            lines, notes = translate(d)
            with open(os.path.join(out, stem + ".fc"), "w") as f:
                f.write("\n".join(lines) + "\n")
            with open(os.path.join(out, stem + ".json"), "w") as f:
                json.dump({"notes": notes, "volume": (truth.get("volume") or 0) * MM**3, "area": (truth.get("area") or 0) * MM**2, "bodies": truth.get("body_count"), "bbox": truth.get("bounding_box")}, f)
            tally["ok"] = tally.get("ok", 0) + 1
        except (Unsupported, KeyError, IndexError, ZeroDivisionError, ValueError) as x:
            why = repr(x)[:60]
            tally[why] = tally.get(why, 0) + 1
    with open(os.path.join(out, "_tally.txt"), "w") as f:
        json.dump(tally, f)
    print(tally)


if __name__ == "__main__":
    main()
