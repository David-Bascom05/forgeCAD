# Shop test parts

Two parts like the ones a job shop quotes. Each is machined with every op that fits it and reports
the full certification. They are CAM's coverage targets. A refusal listed here is a gap to close,
not an expected result.

## manifold.fg / manifold_cam.fg (milling, 3-axis VMC)

A 110 × 70 × 32 aluminium block from saw-cut bar (3 mm over on each end, 4.5 under, 1.5 over the
boss). It is machined in two setups in a 150 mm vise.

| feature | where | op |
|---|---|---|
| top face skimmed from saw-cut stock | op1 | `facing` face-50 |
| round boss Ø30 × 6 with an R3 fillet at its foot (a torus: the 3D area) | op1 | `clear` flat-10 |
| outline 110 × 70, R6 corners, 23 deep | op1 | `contour` outside, 2 finish passes + spring |
| deep pocket 18 deep beside a 1.5 mm wall, round island Ø12 | op1 | `pocket` flat-6 |
| the deep pocket's R2 corners | op1 | `pocket` flat-3 |
| shallow pocket 3 deep | op1 | `pocket` flat-6 |
| groove along a centreline | op1 | `slot` |
| M20 × 1.5 bore through the boss | op1 | `drill` (plan "drill"), then `thread_mill` |
| two 6H7 dowel holes, reamed | op1 | `drill` (plan "auto": spot, drill, ream) |
| four M6 tapped holes | op1 | `drill` (plan "auto": spot, drill, tap) |
| top edges broken 0.3 | op1 | `edge_break` chamfer-10-90 |
| bottom faced to thickness | op2 (turned over) | `facing` |
| relief pocket | op2 | `pocket` |
| two M5 counterbored holes from below | op2 | `drill` (plan "auto") |

Results: see the table at the end.

## hub.fg / hub_turn.fg (turning, 2-axis lathe)

A threaded hub turned from a Ø66 bar in one chucking.

| feature | op |
|---|---|
| front face | `turn_face` |
| Ø26 through hole | `turn_drill` drill-26, peck 10 |
| outside: M36 × 1.5 with a lead chamfer, Ø42 with an R2 fillet (a torus) into a step, a taper (cone), a Ø62 flange with R1 on its front edge | `turn_rough`, `turn_finish` |
| 3 wide thread relief, 4 wide O-ring groove, 8 wide neck | `turn_groove` groove-3 |
| 2 × 45° back chamfer facing the chuck | `turn_finish` left-hand VBMT |
| M36 × 1.5 outside | `turn_thread` thread-60 |
| Ø28 bore, 1 × 45° entry chamfer, M30 × 1.5 inside (minor diameter shown) | `bore_rough`, `turn_finish` id-bore-ccmt |
| inside thread relief 3 wide | `turn_groove` id-groove-2 |
| M30 × 1.5 inside | `turn_thread` id-thread-60 |
| part-off at the back face | `part_off` part-3 |
| a 1.4 mm wall where the two thread reliefs overlap (z −22..−23) | (each groove op) |

## Results (cam/parity)

Measured on cam/parity (tip of this commit), release build, one run (`forgecad cam F --json`). Native and
wasm32-wasip1 tests all pass.

### manifold_cam.fg: 0.43 s in total. Refused (tools and 3D roughing).

`prof` runs `from: 32`, the stock's top over the outline: the strip zlevel's grid leaves at the
outline (x −4.757..0, z 26–28.5) is real stock, so the profile takes it. A loop-major contour is
now staged with its own earlier levels' band gone (per-loop and mid-loop stages in
`certify_posted`), so its finishing levels are checked against the stock they meet, not the
saw-cut block.

On cam/parity alone (no zlevel, the library built in):

| op | result | what blocks it |
|---|---|---|
| skim, prof, well, crn, shal, grv, brk, face2, rel | certified | |
| rough | not built | the 2.5D section at z = 29 meets the boss's foot fillet: `NotPrism { face: 38 }`. Needs CAM-3AXIS's zlevel |
| bored, cbh | refused | NeedTool: no 18.5 / 5.5 mm drill (on CAM-HAAS's tool libraries) |
| thr | refused | the only thread mill's pitch is 1, the thread's 1.5 |
| dwl | refused | NeedTool: no 6 mm H7 reamer record exists yet |
| tap | refused | the holder meets stock `rough` would remove (−0.81 on a plunge) |

On merge-train/4b (zlevel and the drills) merged with cam/parity, with
`rough = zlevel(op1, tool: t1, from: 33.5, to: 26, down: 3, stepover: 4)`: 13 of 15 built and
certified. Left: `thr` (no 1.5-pitch thread mill) and `dwl` (no 6 H7 reamer). The tap is now
certified against its declared thread (568bbddf, 32abf3c3), its R plane lifted over the
in-process stock (195b4712).

### hub_turn.fg: certified in full on claude/cam-turn-hub @f8b0152f (14 ops, 1811 checks, 0.58 s)

CAM-TURN closed every item below there:
- hollow parts: the OD and bore ops split at the front face's inner edge;
- G1 against the part less tol, so a touching tool is provable;
- `skip:` on the finishes over the grooves;
- the blade as head + body;
- new tools `part-3-deep` and `drill-26-4d`.

Its hub_turn.fg uses those tools and `skip:`, so a train that merges both branches takes CAM-TURN's
hub_turn.fg. On cam/parity alone (without forge-turn's changes) the file here still gives the
first result: 0.46 s, 6 of 13 ops certified. The refusals were:

| op | result then | what blocked it |
|---|---|---|
| drl | refused | undecided (−1e-5) where the drill is the finished hole's size (zero gap) |
| rough | refused | undecided (−0.8) on moves above the front face next to the bore |
| fin | refused | "the finish path finds no start over the front face": the part is hollow |
| back | refused | the left-hand finish passes 0.197 from stock left by the roughing (rapid clearance 0.2) |
| bor | refused | undecided (−0.4) above the front face, outside the entry chamfer |
| bfin | refused | undecided (−0.365) where the bore finish meets the inside relief groove |
| ithr | refused | undecided (−0.05) at the entry chamfer |
| cut | refused | the part-off blade reaches 25: short of the Ø66 bar's radius 33 |
