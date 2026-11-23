import ForgeCurved.Homogeneous
import ForgeCurved.EndToEnd

/-!
# The cone model: a cone apex with two axial seams and its nappe plane is conical

In coordinates `u` along the two seam normals and the axis, a circular (or elliptic) cone with
apex at the origin is `a u₁² + b u₂² - c u₃²`, the two axial seams are `u₁` and `u₂`, and the nappe
plane is `u₃`. Every sign class of these four (`modelCell`) is connected
(**`modelCell_preconnected`**): with the signs of the `uᵢ` fixed, `pᵢ = uᵢ²` turns it into a
convex set (an orthant face cut by the sign of the linear form `a p₁ + b p₂ - c p₃`), and
`p ↦ (sᵢ √pᵢ)` maps that back onto it.

**`conicalAt_of_model`**: a point whose carriers are, through a homeomorphism `T` homogeneous about
it, sign-equivalent to the model's four functions (each one used) is conical. With only one
axial seam, or without the nappe plane, a model function is unused and the statement does not
apply. Indeed the cells are not connected then: the two nappes of `u₃ ≠ 0` share a covector.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set

/-- The model's four functions: the cone and its three coordinate planes. -/
def modelF (a b c : ℝ) : Fin 4 → R3 → ℝ
  | 0 => fun u => a * u.1 ^ 2 + b * u.2.1 ^ 2 - c * u.2.2 ^ 2
  | 1 => fun u => u.1
  | 2 => fun u => u.2.1
  | 3 => fun u => u.2.2

/-- A sign class of the model. -/
def modelCell (a b c : ℝ) (σ : Fin 4 → Sgn) : Set R3 := {u | ∀ k, sgnK (modelF a b c k u) = σ k}

/-- The sign of a coordinate, once squared. -/
def sq : Sgn → Sgn
  | .zero => .zero
  | _ => .pos

theorem sgnVal_sq (s : Sgn) : sgnVal s ^ 2 = if s = .zero then 0 else 1 := by
  cases s <;> simp [sgnVal]

/-- `s √p`: back from a square, with the sign `s`. -/
noncomputable def unsq (s : Sgn) (p : ℝ) : ℝ := sgnVal s * Real.sqrt p

theorem sgnK_unsq {s : Sgn} {p : ℝ} (hp : sgnK p = sq s) : sgnK (unsq s p) = s := by
  unfold unsq
  cases s
  · simp only [sq] at hp
    have : 0 < p := by
      by_contra h; push Not at h
      rcases h.lt_or_eq with h | h
      · rw [sgnK_neg h] at hp; cases hp
      · rw [h] at hp; simp [sgnK] at hp
    simp only [sgnVal, neg_mul, one_mul]
    exact sgnK_neg (by have := Real.sqrt_pos.mpr this; linarith)
  · simp [sgnVal, sgnK]
  · simp only [sq] at hp
    have : 0 < p := by
      by_contra h; push Not at h
      rcases h.lt_or_eq with h | h
      · rw [sgnK_neg h] at hp; cases hp
      · rw [h] at hp; simp [sgnK] at hp
    simp only [sgnVal, one_mul]
    exact sgnK_pos (Real.sqrt_pos.mpr this)

theorem sq_unsq {s : Sgn} {p : ℝ} (hp : sgnK p = sq s) : unsq s p ^ 2 = p := by
  unfold unsq
  have hp0 : 0 ≤ p := by
    by_contra h; push Not at h
    rw [sgnK_neg h] at hp; cases s <;> cases hp
  rw [mul_pow, Real.sq_sqrt hp0, sgnVal_sq]
  cases s
  · simp
  · simp only [sq] at hp; simp only [ite_true, zero_mul]; exact (sgnK_zero_iff.mp hp).symm
  · simp

theorem sgnK_sq {s : Sgn} {u : ℝ} (hu : sgnK u = s) : sgnK (u ^ 2) = sq s := by
  subst hu
  rcases lt_trichotomy u 0 with h | h | h
  · rw [sgnK_neg h, sgnK_pos (by nlinarith)]; rfl
  · subst h; simp [sgnK, sq]
  · rw [sgnK_pos h, sgnK_pos (by positivity)]; rfl

