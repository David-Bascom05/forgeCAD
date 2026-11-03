#!/usr/bin/env python3
"""Writes the caster subsystem's .fg files (SPEC 5.10, 6.2, 7) from the numbers below.

Forge has no per-instance params and a subassembly is rigid inside its parent (README, Forge gaps),
so the two caster variants and the flattened casters of test.fg are written from one text here.
Every part's expected volume is worked out here from the drawing numbers, independently of Forge.
usage: python3 casters/gen.py
"""
from math import pi, sqrt, cos, sin, radians, atan2, hypot
from pathlib import Path
import numpy as np

OUT = Path(__file__).resolve().parent


def num(v):
    s = f"{v:.6f}".rstrip("0").rstrip(".")
    return s if s not in ("-0", "") else "0"


def pts(ps):
    return "[" + ", ".join(f"({num(a)}, {num(b)})" for a, b in ps) + "]"


# ---- numbers (SPEC 5.10 / 6.2 unless marked "representative") ------------------------------------

# top plate
PL_W, PL_L, PL_T = 66.0, 90.0, 3.0
PL_DH, PL_BX, PL_BY = 8.5, 22.5, 37.5
KP_HOLE = 10.4
CSK_D = 16.0                   # the kingpin head's top diameter: a 90deg cone, 10.0 shank, 3.0 deep
# swivel ball rows
DW_S = 4.763
R_UP, Z_UP, N_UP = 25.0, 5.38, 24
R_LO, Z_LO, N_LO = 18.0, 12.64, 20
# swivel head (yoke top) and lower race
Z_H0, Z_H1 = 7.76, 10.26
Z_R0, Z_R1 = 15.03, 17.53
RACE_R = 25.5                  # representative: inside the yoke's skirt
# kingpin rivet
KP_D, KP_HEAD_D, KP_HEAD_H = 10.0, 16.0, 3.0
# yoke (representative pressed shape, SPEC numbers where given)
Y_T = 2.5
Y_HEAD_R = 29.0                # head disc, carries the upper row (to r 27.38)
Y_SKIRT_RI = 26.5              # skirt round the lower race
Y_WEB_RI = 11.0                # web under the race, clear of the rivet's lower head (r 8)
Z_W0, Z_W1 = 19.5, 22.0        # web
LEG_XI = 18.20
AX_Y, AX_Z = 40.0, 71.0        # axle at (y -40, z -71)
AX_HOLE = 8.4
PP_Y, PP_Z = 30.0, 40.0        # pedal pivot (y +30, z -40)
PP_D = 6.0
LEG_TOP_Y = 20.0
LEG_R_AX, LEG_R_PP = 8.0, 7.0
# lock pin and notches
LP_D = 5.0
LP_R, LP_ANG = 23.0, 52.5      # representative: r 23 at 52.5deg, so 52.5 = 7.5 + 3 x 15
LP_GUIDE = 5.2
LP_TOP0 = 19.2                 # pin top at brake 0: 1.67 under the race
LP_LIFT = 4.0
LP_DOME_Z0 = 27.5              # dome centre at brake 0
NOTCH_N, NOTCH_W, NOTCH_A0 = 24, 5.3, 7.5
NOTCH_OUT = 28.0
# wheel
WH_D, WH_W = 80.0, 32.0
TREAD_RI = 32.0                # representative: TPU 8 thick on the nylon centre
HUB_L, HUB_BOSS_R = 36.0, 15.0
HUB_BORE, HUB_BORE2 = 22.0, 16.0
# spacers
SP_ID, SP_OD, SP_L = 8.2, 12.0, 14.70
# 608-2RS
B_d, B_D, B_B, B_r = 8.0, 22.0, 7.0, 0.3
DW_B, DM_B, N_B = 3.969, 15.0, 7
CONF_I, CONF_O = 0.52, 0.53     # groove radius / ball diameter (representative)
DEPTH_I = DEPTH_O = 0.75        # groove depth below the ring lands (representative)
SEAL_T, SEAL_IN, SEAL_RO = 0.8, 0.15, 9.7
CAGE_RI, CAGE_RO, CAGE_T, CAGE_H = 6.5, 8.5, 0.45, 2.45
POCKET_R = 2.05
K_CAGE = (1 + DW_B / DM_B) / 2
# pedal (representative lever, computed below)
PED_T = 2.0
CHEEK_X = 13.0                  # inner face of the +x cheek; the pin sits over it
PED_HALF = CHEEK_X + PED_T      # 15
BRAKE = 20.0
SHOE_ALPHA = 10.0               # contact point on the tread, above the axle, facing the kingpin
SHOE_HALF, SHOE_T = 6.0, 2.0
PAD_Y0, PAD_Y1, PAD_Z0, PAD_Z1 = 62.0, 88.0, 38.0, 36.0
PPIN_HEAD_D, PPIN_HEAD_T = 9.0, 1.5
ENVELOPE = 90.0
KINGPINS = {"fl": (-185.0, 100.0), "fr": (185.0, 100.0), "rl": (-185.0, 700.0), "rr": (185.0, 700.0)}

LEG_XO = LEG_XI + Y_T
XP, YP = LP_R * cos(radians(LP_ANG)), LP_R * sin(radians(LP_ANG))
R_CAM = LP_LIFT / radians(BRAKE)          # rack_pinion radius: 4.0 at 20deg


# ---- pedal geometry: the lever drawn at brake 0 --------------------------------------------------

def rot(p, b):
    """A lever point at brake b (deg): pad down, about the pivot."""
    q = (PP_Y, -PP_Z)
    b = radians(b)
    y, z = p[0] - q[0], p[1] - q[1]
    return (q[0] + y * cos(b) + z * sin(b), q[1] - y * sin(b) + z * cos(b))


def convex_hull(ps):
    ps = sorted(set(ps))

    def cr(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    lo, up = [], []
    for p in ps:
        while len(lo) >= 2 and cr(lo[-2], lo[-1], p) <= 0:
            lo.pop()
        lo.append(p)
    for p in reversed(ps):
        while len(up) >= 2 and cr(up[-2], up[-1], p) <= 0:
            up.pop()
        up.append(p)
    return lo[:-1] + up[:-1]


def area(poly):
    return abs(sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1]
                   for i in range(len(poly)))) / 2


def segdist(p, a, b):
    ax, ay = b[0] - a[0], b[1] - a[1]
    t = max(0, min(1, ((p[0] - a[0]) * ax + (p[1] - a[1]) * ay) / (ax * ax + ay * ay)))
    return hypot(p[0] - a[0] - t * ax, p[1] - a[1] - t * ay)


def polydist(poly, c):
    return min(segdist(c, poly[i], poly[(i + 1) % len(poly)]) for i in range(len(poly)))


CW = (-AX_Y, -AX_Z)
a = radians(SHOE_ALPHA)
nrm, tan_ = (cos(a), sin(a)), (-sin(a), cos(a))
P = (CW[0] + WH_D / 2 * nrm[0], CW[1] + WH_D / 2 * nrm[1])
SHOE_E = [(P[0] + s * SHOE_HALF * tan_[0] + k * SHOE_T * nrm[0], P[1] + s * SHOE_HALF * tan_[1] + k * SHOE_T * nrm[1])
          for s, k in [(-1, 0), (1, 0), (1, 1), (-1, 1)]]
SHOE = [rot(p, -BRAKE) for p in SHOE_E]
C0 = (YP, -LP_DOME_Z0)
C1 = rot((YP, -LP_DOME_Z0 + LP_LIFT), -BRAKE)
D = (C1[0] - C0[0], C1[1] - C0[1])
L = hypot(*D)
nc = (-D[1] / L, D[0] / L)
if nc[1] < 0:
    nc = (-nc[0], -nc[1])
u = (D[0] / L, D[1] / L) if D[0] > 0 else (-D[0] / L, -D[1] / L)
P0 = (C0[0] - LP_D / 2 * nc[0], C0[1] - LP_D / 2 * nc[1])
P1 = (C1[0] - LP_D / 2 * nc[0], C1[1] - LP_D / 2 * nc[1])
lo_, hi_ = (P0, P1) if P0[0] < P1[0] else (P1, P0)
CAM = [(lo_[0] - 2 * u[0], lo_[1] - 2 * u[1]), (hi_[0] + 2 * u[0], hi_[1] + 2 * u[1])]
PAD_END = [(PAD_Y1, -PAD_Z1), (PAD_Y1, -PAD_Z0 - 2)]
boss = [(PP_Y + 6 * cos(radians(k / 10)), -PP_Z + 6 * sin(radians(k / 10))) for k in range(3600)]
CHEEK = convex_hull(SHOE + CAM + PAD_END + boss)
assert all(p in SHOE + CAM + PAD_END for p in CHEEK), "the pivot boss must lie inside the cheek"
assert all(c in CHEEK for c in CAM), "the cam edge must be a side of the cheek"
CHEEK_GAP0 = polydist([rot(p, 0) for p in CHEEK], CW) - WH_D / 2
CAM_GAPS = [segdist((YP, -LP_DOME_Z0 + LP_LIFT * b / BRAKE), rot(CAM[0], b), rot(CAM[1], b)) - LP_D / 2
            for b in np.linspace(0, BRAKE, 41)]
assert min(CAM_GAPS) > -1e-9, "the pin dome must never sink into the cam"
assert CHEEK_GAP0 >= 1.0
PAD = [(PAD_Y0, -PAD_Z0), (PAD_Y1, -PAD_Z0), (PAD_Y1, -PAD_Z1), (PAD_Y0, -PAD_Z1)]
PAD_CORNER_R = hypot(PAD_Y1, PED_HALF)
assert PAD_CORNER_R < ENVELOPE


