#!/usr/bin/env python3
"""Generates kernel/forge-machine/data/machines/haas.toml and its parts under
kernel/forge-machine/parts/haas/<machine>/ from the restated spec facts in
research/cam/machines/haas/ (docs/cam-machine.md: format and Provenance).

Every value written is cited: `published` with the page it is restated from, `estimate` with its
basis, `unknown` with which way the stand-in errs, `conflict` with what disagrees. Nothing is filled
from memory. Re-run after editing the facts or the rules below:

    python3 tools/machines/haas.py
"""
import json
import math
import os
import re

ROOT = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))
FACTS = os.path.join(ROOT, "research/cam/machines/haas/machinetoolindex-2026-11-23.json")
OUT = os.path.join(ROOT, "kernel/forge-machine/data/machines/haas.toml")
PARTS = os.path.join(ROOT, "kernel/forge-machine/parts/haas")
TAKEN = "2026-11-23"

# Secondary sources (COORDINATOR decision 2026-11-23: haascnc.com is behind a bot challenge; use
# reputable secondary sources, tag every value, record conflicts).
HFO_MILL = (
    "https://microfluidics.cnsi.ucsb.edu/wiki/lib/exe/fetch.php?media=tools%3Ahaasminimill%3A"
    "haas_mill_programming_manual.pdf"
)  # Haas Factory Outlet (Productivity Inc) Mill Programming training manual, rev 04-2014
UK_UMC750 = "https://www.haas.co.uk/machines/umc-750/"  # Haas Automation UK (Norwich) spec table
DS_UMC750SS = "https://mtscnc.com/haas/images/pdf/DS_UMC750SS_US.pdf"  # Haas data sheet, dealer-hosted


# ---------------------------------------------------------------------------------------------
# reading the restated facts


def metric(s):
    """The metric number in a fact like '30 in (762 mm)' or '1,000 ipm (25.4 m/min)'."""
    m = re.search(r"\(\s*(?:±\s*)?([\d,.]+)\s*([a-zA-Z/·]+)", s)
    if not m:
        m = re.match(r"\s*(?:±\s*)?([\d,.]+)\s*([a-zA-Z/·°]+)", s)
    v = float(m.group(1).replace(",", ""))
    unit = m.group(2)
    return v, unit


def mm(s):
    v, u = metric(s)
    factor = {"mm": 1.0, "cm": 10.0, "m": 1000.0, "in": 25.4}
    assert u in factor, s
    return v * factor[u]


def mm_per_min(s):
    v, u = metric(s)
    factor = {"m/min": 1000.0, "ipm": 25.4}
    assert u in factor, s
    return v * factor[u]


def kw(s):
    m = re.search(r"\(([\d.]+)\s*kW\)", s)
    return float(m.group(1))


def nm_at(s):
    m = re.search(r"\(([\d.,]+)\s*N·m\s*@\s*([\d,]+)\s*rpm\)", s)
    return float(m.group(1).replace(",", "")), float(m.group(2).replace(",", ""))


def rpm(s):
    return float(re.match(r"([\d,]+)", s).group(1).replace(",", ""))


def kg(s):
    return float(re.search(r"\(([\d.]+)\s*kg\)", s).group(1))


def secs(s):
    return float(re.match(r"([\d.]+)\s*s", s).group(1))


def count(s):
    return int(re.match(r"(\d+)", s).group(1))


class Rec:
    """One machine's TOML text and cites."""

    def __init__(self, name, facts):
        self.name = name
        self.facts = facts
        self.url = facts["url"]
        self.lines = []
        self.cites = []
        self.bodies = []  # (name, link, prims, ignore, part_text)

    def f(self, key):
        return self.facts["specs"][key]

    def has(self, key):
        return key in self.facts["specs"]

    def pub(self, path, *keys, note=""):
        """Cite `path` as published on the machine's page under `keys`."""
        val = "; ".join(f"{k}: {self.f(k)}" for k in keys)
        self.cites.append(dict(path=path, status="published", source=self.url, taken=TAKEN, value=val, note=note))

    def cite(self, path, status, basis="", source="", value="", note=""):
        c = dict(path=path, status=status)
        if source:
            c.update(source=source, taken=TAKEN)
        if value:
            c["value"] = value
        if basis:
            c["basis"] = basis
        if note:
            c["note"] = note
        self.cites.append(c)

    def emit(self, s=""):
        self.lines.append(s)


def tv(v):
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        r = round(float(v), 3)
        return str(int(r)) if r == int(r) else repr(r)
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(tv(x) for x in v) + "]"
    return json.dumps(v, ensure_ascii=False)


def table(rec, header, **kv):
    rec.emit(f"[[{header}]]")
    for k, v in kv.items():
        if v is not None:
            rec.emit(f"{k} = {tv(v)}")


# ---------------------------------------------------------------------------------------------
# rules shared by every Haas record (each one cited where it is applied)

HOME_BASIS = (
    "Haas mills zero-return every axis to its extreme positive end (HFO Mill Programming manual, "
    "'Machine Home with Work Offsets'), so machine coordinates run from -travel to 0"
)
CENTRED = (
    "unpublished: the travel is taken as centred on the table/platter (the spindle is over the "
    "table centre at mid-travel); a real machine's offset of a few tens of mm moves every envelope"
)
MIN_RPM = (
    "no minimum speed is published; 1 rpm stands in, so the check never refuses a low speed "
    "(errs permissive)"
)


def power_curve(r, spindle):
    """Conservative piecewise-linear power: min(published torque x omega, published peak kW)."""
    p = kw(r.f("Spindle Motor Power"))
    t, at = nm_at(r.f("Spindle Torque"))
    top = rpm(r.f("Max Spindle Speed"))
    corner = p * 1000.0 / (t * 2 * math.pi / 60.0)
    corner = min(math.ceil(corner * 10) / 10, top)  # rounded up: the torque implied stays <= published
    r.cite(
        f"spindle.{spindle}.curve",
        "estimate",
        basis=(
            f"from the published peak power {p} kW and torque {t} N·m @ {at:g} rpm: power = "
            f"min(torque x omega, peak), so constant torque to {corner:.0f} rpm, then constant "
            "power; never above either published figure. Haas publishes no full curve on the "
            "cited page; the real curve may be higher in places (errs low)"
        ),
        source=r.url,
        value=f"Spindle Motor Power: {r.f('Spindle Motor Power')}; Spindle Torque: {r.f('Spindle Torque')}",
    )
    return [0, round(corner, 1), top], [0, p, p]


