"""Part generators. Each yields (name, source). Nothing here reads downloaded data."""
import math
import os
import random
import re

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
BENCH = os.path.join(ROOT, "tools/bench-parts")


def bench():
    for f in sorted(os.listdir(BENCH)):
        if f.endswith(".fc"):
            yield f[:-3], open(os.path.join(BENCH, f)).read()


# -- parameter sweeps of real parts: the edits that break CAD ---------------------------------

SCALES = (0.25, 0.5, 0.75, 0.9, 0.99, 1.01, 1.1, 1.25, 1.5, 2.0, 3.0)
NUDGES = (1e-3, 1e-6, 1e-8)
FNS = {k: getattr(math, k) for k in ("sqrt", "sin", "cos", "tan", "asin", "acos", "atan", "atan2")}
FNS.update(min=min, max=max, abs=abs, pi=math.pi)
PARAM = re.compile(r"^(\w+)\s*=\s*([^#]+?)(?:\s+(mm|deg))?\s*(?:#.*)?$")


def params(src):
    """Each `NAME = EXPR` line's index, name and value."""
    env, out = dict(FNS), []
    for i, l in enumerate(src.splitlines()):
        m = PARAM.match(l.strip())
        if not m:
            continue
        try:
            v = eval(m.group(2).replace("^", "**"), {"__builtins__": {}}, env)
        except Exception:
            continue
        env[m.group(1)] = v
        out.append((i, m.group(1), float(v), m.group(3) or ""))
    return out


def sweep(parts=None, literals=True):
    """Every dimension across its range, and onto values that make faces coincide or tangent:
    equal to another dimension, their sum, their difference, and a hair either side of those."""
    for pn, src in parts if parts is not None else bench():
        lines = src.splitlines()
        ps = params(src)
        vals = [p[2] for p in ps if p[2] > 0]
        for i, name, v, unit in ps:
            cand = {v * s for s in SCALES}
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
                t[i] = f"{name} = {c!r} {unit}".rstrip()
                yield f"{pn}/{name}={c:.9g}", "\n".join(t) + "\n"
        if not literals:
            continue
        lit = re.compile(r"(?<![\w.@=])\d+(?:\.\d+)?(?![\w.])")
        for i, l in enumerate(lines):
            body = l.split("#")[0]
            if PARAM.match(l.strip()) or not body.strip() or "(" in body and "fillet" not in body and "chamfer" not in body:
                continue
            for m in lit.finditer(body):
                v = float(m.group(0))
                if v == 0 or "pattern" in body:
                    continue
                for s in (0.5, 0.9, 1.1, 2.0):
                    t = list(lines)
                    t[i] = body[: m.start()] + repr(v * s) + body[m.end() :]
                    yield f"{pn}/L{i}.{m.start()}x{s}", "\n".join(t) + "\n"


# -- structured enumeration: every feature after every feature on each face and edge kind -----

