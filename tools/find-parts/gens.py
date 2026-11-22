"""Part generators. Each yields (name, source), the source a .fg program. Nothing here reads downloaded data.

Generated placements are written as the kernel's frames would put them (fgsrc), so a generated part is
the feature history a person would draw, numbers in the statements: the judge builds with the style rules
that would flag that off (forge_lang::fuzz::options).
"""
import ast
import math
import os
import random
import re

from fgsrc import X, Y, Z, Frame, Part, ball, extrude, hole, line, loops, num, revolve, stmt

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BENCH = os.path.join(ROOT, "interface/forge-lang/parts/bench")


def bench():
    for f in sorted(os.listdir(BENCH)):
        if f.endswith(".fg"):
            with open(os.path.join(BENCH, f)) as h:
                yield f[:-3], h.read()


# -- parameter sweeps of real parts: the edits that break CAD ---------------------------------

SCALES = (0.25, 0.5, 0.75, 0.9, 0.99, 1.01, 1.1, 1.25, 1.5, 2.0, 3.0)
NUDGES = (1e-3, 1e-6, 1e-8)
# as a part's params read them, with their units taken off: angles in degrees
FNS = {
    "sqrt": math.sqrt,
    "abs": abs,
    "min": min,
    "max": max,
    "sin": lambda a: math.sin(math.radians(a)),
    "cos": lambda a: math.cos(math.radians(a)),
    "tan": lambda a: math.tan(math.radians(a)),
    "asin": lambda x: math.degrees(math.asin(x)),
    "acos": lambda x: math.degrees(math.acos(x)),
    "atan": lambda x: math.degrees(math.atan(x)),
    "atan2": lambda y, x: math.degrees(math.atan2(y, x)),
    "count": lambda x: x,
    "ratio": lambda x: x,
}
BINOPS = {ast.Add: lambda a, b: a + b, ast.Sub: lambda a, b: a - b, ast.Mult: lambda a, b: a * b, ast.Div: lambda a, b: a / b}
MAX_EXPR, MAX_POWER = 500, 64.0


def expr_value(text, env):
    """A param's EXPR, read as data: numbers, + - * / ^, names in env, calls to FNS. Anything else, a
    power beyond MAX_POWER or a text longer than MAX_EXPR raises ValueError. Every value is a float, so
    nothing grows without bound."""
    if len(text) > MAX_EXPR:
        raise ValueError("expression too long")
    try:
        tree = ast.parse(text.replace("^", "**"), mode="eval")
    except (SyntaxError, RecursionError, MemoryError) as e:
        raise ValueError(f"cannot read {text!r}") from e

    def walk(n):
        if isinstance(n, ast.Constant) and type(n.value) in (int, float):
            return float(n.value)
        if isinstance(n, ast.Name) and n.id in env:
            return float(env[n.id])
        if isinstance(n, ast.UnaryOp) and isinstance(n.op, (ast.USub, ast.UAdd)):
            v = walk(n.operand)
            return -v if isinstance(n.op, ast.USub) else v
        if isinstance(n, ast.BinOp) and type(n.op) in BINOPS:
            return float(BINOPS[type(n.op)](walk(n.left), walk(n.right)))
        if isinstance(n, ast.BinOp) and isinstance(n.op, ast.Pow):
            base, power = walk(n.left), walk(n.right)
            if abs(power) > MAX_POWER:
                raise ValueError("power out of range")
            return float(base**power)
        if isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and n.func.id in FNS and not n.keywords:
            return float(FNS[n.func.id](*[walk(a) for a in n.args]))
        raise ValueError(f"not allowed in an expression: {type(n).__name__}")

    try:
        v = walk(tree.body)
    except (ArithmeticError, TypeError, RecursionError) as e:
        raise ValueError(f"cannot evaluate {text!r}") from e
    if isinstance(v, complex) or not math.isfinite(v):
        raise ValueError(f"not a finite number: {text!r}")
    return v


# `shared NAME = EXPR` or `param FEATURE.NAME = EXPR` (an instance's own, `F[k].NAME`, too)
PARAM = re.compile(r"^(shared|param)\s+([\w.\[\]]+)\s*=\s*([^#]+?)\s*(?:#.*)?$")
DOTTED = re.compile(r"\b([A-Za-z_]\w*)\.([A-Za-z_]\w*)\b")
# data, not numbers: lists, maps, strings
DATA = re.compile(r"[\[{\"]")


def dim_of(text, dims):
    """A param's dimension as the language works it out (`help forge`: Params): mm (a length, or a
    bare number), deg, ratio, count, or None for anything else (an area, a mix)."""
    t = re.sub(r"(\d)\s*deg\b", r"\1*__deg", re.sub(r"(\d)\s*mm\b", r"\1", text))
    try:
        tree = ast.parse(t.replace("^", "**"), mode="eval")
    except SyntaxError:
        return None

    def walk(n):
        if isinstance(n, ast.Constant):
            return "bare"
        if isinstance(n, ast.Name):
            return "deg" if n.id == "__deg" else "ratio" if n.id == "pi" else dims.get(n.id)
        if isinstance(n, ast.UnaryOp):
            return walk(n.operand)
        if isinstance(n, ast.Call) and isinstance(n.func, ast.Name):
            f, xs = n.func.id, [walk(x) for x in n.args]
            if f in ("sin", "cos", "tan"):
                return "ratio"
            if f in ("asin", "acos", "atan", "atan2"):
                return "deg"
            if f in ("count", "ratio"):
                return f
            if f == "sqrt":
                return "mm" if xs[0] == "area" else xs[0]
            known = [x for x in xs if x != "bare"]
            return known[0] if known else "bare"
        if isinstance(n, ast.BinOp):
            x, y = walk(n.left), walk(n.right)
            if isinstance(n.op, (ast.Add, ast.Sub)):
                return y if x == "bare" else x
            if isinstance(n.op, ast.Mult):
                if "bare" in (x, y):
                    return y if x == "bare" else x
                if {x, y} == {"mm"}:
                    return "area"
                if "ratio" in (x, y) or "count" in (x, y):
                    o = y if x in ("ratio", "count") else x
                    return "ratio" if o == "count" and "ratio" in (x, y) else o
                return None
            if isinstance(n.op, ast.Div):
                if y == "bare":
                    return x
                if x == y and x in ("mm", "deg"):
                    return "ratio"
                if y in ("ratio", "count"):
                    return x
                return None
            if isinstance(n.op, ast.Pow):
                return "area" if x == "mm" else x
        return None

    d = walk(tree.body)
    return "mm" if d == "bare" else d


