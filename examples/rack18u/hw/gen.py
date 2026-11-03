#!/usr/bin/env python3
"""Writes the hardware library's .fg part files from the dimension tables below.

Forge has no per-instance params (part(FILE) takes no overrides), so every size is its own file;
this script is the one place the numbers live. Run it from anywhere: python3 hardware/gen.py
Each part's expected volume is worked out here from the drawing dimensions, independently of Forge.
"""
from math import pi, sqrt, asin
from pathlib import Path

OUT = Path(__file__).resolve().parent

# ISO 965 basic minor diameter of the internal thread, D1 = D - 1.0825 P
def d1(d, p):
    return d - 1.082532 * p

def num(v):
    s = f"{v:.6f}".rstrip("0").rstrip(".")
    return s if s not in ("-0", "") else "0"


class Part:
    def __init__(self, name, title, source, frame):
        self.name, self.title, self.source, self.frame = name, title, source, frame
        self.params = []      # (name, expr, comment)
        self.prof = []        # rows of the xz profile sketch
        self.body = []        # statements after the sketch
        self.anchors = []     # (name, zname, kind, size, depth, comment)
        self.zpts = {}
        self.volume = None
        self.anchor_doc = []

    def p(self, name, value, comment=""):
        self.params.append((name, value if isinstance(value, str) else num(value), comment))

    def zpt(self, zexpr):
        if zexpr not in self.zpts:
            self.zpts[zexpr] = f"pz{len(self.zpts)}"
        return self.zpts[zexpr]

    def anchor(self, name, zexpr, doc, kind=None, size=None, depth=None):
        self.anchors.append((name, self.zpt(zexpr), kind, size, depth))
        self.anchor_doc.append((name, doc))

    def text(self):
        out = [f"# {self.title}", f"# source: {self.source}", f"# frame: {self.frame}", ""]
        for n, v, c in self.params:
            out.append(f"shared {n} = {v}" + (f"   # {c}" if c else ""))
        out.append("")
        out.append("sketch prof(on: xz) {")
        out += ["  " + r for r in self.prof]
        out.append("  c0 = pt(0, 0)")
        for z, n in self.zpts.items():
            out.append(f"  {n} = pt(0, {z})")
        out.append("  zax = line((0, 0), (0, zref), construction: true)")
        out.append("  xax = line((0, 0), (xref, 0), construction: true)")
        out.append("}")
        out += self.body
        out.append("")
        for n, z, kind, size, depth in self.anchors:
            a = f"{n} = anchor(plane(at: prof.{z}, normal: prof.zax.dir, x: prof.xax.dir)"
            if kind:
                a += f", kind: {kind}, size: {size}, depth: {depth}"
            out.append(a + ")")
        out.append("")
        out.append("expect valid")
        out.append("expect bodies = 1")
        out.append(f"expect volume = {num(round(self.volume, 3))} +- 0.05%")
        return "\n".join(out) + "\n"

    def write(self):
        (OUT / f"{self.name}.fg").write_text(self.text())


def poly_rows(name, pts):
    return [f"{name} = poly([" + ", ".join(f"({x}, {y})" for x, y in pts) + "])"]


def ring_volume(r_in, r_out, h):
    return pi * (r_out**2 - r_in**2) * h


def cone_frustum(r1, r2, h):
    return pi * h * (r1 * r1 + r1 * r2 + r2 * r2) / 3


def hex_area(s):
    return sqrt(3) / 2 * s * s


def band_area(r, w):
    # area of a straight band of width w through the centre of a circle of radius r
    h = w / 2
    return 2 * (h * sqrt(r * r - h * h) + r * r * asin(h / r))


def common_screw_params(pt, d, P, l):
    pt.p("d", d, "thread major diameter")
    pt.p("P", P, "pitch")
    pt.p("l", l, "nominal length, under the head to the tip")
    pt.p("ch", "0.6 * P", "tip chamfer (ISO 4753 chamfered end, to about the minor diameter)")
    pt.p("rs", "d / 2")
    pt.p("zref", "l")
    pt.p("xref", "rs")


def screw_anchors(pt, thread_start="0", thread_len="l", size="d"):
    pt.anchor("head", "0", "head underside (the seat), normal along the shank towards the tip")
    pt.anchor("thd", thread_start, "external thread, from where the full thread starts towards the tip",
              "external_thread", size, thread_len)
    pt.anchor("tip", "l", "the end of the shank, normal outwards")


