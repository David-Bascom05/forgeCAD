# forge-mesh audit (cdt.rs, tessellate.rs): read-only

Repo /home/user/forgeCAD, 2026-11-22. Line numbers are for the working tree. No file under kernel/ or interface/ was changed.
Files: kernel/forge-mesh/src/cdt.rs (1670 lines), kernel/forge-mesh/src/tessellate.rs (2413 lines), mesh.rs (88), plus
kernel/forge-topo/src/uv.rs (loop drawing in uv: seams and poles), which tessellate depends on.

## 0. Short answer

The CDT core is clean. It uses exact orient2d, exact `==` for duplicates, no merging and no perturbation, and every
input it cannot handle becomes a typed error (DuplicatePoint, ConstraintsIntersect, ConstraintThroughPoint, BadLoop).
"Delaunay" uses the filter-only `certainly_in_circle`, which flips only when certain. That is a valid design: it
changes triangle shape, never validity (predicates.rs:336-341).

Watertightness is decided once, by construction. Each edge is sampled once (tessellate.rs:806-834). Both faces take
those mesh vertex ids. Boundary chords are CDT constraints, and refinement never splits them (cdt.rs:1031 skips
constrained edges; insert_interior cdt.rs:621-662 drops points on or within eps of a constraint). Orientation is
decided per face from `same_sense`: every path emits triangles counter-clockwise in the layout and flips by
same_sense (tessellate.rs:1072, 1306, 1398).

The case-patching sits in tessellate.rs, as a ladder of retries and fallbacks around the CDT:
- cut_finer: re-mesh the whole body with doubled edges, up to 9 rounds;
- a cone retried without the unrolling;
- DuplicatePoint answered by nudging a uv copy 1e-7, up to 16 tries;
- tangled "mend" mode for file bodies;
- fast paths (strips, proc_strips, zip) that decline to the CDT.

The chord guarantee is only partly a bound. It is a derived bound for the analytic grid cells and for the procedural
`Cells`. Elsewhere it is a test at sampled points, the edge midpoints and centroids, with refinement capped at 200
rounds. Hitting that cap does not raise an error.

## 1. Case-patching (ranked by how much it decides outcomes)

1. **cut_finer retry ladder**, tessellate.rs:700-741.
   - Trigger: `CdtError::ConstraintsIntersect` on any face. Response: re-tessellate the WHOLE body with every edge of
     that face cut 2x finer (`finer[e] *= 2`).
   - Up to 9 rounds, or 3 for `from_file` (line 709).
   - From round 5 on, the tightest circle on the face is no longer doubled (line 726). The comment says this is for
     circles tangent at a shared vertex, whose first chords stay crossed under halving.
   - Round 4 calls `pin_touches` (733-735, 744-792). It samples each pair of the face's edges at 256 points
     (brute-force O(N^2) per pair), then runs 8 alternating closest-point iterations. It inserts a "pin" sample on
     both edges where they come within `2*LINEAR` of each other, more than 1e-6 of the width inside the range.
     The cap is 64 edges.
   - Only ConstraintsIntersect triggers a retry. ConstraintThroughPoint and DuplicatePoint do not.
   - Pins are added per edge, so both neighbouring faces see them and the mesh stays watertight.
   - Tested by forge-model/tests/mesh_quality.rs `crossing_boundary_chords` (fuzz seeds 3, 35, 99, 109, 113).
   - This is the classic "retry finer until it stops failing" patch. The principled alternative is to decide, once
     per face, the sampling at which the boundary polygon stays simple. Equivalently, refine the chords that cross
     until they no longer cross, with a separation bound from the curves' known distance.
2. **Cone: unrolled, then retried in plain uv**, tessellate.rs:883-895. The first attempt unrolls when the face
   reaches the apex (`radius_at(lo.y) <= LINEAR`, 989). On ConstraintsIntersect it tries again with `unroll=false`.
   The comment admits that unrolled, coarse chords of curves meeting at a sharp corner can cross.
3. **DuplicatePoint answered by nudging**, tessellate.rs:1330-1350 and `apart_touching` 2094-2105.
   - When two loops touch at a point, the second uv copy is moved 1e-7 of the way toward the midpoint of its loop
     neighbours, up to 16 tries.
   - The move is 2D only. The 3D vertex stays the same id, so the mesh stays closed.
   - The 1e-7 is a relative magic number.
   - If the nudging fails and the body is from a file, the face goes to tangled mode.