def params(src):
    """Each dimension param's line index, kind (shared or param), name, value and dimension (mm, deg
    or ratio). Lists, maps and strings are data and skipped; counts are read (dimensions use them)
    but not listed."""
    env, dims, out = {"pi": math.pi}, {}, []
    for i, l in enumerate(src.splitlines()):
        m = PARAM.match(l.strip())
        if not m:
            continue
        name, text = m.group(2), m.group(3)
        if DATA.search(text):
            continue
        key = DOTTED.sub(r"\1__\2", re.sub(r"\[\w+\]", "", name))
        body = DOTTED.sub(r"\1__\2", re.sub(r"(\d)\s*(mm|deg)\b", r"\1", text))
        # a bare name is the feature's own param first (`param F.n` reads F.n as n), then a shared one
        feat = key.split("__")[0] if "__" in key else None

        def scoped(d):
            return {**d, **{k.split("__", 1)[1]: x for k, x in d.items() if feat and k.startswith(feat + "__")}}

        try:
            v = expr_value(body, scoped(env))
        except ValueError:
            continue
        d = dim_of(DOTTED.sub(r"\1__\2", text), scoped(dims))
        env[key] = v
        dims[key] = d
        if d in ("mm", "deg", "ratio"):
            out.append((i, m.group(1), name, float(v), d))
    return out


def written(v, dim):
    """A value as a param of that dimension is written."""
    return {"mm": num(v), "deg": f"{num(v)}deg", "ratio": f"ratio({num(v)})"}[dim]


def sweep(parts=None, literals=True):
    """Every dimension across its range, and onto values that make faces coincide or tangent:
    equal to another dimension, their sum, their difference, and a hair either side of those."""
    for pn, src in parts if parts is not None else bench():
        lines = src.splitlines()
        ps = params(src)
        vals = [p[3] for p in ps if p[3] > 0 and p[4] == "mm"]
        for i, kw, name, v, unit in ps:
            cand = {v * s for s in SCALES}
            if unit == "mm":
                for w in vals:
                    if w != v:
                        cand.update({w, w / 2, w * 2})
                        for a in vals[:12]:
                            cand.update({w + a, abs(w - a)})
            near = set()
            for c in list(cand):
                if c > 0:
                    near.update({c + NUDGES[1], c - NUDGES[1]})
            cand |= {c for c in near if random.Random(hash((pn, name, c)) & 0xffff).random() < 0.2}
            cand.update(v + n for n in NUDGES)
            cand.update(v - n for n in NUDGES)
            for c in sorted(x for x in cand if x > 0 and x != v):
                t = list(lines)
                t[i] = f"{kw} {name} = {written(c, unit)}"
                yield f"{pn}/{name}={c:.9g}", "\n".join(t) + "\n"
        if not literals:
            continue
        # numbers written in the statements themselves (sketch rows, a fillet's radius)
        lit = re.compile(r"(?<![\w.@=])\d+(?:\.\d+)?(?![\w.(])")
        for i, l in enumerate(lines):
            body = l.split("#")[0]
            if PARAM.match(l.strip()) or not body.strip() or re.search(r"count\(|ratio\(|\"", body):
                continue
            for m in lit.finditer(body):
                v = float(m.group(0))
                if v == 0:
                    continue
                for s in (0.5, 0.9, 1.1, 2.0):
                    t = list(lines)
                    t[i] = body[: m.start()] + num(v * s) + body[m.end() :]
                    yield f"{pn}/L{i}.{m.start()}x{s}", "\n".join(t) + "\n"


# -- structured enumeration: every feature after every feature on each face and edge kind -----

W, D, H = 60.0, 40.0, 20.0
XY = Frame()


def at(orient="xy", x=0.0, y=0.0, z=0.0):
    return Frame(orient, (x, y, z))


def ex(P, op, ext, f, *ls, label=None):
    return extrude(P, op, ext, f, loops(*ls), label)


def box(P):
    return ex(P, "add", H, XY, ("rect", -30, -20, W, D))


def sel(P, prefix, call):
    return stmt(P, prefix, call)


def loft2(P, op, a, b):
    """loft profile FRAME LOOPS profile FRAME LOOPS."""
    s1 = P.sketch(a[0], loops(a[1]))
    s2 = P.sketch(b[0], loops(b[1]))
    flag = "" if op == "add" else f", {op}: true"
    return stmt(P, "lo", f"loft([{s1}, {s2}]{flag})")


def sweep_base(P):
    """A circle swept along a line, a quarter turn and a line in the yz plane."""
    s = P.sketch(at("-xz"), loops(("circle", 0, 0, 8)))
    path = P.name("pth")
    P.add(f"sketch {path}(on: yz) {{\n  path route(start: (0, 0)) {{\n    line(to: (30, 0))\n    tarc(to: (45, 15))\n    line(to: (45, 40))\n  }}\n}}")
    return stmt(P, "sw", f"sweep({s}, along: {path}.route)")


def thread_rod(P):
    ex(P, "add", 20, XY, ("circle", 0, 0, 6))
    return sel(P, "t", 'thread(face: faces(shape: "cyl"), spec: "M12", length: 12)')


HEX = [(20, 0), (10, 17.3205), (-10, 17.3205), (-20, 0), (-10, -17.3205), (10, -17.3205)]
CONE = [(0, 0), (20, 0), (10, 20), (0, 20)]
LBRACKET = [(-30, 0), (30, 0), (30, 6), (-24, 6), (-24, 20), (-30, 20)]

# each a function of the Part being written
BASES = {
    "box": box,
    "cyl": lambda P: ex(P, "add", H, XY, ("circle", 0, 0, 20)),
    "tube": lambda P: ex(P, "add", H, XY, ("circle", 0, 0, 20), ("circle", 0, 0, 12)),
    "cone": lambda P: revolve(P, "add", at("xz"), loops(("poly", CONE))),
    "sphere": lambda P: ball(P, "add", 15, (0, 0, 10)),
    "torus": lambda P: revolve(P, "add", at("xz", 0, 0, 10), loops(("circle", 20, 0, 8))),
    "lbracket": lambda P: ex(P, "add", ("sym", 40), at("xz"), ("poly", LBRACKET)),
    "plate_hole": lambda P: (ex(P, "add", 6, XY, ("rect", -30, -20, 60, 40)), hole(P, 8, at("-xy", 0, 0, 6), "thru")),
    "boss_plate": lambda P: (ex(P, "add", 6, XY, ("rect", -30, -20, 60, 40)), ex(P, "add", 14, at("xy", 0, 0, 6), ("circle", 0, 0, 10))),
    "fillet_box": lambda P: (box(P), sel(P, "fi", "fillet(edges(along: Z), 3)")),
    "chamfer_box": lambda P: (box(P), sel(P, "ch", "chamfer(edges(extreme: Z), 2)")),
    "shell_box": lambda P: (box(P), sel(P, "sh", "shell(2, open: faces(extreme: Z))")),
    "draft_box": lambda P: (box(P), sel(P, "dr", "draft(faces(normal: X) + faces(normal: -X), angle: 5deg, neutral: xy)")),
    "hex": lambda P: ex(P, "add", 20, XY, ("poly", HEX)),
    "rrect": lambda P: ex(P, "add", 20, XY, ("rrect", -30, -20, 60, 40, 6)),
    "slot": lambda P: ex(P, "add", 20, XY, ("slot", -20, 0, 20, 0, 10)),
    "loft": lambda P: loft2(P, "add", (XY, ("rect", -20, -20, 40, 40)), (at("xy", 0, 0, 20), ("rect", -10, -10, 20, 20))),
    "sweep": sweep_base,
    "thread_rod": thread_rod,
}