# ---------------------------------------------------------------------------------------------
# .fg parts: each body a real part inside its envelope


def fg_box(title, lo, hi, cut=None):
    """A box part with an optional pocket/slot cut from its top: cut = (cx, cy, w, l, depth)."""
    s = [f"# {title}", "# frame: the machine's base frame at the home pose (forgecad Haas approximation).",
         f"shared x0 = {tv(lo[0])}", f"shared y0 = {tv(lo[1])}", f"shared z0 = {tv(lo[2])}",
         f"shared lx = {tv(hi[0] - lo[0])}", f"shared ly = {tv(hi[1] - lo[1])}", f"shared lz = {tv(hi[2] - lo[2])}"]
    if cut:
        s += [f"shared cx = {tv(cut[0])}", f"shared cy = {tv(cut[1])}", f"shared cw = {tv(cut[2])}",
              f"shared cl = {tv(cut[3])}", f"shared cd = {tv(cut[4])}"]
    s += ["", "sketch plan(on: offset(xy, z0)) {", "  outline = rect(corner: (x0, y0), size: (lx, ly))", "}",
          "block = extrude(plan, lz)"]
    if cut:
        s += ["sketch top(on: block.end) {", "  r = rect(center: (cx, cy), size: (cw, cl))", "}", "tslot = cut(top.r, cd)"]
    return "\n".join(s) + "\n"


def fg_cyl_z(title, cx, cy, z0, r, h, bore=None):
    """A cylinder along +z from z0 with an optional coaxial bore through it."""
    s = [f"# {title}", "# frame: the machine's base frame at the home pose (forgecad Haas approximation).",
         f"shared cx = {tv(cx)}", f"shared cy = {tv(cy)}", f"shared z0 = {tv(z0)}", f"shared R = {tv(r)}",
         f"shared H = {tv(h)}"]
    if bore:
        s.append(f"shared rb = {tv(bore)}")
    s += ["", "sketch plan(on: offset(xy, z0)) {", "  outline = circle((cx, cy), R)", "}", "body = extrude(plan.outline, H)"]
    if bore:
        s += ["sketch bb(on: body.end) {", "  c = circle((cx, cy), rb)", "}", "through = cut(bb.c, H)"]
    return "\n".join(s) + "\n"


def fg_head(title, ax, ay, nose_z, nose_r, nose_h, box_lo, box_hi):
    """A milling head: a box housing above a nose cylinder on the spindle axis."""
    s = [f"# {title}", "# frame: the machine's base frame at the home pose (forgecad Haas approximation).",
         f"shared ax = {tv(ax)}", f"shared ay = {tv(ay)}", f"shared nz = {tv(nose_z)}", f"shared nr = {tv(nose_r)}",
         f"shared nh = {tv(nose_h)}", f"shared x0 = {tv(box_lo[0])}", f"shared y0 = {tv(box_lo[1])}",
         f"shared z0 = {tv(box_lo[2])}", f"shared lx = {tv(box_hi[0] - box_lo[0])}",
         f"shared ly = {tv(box_hi[1] - box_lo[1])}", f"shared lz = {tv(box_hi[2] - box_lo[2])}",
         "", "sketch plan(on: offset(xy, z0)) {", "  outline = rect(corner: (x0, y0), size: (lx, ly))", "}",
         "housing = extrude(plan, lz)", "sketch nk(on: offset(xy, nz)) {", "  c = circle((ax, ay), nr)", "}",
         "nose = extrude(nk.c, nh)"]
    return "\n".join(s) + "\n"


def body(r, name, link, prims, part, ignore=(), status="estimate", basis="", source="", value=""):
    r.bodies.append((name, link, prims, list(ignore), part))
    r.cite(f"body.{name}", status, basis=basis, source=source, value=value)


def shrink_box(lo, hi, d=1.0):
    return [a + d for a in lo], [b - d for b in hi]


# ---------------------------------------------------------------------------------------------
# kinds


