#!/usr/bin/env python3
"""Writes the fastener stack test assemblies (SPEC 5.12) and their stand-in plates.

Each stack runs along +z, the direction the screw advances. The plates are plain slabs with the
real hole of the sheet they stand for and two anchors, `a` (the face at z = 0) and `b` (z = t),
both with normal +z, so a stack is a chain of fastened(next.a, prev.b). Nothing here is a model
of a rack part: subsystems A-D own those.
"""
from pathlib import Path

HERE = Path(__file__).resolve().parent
PLATES = HERE / "plates"


def num(v):
    s = f"{v:.6f}".rstrip("0").rstrip(".")
    return s or "0"


def plate(name, title, t, hole, w=30.0):
    kind, size = hole
    rows = ["  outline = rect(center: (0, 0), size: (w, w))"]
    if kind == "none":
        pass
    elif kind == "round":
        rows.append("  bore = circle((0, 0), dh / 2)")
    else:
        rows.append("  bore = rect(center: (0, 0), size: (dh, dh))")
    txt = f"""# stand-in: {title}
# frame: origin at the hole centre on the face z = 0; the slab runs z 0..t
shared t = {num(t)}
{f"shared dh = {num(size)}   # " + ('hole diameter' if kind == 'round' else 'square hole side') + chr(10) if kind != 'none' else ''}shared w = {num(w)}
shared xl = 10

sketch plan(on: xy) {{
{chr(10).join(rows)}
  c0 = pt(0, 0)
  xax = line((0, 0), (xl, 0), construction: true)
}}
slab = extrude(plan, t)
sketch top(on: slab.end, at: plan.c0) {{
  c = pt(0, 0)
}}
a = anchor(plane(at: plan.c0, normal: plan.plane.normal, x: plan.xax.dir))
b = anchor(plane(at: top.c, normal: plan.plane.normal, x: plan.xax.dir))

expect valid
"""
    (PLATES / f"{name}.fg").write_text(txt)


PLATE_DEFS = [
    ("pan_floor_t3", "base pan floor at a carrier screw, 3.02, hole 6.6", 3.02, ("round", 6.6)),
    ("carrier_t2", "carrier leg with an S-M6-0, 1.88, hole 8.75", 1.88, ("round", 8.75)),
    ("rail_web_t2", "rail web, 1.88, hole 6.6", 1.88, ("round", 6.6)),
    ("panel_t1", "side panel at a post screw, 1.52, hole 5.5", 1.52, ("round", 5.5)),
    ("post_m5_t3", "post leg with an SS-M5-0, 3.02, hole 6.35", 3.02, ("round", 6.35)),
    ("return_t1", "side panel joint return, 1.52, hole 5.5", 1.52, ("round", 5.5)),
    ("bracket_leg_t2", "hinge bracket or strike leg, 1.88, hole 6.6", 1.88, ("round", 6.6)),
    ("post_m6_t3", "post leg with an S-M6-0, 3.02, hole 8.75", 3.02, ("round", 8.75)),
    ("bracket_pin_t2", "hinge bracket horizontal leg, 1.88, pin bore 8.2", 1.88, ("round", 8.2)),
    ("door_return_t1", "door return with the pin S-M6-0, 1.52, hole 8.75", 1.52, ("round", 8.75)),
    ("caster_plate", "caster top plate, 3.0, hole 8.5", 3.0, ("round", 8.5)),
    ("pan_caster_t3", "base pan floor at a caster bolt, 3.02, hole 9.0", 3.02, ("round", 9.0)),
    ("top_pan_t3", "top pan with an FH-M8-25, 3.02, hole 8.0", 3.02, ("round", 8.0)),
    ("tray_hanger_t2", "fan tray at a hanger, 1.88, hole 8.6", 1.88, ("round", 8.6)),
    ("post_stud_t3", "post leg with an FH-M6-20, 3.02, hole 6.0", 3.02, ("round", 6.0)),
    ("pan_flange_t3", "pan flange at a post stud, 3.02, hole 6.6", 3.02, ("round", 6.6)),
    ("tray_fan_t2", "fan tray with an S-M4-0, 1.88, hole 5.41", 1.88, ("round", 5.41)),
    ("fan_stack", "guard (5.0) and fan (25.4) clamped under a fan screw, hole 4.5", 30.4, ("round", 4.5)),
    ("rail_sq_t2", "EIA rail mount face, 1.88, square hole 9.5", 1.88, ("square", 9.5)),
    ("door_latch_t1", "door field at the latch pivot, 1.52, clearance 8.6", 1.52, ("round", 8.6)),
    ("base_pivot_t3", "latch base with the pivot S-M6-0, 3.02, hole 8.75", 3.02, ("round", 8.75)),
    ("handle_bore_t2", "latch handle at its bore, 1.88, 8.2", 1.88, ("round", 8.2)),
    ("handle_link_t2", "latch handle at a link hole, 1.88, 4.5", 1.88, ("round", 4.5)),
    ("cam_link_t2", "cam with a link S-M4-0, 1.88, hole 5.41", 1.88, ("round", 5.41)),
    ("door_stud_t1", "door field with an FH-M4-12, 1.52, hole 4.0", 1.52, ("round", 4.0)),
    ("base_stud_t3", "latch base at a door stud, 3.02, hole 4.6", 3.02, ("round", 4.6)),
    ("pan_under_t3", "top pan floor clear of any hole (over a fan screw), 3.02", 3.02, ("none", 0)),
    ("ear_3", "equipment ear, 3.0, hole 7.0", 3.0, ("round", 7.0)),
]

