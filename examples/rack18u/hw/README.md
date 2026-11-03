# hw: the RACK18U hardware library (subsystem E)

Every purchased fastener and every PEM part SendCutSend presses in, as Forge parts with the anchors the
assemblies fasten and the fit rules check. SPEC sections 3, 5.8, 5.12, 7, 8 and 9E.

## How to use it

- **One file per size.** `part(FILE, ..)` takes no param overrides, so each size (and each PEM nut per
  sheet thickness) is its own file. `gen.py` holds every number and writes them all; edit it and rerun
  `python3 hw/gen.py`, never the .fg files by hand.
- **Frames.** Every part's axis is its z axis, and +z is the direction the screw advances (from head to
  tip; for nuts, washers and spacers, the direction a screw passes through them). Every anchor's
  normal is +z, so a stack is a chain of `fastened(next.<anchor>, prev.<anchor>)`.
- **Threads are cosmetic** (SPEC gap 18): external threads are modelled at the major diameter, internal
  ones at the ISO 965 minor diameter D1 = d - 1.0825 P. A screw in its nut therefore overlaps it, and
  only a fit rule makes that intended. Without the rules every threaded pair is an Interference (the
  tests check this).
- **Placing:**
  - screw: `fastened(scr.head, <clamped part's face where the head sits>)`;
  - PEM nut: `fastened(nut.flush, sheet.<PEM anchor>)`. Use the file for that sheet's thickness: the
    nut's shank (A) is shorter than the sheet, so its thread starts `t - A` in from the flush face;
  - FH stud: `fastened(stud.flush, sheet.<stud anchor>)`. The head is pressed into the sheet, so
    declare it, for example `fit_rule("../hw/pem_fh_m6_20.fg", "post.fg")`;
  - nyloc nut: `fastened(nut.seat, washer.b)`;
  - cage nut: `cage.seat` is on the rail's REAR face. The rail's `eia[k]` anchors are on its front
    face, so place it with `slider(cage.seat, rail.eia[k], along: 1.88)` (fastened has no offset).
- **Fit rules:** `size:` is a number (6), not "M6". The rules in the tests set `min_engagement` to the
  nut's whole thread, which is what SPEC 5.12 asks for ("thread through the whole nut").

## Running the checks

```
python3 hw/gen.py && python3 hw/tests/gen_tests.py && python3 hw/tests/run_tests.py
```
`run_tests.py` runs every part (must build, pass its expects, no lints), then each stack four ways: as
written (0 interferences, no ShallowEngagement), with its fit rules removed (each threaded pair must
interfere), with `min_engagement` 99 (so Forge reports the engagement it measured), and with the SPEC 8
minima. Renders of every part (iso) and stack (front) are in `renders/`.

## Stack table (SPEC 5.12), from `run_tests.py`