W, D, H = 60.0, 40.0, 20.0
BASES = {
    "box": f"extrude {H} rect -30 -20 {W} {D}",
    "cyl": f"extrude {H} circle 0 0 20",
    "tube": f"extrude {H} circle 0 0 20 circle 0 0 12",
    "cone": "revolve xz axis y poly 0 0 20 0 10 20 0 20",
    "sphere": "ball 15 at 0 0 10",
    "torus": "revolve xz at 0 0 10 axis y circle 20 0 8",
    "lbracket": "extrude sym 40 xz poly -30 0 30 0 30 6 -24 6 -24 20 -30 20",
    "plate_hole": "extrude 6 rect -30 -20 60 40\nhole 8 -xy at 0 0 6 thru",
    "boss_plate": "extrude 6 rect -30 -20 60 40\nextrude add 14 at 0 0 6 circle 0 0 10",
    "fillet_box": f"extrude {H} rect -30 -20 {W} {D}\nfillet 3 (edges along +z)",
    "chamfer_box": f"extrude {H} rect -30 -20 {W} {D}\nchamfer 2 (edges top)",
    "shell_box": f"extrude {H} rect -30 -20 {W} {D}\nshell 2 (faces top)",
    "draft_box": f"extrude {H} rect -30 -20 {W} {D}\ndraft 5 xy (faces normal +x) (faces normal -x)",
    "hex": "extrude 20 poly 20 0 10 17.3205 -10 17.3205 -20 0 -10 -17.3205 10 -17.3205",
    "rrect": "extrude 20 rrect -30 -20 60 40 6",
    "slot": "extrude 20 loop line -20 -10 20 -10 arc 20 0 10 -90 180 line 20 10 -20 10 arc -20 0 10 90 180",
    "loft": "loft profile rect -20 -20 40 40 profile at 0 0 20 rect -10 -10 20 20",
    "sweep": "sweep -xz circle 0 0 8 path line 0 0 0 0 30 0 arc yz at 0 30 15 15 -90 90 line 0 45 15 0 45 40",
    "thread_rod": "extrude 20 circle 0 0 6\nthread M12 face f1 (faces cyl) 12",
}
# each: name, text. X Y are near the part's centre; they read the part by selector wherever they can
FEATS = {
    "fillet_top": "fillet 1 (edges top)",
    "fillet_concave": "fillet 1 (edges concave)",
    "fillet_circle": "fillet 1 (edges circle)",
    "fillet_vertical": "fillet 2 (edges along +z)",
    "fillet_var": "fillet 1..2 (edges top)",
    "chamfer_top": "chamfer 1 (edges top)",
    "chamfer_circle": "chamfer 0.5 (edges circle)",
    "chamfer_2d": "chamfer 1 2 face (faces top) (edges top)",
    "hole_thru": "hole 4 -xy at 5 3 40 thru",
    "hole_blind": "hole 4 -xy at 5 3 40 45",
    "hole_center": "hole 5 -xy at 0 0 40 thru",
    "hole_side": "hole 3 yz at -40 0 10 thru",
    "hole_csink": "hole M6 csink -xy at -6 -4 40 thru",
    "cut_circle": "extrude cut thru -xy at 0 0 40 circle 6 2 3",
    "cut_rect": "extrude cut 5 -xy at 0 0 22 rect -5 -5 10 10",
    "cut_across": "extrude cut thru xz at 0 -50 0 rect -100 15 200 10",
    "add_on_top": "extrude add 5 on (faces top) circle 0 0 3",
    "add_rect_side": "extrude add 5 yz at 30 0 10 rect -3 -3 6 6",
    "revolve_cut": "revolve cut xz at 0 0 10 axis y circle 25 0 3",
    "ball_add": "ball add 6 at 15 0 18",
    "ball_cut": "ball cut 4 at 0 0 20",
    "pattern_lin": "extrude add 3 at -15 -10 0 circle 0 0 2\npattern lastx lin 1 0 0 3 6",
    "pattern_circ": "hole 3 -xy at 12 0 40 thru\npattern lasth circ 0 0 0 0 0 1 4",
    "mirror_body": "pattern body mirror yz",
    "shell_top": "shell 1 (faces top)",
    "draft_sides": "draft 3 xy (faces normal +x)",
    "offset_top": "offset 1 (faces top)",
    "thicken": "thicken add 1 (faces top)",
    "delete_fillet": "delete (faces blend)",
    "delete_hole": "delete (faces cyl)",
    "move_top": "move (faces top) by 0 0 1",
    "replace_top": "wp: plane xy at 0 0 25\nreplace (faces top) with wp",
    "split": "split plane xz at 0 0 0 +",
    "splitface": "splitface (faces top) by plane yz at 0 0 0",
    "rib": "rib 2 xz at 0 0 0 line -20 5 20 5",
    "facefillet": "facefillet 1 (faces normal +x) to (faces top)",
    "fullround": "fullround (faces normal -y) (faces normal +x) (faces normal +y)",
    "thread_hole": "hole 8.4 -xy at 0 0 40 thru\nthread M10 -xy at 0 0 40 int 10",
    "sweep_cut": "sweep cut yz at 0 0 15 circle 0 0 1 path line 0 0 15 20 0 15",
    "loft_add": "loft add profile at 0 0 15 rect -3 -3 6 6 profile at 0 0 25 rect -2 -2 4 4",
    "coil_add": "coil add at 15 0 0 circle 3 0 1 pitch 5 turns 2",
}


