# RACK18U: an 18U enclosed 19" rack in bolted sheet metal

Status: design spec, rev A (2026-11-03). Nothing in this file has been modelled yet. Subsystem
agents build the parts from it; where this file and a model disagree, the model is wrong until
this file is changed.

All dimensions are in mm unless marked. "t3", "t2", "t1" are the three stock thicknesses (section 3).

## Revisions

### Rev B (2026-11-03)

These corrections come from subsystem E's stack tests (`hw/README.md`, "Findings for the design lead"). Each
change is tagged [R1] to [R6] where it lands in the text. Subsystems already building:
- check R1 and R4, which change anchor kinds and how pairs are joined;
- check R2, which moves the fan tray, fans and guards down 1.60.

- **R1. PEM nut thread depth.**
  - A clinch nut is threaded over its shank A plus its head T, not the sheet thickness plus T: S-M6-0
    5.23, S-M4-0 2.77, SS-M5-0 2.77, in every sheet.
  - Sheet parts' PEM anchors are now PLACEMENT anchors (no `kind`, no `depth`), on the nut's flush face.
    The thread anchor belongs to the PEM part in `hw/`, placed with `fastened(nut.flush, sheet.<anchor>)`.
    The same holds for FH studs: the sheet anchor is placement only.
  - The old depths (7.10, 5.96, 5.60, 5.02, 3.88) are gone from 5.1, 5.3, 5.6, 5.8 and 5.9.
  - The section 8 fit-rule minima are now the shortest whole nut per size: M4 2.77, M5 2.77, M6 4.78
    (the cage nut), M8 8.0. The hw stack tests prove each joint fills its own nut.
- **R2. Screw lengths.**
  - M6 x 12, not M6 x 10, at hinge bracket to post, strike to post, and carrier to pan: 3.02 past the nut.
    Rail to carrier stays M6 x 10 (2.16 past).
  - Fan screws: ISO 7045 M4 x 40, not M4 x 35, which went only 0.72 past. To make room, the fan tray
    hangs 1.60 lower: an ISO 7089-8 washer now sits between the 8.0 spacer and the tray (above it), and
    the M8 nyloc bears on the tray's underside without a washer. The hanger is still 2.50 past its nut.
  - New Z values:
    - tray top 893.38 (was 894.98), tray bottom 891.50;
    - fans 891.50 .. 866.10;
    - guards to 861.10, 8.05 above U18's top;
    - the fan screw tip is 7.72 above the tray and 1.88 under the top pan.
  - Fan tray pose: origin (0, 400, 893.38).
- **R3. Arithmetic.** The fan screw was 2.72 (now 7.72 with M4 x 40) above the tray, not 5.72. The
  latch link screw is 4.24 past its nut, not 2.24. The hinge pin engages 5.23, not 5.60. The cage nut
  screw engages 4.78 and runs 2.09 past.
- **R4. Anchor pairs that meet only with an offset.**
  - `fastened` has no offset: join each pair below with `slider(a, b, along: d)` at the fixed d (mobility
    0), or with `cylindrical(.., along: d)` where the pair is also a revolute.
    ```
    panel.scr[*]   -> post.pnl_*          d = 1.52   (panel outer face to post outer face)
    cage.seat      -> rail.eia[k]         d = 1.88   (rail rear face to front face)
    post.stud_*    -> pan.stud_*          d = 3.02   (post outer face to pan flange outer face)
    carrier.pan[*] -> pan.car_*           d = 3.02   (carrier underside on the floor's top; pan anchor on its underside)
    tray.hang[*]   -> top pan hang[*]     d = 12.62  (tray top face to the pan's outer face, along the stud)
    door.pin_bot   -> bracket.pin         d = 9.92   (bottom pivot: door underside Z 39.00, bracket top 29.08;
                                                      the door joint is cylindrical(door.hinge, bracket.pin,
                                                      turn: door_f_turn, along: 9.92); top pin likewise)
    latch_u.axis   -> door.latch          d = 8.17 + lift   (handle inner face to door outer face; 6.1's along)
    ```
  - Pairs not listed meet face to face: carrier rail nuts and rail web, bracket and strike on the post,
    caster on the pan, fans on the tray, the panel joint returns. [R11] The latch base is NOT face to face
    with the door's anchors: `slider(base.stud[k], door.lb_stud[k], along: 1.52)` (lb_stud is on the
    door's outer face, where the FH head sits).
- **R5. Forge syntax.**
  - Section 3's `stock()` limits are `per_t(ratio(0), plus: L)` (a plain number is WrongType), as in
    `hw/shop.fg`.
  - Fit-rule `size:` is a number (`size: 6`), not "M6".
  - `hw/shop.fg` still needs the FH-M4-12 hardware row that section 3 now has (added with the latch).
- **R6. Hardware choices, accepted.**
  - One nylon spacer, 8.2 ID x 14 OD x 8.0, for both the hinges and the fan tray hangers (8.2 clears the
    M8 stud).
  - The latch spring is representative until a maker's row is picked: 1.2 wire, 9.6 ID x 12.0 OD, solid
    4.8, about 8 N/mm, inside 5.8's limits.
  - Fan screw ISO 7045.
  - Rack screw modelled as ISO 1207 (cheese head, the Rittal DK 7094.110 type).
- **R7. Rail carrier clears the pans' inside bends** (found by subsystem A, approved by the coordinator).
  At y = 42 / 758 the carrier ends sat 0.96 from the pan flange's inner face (41.04), but the pan floor is
  only flat from 41.04 + 1.60 (inside radius) = 42.64, so each end corner cut ~0.12 into the bend.
  RACK-111 is now 714 long, spanning y 43 .. 757 (0.36 clear of the bend). Carrier-local positions change,
  rack positions do not: rail nuts at a = 19.5 + 25k (rack y = 62.5 + 25k, unchanged), pan nuts at
  a = 57, 357, 657 (rack y = 100 / 400 / 700, unchanged). Pan, rail and post anchors are unchanged.
  Section 5.3's carrier length and local offsets are superseded by this entry.

