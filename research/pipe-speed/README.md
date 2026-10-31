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