def tip_volume(d, P, l, l0=0.0):
    rs, ch = d / 2, 0.6 * P
    return pi * rs * rs * (l - l0 - ch) + cone_frustum(rs, rs - ch, ch)


def button(name, d, P, l, dk, k, R, s, t, src):
    pt = Part(name, f"ISO 7380-1 hexagon socket button head screw M{num(d)} x {num(l)}", src,
              "origin on the axis at the head underside; +z along the shank to the tip; head at z < 0")
    common_screw_params(pt, d, P, l)
    pt.p("dk", dk, "head diameter")
    pt.p("k", k, "head height")
    pt.p("R", R, "dome radius r1, the middle of the standard's range")
    pt.p("s", s, "hexagon socket across flats")
    pt.p("t", t, "socket depth, minimum")
    pt.p("rk", "dk / 2")
    pt.p("yc", "R - k", "the dome centre, on the axis")
    pt.p("ye", "yc - sqrt(R * R - rk * rk)", "where the dome meets the head's rim")
    pt.p("rc", "s / sqrt(3)", "socket hexagon corner radius")
    pt.p("n6", "count(6)")
    pt.prof += [
        "a = pt(0, -k)", "b = pt(rk, ye)", "c = pt(rk, 0)", "e = pt(rs, 0)", "f = pt(rs, l - ch)",
        "g = pt(rs - ch, l)", "h = pt(0, l)", "o = pt(0, yc)",
        "cap = arc(center: o, from: b, to: a, cw: true)", "rim = line(b, c)", "under = line(c, e)",
        "shank = line(e, f)", "tipc = line(f, g)", "tipf = line(g, h)", "axl = line(h, a)",
    ]
    pt.body += [
        "body = revolve(prof, axis: prof.zax)",
        "sketch hx(on: offset(body.side(prof.under), -k), at: prof.c0) {",
        "  hexo = poly(polar(n: n6, r: rc, start: 0deg))",
        "}",
        "socket = cut(hx, -t)",
    ]
    screw_anchors(pt)
    rk, yc = dk / 2, R - k
    ye = yc - sqrt(R * R - rk * rk)
    hc = R - sqrt(R * R - rk * rk)
    vol_cut = cut_under_dome(in_hex(s), s / sqrt(3), t, k, R)
    pt.volume = pi * hc * hc * (3 * R - hc) / 3 + pi * rk * rk * (-ye) + tip_volume(d, P, l) - vol_cut
    return pt


def pan(name, d, P, l, dk, k, rf, m, w, tr, src):
    pt = Part(name, f"ISO 7045 cross recessed pan head screw M{num(d)} x {num(l)} (fan screw)", src,
              "origin on the axis at the head underside; +z along the shank to the tip; head at z < 0")
    common_screw_params(pt, d, P, l)
    pt.p("dk", dk, "head diameter")
    pt.p("k", k, "head height")
    pt.p("rf", rf, "head top radius")
    pt.p("m", m, "recess diameter at the top (type H, No. 2)")
    pt.p("w", w, "recess wing width (modelled: a plain cross, not the Phillips form)")
    pt.p("tr", tr, "recess depth (modelled)")
    pt.p("rk", "dk / 2")
    pt.p("yc", "rf - k")
    pt.p("ye", "yc - sqrt(rf * rf - rk * rk)")
    pt.p("hm", "m / 2")
    pt.p("hw", "w / 2")
    pt.prof += [
        "a = pt(0, -k)", "b = pt(rk, ye)", "c = pt(rk, 0)", "e = pt(rs, 0)", "f = pt(rs, l - ch)",
        "g = pt(rs - ch, l)", "h = pt(0, l)", "o = pt(0, yc)",
        "cap = arc(center: o, from: b, to: a, cw: true)", "rim = line(b, c)", "under = line(c, e)",
        "shank = line(e, f)", "tipc = line(f, g)", "tipf = line(g, h)", "axl = line(h, a)",
    ]
    pt.body += [
        "body = revolve(prof, axis: prof.zax)",
        "sketch cr(on: offset(body.side(prof.under), -k), at: prof.c0) {",
        "  x = poly([(hm, -hw), (hm, hw), (hw, hw), (hw, hm), (-hw, hm), (-hw, hw), (-hm, hw), (-hm, -hw), "
        "(-hw, -hw), (-hw, -hm), (hw, -hm), (hw, -hw)])",
        "}",
        "recess = cut(cr, -tr)",
    ]
    screw_anchors(pt)
    rk, yc = dk / 2, rf - k
    ye = yc - sqrt(rf * rf - rk * rk)
    hc = rf - sqrt(rf * rf - rk * rk)
    vol_cut = cut_under_dome(in_cross(m, w), m / 2, tr, k, rf)
    pt.volume = pi * hc * hc * (3 * rf - hc) / 3 + pi * rk * rk * (-ye) + tip_volume(d, P, l) - vol_cut
    return pt