def enumerate_pairs(bases=None, feats=None):
    """base + F1 + F2 for every pair: the feature-after-feature matrix."""
    bases = BASES if bases is None else bases
    feats = FEATS if feats is None else feats

    def fix(text, k):
        text = text.replace("lastx", f"x{k}").replace("lasth", f"x{k}")
        ls = text.split("\n")
        # the labelled line a following pattern copies
        if "x" + str(k) in text:
            ls[0] = f"x{k}: " + ls[0]
        return "\n".join(ls)

    for b, bt in bases.items():
        for f1, t1 in feats.items():
            yield f"{b}+{f1}", f"{bt}\n{fix(t1, 1)}\n"
            for f2, t2 in feats.items():
                yield f"{b}+{f1}+{f2}", f"{bt}\n{fix(t1, 1)}\n{fix(t2, 2)}\n"


# the surface-pair x configuration matrix: a tool placed against each base surface
SURF = {
    "plane": ("extrude 20 rect -30 -20 60 40", 20.0),
    "cylinder": ("extrude 20 circle 0 0 20", 20.0),
    "cone": ("revolve xz axis y poly 0 0 20 0 10 20 0 20", 20.0),
    "sphere": ("ball 20 at 0 0 0", 20.0),
    "torus": ("revolve xz axis y circle 20 0 8", 8.0),
    "fillet_cyl": ("extrude 20 rect -30 -20 60 40\nfillet 5 (edges along +z)", 20.0),
    "fillet_torus": ("extrude 20 circle 0 0 20\nfillet 5 (edges top)", 20.0),
    "chamfer": ("extrude 20 circle 0 0 20\nchamfer 5 (edges top)", 20.0),
    "draft": ("extrude 20 rect -30 -20 60 40\ndraft 7 xy (faces normal +x) (faces normal -x)", 20.0),
}
TOOL = {
    "cyl_z": lambda x, y, r, z: f"circle {x} {y} {r}",
    "box_z": lambda x, y, r, z: f"rect {x - r} {y - r} {2 * r} {2 * r}",
    "hex_z": lambda x, y, r, z: "poly " + " ".join(f"{x + r * math.cos(a * math.pi / 3):.9f} {y + r * math.sin(a * math.pi / 3):.9f}" for a in range(6)),
    "slot_z": lambda x, y, r, z: f"loop line {x - r} {y - r} {x + r} {y - r} arc {x + r} {y} {r} -90 180 line {x + r} {y + r} {x - r} {y + r} arc {x - r} {y} {r} 90 180",
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
Z_TOOLS = {"through": "thru -xy at 0 0 200", "blind": "12 -xy at 0 0 20", "flush": "20", "above": "10 at 0 0 20"}
FOLLOW = {
    "": "",
    "fillet": "\nfillet 0.5 (edges top)",
    "chamfer": "\nchamfer 0.5 (edges concave)",
    "hole": "\nhole 2 -xy at 1 1 60 thru",
}


def matrix():
    for s, (bt, R) in SURF.items():
        for t, tool in TOOL.items():
            for c, cf in CONFIG.items():
                x, y, r = cf(R)
                for op in OPS:
                    for z, zt in Z_TOOLS.items():
                        for fo, ft in FOLLOW.items():
                            yield f"M/{s}/{t}/{c}/{op}/{z}/{fo}", f"{bt}\nextrude {op} {zt} {tool(x, y, r, 0)}{ft}\n"


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
    return f"""od = {od}
t = {t}
disc: extrude t circle 0 0 od/2
hub: extrude add {hh} at 0 0 t circle 0 0 {hub / 2}
fillet {min(6, hh / 3):.3f} (edges between disc hub concave)
hole {bore} -xy at 0 0 {t + hh} thru
h1: hole {hd} -xy at {bc / 2} 0 t thru
pattern h1 circ 0 0 0 0 0 1 {n}
chamfer 1.5 (edges of disc circle r={od / 2} top)
chamfer 1 (edges circle r={r} top)
"""


def hexbolt(p, length=30.0):
    size, af, k, c = p
    d = float(size[1:])
    r = af / math.sqrt(3)
    pts = " ".join(f"{r * math.cos(i * math.pi / 3):.9f} {r * math.sin(i * math.pi / 3):.9f}" for i in range(6))
    return f"""head: extrude {k} poly {pts}
shank: extrude add {length} -xy at 0 0 0 circle 0 0 {d / 2}
chamfer {c * 2} (edges of shank circle bottom)
keep: revolve keep xz axis y poly 0 -{length + 1} {r * 0.95:.6f} -{length + 1} {r * 1.2:.6f} {k - 0.8} {af / 2:.6f} {k} 0 {k}
fillet {c} (edges between head shank concave)
"""


def threadedbolt(p):
    size, af, k, c = p
    d = float(size[1:])
    r = af / math.sqrt(3)
    pts = " ".join(f"{r * math.cos(i * math.pi / 3):.9f} {r * math.sin(i * math.pi / 3):.9f}" for i in range(6))
    return f"""head: extrude {k} poly {pts}
shank: extrude add 25 -xy at 0 0 0 circle 0 0 {d / 2}
thread {size} face shank (faces of shank cyl) 18 chamfer
"""


def nut(p):
    size, af, k, c = p
    d = float(size[1:])
    r = af / math.sqrt(3)
    pts = " ".join(f"{r * math.cos(i * math.pi / 3):.9f} {r * math.sin(i * math.pi / 3):.9f}" for i in range(6))
    return f"""h: extrude {d * 0.8:.3f} poly {pts}
b: hole {d * 0.85:.3f} -xy at 0 0 {d * 0.8:.3f} thru
chamfer {c} (edges of b circle)
revolve keep xz axis y poly 0 -1 {r * 0.97:.6f} -1 {r * 1.2:.6f} {c * 2} {r * 1.2:.6f} {d * 0.8 - c * 2:.6f} {r * 0.97:.6f} {d * 0.8 + 1:.6f} 0 {d * 0.8 + 1:.6f}
"""


def shaft(k, g):
    d, kw, kd = k
    _, gd, gw = g if g[0] == d else next(x for x in DIN471 if x[0] == d)
    return f"""main: extrude {d * 4} circle 0 0 {d / 2}
step: extrude add {d} at 0 0 {d * 4} circle 0 0 {d / 2 * 0.8:.4f}
fillet {max(0.5, d / 30):.3f} (edges between main step concave)
kw: extrude cut {d} xz at 0 0 0 loop line {-kw / 2} {d} {kw / 2} {d} arc {kw / 2} {d * 1.5} {d / 2} -90 180 line {kw / 2} {d * 2} {-kw / 2} {d * 2} arc {-kw / 2} {d * 1.5} {d / 2} 90 180
groove: revolve cut xz axis y poly {gd / 2} {d} {d} {d} {d} {d + gw} {gd / 2} {d + gw}
chamfer {d / 20:.3f} (edges of main circle bottom)
chamfer {d / 25:.3f} (edges of step circle top)
hole {d / 6:.3f} xy at 0 0 0 {d / 2} pt 60
"""


def pillow(b):
    bore, od, w = b
    h = od * 0.75
    wid = od * 2.4
    t = max(6, od * 0.2)
    return f"""base: extrude {w} xz at 0 {w / 2} 0 poly {-wid / 2} 0 {wid / 2} 0 {wid / 2} {t} {od * 0.55:.4f} {t} {od * 0.55:.4f} {h:.4f} {-od * 0.55:.4f} {h:.4f} {-od * 0.55:.4f} {t} {-wid / 2} {t}
top: extrude add {w} xz at 0 {w / 2} 0 circle 0 {h:.4f} {od * 0.55 + 4:.4f}
b: hole {od} xz at 0 {w / 2} {h:.4f} thru
fillet {t / 3:.3f} (edges between base top concave)
fillet 2 (edges of base concave along +y)
m: hole {max(6, od / 6):.2f} -xy at {wid / 2 - t} 0 {t} thru
pattern m mirror yz
chamfer 1 (edges of b circle)
"""


def bracket(l, h, t, r):
    return f"""a: extrude sym 40 xz poly 0 0 {l} 0 {l} {t} {t} {t} {t} {h} 0 {h}
fillet {r} (edges concave)
fillet {r + t} (edges of a along +y left bottom)
g: extrude add sym {t} xz poly {t} {t} {l * 0.7:.4f} {t} {t} {h * 0.7:.4f}
hole 6 -xy at {l * 0.75:.4f} 12 {t} thru
hole 6 -xy at {l * 0.75:.4f} -12 {t} thru
chamfer 0.5 (edges of g convex)
"""


def housing(l, w, h, t, r, draft):
    return f"""b: extrude {h} rrect {-l / 2} {-w / 2} {l} {w} {r}
draft {draft} xy (faces of b normal +x) (faces of b normal -x) (faces of b normal +y) (faces of b normal -y) (faces of b cyl)
fillet {r / 2} (edges top)
shell {t} (faces bottom)
boss: extrude add {h - t} xy at {l / 2 - r - 6} {w / 2 - r - 6} 0 circle 0 0 4
hole 3 -xy at {l / 2 - r - 6} {w / 2 - r - 6} {h - t} {h - t - 2}
pattern boss hole1 mirror yz
"""


def enclosure(l, w, h, t):
    return f"""b: extrude {h} rect {-l / 2} {-w / 2} {l} {w}
fillet {t * 2} (edges along +z)
shell {t} (faces top)
lip: extrude cut {t} at 0 0 {h - t} rect {-l / 2 + t / 2} {-w / 2 + t / 2} {l - t} {w - t}
bos: extrude add {h - t * 3} at {l / 2 - t * 4} {w / 2 - t * 4} {t} circle 0 0 {t * 1.5}
pattern bos lin 1 0 0 2 {-(l - t * 8)} 0 1 0 2 {-(w - t * 8)}
vent: extrude cut thru xz at {-l / 4} 0 {h / 2} rect 0 0 {l / 20} {h / 4}
pattern vent lin 1 0 0 5 {l / 10}
"""


def casting(l, w, h, r):
    return f"""p: extrude {h / 4} rect {-l / 2} {-w / 2} {l} {w}
c: extrude add {h} at 0 0 {h / 4} circle 0 0 {w / 3}
r1: extrude add sym {h / 8} xz poly {w / 3 - 1} {h / 4 - 1} {l / 2 - 2} {h / 4 - 1} {w / 3 - 1} {h}
pattern r1 mirror yz
draft 2 xy at 0 0 {h / 4} (faces of c cyl)
fillet {r} (edges between p c concave)
fillet {r / 2} (edges of r1 convex)
hole {w / 3} -xy at 0 0 {h * 1.25} thru
hole 5 -xy at {l / 2 - 6} {w / 2 - 6} {h / 4} thru
pattern hole2 lin 1 0 0 2 {-(l - 12)} 0 1 0 2 {-(w - 12)}
"""


def gear(teeth, module, face, bore):
    """A spur gear: involute flanks by short lines, root and tip arcs."""
    rp = module * teeth / 2
    rb = rp * math.cos(math.radians(20))
    ra = rp + module
    rf = rp - 1.25 * module
    inv = lambda r: math.sqrt(max((r / rb) ** 2 - 1, 0)) - math.acos(min(1, rb / r))
    half = math.pi / (2 * teeth) + inv(rp)
    pts = []
    for i in range(1):
        n = 8
        rs = [max(rf, rb) + (ra - max(rf, rb)) * k / n for k in range(n + 1)]
        right = [(r, -half + inv(r)) for r in rs]
        left = [(r, half - inv(r)) for r in reversed(rs)]
        pts = [(rf, -half)] + right + left + [(rf, half)]
    out = []
    for t in range(teeth):
        a0 = 2 * math.pi * t / teeth
        out += [(r * math.cos(a0 + a), r * math.sin(a0 + a)) for r, a in pts]
    poly = " ".join(f"{x:.6f} {y:.6f}" for x, y in out)
    return f"""g: extrude {face} poly {poly}
b: hole {bore} -xy at 0 0 {face} thru
kw: extrude cut thru -xy at 0 0 {face * 2} rect {-bore / 8} 0 {bore / 4} {bore / 2 + bore / 8}
chamfer {module / 4} (edges of b circle)
"""


def pulley(d, w, groove, bore):
    r = d / 2
    return f"""p: revolve xz axis y poly {bore / 2} 0 {r} 0 {r} {w} {bore / 2} {w}
g: revolve cut xz axis y poly {r - groove} {w / 2 - 1.5} {r + 1} {w / 2 - 1.5 - groove * 0.6:.4f} {r + 1} {w / 2 + 1.5 + groove * 0.6:.4f} {r - groove} {w / 2 + 1.5}
fillet 0.5 (edges of g concave)
chamfer 0.5 (edges of p circle r={r})
"""


def manifold_block(l, w, h, ports):
    t = [f"b: extrude {h} rect 0 0 {l} {w}"]
    for i in range(ports):
        x = l * (i + 1) / (ports + 1)
        t.append(f"hole {w / 5} xz at {x} 0 {h / 2} {w * 0.75}")
        t.append(f"hole {w / 8} -xy at {x} {w / 2} {h} {h * 0.6}")
    t.append(f"hole {h / 5} yz at 0 {w / 2} {h / 2} {l - 2}")
    t.append("chamfer 0.5 (edges of b)")
    return "\n".join(t) + "\n"


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


# -- mutation of real parts: reorder, one op changed, selectors swapped ------------------------

SELS = ("top", "bottom", "left", "right", "front", "back", "convex", "concave", "circle", "plane", "cyl", "line")


def feature_lines(src):
    bs, cur = [], None
    for l in src.splitlines():
        if not l.strip() or l.lstrip().startswith("#"):
            continue
        if cur is not None:
            cur.append(l)
            if l.strip() == "end":
                bs.append("\n".join(cur))
                cur = None
        elif re.match(r"(\w+:\s*)?sketch\b", l):
            cur = [l]
        else:
            bs.append(l)
    return bs


def mutate_once(bs, r, pool):
    """One edit to a part's features: what a person's next try might be."""
    bs = list(bs)
    if not bs:
        return bs, "none"
    k = r.randrange(8)
    i = r.randrange(len(bs))
    if k == 0 and len(bs) > 2:  # move a feature
        x = bs.pop(i)
        bs.insert(r.randrange(1, len(bs) + 1), x)
        return bs, "move"
    if k == 1 and len(bs) > 2:  # drop a feature
        del bs[i]
        return bs, "drop"
    if k == 2:  # change one operation
        for _ in range(8):
            i = r.randrange(len(bs))
            m = re.search(r"\b(extrude|revolve|sweep|coil|loft|ball)( add| cut| keep| new)?\b", bs[i])
            if m:
                bs[i] = bs[i][: m.start()] + m.group(1) + " " + r.choice(("add", "cut", "keep", "new")) + bs[i][m.end() :]
                return bs, "op"
    if k == 3:  # swap a selector word
        for _ in range(8):
            i = r.randrange(len(bs))
            ws = [m for m in re.finditer(r"\b(" + "|".join(SELS) + r")\b", bs[i])]
            if ws and "(" in bs[i]:
                m = r.choice(ws)
                bs[i] = bs[i][: m.start()] + r.choice(SELS) + bs[i][m.end() :]
                return bs, "sel"
    if k == 4 and pool:  # a feature from another real part
        src = r.choice(pool)
        if src:
            bs.insert(r.randrange(1, len(bs) + 1), r.choice(src))
            return bs, "splice"
    if k == 5:  # a dimension moved
        for _ in range(8):
            i = r.randrange(len(bs))
            ms = list(re.finditer(r"(?<![\w.@=])\d+\.?\d*(?![\w.])", bs[i]))
            if ms:
                m = r.choice(ms)
                v = float(m.group(0)) * r.choice((0.5, 0.8, 0.99, 1.01, 1.25, 2))
                bs[i] = bs[i][: m.start()] + f"{v:.6g}" + bs[i][m.end() :]
                return bs, "dim"
    if k == 6:  # a finishing op on what is there
        bs.insert(r.randrange(1, len(bs) + 1), r.choice(("fillet 1 (edges top)", "chamfer 0.5 (edges concave)", "fillet 0.5 (edges circle)", "shell 1 (faces top)", "hole 3 -xy at 1 2 500 thru", "chamfer 0.5 (edges circle)", "draft 2 xy (faces normal +x)")))
        return bs, "finish"
    # duplicate a feature with an offset of one of its numbers
    bs.insert(i + 1, bs[i])
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
    "fillet {s} (edges concave)",
    "fillet {s} (edges convex)",
    "fillet {s} (edges circle)",
    "fillet {s} (edges along +z)",
    "fillet {s} (edges top)",
    "fillet {s} (edges line longer {b})",
    "fillet {s}..{S} (edges top)",
    "chamfer {s} (edges convex)",
    "chamfer {s} (edges circle)",
    "chamfer {s} (edges concave)",
    "chamfer {s} {S} face (faces top) (edges top)",
    "shell {s} (faces top)",
    "shell {s} (faces bottom)",
    "shell {s} out (faces top)",
    "draft 2 xy (faces normal +x)",
    "draft 3 xy (faces normal +y) (faces normal -y)",
    "offset {s} (faces top)",
    "offset -{s} (faces normal +x)",
    "thicken add {s} (faces top)",
    "move (faces top) by 0 0 {s}",
    "delete (faces cyl)",
    "delete (faces blend)",
    "facefillet {s} (faces normal +x) to (faces top)",
    "split plane xy +",
    "pattern body mirror yz",
)


