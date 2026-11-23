import Mathlib

open RealInnerProductSpace

namespace Sag

variable {E : Type*} [NormedAddCommGroup E] [InnerProductSpace ℝ E]

/-- The squared distance from `o` along a chord whose ends are equally far from it: a parabola in
the chord's parameter, symmetric about its middle. -/
theorem chord_sq (o a b : E) (h : ‖a - o‖ = ‖b - o‖) (s : ℝ) :
    ‖(a + s • (b - a)) - o‖ ^ 2 = ‖a - o‖ ^ 2 + (s ^ 2 - s) * ‖b - a‖ ^ 2 := by
  have hb : b - o = (a - o) + (b - a) := by abel
  have h2 : ‖b - o‖ ^ 2 = ‖a - o‖ ^ 2 := by rw [h]
  rw [hb, norm_add_sq_real] at h2
  have : (a + s • (b - a)) - o = (a - o) + s • (b - a) := by abel
  rw [this, norm_add_sq_real, inner_smul_right, norm_smul, mul_pow, Real.norm_eq_abs, sq_abs]
  linear_combination s * h2

/-- A chord whose ends are equally far from `o` comes nearest `o` at its middle. -/
theorem mid_nearest (o a b : E) (h : ‖a - o‖ = ‖b - o‖) (t : ℝ) :
    ‖(a + (1 / 2 : ℝ) • (b - a)) - o‖ ≤ ‖(a + t • (b - a)) - o‖ := by
  have : ‖(a + (1 / 2 : ℝ) • (b - a)) - o‖ ^ 2 ≤ ‖(a + t • (b - a)) - o‖ ^ 2 := by
    rw [chord_sq o a b h, chord_sq o a b h]
    nlinarith [sq_nonneg (t - 1 / 2), sq_nonneg ‖b - a‖]
  exact (sq_le_sq₀ (norm_nonneg _) (norm_nonneg _)).mp this

end Sag

namespace Sag

variable {E : Type*} [NormedAddCommGroup E] [InnerProductSpace ℝ E]

/-- The circumradius in barycentric form, over the corners' offsets uᵢ = xᵢ − f from a point
f = Σ λᵢxᵢ (Σ λᵢ = 1, so Σ λᵢuᵢ = 0), each of length R: R² = Σ λᵢλⱼ |uᵢ − uⱼ|². -/
theorem circum_sq (u₁ u₂ u₃ : E) (l₁ l₂ l₃ R : ℝ) (hs : l₁ + l₂ + l₃ = 1)
    (z : l₁ • u₁ + l₂ • u₂ + l₃ • u₃ = 0)
    (h₁ : ‖u₁‖ = R) (h₂ : ‖u₂‖ = R) (h₃ : ‖u₃‖ = R) :
    R ^ 2 = l₁ * l₂ * ‖u₁ - u₂‖ ^ 2 + l₁ * l₃ * ‖u₁ - u₃‖ ^ 2 + l₂ * l₃ * ‖u₂ - u₃‖ ^ 2 := by
  have hz : ⟪l₁ • u₁ + l₂ • u₂ + l₃ • u₃, l₁ • u₁ + l₂ • u₂ + l₃ • u₃⟫ = 0 := by
    rw [z, inner_zero_left]
  simp only [inner_add_left, inner_add_right, inner_smul_left, inner_smul_right,
    conj_trivial] at hz
  rw [norm_sub_sq_real, norm_sub_sq_real, norm_sub_sq_real, h₁, h₂, h₃]
  have n1 : ⟪u₁, u₁⟫ = R ^ 2 := by rw [real_inner_self_eq_norm_sq, h₁]
  have n2 : ⟪u₂, u₂⟫ = R ^ 2 := by rw [real_inner_self_eq_norm_sq, h₂]
  have n3 : ⟪u₃, u₃⟫ = R ^ 2 := by rw [real_inner_self_eq_norm_sq, h₃]
  rw [n1, n2, n3, real_inner_comm u₁ u₂, real_inner_comm u₁ u₃, real_inner_comm u₂ u₃] at hz
  have hs2 : (l₁ + l₂ + l₃) ^ 2 = 1 := by rw [hs]; ring
  linear_combination hz - R ^ 2 * hs2