def cut_under_dome(inside, half, depth, k, R, n=600):
    # volume a prism cut (floor at depth below the head top) removes from a dome of radius R:
    # midpoint integral of (floor - dome surface) over the outline, where positive
    tot = 0.0
    dx = 2 * half / n
    for i in range(n):
        x = -half + (i + 0.5) * dx
        for j in range(n):
            y = -half + (j + 0.5) * dx
            if inside(x, y):
                top = R - sqrt(R * R - x * x - y * y)
                tot += max(0.0, depth - top)
    return tot * dx * dx


def in_cross(m, w):
    hm, hw = m / 2, w / 2
    return lambda x, y: (abs(x) <= hw and abs(y) <= hm) or (abs(y) <= hw and abs(x) <= hm)


def in_hex(s):
    # corners on the x axis (polar start 0deg): flats face 30, 90, 150 degrees
    c = sqrt(3) / 2
    return lambda x, y: abs(y) <= s / 2 and abs(c * x + 0.5 * y) <= s / 2 and abs(c * x - 0.5 * y) <= s / 2


def cheese(name, d, P, l, dk, k, n, t, src):
    pt = Part(name, f"rack screw M{num(d)} x {num(l)}, slotted cheese head (Rittal DK 7094.110; head per ISO 1207)",
              src, "origin on the axis at the head underside; +z along the shank to the tip; head at z < 0")
    common_screw_params(pt, d, P, l)
    pt.p("dk", dk, "head diameter")
    pt.p("k", k, "head height")
    pt.p("n", n, "slot width")
    pt.p("t", t, "slot depth, minimum")
    pt.p("rk", "dk / 2")
    pt.prof += poly_rows("o", [("0", "-k"), ("rk", "-k"), ("rk", "0"), ("rs", "0"), ("rs", "l - ch"),
                               ("rs - ch", "l"), ("0", "l")])
    pt.body += [
        "body = revolve(prof.o, axis: prof.zax)",
        "sketch sl(on: body.side(prof.o[0]), at: prof.c0) {",
        "  r = rect(center: (0, 0), size: (dk, n))",
        "}",
        "slotcut = cut(sl, t)",
    ]
    screw_anchors(pt)
    pt.volume = pi * (dk / 2) ** 2 * k + tip_volume(d, P, l) - band_area(dk / 2, n) * t
    return pt


def hexbolt(name, std, d, P, l, b, s, k, src):
    full = b is None
    pt = Part(name, f"{std} hexagon head {'screw' if full else 'bolt'} M{num(d)} x {num(l)}", src,
              "origin on the axis at the head underside; +z along the shank to the tip; head at z < 0")
    common_screw_params(pt, d, P, l)
    pt.p("s", s, "across flats")
    pt.p("k", k, "head height")
    pt.p("rc", "s / sqrt(3)", "hexagon corner radius")
    pt.p("n6", "count(6)")
    if not full:
        pt.p("b", b, "thread length (ISO 4014: 2d + 6 for l <= 125)")
    pt.prof += poly_rows("o", [("0", "0"), ("rs", "0"), ("rs", "l - ch"), ("rs - ch", "l"), ("0", "l")])
    pt.body += [
        "shank = revolve(prof.o, axis: prof.zax)",
        "sketch hx(on: shank.side(prof.o[0]), at: prof.c0) {",
        "  hexo = poly(polar(n: n6, r: rc, start: 0deg))",
        "}",
        "hexhead = extrude(hx, k)",
    ]
    if full:
        screw_anchors(pt)
    else:
        screw_anchors(pt, thread_start="l - b", thread_len="b")
    pt.volume = hex_area(s) * k + tip_volume(d, P, l)
    return pt


