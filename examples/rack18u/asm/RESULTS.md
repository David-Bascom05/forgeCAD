# RACK-18U top assembly: results

For David, 2026-11-03. Everything here is on SPEC rev B + R11 + R12, with the subsystems at frame
fa3f682, panels eb0defc/013c62b, casters aa8fe8a, plus fans and hw. fg is forge-lang fb7941fd: the
overlap speed fix, the kernel's bracket/post boolean fix, and the new DFM checks (every hole checked,
PEMs against the other holes).

## Verdict

`asm/rack.fg` passes every check, with zero unintended interference:
- 537 expects, 0 failed, 0 unmeasured;
- interferences 0 at every param set;
- mobility 0: every motion is a drive (2 doors, 2 latches with turn and lift, 4 swivels, 4 wheels, 2 brakes);
- 154 threaded engagements, none short.

The order package in `asm/order/` comes from this run. `order.py` reported no problems.

## What the assembly is

There are 552 instances, 853 parts in the BOM. Every part hangs off the base pan by its SPEC joint
(R8 pairs, R9 turns, R4 offsets), one joint per part. The other anchor pair of each part is checked
by a distance expect.
- **Frame:** posts and carriers are kits with their PEM bodies (`post_kit.fg`, `carrier_kit.fg`).
- **Fan tray:** `fans/fan_assy.fg` on its hanger stacks.
- **Panels, doors, latches:** flattened from panels' DOOR block by `gen_rack.py`, so each door's hinge and latch move here.
- **Casters:** flattened per corner from `casters/caster_lock.fg` (front) and `caster_plain.fg` (rear), so swivel, roll and brake move here.
- **Dummies:** D01/D02/D04 on the front rails, each with 4 cage nuts, plastic washers and rack screws.
- **Rails:** they move by Count params k_front / k_rear (SPEC 6.3 fallback). rail_front and rail_rear are derived from them.

## What passed (counts are expects)

```
casters            238   every 45deg of swivel x brake off/on (front): yoke and tread >= 0.5 to the pan,
                         tread and pedal >= 5 to the side panel, front to rear >= 50, wheel bottom -111.00;
                         roll turns the 608 cage at 0.6323, swivel turns both ball rows at 0.5; all four
                         swivelled and rolled with no interference
panels/doors/latch 118   both doors: top pin coaxial (open, closed), swing clearances at 50 and 105deg,
                         contact past the limit (115deg), latch clear open / swinging / near closed, a
                         closed latch stops the door (0.5deg), U clear of the door, handle >= 3 to the
                         padlock tab, padlock holes on 5.8's point; the R12 pivot nyloc >= 0.4 to the
                         cam's bore and >= 1.3 to the balls; panel, bracket and strike pairs
fans                69   fans >= 20 to every rail, tray >= 5 to the top carriers, 16 screw tips >= 1.5 under
                         the top pan, the 4U dummy >= 5 to every guard and screw, grille bars exactly the
                         drawn area in each air column, fan centres +-0.1
EIA alignment       63   at (100, 725), (50, 750), (100, 400): front/rear y, z of holes 0 / 26 / 53 on all
                         four rails (flipped ones by 53 - k), x = +-232.55
frame               37   every solved origin on its SPEC 5 pose, turned parts' second points, unused
                         R8 pairs coincident (rails) or 3.02 apart (posts, carriers)
equipment           11   every dummy ear hole on its rail hole; dummies >= 50 to the front door
interferences        5+  0 at the default, both other rail positions, two other dummy U sets, doors at
                         50 / 105deg, latches closed / riding, brakes on, casters swivelled and rolled
```

## Order package (`asm/order/`)

- **Per fabricated part (13 numbers, 33 parts):**
  - `RACK-nnn_<name>.step`: the folded part, read back by OpenCascade as 1 valid solid whose volume equals Forge's;
  - `.dxf`: the flat;
  - the run JSON.
- **`parts.md`:** for each part, the quantity (assembly BOM = SPEC 4), thickness, flat size, bend table and PEM list.
  - Every flat fits 762 x 914.4.
  - Every bend is 90deg or less and at most 906 long.
  - Zero shop and DFM warnings on all 13 parts, with fb7941fd's checks (every hole, PEMs against the
    other holes).
