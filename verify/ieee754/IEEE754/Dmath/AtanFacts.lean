/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.AtanPoly
import IEEE754.Dmath.AtanConsts
import Mathlib.Analysis.Real.Pi.Bounds

/-!
# `arctan` facts for `dmath::atan`

The Lipschitz bounds, `atan`'s four reductions (`arctan a = c + arctan t` with the Rust `t`), and
the constants: `ATANHI[i] + ATANLO[i]` within `10^-20` of `arctan(1/2)`, `π/4`, `arctan(3/2)`, `π/2`
(the first and third by `atan_taylor`, `π` by Mathlib's 20 digits).
-/

namespace IEEE754.Dmath

open Real

lemma arctan_lip_m {a b m : ℝ} (hm : 0 ≤ m) (ha : m ≤ a) (hb : m ≤ b) :
    |arctan a - arctan b| ≤ |a - b| / (1 + m ^ 2) := by
  have hd : ∀ s ∈ Set.uIcc b a, HasDerivWithinAt arctan (1 / (1 + s ^ 2)) (Set.uIcc b a) s :=
    fun s _ => (hasDerivAt_arctan s).hasDerivWithinAt
  have hbd : ∀ s ∈ Set.uIcc b a, ‖(1 / (1 + s ^ 2) : ℝ)‖ ≤ 1 / (1 + m ^ 2) := by
    intro s hs
    have hms : m ≤ s := by
      rcases Set.mem_uIcc.mp hs with ⟨h, _⟩ | ⟨h, _⟩ <;> linarith
    rw [Real.norm_eq_abs, abs_of_pos (by positivity)]
    apply one_div_le_one_div_of_le (by positivity); nlinarith
  have := Convex.norm_image_sub_le_of_norm_hasDerivWithin_le hd hbd (convex_uIcc b a)
    Set.left_mem_uIcc Set.right_mem_uIcc
  rw [Real.norm_eq_abs, Real.norm_eq_abs] at this
  rw [div_eq_mul_inv, mul_comm]; simpa [div_eq_mul_inv] using this

lemma arctan_lip (a b : ℝ) : |arctan a - arctan b| ≤ |a - b| := by
  have hd : ∀ s ∈ Set.uIcc b a, HasDerivWithinAt arctan (1 / (1 + s ^ 2)) (Set.uIcc b a) s :=
    fun s _ => (hasDerivAt_arctan s).hasDerivWithinAt
  have hbd : ∀ s ∈ Set.uIcc b a, ‖(1 / (1 + s ^ 2) : ℝ)‖ ≤ 1 := by
    intro s _
    rw [Real.norm_eq_abs, abs_of_pos (by positivity)]
    rw [div_le_one (by positivity)]; nlinarith [sq_nonneg s]
  have := Convex.norm_image_sub_le_of_norm_hasDerivWithin_le hd hbd (convex_uIcc b a)
    Set.left_mem_uIcc Set.right_mem_uIcc
  rw [Real.norm_eq_abs, Real.norm_eq_abs, one_mul] at this
  exact this

lemma red0 {a : ℝ} (ha : -2 < a) : arctan a = arctan (1 / 2) + arctan ((2 * a - 1) / (2 + a)) := by
  have h2 : (0 : ℝ) < 2 + a := by linarith
  have ht : (2 * a - 1) / (2 + a) < 2 := by rw [div_lt_iff₀ h2]; linarith
  rw [arctan_add (by linarith)]; congr 1; field_simp; ring

lemma red1 {a : ℝ} (ha : -1 < a) : arctan a = π / 4 + arctan ((a - 1) / (a + 1)) := by
  have h2 : (0 : ℝ) < a + 1 := by linarith
  have ht : (a - 1) / (a + 1) < 1 := by rw [div_lt_iff₀ h2]; linarith
  rw [← arctan_one, arctan_add (by linarith)]; congr 1; field_simp; ring

lemma red2 {a : ℝ} (ha : 0 ≤ a) : arctan a = arctan (3 / 2) + arctan ((a - 3 / 2) / (1 + 3 / 2 * a)) := by
  have h2 : (0 : ℝ) < 1 + 3 / 2 * a := by linarith
  have ht : (a - 3 / 2) / (1 + 3 / 2 * a) < 2 / 3 := by rw [div_lt_iff₀ h2]; linarith
  rw [arctan_add (by linarith)]; congr 1; field_simp; ring

lemma red3 {a : ℝ} (ha : 0 < a) : arctan a = π / 2 + arctan (-1 / a) := by
  rw [neg_div, arctan_neg, one_div, arctan_inv_of_pos ha]; ring

lemma pi_close : |π - 314159265358979323846 / 10 ^ 20| ≤ 1 / 10 ^ 20 := by
  have h1 := Real.pi_gt_d20; have h2 := Real.pi_lt_d20
  rw [abs_le]; constructor <;> norm_num at h1 h2 ⊢ <;> linarith

lemma taylor_q (N : ℕ) (t : ℚ) :
    |arctan (t : ℝ) - ((∑ k ∈ Finset.range N, (-1) ^ k * t ^ (2 * k + 1) / (2 * k + 1) : ℚ) : ℝ)| ≤
      |(t : ℝ)| ^ (2 * N + 1) := by
  have := atan_taylor N (t : ℝ); push_cast; exact this

lemma c_half : |((ATANHI0.val + ATANLO0.val : ℚ) : ℝ) - arctan (1 / 2)| ≤ 1 / 10 ^ 20 := by
  have h := taylor_q 34 (1 / 2)
  have e : ((ATANHI0.val + ATANLO0.val : ℚ) : ℝ) -
      ((∑ k ∈ Finset.range 34, (-1) ^ k * (1 / 2 : ℚ) ^ (2 * k + 1) / (2 * k + 1) : ℚ) : ℝ) =
      ((ATANHI0.val + ATANLO0.val - ∑ k ∈ Finset.range 34, (-1) ^ k * (1 / 2 : ℚ) ^ (2 * k + 1) / (2 * k + 1) : ℚ) : ℝ) := by
    push_cast; ring
  have hq : |ATANHI0.val + ATANLO0.val - ∑ k ∈ Finset.range 34, (-1) ^ k * (1 / 2 : ℚ) ^ (2 * k + 1) / (2 * k + 1)| ≤
      1 / 10 ^ 21 := by
    rw [ATANHI0_val, ATANLO0_val]; simp only [Finset.sum_range_succ, Finset.sum_range_zero]; norm_num
  have hqr : |((ATANHI0.val + ATANLO0.val - ∑ k ∈ Finset.range 34, (-1) ^ k * (1 / 2 : ℚ) ^ (2 * k + 1) / (2 * k + 1) : ℚ) : ℝ)| ≤
      1 / 10 ^ 21 := by
    have := (Rat.cast_le (K := ℝ)).mpr hq; rw [Rat.cast_abs] at this; push_cast at this ⊢; linarith
  rw [← e] at hqr
  push_cast at h hqr ⊢
  have h69 : |(1 / 2 : ℝ)| ^ (2 * 34 + 1) ≤ 1 / 10 ^ 20 - 1 / 10 ^ 21 := by norm_num
  have := abs_sub_le ((ATANHI0.val : ℝ) + ATANLO0.val)
    (∑ k ∈ Finset.range 34, (-1) ^ k * (1 / 2 : ℝ) ^ (2 * k + 1) / (2 * k + 1)) (arctan (1 / 2))
  rw [abs_sub_comm (arctan _)] at h
  linarith

lemma pi_mul_close {q c d : ℚ} (hc : 0 ≤ c) (h : |q - c * (314159265358979323846 / 10 ^ 20)| ≤ d) :
    |(q : ℝ) - c * π| ≤ d + c / 10 ^ 20 := by
  have hr : |((q - c * (314159265358979323846 / 10 ^ 20) : ℚ) : ℝ)| ≤ (d : ℝ) := by
    have := (Rat.cast_le (K := ℝ)).mpr h; rwa [Rat.cast_abs] at this
  push_cast at hr
  have hp := pi_close
  have hcr : (0 : ℝ) ≤ c := by exact_mod_cast hc
  have h2 : |(c : ℝ) * (π - 314159265358979323846 / 10 ^ 20)| ≤ (c : ℝ) * (1 / 10 ^ 20) := by
    rw [abs_mul, abs_of_nonneg hcr]; exact mul_le_mul_of_nonneg_left hp hcr
  have := abs_sub_le (q : ℝ) (c * (314159265358979323846 / 10 ^ 20)) (c * π)
  rw [show (c : ℝ) * (314159265358979323846 / 10 ^ 20) - c * π = -((c : ℝ) * (π - 314159265358979323846 / 10 ^ 20)) by ring,
    abs_neg] at this
  have e : (c : ℝ) * (1 / 10 ^ 20) = c / 10 ^ 20 := by ring
  linarith

lemma c_quarter : |((ATANHI1.val + ATANLO1.val : ℚ) : ℝ) - π / 4| ≤ 1 / 10 ^ 20 := by
  have := pi_mul_close (q := ATANHI1.val + ATANLO1.val) (c := 1 / 4) (d := 1 / 10 ^ 21) (by norm_num)
    (by rw [ATANHI1_val, ATANLO1_val]; norm_num)
  push_cast at this ⊢; rw [show π / 4 = 1 / 4 * π by ring]; linarith

lemma c_pio2 : |((ATANHI3.val + ATANLO3.val : ℚ) : ℝ) - π / 2| ≤ 1 / 10 ^ 20 := by
  have := pi_mul_close (q := ATANHI3.val + ATANLO3.val) (c := 1 / 2) (d := 2 / 10 ^ 21) (by norm_num)
    (by rw [ATANHI3_val, ATANLO3_val]; norm_num)
  push_cast at this ⊢; rw [show π / 2 = 1 / 2 * π by ring]; linarith

lemma c_pi : |((PI.val + PI_LO.val : ℚ) : ℝ) - π| ≤ 2 / 10 ^ 20 := by
  have := pi_mul_close (q := PI.val + PI_LO.val) (c := 1) (d := 3 / 10 ^ 21) (by norm_num)
    (by rw [PI_val, PI_LO_val]; norm_num)
  push_cast at this ⊢; rw [one_mul] at this; linarith

lemma c_PI : |(PI.val : ℝ) - π| ≤ 123 / 10 ^ 18 := by
  have := pi_mul_close (q := PI.val) (c := 1) (d := 1224647 / 10 ^ 22) (by norm_num)
    (by rw [PI_val]; norm_num)
  push_cast at this ⊢; rw [one_mul] at this; linarith

lemma c_FRAC_PI_2 : |(FRAC_PI_2.val : ℝ) - π / 2| ≤ 62 / 10 ^ 18 := by
  have := pi_mul_close (q := FRAC_PI_2.val) (c := 1 / 2) (d := 612324 / 10 ^ 22) (by norm_num)
    (by rw [FRAC_PI_2_val]; norm_num)
  push_cast at this ⊢; rw [show π / 2 = 1 / 2 * π by ring]; linarith

lemma c_threehalf : |((ATANHI2.val + ATANLO2.val : ℚ) : ℝ) - arctan (3 / 2)| ≤ 1 / 10 ^ 20 := by
  have h := taylor_q 60 (2 / 3)
  have e32 : arctan (3 / 2) = π / 2 - arctan (2 / 3) := by
    rw [← arctan_inv_of_pos (by norm_num)]; norm_num
  set T : ℚ := ∑ k ∈ Finset.range 60, (-1) ^ k * (2 / 3 : ℚ) ^ (2 * k + 1) / (2 * k + 1)
  have hq := pi_mul_close (q := ATANHI2.val + ATANLO2.val + T) (c := 1 / 2) (d := 3 / 10 ^ 21) (by norm_num)
    (by rw [ATANHI2_val, ATANLO2_val]; simp only [T, Finset.sum_range_succ, Finset.sum_range_zero]; norm_num)
  push_cast at hq h ⊢
  rw [e32]
  have h121 : |(2 / 3 : ℝ)| ^ (2 * 60 + 1) ≤ 1 / 10 ^ 21 := by norm_num
  have := abs_le.mp hq; have := abs_le.mp h
  rw [abs_le]; constructor <;> nlinarith

end IEEE754.Dmath
