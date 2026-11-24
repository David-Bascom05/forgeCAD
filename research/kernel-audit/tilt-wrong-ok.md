# Two walls crossing at a tiny angle: wrong Ok against the exact reference

Found by TRANSLATE (2026-11-23) while checking D2's possible finding on forge-geom's `plane_plane`.
Repro tests: `verify/forge-exact/fuzz/tests/tilt_repros.rs` (ignored, each with today's kernel
answer); probe: `cargo run --release -p forge-exact-fuzz --example tilt_probe -- L y0 y1`.

## The family

A = the box [0,L]×[0,10]×[0,10]. B = the prism under A's y = 0 wall: the profile (0,-10), (L,-10),
(L,y1), (0,y0), extruded 10. B's top wall and A's y = 0 wall meet at the angle (y1 - y0)/L and
cross where B's wall passes y = 0. The reference (verify/forge-exact, exact rationals, LINEAR =
1e-5) judges each op.

## Results

"solid" means the reference answers a solid with no ambiguity notes.

| case | L | y0, y1 | \|n1×n2\| | `plane_plane` | union | subtract | intersect | reference |
|---|---|---|---|---|---|---|---|---|
| B1 | 1.6e7 | -2e-5, 6e-5 | 5e-12 | Empty (both orders) | ZeroThickness | ZeroThickness | **Ok, invalid body (EdgeUses)** | solid ×3 |
| B1 sibling | 4e6 | -1.5e-5, 1.5e-5 | 7.5e-12 | Empty (both orders) | ZeroThickness | ZeroThickness | **Ok, invalid body** | solid ×3 |
| B2 | 1e5, 3e5, 1e6, 2e6 | -1.2e-5, 1.2e-5 | 2.4e-10 … 1.2e-11 | the crossing line | ZeroThickness | ZeroThickness | **Ok, invalid body** | solid ×3 |
| B2 small | 1e3, 1e4 | -1.2e-5, 1.2e-5 | 2.4e-8, 2.4e-9 | the crossing line | Internal(InconsistentClassification) | same | same | solid ×3 |
| control | 1e3 … 2e6 | ±2e-5 | > ANGULAR | the crossing line | agree | agree | agree | solid ×3 |
| control | 1e4 | -2e-5, 6e-5 | 8e-9 | the crossing line | agree | agree | agree | solid ×3 |
| control | 1.6e7 | ±1e-4 | 1.25e-11 | the crossing line | agree | agree | agree | solid ×3 |
| control | 1.6e7, 1e6 | both below 0 | — | Empty | agree | agree | agree | solid/empty |

The refusals are not `RefusalTrue`: the reference says the exact result is a manifold solid.

## What is known

- **B1 (confirmed wrong Ok, exact reference).** `plane_plane` (forge-geom intersect.rs) takes
  |n1×n2| ≤ ANGULAR = 1e-11 as parallel and decides Empty/Coincident from `p`'s distance to
  `q.frame.origin` alone. That is sound only where the planes' crossing lies within LINEAR/ANGULAR
  = 1e6 of that origin; InputSize allows |coord| ≤ 1.6e7. Here the walls cross at x = 4e6 and
  `plane_plane` says Empty. The verdict also depends on the argument order: with y0 = 0, y1 = 8e-5
  at L = 1.6e7, `plane_plane(A, B)` = Empty and `plane_plane(B, A)` = Coincident (the reference
  calls that case ambiguous). In a local experiment only (not committed), with the parallel test
  at 1e-14 instead of ANGULAR, B1 becomes Internal(SplitCrossed) refusals on all three ops: the
  Empty verdict leads to the wrong Ok there, and changing it alone does not make B1 agree.
- **B2 (confirmed wrong Ok, exact reference): fixed (FLAT-SITES, 2026-11-24).** `plane_plane`
  returns the right line. The fault was in `imprint` (D4): B's bottom face is coplanar with A's,
  so B's edge along its tilted wall (z = 0, from the crossing at x = L/2 to (L, 1.2e-5)) must be
  imprinted into A's bottom face to split off the wedge's triangle. `imprint` decides each stretch
  of the edge by `point_in_face` at 0.5, 0.382 and 0.618 of it; all three lie within LINEAR of A's
  y = 0 edge (gaps 0.46e-5 … 0.74e-5), so the stretch read On, and its fallback
  `on_to_its_vertex` (coverage G5a) skips an end that is a crossing: here the far end is B's
  vertex (L, 1.2e-5), exactly on A's x = L edge. The stretch was taken as running along A's
  boundary and dropped; without the triangles the wedge's faces are classed by flooding from
  outside (Out), leaving the end cap alone (V4 E4 F1, the wrong Ok), refusing union and subtract,
  and at L = 1e3, 1e4 classifying inconsistently. That also explains why failure tracks gap/LINEAR:
  with ends 2·LINEAR apart the midpoint sample is 1·LINEAR off the edge.
  The fix (`off_near_ends`, called from `on_to_its_vertex` only when its vertex test leaves the
  stretch On): two lines at an angle part linearly, so a straight stretch leaving the edge it began
  beside is farthest from it near an end; a point 4·LINEAR in from each end decides, where it is
  off the boundary. Evidence: every B2 case (L = 1e3 … 2e6, all three ops) now agrees with the
  reference (tilt_repros.rs, no longer ignored); the controls still agree; exact-fuzz totals
  unchanged (agree 463, wrong 0); the callgrind band passes (strict total +0.0006%), bits.txt
  identical; forge-bool's `tilted_wedge_thinner_than_two_tolerances` checks validity and the
  exact volumes without the reference.
- **B1's second half (FLAT-SITES, 2026-11-24): `line_plane_t`, the same flaw one level down.**
  With TRANSLATE's plane_plane fix (proof/b1-plane-parallel) B1 refused Internal(SplitCrossed)
  on A's z = 0 face. Cause: `imprint` asks `curve_surface` whether B's tilted bottom edge (z = 0,
  (0, -2e-5) to (1.6e7, 6e-5), 5e-12 rad to A's y = 0 wall) crosses A's y = 0 edge, and forge-geom's
  `line_plane_t` takes |n·dir| ≤ ANGULAR as parallel and decides Apart by the distance at the
  line's origin alone (2e-5): the crossing at x = 4e6 is lost, the whole edge is imprinted Inside,
  its y < 0 part dangles off the face at (0, -2e-5), and the face's split walk fails. Fix:
  `line_in_range` re-decides that branch over the edge's range (`near_parallel_in`, cold): the
  signed distance is affine along the line, so both ends within LINEAR is On, opposite signs
  beyond LINEAR a crossing at the affine zero, one sign beyond LINEAR Apart, else the origin's
  answer. With both fixes and B2's, all seven tilt repros agree with the reference (B1 at 1.6e7
  and its sibling at 4e6 too); this fix alone leaves B1 refusing ZeroThickness (plane_plane's
  Empty), so B1 needs TRANSLATE's fix with it.
- The two cases share the shape: a slot between the walls that closes to zero thickness at the
  crossing, with the walls ~1–2 LINEAR apart at the far ends.

## Open

- A fix for each, and its speed cost (band), for David.
- Whether the regen fuzz and the exact-fuzz sources can reach this shape (none of today's
  families draws walls at such small angles).
