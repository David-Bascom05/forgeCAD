import ForgeExact.DecideOnce.Correct

/-!
# Decide-once for curved inputs: the new definitions (design: research/decide-once/curved-design.md)

The combinatorial core (`Core.lean`) reads cells by covector and nothing else, so it is kept
unchanged. What changes is where the covectors come from. Here are the definitions that replace
`Correct.lean`'s planes, stated for any carriers over any ordered field:

* a **carrier** is a function `P → K` on the points, its zero set the surface: a plane, quadric,
  torus or offset polynomial, or a *separator* (a seam or a sweep wall, design §2.3) that carries no
  face. `P` is `K × K × K` in the curved spec. Lower-dimensional curved cells (an edge on two
  quadrics) have no rational points, so the spec is over a model of the reals (Alg/Model.lean on
  exact-curved), not over ℚ³ as the flat spec is;
* `covCellK`, `RealisedK`: the cell with covector `X`, as for planes (`covCellK_flat`: the flat
  cells are the instance with the planes' `eval`);
* `IsVertK`, `IsEdgeK`: dimension decided from the cell itself (one point; two or more zeros and
  more than one point), since the rank of the normals is wrong at tangencies (two cylinders
  tangent along a line have parallel normals along an edge);
* `Faithful`: what a plane arrangement has for free (its cells are convex) and a curved one has
  only after the separators: every listed cell is connected, `Cov.le` is closure incidence, each
  facet has both of its sides beside it, and a facet meets an incident edge (R4) or vertex (R5)
  in one sheet (the wing conditions; R4 fails for two tangent cylinders without a seam);
* `ExactK`: `Exact` with these cells. Every field reads as before; `Correct.lean`'s proofs use
  the fields, not the planes, except through `topeDecomp` (the nowhere-dense wall), which becomes
  a hypothesis on the carriers (`wallND`).

No theorem here is large; this file fixes the statements the phases prove.
-/

namespace ForgeExact.DecideOnce.Curved

open ForgeExact ForgeExact.DecideOnce Topology

/-- A carrier: a function on the points whose zero set is the surface. -/
abbrev Carrier (P K : Type) := P → K

variable {P K : Type} [Zero K] [LT K] [DecidableRel (α := K) (· < ·)]

/-- The side of a carrier a value gives. -/
def sgnK (q : K) : Sgn := if 0 < q then .pos else if q < 0 then .neg else .zero

/-- A point's covector. -/
def covOfK (cs : List (Carrier P K)) (x : P) : Cov := cs.map fun c => sgnK (c x)

/-- The cell with covector `X`. -/
def covCellK (cs : List (Carrier P K)) (X : Cov) : Pred P := fun x => covOfK cs x = X

/-- The wall: the points on some carrier. -/
def wallK (cs : List (Carrier P K)) : Pred P := fun x => ∃ c ∈ cs, sgnK (c x) = .zero

/-- A cell with a point. -/
def RealisedK (cs : List (Carrier P K)) (X : Cov) : Prop := ∃ x, covCellK cs X x

/-- A vertex cell: a single point. -/
def IsVertK (cs : List (Carrier P K)) (X : Cov) : Prop :=
  RealisedK cs X ∧ ∀ x y, covCellK cs X x → covCellK cs X y → x = y

/-- An edge cell: on two or more carriers, and more than one point. Under `Faithful` (connected
cells) and with no two carriers sharing a component (the snap merges equal surfaces), this is a
curve. For planes it is the rank test `IsEdge` (FlatK.lean, `isEdgeK_iff`). -/
def IsEdgeK (cs : List (Carrier P K)) (X : Cov) : Prop :=
  RealisedK cs X ∧ (∃ a b : Nat, a ≠ b ∧ X[a]? = some Sgn.zero ∧ X[b]? = some Sgn.zero) ∧ ¬IsVertK cs X

/-! ## The flat cells are an instance -/

theorem covCell_iff_covOf : ∀ (ps : List Ref.Plane) (X : Cov) (x : Ref.V3),
    covCell ps X x ↔ covOf ps x = X
  | [], [], _ => by simp [covCell, covOf]
  | [], _ :: _, _ => by simp [covCell, covOf]
  | _ :: _, [], _ => by simp [covCell, covOf]
  | p :: ps, s :: X, x => by
    have ih := covCell_iff_covOf ps X x
    simp only [covCell, covOf, List.map_cons, List.cons.injEq] at ih ⊢
    rw [ih]

/-- **The flat arrangement is the instance with the planes' values as carriers.** -/
theorem covCellK_flat (ps : List Ref.Plane) (X : Cov) (x : Ref.V3) :
    covCellK (ps.map fun p => p.eval) X x ↔ covCell ps X x := by
  rw [covCell_iff_covOf]
  simp only [covCellK, covOfK, covOf, List.map_map, Function.comp_def]
  rfl

/-! ## Faithfulness -/

/-- Connected, as a subspace: not split by two open sets. -/
def Connected [Topology P] (S : Pred P) : Prop :=
  ∀ U V : Pred P, IsOpen U → IsOpen V → S.Sub (U.union V) →
    (∃ x, S x ∧ U x) → (∃ x, S x ∧ V x) → ∃ x, S x ∧ U x ∧ V x

/-- **Faithful**: the listed cells behave as a plane arrangement's do. This is the curved
obligation the flat proof never states, because convexity gives it. -/
structure Faithful [Topology P] (cs : List (Carrier P K)) (Listed : Cov → Prop) : Prop where
  /-- R1: every listed cell is connected (one sign class, one cell) -/
  connected : ∀ X, Listed X → RealisedK cs X → Connected (covCellK cs X)
  /-- R2: `Cov.le` is closure incidence among listed cells -/
  incidence : ∀ X Y, Listed X → Listed Y → RealisedK cs X → RealisedK cs Y →
    (Cov.le X Y = true ↔ (covCellK cs X).Sub (closure (covCellK cs Y)))
  /-- R3: both sides of a listed facet are beside it, at every point of it -/
  sides : ∀ φ, Listed φ → φ.isFacet = true → ∀ x, covCellK cs φ x →
    closure (covCellK cs φ.plus) x ∧ closure (covCellK cs φ.minus) x
  /-- R4, the wing condition: a listed facet meets a small neighbourhood of any point of an
  incident listed edge in one connected sheet -/
  wing : ∀ e φ, Listed e → Listed φ → IsEdgeK cs e → φ.isFacet = true → Cov.le e φ = true →
    ∀ x, covCellK cs e x → ∀ W : Pred P, IsOpen W → W x →
      ∃ U : Pred P, IsOpen U ∧ U.Sub W ∧ U x ∧ Connected ((covCellK cs φ).inter U)
  /-- R5, the vertex wing condition: the same at any point of an incident listed vertex (a facet
  whose boundary walk passed a vertex twice would make one combinatorial fan two topological ones;
  needed by the q1 session's `xNonManifold_iff_top`) -/
  vwing : ∀ v φ, Listed v → Listed φ → IsVertK cs v → φ.isFacet = true → Cov.le v φ = true →
    ∀ x, covCellK cs v x → ∀ W : Pred P, IsOpen W → W x →
      ∃ U : Pred P, IsOpen U ∧ U.Sub W ∧ U x ∧ Connected ((covCellK cs φ).inter U)

/-! ## The curved exact ledger -/

/-- Membership of the solid `A i` in the cell `T`. -/
def memTK (cs : List (Carrier P K)) (A : Nat → Pred P) (T : Cov) (i : Nat) : Prop :=
  ∃ x, covCellK cs T x ∧ A i x

/-- Body `i`'s membership flips across the facet `φ`. -/
def FlipK (cs : List (Carrier P K)) (A : Nat → Pred P) (φ : Cov) (i : Nat) : Prop :=
  ¬(memTK cs A φ.plus i ↔ memTK cs A φ.minus i)

/-- The full covectors realised: the arrangement's 3-cells. -/
def FullK (cs : List (Carrier P K)) (T : Cov) : Prop := RealisedK cs T ∧ Sgn.zero ∉ T

/-- **An exact curved ledger.** `Exact` (Correct.lean) field for field, over the carriers `cs`
(input surfaces, then separators), with `wallND` in place of nonzero normals. -/
structure ExactK [Topology P] (L : Ledger) (cs : List (Carrier P K)) (A : Nat → Pred P) : Prop where
  /-- the wall (the carriers' zero sets) is nowhere dense: no carrier vanishes on an open set -/
  wallND : NowhereDense (wallK cs)
  compat : ∀ T, FullK cs T → ∀ i, (covCellK cs T).Sub (A i) ∨ ∀ x, covCellK cs T x → ¬A i x
  pieceFacet : ∀ p ∈ L.pieces, p.1.isFacet = true ∧ p.1.zeroIdx = L.clsOf p.2 ∧ L.bodyOf p.2 < L.nb
  oneFace : ∀ p ∈ L.pieces, ∀ q ∈ L.pieces, q.1 = p.1 → L.bodyOf q.2 = L.bodyOf p.2 → q.2 = p.2
  pieceReal : ∀ p ∈ L.pieces, RealisedK cs p.1
  witness : ∀ T ∈ L.topes, ∀ i, i < L.nb → (L.inside.contains (T, i) = true ↔ memTK cs A T i)
  boundary : ∀ φ ∈ L.facets, ∀ i, i < L.nb → (FlipK cs A φ i ↔ L.isPieceOf i φ = true)
  complete : ∀ φ, RealisedK cs φ → φ.isFacet = true → ∀ i, i < L.nb → FlipK cs A φ i → φ ∈ L.facets
  edges : ∀ e, e ∈ L.edges ↔ RealisedK cs e ∧ IsEdgeK cs e ∧ ∃ φ ∈ L.facets, Cov.le e φ = true
  verts : ∀ v, v ∈ L.verts ↔ RealisedK cs v ∧ IsVertK cs v ∧ ∃ φ ∈ L.facets, Cov.le v φ = true

/-- The cells a ledger lists: its facets, their sides, its edges and verts. `Faithful` is asked of
these only (the core reads no other cell). -/
def Listed (L : Ledger) (X : Cov) : Prop :=
  X ∈ L.facets ∨ X ∈ L.topes ∨ X ∈ L.edges ∨ X ∈ L.verts

/-- **The curved target** (design §1.3): an exact curved ledger whose listed cells are faithful.
`Consistent L` follows from `ExactK` exactly as `Exact.consistent` does; (c) and `zeroThickness_iff`
over `ExactK` are `Correct.lean`'s proofs with the cells renamed; `Faithful` is what makes
`XNonManifold` over these cells the geometric non-manifoldness of the result. -/
def CurvedTarget [Topology P] (L : Ledger) (cs : List (Carrier P K)) (A : Nat → Pred P) : Prop :=
  ExactK L cs A ∧ Faithful cs (Listed L)

end ForgeExact.DecideOnce.Curved
