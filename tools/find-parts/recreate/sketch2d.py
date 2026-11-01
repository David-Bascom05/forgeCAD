"""A sketch's plane arrangement: curves split where they meet, the regions they bound, and any set of
regions' boundary as loops. Lines and arcs are exact; ellipses and splines are fine polylines (approx)."""
import math

EPS = 1e-5  # mm: points this close are one point


class Unsupported(Exception):
    pass


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1])


def dist(a, b):
    return math.hypot(a[0] - b[0], a[1] - b[1])


class Piece:
    """A stretch of one entity between arrangement vertices, along the entity's direction."""

    def __init__(self, ent, kind, a, b, c=None, r=0.0, ccw=True, pts=None):
        self.ent, self.kind, self.a, self.b, self.c, self.r, self.ccw, self.pts = ent, kind, a, b, c, r, ccw, pts
        self.others = []  # entities drawn over this one, and whether they run the same way

    def sweep(self):
        a0 = math.atan2(self.a[1] - self.c[1], self.a[0] - self.c[0])
        a1 = math.atan2(self.b[1] - self.c[1], self.b[0] - self.c[0])
        d = a1 - a0
        if self.ccw:
            while d <= 1e-12:
                d += 2 * math.pi
        else:
            while d >= -1e-12:
                d -= 2 * math.pi
        return a0, d

    def sample(self, n=24):
        if self.kind == "line":
            return [self.a, self.b]
        if self.kind == "poly":
            return self.pts
        a0, d = self.sweep()
        return [self.a] + [(self.c[0] + self.r * math.cos(a0 + d * i / n), self.c[1] + self.r * math.sin(a0 + d * i / n)) for i in range(1, n)] + [self.b]

    def near(self, p, tol=1e-4):
        return curve_dist(self, p) <= tol


def curve_dist(pc, p):
    if pc.kind == "point":
        return dist(pc.a, p)
    if pc.kind == "circle":
        return abs(dist(p, pc.c) - pc.r)
    if pc.kind == "arc":
        a0, d = pc.sweep()
        t = math.atan2(p[1] - pc.c[1], p[0] - pc.c[0])
        off = (t - a0) % (2 * math.pi) if d > 0 else (a0 - t) % (2 * math.pi)
        if off <= abs(d):
            return abs(dist(p, pc.c) - pc.r)
        return min(dist(p, pc.a), dist(p, pc.b))
    pts = pc.sample()
    best = math.inf
    for u, v in zip(pts, pts[1:]):
        e = sub(v, u)
        l2 = e[0] ** 2 + e[1] ** 2
        t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - u[0]) * e[0] + (p[1] - u[1]) * e[1]) / l2))
        best = min(best, math.hypot(u[0] + t * e[0] - p[0], u[1] + t * e[1] - p[1]))
    return best


def spline_points(pts, d0=None, d1=None, per=24):
    """A C2 cubic through pts by chord length, with the given end derivatives or natural ends."""
    n = len(pts) - 1
    if n < 1:
        raise Unsupported("spline of one point")
    h = [max(dist(pts[i], pts[i + 1]), 1e-12) for i in range(n)]
    out = [pts[0]]
    ms = []
    for k in (0, 1):
        # tridiagonal for first derivatives
        a, b, c, r = [0.0] * (n + 1), [0.0] * (n + 1), [0.0] * (n + 1), [0.0] * (n + 1)
        for i in range(1, n):
            a[i], b[i], c[i] = h[i], 2 * (h[i] + h[i - 1]), h[i - 1]
            r[i] = 3 * (h[i] * (pts[i][k] - pts[i - 1][k]) / h[i - 1] + h[i - 1] * (pts[i + 1][k] - pts[i][k]) / h[i])
        if d0 is not None:
            b[0], r[0] = 1.0, d0[k]
        else:
            b[0], c[0], r[0] = 2.0, 1.0, 3 * (pts[1][k] - pts[0][k]) / h[0]
        if d1 is not None:
            b[n], r[n] = 1.0, d1[k]
        else:
            a[n], b[n], r[n] = 1.0, 2.0, 3 * (pts[n][k] - pts[n - 1][k]) / h[n - 1]
        for i in range(1, n + 1):
            w = a[i] / b[i - 1]
            b[i] -= w * c[i - 1]
            r[i] -= w * r[i - 1]
        m = [0.0] * (n + 1)
        m[n] = r[n] / b[n]
        for i in range(n - 1, -1, -1):
            m[i] = (r[i] - c[i] * m[i + 1]) / b[i]
        ms.append(m)
    for i in range(n):
        for j in range(1, per + 1):
            t = j / per
            h00, h10, h01, h11 = 2 * t**3 - 3 * t**2 + 1, t**3 - 2 * t**2 + t, -2 * t**3 + 3 * t**2, t**3 - t**2
            out.append(tuple(h00 * pts[i][k] + h10 * h[i] * ms[k][i] + h01 * pts[i + 1][k] + h11 * h[i] * ms[k][i + 1] for k in (0, 1)))
    return out


