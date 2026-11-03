import Mathlib

/-!
# Bernstein enclosure and de Casteljau halving

The certificate clears a box when every Bernstein coefficient of `g` (or of `Psi`) has one strict
sign. These are the facts it relies on, in one variable and as tensor products in three.
-/

namespace LoftSSI

open Finset

/-- The Bernstein basis polynomial `C(n,k) x^k (1-x)^(n-k)` as a real function. -/
noncomputable def bb (n k : ℕ) (x : ℝ) : ℝ := (n.choose k : ℝ) * x ^ k * (1 - x) ^ (n - k)

theorem bb_nonneg (n k : ℕ) {x : ℝ} (h0 : 0 ≤ x) (h1 : x ≤ 1) : 0 ≤ bb n k x := by
  unfold bb
  have : 0 ≤ 1 - x := by linarith
  positivity

/-- Partition of unity. -/
theorem bb_sum (n : ℕ) (x : ℝ) : ∑ k ∈ range (n + 1), bb n k x = 1 := by
  have := (add_pow x (1 - x) n).symm
  simp only [add_sub_cancel, one_pow] at this
  rw [← this]
  refine sum_congr rfl fun k _ => ?_
  unfold bb; ring

/-- A polynomial in Bernstein form on [0,1]. -/
noncomputable def bern (n : ℕ) (b : ℕ → ℝ) (x : ℝ) : ℝ := ∑ k ∈ range (n + 1), b k * bb n k x

/-- Enclosure: coefficients all at least `m` bound the polynomial below by `m` on [0,1]. -/
theorem bern_ge (n : ℕ) (b : ℕ → ℝ) (m : ℝ) (hb : ∀ k ≤ n, m ≤ b k) {x : ℝ} (h0 : 0 ≤ x)
    (h1 : x ≤ 1) : m ≤ bern n b x := by
  calc m = ∑ k ∈ range (n + 1), m * bb n k x := by rw [← mul_sum, bb_sum, mul_one]
    _ ≤ bern n b x := by
      refine sum_le_sum fun k hk => ?_
      exact mul_le_mul_of_nonneg_right (hb k (by simp at hk; omega)) (bb_nonneg n k h0 h1)

theorem bern_le (n : ℕ) (b : ℕ → ℝ) (M : ℝ) (hb : ∀ k ≤ n, b k ≤ M) {x : ℝ} (h0 : 0 ≤ x)
    (h1 : x ≤ 1) : bern n b x ≤ M := by
  calc bern n b x ≤ ∑ k ∈ range (n + 1), M * bb n k x := by
        refine sum_le_sum fun k hk => ?_
        exact mul_le_mul_of_nonneg_right (hb k (by simp at hk; omega)) (bb_nonneg n k h0 h1)
    _ = M := by rw [← mul_sum, bb_sum, mul_one]

/-- Trivariate tensor-product Bernstein form on the unit cube. -/
noncomputable def bern3 (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ) (x y z : ℝ) : ℝ :=
  ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
    b i j k * (bb n1 i x * bb n2 j y * bb n3 k z)

/-- The 3D basis also sums to one. -/
theorem bb3_sum (n1 n2 n3 : ℕ) (x y z : ℝ) :
    ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
      bb n1 i x * bb n2 j y * bb n3 k z = 1 := by
  have h : ∀ i j, ∑ k ∈ range (n3 + 1), bb n1 i x * bb n2 j y * bb n3 k z =
      bb n1 i x * bb n2 j y := by
    intro i j; rw [← mul_sum, bb_sum, mul_one]
  have h2 : ∀ i, ∑ j ∈ range (n2 + 1), bb n1 i x * bb n2 j y = bb n1 i x := by
    intro i; rw [← mul_sum, bb_sum, mul_one]
  simp only [h, h2]
  exact bb_sum n1 x

