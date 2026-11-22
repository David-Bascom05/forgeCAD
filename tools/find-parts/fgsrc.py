"""Writing .fg part text for the generators: frames, profiles, features. A Part keeps its own names, so
fragments (a base, then a feature, then another) compose into one program.

Placement follows the kernel's frames: a frame is a world orientation (xy xz yz -xy -xz -yz: x, y and
normal as `help forge` gives the world planes, the minus ones turned over about x) moved to a point.
Sketches sit on `offset(PLANE, d)`, or a `plane(at:, normal:, x:)` for a turned-over frame, and their
coordinates are shifted to put the profile where the frame puts it, so the kernel builds the same
feature as a frame at that point would. Holes drill against their plane's normal and cuts run against
the sketch's, so their planes face the other way to the frame's direction (`help forge`: Holes, Features).
The style rules a generated placement breaks on purpose (numbers outside params, world planes after the
first feature) are off in `forge_lang::fuzz::options`, which the judge builds with.
"""
import decimal

X, Y, Z = (1.0, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)


def neg(v):
    return tuple(-a for a in v)


# x, y, normal
FRAMES = {
    "xy": (X, Y, Z),
    "xz": (X, Z, neg(Y)),
    "yz": (Y, Z, X),
    "-xy": (X, neg(Y), neg(Z)),
    "-xz": (X, neg(Z), Y),
    "-yz": (Y, neg(Z), neg(X)),
}
# the world plane each normal's frame sits on, and its normal
BASE = {Z: ("xy", Z), neg(Z): ("xy", Z), Y: ("xz", neg(Y)), neg(Y): ("xz", neg(Y)), X: ("yz", X), neg(X): ("yz", X)}
DIRS = {X: "X", Y: "Y", Z: "Z", neg(X): "-X", neg(Y): "-Y", neg(Z): "-Z"}


def num(v):
    """A number as the language reads it: no exponent, every digit Python's repr keeps."""
    v = float(v) + 0.0
    if v == int(v) and abs(v) < 1e15:
        return str(int(v))
    s = format(decimal.Decimal(repr(v)), "f")
    return s


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


class Frame:
    """A kernel frame: an origin and the named orientation's axes."""

    def __init__(self, orient="xy", at=(0.0, 0.0, 0.0)):
        self.x, self.y, self.n = FRAMES[orient]
        self.at = tuple(float(a) for a in at)

    def world(self, u, v, w=0.0):
        return tuple(self.at[i] + u * self.x[i] + v * self.y[i] + w * self.n[i] for i in range(3))


class Part:
    def __init__(self):
        self.lines = []
        self.counts = {}
        self.last_sketch = None
        self.points = {}

    def name(self, prefix):
        k = self.counts.get(prefix, 0) + 1
        self.counts[prefix] = k
        return f"{prefix}{k}"

    def add(self, text):
        self.lines.append(text)

    def text(self):
        return "\n".join(self.lines) + "\n"

    # ---- placement ----

    def plane_for(self, f, flip=False):
        """A plane at f's origin with f's x and normal (flip: the normal turned over, the x kept), and
        the shift that takes f's coordinates to the plane's: (plane, du, dv)."""
        n = neg(f.n) if flip else f.n
        y = tuple(n[(i + 1) % 3] * f.x[(i + 2) % 3] - n[(i + 2) % 3] * f.x[(i + 1) % 3] for i in range(3))
        world, wn = BASE[n]
        # the world plane moved along its own normal to f's origin
        d = dot(f.at, wn)
        base = f"offset({world}, {num(d)})" if d else world
        origin = tuple(d * a for a in wn)
        wx, wy, _ = FRAMES[world]
        if wn == n and wx == f.x:
            pl = base
        else:
            pl = f"plane(at: {base}, normal: {DIRS[n]}, x: {DIRS[f.x]})"
            wx, wy = f.x, y
        rel = sub(f.at, origin)
        # f's (u, v) is the plane's (du + u, dv + v * s): s turns v over with a flipped normal
        s = 1.0 if dot(y, f.y) > 0 else -1.0
        return pl, dot(rel, wx), dot(rel, wy), s

    def sketch(self, f, rows, flip=False, prefix="s"):
        """A sketch of rows (functions of a point map (u, v) -> text) on f's plane; its name."""
        pl, du, dv, s = self.plane_for(f, flip)

        def p(u, v):
            return f"({num(du + u)}, {num(dv + v * s)})"

        name = self.name(prefix)
        body = "\n".join("  " + r(p, s) for r in rows)
        self.add(f"sketch {name}(on: {pl}) {{\n{body}\n}}")
        return name

    def sketch_on(self, plane, rows, prefix="s"):
        """A sketch of rows (fgsrc row functions, in the plane's own coordinates) on a plane; its name."""
        name = self.name(prefix)
        body = "\n".join("  " + r(lambda u, v: f"({num(u)}, {num(v)})", 1.0) for r in rows)
        self.add(f"sketch {name}(on: {plane}) {{\n{body}\n}}")
        return name

    def point(self, pt):
        """A world point, as a helper sketch's fixed point (one sketch per point, kept for the next use)."""
        key = tuple(float(a) + 0.0 for a in pt)
        if key in self.points:
            return self.points[key]
        name = self.sketch(Frame("xy", (0, 0, pt[2])), [lambda p, s: f"q = pt{p(pt[0], pt[1])}"], prefix="q")
        self.points[key] = f"{name}.q"
        return self.points[key]

    def dir(self, d):
        """A direction: a world axis by name, else the axis from the origin to d (helper points, so the
        direction is d's own numbers)."""
        w = tuple(float(a) + 0.0 for a in d)
        if w in DIRS:
            return DIRS[w]
        return f"axis({self.point((0.0, 0.0, 0.0))}, {self.point(w)})"

    def plane(self, o, z, x):
        """The plane at world point o, its normal z and x along x (any directions)."""
        return f"plane(at: {self.point(o)}, normal: {self.dir(z)}, x: {self.dir(x)})"

    def axis(self, o, d):
        """The axis through o along d."""
        a = self.point(o)
        b = self.point(tuple(o[i] + d[i] for i in range(3)))
        return f"axis({a}, {b})"


