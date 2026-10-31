# Loft x loft: novel-math prototypes

Everything here reruns from this folder (numpy only, no sympy needed):

    python3 py/gap.py          # vf6 walls never meet: max elliptic radius 0.879
    python3 py/certify.py      # Bezier-hull SAT certifies all 72 span pairs of vf6, 5432 tests
    python3 py/pencil.py       # exact F(s, sigma); Res_j / q_j^4 is the same for every j
    python3 py/vfx_scan.py     # F's zero set on vfx.fc (core raised 8 mm), lifted to 3D
    python3 py/fcheck.py       # F roots lifted in closed form land on both surfaces (1e-13..1e-9)

`py/loft.py` rebuilds forge-geom's smooth loft (natural spline, chord knots, circles split into two half
arcs) as c(s), X(s), Y(s) cubics per span, for the volute benchmark (`repros/vf6.fc`) and for `vfx.fc`.

The production form is `crates/forge-geom/src/loftmeet.rs` (ideas 1-4, called from forge-bool's
`ssi::surfaces_in` before the tracer, which stays the fallback). On loft-speed 5a0b79f, instructions per
build: vf6 3.65 G -> 1.88 G, vfx 5.07 G -> 2.04 G, tools/bench-parts/volute.fc 10.03 G -> 5.79 G; the
other eight bench parts unchanged; bench_parts and real_part_failures pass and fail exactly as before.

`ring-prototype.patch` is the first Rust prototype, as a diff against `real-parts` (5505efe). Apply it to a
scratch worktree, build forge-cli in release, then:

    NO_RING=1   forgecad build vf6.fc   # today's path
                forgecad build vf6.fc   # + certificate
    RING_MEET=1 forgecad build vf6.fc   # + certificate + closed-form marching
    (SSI_LOG=1 logs each traced or marched pair with its time)

## The ideas, and what happened to each

1. A circle-section span is EXACTLY a rational Bezier patch of bidegree (3,2), with no conversion error.
   S = c(s) + X(s) cos a + Y(s) sin a. Over s in I and a in J (|J| < pi), the control points are
   c_i + X_i cx_j + Y_i sy_j, from the Bernstein points of the cubics over I and the arc's rational
   quadratic over J (weights 1, cos(|J|/2), 1). The patch lies in the convex hull of those 12 points.
   It needs every rail to be an arc with one start and sweep, and no guides; end slopes are fine.
   USED by everything below.
2. Certified disjointness by a separating-axis test on the hulls (x, y, z, both piece normals, the
   centroid difference), splitting the bigger piece. vf6 spends all its loft time proving that walls
   5 mm apart don't meet. This removes it: 101.7 G -> 17.4 G instructions for the whole build. ADOPTION
   CANDIDATE (loft-speed owns production).
3. Ring x quadric (plane, cylinder, cone, sphere): Q(S) w^2 is a polynomial of bidegree (6,4) on a piece.
   Its 7x5 Bernstein coefficients are exact products of the control points. One sign means certified
   empty. Otherwise Newton and march in the face's own (s, a) chart on h = Q(S(s, a)). vf6's 125
   plane/cyl pairs drop from 1.45 s to 11 ms. ADOPTION CANDIDATE.
   Per s, h is a quartic in tan(a/2) (quadratic for a plane), so each branch is a(s) = 2 atan(root) in
   closed form. Branches turn where the discriminant in a vanishes, a univariate polynomial in s.
   That gives exact topology plus closed-form branches; not built.
4. Ring x ring marching in four parameters: minimum-norm Newton on S1(s,a) = S2(sigma,b), seeded by the
   leaves of idea 2's subdivision, marched along n1 x n2. There are no nearest-point solves. vfx: 26
   crossing pairs, 1.7 s -> 26 ms, and the build that fails today is valid. ADOPTION CANDIDATE.
   Totals with 2-4: vf6 101.7 G -> 5.1 G (10.7 s -> 0.50 s); vfx 60.9 G (fails) -> 4.9 G (0.45 s).
5. The exact condition F(s, sigma) (the null line of [X1 Y1 -X2 -Y2] z = c2 - c1, then the Sylvester
   resultant of the two circle quadratics in the line parameter), lifted to the point in closed form.
   It is exact (lifts sit on both surfaces to 1e-13), and its real zeros are the intersection up to
   isolated points. NOT USED for tracing. Lofts that share section planes (the usual shell/core case,
   vf6 and vfx) project both branches to almost the same line sigma ~ s, crossing at every shared knot,
   and F vanishes to high order there (rank-2 M). The (s, sigma) chart is wrong for them, which is why
   idea 4 marches in all four parameters. F stays the exact formula for certifying a pair or counting its
   branches; lean-derive owns its j-free form and its degrees.
6. Considered and dropped: implicitizing a span (degree 12, extraneous sheets from the cubic carried past
   the span, and no better than 4 for tracing); a "sweep-plane" implicit (the section plane through p,
   then the elliptic radius in it). Its zero set is exact and it costs ~300 flops, but it isn't a
   distance, so the octree's 1-Lipschitz pruning breaks. Ideas 2-4 make the octree unnecessary for ring
   pairs anyway. Its one remaining use is as the seed for a near-closed-form project: start s at the
   plane crossing, take the ellipse foot point in that plane, then Newton in s with
   d/ds |S - p|^2 = 2 (S - p) . S_s.

## A note on R17 (cut_cylinder_plane_corner_on_smooth_loft)
On loft-speed 61de2e8 that ignored test passed and the loftmeet patch made it fail (FaceWrongSide); on
5a0b79f it fails with and without the patch. The pass was luck: the tracer's curves on the carried-on
surfaces put vertices on the tool's bottom circle at points 0.07 to 1.2 mm OFF the loft (checked against
py/loft.py; the curves from loftmeet sit on the loft to 2e-4, which is just print rounding). So
R17 is a downstream boolean failure that those stray vertices happened to hide.
