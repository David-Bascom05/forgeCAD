import ForgeExact.DecideOnce.BoxesExact
import ForgeExact.DecideOnce.Witness

/-!
# The decide-once core is correct for every flat input

`sampleLedger ps ss X` builds the core's ledger from the reference's flat B-reps `ss`
(`Ref.Solid`, membership by ray parity), the carriers `ps` and a list of sample points `X`:

* the facets are the samples' cells with one zero;
* a facet is a piece of body `i` when `i`'s ray parity differs at the two points stepped off its
  sample across its carrier (`sideAt`, `side_cells`): no point-in-face test is needed;
* a side cell is inside body `i` by the ray at its side point;
* the edge and vertex cells are the samples' cells of rank 2 and 3 (`IsEdge`, `IsVert`, decided
  exactly) below a facet.

`sample_exact` proves every field of `Exact` for it, for every closed flat-faced input whose face
planes are carriers (`FlatInput`), given one hypothesis on the samples alone: they hit every cell
the boundary needs (`Samples`; implied by `Covers`, a point in every cell lying in a carrier).
`flip_iff` is the heart: the flip of body `i` across a facet is the reference's ray on its two sides
(`brep_compat`, from Jordan–Brouwer). `sample_correct` is then (c) for every flat input.
-/

namespace ForgeExact.DecideOnce.Sample

open ForgeExact ForgeExact.Ref Ledger

instance (ps : List Plane) (X : Cov) : Decidable (IsVert ps X) := by unfold IsVert; infer_instance
instance (ps : List Plane) (X : Cov) : Decidable (IsEdge ps X) := by unfold IsEdge; infer_instance

def solidAt (ss : List Solid) (i : Nat) : Solid := ss.getD i default

/-- The reference's membership of body `i`. -/
def brepA (ss : List Solid) : Nat → Pred V3 := fun i x => (solidAt ss i).contains x = true

def carrier (ps : List Plane) (φ : Cov) : Plane := ps.getD φ.zeroIdx default

/-- The point stepped off the facet's carrier from `m`, to the plus (`s = 1`) or minus side. -/
def sideAt (ps : List Plane) (m : V3) (φ : Cov) (s : Rat) : V3 :=
  m + (s * stepBound (carrier ps φ) m ps) * (carrier ps φ).n

/-- The sample point of a cell. -/
def sampleOf (ps : List Plane) (X : List V3) (φ : Cov) : V3 :=
  (X.find? fun x => covOf ps x == φ).getD default

/-- Body `i`'s membership flips across `φ`, by the reference's ray on both sides. -/
def flipB (ps : List Plane) (ss : List Solid) (X : List V3) (i : Nat) (φ : Cov) : Bool :=
  (solidAt ss i).contains (sideAt ps (sampleOf ps X φ) φ 1) !=
    (solidAt ss i).contains (sideAt ps (sampleOf ps X φ) φ (-1))

def cellsF (ps : List Plane) (X : List V3) : List Cov := dedup ((X.map (covOf ps)).filter Cov.isFacet)

def piecesOf (ps : List Plane) (ss : List Solid) (X : List V3) : List (Cov × Nat) :=
  (cellsF ps X).flatMap fun φ =>
    ((List.range ss.length).filter fun i => flipB ps ss X i φ).map fun i => (φ, i * ps.length + φ.zeroIdx)

def facetsOf (ps : List Plane) (ss : List Solid) (X : List V3) : List Cov :=
  dedup ((piecesOf ps ss X).map Prod.fst)

def insideSide (ps : List Plane) (ss : List Solid) (X : List V3) (φ : Cov) (s : Rat) (T : Cov) :
    List (Cov × Nat) :=
  ((List.range ss.length).filter fun i => (solidAt ss i).contains (sideAt ps (sampleOf ps X φ) φ s)).map
    fun i => (T, i)

/-- **The ledger from samples.** Every decision is the reference's, made once: the facets are the
samples' cells with one zero; a facet is a piece of body `i` when `i`'s ray parity differs on
its two sides; a side cell is inside `i` by the ray at its side point; the edge and vertex cells
are the samples' cells of rank 2 and 3 below a facet. -/
def sampleLedger (ps : List Plane) (ss : List Solid) (X : List V3) : Ledger :=
  { nb := ss.length
    faceBody := (List.range (ss.length * ps.length)).map (· / ps.length)
    faceCls := (List.range (ss.length * ps.length)).map (· % ps.length)
    pieces := piecesOf ps ss X
    inside := (facetsOf ps ss X).flatMap fun φ =>
      insideSide ps ss X φ 1 φ.plus ++ insideSide ps ss X φ (-1) φ.minus
    edges := dedup ((X.map (covOf ps)).filter fun e =>
      decide (IsEdge ps e) && (facetsOf ps ss X).any (Cov.le e ·))
    verts := dedup ((X.map (covOf ps)).filter fun v =>
      decide (IsVert ps v) && (facetsOf ps ss X).any (Cov.le v ·)) }