def circle_from(a, m, b):
    ax, ay, bx, by, cx, cy = a[0], a[1], m[0], m[1], b[0], b[1]
    d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    if abs(d) < 1e-14:
        return None
    ux = ((ax**2 + ay**2) * (by - cy) + (bx**2 + by**2) * (cy - ay) + (cx**2 + cy**2) * (ay - by)) / d
    uy = ((ax**2 + ay**2) * (cx - bx) + (bx**2 + by**2) * (ax - cx) + (cx**2 + cy**2) * (bx - ax)) / d
    return (ux, uy), math.hypot(ax - ux, ay - uy), d > 0


def entities(ents):
    """Non-construction curves as whole-entity Pieces (a circle: None ends, split later), and all by id."""
    out, every, approx = [], {}, False
    for e in ents:
        k = e["kind"]
        p = None
        if k == "skLineSegment":
            p = Piece(e["id"], "line", tuple(e["start"]), tuple(e["end"]))
        elif k == "skCircle":
            c = tuple(e["center"])
            p = Piece(e["id"], "circle", (c[0] + e["radius"], c[1]), (c[0] + e["radius"], c[1]), c, e["radius"])
        elif k == "skArc":
            a, m, b = tuple(e["start"]), tuple(e["mid"]), tuple(e["end"])
            cf = circle_from(a, m, b)
            if cf is None:
                p = Piece(e["id"], "line", a, b)
            else:
                p = Piece(e["id"], "arc", a, b, cf[0], cf[1], cf[2])
        elif k == "skEllipse":
            c, ma, r1, r2 = e["center"], e["majorAxis"], e["majorRadius"], e["minorRadius"]
            l = math.hypot(*ma)
            ux, uy = ma[0] / l, ma[1] / l
            pts = [(c[0] + r1 * math.cos(t) * ux - r2 * math.sin(t) * uy, c[1] + r1 * math.cos(t) * uy + r2 * math.sin(t) * ux) for t in (2 * math.pi * i / 96 for i in range(96))]
            pts.append(pts[0])
            p = Piece(e["id"], "poly", pts[0], pts[-1], pts=pts)
            approx = approx or not e.get("construction")
        elif k == "skFitSpline":
            pts = [tuple(x) for x in e["points"]]
            d0, d1 = e.get("startDerivative"), e.get("endDerivative")
            sp = spline_points(pts, d0, d1)
            p = Piece(e["id"], "poly", sp[0], sp[-1], pts=sp)
            approx = approx or not e.get("construction")
        elif k == "skPoint":
            every[e["id"]] = Piece(e["id"], "point", tuple(e["position"]), tuple(e["position"]))
            continue
        elif k == "skText":
            if not e.get("construction"):
                raise Unsupported("sketch text")
            continue
        else:
            if not e.get("construction"):
                raise Unsupported("sketch curve " + k)
            continue
        every[e["id"]] = p
        if not e.get("construction"):
            out.append(p)
    return out, every, approx


