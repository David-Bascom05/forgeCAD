import ForgeCurved.Bridge

/-!
# The cells of continuous carriers

The curved spec's cells (`covCellK`, Curved.lean) are sign classes of carriers `P → ℝ`. When the
carriers are continuous, three facts hold with no further hypothesis, and they are what the flat
proof got from convexity without needing it:

* **persistence** (`eventually_le`): near a point `x`, every point's covector is above `x`'s
  (`Cov.le`): a carrier nonzero at `x` keeps its sign nearby;
* so **full cells are open** (`full_nhds`), and near a point of a facet the only cells are the
  facet and its two sides (`facet_above`);
* **closure gives `Cov.le`** (`le_of_mem_closure`): a point in the closure of a cell is a face of
  it. The converse is R2 (`Faithful.incidence`), the part that needs the separators.

The covector order lemmas (`le_iff`, `le_trans`, `eq_of_le_of_count`) are pure combinatorics.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Filter Topology

/-! ## The covector order -/

/-- The pointwise relation of `Cov.le`. -/
def Below (a b : Sgn) : Prop := a = .zero ∨ a = b

theorem le_nil_nil : Cov.le [] [] = true := rfl

theorem le_cons (a b : Sgn) (X Y : Cov) :
    Cov.le (a :: X) (b :: Y) = ((a == .zero || a == b) && Cov.le X Y) := by
  unfold Cov.le
  simp only [List.length_cons, List.zip_cons_cons, List.all_cons]
  cases h : (X.length == Y.length) <;> simp_all

theorem le_iff : ∀ (X Y : Cov), Cov.le X Y = true ↔ List.Forall₂ Below X Y
  | [], [] => by simp [le_nil_nil]
  | [], _ :: _ => by simp [Cov.le]
  | _ :: _, [] => by simp [Cov.le]
  | a :: X, b :: Y => by
    rw [le_cons, List.forall₂_cons, ← le_iff X Y]
    simp [Below]

theorem le_refl (X : Cov) : Cov.le X X = true :=
  (le_iff X X).mpr (List.forall₂_same.mpr fun _ _ => Or.inr rfl)

theorem le_trans {X Y Z : Cov} (h₁ : Cov.le X Y = true) (h₂ : Cov.le Y Z = true) :
    Cov.le X Z = true := by
  rw [le_iff] at *
  induction h₁ generalizing Z with
  | nil => cases h₂; exact List.Forall₂.nil
  | cons hab _ ih =>
    cases h₂ with
    | cons hbc h₂ =>
      refine List.Forall₂.cons ?_ (ih h₂)
      rcases hab with h | h
      · exact Or.inl h
      · subst h; exact hbc

theorem length_eq_of_le {X Y : Cov} (h : Cov.le X Y = true) : X.length = Y.length :=
  ((le_iff X Y).mp h).length_eq

/-- Going up the order, zeros only disappear. -/
theorem count_le_of_le {X Y : Cov} (h : Cov.le X Y = true) :
    Y.count .zero ≤ X.count .zero := by
  rw [le_iff] at h
  induction h with
  | nil => simp
  | @cons a b X Y hab _ ih =>
    rcases hab with h | h
    · subst h; simp only [List.count_cons_self]
      by_cases hb : b = .zero
      · subst hb; simp only [List.count_cons_self]; omega
      · rw [List.count_cons_of_ne hb]; omega
    · subst h
      by_cases hb : a = .zero
      · subst hb; simp only [List.count_cons_self]; omega
      · rw [List.count_cons_of_ne hb, List.count_cons_of_ne hb]; omega

/-- **A cell above `X` with as many zeros is `X`.** -/
theorem eq_of_le_of_count {X Y : Cov} (h : Cov.le X Y = true)
    (hc : X.count .zero ≤ Y.count .zero) : X = Y := by
  rw [le_iff] at h
  induction h with
  | nil => rfl
  | @cons a b X Y hab hr ih =>
    have hle : Y.count .zero ≤ X.count .zero := count_le_of_le ((le_iff X Y).mpr hr)
    rcases hab with h | h
    · subst h
      by_cases hb : b = .zero
      · subst hb; simp only [List.count_cons_self] at hc; rw [ih (by omega)]
      · rw [List.count_cons_self, List.count_cons_of_ne hb] at hc; omega
    · subst h
      by_cases hb : a = .zero
      · subst hb; simp only [List.count_cons_self] at hc; rw [ih (by omega)]
      · rw [List.count_cons_of_ne hb, List.count_cons_of_ne hb] at hc
        rw [ih hc]

theorem isFacet_iff (X : Cov) : X.isFacet = true ↔ X.count .zero = 1 := by
  simp [Cov.isFacet]

/-- Two facets in order are equal. -/
theorem facet_eq_of_le {X Y : Cov} (h : Cov.le X Y = true) (hX : X.isFacet = true)
    (hY : Y.isFacet = true) : X = Y :=
  eq_of_le_of_count h (by rw [(isFacet_iff X).mp hX, (isFacet_iff Y).mp hY])

