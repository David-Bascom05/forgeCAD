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

## Open (in the order of the task)

- **`Tier0Sound`**: reduced to `Tier0Spec` (`qRule_tier0Sound`): the check must certify
  transversality (an algebraic fact, dol-curved's side) and R1 (every realised cell connected,
  the Jordan part). Non-transversal points (cone apex, tangent contact lines) need a local model
  (`LocalGood`) in place of the chart; until then a check requiring transversality sends them to
  Tier 1.
- **`Tier0Sound`** (R1) and **`LemmaD`**, i.e. R1, R2, R4 and R5 for the seamed (or walled) quadric
  arrangements. These are the L–XL proofs of design §4 Q1 L3: 2D Jordan–Schoenflies for
  piecewise-conic curves on simply connected quadric pieces, and the slab Morse argument. They
  need the executable side's seams, check and walls (dol-curved's `curvedLedger`) to be stated
  concretely; `SepRule` takes them as parameters. R3 is done for both tiers (`quadric_sides`).
- **The refusal half of `sep_invariant`.** `TopNonManifold` reads the carriers' strata (which
  points are on one carrier, which cells are edges or vertices), so it is not yet carrier-free.
  The half closes once `TopNonManifold cs R ↔ TopNonManifold (cs ++ seps) R` is proven for
  faithful `cs` and `cs ++ seps`.
- `EndToEndCurved` quantifies over every curved target ledger. Instantiating it needs
  `curved_exact : CurvedTarget (curvedLedger …)` (L2, dol-curved) and `LFits` for its encoding.