def shoulder(name, ds, ls, d, P, lt, dk, k, s, t, c, src):
    pt = Part(name, f"ISO 7379 hexagon socket shoulder screw {num(ds)} x {num(ls)}, thread M{num(d)} x {num(lt)}", src,
              "origin on the axis at the head underside; +z along the shoulder to the tip; head at z < 0")
    pt.p("ds", ds, "shoulder diameter (f9)")
    pt.p("ls", ls, "shoulder length")
    pt.p("d", d, "thread major diameter")
    pt.p("P", P, "pitch")
    pt.p("lt", lt, "thread length")
    pt.p("dk", dk, "head diameter")
    pt.p("k", k, "head height")
    pt.p("s", s, "hexagon socket across flats")
    pt.p("t", t, "socket depth (modelled; not on the supplier sheets read)")
    pt.p("c", c, "head top edge chamfer (modelled)")
    pt.p("ch", "0.6 * P", "tip chamfer")
    pt.p("l", "ls + lt", "head underside to the tip")
    pt.p("rsh", "ds / 2")
    pt.p("rs", "d / 2")
    pt.p("rk", "dk / 2")
    pt.p("rc", "s / sqrt(3)")
    pt.p("n6", "count(6)")
    pt.p("zref", "l")
    pt.p("xref", "rs")
    pt.prof += poly_rows("o", [("0", "-k"), ("rk - c", "-k"), ("rk", "c - k"), ("rk", "0"), ("rsh", "0"),
                               ("rsh", "ls"), ("rs", "ls"), ("rs", "l - ch"), ("rs - ch", "l"), ("0", "l")])
    pt.body += [
        "body = revolve(prof.o, axis: prof.zax)",
        "sketch hx(on: body.side(prof.o[0]), at: prof.c0) {",
        "  hexo = poly(polar(n: n6, r: rc, start: 0deg))",
        "}",
        "socket = cut(hx, t)",
    ]
    pt.anchor("head", "0", "head underside, normal along the shoulder towards the tip")
    pt.anchor("shoulder", "0", "the shoulder (a pin), from the head underside along +z", "shaft", "ds", "ls")
    pt.anchor("clamp", "ls", "the shoulder's end face, where it clamps on the nut side part")
    pt.anchor("thd", "ls", "external thread, from the shoulder end to the tip", "external_thread", "d", "lt")
    pt.anchor("tip", "l", "the end of the thread")
    rk = dk / 2
    head = pi * rk * rk * (k - c) + cone_frustum(rk, rk - c, c)
    pt.volume = head + pi * (ds / 2) ** 2 * ls + tip_volume(d, P, lt) - hex_area(s) * t
    return pt


def pem_nut(name, code, t_sheet, d, P, A, C, E, T, c, src):
    pt = Part(name, f"PEM {code} self-clinching nut, as installed in a {num(t_sheet)} sheet", src,
              "origin on the axis in the sheet's flush face (the face the screw enters); +z through the sheet "
              "to the nut head; the shank's end is recessed t - A below the flush face")
    pt.p("tsh", t_sheet, "sheet thickness this file is for")
    pt.p("d", d, "thread")
    pt.p("P", P, "pitch")
    pt.p("A", A, "shank length, max (bulletin CL, shank code 0)")
    pt.p("C", C, "shank diameter, max")
    pt.p("E", E, "head diameter")
    pt.p("T", T, "head height above the sheet")
    pt.p("c", c, "head top edge chamfer (modelled)")
    pt.p("rm", "(d - 1.082532 * P) / 2", "thread bore: half the ISO 965 minor diameter D1 = d - 1.0825 P")
    pt.p("z0", "tsh - A", "where the shank (and the thread) starts")
    pt.p("rcs", "C / 2")
    pt.p("re", "E / 2")
    pt.p("zref", "tsh + T")
    pt.p("xref", "re")
    pt.prof += poly_rows("o", [("rm", "z0"), ("rcs", "z0"), ("rcs", "tsh"), ("re", "tsh"), ("re", "tsh + T - c"),
                               ("re - c", "tsh + T"), ("rm", "tsh + T")])
    pt.body += ["body = revolve(prof.o, axis: prof.zax)"]
    pt.anchor("flush", "0", "the sheet's flush face: place on the sheet part's PEM anchor (same origin, same normal)")
    pt.anchor("thd", "z0", "internal thread, through the shank and head: A + T", "internal_thread", "d", "A + T")
    pt.anchor("end", "tsh + T", "the head's top face (where a screw tip protrudes)")
    rm = d1(d, P) / 2
    pt.volume = (pi * ((C / 2) ** 2 - rm * rm) * A + pi * ((E / 2) ** 2 - rm * rm) * (T - c)
                 + cone_frustum(E / 2, E / 2 - c, c) - pi * rm * rm * c)
    return pt