/-- The centroid's distance from the point all three corners are R from (offsets uᵢ):
|(u₁ + u₂ + u₃)/3|² = R² − (|u₁ − u₂|² + |u₁ − u₃|² + |u₂ − u₃|²)/9. -/
theorem centroid_sq (u₁ u₂ u₃ : E) (R : ℝ) (h₁ : ‖u₁‖ = R) (h₂ : ‖u₂‖ = R) (h₃ : ‖u₃‖ = R) :
    ‖(1 / 3 : ℝ) • (u₁ + u₂ + u₃)‖ ^ 2
      = R ^ 2 - (‖u₁ - u₂‖ ^ 2 + ‖u₁ - u₃‖ ^ 2 + ‖u₂ - u₃‖ ^ 2) / 9 := by
  rw [norm_smul, mul_pow, Real.norm_eq_abs, sq_abs, norm_add_sq_real, norm_add_sq_real,
    inner_add_left, norm_sub_sq_real, norm_sub_sq_real, norm_sub_sq_real, h₁, h₂, h₃]
  ring

/-- Weights summing to one: their pairwise products sum to at most a third. -/
theorem pairs_le (l₁ l₂ l₃ : ℝ) (hs : l₁ + l₂ + l₃ = 1) : l₁ * l₂ + l₁ * l₃ + l₂ * l₃ ≤ 1 / 3 := by
  nlinarith [sq_nonneg (l₁ - l₂), sq_nonneg (l₁ - l₃), sq_nonneg (l₂ - l₃)]

/-- With the point inside (weights ≥ 0), the centroid lies within √(2/9)·L of it, L the longest
side: |g − f|² ≤ (2/9)·L². -/
theorem centroid_near (u₁ u₂ u₃ : E) (l₁ l₂ l₃ R L : ℝ) (hs : l₁ + l₂ + l₃ = 1)
    (p₁ : 0 ≤ l₁) (p₂ : 0 ≤ l₂) (p₃ : 0 ≤ l₃)
    (z : l₁ • u₁ + l₂ • u₂ + l₃ • u₃ = 0)
    (h₁ : ‖u₁‖ = R) (h₂ : ‖u₂‖ = R) (h₃ : ‖u₃‖ = R)
    (e₁ : ‖u₁ - u₂‖ ≤ L) (e₂ : ‖u₁ - u₃‖ ≤ L) (e₃ : ‖u₂ - u₃‖ ≤ L)
    (big : L ≤ ‖u₁ - u₂‖ ∨ L ≤ ‖u₁ - u₃‖ ∨ L ≤ ‖u₂ - u₃‖) :
    ‖(1 / 3 : ℝ) • (u₁ + u₂ + u₃)‖ ^ 2 ≤ 2 / 9 * L ^ 2 := by
  rw [centroid_sq u₁ u₂ u₃ R h₁ h₂ h₃, circum_sq u₁ u₂ u₃ l₁ l₂ l₃ R hs z h₁ h₂ h₃]
  have n₁ := norm_nonneg (u₁ - u₂)
  have n₂ := norm_nonneg (u₁ - u₃)
  have n₃ := norm_nonneg (u₂ - u₃)
  have q₁ : ‖u₁ - u₂‖ ^ 2 ≤ L ^ 2 := pow_le_pow_left₀ n₁ e₁ 2
  have q₂ : ‖u₁ - u₃‖ ^ 2 ≤ L ^ 2 := pow_le_pow_left₀ n₂ e₂ 2
  have q₃ : ‖u₂ - u₃‖ ^ 2 ≤ L ^ 2 := pow_le_pow_left₀ n₃ e₃ 2
  have hp := pairs_le l₁ l₂ l₃ hs
  have hL : L ^ 2 ≤ ‖u₁ - u₂‖ ^ 2 + ‖u₁ - u₃‖ ^ 2 + ‖u₂ - u₃‖ ^ 2 := by
    have L0 : 0 ≤ L := le_trans n₁ e₁
    rcases big with b | b | b <;> nlinarith [pow_le_pow_left₀ L0 b 2, sq_nonneg ‖u₁ - u₂‖,
      sq_nonneg ‖u₁ - u₃‖, sq_nonneg ‖u₂ - u₃‖]
  nlinarith [mul_le_mul_of_nonneg_left q₁ (mul_nonneg p₁ p₂),
    mul_le_mul_of_nonneg_left q₂ (mul_nonneg p₁ p₃), mul_le_mul_of_nonneg_left q₃ (mul_nonneg p₂ p₃)]