"Engaged" is Forge's measurement. "Nut thread" is the nut's whole thread (the rule's minimum), and "past
the nut end" is how far the tip goes beyond the nut's far face. All 14 stacks pass: whole nut engaged, at
least one pitch past it, zero unintended interference, mobility 0, and each part's z where the SPEC puts it.

| stack | fastener | nut | engaged (Forge) | nut thread | past the nut end | 1 pitch | stack ok | without rules | SPEC 8 minimum |
|---|---|---|---|---|---|---|---|---|---|
| bracket_post | iso7380_m6x12 | pem_s_m6_0_t3 | 5.23 | 5.23 | 3.02 | 1.0 | yes | 1 interference(s) | passes |
| carrier_pan | iso7380_m6x12 | pem_s_m6_0_t2 | 5.23 | 5.23 | 3.02 | 1.0 | yes | 1 interference(s) | passes |
| caster | iso4017_m8x20 | iso10511_m8 | 8.00 | 8.00 | 4.38 | 1.25 | yes | 1 interference(s) | passes |
| fan_screw | iso7045_m4x40 | pem_s_m4_0_t2 | 2.77 | 2.77 | 5.72 | 0.7 | yes | 1 interference(s) | passes |
| hanger | pem_fh_m8_25 | iso10511_m8 | 8.00 | 8.00 | 2.50 | 1.25 | yes | 2 interference(s) | passes |
| hinge_pin | iso7379_8x12_m6 | pem_s_m6_0_t1 | 5.23 | 5.23 | 5.40 | 1.0 | yes | 1 interference(s) | passes |
| latch_base_stud | pem_fh_m4_12 | iso10511_m4 | 5.00 | 5.00 | 1.66 | 0.7 | yes | 2 interference(s) | passes |
| latch_link | iso7380_m4x25 | pem_s_m4_0_t2 | 2.77 | 2.77 | 4.24 | 0.7 | yes | 1 interference(s) | passes |
| latch_pivot | iso7379_8x16_m6 | pem_s_m6_0_t3 | 5.23 | 5.23 | 3.90 | 1.0 | yes | 1 interference(s) | passes |
| panel_joint | iso7380_m5x12 | iso10511_m5 | 5.00 | 5.00 | 2.96 | 0.8 | yes | 1 interference(s) | passes |
| panel_post | iso7380_m5x10 | pem_ss_m5_0_t3 | 2.77 | 2.77 | 3.46 | 0.8 | yes | 1 interference(s) | passes |
| post_pan | pem_fh_m6_20 | iso10511_m6 | 6.00 | 6.00 | 6.36 | 1.0 | yes | 2 interference(s) | passes |
| rack_screw | rack_screw_m6x16 | cage_nut_m6 | 4.78 | 4.78 | 2.09 | 1.0 | yes | 1 interference(s) | passes |
| rail_carrier | iso7380_m6x10 | pem_s_m6_0_t2 | 5.23 | 5.23 | 2.16 | 1.0 | yes | 1 interference(s) | passes |

Stacks not run separately: strike <-> post is the bracket_post stack (same screw, same leg, same nut).
The hinge_pin and latch_pivot stacks also check the bearing clearances (pin to bracket bore 0.10, washer
to door 0.32, spring ID to the shoulder 0.80).

### Findings for the design lead (all accepted in SPEC rev B, e14f420)

The first run of these tests found: PEM nut threads are A + T (5.23 M6, 2.77 M4/M5), not sheet + T
(R1); one fit-rule minimum per size can only be the shortest whole nut (now m4 2.77, m5 2.77, m6 4.78,
m8 8.0, R1); M6 x 10 and M4 x 35 went only one pitch past (now M6 x 12 at bracket, strike and carrier to
pan, and M4 x 40 fan screws with the tray 1.60 lower, R2); arithmetic slips in the fan, latch link,
hinge pin and cage nut stacks (R3); anchor pairs that meet only with an offset (R4); `stock()` limits
must be `per_t(..)` (R5); one 8.2 ID spacer for hinge and hanger (R6).

The stacks above are the rev B ones: every one now also passes the SPEC section 8 minima.
- fan_screw: M4 x 40 runs 5.72 past the S-M4-0 and stops 1.88 under the top pan floor (checked by a
  clearance expect against a stand-in floor 9.6 above the tray).
- hanger: the ISO 7089-8 sits between the spacer and the tray, and the nyloc bears on the tray's
  underside (tray at 12.62 below the pan's outer face, Z 893.38); the stud is 2.50 past the nut.
- `iso7045_m4x35.fg` is superseded by `iso7045_m4x40.fg` and is kept only until it can be deleted.
- Spring: still no supplier row. Modelled as wire 1.2 on a 10.8 mean diameter (ID 9.6, OD 12.0), 4
  turns as a plain helix, solid 4.8, about 8 N/mm with 2 active turns.

## Parts

Numbers marked "modelled" in a file are not in the source and were chosen to fit inside it.

### `iso7380_m5x10.fg`
ISO 7380-1 hexagon socket button head screw M5 x 10. Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7380/ (ISO 7380-1 table).  
Dimensions: d 5, P 0.8, l 10, dk 9.5, k 2.75, R 5.5, s 3, t 1.56, n6 count(6).  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso7380_m5x12.fg`
ISO 7380-1 hexagon socket button head screw M5 x 12. Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7380/ (ISO 7380-1 table).  
Dimensions: d 5, P 0.8, l 12, dk 9.5, k 2.75, R 5.5, s 3, t 1.56, n6 count(6).  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso7380_m4x25.fg`
ISO 7380-1 hexagon socket button head screw M4 x 25. Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7380/ (ISO 7380-1 table).  
Dimensions: d 4, P 0.7, l 25, dk 7.6, k 2.2, R 4.4, s 2.5, t 1.3, n6 count(6).  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso7380_m6x10.fg`
ISO 7380-1 hexagon socket button head screw M6 x 10. Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7380/ (ISO 7380-1 table).  
Dimensions: d 6, P 1, l 10, dk 10.5, k 3.3, R 5.9, s 4, t 2.08, n6 count(6).  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso7380_m6x12.fg`
ISO 7380-1 hexagon socket button head screw M6 x 12. Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7380/ (ISO 7380-1 table).  
Dimensions: d 6, P 1, l 12, dk 10.5, k 3.3, R 5.9, s 4, t 2.08, n6 count(6).  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso4017_m8x20.fg`
ISO 4017 hexagon head screw M8 x 20. Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/4017/.  
Dimensions: d 8, P 1.25, l 20, s 13, k 5.3, n6 count(6).  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso4014_m8x55.fg`
ISO 4014 hexagon head bolt M8 x 55. Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/4017/ (head) and ISO 4014 b = 2d + 6.  
Dimensions: d 8, P 1.25, l 55, s 13, k 5.3, n6 count(6), b 22.  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `l - b` `external_thread` size `d` depth `b`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso7379_8x12_m6.fg`
ISO 7379 hexagon socket shoulder screw 8 x 12, thread M6 x 11. Frame: origin on the axis at the head underside; +z along the shoulder to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7379/ (ds 8, dk 13, k 5.5, s 4, b 11); Ganter ISO 7379-8-M6 sheets (l2 11).  
Dimensions: ds 8, ls 12, d 6, P 1, lt 11, dk 13, k 5.5, s 4, t 3, c 0.5, n6 count(6).  
Anchors (all normals +z):
- `head` at z = `0`: head underside, normal along the shoulder towards the tip
- `shoulder` at z = `0` `shaft` size `ds` depth `ls`: the shoulder (a pin), from the head underside along +z
- `clamp` at z = `ls`: the shoulder's end face, where it clamps on the nut side part
- `thd` at z = `ls` `external_thread` size `d` depth `lt`: external thread, from the shoulder end to the tip
- `tip` at z = `l`: the end of the thread

### `iso7379_8x16_m6.fg`
ISO 7379 hexagon socket shoulder screw 8 x 16, thread M6 x 11. Frame: origin on the axis at the head underside; +z along the shoulder to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7379/ (ds 8, dk 13, k 5.5, s 4, b 11); Ganter ISO 7379-8-M6 sheets (l2 11).  
Dimensions: ds 8, ls 16, d 6, P 1, lt 11, dk 13, k 5.5, s 4, t 3, c 0.5, n6 count(6).  
Anchors (all normals +z):
- `head` at z = `0`: head underside, normal along the shoulder towards the tip
- `shoulder` at z = `0` `shaft` size `ds` depth `ls`: the shoulder (a pin), from the head underside along +z
- `clamp` at z = `ls`: the shoulder's end face, where it clamps on the nut side part
- `thd` at z = `ls` `external_thread` size `d` depth `lt`: external thread, from the shoulder end to the tip
- `tip` at z = `l`: the end of the thread

### `iso7045_m4x40.fg`
ISO 7045 cross recessed pan head screw M4 x 40 (fan screw). Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7045/ (dk 8, k 3.1, rf 8, m 4.6).  
Dimensions: d 4, P 0.7, l 40, dk 8, k 3.1, rf 8, m 4.6, w 1, tr 1.9.  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso7045_m4x35.fg`
ISO 7045 cross recessed pan head screw M4 x 35 (fan screw). Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: https://www.fasteners.eu/standards/ISO/7045/ (dk 8, k 3.1, rf 8, m 4.6); superseded by M4 x 40 in SPEC rev B R2, kept until removed.  
Dimensions: d 4, P 0.7, l 35, dk 8, k 3.1, rf 8, m 4.6, w 1, tr 1.9.  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `rack_screw_m6x16.fg`
rack screw M6 x 16, slotted cheese head (Rittal DK 7094.110; head per ISO 1207). Frame: origin on the axis at the head underside; +z along the shank to the tip; head at z < 0.  
Source: Rittal DK 7094.110 (M6, 16 thread length, cheese head); head per ISO 1207 M6 (dk 10, k 3.9, n 1.6, t 1.6).  
Dimensions: d 6, P 1, l 16, dk 10, k 3.9, n 1.6, t 1.6.  
Anchors (all normals +z):
- `head` at z = `0`: head underside (the seat), normal along the shank towards the tip
- `thd` at z = `0` `external_thread` size `d` depth `l`: external thread, from where the full thread starts towards the tip
- `tip` at z = `l`: the end of the shank, normal outwards

### `iso10511_m4.fg`
ISO 10511 prevailing torque type hexagon thin nut with non-metallic insert M4. Frame: origin on the axis in the bearing face; +z through the nut towards the insert crown.  
Source: https://www.fasteners.eu/standards/ISO/10511/ (table per DIN 985: s 7, h 5, m 2.9, dw 5.9).  
Dimensions: d 4, P 0.7, s 7, h 5, m 2.9, dw 5.9, n6 count(6).  
Anchors (all normals +z):
- `seat` at z = `0`: bearing face (on the washer), normal through the nut
- `thd` at z = `0` `internal_thread` size `d` depth `h`: internal thread, the whole height (the screw must pass the insert)
- `end` at z = `h`: top of the insert crown

### `iso10511_m5.fg`
ISO 10511 prevailing torque type hexagon thin nut with non-metallic insert M5. Frame: origin on the axis in the bearing face; +z through the nut towards the insert crown.  
Source: https://www.fasteners.eu/standards/ISO/10511/ (table per DIN 985: s 8, h 5, m 3.2, dw 6.9).  
Dimensions: d 5, P 0.8, s 8, h 5, m 3.2, dw 6.9, n6 count(6).  
Anchors (all normals +z):
- `seat` at z = `0`: bearing face (on the washer), normal through the nut
- `thd` at z = `0` `internal_thread` size `d` depth `h`: internal thread, the whole height (the screw must pass the insert)
- `end` at z = `h`: top of the insert crown

### `iso10511_m6.fg`
ISO 10511 prevailing torque type hexagon thin nut with non-metallic insert M6. Frame: origin on the axis in the bearing face; +z through the nut towards the insert crown.  
Source: https://www.fasteners.eu/standards/ISO/10511/ (s 10, h 6, m 4, dw 8.9).  
Dimensions: d 6, P 1, s 10, h 6, m 4, dw 8.9, n6 count(6).  
Anchors (all normals +z):
- `seat` at z = `0`: bearing face (on the washer), normal through the nut
- `thd` at z = `0` `internal_thread` size `d` depth `h`: internal thread, the whole height (the screw must pass the insert)
- `end` at z = `h`: top of the insert crown

### `iso10511_m8.fg`
ISO 10511 prevailing torque type hexagon thin nut with non-metallic insert M8. Frame: origin on the axis in the bearing face; +z through the nut towards the insert crown.  
Source: https://www.fasteners.eu/standards/ISO/10511/ (s 13, h 8, m 5.5, dw 11.6).  
Dimensions: d 8, P 1.25, s 13, h 8, m 5.5, dw 11.6, n6 count(6).  
Anchors (all normals +z):
- `seat` at z = `0`: bearing face (on the washer), normal through the nut
- `thd` at z = `0` `internal_thread` size `d` depth `h`: internal thread, the whole height (the screw must pass the insert)
- `end` at z = `h`: top of the insert crown

### `iso7089_4.fg`
ISO 7089 plain washer, size 4. Frame: origin on the axis in the bottom face; +z through the part.  
Source: https://www.fasteners.eu/standards/ISO/7089/ (d1 4.3, d2 9, h 0.8).  
Dimensions: di 4.3, do 9, h 0.8.  
Anchors (all normals +z):
- `a` at z = `0`: bottom face, normal +z
- `b` at z = `h`: top face, normal +z

### `iso7089_5.fg`
ISO 7089 plain washer, size 5. Frame: origin on the axis in the bottom face; +z through the part.  
Source: https://www.fasteners.eu/standards/ISO/7089/ (d1 5.3, d2 10, h 1.0).  
Dimensions: di 5.3, do 10, h 1.  
Anchors (all normals +z):
- `a` at z = `0`: bottom face, normal +z
- `b` at z = `h`: top face, normal +z

### `iso7089_6.fg`
ISO 7089 plain washer, size 6. Frame: origin on the axis in the bottom face; +z through the part.  
Source: https://www.fasteners.eu/standards/ISO/7089/ (d1 6.4, d2 12, h 1.6).  
Dimensions: di 6.4, do 12, h 1.6.  
Anchors (all normals +z):
- `a` at z = `0`: bottom face, normal +z
- `b` at z = `h`: top face, normal +z

### `iso7089_8.fg`
ISO 7089 plain washer, size 8. Frame: origin on the axis in the bottom face; +z through the part.  
Source: https://www.fasteners.eu/standards/ISO/7089/ (d1 8.4, d2 16, h 1.6).  
Dimensions: di 8.4, do 16, h 1.6.  
Anchors (all normals +z):
- `a` at z = `0`: bottom face, normal +z
- `b` at z = `h`: top face, normal +z

### `nylon_spacer_8x14x8.fg`
PA66 spacer 8.2 ID x 14 OD x 8.0 (hinge pins and fan tray hangers). Frame: origin on the axis in the bottom face; +z through the part.  
Source: SPEC section 7 (8.2-8.4 ID x 14 x 8); 8.2 taken so it also bears on the 8.0 shoulder.  
Dimensions: di 8.2, do 14, h 8, nb 8.  
Anchors (all normals +z):
- `a` at z = `0`: bottom face, normal +z
- `b` at z = `h`: top face, normal +z
- `bore` at z = `0` `bore` size `nb` depth `h`: the bore, along +z

### `plastic_washer_m6.fg`
plastic washer for the M6 rack screw (supplied with Rittal DK 7094.110). Frame: origin on the axis in the bottom face; +z through the part.  
Source: SPEC 5.11 (1.0 thick, modelled); 6.4 x 12 assumed: Rittal publishes no size.  
Dimensions: di 6.4, do 12, h 1.  
Anchors (all normals +z):
- `a` at z = `0`: bottom face, normal +z
- `b` at z = `h`: top face, normal +z

### `pem_s_m4_0_t2.fg`
PEM S-M4-0 self-clinching nut, as installed in a 1.88 sheet. Frame: origin on the axis in the sheet's flush face (the face the screw enters); +z through the sheet to the nut head; the shank's end is recessed t - A below the flush face.  
Source: PEM bulletin CL (https://damlite.pemnet.com/CLS-M6-1_pdf_2.pdf), S/SS/CLS table, shank code 0.  
Dimensions: tsh 1.88, d 4, P 0.7, A 0.77, C 5.38, E 7.87, T 2, c 0.3.  
Anchors (all normals +z):
- `flush` at z = `0`: the sheet's flush face: place on the sheet part's PEM anchor (same origin, same normal)
- `thd` at z = `z0` `internal_thread` size `d` depth `A + T`: internal thread, through the shank and head: A + T
- `end` at z = `tsh + T`: the head's top face (where a screw tip protrudes)

### `pem_ss_m5_0_t3.fg`
PEM SS-M5-0 self-clinching nut, as installed in a 3.02 sheet. Frame: origin on the axis in the sheet's flush face (the face the screw enters); +z through the sheet to the nut head; the shank's end is recessed t - A below the flush face.  
Source: PEM bulletin CL (https://damlite.pemnet.com/CLS-M6-1_pdf_2.pdf), S/SS/CLS table, shank code 0.  
Dimensions: tsh 3.02, d 5, P 0.8, A 0.77, C 6.33, E 8.64, T 2, c 0.3.  
Anchors (all normals +z):
- `flush` at z = `0`: the sheet's flush face: place on the sheet part's PEM anchor (same origin, same normal)
- `thd` at z = `z0` `internal_thread` size `d` depth `A + T`: internal thread, through the shank and head: A + T
- `end` at z = `tsh + T`: the head's top face (where a screw tip protrudes)

### `pem_s_m6_0_t3.fg`
PEM S-M6-0 self-clinching nut, as installed in a 3.02 sheet. Frame: origin on the axis in the sheet's flush face (the face the screw enters); +z through the sheet to the nut head; the shank's end is recessed t - A below the flush face.  
Source: PEM bulletin CL (https://damlite.pemnet.com/CLS-M6-1_pdf_2.pdf), S/SS/CLS table, shank code 0.  
Dimensions: tsh 3.02, d 6, P 1, A 1.15, C 8.73, E 11.18, T 4.08, c 0.4.  
Anchors (all normals +z):
- `flush` at z = `0`: the sheet's flush face: place on the sheet part's PEM anchor (same origin, same normal)
- `thd` at z = `z0` `internal_thread` size `d` depth `A + T`: internal thread, through the shank and head: A + T
- `end` at z = `tsh + T`: the head's top face (where a screw tip protrudes)

### `pem_s_m6_0_t2.fg`
PEM S-M6-0 self-clinching nut, as installed in a 1.88 sheet. Frame: origin on the axis in the sheet's flush face (the face the screw enters); +z through the sheet to the nut head; the shank's end is recessed t - A below the flush face.  
Source: PEM bulletin CL (https://damlite.pemnet.com/CLS-M6-1_pdf_2.pdf), S/SS/CLS table, shank code 0.  
Dimensions: tsh 1.88, d 6, P 1, A 1.15, C 8.73, E 11.18, T 4.08, c 0.4.  
Anchors (all normals +z):
- `flush` at z = `0`: the sheet's flush face: place on the sheet part's PEM anchor (same origin, same normal)
- `thd` at z = `z0` `internal_thread` size `d` depth `A + T`: internal thread, through the shank and head: A + T
- `end` at z = `tsh + T`: the head's top face (where a screw tip protrudes)

### `pem_s_m6_0_t1.fg`
PEM S-M6-0 self-clinching nut, as installed in a 1.52 sheet. Frame: origin on the axis in the sheet's flush face (the face the screw enters); +z through the sheet to the nut head; the shank's end is recessed t - A below the flush face.  
Source: PEM bulletin CL (https://damlite.pemnet.com/CLS-M6-1_pdf_2.pdf), S/SS/CLS table, shank code 0.  
Dimensions: tsh 1.52, d 6, P 1, A 1.15, C 8.73, E 11.18, T 4.08, c 0.4.  
Anchors (all normals +z):
- `flush` at z = `0`: the sheet's flush face: place on the sheet part's PEM anchor (same origin, same normal)
- `thd` at z = `z0` `internal_thread` size `d` depth `A + T`: internal thread, through the shank and head: A + T
- `end` at z = `tsh + T`: the head's top face (where a screw tip protrudes)

### `pem_fh_m4_12.fg`
PEM FH-M4-12 flush-head self-clinching stud. Frame: origin on the axis in the head's top face, which is flush with the sheet's outer face; +z along the stud.  
Source: PEM bulletin FH (https://damlite.pemnet.com/FH-M5-12ZC_pdf_1.pdf), FH metric table.  
Dimensions: d 4, P 0.7, l 12, H 5.9, hh 0.8, S 2.4.  
Anchors (all normals +z):
- `flush` at z = `0`: head top, flush with the sheet's outer face: place on the sheet part's stud anchor
- `thd` at z = `S + 2 * P` `external_thread` size `d` depth `l - S - 2 * P`: external thread, full from S + 2P to the end
- `tip` at z = `l`: the stud's end

### `pem_fh_m6_20.fg`
PEM FH-M6-20 flush-head self-clinching stud. Frame: origin on the axis in the head's top face, which is flush with the sheet's outer face; +z along the stud.  
Source: PEM bulletin FH (https://damlite.pemnet.com/FH-M5-12ZC_pdf_1.pdf), FH metric table.  
Dimensions: d 6, P 1, l 20, H 8.2, hh 1, S 3.  
Anchors (all normals +z):
- `flush` at z = `0`: head top, flush with the sheet's outer face: place on the sheet part's stud anchor
- `thd` at z = `S + 2 * P` `external_thread` size `d` depth `l - S - 2 * P`: external thread, full from S + 2P to the end
- `tip` at z = `l`: the stud's end

### `pem_fh_m8_25.fg`
PEM FH-M8-25 flush-head self-clinching stud. Frame: origin on the axis in the head's top face, which is flush with the sheet's outer face; +z along the stud.  
Source: PEM bulletin FH (https://damlite.pemnet.com/FH-M5-12ZC_pdf_1.pdf), FH metric table.  
Dimensions: d 8, P 1.25, l 25, H 9.6, hh 1.2, S 3.7.  
Anchors (all normals +z):
- `flush` at z = `0`: head top, flush with the sheet's outer face: place on the sheet part's stud anchor
- `thd` at z = `S + 2 * P` `external_thread` size `d` depth `l - S - 2 * P`: external thread, full from S + 2P to the end
- `tip` at z = `l`: the stud's end

### `latch_spacer_6x15.fg`
steel round spacer 6.0 OD x 4.3 ID x 15.0 (latch link posts). Frame: origin on the axis in the bottom face; +z through the part.  
Source: SPEC 5.8 / section 7 (spec only, no part number).  
Dimensions: di 4.3, do 6, h 15.  
Anchors (all normals +z):
- `a` at z = `0`: bottom face, normal +z
- `b` at z = `h`: top face, normal +z

### `ball_4.fg`
steel ball 4 (ISO 3290 G100). Frame: origin at the ball's centre.  
Source: SPEC 5.8 / section 7: 4.0 steel ball, ISO 3290 G100.  
Dimensions: db 4.  
Anchors (all normals +z):
- `c` at z = `0`: the centre, normal +z

### `latch_spring_free.fg`
latch compression spring at height 12 (representative: wire 1.2, mean diameter 10.8, 4 turns). Frame: origin on the axis in the seat plane z = 0; the coil runs to z = H.  
Source: SPEC 5.8 (ID >= 8.4, OD <= 13, free ~12, solid <= 5.0, ~8 N/mm); no supplier row yet: wire 1.2, mean 10.8 (ID 9.6, OD 12.0), 4 turns, solid 4.8, rate G d^4 / (8 D^3 na) = 8.1 N/mm with 2 active.  
Dimensions: H 12, dw 1.2, D 10.8, nt count(4).  
Anchors (all normals +z):
- `a` at z = `0`: seat at z = 0 (on the handle's inner face)
- `b` at z = `H`: seat at z = H (on the door's outer face)

### `latch_spring_h8_17.fg`
latch compression spring at height 8.17 (representative: wire 1.2, mean diameter 10.8, 4 turns). Frame: origin on the axis in the seat plane z = 0; the coil runs to z = H.  
Source: SPEC 5.8 (ID >= 8.4, OD <= 13, free ~12, solid <= 5.0, ~8 N/mm); no supplier row yet: wire 1.2, mean 10.8 (ID 9.6, OD 12.0), 4 turns, solid 4.8, rate G d^4 / (8 D^3 na) = 8.1 N/mm with 2 active.  
Dimensions: H 8.17, dw 1.2, D 10.8, nt count(4).  
Anchors (all normals +z):
- `a` at z = `0`: seat at z = 0 (on the handle's inner face)
- `b` at z = `H`: seat at z = H (on the door's outer face)

### `latch_spring_h7_77.fg`
latch compression spring at height 7.77 (representative: wire 1.2, mean diameter 10.8, 4 turns). Frame: origin on the axis in the seat plane z = 0; the coil runs to z = H.  
Source: SPEC 5.8 (ID >= 8.4, OD <= 13, free ~12, solid <= 5.0, ~8 N/mm); no supplier row yet: wire 1.2, mean 10.8 (ID 9.6, OD 12.0), 4 turns, solid 4.8, rate G d^4 / (8 D^3 na) = 8.1 N/mm with 2 active.  
Dimensions: H 7.77, dw 1.2, D 10.8, nt count(4).  
Anchors (all normals +z):
- `a` at z = `0`: seat at z = 0 (on the handle's inner face)
- `b` at z = `H`: seat at z = H (on the door's outer face)

### `latch_spring_h7_49.fg`
latch compression spring at height 7.49 (representative: wire 1.2, mean diameter 10.8, 4 turns). Frame: origin on the axis in the seat plane z = 0; the coil runs to z = H.  
Source: SPEC 5.8 (ID >= 8.4, OD <= 13, free ~12, solid <= 5.0, ~8 N/mm); no supplier row yet: wire 1.2, mean 10.8 (ID 9.6, OD 12.0), 4 turns, solid 4.8, rate G d^4 / (8 D^3 na) = 8.1 N/mm with 2 active.  
Dimensions: H 7.49, dw 1.2, D 10.8, nt count(4).  
Anchors (all normals +z):
- `a` at z = `0`: seat at z = 0 (on the handle's inner face)
- `b` at z = `H`: seat at z = H (on the door's outer face)

### `cage_nut_m6.fg`
M6 cage nut for 9.5 square holes (Rittal DK 2094.200), envelope. Frame: origin on the axis in the rail's rear face (the cage's front face); +z behind the rail, the direction a screw from the front advances; x across the rack (the 11.81 side).  
Source: Rittal DK 2094.200 (M6, 0.8-2.5 metal); envelope from Panduit CNWSM6-C drawing 10119-20.  
Dimensions: d 6, P 1, wx 11.81, wy 10.11, dp 8.03, tn 4.78, dc 6.6.  
Anchors (all normals +z):
- `seat` at z = `0`: the cage's front face on the rail's rear face, centred on the square hole
- `thd` at z = `zn` `internal_thread` size `d` depth `tn`: internal thread, tn long
- `end` at z = `dp`: the back of the cage

### `shop.fg`
Written by `gen.py` from SPEC section 3 verbatim (the block from `shared t3` to `hardware: hw_scs)`),
so it always carries the SPEC's hardware rows, now including FH-M4-12. A 80 x 40 t2 probe sheet with an
S-M4-0, an S-M6-0 and an FH-M4-12 proves it builds.

### Test stand-ins (`tests/plates/`)
Plain slabs with the real hole of the sheet they stand for, anchors `a` (z = 0) and `b` (z = t). They
are not rack parts.