theorem facets_eq (ps : List Plane) (ss : List Solid) (X : List V3) :
    (sampleLedger ps ss X).facets = facetsOf ps ss X := rfl

/-- The hypothesis left: the samples hit every cell the boundary needs. -/
def Near (ps : List Plane) (A : Nat → Pred V3) (nb : Nat) (φ : Cov) : Prop :=
  φ.isFacet = true ∧ Realised ps φ ∧ ∃ i, i < nb ∧ Flip ps A φ i

def Samples (ps : List Plane) (A : Nat → Pred V3) (nb : Nat) (X : List V3) : Prop :=
  ∀ x, (Near ps A nb (covOf ps x) ∨
      ((IsEdge ps (covOf ps x) ∨ IsVert ps (covOf ps x)) ∧
        ∃ φ, Near ps A nb φ ∧ Cov.le (covOf ps x) φ = true)) →
    ∃ y ∈ X, covOf ps y = covOf ps x

/-! ## Side points -/

theorem count_set_idx : ∀ (l : List Sgn) (b : Sgn), b ≠ .zero → l.count .zero = 1 →
    (l.set (l.idxOf .zero) b).count .zero = 0
  | [], _, _, h => by simp at h
  | a :: l, b, hb, h => by
    by_cases ha : a = .zero
    · subst ha
      have : l.count .zero = 0 := by simp at h; omega
      simp [List.count_cons, hb, this]
    · rw [idxOf_cons_ne (Ne.symm (fun e => ha e.symm))]
      simp only [List.set_cons_succ]
      have h' : l.count .zero = 1 := by simp [List.count_cons, ha] at h; omega
      have := count_set_idx l b hb h'
      simp [List.count_cons, ha, this]

theorem zero_mem_of_facet {φ : Cov} (h : φ.isFacet = true) : Sgn.zero ∈ φ := by
  unfold Cov.isFacet at h
  have : 0 < φ.count .zero := by simp at h; omega
  exact List.count_pos_iff.mp this

theorem zeroIdx_lt {φ : Cov} (h : φ.isFacet = true) : φ.zeroIdx < φ.length :=
  List.idxOf_lt_length_of_mem (zero_mem_of_facet h)

theorem count_one {φ : Cov} (h : φ.isFacet = true) : φ.count .zero = 1 := by
  unfold Cov.isFacet at h; simpa using h

theorem plus_full {ps : List Plane} {φ : Cov} {m : V3} (hm : covCell ps φ m) (h : φ.isFacet = true) :
    φ.plus ∈ fulls ps.length := by
  apply mem_fulls
  · simp [Cov.plus, covCell_length hm]
  · intro hz
    have := count_set_idx φ .pos (by decide) (count_one h)
    have : 0 < (φ.set φ.zeroIdx .pos).count .zero := List.count_pos_iff.mpr hz
    unfold Cov.zeroIdx at this; omega

theorem minus_full {ps : List Plane} {φ : Cov} {m : V3} (hm : covCell ps φ m) (h : φ.isFacet = true) :
    φ.minus ∈ fulls ps.length := by
  apply mem_fulls
  · simp [Cov.minus, covCell_length hm]
  · intro hz
    have := count_set_idx φ .neg (by decide) (count_one h)
    have : 0 < (φ.set φ.zeroIdx .neg).count .zero := List.count_pos_iff.mpr hz
    unfold Cov.zeroIdx at this; omega

theorem get_carrier {ps : List Plane} {φ : Cov} {m : V3} (hm : covCell ps φ m) (h : φ.isFacet = true) :
    ps[φ.idxOf .zero]? = some (carrier ps φ) := by
  have hl : φ.zeroIdx < ps.length := covCell_length hm ▸ zeroIdx_lt h
  unfold carrier
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl, Option.getD_some]
  exact List.getElem?_eq_getElem hl

theorem carrier_mem {ps : List Plane} {φ : Cov} {m : V3} (hm : covCell ps φ m) (h : φ.isFacet = true) :
    carrier ps φ ∈ ps := by
  have := get_carrier hm h
  exact List.mem_of_getElem? this