def pem_stud(name, code, d, P, L, H, hh, S, src):
    pt = Part(name, f"PEM {code} flush-head self-clinching stud", src,
              "origin on the axis in the head's top face, which is flush with the sheet's outer face; "
              "+z along the stud")
    pt.p("d", d, "thread")
    pt.p("P", P, "pitch")
    pt.p("l", L, "length from the flush face")
    pt.p("H", H, "head diameter")
    pt.p("hh", hh, "head thickness (modelled; the bulletin gives no value)")
    pt.p("S", S, "clinch zone: threads gaugeable from 2 pitches past S max")
    pt.p("ch", "0.6 * P")
    pt.p("rs", "d / 2")
    pt.p("rh", "H / 2")
    pt.p("zref", "l")
    pt.p("xref", "rh")
    pt.prof += poly_rows("o", [("0", "0"), ("rh", "0"), ("rh", "hh"), ("rs", "hh"), ("rs", "l - ch"),
                               ("rs - ch", "l"), ("0", "l")])
    pt.body += ["body = revolve(prof.o, axis: prof.zax)"]
    pt.anchor("flush", "0", "head top, flush with the sheet's outer face: place on the sheet part's stud anchor")
    pt.anchor("thd", "S + 2 * P", "external thread, full from S + 2P to the end", "external_thread", "d", "l - S - 2 * P")
    pt.anchor("tip", "l", "the stud's end")
    pt.volume = pi * (H / 2) ** 2 * hh + pi * (d / 2) ** 2 * (L - hh - 0.6 * P) + cone_frustum(d / 2, d / 2 - 0.6 * P, 0.6 * P)
    return pt


def nyloc(name, d, P, s, h, m, dw, src):
    pt = Part(name, f"ISO 10511 prevailing torque type hexagon thin nut with non-metallic insert M{num(d)}", src,
              "origin on the axis in the bearing face; +z through the nut towards the insert crown")
    pt.p("d", d, "thread")
    pt.p("P", P, "pitch")
    pt.p("s", s, "across flats")
    pt.p("h", h, "overall height")
    pt.p("m", m, "hexagon height (modelled at the table's m min: the metal thread)")
    pt.p("dw", dw, "insert crown diameter (modelled at dw min)")
    pt.p("rm", "(d - 1.082532 * P) / 2", "bore: half the ISO 965 minor diameter D1 = d - 1.0825 P")
    pt.p("rw", "dw / 2")
    pt.p("rc", "s / sqrt(3)")
    pt.p("n6", "count(6)")
    pt.p("zref", "h")
    pt.p("xref", "rw")
    pt.prof += poly_rows("o", [("rm", "m"), ("rw", "m"), ("rw", "h"), ("rm", "h")])
    pt.body += [
        "crown = revolve(prof.o, axis: prof.zax)",
        "sketch hx(on: crown.side(prof.o[0]), at: prof.c0) {",
        "  hexo = poly(polar(n: n6, r: rc, start: 0deg))",
        "  bo = circle((0, 0), rm)",
        "}",
        "hexa = extrude(hx, m)",
    ]
    pt.anchor("seat", "0", "bearing face (on the washer), normal through the nut")
    pt.anchor("thd", "0", "internal thread, the whole height (the screw must pass the insert)",
              "internal_thread", "d", "h")
    pt.anchor("end", "h", "top of the insert crown")
    rm = d1(d, P) / 2
    pt.volume = (hex_area(s) - pi * rm * rm) * m + pi * ((dw / 2) ** 2 - rm * rm) * (h - m)
    return pt


def ring(name, title, src, di, do, h, bore=None, frame=None):
    pt = Part(name, title, src, frame or "origin on the axis in the bottom face; +z through the part")
    pt.p("di", di, "inner diameter")
    pt.p("do", do, "outer diameter")
    pt.p("h", h, "thickness")
    pt.p("ri", "di / 2")
    pt.p("ro", "do / 2")
    pt.p("zref", "h")
    pt.p("xref", "ro")
    if bore:
        pt.p("nb", bore, "nominal bore size for fit rules")
    pt.prof += poly_rows("o", [("ri", "0"), ("ro", "0"), ("ro", "h"), ("ri", "h")])
    pt.body += ["body = revolve(prof.o, axis: prof.zax)"]
    pt.anchor("a", "0", "bottom face, normal +z")
    pt.anchor("b", "h", "top face, normal +z")
    if bore:
        pt.anchor("bore", "0", "the bore, along +z", "bore", "nb", "h")
    pt.volume = ring_volume(di / 2, do / 2, h)
    return pt


