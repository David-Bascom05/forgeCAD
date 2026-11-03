#!/usr/bin/env python3
"""Writes door_asm.fg (the door subassembly) and test.fg (subsystem B's test) from one door block.

Forge subassemblies are rigid in v1 and a parent cannot set a subassembly's params, so test.fg
cannot drive door_asm.fg's hinge and latch: it repeats the door's instances and joints instead.
This script is the one place that block lives. The turning unit U is flattened into both for the
same reason, and because a fit cannot name a part inside a subassembly (the balls seat on the
latch base's detent rims). latch_u.fg remains U's own check. Run: python3 gen_asm.py"""
import os
HERE = os.path.dirname(os.path.abspath(__file__))

DRIVES = """\
# drives: door_f.turn (0 closed .. 105deg open), latch_f.turn (0 open .. 90deg closed), latch_lift
# (U's axial position from detented, positive out of the door: 0 seated, -0.68 riding between
# detents, -0.40 closed on the strike's seat)
param door_f.turn = 0deg
param latch_f.turn = 0deg
shared latch_rest = 8.17        # handle inner face, detented, out from the door's outer face [R4]
shared latch_lift = 0
param latch_f.along = latch_rest + latch_lift
shared hinge_off = -9.92        # bracket top face (Z 29.08) below the door's underside (Z 39.00) [R4]
shared door_open = 105deg
shared latch_closed = 90deg
shared lift_lo = -0.68
shared lift_hi = 0
shared t1 = 1.52
shared t2 = 1.88
shared size_m4 = 4
shared size_m5 = 5
shared size_m6 = 6
shared size_pin = 8
shared min_m4 = 2.77            # SPEC 8 rev B minima: the shortest whole nut per size
shared min_m5 = 2.77
shared min_m6 = 4.78
# poses (SPEC 5.6, 5.7): where the solve starts
shared hx = -277.98
shared hy = 14
shared zb_bot = 29.08
shared zb_top = 876.92
shared dx = -290.98
shared dz = 39
"""

