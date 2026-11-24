#!/usr/bin/env python3
"""Generates kernel/forge-machine/data/machines/hurco.toml and parts/hurco/<machine>/ from Hurco's
public line cards (cms.hurco.com, linked from hurco.com's product pages; read 2026-11-23).
Same rules as tools/machines/haas.py (helpers reused): every value cited, nothing from memory.

    python3 tools/machines/hurco.py
"""
import math
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
import haas as H  # noqa: E402

ROOT = H.ROOT
OUT = os.path.join(ROOT, "kernel/forge-machine/data/machines/hurco.toml")
PARTS = os.path.join(ROOT, "kernel/forge-machine/parts/hurco")
TAKEN = "2026-11-23"
VM = "https://cms.hurco.com/uploads/VM_Hurco_linecard_ea489e84b2.pdf"
VMX = "https://cms.hurco.com/uploads/VMX_Hurco_linecard_24_50_x_travel_7e5555f1ae.pdf"
SRT = "https://cms.hurco.com/uploads/SRT_Hurco_linecard_37566f0a63.pdf"
TM = "https://cms.hurco.com/uploads/TM_Hurco_linecard_87c8b01782.pdf"

# Line-card cells restated per model (the card's own figures; mm and m/min used, inch noted).
# table (mm x mm), nose (min[, max] mm), travels (mm), rpm, P_cont kW @ rpm, taper, pockets, tool
# dia / length (mm), kg, tool-to-tool s, rapids XY / Z (m/min), programmable feed (m/min)
MILLS = {
    "hurco-vm-one": (VM, "VM ONE", (762, 406), (76,), (660, 406, 356), 8000, (3.7, 1450), "CAT40", 16, 90, 200, 7, 3.5, (28, 28), 28),
    "hurco-vm10i": (VM, "VM10i", (762, 406), (102,), (660, 406, 508), 12000, (7.5, 1450), "CAT40", 24, 80, 250, 7, 2, (28, 28), 28),
    "hurco-vm10hsi": (VM, "VM10HSi", (762, 406), (102,), (660, 406, 508), 20000, (7.5, 1500), "BT30", 20, 80, 250, 3, 1.5, (28, 28), 28),
    "hurco-vm15di": (VM, "VM15Di", (900, 406), (102,), (762, 407, 508), 12000, (11, 1500), "CAT40", 24, 80, 250, 7, 2.5, (36, 30), 30),
    "hurco-vm20i": (VM, "VM20i", (1168, 508), (102,), (1016, 508, 508), 12000, (11, 1450), "CAT40", 24, 80, 250, 7, 2.5, (28, 28), 28),
    "hurco-vm30i": (VM, "VM30i", (1321, 508), (102,), (1270, 508, 508), 12000, (11, 1450), "CAT40", 24, 80, 250, 7, 2.5, (28, 28), 28),
    "hurco-vmx24i": (VMX, "VMX24i", (760, 508), (152,), (610, 508, 610), 12000, (11, 1450), "CAT40", 30, 80, 300, 7, 2, (30, 20), 20),
    "hurco-vmx30i": (VMX, "VMX30i", (1020, 508), (152,), (762, 508, 610), 12000, (7.5, 600), "CAT40", 40, 80, 300, 7, 2, (38, 32), 32),
    "hurco-vmx42i": (VMX, "VMX42i", (1270, 610), (152,), (1067, 610, 610), 12000, (11, 1450), "CAT40", 40, 80, 300, 7, 2, (30, 20), 20),
    "hurco-vmx50i": (VMX, "VMX50i", (1500, 660), (101,), (1270, 660, 610), 12000, (11, 720), "CAT40", 40, 76, 300, 7, 2, (32, 24), 24),
    "hurco-vmx50i-50t": (VMX, "VMX50i-50T", (1500, 660), (101,), (1270, 660, 610), 8000, (11, 600), "CAT50", 30, 125, 300, 15, 4, (32, 24), 24),
}
# 5-axis swivel head (B) + rotary table (C): table, nose min/max, travels, rpm, P_cont @ rpm,
# taper, pockets, tool dia/len, kg, s, rapids XY/Z, feed, B range, B rpm, C rpm, rotary dia
SRTS = {
    "hurco-vmx42srti": ("VMX42SRTi", (1270, 610), (10, 620), (1067, 610, 610), 12000, (24, 2900), "CAT40", 40, 76, 249, 7, 4, (38, 32), 32, 92, 50, 100, 599),
    "hurco-vmx42hsrti": ("VMX42HSRTi", (1270, 610), (25, 612), (1067, 610, 610), 24000, (28, 4260), "HSK63", 40, 76, 249, 7, 2, (45, 40), 40, 92, 50, 100, 599),
    "hurco-vmx60srti": ("VMX60SRTi", (1676, 660), (13, 622), (1524, 660, 610), 12000, (24, 2900), "CAT40", 40, 75, 249, 7, 4, (32, 24), 24, 92, 90, 100, 599),
}
# lathes: swing over bed, over cross slide, max turning dia, max turning length, bar, X, Z travel,
# rpm, P_cont @ rpm, nose, chuck, stations, rapids X/Z
LATHES = {
    "hurco-tm6i": ("TM6i", 405, 240, 316, 340, 45, 178, 356, 6000, (7.5, 1090), "A2-5", 170, 12, (30, 30)),
    "hurco-tm8i": ("TM8i", 525, 300, 356, 525, 64, 203, 550, 4000, (15, 1000), "A2-6", 210, 12, (30, 30)),
    "hurco-tm10i": ("TM10i", 582, 402, 450, 758, 76, 250, 790, 3000, (15, 600), "A2-8", 260, 12, (30, 30)),
    "hurco-tm12i": ("TM12i", 680, 510, 510, 1000, 102, 305, 1016, 2800, (30, 868), "A2-11", 315, 12, (30, 30)),
}
CONFLICTS = {
    "hurco-vm20i": [("joint.X.range", "X-axis travel printed '40 in / 1,106 mm' (40 in = 1,016 mm); the record holds 1,016 mm, the smaller")],
    "hurco-vmx50i-50t": [("spindle.S1.curve", "peak torque printed '294.4 ft. lbs (217.2 Nm)' (294.4 ft-lb = 399 Nm); the curve uses only the continuous power rating, not the torque")],
    "hurco-tm8i": [("spindle.main.curve", "torque printed '133.83 ft. lbs (100.7 Nm)' and continuous '105.65 ft. lbs (43.2 Nm) @ 1000 rpm' (105.65 ft-lb = 143 Nm, matching 15 kW at 1000 rpm); the curve uses only the continuous power rating")],
}
CONTROL = ("Hurco's WinMax control runs 'Industry Standard NC' programs (line card); forge-post has no Hurco dialect: "
           "the Fanuc-style dialect stands in. Verify the NC subset")