# ---- volumes from the drawing --------------------------------------------------------------------

def rev_poly(poly):
    """Volume of a polygon (r, z) revolved about z (Pappus, exact)."""
    s = 0.0
    for i in range(len(poly)):
        (x0, y0), (x1, y1) = poly[i], poly[(i + 1) % len(poly)]
        s += (x0 + x1) * (x0 * y1 - x1 * y0)
    return abs(s) * pi / 3


def rev_numeric(inside, r0, r1, z0, z1, h=0.002):
    """Volume of the region inside(r, z) revolved about z, by a midpoint grid."""
    r = np.arange(r0 + h / 2, r1, h)
    v = 0.0
    for z in np.arange(z0 + h / 2, z1, h):
        m = inside(r, z)
        v += (r[m]).sum()
    return 2 * pi * v * h * h


def vox(inside, box, h):
    (x0, x1), (y0, y1), (z0, z1) = box
    x = np.arange(x0 + h / 2, x1, h)
    y = np.arange(y0 + h / 2, y1, h)
    X, Y = np.meshgrid(x, y, indexing="ij")
    n = 0
    for z in np.arange(z0 + h / 2, z1, h):
        n += int(inside(X, Y, z).sum())
    return n * h ** 3


def cyl(r, h):
    return pi * r * r * h


# ---- part files ----------------------------------------------------------------------------------

class Part:
    def __init__(self, name, title, source, frame):
        self.name, self.title, self.source, self.frame = name, title, source, frame
        self.params, self.lines, self.volume, self.tol = [], [], None, "0.05%"

    def p(self, name, value, comment=""):
        v = value if isinstance(value, str) else num(value)
        self.params.append(f"shared {name} = {v}" + (f"   # {comment}" if comment else ""))

    def text(self):
        out = [f"# {self.title}", f"# source: {self.source}", f"# frame: {self.frame}", ""]
        out += self.params + [""] + self.lines + [""]
        out += ["expect valid", "expect bodies = 1", f"expect volume = {self.volume:.3f} +- {self.tol}"]
        return "\n".join(out) + "\n"


REF = """sketch ref(on: xy) {
  o = pt(0, 0)
  xl = line((0, 0), (lref, 0), construction: true)
  yl = line((0, 0), (0, lref), construction: true)
  xn = line((lref, 0), (0, 0), construction: true)
}"""
XZ = "rotate(ref.plane, about: ref.xl, by: 90deg)"      # sketch (x, y) = caster (X, Z), normal -Y
YZ = "rotate(ref.plane, about: ref.yl, by: 90deg)"      # sketch (x, y) = caster (Y, Z), normal +X
CF = "cf = anchor(plane(at: ref.o, normal: ref.plane.normal, x: ref.xl.dir))"
CASTER_FRAME = ("the caster frame (SPEC 5.10): origin on the kingpin axis in the plate's top face, z up, "
                "y along the plate's long side, the wheel towards -y")
BEARING_FRAME = "origin at the bearing's centre, z along its axis"


