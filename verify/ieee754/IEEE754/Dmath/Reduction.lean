/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.RemPio2
import IEEE754.Dmath.Pi30

/-!
# `rem_pio2` against `π`

`remPio2_spec`: for `|x| ≤ 2^20`, `y0 + y1` is `x − n·π/2` within `2^-56`. It follows from
`remPio2_rat` and thirty digits of `π` (`pi_gt_d30` and `pi_lt_d30`), since `|n|` can reach
`667546`.
-/

namespace IEEE754.Dmath

open FP

/-- Each three-part `π/2` is within `4·10^-27` of `π/2`. -/
lemma pio2_close {C : ℚ} (hC : C = PIO2_1.val + PIO2_1T.val ∨ C = PIO2_1.val + PIO2_2.val + PIO2_2T.val ∨
    C = PIO2_1.val + PIO2_2.val + PIO2_3.val + PIO2_3T.val) : |(C : ℝ) - Real.pi / 2| ≤ 4 / 10 ^ 27 := by
  have h1 := pi_gt_d30
  have h2 := pi_lt_d30
  rw [PIO2_1_val, PIO2_1T_val, PIO2_2_val, PIO2_2T_val, PIO2_3_val, PIO2_3T_val] at hC
  rcases hC with rfl | rfl | rfl <;> (push_cast; rw [abs_le]; constructor <;> norm_num at h1 h2 ⊢ <;> linarith)

lemma remPio2_fst (x : F64) : (remPio2 x).1 = FP.toIntSat (-2 ^ 31) (2 ^ 31 - 1) (sub (add (mul x INVPIO2) TOINT) TOINT) := by
  unfold remPio2; dsimp only; split_ifs <;> rfl

