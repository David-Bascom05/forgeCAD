#!/usr/bin/env python3
"""Fills the GENERATED blocks of rack.fg: the side panels, the two doors and the four casters, and their
section 8 checks. Forge cannot drive a subassembly's joints from its parent, so the door block (taken
verbatim from panels/gen_asm.py's DOOR, the one place it lives) and the caster files are flattened here,
one copy per door and per corner, every id prefixed. Everything outside the blocks is hand-written.
Run: python3 gen_rack.py"""
import ast
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
RACK = os.path.join(HERE, "rack.fg")


def door_block():
    tree = ast.parse(open(os.path.join(ROOT, "panels", "gen_asm.py")).read())
    for node in tree.body:
        if isinstance(node, ast.Assign) and getattr(node.targets[0], "id", "") == "DOOR":
            return ast.literal_eval(node.value)
    raise SystemExit("panels/gen_asm.py has no DOOR")


STRING = re.compile(r'("[^"]*")')


def prefix_ids(text, names, pre, folder):
    """Prefix every id in `names` (not after a '.', not inside a string); point bare part files at
    `folder`."""
    word = re.compile(r"(?<![.\w])(" + "|".join(sorted(names, key=len, reverse=True)) + r")(?!\w)")
    out = []
    for i, piece in enumerate(STRING.split(text)):
        if i % 2:
            path = piece[1:-1]
            if path.endswith(".fg") and "/" not in path:
                piece = '"../%s/%s"' % (folder, path)
            out.append(piece)
        else:
            out.append(word.sub(lambda m: pre + m.group(1), piece))
    return "".join(out)


def assigned(text):
    return set(re.findall(r"^\s*(\w+)\s*=", text, re.M))


# --- doors ---------------------------------------------------------------------------------------

FIT_LINES = re.compile(r"^(m4|m6|pin8|press) = fit_rule\(.*\n", re.M)

DOORS = {
    # pre: (door pose, hinge post, strike post, bracket poses, strike pose, side panel at the hinge)
    "f": dict(name="front", door="pose(at: (-290.98, 0, 39), z: (0, -1, 0), x: (1, 0, 0))",
              hinge="post_fl", strike_post="post_fr",
              brk_b="pose(at: (-277.98, 14, 29.08))",
              brk_t="pose(at: (-277.98, 14, 876.92), z: (0, 0, -1), x: (-1, 0, 0))",
              strike="pose(at: (273.48, 35, 453))", panel="panel_ll", ax=-277.98, ay=14),
    "r": dict(name="rear", door="pose(at: (290.98, 800, 39), z: (0, 1, 0), x: (-1, 0, 0))",
              hinge="post_rr", strike_post="post_rl",
              brk_b="pose(at: (277.98, 786, 29.08), x: (-1, 0, 0))",
              brk_t="pose(at: (277.98, 786, 876.92), z: (0, 0, -1), x: (1, 0, 0))",
              strike="pose(at: (-273.48, 765, 453), x: (-1, 0, 0))", panel="panel_lr", ax=277.98, ay=786),
}