def parts():
    out = []

    # ball rows
    b = Part("ball_4763", "steel ball 4.763 (3/16\"), ISO 3290 G100: the caster's swivel ball rows",
             "SPEC 6.2 (upper 24 on a 50.0 pitch circle, lower 20 on 36.0)",
             "origin at the ball's centre; uorb / lorb are the row's orbit centres, the ball out along their x")
    b.p("dw", DW_S, "ball diameter")
    b.p("rw", "dw / 2")
    b.p("ru", R_UP, "upper row pitch radius")
    b.p("rl", R_LO, "lower row pitch radius")
    b.lines = ["""sketch prof(on: xz) {
  c0 = pt(0, 0)
  pu = pt(-ru, 0)
  pl = pt(-rl, 0)
  zax = line((0, 0), (0, rw), construction: true)
  xax = line((0, 0), (rw, 0), construction: true)
}
shot = ball(prof.c0, rw)

c = anchor(plane(at: prof.c0, normal: prof.zax.dir, x: prof.xax.dir))
uorb = anchor(plane(at: prof.pu, normal: prof.zax.dir, x: prof.xax.dir))
lorb = anchor(plane(at: prof.pl, normal: prof.zax.dir, x: prof.xax.dir))"""]
    b.volume = pi * DW_S ** 3 / 6
    out.append(b)

    b = Part("ball_3969", "steel ball 3.969 (5/32\"), ISO 3290 G100: the 608's balls",
             "SPEC 6.2 / 7 (608-2RS: 7 x 3.969)", "origin at the ball's centre")
    b.p("dw", DW_B, "ball diameter")
    b.p("rw", "dw / 2")
    b.lines = ["""sketch prof(on: xz) {
  c0 = pt(0, 0)
  zax = line((0, 0), (0, rw), construction: true)
  xax = line((0, 0), (rw, 0), construction: true)
}
shot = ball(prof.c0, rw)

c = anchor(plane(at: prof.c0, normal: prof.zax.dir, x: prof.xax.dir))"""]
    b.volume = pi * DW_B ** 3 / 6
    out.append(b)

    # top plate
    p = Part("plate", "caster top plate 90 x 66 x 3.0 with the upper raceway and the kingpin's countersink",
             "SPEC 5.10, 6.2; Blickle LKPA-PATH 80KF (plate 90 x 66, holes 8.5 on 75 x 45)", CASTER_FRAME)
    p.p("w", PL_W, "plate width (x)")
    p.p("l", PL_L, "plate length (y)")
    p.p("t", PL_T, "thickness, not published: modelled 3.0")
    p.p("dh", PL_DH, "bolt holes")
    p.p("hx", f"[{num(-PL_BX)}, {num(PL_BX)}, {num(-PL_BX)}, {num(PL_BX)}]", "bolt holes, order bolt[0..3]")
    p.p("hy", f"[{num(-PL_BY)}, {num(-PL_BY)}, {num(PL_BY)}, {num(PL_BY)}]")
    p.p("dk", KP_HOLE, "kingpin hole")
    p.p("dc", CSK_D, "countersink top diameter: the rivet head")
    p.p("ack", "90deg")
    p.p("zu", Z_UP, "upper ball centres below the top face")
    p.p("zl", Z_LO, "lower ball centres")
    p.p("ru", R_UP)
    p.p("nu", f"count({N_UP})")
    p.p("nl", f"count({N_LO})")
    p.p("lref", 10)
    p.lines = [f"""sketch plan(on: xy) {{
  outline = rect(center: (0, 0), size: (w, l))
  for k in 0..4 {{
    bh = circle((hx[k], hy[k]), dh / 2)
  }}
  o = pt(0, 0)
  xl = line((0, 0), (lref, 0), construction: true)
  ex = pt(w / 2, 0)
  ey = pt(0, l / 2)
  for i in 0..{N_UP} {{
    u = line((0, 0), (ru * cos(360deg * i / nu), ru * sin(360deg * i / nu)), construction: true)
  }}
  for i in 0..{N_LO} {{
    v = line((0, 0), (ru * cos(360deg * i / nl), ru * sin(360deg * i / nl)), construction: true)
  }}
}}
slab = extrude(plan, -t)
kp = hole(at: plan.o, d: dk, csink: (dc, ack), depth: thru)
sketch bot(on: offset(plan.plane, -t)) {{
  for k in 0..4 {{
    c = pt(hx[k], hy[k])
  }}
}}
sketch orbu(on: offset(plan.plane, -zu)) {{
  o = pt(0, 0)
}}
sketch orbl(on: offset(plan.plane, -zl)) {{
  o = pt(0, 0)
}}

top = anchor(plane(at: plan.o, normal: plan.plane.normal, x: plan.xl.dir))
kingpin = anchor(plane(at: plan.o, normal: plan.plane.normal, x: plan.xl.dir))
edge_x = anchor(plane(at: plan.ex, normal: plan.plane.normal, x: plan.xl.dir))
edge_y = anchor(plane(at: plan.ey, normal: plan.plane.normal, x: plan.xl.dir))
for k in 0..4 {{
  bolt = anchor(plane(at: bot.c[k], normal: plan.plane.normal, x: plan.xl.dir))
}}
for i in 0..{N_UP} {{
  uorb = anchor(plane(at: orbu.o, normal: plan.plane.normal, x: plan.u[i].dir))
}}
for i in 0..{N_LO} {{
  lorb = anchor(plane(at: orbl.o, normal: plan.plane.normal, x: plan.v[i].dir))
}}"""]
    rh = KP_HOLE / 2
    hc = CSK_D / 2 - rh       # 90deg: depth = radius change
    p.volume = (PL_W * PL_L * PL_T - 4 * cyl(PL_DH / 2, PL_T)
                - pi * hc / 3 * ((CSK_D / 2) ** 2 + CSK_D / 2 * rh + rh * rh) - cyl(rh, PL_T - hc))
    out.append(p)

    # kingpin
    k = Part("kingpin", "kingpin rivet 10.0: countersunk head flush in the plate, lower head 16 x 3 under the race",
             "SPEC 6.2 (representative: Blickle gives only \"riveted kingpin\")", CASTER_FRAME)
    k.p("rs", KP_D / 2, "shank")
    k.p("rh", KP_HEAD_D / 2, "both heads")
    k.p("hk", CSK_D / 2 - KP_D / 2, "90deg countersunk head depth")
    k.p("zr", Z_R1, "lower head's top: the race's underside")
    k.p("hh", KP_HEAD_H, "lower head height")
    k.p("lref", 10)
    k.lines = [REF, f"""sketch prof(on: {XZ}) {{
  o = poly([(0, 0), (rh, 0), (rs, -hk), (rs, -zr), (rh, -zr), (rh, -zr - hh), (0, -zr - hh)])
  zax = line((0, 0), (0, -zr), construction: true)
}}
pin = revolve(prof.o, axis: prof.zax)

{CF}"""]
    k.volume = rev_poly([(0, 0), (8, 0), (5, -3), (5, -Z_R1), (8, -Z_R1), (8, -Z_R1 - 3), (0, -Z_R1 - 3)])
    out.append(k)

    # lower race
    r = Part("lower_race", "caster lower race: 2.5 steel washer, 36.0 ball track, 24 lock notches",
             "SPEC 6.2 (OD and notch form representative)", CASTER_FRAME)
    r.p("zt", Z_R0, "top face below the plate's top")
    r.p("tr", Z_R1 - Z_R0, "thickness")
    r.p("ro", RACE_R, "outer radius, inside the yoke skirt (26.5)")
    r.p("ri", KP_HOLE / 2, "kingpin hole")
    r.p("rp", LP_R, "lock pin radius")
    r.p("rx", NOTCH_OUT, "notch slot's outer centre, past the rim")
    r.p("wn", NOTCH_W, "notch width: the 5.0 pin + 0.15 a side")
    r.p("a0", f"{num(NOTCH_A0)}deg", "first notch: the pin sits at 52.5deg = 7.5 + 3 x 15")
    r.p("nn", f"count({NOTCH_N})")
    r.p("lref", 10)
    r.lines = [REF, f"""sketch plan(on: offset(ref.plane, -zt)) {{
  outer = circle((0, 0), ro)
  inner = circle((0, 0), ri)
}}
ring = extrude(plan, -tr)
sketch nt(on: plan) {{
  for k in 0..{NOTCH_N} {{
    s = slot((rp * cos(a0 + 360deg * k / nn), rp * sin(a0 + 360deg * k / nn)), (rx * cos(a0 + 360deg * k / nn), rx * sin(a0 + 360deg * k / nn)), wn / 2)
  }}
}}
for k in 0..{NOTCH_N} {{
  notch = cut(nt.s[k], thru)
}}

{CF}"""]

    def notch_area():
        h = 0.005
        g = np.arange(-RACE_R, RACE_R, h) + h / 2
        tot = 0
        for x0 in range(0, len(g), 500):
            X, Y = np.meshgrid(g[x0:x0 + 500], g, indexing="ij")
            R2 = X * X + Y * Y
            m = (R2 <= RACE_R ** 2) & (R2 >= (KP_HOLE / 2) ** 2)
            for kk in range(NOTCH_N):
                an = radians(NOTCH_A0 + 360 * kk / NOTCH_N)
                c, s = cos(an), sin(an)
                along = X * c + Y * s
                across = -X * s + Y * c
                t = np.clip(along, LP_R, NOTCH_OUT)
                m &= ~((along - t) ** 2 + across ** 2 <= (NOTCH_W / 2) ** 2)
            tot += int(m.sum())
        return tot * h * h
    r.volume = notch_area() * (Z_R1 - Z_R0)
    r.tol = "0.1%"
    out.append(r)

    # yoke
    y = Part("yoke", "caster yoke: pressed 2.5 swivel head with skirt and web, fork legs, axle and pedal pin holes, lock pin guide",
             "SPEC 6.2 (legs x +-18.20 inner, axle 8.4 at (y -40, z -71), pedal pin at (y 30, z -40)); "
             "the skirt and web are representative (README)", CASTER_FRAME)
    y.p("rk", KP_HOLE / 2, "kingpin clearance in the head")
    y.p("rh", Y_HEAD_R, "head and skirt outer radius")
    y.p("rsi", Y_SKIRT_RI, "skirt inner radius")
    y.p("rwi", Y_WEB_RI, "web inner radius")
    y.p("zh0", Z_H0, "head top face")
    y.p("zh1", Z_H1, "head underside")
    y.p("zw0", Z_W0, "web top")
    y.p("zw1", Z_W1, "web underside")
    y.p("xi", LEG_XI, "leg inner faces")
    y.p("tl", Y_T, "leg thickness")
    y.p("ya", AX_Y, "axle behind the kingpin")
    y.p("za", AX_Z, "axle below the plate top")
    y.p("da", AX_HOLE)
    y.p("yq", PP_Y, "pedal pivot")
    y.p("zq", PP_Z)
    y.p("dq", PP_D, "pedal pin, pressed")
    y.p("rba", LEG_R_AX, "leg round the axle")
    y.p("rbq", LEG_R_PP, "leg round the pedal pin")
    y.p("ylt", LEG_TOP_Y, "leg top edge, under the web")
    y.p("xp", XP, "lock pin guide")
    y.p("yp", YP)
    y.p("dg", LP_GUIDE)
    y.p("zb", AX_Z + WH_D / 2, "wheel bottom")
    y.p("xsp", B_B / 2, "right spacer's inner end")
    y.p("lref", 10)
    y.lines = [REF, f"""sketch prof(on: {XZ}) {{
  o = poly([(rk, -zh0), (rh, -zh0), (rh, -zw1), (rwi, -zw1), (rwi, -zw0), (rsi, -zw0), (rsi, -zh1), (rk, -zh1)])
  zax = line((0, 0), (0, -zw1), construction: true)
}}
cup = revolve(prof.o, axis: prof.zax)
sketch legr(on: offset({YZ}, xi)) {{
  outline = hull(disc((-ya, -za), rba), disc((yq, -zq), rbq), (ylt, -zw0), (-ylt, -zw0))
  ah = circle((-ya, -za), da / 2)
  ph = circle((yq, -zq), dq / 2)
}}
leg_r = extrude(legr, tl)
sketch legl(on: offset({YZ}, -xi)) {{
  outline = hull(disc((-ya, -za), rba), disc((yq, -zq), rbq), (ylt, -zw0), (-ylt, -zw0))
  ah = circle((-ya, -za), da / 2)
  ph = circle((yq, -zq), dq / 2)
}}
leg_l = extrude(legl, -tl)
sketch gd(on: offset(ref.plane, -zw0)) {{
  c = circle((xp, yp), dg / 2)
  g = pt(xp, yp)
}}
guide = cut(gd.c, thru)
sketch side(on: {YZ}) {{
  a = pt(-ya, -za)
  q = pt(yq, -zq)
  wb = pt(-ya, -zb)
  dn = line((-ya, -za), (-ya, -zb), construction: true)
}}
sketch sh(on: offset({YZ}, -xi - tl)) {{
  a = pt(-ya, -za)
}}
sketch sn(on: offset({YZ}, xi + tl)) {{
  a = pt(-ya, -za)
}}
sketch sl(on: offset({YZ}, -xi)) {{
  a = pt(-ya, -za)
}}
sketch sr(on: offset({YZ}, xsp)) {{
  a = pt(-ya, -za)
}}

{CF}
kingpin = anchor(plane(at: ref.o, normal: ref.plane.normal, x: ref.xl.dir))
axle = anchor(plane(at: side.a, normal: side.plane.normal, x: ref.yl.dir))
axle_head = anchor(plane(at: sh.a, normal: side.plane.normal, x: ref.yl.dir))
axle_nut = anchor(plane(at: sn.a, normal: side.plane.normal, x: ref.yl.dir))
sp_l = anchor(plane(at: sl.a, normal: side.plane.normal, x: ref.yl.dir))
sp_r = anchor(plane(at: sr.a, normal: side.plane.normal, x: ref.yl.dir))
pedal_pin = anchor(plane(at: side.q, normal: ref.xn.dir, x: ref.yl.dir))
lock_guide = anchor(plane(at: gd.g, normal: ref.plane.normal, x: ref.xl.dir))
wheel_bottom = anchor(plane(at: side.wb, normal: side.dn.dir, x: ref.yl.dir))"""]
    cup = [(KP_HOLE / 2, -Z_H0), (Y_HEAD_R, -Z_H0), (Y_HEAD_R, -Z_W1), (Y_WEB_RI, -Z_W1), (Y_WEB_RI, -Z_W0),
           (Y_SKIRT_RI, -Z_W0), (Y_SKIRT_RI, -Z_H1), (KP_HOLE / 2, -Z_H1)]
    legpts = ([(-AX_Y + LEG_R_AX * cos(radians(t / 20)), -AX_Z + LEG_R_AX * sin(radians(t / 20))) for t in range(7200)]
              + [(PP_Y + LEG_R_PP * cos(radians(t / 20)), -PP_Z + LEG_R_PP * sin(radians(t / 20))) for t in range(7200)]
              + [(LEG_TOP_Y, -Z_W0), (-LEG_TOP_Y, -Z_W0)])
    leg = convex_hull(legpts)
    a_leg = area(leg) - pi * (AX_HOLE / 2) ** 2 - pi * (PP_D / 2) ** 2
    def yrange(z):
        """The leg's y extent at height z (the hull is convex)."""
        ys = []
        n = len(leg)
        for i in range(n):
            (y0, z0), (y1, z1) = leg[i], leg[(i + 1) % n]
            if (z0 - z) * (z1 - z) <= 0 and z0 != z1:
                ys.append(y0 + (z - z0) * (y1 - y0) / (z1 - z0))
        return min(ys), max(ys)

    # legs over the web: x 18.2..20.7, z -22..-19.5, inside r 29 (y integrated exactly per (x, z))
    hz, hx = 0.0025, 0.0025
    over = 0.0
    for z in np.arange(-Z_W1 + hz / 2, -Z_W0, hz):
        y0, y1 = yrange(z)
        x = np.arange(LEG_XI + hx / 2, LEG_XO, hx)
        yc = np.sqrt(Y_HEAD_R ** 2 - x * x)
        over += np.clip(np.minimum(y1, yc) - np.maximum(y0, -yc), 0, None).sum() * hx * hz
    y.volume = rev_poly(cup) + 2 * (Y_T * a_leg - over) - cyl(LP_GUIDE / 2, Z_W1 - Z_W0)
    y.tol = "0.1%"
    y.leg = leg
    out.append(y)

    # lock pin
    lp = Part("lock_pin", "caster lock pin 5.0 with a domed foot: the lever's cam lifts it into a race notch",
              "SPEC 6.2 (5.0 pin, 4.0 travel); placed at brake 0", CASTER_FRAME + "; drawn at brake 0")
    lp.p("xp", XP)
    lp.p("yp", YP)
    lp.p("rp", LP_D / 2)
    lp.p("zt", LP_TOP0, "top at brake 0")
    lp.p("zd", LP_DOME_Z0, "dome centre at brake 0")
    lp.p("zg", Z_W0, "the guide's top face (the anchor)")
    lp.p("lref", 10)
    lp.lines = [REF, f"""sketch top(on: offset(ref.plane, -zt)) {{
  c = circle((xp, yp), rp)
  g = pt(xp, yp)
}}
body = extrude(top.c, -(zd - zt))
sketch dm(on: offset(ref.plane, -zd)) {{
  c = pt(xp, yp)
}}
foot = ball(dm.c, rp)
sketch ga(on: offset(ref.plane, -zg)) {{
  a = pt(xp, yp)
}}

{CF}
ax = anchor(plane(at: ga.a, normal: ref.plane.normal, x: ref.xl.dir))
tip = anchor(plane(at: top.g, normal: ref.plane.normal, x: ref.xl.dir))"""]
    lp.volume = cyl(LP_D / 2, LP_DOME_Z0 - LP_TOP0) + 2 / 3 * pi * (LP_D / 2) ** 3
    out.append(lp)

    # pedal pin
    pp = Part("pedal_pin", "caster pedal pivot pin 6.0, riveted heads 9.0 x 1.5 outside the yoke legs",
              "SPEC 6.2 (pivot pin 6.0 through the yoke legs at (y 30, z -40)); heads representative", CASTER_FRAME)
    pp.p("yq", PP_Y)
    pp.p("zq", PP_Z)
    pp.p("rs", PP_D / 2)
    pp.p("rh", PPIN_HEAD_D / 2)
    pp.p("th", PPIN_HEAD_T)
    pp.p("xo", LEG_XO, "leg outer faces")
    pp.p("lref", 10)
    pp.lines = [REF, f"""sketch h1(on: offset({YZ}, -xo - th)) {{
  c = circle((yq, -zq), rh)
}}
head1 = extrude(h1.c, th)
sketch sh(on: head1.end) {{
  c = circle((yq, -zq), rs)
}}
shank = extrude(sh.c, 2 * xo)
sketch h2(on: shank.end) {{
  c = circle((yq, -zq), rh)
}}
head2 = extrude(h2.c, th)

{CF}"""]
    pp.volume = 2 * cyl(PPIN_HEAD_D / 2, PPIN_HEAD_T) + cyl(PP_D / 2, 2 * LEG_XO)
    out.append(pp)

    # pedal lever
    pd = Part("pedal", "caster total-lock pedal: 2.0 steel lever, pad, brake shoe and the lock pin's cam edge",
              "SPEC 6.2 (2.0 lever, pivot (y 30, z -40), pad to y 88); shape representative, worked out in gen.py",
              CASTER_FRAME + "; drawn at brake 0")
    pd.p("xc", CHEEK_X, "cheek inner faces")
    pd.p("tc", PED_T, "lever sheet")
    pd.p("yq", PP_Y)
    pd.p("zq", PP_Z)
    pd.p("dh", PP_D + 0.2, "pivot hole, running on the 6.0 pin")
    pd.p("cheek", pts(CHEEK), "cheek outline at brake 0 (gen.py: shoe, cam edge, pad end)")
    pd.p("shoe", pts(SHOE), "shoe section: tangent to the tread at brake 20")
    pd.p("pad", pts(PAD), "pad section")
    pd.p("lref", 10)
    pd.lines = [REF, f"""sketch chr(on: offset({YZ}, xc)) {{
  outline = poly(cheek)
  ph = circle((yq, -zq), dh / 2)
}}
cheek_r = extrude(chr, tc)
sketch chl(on: offset({YZ}, -xc)) {{
  outline = poly(cheek)
  ph = circle((yq, -zq), dh / 2)
}}
cheek_l = extrude(chl, -tc)
sketch cross(on: offset({YZ}, -xc - tc)) {{
  s = poly(shoe)
  p = poly(pad)
}}
shoe_plate = extrude(cross.s, 2 * (xc + tc))
pad_plate = extrude(cross.p, 2 * (xc + tc))
sketch side(on: {YZ}) {{
  q = pt(yq, -zq)
}}

{CF}
pivot = anchor(plane(at: side.q, normal: ref.xn.dir, x: ref.yl.dir))"""]
    pd.volume = (2 * PED_T * (area(CHEEK) - pi * ((PP_D + 0.2) / 2) ** 2)
                 + 2 * CHEEK_X * (area(SHOE) + area(PAD)))
    out.append(pd)

    # spacer
    s = Part("spacer", "caster axle spacer: steel tube 8.2 ID x 12.0 OD x 14.70, leg to the bearing's inner ring",
             "SPEC 6.2", "origin on the axis in one end face; +z through the part")
    s.p("ri", SP_ID / 2)
    s.p("ro", SP_OD / 2)
    s.p("h", SP_L)
    s.lines = ["""sketch prof(on: xz) {
  o = poly([(ri, 0), (ro, 0), (ro, h), (ri, h)])
  pz0 = pt(0, 0)
  pz1 = pt(0, h)
  zax = line((0, 0), (0, h), construction: true)
  xax = line((0, 0), (ro, 0), construction: true)
}
body = revolve(prof.o, axis: prof.zax)

a = anchor(plane(at: prof.pz0, normal: prof.zax.dir, x: prof.xax.dir))
b = anchor(plane(at: prof.pz1, normal: prof.zax.dir, x: prof.xax.dir))"""]
    s.volume = pi * ((SP_OD / 2) ** 2 - (SP_ID / 2) ** 2) * SP_L
    out.append(s)

    # wheel centre and tread
    WF = "origin at the wheel's centre, z along the axle (towards the nut side in the caster)"
    hb = Part("hub", "caster wheel centre: nylon 6, hub 36 long (bosses 2.0 a side), bearing bore 22 with a shoulder",
              "SPEC 6.2; Blickle (hub 36, axle bore 8); section representative", WF)
    hb.p("ro", TREAD_RI, "tread seat")
    hb.p("hw", WH_W / 2, "half the wheel width")
    hb.p("hl", HUB_L / 2, "half the hub length")
    hb.p("rbo", HUB_BOSS_R, "boss radius")
    hb.p("rb", HUB_BORE / 2, "bearing bore")
    hb.p("rb2", HUB_BORE2 / 2, "bore past the shoulder")
    hb.p("zs", B_B / 2, "shoulder: the bearing's +z face")
    hpoly = [(HUB_BORE / 2, -HUB_L / 2), (HUB_BOSS_R, -HUB_L / 2), (HUB_BOSS_R, -WH_W / 2), (TREAD_RI, -WH_W / 2),
             (TREAD_RI, WH_W / 2), (HUB_BOSS_R, WH_W / 2), (HUB_BOSS_R, HUB_L / 2), (HUB_BORE2 / 2, HUB_L / 2),
             (HUB_BORE2 / 2, B_B / 2), (HUB_BORE / 2, B_B / 2)]
    hb.lines = ["""sketch prof(on: xz) {
  o = poly([(rb, -hl), (rbo, -hl), (rbo, -hw), (ro, -hw), (ro, hw), (rbo, hw), (rbo, hl), (rb2, hl), (rb2, zs), (rb, zs)])
  pz0 = pt(0, 0)
  zax = line((0, 0), (0, hl), construction: true)
  xax = line((0, 0), (ro, 0), construction: true)
}
body = revolve(prof.o, axis: prof.zax)

ax = anchor(plane(at: prof.pz0, normal: prof.zax.dir, x: prof.xax.dir))"""]
    hb.volume = rev_poly(hpoly)
    out.append(hb)

    tr = Part("tread", "caster tread: TPU 94 Sh A, 80 dia x 32", "SPEC 6.2, 7 (Blickle: 80 x 32 TPU on nylon)", WF)
    tr.p("ro", WH_D / 2)
    tr.p("ri", TREAD_RI)
    tr.p("hw", WH_W / 2)
    tr.lines = ["""sketch prof(on: xz) {
  o = poly([(ri, -hw), (ro, -hw), (ro, hw), (ri, hw)])
  pz0 = pt(0, 0)
  zax = line((0, 0), (0, hw), construction: true)
  xax = line((0, 0), (ro, 0), construction: true)
}
body = revolve(prof.o, axis: prof.zax)

ax = anchor(plane(at: prof.pz0, normal: prof.zax.dir, x: prof.xax.dir))"""]
    tr.volume = pi * ((WH_D / 2) ** 2 - TREAD_RI ** 2) * WH_W
    out.append(tr)

    # 608-2RS
    h = B_B / 2
    rgi, rgo = CONF_I * DW_B, CONF_O * DW_B
    rbot_i, rbot_o = DM_B / 2 - DW_B / 2, DM_B / 2 + DW_B / 2
    rland_i, rland_o = rbot_i + DEPTH_I, rbot_o - DEPTH_O
    cgi, cgo = rbot_i + rgi, rbot_o - rgo     # groove circle centres (r)
    zs = h - SEAL_IN - SEAL_T                 # seal's inner face
    bi = Part("bearing_inner", "608-2RS inner ring: d 8, B 7, raceway groove", "ISO 15 / SPEC 6.2, 7 (GMN, MinebeaMitsumi 608); "
              "groove radius 0.52 Dw and depth 0.75 representative", BEARING_FRAME)
    bi.p("rb", B_d / 2, "bore")
    bi.p("rl", rland_i, "land (shoulder) radius")
    bi.p("hb", h, "half the width")
    bi.p("ch", B_r, "bore chamfer r 0.3")
    bi.p("rg", rgi, "groove radius")
    bi.p("rgc", cgi, "groove centre: the ball touches the groove bottom on the 15.0 pitch circle")
    ipoly = [(B_d / 2 + B_r, -h), (rland_i, -h), (rland_i, h), (B_d / 2 + B_r, h), (B_d / 2, h - B_r), (B_d / 2, -h + B_r)]
    bi.lines = ["""sketch prof(on: xz) {
  o = poly([(rb + ch, -hb), (rl, -hb), (rl, hb), (rb + ch, hb), (rb, hb - ch), (rb, -hb + ch)])
  pz0 = pt(0, 0)
  zax = line((0, 0), (0, hb), construction: true)
  xax = line((0, 0), (rl, 0), construction: true)
}
ring = revolve(prof.o, axis: prof.zax)
sketch gs(on: prof) {
  g = circle((rgc, 0), rg)
  zax = line((0, 0), (0, hb), construction: true)
}
groove = revolve(gs.g, axis: gs.zax, cut: true)

ax = anchor(plane(at: prof.pz0, normal: prof.zax.dir, x: prof.xax.dir))"""]
    bi.volume = rev_poly(ipoly) - rev_numeric(lambda r, z: ((r - cgi) ** 2 + z * z <= rgi ** 2) & (r <= rland_i),
                                              cgi - rgi, rland_i, -rgi, rgi)
    bi.tol = "0.1%"
    out.append(bi)

    bo = Part("bearing_outer", "608-2RS outer ring: D 22, B 7, raceway groove, seal recesses",
              "ISO 15 / SPEC 6.2, 7; groove radius 0.53 Dw, depth 0.75 and the recess representative", BEARING_FRAME)
    bo.p("ro", B_D / 2, "outside")
    bo.p("rl", rland_o, "land (shoulder) radius")
    bo.p("rs", SEAL_RO, "seal recess")
    bo.p("zs", zs, "recess floor: the seal's inner face")
    bo.p("hb", h)
    bo.p("ch", B_r, "outside chamfer r 0.3")
    bo.p("rg", rgo)
    bo.p("rgc", cgo)
    opoly = [(rland_o, -zs), (SEAL_RO, -zs), (SEAL_RO, -h), (B_D / 2 - B_r, -h), (B_D / 2, -h + B_r), (B_D / 2, h - B_r),
             (B_D / 2 - B_r, h), (SEAL_RO, h), (SEAL_RO, zs), (rland_o, zs)]
    bo.lines = ["""sketch prof(on: xz) {
  o = poly([(rl, -zs), (rs, -zs), (rs, -hb), (ro - ch, -hb), (ro, -hb + ch), (ro, hb - ch), (ro - ch, hb), (rs, hb), (rs, zs), (rl, zs)])
  pz0 = pt(0, 0)
  zax = line((0, 0), (0, hb), construction: true)
  xax = line((0, 0), (ro, 0), construction: true)
}
ring = revolve(prof.o, axis: prof.zax)
sketch gs(on: prof) {
  g = circle((rgc, 0), rg)
  zax = line((0, 0), (0, hb), construction: true)
}
groove = revolve(gs.g, axis: gs.zax, cut: true)

ax = anchor(plane(at: prof.pz0, normal: prof.zax.dir, x: prof.xax.dir))"""]
    bo.volume = rev_poly(opoly) - rev_numeric(lambda r, z: ((r - cgo) ** 2 + z * z <= rgo ** 2) & (r >= rland_o),
                                              rland_o, cgo + rgo, -rgo, rgo)
    bo.tol = "0.1%"
    out.append(bo)

    se = Part("seal", "608-2RS rubber seal 0.8 (one of two): in the outer ring's recess, lip on the inner ring's land",
              "SPEC 6.2 (2 rubber seals 0.8 thick in the outer ring); section representative",
              BEARING_FRAME + "; drawn on the +z side; `back` turns it over for the -z side")
    se.p("ri", rland_i, "lip: on the inner ring's land")
    se.p("ro", SEAL_RO)
    se.p("z0", zs)
    se.p("ts", SEAL_T)
    se.lines = ["""sketch prof(on: xz) {
  o = poly([(ri, z0), (ro, z0), (ro, z0 + ts), (ri, z0 + ts)])
  pz0 = pt(0, 0)
  zax = line((0, 0), (0, z0), construction: true)
  zn = line((0, z0), (0, 0), construction: true)
  xax = line((0, 0), (ro, 0), construction: true)
}
body = revolve(prof.o, axis: prof.zax)

ax = anchor(plane(at: prof.pz0, normal: prof.zax.dir, x: prof.xax.dir))
back = anchor(plane(at: prof.pz0, normal: prof.zn.dir, x: prof.xax.dir))"""]
    se.volume = pi * (SEAL_RO ** 2 - rland_i ** 2) * SEAL_T
    out.append(se)

    cg = Part("cage_half", "608 pressed steel ribbon cage, one half: flat web between 7 spherical pocket cups",
              "SPEC 6.2 (pressed steel 2-piece ribbon cage, modelled, not a catalogue drawing)",
              BEARING_FRAME + "; the half on +z; `back` turns it over for the other half")
    cg.p("rci", CAGE_RI)
    cg.p("rco", CAGE_RO)
    cg.p("tw", CAGE_T, "web (ribbon) thickness")
    cg.p("hc", CAGE_H, "cup height")
    cg.p("rpo", CAGE_H, "cup outer radius")
    cg.p("rpi", POCKET_R, "pocket: the 3.969 ball + 0.065")
    cg.p("rm", DM_B / 2, "pitch radius")
    cg.p("nb", f"count({N_B})")
    cg.p("lref", 10)
    cg.lines = [f"""sketch pk(on: xy) {{
  for i in 0..{N_B} {{
    p = pt(rm * cos(360deg * i / nb), rm * sin(360deg * i / nb))
  }}
  band = circle((0, 0), rco)
  bore = circle((0, 0), rci)
  o = pt(0, 0)
  xl = line((0, 0), (lref, 0), construction: true)
  zn = line((0, 0), (0, lref), construction: true)
}}
sketch prof(on: rotate(pk.plane, about: pk.xl, by: 90deg)) {{
  o = rect(corner: (rci, 0), size: (rco - rci, tw))
  zax = line((0, 0), (0, hc), construction: true)
}}
web = revolve(prof.o, axis: prof.zax)
for i in 0..{N_B} {{
  cup = ball(pk.p[i], rpo)
}}
trim = intersect(pk, hc)
for i in 0..{N_B} {{
  pocket = ball(pk.p[i], rpi, cut: true)
}}
sketch bk(on: offset(pk.plane, hc)) {{
  o = pt(0, 0)
}}

ax = anchor(plane(at: pk.o, normal: pk.plane.normal, x: pk.xl.dir))
for i in 0..{N_B} {{
  pocket_at = anchor(plane(at: pk.p[i], normal: pk.plane.normal, x: pk.xl.dir))
}}"""]
    # `back`: normal -z at the origin; the cup sketch has no -z line, so the turned half uses a pose (gen.py: CAGE_FLIP)

    # one pocket's sector (1/7) on a polar grid, cups of the two neighbours included
    hr, hz = 0.004, 0.004
    sec = 2 * pi / N_B
    r = np.arange(CAGE_RI + hr / 2, CAGE_RO, hr)
    th = np.arange(-sec / 2, sec / 2, sec / 2000) + sec / 4000
    R, T = np.meshgrid(r, th, indexing="ij")
    X, Y = R * np.cos(T), R * np.sin(T)
    vol = 0.0
    for z in np.arange(hz / 2, CAGE_H, hz):
        cups = np.zeros_like(X, dtype=bool)
        pock = np.zeros_like(X, dtype=bool)
        for i in (-1, 0, 1):
            d2 = (X - DM_B / 2 * cos(i * sec)) ** 2 + (Y - DM_B / 2 * sin(i * sec)) ** 2 + z * z
            cups |= d2 <= CAGE_H ** 2
            pock |= d2 <= POCKET_R ** 2
        m = ((z <= CAGE_T) | cups) & ~pock
        vol += (R[m]).sum()
    cg.volume = vol * hr * (sec / 2000) * hz * N_B
    cg.tol = "0.3%"
    out.append(cg)

    # test stand-ins (rack coordinates; not ordered parts)
    st = Part("pan_standin", "TEST STAND-IN: the base pan's floor (RACK-121) with the 16 caster bolt holes",
              "SPEC 5.2 (outer box x +-295.46, y 38.02..761.98, floor 3.02, caster holes 9.0 at kingpin + (+-22.5, "
              "+-37.5), anchors caster_* normal +z, x +x [R9]); flanges, other holes and the cut-out left out",
              "rack coordinates: origin at rack O, the floor's underside at Z 0")
    st.p("x0", 295.46)
    st.p("y0", 38.02)
    st.p("y1", 761.98)
    st.p("t", 3.02)
    st.p("dh", 9.0, "ISO 273 medium M8")
    st.p("kx", f"[{', '.join(num(c[0]) for c in KINGPINS.values())}]", "kingpins fl, fr, rl, rr")
    st.p("ky", f"[{', '.join(num(c[1]) for c in KINGPINS.values())}]")
    st.p("hx", f"[{num(-PL_BX)}, {num(PL_BX)}, {num(-PL_BX)}, {num(PL_BX)}]", "the plate's bolt[0..3] order")
    st.p("hy", f"[{num(-PL_BY)}, {num(-PL_BY)}, {num(PL_BY)}, {num(PL_BY)}]")
    st.p("lref", 10)
    st.lines = ["""sketch plan(on: xy) {
  outline = rect(corner: (-x0, y0), size: (2 * x0, y1 - y0))
  for c in 0..4 {
    for k in 0..4 {
      h = circle((kx[c] + hx[k], ky[c] + hy[k]), dh / 2)
    }
  }
  for c in 0..4 {
    kp = pt(kx[c], ky[c])
  }
  xl = line((0, 0), (lref, 0), construction: true)
}
floor = extrude(plan, t)
sketch tops(on: floor.end) {
  for c in 0..4 {
    for k in 0..4 {
      s = pt(kx[c] + hx[k], ky[c] + hy[k])
    }
  }
}
""" + "\n".join(f"caster_{n} = anchor(plane(at: plan.kp[{i}], normal: plan.plane.normal, x: plan.xl.dir))"
               for i, n in enumerate(KINGPINS)) + "\n" +
        "\n".join(f"for k in 0..4 {{\n  seat_{n} = anchor(plane(at: tops.s[{i}][k], normal: plan.plane.normal, x: plan.xl.dir))\n}}"
                   for i, n in enumerate(KINGPINS))]
    st.volume = 2 * 295.46 * (761.98 - 38.02) * 3.02 - 16 * cyl(4.5, 3.02)
    out.append(st)

    fl = Part("floor_ref", "TEST REFERENCE: the floor plane at Z -111 (RACK-FLOOR), as a 10 thick slab",
              "SPEC 1 (floor Z = -111.00), 4 (RACK-FLOOR)", "rack coordinates")
    fl.p("hx", 300)
    fl.p("ly", 800)
    fl.p("zf", 111, "floor below the pan's underside")
    fl.p("t", 10)
    fl.lines = ["""sketch plan(on: offset(xy, -zf)) {
  outline = rect(corner: (-hx, 0), size: (2 * hx, ly))
}
slab = extrude(plan, -t)"""]
    fl.volume = 600 * 800 * 10
    out.append(fl)

    ev = Part("envelope_ref", "TEST REFERENCE: the swivel envelope gauge, a ring of inner radius 89.5 (90 less 0.5) about the kingpin",
              "SPEC 1 and 6.2 (swivel envelope radius 90: wheel 81.6, pedal 88)", CASTER_FRAME)
    ev.p("ri", ENVELOPE - 0.5, "the envelope less the 0.5 the sweep must keep")
    ev.p("ro", ENVELOPE + 5)
    ev.p("z0", 1, "clear of the plate's top")
    ev.p("z1", AX_Z + WH_D / 2 - 1, "clear of the floor")
    ev.lines = ["""sketch plan(on: offset(xy, -z0)) {
  o = circle((0, 0), ro)
  i = circle((0, 0), ri)
}
ring = extrude(plan, -(z1 - z0))"""]
    ev.volume = pi * ((ENVELOPE + 5) ** 2 - (ENVELOPE - 0.5) ** 2) * (AX_Z + WH_D / 2 - 2)
    out.append(ev)
    return out


