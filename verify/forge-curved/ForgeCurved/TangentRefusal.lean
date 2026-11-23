import ForgeCurved.ContactModel

/-!
# Two cylinders tangent along a line: four wings, and the refusal

Two cylinders `(x ∓ 1)² + y² = 1` touch along the `z` axis. In the face ledger (W2), the outer
facet of each one reaches the contact edge from both sides, so the edge row lists four wings:
`[f₁, f₁, f₂, f₂]`.

**`tangent_edgeLocal`**: at every point of the contact line, F1–F4 (`EdgeLocal`) hold for that
row, for any face geometry whose facet points near the line are the two cylinders off the line
and whose two outer facets are `f₁` and `f₂` there. The wings are the four cells of the
seamed reference (`tangentPair`): `(0, +, ±)` and `(+, 0, ±)`. Local connectivity is
`tangentPair_localConn`.

**`tangent_refused`**: when both outer facets bound the result (the union of the two solid
cylinders, for one), the row has four boundary wings, so `BadEdgeW` holds (the core refuses,
`touch_iff_xNonManifoldF`) and the result has three local sheets there (`ThreeSheetsS`). The
refusal is correct: the union is not a manifold along the contact line.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set FaceLocal Filter

/-- The four wings at the contact line. -/
def tangentWings : List Cov :=
  [[.zero, .pos, .pos], [.zero, .pos, .neg], [.pos, .zero, .pos], [.pos, .zero, .neg]]

theorem covOfK_tangentPair (p : R3) : covOfK tangentPair p =
    [sgnK ((p.1 - 1) ^ 2 + p.2.1 ^ 2 - 1), sgnK ((p.1 + 1) ^ 2 + p.2.1 ^ 2 - 1), sgnK p.2.1] := rfl

/-- The neighbourhood of the contact line used. -/
def tW : Set R3 := {p | |p.1| < 1 / 2 ∧ |p.2.1| < 1 / 2}

theorem isOpen_tW : IsOpen tW :=
  (isOpen_lt (continuous_abs.comp continuous_fst) continuous_const).inter
    (isOpen_lt (continuous_abs.comp (continuous_fst.comp continuous_snd)) continuous_const)