4. **Tangled "mend" mode** for file bodies: tessellate_tolerant 667-680, fallbacks at 1348 and 1354, and
   cdt.rs insert_tangled 538-617.
   - Any CDT error leads to a re-triangulation with crossing chords cut where they cross.
   - The crossing point is a float construction, `pl.lerp(pr, dl/(dl-dr))` (cdt.rs ~566), and it is a new vertex
     the neighbouring face does not have. The result is a T-junction or gap, as documented ("the mesh parts from the
     next face by that little").
   - It has its own budget, `64*chords+1024` (541).
   - It is honest about what it gives up. It is off for bodies made in the kernel.
5. **Fast-path ladder**: strips (2002-2091), then proc_strips (1802-1991), then zip (cdt.rs:772-841), then the CDT.
   - strips handles uv-rectangle faces. For cones it checks the centroid and side midpoints afterwards, and if any
     strays it truncates the vertices it added and returns None (2075-2087).
   - proc_strips returns None as soon as any chord or triangle strays.
   - zip declines slivers with `orient2d <= 1e-5*side^2` (cdt.rs:794), a magic threshold.
   - These are speed paths that check themselves and fall through to the general method. That is acceptable, but
     each one has its own definition of "good enough".
6. **Per-surface constructions** (interior grids and scales):
   - uv_scale (73-82) and interior_steps (85-111). The cylinder and cone row factor `2.0*r*step` is measured, not
     derived (research/mesh-bounds).
   - The sphere and torus `0.9/sqrt2` factor was "chosen by eye on the suite, no measurement kept" (mesh-bounds
     README).
   - Cones have many special cases:
     - the scale's radius floor is LINEAR, or the radius at the face middle (909-914);
     - unrolled layout at the apex;
     - `grid_cone` only when `r0 >= 0.5*r1` (1117-1127);
     - otherwise a row layout with a gap rule (1137-1156);
     - strips get the after-the-fact check.
   - The grid: `reach = 0.25` cell (1527); cells meeting only at a corner are removed iteratively (1572-1590);
     staircase cycles become extra constraint loops (1171-1190).
7. **Seams and poles**:
   - uv.rs:116-200 `draw`. Around singular samples it splits the loop into runs. It then tries all 3^(runs-1)
     period shifts and keeps the one with the smallest |area| whose sign matches same_sense. That search is
     exponential in the number of pole visits.
   - tessellate.rs:1036-1063: points met twice (seams, pinches) are merged only by exact (id, uv bits). That is an
     exact rule, not an epsilon.
   - Triangles with two corners at one pole are dropped (1070, 1392). Legitimate.
   - An unrolled apex run collapses to a single point (1097-1112).
   - `there_and_back` loops are skipped, and on planes back-and-forth runs are popped (931-953).
8. **Chords that would duplicate a segment of another edge**: `on_edge` (1232-1241) and `shared` (1243-1260) force a
   split. The comment gives the reason: a coarse chord across a cylinder that lands on an edge between two other
   faces gives four triangles on one segment. This patches the non-manifold symptom. The cleaner definition would
   be "no inner chord may equal a boundary segment of any face", stated once.
9. **Refinement caps and magic numbers**:
   - cdt.rs:1007: `for _ in 0..200`. When the loop ends without settling, it falls through and returns Ok. Only the
     point budget (`max(4n, 1<<18)`, 1006) and the global time budget give an error. So an unsettled refinement
     returns a mesh that may break the chord bound, with no error.
   - The Delaunay restore after constraint recovery is capped at 64 rounds (501) and then stops silently. That is
     harmless for validity.
   - Constraint recovery budget: `64*(k+4)^2` (449). It gives an error.
   - Walk limit `4*tris+64`, then a scan (121, 138). Fine, as in prior audit §3.
   - eps = 1e-12*size (891) for skipping interior points near vertices or constraints.
   - Super-triangle at 100*size (893), with finite coordinates.
   - tessellate.rs:
     - arc_step floor 1e-4 rad (39);
     - pole noise 1e-8 (125);
     - `1+1e-9` cell slack (508, 1272);
     - 0.98 and 0.9 of the chord (496, 515, 523, 613);
     - 7.2 (608);
     - Gauss-Newton capped at 6 steps in strays_from (1467), which answers "strays" and so stays conservative;
     - MAX_GRID shrink (1021-1029): when node curvature near a pole asks for millions of points, the grid is
       shrunk and refinement is left to make up the difference, up to the 200-round cap;
     - rectangle eps `1e-9*max(..,1.0)` (1730, 2010), with an absolute floor in mixed units.
10. **Edge sampling with no chord bound**: `Parabola | Hyperbola => 64.0` (tessellate.rs:51) whatever q.chord is.
    Boundary chords are never refined, so these edges have no chord guarantee. Intersection edges use the chain's
    stored `sag` (53). They are not capped by max_angle, unlike circles.
11. **FORGE_CDT_DUMP** env debug dump (1356-1363). Harmless.

## 2. Honest refinements and refusals

- CdtError variants (cdt.rs:21-50). Exact duplicate detection by `classify` with `==` (155-169). Constraint through a
  point and constraints crossing are reported, not repaired (393, 434, 418).
- Budgets give typed errors:
  - MAX_EDGE_CUTS (815, 1161);
  - "refinement did not settle" from the point budget (1009);
  - the time budget (457, 552, 957, 961, 1012);
  - tests a_boundary_past_its_budget_fails_cleanly and refinement_that_never_settles_fails.
- tessellate_tolerant mends only `from_file` bodies. On kernel bodies a crossing stays an error (test
  a_crossing_boundary_fails_or_is_drawn_as_it_is).

## 3. Legitimate exact special cases

- `certainly_in_circle` flips only when certain. This is deterministic, terminates, and never affects validity.
- BRIO and Morton order with a fixed seed (683-713); determinism test.
- Exact-bit merging of seam and pinch copies (1036-1063). Pole-collapse triangles dropped.
- Cone unrolled exactly at the apex (developable; Layout::Unrolled 1407-1450, mirrored to keep ccw).
- `Cells` bounds for procedural faces (gap_by, gap_box, shift: 525-583) are derived Taylor bounds on
  `shape_bounds`, checked by mesh_quality `shape_bounds_hold`.
- The analytic grid "cell" skip (1262-1273) follows from arc_step and the sqrt2 split (research/mesh-bounds/check.py).

## 4. What "correct" means here

- The definition in code is `TriMesh::open_edges()` empty (mesh.rs:58-80): each directed edge used exactly once and
  its reverse present. That means closed, manifold, consistently oriented. Also `degenerate_triangles(1e-12) == 0`.
- Watertight across shared edges: decided ONCE, per edge.
  - edge_samples (806-834) holds the mesh ids of the edge's samples.
  - Faces read those positions (967-977) and never re-evaluate them.
  - The `fine` flag (809-812) is per edge: the edge is sampled finer if any face on it is curved both ways.
  - cut_finer and pins also act per edge.
  - Exception: tangled mend mode adds crossing points on one face only.
- Chord deviation: the promise in research/mesh-bounds/README.md is "every triangle within the chord tolerance at its
  centroid and edge midpoints". That is a sampled check, not a sup bound.
  - It is enforced by refinement tests on analytic surfaces with `implicit`, which is the true distance (1281-1288).
  - It is bounded a priori only for grid cells and for procedural `Cells`.
  - It is not bounded for parabola and hyperbola edges, or once the 200-round cap is hit.
  - It is checked by forge-model/tests/mesh_quality.rs over the suite, fuzz histories and fillet cases at several
    chords.
  - The crate's own corner_patch.rs allows `2*q.chord` (line ~55), looser than the promise.
- No self-intersection check in forge-mesh. verify/forge-proof/src/cert.rs `crossings` does one; known.txt line 14
  `certify: Mesh # M8` is an open failure (a body that does not mesh in the proof sweep).
- Orientation: one rule (ccw in the layout, flipped by same_sense) on every path. The uv loop's orientation for pole
  faces comes from the area-sign search in uv.rs.

## 5. History

- `git log -- kernel/forge-mesh` has a single commit, 979190b "Install ForgeCAD launcher and update from latest
  main", a squashed import. No per-fix hashes exist in this repo.
- Commit ids named in research/mesh-bounds (387a882, 2120083, a5c87f0) do not resolve here (`git cat-file` fails).
- Fixes for single parts, as code comments show:
  - cdt test `planar_face_from_the_torture_plate` ("the constraint walk once lost its edge");
  - the `shared`/`on_edge` four-triangles case;
  - circles tangent at a vertex (cut_finer round 5);
  - a hole tangent inside a rim (pin_touches at round 4);
  - the cone retry;
  - a noisy pole (test a_noisy_pole_asks_for_no_steps; line_curvatures 1e-8);
  - fuzz seeds 3/35/99/109/113 for crossing chords.
- Tests in the crate:
  - corner_patch.rs: corner patches and rational quads. Vertices on the surface within 1e-9, chord within
    2*q.chord, facing out.
  - wrap_arc_solid.rs: wrapped disc. Closed, volume short by at most chord*area.
  - varblend.rs: variable blend. Vertices within 1e-9 of the surface, area within 5e-3, facing the natural normal.
  - determinism.rs: one hash, native and wasm32.

## 6. Tests and fuzzing

- `cargo test -p forge-mesh`: lib 22 passed; corner_patch 2; determinism 1; varblend 1; wrap_arc_solid 2; doc 0.
  In total 28 passed, 0 failed, 0 ignored.
- There is no fuzz target for the mesh or the CDT. The `grep -rln cdt` hits are forge-mesh itself,
  verify/forge-bench/examples/cdt_cost.rs (a timing bench) and dwgview (unrelated).
- Fuzz directories (research/*/fuzz, verify/forge-exact/fuzz) do not touch the mesh.
- Mesh failures are exercised only indirectly:
  - forge-lang/src/fuzz.rs:2437 (tessellate in `finish`: open edges, volume <= 0);
  - forge-proof cert.rs;
  - forge-model mesh_quality.
- None of these was run here (heavy release builds). Nothing quick and targeted exists to run.

## Recommendations

1. Make the 200-round refinement cap an error, or at least a flag.
2. Replace cut_finer, pin_touches and the cone retry with a single per-face "simple boundary polygon" sampling pass.
   It would split only the crossing chords, using exact segment-intersection tests, before the CDT is called.
3. Bound parabola and hyperbola edge sampling by curvature.
4. Add a CDT fuzz target: random loops, near-collinear points, tangent holes, refine callbacks. Check against
   `check()` in cdt.rs tests.
5. State the chord definition (sampled at the centroid and edge midpoints, or a sup bound) in one place, and make
   corner_patch use the same 1x chord.