DOOR = """\
door = part("door.fg", at: pose(at: (dx, 0, dz), z: (0, -1, 0), x: (1, 0, 0)))
# bracket first (A = bracket.pin, B = door.hinge): a positive turn opens the door (README)
door_f = cylindrical(brk_b.pin, door.hinge, turn: turn, along: hinge_off, turn_limit: (0deg, door_open))

# pin stacks (5.7): the shoulder screw is fastened to the door, spacer and washer to the bracket
nut_pb = part("../hw/pem_s_m6_0_t1.fg")
nut_pt = part("../hw/pem_s_m6_0_t1.fg")
pin_b = part("../hw/iso7379_8x12_m6.fg")
pin_t = part("../hw/iso7379_8x12_m6.fg")
spc_b = part("../hw/nylon_spacer_8x14x8.fg")
spc_t = part("../hw/nylon_spacer_8x14x8.fg")
wsh_b = part("../hw/iso7089_8.fg")
wsh_t = part("../hw/iso7089_8.fg")
at_nut_pb = fastened(nut_pb.flush, door.pin_bot)
at_nut_pt = fastened(nut_pt.flush, door.pin_top)
at_pin_b = fastened(pin_b.clamp, door.pin_bot)
at_pin_t = fastened(pin_t.clamp, door.pin_top)
at_spc_b = fastened(spc_b.a, brk_b.pin)
at_spc_t = fastened(spc_t.a, brk_t.pin)
at_wsh_b = fastened(wsh_b.a, spc_b.b)
at_wsh_t = fastened(wsh_t.a, spc_t.b)

# latch base (5.8): front face on the field's inner face, t1 behind the studs' outer-face anchors;
# FH-M4-12 studs in the door, ISO 7089-4 washers and ISO 10511 M4 nuts on the base's back
base = part("latch_base.fg")
at_base = slider(base.stud[0], door.lb_stud[0], along: t1)
for i in 0..3 {
  st = part("../hw/pem_fh_m4_12.fg")
  w4 = part("../hw/iso7089_4.fg")
  n4 = part("../hw/iso10511_m4.fg")
  at_st = fastened(st.flush, door.lb_stud[i])
  at_w4 = fastened(w4.a, base.stud_back[i])
  at_n4 = fastened(n4.seat, w4.b)
}
# pivot: ISO 7379 8 x 16 from outside, its shoulder clamping the base's front face against an
# ISO 10511 M6 nyloc on the back face [R12: was an S-M6-0, 7.5 from the detent holes]
nut_lp = part("../hw/iso10511_m6.fg")
piv = part("../hw/iso7379_8x16_m6.fg")
at_nut_lp = fastened(nut_lp.seat, base.pivot_back)
at_piv = fastened(piv.clamp, base.pivot)

# U (latch_u.fg, flattened): the handle on the latch joint, the rest rigid to it
hdl = part("latch_handle.fg")
latch_f = cylindrical(hdl.latch_axis, door.latch, turn: turn, along: along,
                      turn_limit: (0deg, latch_closed), along_limit: (latch_rest + lift_lo, latch_rest + lift_hi))
cam = part("cam.fg")
for i in 0..2 {
  lspc = part("../hw/latch_spacer_6x15.fg")
  lnut = part("../hw/pem_s_m4_0_t2.fg")
  lscr = part("../hw/iso7380_m4x25.fg")
  lball = part("../hw/ball_4.fg")
  at_lspc = fastened(lspc.a, hdl.link_in[i])
  at_lnut = fastened(lnut.flush, cam.link[i])
  at_lscr = fastened(lscr.head, hdl.link[i])
  at_lball = fastened(lball.c, cam.ball_seat[i])
  seat = fit(lball, cam)
  detent = fit(lball, base)
}
at_cam = fastened(cam.link[0], lspc[0].b)

# spring at its working height for the state [R6]: seat a on the handle, b on the door's outer face
if latch_lift == 0 {
  spr = part("../hw/latch_spring_h8_17.fg")
} else {
  if latch_lift == lift_lo {
    spr = part("../hw/latch_spring_h7_49.fg")
  } else {
    spr = part("../hw/latch_spring_h7_77.fg")
  }
}
at_spr = fastened(spr.a, hdl.spring_seat)

m4 = fit_rule(external_thread, internal_thread, size: size_m4, min_engagement: min_m4)
m6 = fit_rule(external_thread, internal_thread, size: size_m6, min_engagement: min_m6)
pin8 = fit_rule(shaft, bore, size: size_pin)
press = fit_rule("../hw/pem_fh_m4_12.fg", "door.fg")
"""

def write(name, text):
    open(os.path.join(HERE, name), "w").write(text)

ASM_HEAD = """\
assembly

# The door subassembly (SPEC 6.1, 5.6-5.8), FRONT door, in rack coordinates: door, two hinge
# brackets (ground: they bolt to the post), two pin stacks, latch base with studs and nuts, pivot,
# spring, and the turning unit U (flattened, see gen_asm.py). GENERATED by gen_asm.py: edit there.
# The rear door is this, turned 180deg about Z through (0, 400).
# door_f: cylindrical about the bottom pin, along fixed at the 9.92 offset [R4]; latch_f:
# cylindrical(hdl.latch_axis, door.latch), both normals out of the door [R9], along =
# 8.17 + lift [R4].

"""

ASM_BRK = """\
brk_b = part("hinge_bracket.fg", ground: true, at: pose(at: (hx, hy, zb_bot)))
brk_t = part("hinge_bracket.fg", ground: true, at: pose(at: (hx, hy, zb_top), z: (0, 0, -1), x: (-1, 0, 0)))
"""