# ---- profiles: rows of a sketch, in the frame's coordinates ----


def rect(x, y, w, h, nm="o"):
    return lambda p, s: f"{nm} = rect(corner: {p(x, y) if s > 0 else p(x, y + h)}, size: ({num(w)}, {num(h)}))"


def circle(cx, cy, r, nm="o"):
    return lambda p, s: f"{nm} = circle({p(cx, cy)}, {num(r)})"


def poly(pts, nm="o"):
    return lambda p, s: f"{nm} = poly([{', '.join(p(x, y) for x, y in pts)}])"


def rrect(x, y, w, h, r, nm="o"):
    cs = [(x + r, y + r), (x + w - r, y + r), (x + w - r, y + h - r), (x + r, y + h - r)]
    return lambda p, s: f"{nm} = hull({', '.join(f'disc({p(a, b)}, {num(r)})' for a, b in (cs if s > 0 else cs[::-1]))})"


def slot(ax, ay, bx, by, r, nm="o"):
    return lambda p, s: f"{nm} = slot({p(ax, ay)}, {p(bx, by)}, {num(r)})"


def line(ax, ay, bx, by, nm="l", construction=False):
    c = ", construction: true" if construction else ""
    return lambda p, s: f"{nm} = line({p(ax, ay)}, {p(bx, by)}{c})"


def pt(x, y, nm="p"):
    return lambda p, s: f"{nm} = pt{p(x, y)}"


def loops(*ls):
    """Profiles named o (the outline), i1, i2.. (holes)."""
    out = []
    for k, l in enumerate(ls):
        kind, *a = l
        out.append({"rect": rect, "circle": circle, "poly": poly, "rrect": rrect, "slot": slot}[kind](*a, nm="o" if k == 0 else f"i{k}"))
    return out


# ---- features ----

OPS = {"add": "", "new": ", new: true", "keep": ", keep: true", "cut": ", cut: true"}


def extent(ext, cut):
    """An extrude's extent as the frame runs it: a number, ("sym", d), ("two", f, b), "thru", "next"."""
    if isinstance(ext, (int, float)):
        return num(-ext if cut else ext)
    if ext == "thru":
        return "thru, flip: true" if cut else "thru"
    if ext == "next":
        return "next, flip: true" if cut else "next"
    if ext[0] == "sym":
        return f"sym({num(ext[1])})"
    if ext[0] == "two":
        f, b = ext[1], ext[2]
        return f"two({num(-f)}, {num(-b)})" if cut else f"two({num(f)}, {num(b)})"
    raise ValueError(ext)


def extrude(P, op, ext, f, rows, label=None):
    """extrude [op] EXT FRAME LOOPS: added along the frame's normal; a cut runs that way too."""
    sk = P.sketch(f, rows)
    P.last_sketch = sk
    nm = label or P.name("e")
    call = {"add": "extrude", "new": "extrude", "cut": "cut", "keep": "intersect"}[op]
    new = ", new: true" if op == "new" else ""
    P.add(f"{nm} = {call}({sk}, {extent(ext, op == 'cut')}{new})")
    return nm


def revolve(P, op, f, rows, label=None):
    """revolve [op] FRAME LOOPS: a whole turn about the frame's y axis."""
    sk = P.sketch(f, rows + [line(0, 0, 0, 1, nm="ax", construction=True)])
    nm = label or P.name("rv")
    P.add(f"{nm} = revolve({sk}, axis: {sk}.ax{OPS[op]})")
    return nm


def ball(P, op, r, at, label=None):
    c = P.point(at)
    nm = label or P.name("bl")
    P.add(f"{nm} = ball({c}, {num(r)}{OPS[op]})")
    return nm


def depth(d):
    return d if isinstance(d, str) else num(d)


def hole(P, size, f, d, label=None, extra=""):
    """hole D FRAME DEPTH: the mouth at the frame's origin, drilled along its normal. size: a diameter, or
    a screw's arguments as text (`screw: "M6", seat: "csink"`)."""
    sk = P.sketch(f, [pt(0, 0, nm="m")], flip=True, prefix="hs")
    nm = label or P.name("hl")
    sz = size if isinstance(size, str) else f"d: {num(size)}"
    P.add(f"{nm} = hole(at: {sk}.m, {sz}, depth: {depth(d)}{extra})")
    return nm


def stmt(P, prefix, call):
    nm = P.name(prefix)
    P.add(f"{nm} = {call}")
    return nm
