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

RESULTS