def cage_nut(name, d, P, wx, wy, depth, tn, dc, src):
    pt = Part(name, f"M{num(d)} cage nut for 9.5 square holes (Rittal DK 2094.200), envelope", src,
              "origin on the axis in the rail's rear face (the cage's front face); +z behind the rail, "
              "the direction a screw from the front advances; x across the rack (the 11.81 side)")
    pt.p("d", d, "thread")
    pt.p("P", P, "pitch")
    pt.p("wx", wx, "cage width, across the rack (Panduit CNWSM6-C .465)")
    pt.p("wy", wy, "cage height, along the rail (Panduit .398)")
    pt.p("dp", depth, "cage depth behind the rail (Panduit .316, read as depth per SPEC 5.11)")
    pt.p("tn", tn, "threaded length of the nut (Panduit .188), at the back of the cage")
    pt.p("dc", dc, "clearance bore in front of the nut")
    pt.p("rm", "(d - 1.082532 * P) / 2", "thread bore: half the ISO 965 minor diameter D1 = d - 1.0825 P")
    pt.p("zn", "dp - tn", "where the thread starts")
    pt.p("hx", "wx / 2")
    pt.p("rcl", "dc / 2")
    pt.p("zref", "dp")
    pt.p("xref", "hx")
    pt.prof += poly_rows("o", [("-hx", "0"), ("hx", "0"), ("hx", "dp"), ("-hx", "dp")])
    pt.body += [
        "block = extrude(prof.o, sym(wy))",
        "sketch fb(on: block.side(prof.o[0]), at: prof.c0) {",
        "  c = circle((0, 0), rcl)",
        "}",
        "front = cut(fb, zn)",
        "sketch tb(on: block.side(prof.o[2]), at: prof.c0) {",
        "  c = circle((0, 0), rm)",
        "}",
        "tcut = cut(tb, tn)",
    ]
    pt.anchor("seat", "0", "the cage's front face on the rail's rear face, centred on the square hole")
    pt.anchor("thd", "zn", "internal thread, tn long", "internal_thread", "d", "tn")
    pt.anchor("end", "dp", "the back of the cage")
    rm = d1(d, P) / 2
    pt.volume = wx * wy * depth - pi * (dc / 2) ** 2 * (depth - tn) - pi * rm * rm * tn
    return pt


def spring(name, H, d, D, n, src):
    pt = Part(name, f"latch compression spring at height {num(H)} (representative: wire {num(d)}, "
              f"mean diameter {num(D)}, {num(n)} turns)", src,
              "origin on the axis in the seat plane z = 0; the coil runs to z = H")
    pt.p("H", H, "working height")
    pt.p("dw", d, "wire diameter")
    pt.p("D", D, "mean coil diameter")
    pt.p("nt", f"count({num(n)})", "turns, modelled as a plain helix over the height (no closed ends)")
    pt.p("pitch", "(H - dw) / nt")
    pt.p("rm", "D / 2")
    pt.p("rw", "dw / 2")
    pt.p("zref", "H")
    pt.p("xref", "rm")
    pt.prof += ["w = circle((rm, rw), rw)", "cax = line((0, 0), (0, H), construction: true)"]
    pt.body += ["coilbody = coil(prof.w, axis: prof.cax, pitch: pitch, turns: nt)"]
    pt.anchor("a", "0", "seat at z = 0 (on the handle's inner face)")
    pt.anchor("b", "H", "seat at z = H (on the door's outer face)")
    # coil() winds the section that lies in the axial plane: Pappus over the turns, pitch-free
    pt.volume = pi * (d / 2) ** 2 * n * pi * D
    return pt


def steel_ball(name, dball, src):
    pt = Part(name, f"steel ball {num(dball)} (ISO 3290 G100)", src, "origin at the ball's centre")
    pt.p("db", dball, "ball diameter")
    pt.p("rb", "db / 2")
    pt.p("zref", "rb")
    pt.p("xref", "rb")
    pt.body += ["shot = ball(prof.c0, rb)"]
    pt.anchor("c", "0", "the centre, normal +z")
    pt.volume = 4 / 3 * pi * (dball / 2) ** 3
    return pt


FE = "https://www.fasteners.eu/standards/ISO/"
PEMCL = "PEM bulletin CL (https://damlite.pemnet.com/CLS-M6-1_pdf_2.pdf), S/SS/CLS table, shank code 0"
SPRING = ("SPEC 5.8 (ID >= 8.4, OD <= 13, free ~12, solid <= 5.0, ~8 N/mm); no supplier row yet: wire 1.2, "
          "mean 10.8 (ID 9.6, OD 12.0), 4 turns, solid 4.8, rate G d^4 / (8 D^3 na) = 8.1 N/mm with 2 active")