- **PEMs:** 270 in all, which matches SPEC 4 after R12 (the latch bases no longer take an S-M6-0):
  - FH-M4-12 6
  - FH-M6-20 16
  - FH-M8-25 4
  - S-M4-0 20
  - S-M6-0 176
  - SS-M5-0 48
- **`purchased.md`:** every line matches SPEC 5.12 and 7, including ISO 10511 M6 at 18 (R12: 2 for the
  latch pivots). The hardware inside each Blickle caster (axle bolt and nut) comes with the caster and is listed apart.
- **Renders:** `renders/rack_front.png`, `rack_iso.png`, and `rack_doors_90.png` (both doors at 90deg).

## Check before ordering

1. **Finger guard loop height (R10).** The M4 x 40 fan screws assume the guard's loops sit 5.0 below
   the fan face. If the bought guard's loops lie flat on the fan, M4 x 40 ends 1.12 into the top pan
   floor: order M4 x 35 instead. Measure the guard first.
2. **Fan bore.** The tray openings are 116, and the model assumes the fan's venturi bore is 114, read
   off Delta's drawing to about +-1. Check the bought fan's bore is 114 or less.
3. **Latch spring.** It is still representative (1.2 wire, 9.6 ID x 12.0 OD, solid 4.8, about 8 N/mm).
   No supplier part number has been verified. Pick a maker's row inside SPEC 7's limits before ordering.
4. **Handle hole-to-bend margin.** RACK-321's link holes are 4.18 from the fin's bend tangent against
   SendCutSend's 4.15: a margin of 0.03. It passes the shop check, but confirm with SendCutSend or
   accept a small hole distortion.
5. **Side panel return.** The bend line is drawn 800 long, but the real bend is 620: the return is cut
   back at both ends. Say so if SendCutSend's quote reads 800.
6. **STEP vs bend table.** SendCutSend takes the STEP for bent parts. The DXF plus bend table in
   `parts.md` is the fallback if a STEP is refused.
7. **Latch pivot nut (R12).** The pivot's ISO 10511 M6 nyloc goes on the latch base's back face. Fit it
   from inside before the cam, and hold it with a 10 mm spanner while the pivot is tightened from
   outside. It sits 0.47 inside the cam's 12.5 bore, at the modelled sharp hex.

## Open items

- **Assembly:** none. All checks pass and no interference is unintended.
- **Forge, fixed tonight:**
  - the overlap narrow phase is 20-50x faster (asm-overlap, integrated);
  - the bracket/post intersection now works (kernel fix, integrated);
  - a failed exact intersection is now typed BooleanFailed with the pair and reason (branch asm-boolfail 61211a10, not yet integrated).
- **Forge, noted for the cleanup round:**
  - a param read only by an expect's `at` is reported UnusedParam (`probes/unused_at_param.fg`), so rack.fg carries 27 false UnusedParam warnings;
  - instances inside a subassembly are named by hash in the engagement and interference text;
  - fit rules declared in a subassembly don't cover its pairs from a parent (rack.fg re-declares them);
  - assemblies can't drive a subassembly's joints, so the doors and casters are flattened by `gen_rack.py`.
- **Run time:** one solve and overlap takes about 1.9 s (load 0.9 s, solve 2 ms, overlap 1.0 s for
  2448 candidate pairs). The full run with 81 `at` param sets takes 51 s.

## Files

```
rack.fg          the top assembly (hand-written frame, fans, dummies, checks; GENERATED blocks between
                 markers for panels, doors, casters and their checks)
gen_rack.py      fills the GENERATED blocks from panels/gen_asm.py's DOOR and casters/*.fg
post_kit.fg      post + 28 PEMs + stud washers and nuts       carrier_kit.fg   carrier + 31 PEMs
dummy_d0N.fg     RACK-D01/D02/D04 reference servers
run.py           capped full run (out/rack.json); --quick drops the `at` lines
order.py         the order package from out/rack.json        step_check.py    OCCT STEP read-back
render.py        the three renders                           probes/          Forge repros
```