def add_on_top(P):
    s = P.name("s")
    P.add(f"sketch {s}(on: faces(extreme: Z)) {{\n  o = circle((0, 0), 3)\n}}")
    return sel(P, "e", f"extrude({s}, 5)")


def pattern_lin(P):
    x = ex(P, "add", 3, at("xy", -15, -10, 0), ("circle", 0, 0, 2))
    return sel(P, "pa", f"pattern(of: {x}, layout: lin(along: X, n: count(3), step: 6))")


def pattern_circ(P):
    x = hole(P, 3, at("-xy", 12, 0, 40), "thru")
    return sel(P, "pa", f"pattern(of: {x}, layout: circ(about: {P.axis((0, 0, 0), Z)}, n: count(4)))")


def replace_top(P):
    wp = sel(P, "wp", "workplane(on: offset(xy, 25))")
    return sel(P, "rf", f"replace_face(faces(extreme: Z), with: {wp})")


def rib(P):
    s = P.sketch(at("xz"), [line(-20, 5, 20, 5)])
    return sel(P, "rb", f"rib({s}.l, 2)")


def thread_hole(P):
    hole(P, 8.4, at("-xy", 0, 0, 40), "thru")
    return sel(P, "t", 'thread(on: offset(xy, 40), spec: "M10", internal: true, length: 10)')


def sweep_cut(P):
    s = P.sketch(at("yz", 0, 0, 15), loops(("circle", 0, 0, 1)))
    path = P.sketch(at("xz"), [line(0, 15, 20, 15)], prefix="pth")
    return sel(P, "sw", f"sweep({s}, along: {path}.l, cut: true)")


def coil_add(P):
    s = P.sketch(at("xy", 15, 0, 0), loops(("circle", 3, 0, 1)) + [line(0, 0, 0, 1, nm="ax", construction=True)])
    return sel(P, "co", f"coil({s}.o, axis: {s}.ax, pitch: 5, turns: 2)")


FEATS = {
    "fillet_top": lambda P: sel(P, "fi", "fillet(edges(extreme: Z), 1)"),
    "fillet_concave": lambda P: sel(P, "fi", 'fillet(edges(crease: "concave"), 1)'),
    "fillet_circle": lambda P: sel(P, "fi", 'fillet(edges(shape: "circle"), 1)'),
    "fillet_vertical": lambda P: sel(P, "fi", "fillet(edges(along: Z), 2)"),
    "fillet_var": lambda P: sel(P, "fi", "fillet(edges(extreme: Z), vary(1, 2))"),
    "chamfer_top": lambda P: sel(P, "ch", "chamfer(edges(extreme: Z), 1)"),
    "chamfer_circle": lambda P: sel(P, "ch", 'chamfer(edges(shape: "circle"), 0.5)'),
    "chamfer_2d": lambda P: sel(P, "ch", "chamfer(edges(extreme: Z), 1, d2: 2, face: faces(extreme: Z))"),
    "hole_thru": lambda P: hole(P, 4, at("-xy", 5, 3, 40), "thru"),
    "hole_blind": lambda P: hole(P, 4, at("-xy", 5, 3, 40), 45),
    "hole_center": lambda P: hole(P, 5, at("-xy", 0, 0, 40), "thru"),
    "hole_side": lambda P: hole(P, 3, at("yz", -40, 0, 10), "thru"),
    "hole_csink": lambda P: hole(P, 'screw: "M6", seat: "csink"', at("-xy", -6, -4, 40), "thru"),
    "cut_circle": lambda P: ex(P, "cut", "thru", at("-xy", 0, 0, 40), ("circle", 6, 2, 3)),
    "cut_rect": lambda P: ex(P, "cut", 5, at("-xy", 0, 0, 22), ("rect", -5, -5, 10, 10)),
    "cut_across": lambda P: ex(P, "cut", "thru", at("xz", 0, -50, 0), ("rect", -100, 15, 200, 10)),
    "add_on_top": add_on_top,
    "add_rect_side": lambda P: ex(P, "add", 5, at("yz", 30, 0, 10), ("rect", -3, -3, 6, 6)),
    "revolve_cut": lambda P: revolve(P, "cut", at("xz", 0, 0, 10), loops(("circle", 25, 0, 3))),
    "ball_add": lambda P: ball(P, "add", 6, (15, 0, 18)),
    "ball_cut": lambda P: ball(P, "cut", 4, (0, 0, 20)),
    "pattern_lin": pattern_lin,
    "pattern_circ": pattern_circ,
    "mirror_body": lambda P: sel(P, "mi", "pattern(whole: true, layout: mirror(on: yz))"),
    "shell_top": lambda P: sel(P, "sh", "shell(1, open: faces(extreme: Z))"),
    "draft_sides": lambda P: sel(P, "dr", "draft(faces(normal: X), angle: 3deg, neutral: xy)"),
    "offset_top": lambda P: sel(P, "of", "offset_faces(faces(extreme: Z), 1)"),
    "thicken": lambda P: sel(P, "th", "thicken(faces(extreme: Z), 1)"),
    "delete_fillet": lambda P: sel(P, "de", 'delete_faces(faces(shape: "blend"))'),
    "delete_hole": lambda P: sel(P, "de", 'delete_faces(faces(shape: "cyl"))'),
    "move_top": lambda P: sel(P, "mv", "move_faces(faces(extreme: Z), along: Z, d: 1)"),
    "replace_top": replace_top,
    "split": lambda P: sel(P, "sp", 'split(by: xz, keep: "plus")'),
    "splitface": lambda P: sel(P, "sf", "splitface(faces: faces(extreme: Z), by: yz)"),
    "rib": rib,
    "facefillet": lambda P: sel(P, "ff", "facefillet(faces(normal: X), faces(extreme: Z), 1)"),
    "fullround": lambda P: sel(P, "fr", "fullround(side: faces(normal: -Y), middle: faces(normal: X), other: faces(normal: Y))"),
    "thread_hole": thread_hole,
    "sweep_cut": sweep_cut,
    "loft_add": lambda P: loft2(P, "add", (at("xy", 0, 0, 15), ("rect", -3, -3, 6, 6)), (at("xy", 0, 0, 25), ("rect", -2, -2, 4, 4))),
    "coil_add": coil_add,
}