/-- **`rem_pio2`'s accuracy.** For finite `x` with `|x| ≤ 2^20`: `y0`, `y1` finite, `|n| ≤ 667546`,
`|y0 + y1 − (x − n·π/2)| ≤ 2^-56`, `|y1| ≤ 2^-53·|y0| + 2^-62`, and `|y0 + y1| ≤ 0.78541`. -/
theorem remPio2_spec {x : F64} (hx : x.isFinite) (hb : |x.val| ≤ 1048576) :
    (remPio2 x).2.1.isFinite ∧ (remPio2 x).2.2.isFinite ∧ |(remPio2 x).1| ≤ 667546 ∧
      |((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (Real.pi / 2))| ≤ 1 / 2 ^ 56 ∧
      |(remPio2 x).2.2.val| ≤ 2 ^ (-53 : ℤ) * |(remPio2 x).2.1.val| + 1 / 2 ^ 62 ∧
      |(remPio2 x).2.1.val + (remPio2 x).2.2.val| ≤ 78541 / 100000 := by
  obtain ⟨h1, h2, h3, ⟨C, hC, h4⟩, h5, -⟩ := remPio2_rat hx hb
  have hmain : |((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (Real.pi / 2))| ≤
      1 / 2 ^ 56 := by
    have hc := pio2_close hC
    have hn : |((remPio2 x).1 : ℝ)| ≤ 667546 := by exact_mod_cast h3
    have h4' : |((remPio2 x).2.1.val + (remPio2 x).2.2.val - (x.val - (remPio2 x).1 * C) : ℚ)| ≤ 1 / 2 ^ 60 := h4
    have h4r : |((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (C : ℝ))| ≤ 1 / 2 ^ 60 := by
      have h := (Rat.cast_le (K := ℝ)).mpr h4'; push_cast at h; exact h
    have key : ((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (Real.pi / 2)) =
        (((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (C : ℝ))) +
          (remPio2 x).1 * ((Real.pi / 2) - C) := by ring
    rw [key]
    have h7 : |((remPio2 x).1 : ℝ) * (Real.pi / 2 - C)| ≤ 667546 * (4 / 10 ^ 27) := by
      rw [abs_mul, abs_sub_comm]; exact mul_le_mul hn hc (abs_nonneg _) (by norm_num)
    have := abs_add_le (((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) - (x.val - (remPio2 x).1 * (C : ℝ)))
      ((remPio2 x).1 * ((Real.pi / 2) - C))
    have : (1 : ℝ) / 2 ^ 60 + 667546 * (4 / 10 ^ 27) ≤ 1 / 2 ^ 56 := by norm_num
    linarith
  refine ⟨h1, h2, h3, hmain, h5, ?_⟩
  -- `|x − n·π/2| ≤ π/4 + 10^-9`, from `|n − x·INVPIO2| ≤ 1/2 + 2^-34`
  obtain ⟨hf, ⟨k, hk, hk2⟩, hkx⟩ := quadrant hx hb
  have hn : (remPio2 x).1 = k := by rw [remPio2_fst]; exact toIntSat_int hf hk hk2
  rw [hk] at hkx
  have hkxr : |(k : ℝ) - (x.val : ℝ) * (5734161139222659 / 2 ^ 53)| ≤ 1 / 2 + 1 / 2 ^ 34 := by
    rw [INVPIO2_val, show (2 : ℚ) ^ (-34 : ℤ) = 1 / 2 ^ 34 by norm_num] at hkx
    have h := (Rat.cast_le (K := ℝ)).mpr hkx; rw [Rat.cast_abs] at h; push_cast at h; exact h
  have hxr : |(x.val : ℝ)| ≤ 1048576 := by exact_mod_cast hb
  have hp1 := pi_gt_d30
  have hp2 := pi_lt_d30
  have hI : |1 - (5734161139222659 / 2 ^ 53 : ℝ) * (Real.pi / 2)| ≤ 7 / 10 ^ 17 := by
    rw [abs_le]; constructor <;> norm_num at hp1 hp2 ⊢ <;> linarith
  have hred : |(x.val : ℝ) - k * (Real.pi / 2)| ≤ 78540 / 100000 := by
    have key : (x.val : ℝ) - k * (Real.pi / 2) =
        (Real.pi / 2) * ((x.val : ℝ) * (5734161139222659 / 2 ^ 53) - k) +
          x.val * (1 - (5734161139222659 / 2 ^ 53 : ℝ) * (Real.pi / 2)) := by ring
    rw [key]
    have a1 : |(Real.pi / 2) * ((x.val : ℝ) * (5734161139222659 / 2 ^ 53) - k)| ≤
        (31415926536 / 2 / 10 ^ 10) * (1 / 2 + 1 / 2 ^ 34) := by
      rw [abs_mul, abs_of_pos (by linarith [Real.pi_pos]), abs_sub_comm]
      apply mul_le_mul _ hkxr (abs_nonneg _) (by norm_num)
      norm_num at hp2 ⊢; linarith
    have a2 : |(x.val : ℝ) * (1 - (5734161139222659 / 2 ^ 53 : ℝ) * (Real.pi / 2))| ≤ 1048576 * (7 / 10 ^ 17) := by
      rw [abs_mul]; exact mul_le_mul hxr hI (abs_nonneg _) (by norm_num)
    have := abs_add_le ((Real.pi / 2) * ((x.val : ℝ) * (5734161139222659 / 2 ^ 53) - k))
      ((x.val : ℝ) * (1 - (5734161139222659 / 2 ^ 53 : ℝ) * (Real.pi / 2)))
    have n : (31415926536 / 2 / 10 ^ 10 : ℝ) * (1 / 2 + 1 / 2 ^ 34) + 1048576 * (7 / 10 ^ 17) ≤ 78540 / 100000 := by
      norm_num
    linarith
  rw [hn] at hmain
  have hsum : |((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ)| ≤ 78541 / 100000 := by
    have := abs_sub_abs_le_abs_sub ((remPio2 x).2.1.val + (remPio2 x).2.2.val : ℝ) ((x.val : ℝ) - k * (Real.pi / 2))
    have n : (78540 : ℝ) / 100000 + 1 / 2 ^ 56 ≤ 78541 / 100000 := by norm_num
    linarith
  have : (((|(remPio2 x).2.1.val + (remPio2 x).2.2.val| : ℚ)) : ℝ) ≤ ((78541 / 100000 : ℚ) : ℝ) := by
    push_cast; exact hsum
  exact_mod_cast this

end IEEE754.Dmath