theorem count_pos_of_mem {X : Cov} (h : Sgn.zero ∈ X) : 0 < X.count .zero :=
  List.count_pos_iff.mpr h

/-- **A full cell above a facet is one of its two sides.** -/
theorem full_above_facet {φ Y : Cov} (h : Cov.le φ Y = true) (hφ : φ.isFacet = true)
    (hY : Sgn.zero ∉ Y) : Y = φ.plus ∨ Y = φ.minus := by
  rw [isFacet_iff] at hφ
  rw [le_iff] at h
  induction h with
  | nil => simp at hφ
  | @cons a b X Y hab hr ih =>
    have hY' : Sgn.zero ∉ Y := fun hm => hY (List.mem_cons_of_mem _ hm)
    have hb : b ≠ .zero := fun hb => hY (hb ▸ List.mem_cons_self ..)
    by_cases ha : a = .zero
    · subst ha
      simp only [List.count_cons_self] at hφ
      have hXY : X = Y := eq_of_le_of_count ((le_iff X Y).mpr hr)
        (by have := List.count_eq_zero.mpr hY'; omega)
      subst hXY
      simp only [Cov.plus, Cov.minus, Cov.zeroIdx, List.idxOf_cons_self, List.set_cons_zero]
      cases b
      · exact Or.inr rfl
      · exact absurd rfl hb
      · exact Or.inl rfl
    · rcases hab with h | h
      · exact absurd h ha
      subst h
      rw [List.count_cons_of_ne ha] at hφ
      have hz : Cov.zeroIdx (a :: X) = Cov.zeroIdx X + 1 := by
        simp only [Cov.zeroIdx]
        rw [List.idxOf_cons_ne _ (by simpa using ha)]
      simp only [Cov.plus, Cov.minus, hz, List.set_cons_succ]
      rcases ih hφ hY' with e | e
      · exact Or.inl (by rw [e]; rfl)
      · exact Or.inr (by rw [e]; rfl)

/-- A facet's sides are full. -/
theorem facet_side_full {φ : Cov} (hφ : φ.isFacet = true) :
    Sgn.zero ∉ φ.plus ∧ Sgn.zero ∉ φ.minus := by
  rw [isFacet_iff] at hφ
  induction φ with
  | nil => simp at hφ
  | cons a X ih =>
    by_cases ha : a = .zero
    · subst ha
      simp only [List.count_cons_self] at hφ
      have h0 : Sgn.zero ∉ X := fun hm => by have := count_pos_of_mem hm; omega
      simp only [Cov.plus, Cov.minus, Cov.zeroIdx, List.idxOf_cons_self, List.set_cons_zero]
      exact ⟨by simpa using h0, by simpa using h0⟩
    · rw [List.count_cons_of_ne ha] at hφ
      have hz : Cov.zeroIdx (a :: X) = Cov.zeroIdx X + 1 := by
        simp only [Cov.zeroIdx]
        rw [List.idxOf_cons_ne _ (by simpa using ha)]
      simp only [Cov.plus, Cov.minus, hz, List.set_cons_succ]
      obtain ⟨h1, h2⟩ := ih hφ
      exact ⟨by simpa [Ne.symm ha, Cov.plus] using h1, by simpa [Ne.symm ha, Cov.minus] using h2⟩

/-- A facet's sides are above it. -/
theorem le_plus_minus {φ : Cov} (hφ : φ.isFacet = true) :
    Cov.le φ φ.plus = true ∧ Cov.le φ φ.minus = true := by
  rw [isFacet_iff] at hφ
  rw [le_iff, le_iff]
  induction φ with
  | nil => simp at hφ
  | cons a X ih =>
    by_cases ha : a = .zero
    · subst ha
      simp only [Cov.plus, Cov.minus, Cov.zeroIdx, List.idxOf_cons_self, List.set_cons_zero]
      exact ⟨List.Forall₂.cons (Or.inl rfl) (List.forall₂_same.mpr fun _ _ => Or.inr rfl),
        List.Forall₂.cons (Or.inl rfl) (List.forall₂_same.mpr fun _ _ => Or.inr rfl)⟩
    · rw [List.count_cons_of_ne ha] at hφ
      have hz : Cov.zeroIdx (a :: X) = Cov.zeroIdx X + 1 := by
        simp only [Cov.zeroIdx]
        rw [List.idxOf_cons_ne _ (by simpa using ha)]
      simp only [Cov.plus, Cov.minus, hz, List.set_cons_succ]
      obtain ⟨h1, h2⟩ := ih hφ
      exact ⟨List.Forall₂.cons (Or.inr rfl) h1, List.Forall₂.cons (Or.inr rfl) h2⟩

/-! ## Persistence -/

variable {P : Type} [TopologicalSpace P]

theorem covOfK_cons (c : Carrier P ℝ) (cs : List (Carrier P ℝ)) (x : P) :
    covOfK (c :: cs) x = sgnK (c x) :: covOfK cs x := rfl

theorem length_covOfK (cs : List (Carrier P ℝ)) (x : P) : (covOfK cs x).length = cs.length := by
  simp [covOfK]

theorem sgnK_pos {q : ℝ} (h : 0 < q) : sgnK q = .pos := by simp [sgnK, h]
theorem sgnK_neg {q : ℝ} (h : q < 0) : sgnK q = .neg := by
  simp [sgnK, h, not_lt.mpr h.le]
theorem sgnK_zero_iff {q : ℝ} : sgnK q = .zero ↔ q = 0 := by
  unfold sgnK
  rcases lt_trichotomy q 0 with h | h | h
  · simp [h, not_lt.mpr h.le, h.ne]
  · simp [h]
  · simp [h, h.ne']

/-- **Persistence**: near `x`, every point's covector is above `x`'s. -/
theorem eventually_le {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c) (x : P) :
    ∀ᶠ y in 𝓝 x, Cov.le (covOfK cs x) (covOfK cs y) = true := by
  induction cs with
  | nil => exact Filter.Eventually.of_forall fun _ => rfl
  | cons c cs ih =>
    have hcc : Continuous c := hc c (List.mem_cons_self ..)
    have ih := ih fun d hd => hc d (List.mem_cons_of_mem _ hd)
    have hs : ∀ᶠ y in 𝓝 x, Below (sgnK (c x)) (sgnK (c y)) := by
      rcases lt_trichotomy (c x) 0 with h | h | h
      · filter_upwards [hcc.continuousAt.eventually (gt_mem_nhds h)] with y hy
        exact Or.inr (by rw [sgnK_neg h, sgnK_neg hy])
      · exact Filter.Eventually.of_forall fun y => Or.inl (sgnK_zero_iff.mpr h)
      · filter_upwards [hcc.continuousAt.eventually (lt_mem_nhds h)] with y hy
        exact Or.inr (by rw [sgnK_pos h, sgnK_pos hy])
    filter_upwards [hs, ih] with y h1 h2
    rw [covOfK_cons, covOfK_cons, le_iff] at *
    exact List.Forall₂.cons h1 h2

/-- Persistence, as an open neighbourhood. -/
theorem exists_open_le {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c) (x : P) :
    ∃ N : Set P, IsOpen N ∧ x ∈ N ∧ ∀ y ∈ N, Cov.le (covOfK cs x) (covOfK cs y) = true := by
  obtain ⟨N, hN, ho, hx⟩ := eventually_nhds_iff.mp (eventually_le hc x)
  exact ⟨N, ho, hx, hN⟩

/-- The set of a cell, as a Mathlib set. -/
def cell (cs : List (Carrier P ℝ)) (X : Cov) : Set P := {y | covOfK cs y = X}

theorem mem_cell {cs : List (Carrier P ℝ)} {X : Cov} {y : P} : y ∈ cell cs X ↔ covOfK cs y = X :=
  Iff.rfl

theorem cell_eq (cs : List (Carrier P ℝ)) (X : Cov) : {y | covCellK cs X y} = cell cs X := rfl

/-- **Full cells are open.** -/
theorem full_open {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c) {T : Cov}
    (hT : Sgn.zero ∉ T) : IsOpen (cell cs T) := by
  rw [isOpen_iff_mem_nhds]
  intro x hx
  filter_upwards [eventually_le hc x] with y hy
  rw [mem_cell] at hx ⊢
  rw [hx] at hy
  exact (eq_of_le_of_count hy (by rw [List.count_eq_zero.mpr hT]; omega)).symm

/-- **Closure gives `Cov.le`**: a point in the closure of a cell is a face of it. -/
theorem le_of_mem_closure {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c) {X : Cov}
    {x : P} (hx : x ∈ closure (cell cs X)) : Cov.le (covOfK cs x) X = true := by
  obtain ⟨y, hy, hyX⟩ := (mem_closure_iff_nhds.mp hx) _ (eventually_le hc x)
  rw [mem_cell] at hyX
  rwa [← hyX]

/-- The wall, as a Mathlib set: the points with a zero. -/
theorem wallK_iff (cs : List (Carrier P ℝ)) (x : P) : wallK cs x ↔ Sgn.zero ∈ covOfK cs x := by
  simp only [wallK, covOfK, List.mem_map]

/-- A nowhere-dense wall leaves a point off it in every nonempty open set. -/
theorem exists_off_wall {cs : List (Carrier P ℝ)} (hw : ForgeExact.Topology.NowhereDense (wallK cs))
    {O : Set P} (hO : IsOpen O) {x : P} (hx : x ∈ O) : ∃ y ∈ O, Sgn.zero ∉ covOfK cs y := by
  rw [nowhereDense_iff] at hw
  by_contra hno
  push Not at hno
  have hsub : O ⊆ interior (closure {y | wallK cs y}) :=
    interior_maximal (fun y hy => subset_closure ((wallK_iff cs y).mpr (hno y hy))) hO
  rw [hw] at hsub
  exact hsub hx

end ForgeCurved