def program(*fns):
    P = Part()
    for f in fns:
        f(P)
    return P.text()


def enumerate_pairs(bases=None, feats=None):
    """base + F1 + F2 for every pair: the feature-after-feature matrix."""
    bases = BASES if bases is None else bases
    feats = FEATS if feats is None else feats
    for b, bt in bases.items():
        for f1, t1 in feats.items():
            yield f"{b}+{f1}", program(bt, t1)
            for f2, t2 in feats.items():
                yield f"{b}+{f1}+{f2}", program(bt, t1, t2)


# the surface-pair x configuration matrix: a tool placed against each base surface
SURF = {
    "plane": (box, 20.0),
    "cylinder": (lambda P: ex(P, "add", 20, XY, ("circle", 0, 0, 20)), 20.0),
    "cone": (lambda P: revolve(P, "add", at("xz"), loops(("poly", CONE))), 20.0),
    "sphere": (lambda P: ball(P, "add", 20, (0, 0, 0)), 20.0),
    "torus": (lambda P: revolve(P, "add", at("xz"), loops(("circle", 20, 0, 8))), 8.0),
    "fillet_cyl": (lambda P: (box(P), sel(P, "fi", "fillet(edges(along: Z), 5)")), 20.0),
    "fillet_torus": (lambda P: (ex(P, "add", 20, XY, ("circle", 0, 0, 20)), sel(P, "fi", "fillet(edges(extreme: Z), 5)")), 20.0),
    "chamfer": (lambda P: (ex(P, "add", 20, XY, ("circle", 0, 0, 20)), sel(P, "ch", "chamfer(edges(extreme: Z), 5)")), 20.0),
    "draft": (lambda P: (box(P), sel(P, "dr", "draft(faces(normal: X) + faces(normal: -X), angle: 7deg, neutral: xy)")), 20.0),
}
TOOL = {
    "cyl_z": lambda x, y, r: ("circle", x, y, r),
    "box_z": lambda x, y, r: ("rect", x - r, y - r, 2 * r, 2 * r),
    "hex_z": lambda x, y, r: ("poly", [(float(f"{x + r * math.cos(a * math.pi / 3):.9f}"), float(f"{y + r * math.sin(a * math.pi / 3):.9f}")) for a in range(6)]),
    "slot_z": lambda x, y, r: ("slot", x - r, y, x + r, y, r),
}
# placements in terms of the feature R: tangent, coaxial, coincident, through a vertex, near
CONFIG = {
    "coaxial": lambda R: (0, 0, R),
    "coaxial_bigger": lambda R: (0, 0, R + 3),
    "same_r": lambda R: (0, 0, R),
    "tangent_in": lambda R: (R - 4, 0, 4),
    "tangent_out": lambda R: (R + 4, 0, 4),
    "near_tan_in": lambda R: (R - 4 - 1e-6, 0, 4),
    "near_tan_out": lambda R: (R + 4 + 1e-6, 0, 4),
    "very_near": lambda R: (R + 4 + 1e-9, 0, 4),
    "crossing": lambda R: (R, 0, 4),
    "at_vertex": lambda R: (30, 20, 4),
    "edge_flush": lambda R: (30 - 4, 0, 4),
    "edge_over": lambda R: (30, 0, 4),
    "diagonal": lambda R: (R / math.sqrt(2), R / math.sqrt(2), 4),
    "small": lambda R: (R / 2, 1.5, 1e-3),
}
OPS = ("add", "cut", "keep")
# extent and frame
Z_TOOLS = {"through": ("thru", at("-xy", 0, 0, 200)), "blind": (12, at("-xy", 0, 0, 20)), "flush": (20, XY), "above": (10, at("xy", 0, 0, 20))}
FOLLOW = {
    "": lambda P: None,
    "fillet": lambda P: sel(P, "fi", "fillet(edges(extreme: Z), 0.5)"),
    "chamfer": lambda P: sel(P, "ch", 'chamfer(edges(crease: "concave"), 0.5)'),
    "hole": lambda P: hole(P, 2, at("-xy", 1, 1, 60), "thru"),
}


def matrix():
    for s, (bt, R) in SURF.items():
        for t, tool in TOOL.items():
            for c, cf in CONFIG.items():
                x, y, r = cf(R)
                for op in OPS:
                    for z, (ext, f) in Z_TOOLS.items():
                        for fo, ft in FOLLOW.items():
                            P = Part()
                            bt(P)
                            ex(P, op, ext, f, tool(x, y, r))
                            ft(P)
                            yield f"M/{s}/{t}/{c}/{op}/{z}/{fo}", P.text()


# -- standard parts: what people really model, to textbook and standard proportions -----------

# ASME B16.5 class 150 slip-on flanges: NPS, OD, thickness, hub dia, hub height, bore, bolt circle, holes, hole dia (mm)
B16_5 = [
    ("1/2", 90, 11.2, 30, 16, 22.2, 60.5, 4, 15.7), ("1", 110, 14.2, 49, 17, 34.9, 79.2, 4, 15.7),
    ("2", 150, 17.5, 78, 24, 61.1, 120.7, 4, 19.1), ("3", 190, 22.4, 108, 29, 90.7, 152.4, 4, 19.1),
    ("4", 230, 22.4, 135, 32, 116.1, 190.5, 8, 19.1), ("6", 280, 23.9, 192, 38, 170.5, 241.3, 8, 22.4),
    ("8", 345, 27, 246, 43, 221.5, 298.5, 8, 22.4), ("12", 485, 30.2, 365, 55, 325, 431.8, 12, 25.4),
]
# ISO 4017 hex bolts: thread, across-flats, head height, chamfer
ISO4017 = [("M6", 10, 4, 0.3), ("M8", 13, 5.3, 0.4), ("M10", 16, 6.4, 0.4), ("M12", 18, 7.5, 0.5), ("M16", 24, 10, 0.6), ("M20", 30, 12.5, 0.8)]
# DIN 6885 keyways: shaft dia, key width, shaft key depth
DIN6885 = [(12, 4, 2.5), (20, 6, 3.5), (30, 8, 4), (40, 12, 5), (50, 14, 5.5), (60, 18, 7)]
# DIN 471 retaining ring grooves: shaft, groove dia, width
DIN471 = [(12, 11.5, 1.1), (20, 19, 1.3), (30, 28.6, 1.6), (40, 37.5, 1.85), (50, 47, 2.15), (60, 57, 2.15)]
# ISO 15 deep groove bearings: bore, outer dia, width (6200 series)
B6200 = [(10, 30, 9), (15, 35, 11), (20, 47, 14), (25, 52, 15), (30, 62, 16), (40, 80, 18), (50, 90, 20)]


