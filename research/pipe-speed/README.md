# Tracing against a fillet's pipe face

What a boolean pays when a cut meets a fillet: `Pipe::implicit` is the distance to a traced spine,
and the tracer asks for it thousands of times. Measured in frame#133 (645M, spine search 87%) and
manifold#20 (328M), by temporary counters in `intcurve.rs` and `blend.rs` (nothing of them is kept).

## What the calls were (frame#133)

```
octree cells      6000
refining seeds   11100    about 3000 seeds, of which nearly all landed outside the region
marching           240
finishing          240
```
So the guess that started this (warm-start the spine search while marching) would have saved almost
nothing: the march is 1.4% of the calls. Each call cost about 33k instructions, 13 exact evaluations
of the spine, each one a Newton step on the spine's own two surfaces.

Seeds by outcome over the bench, `outside` = refined to a point off the region and thrown away:
```
pair               seeds  covered  none  outside  nearchain  traced
Cylinder x Cylinder 143464  17558  19802    80525      25554      25
Cylinder x Torus     11632   3810      0     7027        787       8
Ruled x Plane         4568   1480      0     3084          0       4
Screw x Plane        10270   6941      0     2846        470      13
Torus x Cylinder      3503    671    184     2647          0       1
Pipe x Plane          1980    480      0     1496          0       4
Pipe x Cylinder        910      0      2      908          0       0
```
The octree's root is the cube on the region's longest side, and nothing clipped its cells to the
region, so a flat region (a planar face's box) searched a cube around it and refined every seed there.

## What changed, each measured alone (instructions:u, seq)

```
                               frame  manifold  bits
start                          1484M    667M
one search for value+gradient  1212M    588M    same
scan the chain's own samples   1039M    515M    same
cell test without the value     856M    387M    same
foot on the guiding cubic       662M    316M    manifold, by 4e-14 mm
one side, no finish when far    612M    298M    same as the line above
cells must touch the region     468M    257M    same as the line above
```
- `IntCurve::closest` read each sample through `locate` and `lerp`; with f = 0 that is the sample.
- `Procedural::within`: the spine lies within `reach` (half the longest chord and the sag) of a chain
  sample, so the nearest sample's distance d gives implicit in [d - reach - R, d - R]. Only a cell whose
  r falls inside that band needs the search. It gives the same answer as the value would: no bits move.
- `IntCurve::closest_on_cubic`: `spine_closest` used to run the exact search and then
  `intersection_refine`, which alone reaches the exact foot from anywhere near. The guiding cubic costs
  no surface evaluations and is within about 1e-6 of the curve.
- `Pipe::nearest_on_spine`: more than 1e-2 off the pipe the cubic's point is the answer. The value then
  serves a Newton step or a cell test; `refine` only stops on values below 1e-8, always exact ones.
- `trace`: a cell is kept only if it touches the region grown by a step. A point of the curve in the
  region lies in a leaf cell touching it, so no branch goes missing.

Manifold's difference is at most 4.3e-14 mm on 12 vertices and 28 edge ends, from the foot's last
bits (the exact finish stops at 1e-8 either way); faces, edges and validity are unchanged.

## How it is checked

`blend.rs`, `a_pipes_value_and_cell_test_agree_with_a_fine_walk_of_its_spine`: closed and open traced
spines, points near the spine, around the pipe, across the medial axis and far off, against a walk ten
times finer than the chain: the search never answers farther than the walk (a wrong local minimum would),
and `within` agrees with the value. Run once at 8000 points per spine.

To rerun the counts: put a counter in `Pipe::implicit` and `implicit_gradient`, a phase in `trace` around the
octree, the seed refine, `march` and `finish`, and tally each seed's outcome in `trace`'s seed loop.

`fuzz/gen.py` and `fuzz/compare.py` (with `research/feature-costs/featcost.rs` in examples, old binary
from a checkout of main): 200 random parts with traced-spine fillets cut through: every outcome the same as
main's, including the failures (`gen.py DIR 0 200`, `compare.py OLD NEW DIR OUT`). Main fails 158 of the 200
in its own right (53 at the fillet; 105 at a cut through it: "inconsistent classification", "hole outside
every region"); this batch neither fixes nor worsens them.

## Newton's last step

`corner::newton` and `screw::polish` (screw, ruled and skinned surfaces) ran until a step fell below 1e-13,
so the final iteration only confirmed the one before. Newton's next step after one under 1e-7 is about
1e-14: stop there. Heatsink's bits do not change; volute's and crank's move in the last bits. Regen, seq:
heatsink 1346M to 1282M, volute 2546M to 2422M, crank 476M to 456M, gearbox 405M to 388M. Mesh: heatsink
845M to 785M. Not for Gauss-Newton steps (`polish`'s second try), which converge only linearly off the surface.

Tried for these faces and dropped (mesh):
- First derivatives only near the surface in `corner::newton`: heatsink -1.4%, gearbox regen +4%.
- A triangle's test skipped by the 4/9 rule from its three chords' stored sags: volute +11%, the hash costs
  more than it saves.
- `proc_strips` on all procedural faces, not only swept: volute +16%, the strips fail and the Delaunay runs anyway.

## Regen after the fillets: chamfer's check, corner boxes, loft boundaries

Profiled over the bench (`research/feature-costs/centres.py`); each measured alone, seq, bits unchanged throughout.

- `selfx::crossing`: chamfer flags a handful of faces, but it drew every face's uv loops, took 64 boxes of every
  procedural face and listed every face's vertices first. All three now wait for a pair that needs them: volute
  2422M to 2219M, heatsink 1282M to 1093M.
- A loft against a plane or quadric in `crossing`: `loftmeet::meet` before the tracer, volute 2219M to 2017M.
  `meet::intersect` there as well made guard and quadarm 130% slower, so it is left out: the gate found that.
- `loops_cross`: the y test before the shared-end test, which ran on nearly every pair: volute 2017M to 1942M.
- `HolePatch::whole_box`: a corner patch's face box is its surface's over the whole domain. `procedural_box` drew
  its uv loops by projection, then boxed columns: 42% of heatsink. Heatsink 1093M to 655M, gearbox 388M to 312M. A
  patch that has been cut gets a looser box: a superset, so it can cost time but drops nothing.
- `Hull::misses`: each control point's terms worked out once in the Bernstein sign test: volute 1942M to 1839M.
- `Skinned::curve_uv`: a `SplineRail` between two walls is a side of each, at u = 0 or 1 with v its parameter. Its
  samples (96.6k of volute's 110k boundary samples on loft faces) read their parameters instead of being projected:
  volute 1839M to 1509M. It holds only where the rail's knots are the wall's and rail and wall agree at the rail's
  ends and middle within 1e-8.

Tried and dropped:
- Taking `Screw::project_near` out once helix edges read theirs directly: valve regen +5%.
- The closed form per edge sample in the mesher: volute mesh -1.8%, valve -3%, but history seed 30 failed to mesh
  (`ConstraintsIntersect` on a run-out thread's face). Not worth finding out why for that gain.

Then, also with bits unchanged:
- `IntCurve::onto`, `onto_from_chain`: after a Newton step from values under 1e-6 what is left is about the curvature
  times 1e-12, below `refine`'s 1e-8 up to a curvature of 1e4, so the second look at both surfaces goes: volute 1509M
  to 1418M, frame 404M to 397M, manifold 233M to 223M.
- `Hull::misses` stops at two corner coefficients of opposite signs (they are the quadric's own values there): volute
  1418M to 1372M, crank 454M to 414M.
- `Ruled::within`: as `Pipe::within`, from the nearest of its rows. `reach` bounds how far the carried-on surface
  lies from a row; a cell further off than that is out, one nearer than r to a ruling is in. `ruled.rs` test: a cell
  holding a surface point is never ruled out. Crank 414M to 393M, jack 354M to 331M.

## Corner patch evaluation

Heatsink's profile after the boxes: `deriv` 36% with `eval<J>` 30%; half of that was projection, by Newton through the
whole n-sided blend per iteration.
- `HolePatch::on_side` (from `project_near`): a boundary sample lies on a rail. A short 1-D solve on that one rail finds t;
  the domain point follows in closed form. On side i the ribbon's s = a / (a + b), a and b the distances from the
  neighbouring sides, which are linear along side i: a from 0 to ae, b from bs to 0. So the fraction along the side is
  s bs / (s bs + (1 - s) ae). Mesh: heatsink 787M to 542M, gearbox 321M to 281M; regen: heatsink 655M to 594M.
- `check_geometry`: each edge point from the one before (`project_near`), the full search only to confirm one that looks
  off: heatsink 594M to 517M.
- `eval`: no array of ribbons (a 12-side array of 18-float jets, zeroed on every evaluation), and only the ribbons it comes
  to (none but the nearest at a corner): heatsink 517M to 493M, gearbox 312M to 298M.
- `Scalar::ribbon` for `J`: R(s, d) = P + T d + K d^2 by the chain rule, each partial once, not three generic jet products.
  `ribbon_jet.py` proves it: `~/laya/.venv/bin/python -I research/pipe-speed/ribbon_jet.py`. Heatsink 493M to 475M,
  gearbox 298M to 291M. Bits stay: the Newton they feed stops within rounding of the same point.

## Lofts: construction, and seeds for ruled walls

- `plan` ran twice per loft, for `loft_bounds` and for `loft`: now `loft_and_bounds` from one plan. The regularity check keeps
  a flat list of three vectors a grid point, not nested lists of full derivatives. Volute 1369M to 1244M.
- `Ruled::seeds` (`Procedural::seeds`, `intcurve::trace`): crank spends 42% in the tracer, 70% of it the octree, on ruled walls
  against planes and cylinders. A ruling is a line and the other surface a quadric, so each of a fan of rulings a leaf apart
  meets it at the roots of a quadratic: those are the seeds. Rulings are clipped to the region first, or a thin hole along the
  wall has no seed. A ruling with no root in that stretch gives its point nearest the quadric, where that is within the fan's
  spacing: a branch narrower than the fan lies between two rulings. Seeds go widest roots first, and steps are at most a quarter of
  a small round's radius: from a seed at a narrow loop's tip a chain ran past its own start, and on `p00158` (a 1 mm hole down a
  wall, loop 52.6 mm by a direct count) closed at 73.8 mm. Crank 390M to 275M.
- Checked against the octree in real regens, with a temporary patch that traced both ways and compared (all bench parts;
  `fuzz/gen_ruled.py`, 300 parts, 1410 traces; the bench's 58 all the same): 1391 the same. In 14 traces the octree alone found
  15 branches, each off the wall by more than 1e-3 (`off`, measured from the wall's nearest point; those looked at were 2-8 mm):
  false zeros of the implicit past the carried-on wall, where its foot is clamped. In 5 the counts differ with no point of any
  chain inside the region (p00045, p00139, p00143, p00291 octree 1, named 0; p00106 octree 1, named 2): chains in the margin only.
  Not one branch on the wall was lost.

Against the trim of curves on lofts (`subdivide_pieces`), tried and dropped: pruning first by the guiding cubic plus its error
(no gain; with the error folded into the reach itself the merged leaves broke volute and frame), and a grid `within` for `Skinned`
(volute +1%). What is left there is the loft's own nearest point, which most pieces need.

Seed 27 of the history test no longer needs the n-ary name fallback after these, so the list that only shrinks is empty.

`fuzz/gen_ruled.py` also found a fault in `forge_topo::mass`, on main as here and not touched: on ruled lofts with holes its volume differs from the
mesh's by up to 1% (p00137: mass 96871 or 97655 by which chain the curve has, mesh 97771.64 and 97771.65 either way), and it gives 0 on p00215. 12
of 300 parts differ in `mass` between main and this; their meshes agree to 1e-7, so the geometry is the same.

## Curve search (after the seeds)

- `bisect_sign`: Illinois steps, stopping at a value under 1e-13, not bisection to rounding (30-odd curve evaluations per crossing; now
  median 5). An end already within 1e-13 is returned as it is: at a chain sample on the surface (value exactly 0) the secant step sits on that
  end, which left plain bisection for 25 steps and stopped short of the end by 2e-12, and that sliver of curve cost upto-flat +4%. Checked against
  bisection in the same run, by a temporary patch calling both (bench and about 110 fresh fuzz parts, 5300 crossings): at most 2e-11 mm apart, at a
  crossing of sine 1.6e-3; t at most 7e-11 apart; a slow curve (speed 0.005) has the largest t differences and 7e-14 mm between the points.
- What moved the bits, on the bench's parts against the same run by bisection: vertices at most 2.6e-13 mm apart, edge ends at most 5e-12 in t, same counts.
- `ssi::searched`'s cache takes a box within 1e-11 of the one asked about. Bisection reached the same bits from different brackets, Illinois stops
  up to about 1e-14 apart, so boxes built from those points missed by rounding and retraced. The cache also gains on its own: manifold 225M to 213M,
  nary1 298M to 276M. Mixed 1557M to 1479M, volute 1057M to 1033M, upto 37.4M to 35.8M, crank 275M to 267M.
- Dropped, after measuring: a traced curve's large pieces reading the guiding cubic in `subdivide_pieces`, with `1e-5 + sag` added to the reach
  (mixed a further -1%, volute -2%, crank -1%). That bound is not one: on thread fuzz seed 5004 the cubic lies 2.5 times that from the curve
  (the sag is measured at the chord's middle only, and says nothing of the tangents), so a piece could be pruned with a crossing in it. A bound that does hold,
  0.3 chord + sag (the cubic's terms sum to at most 0.27 chord; measured at most 0.63 of the bound), made mixed +4% and valve +3.6%. As a first test ahead of the
  exact one, against analytic surfaces, it made mixed 5% slower: the pieces rarely prune there.
- Dropped: a radial reject in `Screw::within` (no change on jack, frame, gearbox, valve).

## The near match out

`ssi::searched` taking a box within 1e-11 of one it knew made results depend on load: `bits` on main varied on n-ary lines (plate, torture, fuzz1, fuzz7, fuzz8, housing, mixed at load 60-70). `nary::par_map` starts helpers after 200 microseconds, the memory is per thread, and a box 1e-13 off has its
own answer. With only this removed, 9 runs alike beside a control that varied. It costs a box a rounding off a second trace (upto, manifold, mixed, cold). `ssi.rs`, `a_box_a_little_off_is_another_question`, fails with the near match.