# ---- assemblies ----------------------------------------------------------------------------------

def caster_body(p, lock, hw, plate_ground, cx=0.0, cy=0.0):
    """The caster's instances, joints, relations and fits, every id prefixed with p, with its plate at
    (cx, cy, 0). Every instance is written where it sits at zero drives: Forge takes each relation's
    offset from the written poses (README, Forge gaps)."""
    L = []
    X, Y = num(cx), num(cy)
    at0 = f"pose(at: ({X}, {Y}, 0))"
    def ax_at(dx, flip=False):
        return f"pose(at: ({num(cx + dx)}, {num(cy - AX_Y)}, -{num(AX_Z)}), z: ({'-1' if flip else '1'}, 0, 0), x: (0, 1, 0))"
    wh = ax_at(0)
    a = L.append
    a(f'{p}plate = part("plate.fg", at: {at0}{", ground: true" if plate_ground else ""})')
    a(f'{p}kpin = part("kingpin.fg", at: {at0})')
    a(f'{p}race = part("lower_race.fg", at: {at0})')
    a(f'{p}yoke = part("yoke.fg", at: {at0})')
    a(f'{p}fix_kpin = fastened({p}kpin.cf, {p}plate.top)')
    a(f'{p}fix_race = fastened({p}race.cf, {p}plate.top)')
    a(f'{p}swivel = revolute({p}yoke.kingpin, {p}plate.kingpin, turn: {p}swivel_turn)')
    a(f'{p}f_kpin = fit({p}kpin, {p}plate)')
    a(f'{p}f_race = fit({p}kpin, {p}race)')
    a(f'for i in 0..{N_UP} {{')
    a(f'  {p}ub = part("ball_4763.fg", at: pose(at: ({X} + ru * cos(au * i), {Y} + ru * sin(au * i), -zu), rz: au * i))')
    a(f'  {p}up = revolute({p}ub.uorb, {p}plate.uorb[i])')
    a(f'  {p}up_rel = gear({p}swivel, {p}up, ratio: k_race)')
    a(f'  {p}f_ub = fit({p}ub, {p}yoke)')
    a('}')
    a(f'for i in 0..{N_LO} {{')
    a(f'  {p}lb = part("ball_4763.fg", at: pose(at: ({X} + rl * cos(al * i), {Y} + rl * sin(al * i), -zl), rz: al * i))')
    a(f'  {p}lo = revolute({p}lb.lorb, {p}plate.lorb[i])')
    a(f'  {p}lo_rel = gear({p}swivel, {p}lo, ratio: k_race)')
    a(f'  {p}f_lb = fit({p}lb, {p}yoke)')
    a('}')
    a(f'{p}axle = part("{hw}iso4014_m8x55.fg", at: {ax_at(-LEG_XO)})')
    a(f'{p}nut = part("{hw}iso10511_m8.fg", at: {ax_at(LEG_XO)})')
    a(f'{p}fix_axle = fastened({p}axle.head, {p}yoke.axle_head)')
    a(f'{p}fix_nut = fastened({p}nut.seat, {p}yoke.axle_nut)')
    a(f'{p}f_axle = fit({p}axle, {p}yoke)')
    a(f'{p}f_nut = fit({p}nut, {p}yoke)')
    a(f'{p}sp_l = part("spacer.fg", at: {ax_at(-LEG_XI)})')
    a(f'{p}sp_r = part("spacer.fg", at: {ax_at(B_B / 2)})')
    a(f'{p}fix_sp_l = fastened({p}sp_l.a, {p}yoke.sp_l)')
    a(f'{p}fix_sp_r = fastened({p}sp_r.a, {p}yoke.sp_r)')
    a(f'{p}f_sp_l = fit({p}sp_l, {p}yoke)')
    a(f'{p}f_sp_r = fit({p}sp_r, {p}yoke)')
    # the bearing, part by part (a subassembly is rigid inside its parent: README, Forge gaps)
    a(f'{p}bi = part("bearing_inner.fg", at: {wh})')
    a(f'{p}ring_in = fastened({p}bi.ax, {p}yoke.axle)')
    a(f'{p}f_bi_axle = fit({p}bi, {p}axle)')
    a(f'{p}f_bi_sp_l = fit({p}bi, {p}sp_l)')
    a(f'{p}f_bi_sp_r = fit({p}bi, {p}sp_r)')
    a(f'{p}hub = part("hub.fg", at: {wh})')
    a(f'{p}wheel = revolute({p}hub.ax, {p}yoke.axle, turn: {p}roll)')
    a(f'{p}tread = part("tread.fg", at: {wh})')
    a(f'{p}fix_tread = fastened({p}tread.ax, {p}hub.ax)')
    a(f'{p}f_tread = fit({p}tread, {p}hub)')
    a(f'{p}bo = part("bearing_outer.fg", at: {wh})')
    a(f'{p}ring_out = fastened({p}bo.ax, {p}hub.ax)')
    a(f'{p}f_bo = fit({p}bo, {p}hub)')
    a(f'{p}cage_a = part("cage_half.fg", at: {wh})')
    a(f'{p}cage = revolute({p}cage_a.ax, {p}yoke.axle)')
    a(f'{p}cage_rel = gear({p}wheel, {p}cage, ratio: k_cage)')
    a(f'{p}cage_b = part("cage_half.fg", at: {ax_at(0, True)})')
    a(f'{p}fix_cage_b = fastened({p}cage_b.ax, {p}cage_a.ax, flip: true)')
    a(f'{p}f_cage = fit({p}cage_a, {p}cage_b)')
    a(f'for i in 0..{N_B} {{')
    a(f'  {p}bb = part("ball_3969.fg", at: pose(at: ({X}, {num(cy - AX_Y)} + pr * cos(ab * i), -{num(AX_Z)} + pr * sin(ab * i)), z: (1, 0, 0), x: (0, 1, 0)))')
    a(f'  {p}ball_fix = fastened({p}bb.c, {p}cage_a.pocket_at[i])')
    a(f'  {p}f_bb_in = fit({p}bb, {p}bi)')
    a(f'  {p}f_bb_out = fit({p}bb, {p}bo)')
    a('}')
    a(f'{p}seal_a = part("seal.fg", at: {wh})')
    a(f'{p}seal_b = part("seal.fg", at: {ax_at(0, True)})')
    a(f'{p}fix_seal_a = fastened({p}seal_a.ax, {p}bo.ax)')
    a(f'{p}fix_seal_b = fastened({p}seal_b.back, {p}bo.ax)')
    for s in "ab":
        a(f'{p}f_seal_{s}_out = fit({p}seal_{s}, {p}bo)')
        a(f'{p}f_seal_{s}_in = fit({p}seal_{s}, {p}bi)')
    if lock:
        a(f'{p}lever = part("pedal.fg", at: {at0})')
        a(f'{p}pedal = revolute({p}lever.pivot, {p}yoke.pedal_pin, turn: {p}brake, limit: (brake_lo, brake_hi))')
        a(f'{p}ppin = part("pedal_pin.fg", at: {at0})')
        a(f'{p}fix_ppin = fastened({p}ppin.cf, {p}yoke.kingpin)')
        a(f'{p}f_ppin = fit({p}ppin, {p}yoke)')
        a(f'{p}lpin = part("lock_pin.fg", at: {at0})')
        a(f'{p}lockpin = slider({p}lpin.ax, {p}yoke.lock_guide, limit: (lift_lo, lift_hi))')
        a(f'{p}pin_rel = rack_pinion({p}pedal, {p}lockpin, radius: r_cam)')
        a(f'{p}f_cam = fit({p}lever, {p}lpin)')
        a(f'{p}f_shoe = fit({p}lever, {p}tread)')
    return L