def flange(p):
    nps, od, t, hub, hh, bore, bc, n, hd = p
    r = bore / 2
    P = Part()
    disc = ex(P, "add", t, XY, ("circle", 0, 0, od / 2), label="plate")
    hb = ex(P, "add", hh, at("xy", 0, 0, t), ("circle", 0, 0, hub / 2), label="hub")
    sel(P, "fi", f'fillet(edges(between: ({disc}, {hb}), crease: "concave"), {float(f"{min(6, hh / 3):.3f}")})')
    hole(P, bore, at("-xy", 0, 0, t + hh), "thru")
    h1 = hole(P, hd, at("-xy", bc / 2, 0, t), "thru", label="h1")
    sel(P, "pa", f"pattern(of: {h1}, layout: circ(about: {P.axis((0, 0, 0), Z)}, n: count({n})))")
    sel(P, "ch", f'chamfer(edges(of: {disc}, shape: "circle", r: {num(od / 2)}, extreme: Z), 1.5)')
    sel(P, "ch", f'chamfer(edges(shape: "circle", r: {num(r)}, extreme: Z), 1)')
    return P.text()


def hexagon(r):
    return [(float(f"{r * math.cos(i * math.pi / 3):.9f}"), float(f"{r * math.sin(i * math.pi / 3):.9f}")) for i in range(6)]


def hexbolt(p, length=30.0):
    size, af, k, c = p
    d = float(size[1:])
    r = af / math.sqrt(3)
    P = Part()
    head = ex(P, "add", k, XY, ("poly", hexagon(r)), label="head")
    shank = ex(P, "add", length, at("-xy"), ("circle", 0, 0, d / 2), label="shank")
    sel(P, "ch", f'chamfer(edges(of: {shank}, shape: "circle", extreme: -Z), {num(c * 2)})')
    cone = [(0, -(length + 1)), (float(f"{r * 0.95:.6f}"), -(length + 1)), (float(f"{r * 1.2:.6f}"), k - 0.8), (float(f"{af / 2:.6f}"), k), (0, k)]
    revolve(P, "keep", at("xz"), loops(("poly", cone)), label="keep")
    sel(P, "fi", f'fillet(edges(between: ({head}, {shank}), crease: "concave"), {num(c)})')
    return P.text()


def threadedbolt(p):
    size, af, k, c = p
    d = float(size[1:])
    r = af / math.sqrt(3)
    P = Part()
    ex(P, "add", k, XY, ("poly", hexagon(r)), label="head")
    shank = ex(P, "add", 25, at("-xy"), ("circle", 0, 0, d / 2), label="shank")
    sel(P, "t", f'thread(face: faces(of: {shank}, shape: "cyl"), spec: "{size}", length: 18, chamfer: true)')
    return P.text()


def nut(p):
    size, af, k, c = p
    d = float(size[1:])
    r = af / math.sqrt(3)
    h = float(f"{d * 0.8:.3f}")
    P = Part()
    ex(P, "add", h, XY, ("poly", hexagon(r)), label="h")
    b = hole(P, float(f"{d * 0.85:.3f}"), at("-xy", 0, 0, h), "thru", label="b")
    sel(P, "ch", f'chamfer(edges(of: {b}, shape: "circle"), {num(c)})')
    f6 = lambda v: float(f"{v:.6f}")
    cone = [(0, -1), (f6(r * 0.97), -1), (f6(r * 1.2), c * 2), (f6(r * 1.2), f6(d * 0.8 - c * 2)), (f6(r * 0.97), f6(d * 0.8 + 1)), (0, f6(d * 0.8 + 1))]
    revolve(P, "keep", at("xz"), loops(("poly", cone)))
    return P.text()


def shaft(k, g):
    d, kw, kd = k
    _, gd, gw = g if g[0] == d else next(x for x in DIN471 if x[0] == d)
    P = Part()
    main = ex(P, "add", d * 4, XY, ("circle", 0, 0, d / 2), label="main")
    step = ex(P, "add", d, at("xy", 0, 0, d * 4), ("circle", 0, 0, float(f"{d / 2 * 0.8:.4f}")), label="step")
    sel(P, "fi", f'fillet(edges(between: ({main}, {step}), crease: "concave"), {float(f"{max(0.5, d / 30):.3f}")})')
    ex(P, "cut", d, at("xz"), ("slot", -kw / 2, d * 1.5, kw / 2, d * 1.5, d / 2), label="kw")
    revolve(P, "cut", at("xz"), loops(("poly", [(gd / 2, d), (d, d), (d, d + gw), (gd / 2, d + gw)])), label="groove")
    sel(P, "ch", f'chamfer(edges(of: {main}, shape: "circle", extreme: -Z), {float(f"{d / 20:.3f}")})')
    sel(P, "ch", f'chamfer(edges(of: {step}, shape: "circle", extreme: Z), {float(f"{d / 25:.3f}")})')
    hole(P, float(f"{d / 6:.3f}"), XY, d / 2, extra=", point: 60deg")
    return P.text()


def pillow(b):
    bore, od, w = b
    h = float(f"{od * 0.75:.4f}")
    wid = od * 2.4
    t = max(6, od * 0.2)
    f4 = lambda v: float(f"{v:.4f}")
    P = Part()
    outline = [(-wid / 2, 0), (wid / 2, 0), (wid / 2, t), (f4(od * 0.55), t), (f4(od * 0.55), h), (f4(-od * 0.55), h), (f4(-od * 0.55), t), (-wid / 2, t)]
    base = ex(P, "add", w, at("xz", 0, w / 2, 0), ("poly", outline), label="base")
    top = ex(P, "add", w, at("xz", 0, w / 2, 0), ("circle", 0, h, f4(od * 0.55 + 4)), label="top")
    b = hole(P, od, at("xz", 0, w / 2, h), "thru", label="b")
    sel(P, "fi", f'fillet(edges(between: ({base}, {top}), crease: "concave"), {float(f"{t / 3:.3f}")})')
    sel(P, "fi", f'fillet(edges(of: {base}, crease: "concave", along: Y), 2)')
    m = hole(P, float(f"{max(6, od / 6):.2f}"), at("-xy", wid / 2 - t, 0, t), "thru", label="mount")
    sel(P, "pa", f"pattern(of: {m}, layout: mirror(on: yz))")
    sel(P, "ch", f'chamfer(edges(of: {b}, shape: "circle"), 1)')
    return P.text()


