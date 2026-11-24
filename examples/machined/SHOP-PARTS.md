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

Measured on cam/parity 66d70c79, release build, one run (`forgecad cam F --json`). Native and
wasm32-wasip1 tests all pass.

### manifold_cam.fg: 25.8 s in total (p1 25.4 s, almost all certification; p2 3 ms). Refused.

(At e11804e6: 146 s, with well and crn refused. The section pocket then cut every drilled hole
of its mid-height section down to `to:`. A dowel's drill point was gouged, and 281k cells went to
certifying it. Holes that end above the bottom are now left to their own ops: bc164b71.)

| op | result | what blocks it |
|---|---|---|
| skim | certified | |
| rough | not built | the 2.5D section at z = 29 meets the boss's foot fillet: `NotPrism { face: 38 }` (no 3D roughing on this branch) |
| prof | refused | the holder meets stock (−1.43) on a plunge: stock that `rough` would have removed |
| well | certified | 3,519 cells |
| crn | certified | 23,302 cells |
| shal | certified | 199,182 cells: most of the time |
| grv | certified | |
| bored | refused | NeedTool: no 18.5 mm drill in the library |
| thr | refused | the library's thread mill has pitch 1; the thread is 1.5 |
| dwl | refused | NeedTool: no 6 mm drill (the reamer's pilot) |
| tap | refused | a rapid 0.088 into stock: stock that `rough` would have removed |
| brk | certified | |
| face2, rel | certified | |
| cbh | refused | NeedTool: no 5.5 mm drill |

### hub_turn.fg: 0.47 s in total. Refused (6 of 13 ops certified).

| op | result | what blocks it |
|---|---|---|
| face, relf, oring, neck, thr, irel | certified | |
| drl | refused | undecided (−1e-5) where the drill is the finished hole's size (zero gap) |
| rough | refused | undecided (−0.8) on moves above the front face next to the bore |
| fin | refused | "the finish path finds no start over the front face": the part is hollow |
| back | refused | the left-hand finish passes 0.197 from stock left by the roughing (rapid clearance 0.2) |
| bor | refused | undecided (−0.4) above the front face, outside the entry chamfer |
| bfin | refused | undecided (−0.365) where the bore finish meets the inside relief groove (it certifies stopped before the groove) |
| ithr | refused | undecided (−0.05) at the entry chamfer |
| cut | refused | the part-off blade reaches 25: short of the Ø66 bar's radius 33 (no blade for bars over Ø50) |