/-- The certificate's clearing rule: all coefficients strictly positive (beyond a margin `e >= 0`
covering their rounding) means the polynomial has no zero on the box. -/
theorem bern3_pos (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ) (m : ℝ) (hm : 0 < m)
    (hb : ∀ i ≤ n1, ∀ j ≤ n2, ∀ k ≤ n3, m ≤ b i j k) {x y z : ℝ}
    (hx : 0 ≤ x ∧ x ≤ 1) (hy : 0 ≤ y ∧ y ≤ 1) (hz : 0 ≤ z ∧ z ≤ 1) :
    0 < bern3 n1 n2 n3 b x y z := by
  have key : m ≤ bern3 n1 n2 n3 b x y z := by
    calc m = m * 1 := (mul_one m).symm
      _ = m * ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
          bb n1 i x * bb n2 j y * bb n3 k z := by rw [bb3_sum]
      _ = ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
          m * (bb n1 i x * bb n2 j y * bb n3 k z) := by simp only [mul_sum]
      _ ≤ _ := by
        unfold bern3
        refine sum_le_sum fun i hi => sum_le_sum fun j hj => sum_le_sum fun k hk => ?_
        simp only [mem_range] at hi hj hk
        refine mul_le_mul_of_nonneg_right (hb i (by omega) j (by omega) k (by omega)) ?_
        have := bb_nonneg n1 i hx.1 hx.2
        have := bb_nonneg n2 j hy.1 hy.2
        have := bb_nonneg n3 k hz.1 hz.2
        positivity
  linarith

/-- Rounding: coefficients computed with error at most `e` and found above `e` are positive. -/
theorem rounded_pos {b bt e : ℝ} (herr : |bt - b| ≤ e) (h : e < bt) : 0 < b := by
  have := (abs_le.mp herr).2
  linarith

/-- de Casteljau halving: the basis on the left half re-expands in the basis on [0,1]
(`B^n_k(t/2) = sum_j B^n_j(t) B^j_k(1/2)`), so halving maps coefficients b to
`b'_j = sum_k b_k C(j,k) 2^-j`: the subdivided coefficients describe the same polynomial. -/
theorem bb_half (n k : ℕ) (t : ℝ) :
    bb n k (t / 2) = ∑ j ∈ range (n + 1), bb n j t * ((j.choose k : ℝ) * (2 : ℝ)⁻¹ ^ j) := by
  rcases lt_or_ge n k with hnk | hkn
  · -- both sides vanish
    have l : bb n k (t / 2) = 0 := by unfold bb; rw [Nat.choose_eq_zero_of_lt hnk]; simp
    rw [l]; symm
    refine sum_eq_zero fun j hj => ?_
    simp only [mem_range] at hj
    rw [Nat.choose_eq_zero_of_lt (by omega)]; simp
  · have split : range (n + 1) = range k ∪ Ico k (n + 1) := by
      rw [range_eq_Ico, range_eq_Ico, Ico_union_Ico_eq_Ico (Nat.zero_le _) (by omega)]
    rw [split, sum_union (by
      rw [range_eq_Ico]; exact Ico_disjoint_Ico_consecutive 0 k (n + 1))]
    rw [sum_eq_zero (fun j hj => by
      simp only [mem_range] at hj; rw [Nat.choose_eq_zero_of_lt hj]; simp), zero_add]
    rw [sum_Ico_eq_sum_range]
    have hsub : n + 1 - k = (n - k) + 1 := by omega
    rw [hsub]
    unfold bb
    simp only [div_eq_mul_inv]
    have e1 : (1 : ℝ) - t * 2⁻¹ = t * 2⁻¹ + (1 - t) := by ring
    rw [e1, add_pow, mul_sum]
    refine sum_congr rfl fun m hm => ?_
    simp only [mem_range] at hm
    have hc : ((n.choose (k + m) : ℕ) : ℝ) * (((k + m).choose k : ℕ) : ℝ) =
        ((n.choose k : ℕ) : ℝ) * (((n - k).choose m : ℕ) : ℝ) := by
      have := Nat.choose_mul (n := n) (k := k + m) (s := k) (by omega)
      rw [show k + m - k = m by omega] at this
      exact_mod_cast this
    have hp : n - (k + m) = n - k - m := by omega
    rw [hp]
    have : ((n.choose (k + m) : ℕ) : ℝ) * t ^ (k + m) * (1 - t) ^ (n - k - m) *
        ((((k + m).choose k : ℕ) : ℝ) * (2 : ℝ)⁻¹ ^ (k + m)) =
        (((n.choose (k + m) : ℕ) : ℝ) * (((k + m).choose k : ℕ) : ℝ)) *
          ((t * 2⁻¹) ^ k * (t * 2⁻¹) ^ m * (1 - t) ^ (n - k - m)) := by
      rw [mul_pow, mul_pow, pow_add, pow_add]; ring
    rw [this, hc]; ring

end LoftSSI