/-- **The side points are in the side cells.** -/
theorem side_cells {ps : List Plane} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) {φ : Cov} {m : V3}
    (hm : covCell ps φ m) (h : φ.isFacet = true) :
    covCell ps φ.plus (sideAt ps m φ 1) ∧ covCell ps φ.minus (sideAt ps m φ (-1)) := by
  have hP := hnz _ (carrier_mem hm h)
  have hδ := stepBound_pos (carrier ps φ) m ps
  have hb := fun q hq hne => stepBound_le (carrier ps φ) m ps q hq hne
  have h1 := side_point (carrier ps φ) hP m 1 (Or.inl rfl) ps φ hm (count_one h) (get_carrier hm h) _ hδ hb
  have h2 := side_point (carrier ps φ) hP m (-1) (Or.inr rfl) ps φ hm (count_one h) (get_carrier hm h) _ hδ hb
  simp only [ite_true] at h1
  simp only [show ¬((-1 : Rat) = 1) by decide, ite_false] at h2
  exact ⟨h1, h2⟩

/-! ## The solids -/

/-- The inputs are closed flat B-reps on the carriers. -/
structure FlatInput (ps : List Plane) (ss : List Solid) : Prop where
  flat : ∀ s ∈ ss, Jordan.FacesFlat s
  closed : ∀ s ∈ ss, Jordan.Closed s
  planes : ∀ s ∈ ss, ∀ f ∈ Jordan.facesL s, Scaled ps f.plane

theorem solidAt_cases (ss : List Solid) (i : Nat) : solidAt ss i ∈ ss ∨ solidAt ss i = default := by
  unfold solidAt
  rw [List.getD_eq_getElem?_getD]
  by_cases h : i < ss.length
  · left; rw [List.getElem?_eq_getElem h]; exact List.getElem_mem h
  · right; rw [List.getElem?_eq_none (by omega)]; rfl

theorem facesL_default : Jordan.facesL (default : Solid) = [] := rfl

theorem input_at {ps : List Plane} {ss : List Solid} (hi : FlatInput ps ss) (i : Nat) :
    Jordan.FacesFlat (solidAt ss i) ∧ Jordan.Closed (solidAt ss i) ∧
      ∀ f ∈ Jordan.facesL (solidAt ss i), Scaled ps f.plane := by
  rcases solidAt_cases ss i with h | h
  · exact ⟨hi.flat _ h, hi.closed _ h, hi.planes _ h⟩
  · rw [h]
    refine ⟨fun f hf => by simp [facesL_default] at hf, fun e => ?_, fun f hf => by simp [facesL_default] at hf⟩
    simp [Jordan.Closed, Jordan.allEdges, facesL_default]

/-- On a full cell, membership is the reference's ray at any one point of it. -/
theorem memT_at {ps : List Plane} {ss : List Solid} (hi : FlatInput ps ss) {T : Cov}
    (hT : T ∈ fulls ps.length) {y : V3} (hy : covCell ps T y) (i : Nat) :
    memT ps (brepA ss) T i ↔ (solidAt ss i).contains y = true := by
  have hc := brep_compat ps (solidAt ss) (fun i => (input_at hi i).1) (fun i => (input_at hi i).2.1)
    (fun i => (input_at hi i).2.2) T hT i
  constructor
  · rintro ⟨x, hx, hA⟩
    rcases hc with hsub | hdis
    · exact hsub y hy
    · exact absurd hA (hdis x hx)
  · intro h; exact ⟨y, hy, h⟩

/-- **The flip is the ray's, on the two sides.** -/
theorem flip_iff {ps : List Plane} {ss : List Solid} (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩)
    (hi : FlatInput ps ss) {φ : Cov} {m : V3} (hm : covCell ps φ m) (h : φ.isFacet = true) (i : Nat) :
    Flip ps (brepA ss) φ i ↔
      ((solidAt ss i).contains (sideAt ps m φ 1) != (solidAt ss i).contains (sideAt ps m φ (-1))) = true := by
  obtain ⟨h1, h2⟩ := side_cells hnz hm h
  unfold Flip
  rw [memT_at hi (plus_full hm h) h1, memT_at hi (minus_full hm h) h2]
  cases (solidAt ss i).contains (sideAt ps m φ 1) <;> cases (solidAt ss i).contains (sideAt ps m φ (-1)) <;> simp

/-! ## The ledger's lists -/

variable {ps : List Plane} {ss : List Solid} {X : List V3}