PEMFH = "PEM bulletin FH (https://damlite.pemnet.com/FH-M5-12ZC_pdf_1.pdf), FH metric table"

PARTS = [
    button("iso7380_m5x10", 5, 0.8, 10, 9.5, 2.75, 5.5, 3, 1.56, FE + "7380/ (ISO 7380-1 table)"),
    button("iso7380_m5x12", 5, 0.8, 12, 9.5, 2.75, 5.5, 3, 1.56, FE + "7380/ (ISO 7380-1 table)"),
    button("iso7380_m4x25", 4, 0.7, 25, 7.6, 2.2, 4.4, 2.5, 1.3, FE + "7380/ (ISO 7380-1 table)"),
    button("iso7380_m6x10", 6, 1.0, 10, 10.5, 3.3, 5.9, 4, 2.08, FE + "7380/ (ISO 7380-1 table)"),
    button("iso7380_m6x12", 6, 1.0, 12, 10.5, 3.3, 5.9, 4, 2.08, FE + "7380/ (ISO 7380-1 table)"),
    hexbolt("iso4017_m8x20", "ISO 4017", 8, 1.25, 20, None, 13, 5.3, FE + "4017/"),
    hexbolt("iso4014_m8x55", "ISO 4014", 8, 1.25, 55, 22, 13, 5.3, FE + "4017/ (head) and ISO 4014 b = 2d + 6"),
    shoulder("iso7379_8x12_m6", 8, 12, 6, 1.0, 11, 13, 5.5, 4, 3.0, 0.5,
             FE + "7379/ (ds 8, dk 13, k 5.5, s 4, b 11); Ganter ISO 7379-8-M6 sheets (l2 11)"),
    shoulder("iso7379_8x16_m6", 8, 16, 6, 1.0, 11, 13, 5.5, 4, 3.0, 0.5,
             FE + "7379/ (ds 8, dk 13, k 5.5, s 4, b 11); Ganter ISO 7379-8-M6 sheets (l2 11)"),
    pan("iso7045_m4x40", 4, 0.7, 40, 8, 3.1, 8, 4.6, 1.0, 1.9, FE + "7045/ (dk 8, k 3.1, rf 8, m 4.6)"),
    pan("iso7045_m4x35", 4, 0.7, 35, 8, 3.1, 8, 4.6, 1.0, 1.9,
        FE + "7045/ (dk 8, k 3.1, rf 8, m 4.6); superseded by M4 x 40 in SPEC rev B R2, kept until removed"),
    cheese("rack_screw_m6x16", 6, 1.0, 16, 10, 3.9, 1.6, 1.6,
           "Rittal DK 7094.110 (M6, 16 thread length, cheese head); head per ISO 1207 M6 (dk 10, k 3.9, n 1.6, t 1.6)"),
    nyloc("iso10511_m4", 4, 0.7, 7, 5.0, 2.9, 5.9, FE + "10511/ (table per DIN 985: s 7, h 5, m 2.9, dw 5.9)"),
    nyloc("iso10511_m5", 5, 0.8, 8, 5.0, 3.2, 6.9, FE + "10511/ (table per DIN 985: s 8, h 5, m 3.2, dw 6.9)"),
    nyloc("iso10511_m6", 6, 1.0, 10, 6.0, 4.0, 8.9, FE + "10511/ (s 10, h 6, m 4, dw 8.9)"),
    nyloc("iso10511_m8", 8, 1.25, 13, 8.0, 5.5, 11.6, FE + "10511/ (s 13, h 8, m 5.5, dw 11.6)"),
    ring("iso7089_4", "ISO 7089 plain washer, size 4", FE + "7089/ (d1 4.3, d2 9, h 0.8)", 4.3, 9, 0.8),
    ring("iso7089_5", "ISO 7089 plain washer, size 5", FE + "7089/ (d1 5.3, d2 10, h 1.0)", 5.3, 10, 1.0),
    ring("iso7089_6", "ISO 7089 plain washer, size 6", FE + "7089/ (d1 6.4, d2 12, h 1.6)", 6.4, 12, 1.6),
    ring("iso7089_8", "ISO 7089 plain washer, size 8", FE + "7089/ (d1 8.4, d2 16, h 1.6)", 8.4, 16, 1.6),
    ring("nylon_spacer_8x14x8", "PA66 spacer 8.2 ID x 14 OD x 8.0 (hinge pins and fan tray hangers)",
         "SPEC section 7 (8.2-8.4 ID x 14 x 8); 8.2 taken so it also bears on the 8.0 shoulder", 8.2, 14, 8.0, bore=8),
    ring("plastic_washer_m6", "plastic washer for the M6 rack screw (supplied with Rittal DK 7094.110)",
         "SPEC 5.11 (1.0 thick, modelled); 6.4 x 12 assumed: Rittal publishes no size", 6.4, 12, 1.0),
    pem_nut("pem_s_m4_0_t2", "S-M4-0", 1.88, 4, 0.7, 0.77, 5.38, 7.87, 2.0, 0.3, PEMCL),
    pem_nut("pem_ss_m5_0_t3", "SS-M5-0", 3.02, 5, 0.8, 0.77, 6.33, 8.64, 2.0, 0.3, PEMCL),
    pem_nut("pem_s_m6_0_t3", "S-M6-0", 3.02, 6, 1.0, 1.15, 8.73, 11.18, 4.08, 0.4, PEMCL),
    pem_nut("pem_s_m6_0_t2", "S-M6-0", 1.88, 6, 1.0, 1.15, 8.73, 11.18, 4.08, 0.4, PEMCL),
    pem_nut("pem_s_m6_0_t1", "S-M6-0", 1.52, 6, 1.0, 1.15, 8.73, 11.18, 4.08, 0.4, PEMCL),
    pem_stud("pem_fh_m4_12", "FH-M4-12", 4, 0.7, 12, 5.9, 0.8, 2.4, PEMFH),
    pem_stud("pem_fh_m6_20", "FH-M6-20", 6, 1.0, 20, 8.2, 1.0, 3.0, PEMFH),
    pem_stud("pem_fh_m8_25", "FH-M8-25", 8, 1.25, 25, 9.6, 1.2, 3.7, PEMFH),
    ring("latch_spacer_6x15", "steel round spacer 6.0 OD x 4.3 ID x 15.0 (latch link posts)",
         "SPEC 5.8 / section 7 (spec only, no part number)", 4.3, 6.0, 15.0),
    steel_ball("ball_4", 4.0, "SPEC 5.8 / section 7: 4.0 steel ball, ISO 3290 G100"),
    spring("latch_spring_free", 12.0, 1.2, 10.8, 4, SPRING),
    spring("latch_spring_h8_17", 8.17, 1.2, 10.8, 4, SPRING),
    spring("latch_spring_h7_77", 7.77, 1.2, 10.8, 4, SPRING),
    spring("latch_spring_h7_49", 7.49, 1.2, 10.8, 4, SPRING),
    cage_nut("cage_nut_m6", 6, 1.0, 11.81, 10.11, 8.03, 4.78, 6.6,
             "Rittal DK 2094.200 (M6, 0.8-2.5 metal); envelope from Panduit CNWSM6-C drawing 10119-20"),
]