ASM_EXPECTS = """
# checks inside the door subassembly
shared yb_seat = 5.862         # ball centres, rack y (5.8 table, R11): seated, between detents, closed
shared yb_ride = 6.542
shared yb_shut = 6.262
shared yb_tol = 0.005
shared ax = -277.98
shared ay = 14
shared c_pin_bore = 0.09       # 8.0 shoulder in the 8.2 bore: 0.10 radial (mesh chord 0.01)
shared c_wsh_door = 0.31       # running play under the door: 0.32
shared c_spc_slot = 0.18       # link posts 6.0 in the 6.4 arc slots: 0.20 radial
shared c_u_door = 0.5
shared c_hdl_tab = 3
shared c_base_cam = 2
shared c_nut_cam = 0.4         # the nyloc inside the cam's 12.5 bore: 0.47 at the modelled sharp hex (e 11.55), 0.72 at ISO e min 11.05
shared c_nut_ball = 1.3        # nyloc corners to the balls: 1.47
shared co = 0.05
shared lock_tol = 0.3
shared shut = -0.4
shared t30 = 30deg
shared t45 = 45deg
shared t60 = 60deg
shared t50 = 50deg
expect mobility = 0
expect interferences = 0
expect interferences = 0 at latch_f.turn = 90deg, latch_lift = shut
expect interferences = 0 at latch_f.turn = t45, latch_lift = lift_lo
expect interferences = 0 at door_f.turn = door_open
expect y(lball[0]) = yb_seat +- yb_tol
expect y(lball[1]) = yb_ride +- yb_tol at latch_f.turn = t45, latch_lift = lift_lo
expect y(lball[0]) = yb_shut +- yb_tol at latch_f.turn = 90deg, latch_lift = shut
# the top pin lies on the bottom pin's axis (SPEC 8: coaxial within 0.05), closed and open
expect x(door.pin_top) = ax +- co
expect y(door.pin_top) = ay +- co
expect x(door.pin_top) = ax +- co at door_f.turn = door_open
expect y(door.pin_top) = ay +- co at door_f.turn = door_open
# the bearing: pin in the bracket bores, the washer's running play under the door
expect clearance(pin_b, brk_b) >= c_pin_bore
expect clearance(pin_t, brk_t) >= c_pin_bore
expect clearance(wsh_b, door) >= c_wsh_door
expect clearance(wsh_t, door) >= c_wsh_door
# U clears the door except the link posts in the arc slot (SPEC 8)
expect clearance(hdl, door) >= c_u_door
expect clearance(hdl, door) >= c_u_door at latch_f.turn = t30
expect clearance(hdl, door) >= c_u_door at latch_f.turn = t60
expect clearance(hdl, door) >= c_u_door at latch_f.turn = 90deg, latch_lift = shut
expect clearance(cam, door) >= c_u_door
expect clearance(cam, door) >= c_u_door at latch_f.turn = 90deg, latch_lift = shut
expect clearance(lspc[0], door) >= c_spc_slot
expect clearance(lspc[1], door) >= c_spc_slot at latch_f.turn = 90deg, latch_lift = shut
expect clearance(lspc[0], base) >= c_spc_slot
expect clearance(lspc[1], base) >= c_spc_slot at latch_f.turn = 90deg, latch_lift = shut
expect clearance(cam, base) >= c_base_cam
expect clearance(nut_lp, cam) >= c_nut_cam
expect clearance(nut_lp, cam) >= c_nut_cam at latch_f.turn = t45, latch_lift = lift_lo
expect clearance(nut_lp, cam) >= c_nut_cam at latch_f.turn = 90deg, latch_lift = shut
expect clearance(nut_lp, lball[0]) >= c_nut_ball
expect clearance(nut_lp, lball[1]) >= c_nut_ball at latch_f.turn = t45, latch_lift = lift_lo
# handle vs the padlock tab (SPEC 8: >= 3.0 at 0, 45, 90deg)
expect clearance(hdl, base) >= c_hdl_tab
expect clearance(hdl, base) >= c_hdl_tab at latch_f.turn = t45, latch_lift = lift_lo
expect clearance(hdl, base) >= c_hdl_tab at latch_f.turn = 90deg, latch_lift = shut
# padlock: closed, the fin's hole and the tab's hole are coaxial within 0.3 (both normals +Z)
# (an expect's limit is a number, not another measurement: both holes are checked against 5.8's
# rack point (x 265, y -25.65) instead)
shared lock_x = 265
shared lock_y = -25.65
expect x(base.padlock) = lock_x +- lock_tol
expect y(base.padlock) = lock_y +- lock_tol
expect x(hdl.padlock) = lock_x +- lock_tol at latch_f.turn = 90deg, latch_lift = shut
expect y(hdl.padlock) = lock_y +- lock_tol at latch_f.turn = 90deg, latch_lift = shut
"""