theorem sampleOf_cov {φ : Cov} (h : ∃ x ∈ X, covOf ps x = φ) : covOf ps (sampleOf ps X φ) = φ := by
  unfold sampleOf
  cases hf : X.find? (fun x => covOf ps x == φ) with
  | none =>
    obtain ⟨x, hx, he⟩ := h
    have := List.find?_eq_none.mp hf x hx
    simp [he] at this
  | some y =>
    have := List.find?_some hf
    simpa using this

theorem sampleOf_cell {φ : Cov} (h : ∃ x ∈ X, covOf ps x = φ) : covCell ps φ (sampleOf ps X φ) := by
  have := covCell_covOf ps (sampleOf ps X φ)
  rwa [sampleOf_cov h] at this

theorem mem_cellsF {φ : Cov} : φ ∈ cellsF ps X ↔ (∃ x ∈ X, covOf ps x = φ) ∧ φ.isFacet = true := by
  unfold cellsF
  simp [mem_dedup, List.mem_filter, List.mem_map]

theorem mem_piecesOf {p : Cov × Nat} : p ∈ piecesOf ps ss X ↔
    ∃ φ ∈ cellsF ps X, ∃ i, i < ss.length ∧ flipB ps ss X i φ = true ∧ p = (φ, i * ps.length + φ.zeroIdx) := by
  unfold piecesOf
  simp only [List.mem_flatMap, List.mem_map, List.mem_filter, List.mem_range]
  constructor
  · rintro ⟨φ, hφ, i, ⟨hi, hf⟩, rfl⟩; exact ⟨φ, hφ, i, hi, hf, rfl⟩
  · rintro ⟨φ, hφ, i, hi, hf, rfl⟩; exact ⟨φ, hφ, i, ⟨hi, hf⟩, rfl⟩

theorem mem_facetsOf {φ : Cov} : φ ∈ facetsOf ps ss X ↔
    φ ∈ cellsF ps X ∧ ∃ i, i < ss.length ∧ flipB ps ss X i φ = true := by
  unfold facetsOf
  rw [mem_dedup]
  simp only [List.mem_map]
  constructor
  · rintro ⟨p, hp, rfl⟩
    obtain ⟨φ, hφ, i, hi, hf, rfl⟩ := mem_piecesOf.mp hp
    exact ⟨hφ, i, hi, hf⟩
  · rintro ⟨hφ, i, hi, hf⟩
    exact ⟨_, mem_piecesOf.mpr ⟨φ, hφ, i, hi, hf, rfl⟩, rfl⟩

theorem idx_lt {φ : Cov} (hφ : φ ∈ cellsF ps X) : φ.zeroIdx < ps.length := by
  obtain ⟨⟨x, _, rfl⟩, hf⟩ := mem_cellsF.mp hφ
  have := zeroIdx_lt hf
  simpa [covOf] using this

theorem face_lt {n nb i z : Nat} (hi : i < nb) (hz : z < n) : i * n + z < nb * n := by
  have : (i + 1) * n ≤ nb * n := Nat.mul_le_mul_right n hi
  rw [Nat.succ_mul] at this; omega

theorem face_div {n i z : Nat} (hz : z < n) : (i * n + z) / n = i := by
  rw [Nat.mul_comm, Nat.mul_add_div (by omega), Nat.div_eq_of_lt hz]; rfl

theorem face_mod {n i z : Nat} (hz : z < n) : (i * n + z) % n = z := by
  rw [Nat.mul_comm, Nat.mul_add_mod, Nat.mod_eq_of_lt hz]

theorem bodyOf_eq {i z : Nat} (hi : i < ss.length) (hz : z < ps.length) :
    (sampleLedger ps ss X).bodyOf (i * ps.length + z) = i := by
  unfold Ledger.bodyOf sampleLedger
  simp only
  rw [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range (face_lt hi hz)]
  simp [face_div hz]

theorem clsOf_eq {i z : Nat} (hi : i < ss.length) (hz : z < ps.length) :
    (sampleLedger ps ss X).clsOf (i * ps.length + z) = z := by
  unfold Ledger.clsOf sampleLedger
  simp only
  rw [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range (face_lt hi hz)]
  simp [face_mod hz]

