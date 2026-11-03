# casters: subsystem C (Blickle LKPA-PATH 80KF / 80KF-FI)

SPEC 5.10, 6.2, 7 and 9C. Every moving part of the caster is its own part with its own joint: the
swivel on its two ball rows, the wheel on a 608-2RS bearing (rings, 7 balls, two cage halves, two
seals), and the total-lock brake (pedal lever, pivot pin, lock pin into the lower race's notches).
Blickle publishes only the outside dimensions, so the internals are representative and sized to fit
inside them; the outside dimensions match Blickle's (111 / 80 x 32 / 90 x 66 / 40 offset).

## Files

`gen.py` holds every number and writes every `.fg` file here; edit it and rerun, never the `.fg`
files by hand. Run it capped (its volume integrals use numpy):
```
systemd-run --user --scope -p MemoryMax=4G -p MemorySwapMax=0 nice python3 casters/gen.py
systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 nice python3 casters/run.py   # all files + renders
```

```
assemblies
  caster_lock.fg     Blickle LKPA-PATH 80KF-FI (front corners): 70 instances
  caster_plain.fg    Blickle LKPA-PATH 80KF (rear corners): 67 instances
  bearing_608.fg     608-2RS alone: inner ring ground, outer ring turns, cage follows
  test.fg            the subsystem test (below)
parts (caster frame: SPEC 5.10, origin on the kingpin axis in the plate's top face, z up, wheel at -y)
  plate.fg           90 x 66 x 3.0, holes 8.5 on 75 x 45, kingpin countersink, ball-row orbit anchors
  kingpin.fg         rivet 10.0: 90deg countersunk head flush at z 0, lower head 16 x 3 under the race
  lower_race.fg      2.5 washer z -15.03..-17.53, R 25.5, 24 lock notches (15deg pitch)
  yoke.fg            swivel head z -7.76..-10.26 + skirt + web, legs x +-18.20..20.70, axle 8.4, pedal pin 6.0, pin guide 5.2
  ball_4763.fg       3/16" ball, used 24 + 20 times; anchors uorb / lorb at the rows' orbit centres
  pedal.fg           2.0 lever: two cheeks, shoe, pad (to y 88), cam edge under the lock pin; drawn at brake 0
  pedal_pin.fg       6.0 pin, riveted heads 9 x 1.5 outside the legs
  lock_pin.fg        5.0 pin with a domed foot; drawn at brake 0
  spacer.fg          8.2 x 12.0 x 14.70 tube (x2)
parts (wheel frame: origin at the wheel centre, z along the axle)
  hub.fg             nylon centre, hub 36 (bosses 2 a side), bore 22 to a shoulder, 16 past it
  tread.fg           TPU 80 x 32 on r 32
parts (bearing frame: origin at the 608's centre, z on its axis)
  bearing_inner.fg   d 8, land 12.53, groove r 0.52 Dw on the 15.0 pitch circle, bore chamfer 0.3
  bearing_outer.fg   D 22, land 17.47, groove r 0.53 Dw, seal recesses 19.4, chamfer 0.3
  ball_3969.fg       5/32" ball (x7)
  cage_half.fg       ribbon cage half: web r 6.5..8.5 x 0.45, 7 cups r 2.45, pockets r 2.05
  seal.fg            0.8 rubber seal, in the outer ring's recess, lip on the inner ring's land
test stand-ins and references (not ordered)
  pan_standin.fg     base pan floor with the 16 caster holes and SPEC 5.2 caster_* anchors (R9: normal +z, x +x)
  floor_ref.fg       the floor, top at Z -111
  envelope_ref.fg    ring of inner radius 89.5 about a kingpin: the swivel envelope (90) less the 0.5 the sweep keeps
probes/              Forge gap repros (below); rel.fg shows gear's sign and linear's dims
renders/             iso of every part, caster_lock iso/right/front, caster_plain, bearing_608 iso/front, test
                     (test.png mostly shows the stand-in pan and the envelope gauges; the casters are under them)
```
Axle bolt and nut are `hw/iso4014_m8x55.fg` and `hw/iso10511_m8.fg`; the plate bolts in test.fg are
`hw/iso4017_m8x20.fg`, `hw/iso7089_8.fg`, `hw/iso10511_m8.fg` (SPEC 5.10 stack).

## Using the casters in the rack (subsystem F)

- Place `caster_lock.fg` at FL/FR and `caster_plain.fg` at RL/RR as subassemblies, written at the
  kingpin `pose(at: (+-185, 100 | 700, 0))`, and `fastened(caster_fl.plate.top, pan.caster_fl)`: no
  flip since R9 (both normals +z, x +x).
- References go through the subassembly's instances: SPEC's `caster.top` is `caster_fl.plate.top`, the
  four bolt seats `caster_fl.plate.bolt[0..3]` (normal +z on the plate's underside, order (-22.5, -37.5),
  (22.5, -37.5), (-22.5, 37.5), (22.5, 37.5)), and SPEC 8's `caster_fl.wheel_bottom` is
  `caster_fl.yoke.wheel_bottom` (on the yoke, so it stays at the tread's lowest point while the wheel rolls).
- A subassembly's own fits don't reach the parent (gap G3): copy the 16 part-file `fit_rule`s from
  test.fg (`caster contacts` block) into rack.fg, or every caster brings ~11 zero-volume Interferences.
- A subassembly is rigid at its own file's drives (G1, G2): the rack's casters can't swivel, roll or
  brake from rack.fg, so SPEC 8's mobility line can't count them. test.fg does the swivel sweep.

## How it moves (SPEC 6.2)

```
swivel    = revolute(yoke.kingpin, plate.kingpin, turn: swivel_turn)           free
up[i]     = revolute(ub[i].uorb, plate.uorb[i]);  gear(swivel, up[i], ratio: -0.5)   24 balls, orbit at half
lo[i]     = revolute(lb[i].lorb, plate.lorb[i]);  gear(swivel, lo[i], ratio: -0.5)   20 balls
wheel     = revolute(hub.ax, yoke.axle, turn: roll)                            free
ring_in   = fastened(bi.ax, yoke.axle)            ring_out = fastened(bo.ax, hub.ax)
cage      = revolute(cage_a.ax, yoke.axle);  cage_rel = gear(wheel, cage, ratio: -0.632345)
ball_fix[i] = fastened(bb[i].c, cage_a.pocket_at[i])                              7 balls ride in the cage
pedal     = revolute(lever.pivot, yoke.pedal_pin, turn: brake, limit: (0deg, 20deg))      lock only
lockpin   = slider(lpin.ax, yoke.lock_guide, limit: (0, 4.0))
pin_rel   = rack_pinion(pedal, lockpin, radius: 11.4592)                       4.0 at 20deg
```
Forge's gear turns the follower by -ratio x leader, so SPEC's 0.5 and 0.632 are written negative
(same direction as the leader). A ball's spin about its own centre is not modelled (SPEC 6.2).

**Brake.** The pedal pivots at (y 30, z -40) between its pad (y 62..88) and the wheel (y <= 0), so
pressing the pad down lifts everything on the wheel side of the pivot:
- the cam edge on the +x cheek lifts the lock pin 4.0 into a notch of the lower race (pin top -19.20
  at 0, -15.20 at 20deg, inside the race's -15.03..-17.53). The cam edge is the line tangent to the
  pin's domed foot at both 0 and 20deg; in between the pin lags it by at most 0.22 (gen.py checks it
  never sinks in);
- the shoe (a cross plate between the cheeks) swings into the tread on the side facing the kingpin,
  10deg above axle height, and is tangent to it at 20deg. At 0 the lever clears the tread by 6.57.
The lock pin is at r 23 and 52.5deg, the notches at 7.5 + 15k deg, so it drops into a notch at every
swivel angle that is a multiple of 15deg; between notches it strikes the race (checked at 7.5deg).

**Swivel envelope.** Wheel 81.58 (r of the tread's corner), pedal pad corner 89.27, axle bolt head
~54, yoke ~52. All at least 0.5 inside 90 at every swivel angle and brake state (test.fg); the
pad corner is the closest, 0.73 inside.

## Results

`run.py`, all files ok (no errors, no lints, no warnings but IntendedContact, every expect met):
```
parts            19 files, each `expect valid`, 1 body, volume from gen.py's drawing math (0.05%, 0.1% or
                 0.3% where it is numeric); measured vs drawing within 0.02% except the cage half (0.11%)
bearing_608      13 instances, mobility 0, 0 interferences, 5/5 expects (cage 56.91deg at 90deg of the
                 outer ring; a ball centre 7.5 from the axis)
caster_plain     67 instances, mobility 0, 0 interferences, 25 intended contacts, axle nut 8.0 engaged, 5/5
caster_lock      70 instances, mobility 0, 0 interferences, 26 intended contacts, 9/9 (brake on and off)
test             193 instances, mobility 0, 0 interferences, 169 intended contacts, 20 M8 threads engaged
                 (16 plate bolts, 4 axles; none short), 40/40 expects; ~5 min (every `at` line is a full
                 re-solve and overlap pass, ~35 s)
```
test.fg, what each group proves (FL = lock caster written out, RL = plain written out, FR / RR = the
subassembly files, all bolted to the stand-in pan with ISO 4017 M8 x 20 + ISO 7089-8 + ISO 10511-M8):
```
Blickle outside dims   plate top z 0; wheel bottom -111.00 (anchor) and the tread touching the floor at
                       -111 (clearance 0.0025, no interference); hub centre -71 (80 dia); tread to legs 2.20
                       a side (32 wide in 36.40); plate edges at x -152 / y 145 (66 x 90 about (-185, 100));
                       hub 40 behind the kingpin at swivel 0 (y 60) and beside it at 90deg (x -145); the same
                       wheel-bottom and offset checks on FR, RR, RL
swivel sweep           0..315deg in 45deg steps on FL and RL together, brake on at the odd 45s, both wheels
                       rolled 33deg at 135deg: interferences = 0 each time, with the 89.5 envelope gauges and
                       the floor in the check: inside the 90 envelope with 0.5 to spare, never into the pan,
                       the bolts, the floor or the other casters
fixed clearances       yoke to pan 7.76, upper balls to pan 3.00, pedal to pan 25.7 (brake on), pedal to the
                       envelope gauge 0.22 (r 89.27)
brake off              shoe 6.58 off the tread, lock pin 1.68 under the race
brake on (20deg)       shoe on the tread (0.0003), pin top at -15.20 inside the race, 0.14 clear in its notch;
                       at swivel 7.5deg (between notches) the pin strikes the race (13.3 mm3): it locks only
                       in a notch
rolling elements       upper ball 0 at 22.5deg for swivel 45deg; cage 56.91deg for roll 90deg; RL's balls at 0
```

## SPEC issues (for the design lead)

1. **Lower ball row vs the fork legs.** The lower row (pitch 36.0: balls out to r 20.38, z -10.26..-15.02)
   and any race under it sit where fork legs with inner faces at x +-18.20 would be if they came
   straight down off the swivel head. Without changing a SPEC number, the yoke is modelled as a pressed
   cup: head disc R 29, a skirt r 26.5..29 down to z -22, and a web (r 11..29, z -19.5..-22) from which
   the legs hang. The lower race is then R 25.5 (inside the skirt) and the lock pin is at r 23. If the
   real caster has a narrower lower row, the pitch (36.0) is the number to change.
2. **24 notches at r 23 leave teeth only 0.72 wide** (5.3 notch on a 6.02 pitch). That's the consequence
   of 24 notches + a 5.0 pin inside the skirt; fewer notches (12, 30deg) or a smaller pin would make
   sense in a real part. Representative only; no interface depends on it.
3. **The ball rows have no room for a groove.** 4.763 balls in the 4.76 gaps (plate underside -3.00 to
   head top -7.76; head underside -10.26 to race top -15.03) touch flat faces: the upper balls overlap
   the head by 0.0015, the lower ones overlap the head by 0.0015 and miss the race by 0.0085. A formed
   groove of any real depth would move the ball centres. The raceways are modelled flat at the SPEC z
   values; the ball/head contacts are declared fits.
4. **"Shoe on the tread at the wheel's top-rear"** can't happen with the pivot between pad and wheel
   (a seesaw: the pad goes down, the shoe side goes up). The shoe acts on the tread's kingpin-facing
   side, 10deg above the axle. The lock pin rising with the same motion is consistent with SPEC.
5. **Relation syntax in 6.2:** `gear(.., ratio: 0.5)` turns the balls backwards in Forge (written
   -0.5, -0.632345); `linear(pedal, lockpin, ratio: 0.2 mm/deg)` is a BadRelation (linear relates two
   lengths): `rack_pinion(pedal, lockpin, radius: 11.4592)` gives 0.2 mm/deg. `cage = revolute(..,
   turn: cage_turn)` plus the gear over-constrains the cage: the cage has no drive. `ball[i]` is a
   built-in's name (`ball_fix[i]`).
6. **SPEC 8 / 5.12 names through a subassembly:** `caster.top` -> `caster_fl.plate.top`;
   `caster_fl.wheel_bottom` -> `caster_fl.yoke.wheel_bottom`.
7. **"Pedal 88" in the envelope:** the pad ends at y 88 but is 30 wide, so its corners are at r 89.27.
   Still inside 90 (0.73 clear).

## Modelling choices (not SPEC issues)

- One `yoke.fg` for both variants: the plain caster's yoke carries the (unused) pedal pin and guide holes.
- The 2RS seal lip is modelled touching the inner ring's land (a contact seal, declared fits).
- Cage pockets clear the balls by 0.065; the cage band clears both ring lands (0.235 inside, 0.235 outside).
- Part volumes in each file's `expect volume` are computed in gen.py from the drawing (Pappus for
  revolved sections, shoelace for prisms, fine numeric integration for the groove torus cuts, the
  notches, the cage and the yoke legs over the web), not copied from a run.

## Forge gaps (probes in `probes/`)

- **G1. A subassembly is rigid inside its parent** (`probes/sub_rigid.fg`): `bearing_608.fg` placed
  with its inner ring on an axle and its outer ring on a turning wheel is a `Conflict` (the drive is
  left out). So the caster can't use the bearing as a moving subassembly: its parts are instanced in
  the caster directly (written from the same gen.py text), and `bearing_608.fg` is the bearing alone.
  Same reason the casters can't swivel inside rack.fg.
- **G2. No per-instance drives for a subassembly** (`probes/sub_drive.fg`): a parent can't set
  `swivel_turn`/`roll`/`brake` of a caster instance, and `expect .. at swivel_turn = 90deg` in the parent
  comes back "not measured" (no error). test.fg therefore writes two casters out part by part (fl_,
  rl_) to sweep them; the other two are the subassembly files.
- **G3. A subassembly's fits don't reach the parent's interference check** (`probes/sub_fits.fg`):
  caster_plain.fg alone has 0 interferences, placed in a parent it shows 11 zero-volume Interferences
  between its own fitted pairs, named by hash (`c.#1258b676cbe70085`), not by path. Workaround: the
  same contacts as part-file `fit_rule`s in the parent (works: 0 interferences).
- **G4. A relation's offset comes from the written poses** (`probes/relation_offset.fg`): forge-asm
  takes `follower - slope x leader` at the written poses as the relation's constant. Written at the
  default pose, all 24 balls of a row piled up on one spot (466 interferences). Worse, the "written"
  value is read through each rigid cluster's written pose, so a fastened part written in the wrong place
  (here the axle bolt, which shares the yoke's cluster) shifts every relation hanging off that cluster:
  RL's balls ran 45deg and its cage 33deg off while FL's were right. Every caster instance is therefore
  written at its true zero-drive pose, fastened ones included. SPEC 6 and the reference say written poses
  are only a solve's start; that isn't true once a relation is involved.
- **G5. Part bbox in `fg run --json` is loose for revolved parts** (the torus groove cutter's bounds,
  e.g. 9.644 for an inner ring of land 6.265), so outside dimensions are checked by anchors and
  clearances in test.fg, not by bbox (SPEC gap 16).
- **G6. Every `expect .. at` line is a full re-solve and overlap pass** (~35 s for test.fg's 193
  instances), even for a `value()` line: test.fg's sweep is one interferences line per angle with gauge
  parts, rather than a clearance line per part per angle (which ran over 25 min).
- Gear sign (follower = -ratio x leader) and `linear` taking lengths only are in forge-asm's docs but
  not in `fg reference` (`probes/rel.fg`).
- The names `ball`, `spin`, `dome`, `axis` are built-ins (ShadowsBuiltin); SPEC's `ball[i]` is `ball_fix[i]`.