def line_x(a, b):
    """Parameters and points where two segments cross (their ends count)."""
    r, s = sub(a.b, a.a), sub(b.b, b.a)
    d = r[0] * s[1] - r[1] * s[0]
    w = sub(b.a, a.a)
    if abs(d) < 1e-12:
        out = []
        for p in (b.a, b.b):
            if a.near(p, EPS):
                out.append(p)
        for p in (a.a, a.b):
            if b.near(p, EPS):
                out.append(p)
        return out
    t, u = (w[0] * s[1] - w[1] * s[0]) / d, (w[0] * r[1] - w[1] * r[0]) / d
    la, lb = max(math.hypot(*r), 1e-12), max(math.hypot(*s), 1e-12)
    if -EPS / la <= t <= 1 + EPS / la and -EPS / lb <= u <= 1 + EPS / lb:
        return [(a.a[0] + t * r[0], a.a[1] + t * r[1])]
    return []


def on_arc(pc, p):
    return pc.kind == "circle" or curve_dist(pc, p) <= EPS * 5


def line_circle(l, c):
    d = sub(l.b, l.a)
    f = sub(l.a, c.c)
    A, B, C = d[0] ** 2 + d[1] ** 2, 2 * (f[0] * d[0] + f[1] * d[1]), f[0] ** 2 + f[1] ** 2 - c.r**2
    if A == 0:
        return []
    disc = B * B - 4 * A * C
    if disc < -1e-9 * max(A * c.r * c.r, 1e-12):
        return []
    disc = math.sqrt(max(disc, 0.0))
    out = []
    la = math.sqrt(A)
    for t in ((-B - disc) / (2 * A), (-B + disc) / (2 * A)):
        if -EPS / la <= t <= 1 + EPS / la:
            p = (l.a[0] + t * d[0], l.a[1] + t * d[1])
            if on_arc(c, p):
                out.append(p)
    return out


def circle_circle(a, b):
    d = dist(a.c, b.c)
    if d < 1e-12 or d > a.r + b.r + EPS or d < abs(a.r - b.r) - EPS:
        return []
    x = (d * d + a.r * a.r - b.r * b.r) / (2 * d)
    h = math.sqrt(max(a.r * a.r - x * x, 0.0))
    u = ((b.c[0] - a.c[0]) / d, (b.c[1] - a.c[1]) / d)
    m = (a.c[0] + x * u[0], a.c[1] + x * u[1])
    ps = [(m[0] - h * u[1], m[1] + h * u[0]), (m[0] + h * u[1], m[1] - h * u[0])]
    return [p for p in ps if on_arc(a, p) and on_arc(b, p)]


def segs(p):
    if p.kind == "poly":
        return [Piece(p.ent, "line", u, v) for u, v in zip(p.pts, p.pts[1:]) if dist(u, v) > 0]
    return [p]


def box(p):
    if p.kind == "circle":
        return (p.c[0] - p.r, p.c[1] - p.r, p.c[0] + p.r, p.c[1] + p.r)
    if p.kind == "arc":
        # the whole circle: enough to rule out most pairs
        return (p.c[0] - p.r, p.c[1] - p.r, p.c[0] + p.r, p.c[1] + p.r)
    ps = p.pts if p.kind == "poly" else [p.a, p.b]
    xs, ys = [q[0] for q in ps], [q[1] for q in ps]
    return (min(xs), min(ys), max(xs), max(ys))


def apart(a, b):
    return a[2] < b[0] - EPS or b[2] < a[0] - EPS or a[3] < b[1] - EPS or b[3] < a[1] - EPS


def meets(a, b):
    out = []
    for x in segs(a):
        for y in segs(b):
            if x.kind == "line" and y.kind == "line":
                out += line_x(x, y)
            elif x.kind == "line":
                out += line_circle(x, y)
            elif y.kind == "line":
                out += line_circle(y, x)
            else:
                out += circle_circle(x, y)
    return out