def doors():
    src = FIT_LINES.sub("", door_block())
    names = assigned(src) | {"latch_lift", "brk_b", "brk_t", "strike"}
    out = ["""\
# side half-panels (SPEC 5.5): each lower panel on its front post by scr[0], each upper (turned) one
# by its R8 pair; M5 x 10 into the posts' SS-M5-0 [R4: 1.52 offset]; R8: LL scr[0..2]-FL pnl_b[0..2],
# scr[3..5]-RL pnl_a[0..2]; LR scr[0..2]-RR pnl_b[0..2], scr[3..5]-FR pnl_a[0..2]; UL scr[3,4,5]-FL
# pnl_b[5,4,3], scr[0,1,2]-RL pnl_a[5,4,3]; UR scr[0,1,2]-FR pnl_a[5,4,3], scr[3,4,5]-RR pnl_b[5,4,3]
panel_ll = part("../panels/side_panel.fg", body: "field", at: pose(at: (-300, 0, 0), z: (1, 0, 0), x: (0, 1, 0)))
panel_ul = part("../panels/side_panel.fg", body: "field", at: pose(at: (-300, 800, 906), z: (1, 0, 0), x: (0, -1, 0)))
panel_lr = part("../panels/side_panel.fg", body: "field", at: pose(at: (300, 800, 0), z: (-1, 0, 0), x: (0, -1, 0)))
panel_ur = part("../panels/side_panel.fg", body: "field", at: pose(at: (300, 0, 906), z: (-1, 0, 0), x: (0, 1, 0)))
at_panel_ll = slider(panel_ll.scr[0], post_fl.post.pnl_b[0], along: -t1)
at_panel_ul = slider(panel_ul.scr[3], post_fl.post.pnl_b[5], along: -t1)
at_panel_lr = slider(panel_lr.scr[0], post_rr.post.pnl_b[0], along: -t1)
at_panel_ur = slider(panel_ur.scr[0], post_fr.post.pnl_a[5], along: -t1)
for i in 0..6 {
  pscr_ll = part("../hw/iso7380_m5x10.fg")
  at_pscr_ll = fastened(pscr_ll.head, panel_ll.scr[i])
  pscr_ul = part("../hw/iso7380_m5x10.fg")
  at_pscr_ul = fastened(pscr_ul.head, panel_ul.scr[i])
  pscr_lr = part("../hw/iso7380_m5x10.fg")
  at_pscr_lr = fastened(pscr_lr.head, panel_lr.scr[i])
  pscr_ur = part("../hw/iso7380_m5x10.fg")
  at_pscr_ur = fastened(pscr_ur.head, panel_ur.scr[i])
}
# panel joints (5.12): M5 x 12 down through the upper return, ISO 7089-5 and ISO 10511 M5 under the
# lower, on the lower panel's jnt (normal down, the screw's way)
for i in 0..3 {
  js_l = part("../hw/iso7380_m5x12.fg")
  jw_l = part("../hw/iso7089_5.fg")
  jn_l = part("../hw/iso10511_m5.fg")
  at_js_l = slider(js_l.head, panel_ll.jnt[i], along: -t1)
  at_jw_l = slider(jw_l.a, panel_ll.jnt[i], along: t1)
  at_jn_l = fastened(jn_l.seat, jw_l.b)
  js_r = part("../hw/iso7380_m5x12.fg")
  jw_r = part("../hw/iso7089_5.fg")
  jn_r = part("../hw/iso10511_m5.fg")
  at_js_r = slider(js_r.head, panel_lr.jnt[i], along: -t1)
  at_jw_r = slider(jw_r.a, panel_lr.jnt[i], along: t1)
  at_jn_r = fastened(jn_r.seat, jw_r.b)
}

# doors (SPEC 5.6-5.8, 6.1): drives f_door_f.turn / r_door_f.turn (0 closed .. 105deg open),
# f_latch_f.turn / r_latch_f.turn (0 open .. 90deg closed), f_latch_lift / r_latch_lift (0 seated,
# -0.68 riding between detents, -0.40 closed on the strike's seat). The rear door is the front one
# turned 180deg about Z through (0, 400): hinge on post RR, strike on post RL.
shared latch_rest = 8.17        # handle inner face, detented, out from the door's outer face [R4]
shared hinge_off = -9.92        # bracket top face (Z 29.08) below the door's underside (Z 39.00) [R4]
shared door_open = 105deg
shared latch_closed = 90deg
shared lift_lo = -0.68
shared lift_hi = 0
shared size_pin = 8
"""]
    for pre, d in DOORS.items():
        block = prefix_ids(src, names, pre + "_", "panels")
        block = re.sub(r'(door = part\("\.\./panels/door\.fg"[^\n]*?at: )pose\([^\n]*\)\)$',
                       lambda m: m.group(1) + d["door"] + ")", block, flags=re.M)
        assert d["door"] in block, "door pose line changed in panels/gen_asm.py"
        h, s = d["hinge"], d["strike_post"]
        out.append("""
# ---- {name} door ----
param {p}door_f.turn = 0deg
param {p}latch_f.turn = 0deg
shared {p}latch_lift = 0
param {p}latch_f.along = latch_rest + {p}latch_lift
# hinge brackets on post {H}'s hng_a (R8, R11: bottom scr[0,1]-hng_a[0,1], top scr[2,3]-hng_a[3,2]),
# M6 x 12 [R2] into the post's S-M6-0; the strike on post {S}'s stk_b
{p}brk_b = part("../panels/hinge_bracket.fg", body: "leg", at: {bb})
{p}brk_t = part("../panels/hinge_bracket.fg", body: "leg", at: {bt})
at_{p}brk_b = fastened({p}brk_b.scr[0], {h}.post.hng_a[0])
at_{p}brk_t = fastened({p}brk_t.scr[2], {h}.post.hng_a[3])
{p}strike = part("../panels/strike.fg", body: "base", at: {st})
at_{p}strike = fastened({p}strike.scr[0], {s}.post.stk_b[0])
for i in 0..2 {{
  {p}hscr_b = part("../hw/iso7380_m6x12.fg")
  at_{p}hscr_b = slider({p}hscr_b.head, {p}brk_b.scr[i], along: -t2)
  {p}hscr_t = part("../hw/iso7380_m6x12.fg")
  at_{p}hscr_t = slider({p}hscr_t.head, {p}brk_t.scr[i + 2], along: -t2)
  {p}sscr = part("../hw/iso7380_m6x12.fg")
  at_{p}sscr = slider({p}sscr.head, {p}strike.scr[i], along: -t2)
}}
""".format(name=d["name"], p=pre + "_", H=h[-2:].upper(), S=s[-2:].upper(), h=h, s=s,
           bb=d["brk_b"], bt=d["brk_t"], st=d["strike"]))
        out.append(block)
        out.append("""# intended contacts with the strike: its prong tip on the door field (the stop), the cam on its seat
{p}stop = fit({p}door, {p}strike)
{p}seat_cam = fit({p}cam, {p}strike)
""".format(p=pre + "_"))
    out.append("""
pin8 = fit_rule(shaft, bore, size: size_pin)
press_fh4 = fit_rule("../hw/pem_fh_m4_12.fg", "../panels/door.fg")
""")
    return "".join(out)


