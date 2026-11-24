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

### manifold_cam.fg: 21.3 s in total (p1 almost all of it, certification; p2 3 ms). Refused.

(At e11804e6: 146 s, with well and crn refused. The section pocket then cut every drilled hole
of its mid-height section down to `to:`. A dowel's drill point was gouged, and 281k cells went to
certifying it. Holes that end above the bottom are now left to their own ops (bc164b71), and so are
holes and slots the tool only just fits (shal was pocketing the dowels and the groove: 199k cells
to 103k).)

With CAM-HAAS's catalogue tools (tools/turn-presets @b8d58f49, merged for the run, with t5 the
Tungaloy MTEC0807C171.5ISO thread mill): 22.4 s, 12 of 15 ops certified. Every refusal left is
stock that `rough` did not remove.

| op | result | what blocks it |
|---|---|---|
| skim | certified | |
| rough | not built | the 2.5D section at z = 29 meets the boss's foot fillet: `NotPrism { face: 38 }`. Needs 3D roughing (CAM-3AXIS's zlevel) |
| prof | refused | the holder meets stock (−1.43) on a plunge: stock that `rough` would have removed |
| well | certified | 3,519 cells |
| crn | certified | 23,302 cells |
| shal | certified | 103,298 cells: most of the time (the deep well's top, the shallow pocket, the bore's top) |
| grv | certified | |
| bored | certified | Guhring 18.5 drill (tools/turn-presets); refused NeedTool without it |
| thr | certified | Tungaloy 1.5-pitch thread mill (tools/turn-presets); refused on pitch without it |
| dwl | refused | the dowels are now 6 H7 in the part (66c073d8): the plan is a 5.8 drill then a 6 mm reamer (6d91b24f). The 5.8 drill is on CAM-HAAS's tools/libraries; no 6 H7 reamer record exists yet (NeedTool). Before that: a rapid 0.088 into the stock `rough` leaves |
| tap | refused | the same rapid into the same unremoved stock |
| brk | certified | |
| face2, rel | certified | |
| cbh | certified | Guhring 5.5 drill, its point stopped on the flat floor (e7b3cd87: it was a −0.52 gouge through the floor); then the Ø10 × 5 counterbore pocketed by the largest end mill under 10 (9b4e598a). The floor's ring is said |

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