theorem isPieceOf_iff {i : Nat} {φ : Cov} :
    (sampleLedger ps ss X).isPieceOf i φ = true ↔ φ ∈ cellsF ps X ∧ i < ss.length ∧ flipB ps ss X i φ = true := by
  unfold Ledger.isPieceOf
  rw [List.any_eq_true]
  constructor
  · rintro ⟨p, hp, h⟩
    obtain ⟨ψ, hψ, j, hj, hf, rfl⟩ := mem_piecesOf.mp hp
    simp only [Bool.and_eq_true, beq_iff_eq] at h
    obtain ⟨he, hb⟩ := h
    rw [bodyOf_eq hj (idx_lt hψ)] at hb
    subst hb he
    exact ⟨hψ, hj, hf⟩
  · rintro ⟨hφ, hi, hf⟩
    refine ⟨_, mem_piecesOf.mpr ⟨φ, hφ, i, hi, hf, rfl⟩, ?_⟩
    simp [bodyOf_eq hi (idx_lt hφ)]

/-- At a sampled facet, `flipB` is the flip. -/
theorem flipB_iff (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss) {φ : Cov}
    (hφ : φ ∈ cellsF ps X) (i : Nat) : flipB ps ss X i φ = true ↔ Flip ps (brepA ss) φ i := by
  obtain ⟨hs, hf⟩ := mem_cellsF.mp hφ
  rw [flip_iff hnz hi (sampleOf_cell hs) hf]
  rfl

theorem cellsF_of_facets {φ : Cov} (h : φ ∈ facetsOf ps ss X) : φ ∈ cellsF ps X := (mem_facetsOf.mp h).1

theorem mem_insideSide {φ : Cov} {s : Rat} {T : Cov} {q : Cov × Nat} :
    q ∈ insideSide ps ss X φ s T ↔ q.1 = T ∧ q.2 < ss.length ∧
      (solidAt ss q.2).contains (sideAt ps (sampleOf ps X φ) φ s) = true := by
  obtain ⟨q1, q2⟩ := q
  unfold insideSide
  simp only [List.mem_map, List.mem_filter, List.mem_range, Prod.mk.injEq]
  constructor
  · rintro ⟨j, ⟨hj, hc⟩, rfl, rfl⟩; exact ⟨rfl, hj, hc⟩
  · rintro ⟨rfl, hj, hc⟩; exact ⟨q2, ⟨hj, hc⟩, rfl, rfl⟩

theorem mem_inside {T : Cov} {i : Nat} : (T, i) ∈ (sampleLedger ps ss X).inside ↔
    ∃ φ ∈ facetsOf ps ss X, i < ss.length ∧
      ((T = φ.plus ∧ (solidAt ss i).contains (sideAt ps (sampleOf ps X φ) φ 1) = true) ∨
       (T = φ.minus ∧ (solidAt ss i).contains (sideAt ps (sampleOf ps X φ) φ (-1)) = true)) := by
  show (T, i) ∈ (facetsOf ps ss X).flatMap _ ↔ _
  simp only [List.mem_flatMap, List.mem_append, mem_insideSide]
  constructor
  · rintro ⟨φ, hφ, ⟨h1, h2, h3⟩ | ⟨h1, h2, h3⟩⟩
    · exact ⟨φ, hφ, h2, Or.inl ⟨h1, h3⟩⟩
    · exact ⟨φ, hφ, h2, Or.inr ⟨h1, h3⟩⟩
  · rintro ⟨φ, hφ, h2, ⟨h1, h3⟩ | ⟨h1, h3⟩⟩
    · exact ⟨φ, hφ, Or.inl ⟨h1, h2, h3⟩⟩
    · exact ⟨φ, hφ, Or.inr ⟨h1, h2, h3⟩⟩

theorem mem_topes {T : Cov} : T ∈ (sampleLedger ps ss X).topes ↔
    ∃ φ ∈ facetsOf ps ss X, T = φ.plus ∨ T = φ.minus := by
  unfold Ledger.topes
  rw [mem_dedup, facets_eq]
  simp only [List.mem_flatMap, List.mem_cons, List.mem_nil_iff, or_false]

/-- The two side cells of a ledger facet, with their side points. -/
theorem facet_sides (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) {φ : Cov} (hφ : φ ∈ facetsOf ps ss X) :
    φ.plus ∈ fulls ps.length ∧ φ.minus ∈ fulls ps.length ∧
    covCell ps φ.plus (sideAt ps (sampleOf ps X φ) φ 1) ∧
    covCell ps φ.minus (sideAt ps (sampleOf ps X φ) φ (-1)) := by
  obtain ⟨hs, hf⟩ := mem_cellsF.mp (cellsF_of_facets hφ)
  have hm := sampleOf_cell hs
  exact ⟨plus_full hm hf, minus_full hm hf, side_cells hnz hm hf⟩

