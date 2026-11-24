#!/usr/bin/env python3
"""Generates kernel/forge-machine/data/machines/tormach.toml and parts/tormach/<machine>/ from
the facts below, restated from tormach.com's machine pages (read 2026-11-23). Same rules as
tools/machines/haas.py (whose helpers this uses): every value cited, estimates and unknowns
marked with their basis, nothing from memory.

    python3 tools/machines/tormach.py
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
import haas as H  # noqa: E402  (helpers: Rec, table, tv, fg_*, body, shrink_box)

ROOT = H.ROOT
OUT = os.path.join(ROOT, "kernel/forge-machine/data/machines/tormach.toml")
PARTS = os.path.join(ROOT, "kernel/forge-machine/parts/tormach")
TAKEN = "2026-11-23"
T = "https://tormach.com/machines/"

# Facts: (value in mm / mm/min / rpm / kW, the page's statement restated). None = not published.
MILLS = {
    "tormach-pcnc-440": dict(
        url=T + "mills/pcnc-440.html", x=(254, "X-Axis: 10 in. (254 mm)"), y=(158.75, "Y-Axis: 6.25 in. (158.75 mm)"),
        z=(254, "Z-Axis: 10 in. (254 mm)"), fxy=(3430, "Max feed X and Y: 135 IPM (3.43 m/min)"),
        fz=(2790, "Max feed Z: 110 IPM (2.79 m/min)"), rpm=(10000, "Maximum Speed: 10,000 RPM"),
        taper="R8", kw=(0.56, "Spindle Power: 0.75 hp (0.56 kW)"), table=((458, 160), "Table Size: 18 in x 6.30 in. (458 mm x 160 mm)"),
        nose=(305, "Spindle Nose to Table Max Clearance: 12 in. (305 mm)"), col=(165, "Spindle Centerline to Machine Column: 6.5 in. (165 mm)"),
        atc="8-pocket automatic tool changer (in packages; optional)"),
    "tormach-770m": dict(
        url=T + "mills/770m.html", x=(356, "X-Axis: 14 in. (356 mm)"), y=(191, "Y-Axis: 7.5 in. (191 mm)"),
        z=(337, "Z-Axis: 13.25 in. (337 mm)"), fxy=(3430, "Max feed X and Y: 135 IPM (3.43 m/min)"),
        fz=(2790, "Max feed Z: 110 IPM (2.79 m/min)"), rpm=(10000, "Maximum Speed: 10,000 RPM"), taper="R8",
        kw=(1.12, "Spindle Power: 1.5 hp / 1.12 kW"), table=((660, 203), "Table Size: 26 in. x 8 in. / 660 mm x 203 mm"),
        nose=(362, "Spindle Nose to Table Max Clearance: 14.25 in. / 362 mm"), col=(218, "Spindle Centerline to Machine Column: 8.6 in. / 218 mm"),
        atc=None),
    "tormach-770mx": dict(
        url=T + "mills/770mx.html", x=(356, "X-Axis: 14 in. (356 mm)"), y=(191, "Y-Axis: 7.5 in. (191 mm)"),
        z=(337, "Z-Axis: 13.25 in. (337 mm)"), fxy=(11400, "Max feed X and Y: 450 IPM (11.4 m/min)"),
        fz=(10600, "Max feed Z: 420 IPM (10.6 m/min)"), rpm=(10000, "Maximum Speed: 10,000 RPM"), taper="BT30",
        kw=(1.49, "Spindle Power: 2 hp / 1.49 kW (peak 4.5 hp / 3.35 kW)"), table=((660, 203), "Table Size: 26 in. x 8 in. (660 mm x 203 mm)"),
        nose=(349, "Spindle Nose to Table Max Clearance: 13.75 in. (349 mm)"), col=(218, "Spindle Centerline to Machine Column: 8.6 in. (218 mm)"),
        atc=None),
    "tormach-1100m": dict(
        url=T + "mills/1100m.html", x=(457, "X-Axis: 18 in. (457 mm)"), y=(279, "Y-Axis: 11 in. (279 mm)"),
        z=(412, "Z-Axis: 16.25 in. (412 mm)"), fxy=(2790, "Max feed X and Y: 110 IPM (2.79 m/min)"),
        fz=(2290, "Max feed Z: 90 IPM (2.29 m/min)"), rpm=(7500, "Maximum Speed: 7500 RPM"), taper="R8",
        kw=(1.5, "Spindle Power: 2 hp (1.5 kW)"), table=None, nose=None, col=None, atc=None),
    "tormach-1100mx": dict(
        url=T + "mills/1100mx.html", x=(457, "X-Axis: 18 in. (457 mm)"), y=(279, "Y-Axis: 11 in. (279 mm)"),
        z=(413, "Z-Axis: 16.25 in. (413 mm)"), fxy=(7620, "Max feed X and Y: 300 IPM (7.62 m/min)"),
        fz=(5840, "Max feed Z: 230 IPM (5.84 m/min)"), rpm=(10000, "Maximum Speed: 10,000 RPM"), taper="BT30",
        kw=(1.49, "Spindle Power: 2 hp / 1.49 kW"), table=((863, 241), "Table Size: 34 in. x 9.5 in. (863 mm x 241 mm)"),
        nose=(438, "Spindle Nose to Table Max Clearance: 17.25 in. (438 mm)"), col=(279, "Spindle Centerline to Machine Column: 11 in. (279 mm)"),
        atc="12-pocket automatic tool changer (in packages; optional)"),
    "tormach-1500mx": dict(
        url=T + "mills/1500mx.html", x=(500, "X-Axis: 19.7 in. (500 mm)"), y=(350, "Y-Axis: 13.8 in. (350 mm)"),
        z=(356, "Z-Axis: 14 in. (356 mm)"), fxy=(30500, "Max feed X and Y: 1,200 IPM (30.5 m/min)"),
        fz=(15200, "Max feed Z: 600 IPM (15.2 m/min)"), rpm=(10000, "Spindle Speed: 250-10,000 RPM"), rpm_min=250,
        taper="BT30", kw=(3.0, "Continuous Power: 4 hp (3 kW); Peak Power: 6 hp (4.5 kW)"),
        table=((889, 324), "Table Size: 35 in. x 12.75 in. (0.9 m x 0.3 m)"),
        nose=(450, "Spindle Nose to Table: 3.7-17.7 in. (96-450 mm)"), col=None,
        atc="16-pocket automatic tool changer (ATC) (optional accessory)"),
}

ROUTERS = {
    "tormach-24r": dict(
        url=T + "routers/24r-cnc-router.html", x=(625, "X-Axis: 24.75 in. (625 mm)"), y=(1415, "Y-Axis: 55.75 in. (1415 mm)"),
        z=(170, "Z-Axis: 6.70 in. (170 mm)"), f=(5000, "Maximum Feed Rate: 200 IPM (5.0 m/min)"),
        rpm=((10000, 24000), "Minimum Speed: 10,000 RPM; Maximum Speed: 24,000 RPM"), taper="ER20",
        table=((680, 1650), "Table Size: 26.5 in. x 65 in. (680 mm x 1650 mm)"), gantry=(154, "Gantry Clearance: 6 in. (154 mm)")),
    "tormach-xstech": dict(
        url=T + "routers/xstech-router.html", x=(220, "X-Axis: 8.6 in. (220 mm)"), y=(160, "Y-Axis: 6.3 in. (160 mm)"),
        z=(61, "Z-Axis: 2.4 in. (61 mm)"), f=None, rpm=((1, 20000), "Maximum Speed: 20,000 RPM"), taper="ER11",
        table=((320, 150), "Table Size: 12.6 in. x 5.9 in. (320 mm x 150 mm)"), gantry=None),
    "tormach-xstech-pro": dict(
        url=T + "routers/xstech-pro-cnc-router.html", x=(259, "X-Axis: 10.2 in. (259 mm)"), y=(237, "Y-Axis: 9.3 in. (237 mm)"),
        z=(160, "Z-Axis: 6.3 in. (160 mm)"), f=None, rpm=((1, 20000), "Maximum Speed: 20,000 RPM"), taper="ISO 10",
        table=None, gantry=None),
}

LATHES = {
    "tormach-8l": dict(
        url=T + "lathes/8l-lathe.html", x=(114, "X-Axis: 4.5 in. (114 mm)"), z=(254, "Z-Axis: 10 in. (254 mm) with tailstock"),
        f=(3800, "X- and Z-Axis Maximum Feed Rate: 150 IPM (3.8 m/min)"), rpm=((180, 5000), "Spindle Speed: 180 to 5,000 rpm"),
        kw=(1.11, "Spindle Power: 1.5 hp (1.11 kW)"), nose="5C", swing=(200, "Maximum Swing Over Bed: 8 in. (200 mm)"),
        carriage=(100, "Maximum Swing Over Carriage: 4 in. (100 mm)"), chuck=None, stations=None),
    "tormach-15lx-turret": dict(
        url=T + "lathes/15lx-lathe.html", x=(254, "X-Axis Travel: 10 in. (254 mm)"), z=(356, "Z-Axis Travel: 14 in. (356 mm)"),
        f=(9140, "Max Feed Rate: 360 IPM (9.14 m/min) (X), 360 IPM (9.14 m/min) (Z)"),
        rpm=((1, 3500), "Spindle Speed: Max: 3,500 RPM"), kw=(4.5, "Spindle Power: Continuous: 6 hp (4.5 kW), Peak: 10 hp (7.4 kW)"),
        nose="D1-4", swing=(381, "Swing Over Bed: 15 in. (381 mm) Diameter"),
        carriage=(157.5, "Swing Over Carriage: 6.2 in. (157.5 mm) Diameter"),
        chuck=(152, "Recommended Chuck Size: 6 in. (152 mm)"), stations=(8, "8-Station Turret (Optional)")),
}

CONTROL_BASIS = ("PathPilot is Tormach's control; it is LinuxCNC-based per Tormach's documentation (not on the "
                 "spec page), so the linuxcnc dialect is used. Verify its G-code subset")
LAYOUT_MILL = ("bed mill: the table moves X on a saddle that moves Y, the head moves Z on a fixed column. Not stated "
               "on the spec page; ISO 841 directions with the table moving opposite the program's tool motion")
HOME_UNKNOWN = ("the travel is published; where machine zero sits in it is not on the spec page: placed at the "
                "+ end (-travel..0) like most mills. WCS values carry the difference")


def cite_pub(r, path, url, stmt, note=""):
    r.cite(path, "published", source=url, value=stmt, note=note)


def mill(r, f):
    url = f["url"]
    tx, ty, tz = f["x"][0], f["y"][0], f["z"][0]
    ax, ay = tx / 2, ty / 2
    H.table(r, "machine", name=r.name, kind="vmc3", control="linuxcnc", real=True, length_joint="Z", coolants=[],
            clearance=1.0, source=url, taken=TAKEN,
            note=("forgecad's Tormach record from tormach.com's spec page; every value cited. Base frame = table top centre "
                  "at home; +x right, +y toward the column, +z up. Coolant, ATC and enclosure are options: none declared."))
    r.cite("machine.control", "estimate", basis=CONTROL_BASIS)
    r.cite("machine.kinematics", "estimate", basis=LAYOUT_MILL)
    for n, parent, axis, t, rate in (("Y", "base", [0, -1, 0], f["y"], f["fxy"]), ("X", "Y", [-1, 0, 0], f["x"], f["fxy"]),
                                      ("Z", "base", [0, 0, 1], f["z"], f["fz"])):
        H.table(r, "machine.joint", name=n, parent=parent, side="head" if n == "Z" else "table", type="linear", axis=axis,
                range=[-t[0], 0], home=0, rapid=rate[0], max_feed=rate[0])
        cite_pub(r, f"joint.{n}.range", url, t[1], note=HOME_UNKNOWN)
        cite_pub(r, f"joint.{n}.rapid", url, rate[1], note="Tormach publishes one 'maximum feed rate' per axis: used for rapid and feed")
        cite_pub(r, f"joint.{n}.max_feed", url, rate[1])
    if f["nose"]:
        nmax = f["nose"][0]
        cite_pub(r, "spindle.S1.nose", url, f["nose"][1], note="nose at Z = 0 (top); x, y: " + H.CENTRED)
        cite_pub(r, "mount.spindle.at", url, f["nose"][1], note="gauge point at the nose face; x, y: " + H.CENTRED)
    else:
        nmax = tz + 25
        r.cite("spindle.S1.nose", "unknown", basis=f"nose-to-table clearance not on the 1100M page; Z travel + 25 mm stands in ({nmax:g} mm), like the 1100MX's published 438 - 413")
        r.cite("mount.spindle.at", "unknown", basis="as spindle.S1.nose")
    nose = [ax, ay, nmax]
    rpm_min = f.get("rpm_min", 1)
    H.table(r, "machine.spindle", name="S1", link="Z", kind="milling", axis=[0, 0, 1], nose=nose, rpm=[rpm_min, f["rpm"][0]],
            max=[100, 50], interface=f["taper"])
    cite_pub(r, "spindle.S1.rpm", url, f["rpm"][1], note="" if rpm_min > 1 else H.MIN_RPM)
    r.cite("spindle.S1.max", "unknown", basis="tool length and diameter limits are not published; 100 x 50 mm stand in (errs small: refuses larger tools)")
    cite_pub(r, "spindle.S1.interface", url, "Spindle Taper: " + f["taper"])
    r.cite("machine.power", "published", source=url, value=f["kw"][1],
           note="power published without a torque figure: no power curve declared (no power check), rather than a flat curve that would overstate low-rpm power")
    if f["atc"]:
        r.cite("machine.changer", "published", source=url, value=f["atc"],
               note="the ATC is optional: no changer record (pocket count, time and tool weight for it are not on the page)")
    H.table(r, "machine.mount", name="spindle", kind="tool", link="Z", at=nose, dir=[0, 0, -1])
    H.table(r, "machine.mount", name="table", kind="part", link="X", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.cite("mount.table.at", "estimate", basis="the table top centre is the frame origin")
    if f["table"]:
        tl, tw = f["table"][0]
        th = 60.0
        lo, hi = [-tl / 2, -tw / 2, -th], [tl / 2, tw / 2, 0.0]
        H.body(r, "table", "X", [("box", lo, hi)],
               H.fg_box(f"{r.name} table: {tl:g} x {tw:g} published, thickness estimated; one T-slot", *H.shrink_box(lo, hi),
                        cut=(0, 0, tl, 16, 12)),
               status="published", source=url, value=f["table"][1])
        r.cite("body.table", "estimate", basis=f"thickness {th:g} mm unpublished")
    else:
        tl, tw, th = tx + 200, ty, 60.0
        lo, hi = [-tl / 2, -tw / 2, -th], [tl / 2, tw / 2, 0.0]
        H.body(r, "table", "X", [("box", lo, hi)],
               H.fg_box(f"{r.name} table (size unpublished on its page)", *H.shrink_box(lo, hi), cut=(0, 0, tl, 16, 12)),
               status="unknown", basis="table size not on the page: X travel + 200 by the Y travel, 60 thick (stand-in)")
    nr, nh = 55.0, 120.0
    back = f["col"][0] if f["col"] else 250.0
    hb_lo, hb_hi = [ax - 120, ay - 120, nmax + nh], [ax + 120, ay + back - 5, nmax + nh + 300]
    H.body(r, "head", "Z", [("cyl", [ax, ay, nmax], [0, 0, 1], nr, nh), ("box", hb_lo, hb_hi)],
           H.fg_head(f"{r.name} head: nose cylinder and housing back to the column (sizes unpublished)", ax, ay, nmax + 1,
                     nr - 1, nh - 1, [hb_lo[0] + 1, hb_lo[1] + 1, hb_lo[2]], [hb_hi[0] - 1, hb_hi[1] - 1, hb_hi[2] - 1]),
           ignore=["column"] if f["col"] else [], status="unknown",
           basis=(f"head size unpublished: nose r {nr:g} x {nh:g} mm, housing 240 mm wide reaching back to the column; "
                  "errs large for an R8/BT30 nose"))
    if f["col"]:
        cy = ay + f["col"][0]
        clo, chi = [-200, cy, -th - 300], [200, cy + 250, nmax + nh + 600]
        H.body(r, "column", "base", [("box", clo, chi)],
               H.fg_box(f"{r.name} column: its face at the published centreline distance; size unpublished", *H.shrink_box(clo, chi)),
               ignore=["head"], status="published", source=url, value=f["col"][1])
        r.cite("body.column", "estimate",
               basis="the face's position is published (spindle centreline to column, with the travel taken centred); width 400 and depth 250 are stand-ins")


def router(r, f):
    """A gantry router: Y moves the gantry along the table, X the carriage on it, Z the spindle;
    the table is fixed (part mount on the base)."""
    url = f["url"]
    tx, ty, tz = f["x"][0], f["y"][0], f["z"][0]
    H.table(r, "machine", name=r.name, kind="vmc3", control="linuxcnc", real=True, length_joint="Z", coolants=[], clearance=1.0,
            source=url, taken=TAKEN,
            note=("forgecad's Tormach router record from tormach.com; every value cited. Base frame = table top centre; the "
                  "spindle over the table centre at mid-travel. Gantry routers: the paired gantry motors are one joint "
                  "(FORMAT-GAPS #4)."))
    r.cite("machine.control", "estimate", basis=CONTROL_BASIS)
    r.cite("machine.kinematics", "estimate" if f["gantry"] else "unknown",
           basis=("moving gantry: Y the gantry, X the carriage on it, Z the spindle; the table fixed" +
                  (" (the page gives a gantry clearance)" if f["gantry"] else
                   "; whether this machine moves its table instead is not on the page (UNKNOWN)")))
    rate = f["f"][0] if f["f"] else 2500.0
    for n, parent, axis, t in (("Y", "base", [0, 1, 0], f["y"]), ("X", "Y", [1, 0, 0], f["x"]), ("Z", "X", [0, 0, 1], f["z"])):
        H.table(r, "machine.joint", name=n, parent=parent, side="head", type="linear", axis=axis, range=[-t[0], 0], home=0,
                rapid=rate, max_feed=rate)
        cite_pub(r, f"joint.{n}.range", url, t[1], note=HOME_UNKNOWN)
        if f["f"]:
            cite_pub(r, f"joint.{n}.rapid", url, f["f"][1], note="one rate published for all axes: used for rapid and feed")
            cite_pub(r, f"joint.{n}.max_feed", url, f["f"][1])
        else:
            r.cite(f"joint.{n}.rapid", "unknown", basis="no feed rate on the page; 2500 mm/min stands in (time estimates only)")
            r.cite(f"joint.{n}.max_feed", "unknown", basis="as the rapid")
    nmax = tz + 30
    nose = [tx / 2, ty / 2, nmax]
    lo_rpm, hi_rpm = f["rpm"][0]
    H.table(r, "machine.spindle", name="S1", link="Z", kind="milling", axis=[0, 0, 1], nose=nose, rpm=[lo_rpm, hi_rpm],
            max=[60, 12.7], interface=f["taper"])
    cite_pub(r, "spindle.S1.rpm", url, f["rpm"][1], note="" if lo_rpm > 1 else H.MIN_RPM)
    r.cite("spindle.S1.nose", "unknown", basis=f"nose height unpublished: Z travel + 30 mm ({nmax:g}) above the table at the top of Z; x, y: " + H.CENTRED)
    r.cite("mount.spindle.at", "unknown", basis="as spindle.S1.nose")
    r.cite("spindle.S1.max", "unknown", basis="tool limits unpublished; 60 x 12.7 mm stand in (errs small)")
    cite_pub(r, "spindle.S1.interface", url, "Spindle Taper: " + f["taper"])
    H.table(r, "machine.mount", name="spindle", kind="tool", link="Z", at=nose, dir=[0, 0, -1])
    H.table(r, "machine.mount", name="table", kind="part", link="base", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.cite("mount.table.at", "estimate", basis="the table top centre is the frame origin")
    if f["table"]:
        tw, tl = f["table"][0] if r.name == "tormach-24r" else f["table"][0]
        if r.name != "tormach-24r":
            tl, tw = f["table"][0]
            lx, ly = tl, tw
        else:
            lx, ly = tw, tl  # 24R: 680 wide (x) by 1650 long (y, along the gantry travel)
        lo, hi = [-lx / 2, -ly / 2, -40.0], [lx / 2, ly / 2, 0.0]
        H.body(r, "table", "base", [("box", lo, hi)],
               H.fg_box(f"{r.name} table: {lx:g} x {ly:g} published, 40 thick (estimate)", *H.shrink_box(lo, hi)),
               status="published", source=url, value=f["table"][1])
        r.cite("body.table", "estimate", basis="thickness 40 mm unpublished; orientation: the long side along the longer travel")
    else:
        lx, ly = tx + 40, ty + 40
        lo, hi = [-lx / 2, -ly / 2, -40.0], [lx / 2, ly / 2, 0.0]
        H.body(r, "table", "base", [("box", lo, hi)], H.fg_box(f"{r.name} table (unpublished)", *H.shrink_box(lo, hi)),
               status="unknown", basis="table size not on the page: the travel + 40 mm (stand-in)")
    nr, nh = 40.0, 80.0
    hb_lo, hb_hi = [tx / 2 - 70, ty / 2 - 70, nmax + nh], [tx / 2 + 70, ty / 2 + 70, nmax + nh + 160]
    H.body(r, "spindle", "Z", [("cyl", [tx / 2, ty / 2, nmax], [0, 0, 1], nr, nh), ("box", hb_lo, hb_hi)],
           H.fg_head(f"{r.name} spindle and its carriage (unpublished sizes)", tx / 2, ty / 2, nmax + 1, nr - 1, nh - 1,
                     [hb_lo[0] + 1, hb_lo[1] + 1, hb_lo[2]], [hb_hi[0] - 1, hb_hi[1] - 1, hb_hi[2] - 1]),
           status="unknown", basis="spindle body unpublished: r 40 x 80 mm and a 140 mm carriage block (stand-in, errs large for an ER/ISO10-20 spindle)")


def lathe(r, f):
    url = f["url"]
    tx, tz = f["x"][0], f["z"][0]
    H.table(r, "machine", name=r.name, kind="lathe", control="fanuc-lathe", real=True, coolants=[], clearance=1.0,
            source=url, taken=TAKEN,
            note=("forgecad's Tormach lathe record from tormach.com; every value cited. Base frame = spindle centre at the "
                  "chuck/collet face; +z away from the headstock, +x toward the tool side."))
    r.cite("machine.control", "estimate",
           basis="PathPilot lathe (LinuxCNC-based); forge-post has no PathPilot lathe dialect: the Fanuc-style lathe dialect stands in (diameter X)")
    r.cite("machine.kinematics", "estimate", basis="tool post / turret on cross (X) and long (Z) slides, the spindle fixed; not stated on the page")
    rmax = f["carriage"][0] / 2
    H.table(r, "machine.joint", name="Z", parent="base", side="head", type="linear", axis=[0, 0, 1], range=[0, tz], home=tz,
            rapid=f["f"][0], max_feed=f["f"][0])
    cite_pub(r, "joint.Z.range", url, f["z"][1], note="placed from the chuck face (0) to +travel (estimate)")
    cite_pub(r, "joint.Z.rapid", url, f["f"][1], note="one rate published: rapid and feed")
    cite_pub(r, "joint.Z.max_feed", url, f["f"][1])
    H.table(r, "machine.joint", name="X", parent="Z", side="head", type="linear", axis=[1, 0, 0],
            range=[round(rmax - tx, 1), rmax], home=rmax, rapid=f["f"][0], max_feed=f["f"][0], diameter=True)
    cite_pub(r, "joint.X.range", url, f["x"][1] + "; " + f["carriage"][1],
             note="travel published; the upper end at the swing-over-carriage radius, the lower end = upper - travel (estimate)")
    cite_pub(r, "joint.X.rapid", url, f["f"][1])
    cite_pub(r, "joint.X.max_feed", url, f["f"][1])
    lo_rpm, hi_rpm = f["rpm"][0]
    H.table(r, "machine.spindle", name="main", link="base", kind="turning", axis=[0, 0, 1], nose=[0, 0, 0], rpm=[lo_rpm, hi_rpm],
            max=[f["swing"][0], tz], interface=f["nose"])
    cite_pub(r, "spindle.main.rpm", url, f["rpm"][1], note="" if lo_rpm > 1 else H.MIN_RPM)
    r.cite("spindle.main.nose", "estimate", basis="the chuck/collet face centre is the frame origin")
    cite_pub(r, "spindle.main.max", url, f["swing"][1] + "; " + f["z"][1], note="part length taken as the Z travel")
    cite_pub(r, "spindle.main.interface", url, "Spindle Nose: " + f["nose"])
    r.cite("machine.power", "published", source=url, value=f["kw"][1], note="no torque figure: no power curve declared")
    if f["stations"]:
        st = f["stations"][0]
        cite_pub(r, "turret.T1.stations", url, f["stations"][1], note="the turret is optional (gang tooling is the other choice; FORMAT-GAPS #5)")
        cx, R, TT = 150.0, 110.0, 80.0
    else:
        st = 1
        r.cite("turret.T1.stations", "estimate", basis="quick-change tool post (standard): one station, changed by hand; the format's lathe needs a turret")
        cx, R, TT = 90.0, 60.0, 70.0
    H.table(r, "machine.turret", name="T1", link="X", stations=st, index_s=1.0, center=[cx, 0, 0], index_axis=[0, 0, 1])
    r.cite("turret.T1.index_s", "unknown", basis="index time unpublished; 1 s stand-in")
    r.cite("turret.T1.center", "unknown", basis=f"turret/tool post geometry unpublished; {cx:g} mm behind the reference tool point (stand-in)")
    H.table(r, "machine.mount", name="turret", kind="tool", link="X", at=[0, 0, 0], dir=[-1, 0, 0])
    r.cite("mount.turret.at", "estimate", basis="the reference tool point the X/Z joint values place")
    H.table(r, "machine.mount", name="chuck", kind="part", link="base", at=[0, 0, 0], dir=[0, 0, 1], xdir=[1, 0, 0])
    r.cite("mount.chuck.at", "estimate", basis="the chuck/collet face centre is the frame origin")
    if f["chuck"]:
        cd = f["chuck"][0]
        H.body(r, "chuck", "base", [("cyl", [0, 0, -90], [0, 0, 1], cd / 2 + 5, 90)],
               H.fg_cyl_z(f"{r.name} chuck: {cd:g} mm recommended (published) + 5 mm, 90 long (estimate)", 0, 0, -89, cd / 2 + 4, 88, bore=15),
               status="published", source=url, value=f["chuck"][1])
        r.cite("body.chuck", "estimate", basis="length 90 mm and a 5 mm jaw margin are stand-ins")
    else:
        H.body(r, "chuck", "base", [("cyl", [0, 0, -60], [0, 0, 1], 30, 60)],
               H.fg_cyl_z(f"{r.name} 5C collet nose (unpublished size)", 0, 0, -59, 29, 58, bore=13),
               status="unknown", basis="5C collet nose: r 30 x 60 mm stand-in (5C collets take up to 1-1/16 in. stock)")
    tlo = -TT / 2
    H.body(r, "turret", "X", [("cyl", [cx, 0, tlo], [0, 0, 1], R, TT)],
           H.fg_cyl_z(f"{r.name} turret / tool post (unpublished, stand-in)", cx, 0, tlo + 1, R - 1, TT - 2, bore=15),
           status="unknown", basis=f"r {R:g} x {TT:g} mm, its rim {cx - R:g} mm behind the reference tool point (stand-in)")


def main():
    text = [
        "# Tormach machines: forgecad's records from tormach.com's public spec pages, restated (never copied).",
        "# GENERATED by tools/machines/tormach.py; edit its facts, then re-run. Every value carries a",
        "# [[machine.cite]] (docs/cam-machine.md, Provenance). Bodies are approximations.",
        "",
    ]
    for group, build in ((MILLS, mill), (ROUTERS, router), (LATHES, lathe)):
        for name, f in group.items():
            r = H.Rec(name, {"url": f["url"], "specs": {}})
            build(r, f)
            text.append("# " + "-" * 93)
            text += r.lines
            os.makedirs(os.path.join(PARTS, name), exist_ok=True)
            for (bname, link, prims, ignore, part) in r.bodies:
                out = H.prim_toml(r, bname, link, prims, ignore, part)
                text += [s.replace("parts/haas/", "parts/tormach/") for s in out]
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
