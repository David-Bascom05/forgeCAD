# Tessellation to the boolean's proof level: the plan, and what the refinement rule guarantees

The master's brief (2026-11-23): L1 (spec in Lean), L3 (the CDT and the refinement rule meet it,
refusing otherwise), L4 (Aeneas on the CDT core), L5 (the proven predicates tied in). This note is
the working derivation behind the Lean statements; each claim here becomes a lemma there.

## L1, the spec

For a valid body B and a chord c > 0, the mesh M of B at chord c:
- (a) is closed and consistently oriented: `openEdges M = []` (verify/forge-mesh-check,
  `closed_of_open_edges`), and one fan round each vertex (the `pinch` is not a mesh of a valid
  body), the mesh image of check's manifold rule;
- (b) shares its edges: each body edge's cut points are the same vertices in both faces;
- (c) puts each vertex of face f on f's surface S_f;
- (d) sags at most c: every point of every triangle of f lies within c of S_f.

## What the refinement accepts, per surface kind

A triangle of an analytic face is accepted when its three edge midpoints and (on a surface that
curves both ways, a cone, or a procedural one) its centroid lie within c of the surface,
measured by `implicit`, the surface's true signed distance (forge-geom surface.rs:859). Grid
cells, whose steps come from the exact radii, are accepted by their own rule (tessellate.rs
`interior_steps`; MeshCheck's `segments_sag`).

**Plane.** Sag 0: every triangle lies in it.

**Cylinder, radius r.** Project along the axis: the projection is affine, so a triangle goes
to a triangle inscribed in the circle of radius r, and a point's distance from the axis is its
image's distance from the centre. A chord of the circle comes closest to the centre at its
middle, the image of the edge's midpoint. So unless the image holds the centre (the three
vertices spanning more than half a turn, a sag of r), the deepest point of the triangle is an
edge midpoint, and the midpoint tests are exact: accepted ⇒ sag ≤ c.

**Sphere, radius r.** The deepest point of the triangle is the point nearest the centre: the
circumcentre (the foot of the centre on the triangle's plane), when it lies in the triangle,
else an edge midpoint (each edge's ends are equidistant from the centre). With d the
circumcentre's distance from the centre and R the circumradius (r² = d² + R²), sag(cc) = r − d.
For an acute triangle the circumcentre is inside, and:
- the longest edge l ≥ √3·R, so its midpoint test gives c ≥ r − √(r² − l²/4) ≥ l²/(8r) ≥ 3R²/(8r);
- the centroid lies within R/3 of the circumcentre (Euler line: |OH|² = 9R² − Σa² < R² when
  acute), so sag(cc) − sag(g) ≤ (R/3)²/(2d);
- hence sag(cc) ≤ c + R²/(18d) ≤ c·(1 + 4r/(27d)), about 1.15·c for a triangle small beside
  the sphere.

So the rule as it stood guaranteed sag ≤ (1 + 4r/(27d))·c on a sphere, not ≤ c. Sampled
(200 000 random small triangles on the unit sphere, each against the largest of its own midpoint
and centroid sags): the deepest point sags up to 1.070 times what the tests see; the bound's
1.148 is not tight. The meshes made then stayed under the chord all the same (a ball at chords
0.1 to 0.001: worst 0.965 to 0.995 of it; a ball cut by a box, 0.3 to 0.003: worst 0.768 to
0.991), the grid cells (steps from the exact radius) covering most of each face: a gap in what
the rule guaranteed, not in what it had made.

**The rule now (the master's choice B): the deepest point, exactly.** A sphere's triangle whose
centroid passes is also judged at the centre's foot on its plane, when the foot lies in it
(tessellate.rs `foot_sags`). It is the triangle's deepest point there; where the foot lies
outside, the deepest points are on the edges, whose middles the edge tests judge. The foot is
looked at only where the centroid's sag is within what the foot can add to it: with the edges'
middles within c, at most 8rc/(9d), d ≥ √(r² − 8rc/3). Proven (verify/forge-tess/lean,
`Sag.sphere_filter`), in plain algebra:
- R² = Σ λᵢλⱼ lᵢⱼ² for the foot f = Σ λᵢxᵢ (`circum_sq`);
- |g − f|² = R² − Σ l²/9 (`centroid_sq`), so with f inside (λ ≥ 0, Σ λᵢλⱼ ≤ 1/3) at most
  (2/9)·L², L the longest side (`centroid_near`);
- a side whose middle sags at most c is at most √(8rc) long (`chord_short`);
- the foot is deeper than the centroid by at most |g − f|²/(2d) (`foot_over`).

No mesh changed: the foot rejected no triangle the centroid had accepted, over the release suites
of forge-mesh, forge-model, forge-lang, forge-step and forge-topo (1102 tests) and the 2592 STEP
files of the corpus.

Still to prove for the sphere: that with the foot outside the triangle, its deepest point is on
an edge (the segment from the foot to any point of it crosses an edge, nearer the foot).

**Cone.** Along a chord between two points of the cone, the signed distance
(ρ − R₀)·cos α − z·sin α is convex in the chord's parameter (ρ is the norm of an affine map), zero
at both ends, so the chord lies inside; but its deepest point is not its middle unless the ends
are symmetric, and convexity alone gives only sag ≤ 2× the middle's. Both the edge and the
triangle tests need the exact deepest point: a convex minimisation with a closed form (the slope's
zero solves a quadratic), and for a triangle, the axis's foot when the triangle's image holds the
axis, else its edges.

**Torus.** The distance along a chord is not convex, and its exact minimum is a root of a
polynomial of high degree. A sound test there is a proven bound (accepting only what the bound
shows within c), not a closed form.

**Procedural.** Accepted by the Cells bounds on its bending (tessellate.rs `Cells::within`), so its
statement is relative to those bounds.

## Levels next

- L3: the CDT covers each face's drawn region exactly (every constraint edge present, the
  triangles partitioning it), which with the shared edge samples gives (a) and (b); the
  refinement terminates or refuses (`Unsettled` after 200 rounds or past its budget, `TooFine`
  past the ceiling).
- L4: cdt.rs in research/aeneas/STYLE.md form, bits identical, then translated.
- L5: `orient2d` and `incircle` (forge-math predicates) are proven (FvPredicates,
  ForgeMath/Predicates); the CDT's uses are to be tied to those statements.