def bracket(l, h, t, r):
    P = Part()
    a = ex(P, "add", ("sym", 40), at("xz"), ("poly", [(0, 0), (l, 0), (l, t), (t, t), (t, h), (0, h)]), label="a")
    sk = P.last_sketch
    sel(P, "fi", f'fillet(edges(crease: "concave"), {num(r)})')
    # the outside corner of the L: where the left side meets the bottom
    sel(P, "fi", f"fillet(edges(between: ({a}.side({sk}.o[5]), {a}.side({sk}.o[0]))), {num(r + t)})")
    g = ex(P, "add", ("sym", t), at("xz"), ("poly", [(t, t), (float(f"{l * 0.7:.4f}"), t), (t, float(f"{h * 0.7:.4f}"))]), label="g")
    hole(P, 6, at("-xy", float(f"{l * 0.75:.4f}"), 12, t), "thru")
    hole(P, 6, at("-xy", float(f"{l * 0.75:.4f}"), -12, t), "thru")
    sel(P, "ch", f'chamfer(edges(of: {g}, crease: "convex"), 0.5)')
    return P.text()


def housing(l, w, h, t, r, draft):
    P = Part()
    b = ex(P, "add", h, XY, ("rrect", -l / 2, -w / 2, l, w, r), label="b")
    walls = " + ".join(f"faces(of: {b}, normal: {d})" for d in ("X", "-X", "Y", "-Y")) + f' + faces(of: {b}, shape: "cyl")'
    sel(P, "dr", f"draft({walls}, angle: {num(draft)}deg, neutral: xy)")
    sel(P, "fi", f"fillet(edges(extreme: Z), {num(r / 2)})")
    sel(P, "sh", f"shell({num(t)}, open: faces(extreme: -Z))")
    x, y = l / 2 - r - 6, w / 2 - r - 6
    boss = ex(P, "add", h - t, at("xy", x, y, 0), ("circle", 0, 0, 4), label="boss")
    h1 = hole(P, 3, at("-xy", x, y, h - t), h - t - 2, label="hole1")
    sel(P, "pa", f"pattern(of: [{boss}, {h1}], layout: mirror(on: yz))")
    return P.text()


def lin(d, n, step):
    """A row along a world direction: a negative step runs the other way."""
    names = {X: ("X", "-X"), Y: ("Y", "-Y")}[d]
    return names[step < 0], n, num(abs(step))


def enclosure(l, w, h, t):
    P = Part()
    ex(P, "add", h, XY, ("rect", -l / 2, -w / 2, l, w), label="b")
    sel(P, "fi", f"fillet(edges(along: Z), {num(t * 2)})")
    sel(P, "sh", f"shell({num(t)}, open: faces(extreme: Z))")
    ex(P, "cut", t, at("xy", 0, 0, h - t), ("rect", -l / 2 + t / 2, -w / 2 + t / 2, l - t, w - t), label="lip")
    bos = ex(P, "add", h - t * 3, at("xy", l / 2 - t * 4, w / 2 - t * 4, t), ("circle", 0, 0, t * 1.5), label="bos")
    a1, n1, s1 = lin(X, 2, -(l - t * 8))
    a2, n2, s2 = lin(Y, 2, -(w - t * 8))
    sel(P, "pa", f"pattern(of: {bos}, layout: lin(along: {a1}, n: count({n1}), step: {s1}, along2: {a2}, n2: count({n2}), step2: {s2}))")
    vent = ex(P, "cut", "thru", at("xz", -l / 4, 0, h / 2), ("rect", 0, 0, l / 20, h / 4), label="vent")
    sel(P, "pa", f"pattern(of: {vent}, layout: lin(along: X, n: count(5), step: {num(l / 10)}))")
    return P.text()


def casting(l, w, h, r):
    P = Part()
    p = ex(P, "add", h / 4, XY, ("rect", -l / 2, -w / 2, l, w), label="pad")
    c = ex(P, "add", h, at("xy", 0, 0, h / 4), ("circle", 0, 0, w / 3), label="col")
    r1 = ex(P, "add", ("sym", h / 8), at("xz"), ("poly", [(w / 3 - 1, h / 4 - 1), (l / 2 - 2, h / 4 - 1), (w / 3 - 1, h)]), label="r1")
    r2 = sel(P, "pa", f"pattern(of: {r1}, layout: mirror(on: yz))")
    sel(P, "dr", f'draft(faces(of: {c}, shape: "cyl"), angle: 2deg, neutral: offset(xy, {num(h / 4)}))')
    sel(P, "fi", f'fillet(edges(between: ({p}, {c}), crease: "concave"), {num(r)})')
    sel(P, "fi", f'fillet(edges(of: [{r1}, {r2}], crease: "convex"), {num(r / 2)})')
    hole(P, w / 3, at("-xy", 0, 0, h * 1.25), "thru")
    h2 = hole(P, 5, at("-xy", l / 2 - 6, w / 2 - 6, h / 4), "thru", label="hole2")
    a1, n1, s1 = lin(X, 2, -(l - 12))
    a2, n2, s2 = lin(Y, 2, -(w - 12))
    sel(P, "pa", f"pattern(of: {h2}, layout: lin(along: {a1}, n: count({n1}), step: {s1}, along2: {a2}, n2: count({n2}), step2: {s2}))")
    return P.text()


def gear(teeth, module, face, bore):
    """A spur gear: involute flanks by short lines, root and tip arcs."""
    rp = module * teeth / 2
    rb = rp * math.cos(math.radians(20))
    ra = rp + module
    rf = rp - 1.25 * module
    inv = lambda r: math.sqrt(max((r / rb) ** 2 - 1, 0)) - math.acos(min(1, rb / r))
    half = math.pi / (2 * teeth) + inv(rp)
    n = 8
    rs = [max(rf, rb) + (ra - max(rf, rb)) * k / n for k in range(n + 1)]
    right = [(r, -half + inv(r)) for r in rs]
    left = [(r, half - inv(r)) for r in reversed(rs)]
    pts = [(rf, -half)] + right + left + [(rf, half)]
    out = []
    for t in range(teeth):
        a0 = 2 * math.pi * t / teeth
        out += [(float(f"{r * math.cos(a0 + a):.6f}"), float(f"{r * math.sin(a0 + a):.6f}")) for r, a in pts]
    P = Part()
    ex(P, "add", face, XY, ("poly", out), label="g")
    b = hole(P, bore, at("-xy", 0, 0, face), "thru", label="b")
    ex(P, "cut", "thru", at("-xy", 0, 0, face * 2), ("rect", -bore / 8, 0, bore / 4, bore / 2 + bore / 8), label="kw")
    sel(P, "ch", f'chamfer(edges(of: {b}, shape: "circle"), {num(module / 4)})')
    return P.text()


