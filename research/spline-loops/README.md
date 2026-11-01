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
