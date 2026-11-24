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
(ρ − R₀)·cos α − z·sin α is convex in the chord's parameter (ρ is the norm of an affine map;
`chord_convex`), zero at both ends, so the chord lies inside; but its deepest point is not its
middle unless the ends are symmetric, and convexity alone gives only sag ≤ 2× the middle's
(`convex_dip`). Today's meshes had chords past the chord there: buntinx2014_tray25 at chord 0.01,
8 triangles up to 1.12·c, all on long chords of a cone's `strips` (the row-by-row path for
revolve and extrude walls, which judged a cone's triangles only at their middles and centroids).

The rule now (tessellate.rs `cone_chord_sags`, `cone_axis_sags`): a chord is judged at its
deepest point, where the slope is zero (ρ′ = tan α·Δz: a root of
p(p − k²)s² + 2q(p − k²)s + (q² − k²c₀) = 0) or where it passes nearest the axis (ρ's kink),
asked only when its middle sags past c/2. A triangle: the distance is convex over its plane, its
slope zero inside only along a line parallel to a ruling, which reaches the edges; so its deepest
point is on an edge or where the axis crosses it (near the apex). Still to prove: that the
candidates hold the minimum (Fermat, and the derivative's algebra), and the triangle's.

**Torus (major R, minor r).** With h the distance from the core circle and F = h² − r² =
|p|² + R² − r² − 2R·ρ: |h − r| = |F|/(h + r), so a point where |h − r| reached c has
|F| ≥ c·(2r − c), and the tests work with F.
- A chord (tessellate.rs `torus_chord_holds`): F is zero at its ends; along a + s·d its third
  derivative is −2R·ρ‴, ρ‴ = −3(αγ − β²)(γs + β)/ρ⁵ (ρ² = α + 2βs + γs² over the shadows on the
  plane square to the axis), at most 6R·(αγ − β²)·max(|β|, |β + γ|)/ρ⁵ with ρ the chord's least;
  F strays from the parabola through its ends and middle by at most that times √3/216 (the
  remainder of interpolation at 0, ½, 1). Accepted when |F(middle)| + stray < c·(2r − c), a
  proven bound, not the exact deepest point. For a chord running round the major circle the
  stray is fourth order in its length (αγ − β² ≈ ρ²l², |β| ≈ l²/2).
- A triangle (`torus_triangle_holds`): F's gradient is 2(p − c(p)), c(p) the core circle's point
  nearest p, so in the triangle's plane (normal n) its slope is zero only where p − c ∥ n: c at
  the angle of n's shadow on the plane square to the axis or the opposite one, p = c + t·n on the
  plane; with the axis's point (ρ's kink), each such point in the triangle is judged exactly.
- A chord that reaches the axis (ρ's kink, where F is not smooth) is not shown within the chord,
  and is split: before this, the bound let it through (a torus with R = 2r: the diameter from
  the outer equator to the inner one opposite has its middle on the torus, and strays r at the
  axis). When the chord's nearest point to the axis lies inside it, αγ − β² is taken as γ·ρ² there
  (exactly, without cancellation). The rule needs only the chord's own least ρ > 0 and c < r, so
  it holds for tori near their axis too; a tube no thicker than the chord stays with the middle's
  test.

**Procedural.** Accepted by the Cells bounds on its bending (tessellate.rs `Cells::within`), so its
statement is relative to those bounds.

## Proof status (verify/forge-tess/lean, standard axioms only)

| Kind | The code's rule | Proven | Open |
|---|---|---|---|
| Plane | none needed | (sag 0) | |
| Cylinder | edge middles | | the projection argument (exact by midpoint) |
| Sphere | edge middles, the foot (filtered) | `sphere_exact`, `sphere_filter`, `foot_far` | |
| Cone | exact chord (filtered), the axis point | `chord_convex`, `convex_dip` (the filter) | the candidates hold the chord's minimum; a triangle's deepest point is on its edges or the axis |
| Torus | chord bound, triangle's stationary points | `stray_bound`, `F_sag`, `interp_remainder` (the chord) | the triangle's candidates are all its interior stationary points; a chord whose shadow lies on a line through the axis but misses it (αγ = β², ρ affine on it, F a parabola: stray 0, exact, unproven) |
| Procedural | the Cells bounds | | all |

All of it over ℝ: the code's f64 is L5's (the float bridge, cdt-l4's). Boundary chords' sag:
MeshCheck's `segments_sag` (to link). The CDT's cover and its settled ⇒ every test passes: cdt-l4.

## To resume (the work stopped here, 2026-11-23, under the owner's rule: proofs follow code
being changed, and the tessellator's changes are done)

Proven and linked to the spec: the sphere (`sphere_within`). Proven at the chord, not yet linked to
`Spec.within`: the cone's chord (`cone_candidates`, `convex_dip`, `chord_convex`) and the torus's
chord (`stray_bound`, `F_sag`). Open:
- the cone's triangle: its deepest point is on an edge or where the axis crosses it (convexity
  over the plane, the slope zero inside only along a line parallel to a ruling);
- the torus's triangle: its interior stationary points are the two `torus_triangle_holds` finds
  (p − c(p) ∥ n), and the plane square to the axis (F constant round the circle ρ = R);
- the torus's collinear-shadow chord (above);
- the cylinder: the projection argument (a chord nearest the axis at its middle);
- procedural faces: the Cells bounds (tessellate.rs `Cells::within`), nothing proven;
- boundary chords: MeshCheck's `segments_sag` (verify/forge-mesh-check, fundamentals' branch;
  restated, not imported, since that crate is not on this branch);
- the CDT's cover, and "settled ⇒ every test passed": cdt-l4's (M3/M4), the seam agreed;
- L5: all of this is over ℝ; the f64 code against it is the float bridge's (cdt-l4).

## Levels next

- L3: the CDT covers each face's drawn region exactly (every constraint edge present, the
  triangles partitioning it), which with the shared edge samples gives (a) and (b); the
  refinement terminates or refuses (`Unsettled` after 200 rounds or past its budget, `TooFine`
  past the ceiling).
- L4: cdt.rs in research/aeneas/STYLE.md form, bits identical, then translated.
- L5: `orient2d` and `incircle` (forge-math predicates) are proven (FvPredicates,
  ForgeMath/Predicates); the CDT's uses are to be tied to those statements.