SHARED = [
    ("ru", num(R_UP), "upper ball row pitch radius (written poses)"),
    ("zu", num(Z_UP), ""),
    ("au", f"{num(360 / N_UP)}deg", ""),
    ("rl", num(R_LO), "lower ball row"),
    ("zl", num(Z_LO), ""),
    ("al", f"{num(360 / N_LO)}deg", ""),
    ("pr", num(DM_B / 2), "608 pitch radius"),
    ("ab", f"{num(360 / N_B)}deg", ""),
    ("k_race", "ratio(-0.5)", "ball rows: orbit at half the swivel, the same way (Forge's gear turns the follower by -ratio)"),
    ("k_cage", f"ratio({-K_CAGE:.6f})", "608 cage: (1 + Dw/dm)/2 = 0.6323 of the outer ring, inner ring fixed"),
    ("brake_lo", "0deg", ""),
    ("brake_hi", f"{num(BRAKE)}deg", "pedal down: shoe on the tread, pin in a notch"),
    ("lift_lo", "0", ""),
    ("lift_hi", num(LP_LIFT), "lock pin travel"),
    ("r_cam", num(R_CAM), "lock pin lift per radian of pedal: 4.0 at 20deg"),
]


def drives(p, lock):
    d = [(f"{p}swivel_turn", "0deg"), (f"{p}roll", "0deg")]
    if lock:
        d.append((f"{p}brake", "0deg"))
    return d


