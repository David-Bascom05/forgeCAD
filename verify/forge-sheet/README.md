# forge-sheet's fold, proven

research/kernel-audit/audit2.md §2.0 in Lean 4 with Mathlib: the fold of a bend zone
(`ForgeSheet/Fold.lean`), the formula forge-sheet lays a zone out by (`Geo::Round`) and
`forge_sheet::refold` refolds the flat with. Proven, with no `sorry` and the standard axioms only:

- `Zone.iso`: on the neutral layer (radius rn = BA / θ) the fold is an isometry;
- `Zone.start`, `Zone.start_tangent`, `Zone.at_end`, `Zone.end_tangent`, `Zone.angle_at_end`: a zone
  leaves its wall along it and ends turned exactly θ;
- `Zone.neutral_inside`, `Zone.neutral_radius`: physical K, R ≤ rn ≤ R + t;
- `setback`: ρ sin θ / (1 + cos θ) = ρ tan (θ / 2); `deduction`: the length law;
- `Chain.refold`, `Chain.wall_after_zone`: along a chain of walls and zones (the cross-section),
  the flat folded zone by zone is the part placed wall by wall by the zones' turns.

The 3D tree (bend axes not parallel) is the chain's statement in each joint's frame; it is not
formalised separately.

Build: `lake exe cache get && lake build` (Mathlib v4.36.0, prebuilt from cache.mathlib.org).