write("door_asm.fg", ASM_HEAD + DRIVES + "\n" + ASM_BRK + DOOR + ASM_EXPECTS)

TEST_HEAD = """\
assembly

# Subsystem B's test (SPEC 9B, 8): stand-in front posts (SPEC 5.1 anchors, test_parts/), the hinge
# brackets and strike bolted to them, the four side half-panels with their joint stacks, the front
# door subassembly's instances (repeated from door_asm.fg, which Forge cannot drive as a
# subassembly) and the swing and latch checks of section 8. GENERATED by gen_asm.py: edit there.

"""

TEST_FRAME = """\
# stand-in posts FL and FR (5.1 poses)
shared post_x = 298.48
shared post_y = 35
post_fl = part("test_parts/post_standin.fg", ground: true, at: pose(at: (-post_x, post_y, 0), z: (0, 0, 1), x: (1, 0, 0)))
post_fr = part("test_parts/post_standin.fg", ground: true, at: pose(at: (post_x, post_y, 0), z: (0, 0, 1), x: (0, 1, 0)))

# hinge brackets on post FL's hng_a at s 32.5 [R11]: the bottom bracket's scr[0,1] (i +12) on
# hng_a[0,1]; the turned top bracket's scr[2,3] (i -12) on hng_a[3,2] [R8]; M6 x 12 [R2] from the
# bracket's front face into the post's S-M6-0
brk_b = part("hinge_bracket.fg", at: pose(at: (hx, hy, zb_bot)))
brk_t = part("hinge_bracket.fg", at: pose(at: (hx, hy, zb_top), z: (0, 0, -1), x: (-1, 0, 0)))
at_brk_b = fastened(brk_b.scr[0], post_fl.hng_a[0])
at_brk_t = fastened(brk_t.scr[2], post_fl.hng_a[3])
strike = part("strike.fg", at: pose(at: (273.48, post_y, 453)))
at_strike = fastened(strike.scr[0], post_fr.stk_b[0])
shared n2 = count(2)
for i in 0..4 {
  hnut = part("../hw/pem_s_m6_0_t3.fg")
  at_hnut = fastened(hnut.flush, post_fl.hng_a[i])
}
for i in 0..2 {
  hscr_b = part("../hw/iso7380_m6x12.fg")
  hscr_t = part("../hw/iso7380_m6x12.fg")
  at_hscr_b = slider(hscr_b.head, brk_b.scr[i], along: -t2)
  at_hscr_t = slider(hscr_t.head, brk_t.scr[i + n2], along: -t2)
  snut = part("../hw/pem_s_m6_0_t3.fg")
  sscr = part("../hw/iso7380_m6x12.fg")
  at_snut = fastened(snut.flush, post_fr.stk_b[i])
  at_sscr = slider(sscr.head, strike.scr[i], along: -t2)
}

# side half-panels (5.5 poses), each on its front post by one screw pair [R4: 1.52 offset]; the
# turned (upper) panels meet the post's upper nuts with their scr[3..5]
panel_ll = part("side_panel.fg", at: pose(at: (-300, 0, 0), z: (1, 0, 0), x: (0, 1, 0)))
panel_ul = part("side_panel.fg", at: pose(at: (-300, 800, 906), z: (1, 0, 0), x: (0, -1, 0)))
panel_lr = part("side_panel.fg", at: pose(at: (300, 800, 0), z: (-1, 0, 0), x: (0, -1, 0)))
panel_ur = part("side_panel.fg", at: pose(at: (300, 0, 906), z: (-1, 0, 0), x: (0, 1, 0)))
at_ll = slider(panel_ll.scr[0], post_fl.pnl_b[0], along: -t1)
at_ul = slider(panel_ul.scr[3], post_fl.pnl_b[5], along: -t1)       # R8: UL scr[3,4,5] - FL pnl_b[5,4,3]
at_lr = slider(panel_lr.scr[3], post_fr.pnl_a[0], along: -t1)
at_ur = slider(panel_ur.scr[0], post_fr.pnl_a[5], along: -t1)
# panel screws into the posts' SS-M5-0 (front ends; lower panels scr[0..2], upper scr[3..5])
shared n3 = count(3)
for i in 0..3 {
  pn_l = part("../hw/pem_ss_m5_0_t3.fg")
  pn_ul = part("../hw/pem_ss_m5_0_t3.fg")
  pn_r = part("../hw/pem_ss_m5_0_t3.fg")
  pn_ur = part("../hw/pem_ss_m5_0_t3.fg")
  at_pn_l = fastened(pn_l.flush, post_fl.pnl_b[i])
  at_pn_ul = fastened(pn_ul.flush, post_fl.pnl_b[i + n3])
  at_pn_r = fastened(pn_r.flush, post_fr.pnl_a[i])
  at_pn_ur = fastened(pn_ur.flush, post_fr.pnl_a[i + n3])
  ps_l = part("../hw/iso7380_m5x10.fg")
  ps_ul = part("../hw/iso7380_m5x10.fg")
  ps_r = part("../hw/iso7380_m5x10.fg")
  ps_ur = part("../hw/iso7380_m5x10.fg")
  at_ps_l = fastened(ps_l.head, panel_ll.scr[i])
  at_ps_ul = fastened(ps_ul.head, panel_ul.scr[i + n3])
  at_ps_r = fastened(ps_r.head, panel_lr.scr[i + n3])
  at_ps_ur = fastened(ps_ur.head, panel_ur.scr[i])
}
# panel joints (5.12): M5 x 12 down through the upper return, ISO 7089-5 and ISO 10511 M5 under the
# lower, placed on the lower panel's jnt (normal down, the screw's way); the upper panel is turned,
# so its jnt[i] lies on the lower's jnt[2 - i] (checked below)
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
m5 = fit_rule(external_thread, internal_thread, size: size_m5, min_engagement: min_m5)
"""

