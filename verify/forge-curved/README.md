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
| Delineable, DelineablePoly | `local_graphs`, **`delineable`**, `delineable_poly` | the sweep's delineability (dol-curved's `qsampler_covers` input): over any preconnected parameter set `C` (an interval, an event point, a CAD base cell) with simple, locally bounded zeros, the zeros of `f t ·` are `n` continuous, strictly ordered graphs; for `F : ℝ[t][y]`, a leading coefficient nonzero on `I` gives the bound (Mathlib's Cauchy bound) |
| Sweep | `ZeroGraphs`, `NoCrit`, `Proper`, `CrossesAll`, `GraphLemma`, `FacetCrossing`; **`crossesAll_edges`** | dol-curved's sweep statements for `qsampler_covers`, over any normed space with a linear sweep `ℓ` (ℝ³, or a sweep plane as ℝ²). Proven: the edge half of `CrossesAll` from the curve graphs and no triple point. `Proper` is needed: `xy = 1` against `ℓ = x` has no critical point but is not graphs over a slab containing `x = 0` |
| Rooms | `room_preconnected`, `room_class`, **`res_eq_reg`** | F5's room facts (route B). A room flooded over preconnected pieces whose closures meet in the room at each step is preconnected. A connected room missing a body's frontier is inside or outside it. Rooms that are open, connected, cover off a nowhere-dense set, and are classified at one point each give `Res G keep = reg (combine op A)` (via `reg_eq_of_sdiff`) |
| Chart, FacetSides | `box_pos`, `box_halves`; `side_all`, **`sideLocal_of_data`** | F5 at facet points (route B). At a zero with strict derivative `≠ 0`, arbitrarily small boxes have connected `±` halves reaching the point. From the builder's `FacetData` (rooms open, disjoint, off the faces, covering; each facet connected on a regular carrier, alone near its points; one certified `±` sample per facet), `SideLocal G keep` holds for every `keep` |
| Thom, CAD, CADLevels | `thom`, `merge`, `ed_of_count`, `cyl_R1`, `cad3_R1`, `cad_R1`, `fibreOI_of_counts` | Tier 1's R1 for the CAD with Thom closure (paused under route B, kept as the proof-only reference's R1) |
| WallsCounter | `parallel_walls_not_faithful`, **`lemmaD_parallel_walls_false`** | **`LemmaD` is false for one-direction sweep walls** (design §2.3 Tier 1): see below |
| Local | `LocalGood`, **`faithfulAll_of_local`**, `faithfulAll_of_local'` | R2–R5 are local: continuous carriers, R1, R3 and a local picture at every point (small neighbourhoods meet each cell above the point's in a connected set; each realised one comes near) give `FaithfulAll` |
| Transversal | `TransversalAt`, `transversal_chart`, `localGood'_of_transversal`, **`faithfulAll_of_transversal`** | at a point where the gradients of the `C¹` carriers through it are jointly onto, the inverse function theorem makes the cells near it convex orthant faces; so for transversal carriers `FaithfulAll` is R1 alone |
| Tier0 | `Tier0Spec`, `tier0Sound_of_spec`, **`qRule_tier0Sound`** | a Tier-0 check is sound once it certifies transversality and R1 of the seamed arrangement; for Q1 nothing else is needed |
| FaceLocal | `frontier_of_bd`, `bd_of_frontier`, `edge_iff`, `vert_iff`, **`xNonManifoldF_iff_top`** | **route B**: over the face ledger (A8, W2 wings and sectors), the core's refusal (a row with three boundary wings, or boundary sectors in two fan classes) is the result's topological non-manifoldness, given the builder's local obligations `Obligations` (F5 `SideLocal`; F1–F4 `EdgeLocal`, `VertLocal`). No carriers, no `Faithful`, no R1 |
| Homogeneous | `HomogeneousAt`, `coneCell`, **`localGood_of_homogeneous`**, `ConicalAt`, **`faithfulAll_of_mixed`** | at a point where the carriers through it keep their sign along rays (a cone apex with its axial seams and nappe plane), cells near it are cones, and a radial map onto a small ball shows the local picture holds when the apex's own cone cells are connected; so with every point transversal or conical, `FaithfulAll` is R1 |
| ConeModel | **`modelCell_preconnected`**, **`conicalAt_of_model`** | the seamed cone model `a u₁²+b u₂²−c u₃²`, `u₁`, `u₂`, `u₃` (two axial seams and the nappe plane) has connected sign classes (squares turn each into a convex set); a point modelled on it through a homogeneous homeomorphism is conical. With one axial seam the cells are not connected |
| Faces | `FacesLinked`, `NoClosedPiece`, **`connected_of_faces`**, **`cellsConnected_of_faces`**, `noClosedPiece_full` | R1 without a Jordan theorem: only "at most one component" is needed, since the sides of a curve have different covectors. A cell whose proper faces are connected and linked by incidence, and which has no closed piece, is connected (local connectivity carries one component along every face). Induction on the zeros gives R1. Full cells of ℝ³ have no closed piece |
| Strata | `zeroLocus`, **`noClosedPiece_of_locus`** | a closed piece of a cell is clopen in its zero locus, so it suffices that every component of the locus meets another carrier |
| Linked | `linkedB`, **`facesLinked_of_linkedB`** | `FacesLinked` decided by graph search on the enumerator's list of realised covectors |
| Tier0 | `Tier0Spec`, `qRule_tier0Sound`, `Tier0Data`, `tier0B`, **`qRule_tier0Sound_of_data`** | a Tier-0 check is sound once it certifies transversality and R1; R1 follows from `tier0B` (every realised cell is a point or has linked faces) on `Tier0Data`: the realised list with its covering theorem, every point transversal or conical, the cells certified connected outright, and the locus condition |

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

- **Route B is adopted** (master, 2026-11-23). `xNonManifoldF_iff_top` (FaceLocal) states the
  end-to-end topological reading over the face ledger. What remains are the builder obligations
  `Obligations`:
  - F1, F2, F4 (`EdgeLocal`, `VertLocal`): the tier0 session;
  - F5 (`SideLocal`, room floods): the LemmaD session.

  The route A items below (`Tier0Sound`, `LemmaD`, R1) are paused: the end-to-end path no
  longer needs them.
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
- **`Tier0Sound`**: reduced to the combinatorial check `tier0B` on `Tier0Data`
  (`qRule_tier0Sound_of_data`). What is left is the enumerator's facts: covering of the realised
  list, transversality (algebraic), single-point vertex cells, and that every curve or surface
  component of a lower cell's zero locus meets another carrier. Non-transversal points (cone apex, tangent contact lines) need a local model
  (`LocalGood`) in place of the chart; until then a check requiring transversality sends them to
  Tier 1.
- **`Tier0Sound`** (R1) and **`LemmaD`**, i.e. R1, R2, R4 and R5 for the seamed (or walled) quadric
  arrangements. These are the L–XL proofs of design §4 Q1 L3: 2D Jordan–Schoenflies for
  piecewise-conic curves on simply connected quadric pieces, and the slab Morse argument. They
  need the executable side's seams, check and walls (dol-curved's `curvedLedger`) to be stated
  concretely; `SepRule` takes them as parameters. R3 is done for both tiers (`quadric_sides`).