def pulley(d, w, groove, bore):
    r = d / 2
    f4 = lambda v: float(f"{v:.4f}")
    P = Part()
    p = revolve(P, "add", at("xz"), loops(("poly", [(bore / 2, 0), (r, 0), (r, w), (bore / 2, w)])), label="rim")
    gp = [(r - groove, w / 2 - 1.5), (r + 1, f4(w / 2 - 1.5 - groove * 0.6)), (r + 1, f4(w / 2 + 1.5 + groove * 0.6)), (r - groove, w / 2 + 1.5)]
    g = revolve(P, "cut", at("xz"), loops(("poly", gp)), label="g")
    sel(P, "fi", f'fillet(edges(of: {g}, crease: "concave"), 0.5)')
    sel(P, "ch", f'chamfer(edges(of: {p}, shape: "circle", r: {num(r)}), 0.5)')
    return P.text()


def manifold_block(l, w, h, ports):
    P = Part()
    b = ex(P, "add", h, XY, ("rect", 0, 0, l, w), label="b")
    for i in range(ports):
        x = l * (i + 1) / (ports + 1)
        hole(P, w / 5, at("xz", x, 0, h / 2), w * 0.75)
        hole(P, w / 8, at("-xy", x, w / 2, h), h * 0.6)
    hole(P, h / 5, at("yz", 0, w / 2, h / 2), l - 2)
    sel(P, "ch", f"chamfer(edges(of: {b}), 0.5)")
    return P.text()


def standard():
    for p in B16_5:
        yield f"flange/{p[0]}", flange(p)
    for p in ISO4017:
        yield f"hexbolt/{p[0]}", hexbolt(p)
        yield f"threaded/{p[0]}", threadedbolt(p)
        yield f"nut/{p[0]}", nut(p)
    for k, g in zip(DIN6885, DIN471):
        yield f"shaft/{k[0]}", shaft(k, g)
    for b in B6200:
        yield f"pillow/{b[0]}", pillow(b)
    for l, h, t, r in ((30, 30, 3, 2), (50, 40, 5, 3), (80, 60, 6, 4), (120, 100, 10, 6), (60, 60, 2, 1)):
        yield f"bracket/{l}x{h}x{t}", bracket(l, h, t, r)
    for l, w, h, t, r, dr in ((60, 40, 25, 2, 4, 2), (100, 70, 30, 2.5, 6, 3), (150, 100, 50, 3, 8, 1), (40, 40, 15, 1.5, 3, 5)):
        yield f"housing/{l}x{w}x{h}", housing(l, w, h, t, r, dr)
        yield f"enclosure/{l}x{w}x{h}", enclosure(l, w, h, t)
    for l, w, h, r in ((80, 50, 40, 4), (120, 80, 60, 6), (60, 60, 30, 3), (200, 120, 90, 10)):
        yield f"casting/{l}x{w}x{h}", casting(l, w, h, r)
    for z, m, fw, b in ((12, 2, 10, 6), (17, 2, 12, 8), (24, 1.5, 10, 8), (32, 1, 8, 8), (48, 1, 8, 12), (20, 3, 20, 12)):
        yield f"gear/z{z}m{m}", gear(z, m, fw, b)
    for d, w, g, b in ((50, 16, 6, 12), (80, 18, 8, 15), (120, 22, 10, 20), (200, 30, 12, 30)):
        yield f"pulley/{d}", pulley(d, w, g, b)
    for l, w, h, n in ((60, 30, 30, 2), (100, 40, 40, 3), (150, 50, 40, 5)):
        yield f"manifold/{l}x{w}x{h}", manifold_block(l, w, h, n)


# -- mutation of real parts: reorder, one op changed, selections swapped ------------------------

# a selection's words, each swapped for another of its kind
SELS = (
    ("crease", ('"convex"', '"concave"', '"smooth"')),
    ("shape", ('"line"', '"circle"', '"plane"', '"cyl"', '"cone"', '"blend"')),
    ("extreme", ("X", "-X", "Y", "-Y", "Z", "-Z")),
    ("normal", ("X", "-X", "Y", "-Y", "Z", "-Z")),
    ("along", ("X", "Y", "Z")),
)
SEL = re.compile(r"\b(" + "|".join(k for k, _ in SELS) + r"): (\"\w+\"|-?[XYZ])")
BIND = re.compile(r"^(\s*)([A-Za-z_]\w*) = ")


def feature_lines(src):
    """A program's statements: a line, or a block (a sketch, a loop) or call running over lines."""
    out, cur, depth = [], [], 0
    for l in src.splitlines():
        if not cur and (not l.strip() or l.lstrip().startswith("#")):
            continue
        cur.append(l)
        body = re.sub(r'"[^"]*"', "", l.split("#")[0])
        depth += body.count("{") + body.count("(") + body.count("[") - body.count("}") - body.count(")") - body.count("]")
        if depth <= 0:
            out.append("\n".join(cur))
            cur, depth = [], 0
    if cur:
        out.append("\n".join(cur))
    return out


def fresh(stmt, r):
    """The statement bound to a name no other has (a copy of it, or one spliced in). A param keeps its
    name: a second one would be ParamTwice, so a param is copied as a binding of its value."""
    m = PARAM.match(stmt.strip())
    if m:
        return f"{m.group(2).replace('.', '_').replace('[', '_').replace(']', '')}_{r.randrange(10**6)} = {m.group(3)}"
    m = BIND.match(stmt)
    if not m:
        m = re.match(r"^(sketch )([A-Za-z_]\w*)", stmt)
        if not m:
            return stmt
    return stmt[: m.start(2)] + f"{m.group(2)}_{r.randrange(10**6)}" + stmt[m.end(2) :]


