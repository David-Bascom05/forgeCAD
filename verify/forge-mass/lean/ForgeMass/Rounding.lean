import Mathlib

/-!
# Summing under the standard rounding model

The model: each floating-point addition returns `(a + b)(1 + δ)` with |δ| ≤ u, u = 2⁻⁵³ for f64.
verify/ieee754 (branch claude/repo-review-mbdrlj-ieee) proves this of its IEEE 754 model, as
`F64.add_rel_err`, outside the subnormal range, where addition is exact anyway (`add_exact_of_small`).
Here it is a hypothesis.

`sum_err`: a running sum of n terms is off by at most ((1 + u)ⁿ − 1) Σ|xᵢ|. This is how far
`mass_rel`'s face-by-face totals (mass.rs:137–140) can drift from the exact sum of the faces'
computed values. `face_sum_err` gives it as γₙ Σ|xᵢ|, γₙ = nu/(1 − nu), the term mass.rs now adds
for its face sum. The allowance inside a face, 1e-15 of the absolute contributions per piece
(mass.rs, `est` and the polygon sum), is still a rule of thumb: deriving it needs the integrand's
flops through the f64 bridge.
-/

noncomputable section

namespace ForgeMass

/-- A running sum with each addition rounded by its own δ. -/
def fsum (x δ : ℕ → ℝ) : ℕ → ℝ
  | 0 => 0
  | n + 1 => (fsum x δ n + x n) * (1 + δ n)

theorem sum_err (u : ℝ) (hu : 0 ≤ u) (x δ : ℕ → ℝ) (hδ : ∀ i, |δ i| ≤ u) (n : ℕ) :
    |fsum x δ n - ∑ i ∈ Finset.range n, x i| ≤ ((1 + u) ^ n - 1) * ∑ i ∈ Finset.range n, |x i| := by
  induction n with
  | zero => simp [fsum]
  | succ n ih =>
    set E := fsum x δ n - ∑ i ∈ Finset.range n, x i
    set A := ∑ i ∈ Finset.range n, |x i|
    have hA : 0 ≤ A := Finset.sum_nonneg fun i _ => abs_nonneg _
    have hs : |∑ i ∈ Finset.range n, x i| ≤ A := Finset.abs_sum_le_sum_abs _ _
    have hp : 1 ≤ (1 + u) ^ n := one_le_pow₀ (by linarith)
    have key : fsum x δ (n + 1) - ∑ i ∈ Finset.range (n + 1), x i =
        E + δ n * (fsum x δ n + x n) := by
      simp only [fsum, Finset.sum_range_succ, E]; ring
    rw [key, Finset.sum_range_succ]
    have h1 : |δ n * (fsum x δ n + x n)| ≤ u * (|E| + A + |x n|) := by
      rw [abs_mul]
      apply mul_le_mul (hδ n) _ (abs_nonneg _) hu
      have : fsum x δ n + x n = E + ∑ i ∈ Finset.range n, x i + x n := by simp only [E]; ring
      rw [this]
      calc |E + ∑ i ∈ Finset.range n, x i + x n| ≤ |E + ∑ i ∈ Finset.range n, x i| + |x n| := abs_add_le _ _
        _ ≤ |E| + |∑ i ∈ Finset.range n, x i| + |x n| := by gcongr; exact abs_add_le _ _
        _ ≤ |E| + A + |x n| := by gcongr
    calc |E + δ n * (fsum x δ n + x n)| ≤ |E| + |δ n * (fsum x δ n + x n)| := abs_add_le _ _
      _ ≤ |E| + u * (|E| + A + |x n|) := by gcongr
      _ = (1 + u) * |E| + u * (A + |x n|) := by ring
      _ ≤ (1 + u) * (((1 + u) ^ n - 1) * A) + u * (A + |x n|) := by gcongr
      _ ≤ ((1 + u) ^ (n + 1) - 1) * (A + |x n|) := by
        have hx : 0 ≤ |x n| := abs_nonneg _
        have : (1 + u) * (((1 + u) ^ n - 1) * A) + u * (A + |x n|) =
            ((1 + u) ^ (n + 1) - 1) * A + u * |x n| := by ring
        rw [this, mul_add]
        have : u ≤ (1 + u) ^ (n + 1) - 1 := by
          rw [pow_succ]; nlinarith
        nlinarith

/-- The factor mass.rs adds for its face sum: (1 + u)ⁿ − 1 ≤ γₙ = n u / (1 − n u) while n u < 1. -/
theorem pow_sub_one_le_gamma (u : ℝ) (hu : 0 ≤ u) (n : ℕ) (hn : n * u < 1) :
    (1 + u) ^ n - 1 ≤ n * u / (1 - n * u) := by
  have hpos : 0 < 1 - n * u := by linarith
  rw [le_div_iff₀ hpos]
  -- (1 + u)ⁿ (1 − n u) ≤ 1, by induction
  have key : ∀ m : ℕ, m * u < 1 → (1 + u) ^ m * (1 - m * u) ≤ 1 := by
    intro m
    induction m with
    | zero => intro _; simp
    | succ m ih =>
      intro hm
      have hm' : (m : ℝ) * u < 1 := by
        have : (m : ℝ) * u ≤ ((m + 1 : ℕ) : ℝ) * u := by
          apply mul_le_mul_of_nonneg_right _ hu; push_cast; linarith
        linarith
      have h := ih hm'
      have hp : 0 ≤ (1 + u) ^ m := pow_nonneg (by linarith) m
      push_cast at hm ⊢
      -- (1+u)^(m+1)(1 − (m+1)u) = (1+u)^m [(1+u)(1 − m u − u)] ≤ (1+u)^m (1 − m u)
      have : (1 + u) * (1 - (m + 1) * u) ≤ 1 - m * u := by nlinarith [sq_nonneg u, hu]
      calc (1 + u) ^ (m + 1) * (1 - (m + 1) * u) = (1 + u) ^ m * ((1 + u) * (1 - (m + 1) * u)) := by ring
        _ ≤ (1 + u) ^ m * (1 - m * u) := mul_le_mul_of_nonneg_left this hp
        _ ≤ 1 := h
  have := key n hn
  nlinarith [this]

/-- So the face sum of `mass_rel` (mass.rs, `t[k] += v[k]` over n faces) is within
γₙ Σ|vᵢ| of the exact sum of the faces' computed values: the term mass.rs adds to `e[k]`. -/
theorem face_sum_err (u : ℝ) (hu : 0 ≤ u) (x δ : ℕ → ℝ) (hδ : ∀ i, |δ i| ≤ u) (n : ℕ)
    (hn : n * u < 1) :
    |fsum x δ n - ∑ i ∈ Finset.range n, x i| ≤ n * u / (1 - n * u) * ∑ i ∈ Finset.range n, |x i| := by
  calc _ ≤ ((1 + u) ^ n - 1) * ∑ i ∈ Finset.range n, |x i| := sum_err u hu x δ hδ n
    _ ≤ _ := mul_le_mul_of_nonneg_right (pow_sub_one_le_gamma u hu n hn)
        (Finset.sum_nonneg fun i _ => abs_nonneg _)

end ForgeMass
end