SHOP_HEAD = """# SendCutSend shop, cold rolled 1008: SPEC section 3, copied verbatim by hw/gen.py (do not edit here).
# Sheet parts copy this text until Forge has imports (SPEC section 10, gap 12). The probe sheet at
# the end only proves the text parses and builds: one t2 plate with one of each hardware kind.
"""

SHOP_PROBE = """
shared probe_w = 80
shared probe_h = 40
shared probe_x = 12
shared probe_y = 20
sketch plan(on: xy) {
  outline = rect(corner: (0, 0), size: (probe_w, probe_h))
  n4 = pt(probe_x, probe_y)
  n6 = pt(probe_w / 2, probe_y)
  s4 = pt(probe_w - probe_x, probe_y)
}
probe = sheet(plan, t: t2, shop: scs)
nut4 = pem("nut", "M4", at: plan.n4, wall: probe)
nut6 = pem("nut", "M6", at: plan.n6, wall: probe)
stud4 = pem("stud", "M4", at: plan.s4, wall: probe)
expect valid
"""


def write_shop():
    spec = (OUT.parent / "SPEC.md").read_text().splitlines()
    a = next(i for i, l in enumerate(spec) if l.startswith("shared t3 = "))
    b = next(i for i in range(a, len(spec)) if "kerf: kerf_scs, hardware: hw_scs)" in spec[i])
    (OUT / "shop.fg").write_text(SHOP_HEAD + "\n" + "\n".join(spec[a:b + 1]) + "\n" + SHOP_PROBE)


if __name__ == "__main__":
    write_shop()
    for p in PARTS:
        p.write()
        print(f"{p.name:24s} volume {p.volume:10.3f}")