/-- A point of the plane through the foot `f` of `o` is no nearer `o` than the foot, and the
foot is deeper by at most |g − f|²/(2d), d = |f − o| > 0. -/
theorem foot_over (o f g : E) (d : ℝ) (hd : 0 < d) (hfd : ‖f - o‖ = d)
    (hperp : ⟪f - o, g - f⟫ = 0) :
    d ≤ ‖g - o‖ ∧ ‖g - o‖ - d ≤ ‖g - f‖ ^ 2 / (2 * d) := by
  have py : ‖g - o‖ ^ 2 = d ^ 2 + ‖g - f‖ ^ 2 := by
    have : g - o = (f - o) + (g - f) := by abel
    rw [this, norm_add_sq_real, hperp, hfd]
    ring
  have G0 := norm_nonneg (g - o)
  have dG : d ≤ ‖g - o‖ := by nlinarith [sq_nonneg ‖g - f‖]
  refine ⟨dG, ?_⟩
  rw [le_div_iff₀ (by linarith)]
  nlinarith [sq_nonneg (‖g - o‖ - d)]

/-- A chord of the sphere (centre o, radius r ≥ c > 0) whose middle sags at most c is at most
√(8rc) long. -/
theorem chord_short (o a b : E) (r c : ℝ) (_hc : 0 < c) (hcr : c ≤ r)
    (ha : ‖a - o‖ = r) (hb : ‖b - o‖ = r)
    (hm : r - ‖(a + (1 / 2 : ℝ) • (b - a)) - o‖ ≤ c) : ‖a - b‖ ^ 2 ≤ 8 * r * c := by
  have k := chord_sq o a b (by rw [ha, hb]) (1 / 2)
  rw [ha] at k
  have m0 := norm_nonneg ((a + (1 / 2 : ℝ) • (b - a)) - o)
  have hrm : r - c ≤ ‖(a + (1 / 2 : ℝ) • (b - a)) - o‖ := by linarith
  have : (r - c) ^ 2 ≤ ‖(a + (1 / 2 : ℝ) • (b - a)) - o‖ ^ 2 :=
    pow_le_pow_left₀ (by linarith) hrm 2
  rw [norm_sub_rev a b]
  nlinarith [sq_nonneg c]