theorem unsq_sq {s : Sgn} {u : ℝ} (hu : sgnK u = s) : unsq s (u ^ 2) = u := by
  unfold unsq
  rw [Real.sqrt_sq_eq_abs]
  subst hu
  rcases lt_trichotomy u 0 with h | h | h
  · rw [sgnK_neg h, abs_of_neg h]; simp [sgnVal]
  · subst h; simp
  · rw [sgnK_pos h, abs_of_pos h]; simp [sgnVal]

/-- The convex set of squares. -/
def sqCell (a b c : ℝ) (σ : Fin 4 → Sgn) : Set R3 :=
  {p | sgnK p.1 = sq (σ 1) ∧ sgnK p.2.1 = sq (σ 2) ∧ sgnK p.2.2 = sq (σ 3) ∧
    sgnK (a * p.1 + b * p.2.1 - c * p.2.2) = σ 0}

theorem convex_sqCell (a b c : ℝ) (σ : Fin 4 → Sgn) : Convex ℝ (sqCell a b c σ) := by
  have e : sqCell a b c σ =
      (LinearMap.fst ℝ ℝ (ℝ × ℝ)) ⁻¹' {r | sgnK r = sq (σ 1)} ∩
      ((LinearMap.fst ℝ ℝ ℝ ∘ₗ LinearMap.snd ℝ ℝ (ℝ × ℝ)) ⁻¹' {r | sgnK r = sq (σ 2)} ∩
      ((LinearMap.snd ℝ ℝ ℝ ∘ₗ LinearMap.snd ℝ ℝ (ℝ × ℝ)) ⁻¹' {r | sgnK r = sq (σ 3)} ∩
      ((a • LinearMap.fst ℝ ℝ (ℝ × ℝ) + b • (LinearMap.fst ℝ ℝ ℝ ∘ₗ LinearMap.snd ℝ ℝ (ℝ × ℝ)) -
        c • (LinearMap.snd ℝ ℝ ℝ ∘ₗ LinearMap.snd ℝ ℝ (ℝ × ℝ))) ⁻¹' {r | sgnK r = σ 0}))) := by
    ext p; simp [sqCell]
  rw [e]
  exact ((convex_sgnK _).linear_preimage _).inter (((convex_sgnK _).linear_preimage _).inter
    (((convex_sgnK _).linear_preimage _).inter ((convex_sgnK _).linear_preimage _)))

/-- The map back. -/
noncomputable def unsqMap (σ : Fin 4 → Sgn) (p : R3) : R3 :=
  (unsq (σ 1) p.1, unsq (σ 2) p.2.1, unsq (σ 3) p.2.2)

theorem continuous_unsqMap (σ : Fin 4 → Sgn) : Continuous (unsqMap σ) := by
  unfold unsqMap unsq; fun_prop

theorem modelCell_eq_image (a b c : ℝ) (σ : Fin 4 → Sgn) :
    modelCell a b c σ = unsqMap σ '' sqCell a b c σ := by
  ext u
  constructor
  · intro hu
    have h1 := hu 1; have h2 := hu 2; have h3 := hu 3; have h0 := hu 0
    simp only [modelF] at h0 h1 h2 h3
    refine ⟨(u.1 ^ 2, u.2.1 ^ 2, u.2.2 ^ 2), ⟨sgnK_sq h1, sgnK_sq h2, sgnK_sq h3, h0⟩, ?_⟩
    simp only [unsqMap, unsq_sq h1, unsq_sq h2, unsq_sq h3]
  · rintro ⟨p, ⟨h1, h2, h3, h0⟩, rfl⟩ k
    fin_cases k
    · simp only [modelF, unsqMap, sq_unsq h1, sq_unsq h2, sq_unsq h3]; exact h0
    · exact sgnK_unsq h1
    · exact sgnK_unsq h2
    · exact sgnK_unsq h3

/-- **Every sign class of the cone model is connected.** -/
theorem modelCell_preconnected (a b c : ℝ) (σ : Fin 4 → Sgn) :
    IsPreconnected (modelCell a b c σ) := by
  rw [modelCell_eq_image]
  exact (convex_sqCell a b c σ).isPreconnected.image _ (continuous_unsqMap σ).continuousOn

theorem sgnK_mul_pos {t : ℝ} (ht : 0 < t) (r : ℝ) : sgnK (t * r) = sgnK r := by
  rcases lt_trichotomy r 0 with h | h | h
  · rw [sgnK_neg h, sgnK_neg (mul_neg_of_pos_of_neg ht h)]
  · rw [h, mul_zero]
  · rw [sgnK_pos h, sgnK_pos (mul_pos ht h)]

theorem modelF_hom (a b c : ℝ) (k : Fin 4) (u : R3) {t : ℝ} (ht : 0 < t) :
    sgnK (modelF a b c k (t • u)) = sgnK (modelF a b c k u) := by
  fin_cases k <;> simp only [modelF, Prod.smul_fst, Prod.smul_snd, smul_eq_mul]
  · have : a * (t * u.1) ^ 2 + b * (t * u.2.1) ^ 2 - c * (t * u.2.2) ^ 2 =
        t ^ 2 * (a * u.1 ^ 2 + b * u.2.1 ^ 2 - c * u.2.2 ^ 2) := by ring
    rw [this, sgnK_mul_pos (by positivity)]
  all_goals exact sgnK_mul_pos ht _

/-- **A point modelled on the seamed cone is conical**: through a homeomorphism `T`, homogeneous
about `x` and sending `x` to the apex, each carrier through `x` has the sign of one of the model's
four functions, and each of the four is used. -/
theorem conicalAt_of_model {cs : List (Carrier R3 ℝ)} {x : R3}
    (hnf : (covOfK cs x).isFacet = false) (a b c : ℝ) (T : R3 ≃ₜ R3)
    (hT : ∀ y (t : ℝ), 0 < t → T (x + t • (y - x)) = t • T y) (m : Fin cs.length → Fin 4)
    (hm : ∀ i : Fin cs.length, cs[i] x = 0 → ∀ y, sgnK (cs[i] y) = sgnK (modelF a b c (m i) (T y)))
    (hsurj : ∀ k, ∃ i : Fin cs.length, cs[i] x = 0 ∧ m i = k) : ConicalAt cs x where
  notFacet := hnf
  hom i hi y t ht := by
    rw [hm i hi, hm i hi, hT y t ht, modelF_hom a b c _ _ ht]
  cone Y hY := by
    classical
    obtain ⟨hlen, -⟩ := le_iff_getElem.mp hY
    rw [length_covOfK] at hlen
    by_cases hcons : ∀ i j : Fin cs.length, cs[i] x = 0 → cs[j] x = 0 → m i = m j →
        Y[i.1]'(hlen ▸ i.2) = Y[j.1]'(hlen ▸ j.2)
    · choose ik hik using hsurj
      set σ : Fin 4 → Sgn := fun k => Y[(ik k).1]'(hlen ▸ (ik k).2)
      have he : coneCell cs x Y = T ⁻¹' modelCell a b c σ := by
        ext y
        constructor
        · intro hy k
          have h1 := hy (ik k).1 (ik k).2 (hlen ▸ (ik k).2) (hik k).1
          have h2 := hm (ik k) (hik k).1 y
          rw [(hik k).2] at h2
          rw [← h2]
          exact h1
        · intro hy i hi hi' hx
          have h2 := hm ⟨i, hi⟩ hx y
          simp only [Fin.getElem_fin] at h2
          rw [h2, hy (m ⟨i, hi⟩)]
          exact (hcons ⟨i, hi⟩ (ik (m ⟨i, hi⟩)) hx (hik _).1 (hik _).2.symm).symm
      rw [he, ← T.image_symm]
      exact (modelCell_preconnected a b c σ).image _ T.symm.continuous.continuousOn
    · push Not at hcons
      obtain ⟨i, j, hi, hj, hij, hne⟩ := hcons
      have : coneCell cs x Y = ∅ := by
        ext y
        simp only [mem_empty_iff_false, iff_false]
        intro hy
        apply hne
        rw [← hy i.1 i.2 (hlen ▸ i.2) hi, ← hy j.1 j.2 (hlen ▸ j.2) hj]
        have e1 := hm i hi y
        have e2 := hm j hj y
        rw [hij] at e1
        exact e1.trans e2.symm
      rw [this]; exact isPreconnected_empty

end ForgeCurved
