# forge-curved: decide-once for curved inputs, over ℝ

A Mathlib package (decision D1 of research/decide-once/curved-design.md §5). It proves theorems
about forge-exact's curved definitions (`ForgeExact.DecideOnce.Curved`, `CorrectK`, `QForm`). It
requires verify/forge-exact by path, as the aeneas project does. The executable reference stays
in forge-exact (core Lean, over `Alg`).

Build: `lake exe cache get && lake build`. Axiom report: `lake env lean ForgeCurved/Axioms.lean`,
checked in CI by .github/workflows/forge-curved.yml. Standard axioms only, no `sorry`. The
package also requires the aeneas lean project (research/kernel-audit/decide-once/aeneas/lean),
for `EndToEndCurved`. That project builds in about 2 minutes on the shared Mathlib cache.

## Proven

| File | Theorems | What |
|---|---|---|
| Bridge | `toForge`, `closure_iff`, `interior_iff`, `nowhereDense_iff`, `connected_iff` | a Mathlib topology is a forge `Topology`; `Curved.Connected` is `IsPreconnected` |
| Cells | `eventually_le`, `full_open`, `full_above_facet`, `facet_eq_of_le`, `le_of_mem_closure` | continuous carriers: signs persist near a point; full cells are open; the cells above a facet are it and its sides; closure ⇒ `Cov.le` |
| Boundary | `R_eq`, `not_frontier_of_full`, `frontier_iff` | the regularised result `R` is the closure of the kept full cells; a facet point is on `frontier R` iff the facet is an exact boundary facet (uses R3) |
| Manifold | **`xNonManifold_iff_top`**, `zeroThickness_iff_top` | under `ExactK` + `Faithful` (R1–R5): the core refuses `ZeroThickness` iff `R` has three local sheets at an edge point or two local fans at a vertex (`TopNonManifold`) |
| Tangent | `unseamed_no_refusal`, `unseamed_not_faithful`, `union_three_sheets`, `seamed_refuses` | the tangent-cylinder test (design §1.3 (iii)); see below |
| EndToEnd | `core_spec_proof`, `curved_correct`, `curved_correct_top`, **`end_to_end_curved : EndToEndCurved`** | the Rust core on any curved target ledger of quadric carriers and formula bodies: `CoreSpec`, `coreM_nil`, `curved_correct`, as `end_to_end_of` is for flat |
| Sides | `sides_of_fderiv`, `sides_of_seamed`, `quadric_sides` | **R3 for every facet** of differentiable carriers whose singular zeros lie on another carrier (`SingularSeamed`), so for quadrics with the apex seamed |
| Separators | `SepRule.separators`, `FaithfulAll.listed`, `faithful_of_rule` | the D3 rule as one function; `Tier0Sound` and `LemmaD` stated; both together give faithful cells whichever tier the rule takes |
| Delineable, DelineablePoly | `local_graphs`, **`delineable`**, `delineable_poly` | the sweep's delineability (dol-curved's `qsampler_covers` input): on an open preconnected `I` with simple, locally bounded zeros, the zeros of `f t ·` are `n` continuous, strictly ordered graphs; for `F : ℝ[t][y]`, a leading coefficient nonzero on `I` gives the bound (Mathlib's Cauchy bound) |
| WallsCounter | `parallel_walls_not_faithful`, **`lemmaD_parallel_walls_false`** | **`LemmaD` is false for one-direction sweep walls** (design §2.3 Tier 1): see below |
| Local | `LocalGood`, **`faithfulAll_of_local`**, `faithfulAll_of_local'` | R2–R5 are local: continuous carriers, R1, R3 and a local picture at every point (small neighbourhoods meet each cell above the point's in a connected set; each realised one comes near) give `FaithfulAll` |
| Transversal | `TransversalAt`, `transversal_chart`, `localGood'_of_transversal`, **`faithfulAll_of_transversal`** | at a point where the gradients of the `C¹` carriers through it are jointly onto, the inverse function theorem makes the cells near it convex orthant faces; so for transversal carriers `FaithfulAll` is R1 alone |
| Tier0 | `Tier0Spec`, `tier0Sound_of_spec`, **`qRule_tier0Sound`** | a Tier-0 check is sound once it certifies transversality and R1 of the seamed arrangement; for Q1 nothing else is needed |

**R5, the vertex wing condition** (`Faithful.vwing`, added to Curved.lean by dol-curved at 1abc5b0
for this theorem): R4 at vertices. Without it, a facet whose boundary walk passes a vertex
twice makes one covector fan into two topological fans. Its Tier-0 check is "no vertex occurs
twice in one facet's boundary walk".

**The tangent cylinders** `(x∓1)² + y² = 1`, union:
- Without a seam, the realised facets above the contact edge are two, so every exact ledger
  answers without a refusal. Meanwhile the union has three (in fact four) local sheets at the
  axis, and R4 fails there.
- With the contact seam `y = 0`, every exact ledger refuses.

So the hypotheses of `xNonManifold_iff_top` are not vacuous: they fail exactly where the bare
covector count is wrong.

## Open

- **Which route gives the end-to-end `Faithful`** (decision pending with the master, 2026-11-23):
  - Route A: Faithful on the carrier reference, as `Tier0Sound` + `LemmaD`.
  - Route B: restate `xNonManifold_iff_top` over the face ledger (A8) with local row obligations
    F1–F5. Here R1 is not needed: `xNonManifold_iff_top` never uses it.
  - q1 recommends B.
- **`Tier0Sound`**: reduced to `Tier0Spec` (`qRule_tier0Sound`). The check must certify two things:
  - transversality: an algebraic fact, on dol-curved's side;
  - R1: every realised cell connected. This is the Jordan part.

  Non-transversal points (cone apex, tangent contact lines) need a local model (`LocalGood`) in
  place of the chart.
- **`LemmaD` needs the new Tier 1.** As first designed, with parallel planes `d·x = c` between
  consecutive events, it is false: `lemmaD_parallel_walls_false`.
  - The input is a plate with a hole: `z = 0`, the cylinder `x² + y² = 1`, and its seam `y = 0`,
    with `d ∝ (2, 3, 6)`.
  - A wall level between the vertex event `2x + 3y = 2` and the circle's critical value `√13`
    leaves the facet `{z = 0, outside, y > 0}` below it with two components, split by `3x = 2y`.
  - The replacement is a CAD with Thom closure (curved-design.md §2.3, d223056): polar planes,
    projection cylinders and level-1 walls, else `Skip::Tier1`. `Separators.lean` will take it
    (walls → cad, with a third outcome) once the route is decided.
  - `delineable` is the 1D step.
- **The refusal half of `sep_invariant`.** `TopNonManifold` reads the carriers' strata, so it is
  not yet carrier-free. It closes once `TopNonManifold cs R ↔ TopNonManifold (cs ++ seps) R` is
  proven for faithful `cs` and `cs ++ seps`. `faithfulAll_of_local` may help.
- `EndToEndCurved` quantifies over every curved target ledger. Instantiating it needs two things:
  - `curved_exact : CurvedTarget (curvedLedger …)` (L2, dol-curved);
  - `LFits` for its encoding.