def split(p, pts):
    """p cut at the points on it, in order along it."""
    if p.kind == "line":
        d = sub(p.b, p.a)
        l2 = d[0] ** 2 + d[1] ** 2
        ts = sorted({0.0, 1.0} | {max(0.0, min(1.0, ((q[0] - p.a[0]) * d[0] + (q[1] - p.a[1]) * d[1]) / l2)) for q in pts} if l2 else {0.0, 1.0})
        ps = [(p.a[0] + t * d[0], p.a[1] + t * d[1]) for t in ts]
        out = []
        for u, v in zip(ps, ps[1:]):
            if dist(u, v) > EPS:
                out.append(Piece(p.ent, "line", u, v))
        return out
    if p.kind == "arc" or p.kind == "circle":
        if p.kind == "circle":
            a0, d = math.atan2(p.a[1] - p.c[1], p.a[0] - p.c[0]), 2 * math.pi
        else:
            a0, d = p.sweep()
        off = set()
        for q in pts:
            t = math.atan2(q[1] - p.c[1], q[0] - p.c[0])
            o = (t - a0) % (2 * math.pi) if d > 0 else (a0 - t) % (2 * math.pi)
            if o <= abs(d):
                off.add(o)
        if p.kind == "circle" and off:
            a0 += min(off) * (1 if d > 0 else -1)
            off = {x - min(off) for x in off}
        ts = sorted({0.0, abs(d)} | off)
        ps = [(p.c[0] + p.r * math.cos(a0 + (1 if d > 0 else -1) * t), p.c[1] + p.r * math.sin(a0 + (1 if d > 0 else -1) * t)) for t in ts]
        if len(ts) == 2 and p.kind == "circle":
            return [Piece(p.ent, "circle", ps[0], ps[0], p.c, p.r, True)]
        out = []
        for u, v, ta, tb in zip(ps, ps[1:], ts, ts[1:]):
            if p.r * (tb - ta) > EPS:
                out.append(Piece(p.ent, "arc", u, v, p.c, p.r, d > 0))
        return out
    # polyline: cut by index
    out, cur = [], [p.pts[0]]
    for u, v in zip(p.pts, p.pts[1:]):
        s = Piece(p.ent, "line", u, v)
        hits = sorted((dist(u, q), q) for q in pts if s.near(q, EPS * 2) and dist(u, q) > EPS and dist(v, q) > EPS)
        for _, q in hits:
            cur.append(q)
            out.append(Piece(p.ent, "poly", cur[0], cur[-1], pts=cur))
            cur = [q]
        cur.append(v)
        if any(dist(v, q) <= EPS * 2 for q in pts) and v != p.pts[-1]:
            out.append(Piece(p.ent, "poly", cur[0], cur[-1], pts=cur))
            cur = [v]
    if len(cur) > 1:
        out.append(Piece(p.ent, "poly", cur[0], cur[-1], pts=cur))
    return out


