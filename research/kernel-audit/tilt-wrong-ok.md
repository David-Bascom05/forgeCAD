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
- **B2 (confirmed wrong Ok, exact reference).** `plane_plane` returns the right line, yet
  intersect returns an invalid body at L = 1e5 … 2e6 (well inside 1e6). **Root cause UNKNOWN**,
  downstream of D2 (trim, classification or assembly: D4–D8 in verify/forge-bool-proof/LEDGER.md).
- The two cases share the shape: a slot between the walls that closes to zero thickness at the
  crossing, with the walls ~1–2 LINEAR apart at the far ends.

## Open

- A fix for each, and its speed cost (band), for David.
- Whether the regen fuzz and the exact-fuzz sources can reach this shape (none of today's
  families draws walls at such small angles).
