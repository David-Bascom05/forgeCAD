# Spline and ellipse profile loops

What the sketch profile, extrude, revolve and the booleans use for real free-form curves, and how
it was reached. Code: `kernel/forge-geom/src/spline.rs`, `spline/{extruded,spun,meet}.rs`.

## What real files hold (measured 2026-10-31)
Fusion 360 Gallery, 8625 designs (`scan.py`): profile curves 164845 lines, 44572 arcs, 34747 circles,
10222 NURBS (517 designs), 366 ellipses (102), 178 elliptical arcs (30). NURBS always clamped, none
periodic; degree 3 (8434), 5 (1506), 2 (282); 2305 flagged rational, 15 with weights other than 1; 35
closed; control points median 5, p99 75, max 775; 1110 inner knots of full multiplicity (corners,
some of them only speed jumps). CADFS: fit splines with optional end derivatives, periodic flag.
WHUCAD: fit splines. FreeCAD: B-splines, some periodic.

## Choices
- One curve: piecewise rational Bezier in homogeneous coordinates, B-spline data in (Boehm insertion).
  Conics are rational quadratics (a quarter turn per piece, middle weight cos(half sweep)), so an
  ellipse needs no code of its own past its part-file form.
- Surfaces are by construction, as Parasolid's swept and spun surfaces and STEP's
  surface_of_linear_extrusion and surface_of_revolution: `Extruded`, `Spun`. No surface spline.
- A corner in a curve, or a curve closing on itself, is cut at normalize time; each arc is a face.

## Closed forms (`derive.py` checks each symbolically; needs sympy 1.14, offline)
1. Plane on an extruded curve: an exact spline with the same weights, control heights
   kx X + ky Y + k0 W. A plane along the rulings: Bernstein roots of degree p, one ruling each.
2. Nearest point: stationary points are roots of (A - p w).(A' w - A w'), degree 3p - 1 (2p - 1 with
   no weights), so `Spline::closest` finds the nearest and does not start from a guess.
3. Circle or coaxial surface against a section: a Bernstein polynomial of degree 2p. Parallel
   cylinders give rulings, coaxial surfaces circles.
4. Ray against a spun wall (`upto.rs`): a quadric in (r, h).
5. Spun normal: du x dv = r (h' cos u, h' sin u, -r').
6. Ellipse arc: the rational quadratic above.

`loftmeet` (hull separation, Bernstein quadric sign test, Newton march) takes these walls through
its `Span` trait: their Lean-proved certificates need only positive weights and the patch form, both
held here (research/loft-ssi/shipped). What has no closed form goes there first, then the tracer.

## Loft seams (`seam.py`; `align auto`, `kernel/forge-ops/src/loft.rs`)
Parasolid's seam between circles is not in a loft's history, and it changes the volume by tens of percent.
From the rulings in Onshape's STEP (periodic ruled B-spline walls, the seam at the middle of the first and
last control rows), 19 circle-to-circle walls in 12 parts of CADFS: a section's seam is the point on it
nearest the last section's seam, within 0.6 degrees on every wall; run the other way it misses by up to 180
degrees. The first section starts on its circle's own axis, which fits `ey x n` (`ex x n` for n along y) up to
sign; world-axis dependent, so it is the importer's `seam X Y`, not the kernel's rule.
Polygons do not follow it: over the 119 lofts translated by the importer (`polyseam` in the session,
the part's STEP volume as truth) nearest-vertex lost 2 parts (00420299, 00753783) against rotating by least
squares about each section's centre, which lost none (44 agree, 18 wrong both ways, 1 refused).
OCCT's ThruSections and ACIS's skinning were looked for and not found documented; Onshape's help says
alignment falls back to "estimated proximity" between vertices when no guides are set.

## A smooth loft's curve (2026-11-01; scripts in the session, `apex_volume.py` here)
A natural cubic at each matched point over knots shared by the whole loft, as Parasolid's STEP surfaces show: interior knots
of multiplicity one at the sections, the same on every wall (58 of 59 lofts), second derivative zero at the ends to the finite
difference's resolution. What differs is the knots. Ours were the distances between section centres; Parasolid's are not.
It is not chord length of any one rail, the mean chord, median, rms, harmonic mean, arc-weighted mean, arc length, closest
distance between sections or Hausdorff distance. Fit on the 25 lofts of the dev half, the tenth percentile of the chords between matched
points came closest of those tried, and is what `smooth` does; on the 34 held out the knots miss by median 6e-5, p90 0.004,
worst 0.058. That is a close fit, not Parasolid's rule, which remains unrecovered (it stays near the shortest
rail in symmetric parts: 1e-4 off it, below every rail on some spans).
Two rotations with the same least squares cost go forward about the loft's way, then back to the first section's start: 3 parts, all
exact after it (5e-7, 2e-5, 1e-6), none in the held-out half, so too few to call a rule.