def caster_doc(lock):
    name = "caster_lock" if lock else "caster_plain"
    title = ("Blickle LKPA-PATH 80KF-FI (ID 755344): swivel caster, total lock" if lock
             else "Blickle LKPA-PATH 80KF (ID 755346): swivel caster")
    out = ["assembly", "", f"# {title}",
           "# SPEC 5.10, 6.2, 7. Frame: the caster frame (plate top at z 0, kingpin on z, wheel towards -y).",
           "# Drives: swivel_turn (free), roll (free), brake (lock only, 0..20deg); the cage and the ball",
           "# orbits follow by relations. Written by casters/gen.py.", ""]
    for n, v, c in SHARED:
        if not lock and n in ("brake_lo", "brake_hi", "lift_lo", "lift_hi", "r_cam"):
            continue
        out.append(f"shared {n} = {v}" + (f"   # {c}" if c else ""))
    for n, v in drives("", lock):
        out.append(f"shared {n} = {v}")
    out += ["shared m8 = 8", "shared m8_min = 8   # the nyloc's whole thread", ""]
    out += caster_body("", lock, "../hw/", True)
    out += ["m8_rule = fit_rule(external_thread, internal_thread, size: m8, min_engagement: m8_min)", ""]
    out += [
        "expect interferences = 0",
        "expect mobility = 0",
        f"expect z(yoke.wheel_bottom) = -{num(AX_Z + WH_D / 2)} +- 0.01",
        f"expect y(hub) = -{num(AX_Y)} +- 0.01",
        f"expect z(hub) = -{num(AX_Z)} +- 0.01",
    ]
    if lock:
        out += [
            "expect interferences = 0 at brake = 20deg",
            "expect clearance(lever, tread) >= 1.0",
            "expect clearance(lever, tread) <= 0.01 at brake = 20deg",
            "expect clearance(lpin, race) >= 1.0",
        ]
    (OUT / f"{name}.fg").write_text("\n".join(out) + "\n")