TEST_EXPECTS = """
# intended contacts with the strike: its prong tip on the door field (the stop), the cam on its seat
stop = fit(door, strike)
seat_cam = fit(cam, strike)
# ---- section 8 checks (front door) ----
shared c1 = 1
shared c2 = 2
shared c_lu = 0.2
shared c_u_door = 0.5
shared c_hdl_tab = 3
shared co = 0.05
shared z0 = 0
shared lock_tol = 0.3
shared shut = -0.4
shared t05 = 0.5deg      # the cam's tip moves 5 out: into the 5.71 prong (at 1deg it jumps 9.9, past it)
shared t30 = 30deg
shared t45 = 45deg
shared t50 = 50deg
shared t60 = 60deg
shared t80 = 80deg
shared t115 = 115deg
shared t120 = 120deg
shared ax = -277.98
shared ay = 14
expect mobility = 0
expect interferences = 0
expect interferences = 0 at latch_f.turn = 90deg, latch_lift = shut
# interfaces: the second screw of each pair lands on its nut (the first is the joint)
expect distance(brk_b.scr[1], post_fl.hng_a[1]) = z0 +- co
expect distance(brk_t.scr[3], post_fl.hng_a[2]) = z0 +- co
expect distance(strike.scr[1], post_fr.stk_b[1]) = z0 +- co
expect distance(panel_ll.jnt[0], panel_ul.jnt[2]) = z0 +- co
expect distance(panel_lr.jnt[1], panel_ur.jnt[1]) = z0 +- co
expect x(door.pin_top) = ax +- co
expect y(door.pin_top) = ay +- co
expect x(door.pin_top) = ax +- co at door_f.turn = door_open
expect y(door.pin_top) = ay +- co at door_f.turn = door_open
# swing (SPEC 8, 6.1)
expect clearance(door, panel_ll) >= c1 at door_f.turn = t50
expect clearance(door, panel_ll) >= c1 at door_f.turn = door_open
expect clearance(door, brk_t) >= c1 at door_f.turn = t50
expect clearance(door, post_fl) >= c2 at door_f.turn = t50
# at 50deg the hinge return's tip passes closest to the bracket (R11 moved the screw heads 12 off the axis)
expect interferences = 0 at door_f.turn = t50
expect clearance(door, hscr_b[0]) >= c1 at door_f.turn = t50
expect clearance(door, hscr_t[0]) >= c1 at door_f.turn = t50
expect interferences = 0 at door_f.turn = door_open
# past the limit: Forge evaluates an `at` beyond a joint's limit at the limit, silently (README Forge
# gap 11), so the limit is lifted for this line
expect interference(door, panel_ll) > z0 at door_f.turn = t115, door_open = t120
# latch vs strike: open, swinging in, near closed (SPEC 8); closed and latched: no interference
expect clearance(cam, strike) >= c_lu
expect clearance(hdl, strike) >= c_lu
expect clearance(cam, strike) >= c_lu at latch_f.turn = t45, latch_lift = lift_lo
expect clearance(cam, strike) >= c_lu at latch_f.turn = t80, latch_lift = lift_lo
expect clearance(lnut[0], strike) >= c_lu at latch_f.turn = t80, latch_lift = lift_lo
# a closed latch stops the door: the cam runs into the prong as soon as the door opens
expect interference(cam, strike) > z0 at door_f.turn = t05, latch_f.turn = 90deg, latch_lift = shut
# (SPEC 8 asks this at 10deg: there the cam is ~100 in front of the rack, past the prong, and
# nothing overlaps; README SPEC issue 9)
# U clears the door (except the link posts in the slot), the handle clears the padlock tab
expect clearance(hdl, door) >= c_u_door at latch_f.turn = t30
expect clearance(cam, door) >= c_u_door at latch_f.turn = t60
expect clearance(hdl, base) >= c_hdl_tab at latch_f.turn = t45, latch_lift = lift_lo
expect clearance(hdl, base) >= c_hdl_tab at latch_f.turn = 90deg, latch_lift = shut
# padlock holes coaxial within 0.3 when closed
# (an expect's limit is a number, not another measurement: both holes are checked against 5.8's
# rack point (x 265, y -25.65) instead)
shared lock_x = 265
shared lock_y = -25.65
expect x(base.padlock) = lock_x +- lock_tol
expect y(base.padlock) = lock_y +- lock_tol
expect x(hdl.padlock) = lock_x +- lock_tol at latch_f.turn = 90deg, latch_lift = shut
expect y(hdl.padlock) = lock_y +- lock_tol at latch_f.turn = 90deg, latch_lift = shut
# the thin clearances SPEC 5.8 flags: cam tip to the latch-side return 1.96 (closed), latch base
# edge to the strike web 1.6
shared c_tip = 1.9
shared c_web = 1.55
expect clearance(cam, door) >= c_tip at latch_f.turn = 90deg, latch_lift = shut
expect clearance(base, strike) >= c_web
"""

write("test.fg", TEST_HEAD + DRIVES + "\n" + TEST_FRAME + DOOR + TEST_EXPECTS)
print("wrote door_asm.fg, test.fg")
