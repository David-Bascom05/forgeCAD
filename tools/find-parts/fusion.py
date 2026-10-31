#!/usr/bin/env python3
"""Fusion 360 Gallery / DeepCAD sketch+extrude timelines as .fc part files.

Both datasets share one JSON form: entities (Sketch with profiles in sketch coordinates, a
transform to world; ExtrudeFeature naming profiles, an operation, extents) and a sequence. The data
stays out of the repo (see research/find-parts/README.md for the licences); run with python3 -I.

  fusion.py FILE.json ... --out DIR [--mm SCALE]    write DIR/<stem>.fc, DIR/<stem>.json (truth)

Exact curves: lines, arcs, circles. Ellipses and splines have no row in a part file: they become
fine polylines and the part is marked approx. A tapered extrude has no form yet: untranslatable.
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
            self.kind = "poly"
            self.approx = True
            ce, mx = self.mm(c["center_point"]), vec(c["major_axis"])
            A, B = c["major_axis_radius"] * MM, c["minor_axis_radius"] * MM
            n = vec(c["normal"])
            my = cross(n, mx)
            a0, a1 = c.get("start_angle", 0.0), c.get("end_angle", 2 * math.pi)
            if t == "EllipticalArc3D" and "start_angle" not in c:
                raise Unsupported("elliptical arc without angles")
            self.pts = [
                (ce[0] + A * math.cos(u) * mx[0] + B * math.sin(u) * my[0], ce[1] + A * math.cos(u) * mx[1] + B * math.sin(u) * my[1], ce[2])
                for u in [a0 + (a1 - a0) * i / 48 for i in range(49)]
            ]
            self.a, self.b = self.pts[0], self.pts[-1]
        elif t == "NurbsCurve3D":
            self.kind = "poly"
            self.approx = True
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
    loops = chain(prune(edges))
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


def emit_loop(loop, f):
    """A loop's text in frame f (maps sketch points to the frame's x, y)."""
    if len(loop) == 1 and loop[0][0].kind == "circle":
        e = loop[0][0]
        c = f(e.c)
        return f"circle {num(c[0])} {num(c[1])} {num(e.r)}"
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
            if d > 1e-9 and abs(abs(sw) - 2 * math.pi) > 1e-6:
                h2 = e.r * e.r - d * d / 4
                h = math.sqrt(max(h2, 0.0))
                m = ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2)
                nx, ny = (q[1] - p[1]) / d, (p[0] - q[0]) / d
                c1, c2 = (m[0] + h * nx, m[1] + h * ny), (m[0] - h * nx, m[1] - h * ny)
                c = c1 if math.dist(c1, c) <= math.dist(c2, c) else c2
            a0 = math.degrees(math.atan2(p[1] - c[1], p[0] - c[0]))
            r = math.dist(p, c)
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
        else:
            ps = list(map(f, e.sample(rev)))
            ps[0], ps[-1] = p, q
            for a, b in zip(ps, ps[1:]):
                parts.append(f"line {num(a[0])} {num(a[1])} {num(b[0])} {num(b[1])}")
    if all(e.kind != "arc" for e, _ in loop):
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
    t = e["extent_type"]
    one = e["extent_one"]
    if abs(one["taper_angle"]["value"]) > 1e-9:
        raise Unsupported("taper")
    d1 = one["distance"]["value"] * MM
    if t == "OneSideFeatureExtentType":
        if d1 == 0 and e.get("extent_two"):
            return 0.0, e["extent_two"]["distance"]["value"] * MM
        return d1, 0.0
    if t == "SymmetricFeatureExtentType":
        if one.get("is_full_length", False):
            return d1 / 2, d1 / 2
        return d1, d1
    two = e.get("extent_two")
    if two and abs(two["taper_angle"]["value"]) > 1e-9:
        raise Unsupported("taper")
    return d1, (two["distance"]["value"] * MM if two else 0.0)


def translate(d):
    """(lines, notes) for a design; raises Unsupported."""
    ents = d["entities"]
    seq = []
    for s in d["sequence"]:
        if s["type"] == "ExtrudeFeature" and (not seq or seq[-1] != s["entity"]):
            seq.append(s["entity"])
    lines, notes = [], set()
    bodies = 0
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
        up, down = extent(e)
        off = 0.0
        st = e["start_extent"]
        if st["type"] == "OffsetStartDefinition":
            off = st.get("offset", {}).get("value", 0.0) * MM
        op = {"NewBodyFeatureOperation": "new", "JoinFeatureOperation": "add", "CutFeatureOperation": "cut", "IntersectFeatureOperation": "keep"}[e["operation"]]
        for sk, profs in by.items():
            T = ents[sk]["transform"]
            O, X, Y, Z = (vec(T["origin"]), vec(T["x_axis"]), vec(T["y_axis"]), vec(T["z_axis"]))
            O = tuple(x * MM for x in O)
            lift = 0.0
            try:
                regs = regions(profs)
            except (Unsupported, IndexError):
                # one profile at a time, the way it was drawn
                notes.add("unmerged")
                regs = [r for p in profs for r in regions([p])]
            for r in regs:
                for lp in r:
                    for ed, _ in lp:
                        if ed.approx:
                            notes.add("approx")
                        lift = ed.c[2] if ed.kind in ("circle", "arc") else ed.a[2]
                # the slab from -down to up along the normal: from the frame turned over when it
                # all lies below, from a shifted origin when it does not reach the plane
                if not up and not down:
                    notes.add("zero-length")
                    continue
                lo, hi = (-down, up) if down else ((0.0, up) if up > 0 else (up, 0.0))
                if lo > hi:
                    lo, hi = hi, lo
                if hi <= 0:
                    z, a, b, shift = tuple(-x for x in Z), hi - lo, 0.0, hi
                elif lo >= 0:
                    z, a, b, shift = Z, hi - lo, 0.0, lo
                else:
                    z, a, b, shift = Z, hi, -lo, 0.0
                y = cross(z, X)
                org = add(O, Z, lift + off + shift)

                f2 = Frame(X, Y, y)
                frame = f"z {num(z[0])} {num(z[1])} {num(z[2])} x {num(X[0])} {num(X[1])} {num(X[2])} at {num(org[0])} {num(org[1])} {num(org[2])}"
                loops = " ".join(emit_loop(lp, f2) for lp in r)
                ext = num(a) if b == 0 else f"two {num(a)} {num(b)}"
                this = op
                if bodies == 0:
                    if op in ("cut", "keep"):
                        raise Unsupported("first feature cuts")
                    this = ""
                if this == "new" and "unmerged" in notes and r is not regs[0]:
                    # a feature's profiles that would not merge still make one body
                    this = "add"
                if this == "new" or this == "":
                    bodies += 1
                    tail = ""
                else:
                    tail = " in all" if bodies > 1 and op != "add" else ""
                lines.append(f"extrude {this + ' ' if this else ''}{ext} {frame} {loops}{tail}")
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