def bearing_doc():
    out = ["assembly", "", "# 608-2RS deep groove ball bearing, d 8 x D 22 x B 7 (ISO 15), as modelled for the caster wheel.",
           "# SPEC 6.2, 7. Frame: the inner ring's (origin at the centre, z on the axis). The inner ring is the",
           "# reference, as on the caster's axle; the outer ring turns (outer_turn) and the cage follows at",
           "# (1 + Dw/dm)/2 of it. Written by casters/gen.py.", "",
           f'shared k_cage = ratio({-K_CAGE:.6f})   # (1 + 3.969/15)/2 = 0.6323, the way the outer ring turns',
           "shared outer_turn = 0deg", f"shared pr = {num(DM_B / 2)}", f"shared ab = {num(360 / N_B)}deg", "",
           'inner = part("bearing_inner.fg", ground: true)',
           'outer = part("bearing_outer.fg")',
           "ring = revolute(outer.ax, inner.ax, turn: outer_turn)",
           'cage_a = part("cage_half.fg")',
           "cage = revolute(cage_a.ax, inner.ax)",
           "cage_rel = gear(ring, cage, ratio: k_cage)",
           'cage_b = part("cage_half.fg", at: pose(z: (0, 0, -1), x: (1, 0, 0)))',
           "fix_cage_b = fastened(cage_b.ax, cage_a.ax, flip: true)",
           "f_cage = fit(cage_a, cage_b)",
           f"for i in 0..{N_B} {{",
           '  bb = part("ball_3969.fg", at: pose(at: (pr * cos(ab * i), pr * sin(ab * i), 0)))',
           "  ball_fix = fastened(bb.c, cage_a.pocket_at[i])",
           "  f_in = fit(bb, inner)",
           "  f_out = fit(bb, outer)",
           "}",
           'seal_a = part("seal.fg")',
           'seal_b = part("seal.fg", at: pose(z: (0, 0, -1), x: (1, 0, 0)))',
           "fix_seal_a = fastened(seal_a.ax, outer.ax)",
           "fix_seal_b = fastened(seal_b.back, outer.ax)",
           "f_sa_out = fit(seal_a, outer)", "f_sa_in = fit(seal_a, inner)",
           "f_sb_out = fit(seal_b, outer)", "f_sb_in = fit(seal_b, inner)", "",
           "expect interferences = 0",
           "expect mobility = 0",
           "expect interferences = 0 at outer_turn = 40deg",
           f"expect value(cage.turn) = {K_CAGE * 90:.4f}deg +- 0.001deg at outer_turn = 90deg",
           f"expect distance(bb[0].c, inner.ax) = {num(DM_B / 2)} +- 0.001",
           ]
    (OUT / "bearing_608.fg").write_text("\n".join(out) + "\n")


SUB_RULES = [("ball_4763.fg", "yoke.fg"), ("kingpin.fg", "plate.fg"), ("kingpin.fg", "lower_race.fg"),
             ("../hw/iso4014_m8x55.fg", "yoke.fg"), ("../hw/iso10511_m8.fg", "yoke.fg"),
             ("spacer.fg", "yoke.fg"),
             ("spacer.fg", "bearing_inner.fg"), ("bearing_inner.fg", "../hw/iso4014_m8x55.fg"),
             ("tread.fg", "hub.fg"), ("bearing_outer.fg", "hub.fg"), ("cage_half.fg", "cage_half.fg"),
             ("ball_3969.fg", "bearing_inner.fg"), ("ball_3969.fg", "bearing_outer.fg"),
             ("seal.fg", "bearing_outer.fg"), ("seal.fg", "bearing_inner.fg"),
             ("pedal_pin.fg", "yoke.fg"), ("pedal.fg", "lock_pin.fg"), ("pedal.fg", "tread.fg"),
             ("../hw/iso4017_m8x20.fg", "plate.fg"), ("plate.fg", "pan_standin.fg"), ("tread.fg", "floor_ref.fg")]