def door_checks():
    out = ["""\
# ---- panels and doors (SPEC 8) ----
shared c1 = 1
shared c2 = 2
shared c_lu = 0.2
shared c_u_door = 0.5
shared c_hdl_tab = 3
shared c_door_dummy = 50
shared co = 0.05
shared lock_tol = 0.3
shared shut = -0.4
shared t05 = 0.5deg             # the cam's tip moves 5 out: into the 5.71 prong (panels/gen_asm.py)
shared t120 = 120deg
shared t30 = 30deg
shared t45 = 45deg
shared t50 = 50deg
shared t60 = 60deg
shared t80 = 80deg
shared t115 = 115deg
shared lock_x = 265             # 5.8's padlock point, front door (the rear door's is turned about (0, 400))
shared lock_y = -25.65
shared c_nut_cam = 0.4          # R12 pivot nyloc in the cam's 12.5 bore (0.47 radial at a sharp hex)
shared c_nut_ball = 1.3         # R12: nyloc corners to the balls 1.47 (panels/gen_asm.py)
shared lock_xr = -265
shared lock_yr = 825.65
# the other pair of each panel (1.52 off, R4), bracket and strike lands on its nut (the first is the joint)
expect distance(panel_ll.scr[3], post_rl.post.pnl_a[0]) = t1 +- co
expect distance(panel_ul.scr[0], post_rl.post.pnl_a[5]) = t1 +- co
expect distance(panel_lr.scr[3], post_fr.post.pnl_a[0]) = t1 +- co
expect distance(panel_ur.scr[3], post_rr.post.pnl_b[5]) = t1 +- co
expect distance(panel_ll.jnt[0], panel_ul.jnt[2]) = 0 +- co
expect distance(panel_lr.jnt[1], panel_ur.jnt[1]) = 0 +- co
expect interferences = 0 at f_door_f.turn = t50, r_door_f.turn = t50
expect interferences = 0 at f_door_f.turn = door_open, r_door_f.turn = door_open
expect interferences = 0 at f_latch_f.turn = latch_closed, f_latch_lift = shut, r_latch_f.turn = latch_closed, r_latch_lift = shut
expect interferences = 0 at f_latch_f.turn = t45, f_latch_lift = lift_lo, r_latch_f.turn = t45, r_latch_lift = lift_lo
"""]
    for pre, d in DOORS.items():
        p = pre + "_"
        h, s, pan = d["hinge"], d["strike_post"], d["panel"]
        lx, ly = ("lock_x", "lock_y") if pre == "f" else ("lock_xr", "lock_yr")
        out.append("""
# {name} door
expect distance({p}brk_b.scr[1], {h}.post.hng_a[1]) = 0 +- co
expect distance({p}brk_t.scr[3], {h}.post.hng_a[2]) = 0 +- co
expect distance({p}strike.scr[1], {s}.post.stk_b[1]) = 0 +- co
expect x({p}door.pin_top) = {ax} +- co
expect y({p}door.pin_top) = {ay} +- co
expect x({p}door.pin_top) = {ax} +- co at {p}door_f.turn = door_open
expect y({p}door.pin_top) = {ay} +- co at {p}door_f.turn = door_open
expect clearance({p}door, {pan}) >= c1 at {p}door_f.turn = t50
expect clearance({p}door, {pan}) >= c1 at {p}door_f.turn = door_open
expect clearance({p}door, {p}brk_t) >= c1 at {p}door_f.turn = t50
expect clearance({p}door, {h}) >= c2 at {p}door_f.turn = t50
expect interference({p}door, {pan}) > 0 at {p}door_f.turn = t115, door_open = t120
expect clearance({p}cam, {p}strike) >= c_lu
expect clearance({p}hdl, {p}strike) >= c_lu
expect clearance({p}cam, {p}strike) >= c_lu at {p}latch_f.turn = t45, {p}latch_lift = lift_lo
expect clearance({p}cam, {p}strike) >= c_lu at {p}latch_f.turn = t80, {p}latch_lift = lift_lo
expect interference({p}cam, {p}strike) > 0 at {p}door_f.turn = t05, {p}latch_f.turn = latch_closed, {p}latch_lift = shut
expect clearance({p}hdl, {p}door) >= c_u_door
expect clearance({p}hdl, {p}door) >= c_u_door at {p}latch_f.turn = t30
expect clearance({p}hdl, {p}door) >= c_u_door at {p}latch_f.turn = t60
expect clearance({p}hdl, {p}door) >= c_u_door at {p}latch_f.turn = latch_closed, {p}latch_lift = shut
expect clearance({p}nut_lp, {p}cam) >= c_nut_cam
expect clearance({p}nut_lp, {p}cam) >= c_nut_cam at {p}latch_f.turn = t45, {p}latch_lift = lift_lo
expect clearance({p}nut_lp, {p}cam) >= c_nut_cam at {p}latch_f.turn = latch_closed, {p}latch_lift = shut
expect clearance({p}nut_lp, {p}lball[0]) >= c_nut_ball
expect clearance({p}nut_lp, {p}lball[1]) >= c_nut_ball at {p}latch_f.turn = t45, {p}latch_lift = lift_lo
expect clearance({p}hdl, {p}base) >= c_hdl_tab
expect clearance({p}hdl, {p}base) >= c_hdl_tab at {p}latch_f.turn = t45, {p}latch_lift = lift_lo
expect clearance({p}hdl, {p}base) >= c_hdl_tab at {p}latch_f.turn = latch_closed, {p}latch_lift = shut
expect x({p}base.padlock) = {lx} +- lock_tol
expect y({p}base.padlock) = {ly} +- lock_tol
expect x({p}hdl.padlock) = {lx} +- lock_tol at {p}latch_f.turn = latch_closed, {p}latch_lift = shut
expect y({p}hdl.padlock) = {ly} +- lock_tol at {p}latch_f.turn = latch_closed, {p}latch_lift = shut
""".format(name=d["name"], p=p, h=h, s=s, pan=pan, ax=d["ax"], ay=d["ay"], lx=lx, ly=ly))
    out.append("# room in front of the equipment for handles and cables (SPEC 8: dummies to the doors >= 50)\n")
    for dm in ("d01", "d02", "d04"):
        out.append("expect clearance(%s, f_door) >= c_door_dummy\n" % dm)
    return "".join(out)