/-- **The sphere's filter.** A triangle with its corners on the sphere (centre o, radius r) and
its edges' middles within c (0 < c ≤ r) of it, whose foot f of the centre (in its plane:
f − o square to each xᵢ − f) lies in it (weights λᵢ ≥ 0), at distance d > 0: the foot, its
deepest point, sags at most 8rc/(9d) more than its centroid g. -/
theorem sphere_filter (o x₁ x₂ x₃ f : E) (l₁ l₂ l₃ r c d : ℝ)
    (hc : 0 < c) (hcr : c ≤ r) (hd : 0 < d) (hfd : ‖f - o‖ = d)
    (h₁ : ‖x₁ - o‖ = r) (h₂ : ‖x₂ - o‖ = r) (h₃ : ‖x₃ - o‖ = r)
    (hs : l₁ + l₂ + l₃ = 1) (p₁ : 0 ≤ l₁) (p₂ : 0 ≤ l₂) (p₃ : 0 ≤ l₃)
    (hf : f = l₁ • x₁ + l₂ • x₂ + l₃ • x₃)
    (q₁ : ⟪f - o, x₁ - f⟫ = 0) (q₂ : ⟪f - o, x₂ - f⟫ = 0) (q₃ : ⟪f - o, x₃ - f⟫ = 0)
    (m₁₂ : r - ‖(x₁ + (1 / 2 : ℝ) • (x₂ - x₁)) - o‖ ≤ c)
    (m₁₃ : r - ‖(x₁ + (1 / 2 : ℝ) • (x₃ - x₁)) - o‖ ≤ c)
    (m₂₃ : r - ‖(x₂ + (1 / 2 : ℝ) • (x₃ - x₂)) - o‖ ≤ c) :
    r - d ≤ (r - ‖(1 / 3 : ℝ) • (x₁ + x₂ + x₃) - o‖) + 8 * r * c / (9 * d) := by
  -- the corners are all R = √(r² − d²) from the foot
  have side : ∀ x : E, ‖x - o‖ = r → ⟪f - o, x - f⟫ = 0 → ‖x - f‖ ^ 2 = r ^ 2 - d ^ 2 := by
    intro x hx hq
    have e : ‖x - o‖ ^ 2 = ‖(f - o) + (x - f)‖ ^ 2 := by
      rw [show x - o = (f - o) + (x - f) by abel]
    rw [hx, norm_add_sq_real, hq, hfd] at e
    linarith
  have hR : ∀ x : E, ‖x - o‖ = r → ⟪f - o, x - f⟫ = 0 →
      ‖x - f‖ = Real.sqrt (r ^ 2 - d ^ 2) := by
    intro x hx hq
    rw [← side x hx hq, Real.sqrt_sq (norm_nonneg _)]
  have z : l₁ • (x₁ - f) + l₂ • (x₂ - f) + l₃ • (x₃ - f) = 0 := by
    have : l₁ • f + l₂ • f + l₃ • f = f := by rw [← add_smul, ← add_smul, hs, one_smul]
    rw [smul_sub, smul_sub, smul_sub]
    calc l₁ • x₁ - l₁ • f + (l₂ • x₂ - l₂ • f) + (l₃ • x₃ - l₃ • f)
        = (l₁ • x₁ + l₂ • x₂ + l₃ • x₃) - (l₁ • f + l₂ • f + l₃ • f) := by abel
      _ = 0 := by rw [this, ← hf, sub_self]
  -- the sides, and the longest
  have s₁₂ := chord_short o x₁ x₂ r c hc hcr h₁ h₂ m₁₂
  have s₁₃ := chord_short o x₁ x₃ r c hc hcr h₁ h₃ m₁₃
  have s₂₃ := chord_short o x₂ x₃ r c hc hcr h₂ h₃ m₂₃
  set L := max (max ‖x₁ - x₂‖ ‖x₁ - x₃‖) ‖x₂ - x₃‖
  have e₁ : ‖(x₁ - f) - (x₂ - f)‖ ≤ L := by
    rw [sub_sub_sub_cancel_right]; exact le_trans (le_max_left _ _) (le_max_left _ _)
  have e₂ : ‖(x₁ - f) - (x₃ - f)‖ ≤ L := by
    rw [sub_sub_sub_cancel_right]; exact le_trans (le_max_right _ _) (le_max_left _ _)
  have e₃ : ‖(x₂ - f) - (x₃ - f)‖ ≤ L := by
    rw [sub_sub_sub_cancel_right]; exact le_max_right _ _
  have big : L ≤ ‖(x₁ - f) - (x₂ - f)‖ ∨ L ≤ ‖(x₁ - f) - (x₃ - f)‖ ∨ L ≤ ‖(x₂ - f) - (x₃ - f)‖ := by
    simp only [sub_sub_sub_cancel_right, L]
    rcases le_total ‖x₁ - x₂‖ ‖x₁ - x₃‖ with h | h <;>
      rcases le_total (max ‖x₁ - x₂‖ ‖x₁ - x₃‖) ‖x₂ - x₃‖ with k | k <;>
      simp_all
  have near := centroid_near (x₁ - f) (x₂ - f) (x₃ - f) l₁ l₂ l₃ (Real.sqrt (r ^ 2 - d ^ 2)) L hs p₁ p₂ p₃ z
    (hR x₁ h₁ q₁) (hR x₂ h₂ q₂) (hR x₃ h₃ q₃) e₁ e₂ e₃ big
  have hL : L ^ 2 ≤ 8 * r * c := by
    have L0 : 0 ≤ L := le_trans (norm_nonneg _) (le_max_right _ _)
    rcases big with b | b | b <;> simp only [sub_sub_sub_cancel_right] at b <;>
      nlinarith [pow_le_pow_left₀ L0 b 2, norm_nonneg (x₁ - x₂), norm_nonneg (x₁ - x₃),
        norm_nonneg (x₂ - x₃), le_antisymm b (by simp only [L]; first
          | exact le_trans (le_max_left _ _) (le_max_left _ _)
          | exact le_trans (le_max_right _ _) (le_max_left _ _)
          | exact le_max_right _ _)]
  -- the centroid, in the plane: f − o square to g − f
  set g := (1 / 3 : ℝ) • (x₁ + x₂ + x₃)
  have hg : g - f = (1 / 3 : ℝ) • ((x₁ - f) + (x₂ - f) + (x₃ - f)) := by
    simp only [g]; module
  have qg : ⟪f - o, g - f⟫ = 0 := by
    rw [hg, inner_smul_right, inner_add_right, inner_add_right, q₁, q₂, q₃]; ring
  obtain ⟨_, hov⟩ := foot_over o f g d hd hfd qg
  rw [hg] at hov
  have : ‖(1 / 3 : ℝ) • ((x₁ - f) + (x₂ - f) + (x₃ - f))‖ ^ 2 / (2 * d) ≤ 8 * r * c / (9 * d) := by
    rw [div_le_div_iff₀ (by positivity) (by positivity)]
    nlinarith
  linarith

end Sag

