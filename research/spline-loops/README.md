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

## A smooth loft on to a point (`apex_volume.py`, measured 2026-11-01)
Against OCCT's ThruSections these differ by 2-9%, so the volume was checked from the definition. A slice is the
first section scaled by L(s) about C(s), L and Z natural cubics over the sections' centre distances, so
V = A0 * integral L^2 Z' ds, exact over the rationals. Mass from the B-rep agrees to 1e-12 in every case.
```
                 exact             our mass          OCCT
cone (no spline assumed)  523.598775598     523.598775598     523.598772
oblique cone              523.598775598     523.598775598     523.598772
cone, 3 sections          523.598775598     523.598775598     523.598774
coaxial                   532.073595780     532.073595780     542.597914   +2.0%
bulge                     602.606090845     602.606090845     629.135298   +4.4%
loft_ends/4               881.578653373     881.578653373     926.498360   +5.1%
loft_ends/36             1143.881052477    1143.881052477    1200.683484   +5.0%
loft_ends/9              1480.192527371    1480.192527371    1529.071525   +3.3%
loft_ends/14             2440.898891760    2440.898891760    2537.573438   +4.0%
loft_ends/20             2048.522605911    2048.522605911    2135.840558   +4.3%
loft_ends/10             1151.883553986    1151.883553986    1261.373229   +9.5%
```
On a cone, where any spline reproducing a line gives one answer, both kernels hold it, so both volumes are right for
their surfaces. OCCT passes through the sections too (slice radius 4.00001 at the middle of `coaxial`) with its own
curve between them: on `bulge` radius 5.203 at z 2.5 and 3.717 at z 7.5 where the natural spline has 5.469 and 2.969.
This proves our B-rep and mass hold the stated definition. It does not prove that definition is Parasolid's: that
takes real lofts with a STEP volume, and none with an apex has been measured here.

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
