# Subsystem D: fan tray (SPEC 5.9 rev B)

`python3 fans/run.py` runs everything below through `fbuild`: each part (lint-free, no warnings or
violations, every expect), the tray's flat checks, the subassembly and both tests. Last run: all ok.

## Parts

| file | what | volume (expect from drawing math) |
|---|---|---|
| `rack401_tray.fg` | RACK-401, t2, 300 x 300 plate, 2 flanges 20 down on x = +-150, 4 openings 116, 4 hanger holes 8.6, 16 `pem("nut", "M4")` | 107807.6, exact to the formula (blank less holes plus the bends' (1/2 - K) t arc) |
| `fan_120x25.fg` | 120 x 120 x 25.4 fan envelope: two 5.0 corner flanges with 4.5 holes on 105, venturi tube, hub 46, 4 struts | 99125.3, +-0.5% |
| `guard_120.fg` | wire finger guard, every wire a swept circle: 6 rings 1.6, 2 diagonal ribs 2.0 bent down into 4 loops (2.0 wire, 4.6 ID) | 3802.8, +-3% (welds) |
| `fan_assy.fg` | the fan subassembly: tray, 16 S-M4-0 bodies (hw), 4 fans, 4 guards, 16 ISO 7045 M4 x 40 (hw) | 41 instances |
| `test.fg` | the subassembly hung from a stand-in top pan on 4 hanger stacks (hw FH-M8-25, spacer, washer, nyloc) | 83 expects |
| `test_air.fg` | the air through the roof: how much pan metal stands in each fan's air column | 6 expects |
| `standin/top_pan_standin.fg` | STAND-IN RACK-131 from SPEC 5.2 only (floor, 4 flanges up, hanger studs, 4 grilles, cable cut-out) | rough, +-2% |
| `standin/air_column.fg` | reference: the 114 bore's air from the fan outlet up to the pan's outer face | |

Frames and anchors (rack values at the SPEC poses):
- Tray: frame per 5.9, pose (0, 400, 893.38). `fscr[0..15]` placement anchors on z = -1.88, +z (R1);
  `fan[0..3]` on z = -1.88, -z; `hang[0..3]` on the top face z = 0, -z, x along -x (see SPEC issue 2).
- Fan: origin at the outlet face centre, +z along the airflow; `out` (outlet, +z), `inlet` (inlet face,
  +z), `fhole[0..3]` (bore kind, 4.5). Joined `fastened(fan.out, tray.fan[k], flip: true)`.
- Guard: origin on the face against the fan, +z towards the fan, body z -5..0; `fan` (z = 0) and
  `loop[0..3]` (screw seat z = -5, +z). Joined `fastened(guard.fan, fan.inlet)`; screws
  `fastened(screw.head, guard.loop[j])`.
- Fan k's screws and nuts are `screw[k][j]` and `nut[4k + j]` (tray `fscr[4k + j]`).

Fan dims: Delta's AFB1212M-F00 drawing (spec page 6) gives 120.0, 25.4, 8 x 4.5 on 105.0 and 5.0
flanges. Bore, hub and corner radius are not dimensioned; read off the drawing to about +-1: bore 114,
hub 46, corner 4.5. The guard's ring count and spacing (6 rings, 8.4 gaps) are mine; SPEC gives only the
wire sizes and loops.

## Tray DFM (SPEC 8)

- `shop: scs` (text copied from `hw/shop.fg` rev B), 0 violations; PEM rows checked by Forge.
- Flat (`rack401_tray.dxf`, `renders/rack401_tray_flat.png`): 300.00 x 333.47, inside 762 x 914.4.
- 2 bends, 300 long (<= 1117.6), 90deg (<= 130). Flanges 20, with no hardware (<= 76.2).
- DXF area x t = flat body volume, rel 6.7e-8.

Bend table for SendCutSend (SPEC 10 #9/#10, until STEP export). Both bends: 90deg, inside radius 1.60,
K 0.40, allowance 3.695, setback 3.48, 300 long. Both turn down, away from the PEM heads: the nut bodies
are on the top face (+z), and the flanges hang below.

## Test results

`test.fg` (rack coordinates; the pan stand-in is turned 180deg about Y as SPEC 5.2 says):
- 0 interferences, mobility 0, 83/83 expects.
- The thread rules are proven to matter: without them there are 20 interferences (16 M4 + 4 M8).
- Engagement: M4 2.77 (S-M4-0's whole thread, rule 2.77) and M8 8.0 (nyloc, rule 8.0), all 20 pairs.
- Heights:
  - tray 893.38;
  - washer 894.98;
  - nyloc 891.50;
  - fan outlet 891.50;
  - guard 866.10 .. 861.10;
  - screw head underside 861.10, tip 901.10.
- Hole alignment:
  - The tray is joined at one hanger only (R4 slider, 12.62). The other three tray `hang` anchors
    measure 12.62 straight below their pan anchors.
  - The 16 fan holes coincide with their `fscr` anchors (distance <= 0.01).
  - The 16 guard loops are 30.40 below them.
- Fan clearance: all 16 screw tips 1.88 under the top pan (SPEC 8: >= 1.5).
- Airflow, tray: the tray clears all 4 air columns by 0.99 (116 opening over the 114 bore; 1.00 by
  drawing, 0.99 measured on the mesh).
- Airflow, grilles: the 4 grille centres sit on the fan centres (x, y within 0.1, SPEC 8), read by
  anchor x()/y() on both sides.
- Interference covers the reference air columns too, so nothing else stands in the airflow: no screw,
  nut, washer, stud or tray metal.

`test_air.fg`:
- Exactly 4 interferences: the air columns with the pan floor. Each is 13491.587 mm3, equal to
  3.02 x (pi 57^2 - 11 slots) to 1e-12.
- So the only metal over a fan is the grille's bars, as SPEC 5.2 draws them. The grille is open over
  5739.6 of the bore's 10207.0 mm2 (56%; 58% of the 112 circle).

Not checked here, because they need parts from other subsystems (F's top assembly). By hand from SPEC
numbers:
- fan to rail: fans reach x +-130, rails' inner edge x +-225, so 95 (>= 20);
- tray to top carriers: 150 against 250.00 (carrier leg inner face), so 100 (>= 5);
- guards to U18: 8.05. See SPEC issue 4 for the screw heads.

Renders: `renders/`: each part (iso), the tray flat, `fan_assy.png` and `fan_assy_bottom.png`, `test.png` and `test_front.png`, `test_air.png`.

## SPEC issues (for the design lead)

1. **Hanger and grille index pairing.** The top pan is turned 180deg about Y, so its `hang[k]` and
   grille k land at rack x opposite to the tray's `hang[k]` and `fan[k]`. The pairs that meet are
   pan.hang[0] with tray.hang[1], [1]-[0], [2]-[3] and [3]-[2], and the same for grille[k] with fan.
   SPEC 5.12 (`pan.hang[*] -> tray.hang[*]`) and 5.9 ("same order as the top pan") pair the wrong holes.
   I used the pairs that meet, and the test proves them. Suggest SPEC give the tray's order in rack
   terms, or write the pairing out.
2. **Anchor x axes.** `fastened` and `slider` align the whole frame, not only origin and normal. A
   tray `hang` anchor with x along +x, joined to the turned pan's (x = rack -X), spins the tray 180deg
   about Z. That happened on the first run: everything was 260 off. I gave tray.hang x along -x. SPEC 5's
   "same origin and same normal" should also fix x for every pair that a joint places (this hits A, B and
   C too: pans, posts, carriers are turned).
3. **Guard clamp depth decides the screw.**
   - Rev B puts the M4 x 40 head 5.0 below the fan face, as if the guard is 5.0 thick at its loops.
   - On a flat wire guard (the Qualtek 08141 type SPEC cites) the loops usually lie in the ring's plane
     against the fan. Then the clamp is the 2.0 loop wire, and the M4 x 40 tip would be at 904.10:
     1.12 into the top pan floor (902.98).
   - I modelled the guard to rev B: ribs bent down 1.6 into loops in the outer plane, so the head seats
     5.0 down.
   - Before ordering, confirm the bought guard's loop height. If its loops sit on the fan, use M4 x 35:
     tip 899.10, 3.88 under the pan, 3.72 past the nut.
4. **Lowest point.** SPEC 5.9 says the lowest point is the guard at 861.10 (8.05 above U18). The M4 pan
   heads (k 3.1) hang below it, to 858.00: 4.95 above U18's top.
   - It's still clear of a dummy: the 4U body ends at 852.25 and its ears at 852.65, so 5.75 and 5.35.
   - But F's `clearance(dummy_4u, fan_guard) >= 5` doesn't cover the screws. A check against the screws
     at >= 5 would pass only by 0.35 on the ears.
5. **Fan bore** is not on Delta's drawing. The tray's 116 opening clears the 114 I read off it by 1.0.
   A fan with a bore over 116 would be shaded by the tray: check the bought fan.
6. **R1 depths in 5.9:** tray `fscr` anchors are placement-only now. Nothing else in 5.9 conflicted.

## Forge gaps (repros in `probes/`)

- `probes/sub_clearance.fg`: `expect clearance(SUB, part)` with a subassembly instance reads `inf`,
  and a `>=` expect passes silently. The same pair through the subassembly's own instance
  (`s.pin`) measures 8. test.fg uses per-instance lines (`fans.screw[k][j]`) for that reason.
  Side note: a subassembly whose own part is `ground: true` has mobility 6 in its parent until a joint
  holds it.
- `probes/fit_interference.fg`: `expect interference(a, b)` reads 0 for a pair a `fit()` declares. So
  the allowed overlap (air through the grille) can't be measured in the file that declares the fit.
  Worked around with `test_air.fg` (no fit, `expect interferences = 4`).
- Small ones, no repro needed:
  - `i / 4` on a loop index is a Ratio, so no integer division for an index. The 16 nut positions are
    written out as lists.
  - `hole`, `rib` and `mid` are built-in names, so features can't be called that (ShadowsBuiltin).