LAYOUT = ("table X on saddle Y, head Z on the column (Hurco VM/VMX layout, not stated on the line card; ISO 841 "
          "directions with the table moving opposite the program's tool motion)")
HOME = "the travel is published; machine zero's place in it is not: -travel..0 (+ end) is a stand-in"


def curve(r, path, url, p, at, top):
    r.cite(path, "estimate", source=url,
           value=f"spindle power (30 min/continuous): continuous {p:g} kW @ {at:g} rpm",
           basis=(f"from the continuous rating only: power rises linearly to {p:g} kW at {at:g} rpm (constant torque "
                  f"{p * 1000 / (at * math.tau / 60):.1f} N·m), then stays flat; never above the published continuous rating "
                  "(the 30-min peak is higher: errs low)"))
    return [0, at, top], [0, p, p]


def mills(r, row, fivex=None):
    url = VM if row[0] == VM else VMX if row[0] == VMX else SRT
    if fivex:
        (model, (tl, tw), (nmin, nmax), (tx, ty, tz), top, (pk, prpm), taper, pockets, tdia, tlen, kg, tts,
         (rxy, rz), feed, brange, brpm, crpm, rdia) = fivex
        url = SRT
    else:
        (_, model, (tl, tw), nose, (tx, ty, tz), top, (pk, prpm), taper, pockets, tdia, tlen, kg, tts, (rxy, rz), feed) = row
        nmin = nose[0]
        nmax = nmin + tz
    ax, ay = tx / 2, ty / 2
    kind = "head-table" if fivex else "vmc3"
    H.table(r, "machine", name=r.name, kind=kind, control="fanuc", real=True, length_joint="Z", coolants=[], clearance=1.0,
            source=url, taken=TAKEN,
            note=(f"forgecad's Hurco {model} from Hurco's public line card; every value cited. Base frame = table top centre at "
                  "home; +x right, +y toward the column, +z up. Coolant options not declared."))
    r.cite("machine.control", "estimate", basis=CONTROL)
    r.cite("machine.kinematics", "estimate",
           basis=LAYOUT + ("; B swivels the head about y (the card's B-axis +/-92 deg), C turns the rotary table on the X "
                           "table" if fivex else ""))
    for n, parent, axis, t, rate in (("Y", "base", [0, -1, 0], ty, rxy), ("X", "Y", [-1, 0, 0], tx, rxy),
                                      ("Z", "base", [0, 0, 1], tz, rz)):
        H.table(r, "machine.joint", name=n, parent=parent, side="head" if n == "Z" else "table", type="linear", axis=axis,
                range=[-t, 0], home=0, rapid=rate * 1000, max_feed=min(feed, rate) * 1000)
        r.cite(f"joint.{n}.range", "published", source=url, value=f"{model} {n}-axis travel {t:g} mm", note=HOME)
        r.cite(f"joint.{n}.rapid", "published", source=url, value=f"{model} rapid traverse {n}: {rate:g} m/min")
        r.cite(f"joint.{n}.max_feed", "published", source=url, value=f"{model} programmable feedrate: {feed:g} m/min")
    gauge = [ax, ay, nmax]
    head_link = "Z"
    if fivex:
        piv = 250.0
        H.table(r, "machine.joint", name="B", parent="Z", side="head", type="rotary", axis=[0, 1, 0],
                pivot=[ax, ay, nmax + piv], range=[-brange, brange], home=0, rapid=brpm * 360, max_feed=brpm * 360, tcp=True)
        r.cite("joint.B.range", "published", source=url, value=f"{model} B-axis: +/-{brange} deg")
        r.cite("joint.B.rapid", "published", source=url, value=f"{model} B-axis maximum rpm: {brpm} (x 360 = deg/min)")
        r.cite("joint.B.max_feed", "unknown", basis="no rotary feed limit published; taken equal to the rapid (errs permissive)")
        r.cite("joint.B.pivot", "unknown", basis=f"the B pivot's distance above the spindle nose (pivot length) is unpublished; {piv:g} mm stands in. Every tilted verdict depends on it")
        H.table(r, "machine.joint", name="C", parent="X", side="table", type="rotary", axis=[0, 0, -1], pivot=[0, 0, 0],
                range=[-360, 360], home=0, rapid=crpm * 360, max_feed=crpm * 360, cyclic=True, tcp=True)
        r.cite("joint.C.range", "published", source=url, value=f"{model} C-axis: 360 deg")
        r.cite("joint.C.rapid", "published", source=url, value=f"{model} C-axis maximum rpm: {crpm} (x 360 = deg/min)")
        r.cite("joint.C.max_feed", "unknown", basis="no rotary feed limit published; taken equal to the rapid")
        r.cite("joint.C.pivot", "unknown", basis="where the rotary table sits on the X table is unpublished: its centre at the table centre stands in")
        head_link = "B"
    cr, ck = curve(r, "spindle.S1.curve", url, pk, prpm, top)
    H.table(r, "machine.spindle", name="S1", link=head_link, kind="milling", axis=[0, 0, 1], nose=gauge, rpm=[1, top],
            curve_rpm=cr, curve_kw=ck, max=[tlen, tdia], interface=taper)
    r.cite("spindle.S1.rpm", "published", source=url, value=f"{model} maximum spindle speed {top:g} rpm", note=H.MIN_RPM)
    r.cite("spindle.S1.nose", "published", source=url,
           value=f"{model} spindle nose to table: {nmin:g} mm" + (f" / {nmax:g} mm" if fivex else ""),
           note=("min + Z travel = the height at Z = 0" if not fivex else "") + "; x, y: " + H.CENTRED)
    r.cite("spindle.S1.max", "published", source=url, value=f"{model} max tool diameter {tdia:g} mm, length {tlen:g} mm")
    r.cite("spindle.S1.interface", "published", source=url, value=f"{model} tool type {taper}")
    H.table(r, "machine.changer", pockets=pockets, change_s=tts, max_tool_kg=kg)
    r.cite("changer.pockets", "published", source=url, value=f"{model} tool capacity {pockets}")
    r.cite("changer.change_s", "estimate", source=url, value=f"{model} tool-to-tool ATC time {tts:g} s",
           basis="Hurco publishes tool-to-tool, not chip-to-chip: the real change with spindle stop/start is longer (errs short)")
    r.cite("changer.max_tool_kg", "published", source=url, value=f"{model} maximum tool weight {kg:g} kg")
    H.table(r, "machine.mount", name="spindle", kind="tool", link=head_link, at=gauge, dir=[0, 0, -1])
    r.cite("mount.spindle.at", "published", source=url, value=f"{model} spindle nose to table", note="gauge point at the nose face; x, y: " + H.CENTRED)
    part_link = "C" if fivex else "X"
    H.table(r, "machine.mount", name="table", kind="part", link=part_link, at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.cite("mount.table.at", "estimate", basis="the table (rotary table) top centre is the frame origin")
    th = 90.0
    lo, hi = [-tl / 2, -tw / 2, -th], [tl / 2, tw / 2, 0.0]
    if fivex:
        # the rotary table: its top is the part datum; the X table under it, 80 mm lower (stand-in)
        lo, hi = [-tl / 2, -tw / 2, -80 - th], [tl / 2, tw / 2, -80.0]
    H.body(r, "table", "X", [("box", lo, hi)],
           H.fg_box(f"{r.name} table: {tl:g} x {tw:g} published, thickness estimated", *H.shrink_box(lo, hi), cut=(0, 0, tl, 18, 12)),
           ignore=["rotary"] if fivex else [], status="published", source=url, value=f"{model} table size {tl:g} x {tw:g} mm")
    r.cite("body.table", "estimate", basis=f"thickness {th:g} mm unpublished" + ("; on the SRT the rotary table stands 80 mm above it (stand-in)" if fivex else ""))
    if fivex:
        H.body(r, "rotary", "C", [("cyl", [0, 0, -80], [0, 0, 1], rdia / 2, 80)],
               H.fg_cyl_z(f"{r.name} rotary table: {rdia:g} mm (published), 80 high (estimate)", 0, 0, -79, rdia / 2 - 1, 78, bore=20),
               ignore=["table"], status="published", source=url, value=f"{model} rotary table diameter {rdia:g} mm")
        r.cite("body.rotary", "estimate", basis="height 80 mm and placement at the table centre are stand-ins")
    nr, nh = 95.0, 160.0
    hb_lo, hb_hi = [ax - 190, ay - 190, nmax + nh], [ax + 190, ay + 190, nmax + nh + 380]
    H.body(r, "head", head_link, [("cyl", [ax, ay, nmax], [0, 0, 1], nr, nh), ("box", hb_lo, hb_hi)],
           H.fg_head(f"{r.name} head (unpublished sizes, conservative)", ax, ay, nmax + 1, nr - 1, nh - 1,
                     [hb_lo[0] + 1, hb_lo[1] + 1, hb_lo[2]], [hb_hi[0] - 1, hb_hi[1] - 1, hb_hi[2] - 1]),
           status="unknown", basis="head size unpublished: the 40-taper stand-in of the Haas records (errs large)")


def lathe(r, row):
    (model, swing, cross, tdia, tlen, bar, tx, tz, top, (pk, prpm), nose, chuck, st, (rx, rz)) = row
    url = TM
    H.table(r, "machine", name=r.name, kind="lathe", control="fanuc-lathe", real=True, coolants=[], clearance=1.0,
            source=url, taken=TAKEN,
            note=(f"forgecad's Hurco {model} from Hurco's public line card; every value cited. Base frame = spindle centre at "
                  "the chuck face; +z away from the headstock, +x toward the turret."))
    r.cite("machine.control", "estimate", basis=CONTROL + " (lathe: Fanuc-style lathe dialect)")
    r.cite("machine.kinematics", "estimate", basis="turret on cross (X) and long (Z) slides, the spindle fixed; not stated on the card")
    rmax = tdia / 2
    H.table(r, "machine.joint", name="Z", parent="base", side="head", type="linear", axis=[0, 0, 1], range=[0, tz], home=tz,
            rapid=rz * 1000, max_feed=rz * 1000)
    r.cite("joint.Z.range", "published", source=url, value=f"{model} Z-axis travel {tz:g} mm; max turning length {tlen:g} mm",
           note="placed from the chuck face to +travel (estimate)")
    r.cite("joint.Z.rapid", "published", source=url, value=f"{model} rapid traverse Z {rz:g} m/min")
    r.cite("joint.Z.max_feed", "unknown", basis="no lathe feed limit on the card; taken equal to the rapid (errs permissive)")
    H.table(r, "machine.joint", name="X", parent="Z", side="head", type="linear", axis=[1, 0, 0], range=[round(rmax - tx, 1), rmax],
            home=rmax, rapid=rx * 1000, max_feed=rx * 1000, diameter=True)
    r.cite("joint.X.range", "published", source=url, value=f"{model} X-axis travel {tx:g} mm; max turning diameter {tdia:g} mm",
           note="the upper end at the max turning radius, the lower = upper - travel (estimate)")
    r.cite("joint.X.rapid", "published", source=url, value=f"{model} rapid traverse X {rx:g} m/min")
    r.cite("joint.X.max_feed", "unknown", basis="no lathe feed limit on the card; taken equal to the rapid (errs permissive)")
    cr, ck = curve(r, "spindle.main.curve", url, pk, prpm, top)
    H.table(r, "machine.spindle", name="main", link="base", kind="turning", axis=[0, 0, 1], nose=[0, 0, 0], rpm=[1, top],
            curve_rpm=cr, curve_kw=ck, max=[swing, tlen], interface=nose)
    r.cite("spindle.main.rpm", "published", source=url, value=f"{model} maximum speed high gear {top:g} rpm", note=H.MIN_RPM)
    r.cite("spindle.main.nose", "estimate", basis="the chuck face centre is the frame origin")
    r.cite("spindle.main.max", "published", source=url, value=f"{model} swing over bed {swing:g} mm; max turning length {tlen:g} mm")
    r.cite("spindle.main.interface", "published", source=url, value=f"{model} spindle nose {nose}")
    cx = 230.0
    H.table(r, "machine.turret", name="T1", link="X", stations=st, index_s=0.5, center=[cx, 0, 0], index_axis=[0, 0, 1])
    r.cite("turret.T1.stations", "published", source=url, value=f"{model} tool capacity {st} stations (slotted disc)")
    r.cite("turret.T1.index_s", "unknown", basis="index time unpublished; 0.5 s stand-in")
    r.cite("turret.T1.center", "unknown", basis="turret geometry unpublished; centre 230 mm behind the reference tool point (stand-in)")
    H.table(r, "machine.mount", name="turret", kind="tool", link="X", at=[0, 0, 0], dir=[-1, 0, 0])
    r.cite("mount.turret.at", "estimate", basis="the reference tool point the X/Z joint values place")
    H.table(r, "machine.mount", name="chuck", kind="part", link="base", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.cite("mount.chuck.at", "estimate", basis="the chuck face centre is the frame origin")
    L = 110.0
    H.body(r, "chuck", "base", [("cyl", [0, 0, -L], [0, 0, 1], chuck / 2 + 5, L)],
           H.fg_cyl_z(f"{r.name} chuck: {chuck:g} mm (published) + 5 mm, {L:g} long (estimate)", 0, 0, -L + 1, chuck / 2 + 4, L - 2, bore=15),
           status="published", source=url, value=f"{model} chuck diameter {chuck:g} mm")
    r.cite("body.chuck", "estimate", basis="length and a 5 mm jaw margin are stand-ins")
    R, TT = 175.0, 110.0
    H.body(r, "turret", "X", [("cyl", [cx, 0, -TT / 2], [0, 0, 1], R, TT)],
           H.fg_cyl_z(f"{r.name} turret disc (unpublished, stand-in)", cx, 0, -TT / 2 + 1, R - 1, TT - 2, bore=25),
           status="unknown", basis=f"slotted-disc turret, size unpublished: r {R:g} x {TT:g} mm, rim {cx - R:g} mm behind the reference point (stand-in)")


def main():
    text = [
        "# Hurco machines: forgecad's records from Hurco's public line cards (cms.hurco.com), restated (never copied).",
        "# GENERATED by tools/machines/hurco.py; edit its facts, then re-run. Every value carries a [[machine.cite]].",
        "",
    ]
    todo = [(n, mills, (row,)) for n, row in MILLS.items()]
    todo += [(n, mills, (None, row)) for n, row in SRTS.items()]
    todo += [(n, lathe, (row,)) for n, row in LATHES.items()]
    for name, build, args in todo:
        r = H.Rec(name, {"url": "", "specs": {}})
        if build is mills and args[0] is None:
            mills(r, (SRT,), fivex=args[1])
        else:
            build(r, *args)
        for path, note in CONFLICTS.get(name, []):
            r.cite(path, "conflict", source={"hurco-tm8i": TM, "hurco-vm20i": VM, "hurco-vm10hsi": VM}.get(name, VMX), note=note,
                   value="the card's own figures disagree")
        text.append("# " + "-" * 93)
        text += r.lines
        os.makedirs(os.path.join(PARTS, name), exist_ok=True)
        for (bname, link, prims, ignore, part) in r.bodies:
            text += [s.replace("parts/haas/", "parts/hurco/") for s in H.prim_toml(r, bname, link, prims, ignore, part)]
            with open(os.path.join(PARTS, name, f"{bname}.fg"), "w") as fh:
                fh.write(part)
        for c in r.cites:
            text.append("[[machine.cite]]")
            for k in ("path", "status", "source", "taken", "value", "basis", "note"):
                if c.get(k):
                    text.append(f"{k} = {H.tv(c[k])}")
        text.append("")
    with open(OUT, "w") as fh:
        fh.write("\n".join(text))
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