def test_doc():
    o = ["assembly", "",
         "# casters/test.fg: SPEC 9C. The front-left caster (lock) and rear-left caster (plain) are written out part by",
         "# part (prefixes fl_, rl_) so the test can drive their swivels and brake; the right-hand two are the",
         "# caster_lock.fg / caster_plain.fg subassemblies, rigid at their own drives (README). All on a stand-in",
         "# base pan floor with the SPEC 5.10 bolt stacks. Written by casters/gen.py.", ""]
    for n, v, c in SHARED:
        o.append(f"shared {n} = {v}" + (f"   # {c}" if c else ""))
    for p_, lock in (("fl_", True), ("rl_", False)):
        for n, v in drives(p_, lock):
            o.append(f"shared {n} = {v}")
    o += ["shared m8 = 8", "shared m8_min = 8",
          f"shared hx = [{num(-PL_BX)}, {num(PL_BX)}, {num(-PL_BX)}, {num(PL_BX)}]   # the plate's bolt[0..3]",
          f"shared hy = [{num(-PL_BY)}, {num(-PL_BY)}, {num(PL_BY)}, {num(PL_BY)}]", ""]
    o += ['pan = part("pan_standin.fg", ground: true)',
          'floor = part("floor_ref.fg", reference: true, ground: true)']
    for n, (x, y) in KINGPINS.items():
        o.append(f'env_{n} = part("envelope_ref.fg", at: pose(at: ({num(x)}, {num(y)}, 0)), reference: true, ground: true)')
    o.append("")
    for n, p_, lock in (("fl", "fl_", True), ("rl", "rl_", False)):
        x, y = KINGPINS[n]
        o += caster_body(p_, lock, "../hw/", False, x, y)
        o.append(f"{p_}on_pan = fastened({p_}plate.top, pan.caster_{n})")
        o.append(f"{p_}f_pan = fit({p_}plate, pan)")
        o.append(f"{p_}f_floor = fit({p_}tread, floor)")
        o.append("")
    for n, f in (("fr", "caster_lock.fg"), ("rr", "caster_plain.fg")):
        x, y = KINGPINS[n]
        o.append(f'{n} = subassembly("{f}", at: pose(at: ({num(x)}, {num(y)}, 0)))')
        o.append(f"{n}_on_pan = fastened({n}.plate.top, pan.caster_{n})")
        o.append(f"{n}_f_pan = fit({n}, pan)")
        o.append(f"{n}_f_floor = fit({n}, floor)")
    o.append("")
    for n in KINGPINS:
        plate = f"{n}_plate" if n in ("fl", "rl") else f"{n}.plate"
        inst = f"{n}_plate" if n in ("fl", "rl") else n
        x, y = KINGPINS[n]
        o.append("for k in 0..4 {")
        o.append(f'  {n}_bolt = part("../hw/iso4017_m8x20.fg", at: pose(at: ({num(x)} + hx[k], {num(y)} + hy[k], -{num(PL_T)})))')
        o.append(f'  {n}_wsh = part("../hw/iso7089_8.fg", at: pose(at: ({num(x)} + hx[k], {num(y)} + hy[k], 3.02)))')
        o.append(f'  {n}_bnut = part("../hw/iso10511_m8.fg", at: pose(at: ({num(x)} + hx[k], {num(y)} + hy[k], 4.62)))')
        o.append(f"  {n}_fix_bolt = fastened({n}_bolt.head, {plate}.bolt[k])")
        o.append(f"  {n}_fix_wsh = fastened({n}_wsh.a, pan.seat_{n}[k])")
        o.append(f"  {n}_fix_bnut = fastened({n}_bnut.seat, {n}_wsh.b)")
        o.append(f"  {n}_f_head = fit({n}_bolt, {inst})")
        o.append(f"  {n}_f_wsh = fit({n}_wsh, pan)")
        o.append(f"  {n}_f_bnut = fit({n}_bnut, {n}_wsh)")
        o.append("}")
    o += ["", "m8_rule = fit_rule(external_thread, internal_thread, size: m8, min_engagement: m8_min)", ""]
    o.append("# caster contacts: a subassembly's own fits don't reach this file (README, gap G3), so the")
    o.append("# right-hand casters' intended contacts are declared again here, by part file")
    for i, (x, y_) in enumerate(SUB_RULES):
        o.append(f'c_rule{i} = fit_rule("{x}", "{y_}")')
    o.append("")
    E = o.append
    E("# as placed: every drive at 0")
    E("expect interferences = 0")
    E("expect mobility = 0")
    # outside dimensions against Blickle (111 / 80 x 32 / 90 x 66 / 40 offset)
    fx, fy = KINGPINS["fl"]
    E("# Blickle LKPA-PATH 80KF: height 111, wheel 80 x 32, plate 90 x 66, offset 40")
    E("expect z(fl_plate.top) = 0 +- 0.01")
    E(f"expect z(fl_yoke.wheel_bottom) = -{num(AX_Z + WH_D / 2)} +- 0.01")
    E("expect clearance(fl_tread, floor) <= 0.01                   # the tread stands on the floor: 111 overall")
    E("expect interference(fl_tread, floor) <= 0.01")
    E(f"expect z(fl_hub) = -{num(AX_Z)} +- 0.01                       # axle 71 up: the 80 wheel's centre")
    E(f"expect clearance(fl_tread, fl_yoke) = {num(LEG_XI - WH_W / 2)} +- 0.02        # 32 wide between legs 36.40 apart")
    E(f"expect x(fl_plate.edge_x) = {num(fx + PL_W / 2)} +- 0.01              # plate 66 across")
    E(f"expect y(fl_plate.edge_y) = {num(fy + PL_L / 2)} +- 0.01              # plate 90 along Y")
    E(f"expect y(fl_hub) = {num(fy - AX_Y)} +- 0.01                         # offset 40 behind the kingpin")
    E(f"expect x(fl_hub) = {num(fx + AX_Y)} +- 0.01 at fl_swivel_turn = 90deg")
    E(f"expect distance(fl_hub, fl_plate.top) = {sqrt(AX_Y ** 2 + AX_Z ** 2):.4f} +- 0.01")
    for n in ("fr", "rr", "rl"):
        yk = f"{n}_yoke" if n == "rl" else f"{n}.yoke"
        hb = f"{n}_hub" if n == "rl" else f"{n}.hub"
        E(f"expect z({yk}.wheel_bottom) = -{num(AX_Z + WH_D / 2)} +- 0.01")
        E(f"expect y({hb}) = {num(KINGPINS[n][1] - AX_Y)} +- 0.01")
    # swivel sweep. Every `at` line is a full re-solve and overlap pass (~35 s here), so the sweep is one
    # interferences line per angle: the envelope gauges (inner radius 90 - 0.5) and the floor are in it, so
    # it also proves "inside the swivel envelope with 0.5 to spare" and "clear of the pan and the bolts".
    E("")
    E("# swivel sweep: brake off at 0, 90, 180, 270deg, on at 45, 135, 225, 315deg; rolled a little at 135deg")
    for a_ in range(0, 360, 45):
        sw = f"fl_swivel_turn = {a_}deg, rl_swivel_turn = {a_}deg"
        if a_ % 90:
            sw += ", fl_brake = 20deg"
        if a_ == 135:
            sw += ", fl_roll = 33deg, rl_roll = 33deg"
        E(f"expect interferences = 0 at {sw}")
    E("")
    E("# clearances that a rotation about the kingpin can't change: checked once, brake off and on")
    E("expect clearance(fl_yoke, pan) >= 0.5")
    E("expect clearance(fl_ub[0], pan) >= 0.5")
    E("expect clearance(fl_lever, pan) >= 0.5 at fl_brake = 20deg")
    E("expect clearance(fl_lever, env_fl) >= 0.2")
    E("")
    E("# brake states")
    E("expect clearance(fl_lever, fl_tread) >= 1.0                       # off: shoe clear of the tread")
    E("expect clearance(fl_lpin, fl_race) >= 1.0                         # off: pin under the race")
    E("expect clearance(fl_lever, fl_tread) <= 0.01 at fl_brake = 20deg   # on: shoe on the tread (fit f_shoe)")
    E(f"expect z(fl_lpin.tip) = -{num(LP_TOP0 - LP_LIFT)} +- 0.01 at fl_brake = 20deg     # pin top inside the race (-15.03..-17.53)")
    E("expect clearance(fl_lpin, fl_race) >= 0.1 at fl_brake = 20deg     # in a notch, 0.15 a side")
    E("expect interference(fl_lpin, fl_race) > 0 at fl_brake = 20deg, fl_swivel_turn = 7.5deg   # between notches it strikes the race")
    E("")
    E("# rolling elements follow their drives")
    E(f"expect value(fl_up[0].turn) = 22.5deg +- 0.001deg at fl_swivel_turn = 45deg")
    E(f"expect value(fl_cage.turn) = {K_CAGE * 90:.4f}deg +- 0.001deg at fl_roll = 90deg")
    E(f"expect value(rl_lo[7].turn) = 0deg +- 0.001deg")
    (OUT / "test.fg").write_text("\n".join(o) + "\n")


def main():
    for pt_ in parts():
        (OUT / f"{pt_.name}.fg").write_text(pt_.text())
        print(f"{pt_.name:16s} volume {pt_.volume:10.3f}")
    caster_doc(False)
    caster_doc(True)
    bearing_doc()
    test_doc()
    print(f"pedal: cheek clears the tread by {CHEEK_GAP0:.2f} at brake 0; cam gap max {max(CAM_GAPS):.3f} mid-stroke; "
          f"pad corner r {PAD_CORNER_R:.2f}; pin ({XP:.3f}, {YP:.3f}); r_cam {R_CAM:.4f}")


if __name__ == "__main__":
    main()