/-- On the first cylinder off the line, near it, the point is on wing 0 or 1. -/
theorem cov_on_c1 {y : R3} (hy : y ∈ tW) (h1 : (y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 = 0)
    (hs : y.2.1 ≠ 0) : covOfK tangentPair y = [.zero, .pos, sgnK y.2.1] := by
  have hp : 0 < y.2.1 ^ 2 := by positivity
  have hx : 0 < y.1 := by nlinarith [abs_lt.mp hy.1]
  have h2 : 0 < (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 := by nlinarith
  rw [covOfK_tangentPair, h1, sgnK_pos h2]
  simp [sgnK]

theorem cov_on_c2 {y : R3} (hy : y ∈ tW) (h2 : (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 = 0)
    (hs : y.2.1 ≠ 0) : covOfK tangentPair y = [.pos, .zero, sgnK y.2.1] := by
  have hp : 0 < y.2.1 ^ 2 := by positivity
  have hx : y.1 < 0 := by nlinarith [abs_lt.mp hy.1]
  have h1 : 0 < (y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 := by nlinarith
  rw [covOfK_tangentPair, h2, sgnK_pos h1]
  simp [sgnK]

/-- The wing points `(±(1 - √(1 - t²)), t, z₀)`. -/
noncomputable def wingPt (e : ℝ) (z₀ t : ℝ) : R3 := (e * (1 - Real.sqrt (1 - t ^ 2)), t, z₀)

theorem wingPt_cont (e z₀ : ℝ) : Continuous (wingPt e z₀) := by unfold wingPt; fun_prop

theorem wingPt_zero (e z₀ : ℝ) : wingPt e z₀ 0 = (0, 0, z₀) := by simp [wingPt]

theorem wingPt_mem {e z₀ t : ℝ} (he : e = 1 ∨ e = -1) (ht : 0 < |t|) (ht' : |t| < 1 / 2) :
    wingPt e z₀ t ∈ tW ∧ (if e = 1 then ((wingPt e z₀ t).1 - 1) ^ 2 + (wingPt e z₀ t).2.1 ^ 2 - 1
      else ((wingPt e z₀ t).1 + 1) ^ 2 + (wingPt e z₀ t).2.1 ^ 2 - 1) = 0 := by
  have ht2 : t ^ 2 < 1 / 4 := by nlinarith [abs_lt.mp ht', sq_abs t]
  set s := Real.sqrt (1 - t ^ 2)
  have hs : s ^ 2 = 1 - t ^ 2 := Real.sq_sqrt (by linarith)
  have hs0 : 0 ≤ s := Real.sqrt_nonneg _
  have hs1 : s ≤ 1 := by nlinarith
  have hs2 : 1 / 2 < s := by nlinarith
  refine ⟨⟨?_, ht'⟩, ?_⟩
  · simp only [wingPt]
    rcases he with rfl | rfl
    · rw [abs_lt]; constructor <;> nlinarith
    · rw [abs_lt]; constructor <;> nlinarith
  · simp only [wingPt]
    rcases he with rfl | rfl
    · simp only [if_true]; nlinarith
    · simp only [show (-1 : ℝ) ≠ 1 by norm_num, if_false]; nlinarith

/-- The wing curves lie in the wings. -/
theorem wing_img {e σ z₀ : ℝ} (he : e = 1 ∨ e = -1) (hσ : σ = 1 ∨ σ = -1) :
    (fun t => wingPt e z₀ (σ * t)) '' Ioo 0 (1 / 2) ⊆
      cell tangentPair (if e = 1 then [.zero, .pos, sgnK σ] else [.pos, .zero, sgnK σ]) := by
  rintro _ ⟨t, ht, rfl⟩
  have hat : |σ * t| = t := by
    rcases hσ with rfl | rfl
    · rw [one_mul, abs_of_pos ht.1]
    · rw [neg_one_mul, abs_neg, abs_of_pos ht.1]
  obtain ⟨hW, hc⟩ := wingPt_mem (z₀ := z₀) he (by rw [hat]; exact ht.1) (by rw [hat]; exact ht.2)
  have hs : (wingPt e z₀ (σ * t)).2.1 = σ * t := rfl
  have hσ0 : σ ≠ 0 := by rcases hσ with rfl | rfl <;> norm_num
  have hne : (wingPt e z₀ (σ * t)).2.1 ≠ 0 := by rw [hs]; exact mul_ne_zero hσ0 ht.1.ne'
  have hsg : sgnK (wingPt e z₀ (σ * t)).2.1 = sgnK σ := by rw [hs, sgnK_mul_pos' ht.1]
  rw [mem_cell]
  by_cases h1 : e = 1
  · rw [if_pos h1] at hc ⊢; rw [cov_on_c1 hW hc hne, hsg]
  · rw [if_neg h1] at hc ⊢; rw [cov_on_c2 hW hc hne, hsg]

/-- Each wing reaches every point of the contact line. -/
theorem wing_near (z₀ : ℝ) :
    ∀ φ ∈ tangentWings, ((0 : ℝ), (0 : ℝ), z₀) ∈ closure (cell tangentPair φ) := by
  have key : ∀ (e σ : ℝ), ((0 : ℝ), (0 : ℝ), z₀) ∈
      closure ((fun t => wingPt e z₀ (σ * t)) '' Ioo 0 (1 / 2)) := by
    intro e σ
    have hc : Continuous fun t => wingPt e z₀ (σ * t) := (wingPt_cont e z₀).comp (by fun_prop)
    have h0 : (fun t => wingPt e z₀ (σ * t)) 0 = (0, 0, z₀) := by simp [wingPt_zero]
    rw [← h0]
    refine mem_closure_of_tendsto ((hc.tendsto 0).mono_left (nhdsWithin_le_nhds (s := Ioi 0))) ?_
    filter_upwards [Ioo_mem_nhdsGT (by norm_num : (0 : ℝ) < 1 / 2)] with t ht
    exact ⟨t, ht, rfl⟩
  have s1 : sgnK (1 : ℝ) = .pos := sgnK_pos one_pos
  have s2 : sgnK (-1 : ℝ) = .neg := sgnK_neg (by norm_num)
  intro φ hφ
  simp only [tangentWings, List.mem_cons, List.not_mem_nil, or_false] at hφ
  rcases hφ with rfl | rfl | rfl | rfl
  · have := closure_mono (wing_img (z₀ := z₀) (Or.inl rfl) (Or.inl rfl)) (key 1 1)
    rwa [if_pos rfl, s1] at this
  · have := closure_mono (wing_img (z₀ := z₀) (Or.inl rfl) (Or.inr rfl)) (key 1 (-1))
    rwa [if_pos rfl, s2] at this
  · have := closure_mono (wing_img (z₀ := z₀) (Or.inr rfl) (Or.inl rfl)) (key (-1) 1)
    rwa [if_neg (by norm_num), s1] at this
  · have := closure_mono (wing_img (z₀ := z₀) (Or.inr rfl) (Or.inr rfl)) (key (-1) (-1))
    rwa [if_neg (by norm_num), s2] at this

variable {P : Type}

/-- The entries of a covector of the pair. -/
theorem tangent_entries {y : R3} {a b c : Sgn} (h : covOfK tangentPair y = [a, b, c]) :
    sgnK ((y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1) = a ∧ sgnK ((y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1) = b ∧
      sgnK y.2.1 = c := by
  rw [covOfK_tangentPair] at h
  simp only [List.cons.injEq, and_true] at h
  exact h

theorem sgnK_ne_zero {r : ℝ} (h : r ≠ 0) : sgnK r = .pos ∨ sgnK r = .neg := by
  rcases lt_or_gt_of_ne h with h | h
  · exact Or.inr (sgnK_neg h)
  · exact Or.inl (sgnK_pos h)

/-- **The local model at a point of the contact line**, for any face geometry whose facet points
near it are the two cylinders off the line, with outer facets `f₁` and `f₂`. -/
noncomputable def tangentModel {G : FaceGeo R3} {f₁ f₂ : Nat} (z₀ : ℝ)
    (hF : ∀ y ∈ tW, y ∈ FacetPts G →
      ((y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 ∨ (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 = 0) ∧ y.2.1 ≠ 0)
    (h₁ : ∀ y ∈ tW, (y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 → y ∈ G.facet f₁)
    (h₂ : ∀ y ∈ tW, (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 → y ∈ G.facet f₂) :
    EdgeModel G tangentPair [f₁, f₁, f₂, f₂] (0, 0, z₀) where
  W := tW
  hWo := isOpen_tW
  hxW := by norm_num [tW]
  Φ := tangentWings
  len := rfl
  nodup := by decide
  facet := by decide
  above := by
    have : covOfK tangentPair ((0 : ℝ), (0 : ℝ), z₀) = [.zero, .zero, .zero] := by
      rw [covOfK_tangentPair]; norm_num [sgnK]
    rw [this]; decide
  near := wing_near z₀
  sub i hi := by
    rintro y ⟨hy, hyW⟩
    rw [mem_cell] at hy
    simp only [tangentWings, List.length_cons, List.length_nil] at hi
    interval_cases i
    · exact h₁ y hyW (sgnK_zero_iff.mp (tangent_entries hy).1)
    · exact h₁ y hyW (sgnK_zero_iff.mp (tangent_entries hy).1)
    · exact h₂ y hyW (sgnK_zero_iff.mp (tangent_entries hy).2.1)
    · exact h₂ y hyW (sgnK_zero_iff.mp (tangent_entries hy).2.1)
  complete y hyW hyF := by
    obtain ⟨hc | hc, hs⟩ := hF y hyW hyF
    · rw [cov_on_c1 hyW hc hs]
      rcases sgnK_ne_zero hs with h | h <;> rw [h] <;> simp [tangentWings]
    · rw [cov_on_c2 hyW hc hs]
      rcases sgnK_ne_zero hs with h | h <;> rw [h] <;> simp [tangentWings]

theorem tangentPair_cont : ∀ c ∈ tangentPair, Continuous c := by
  intro c hc
  simp only [tangentPair, List.mem_cons, List.not_mem_nil, or_false] at hc
  rcases hc with rfl | rfl | rfl <;> fun_prop

/-- **F1–F4 at every point of the contact line**: four wings, `[f₁, f₁, f₂, f₂]`. -/
theorem tangent_edgeLocal {G : FaceGeo R3} {f₁ f₂ : Nat} (z₀ : ℝ)
    (hF : ∀ y ∈ tW, y ∈ FacetPts G →
      ((y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 ∨ (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 = 0) ∧ y.2.1 ≠ 0)
    (h₁ : ∀ y ∈ tW, (y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 → y ∈ G.facet f₁)
    (h₂ : ∀ y ∈ tW, (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 → y ∈ G.facet f₂) :
    Nonempty (EdgeLocal G [f₁, f₁, f₂, f₂] (0, 0, z₀)) :=
  edgeLocal_of_model tangentPair_cont (tangentModel z₀ hF h₁ h₂) (tangentPair_localConn z₀)

/-- **The refusal is correct**: when both outer facets bound the result, the contact edge has
four boundary wings (the core refuses) and the result has three local sheets there. -/
theorem tangent_refused {G : FaceGeo R3} {keep : Nat → Bool} {f₁ f₂ : Nat} (z₀ : ℝ)
    (hs : SideLocal G keep)
    (hF : ∀ y ∈ tW, y ∈ FacetPts G →
      ((y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 ∨ (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 = 0) ∧ y.2.1 ≠ 0)
    (h₁ : ∀ y ∈ tW, (y.1 - 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 → y ∈ G.facet f₁)
    (h₂ : ∀ y ∈ tW, (y.1 + 1) ^ 2 + y.2.1 ^ 2 - 1 = 0 → y ∈ G.facet f₂)
    (b₁ : Bd G keep f₁) (b₂ : Bd G keep f₂) :
    BadEdgeW G keep [f₁, f₁, f₂, f₂] ∧ ThreeSheetsS G (Res G keep) (0, 0, z₀) := by
  have hb : BadEdgeW G keep [f₁, f₁, f₂, f₂] :=
    ⟨0, 1, 2, by simp, by simp, by simp, by simp, by simp, by simp, b₁, b₁, b₂⟩
  obtain ⟨h⟩ := tangent_edgeLocal z₀ hF h₁ h₂
  exact ⟨hb, (edge_iff hs h).mp hb⟩

end ForgeCurved
