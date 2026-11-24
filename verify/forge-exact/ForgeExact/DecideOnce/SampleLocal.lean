import ForgeExact.DecideOnce.Sample
import ForgeExact.DecideOnce.Local
import ForgeExact.DecideOnce.Sampler

/-!
# The local ledger is a sampler ledger, and is `ExactIn`

The bridge's local ledger (spec §3a) is the sampler's ledger built from the samples lying in the
open region: `sampleLedger ps ss (X.filter In)`. Every clause of `ExactIn` but compatibility follows
from the sampler's own lemmas, restricted to the region:
* pieces, facets, edges and verts come from region samples, so they lie in the region;
* the witness and the flip are decided on side cells of region facets, which are region cells
  (`up`);
* completeness and the edges/verts lists need region cells to be sampled (`SamplesIn`).

Compatibility in the region is the named obligation `LocalCompat` (the localized Jordan lemma,
group dol-jordan). It enters only through `memT_in`. `local_exact` proves `ExactIn` from it and the
region's facts; `local_correct` then gives (c′).
-/

namespace ForgeExact.DecideOnce.Sample

open ForgeExact ForgeExact.Ref Ledger Topology

variable {ps : List Plane} {ss : List Solid} {X : List V3} {In : Cov → Prop} [DecidablePred In]

theorem cov_le_refl : ∀ (X : Cov), Cov.le X X = true
  | [] => by simp [Cov.le]
  | a :: t => by
    have := cov_le_refl t
    simp only [Cov.le, List.length_cons, List.zip_cons_cons, List.all_cons, Bool.and_eq_true, beq_iff_eq] at this ⊢
    exact ⟨by simp, by simp, this.2⟩

/-- A covector is below the one with its zero at `i` set: `X ≤ X.set i s` when `X[i]` is zero. -/
theorem le_set_zero : ∀ (X : Cov) (i : Nat) (s : Sgn), X.getD i .pos = .zero → Cov.le X (X.set i s) = true
  | [], _, _, _ => by simp [Cov.le]
  | a :: t, 0, s, h => by
    simp only [List.getD_cons_zero] at h
    subst h
    have : Cov.le t t = true := cov_le_refl t
    simp only [Cov.le, List.set_cons_zero, List.length_cons, List.zip_cons_cons, List.all_cons] at this ⊢
    simpa using this
  | a :: t, i + 1, s, h => by
    simp only [List.getD_cons_succ] at h
    have := le_set_zero t i s h
    simp only [Cov.le, List.set_cons_succ, List.length_cons, List.zip_cons_cons, List.all_cons,
      Bool.and_eq_true, beq_iff_eq] at this ⊢
    refine ⟨by simpa using this.1, by simp, this.2⟩

theorem le_plus {φ : Cov} (h : φ.isFacet = true) : Cov.le φ φ.plus = true := by
  apply le_set_zero
  have hl := zeroIdx_lt h
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl, Option.getD_some]
  exact List.getElem_idxOf hl

theorem le_minus {φ : Cov} (h : φ.isFacet = true) : Cov.le φ φ.minus = true := by
  apply le_set_zero
  have hl := zeroIdx_lt h
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl, Option.getD_some]
  exact List.getElem_idxOf hl

/-- The region's samples. -/
def regionX (ps : List Plane) (X : List V3) (In : Cov → Prop) [DecidablePred In] : List V3 :=
  X.filter fun x => decide (In (covOf ps x))

theorem mem_regionX {x : V3} : x ∈ regionX ps X In ↔ x ∈ X ∧ In (covOf ps x) := by simp [regionX]

/-- The samples hit every region cell the boundary needs. -/
def SamplesIn (ps : List Plane) (A : Nat → Pred V3) (nb : Nat) (X : List V3) (In : Cov → Prop) : Prop :=
  ∀ x, In (covOf ps x) → (Near ps A nb (covOf ps x) ∨
      ((IsEdge ps (covOf ps x) ∨ IsVert ps (covOf ps x)) ∧
        ∃ φ, Near ps A nb φ ∧ Cov.le (covOf ps x) φ = true)) →
    ∃ y ∈ X, covOf ps y = covOf ps x