# --- casters ---------------------------------------------------------------------------------------

CORNERS = [("fl", "caster_lock.fg"), ("fr", "caster_lock.fg"), ("rl", "caster_plain.fg"), ("rr", "caster_plain.fg")]
DRIVES_C = {"swivel_turn", "roll", "brake"}


def caster_src(name):
    text = open(os.path.join(ROOT, "casters", name)).read()
    head, body = text.split("\nplate = part(", 1)
    body = "plate = part(" + body
    body = body[:body.index("\nm8_rule")] + "\n"
    shared = re.findall(r"^shared (\w+) = (.*)$", head, re.M)
    return shared, body


def casters():
    out = ["""\
# casters (SPEC 5.10, 6.2): casters/caster_lock.fg at the front, caster_plain.fg at the rear, flattened
# (prefix fl_, fr_, rl_, rr_) so each corner's swivel, roll and brake are this file's drives. The
# plate is fastened to the base pan's caster_* anchor [R9]; 4 ISO 4017 M8 x 20 up through plate and
# pan, ISO 7089-8 and ISO 10511 M8 inside the pan.
"""]
    consts = {}
    for _, name in CORNERS:
        shared, _ = caster_src(name)
        for k, v in shared:
            if k not in DRIVES_C and k not in ("m8", "m8_min"):
                consts.setdefault(k, v)
    # the casters' own constants, prefixed cs_ (lift_lo, r_cam .. would meet the doors' names)
    out += ["shared cs_%s = %s\n" % (k, re.sub(r"(?<![.\w])(%s)(?!\w)" % "|".join(consts), r"cs_\1", v))
            for k, v in consts.items()]
    out.append("shared t_plate = 3\nshared t_pan = 3.02\n")
    for c, name in CORNERS:
        shared, body = caster_src(name)
        drives = [k for k, _ in shared if k in DRIVES_C]
        names = assigned(body) | set(drives)
        body = prefix_ids(body, names, c + "_", "casters")
        body = prefix_ids(body, set(consts), "cs_", "casters")
        body = body.replace('%s_plate = part("../casters/plate.fg", at: pose(at: (0, 0, 0)), ground: true)' % c,
                            '%s_plate = part("../casters/plate.fg", at: pose(at: (0, 0, 0)))' % c)
        assert "ground: true" not in body, name
        out.append("\n# ---- caster %s (%s) ----\n" % (c.upper(), name[:-3]))
        out += ["shared %s_%s = %s\n" % (c, k, v) for k, v in shared if k in DRIVES_C]
        out.append(body)
        out.append("""\
at_{c}_caster = fastened({c}_plate.top, base_pan.caster_{c})
{c}_on_pan = fit({c}_plate, base_pan)
for k in 0..4 {{
  {c}_cbolt = part("../hw/iso4017_m8x20.fg")
  at_{c}_cbolt = fastened({c}_cbolt.head, {c}_plate.bolt[k])
  {c}_cwsh = part("../hw/iso7089_8.fg")
  at_{c}_cwsh = slider({c}_cwsh.a, {c}_plate.bolt[k], along: t_plate + t_pan)
  {c}_cnut = part("../hw/iso10511_m8.fg")
  at_{c}_cnut = fastened({c}_cnut.seat, {c}_cwsh.b)
}}
""".format(c=c))
    return "".join(out)


