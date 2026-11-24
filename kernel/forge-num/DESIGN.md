# forge-num: what is shared, and what is not

forge-num holds the numbers every exact part of ForgeCAD shares: `Nat`, `Big`, `Dy`, the
Shewchuk error-free transformations, and (from train 6) CAM's `Ival` and exact reals. Before
moving more code here, read this page: the next section lists code that looks duplicated but
is not.

## 2D line/circle and circle/circle: six policies, not one routine

ForgeCAD meets lines and circles in the plane in six places. They share a formula's shape but
not a contract: each decides "touching" by its own rule, and each rule is part of what its
crate promises. Owner's decision (2026-11-24, step 5 of the exact-core plan): **they stay
separate.** Do not merge them into one routine. A shared routine would silently change one
crate's tangency rule, and with it that crate's results (bits.txt, its fixtures, and what it
refuses).

| where (main) | inputs | when do they touch | why that rule |
|---|---|---|---|
| forge-geom `meet.rs` `coaxial` / `line_circle` | parametric line (point, unit direction), circle | within `tol::LINEAR` of tangent (also the concentric band, and `tol::ANGULAR` for parallel lines) | the tolerant kernel: near-tangent coaxial surfaces of revolution meet in one circle, as its booleans expect |
| forge-sheet `region.rs` `circle_line` / `circle_circle` | line through two points, circle | `TOL` band, one point | sheet refuses near-coincidence (`NearCoincident`, the 2·TOL band); it never snaps |
| forge-weld `frame.rs` `line_circle` / `circle_circle` | implicit line `a0 + au·x + av·y = 0`, circle | no tolerance: a touch is two equal points | weld's profile corners; a bit-exact copy is in `meet2` below |
| forge-dwg `draft/intersect.rs` `line_circle` / `circle_circle` | parametric, with a caller's `tol` | scale-relative `tol`, flagged `tangent` | drafting snaps at the drawing's tolerance; conics and splines stay in dwg |
| forge-cam-geom `arr.rs` `meet` / `line_circle` | exact reals (`Ctx`) | exact (the sign of the discriminant) | CAM's arrangement is exact end to end: no epsilon |
| forge-brake `geo.rs` (`Seg::params_vs_line`, `line_params`) | segments and arcs of the outline | its own parameter-space rule | the bend simulator's outline geometry |

`meet2` (exact classification by a filtered sign, then exact dyadics, with f64 points) is on the
branch `exact-core/meet2-weld`, unlanded. Weld is its only client: moving it was byte-identical
but +0.0002% on the strict total (layout only, glibc memcmp alignment), so weld keeps its copy.
A new client that wants exact 2D classification can take `meet2` as it is. Changing an existing
routine's rule above is a semantic change: list every value that moves and have the owner judge
it.

## Certified branch and bound

Brake (`forge-brake bb.rs`), CAM (`forge-cam-geom check.rs`, `slab.rs`) and turning
(`forge-turn check.rs`) each certify a clearance. Brake and CAM share a contract (Clear means
proven at least the limit; Near, brake only, means proven within the resolution; Hit is a real
point under the limit; anything else is Undecided). That contract is meant to move into one
crate, `forge-bb`, with each client byte-identical and no slower. Turning is a fixed refinement
ladder, not a search, and stays out.