- **R8. Index pairing on turned parts** (found by subsystem D; checked for every turned part by script,
  `pairs.py` in the design lead's scratch).
  - Where a part is turned 180deg, its anchor k does not meet the partner's anchor k. The pairs that
    meet, at the default rails (y_f 100, y_r 725):
    ```
    fan tray  -> top pan    tray.hang[k] meets pan.hang[k xor 1] (0-1, 1-0, 2-3, 3-2); grille k sits over fan k xor 1
    carrier   -> pans       BL.pan[i]-base car_l[i]; BR.pan[i]-base car_r[2-i]; TR.pan[i]-top car_l[i]; TL.pan[i]-top car_r[2-i]
    rail      -> carriers   web_bot/top[i] on BL/TR: rail[k_front + i] (front rails), rail[k_rear - i] (rear rails);
                            on BR/TL: rail[27 - that]. Default: LF bot[0,1]-BL[2,3], top[0,1]-TL[25,24];
                            RF top[0,1]-BR[25,24], bot[0,1]-TR[2,3]; LR top[0,1]-BL[26,25], bot[0,1]-TL[1,2];
                            RR bot[0,1]-BR[1,2], top[0,1]-TR[26,25]
    posts     -> top pan    the top pan's stud names are in pan-frame (base) terms, so at the top they swap
                            left/right: post FL's stud_a/b[1] meet top stud_fr_f / stud_fr_s; FR's meet
                            stud_fl_s / stud_fl_f; RL's meet stud_rr_s / stud_rr_r; RR's meet stud_rl_r / stud_rl_s.
                            At the base: FL a[0]-fl_f, b[0]-fl_s; FR a[0]-fr_s, b[0]-fr_f; RL a[0]-rl_s, b[0]-rl_r;
                            RR a[0]-rr_r, b[0]-rr_s
    side panel-> posts      LL scr[0..2]-FL pnl_b[0..2], scr[3..5]-RL pnl_a[0..2];
                            LR scr[0..2]-RR pnl_b[0..2], scr[3..5]-FR pnl_a[0..2];
                            UL scr[3,4,5]-FL pnl_b[5,4,3], scr[0,1,2]-RL pnl_a[5,4,3];
                            UR scr[0,1,2]-FR pnl_a[5,4,3], scr[3,4,5]-RR pnl_b[5,4,3]
    hinge brk -> posts      [R11] FB scr[0,1]-FL hng_a[0,1]; FT scr[2,3]-FL hng_a[3,2]; RB scr[0,1]-RR hng_a[0,1];
                            RT scr[2,3]-RR hng_a[3,2]  (scr[0,1] at i = +12, scr[2,3] at i = -12)
    doors, strikes, casters  no index change (door pin_bot/pin_top meet the bottom/top bracket's pin;
                            strike scr[k] meets stk_b[k]; caster_xx meets the caster at xx)
    ```
  - 5.9 and 5.12 are corrected to match.
- **R9. Anchor x axes.**
  - Forge's `fastened`, `slider`, `revolute` and `cylindrical` align the anchors' x axes as well as their
    origins and normals. Every anchor on a joined pair therefore has a fixed x.
  - **The rule** (subsystem A's, adopted for all): x lies in the joint face and points toward the part's
    middle along its length, or toward the flange tip.
  - **Normals:** both anchors of a pair have the same normal (the screw's or the joint axis's direction),
    so no joint uses `flip`.
  - **Values per anchor** (part frame), with the turn each joint needs. A turn is the fixed angle from
    B's x to A's x about the normal; give it as a drive, as in
    `cylindrical(A, B, turn: 180deg, along: d)`.
    ```
    post (frame, A)    toward w = 453: stud_a/b[0], pnl_*[0..2], hng_*[0..1], stk_* x = +w;
                       stud_a/b[1], pnl_*[3..5], hng_*[2..3] x = -w
    pans               stud_* x = +z (floor to flange tip), base and top; car_l[*] x = -x, car_r[*] x = +x;
                       caster_* normal +z, x = +x; hang[*] (top) x = +x
    carrier            pan[i] x = +b; rail[k] x = +c
    rail               web_bot[*] x = +r; web_top[*] x = -r; eia[k] x = +r
    side panel         scr[*] x = +f; jnt[*] x = +e
    hinge bracket      scr[*] x = +zb (toward the leg's tip); pin x = +j (toward the vertical leg)
    door               pin_bot, pin_top x = -n (toward the returns' tips); latch x = -k; lb_stud[*] x = +k
    strike             scr[*] x = +z
    latch base         stud[*] x = rack +Z on the front instance (its "up"); pivot x likewise
    latch handle, cam  axis: normal +w (out of the door, as door.latch), x = +u; turn = the latch's turn
    fan tray           hang[*] x = -x; fan[*] x = +x
    caster             top: normal +z, x = +x
    ```
  - **Resulting turns and offsets** (along = (A's origin - B's origin) . B's normal):
    ```
    A -> B                              turn    along
    post stud[0] -> base pan stud        0      -3.02
    post stud[1] -> top pan stud         0      -3.02
    carrier pan[i] -> pan car_*          0      +3.02
    rail web_* -> carrier rail[k]        0       0
    lower panel scr -> post pnl          0      -1.52
    upper panel scr -> post pnl          0      -1.52   (pnl_*[3..5] x = -w: the upper panel is the lower turned about X)
    bottom bracket scr -> post hng       0       0
    top bracket scr -> post hng          0       0      (hng_*[2..3] x = -w)
    strike scr -> post stk               0       0
    bracket pin -> door pin_bot          0      -9.92   [R11] bracket first: cylindrical(bracket.pin, door.hinge,
                                                         turn: door turn, along: -9.92); +turn opens the door
    latch base stud -> door lb_stud      0      +1.52   [R11] (base front face to the door's outer face)
    handle latch_axis -> door latch     turn   8.17 + lift, lift 0 / -0.68 / -0.40 [R11]
    tray hang -> top pan hang            0      +12.62
    caster top -> base pan caster_*      0       0
    dummy ear -> rail eia (via cage)     0 on LF/RR, 180 on RF/LR (rail eia x = +r runs down on the
                                         flipped rails), along per 5.11 and R4
    ```
  - Subsystem A built the post with pnl_*[3..5] and hng_*[2..3] at x = -w (its "toward the middle" rule,
    as for stud[1]), so every turn above is 0 except the dummy ears on the flipped rails.
  - The base pan's `caster_*` anchors now have normal +z (into the pan, the same as the caster's `top`); they
    were -z. Section 5.2's text says so, marked [R9], and `frame/base_pan.fg` is built that way.
- **R10. Fan screw depends on the bought guard; lowest point** (subsystem D).
  - Rev B's M4 x 40 assumes guard loops 5.0 below the fan face. If the bought guard's loops lie flat on
    the fan, M4 x 40 would end 1.12 into the top pan floor; use M4 x 35. Check the guard and the fan's
    bore (<= 114) before ordering (section 7).
  - The lowest point under the roof is the fan screw heads at Z 858.00, 4.95 above U18's top, not the
    guard. Section 8 now checks the screws against the 4U dummy as well.

- **R11. Panels and doors fixes** (found by subsystem B, `panels/README.md` "SPEC issues"; the
  coordinator decided them for David).
  1. **Bracket screw heads were in the door's sweep.** The hinge return's tip ran 1.2 into the M6 heads at
     about 50deg. Fix:
     - the post's hinge nuts `hng_*[0..3]` move to s = 32.5 (was 20.5), same w (50, 66, 840, 856), so the
       front bracket screws sit at rack x -265.98;
     - the hinge bracket gets 4 holes, at i = +12 and i = -12 each at zb 20.92 and 36.92. Bottom brackets
       use scr[0,1] (i +12), top brackets scr[2,3] (i -12), so one bracket serves both (R8 updated);
     - the heads are 12 off the pin axis; the sweep passes y 29.82 only within 6.3 of it.
  2. **Hinge joint written bracket first:** `cylindrical(bracket.pin, door.hinge, turn: t, along: -9.92)`,
     limit (0, 105deg). Positive t opens the door. The R9 table and 6.1 are fixed to match.
  3. **Closed lift is -0.40.** With along = 8.17 + lift (the handle's inner face out from the door's
     outer face), lift is 0 seated, -0.68 between detents and -0.40 closed; along_limit is (7.49, 8.17).
     6.1 and section 8 are fixed to match.
  4. **Ball centres** are rack y 5.86 (seated), 6.54 (between) and 6.26 (closed) (5.8 table).
  5. **Latch base studs** move to rack angle 140, 182.5 and 225deg (r 28; was 60, 150, 225), in the door
     and in RACK-322. The 60deg stud ran 0.75 into the padlock tab slot. The base's 60deg hole was 0.83
     from the tab's bend (shop 5.84); the 140deg hole is now 7.08 from it. The 182.5deg hole is 1.73
     from the base's left edge (shop 0.91), and all three clear the arc slot, the cam's sweep and the
     vents.
  6. **The "closed latch stops the door" check** runs at door 0.5deg, not 10deg. At 10deg the cam is
     already about 100 out, past the 5.71 prong.
  7. **Anchor names:** `latch_axis` (handle, cam) and `ball_seat[0..1]` (cam). `axis` and `ball` are
     Forge built-ins (ShadowsBuiltin).
  8. **Latch base to door** is `slider(base.stud[k], door.lb_stud[k], along: 1.52)`, not face to face
     (R4 fixed).
  9. **Shop violations from SPEC numbers.** The latch link radius goes from 15.0 to 14.8, everywhere
     the link touches:
     - handle link holes at (u, v) = (-10.465, +-10.465): 4.18 from the fin's bend tangent line (shop 4.15;
       was 4.04);
     - cam S-M4-0 at the same points: 7.1 from the hub edge R 21.9 (PEM 6.9; was exactly 6.9);
     - the door's and the base's arc slot: radius 14.8, the same width 6.4 and the same 45 to 225deg. The
       slot's inner edge (11.6) keeps 1.1 to the base's detent holes (r 9 + 1.5);
     - spacers, link screws and the R8 pairing are unchanged.
  Also accepted from B's list, with no change to the design:
  - the handle fin is 24.0 beyond the handle's OUTER face (flat ~47 x 66.5);
  - the spring is fastened to the handle's `spring_seat`;
  - the strike's `seat` and `stop` have x = +z;
  - `bracket.pin` is kind bore, size 8, depth 1.88;
  - U-to-door clearance is checked part by part (link posts 0.18 radial in the slots);
  - the side panel return's real bend length is 620 (order note); Forge no longer reports its 2 cut-back
    HoleNearBend.

  **Part files to change for R11:**
  ```
  frame/post.fg           hng_a/hng_b[0..3] at s 32.5 (was 20.5)                                       (subsystem A)
  panels/hinge_bracket    4 holes at i +-12, anchors scr[0..3]                                          (B)
  panels/latch_base       stud holes at 140 / 182.5 / 225deg; arc slot r 14.8                           (B)
  panels/latch_handle     link holes r 14.8 at (-10.465, +-10.465)                                      (B)
  panels/cam              S-M4-0 r 14.8 at (-10.465, +-10.465)                                          (B)
  panels/door             lb_stud at 140 / 182.5 / 225deg; arc slot r 14.8                              (B)
  tests                   frame/test.fg (hng positions), panels/test.fg (bracket scr[2,3] on the top brackets,
                          hinge bracket-first, lift -0.40, 0.5deg check), asm (when F starts)
  ```
- **R12. Latch pivot nut: an ISO 10511 M6 nyloc, not an S-M6-0** (found by subsystem B with Forge's new
  HardwareNearHole check; approved by the coordinator for David).
  - The problem: the latch base's S-M6-0 at the axis had its centre 7.5 from the edges of all four detent
    holes (3.0 at r 9.0); PEM needs 8.6. That is 4 violations on RACK-322.
  - Why not move holes: clearing it needs the detents at r >= 10.1, but the detents at 90 and 180deg lie
    inside the arc slot (inner edge r 11.6), so the bridge to the slot would be 0 or less (shop 0.89 in
    t3). Detents outside the slot (r >= 20.4) would put the cam's ball holes through its R 21.9 edge.
    The detent radius is also tied to the balls in the cam and to the 5.8 ball table.
  - The change (the base only; the door, the cam, the handle, the detents, the strike and the tab keep
    every hole and interface):
    - RACK-322: the S-M6-0 and its 8.75 hole become a plain 6.6 hole (ISO 273 medium for M6). The detent
      holes are then 4.2 from it.
    - The pivot ISO 7379 8 x 16 is unchanged. Its shoulder end still clamps on the base's front face
      (y 1.52), now against an ISO 10511 M6 nyloc nut on the base's back face: nut y 4.54 .. 10.54, the
      thread (end y 12.52) engaged over the whole nut (6.0) and 1.98 past it.
    - The nut sits inside the cam's 12.5 bore: 0.47 radial at a sharp hex (e 11.55), 0.72 at ISO e min
      11.05. It is 2.0 from the balls seated and 1.7 between detents.
    - New anchor on RACK-322: `pivot_back` (back face at the axis, normal +y, x +Z), the nyloc's seat.
      `pivot` stays (front face; the shoulder's clamp).
    - Assembly: fit the nyloc from inside before the cam, and hold it with a 10 mm spanner while the
      pivot is tightened from outside.
  - Counts: 270 PEM parts (was 272: no S-M6-0 in the 2 latch bases); 2 more ISO 10511 M6 (18 in all).
  - Text changed to match: section 4 (RACK-322, the PEM count), 5.8 (RACK-322, pivot), 5.12 (latch pivot),
    section 7 (nuts ISO 10511).
  - Checked: latch_base.fg 0 shop warnings; door_asm.fg 39 of 39 and panels/test.fg 37 of 37 expects pass,
    mobility 0, 0 interferences, pivot engaged 6.0 in its nut.

## 1. Overall concept and dimensions

```
outer width  W   = 600.00   side panel outer face to outer face
outer depth  D   = 800.00   front door outer face to rear door outer face
frame height Hf  = 906.00   base pan underside (Z = 0) to top pan top face
caster height Hc = 111.00   floor to base pan underside (Blickle LKPA-PATH 80KF, section 7)
overall      H   = 1017.00  floor to the top face
usable       18U = 800.10   U1 bottom Z = 52.95, U18 top Z = 853.05, centred on Z = 453 (frame mid-height)
EIA opening      = 450.00   between rail inner edges (x = +-225.00)
EIA hole pitch   = 465.10   hole centres x = +-232.55
rail depth       = 300 to 700 in 25 steps (front rail mount face to rear rail mount face)
                   default: front face y_f = 100, rear face y_r = 725 -> 625
ground clearance = 111      under the base pan; casters on a 370 x 600 kingpin rectangle
mass (estimate)  ~ 72 kg empty (steel), casters rated 4 x 120 kg
```

What it is:
- **Frame:** four identical 50 x 50 L-section corner posts (3.02 steel) bolted to a base pan and a top pan
  (3.02 steel, 30 mm flanges). This is the structure.
- **Rails:** four identical L-section EIA rails (1.88 steel, 9.5 square holes). They bolt to four identical
  rail carriers, which sit on the base pan and hang under the top pan. The carriers are drilled every 25
  mm along the depth, so rails move in 25 mm steps without new parts.
- **Panels and doors:**
  - Four identical side half-panels (1.52 steel), two per side. They are split at mid-height because
    SendCutSend's powder coat limit is 30" x 36" (section 3), and their returns bolt together at the joint.
  - Two identical vented doors (1.52 steel), front and rear. Each hangs on its own pivot hinge (two
    brackets, two shoulder-screw pins) and closes with our own spring-loaded quarter-turn latch (section 5.8),
    whose cam drops into a notched strike on a post.
- **Fan tray:** four 120 mm fans hang from the top pan under a slotted roof grille and exhaust upward.
- **Casters:** four swivel casters, the two front ones total-lock.

Every fabricated part fits 30" x 36" flat (762 x 914.4), so every part can be ordered powder coated.

Rack coordinate system (used by every table in this file):
```
origin  O  : on the rack centreline (X = 0), in the plane of the front door's outer face (Y = 0),
             at the underside of the base pan (Z = 0)
+X         : to the right, seen from the front
+Y         : from front to back (into the rack)
+Z         : up
floor      : Z = -111.00
the rack is mirror-symmetric about X = 0 except the door hinges and latches, and symmetric about
Y = 400 except the same (rear parts are front parts turned 180deg about the vertical axis X = 0, Y = 400)
```

Layout. Front view with the door removed, then a plan section at Z = 453:
```
            X=-300                                X=0                                 X=+300
   Z=906  +----------------------------------------------------------------------------+  top pan (30 deep, flanges down)
          |  post ||   carrier ==|   [ fan ][ fan ]   (fan tray, Z 861.1..893.38)  |== carrier || post |
   Z=876  |-------||-------------|--------------------------------------------------|-------------||-----|
          |       ||  rail web  |o| EIA holes x=-232.55        x=+232.55 |o| rail  |              ||     |
   Z=853  |       ||            |o| ------------- U18 top ----------------|o|       |              ||     |
          | side  ||            |o|                                      |o|       |              || side|
          | panel ||            |o|     18U = 800.10, 450.00 opening      |o|       |              ||panel|
   Z=453  |=======||   (panel joint, returns bolted)                    ...       ...            ||=====|
          |       ||            |o|                                      |o|       |              ||     |
   Z=53   |       ||            |o| ------------- U1 bottom --------------|o|       |              ||     |
   Z=30   |-------||-------------|--------------------------------------------------|-------------||-----|
          |  post ||  carrier ==|                (cable cut-out at rear)            |== carrier   ||post |
   Z=0    +----------------------------------------------------------------------------+  base pan (30 deep, flanges up)
              [caster]                                                          [caster]
   Z=-111 ========================================= floor ==========================================
             x=-185                                                           x=+185

 plan, Z = 453 (front at the bottom)
   Y=800  +--rear door (hinge at x=+278, latch at x=-265)-------------------------+
   Y=765  |L post|                                                      |post J|
          |      |    rear rail mount face y_r = 725 (web forward 45)          |      |
          |      |                                                      |      |
          | side |                 equipment, 450 wide                  | side |
          | panel|                                                      |panel |
          |      |    front rail mount face y_f = 100 (web back 45)          |      |
   Y=35   |L post|  strike (right)                                      |post J|
   Y=25   |  [hinge pin x=-277.98, y=14]  front door, 25 deep  [latch x=265]   |
   Y=0    +-------------------------------------------------------------------------+
         X=-300                                                                X=+300
```

Key planes. Every interface in section 5 is built from these:
```
side panel outer face        x = +-300.00     panel 1.52 thick, inner face x = +-298.48
post outer faces             x = +-298.48 (side legs), y = 35.00 (front legs), y = 765.00 (rear legs)
post inner faces             x = +-295.46, y = 38.02, y = 761.98 (3.02 legs, 50.00 outer each way)
pan outer box                x = +-295.46, y = 38.02 .. 761.98 (pans nest inside the 4 post L's)
base pan                     Z = 0 .. 30.00, floor Z = 0 .. 3.02, flanges up
top pan                      Z = 876.00 .. 906.00, floor Z = 902.98 .. 906.00, flanges down
carrier (rail contact face)  x = +-251.88, y = 42.00 .. 758.00
rail outer corner            x = +-251.88 (web outer face), mount face at y_f (front) / y_r (rear)
rail inner edge              x = +-225.00
door (front)                 x = -290.98 .. +295.48, y = 0 .. 25.00, Z = 39.00 .. 867.00
hinge axis (front)           x = -277.98, y = 14.00, vertical      (rear: x = +277.98, y = 786.00)
latch axis (front)           x = +265.00, Z = 453.00, along Y      (rear: x = -265.00)
caster kingpins              (x, y) = (+-185, 100) and (+-185, 700), plate top at Z = 0, plate long side along Y
fan centres                  (x, y) = (+-70, 330) and (+-70, 470), fan outlet face Z = 891.50
```

Ground clearance and caster positions: the kingpins sit 100 in from the front and rear faces and 115
in from the sides. The swivel envelope (radius 90, section 6) stays inside the 600 x 800 footprint
(x to 275, y 10 to 790), so a swivelling wheel never sticks out past the panels. The kingpins are
not further out because the caster bolts' nuts inside the base pan must stay clear of the rails'
lower ends (x >= 225) at every rail position.

Airflow: cool air enters through the vent slots of the front door, crosses the equipment, and leaves
through the rear door slots and the four roof fans. A cable cut-out at the rear of each pan takes
cables in from the floor or the roof.

## 2. Construction method

- **Bolted, no welding.** Every joint is a screw into a PEM press-in nut, a PEM flush stud with a
  nut, or a through-bolt with a nylon-insert lock nut. SendCutSend welds, but a welded frame can't be
  shipped flat or powder coated in one piece within 30" x 36". Bolting keeps every part small, flat-packed,
  replaceable, and exactly what the model says.
- **Every fabricated part is one cut-and-bent sheet**, ordered from SendCutSend as STEP (bent parts) or
  DXF (flat parts). PEM hardware is pressed in by SendCutSend from their catalogue (section 3). Finish:
  powder coat. SendCutSend inserts hardware before powder coat and masks the threads.
- **Everything else is catalogue hardware** (section 7): casters, cage nuts, screws, nuts, washers,
  shoulder screws, nylon spacers, fans, fan guards, and the latch's spring, balls and spacers.
- **One part number serves every corner where geometry allows:**
  - posts x4, rails x4, carriers x4, side half-panels x4;
  - doors x2, hinge brackets x4, strikes x2, cams x2, latch handles x2, latch bases x2.
  
  Each of these is placed by a rigid motion, never by a mirror (section 5 gives the poses). This needs
  hole patterns symmetric about Z = 453 and Y = 400, which the hole tables respect. Don't break that
  symmetry in a model.
- **Nothing is countersunk.** SendCutSend countersinks only aluminium and plastic 3.175 mm or thicker.
  Flush faces under panels come from PEM FH studs, whose heads sit flush.
- **No tapped holes.** They are available to M6 in 1.88 and thicker, but a thread 1.88 to 3.02 long
  holds less than a pressed nut.
- **No hems, closed corners, jogs, louvres or lanced features.** These are Forge gaps (section 10), and
  none of them is needed.

## 3. Shop profile (SendCutSend, cold rolled 1008 mild steel)

Stocks used (SendCutSend part pages, values as published, read 2026-11-03):
```
                               t3: .119" (11 ga)   t2: .074" (14 ga)   t1: .059" (16 ga)
thickness (mm)                 3.02                1.88                1.50 nominal; model 1.52 *
K factor                       0.38                0.40                0.40
inside bend radius @90 (in)    .063 = 1.60         .063 = 1.60         .063 = 1.60
bend deduction @90 (in)        .196 = 4.98         .129 = 3.28         .108 = 2.74
die width (in)                 .630 = 16.0         .472 = 12.0         .472 = 12.0
min flange after bend (in)     .466 = 11.84        .320 = 8.13         .309 = 7.85
min flange flat (in)           .368 = 9.35         .255 = 6.48         .255 = 6.48
bend relief depth (in)         .202 = 5.13         .157 = 3.99         .142 = 3.61
min corner relief dist (in)    .036 = 0.91         .025 = 0.64         .020 = 0.51
min hole (in)                  .028 = 0.71         .017 = 0.43         -
recommended min hole (in)      .040 = 1.02         .028 = 0.71         -
min hole to edge (in)          .036 = 0.91         .022 = 0.56         - (use 0.56)
min bridge (in)                .035 = 0.89         .032 = 0.81         -
max bending flat part          44" x 30" = 1117.6 x 762.0, max bend length 44", max angle 130deg
max powder coat part           30" x 36" = 762.0 x 914.4      (the binding limit in this design)
max hardware part              36" x 46"; 4-sided box with hardware: flanges <= 3" (76.2)
cut features to bend           >= 1/2 die width from the bend centreline: 8.0 (t3), 6.0 (t2, t1)
bend angle tolerance           +-1deg up to 24" bend length, +-2deg over; bend-to-edge +-.015"
hardness                       1008 CR ~45 HRB (cold rolled page); Brinell 100 on the 14 ga page
                               (about HRB 57): below the HRB 80 limit of S/SS/FH hardware
* the 16 ga model thickness: SendCutSend lists .059" (1.50). The doors and panels are modelled
  at 1.52 so their bend numbers come from the same K, R as the table; change to 1.50 if David wants
  the nominal (no interface depends on it to better than 0.02).
```
The 16 ga cut-spec popup wasn't read (only its bending row, from the bending calculator table). Its
hole-to-edge and min-hole values above are taken from 14 ga, which is the larger, conservative value.

The tooling numbers above are consistent with each other: Forge's BD = 2(r + t) - (pi/2)(r + K t)
gives 4.92 / 3.27 / 2.77 against SendCutSend's 4.98 / 3.28 / 2.74.

Sources:
- https://sendcutsend.com/materials/mild-steel/cold-rolled/ (per-thickness spec tabs .074" and .119")
- https://sendcutsend.com/bending-calculator/ (K, BD, R, die, min flange, relief for every thickness)
- https://sendcutsend.com/materials/processing-min-max/ (bending, hardware, powder coat size limits)
- https://sendcutsend.com/bending/ (tolerances, box flange limits, "1/2 die width from each bend centerline")
- https://sendcutsend.com/guidelines/sheet-cutting/ (min hole 50% of t for laser)
- https://sendcutsend.com/services/hardware/ and https://sendcutsend.com/guidelines/hardware/catalog/
  (hardware they install, ranges, insertion before powder coat)
- https://sendcutsend.com/services/countersinking/ (aluminium and plastic >= .125" only)
- https://sendcutsend.com/services/tapping/ (M6 max at .074", hole sizes)

Forge `hole_to_bend` is measured from the hole edge to the bend's tangent line; SendCutSend's rule is
from the bend centreline. The difference is half the 90deg bend allowance (BA/2 = 2.16 / 1.85 / 1.73),
so the Forge limits are 8.0 - 2.16 = 5.84 (t3) and 6.0 - 1.85 = 4.15 (t2), 6.0 - 1.73 = 4.27 (t1).

Hardware SendCutSend installs that this design uses (PEM bulletins CL, FH; values per PEM, which
overrides two typos in SendCutSend's catalogue):
```
code      kind   thread  hole (+0.08)  min sheet  C/L to edge  body (E x T)     max hole in mate  source
S-M4-0    nut    M4      5.41          0.80       6.9          7.87 x 2.00      -                 PEM CL bulletin
SS-M5-0   nut    M5      6.35          0.80       7.1          8.64 x 2.00      -                 PEM CL bulletin
S-M6-0    nut    M6      8.75          1.22       8.6          11.18 x 4.08     -                 PEM CL bulletin, SCS catalogue
FH-M4-12  stud   M4x12   4.00          1.00       7.2          head 5.9 flush   4.6               PEM FH bulletin, SCS catalogue
FH-M6-20  stud   M6x20   6.00          1.60       7.9          head 8.2 flush   6.6               PEM FH bulletin
FH-M8-25  stud   M8x25   8.00          2.40       9.6          head 9.6 flush   8.6               PEM FH bulletin, SCS catalogue
sheet hardness limit: S, SS, FH: HRB 80 (all three stocks pass)
```
- PEM CL bulletin: https://damlite.pemnet.com/CLS-M6-1_pdf_2.pdf
- PEM FH bulletin: https://damlite.pemnet.com/FH-M5-12ZC_pdf_1.pdf
- Plain-steel S/SS nuts are what SendCutSend presses into steel. They do not offer CLS/CLSS, the
  stainless versions, and those are only for HRB <= 70 sheet anyway.

The shop in Forge (one shared file, `hw/shop.fg`, included by value: every sheet part declares the
same `shared shop = ...` text until Forge has imports; section 10):
```
shared t3 = 3.02
shared t2 = 1.88
shared t1 = 1.52
shared r_scs = 1.6
shared kerf_scs = 0.2                       # not published by SendCutSend; Forge's stand-in
shared hw_scs = [
  hardware(kind: "nut",  size: "M4", sheet: (0.8, 12.7), hole: 5.41, edge: 6.9, max_hardness: ratio(80),
           code: "S-M4-0",   source: "PEM CL bulletin; sendcutsend.com hardware catalog"),
  hardware(kind: "nut",  size: "M5", sheet: (0.8, 12.7), hole: 6.35, edge: 7.1, max_hardness: ratio(80),
           code: "SS-M5-0",  source: "PEM CL bulletin; sendcutsend.com hardware catalog"),
  hardware(kind: "nut",  size: "M6", sheet: (1.22, 12.7), hole: 8.75, edge: 8.6, max_hardness: ratio(80),
           code: "S-M6-0",   source: "PEM CL bulletin; sendcutsend.com hardware catalog"),
  hardware(kind: "stud", size: "M4", sheet: (1.0, 12.7), hole: 4.0, edge: 7.2, max_hardness: ratio(80),
           code: "FH-M4-12", source: "PEM FH bulletin; sendcutsend.com hardware catalog"),
  hardware(kind: "stud", size: "M6", sheet: (1.6, 12.7), hole: 6.0, edge: 7.9, max_hardness: ratio(80),
           code: "FH-M6-20", source: "PEM FH bulletin; sendcutsend.com hardware catalog"),
  hardware(kind: "stud", size: "M8", sheet: (2.4, 12.7), hole: 8.0, edge: 9.6, max_hardness: ratio(80),
           code: "FH-M8-25", source: "PEM FH bulletin; sendcutsend.com hardware catalog")]
# [R5] stock() limits are per_t(..) values (a plain number is WrongType), as in hw/shop.fg
shared scs = shop([
  stock(t: t3, bend: kfactor(ratio(0.38)), radii: [r_scs], min_flange: per_t(ratio(0), plus: 11.84),
        hole_to_bend: per_t(ratio(0), plus: 5.84), hole_to_edge: per_t(ratio(0), plus: 0.91), min_hole: per_t(ratio(0), plus: 1.02), min_slot: per_t(ratio(0), plus: 1.02), hardness: ratio(57),
        relief: relief_rule(shape: "rect", width: per_t(ratio(1)), depth: 5.13, from: "bend_line",
                            corner: "round", corner_placement: "intersection")),
  stock(t: t2, bend: kfactor(ratio(0.40)), radii: [r_scs], min_flange: per_t(ratio(0), plus: 8.13),
        hole_to_bend: per_t(ratio(0), plus: 4.15), hole_to_edge: per_t(ratio(0), plus: 0.56), min_hole: per_t(ratio(0), plus: 0.71), min_slot: per_t(ratio(0), plus: 0.71), hardness: ratio(57),
        relief: relief_rule(shape: "rect", width: per_t(ratio(1)), depth: 3.99, from: "bend_line",
                            corner: "round", corner_placement: "intersection")),
  stock(t: t1, bend: kfactor(ratio(0.40)), radii: [r_scs], min_flange: per_t(ratio(0), plus: 7.85),
        hole_to_bend: per_t(ratio(0), plus: 4.27), hole_to_edge: per_t(ratio(0), plus: 0.56), min_hole: per_t(ratio(0), plus: 0.71), min_slot: per_t(ratio(0), plus: 0.71), hardness: ratio(57),
        relief: relief_rule(shape: "rect", width: per_t(ratio(1)), depth: 3.61, from: "bend_line",
                            corner: "round", corner_placement: "intersection"))],
  kerf: kerf_scs, hardware: hw_scs)
```
- Relief width isn't published, so `width = t` is Forge's rule of thumb.
- The relief depth is SendCutSend's "bend relief depth", measured from the bend line. Forge deepens it
  to the tangent line if that falls short.
- Hardness 57 is the 14 ga page's Brinell 100, converted. It is the larger of SendCutSend's two
  figures, so it's the conservative one.

These limits are not in Forge's shop, so each part's README states them and they are checked by hand
(section 8):
- max flat 762 x 914.4 (powder coat);
- max bend length 1117.6;
- max 76.2 box flange with hardware;
- max bend angle 130deg.

## 4. Part list

Fabricated parts. "Flat" is the bounding box of the flat pattern, estimated from the outer dimensions
less the bend deductions above. Every flat fits 762 x 914.4. Subsystems: A frame and rails, B panels and
doors, C casters, D fan tray, E hardware library, F top assembly.
```
RACK-101  post                       x4  t3  L 50 x 50 outer, 906 long          1 bend (90)   flat ~95 x 906     A
          features per leg (both legs identical): 2 FH-M6-20 studs (pans), 6 SS-M5-0 (side panels),
          4 S-M6-0 (hinge brackets), 2 S-M6-0 (strike); 28 hardware parts per post
RACK-111  rail carrier               x4  t2  L 32 (horizontal) x 40 (vertical) outer, 716 long
                                                                                1 bend         flat ~69 x 716     A
          28 S-M6-0 in the vertical leg (rail screws, 25 pitch), 3 S-M6-0 in the horizontal leg (pan screws)
RACK-121  base pan                   x1  t3  591 x 724 outer, 30 flanges up       4 bends, 4 auto corner reliefs
                                                                                               flat ~646 x 779    A
          holes: 8 post-stud clearances, 6 carrier screw clearances, 16 caster bolt holes, rear cable cut-out
RACK-131  top pan                    x1  t3  same outline as RACK-121              4 bends        flat ~646 x 779    A
          holes: 8 post-stud clearances, 6 carrier screw clearances, 4 FH-M8-25 studs (fan tray),
          4 fan grilles (11 slots each), rear cable cut-out
RACK-201  EIA rail                   x4  t2  L 26.88 (mount face) x 45 (web) outer, 890 long
                                                                                1 bend         flat ~69 x 890     A
          54 square holes 9.50 x 9.50 (18U x 3) on x = 465.1 pitch; 4 web holes 6.6 (2 per end)
RACK-141  side half-panel            x4  t1  800 x 453 field, 20 return on the joint edge (y 90..710 only)
                                                                                1 bend         flat ~800 x 471    B
          6 screw holes 5.5 (to posts), 3 joint holes 5.5 (in the return)
RACK-301  door                       x2  t1  586.46 x 828 field, 4 returns 25 deep   4 bends, 4 auto corner reliefs
                                                                                               flat ~632 x 874    B
          2 S-M6-0 (hinge pins, in top and bottom returns), 3 FH-M4-12 (latch base), latch holes (5.8),
          154 vent slots 6 x 100
RACK-311  hinge bracket              x4  t2  L: 34 wide, 34 horizontal leg x 47 vertical leg outer
                                                                                1 bend         flat ~34 x 78      B
          pin hole 8.2, 2 screw holes 6.6
RACK-312  strike                     x2  t2  L: base 30.4 x 40 (on post) + web 33.48 deep with an open notch (5.8)
                                                                                1 bend         flat ~40 x 48      B
RACK-313  latch cam                  x2  t2  flat, disc R 21.9 + arm to r 27, 2 S-M4-0                0 bends  B
RACK-321  latch handle               x2  t2  disc R 23.5 with a 24 fin, padlock hole       1 bend   flat ~47 x 61      B
RACK-322  latch base                 x2  t3  53.4 x 59 plate, pivot hole 6.6 [R12], padlock tab 40.5  1 bend   flat ~54 x 97   B
RACK-401  fan tray                   x1  t2  300 x 300 plate, 2 flanges 20 (down, along Y)   2 bends  flat ~300 x 336  D
          4 openings 116, 16 S-M4-0 (fan screws), 4 holes 8.6 (hangers)
```
Reference parts (modelled for checks, not ordered):
- RACK-D01, D02, D04: dummy 1U, 2U and 4U servers;
- RACK-FLOOR: the floor plane at Z = -111.

Count: 33 fabricated parts of 13 numbers; 112 (posts) + 124 (carriers) + 10 (doors: 4 S-M6-0, 6 FH-M4-12)
+ 4 (top pan) + 16 (fan tray) + 4 (cams) = 270 PEM parts [R12: was 272 with 2 in the latch bases] pressed in by SendCutSend. Order these
quantities plus one spare of each small part.

## 5. Interface control

This is the section that makes independently modelled parts fit.

**Part frames.** Each part file is modelled so that its own world frame is its **part frame**, defined
below by the part's faces. That way a part can be checked alone, and the assembly places it by the
pose given.
- A pose is written `origin -> (rack point)` with each part axis mapped onto a rack axis.
- All poses are rigid motions; none is a mirror.

**Anchors.** Every anchor named here must exist in the part file, spelled exactly as written:
- each is `anchor(plane(at: POINT, normal: DIR, x: DIR), kind: .., size: .., depth: ..)`;
- the normal is the bolt or joint axis, pointing in the direction stated;
- a pair that is fastened has the same origin and the same normal in rack coordinates;
- kinds: `internal_thread` (a PEM nut's or a nut's thread), `external_thread` (a screw's or a stud's
  thread), `bore` (a running hole), `shaft` (a pin);
- [R1] a sheet part's PEM nut anchor is a PLACEMENT anchor (no `kind`): origin on the nut's flush face,
  normal the screw's direction. The thread anchor belongs to the PEM nut part in `hw/` (`hw/pem_*.fg`, its
  thread is shank A + head T: 5.23 for S-M6-0, 2.77 for S-M4-0 and SS-M5-0, in every sheet), placed with
  `fastened(nut.flush, sheet.<anchor>)`. Stud anchors likewise: placement only; the thread is the stud part's.

Hole sizes used throughout (ISO 273 medium clearance): M5 5.5, M6 6.6, M8 9.0; a hole a PEM stud passes
through is the bulletin's "max hole in attached part" or less: FH-M6 6.6, FH-M8 8.6. Hinge pin bore 8.2.

### 5.1 Post RACK-101

Part frame: origin at the bottom end of the outer corner line. **u** runs along leg A's outer face,
away from the corner. **v** runs along leg B's outer face. **w** runs along the length (u x v = w).
```
leg A   u 0..50.00, v 0..3.02, w 0..906.00     outer face v = 0
leg B   u 0..3.02,  v 0..50.00, w 0..906.00    outer face u = 0
bend    inside radius 1.60 at the corner
```
Both legs carry the same pattern. Here s is the distance from the corner along the leg: u for leg A,
v for leg B.
```
item               PEM / hole      s      w                                        anchors (leg A / leg B)        normal
pan stud           FH-M6-20        25.0   15.0, 891.0                              stud_a[0..1] / stud_b[0..1]    into the post (+v / +u)
side panel nut     SS-M5-0         25.0   100, 260, 420, 486, 646, 806              pnl_a[0..5] / pnl_b[0..5]      +v / +u
hinge bracket nut  S-M6-0          32.5   50, 66, 840, 856  [R11, was 20.5]         hng_a[0..3] / hng_b[0..3]      +v / +u
strike nut         S-M6-0          25.0   443, 463                                  stk_a[0..1] / stk_b[0..1]      +v / +u
```
- **Stud anchors** are `external_thread` M6, depth 20. Each origin is on the outer face, where the
  flush head sits; the shank runs inward.
- **Nut anchors** [R1] are placement anchors (no kind; the hw nut part carries the thread: M5 2.77,
  M6 5.23). Each origin is on the outer
  face, the nut's flush side; the bodies sit on the inner side.
- **The pattern is symmetric about w = 453**, so a post turned end over end is the same part.

Poses. Leg roles at each corner:
```
corner  origin (rack)            u     v     w     front/rear leg   side leg   uses
FL      (-298.48,  35.00, 0)     +X    +Y    +Z    A                B          hng_a (front door hinge), pnl_b, stud_a, stud_b
FR      (+298.48,  35.00, 0)     +Y    -X    +Z    B                A          stk_b (front strike), pnl_a, studs
RR      (+298.48, 765.00, 0)     -X    -Y    +Z    A                B          hng_a (rear door hinge), pnl_b, studs
RL      (-298.48, 765.00, 0)     -Y    +X    +Z    B                A          stk_b (rear strike), pnl_a, studs
```
Unused nuts and studs stay in. They cost a few cents, and one part number saves far more.

### 5.2 Base pan RACK-121 and top pan RACK-131

Part frame for both pans: origin at the centre of the floor's outer face. x and y are in the floor;
z points into the pan, towards the flanges.
```
outer box   x -295.46..295.46, y -361.98..361.98, z 0..30.00 (outer, flange height measured outer)
floor       z 0..3.02;  4 flanges up (+z), 3.02 thick, automatic circular corner reliefs
```
Poses:
```
base pan  origin (0, 400, 0)       x -> +X, y -> +Y, z -> +Z
top pan   origin (0, 400, 906)     x -> -X, y -> +Y, z -> -Z      (turned 180deg about Y)
```
Holes, in pan frame. Both pans unless marked.
```
post stud clearance  6.6   side flanges (x = +-295.46): (y = -340, z = 15), (y = +340, z = 15)
                           front/rear flanges (y = -+361.98): (x = -273.48, z = 15), (x = +273.48, z = 15)
                           anchors stud_fl_f, stud_fl_s, stud_fr_f, stud_fr_s, stud_rl_r, stud_rl_s,
                           stud_rr_r, stud_rr_s (corner letters as RACK-101 poses, in the BASE pan's
                           orientation; the top pan uses the same names at the same pan-frame points);
                           origin on the flange's outer face, normal into the pan
carrier screws       6.6   floor: (x = -271.00, y = -300, 0, +300) and (x = +271.00, same y)
                           anchors car_l[0..2], car_r[0..2]: origin on the outer face (z = 0), normal +z
caster bolts         9.0   base pan only; kingpins (x, y) = (-185, -300), (+185, -300), (-185, +300), (+185, +300)
                           (corners fl, fr, rl, rr); 4 holes at kingpin + (+-22.5, +-37.5)
                           anchors caster_fl, caster_fr, caster_rl, caster_rr: origin at the kingpin on
                           the floor's outer face, normal +z (into the pan, as the caster's `top`) [R9], x along +x
cable cut-out        -     rect x -125..125, y 250..310, corner radius 10 (the rear in both pans)
fan tray studs       FH-M8-25, top pan only: (x, y) = (+-130, +-140); heads flush on the outer face (z = 0),
                           shanks into the pan (+z); anchors hang[0..3] (order: (-130,-140), (130,-140),
                           (-130,140), (130,140)), external_thread M8 depth 25
fan grilles          -     top pan only, 4 grilles at (x, y) = (+-70, +-70); each 11 slots parallel to x,
                           width 6.0, centrelines at y = c + (i - 5) * 9.5 (i = 0..10), each slot's ends
                           on a circle of diameter 112 about the fan centre:
                           overall length L_i = 2 sqrt(56^2 - (|y_i - c| + 3)^2)
```
The top pan's local -x lands on rack +X, but every hole set above is symmetric in x, so the pans need no
handed features. Floor flatness: the base pan carries the casters on a 3.02 floor; no stiffener is
specified. If the floor deflects visibly under load, add a hat-section cross member (a new part) over
the caster lines.

### 5.3 Rail carrier RACK-111

Part frame: origin on the outer corner line (the L's outside corner) at the front end. **a** runs
along the length, **b** along the horizontal leg, **c** up the vertical leg (a x b = c).
```
horizontal leg   a 0..716, b 0..32.00, c 0..1.88       outer face c = 0 (sits on the pan)
vertical leg     a 0..716, b 0..1.88,  c 0..40.00      outer face b = 0 (the rail web bears on it)
```
```
rail nuts   S-M6-0   vertical leg, flush on b = 0, body on +b   a = 20.5 + 25 k (k = 0..27), c = 20.00
            anchors rail[0..27]: placement [R1] (hw S-M6-0, thread 5.23), origin on b = 0, normal +b
pan nuts    S-M6-0   horizontal leg, flush on c = 0, body on +c   (a, b) = (58, 19.12), (358, 19.12), (658, 19.12)
            anchors pan[0..2]: placement [R1] (hw S-M6-0), origin on c = 0, normal +c
```
Poses:
```
BL  origin (-251.88,  42.00,   3.02)   a +Y   b -X   c +Z
BR  origin (+251.88, 758.00,   3.02)   a -Y   b +X   c +Z
TL  origin (-251.88, 758.00, 902.98)   a -Y   b -X   c -Z
TR  origin (+251.88,  42.00, 902.98)   a +Y   b +X   c -Z
```
Results, in rack coordinates:
- rail nuts at y = 62.5 + 25 k (the same set from both ends, because the pattern is symmetric about
  a = 358), Z = 23.02 (bottom) and 882.98 (top);
- pan nuts at (x = +-271.00, y = 100, 400, 700).

### 5.4 EIA rail RACK-201

Part frame: origin on the outer corner line at the bottom end. **p** runs across the mount face, **q**
along the web, **r** along the length.
```
mount face   p 0..26.88, q 0..1.88, r 0..890      front face q = 0 (equipment ears bear on it)
web          p 0..1.88,  q 0..45.00, r 0..890     outer face p = 0 (bears on the carrier)
inner edge   p = 26.88  (rack x = +-225.00: the 450.00 opening)
```
```
EIA holes   9.50 x 9.50 square, sharp corners (laser), sides along p and r
            centre p = 19.33 (rack x = +-232.55), r = 44.95 + 44.45 (n - 1) + o
            n = 1..18, o in {6.35, 22.225, 38.10}; r runs 51.30 .. 838.70 (symmetric about r = 445)
            anchors eia[0..53] (k = 3 (n - 1) + j): origin on q = 0 at the centre, normal +q (a screw
            from the front advances +q), x along +r
web holes   6.6 at (q = 12.5, r = 15.02), (q = 37.5, r = 15.02), (q = 12.5, r = 874.98), (q = 37.5, r = 874.98)
            anchors web_bot[0..1], web_top[0..1]: origin on p = 0, normal -p
```
Poses. y_f and y_r are the assembly params `rail_front` and `rail_rear`, multiples of 25 with
50 <= y_f < y_r <= 750 and y_r - y_f >= 300.
```
LF  origin (-251.88, y_f,   8.00)   p +X   q +Y   r +Z
RF  origin (+251.88, y_f, 898.00)   p -X   q +Y   r -Z      (LF turned 180deg about Y)
RR  origin (+251.88, y_r,   8.00)   p -X   q -Y   r +Z      (LF turned 180deg about Z)
LR  origin (-251.88, y_r, 898.00)   p +X   q -Y   r -Z
```
Results, in rack coordinates:
- U n's bottom boundary is at Z = 52.95 + 44.45 (n - 1), the same on all four rails, so front and rear
  holes line up;
- the web holes land on carrier nuts when (y_f + 12.5) and (y_r - 12.5) are in {62.5 + 25 k}, which
  is true for every multiple of 25 in the range.

The web holes are 15.02 from the rail end; the carrier nut line is Z = 23.02 / 882.98.

### 5.5 Side half-panel RACK-141

Part frame: origin at the outer face's corner that sits at the front bottom on the lower-left
instance. **e** runs along the depth, **f** up the height, **g** inward, through the thickness
(e x f = g).
```
field    e 0..800, f 0..453.00, g 0..1.52     outer face g = 0
return   on the f = 453 edge, turned to +g, 20.00 outer: f 451.48..453.00, g 0..20.00, e 90..710 only
         (cut back clear of the posts; section 10, gap 1)
holes    5.5 at e in {60, 740} x f in {100, 260, 420}      anchors scr[0..5] (e = 60 first, f ascending),
                                                           origin on g = 0, normal +g
         5.5 in the return at (e = 250, 400, 550; g = 14)  anchors jnt[0..2]: origin on the return's
                                                           outer face (f = 453), normal -f
```
Poses:
```
LL  origin (-300.00,   0, 0)     e +Y   f +Z   g +X
UL  origin (-300.00, 800, 906)   e -Y   f -Z   g +X      (LL turned 180deg about X)
LR  origin (+300.00, 800, 0)     e -Y   f +Z   g -X      (LL turned 180deg about Z)
UR  origin (+300.00,   0, 906)   e +Y   f -Z   g -X
```
Results, in rack coordinates:
- panel screws at (y = 60, 740) and Z = 100, 260, 420 (lower) and 486, 646, 806 (upper), which are
  the post nuts pnl_*;
- the two returns meet face to face at Z = 453.00, and their jnt holes coincide.

### 5.6 Door RACK-301

Part frame: origin at the outer face's bottom corner on the hinge side. **h** runs across the width
towards the latch, **k** up, **n** out of the outer face (h x k = n). The material is at n <= 0.
```
field     h 0..586.46, k 0..828.00, n -1.52..0
returns   all 4 edges, turned to -n, 25.00 outer (n -25..0); automatic circular corner reliefs
pin nuts  S-M6-0 in the bottom return, flush on k = 0, body +k: (h = 13.0, n = -14.0)    anchor pin_bot, normal +k
          S-M6-0 in the top return,    flush on k = 828, body -k: (h = 13.0, n = -14.0)  anchor pin_top, normal -k
          both placement anchors [R1] (hw S-M6-0, thread 5.23); hinge axis = the line h = 13.0, n = -14.0 (anchor hinge = pin_bot)
latch     centred (h = 555.98, k = 414.00): pivot hole, arc slot, 3 FH-M4-12 studs, padlock tab slot (5.8);
          anchor latch: origin on n = 0, normal +n, x along -k
vents     154 slots (160 less 6 by the latch, 5.8), 6.0 wide x 100.0 overall, parallel to h: 4 columns centred h = 113.23, 233.23,
          353.23, 473.23; 40 rows centred k = 414 + (i - 19.5) * 16 (i = 0..39)
```
Poses:
```
front  origin (-290.98,   0, 39)   h +X   k +Z   n -Y
rear   origin (+290.98, 800, 39)   h -X   k +Z   n +Y      (front turned 180deg about Z through (0, 400))
```
Results, in rack coordinates:
- front hinge axis x = -277.98, y = 14.00; rear hinge axis x = +277.98, y = 786.00;
- front latch axis at x = +265.00, Z = 453.00; rear latch axis at x = -265.00.

### 5.7 Hinge bracket RACK-311 and the pivot stack

Part frame: origin on the pin axis, on the horizontal leg's top face. **i**, **j**, **zb** follow rack
X, Y, Z on the front-bottom instance.
```
horizontal leg  i -17..17, j -13.00..21.00, zb -1.88..0        outer (bottom) face zb = -1.88
vertical leg    i -17..17, j 19.12..21.00, zb -1.88..44.92     outer (rear) face j = 21.00
pin bore        8.2 at (0, 0)                                  anchor pin: origin (0, 0, 0), normal +zb, kind bore 8
screw holes     [R11] 6.6 at i = +12 and i = -12, each at zb = 20.92 and 36.92: anchors scr[0] (+12, 20.92),
                scr[1] (+12, 36.92), scr[2] (-12, 20.92), scr[3] (-12, 36.92); origin on j = 21, normal +j,
                x = +zb. Bottom brackets use scr[0,1], top brackets scr[2,3] (R8); the other pair is unused
```
Poses:
```
front-bottom  origin (-277.98,  14.00,  29.08)   i +X   j +Y   zb +Z
front-top     origin (-277.98,  14.00, 876.92)   i -X   j +Y   zb -Z
rear-bottom   origin (+277.98, 786.00,  29.08)   i -X   j -Y   zb +Z
rear-top      origin (+277.98, 786.00, 876.92)   i +X   j -Y   zb -Z
```
Results, in rack coordinates:
- front bracket screws at (x = -265.98, y = 35, Z = 50, 66 / 840, 856), which are post FL hng_a (s 32.5)
  [R11]. Their heads (r 5.25) are 12 off the pin axis, clear of the hinge return's sweep (which passes
  y 29.82 only within 6.3 of the axis);
- the vertical leg's front face is at y = 33.12. The door's swept volume reaches y = 31.03 at most
  (section 6).

Pivot stack, bottom hinge, from the bottom up (the top hinge is the same stack, mirrored about Z = 453):
```
Z 21.50 .. 27.00   shoulder screw head (ISO 7379, d_s 8, M6, l_s 12: head 13 dia x 5.5)
Z 27.20 .. 29.08   bracket horizontal leg (0.20 axial play under the head)
Z 29.08 .. 37.08   nylon spacer 8.2 ID x 14 OD x 8.0
Z 37.08 .. 38.68   washer ISO 7089-8 (8.4 ID x 16 OD x 1.6)
Z 38.68 .. 39.00   running play 0.32
Z 39.00 .. 40.52   door bottom return; the shoulder ends here and clamps on it
Z 40.52 .. 50.00   M6 thread (11 long) in the door's S-M6-0 nut; 5.23 engaged (the nut's whole thread) [R3], 5.40
                   through and past the nut, inside the door
```
The screw turns with the door. The bracket bore (8.2 on 8.0) and the spacer are the bearing. The door's
weight goes through washer, spacer and bracket.

### 5.8 Latch: our own quarter-turn compression latch

David's decision (section 12): no purchased lock. The latch is built from SendCutSend sheet parts and
catalogue hardware only.

How it works:
- A handle outside the door and a cam inside are one rigid rotating unit (**U**). They are joined by
  two link posts that run through a 180deg arc slot in the door and in a latch base screwed inside the
  door. U turns on a fixed shoulder-screw pivot.
- A coil spring between the door's outer face and the handle pulls U outward. That presses two steel
  balls, held in the cam, against the back of the latch base, which has detent holes at open and at
  closed.
- At closed, the cam's arm has swung into a notch in the strike. The notch's front edge holds the cam
  0.40 further in than its detent seat, so the spring's force passes to the door:
  - it pulls the door inward onto the strike's tip, which is the door stop (preload);
  - an outward pull meets the balls after 0.40, so the door moves at most 0.40 before it stops hard.
- The ends of the arc slot are the hard stops at 0 and 90deg.
- Closed, a hole in the handle's fin lines up with a hole in a tab that the latch base pushes out
  through the door, so a padlock fits.

**Frames.** The latch axis is normal to the door. Its anchor `latch` is on the door, with its normal
**out** of the door's outer face (the door's +n). Latch frame:
- **u** = the arm's direction;
- **v** = u turned +90deg about the latch normal;
- **w** = the latch normal.

Turn 0 is OPEN, with the arm pointing down (rack -Z). Turn 90deg is CLOSED, with the arm pointing at
the strike (front door: rack +X). Seen from outside the front door, closing is anticlockwise. Front
door: the latch axis is at rack (x = 265.00, Z = 453.00), y running along -Y; the rear door is
turned 180deg about Z, giving x = -265.00. Every radius below is about this axis.

Rack y of the U parts, front door. y_d = detented (balls seated), y_b = balls riding between detents,
y_c = closed (held by the strike):
```
part                    y_d (detented)      y_b (between)       y_c (closed)
latch base (fixed)      1.52 .. 4.54        same                same          (on the door field's inner face)
balls 4.0               centres 5.86        centres 6.54        centres 6.26   (captive in the cam) [R11]
cam front .. back       6.83 .. 8.71        7.51 .. 9.39        7.23 .. 9.11
handle inner .. outer   -8.17 .. -10.05     -7.49 .. -9.37      -7.77 .. -9.65
spring height           8.17                7.49                7.77
fin tip (outward)       -34.05              -33.37              -33.65
pivot head underside    -14.48 (fixed; 4.43 clear of the handle when detented)
```
A ball (4.0) sits in a 3.5 hole in the cam, so its centre is 0.97 in front of the cam's front face.
It meets the base's back face (y 4.54) when off a detent, and drops 0.68 into a 3.0 detent hole.
Closed, the strike holds the cam 0.40 behind its detented position. The balls stay 0.28 engaged, so
the closed position has both the strike's seat and part of the ball detent.

**Parts** (all t2 = 1.88 steel except the base, t3 = 3.02):
```
RACK-321  handle       x2  t2   disc R 23.5 about the axis, cut by a chord at v = +18.5; a fin turned out (away from
                                the door, +w) off that chord, 24.0 outer; the fin's width is the chord, 29.0
                                bore 8.2 at the axis (rides on the pivot's shoulder)
                                2 holes 4.5 at (u, v) = (-10.465, +10.465) and (-10.465, -10.465)  (r 14.8 at 135deg,
                                225deg) [R11: was r 15; 4.18 from the fin's bend tangent, shop 4.15]
                                padlock hole 7.0 in the fin at u = 0, 16.0 out from the handle's outer face
                                flat ~47 x 61, 1 bend
RACK-322  latch base   x2  t3   rect, in the latch frame at CLOSED: x -32.0 .. +21.4, z -27.0 .. +32.0 (rack: x 233.0 ..
                                286.4, Z 426.0 .. 485.0 on the front door)
                                pivot hole 6.6 at the axis [R12: was an S-M6-0]; the pivot's ISO 10511 M6 nyloc seats on the back face
                                3 holes 4.6 for the door's FH-M4-12 studs at r 28.0, rack angle 140, 182.5, 225deg
                                [R11: was 60, 150, 225; 7.08 from the tab's bend, 1.73 from the left edge]
                                arc slot: r 14.8 [R11], width 6.4, from rack angle 45 to 225deg (round ends)
                                4 detent holes 3.0 at r 9.0, rack angle 0, 90, 180, 270deg
                                padlock tab: a flange off the top edge (Z 485), turned out through the door
                                (rack -Y), 40.54 outer from the base's back face, so it reaches y = -36.0;
                                padlock hole 7.0 at rack (x 265, y -25.65): on the closed handle fin hole
                                flat ~54 x 97, 1 bend
RACK-313  cam          x2  t2   a disc R 21.9 about the axis, plus an arm along +u to r 27.0 (width 16, R 8 end);
                                bore 12.5 (clears the pivot nut's body and thread end)
                                2 S-M4-0 at (u, v) = (-10.465, +-10.465) (r 14.8 [R11]: 7.1 to the hub edge, PEM 6.9),
                                flush on the front face, bodies to the back
                                2 ball holes 3.5 at (u, v) = (0, +9.0), (0, -9.0); flat part, no bends
door additions (RACK-301), in the door frame (front pose: rack x = h - 290.98, Z = k + 39):
                                latch centre (h 555.98, k 414.00): pivot clearance 8.6
                                arc slot r 14.8 [R11], width 6.4, rack angle 45 to 225deg (same as the base)
                                3 FH-M4-12 studs (heads flush on the outer face) at r 28.0, rack angle 140, 182.5, 225deg
                                [R11: the 60deg stud ran into the padlock tab slot]
                                padlock tab slot: rack x 232.6 .. 286.8, Z 478.5 .. 485.5 (clears the tab's bend zone)
                                vents: the 6 slots of column 4 with rows i = 17..22 are left out (154 slots), clear
                                of the latch base
```
Rack angle is measured in the XZ plane of the front door from +X towards +Z, as seen from the front.
On the rear door, use the same angles seen from behind.

**Strike RACK-312 (revised).** Base unchanged: x -14.00 .. 16.40, z -20 .. 20, on post FR's stk_b
nuts. The web and the notch change, in the strike frame (origin rack (273.48, 35, 453)):
```
web    x 14.52 .. 16.40 (rack 288.00 .. 289.88), y -33.48 .. 0 (rack 1.52 .. 35.00), z -20 .. 20
       its front end (y = -33.48, rack y 1.52) is the door stop: the door field's inner face rests on it
notch  in the web, open at the web's bottom edge: y -27.77 .. -25.00 (rack 7.23 .. 10.00), z -20 .. +10
       (rack Z 433 .. 463); its front edge (rack y 7.23, facing the rack's inside) is the cam's seat
       prong (web ahead of the notch): rack y 1.52 .. 7.23, 5.71 deep
anchors scr[0..1] as before; seat: origin (15.46, -27.77, 0), normal +y; stop: origin (15.46, -33.48, 0), normal -y
```
- Closed, the cam crosses the web plane through the notch at rack Z 446.1 to 459.9. Its tip at x 292.0
  is 2.12 past the web's far face and 1.96 short of the door's side return.
- Swinging in, the cam's front face is at y_b = 7.51, behind the seat (7.23), so it enters the notch
  without touching it. It drops onto the seat only at 90deg.
- The cam's hub (R 21.9) never reaches the web (x 286.9 at most).

**Fasteners** (per latch):
```
pivot        ISO 7379 8 x 16, M6: from outside, through the handle bore and the door's 8.6 hole; the shoulder
             end clamps on the base's front face (y 1.52); thread through the base's 6.6 hole into an ISO 10511 M6 nyloc on its back face (6.0 engaged, 1.98 past) [R12]; thread
             end y 12.52, inside the cam's bore
spring       compression spring on the pivot shoulder, between the door's outer face and the handle: ID >= 8.4,
             OD <= 13, free length ~12, solid length <= 5.0, rate ~8 N/mm (~33 N at 7.77: the door preload)
link posts   2 round spacers 6.0 OD x 4.3 ID x 15.0, steel; through the door and base arc slots, between the
             handle's inner face and the cam's front face
link screws  2 M4 x 25 (ISO 7380-1) from outside through the handle and spacer into the cam's S-M4-0
             (4.24 past the nut) [R3]
balls        2 steel balls 4.0 (ISO 3290 G100), captive between the cam and the base
base studs   3 FH-M4-12 in the door (SendCutSend), through the base's 4.6 holes; ISO 7089-4 washer +
             ISO 10511-M4 nylon-insert nut inside (1.66 past the nut)
```

**Anchors:**
- **door:**
  - `latch`: origin at the latch centre on the door's outer face, normal +n (outward), x = rack -Z
    (the arm direction at turn 0);
  - `lb_stud[0..2]` (rack angle 140, 182.5, 225deg in that order [R11]): placement (R1), on the outer face,
    normal -n, x +k.
- **latch base:**
  - `pivot`: placement [R1] (hw S-M6-0), origin on the front face, normal +y (into the rack);
  - `stud[0..2]`;
  - `padlock`: hole centre, normal +Z.
- **handle:**
  - `latch_axis` [R11: `axis` is a Forge built-in]: origin on the inner face at the bore, normal +w (out of
    the door, as door.latch) [R9], x = +u;
  - `link[0..1]`;
  - `padlock`: hole centre, normal along the fin's normal.
- **cam:**
  - `latch_axis` [R11]: origin on the front face at the axis, normal +w (out of the door) [R9], x = +u;
  - `link[0..1]`: placement [R1] (hw S-M4-0, thread 2.77);
  - `ball_seat[0..1]` [R11: `ball` is a Forge built-in].

Order note: the latch hardware (spring, balls, spacers) is catalogue. Every sheet part (RACK-301
with its studs, 312, 313, 321, 322) can be ordered now: nothing waits on a datasheet.

### 5.9 Fan tray RACK-401, fans, guards

Fan tray part frame: origin at the top face's centre. Axes follow rack X, Y, Z. Pose: origin
(0, 400, 893.38) [R2: was 894.98].
```
plate     x -150..150, y -150..150, z -1.88..0
flanges   on the x = +-150 edges, turned to -z, 20.00 outer
openings  116 dia at (+-70, +-70)
fan nuts  S-M4-0, flush on z = -1.88, body +z, at (+-70 +- 52.5, +-70 +- 52.5): 16
          anchors fscr[0..15], placement [R1] (hw S-M4-0, thread 2.77), normal +z; fan k's four are fscr[4k..4k+3]
hangers   8.6 at (+-130, +-140)                         anchors hang[0..3] in TRAY order (-130,-140), (130,-140), (-130,140), (130,140), normal -z, x = -x
          [R8, R9]: tray.hang[k] meets top-pan hang[k xor 1] (0-1, 1-0, 2-3, 3-2)
fans      anchors fan[0..3] at (-70,-70), (70,-70), (-70,70), (70,70) on z = -1.88, normal -z
```
Hanger stack, from the top pan down:
- FH-M8-25 stud (head flush in the top pan, outer face Z = 906);
- top pan Z 906 .. 902.98;
- spacer 8.2 ID x 14 OD x 8.0 [R6], Z 902.98 .. 894.98;
- washer ISO 7089-8, Z 894.98 .. 893.38 [R2: moved above the tray];
- tray Z 893.38 .. 891.50;
- ISO 10511 M8 nyloc nut, Z 891.50 .. 883.50.

The stud ends at Z 906 - 25 = 881.00, 2.5 (two pitches) past the nut.

Fan stack at each fan, from the top down:
- tray;
- fan 120 x 120 x 25.4, outlet face up against the tray (Z 891.50 .. 866.10);
- guard (wire, 5 deep, Z 866.10 .. 861.10);
- 4 screws ISO 7045 M4 x 40 from below [R2, R6].

Each screw goes up through the guard loop, both fan flanges and the tray, and into the S-M4-0. It
protrudes 7.72 above the tray top into the 9.6 gap, 1.88 short of the top pan, and runs 5.72 past the
nut [R2, R3]. The lowest points are the fan screws' pan heads (k 3.1), at Z 858.00, 4.95 above U18's
top (853.05); the guard is at 861.10 [R10]. A 4U dummy's body ends at 852.25 and its ears at 852.65.

### 5.10 Casters

Caster part frame: origin on the kingpin axis, on the plate's top face. **z** points up and **y**
runs along the plate's long side.
```
plate    x -33..33, y -45..45, z -3.0..0   (90 x 66; thickness not published, modelled 3.0)
holes    8.5 at (x +-22.5, y +-37.5)     (Blickle pattern 75 x 45)     anchor top: origin (0,0,0), normal +z
wheel    80 dia x 32, axle 40 behind the kingpin (offset), wheel bottom at z = -111.00
```
- **Poses:** origin on the base pan's `caster_*` anchor point, with z = +Z and y = +Y. The plate's
  long side runs front to back.
- **Rack positions:** the kingpins are at (+-185, 100) and (+-185, 700), and the plate top touches the
  base pan underside at Z = 0. [R9] The pan's `caster_*` anchors now point +z (into the pan), the same as
  the caster's `top`, with x = +x on both: `fastened(caster.top, pan.caster_*)`, no flip.
- **Bolts, 4 per caster:** ISO 4017 M8 x 20 from below. The head is under the plate. The shank goes
  through the plate (3.0) and the pan floor (3.02) to an ISO 7089-8 washer and an ISO 10511 M8 nyloc
  inside the pan. The nut top is at Z 12.62; the bolt ends at Z 17.00.
- **Clearance:** the nearest rail lower end is at x = 225. The nut corner reaches x = 215.

### 5.11 Rack-mounted equipment and cage nuts

Dummy server part frame: origin on the ear's back face (the face against the rail) at X = 0, at the
bottom boundary of the server's lowest U. Axes follow rack X, Y, Z. Pose: origin
(0, y_f, 52.95 + 44.45 (U - 1)).
```
nU             1, 2, 4 (RACK-D01, D02, D04)
ear            x -241.30..241.30, y -3.0..0, z 0.40..(nU * 44.45 - 0.40)   (482.6 wide; 0.79 vertical clearance)
ear holes      7.0 dia at x = +-232.55, z = 6.35 and nU * 44.45 - 6.35       anchors ear[0..3], origin on y = 0, normal +y
body           x -215..215, y 0..450, z 0.80..(nU * 44.45 - 0.80)
placements     D01 at U1, D02 at U10 (U10-U11), D04 at U15 (U15-U18)
```
- **Cage nut:** Rittal DK 2094.200 (M6, for 0.8 to 2.5 mm metal). It snaps into the rail's square
  hole from behind. Modelled envelope, from the Panduit CNWSM6-C drawing:
  - cage 11.81 (across, rack x) x 10.11 (vertical) x 8.03 (behind the rail face);
  - nut thread M6 x 1, centred on the hole;
  - anchor seat: origin on the rail's rear face, i.e. 1.88 behind `rail.eia[k]` along its normal [R4]:
    place it with `slider(cage.seat, rail.eia[k], along: 1.88)` (fastened has no offset).
- **Screw:** M6 x 16 rack screw with a plastic washer (Rittal DK 7094.110 type: cheese head; modelled as
  ISO 1207 [R6]).
  - Stack: plastic washer (1.0 modelled), ear 3.0, rail 1.88, then the cage nut thread.
  - Engagement: 4.78, the cage nut's whole thread, ending 2.09 past it [R3].

### 5.12 Fastener table

Every fastener and the parts it joins. Counts are for the whole rack.
```
joint                         fastener (+ washer, nut)                                    count  A anchor -> B anchor
post <-> pan                  FH-M6-20 stud in post; ISO 7089-6 + ISO 10511-M6 inside pan   16     post.stud_* -> pan.stud_*
carrier <-> pan               ISO 7380-1 M6 x 12 [R2] from outside the pan into carrier S-M6-0 12     pan.car_* -> carrier.pan[*]
rail <-> carrier              ISO 7380-1 M6 x 10 from inside the rail web into S-M6-0        16     rail.web_* -> carrier.rail[k]
side panel <-> post           ISO 7380-1 M5 x 10 into post SS-M5-0                         24     panel.scr[*] -> post.pnl_*
                              [R4] panel.scr is on the panel's outer face, post.pnl on the post's outer face:
                              1.52 apart along the screw; slider(.., along: 1.52), not fastened
panel joint                   ISO 7380-1 M5 x 12 + ISO 7089-5 + ISO 10511-M5 (nut below)   6      upper.jnt[*] -> lower.jnt[*]
hinge bracket <-> post        ISO 7380-1 M6 x 12 [R2] into post S-M6-0                     8      bracket.scr[0,1 | 2,3] -> post.hng_a[*] (R8)
hinge pin                     ISO 7379 8 x 12 M6 shoulder screw, spacer, ISO 7089-8 washer  4      bracket.pin -> door.pin_bot/pin_top
strike <-> post               ISO 7380-1 M6 x 12 [R2] into post S-M6-0                     4      strike.scr[*] -> post.stk_b[*]
latch pivot                   ISO 7379 8 x 16 M6 + ISO 10511-M6 on the base's back [R12]   2      base.pivot / base.pivot_back
latch links                   ISO 7380-1 M4 x 25 + spacer 6 x 15 into the cam's S-M4-0        4      handle.link[*] -> cam.link[*]
latch base <-> door           FH-M4-12 in door; ISO 7089-4 + ISO 10511-M4                  6      base.stud[k] -> door.lb_stud[k], slider 1.52 [R11]
strike <-> door (closed)      contact only: the prong tip on the door field, the cam on the seat
caster <-> base pan           ISO 4017 M8 x 20 + ISO 7089-8 + ISO 10511-M8                 16     caster.top -> pan.caster_*
fan tray hangers              FH-M8-25 in top pan; spacer 8 + ISO 7089-8 above the tray;
                              ISO 10511-M8 under the tray (2.50 past) [R2]                    4      tray.hang[k] -> pan.hang[k xor 1] [R8]
fans + guards                 ISO 7045 M4 x 40 [R2] from below into tray S-M4-0             16     guard/fan holes -> tray.fscr[*]
equipment                     M6 x 16 rack screw + plastic washer into cage nut             per U  ear[*] -> cage nut thd
```
Screw lengths and engagement (thread through the whole nut, at least two pitches past it where there
is room):
- [R1, R2, R3] The measured stacks are in `hw/README.md` (stack table, from `hw/tests/run_tests.py`). Every
  joint engages its own nut's whole thread and runs at least one pitch past it:
  - M6 x 10 rail to carrier: 2.16 past;
  - M6 x 12 bracket, strike and carrier to pan: 3.02 past;
  - M5 x 10 panel: 3.46 past;
  - M4 x 40 fan: 5.72 past;
  - M4 x 25 latch link: 4.24 past;
  - pins: 5.40 (hinge) and 3.90 (latch);
  - cage nut screw: 2.09 past.
- If a stack ends short of the full nut, E raises the screw to the next ISO length, not the nut.

## 6. Moving parts and joints

Joints follow Forge assembly semantics:
- A is the moving part, B the reference;
- a drive is a param, and `limit:` bounds it;
- relations (`gear`, `linear`) couple drives.

Every drive named here is an assembly param, so `fg serve` re-solves only what moved.

### 6.1 Doors

```
door_f = cylindrical(hinge_fb.pin, door_front.hinge, turn: door_f_turn, along: -9.92, turn_limit: (0deg, 105deg))   # [R4, R11]
door_r = cylindrical(hinge_rb.pin, door_rear.hinge,  turn: door_r_turn, along: -9.92, turn_limit: (0deg, 105deg))
# [R11] bracket first: both pin anchors point +Z, and door-first would make +turn swing the latch edge INTO
# the rack. Forge drives are joint params: write door_f.turn where this file says door_f_turn.
```
- **Axis:** the bottom pin. A positive turn opens the door: the latch edge swings out, away from the
  rack.
- **Top pin:** it lies on the same axis by geometry. The top pin is not a second joint, which would
  be `Redundant`; section 8 checks it instead with a coaxiality expect.
- **Pin screws:** each shoulder screw is `fastened` to its door (through `pin_bot` / `pin_top`), so
  it turns with the door. Its spacer and washer are `fastened` to the bracket.
- **Swing:** computed in plan with the door outline and the actual hinge axis.
  - The first contact is with the side panel's front edge, at 112deg. The limit is 105deg.
  - Over 0 to 105deg, the door's closest approach is 1.42 to the side panel inner face (at about 50deg).
  - Its largest Y on the hinge side is 31.03 (at about 50deg), against the bracket leg at 33.12 and
    the post at 35.00.
  - The free edge only moves outward.
- **Latch:** U (handle, 2 spacers, 2 link screws, cam, 2 balls) is one subassembly, `latch_u`. Its handle
  `axis` anchor and the door's `latch` anchor:
  ```
  latch_f = cylindrical(latch_u_front.latch_axis, door_front.latch, turn: latch_f_turn,
                        along: 8.17 + latch_lift, turn_limit: (0deg, 90deg), along_limit: (7.49, 8.17))   # [R11]
  ```
  - turn 0 = open, 90deg = closed; the arc slot ends are the hard stops there (declared fits).
  - [R11] along = the handle's inner face out from the door's outer face: 8.17 + lift, with lift 0 seated,
    -0.68 riding between detents, -0.40 closed
    (the strike's seat holds it). Set it as a param with the turn; Forge doesn't derive it from contact.
  - The pivot screw, spring and latch base are `fastened` to the door. The spring is modelled at its
    working height for each state (or as three variants: 8.17, 7.49, 7.77).
  - `expect` in section 8: closed and latched has no interference. Door at 0.5deg [R11] with the latch
    closed must show the cam hitting the strike's prong, which proves the latch holds.

### 6.2 Casters

The caster is its own subassembly, `casters/caster.fg`, with two variants: `caster_plain` and
`caster_lock` (with brake). Front corners use `caster_lock`, rear corners `caster_plain`. Its internal
parts are representative: Blickle publishes only the outside dimensions (section 7). The modelled
parts reproduce those exactly, and the internals are sized to fit inside them.
```
top plate        90 x 66 x 3.0 steel, formed upper raceway: groove on a 50.0 pitch circle, ball row below
                 the plate; kingpin hole 10.4; 4 holes 8.5 (section 5.10)
upper balls      24 x 4.763 (3/16") steel balls, pitch circle 50.0, centres at z = -5.38
swivel head      (the yoke's top plate) 2.5 steel, z -7.76 .. -10.26, raceways both faces
lower balls      20 x 4.763, pitch circle 36.0, centres at z = -12.64
lower race       2.5 steel washer, z -15.03 .. -17.53, 36.0 groove, kingpin hole 10.4
kingpin          rivet 10.0, head countersunk flush in the plate's top face (z = 0; nothing proud of the
                 plate, because the plate bears on the pan), lower head formed under the lower race (dia 16 x 3)
yoke (fork)      pressed 2.5 steel, legs inner faces at x = +-18.20, axle holes 8.4 at (y = -40.0, z = -71.0)
axle             ISO 4014 M8 x 55 hex bolt through the legs + ISO 10511 M8 nyloc outside the far leg
spacers          2 x steel tube 8.2 ID x 12.0 OD x 14.70, axle-to-bearing inner ring (clamped)
wheel            80.0 dia x 32.0 TPU tread on a nylon centre, hub 36.0 long (bosses 2.0 each side),
                 bore 22.0 for the bearing, centred on the axle (y = -40.0, z = -71.0); bottom z = -111.00
wheel bearing    608-2RS (assumed; Blickle says only "central ball bearing, 2RS", bore 8):
                 d 8, D 22, B 7, r 0.3; 7 balls 3.969 (5/32") on a 15.0 pitch circle (GMN gives 14.7 for
                 its precision 608; 15.0 = (d + D)/2 is used); pressed steel 2-piece ribbon cage (modelled,
                 not a catalogue drawing); 2 rubber seals 0.8 thick in the outer ring
brake (lock variant only, "stop-fix" total lock)
                 pedal: 2.0 steel lever, pivot pin 6.0 through the yoke legs at (y = +30, z = -40), pedal pad
                 out to y = +88 (inside the 90 swivel envelope); shoe on the lever bears on the tread at the
                 wheel's top-rear when engaged
                 lock pin: 5.0 steel pin in a guide in the yoke top, pushed up by the lever's cam into one of
                 24 notches round the lower race (15deg pitch): it locks the swivel
swivel envelope  radius 90 about the kingpin (wheel 81.6, pedal 88); every swivel angle stays clear of the
                 rack (section 8)
```
Joints inside a caster:
```
swivel    = revolute(yoke.kingpin, plate.kingpin, turn: swivel_turn)                 free (no limit)
wheel     = revolute(wheel.hub, yoke.axle, turn: roll)                                free
ring_in   = fastened(bearing_inner.bore, yoke.axle)                                  inner race on the axle
ring_out  = fastened(bearing_outer.od, wheel.hub)                                    outer race in the wheel
cage      = revolute(cage.axis, yoke.axle, turn: cage_turn)
cage_rel  = gear(wheel, cage, ratio: k_cage)       k_cage = (1 + Dw/dm)/2 = (1 + 3.969/15)/2 = 0.6323
for i in 0..7:  ball[i] = fastened(ball[i].c, cage.pocket[i])                          balls ride in the cage
up[i]     = revolute(upper_ball[i].c, plate.kingpin, ..) with gear(swivel, up[i], ratio: 0.5)   i in 0..24
lo[i]     = revolute(lower_ball[i].c, plate.kingpin, ..) with gear(swivel, lo[i], ratio: 0.5)   i in 0..20
pedal     = revolute(pedal.pivot, yoke.pedal_pin, turn: brake, limit: (0deg, 20deg))  lock variant
lockpin   = slider(lock_pin.axis, yoke.lock_guide, along: z, limit: (0, 4.0))
pin_rel   = linear(pedal, lockpin, ratio: 0.2 mm/deg)                                 4.0 at 20deg
```
- **Balls:**
  - Each ball's centre orbits at the ratio a rolling ball gives: 0.5 between a fixed and a turning
    flat race, 0.632 in the 608 with its inner ring fixed.
  - A ball's spin about its own centre is not modelled. It leaves a sphere's geometry unchanged, and
    it would need a second freedom per ball.
  - If Forge can't relate a revolute in the caster subassembly to the instance drive `roll`, the
    balls are `fastened` to the cage and only the cage turns. Report that as a Forge gap; don't hide it.
- **Brake:** at `brake = 20deg` the shoe touches the tread (an intended contact, declared `fit`) and the
  lock pin sits in a notch. At 0 both clear by at least 1.0.

### 6.3 Rails

Rails are `fastened` at a chosen position. They are not sliders: the 25 mm hole grid is the adjustment,
and a slider would let the solver put a rail between holes. Assembly params:
```
param rail_front = 100     # y_f, multiple of 25, 50..450
param rail_rear  = 725     # y_r, multiple of 25, 350..750, y_r - y_f >= 300
k_front = (rail_front + 12.5 - 62.5) / 25      -> rack y = 62.5 + 25 k: holes k_front, k_front + 1
k_rear  = (rail_rear  - 12.5 - 62.5) / 25      -> holes k_rear, k_rear - 1
```
The k above count from the front, which is the carrier's own index on BL and TR (a along +Y). On BR and TL
(a along -Y) the same rack hole is `rail[27 - k]`.
- **Moving a rail is one SetParam.** Section 8 runs the checks at the default and at both ends of the
  range: (50, 750) and (100, 400).
- If Forge refuses an anchor index computed from a param, write one `fastened` per allowed position
  under an `if` on the param, or make rail position a `Count` param. Report which one worked.

### 6.4 Everything else

Everything else is `fastened`:
- posts, pans, carriers, panels, brackets, strikes, fan tray, fans, guards;
- hardware and dummy servers.

Fits are declared for every intended contact: pin in bore, screw in nut, stud in hole, spacer
faces, cam in slot.

## 7. Purchased parts

David's rule: a supplier part number only where it was verified on the supplier's own page today
(2026-11-03). Otherwise this section gives the spec and a search query. "Verified" means the page was
fetched and its numbers read. It does not mean stock or price was checked.

### Verified part numbers
```
caster, total lock (front, x2)  Blickle LKPA-PATH 80KF-FI, ID 755344          https://www.blickle.com/product/lkpa-path-80kf-fi-755344
caster, swivel (rear, x2)       Blickle LKPA-PATH 80KF, ID 755346             https://www.blickle.com/product/lkpa-path-80kf-755346
                                80 x 32 wheel, TPU 94 Sh A on nylon 6; 120 kg at 3 km/h (300 kg static); total height
                                111; plate 90 x 66; holes 8.5 on 75 x 45 (also 61 x 51); offset 40; hub 36; axle bore 8;
                                "C" bearing = one central ball bearing, 2RS (https://www.blickle.com/guide/wheel-bearing-types);
                                double ball bearing in the swivel head, riveted kingpin; stop-fix brake locks wheel and
                                swivel (https://www.blickle.com/guide/castor-wheels-with-brakes)
cage nut M6 (per U used, x3)    Rittal DK 2094.200, for 0.8-2.5 mm metal      https://www.rittal.com/pdf-creator/variant/uk-en/2094200
rack screw M6 x 16              Rittal DK 7094.110 (cheese head, plastic washers included)
                                                                              https://www.rittal.com/pdf-creator/variant/uk-en/7094110
PEM hardware (pressed by SCS)   S-M4-0, SS-M5-0, S-M6-0, FH-M4-12, FH-M6-20, FH-M8-25   https://sendcutsend.com/guidelines/hardware/catalog/
fan (x4, datasheet verified)    Delta AFB1212M-F00: 120 x 120 x 25.4, holes 4.5 on 105, flanges 5.0
                                https://www.delta-fan.com/Download/Spec/AFB1212M-F00.pdf  (12 V; pick the voltage and
                                speed David wants; any 120 x 25 fan with the same frame fits)
```
US alternative caster, verified page: Colson 2 Series 3-1/2" total lock, 2.03256.445 BRK5, 250 lb:
- https://colsongroup.com/2-series-casters-top-plate-swivel-zinc-with-3-5-x-1-25-grey-on-grey-performa-rubber-flat-wheel-and-total-lock-brake-2-03256-445-brk5
- Its plate (3-5/8 x 2-1/2, 5/16 bolts, slotted 1-3/4 x 2-13/16..3-1/16) and height (4-11/16 = 119.1)
  differ, so choosing it changes the base pan's caster holes and the rack height (section 11).

### Spec only (no part number verified)
```
latch spring (x2)      compression spring, steel, ID >= 8.4, OD <= 13, free length ~12, solid <= 5.0, rate
                       ~8 N/mm (Lee Spring, Gutekunst and Sodemann publish such tables; pick one in range and
                       give its row to subsystem E)    search: "compression spring 8.5 ID 12 OD 12 long"
latch balls (x4)       steel ball 4.0, ISO 3290 grade G100 (bearing ball)            search: "4mm steel ball G100"
latch spacers (x4)     round spacer, steel, 6.0 OD x 4.3 ID x 15.0, unthreaded       search: "round spacer M4 6x15"
latch screws           ISO 7379 8 x 16 M6 (x2); ISO 7380-1 M4 x 25 (x4); ISO 7089-4 washer and ISO 10511-M4
                       nut (x6 each; dims to be read from the ISO tables by subsystem E)
finger guard 120 (x4)  [R10] CHECK THE BOUGHT GUARD'S LOOP HEIGHT BEFORE ORDERING SCREWS: rev B assumes the
                       loops sit 5.0 below the fan face (ISO 7045 M4 x 40, tip 1.88 under the top pan). If the
                       loops lie flat on the fan (2.0 wire clamp), M4 x 40 would end 1.12 INTO the top pan floor:
                       use M4 x 35 (tip 3.88 under the pan, 3.72 past the nut). Also check the fan's bore is <= 114
                       (the tray opening is 116).
                       wire guard, holes on 105, loops 4.6, height ~5, ring wire 1.6 / ribs 2.0
                       (a Qualtek 08141 drawing gives these: https://static.chipdip.ru/lib/774/DOC044774651.pdf,
                       a distributor copy, so not a verified supplier page)   search: "120mm fan finger guard 105mm"
fan screw (x16)        ISO 7045 M4 x 40 pan head, steel zinc [R2, R6]                         search: "ISO 7045 M4x40"
shoulder screw (x4)    ISO 7379, shoulder 8 x 12, thread M6 x 11 (min), head 13 x 5.5, socket 4
                       (https://www.fasteners.eu/standards/ISO/7379/)                         search: "ISO 7379 8x12 M6"
nylon spacer (x8)      8.2 ID x 14 OD x 8.0, PA66, one part for the 4 hinges and 4 hangers [R6]  search: "nylon spacer 8.2 x 14 x 8"
screws ISO 7380-1      M5 x 10 (24), M5 x 12 (6), M6 x 10 (16), M6 x 12 (24) [R2]: button head, hex socket, 8.8 zinc
                       M5 dk 9.5 k 2.75 s 3; M6 dk 10.5 k 3.3 s 4 (https://www.fasteners.eu/standards/ISO/7380/)
bolts ISO 4017         M8 x 20 (16): s 13, k 5.3 (https://www.fasteners.eu/standards/ISO/4017/)
bolts ISO 4014         M8 x 55 (4, caster axles; part of the caster as bought, modelled only)
nuts ISO 10511         M5 (6), M6 (16 + 2 latch pivots [R12]), M8 (20 + 4 in casters): M6 s 10 h 6.0, M8 s 13 h 8.0
                       (https://www.fasteners.eu/standards/ISO/10511/; the M5 row is to be read by subsystem E)
washers ISO 7089       M5 (6): 5.3 / 10 / 1.0; M6 (16): 6.4 / 12 / 1.6; M8 (24): 8.4 / 16 / 1.6
                       (https://www.fasteners.eu/standards/ISO/7089/)
bearings (modelled)    608-2RS: d 8, D 22, B 7, r 0.3, 7 x 3.969 balls (GMN datasheet
                       https://www.gmnbt.com/assets/pdf/Radial/608.pdf; MinebeaMitsumi
                       https://product.minebeamitsumi.com/en/product/category/bearing/miniature_small/parts/608.html)
                       6201 for reference: d 12, D 32, B 10, r 0.6, 7 x 5.953, pressed steel cage
                       (https://www.nsk.com/engineering/6201-dgbb-sr.html)
cable brushes          optional: brush strip for the 250 x 60 cable cut-outs              search: "cable brush strip 60mm"
```
EIA-310 sources for the rack geometry:
- 465.1 (18.312") hole spacing, +-1.6 tolerance:
  - https://help.racksolutions.com/hc/en-us/articles/10431608060692-What-is-EIA-310
  - https://www.se.com/se/sv/faqs/FA158600
- Hole groups 15.875 / 15.875 / 12.7 within 44.45 per U:
  - https://www.ibm.com/docs/POWER9/p9had/p9had_oemrack.htm
  - https://digital-loggers.com/rackspecs.html
- 0.375" (9.5) square holes: the cage nut makers' cut-out size, 9.53 x 9.53 (RS Pro 523-137:
  https://nonprod.docs.rs-online.com/1c63/0900766b815835f2.pdf).
- The 450 opening between rails is the commonly quoted EIA minimum. I didn't find the standard's own
  text for it (it is sold by IHS), so the design meets it exactly and doesn't depend on it otherwise.

## 8. Verification plan

The top assembly `asm/rack.fg` must pass every line below. Part files must pass their own lines first.
Lines are written in Forge assembly syntax. Where Forge can't yet express one, it is marked [gap].

### Fit rules (so intended contacts are not interferences)
```
m4 = fit_rule(external_thread, internal_thread, size: 4, min_engagement: 2.77)     # [R1, R5]
m5 = fit_rule(external_thread, internal_thread, size: 5, min_engagement: 2.77)
m6 = fit_rule(external_thread, internal_thread, size: 6, min_engagement: 4.78)
m8 = fit_rule(external_thread, internal_thread, size: 8, min_engagement: 8.0)
# each is the shortest whole nut of that size (S-M4-0 2.77, SS-M5-0 2.77, cage nut 4.78, nyloc 8.0);
# a rule matches by size only, so hw/tests prove each joint fills ITS OWN nut (hw/README stack table)
pin = fit_rule(shaft, bore, size: 8)
```
- **About "every screw engaged at least one diameter":**
  - For nyloc and cage nuts it holds as written: the nut is at least 1d tall, and every screw passes
    through it.
  - A PEM nut in 1.5 to 3 mm sheet is shorter than 1d by design. Its rated strength assumes the screw
    fills its whole thread, so each rule's `min_engagement` is the shortest full-thread depth of that
    size's nuts (sheet + T).
  - Each screw is also checked to protrude past its nut by at least one pitch:
    `expect` per joint family [gap: no expect for protrusion; checked in the hardware library's stack
    table instead, section 9E].

### Interference and contact
```
expect interferences = 0                                     every overlap is declared as a fit or is an error
expect mobility = <the drives only: 2 doors + 2 latches (turn, lift) + 4 swivels + 4 wheels + 2 brakes>
expect clearance(door_front, panel_ll) >= 1.0 at door_f_turn = 50deg
expect clearance(door_front, panel_ll) >= 1.0 at door_f_turn = 105deg
expect clearance(door_front, hinge_ft) >= 1.0 at door_f_turn = 50deg       (bracket legs behind the door)
expect clearance(door_front, post_fl) >= 2.0 at door_f_turn = 50deg
expect interference(door_front, panel_ll) > 0 at door_f_turn = 115deg       (the limit is real: contact past 112deg)
expect clearance(latch_u_front, strike_front) >= 0.2 at latch_f_turn = 0deg, latch_lift = 0     (open: clear)
expect clearance(latch_u_front, strike_front) >= 0.2 at latch_f_turn = 45deg, latch_lift = -0.68 (swinging in)
expect clearance(latch_u_front, strike_front) >= 0.2 at latch_f_turn = 80deg, latch_lift = -0.68
  (at 90deg, lift -0.40 the cam touches the seat and the prong tip touches the door: declared fits)
expect interference(latch_u_front, strike_front) > 0 at door_f_turn = 0.5deg, latch_f_turn = 90deg, latch_lift = -0.40   [R11: at 10deg the cam is
                                                                             already ~100 out, past the 5.71 prong]
                                                                             (a closed latch stops the door)
expect clearance(latch_u_front, door_front) >= 0.5 at latch_f_turn = 0, 30, 60, 90deg   (U clears the door except
                                                                             at the declared ball/post fits)
expect clearance(latch_handle_front, latch_base_front) >= 3.0 at latch_f_turn = 0, 45, 90deg  (handle vs padlock tab)
padlock: at latch_f_turn = 90deg the handle's and the tab's padlock holes are coaxial within 0.3 (x(), y())
the same five for the rear door
expect distance(pin_top_axis, pin_bot_axis) ... coaxial within 0.05          [gap: no axis-to-axis expect;
                                                                              use x(), y() of both pin anchors]
```
Casters, at swivel_turn = 0, 45, 90, ..., 315deg for each caster, brake 0 and 20deg:
```
expect clearance(caster_fl, base_pan) >= 0.5        (the plate is fastened; the swivelling parts stay below it)
expect clearance(caster_fl, panel_ll) >= 5.0
expect clearance(caster_fl, caster_rl) >= 50
expect z(caster_fl.wheel_bottom) = -111.00 +- 0.05
```

### EIA alignment, front to rear and side to side
```
expect y(rail_lr.eia[53]) - y(rail_lf.eia[0]) = rail_rear - rail_front +- 0.01
expect z(rail_lf.eia[k]) = z(rail_rf.eia[53 - k]) +- 0.01 for k in {0, 26, 53}   (the flipped right rail)
expect z(rail_lf.eia[k]) = z(rail_lr.eia[53 - k]) +- 0.01 for k in {0, 26, 53}
expect x(rail_rf.eia[0]) - x(rail_lf.eia[0]) = 465.10 +- 0.05
expect z(rail_lf.eia[0]) = 59.30 +- 0.01         (U1 bottom 52.95 + 6.35)
```
Each line runs at the defaults and at (rail_front, rail_rear) = (50, 750) and (100, 400).

### Equipment
```
dummy 1U at U1, 2U at U10, 4U at U15, each with 4 cage nuts and 4 screws (rails as above)
expect interferences = 0                                     (dummies vs rails, cage nuts, carriers, doors, fans)
expect clearance(dummy_4u, fan_guard[*]) >= 5.0              (guards at 861.10)
expect clearance(dummy_4u, fan_screw[*]) >= 5.0              [R10] (screw heads at 858.00: 5.75 to the body, 5.35 to the ears)
expect clearance(dummy_*, door_front) >= 50                  (room for handles and cables)
```

### Fans
```
expect clearance(fan[*], rail_*) >= 20
expect clearance(fan_tray, carrier_t*) >= 5
expect clearance(fan_screw[*], top_pan) >= 1.5              (screw tips in the 8.0 gap)
grille / opening concentricity: x(), y() of top_pan grille centres = fan[*] centres +- 0.1
```

### Per-part DFM (each part file, `fg run --json`)
- `shop: scs` on every sheet.
- Violations zero, except the accepted ones below.
  - Accepted: RACK-141's return cut-back cutouts. A cut across a bend is reported as HoleNearBend at 0
    (Forge bug N4). It goes away with flange end offsets (section 10).
- Flat checks by script (`run.py` reads the DXF; these limits are not in Forge's shop):
  - flat bbox <= 762.0 x 914.4 (powder coat);
  - bend lines <= 1117.6;
  - box flanges with hardware <= 76.2;
  - every bend <= 130deg.
- Flat area x t equals the flat body volume (to 1e-6), as in complex-parts.
- `expect volume` per part from drawing math (blank less bend deductions less holes), not copied from
  a run.
- Bodies valid.

## 9. Subsystem split for modelling

Directory layout under `rack18u/`:
- `hw/` hardware library and shop;
- `frame/`;
- `panels/`;
- `casters/`;
- `fans/`;
- `asm/`.

Each subsystem:
- writes its part files with the part frames and anchors of section 5, exactly as named;
- writes one subassembly test file `<sub>/test.fg` that places its parts by the section 5 poses
  (rack coordinates) and passes its own expects;
- doesn't edit another subsystem's files. A wrong number in this SPEC goes back to the design lead.

**E. Hardware library (`hw/`)**: build first, because everyone needs it.
- `shop.fg` text (section 3).
- ISO screws as parts with `head` (under-head plane) and `thd` (`external_thread`) anchors:
  - 7380-1 M5 x 10, M5 x 12, M6 x 10, M6 x 12;
  - 4017 M8 x 20;
  - 4014 M8 x 55;
  - 7379 8 x 12;
  - ISO 7045 M4 x 40 pan;
  - rack screw M6 x 16.
- Nuts 10511 M5, M6 and M8, with an `internal_thread` anchor.
- Washers 7089 M5, M6 and M8.
- Spacers.
- PEM bodies S-M4-0, SS-M5-0, S-M6-0, FH-M6-20 and FH-M8-25, as solids with the bulletin dims and an
  anchor on the flush face. These are separate parts placed on the sheet parts' PEM anchors, because
  `pem()` cuts only the hole.
- Cage nut.
- The latch's catalogue parts: spring (modelled at each working height), 4.0 balls, 6 x 4.3 x 15 spacers,
  ISO 7379 8 x 16, FH-M4-12 with ISO 10511-M4.
- A stack table with screw lengths and engagement per section 5.12, checked by running each joint as a
  two-part test assembly.

Depends on: nothing.

**A. Frame and rails (`frame/`)**:
- RACK-101 post, RACK-111 carrier, RACK-121 base pan, RACK-131 top pan, RACK-201 rail;
- the anchors of sections 5.1 to 5.4;
- `frame/test.fg`: 4 posts, 2 pans, 4 carriers, 4 rails, studs and screws, at rail positions (100, 725),
  (50, 750) and (100, 400).

Depends on: E.

**B. Panels and doors (`panels/`)**:
- RACK-141 side half-panel, RACK-301 door, RACK-311 hinge bracket, RACK-312 strike, RACK-313 cam,
  RACK-321 latch handle, RACK-322 latch base;
- the door subassembly: door, two brackets, two pin stacks, latch base, pivot, spring, the U subassembly;
- `panels/test.fg`: a stand-in post (a plain L with the section 5.1 anchors, made in B's test folder
  from SPEC numbers, not A's file), the door subassembly, the latch and the side panels, plus the swing
  checks of section 8.

Depends on: E, and SPEC section 5.1 for the post's anchors.

**C. Casters (`casters/`)**:
- the caster subassembly in two variants, with every part of section 6.2 and its joints;
- the bearing as its own subassembly, `casters/bearing_608.fg`: rings, 7 balls, cage halves, seals;
- `casters/test.fg`: the swivel sweep, the brake states, the outside dims against Blickle's
  (111 / 80 x 32 / 90 x 66 / 40 offset).

Depends on: E.

**D. Fan tray (`fans/`)**:
- RACK-401, the fan (an envelope with corner flanges, holes and a hub, as an `out` anchor part) and the
  guard (wire as swept circles: ring 1.6, ribs 2.0, loops 4.6);
- the fan subassembly: tray, 4 fans, 4 guards, 16 screws;
- `fans/test.fg` with a stand-in top pan (section 5.2 anchors only).

Depends on: E.

**F. Top assembly (`asm/`)**:
- `asm/rack.fg`: all subassemblies placed by section 5, the dummies (RACK-D01, D02, D04), every
  expect of section 8, renders (front, iso, door at 90deg) and a BOM cross-checked with section 4 and
  section 7 counts;
- the order package: STEP per bent part (or DXF plus bend table, section 10), DXF per flat part, a
  hardware list per part for SendCutSend's hardware step, and the purchased BOM.

Depends on: A to E.

Order of work: E, then A, B, C and D in parallel, then F.

## 10. Forge capabilities and gaps

Forge here means the forge-lang worktree, at 779b8ac4 (`fg reference`, and the 26 sheet probes in
`forgecad-gate/sheet-metal/complex-parts/probes/results.json`). The design avoids every gap that has no
workaround. Each gap below is either avoided (with how) or flagged as a kernel fix.
```
 #  need                                    Forge today                        here
 1  flange end offsets (a return shorter    UnknownArg (probe partial_flange)  RACK-141's joint return runs y 90..710 only.
    than its edge)                                                             Workaround: full-length flange, then a
                                                                               cutout through the return and its bend at
                                                                               each end (as complex-parts' rack tray does).
                                                                               Costs a small notch in the field and 2 false
                                                                               HoleNearBend (N4). KERNEL FIX: flange(..,
                                                                               offsets: (start, end)).
 2  patterns of cutouts (54 EIA holes x 4,  pattern(of: cutout) is WrongType  Loops of small sketches: one sketch per U
    154 door slots x 2, 44 grille slots,                                       (3 holes), per door row (4 slots), per fan
    16 fan PEMs)                                                               (11 slots). pem() in a loop. Works.
                                                                               KERNEL FIX wanted (pattern of sheet features)
                                                                               for edit speed, not needed for correctness.
 3  many-hole sketch speed                  super-linear (100 circles 2.7 s,   Kept every sketch <= 11 entities. Expected
                                            bug 1); warm re-run re-evaluates   run: rail ~1 s, door ~2 s (40 sketches).
                                            sketches (bug 2)                   KERNEL FIX: bugs 1 and 2; they dominate
                                                                               every part here.
 4  hems (door and panel edge safety)       180deg is BendAngle; open hem      Avoided: doors have 25 returns all round;
                                            WallTooShort                       panels' front/rear edges are plain cut edges,
                                                                               deburred (SendCutSend deburring). Nice to have.
 5  closed corners (sealed box corners)     CornerUnbuilt                      Avoided: pans and doors use automatic
                                                                               circular reliefs; the rack isn't sealed.
 6  two DOWN flanges on adjacent edges      always FoldedClash (bug N1)        Avoided: pans and doors are modelled with
                                                                               flanges UP in their own frame; the top pan is
                                                                               the base pan turned over by its pose.
 7  flange off a flange's end edge (ears,   EdgeMoveBlocked (bug 5)            Avoided: no part needs one (no rack ears on
    tabs)                                                                      the frame; brackets are plain L's).
 8  shop limits beyond hole rules: max      not in stock()/shop()              Checked by script on the DXF (section 8).
    flat size (powder 762 x 914.4), max                                        KERNEL FIX: stock(max_flat: (w, l),
    bend length, max box flange with                                           max_bend_length:, max_angle:) with typed
    hardware, max angle 130                                                    violations, like the hole rules.
 9  STEP export of folded sheet parts       forge-step has a writer, but fg    SendCutSend takes STEP for bent parts. Until
                                            run has only --dxf                 then: DXF flat + bend lines and enter each bend
                                                                               in their app. KERNEL/CLI FIX: fg run --step.
10  DXF bend notes (direction, angle,       bend lines only on layer BEND      Same as 9; the bend table goes in each part's
    radius per line)                                                           README. CLI FIX: bend notes, or STEP.
11  PEM bodies in the model                 pem() cuts the mounting hole only  The hardware library models the bodies as
                                                                               parts; the assembly places them on the PEM
                                                                               anchors. KERNEL FIX (small): pem() emits its
                                                                               body as a second body of the part, so part
                                                                               files are complete alone.
12  shared shop data across files           no import / include                The shop text is copied verbatim into each
                                                                               part (one source in hw/shop.fg, diffed in
                                                                               run.py). FORGE FIX: `use "hw/shop.fg"`.
13  "cut across a bend" kind                HoleNearBend at 0 (bug N4)         Only #1's workaround needs one. FIX N4.
14  anchor index from a param (rail k)      not known                          Section 6.3 gives the fallback.
15  rolling elements: ball spin + orbit     one freedom per joint; relations   Orbit by gear relation; spin dropped
                                            exist (gear)                       (invisible on a sphere). Gap only if relations
                                                                               can't cross a subassembly boundary.
16  expect for screw protrusion, axis       expect has x/y/z, clearance,      Done in the hardware stack table and by x()/y()
    coaxiality, flat bbox                   interference, distance            of anchors. FORGE FIX: expect bbox (complex-
                                                                               parts gap 14), coaxial(a, b).
17  assembly size: ~750 instances (each     the 230-instance test is the      Risk: the overlap pass may be slow. Mitigate by
    caster ~70 parts, 272 PEM bodies,       largest measured                   suppressing ball instances in the top
    ~140 screws)                                                               assembly (casters checked in their own test),
                                                                               and report timings.
18  helical threads on 400 fasteners        thread() models exact flanks      Don't model threads; anchors carry the thread
                                                                               (kind, size, depth) for the fit rules.
```
What the design needs from Forge before parts can be ordered:
- #9 or #10 (STEP export, or DXF bend notes), for the bent parts' order files;
- #8 is wanted, so the order is checked by Forge and not only by script.

Everything else has a working route today.

## 11. Open decisions for David

1. **Depth: 800 or 1000?** 800 takes rail depths 300 to 700. That's enough for short-depth servers,
   switches and most NAS/1U boxes with 610 mm minimum slide rails. Full-depth servers need about 740
   to 900 between rails. 1000 deep means 3-piece side panels (powder coat limit), pans split in two, and
   about 15% more steel. **Recommendation: 800** unless a full-depth server is planned.
2. **Caster: Blickle LKPA-PATH 80KF / -FI (metric, verified, 120 kg each) or Colson 2 Series 3-1/2"
   (US stock, 250 lb, 119 tall)?** **Recommendation: Blickle**: lower, metric bolts, and the total lock
   holds both swivel and wheel. Colson changes the base pan holes and the height (+8).
3. **Finish: powder coat every part (all are inside 30" x 36") or bare steel painted at home?**
   **Recommendation: powder coat, black textured**, ordered with the parts. The PEMs go in before
   coating.
4. ~~Cam lock~~ resolved (section 12): our own latch, section 5.8. Its spring is the one catalogue part
   still to be picked from a maker's table.

## 12. David's decisions (2026-11-03)
1. Depth 800 (as specified).
2. Casters: Blickle LKPA-PATH 80KF, front total-lock -FI (as specified).
3. Finish: black textured powder coat on every fabricated part.
4. Cam lock: "Make one" — no purchased lock. Design our own quarter-turn latch from orderable pieces (see 5.8 rev).