/-- On a full region cell, membership is the ray at any one point of it (from `LocalCompat`). -/
theorem memT_in (hc : LocalCompat ps (brepA ss) In) {T : Cov} (hT : T ∈ fulls ps.length) (hin : In T)
    {y : V3} (hy : covCell ps T y) (i : Nat) :
    memT ps (brepA ss) T i ↔ (solidAt ss i).contains y = true := by
  constructor
  · rintro ⟨x, hx, hA⟩
    rcases hc T hT hin i with hsub | hdis
    · exact hsub y hy
    · exact absurd hA (hdis x hx)
  · intro h; exact ⟨y, hy, h⟩

theorem flip_in (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hc : LocalCompat ps (brepA ss) In)
    (hup : ∀ X Y, In X → Cov.le X Y = true → In Y)
    {φ : Cov} {m : V3} (hm : covCell ps φ m) (h : φ.isFacet = true) (hin : In φ) (i : Nat) :
    Flip ps (brepA ss) φ i ↔
      ((solidAt ss i).contains (sideAt ps m φ 1) != (solidAt ss i).contains (sideAt ps m φ (-1))) = true := by
  obtain ⟨h1, h2⟩ := side_cells hnz hm h
  unfold Flip
  rw [memT_in hc (plus_full hm h) (hup _ _ hin (le_plus h)) h1,
    memT_in hc (minus_full hm h) (hup _ _ hin (le_minus h)) h2]
  cases (solidAt ss i).contains (sideAt ps m φ 1) <;> cases (solidAt ss i).contains (sideAt ps m φ (-1)) <;> simp

/-- The region's sampled facets lie in the region. -/
theorem cellsF_in {φ : Cov} (h : φ ∈ cellsF ps (regionX ps X In)) : In φ := by
  obtain ⟨⟨x, hx, rfl⟩, _⟩ := mem_cellsF.mp h
  exact (mem_regionX.mp hx).2