def mutate_once(bs, r, pool):
    """One edit to a part's statements: what a person's next try might be."""
    bs = list(bs)
    if not bs:
        return bs, "none"
    k = r.randrange(8)
    i = r.randrange(len(bs))
    if k == 0 and len(bs) > 2:  # move a statement
        x = bs.pop(i)
        bs.insert(r.randrange(1, len(bs) + 1), x)
        return bs, "move"
    if k == 1 and len(bs) > 2:  # drop a statement
        del bs[i]
        return bs, "drop"
    if k == 2:  # change one operation
        for _ in range(8):
            i = r.randrange(len(bs))
            m = re.search(r"= (extrude|cut|intersect)\(", bs[i])
            if m:
                bs[i] = bs[i][: m.start(1)] + r.choice(("extrude", "cut", "intersect")) + bs[i][m.end(1) :]
                return bs, "op"
            m = re.search(r"\b(revolve|sweep|coil|loft|ball)\((.*?)(, (cut|keep|new): true)?\)\s*$", bs[i])
            if m:
                flag = r.choice(("", ", cut: true", ", keep: true", ", new: true"))
                bs[i] = bs[i][: m.start(3) if m.group(3) else m.end(2)] + flag + ")" + bs[i][m.end() :]
                return bs, "op"
    if k == 3:  # swap a selection's word
        for _ in range(8):
            i = r.randrange(len(bs))
            ws = list(SEL.finditer(bs[i]))
            if ws:
                m = r.choice(ws)
                kinds = dict(SELS)[m.group(1)]
                bs[i] = bs[i][: m.start(2)] + r.choice(kinds) + bs[i][m.end(2) :]
                return bs, "sel"
    if k == 4 and pool:  # a statement from another real part
        src = r.choice(pool)
        if src:
            bs.insert(r.randrange(1, len(bs) + 1), fresh(r.choice(src), r))
            return bs, "splice"
    if k == 5:  # a dimension moved
        for _ in range(8):
            i = r.randrange(len(bs))
            ms = list(re.finditer(r"(?<![\w.@=])\d+\.?\d*(?![\w.(])", bs[i]))
            if ms and "count(" not in bs[i]:
                m = r.choice(ms)
                v = float(m.group(0)) * r.choice((0.5, 0.8, 0.99, 1.01, 1.25, 2))
                bs[i] = bs[i][: m.start()] + num(float(f"{v:.6g}")) + bs[i][m.end() :]
                return bs, "dim"
    if k == 6:  # a finishing op on what is there
        f = r.choice(("fillet(edges(extreme: Z), 1)", 'chamfer(edges(crease: "concave"), 0.5)', 'fillet(edges(shape: "circle"), 0.5)', "shell(1, open: faces(extreme: Z))", 'chamfer(edges(shape: "circle"), 0.5)', "draft(faces(normal: X), angle: 2deg, neutral: xy)"))
        bs.insert(r.randrange(1, len(bs) + 1), f"fin_{r.randrange(10**6)} = {f}")
        return bs, "finish"
    # duplicate a statement under a name of its own
    bs.insert(i + 1, fresh(bs[i], r))
    return bs, "dup"


def mutated(parts, seed=0, n=None):
    """Random single and double mutations of the given parts, forever or n of them."""
    r = random.Random(seed)
    parts = [(k, feature_lines(v)) for k, v in parts]
    pool = [b for _, b in parts]
    i = 0
    while n is None or i < n:
        pn, bs = r.choice(parts)
        t, what = mutate_once(bs, r, pool)
        if r.random() < 0.3:
            t, w2 = mutate_once(t, r, pool)
            what += "+" + w2
        yield f"mut{i}/{pn}/{what}", "\n".join(t) + "\n"
        i += 1


# -- real shapes with the finishing a person adds next ------------------------------------------

FINISH = (
    'fillet(edges(crease: "concave"), {s})',
    'fillet(edges(crease: "convex"), {s})',
    'fillet(edges(shape: "circle"), {s})',
    "fillet(edges(along: Z), {s})",
    "fillet(edges(extreme: Z), {s})",
    'fillet(edges(shape: "line", longer: {b}), {s})',
    "fillet(edges(extreme: Z), vary({s}, {S}))",
    'chamfer(edges(crease: "convex"), {s})',
    'chamfer(edges(shape: "circle"), {s})',
    'chamfer(edges(crease: "concave"), {s})',
    "chamfer(edges(extreme: Z), {s}, d2: {S}, face: faces(extreme: Z))",
    "shell({s}, open: faces(extreme: Z))",
    "shell({s}, open: faces(extreme: -Z))",
    "shell({s}, open: faces(extreme: Z), out: true)",
    "draft(faces(normal: X), angle: 2deg, neutral: xy)",
    "draft(faces(normal: Y) + faces(normal: -Y), angle: 3deg, neutral: xy)",
    "offset_faces(faces(extreme: Z), {s})",
    "offset_faces(faces(normal: X), -{s})",
    "thicken(faces(extreme: Z), {s})",
    "move_faces(faces(extreme: Z), along: Z, d: {s})",
    'delete_faces(faces(shape: "cyl"))',
    'delete_faces(faces(shape: "blend"))',
    "facefillet(faces(normal: X), faces(extreme: Z), {s})",
    'split(by: xy, keep: "plus")',
    "pattern(whole: true, layout: mirror(on: yz))",
)


def augmented(parts, seed=0):
    """A part as built, then one or two finishing steps sized to it. Only parts with one body:
    a selection reads the whole part, and that may not be the body a person means."""
    r = random.Random(seed)
    for n, src in parts:
        ls = [l for l in src.splitlines() if l.strip() and not l.lstrip().startswith("#")]
        size = 1.0
        nums = [float(x) for l in ls for x in re.findall(r"(?<![\w.])\d+\.?\d*(?![\w.(])", l.split("#")[0])][:200]
        if nums:
            nums.sort()
            size = max(nums[len(nums) // 2], 1.0)
        s = float(f"{size * r.choice((0.02, 0.05, 0.1)):.3g}")
        fill = {"s": num(s), "S": num(float(f"{s * 2:.3g}")), "b": num(float(f"{size:.3g}"))}
        f1 = r.choice(FINISH).format(**fill)
        k1 = f1.split("(")[0]
        yield f"aug/{n}/{k1}", src.rstrip("\n") + f"\nfin_1 = {f1}\n"
        f2 = r.choice(FINISH).format(**fill)
        yield f"aug2/{n}/{k1}+{f2.split('(')[0]}", src.rstrip("\n") + f"\nfin_1 = {f1}\nfin_2 = {f2}\n"


# -- noise: real data carries 1e-7 of float noise, and a part must not care --------------------

NOISE_NUM = re.compile(r"(?<![\w.@])-?\d+\.?\d*(?![\w.(])")


def perturb(src, seed, size=3e-7):
    """Every coordinate moved by up to `size` (a sketch's rows, a length param): the part a sloppier
    float would have made."""
    r = random.Random(seed)
    out = []
    depth = 0

    def bump(n):
        return num(float(n.group(0)) + r.uniform(-size, size))

    for l in src.splitlines():
        code, _, note = l.partition("#")
        in_sketch = depth > 0
        if l.startswith("sketch "):
            depth = 0
        depth += code.count("{") - code.count("}")
        m = PARAM.match(l.strip())
        if m and not re.search(r"deg\b|count\(|ratio\(|[\[\"]", m.group(3)):
            head = l[: l.index("=") + 1]
            out.append(head + NOISE_NUM.sub(bump, code[len(head) :]) + (f"#{note}" if note else ""))
        elif in_sketch and "=" in code:
            head = code[: code.index("=") + 1]
            out.append(head + NOISE_NUM.sub(bump, code[len(head) :]) + (f"#{note}" if note else ""))
        else:
            out.append(l)
    return "\n".join(out) + "\n"