def caster_checks():
    out = ["""\
# ---- casters (SPEC 8): every 45deg of swivel, brake off and on (front) ----
shared c_cast_pan = 0.5
shared c_cast_panel = 5
shared c_cast_cast = 50
shared z_floor = -111
shared tol_floor = 0.05
"""]
    for c, name in CORNERS:
        out.append("expect z(%s_yoke.wheel_bottom) = z_floor +- tol_floor\n" % c)
    pairs = {"fl": ("panel_ll", "rl"), "fr": ("panel_lr", "rr"), "rl": ("panel_ll", None), "rr": ("panel_lr", None)}
    for a in range(0, 360, 45):
        out.append("shared sw%d = %ddeg\n" % (a, a))
    for c, name in CORNERS:
        panel, other = pairs[c]
        brakes = ["", ", %s_brake = cs_brake_hi" % c] if "lock" in name else [""]
        for a in range(0, 360, 45):
            for b in brakes:
                at = " at %s_swivel_turn = sw%d%s" % (c, a, b)
                out.append("expect clearance(%s_yoke, base_pan) >= c_cast_pan%s\n" % (c, at))
                out.append("expect clearance(%s_tread, base_pan) >= c_cast_pan%s\n" % (c, at))
                out.append("expect clearance(%s_tread, %s) >= c_cast_panel%s\n" % (c, panel, at))
                if "lock" in name:
                    out.append("expect clearance(%s_lever, %s) >= c_cast_panel%s\n" % (c, panel, at))
                if other:
                    out.append("expect clearance(%s_tread, %s_tread) >= c_cast_cast%s\n" % (c, other, at))
    out.append("expect interferences = 0 at fl_brake = cs_brake_hi, fr_brake = cs_brake_hi\n")
    # the casters move in the rack: their drives are this file's params (flattened, not a rigid
    # subassembly), so a swivel turns the ball rows at half its angle and a roll turns the 608 cage
    # at 0.6323 of it; all four swivelled and rolled at once, nothing interferes
    out.append("""\
shared roll90 = 90deg
shared orbit45 = 45deg
shared cage_k = 56.907deg       # 0.6323 x 90deg
shared c_ang = 0.01deg
""")
    for c, _ in CORNERS:
        out.append("expect value(%s_wheel.turn) = sw90 +- c_ang at %s_roll = roll90\n" % (c, c))
        out.append("expect value(%s_cage.turn) = cage_k +- c_ang at %s_roll = roll90\n" % (c, c))
        out.append("expect value(%s_swivel.turn) = sw90 +- c_ang at %s_swivel_turn = sw90\n" % (c, c))
        out.append("expect value(%s_up[0].turn) = orbit45 +- c_ang at %s_swivel_turn = sw90\n" % (c, c))
        out.append("expect value(%s_lo[0].turn) = orbit45 +- c_ang at %s_swivel_turn = sw90\n" % (c, c))
        out.append("expect z(%s_yoke.wheel_bottom) = z_floor +- tol_floor at %s_swivel_turn = sw135, %s_roll = roll90\n" % (c, c, c))
    out.append("expect interferences = 0 at %s\n" % ", ".join(
        "%s_swivel_turn = sw135, %s_roll = roll90" % (c, c) for c, _ in CORNERS))
    return "".join(out)


def fill(text, tag, body):
    pat = re.compile(r"(# BEGIN GENERATED %s\n).*?(# END GENERATED %s\n)" % (tag, tag), re.S)
    assert pat.search(text), tag
    return pat.sub(lambda m: m.group(1) + body + m.group(2), text)


def main():
    text = open(RACK).read()
    text = fill(text, "panels", doors())
    text = fill(text, "casters", casters())
    text = fill(text, "checks", door_checks() + caster_checks())
    open(RACK, "w").write(text)


if __name__ == "__main__":
    main()