# Each test: the instances in stack order, how each is placed, the fit rules, and expects.
# place: ("ground", z) | ("on", other, other_anchor, own_anchor) | ("off", other, other_anchor,
# own_anchor, along) -- off is a slider with a fixed value: fastened has no offset.
TESTS = {
    "carrier_pan": dict(
        doc="carrier <-> pan: ISO 7380 M6x12 from under the pan floor into the carrier's S-M6-0 (t2)",
        inst=[("pan", "plates/pan_floor_t3.fg", ("ground", 0)),
              ("car", "plates/carrier_t2.fg", ("on", "pan", "b", "a")),
              ("nut", "../pem_s_m6_0_t2.fg", ("on", "car", "a", "flush")),
              ("scr", "../iso7380_m6x12.fg", ("on", "pan", "a", "head"))],
        rules=[("m6", "external_thread", "internal_thread", 6, 5.23)],
        z={"nut": 3.02, "scr": 0}),
    "rail_carrier": dict(
        doc="rail <-> carrier: ISO 7380 M6x10 through the rail web into the carrier's S-M6-0 (t2)",
        inst=[("web", "plates/rail_web_t2.fg", ("ground", 0)),
              ("car", "plates/carrier_t2.fg", ("on", "web", "b", "a")),
              ("nut", "../pem_s_m6_0_t2.fg", ("on", "car", "a", "flush")),
              ("scr", "../iso7380_m6x10.fg", ("on", "web", "a", "head"))],
        rules=[("m6", "external_thread", "internal_thread", 6, 5.23)],
        z={"nut": 1.88, "scr": 0}),
    "panel_post": dict(
        doc="side panel <-> post: ISO 7380 M5x10 through the panel into the post's SS-M5-0 (t3)",
        inst=[("pnl", "plates/panel_t1.fg", ("ground", 0)),
              ("post", "plates/post_m5_t3.fg", ("on", "pnl", "b", "a")),
              ("nut", "../pem_ss_m5_0_t3.fg", ("on", "post", "a", "flush")),
              ("scr", "../iso7380_m5x10.fg", ("on", "pnl", "a", "head"))],
        rules=[("m5", "external_thread", "internal_thread", 5, 2.77)],
        z={"nut": 1.52, "scr": 0}),
    "panel_joint": dict(
        doc="panel joint: ISO 7380 M5x12 through both returns, ISO 7089-5, ISO 10511-M5",
        inst=[("lower", "plates/return_t1.fg", ("ground", 0)),
              ("upper", "plates/return_t1.fg", ("on", "lower", "b", "a")),
              ("wsh", "../iso7089_5.fg", ("on", "upper", "b", "a")),
              ("nut", "../iso10511_m5.fg", ("on", "wsh", "b", "seat")),
              ("scr", "../iso7380_m5x12.fg", ("on", "lower", "a", "head"))],
        rules=[("m5", "external_thread", "internal_thread", 5, 5.0)],
        z={"nut": 4.04, "scr": 0}),
    "bracket_post": dict(
        doc="hinge bracket (or strike) <-> post: ISO 7380 M6x12 through the 1.88 leg into the post's S-M6-0 (t3)",
        inst=[("brk", "plates/bracket_leg_t2.fg", ("ground", 0)),
              ("post", "plates/post_m6_t3.fg", ("on", "brk", "b", "a")),
              ("nut", "../pem_s_m6_0_t3.fg", ("on", "post", "a", "flush")),
              ("scr", "../iso7380_m6x12.fg", ("on", "brk", "a", "head"))],
        rules=[("m6", "external_thread", "internal_thread", 6, 5.23)],
        z={"nut": 1.88, "scr": 0}),
    "hinge_pin": dict(
        doc="hinge pivot stack (SPEC 5.7) at rack Z: ISO 7379 8x12 M6 up through bracket, spacer and washer "
            "into the door return's S-M6-0 (t1)",
        inst=[("door", "plates/door_return_t1.fg", ("ground", 39.0)),
              ("nut", "../pem_s_m6_0_t1.fg", ("on", "door", "a", "flush")),
              ("pin", "../iso7379_8x12_m6.fg", ("on", "door", "a", "clamp")),
              ("brk", "plates/bracket_pin_t2.fg", ("off", "door", "a", "b", -9.92)),
              ("spc", "../nylon_spacer_8x14x8.fg", ("on", "brk", "b", "a")),
              ("wsh", "../iso7089_8.fg", ("on", "spc", "b", "a"))],
        rules=[("m6", "external_thread", "internal_thread", 6, 5.23),
               ("pin8", "shaft", "bore", 8, None)],
        z={"pin": 27.0, "brk": 27.2, "spc": 29.08, "wsh": 37.08, "nut": 39.0},
        clear=[("pin", "brk", ">=", 0.099), ("wsh", "door", ">=", 0.319), ("pin", "spc", ">=", 0.099)]),
    "caster": dict(
        doc="caster <-> base pan at rack Z: ISO 4017 M8x20 up through plate and floor, ISO 7089-8, ISO 10511-M8",
        inst=[("plate", "plates/caster_plate.fg", ("ground", -3.0)),
              ("pan", "plates/pan_caster_t3.fg", ("on", "plate", "b", "a")),
              ("wsh", "../iso7089_8.fg", ("on", "pan", "b", "a")),
              ("nut", "../iso10511_m8.fg", ("on", "wsh", "b", "seat")),
              ("bolt", "../iso4017_m8x20.fg", ("on", "plate", "a", "head"))],
        rules=[("m8", "external_thread", "internal_thread", 8, 8.0)],
        z={"pan": 0, "wsh": 3.02, "nut": 4.62, "bolt": -3.0}),
    "hanger": dict(
        doc="fan tray hanger (SPEC 5.9 rev B): FH-M8-25 in the top pan, spacer, ISO 7089-8, tray, ISO 10511-M8 "
            "on the tray's underside (z measured down from the top pan's outer face, Z = 906 - z)",
        inst=[("pan", "plates/top_pan_t3.fg", ("ground", 0)),
              ("stud", "../pem_fh_m8_25.fg", ("on", "pan", "a", "flush")),
              ("spc", "../nylon_spacer_8x14x8.fg", ("on", "pan", "b", "a")),
              ("wsh", "../iso7089_8.fg", ("on", "spc", "b", "a")),
              ("tray", "plates/tray_hanger_t2.fg", ("on", "wsh", "b", "a")),
              ("nut", "../iso10511_m8.fg", ("on", "tray", "b", "seat"))],
        rules=[("m8", "external_thread", "internal_thread", 8, 8.0),
               ("press", '"../pem_fh_m8_25.fg"', '"plates/top_pan_t3.fg"', None, None)],
        z={"wsh": 11.02, "tray": 12.62, "nut": 14.5}),
    "post_pan": dict(
        doc="post <-> pan: FH-M6-20 in the post, pan flange, ISO 7089-6, ISO 10511-M6",
        inst=[("post", "plates/post_stud_t3.fg", ("ground", 0)),
              ("stud", "../pem_fh_m6_20.fg", ("on", "post", "a", "flush")),
              ("pan", "plates/pan_flange_t3.fg", ("on", "post", "b", "a")),
              ("wsh", "../iso7089_6.fg", ("on", "pan", "b", "a")),
              ("nut", "../iso10511_m6.fg", ("on", "wsh", "b", "seat"))],
        rules=[("m6", "external_thread", "internal_thread", 6, 6.0),
               ("press", '"../pem_fh_m6_20.fg"', '"plates/post_stud_t3.fg"', None, None)],
        z={"nut": 7.64}),
    "fan_screw": dict(
        doc="fan + guard (SPEC 5.9 rev B): ISO 7045 M4x40 up through guard and fan (stand-in block) into the "
            "tray's S-M4-0 (t2); the top pan's floor 9.6 above the tray (spacer 8.0 + washer 1.6). z = Z - 861.10",
        inst=[("fan", "plates/fan_stack.fg", ("ground", 0)),
              ("tray", "plates/tray_fan_t2.fg", ("on", "fan", "b", "a")),
              ("nut", "../pem_s_m4_0_t2.fg", ("on", "tray", "a", "flush")),
              ("pan", "plates/pan_under_t3.fg", ("off", "tray", "b", "a", 9.6)),
              ("scr", "../iso7045_m4x40.fg", ("on", "fan", "a", "head"))],
        rules=[("m4", "external_thread", "internal_thread", 4, 2.77)],
        z={"nut": 30.4, "scr": 0, "pan": 41.88},
        clear=[("scr", "pan", ">=", 1.879)]),
    "rack_screw": dict(
        doc="equipment: M6x16 rack screw, plastic washer, 3.0 ear, rail (square hole), cage nut",
        inst=[("rail", "plates/rail_sq_t2.fg", ("ground", 4.0)),
              ("ear", "plates/ear_3.fg", ("off", "rail", "a", "a", -3.0)),
              ("pw", "../plastic_washer_m6.fg", ("off", "ear", "a", "a", -1.0)),
              ("cage", "../cage_nut_m6.fg", ("on", "rail", "b", "seat")),
              ("scr", "../rack_screw_m6x16.fg", ("on", "pw", "a", "head"))],
        rules=[("m6", "external_thread", "internal_thread", 6, 4.78)],
        z={"cage": 5.88, "scr": 0, "ear": 1.0}),
    "latch_pivot": dict(
        doc="latch pivot (SPEC 5.8), z = rack y on the front door, detented: ISO 7379 8x16 through the handle, "
            "spring and door into the latch base's S-M6-0 (t3)",
        inst=[("base", "plates/base_pivot_t3.fg", ("ground", 1.52)),
              ("nut", "../pem_s_m6_0_t3.fg", ("on", "base", "a", "flush")),
              ("door", "plates/door_latch_t1.fg", ("off", "base", "a", "a", -1.52)),
              ("pin", "../iso7379_8x16_m6.fg", ("on", "base", "a", "clamp")),
              ("hdl", "plates/handle_bore_t2.fg", ("off", "door", "a", "b", -8.17)),
              ("spr", "../latch_spring_h8_17.fg", ("on", "hdl", "b", "a"))],
        rules=[("m6", "external_thread", "internal_thread", 6, 5.23)],
        z={"pin": -14.48, "hdl": -10.05, "spr": -8.17, "door": 0},
        clear=[("pin", "hdl", ">=", 0.099), ("pin", "spr", ">=", 0.79), ("pin", "door", ">=", 0.29)]),
    "latch_link": dict(
        doc="latch link (SPEC 5.8), z = rack y, detented: ISO 7380 M4x25 through the handle and a 6x15 spacer "
            "into the cam's S-M4-0 (t2)",
        inst=[("hdl", "plates/handle_link_t2.fg", ("ground", -10.05)),
              ("spc", "../latch_spacer_6x15.fg", ("on", "hdl", "b", "a")),
              ("cam", "plates/cam_link_t2.fg", ("on", "spc", "b", "a")),
              ("nut", "../pem_s_m4_0_t2.fg", ("on", "cam", "a", "flush")),
              ("scr", "../iso7380_m4x25.fg", ("on", "hdl", "a", "head"))],
        rules=[("m4", "external_thread", "internal_thread", 4, 2.77)],
        z={"cam": 6.83, "scr": -10.05}),
    "latch_base_stud": dict(
        doc="latch base <-> door (SPEC 5.8), z = rack y: FH-M4-12 in the door, base, ISO 7089-4, ISO 10511-M4",
        inst=[("door", "plates/door_stud_t1.fg", ("ground", 0)),
              ("stud", "../pem_fh_m4_12.fg", ("on", "door", "a", "flush")),
              ("base", "plates/base_stud_t3.fg", ("on", "door", "b", "a")),
              ("wsh", "../iso7089_4.fg", ("on", "base", "b", "a")),
              ("nut", "../iso10511_m4.fg", ("on", "wsh", "b", "seat"))],
        rules=[("m4", "external_thread", "internal_thread", 4, 5.0),
               ("press", '"../pem_fh_m4_12.fg"', '"plates/door_stud_t1.fg"', None, None)],
        z={"nut": 5.34}),
}