def vmc(r, kind_note, changer_note=""):
    """A Haas VMC (VF, Mini Mill, TM, DT/DM): table X on saddle Y, head Z on the column."""
    tx, ty, tz = mm(r.f("X-Axis Travel")), mm(r.f("Y-Axis Travel")), mm(r.f("Z-Axis Travel"))
    nmax, nmin = mm(r.f("Spindle Nose to Table (max)")), mm(r.f("Spindle Nose to Table (min)"))
    tl, tw = [float(x.replace(",", "")) for x in re.findall(r"[\d.,]+", re.search(r"\((.*)\)", r.f("Table Size")).group(1))[:2]]
    assert re.search(r"mm\)$", r.f("Table Size")), r.f("Table Size")
    rapid = mm_per_min(r.f("Rapid Traverse Rate"))
    feed = mm_per_min(r.f("Max Cutting Feedrate")) if r.has("Max Cutting Feedrate") else rapid
    # spindle over the table centre at mid-travel; at home (0, 0, 0) it is at +x/2, +y/2
    ax, ay = tx / 2, ty / 2
    table(r, "machine", name=r.name, kind="vmc3", control="haas", real=True, length_joint="Z",
          coolants=["flood"], clearance=1.0,
          source=r.url + " (restating haascnc.com; see the cites)", taken=TAKEN,
          note=("forgecad's Haas record from public spec data; every value cited, estimates and unknowns "
                "marked. Geometry: base frame = table top centre at home (every joint 0); +x right, +y toward "
                "the column, +z up. " + kind_note))
    r.cite("machine.coolants", "published" if r.has("Coolant Capacity") else "unknown", source=r.url if r.has("Coolant Capacity") else "",
           value=f"Coolant Capacity: {r.f('Coolant Capacity')}" if r.has("Coolant Capacity") else "",
           basis="" if r.has("Coolant Capacity") else "no coolant system listed",
           note="flood coolant tank listed; through-spindle coolant and air blast are options, not declared")
    r.cite("machine.kinematics", "estimate",
           basis=("Haas VF-style layout: the table moves X on a saddle that moves Y, the head moves Z on a fixed "
                  "column. Not stated on the cited spec page; ISO 841 directions with the table moving opposite "
                  "the program's tool motion (docs/cam-machine.md). Verify on the machine"))
    for (n, parent, axis, travel, key) in (("Y", "base", [0, -1, 0], ty, "Y-Axis Travel"),
                                            ("X", "Y", [-1, 0, 0], tx, "X-Axis Travel"),
                                            ("Z", "base", [0, 0, 1], tz, "Z-Axis Travel")):
        table(r, "machine.joint", name=n, parent=parent, side="head" if n == "Z" else "table", type="linear",
              axis=axis, range=[-travel, 0], home=0, rapid=rapid, max_feed=feed)
        r.pub(f"joint.{n}.range", key, note="travel published; placement -travel..0 from the zero-return rule below")
        r.cite(f"joint.{n}.range", "published", source=HFO_MILL, basis="", value="zero return moves every axis to its extreme positive location",
               note=HOME_BASIS)
        r.pub(f"joint.{n}.rapid", "Rapid Traverse Rate", note="one rate published for all axes")
        if r.has("Max Cutting Feedrate"):
            r.pub(f"joint.{n}.max_feed", "Max Cutting Feedrate")
        else:
            r.cite(f"joint.{n}.max_feed", "unknown", basis="no max cutting feed published; taken equal to the rapid (errs permissive)")
        r.cite(f"joint.{n}.axis", "estimate", basis="ISO 841 / docs/cam-machine.md table-side convention; Haas sign conventions not stated on the spec page")
    nose = [ax, ay, nmax]
    tool_len = mm(r.f("Max Tool Length")) if r.has("Max Tool Length") else None
    tool_dia = mm(r.f("Max Tool Diameter (full magazine)")) if r.has("Max Tool Diameter (full magazine)") else None
    curve_rpm, curve_kw = power_curve(r, "S1")
    table(r, "machine.spindle", name="S1", link="Z", kind="milling", axis=[0, 0, 1], nose=nose,
          rpm=[1, rpm(r.f("Max Spindle Speed"))], curve_rpm=curve_rpm, curve_kw=curve_kw,
          max=[tool_len or r.default_len, tool_dia or r.default_dia],
          interface=r.f("Spindle Taper").split("/")[0].strip())
    r.pub("spindle.S1.rpm", "Max Spindle Speed", note=MIN_RPM)
    r.pub("spindle.S1.nose", "Spindle Nose to Table (max)", "Spindle Nose to Table (min)",
          note="nose height at Z = 0 published (max = min + Z travel); x, y: " + CENTRED)
    if tool_len and tool_dia:
        r.pub("spindle.S1.max", "Max Tool Length", "Max Tool Diameter (full magazine)")
    else:
        r.cite("spindle.S1.max", "estimate", basis=r.max_basis)
    r.pub("spindle.S1.interface", "Spindle Taper", note="the first listed taper; the others are options")
    if r.has("Tool Capacity"):
        table(r, "machine.changer", pockets=count(r.f("Tool Capacity")), change_s=secs(r.f("Tool Change (chip-to-chip)")),
              max_tool_kg=kg(r.f("Max Tool Weight")))
        r.pub("changer.pockets", "Tool Capacity", "Tool Changer Type", note="the '+1' spindle tool is not a pocket")
        r.pub("changer.change_s", "Tool Change (chip-to-chip)")
        r.pub("changer.max_tool_kg", "Max Tool Weight")
    table(r, "machine.mount", name="spindle", kind="tool", link="Z", at=nose, dir=[0, 0, -1])
    r.pub("mount.spindle.at", "Spindle Nose to Table (max)",
          note="gauge point taken at the spindle nose face (the taper gauge line is a few mm inside: unmodelled); x, y: " + CENTRED)
    table(r, "machine.mount", name="table", kind="part", link="X", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.pub("mount.table.at", "Table Size", note="the table top centre is the frame origin")
    # bodies
    th = 90.0
    lo, hi = [-tl / 2, -tw / 2, -th], [tl / 2, tw / 2, 0.0]
    slot = (0, 0, tl, 16, 14)
    body(r, "table", "X", [("box", lo, hi)],
         fg_box(f"{r.name} table: {tl:g} x {tw:g} published, thickness estimated; one T-slot", *shrink_box(lo, hi), cut=slot),
         status="published", source=r.url, value=f"Table Size: {r.f('Table Size')}",
         basis="")
    r.cite("body.table", "estimate", basis=f"thickness {th:g} mm unpublished (only the top surface and outline matter for tool collisions)")
    nr, nh = 95.0, 160.0
    hb_lo = [ax - 190, ay - 190, nmax + nh]
    hb_hi = [ax + 190, ay + 420, nmax + nh + 420]
    body(r, "head", "Z", [("cyl", [ax, ay, nmax], [0, 0, 1], nr, nh), ("box", hb_lo, hb_hi)],
         fg_head(f"{r.name} head: spindle nose cylinder and housing (unpublished sizes, conservative)",
                 ax, ay, nmax + 1, nr - 1, nh - 1, [hb_lo[0] + 1, hb_lo[1] + 1, hb_lo[2]], [hb_hi[0] - 1, hb_hi[1] - 1, hb_hi[2] - 1]),
         status="unknown",
         basis=(f"no head dimensions published: nose cylinder r {nr:g} mm x {nh:g} mm and a housing "
                f"{hb_hi[0]-hb_lo[0]:g} x {hb_hi[1]-hb_lo[1]:g} x {hb_hi[2]-hb_lo[2]:g} mm chosen larger than a 40-taper nose "
                "(errs large: a short tool deep in a pocket may report a hit the real nose would clear)"))


def umc(r):
    """UMC-750: X Y Z on the head side (Haas UMC: column/saddle move the head), B tilt and C
    rotary on the table side."""
    tx, ty, tz = mm(r.f("X-Axis Travel")), mm(r.f("Y-Axis Travel")), mm(r.f("Z-Axis Travel"))
    nmax = mm(r.f("Spindle Nose to Platter (max)"))
    pd = mm(r.f("Platter Diameter"))
    rapid = mm_per_min(r.f("Rapid Traverse Rate"))
    feed = mm_per_min(r.f("Max Cutting Feedrate"))
    brate = float(re.match(r"([\d.]+)", r.f("B Axis Max Speed")).group(1)) * 60
    crate = float(re.match(r"([\d.]+)", r.f("C Axis Max Speed")).group(1)) * 60
    bt = [float(x) for x in re.findall(r"-?\d+", r.f("B Axis Travel"))]
    blo, bhi = min(bt), max(bt)
    ax, ay = tx / 2, ty / 2
    table(r, "machine", name=r.name, kind="table-table", control="haas", real=True, length_joint="Z",
          coolants=["flood"], clearance=1.0, source=r.url + " (restating haascnc.com; see the cites)", taken=TAKEN,
          note=("forgecad's Haas record from public spec data; every value cited, estimates and unknowns marked. "
                "Base frame = platter top centre at home (every joint 0), B and C at 0; +x right, +y toward the "
                "column, +z up. G234 (TCPC) / G254 (DWO) are Haas options; with them active do not G28 G91 home "
                "(Gerotech NGC training), use G53."))
    r.cite("machine.coolants", "published", source=r.url, value=f"Coolant Capacity: {r.f('Coolant Capacity')}",
           note="flood listed; through-spindle coolant is an option, not declared")
    r.cite("machine.kinematics", "estimate",
           basis=("the B tilt axis is taken parallel to y (B about Y, ISO 841): the published tilt is asymmetric "
                  "(+120/-35) and reported as keeping the platter clear of the side-mount tool changer, i.e. "
                  "it tilts toward +-x; a review describes the tilt as perpendicular to the door opening "
                  "(machinery.co.uk via search; page not fetched, 403). C is carried by B (Table/Table, C on B). "
                  "The X, Y, Z split between head and table is not on the spec page: taken as all on the head "
                  "(the trunnion stays fixed in X and Y). Which tilt direction is +B is UNKNOWN"))
    for (n, parent, axis, travel, key) in (("X", "base", [1, 0, 0], tx, "X-Axis Travel"),
                                            ("Y", "X", [0, 1, 0], ty, "Y-Axis Travel"),
                                            ("Z", "Y", [0, 0, 1], tz, "Z-Axis Travel")):
        table(r, "machine.joint", name=n, parent=parent, side="head", type="linear", axis=axis,
              range=[-travel, 0], home=0, rapid=rapid, max_feed=feed)
        r.pub(f"joint.{n}.range", key)
        r.cite(f"joint.{n}.range", "published", source=HFO_MILL, value="zero return moves every axis to its extreme positive location",
               note=HOME_BASIS)
        r.pub(f"joint.{n}.rapid", "Rapid Traverse Rate")
        r.cite(f"joint.{n}.rapid", "conflict", source=UK_UMC750, value="Rapids on X/Y/Z: 1200 in/min (30.5 m/min)",
               note=f"Haas UK lists 1200 ipm; the cited page (restating haascnc.com, verified 2026-10-02) lists 900 ipm. The record holds 900 ipm = {rapid:g} mm/min (current listing; slower rapids give longer, not shorter, time estimates)")
        r.pub(f"joint.{n}.max_feed", "Max Cutting Feedrate")
    pivot_z = -150.0
    table(r, "machine.joint", name="B", parent="base", side="table", type="rotary", axis=[0, -1, 0],
          pivot=[0, 0, pivot_z], range=[blo, bhi], home=0, rapid=brate, max_feed=brate, tcp=True)
    r.pub("joint.B.range", "B Axis Travel")
    r.cite("joint.B.range", "conflict", source=DS_UMC750SS, value="B Axis - Tilt Travels: +110, -35 (UMC-750SS data sheet)",
           note="the older UMC-750SS data sheet lists +110/-35; the current UMC-750 listing (and Haas UK) +120/-35. The record holds +120/-35 for the UMC-750")
    r.cite("joint.B.range", "published", source=UK_UMC750, value="B Axis - Tilt Travel: +120 to -35")
    r.pub("joint.B.rapid", "B Axis Max Speed", note="50 deg/s = 3000 deg/min")
    r.cite("joint.B.max_feed", "unknown", basis="no rotary feed limit published; taken equal to the rapid (errs permissive)")
    r.cite("joint.B.pivot", "unknown",
           basis=(f"the tilt axis height under the platter is unpublished (Haas sets it per machine as MRZP, settings 254-257 / 300-305); "
                  f"{-pivot_z:g} mm below the platter top is a stand-in. Collision verdicts at B != 0 depend on it"))
    r.cite("joint.B.axis", "estimate", basis="see machine.kinematics; the sign (+B direction) is UNKNOWN, table convention of docs/cam-machine.md")
    table(r, "machine.joint", name="C", parent="B", side="table", type="rotary", axis=[0, 0, -1], pivot=[0, 0, 0],
          range=[-360, 360], home=0, rapid=crate, max_feed=crate, cyclic=True, tcp=True)
    r.pub("joint.C.range", "C Axis Travel", note="360 deg continuous: cyclic")
    r.pub("joint.C.rapid", "C Axis Max Speed", note="50 deg/s = 3000 deg/min")
    r.cite("joint.C.max_feed", "unknown", basis="no rotary feed limit published; taken equal to the rapid (errs permissive)")
    r.pub("joint.C.pivot", "Platter Diameter", note="C turns about the platter centre")
    nose = [ax, ay, nmax]
    curve_rpm, curve_kw = power_curve(r, "S1")
    table(r, "machine.spindle", name="S1", link="Z", kind="milling", axis=[0, 0, 1], nose=nose,
          rpm=[1, rpm(r.f("Max Spindle Speed"))], curve_rpm=curve_rpm, curve_kw=curve_kw,
          max=[mm(r.f("Max Tool Length")), mm(r.f("Max Tool Diameter (full magazine)"))],
          interface=r.f("Spindle Taper").split("/")[0].strip())
    r.pub("spindle.S1.rpm", "Max Spindle Speed", note=MIN_RPM)
    r.pub("spindle.S1.nose", "Spindle Nose to Platter (max)", "Spindle Nose to Platter (min)", note="x, y: " + CENTRED)
    r.pub("spindle.S1.max", "Max Tool Length", "Max Tool Diameter (full magazine)")
    r.cite("spindle.S1.max", "conflict", source=UK_UMC750, value="Max Tool Diameter (full): 3 in (76 mm)",
           note="Haas UK lists 3 in with a full magazine, the cited page 2.5 in; the record holds the smaller, 64 mm")
    r.pub("spindle.S1.interface", "Spindle Taper", note="the first listed taper; the others are options")
    table(r, "machine.changer", pockets=count(r.f("Tool Capacity")), change_s=secs(r.f("Tool Change (chip-to-chip)")),
          max_tool_kg=kg(r.f("Max Tool Weight")))
    r.pub("changer.pockets", "Tool Capacity", "Tool Changer Type")
    r.pub("changer.change_s", "Tool Change (chip-to-chip)")
    r.pub("changer.max_tool_kg", "Max Tool Weight")
    table(r, "machine.mount", name="spindle", kind="tool", link="Z", at=nose, dir=[0, 0, -1])
    r.pub("mount.spindle.at", "Spindle Nose to Platter (max)", note="gauge point at the nose face; x, y: " + CENTRED)
    table(r, "machine.mount", name="platter", kind="part", link="C", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.pub("mount.platter.at", "Platter Diameter", note="the platter top centre is the frame origin")
    # bodies: platter (C), cradle (B), the two trunnion supports (base), the head (Z)
    pr = pd / 2
    body(r, "platter", "C", [("cyl", [0, 0, -60], [0, 0, 1], pr, 60)],
         fg_cyl_z(f"{r.name} platter: {pd:g} mm published, 60 mm thick (estimate)", 0, 0, -59, pr - 1, 58, bore=20),
         ignore=["cradle"], status="published", source=r.url, value=f"Platter Diameter: {r.f('Platter Diameter')}")
    r.cite("body.platter", "estimate", basis="platter thickness 60 mm unpublished")
    cl, ch = [-(pr + 60), -(pr + 20), pivot_z - 90], [pr + 60, pr + 20, -60]
    body(r, "cradle", "B", [("box", cl, ch)], fg_box(f"{r.name} trunnion cradle (unpublished, conservative)", *shrink_box(cl, ch)),
         ignore=["platter", "support-l", "support-r"], status="unknown",
         basis="cradle size unpublished: a box under the platter, wider than it by 60 mm a side, down to 90 mm under the tilt axis (stand-in)")
    for side, s in (("l", -1), ("r", 1)):
        y0 = s * (pr + 20)
        lo_, hi_ = [-160, min(y0, y0 + s * 140), pivot_z - 400], [160, max(y0, y0 + s * 140), 40]
        body(r, f"support-{side}", "base", [("box", lo_, hi_)],
             fg_box(f"{r.name} trunnion support {side} (unpublished, conservative)", *shrink_box(lo_, hi_)),
             ignore=["cradle"], status="unknown",
             basis="trunnion supports unpublished: 140 mm thick boxes at the ends of the B axis, to 40 mm above the platter top (stand-in)")
    nr, nh = 95.0, 160.0
    hb_lo, hb_hi = [ax - 190, ay - 190, nmax + nh], [ax + 190, ay + 420, nmax + nh + 420]
    body(r, "head", "Z", [("cyl", [ax, ay, nmax], [0, 0, 1], nr, nh), ("box", hb_lo, hb_hi)],
         fg_head(f"{r.name} head (unpublished sizes, conservative)", ax, ay, nmax + 1, nr - 1, nh - 1,
                 [hb_lo[0] + 1, hb_lo[1] + 1, hb_lo[2]], [hb_hi[0] - 1, hb_hi[1] - 1, hb_hi[2] - 1]),
         status="unknown", basis="no head dimensions published; the VMC head stand-in (errs large)")


def lathe_common(r, kind, control_note):
    table(r, "machine", name=r.name, kind=kind, control="fanuc-lathe", real=True,
          length_joint=None, coolants=["flood"], clearance=1.0,
          source=r.url + " (restating haascnc.com; see the cites)", taken=TAKEN,
          note=("forgecad's Haas record from public spec data; every value cited, estimates and unknowns marked. "
                "Base frame = main spindle centre at the chuck face; +z along the spindle away from the headstock, "
                "+x radial toward the turret, +y = z cross x. " + control_note))
    r.cite("machine.coolants", "published", source=r.url, value=f"Coolant Capacity: {r.f('Coolant Capacity')}")
    r.cite("machine.control", "estimate",
           basis="Haas lathes speak a Fanuc-style lathe dialect (diameter X, G50 clamp, G96/G97, G70-G76); forge-post has no haas-lathe dialect yet (research/cam/machines/haas-dialect-audit.md)")


def lathe_axes(r, y=False):
    """Z then X (diameter) on the head side; Y across when present."""
    tx, tz = mm(r.f("X-Axis Travel")), mm(r.f("Z-Axis Travel"))
    rmax = mm(r.f("Max Turning Diameter")) / 2
    if r.has("Rapid Traverse Rate"):
        rx = rz = ry = mm_per_min(r.f("Rapid Traverse Rate"))
        rk = {"X": "Rapid Traverse Rate", "Z": "Rapid Traverse Rate", "Y": "Rapid Traverse Rate"}
    else:
        rx, rz = mm_per_min(r.f("Rapid Traverse X")), mm_per_min(r.f("Rapid Traverse Z"))
        ry = mm_per_min(r.f("Rapid Traverse Y")) if y else rx
        rk = {"X": "Rapid Traverse X", "Z": "Rapid Traverse Z", "Y": "Rapid Traverse Y"}
    r.cite("machine.kinematics", "estimate",
           basis=("turret on cross (X) and long (Z) slides, the spindle fixed: the usual slant-bed turning-centre "
                  "layout, not stated on the spec page. Joint X = the reference tool point's radius; Z = its "
                  "distance from the chuck face"))
    table(r, "machine.joint", name="Z", parent="base", side="head", type="linear", axis=[0, 0, 1],
          range=[0, tz], home=tz, rapid=rz, max_feed=rz)
    r.pub("joint.Z.range", "Z-Axis Travel", "Max Turning Length",
          note="travel published; placed from the chuck face (0) to +travel (estimate: the reach of the reference point; real reach depends on the tool offsets)")
    r.pub("joint.Z.rapid", rk["Z"])
    r.cite("joint.Z.max_feed", "unknown", basis="no max cutting feed published for lathes on the cited page; taken equal to the rapid (errs permissive)")
    table(r, "machine.joint", name="X", parent="Z", side="head", type="linear", axis=[1, 0, 0],
          range=[round(rmax - tx, 1), rmax], home=rmax, rapid=rx, max_feed=rx, diameter=True)
    r.pub("joint.X.range", "X-Axis Travel", "Max Turning Diameter",
          note="travel published; the upper end placed at the max turning radius, the lower end = upper - travel (estimate)")
    r.pub("joint.X.rapid", rk["X"])
    r.cite("joint.X.max_feed", "unknown", basis="no max cutting feed published for lathes on the cited page; taken equal to the rapid (errs permissive)")
    tip_parent = "X"
    if y:
        half = mm(r.f("Y-Axis Travel"))
        table(r, "machine.joint", name="Y", parent="X", side="head", type="linear", axis=[0, 1, 0],
              range=[-half, half], home=0, rapid=ry, max_feed=ry)
        r.pub("joint.Y.range", "Y-Axis Travel")
        r.pub("joint.Y.rapid", rk["Y"])
        r.cite("joint.Y.max_feed", "unknown", basis="no max cutting feed published; taken equal to the rapid (errs permissive)")
        tip_parent = "Y"
    return tip_parent, rmax


def main_spindle(r, link, c_joint=None):
    top = rpm(r.f("Max Spindle Speed"))
    curve_rpm, curve_kw = power_curve(r, "main")
    swing = mm(r.f("Max Swing"))
    table(r, "machine.spindle", name="main", link=link, kind="turning", axis=[0, 0, 1], nose=[0, 0, 0],
          rpm=[1, top], curve_rpm=curve_rpm, curve_kw=curve_kw, max=[swing, mm(r.f("Max Turning Length"))],
          interface=r.f("Spindle Nose"), c_joint=c_joint)
    r.pub("spindle.main.rpm", "Max Spindle Speed", note=MIN_RPM)
    r.pub("spindle.main.nose", "Chuck Size", note="the chuck face centre is the frame origin")
    r.pub("spindle.main.max", "Max Swing", "Max Turning Length")
    r.pub("spindle.main.interface", "Spindle Nose")


def chuck_body(r, name, link, chuck_d, z_face, toward):
    """A chuck: a cylinder of the published chuck diameter behind its face, jaws inside the OD."""
    rr = chuck_d / 2 + 5
    L = 110.0
    z0 = z_face - L if toward > 0 else z_face
    body(r, name, link, [("cyl", [0, 0, z0], [0, 0, 1], rr, L)],
         fg_cyl_z(f"{r.name} {name}: {chuck_d:g} mm chuck (published) + 5 mm, {L:g} mm long (estimate)", 0, 0, z0 + 1, rr - 1, L - 2, bore=15),
         ignore=[], status="published", source=r.url, value=f"Chuck Size: {r.f('Chuck Size') if name == 'chuck' else r.f('Sub Spindle Chuck Size')}")
    r.cite(f"body.{name}", "estimate", basis=f"chuck length {L:g} mm and a 5 mm radial margin for jaws are unpublished stand-ins; jaws standing past the face are not modelled")


def turret_body(r, link, rmax):
    """A BOT turret disc behind the reference tool point."""
    R = 190.0
    T = 120.0
    cx = R + 60.0
    body(r, "turret", link, [("cyl", [cx, 0, -T / 2], [0, 0, 1], R, T)],
         fg_cyl_z(f"{r.name} turret disc (unpublished, conservative)", cx, 0, -T / 2 + 1, R - 1, T - 2, bore=30),
         status="unknown",
         basis=(f"turret disc size unpublished: r {R:g} mm x {T:g} mm, its rim 60 mm behind the reference tool point "
                "(the tool's own body is the setup's). Errs large"))
    return cx


def lathe(r, note=""):
    lathe_common(r, "lathe", note)
    tip, rmax = lathe_axes(r)
    main_spindle(r, "base")
    stations = count(r.f("Turret Stations"))
    cx = 250.0
    table(r, "machine.turret", name="T1", link=tip, stations=stations, index_s=0.5, center=[cx, 0, 0], index_axis=[0, 0, 1])
    r.pub("turret.T1.stations", "Turret Stations")
    r.cite("turret.T1.index_s", "unknown", basis="turret index time not published on the cited page; 0.5 s stand-in")
    r.cite("turret.T1.center", "unknown", basis="turret geometry unpublished; centre 250 mm behind the reference tool point (stand-in)")
    table(r, "machine.mount", name="turret", kind="tool", link=tip, at=[0, 0, 0], dir=[-1, 0, 0])
    r.cite("mount.turret.at", "estimate", basis="the reference tool point is the frame point the X/Z joint values place (X = its radius, Z = its distance from the chuck face)")
    table(r, "machine.mount", name="chuck", kind="part", link="base", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.pub("mount.chuck.at", "Chuck Size", note="the chuck face centre is the frame origin")
    chuck_body(r, "chuck", "base", mm(r.f("Chuck Size")), 0.0, 1)
    turret_body(r, tip, rmax)


def millturn(r, sub=False, note=""):
    """ST-xxY / DS-30Y: C on the main spindle, Y on the turret, live tools; DS adds the sub."""
    lathe_common(r, "mill-turn", note)
    tip, rmax = lathe_axes(r, y=True)
    crpm = float(re.match(r"([\d.]+)", r.f("C Axis Speed")).group(1))
    table(r, "machine.joint", name="C", parent="base", side="table", type="rotary", axis=[0, 0, 1], pivot=[0, 0, 0],
          range=[-360, 360], home=0, rapid=crpm * 360, max_feed=crpm * 360, cyclic=True)
    r.pub("joint.C.range", "C Axis Control Type", note="interpolated C: continuous, cyclic")
    r.pub("joint.C.rapid", "C Axis Speed", note="rpm x 360 = deg/min")
    r.pub("joint.C.max_feed", "C Axis Speed", note="the only C rate published; used for feed too")
    r.pub("joint.C.pivot", "Chuck Size", note="C turns about the spindle axis")
    main_spindle(r, "C", c_joint="C")
    if sub:
        tz = mm(r.f("Z-Axis Travel"))
        zs = tz + 250.0
        table(r, "machine.joint", name="Z2", parent="base", side="table", type="linear", axis=[0, 0, -1],
              range=[0, tz], home=0, rapid=mm_per_min(r.f("Rapid Traverse Z")), max_feed=mm_per_min(r.f("Rapid Traverse Z")))
        r.cite("joint.Z2.range", "unknown",
               basis=("sub-spindle travel not published on the DS-30Y page (the older DS-30 page lists 31.9 in / 810 mm, "
                      "machinetoolindex.com/machines/haas-ds-30); taken as the Z travel, the sub's face starting 250 mm "
                      "past it (stand-in). Haas names this axis B (linear) on DS lathes per common usage, unverified: "
                      "the format allows A-C on rotaries only (FORMAT-GAPS)"))
        r.cite("joint.Z2.rapid", "unknown", basis="sub-spindle rapid not published on the DS-30Y page; the main Z rapid stands in")
        r.cite("joint.Z2.max_feed", "unknown", basis="taken equal to the rapid (errs permissive)")
        sc = float(re.match(r"([\d.]+)", r.f("Sub Spindle C Axis Max Speed")).group(1))
        table(r, "machine.joint", name="C2", parent="Z2", side="table", type="rotary", axis=[0, 0, -1], pivot=[0, 0, zs],
              range=[-360, 360], home=0, rapid=sc * 360, max_feed=sc * 360, cyclic=True)
        r.pub("joint.C2.range", "Sub Spindle C Axis Control Type")
        r.pub("joint.C2.rapid", "Sub Spindle C Axis Max Speed", note="rpm x 360 = deg/min")
        r.pub("joint.C2.max_feed", "Sub Spindle C Axis Max Speed")
        r.cite("joint.C2.pivot", "estimate", basis="on the spindle axis (published coaxial); z = the sub's face at home (stand-in, see joint.Z2.range)")
        sp = kw(r.f("Sub Spindle Power"))
        st, sat = nm_at(r.f("Sub Spindle Torque"))
        stop = rpm(r.f("Sub Spindle Speed"))
        corner = min(math.ceil(sp * 1000.0 / (st * 2 * math.pi / 60.0) * 10) / 10, stop)
        table(r, "machine.spindle", name="sub", link="C2", kind="turning", axis=[0, 0, -1], nose=[0, 0, zs],
              rpm=[1, stop], curve_rpm=[0, round(corner, 1), stop], curve_kw=[0, sp, sp],
              max=[mm(r.f("Sub Spindle Chuck Size")), mm(r.f("Max Turning Length"))], interface=r.f("Sub Spindle Nose"), c_joint="C2")
        r.pub("spindle.sub.rpm", "Sub Spindle Speed", note=MIN_RPM)
        r.cite("spindle.sub.nose", "estimate", basis="coaxial with the main spindle; z = the sub's face at home (stand-in)")
        r.cite("spindle.sub.max", "estimate", basis="swing taken as the sub chuck size, length as the main's max turning length (the sub's own limits are not published)")
        r.pub("spindle.sub.curve", "Sub Spindle Power", "Sub Spindle Torque", note="conservative min(torque x omega, peak power), as for the main")
        r.pub("spindle.sub.interface", "Sub Spindle Nose")
        table(r, "machine.mount", name="sub", kind="part", link="C2", at=[0, 0, zs], dir=[0, 0, -1], xdir=[1, 0, 0])
        r.cite("mount.sub.at", "estimate", basis="the sub's chuck face centre at home (stand-in, see joint.Z2.range)")
    stations = count(r.f("Turret Stations"))
    cx = 250.0
    live_rpm = rpm(r.f("Live Tool Speed"))
    table(r, "machine.spindle", name="live", link=tip, kind="milling", axis=[1, 0, 0], nose=[0, 0, 0],
          rpm=[1, live_rpm], max=[150, 25], interface=r.f("Live Tooling"))
    r.pub("spindle.live.rpm", "Live Tool Speed", note=MIN_RPM)
    r.cite("spindle.live.nose", "estimate", basis="the live tool's gauge point at the reference tool point (radial tools; axial live tools turn about z, not modelled)")
    r.cite("spindle.live.max", "unknown", basis="live tool length/diameter limits unpublished; 150 x 25 mm stand-in (errs small: refuses larger tools)")
    r.pub("spindle.live.interface", "Live Tooling")
    table(r, "machine.turret", name="T1", link=tip, stations=stations, index_s=0.5, center=[cx, 0, 0], index_axis=[0, 0, 1], live="live")
    r.pub("turret.T1.stations", "Turret Stations")
    r.cite("turret.T1.index_s", "unknown", basis="turret index time not published on the cited page; 0.5 s stand-in")
    r.cite("turret.T1.center", "unknown", basis="turret geometry unpublished; centre 250 mm behind the reference tool point (stand-in)")
    table(r, "machine.mount", name="turret", kind="tool", link=tip, at=[0, 0, 0], dir=[-1, 0, 0])
    r.cite("mount.turret.at", "estimate", basis="the reference tool point is the frame point the X/Y/Z joint values place")
    table(r, "machine.mount", name="main", kind="part", link="C", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.pub("mount.main.at", "Chuck Size", note="the main chuck face centre is the frame origin")
    chuck_body(r, "chuck", "C", mm(r.f("Chuck Size")), 0.0, 1)
    if sub:
        sd = mm(r.f("Sub Spindle Chuck Size"))
        rr = sd / 2 + 5
        L = 110.0
        zs = mm(r.f("Z-Axis Travel")) + 250.0
        body(r, "subchuck", "C2", [("cyl", [0, 0, zs], [0, 0, 1], rr, L)],
             fg_cyl_z(f"{r.name} sub chuck: {sd:g} mm (published) + 5 mm, {L:g} mm (estimate)", 0, 0, zs + 1, rr - 1, L - 2, bore=15),
             status="published", source=r.url, value=f"Sub Spindle Chuck Size: {r.f('Sub Spindle Chuck Size')}")
        r.cite("body.subchuck", "estimate", basis="length and jaw margin unpublished stand-ins")
    turret_body(r, tip, rmax)


# ---------------------------------------------------------------------------------------------
# the library

MACHINES = [
    # (record name, facts key, builder, extra)
    ("haas-vf-2", "haas-vf-2", "vmc", dict(default_len=279, max_basis=(
        "max tool length unpublished on the VF-2 page; the VF-2SS page's 11 in (279 mm) taken (the smaller published "
        "sibling value: errs small). Diameter: VF-2 page, full magazine"))),
    ("haas-vf-2ss", "haas-vf-2ss", "vmc", {}),
    ("haas-vf-3", "haas-vf-3", "vmc", {}),
    ("haas-vf-4", "haas-vf-4", "vmc", {}),
    ("haas-mini-mill", "haas-mini-mill", "vmc", {}),
    ("haas-tm-1", "haas-tm-1", "vmc", dict(default_len=279, default_dia=89, max_basis=(
        "manual tool change: no tool limits published; 279 x 89 mm (the VF-2SS length, the VF-2 full-magazine "
        "diameter) stand in (errs small)"))),
    ("haas-tm-2", "haas-tm-2", "vmc", dict(default_len=279, default_dia=89, max_basis=(
        "manual tool change: no tool limits published; 279 x 89 mm (the VF-2SS length, the VF-2 full-magazine "
        "diameter) stand in (errs small)"))),
    ("haas-dt-1", "haas-dt-1", "vmc", {}),
    ("haas-dm-1", "haas-dm-1", "vmc", {}),
    ("haas-umc-750", "haas-umc-750", "umc", {}),
    ("haas-st-10", "haas-st-10", "lathe", {}),
    ("haas-st-20", "haas-st-20", "lathe", {}),
    ("haas-st-30", "haas-st-30", "lathe", {}),
    ("haas-st-10y", "haas-st-10y", "millturn", {}),
    ("haas-st-20y", "haas-st-20y", "millturn", {}),
    ("haas-st-30y", "haas-st-30y", "millturn", {}),
    ("haas-ds-30y", "haas-ds-30y", "millturn-sub", {}),
]

VMC_NOTES = {
    "haas-tm-1": "Toolroom mill: manual tool change (no changer).",
    "haas-tm-2": "Toolroom mill: manual tool change (no changer).",
    "haas-dt-1": "Drill/tap centre, BT30.",
    "haas-dm-1": "Drill/mill centre, 40 taper.",
}


def prim_toml(r, bname, link, prims, ignore, part):
    out = [f"[[machine.body]]", f"name = {tv(bname)}", f"link = {tv(link)}",
           f"part = {tv(f'parts/haas/{r.name}/{bname}.fg')}"]
    if ignore:
        out.append(f"ignore = {tv(ignore)}")
    for p in prims:
        if p[0] == "box":
            out += ["[[machine.body.box]]", f"min = {tv(p[1])}", f"max = {tv(p[2])}"]
        else:
            out += ["[[machine.body.cyl]]", f"base = {tv(p[1])}", f"axis = {tv(p[2])}", f"r = {tv(p[3])}", f"h = {tv(p[4])}"]
    return out


def main():
    facts = json.load(open(FACTS))
    text = [
        "# Haas Automation machines: forgecad's records from PUBLIC spec data, restated (never copied).",
        "# GENERATED by tools/machines/haas.py from research/cam/machines/haas/; edit those, then re-run.",
        "# Every value carries a [[machine.cite]] (docs/cam-machine.md, Provenance): published (with the page",
        "# it is restated from and the date read), estimate (with its basis), unknown (a conservative or",
        "# stated stand-in), conflict (what disagrees). Primary source: machinetoolindex.com, which restates",
        "# haascnc.com (verified 2026-10-02 per its pages); haascnc.com itself refuses automated reads (bot",
        "# challenge). Bodies are approximations: published outlines where there are any, conservative",
        "# stand-ins elsewhere. Verify against your machine's own manual and layout drawings.",
        "",
    ]
    for name, key, kind, extra in MACHINES:
        r = Rec(name, facts[key])
        r.default_len = extra.get("default_len", 279)
        r.default_dia = extra.get("default_dia")
        r.max_basis = extra.get("max_basis", (
            "max tool length unpublished on this page; the VF-2SS page's 11 in (279 mm) taken (the smallest "
            "published 40-taper side-mount value: errs small). Diameter: this page, full magazine"))
        if kind == "vmc":
            vmc(r, VMC_NOTES.get(name, ""))
        elif kind == "umc":
            umc(r)
        elif kind == "lathe":
            lathe(r)
        elif kind == "millturn":
            millturn(r)
        else:
            millturn(r, sub=True)
        text.append("# " + "-" * 93)
        text += r.lines
        os.makedirs(os.path.join(PARTS, name), exist_ok=True)
        for (bname, link, prims, ignore, part) in r.bodies:
            text += prim_toml(r, bname, link, prims, ignore, part)
            with open(os.path.join(PARTS, name, f"{bname}.fg"), "w") as fh:
                fh.write(part)
        for c in r.cites:
            text.append("[[machine.cite]]")
            for k in ("path", "status", "source", "taken", "value", "basis", "note"):
                if c.get(k):
                    text.append(f"{k} = {tv(c[k])}")
        text.append("")
    with open(OUT, "w") as fh:
        fh.write("\n".join(text))
    print(f"wrote {OUT}: {len(MACHINES)} machines")


if __name__ == "__main__":
    main()