/-! ## `Exact`, field by field -/

theorem s_witness (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss) :
    ∀ T ∈ (sampleLedger ps ss X).topes, ∀ i, i < (sampleLedger ps ss X).nb →
      ((sampleLedger ps ss X).inside.contains (T, i) = true ↔ memT ps (brepA ss) T i) := by
  intro T hT i hi'
  rw [List.contains_iff_mem, mem_inside]
  constructor
  · rintro ⟨ψ, hψ, _, ⟨rfl, hc⟩ | ⟨rfl, hc⟩⟩
    · obtain ⟨f1, _, c1, _⟩ := facet_sides hnz hψ
      exact (memT_at hi f1 c1 i).mpr hc
    · obtain ⟨_, f2, _, c2⟩ := facet_sides hnz hψ
      exact (memT_at hi f2 c2 i).mpr hc
  · intro hm
    obtain ⟨φ, hφ, rfl | rfl⟩ := mem_topes.mp hT
    · obtain ⟨f1, _, c1, _⟩ := facet_sides hnz hφ
      exact ⟨φ, hφ, hi', Or.inl ⟨rfl, (memT_at hi f1 c1 i).mp hm⟩⟩
    · obtain ⟨_, f2, _, c2⟩ := facet_sides hnz hφ
      exact ⟨φ, hφ, hi', Or.inr ⟨rfl, (memT_at hi f2 c2 i).mp hm⟩⟩

theorem s_boundary (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss) :
    ∀ φ ∈ (sampleLedger ps ss X).facets, ∀ i, i < (sampleLedger ps ss X).nb →
      (Flip ps (brepA ss) φ i ↔ (sampleLedger ps ss X).isPieceOf i φ = true) := by
  intro φ hφ i hi'
  rw [facets_eq] at hφ
  have hc := cellsF_of_facets hφ
  rw [isPieceOf_iff, ← flipB_iff hnz hi hc]
  exact ⟨fun h => ⟨hc, hi', h⟩, fun h => h.2.2⟩

/-- A ledger facet is `Near`. -/
theorem near_of_facet (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss) {φ : Cov}
    (hφ : φ ∈ facetsOf ps ss X) : Near ps (brepA ss) ss.length φ := by
  obtain ⟨hc, i, hi', hf⟩ := mem_facetsOf.mp hφ
  obtain ⟨hs, hfa⟩ := mem_cellsF.mp hc
  exact ⟨hfa, ⟨_, sampleOf_cell hs⟩, i, hi', (flipB_iff hnz hi hc i).mp hf⟩