- **The CAD's R1 (paused under route B).** Proven: `Thom.thom` (Thom's lemma),
  `Thom.merge` / `ed_of_count` (order invariance), `CAD.cyl_R1` (one fibre level),
  `CAD.cad3_R1` / `cad_R1` (three levels, then ℝ³), and `fibreOI_of_counts`. What is left is the
  quadratic algebra: Disc, Res and psc₁ sign invariance ⇒ constant common-root counts, and
  level 2 from `delineable`.
- **`GraphLemma` and `FacetCrossing`** (Sweep.lean), the rest of `qsampler_covers`' input. The
  graph lemma needs the 2D implicit function theorem (Mathlib's `HasStrictFDerivAt.implicitFunction`)
  plus properness for the covering argument.
- **The refusal half of `sep_invariant`.** `TopNonManifold` reads the carriers' strata (which
  points are on one carrier, which cells are edges or vertices), so it is not yet carrier-free.
  The half closes once `TopNonManifold cs R ↔ TopNonManifold (cs ++ seps) R` is proven for
  faithful `cs` and `cs ++ seps`.
- `EndToEndCurved` quantifies over every curved target ledger. Instantiating it needs
  `curved_exact : CurvedTarget (curvedLedger …)` (L2, dol-curved) and `LFits` for its encoding.