def assembly(name, t):
    out = [f"assembly", "", f"# {t['doc']}", ""]
    shared, body = [], []
    for inst, file, place in t["inst"]:
        if place[0] == "ground":
            shared.append(f"shared z_{inst} = {num(place[1])}")
            body.append(f'{inst} = part("{file}", ground: true, at: pose(at: (0, 0, z_{inst})))')
        else:
            body.append(f'{inst} = part("{file}")')
    for inst, file, place in t["inst"]:
        if place[0] == "on":
            _, other, oa, own = place
            body.append(f"at_{inst} = fastened({inst}.{own}, {other}.{oa})")
        elif place[0] == "off":
            _, other, oa, own, along = place
            shared.append(f"shared off_{inst} = {num(along)}")
            body.append(f"at_{inst} = slider({inst}.{own}, {other}.{oa}, along: off_{inst})")
    for rname, a, b, size, mine in t["rules"]:
        args = [a, b]
        if size is not None:
            shared.append(f"shared size_{rname} = {num(size)}")
            args.append(f"size: size_{rname}")
        if mine is not None:
            shared.append(f"shared min_{rname} = {num(mine)}   # the nut's whole thread")
            args.append(f"min_engagement: min_{rname}")
        body.append(f"{rname} = fit_rule({', '.join(args)})")
    exp = ["expect interferences = 0", "expect mobility = 0"]
    for inst, z in t.get("z", {}).items():
        shared.append(f"shared zx_{inst} = {num(z)}")
        exp.append(f"expect z({inst}) = zx_{inst} +- 0.001")
    for a, b, cmp, v in t.get("clear", []):
        shared.append(f"shared c_{a}_{b} = {num(v)}")
        exp.append(f"expect clearance({a}, {b}) {cmp} c_{a}_{b}")
    return "\n".join(out + shared + [""] + body + [""] + exp) + "\n"


if __name__ == "__main__":
    PLATES.mkdir(exist_ok=True)
    for p in PLATE_DEFS:
        plate(*p)
    for name, t in TESTS.items():
        (HERE / f"{name}.fg").write_text(assembly(name, t))
    print(f"{len(PLATE_DEFS)} plates, {len(TESTS)} assemblies")