class Arrangement:
    def __init__(self, ents):
        self.angles = {}
        self.polys = {}
        self.curves, self.every, self.approx = entities(ents)
        if len(self.curves) > 400:
            raise Unsupported("sketch of over 400 curves")
        hits = [[] for _ in self.curves]
        boxes = [box(c) for c in self.curves]
        for i, a in enumerate(self.curves):
            for j in range(i + 1, len(self.curves)):
                if apart(boxes[i], boxes[j]):
                    continue
                ps = meets(a, self.curves[j])
                hits[i] += ps
                hits[j] += ps
            # a closed spline or polyline meets itself only at its ends
        self.pieces = []
        for c, h in zip(self.curves, hits):
            self.pieces += split(c, h)
        self.verts = []
        keep = []
        for p in self.pieces:
            p.va = self.vert(p.a)
            p.vb = self.vert(p.b)
            if p.va != p.vb or p.kind in ("circle", "poly"):
                keep.append(p)
        # curves drawn over each other are one curve; each entity still names it
        once = {}
        for p in keep:
            if p.kind == "line":
                mid = ((p.a[0] + p.b[0]) / 2, (p.a[1] + p.b[1]) / 2)
            elif p.kind == "circle":
                mid = p.c
            else:
                s = p.sample(8)
                mid = s[len(s) // 2]
            k = (min(p.va, p.vb), max(p.va, p.vb), p.kind == "circle" and p.r, round(mid[0], 4), round(mid[1], 4))
            q = once.setdefault(k, p)
            if q is not p:
                q.others.append((p.ent, p.va == q.va))
        self.pieces = list(once.values())
        self.walk()

    def vert(self, q):
        for i, v in enumerate(self.verts):
            if dist(v, q) <= EPS * 2:
                return i
        self.verts.append(q)
        return len(self.verts) - 1

    def walk(self):
        # prune what bounds nothing: a free end, or a bridge with one region on both sides
        live = set(range(len(self.pieces)))
        while True:
            while True:
                deg = {}
                for i in live:
                    p = self.pieces[i]
                    if p.kind == "circle":
                        continue
                    deg[p.va] = deg.get(p.va, 0) + 1
                    deg[p.vb] = deg.get(p.vb, 0) + 1
                dead = {i for i in live if self.pieces[i].kind != "circle" and (deg[self.pieces[i].va] == 1 or deg[self.pieces[i].vb] == 1)}
                if not dead:
                    break
                live -= dead
            self.live = live
            self.polys = {}
            self.faces_from(trace({(i, f) for i in live for f in (True, False)}, self))
            bridge = {i for i in live if self.left.get((i, True)) == self.left.get((i, False))}
            if not bridge:
                return
            live = live - bridge

    def faces_from(self, loops):
        self.loops = loops
        self.left = {}
        outers, holes = [], []
        for k, lp in enumerate(self.loops):
            (outers if area(lp, self) > 0 else holes).append(k)
        self.faces = []
        for k in outers:
            self.faces.append({"outer": k, "holes": [], "area": area(self.loops[k], self)})
        for k in holes:
            pt = point_of(self.loops[k], self)
            best = None
            h = -area(self.loops[k], self)
            for f in self.faces:
                if h < f["area"] * (1 - 1e-6) and inside(pt, self.loops[f["outer"]], self) and (best is None or f["area"] < best["area"]):
                    best = f
            if best:
                best["holes"].append(k)
        for fi, f in enumerate(self.faces):
            for k in [f["outer"]] + f["holes"]:
                for he in self.loops[k]:
                    self.left[he] = fi

    def pieces_of(self, ent):
        return [i for i in self.live if self.pieces[i].ent == ent or any(o[0] == ent for o in self.pieces[i].others)]

    def region_at(self, piece, side, ent=None):
        """The region to the left (side > 0) or right of a piece, along its entity's direction."""
        p = self.pieces[piece]
        if ent is not None and p.ent != ent and not any(o == (ent, True) for o in p.others):
            side = -side
        return self.left.get((piece, side > 0))

    def outermost(self):
        """Regions not inside another region's hole."""
        out = []
        for i, f in enumerate(self.faces):
            pt = point_within(self.loops[f["outer"]], self)
            if not any(j != i and inside(pt, self.loops[g["outer"]], self) and g["area"] > f["area"] for j, g in enumerate(self.faces)):
                out.append(i)
        return out

    def merged(self, regions):
        """The boundary of a set of regions as profiles: [outer loop, hole loop..], each loop of half-edges."""
        rs = set(regions)
        if len(rs) == 1:
            f = self.faces[next(iter(rs))]
            return [[self.loops[f["outer"]]] + [self.loops[h] for h in f["holes"]]]
        hes = set()
        for r in rs:
            f = self.faces[r]
            for k in [f["outer"]] + f["holes"]:
                for he in self.loops[k]:
                    if self.left.get((he[0], not he[1])) not in rs:
                        hes.add(he)
        loops = trace(hes, self)
        pos = [l for l in loops if area(l, self) > 0]
        neg = [l for l in loops if area(l, self) <= 0]
        groups = [[o] for o in sorted(pos, key=lambda l: -area(l, self))]
        for h in neg:
            pt = point_of(h, self)
            best = None
            for g in groups:
                if -area(h, self) < area(g[0], self) * (1 - 1e-6) and inside(pt, g[0], self) and (best is None or area(g[0], self) < area(best[0], self)):
                    best = g
            if best is None:
                raise Unsupported("a hole outside its region")
            best.append(h)
        return groups


def tangent_angle(arr, he):
    t = arr.angles.get(he)
    if t is None:
        i, fwd = he
        p = arr.pieces[i]
        pts = p.sample(48)
        if not fwd:
            pts = pts[::-1]
        a, b = pts[0], pts[1]
        t = arr.angles[he] = math.atan2(b[1] - a[1], b[0] - a[0])
    return t


def trace(hes, arr):
    """Cycles of half-edges with the face on their left: at each vertex turn most clockwise."""
    out_at = {}
    loops = []
    used = set()
    for he in hes:
        p = arr.pieces[he[0]]
        if p.kind == "circle":
            continue
        v = p.va if he[1] else p.vb
        out_at.setdefault(v, []).append(he)
    for v in out_at:
        out_at[v].sort(key=lambda h: tangent_angle(arr, h))
    for he in sorted(hes):
        p = arr.pieces[he[0]]
        if he in used:
            continue
        if p.kind == "circle":
            used.add(he)
            loops.append([he])
            continue
        lp, cur = [], he
        while cur not in used:
            used.add(cur)
            lp.append(cur)
            q = arr.pieces[cur[0]]
            v = q.vb if cur[1] else q.va
            twin = (cur[0], not cur[1])
            opts = out_at.get(v, [])
            if not opts:
                raise Unsupported("an open boundary")
            # the arriving edge's reverse direction, then the next clockwise from it
            ra = tangent_angle(arr, twin) if twin in hes or True else 0.0
            def turn(h):
                d = (ra - tangent_angle(arr, h)) % (2 * math.pi)
                return d if d > 1e-9 else 2 * math.pi
            cur = min(opts, key=turn)
        loops.append(lp)
    return loops


def polygon(lp, arr):
    key = tuple(lp)
    got = arr.polys.get(key)
    if got is None:
        got = arr.polys[key] = polygon_of(lp, arr)
    return got


def polygon_of(lp, arr):
    pts = []
    for i, fwd in lp:
        p = arr.pieces[i]
        if p.kind == "circle":
            n = 96
            s = [(p.c[0] + p.r * math.cos(2 * math.pi * k / n), p.c[1] + p.r * math.sin(2 * math.pi * k / n)) for k in range(n)]
            return s if fwd else s[::-1]
        s = p.sample()
        pts += (s if fwd else s[::-1])[:-1]
    return pts


def area(lp, arr):
    ps = polygon(lp, arr)
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ps, ps[1:] + ps[:1])) / 2


def inside(q, lp, arr):
    ps = polygon(lp, arr)
    c = False
    for a, b in zip(ps, ps[1:] + ps[:1]):
        if (a[1] > q[1]) != (b[1] > q[1]) and q[0] < (b[0] - a[0]) * (q[1] - a[1]) / (b[1] - a[1]) + a[0]:
            c = not c
    return c


def point_within(lp, arr):
    """A point just inside a counter-clockwise loop: off the boundary any neighbouring region shares."""
    ps = polygon(lp, arr)
    a, b = ps[0], ps[1]
    l = max(dist(a, b), 1e-12)
    d = min(1e-4, l / 8)
    return ((a[0] + b[0]) / 2 - d * (b[1] - a[1]) / l, (a[1] + b[1]) / 2 + d * (b[0] - a[0]) / l)


def point_of(lp, arr):
    # the middle of a boundary stretch: off any vertex it may share with a loop around it
    ps = polygon(lp, arr)
    return ((ps[0][0] + ps[1][0]) / 2, (ps[0][1] + ps[1][1]) / 2)