Volume against STEP truth, CADFS sample of 1112 parts by fewest features, "pure" = only sketches and lofts (held-out
half separately). "many" = 3+ sections, "apex" = to a point with 3+ sections or more, "cone" = one section and a point.
```
centres (before)            <=1e-5 <=1e-3 <=1%  <=5%  >5%  stopped untranslated
many pure, dev                   5     23   31    7    3        7     75
many pure, held out              6     18   23   12    2       13     61
many, other ops                  6     17   22    6    7      160    294
apex pure                        0      0    3    3    1        2     15
apex, other ops                  0      0    2    0    0        7     31
cone pure                      215      1    0    0    0        1     18
tenth percentile (after, before the tie rule)
many pure, dev                  12     49    5    2    1        7     74
many pure, held out             17     34    8    2    3       10     61
many, other ops                  7     37    8    6    2      158    295
apex pure                        1      3    1    2    0        2     15
apex, other ops                  0      1    1    0    0        7     31
cone pure                      215      1    0    0    0        1     18
```
The held-out half was seen once under the shortest-chord rule before the tenth percentile was chosen on dev numbers.
A fresh sample, seen only after the rule was fixed: the other 366 plain 3+ section parts of CADFS, more features each; 297 do not
translate (125 spline sections) and 54 stop, so only 15 have a volume to compare, too few to say much: within 1e-3 13 of 15
after, 8 of 15 before; worst 2.9% (00591958, off axis, 2.8% before).
End speed `start W`/`end W` is open: real end speed varies along a section, a single W times a length cannot hold it, and 6
parts did not settle which length; 00992463 went from 0.29% to 1.13%, and the old length costs others as much.
Not measured: 81 lofts between splines and 70 of different segment counts among "many" do not translate (the importer does not translate spline sections, though
the kernel lofts them, or sections of unequal counts), so curves that may matter most are hidden. Remaining worst errors are mostly
sections off a common axis ("oblique", 00125711 30%) and end conditions: not explained yet.

## Fit splines in CADFS, and the truth volumes (2026-11-01)
A sketch's fit spline in Onshape is centripetal: knots by the square root of the chord, periodic when it closes. On 00500055 the
STEP section's area is 1259.51, this gives 1259.47 and matches its box to 1e-3; chord length gives 1530.92, uniform 1206.31. One part
checked this closely; a sweep of the fit-spline extrudes is the wider test. An end derivative is per a parameter running 0..1 over the whole curve (00181673:
control point = end + derivative / 3, exactly). The importer samples them so; the kernel's `thru` still spaces by
the chord, so a `thru` is not Onshape's curve through the same points.
`BRepGProp` on a STEP misreads some faces: 00500055's wall integrates by divergence to 41984 = area x height / 3 where GProp says 27289. So
every truth here was also taken from a mesh of the same file. Where the two differ by more than 3e-3 a part should not be scored:
26 of 181 smooth lofts. Neither is ground truth then (the mesh loses faces on some files: 00707398 is 80% off GProp).

Lofts between spline sections in CADFS, 216 parts, truth as above: 27 build; 14 within 1e-3, none worse among those scored, 13 unsettled (ours
is near the mesh on most of them, which is not a score). 131 do not translate (100 different segment counts, 3 a spline left out of the sketch's
regions), 58 stop, fail or time out (27 twisted, not diagnosed). The two worst before were not spline faults: 00138678's ellipse sketch took a look-alike
record's plane; 00409237's cut lost a region, so a spline in no region is now refused.

Sections of different counts: circles against polygons of one count translate only where the circle's in-line cuts are also even, so that both readings of STEP agree: 6 parts of 190, all
6 within 1e-3. Everything else stays refused. Cutting a polygon of fewer sides to the fullest's count by perimeter fraction was tried (read from 00377891, 3 to 7 sides): 31 built, 16 within 1e-3
and 15 from 0.1% to 4.85% off, and section vertices often missed STEP's, even on that part (the triangle, 4 of 7). Parasolid's matching there is not recovered.

## A smooth loft on to a point (`apex_volume.py`)
A slice is the first section scaled by L(s) about C(s), L and Z natural cubics over the knots, so V = A0 * integral L^2 Z' ds, exact
over the rationals. Mass from the B-rep agrees to 1e-12 (`tests/spline.rs`). OCCT's ThruSections passes through
the sections with its own curve between them, 2-10% off, so the oracle leaves these lofts out; on one cone, where no spline is
assumed, all three agree (523.59877560). Against Parasolid, 7 pure parts: 4 within 1e-3, see the table above; too few to say more.
## Proofs (`lean/`; Lean + Mathlib, the toolchain of research/loft-ssi/shipped)
`SplineLoops/Certs.lean`, no sorry: coefficients of one sign leave no root (`bern_pos`, `bern2_pos`: `bern_roots`'s
test and `CurveHull::misses`), the factor in a product of basis polynomials (`bb_mul`: `bern_mul`), and one sign change
among the coefficients keeps a polynomial at or above zero once there (`one_change_stays`: what `crossings` takes
to mean one root). The patch form and hull of a piece are research/loft-ssi/shipped's. Not proved: rounding.
    lake env lean SplineLoops/Certs.lean

## Measured
`kernel/forge-geom/examples/spline_plane.rs` (cargo run --release -p forge-geom --example
spline_plane), 400 random planes on degree 2/3/5 walls: closed form worst 2.5e-14 mm off either
surface in about 62 us; tracer 9.7e-9 mm in about 19 ms (wall time on a loaded machine); the tracer
returned nothing on 13, not looked into (the closed form returns the whole carried-on curve whatever
the region).

## Prior art
Parasolid docs (q-solid.com/Parasolid_Docs/chapters/fd_chap.26.html) describe B-curves swept and spun
into B-surfaces; its internal intersection methods were not found. Plane against surfaces of
revolution as coaxial revolute quadrics: "Computing planar sections of surfaces of revolution with
revolute quadric decomposition" (researchgate 4082564). Bezier clipping (Sederberg) is the known
alternative for curve-curve; hull subdivision is enough at sketch piece counts.