theorem flipB_in (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (hc : LocalCompat ps (brepA ss) In)
    (hup : ∀ X Y, In X → Cov.le X Y = true → In Y)
    {φ : Cov} (hφ : φ ∈ cellsF ps (regionX ps X In)) (i : Nat) :
    flipB ps ss (regionX ps X In) i φ = true ↔ Flip ps (brepA ss) φ i := by
  obtain ⟨hs, hf⟩ := mem_cellsF.mp hφ
  rw [flip_in hnz hc hup (sampleOf_cell hs) hf (cellsF_in hφ)]
  rfl

/-- **The local ledger is exact in the region**, given the named obligation `LocalCompat`, the region's
facts (an open union of cells, closed upward), and samples hitting the region's boundary cells. -/
theorem local_exact (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) {U : Pred V3}
    (hc : LocalCompat ps (brepA ss) In) (hU : IsOpen U)
    (hreg : ∀ Y x, covCell ps Y x → (U x ↔ In Y)) (hup : ∀ X Y, In X → Cov.le X Y = true → In Y)
    (hX : SamplesIn ps (brepA ss) ss.length X In) :
    ExactIn (sampleLedger ps ss (regionX ps X In)) ps (brepA ss) In U where
  nonzero := hnz
  open_ := hU
  region := hreg
  up := hup
  compat := hc
  pieceFacet := s_pieceFacet
  oneFace := s_oneFace
  pieceReal := s_pieceReal
  pieceIn := by
    intro p hp
    obtain ⟨φ, hφ, _, _, _, rfl⟩ := mem_piecesOf.mp hp
    exact cellsF_in hφ
  witness := by
    intro T hT i hi'
    rw [List.contains_iff_mem, mem_inside]
    have hside : ∀ ψ ∈ facetsOf ps ss (regionX ps X In), In ψ.plus ∧ In ψ.minus := by
      intro ψ hψ
      have hf := (mem_cellsF.mp (cellsF_of_facets hψ)).2
      have hin := cellsF_in (cellsF_of_facets hψ)
      exact ⟨hup _ _ hin (le_plus hf), hup _ _ hin (le_minus hf)⟩
    constructor
    · rintro ⟨ψ, hψ, _, ⟨rfl, hcn⟩ | ⟨rfl, hcn⟩⟩
      · obtain ⟨f1, _, c1, _⟩ := facet_sides hnz hψ
        exact (memT_in hc f1 (hside ψ hψ).1 c1 i).mpr hcn
      · obtain ⟨_, f2, _, c2⟩ := facet_sides hnz hψ
        exact (memT_in hc f2 (hside ψ hψ).2 c2 i).mpr hcn
    · intro hm
      obtain ⟨φ, hφ, rfl | rfl⟩ := mem_topes.mp hT
      · obtain ⟨f1, _, c1, _⟩ := facet_sides hnz hφ
        exact ⟨φ, hφ, hi', Or.inl ⟨rfl, (memT_in hc f1 (hside φ hφ).1 c1 i).mp hm⟩⟩
      · obtain ⟨_, f2, _, c2⟩ := facet_sides hnz hφ
        exact ⟨φ, hφ, hi', Or.inr ⟨rfl, (memT_in hc f2 (hside φ hφ).2 c2 i).mp hm⟩⟩
  boundary := by
    intro φ hφ i hi'
    rw [facets_eq] at hφ
    have hcf := cellsF_of_facets hφ
    rw [isPieceOf_iff, ← flipB_in hnz hc hup hcf]
    exact ⟨fun h => ⟨hcf, hi', h⟩, fun h => h.2.2⟩
  complete := by
    intro φ hin ⟨x, hx⟩ hf i hi' hfl
    have e := BoxEx.covCell_eq_covOf hx
    subst e
    obtain ⟨y, hy, he⟩ := hX x hin (Or.inl ⟨hf, ⟨x, hx⟩, i, hi', hfl⟩)
    have hyr : y ∈ regionX ps X In := mem_regionX.mpr ⟨hy, he ▸ hin⟩
    have hcf : covOf ps x ∈ cellsF ps (regionX ps X In) := mem_cellsF.mpr ⟨⟨y, hyr, he⟩, hf⟩
    rw [facets_eq]
    exact mem_facetsOf.mpr ⟨hcf, i, hi', (flipB_in hnz hc hup hcf i).mpr hfl⟩
  edges := by
    intro e
    show e ∈ dedup _ ↔ _
    rw [mem_dedup, facets_eq]
    simp only [List.mem_filter, List.mem_map, Bool.and_eq_true, decide_eq_true_eq, List.any_eq_true]
    constructor
    · rintro ⟨⟨y, hy, rfl⟩, hd, φ, hφ, hle⟩
      exact ⟨(mem_regionX.mp hy).2, ⟨y, covCell_covOf ps y⟩, hd, φ, hφ, hle⟩
    · rintro ⟨hin, ⟨x, hx⟩, hd, φ, hφ, hle⟩
      have e1 := BoxEx.covCell_eq_covOf hx
      subst e1
      have hnear : Near ps (brepA ss) ss.length φ := by
        obtain ⟨hcf, i, hi', hfb⟩ := mem_facetsOf.mp hφ
        obtain ⟨hs, hfa⟩ := mem_cellsF.mp hcf
        exact ⟨hfa, ⟨_, sampleOf_cell hs⟩, i, hi', (flipB_in hnz hc hup hcf i).mp hfb⟩
      obtain ⟨y, hy, he⟩ := hX x hin (Or.inr ⟨Or.inl hd, φ, hnear, hle⟩)
      exact ⟨⟨y, mem_regionX.mpr ⟨hy, he ▸ hin⟩, he⟩, hd, φ, hφ, hle⟩
  verts := by
    intro e
    show e ∈ dedup _ ↔ _
    rw [mem_dedup, facets_eq]
    simp only [List.mem_filter, List.mem_map, Bool.and_eq_true, decide_eq_true_eq, List.any_eq_true]
    constructor
    · rintro ⟨⟨y, hy, rfl⟩, hd, φ, hφ, hle⟩
      exact ⟨(mem_regionX.mp hy).2, ⟨y, covCell_covOf ps y⟩, hd, φ, hφ, hle⟩
    · rintro ⟨hin, ⟨x, hx⟩, hd, φ, hφ, hle⟩
      have e1 := BoxEx.covCell_eq_covOf hx
      subst e1
      have hnear : Near ps (brepA ss) ss.length φ := by
        obtain ⟨hcf, i, hi', hfb⟩ := mem_facetsOf.mp hφ
        obtain ⟨hs, hfa⟩ := mem_cellsF.mp hcf
        exact ⟨hfa, ⟨_, sampleOf_cell hs⟩, i, hi', (flipB_in hnz hc hup hcf i).mp hfb⟩
      obtain ⟨y, hy, he⟩ := hX x hin (Or.inr ⟨Or.inr hd, φ, hnear, hle⟩)
      exact ⟨⟨y, mem_regionX.mpr ⟨hy, he ▸ hin⟩, he⟩, hd, φ, hφ, hle⟩

/-- **(c′) for the bridge's local ledger.** On the sampler's ledger of the region's samples, with the
named obligation `LocalCompat`, an answered body is valid, its faces are exactly the region's exact
boundary facets, and the kept cells regularise to the regularised boolean at every point of `R°`.
`ZeroThickness` holds iff the exact result is non-manifold at a region cell. -/
theorem local_correct (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) {U : Pred V3}
    (hc : LocalCompat ps (brepA ss) In) (hU : IsOpen U)
    (hreg : ∀ Y x, covCell ps Y x → (U x ↔ In Y)) (hup : ∀ X Y, In X → Cov.le X Y = true → In Y)
    (hX : SamplesIn ps (brepA ss) ss.length X In) (rule : Rule) :
    (∀ b, core (sampleLedger ps ss (regionX ps X In)) rule = .ok b →
      Valid (sampleLedger ps ss (regionX ps X In)) ((sampleLedger ps ss (regionX ps X In)).keepTab rule) b ∧
      (∀ φ, (∃ F ∈ b.faces, φ ∈ F.facets) ↔ In φ ∧ XBoundary (sampleLedger ps ss (regionX ps X In)) ps (brepA ss) rule φ) ∧
      ∀ x, U x → (reg ((decompIn (local_exact hnz hc hU hreg hup hX)).kept fun T =>
          xkeepIn (sampleLedger ps ss (regionX ps X In)) ps (brepA ss) In rule T) x ↔
        reg (combine (ruleOp rule) (opnds (sampleLedger ps ss (regionX ps X In)) (brepA ss))) x)) ∧
    ((∃ x, core (sampleLedger ps ss (regionX ps X In)) rule = .refused (.zeroThickness x)) ↔
      XNonManifoldIn (sampleLedger ps ss (regionX ps X In)) ps (brepA ss) In rule) :=
  ⟨fun _ hb => correct_ok_in (local_exact hnz hc hU hreg hup hX) rule hb,
   zeroThickness_iff_in (local_exact hnz hc hU hreg hup hX) rule⟩

/-! ## The region of the six walls -/

/-- The walls: indices into the carriers, each oriented with the region on its positive side. -/
def wallIn (ws : List Nat) (X : Cov) : Prop := ∀ k ∈ ws, X.getD k .zero = .pos

instance (ws : List Nat) : DecidablePred (wallIn ws) := fun X => by unfold wallIn; infer_instance

/-- The open region: strictly inside every wall. -/
def wallU (ps : List Plane) (ws : List Nat) : Pred V3 := Flat.allPos (ws.map fun k => ps.getD k default)

theorem wallU_open (ps : List Plane) (ws : List Nat) : IsOpen (wallU ps ws) := Flat.isOpen_allPos _

theorem covOf_getD (ps : List Plane) (x : V3) {k : Nat} (hk : k < ps.length) :
    (covOf ps x).getD k .zero = sgnQ ((ps.getD k default).eval x) := by
  simp [covOf, List.getD_eq_getElem?_getD, hk]

theorem sgnQ_pos_iff (q : Rat) : sgnQ q = .pos ↔ q > 0 := by
  unfold sgnQ; split
  · simp_all
  · split <;> simp_all

theorem wall_region (ps : List Plane) (ws : List Nat) (hws : ∀ k ∈ ws, k < ps.length) :
    ∀ Y x, covCell ps Y x → (wallU ps ws x ↔ wallIn ws Y) := by
  intro Y x hx
  have e := BoxEx.covCell_eq_covOf hx
  subst e
  unfold wallU wallIn Flat.allPos
  simp only [List.mem_map, forall_exists_index, and_imp, forall_apply_eq_imp_iff₂]
  constructor
  · intro h k hk; rw [covOf_getD ps x (hws k hk), sgnQ_pos_iff]; exact h k hk
  · intro h k hk; have := h k hk; rw [covOf_getD ps x (hws k hk), sgnQ_pos_iff] at this; exact this

/-- `≤` keeps every nonzero sign. -/
theorem le_getD : ∀ {X Y : Cov}, Cov.le X Y = true → ∀ k, X.getD k .zero ≠ .zero → Y.getD k .zero = X.getD k .zero
  | [], _, _, k, hk => by simp at hk
  | a :: X, [], h, _, _ => by simp [Cov.le] at h
  | a :: X, b :: Y, h, k, hk => by
    have h' : (a == .zero || a == b) = true ∧ Cov.le X Y = true := by
      simp only [Cov.le, List.length_cons, List.zip_cons_cons, List.all_cons, Bool.and_eq_true, beq_iff_eq] at h ⊢
      have h1 := h.1
      refine ⟨h.2.1, ?_⟩
      simp only [h.2.2, and_true]
      omega
    cases k with
    | zero =>
      simp only [List.getD_cons_zero] at hk ⊢
      have := h'.1; simp only [Bool.or_eq_true, beq_iff_eq] at this
      rcases this with e | e
      · exact absurd e hk
      · exact e.symm
    | succ k =>
      simp only [List.getD_cons_succ] at hk ⊢
      exact le_getD h'.2 k hk

theorem wall_up (ws : List Nat) : ∀ X Y, wallIn ws X → Cov.le X Y = true → wallIn ws Y := by
  intro X Y hX hle k hk
  rw [le_getD hle k (by rw [hX k hk]; decide), hX k hk]

/-- **(c′) for the bridge's local ledger over the six walls**: as `local_correct`, with the region's
facts proven. What it still takes is `LocalCompat` and the region's samples (`SamplesIn`). -/
def local_correct_walls (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (ws : List Nat) (hws : ∀ k ∈ ws, k < ps.length)
    (hc : LocalCompat ps (brepA ss) (wallIn ws)) (hX : SamplesIn ps (brepA ss) ss.length X (wallIn ws))
    (rule : Rule) :=
  local_correct hnz hc (wallU_open ps ws) (wall_region ps ws hws) (wall_up ws) hX rule

/-- Global samples are region samples. -/
theorem samplesIn_of {A : Nat → Pred V3} {nb : Nat} (h : Samples ps A nb X) : SamplesIn ps A nb X In :=
  fun x _ hx => h x hx

/-- **(c′) for the local ledger the sampler builds**: the region's samples of the local arrangement's
sampler. Everything is proven but the named obligation `LocalCompat`. -/
def local_correct_sampler (hnz : ∀ p ∈ ps, p.n ≠ ⟨0, 0, 0⟩) (ws : List Nat) (hws : ∀ k ∈ ws, k < ps.length)
    (hc : LocalCompat ps (brepA ss) (wallIn ws)) (rule : Rule) :=
  local_correct_walls (X := Sampler.sampler ps) hnz ws hws hc
    (samplesIn_of (samples_of_covers (Sampler.sampler_covers hnz))) rule

end ForgeExact.DecideOnce.Sample