def augmented(parts, seed=0):
    """A part as built, then one or two finishing steps sized to it. Only parts with one body:
    a selector reads the last body, and that may not be the one a person means."""
    r = random.Random(seed)
    for n, src in parts:
        ls = [l for l in src.splitlines() if l.strip()]
        size = 1.0
        nums = [float(x) for l in ls for x in re.findall(r"(?<![\w.])\d+\.?\d*(?![\w.])", l)][:200]
        if nums:
            nums.sort()
            size = max(nums[len(nums) // 2], 1.0)
        s = float(f"{size * r.choice((0.02, 0.05, 0.1)):.3g}")
        fill = {"s": s, "S": float(f"{s * 2:.3g}"), "b": float(f"{size:.3g}")}
        f1 = r.choice(FINISH).format(**fill)
        yield f"aug/{n}/{f1.split()[0]}", src.rstrip("\n") + "\n" + f1 + "\n"
        f2 = r.choice(FINISH).format(**fill)
        yield f"aug2/{n}/{f1.split()[0]}+{f2.split()[0]}", src.rstrip("\n") + "\n" + f1 + "\n" + f2 + "\n"


# -- noise: real data carries 1e-7 of float noise, and a part must not care --------------------

NOISE_LINE = re.compile(r"^(\w+:\s*)?(extrude|revolve|hole|ball|sweep|coil|loft)\b")
NOISE_FROM = re.compile(r"\b(at|circle|rect|rrect|poly|loop|profile|path)\b")
NOISE_NUM = re.compile(r"(?<![\w.@])-?\d+\.?\d*(?![\w.])")


def perturb(src, seed, size=3e-7):
    """Every coordinate moved by up to `size`: the part a sloppier float would have made."""
    r = random.Random(seed)
    out = []
    for l in src.splitlines():
        m = NOISE_FROM.search(l) if NOISE_LINE.match(l) else None
        if not m:
            out.append(l)
            continue
        head, tail = l[: m.start()], l[m.start() :]

        def bump(n):
            return repr(float(n.group(0)) + r.uniform(-size, size))

        out.append(head + NOISE_NUM.sub(bump, tail))
    return "\n".join(out) + "\n"
