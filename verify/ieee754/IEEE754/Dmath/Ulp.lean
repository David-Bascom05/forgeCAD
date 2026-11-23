/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.Approx

/-!
# Rounding error by binade

`round_ulp`: a finite rounding of `|q| < 2^k` (`k ≥ -1021`) is within `2^(k-54)` (half an ulp of
the binade below `2^k`). `addU`, `subU`, `mulU`, `divU`: the error calculus of `Approx.lean` with
this bound for the operation's own rounding, which is sharper than `ε·|result|` near a binade's top.
-/

namespace IEEE754.Dmath

open FP

theorem round_ulp {zs : Bool} {q : ℚ} {k : ℤ} (hfin : (round binary64 zs q).isFinite) (hq : |q| < 2 ^ k)
    (hk : -1021 ≤ k) : |(round binary64 zs q).val - q| ≤ 2 ^ (k - 54) := by
  by_cases h0 : q = 0
  · subst h0; simp only [round_zero, val]; norm_num; exact (two_zpow_pos (K := ℚ) _).le
  have ha : 0 < |q| := abs_pos.mpr h0
  have e := round_val_abs (K := ℚ) binary64 h0 hfin
  rw [Rat.cast_id] at e
  rw [e]
  refine le_trans (uround_err binary64 ha) (two_zpow_le ?_)
  have hl := (log_spec ha).1
  have hlk : Int.log 2 |q| < k := by
    have := lt_of_le_of_lt hl hq
    exact (zpow_lt_zpow_iff_right₀ (by norm_num : (1 : ℚ) < 2)).mp this
  rw [roundPos_snd]; simp only [binary64_prec, binary64_qmin]; omega

lemma addU {a b : F64} (ha : a.isFinite) (hb : b.isFinite) {A B ea eb MA MB E : ℚ} {k : ℤ}
    (h1 : |a.val - A| ≤ ea) (h2 : |b.val - B| ≤ eb) (h3 : |A| ≤ MA) (h4 : |B| ≤ MB)
    (hM : MA + ea + (MB + eb) < 2 ^ k) (hk : -1021 ≤ k) (hk8 : (2 : ℚ) ^ k ≤ 8)
    (hE : 2 ^ (k - 54) + (ea + eb) ≤ E) :
    (add a b).isFinite ∧ |(add a b).val - (A + B)| ≤ E ∧ |A + B| ≤ MA + MB := by
  obtain ⟨hf, -, hAB⟩ := addA ha hb h1 h2 h3 h4 (by linarith) le_rfl
  refine ⟨hf, ?_, hAB⟩
  have ha' := tri h1 h3
  have hb' := tri h2 h4
  have hab : |a.val + b.val| < 2 ^ k := lt_of_le_of_lt (le_trans (abs_add_le _ _) (add_le_add ha' hb')) hM
  have hr := round_ulp (zs := a.sign && b.sign) (by rw [← add_eq_round ha hb]; exact hf) hab hk
  rw [← add_eq_round ha hb] at hr
  have hd : |a.val + b.val - (A + B)| ≤ ea + eb := by
    rw [show a.val + b.val - (A + B) = (a.val - A) + (b.val - B) by ring]
    exact le_trans (abs_add_le _ _) (add_le_add h1 h2)
  have := abs_sub_le (add a b).val (a.val + b.val) (A + B)
  linarith

lemma subU {a b : F64} (ha : a.isFinite) (hb : b.isFinite) {A B ea eb MA MB E : ℚ} {k : ℤ}
    (h1 : |a.val - A| ≤ ea) (h2 : |b.val - B| ≤ eb) (h3 : |A| ≤ MA) (h4 : |B| ≤ MB)
    (hM : MA + ea + (MB + eb) < 2 ^ k) (hk : -1021 ≤ k) (hk8 : (2 : ℚ) ^ k ≤ 8)
    (hE : 2 ^ (k - 54) + (ea + eb) ≤ E) :
    (sub a b).isFinite ∧ |(sub a b).val - (A - B)| ≤ E ∧ |A - B| ≤ MA + MB := by
  have hbn : b.neg.isFinite := by simpa using hb
  have := addU ha hbn (A := A) (B := -B) (MB := MB) h1
    (by rw [FP.val_neg]; rw [show -b.val - -B = -(b.val - B) by ring, abs_neg]; exact h2)
    h3 (by rw [abs_neg]; exact h4) hM hk hk8 hE
  rwa [← sub_eq_add_neg] at this

end IEEE754.Dmath
