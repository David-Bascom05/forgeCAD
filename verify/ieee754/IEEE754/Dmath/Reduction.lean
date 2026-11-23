/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.RemPio2
import Mathlib.Analysis.Real.Pi.Bounds

/-!
# `rem_pio2` against `π`

`remPio2_spec`: for `|x| ≤ 2048`, `y0 + y1` is `x − n·π/2` within `2^-56`, from `remPio2_rat` and
Mathlib's twenty digits of `π` (`Real.pi_gt_d20`, `Real.pi_lt_d20`).
-/

namespace IEEE754.Dmath

open FP

/-- Each three-part `π/2` is within `4·10^-21` of `π/2`. -/
lemma pio2_close {C : ℚ} (hC : C = PIO2_1.val + PIO2_1T.val ∨ C = PIO2_1.val + PIO2_2.val + PIO2_2T.val ∨
    C = PIO2_1.val + PIO2_2.val + PIO2_3.val + PIO2_3T.val) : |(C : ℝ) - Real.pi / 2| ≤ 4 / 10 ^ 21 := by
  have h1 := Real.pi_gt_d20
  have h2 := Real.pi_lt_d20
  rw [PIO2_1_val, PIO2_1T_val, PIO2_2_val, PIO2_2T_val, PIO2_3_val, PIO2_3T_val] at hC
  rcases hC with rfl | rfl | rfl <;> (push_cast; rw [abs_le]; constructor <;> norm_num at h1 h2 ⊢ <;> linarith)

/-- **`rem_pio2`'s accuracy.** For finite `x` with `|x| ≤ 2048`: `y0`, `y1` finite, `|n| ≤ 1305`,
`|y0 + y1 − (x − n·π/2)| ≤ 2^-56`, `|y1| ≤ 2^-53·|y0| + 2^-64`, and `|y0 + y1| ≤ 0.7855`. -/
theorem remPio2_spec {x : F64} (hx : x.isFinite) (hb : |x.val| ≤ 2048) :
    (remPio2 x).2.1.isFinite ∧ (remPio2 x).2.2.isFinite ∧ |(remPio2 x).1| ≤ 1305 ∧
      |((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (Real.pi / 2))| ≤ 1 / 2 ^ 56 ∧
      |(remPio2 x).2.2.val| ≤ 2 ^ (-53 : ℤ) * |(remPio2 x).2.1.val| + 1 / 2 ^ 64 ∧
      |(remPio2 x).2.1.val + (remPio2 x).2.2.val| ≤ 7855 / 10000 := by
  obtain ⟨h1, h2, h3, ⟨C, hC, h4⟩, h5, h6⟩ := remPio2_rat hx hb
  refine ⟨h1, h2, h3, ?_, h5, ?_⟩
  · have hc := pio2_close hC
    have hn : |((remPio2 x).1 : ℝ)| ≤ 1305 := by exact_mod_cast h3
    have h4' : |((remPio2 x).2.1.val + (remPio2 x).2.2.val - (x.val - (remPio2 x).1 * C) : ℚ)| ≤ 1 / 2 ^ 60 := h4
    have h4r : |((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (C : ℝ))| ≤ 1 / 2 ^ 60 := by
      have h := (Rat.cast_le (K := ℝ)).mpr h4'; push_cast at h; exact h
    have key : ((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (Real.pi / 2)) =
        (((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (C : ℝ))) +
          (remPio2 x).1 * ((Real.pi / 2) - C) := by ring
    rw [key]
    have h7 : |((remPio2 x).1 : ℝ) * (Real.pi / 2 - C)| ≤ 1305 * (4 / 10 ^ 21) := by
      rw [abs_mul, abs_sub_comm]; exact mul_le_mul hn hc (abs_nonneg _) (by norm_num)
    have := abs_add_le (((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (C : ℝ)))
      ((remPio2 x).1 * ((Real.pi / 2) - C))
    have : (1 : ℝ) / 2 ^ 60 + 1305 * (4 / 10 ^ 21) ≤ 1 / 2 ^ 56 := by norm_num
    linarith
  · have hn : |((remPio2 x).1 : ℚ)| ≤ 1305 := by exact_mod_cast h3
    have hCP : |(remPio2 x).1 * (C - PIO2_1.val)| ≤ 1305 * (61 / 10 ^ 12) := by
      rw [abs_mul]
      refine mul_le_mul hn ?_ (abs_nonneg _) (by norm_num)
      rw [PIO2_1_val, PIO2_1T_val, PIO2_2_val, PIO2_2T_val, PIO2_3_val, PIO2_3T_val] at hC
      rcases hC with rfl | rfl | rfl <;> (rw [PIO2_1_val]; norm_num [abs_le])
    have key : (remPio2 x).2.1.val + (remPio2 x).2.2.val =
        ((remPio2 x).2.1.val + (remPio2 x).2.2.val - (x.val - (remPio2 x).1 * C)) +
          (x.val - (remPio2 x).1 * PIO2_1.val) - (remPio2 x).1 * (C - PIO2_1.val) := by ring
    rw [key]
    have a := abs_add_le ((remPio2 x).2.1.val + (remPio2 x).2.2.val - (x.val - (remPio2 x).1 * C))
      (x.val - (remPio2 x).1 * PIO2_1.val)
    have b := abs_sub (((remPio2 x).2.1.val + (remPio2 x).2.2.val - (x.val - (remPio2 x).1 * C)) +
      (x.val - (remPio2 x).1 * PIO2_1.val)) ((remPio2 x).1 * (C - PIO2_1.val))
    have : (1 : ℚ) / 2 ^ 60 + 78540 / 100000 + 1305 * (61 / 10 ^ 12) ≤ 7855 / 10000 := by norm_num
    linarith

end IEEE754.Dmath
