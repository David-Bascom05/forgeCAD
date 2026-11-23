# Decide-once for curved inputs: design (2026-11-23)

The owner's rule: "it should all be proven to the same level as boolean is" (research/PROOF-STANDARD.md,
L1–L6). The flat boolean has it end to end: `forge_bool_core.end_to_end_proof : EndToEnd`
(research/kernel-audit/decide-once/aeneas/lean/ForgeBoolCore/EndToEnd.lean, Specs.lean), with no
hypotheses. This note designs the curved path to the same level. It proves nothing large. The new
definitions it relies on are in `verify/forge-exact/ForgeExact/DecideOnce/Curved.lean`, which compiles
in the default build with one small theorem (`covCellK_flat`, listed in Axioms.lean).

Branches read: decide-once-lean @ 24f806a (this branch's base), exact-curved @ 129a695, trace @ 11969fe.
`research/aeneas/STYLE.md` is not on any of these branches. The Aeneas style rules used here are
the ones in research/kernel-audit/decide-once/aeneas/README.md ("What translates"): no struct
holding a borrow, no `break` or `return` inside a loop, no `Ordering`.

## 0. The result in five lines

1. **The core stays unchanged, Rust and Lean.** `forge_bool_core::core` reads covectors and nothing
   else, so its whole L4 proof carries over. The curved end-to-end theorem is the same `CoreSpec`
   followed by a curved `curved_correct` in place of `flat_correct`, exactly as `end_to_end_of`
   does it now.
2. **The flat proof leans on convexity without saying so.** A plane arrangement's sign classes are
   convex, so each is one connected cell. `Cov.le` is closure incidence. A facet meets each edge it
   bounds from one side. The rank of the normals gives the dimension. Curved arrangements break all
   four (§1.3). The design restores them with **separators**: extra carriers (seam planes, and
   sweep walls only when needed) that carry no face. The listed cells are then `Faithful`
   (Curved.lean).
3. **Classification without Jordan.** Each body carries its **sign formula** over its carriers,
   which is how exact-curved's L1 already states curved solids (`QForm`, `quad_boolean`). Then
   `compat`, `witness`, `boundary` and `complete` are evaluations of that formula on covectors,
   proven the way `nef_compat` is. No ray, no point-in-face test, and no curved Jordan–Brouwer.
   B-rep-only inputs (imports, fillets before their own L1) use a flip-flood instead (§2.5b).
4. **Edges come from exact curves for quadrics, and from certified tracing beyond them.** The
   rule is: **zeros are constructed, signs are evaluated.** A cell's zeros come from how it was
   built: a curve of surfaces j and k, a Krawczyk-certified vertex, a seam through an axis. Its
   other signs are interval-filtered evaluations with an exact algebraic fallback. A sign is never
   decided to be zero by evaluating it.
5. **Phases** (§4): Q0, the generalised `Exact` (Lean, small; done, §5.2); Q1, quadrics
   (Lean L1–L3 and a Rust bridge); Q2, tori; Q3, offsets; Q4, splines. The seven decisions are made
   (§5).

## 1. Audit of the flat proof

### 1.1 Lean, verify/forge-exact

"Carries" means it is used unchanged. "Generalises" means the same statement, with planes replaced
by carriers (`Curved.Carrier`). "Replaced" means a curved component takes its place.

| File | Main definitions and theorems | Depends on flatness? | Curved |
|---|---|---|---|
| Topology.lean | `Topology`, `interior`, `closure`, `NowhereDense` | no | carries |
| Regular.lean | `reg`, `combine`, `Decomposition`, `reg_kept`, `reg_eq_of_sdiff` | no | carries |
| Flat.lean | `HForm`, `allPos`, `escape`, `wall_nowhereDense`, `flat_boolean` | yes (half-spaces) | replaced by exact-curved Quad/Basic.lean (`wall_nowhereDense`, `quad_boolean`, `member_off_wall`); lifted from ℚ³ to a model K³ (§2.2) |
| Snap.lean, SnapSym.lean | `tolEq`, `cls`, `cls_spec`, `rep`, `rep_order`, `spreadOk`, `onIn_class`, `vertexAt_on`, `snap_result`, `tolEq_sym` | `tolEq`, `near` and `vertexAt` are plane-specific; `cls`, `rep` and the spread cap are generic | generalises: per-kind `tolEq` (exact-curved QRef/Run.lean `tolSame`, `snapFaces`: spheres, parallel cylinders; cones not yet), the same `cls`/`rep`. Needs an L1 like Snap's |
| RefFacts.lean | `step_keeps_sign`, `step_leaves_plane` | yes | replaced; not needed on the formula route (§2.5a) |
| Jordan.lean (1401 lines) | `contains_const`, `move`, `sweep3`, `ref_eq_exact` | yes: polygons, projected loops, affine sweeps | replaced. Formula route: not needed. B-rep route: the flip-flood (§2.5b) |
| DecideOnce/Closure.lean | `Conn`, `cls`, `cls_spec`, `cls_eq`, `head`, `head_eq`, `dedup` | no | carries |
| DecideOnce/Core.lean | `Sgn`, `Cov`, `Cov.le`, `isFacet`, `plus`, `minus`, `Ledger`, `splitOk`, `topes`, `memb`, `keepTab`, `result`, `around`, `badEdge`, `badVert`, `fanRel`, `joinRel`, `unify`, `core` | no (covectors only) | carries; the core is unchanged |
| DecideOnce/Coverage.lean | `Consistent`, `Valid`, `memb_eq`, `splitOk_of`, `classifyOk_of`, `coverage`, `no_retry` | no | carries: (b) holds for any consistent ledger |
| DecideOnce/Metric.lean | `collapse`, `coreM`, `coverageM`, `correct_off_thin` | no (`correct_off_thin` takes `Exact`) | carries; `correct_off_thin` generalises with `ExactK`. Rule: separator facets never enter the metric decisions (§2.3) |
| DecideOnce/Ledger.lean | `Table.decideAll`, `decideAll_once`, `get?_decideAll`, `Decisions`, `ask`, `ask_build`, closure laws | no | carries. `sideIn_class` uses `Snap.onIn_class`, so it generalises with the curved snap |
| DecideOnce/Correct.lean | `covCell`, `covOf`, `fulls`, `topeDecomp`, `memT`, `Flip`, `IsVert`, `IsEdge` (rank of normals, `det3`), `Exact`, `correct`, `correct_ok`, `XNonManifold`, `zeroThickness_iff` | partly: the cells and `topeDecomp` (via `Flat.wall_nowhereDense`), and `IsEdge`/`IsVert` by the normals' rank | generalises to `ExactK` (Curved.lean). The proofs use `Exact`'s fields and `covCell_length`, so they port with the cells renamed. `IsEdge`/`IsVert` are replaced by `IsEdgeK`/`IsVertK` (dimension from the cell, §1.3 (iv)) |
| DecideOnce/Nef.lean | `nef_compat`, `den_const`, `correct_bool`, `opRule` | `nef_compat` takes half-space formulas | generalises: `qform_compat` (formula over carrier signs), the basis of the formula route |
| DecideOnce/BRep.lean | `brep_compat` | yes (Jordan) | replaced (§2.5) |
| DecideOnce/Witness.lean | `stepBound`, `side_point`, `witness_of_samples` | yes (stepping along a plane normal) | replaced: the formula route needs no point; the B-rep route uses certified boxes (`Quad.member_box`) |
| DecideOnce/Sample.lean | `sampleLedger`, `flipB`, `FlatInput`, `flip_iff`, `s_witness`, `s_boundary`, `s_complete`, `s_low`, `sample_exact`, `Covers`, `sample_correct` | yes | replaced by `curvedLedger` and `curved_exact`. The shape is kept: pieces by a flip on both sides, edges/verts as the listed cells of rank 1/0 below a facet |
| DecideOnce/Sampler.lean | `cand1`, `cand1_ok`, `cover2`, `S2`, `lift`, `coef`, `sampler_covers`, `flat_exact`, `flat_correct` | yes (lines on a plane) | replaced by the curved cell enumerator (§2.4). `cand1`/`minAbove` (midpoints between sorted roots) is reused for the sweep walls' rational midpoints |
| DecideOnce/Frontier.lean | `reg_local`, `far_unchanged`, `frontier_local`, `split_at_region` | no (pure topology) | carries |
| DecideOnce/Local.lean | `LocalCompat`, `ExactIn`, `result_iff_in`, `reg_in`, `zeroThickness_iff_in` | as Correct.lean | generalises (`ExactInK`). On the formula route `LocalCompat` is a theorem: a formula's value is constant on every sign class, region-restricted or not. That would also close flat's one open hypothesis, if flat bodies carry formulas (§5, D2) |
| DecideOnce/Boxes, BoxesExact, Rungs | box ledgers, `box_exact`, `rung_covered`, `rung_exact` | boxes are flat | carry as flat instances. `Rungs.lean` names five rungs whose real inputs are curved; Q1/Q2 make them curved instances (§4) |

### 1.2 Rust and Aeneas

| Piece | Depends on flatness? | Curved |
|---|---|---|
| kernel/forge-bool-core (`core`, `split`, `membership`, `around`, `bad_vert`, `collapse`, `unify`; `ENUM_ZEROS = 12`) | no | **unchanged**. `ENUM_ZEROS` is a speed switch, not a limit: past 12 zeros `around` scans. Separators are chosen so that few of them pass through any one point (§2.3) |
| ForgeBoolCore/Specs.lean: `LedgerWF`, `SidesListed`, `CoreSpec`, `LFits`, `EncodeSpec` | no | carry |
| ForgeBoolCore/Specs.lean: `EndToEnd` (`FlatInput`, `sampleLedger ps ss (sampler ps)`) | yes (statement only) | a new `EndToEndCurved`, proven as `end_to_end_of` is: `CoreSpec`, `coreM_nil`, then `curved_correct` |
| kernel/forge-bool/src/decide_once.rs (`inputs` returns `Skip::NotFlat`; dyadic `DyPlane`, `meet3`; ray parity in `Ref.Face.pierced`) | yes | a new bridge, `decide_once_curved.rs` (§2, §3), written in the Aeneas style. The flat bridge is untouched |

### 1.3 What the flat proof gets from convexity, and the curved counterexamples

These are the four properties the core's combinatorics silently relies on. Each fails for curved
carriers, with a concrete case.

| | Property | Why it holds for planes | Curved failure |
|---|---|---|---|
| (i) | one sign class is one cell (R1) | an intersection of open half-spaces is convex | two cylinders x²+y²=1 and x²+z²=1 (Steinmetz): "inside the first, outside the second" is two components, z > 0 and z < 0. A body holding only one of them is not a function of the sign vector, so `compat` fails |
| (ii) | `Cov.le` is closure incidence (R2) | the face poset of a hyperplane arrangement | a disconnected or non-adjacent class with matching signs. It also fails at one-sided tangencies, where `Cov.le` claims sides that are not there locally |
| (iii) | a facet meets an edge it bounds in one sheet (R4) | facets are convex polygons | two parallel cylinders tangent along a line. The facet "cylinder 1 off cylinder 2" wraps round and reaches the contact line from both sides. Around the edge `around` counts 2 result facets for the union, but there are 4 sheets. The union is non-manifold, and the core as is would answer `ok` |
| (iv) | dimension from the normals' rank | linear algebra | the same contact line: the normals are parallel and the cell is 1-dimensional. Two spheres tangent at a point: rank 1, dimension 0 |

The R3 property (`sides`: both sides of a facet are beside it) holds wherever the facet's surface is
smooth, by the implicit function theorem. It fails only at singular points, and those are vertices
by construction (§2.3). Faces with holes and faces that are annuli (a plate face round a drilled
hole, the band of a cylinder between two caps) are not a failure in themselves. The core never
asks a face to be a disc ("What the theorems do not claim", decide-once-spec.md §6). But they
are where (i)–(iii) break, so the separators target them.

Without (i)–(iv), `ExactK` alone still gives `correct` and a combinatorial `zeroThickness_iff`, as
in flat. But `XNonManifold` would then no longer be the result's non-manifoldness (case (iii)).
That would miss L3's "a refusal happens exactly when L1 has no valid result". So the curved
target is `CurvedTarget := ExactK ∧ Faithful` (Curved.lean), and Q1 proves
`xNonManifold_iff_top`: over faithful cells, `XNonManifold` holds iff the regularised result is
not a 3-manifold with boundary at some listed edge or vertex.

## 2. The curved ledger

### 2.1 Carriers

- **Input surfaces.** Each face's quadric has rational coefficients, written from its stored f64
  frame without normalising. This is exact-curved's `QFace.quadric` (QRef/Faces.lean), canonical
  up to scale (`canon` in QRef/Decide.lean), so faces on one surface share a carrier. Later
  phases: the torus quartic (trace §3.1's form), offsets, and spline patches.
- **The snap** (§6.1, for curved carriers): tolerance-equal spheres and parallel cylinders are
  merged into the least-named member's class (`snapFaces`). Contacts within LINEAR become exact
  sub-tolerance contacts, which the metric ledger collapses or refuses as `ZeroThickness`. L1:
  generalise Snap.lean's `cls`/`rep`/`spreadOk` with a per-kind `tolEq`. Cones must be added
  (QRef/Run.lean: "Cones are not tested for near contacts").
- **Separators** (§2.3). They are planes with rational coefficients, or `Quad` (ℚ(√d))
  coefficients for contact circles of cones. They carry no face. Several solids sharing one
  separator plane collapse to one class by the same canonical form. If a separator coincides
  with an input plane, the two are one class, and that is harmless.
- **The region walls** of decide-once-spec §3a are separators already. The local ledger, F1, F2,
  `far_unchanged` and `frontier_local` carry over.

Ledger width: `nc` = classes = near surfaces + seams + walls. The covector of every cell is a row of
`nc` signs, as now (`LFits.width`).

### 2.2 Points and numbers

The flat spec is over ℚ³: every cell of a rational plane arrangement has rational points. A curved
edge or vertex usually has none, so `Realised` over ℚ³ would drop it. The curved spec is over a
**model of the reals**, K³ (exact-curved Alg/Model.lean `Model K`, or ℝ; this is decision D1).
Curved.lean's definitions are stated for any point type `P` and ordered `K`, and `covCellK_flat`
shows the flat cells are the instance `P = V3`, `K = ℚ`, carriers `Plane.eval`.

The executable's points, all exact:
- **Facet samples.** A rational line L on surface j's sweep line (a wall, or a rational plane
  through a seam), met with j: `t` is a root of a quadratic, a `Quad` number (ℚ(√d)). Every other
  carrier's sign at `L(t)` is a univariate sign at `t` (`Alg.Num.sign`, `sign_spec`).
- **Edge samples.** Curve j∩k met with a rational wall plane: a zero-dimensional system. It is
  represented as a rational univariate representation (RUR): a minimal polynomial `R(u)` of a
  rational linear form, plus coordinates `P_x(u)/Q(u)` and the like. A carrier's sign at the point
  is a univariate sign at the root. This is a new L2 component, `Alg/RUR.lean` (subresultants as
  certificates, checked by reduction identities like `Alg/Sum.lean`'s `redCert`).
- **Vertices.** The same, for three carriers.
- **Side points** (B-rep route only): a rational box certified by `Quad.boxSign` on every carrier
  but j, straddling j. Its sub-boxes on j's two sides give rational points of `φ.plus` and
  `φ.minus` (`Quad.member_box`). No step bound is needed.

### 2.3 Separators: the rule that makes the cells faithful

The rule is decided once per boolean, from the near carriers alone, in two tiers.

**Tier 0: seams.** These are fixed planes per surface, chosen so that every surface piece left in
the region is simply connected. Its singular points become vertices.

| Surface | Seams | What they fix |
|---|---|---|
| plane | none | — |
| cylinder (axis `o + t z`) | the plane through the axis with normal `z × e`, `e` the first of x, y, z not parallel to `z` (rational) | the band between caps (an annulus) becomes 2 discs; a plate's annulus round a hole on this axis is cut too |
| cone | the axial seam as for a cylinder, and the **nappe plane** through the apex ⊥ axis (`QFace.nappe`) | the apex is a vertex (zeros: cone, seam, nappe plane); the two nappes are apart, so a double cone is 2 fans at the apex and is refused, as it must be (exact-curved §3, Theorem 3.4) |
| sphere | the plane through the centre ⊥ e₁ (any fixed rational normal) | two open hemispheres, each a disc |
| torus (Q2) | the axial seam and the equatorial plane | 4 discs; the horn torus's centre is a vertex |
| contact curve (from exact-curved `localTouch`, or trace §7's closed forms) | the plane holding a contact line (two parallel axes span a rational plane), or the plane of a contact circle (⊥ the common axis, at a rational or `Quad` height) | R4 at a tangential contact: each facet now meets the contact edge from one side, so `around` counts the real sheets (case (iii) is refused) |
| isolated tangent point, crossing node of j∩k (exact-curved §2.4, saddle) | a plane through the node, normal fixed by rule (`Quad` coefficients if the node is) | the node becomes a vertex (zeros j, k, separator), apart from its branches |

Coaxial surfaces share their seams, by the canonical form. Seams through one axis meet only on
that axis. So the zero count at any point stays ≈ (surfaces through it) + 2, far from
`ENUM_ZEROS = 12`.

**Tier-0 check** (exact, combinatorial, on the per-surface arrangement graphs of §2.4):
- every facet has one boundary walk on a simply connected surface piece (so it is a disc);
- no edge occurs twice in one facet's walk (the wing condition, R4);
- no vertex occurs twice in one facet's walk (the vertex wing condition, R5, which the q1 session's
  `xNonManifold_iff_top` needs: a facet passing a vertex twice would make one combinatorial fan
  two topological ones);
- the covector map is injective on components: union–find over the arrangement graph gives one
  component per listed class;
- every 3-cell's wedges (round its edges, in their exact cyclic order) are wedge-connected. This
  test is conservative: a spherical shell fails it, and the check then sends the input to Tier 1.

**Tier 1, withdrawn (2026-11-23): parallel walls are not enough.** The first version of Tier 1
put planes `d·x = c` at rational midpoints between d-events. Its Lemma D is false. The LemmaD
session's counterexample is in the Q1 domain:
- a plate `z = 0` with a hole `x² + y² = 1`, its seam `y = 0`, and `d ∝ (2, 3, 6)`;
- the cell {z = 0, outside, y > 0} in a slab next to an event has two components with one
  covector;
- an event-free slab fails too: a level circle crossing another carrier four times.

The reason: within a slab, two sectors can carry the same signs. A plane wall can separate them
only along d. Two branches of one curve can swap the order of roots along a fibre and swap it
back, so the sectors before the first swap and after the second agree on every sign.

**Tier 1 (agreed with the LemmaD session, 2026-11-23): a cylindrical algebraic decomposition
with Thom closure.** It is used only if the Tier-0 check fails. Since A8 (§7), it lives in the
proof-only reference; no runtime builds it.

**Directions.** `u` (the fibres), `w` (the base's fibres) and `d` (level 1): rational, mutually
orthogonal, and the first triple in a fixed list such that every member of `F` below has a nonzero
**constant** `u`-leading coefficient. This excludes planes containing `u`, and quadrics with a
ruling along `u`.

**Separators.** Polynomial carriers that carry no face.
1. **Fibre family `F` (level 3).** `F` holds:
   - the input carriers, the input planes and the Tier-0 seams;
   - for every quadric `q`, its polar plane `∂_u q = 0`. This is a plane: write `q = a s² + b s + c`
     in the `u`-coordinate `s`; then `a` is constant and `b = ∂_u q` is linear.

   `F` is closed under `d/ds`, so by **Thom's lemma** every sign condition on it is an interval
   or a point of a `u`-line.
2. **Base family `B` (level 2).** Polynomials in `(t, r) = (d·x, w·x)`, constant along `u`, so
   cylinders:
   - every `Disc_s(f)` and `Res_s(f, g)` for `f, g ∈ F`;
   - **C1**: every `psc₁(f, g) = a·b′ − a′·b`, the principal subresultant coefficient: a plane,
     the polar plane of the pencil member `a′q − aq′`. It is needed because `Res ≡ 0` on a base cell
     does not fix the order of the roots (`f ∝ g` at a point swaps which roots coincide). For a
     pair containing a plane it is a nonzero constant, and nothing is added;
   - their closure under `∂_r`;
   - **C3**: then replace all of it by a **squarefree basis** (pairwise coprime squarefree
     factors). A discriminant can be a perfect square, and a resultant can have repeated factors;
     without the basis, `Disc_r ≡ 0` and the level-1 events miss them.
3. **Walls (level 1).** The planes `t = e_k` at every root of `Disc_r(b)`, `Res_r(b, b′)` and
   the leading coefficients, over the basis `B`. They have algebraic constants, and their sign at
   a sample is `Alg.Num.cmp`.

Walls and level-2 cylinders are not in `F`, since they are constant along `u` (**C4**).

**Lemma D (agreed).**
- **R1, `cad_R1 : CADSpec … → ∀ X, RealisedK cs X → Connected (covCellK cs X)`.** `CADSpec`
  states the semantic hypotheses:
  - `F` closed under `∂_u` with constant nonzero leading coefficients;
  - `B` sign-invariant on base cells implies `F` delineable and order-invariant over them (proved
    for degree ≤ 2 from the Disc/Res/psc₁ formulas);
  - `B` closed under `∂_r`;
  - walls at all level-1 roots.

  Delineability is parametric over any topological parameter space and any preconnected `C`
  (event points and sections are not open).

  **C2**: on a base cell where `Disc_s(f) ≡ 0`, `f = a(s − ρ)²` with `ρ = −b/2a` continuous (the
  polar's root). No textbook "simple roots" hypothesis is needed for degree 2.

  The executable side establishes `CADSpec` for its separator list.
- **R2–R5**: Tier 1 reruns the graph check on its own arrangement. If it passes, `Faithful` (one
  check-soundness theorem, generic in the arrangement and taking R1 as a hypothesis, shared with
  Tier 0). If not, `Skip::Tier1`, counted. One decided rule (D3): seams; if their check fails, the
  CAD; if its check fails, skip.

Cost: degree-4 cylinders and level-1 polynomials of degree up to 16 make Tier 1 expensive. It runs
only when the Tier-0 check fails, and every gate line it touches must still be measured (D5).
The earlier claim that "Tier 1 adds W walls, about W·S arcs" (§3) is withdrawn with it.

"Tier 1 iff the Tier-0 check fails" is one decided rule, not a retry: each tier gives `Faithful`
or a counted skip, and the answer does not depend on the tier. That independence is the
**separator-invariance theorem**, `sep_invariant`: adding a carrier that carries no face changes
neither the regularised result nor the outcome kind. It also leaves `unify`'s output unchanged
(faces, edges and vertices as point sets), because:
- result facets are never on a separator (both sides have one membership);
- `joinRel` joins the facets a separator split, across its edges;
- separator vertices meet fewer than 3 faces, so they are not output vertices.

**Metric rule.** `thin` and `enclosed` (Metric.lean) are decided on the cells of the input carriers
alone, then given to every sub-cell. A wall passing near a vertex never creates a thin cell of its
own, so no `ZeroThickness` arises that the separator-free geometry would not give.

### 2.4 Cells, sides and incidence

For each near surface j, the bridge builds a planar graph on j:
1. **Curves**: the curves j∩k for every near carrier k, separators included (separator traces on
   a quadric are conics, which are exact and rational or `Quad`).
2. **Vertices**: crossings of two curves on j (three carriers), singular points, and the apex.
3. **Arcs** between vertices: the **edge cells**. **Facets** come from the faces of the graph by
   boundary walks.
4. **Covectors.** The zeros come from the construction: an arc of j∩k has zeros j and k (and any
   separator the arc lies in). The other signs come from one sample point per cell, interval
   first, exact second (§3).
5. **Sides**: `φ.plus` and `φ.minus` are covector operations, as now. R3 gives them as the local
   sides. **Incidence**: R2 makes `Cov.le` the graph's incidence, so the core's `around`,
   `fanRel` and `joinRel` read the true local picture.
6. **Edges and verts** for the ledger: the arcs and vertices below a listed facet, with dimension
   from the construction (`IsEdgeK`, `IsVertK` in Curved.lean).

The covering property (flat's `Covers`, `sampler_covers`) becomes: every realised cell lying in a
carrier is a vertex, arc or face of some surface's graph. This follows from completeness of the
curves (trace (ii), or the QSIC's exactness for quadrics) and of the vertices (every crossing of
two arcs is found: interval exclusion plus Krawczyk existence).

### 2.5 What replaces the flat sampler and Jordan

**(a) The formula route (recommended for Q1).** Each body carries its sign formula
- over its own carriers and the separators it needs (a `BF` as in QRef/BF.lean: `eval2`/`eval3`);
- produced by every decide-once boolean as the history formula (`historyBF`), and by each other
  feature from its own L1, or once per import by an L2 classification;
- not grown without bound: a separator is kept in a body's formula only while two cells that
  differ in its sign alone have different memberships (decided combinatorially).

Then:
- `compat` is `qform_compat`: Nef.lean's `nef_compat`, with literals on any carriers (the formula
  reads only signs);
- the witness `inside(T, i)` is body i's formula on T's restriction to its carriers, so no point
  and no ray is needed;
- a piece is a facet whose two sides evaluate differently: this is flat's `flipB` with the formula
  in place of the ray, so `boundary` and `complete` are as `s_boundary` and `s_complete` with
  `flip_iff` trivial.

What Jordan.lean did for flat, establishing that the B-rep's set is constant on cells, is here
true by the formula's definition. R1 is still needed, because without it a formula over the
*input* carriers alone cannot say "one lobe" (Steinmetz). The separators the body needs are part
of its formula.

**(b) The B-rep route** (imports, and features whose L1 is not yet a formula). The input set `A i`
is the producing feature's L1 set: by induction over the history, every body is the output of a
feature with an L1, and decide-once's own outputs come with `correct_ok`. With a `Faithful`
complex of the region:
1. *pieces*: a 2D flood on each surface, across arcs that are not the face's trim. Trim edges are
   located as arrangement arcs through their exact end vertices (the curved analogue of
   `Snap.vertexAt`).
2. *witness*: a 3D flip-flood from the outer cell (outside every tool, by assumption B of
   curved-l2-proofs.md §1), flipping exactly across body i's pieces. Its correctness:
   membership is constant on each connected full cell (R1, with the boundary inside the wall), and
   flips across a facet iff the facet lies in the boundary (A i is a valid solid). Path
   independence is then a consequence, not a theorem to prove: `A i` exists.

The one new topological lemma is that the flood reaches every listed 3-cell: the dual graph of the
region's cells is connected, the region being a box.

**Singular points:**
- the **cone apex** is a vertex by the seams;
- **poles** are not cells: they are events inside a slab of Tier 1. A pole on a seam is a point of
  an arc where the arc turns. The parametrisation's poles never appear, because the carriers are
  implicit;
- **isolated tangencies** are already vertices (two zeros and a single point: `IsVertK`). The fan
  rule then decides them exactly as exact-curved's Theorems 2.2–2.3 do:
  - two balls touching from outside, union: 2 fans, so `ZeroThickness`;
  - the same, difference: 1 fan, so `ok`;
  - internally tangent shell: 2 fans, so `ZeroThickness`;
- **contact curves and crossing nodes** get their contact separators (§2.3).

### 2.6 Edges from certified intersection tracing

The trace theorem (certified-tracing.md §3.5) supplies exactly the facts that the ledger's
`edges` field and the covering property need:

| Trace guarantee | Ledger use |
|---|---|
| (i) each chain is one connected arc of j∩k, with no branch jump | an edge cell's arc; its zeros j and k are by construction |
| (ii) every zero in the region lies on a chain | completeness: no edge cell missing (`edges` ⇐), and the facets between are faces of the graph |
| (iii) no zero claimed twice | the arcs are distinct cells (`LedgerWF.edges_nodup`) |
| junction Krawczyk on Π_{i+1} | the arc is continuous through the chain |
| Refusal (`Step`, `Junction`, `Uncovered`, `Twice`, `OutOfScope`) | → `Skip::Uncertified`: the old path runs, counted (as `NotFlat` is now) |

Vertices: Krawczyk on the 3×3 system (f_j, f_k, f_l) over a small box proves one exact common zero
(verify/krawczyk `unique_zero`). That zero *is* the vertex, so its three zeros are constructed.
The other carriers' signs come from interval exclusion on the box. A fourth carrier the box
cannot exclude is the only exact fallback (resultant or gcd, `Alg.Num.sign`'s zero certificate).
If it is truly on the vertex, it is a fourth zero, as at a box corner.

**Phase B of trace is a prerequisite** for the singular cases. Trace §7's three kinds:
- the isolated singular point;
- the surface singular point (apex);
- the tangential contact curve.

They map one to one onto the singular rows of §2.3. Trace decides where they are, the exact
reference decides that they are exact, and the contact separators make the cells faithful.

For **quadric pairs**, tracing is not needed. The intersection curve has an exact rational
parametrisation with at most one square root: the pencil method (Levin), as refined by Dupont,
Lazard, Lazard and Petitjean (2008). Its components come from the pencil's real Segre
classification, decided once from `det(A_i + t A_j)`. That is the same pencil exact-curved's
`localTouch` already uses. meet.rs's closed forms are the float filter for it, and the exact
parametrisation is the fallback and the L2. Tracing is for tori (Q2) onwards, and for the
procedural surfaces.

## 3. Speed plan

The owner's rule: nothing gets slower. The flat path's lessons apply:
- decisions are float-filtered, with proven exact fallbacks (`forge_math::exact`, dyadic);
- the ledger is local (§3a), which took `coplanar` from 230 s to below the old path.

**The filters, each with an L5 obligation:**

| Decision | Filter (floats, outward-rounded `forge_math::Interval`, trace §3.1) | Exact fallback (L2) | How often the fallback runs |
|---|---|---|---|
| carrier sign at a cell sample | interval value on the sample's Krawczyk/isolation box excludes 0 | `Alg.Num.sign` at the RUR root | only when the box cannot exclude: a true incidence the construction missed (then it is a zero), or a near miss under LINEAR (the snap handles it first) |
| QSIC topology (pencil's Segre class) | float roots of the quartic `det(A + tB)` with a certified separation | `Alg` isolation of the quartic's roots | at multiple roots, which means a tangency, so rare and wanted |
| vertex existence | Krawczyk 3×3 (trace's code) | resultant system | at singular J only (tangencies), which phase B decides |
| event order (Tier 1) | interval comparison | `Num.cmp` (`cmp_spec`) | equal or near-equal event values |
| sweep-wall placement | float midpoint, rounded to a dyadic | none needed: any rational strictly between works | never |
| point-in-face, ray parity | **none: not computed** (formula route) | — | — |

**Cost per boolean against the old curved path.** These are estimates from the operation counts
below. None is measured yet; measurements on the speed gate are Q1's deliverable.

| Stage | Old curved path (meet.rs, classify, retry rungs) | New | Ratio |
|---|---|---|---|
| intersections | meet.rs closed form per near pair; `trace` where it gives up | the same closed forms, plus a pencil classification (one quartic) and Krawczyk at each vertex. Certify only the pairs meet.rs does not settle (tori Q2 on). Do not run trace's 40× exclusion walk on closed-form pairs | 1.1–1.5× |
| classification | rays, point-in-face on curved patches, per face piece | a formula evaluation per side cell, O(cells × formula size) | ≤ 0.5× |
| retries | CAREFUL, ABSORB_HAIR_ROOTS, CHORDS, FENCES, NO_RUN_ON, n-ary → sequential: each a whole re-run | none (`no_retry`) | on retrying inputs ≪ 1× |
| cells | — | Tier 0 adds ≤ 2 planes per curved surface. Tier 1 (rare) adds W walls cutting the region's S surfaces, about W·S arcs more | Tier 0 ≈ +10–30% of cells; Tier 1 up to 2–4× cells |
| core | — | as flat: hashing covectors of width `nc`, µs–ms | small |
| write | fresh B-rep | fresh B-rep, with edge curves from arcs (QSIC params or certified chains) | ≈ 1× |

**Keeping it from getting slower:**
- the local region (§3a) bounds S;
- seams are shared by coaxial surfaces;
- Tier 1 only on a failed check;
- answers memoised per (carrier pair, region), as trace already does;
- covectors hashed once, as now.

If a gate line regresses, the next lever is the history-incremental ledger: the target's
arrangement is kept from the previous boolean and only the tools' carriers are added. It is
sound because a refinement's covectors restrict to the old ones.

**What must hold before the curved path is switched on** (per surface class, behind a feature like
`decide_once`):
1. **L1–L3** for the class, with no hypotheses but stated size limits: `curved_correct` and
   `xNonManifold_iff_top` for every input in the domain (quadric carriers, formula-carrying bodies).
2. **L4**: `EndToEndCurved` from `CoreSpec`. The bridge's combinatorial parts go through Aeneas:
   graph assembly, boundary walks, the Tier-0 check, covector assembly and the formula
   evaluation. Its numeric parts (Alg/RUR in Rust) are interim: a differential test against the
   Lean L2 with a committed digest, named as interim.
3. **L5**: every float filter proven against verify/ieee754 (the Interval contract, the
   Krawczyk step and the interval quadric sign), with debug-checked preconditions.
4. **L6**: exact-curved's harness (`exact-curved` families, degenerate, lang and band) judged
   stage by stage (ledger against the Lean L2's ledger, then the outcome), with 0 disagreements;
   the reliability scoreboard (985 parts) no worse.
5. **Speed**: tools/speed/gate.sh shows no REGRESSED line with the feature on, and the bits are
   identical wherever the curved path is skipped. Flat inputs never reach the curved bridge.
6. **Skips** (`Uncertified`, `OutOfDomain`, `Ambiguous`, `Budget`, `Tier1`) run the old path,
   counted in the report.
7. **The tier counts** (the master, 2026-11-23). On the exact-curved quadric families and on the
   corpus, report:
   - how often Tier 0 passes;
   - how often Tier 1 runs, and how often it then skips;
   - Tier 1's cost on the gate lines it touches.

   Tier 1 may not make any gate line slower. If it would, those inputs skip (`Skip::Tier1`) rather
   than run it.
   A skip is decided from the inputs alone, so it is one rule.

## 4. Phases

Sizes: S < 1k lines, M 1–4k, L 4–10k, XL > 10k (Lean or Rust, proofs included). "Lean" means a
proof session on verify/forge-exact; "Rust" means a bridge session in the kernel, in the Aeneas style.

### Q0. The generalised core theorems (Lean, S–M)

- `ExactK`, `Faithful` and `CurvedTarget`: **done** (Curved.lean, `covCellK_flat`).
- `correct_k`, `correct_ok_k`, `zeroThickness_iff_k`, `ExactK.consistent`: Correct.lean
  re-parametrised over carriers. `topeDecomp` takes `wallND`, and `flat_correct` becomes an
  instance through `covCellK_flat`.
- `ExactInK` with Local.lean's theorems.
- `qform_compat` (Nef.lean generalised to literals on any carriers).
- `sep_invariant` (§2.3).
- Deliverable: L1/L3 statements for any carriers, with the flat theorems re-derived as an instance.
- Status: done but for the refusal half of `sep_invariant` (§5.2).

### Q1. Quadrics (planes, cylinders, cones, spheres)

| Level | Deliverable | Owner | Size |
|---|---|---|---|
| L1 | the regularised boolean of formula solids over quadrics, over K³. exact-curved's `quad_boolean`/`member_off_wall` are over ℚ³ and lift; `xNonManifold_iff_top` defines the geometric non-manifoldness the refusal must match | Lean (exact-curved) | M |
| L1 | the curved snap rule: `tolEq` per kind, `cls`/`rep`/`spreadOk` reused; cones added | Lean | M |
| L3 | **`faithful_of_seams_check`** (Tier 0) and **Lemma D** (`faithful_of_walls`, Tier 1). The curved analogue of Jordan.lean and the largest proof: 2D Jordan–Schoenflies for piecewise-conic curves on a simply connected quadric piece (Tier 0), and slab products for quadrics (Tier 1) | Lean (with Mathlib if D1) | L–XL |
| L2 | `curvedLedger`: the exact enumerator of §2.4 over `Alg`/`Quad`, with `Alg/RUR.lean` and the exact QSIC parametrisation; `curved_exact : CurvedTarget (curvedLedger …)` | Lean | L |
| L3 | `curved_correct` (Q0's theorems plus `curved_exact`) and refusal iff `xNonManifold_iff_top`; the five curved rungs of Rungs.lean as curved instances | Lean | M |
| L4 | `decide_once_curved.rs`. Combinatorial parts through Aeneas: graph, walks, check, covectors, formula `eval`. `EndToEndCurved` from `CoreSpec`. Numeric parts interim (differential against L2, digest) | Rust and Lean (aeneas) | L |
| L5 | interval quadric sign, pencil root separation, Krawczyk 3×3 vertex: against verify/ieee754 (the Interval contract is on trace) | Lean (ieee) | M |
| L6 | the exact-curved harness, stage by stage, in CI; speed gate; scoreboard | Rust | S–M |

### Q2. Tori (ring, then horn; spindle out of scope until a feature makes one)

| Level | Deliverable | Owner | Size |
|---|---|---|---|
| L1 | quartic carriers in the same `ExactK` (no change); torus seams | Lean | S |
| L2 | torus–quadric and torus–torus curves have degree up to 8 and are not rational in general: certified chains (trace) as the arcs, with exact confirmation of events by `Alg` (resultants of degree ≤ 16); trace phase B as a prerequisite | Lean and Rust (trace) | L |
| L3 | Lemma D for tori: 4 critical points (max, min, 2 saddles); the horn centre a vertex | Lean | M–L |
| L4–L6 | bridge extension; certify.rs already proven at step level; harness families with tori, the CAREFUL rung's case (`a_hole_on_a_rounds_centre_circle_at_any_angle`) | Rust and Lean | M |

### Q3. Offsets

| Level | Deliverable | Owner | Size |
|---|---|---|---|
| L1 | Offsets of quadrics and tori are quadrics and tori, so they reduce to Q1/Q2 exactly. General offsets (`dist(x, spine) − r`, pipes, blends) are not algebraic, so their carriers are **certified-sign functions**: a sign may be evaluated (interval refinement with the Lipschitz bounds of trace §3.6, `f` 1-Lipschitz within the reach) but **never decided zero**. Every zero is constructed (a Krawczyk curve or vertex). A near-zero that is not constructed is the snap's (§6.1) | Lean | M |
| L2 | the same enumerator; signs by certified enclosure. The exact reference for these carriers is the enclosure plus construction, stated as L2 for "decidable away from constructed zeros" | Lean and Rust | L |
| L3 | Lemma D for pipes within the reach (a tube is a product with the spine) | Lean | L |
| L4–L6 | as Q2, plus the fillet/pipe–torus crease cases (rack18u guard_120) | Rust | M |

### Q4. Splines

| Level | Deliverable | Owner | Size |
|---|---|---|---|
| L1 | A spline patch has no global implicit form, so its carrier is a **local carrier**: a signed distance defined on the patch's tubular neighbourhood U (within the reach), with sign constant on each component of U minus the patch. The ledger's region (§3a) becomes per-carrier: a cell's sign for a local carrier is asked only inside its U. This is Frontier.lean's locality, one level down | Lean | L |
| L2 | patches split at their own turning and singular points first (Grandine–Klein); signs by certified foot points; curves by trace with spline enclosures | Rust and Lean | XL |
| L3 | Lemma D for patches within the reach | Lean | L |
| L4–L6 | as before; the FENCES rung's case (`a_rounded_blade_stands_on_a_spline_hub`) | Rust | L |

## 5. Decisions (made by the master, 2026-11-23)

| | Decision | Consequence |
|---|---|---|
| D1 | **Mathlib over ℝ for the curved L1/L3**; the executable reference stays core Lean over `Alg` | R1–R5, Lemma D and `xNonManifold_iff_top` are proven in a Mathlib package importing forge-exact's definitions; the reference is proven against any `Model`, ℝ among them |
| D2 | **A sign-formula sidecar for Q1** | the formula route (§2.5a); `qform_compat` (QForm.lean) is `compat` |
| D3 | **"Tier 1 iff the Tier-0 check fails" is one decided rule** | **The rule, as spec:** the separators of a boolean are the Tier-0 seams of §2.3's table for its near carriers; if and only if the Tier-0 check on the resulting arrangement fails, the Tier-1 walls (direction, events, midpoints as in §2.3) are added to them. Both are functions of the near carriers alone; no other separator is ever added, and nothing is retried |
| D4 | **Switch-on domain: quadric carriers on formula-carrying bodies**; everything else counted and sent to the old path | `Skip::OutOfDomain` beside `Uncertified` and `Ambiguous` |
| D5 | **"Nothing slower" is per gate line**: no user-facing or synthetic line of tools/speed/gate.sh regresses before the switch goes on | switch-on condition 5 of §3 |
| D6 | **Numeric L4 is interim only, with the blocker named** (Aeneas and bignums); the bignum/RUR is written in the Aeneas style (inline limbs, as decide-once is doing for `Big`) so it can reach L4 later | §4 Q1 L4 |
| D7 | **Certified enclosures count as L2 for non-algebraic carriers**, stated as such in PROOF-STANDARD.md | Q3, Q4 |

### 5.1 `qform_compat` against the coordinator's `LocalCompat` (dfe79a63)

The coordinator proved flat's `LocalCompat` for closed flat B-reps classified by ray parity
(`contains_const_local`, JordanLocal.lean, 1395 lines; `local_compat`, `localCompat_of`,
LocalJordan.lean and LocalFinal.lean). `qform_localCompat` (QForm.lean, a few lines over
`qform_compat` and `covCellK_flat`) proves it for bodies given by sign formulas over the carrier
planes, for every region. The two are not interchangeable:
- the formula route is far shorter, and it is the right one where a body carries its formula
  (the curved Q1 domain, D2);
- it does not cover a B-rep that has no formula: computing that formula is the same
  classification Jordan justifies. So the coordinator's proof stays the one for flat B-rep inputs,
  and for the curved B-rep route (§2.5b).

### 5.2 Q0 status

Done, standard axioms only (all in Axioms.lean):
- `Curved.lean`: the definitions, and `covCellK_flat`. `IsEdgeK` is now "two distinct zero
  indices, and more than one point".
- `CorrectK.lean`: `ExactK.consistent`, `result_iff_k`, `correct_k`, `correct_ok_k`,
  `badEdge_iff_k`, `badVert_iff_k`, `zeroThickness_iff_k`. This is Correct.lean for any carriers;
  `topeDecompK` works over any nowhere-dense wall.
- `LocalK.lean`: `ExactInK`, `LocalCompatK`, `result_iff_in_k`, `reg_in_k`,
  `zeroThickness_iff_in_k`, `correct_ok_in_k`. This is Local.lean for any carriers.
- `QForm.lean`:
  - `SForm`, `den_iff`, `den_cell`;
  - `qform_compat`, `qform_localCompat` (flat), `qform_localCompatK` (any carriers);
  - `den_append`, `den_sep`, and `sep_invariant_reg` (the set half of `sep_invariant`).
- `FlatK.lean`: **the flat (c) is an instance of the curved one.**
  - `exactK_of_exact : Exact L ps A → NoScaled ps → ExactK L (ps.map Plane.eval) A`.
  - `flat_correct_k` gives `correct_k` and `zeroThickness_iff_k` for every flat exact ledger.
  - The one real step is that the rank tests are the cell tests on realised cells
    (`isVertK_iff`, `isEdgeK_iff`). It rests on Cramer's identity (`vert_unique`), a step that
    keeps a point in its cell (`line_in_cell`, by `Flat.stepFor`), and `perp_exists`.
  - `NoScaled` (no two carriers are one plane up to scale) is the snap's distinct classes.
    `Exact` does not state it, so it is a named hypothesis here.

- `Faithful` gained **R5** (`vwing`, the wing condition at listed vertices) at the q1 session's
  request. It proved `xNonManifold_iff_top` and `zeroThickness_iff_top` (verify/forge-curved,
  ForgeCurved/Manifold.lean, branch `dol-curved-q1`) from `ExactK`, `Faithful` and R5.

Open:
- The refusal half of `sep_invariant` needs Q1's `xNonManifold_iff_top`: both refusals are then
  the same topological statement.
- ~~`NoScaled` for the sampler's planes~~ done in `Classes.lean`:
  - `classesOf` is the bridge's class rule (`decide_once.rs` `classes`: a plane joins an earlier
    class iff they have one point set, i.e. one `Plane.canon`);
  - `noScaled_classesOf` and `scaled_classesOf`;
  - **`flat_correct_classes`**: for every flat input on nonzero planes, the sampler's ledger over
    the classes is `ExactK`, with `correct_k` and `zeroThickness_iff_k`, and no extra hypothesis.

## 6. Files and references

New on this branch (also `CorrectK.lean`, `LocalK.lean`, `QForm.lean`, `FlatK.lean`, `Classes.lean`, §5.2; Q1 L2 begun: `QLine.lean`, plan in curved-l2-plan.md):
- `verify/forge-exact/ForgeExact/DecideOnce/Curved.lean`: `Carrier`, `sgnK`, `covOfK`, `covCellK`,
  `RealisedK`, `IsVertK`, `IsEdgeK`, `covCell_iff_covOf`, **`covCellK_flat`**, `Connected`,
  `Faithful` (R1 `connected`, R2 `incidence`, R3 `sides`, R4 `wing`), `memTK`, `FlipK`, `FullK`,
  `ExactK`, `Listed`, `CurvedTarget`. It is imported by ForgeExact.lean and Axioms.lean
  (`covCellK_flat`: standard axioms only).

Referenced:
- **Flat.** verify/forge-exact/ForgeExact/DecideOnce/{Core, Coverage, Correct, Ledger, Metric, Nef,
  BRep, Witness, Sample, Sampler, Frontier, Local}.lean; Jordan.lean; Snap.lean; RefFacts.lean.
- **Aeneas.** research/kernel-audit/decide-once/aeneas/lean/ForgeBoolCore/{Specs, EndToEnd,
  Encode}.lean; kernel/forge-bool-core/src/lib.rs (`ENUM_ZEROS`); kernel/forge-bool/src/decide_once.rs
  (`Skip::NotFlat`).
- **exact-curved.** verify/forge-exact/ForgeExact/{Alg/Model, Alg/Real (`Num.sign`, `sign_spec`,
  `cmp_spec`), Alg/Sum (`add_spec`, `mul_spec`, `redCert`), Quad/Basic (`quad_boolean`,
  `member_off_wall`, `wall_nowhereDense`, `box_bound`, `member_box`, `sign_nbhd`), Quad/Cert
  (`pairCheck_spec`, `empty_of_patterns`), QRef/BF (`eval2`, `eval3_sound`), QRef/Octree
  (`empty_of_leaves`, `reg_off_wall`), QRef/Decide (`canon`, `localTouch`, `pinched`, `saddleBad`,
  `apexDecide`, `tripleCurves`, `edgeTouches`, `historyBF`), QRef/Faces (`QFace.quadric`,
  `QFace.nappe`), QRef/Run (`tolSame`, `snapFaces`)}.lean; research/kernel-audit/curved-l2-proofs.md
  §§1–4.
- **trace.** research/kernel-audit/certified-tracing.md §§3.2–3.6 and §7;
  kernel/forge-geom/src/certify.rs; verify/krawczyk/Krawczyk/Step.lean (`unique_zero`);
  kernel/forge-geom/src/meet.rs (`intersect`, `coaxial`).
- **Literature.**
  - Dupont, Lazard, Lazard and Petitjean, "Near-optimal parameterization of the intersection of
    quadrics", I–III, J. Symbolic Comput. 43 (2008): the QSIC with at most one square root.
  - Grandine and Klein (1997): topology first.
  - Basu, Pollack and Roy, *Algorithms in Real Algebraic Geometry*: Thom encodings, RUR, why a
    sign class needs refining to be connected.
  - Berberich, Kerber and Sagraloff, "Exact geometric-topological analysis of algebraic surfaces"
    (SoCG 2008): exact arrangements of quadrics by projection and lifting, the heavier alternative
    to Tier 1.

## 7. A8 for curved: the face arrangement (assessment, 2026-11-23)

The master moved the flat path to A8 (research/decide-once/face-arrangement.md, 2ff687bd on
`decide-once-lean`):
- the cells are **rooms** (components of the region minus the near faces) and **facets** (pieces
  of faces, split only where other faces cross them), with explicit incidence (`Csr` tables);
- the carrier ledger stays the proof's **reference**: a Lean definition that is never run;
- the face ledger is proven a coarsening of it (`Refines`: maps ρ, σ, τ, ν with star bijections),
  and `core_refines` carries every theorem over.

The scheme is combinatorial over cells, so it applies to curved unchanged. The assessment:

**1. Tier-0 failures: most disappear at runtime.** Rooms and facets are connected components by
definition, so R1 holds of the face ledger by construction. The R1 failures behind §2.3 cannot
occur in it:
- the Steinmetz lobes, the plate with a hole, the LemmaD counterexample are disconnected sign
  classes. In the face ledger they are distinct rooms or facets by construction.

What remains is the **wing condition**. A face piece that reaches a crossing curve from both sides
(two cylinders tangent along a line; a facet wrapping round an edge) breaks (e)'s bijection: two
fine facets round the edge map to one face facet. Two remedies:
- (W1) the builder splits such a facet locally, along a seam piece through the edge, only where it
  occurs;
- (W2) `edge_facets` and `vert_facets` rows count **wings** (a facet listed once per side it
  reaches the cell from), with (e)/(v) a bijection onto wings. `around` then counts sheets, and the
  tangent-cylinder union shows 4 and is refused.

**Settled, final (face-arrangement.md at ad97832a; confirmed by the master): W2 with sectors;
cutting allowed.** This supersedes the W1 reply of 9453b06f. A builder that cuts along seams just
produces rows of multiplicity 1 and one sector per facet, which is a valid W2 ledger. So curved
chooses per case: cut along a Tier-0 seam piece where one is natural, emit two wings or several
sectors where not (tangent cylinders). Plain W2 fails at vertices: a pinched facet, reaching a vertex
in two sectors but listed once, can merge fan classes and hide a non-manifold vertex. So:
- `edge_facets[e]` lists a facet once per wing, at most twice (`LedgerWF` checks it);
- `vert_sectors[v]` lists `(facet, bounding edge a, bounding edge b)` per sector, replacing
  `vert_facets`. A facet may appear in several sectors;
- `fanRel` joins two sectors at `v` that share a bounding edge.

(e) is a bijection onto wings. (v) is a surjection from fine facets onto sectors, whose fibres
are connected through fine edges with τ = none. Flat convex facets have multiplicity 1 and one
sector per facet per vertex.

**The curved builder's sectors.**
- **Transversal vertices and cone apexes.** The cyclic order of the arcs round `v` on surface j is
  decided exactly from their tangents `±(∇q_j × ∇q_k)(v)` at the certified vertex sample (QVert):
  angle comparison by half-plane and cross product against a fixed direction, each a `Num.sign`
  at `α`. Consecutive arcs bound a sector, and its facet is a lookup.
- **Tangential vertices** (two arcs tangent at `v`). The order is read on the link instead. Take
  a certified isolating box round `v` (trace phase B; exact-curved decides the touch). The arcs'
  crossings with its faces are edge samples (`proposeEdgeAny`, complete by
  `proposeEdgeAny_complete`). The order along `j ∩ ∂Q` gives the sectors.
- No box within budget: `Skip::Budget`, counted.

The obligation on the curved side is completeness of the arcs at `v`.

**2. Cells: fewer than flat's ratio, and the separators leave the runtime.** The carrier ledger
pays more than flat's ~300 cells, because every seam (and Tier 1's CAD) splits space everywhere.
The face ledger splits a face only where another near face crosses it, plus W1's local splits.
Seams, polar planes, projection cylinders and level-1 walls all live only in the **reference**,
which is never run. So:
- Tier 1's cost leaves the gate entirely. §3's switch-on item 7 then counts the builder's local
  splits, not CAD runs;
- the only runtime geometry is the faces' own crossings: QSIC arcs for quadric pairs, certified
  chains beyond them, vertices.

**3. What the proof still needs.** `EndToEndF`'s conclusions are the reference's:
- `correct_k`, `zeroThickness_iff_k` over `ExactK`;
- and, for topological meaning, `xNonManifold_iff_top` over `Faithful` (q1).

So the reference must still be `ExactK` and `Faithful`. Tier 0 / Tier 1 and Lemma D stay as
proof obligations, now with no runtime cost. One open point: without a runtime check, the
reference's R2–R5 must hold for every input in the domain. Tier 1's CAD does not give them
automatically (§2.3).

Two routes:
- (a) prove them for the reference construction outright;
- (b) state the topological theorem over the face ledger directly. Its rooms and facets are
  components, so R1 is free there, and its incidence is explicit, so R2/R4/R5 become builder
  obligations of the same kind as `Refines`.

Route (b) may make `Faithful` on the reference unnecessary.

The coordinator (9453b06f) answered:
- `EndToEndF` is stated over any reference that has the old core's end-to-end facts, so curved
  instantiates it with the `ExactK` carrier ledger, with no second simulation proof.
- Route (b) would be a new L1 for the face ledger, not a corollary of `Refines`, which transfers
  facts from the reference and not to it. It may still be cheaper than Lemma D on the CAD.

q1 answered (2026-11-23), recommending route (b). Manifold.lean's use of `Faithful` is local
and never uses R1. On the face ledger, five local obligations replace R1–R5:
- **F1, row soundness**: everything in `row(x)` has `x` in its closure.
- **F2, row completeness**: a neighbourhood of `x` has its sheet points in row facets, and at
  vertices its edge points in row edges.
- **F3, separation**: facets are disjoint and relatively open in their union; likewise edges
  within edges ∪ facets.
- **F4, wings**: small neighbourhoods meet each row facet in exactly its listed wings, one
  component per wing. This replaces R4/R5, and is the wing decision made into a statement.
- **F5, sides**: facet points lie in the closure of both rooms. Rooms are open, connected and
  disjoint from the faces, so membership is constant per room. This replaces R3.

No global Jordan is needed. Estimate:
- **(b)** ≈ 1–1.5k lines of generic Lean, plus M-sized builder obligations;
- **(a)** (Tier0Sound and `cad_R1`) ≈ 8–15k lines.

**Ruled by the master (2026-11-23): route (b) approved.** The reference needs only `ExactK`
(done, CorrectK.lean). The Tier-1 CAD stays out of the runtime and off the theorem's path; §2.3
remains as the agreed design of a proof-only reference, should it ever be wanted.

The obligations are now split:
- Tier0 session: F1, F2 and F4;
- LemmaD session: F5, plus the delineability lemmas (Sweep.lean, for `qsampler_covers`);
- q1: the generic face-ledger theorem.

**Wings, final (the master):** W2 with sectors, cutting allowed, as recorded above (9ad6336).

**The generic theorem is done (q1, 6484f71 on `dol-curved-q1`):** `xNonManifoldF_iff_top`,
ForgeCurved/FaceLocal.lean, standard axioms only. F4 reads, per wing (`EdgeLocal`) and per
sector (`VertLocal`): the labelled piece reaches the point, is one connected piece in arbitrarily
small neighbourhoods, and labels are locally constant. A facet cut along a reference seam is just
more facets with one wing each.

**A note for the record on Tier-0 seams (tier0 session, 2026-11-23).** One axial plane per axis
does not make the seam's own cells connected:
- on a cylinder, the seam plane meets it in two lines `x = ±R` with one covector;
- on a cone, it meets each nappe in two rays.

So Tier0Sound would fail for every cylinder and cone. The fix, should Tier 0 ever return to the
theorem's path, is a second axial seam per axis, through the axis with normal `z × (z × e)`
(`x = 0` and `y = 0` for the axis `z`). Under route B the reference needs only `ExactK`, whose
`IsEdgeK` asks no connectedness, so QSep is unchanged for now.

**4. Effect on the work in flight.**
- L2 (this branch: QLine, QEdge, QVert, RootCover, QPropose) is the reference's executable and
  stays. It also becomes the builder's exact fallback: the same certified crossings, computed only
  between faces that meet.
- Formula bodies make a room's membership a formula evaluation on any of its points
  (`qform_compat`), so the curved `room_const` is free on the formula route.
- Rust (dol-curved-rust): the builder targets the v2 ledger (ids, `Csr`), not covectors; Tier 1 is
  never built in Rust.
- Tier0 / LemmaD: their theorems are about the reference and remain needed (§3 above), at no
  runtime cost.