theorem s_complete (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss)
    (hX : Samples ps (brepA ss) ss.length X) :
    ∀ φ, Realised ps φ → φ.isFacet = true → ∀ i, i < (sampleLedger ps ss X).nb →
      Flip ps (brepA ss) φ i → φ ∈ (sampleLedger ps ss X).facets := by
  intro φ ⟨x, hx⟩ hf i hi' hfl
  have e := BoxEx.covCell_eq_covOf hx
  subst e
  obtain ⟨y, hy, he⟩ := hX x (Or.inl ⟨hf, ⟨x, hx⟩, i, hi', hfl⟩)
  have hc : covOf ps x ∈ cellsF ps X := mem_cellsF.mpr ⟨⟨y, hy, he⟩, hf⟩
  rw [facets_eq]
  exact mem_facetsOf.mpr ⟨hc, i, hi', (flipB_iff hnz hi hc i).mpr hfl⟩

theorem s_low (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss)
    (hX : Samples ps (brepA ss) ss.length X) (D : Cov → Prop) [DecidablePred D]
    (hD : ∀ e, D e → IsEdge ps e ∨ IsVert ps e) :
    ∀ e, e ∈ dedup ((X.map (covOf ps)).filter fun e => decide (D e) && (facetsOf ps ss X).any (Cov.le e ·)) ↔
      Realised ps e ∧ D e ∧ ∃ φ ∈ (sampleLedger ps ss X).facets, Cov.le e φ = true := by
  intro e
  rw [mem_dedup, facets_eq]
  simp only [List.mem_filter, List.mem_map, Bool.and_eq_true, decide_eq_true_eq, List.any_eq_true]
  constructor
  · rintro ⟨⟨y, _, rfl⟩, hd, φ, hφ, hle⟩
    exact ⟨⟨y, covCell_covOf ps y⟩, hd, φ, hφ, hle⟩
  · rintro ⟨⟨x, hx⟩, hd, φ, hφ, hle⟩
    have e1 := BoxEx.covCell_eq_covOf hx
    subst e1
    obtain ⟨y, hy, he⟩ := hX x (Or.inr ⟨hD _ hd, φ, near_of_facet hnz hi hφ, hle⟩)
    exact ⟨⟨y, hy, he⟩, hd, φ, hφ, hle⟩

theorem s_pieceFacet : ∀ p ∈ (sampleLedger ps ss X).pieces, p.1.isFacet = true ∧
    p.1.zeroIdx = (sampleLedger ps ss X).clsOf p.2 ∧
    (sampleLedger ps ss X).bodyOf p.2 < (sampleLedger ps ss X).nb := by
  intro p hp
  obtain ⟨φ, hφ, i, hi, _, rfl⟩ := mem_piecesOf.mp hp
  exact ⟨(mem_cellsF.mp hφ).2, (clsOf_eq hi (idx_lt hφ)).symm, by rw [bodyOf_eq hi (idx_lt hφ)]; exact hi⟩

theorem s_oneFace : ∀ p ∈ (sampleLedger ps ss X).pieces, ∀ q ∈ (sampleLedger ps ss X).pieces, q.1 = p.1 →
    (sampleLedger ps ss X).bodyOf q.2 = (sampleLedger ps ss X).bodyOf p.2 → q.2 = p.2 := by
  intro p hp q hq h1 h2
  obtain ⟨φ, hφ, i, hi, _, rfl⟩ := mem_piecesOf.mp hp
  obtain ⟨ψ, hψ, j, hj, _, rfl⟩ := mem_piecesOf.mp hq
  simp only at h1 h2 ⊢
  subst h1
  rw [bodyOf_eq hi (idx_lt hφ), bodyOf_eq hj (idx_lt hψ)] at h2
  rw [h2]

theorem s_pieceReal : ∀ p ∈ (sampleLedger ps ss X).pieces, Realised ps p.1 := by
  intro p hp
  obtain ⟨φ, hφ, _, _, _, rfl⟩ := mem_piecesOf.mp hp
  exact ⟨_, sampleOf_cell (mem_cellsF.mp hφ).1⟩

/-- **`Exact` for every flat input.** For carriers with nonzero normals, solids that are closed
flat-faced B-reps whose face planes are carriers (up to scale), and sample points hitting every
cell the boundary needs (`Samples`), the ledger built from the samples is exact for the reference's
ray-parity solids. Every decision in it is one evaluation of the reference (the ray, at a point
stepped off a sample; the rank of zero normals); no other hypothesis. -/
theorem sample_exact (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss)
    (hX : Samples ps (brepA ss) ss.length X) : Exact (sampleLedger ps ss X) ps (brepA ss) where
  nonzero := hnz
  compat := brep_compat ps (solidAt ss) (fun i => (input_at hi i).1) (fun i => (input_at hi i).2.1)
    (fun i => (input_at hi i).2.2)
  pieceFacet := s_pieceFacet
  oneFace := s_oneFace
  pieceReal := s_pieceReal
  witness := s_witness hnz hi
  boundary := s_boundary hnz hi
  complete := s_complete hnz hi hX
  edges := s_low hnz hi hX (IsEdge ps) fun _ h => Or.inl h
  verts := s_low hnz hi hX (IsVert ps) fun _ h => Or.inr h

/-- A sufficient form of `Samples` that does not mention the solids: the samples hit every cell
of the arrangement lying in a carrier. -/
def Covers (ps : List Plane) (X : List V3) : Prop :=
  ∀ x, Sgn.zero ∈ covOf ps x → ∃ y ∈ X, covOf ps y = covOf ps x

theorem zero_of_edge {e : Cov} (h : IsEdge ps e ∨ IsVert ps e) : Sgn.zero ∈ e := by
  have : ∃ a ∈ zerosOf e, True := by
    rcases h with ⟨_, a, ha, _⟩ | ⟨a, ha, _⟩ <;> exact ⟨a, ha, trivial⟩
  obtain ⟨a, ha, _⟩ := this
  unfold zerosOf at ha
  simp only [List.mem_filter, List.mem_range, beq_iff_eq] at ha
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem ha.1, Option.getD_some] at ha
  exact ha.2 ▸ List.getElem_mem ha.1

theorem samples_of_covers {A : Nat → Pred V3} {nb : Nat} (h : Covers ps X) : Samples ps A nb X := by
  intro x hx
  apply h x
  rcases hx with ⟨hf, _⟩ | ⟨he, _⟩
  · exact zero_mem_of_facet hf
  · exact zero_of_edge he

/-- **(c) for every flat input.** Each valid answer of the core on the sample ledger is the exact
regularised boolean of the reference's solids, its faces exactly the boundary facets; and the
zero-thickness refusal is exactly non-manifoldness. -/
theorem sample_correct (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hi : FlatInput ps ss)
    (hX : Samples ps (brepA ss) ss.length X) (rule : Rule) :
    (∀ b, core (sampleLedger ps ss X) rule = .ok b →
      DecideOnce.Valid (sampleLedger ps ss X) ((sampleLedger ps ss X).keepTab rule) b ∧
      (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔ XBoundary (sampleLedger ps ss X) ps (brepA ss) rule φ) ∧
      reg ((decomp (sample_exact hnz hi hX)).kept fun T =>
          xkeep (sampleLedger ps ss X) ps (brepA ss) rule T = true) =
        reg (combine (ruleOp rule) (opnds (sampleLedger ps ss X) (brepA ss)))) ∧
    ((∃ x, core (sampleLedger ps ss X) rule = .refused (.zeroThickness x)) ↔
      XNonManifold (sampleLedger ps ss X) ps (brepA ss) rule) :=
  ⟨fun _ hb => correct_ok (sample_exact hnz hi hX) rule hb, zeroThickness_iff (sample_exact hnz hi hX) rule⟩

/-! ## Checks (compiled evaluation: tests, not theorems)

Boxes given as reference B-reps, samples at the half-grid: a cube is 6 faces / 12 edges / 8 vertices;
two cubes face to face unite to one box (10 facets in its 6 faces); two cubes along an edge are
refused at the edge. -/

namespace Check
def v (x y z : Rat) : V3 := ⟨x, y, z⟩
def pl (a b c d : Rat) : Plane := ⟨⟨a, b, c⟩, d⟩
/-- The box [x0,x1]×[y0,y1]×[z0,z1] as a reference B-rep. -/
def boxS (x0 x1 y0 y1 z0 z1 : Rat) : Solid :=
  ⟨"new", #[
    ⟨pl 1 0 0 x0, #[#[v x0 y0 z0, v x0 y1 z0, v x0 y1 z1, v x0 y0 z1]]⟩,
    ⟨pl 1 0 0 x1, #[#[v x1 y0 z0, v x1 y0 z1, v x1 y1 z1, v x1 y1 z0]]⟩,
    ⟨pl 0 1 0 y0, #[#[v x0 y0 z0, v x0 y0 z1, v x1 y0 z1, v x1 y0 z0]]⟩,
    ⟨pl 0 1 0 y1, #[#[v x0 y1 z0, v x1 y1 z0, v x1 y1 z1, v x0 y1 z1]]⟩,
    ⟨pl 0 0 1 z0, #[#[v x0 y0 z0, v x1 y0 z0, v x1 y1 z0, v x0 y1 z0]]⟩,
    ⟨pl 0 0 1 z1, #[#[v x0 y0 z1, v x0 y1 z1, v x1 y1 z1, v x1 y0 z1]]⟩]⟩
def grid (ks : List Rat) : List Plane := (ks.map (pl 1 0 0 ·)) ++ (ks.map (pl 0 1 0 ·)) ++ (ks.map (pl 0 0 1 ·))
def pts (cs : List Rat) : List V3 := cs.flatMap fun x => cs.flatMap fun y => cs.map fun z => v x y z
def cs2 : List Rat := [-1/2, 0, 1/2, 1, 3/2]
def uni : Rule := fun m => m 0 || m 1
def L1 := sampleLedger (grid [0, 1]) [boxS 0 1 0 1 0 1] (pts cs2)
#guard (core L1 uni).summary == .ok 6 6 12 8
def cs3 : List Rat := [-1/2, 0, 1/2, 1, 3/2, 2, 5/2]
def L2 := sampleLedger (grid [0, 1, 2]) [boxS 0 1 0 1 0 1, boxS 1 2 0 1 0 1] (pts cs3)
#guard (core L2 uni).summary == .ok 6 10 12 8
def L3 := sampleLedger (grid [0, 1, 2]) [boxS 0 1 0 1 0 1, boxS 1 2 1 2 0 1] (pts cs3)
#guard (match (core L3 uni).summary with | .zeroThickness _ => true | _ => false)
end Check

end ForgeExact.DecideOnce.Sample
