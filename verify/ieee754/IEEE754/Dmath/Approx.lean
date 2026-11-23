/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.RemPio2

/-!
# An error calculus for `f64` evaluation

`mulA`, `addA`, `subA`: from inputs within `ea`, `eb` of exact values `A`, `B` (`|A| ≤ MA`,
`|B| ≤ MB`), the rounded result is finite and within a stated `E` of `A ∘ B`, given the side
condition `bound ≤ E` (closed by `norm_num`). The bounds use `ε' = 1/(2^53 − 1)` and an underflow
term `τ = 2^-99`, which covers products that underflow.
-/

namespace IEEE754.Dmath

open FP

def ep : ℚ := 1 / 9007199254740991
def tau : ℚ := 1 / 2 ^ 99

lemma tri {a A ea MA : ℚ} (h1 : |a - A| ≤ ea) (h3 : |A| ≤ MA) : |a| ≤ MA + ea := by
  have := abs_sub_abs_le_abs_sub a A; linarith

lemma mulA {a b : F64} (ha : a.isFinite) (hb : b.isFinite) {A B ea eb MA MB E : ℚ}
    (h1 : |a.val - A| ≤ ea) (h2 : |b.val - B| ≤ eb) (h3 : |A| ≤ MA) (h4 : |B| ≤ MB)
    (hM : (MA + ea) * (MB + eb) ≤ 8)
    (hE : ep * ((MA + ea) * (MB + eb)) + tau + ((MA + ea) * eb + MB * ea) ≤ E) :
    (mul a b).isFinite ∧ |(mul a b).val - A * B| ≤ E ∧ |A * B| ≤ MA * MB := by
  have ha' := tri h1 h3
  have hb' := tri h2 h4
  have hea : 0 ≤ ea := le_trans (abs_nonneg _) h1
  have heb : 0 ≤ eb := le_trans (abs_nonneg _) h2
  have hab : |a.val * b.val| ≤ (MA + ea) * (MB + eb) := by
    rw [abs_mul]; exact mul_le_mul ha' hb' (abs_nonneg _) (by linarith [abs_nonneg A])
  have hf := mul_fin_small ha hb (by linarith)
  have e := mul_err_abs ha hb hf
  simp only [binary64_prec, binary64_qmin] at e
  have h5 : (2 : ℚ) ^ (-((53 : ℕ) : ℤ)) = 1 / 9007199254740992 := by norm_num
  have h6 : (2 : ℚ) ^ (-1074 - 1 : ℤ) ≤ 1 / 2 ^ 100 :=
    le_trans (two_zpow_le (by norm_num : (-1074 - 1 : ℤ) ≤ -100)) (by norm_num)
  rw [h5] at e
  have hm := abs_sub_abs_le_abs_sub (mul a b).val (a.val * b.val)
  have hr : |(mul a b).val - a.val * b.val| ≤ ep * ((MA + ea) * (MB + eb)) + tau := by
    unfold ep tau; linarith
  have hd : |a.val * b.val - A * B| ≤ (MA + ea) * eb + MB * ea := by
    rw [show a.val * b.val - A * B = a.val * (b.val - B) + B * (a.val - A) by ring]
    calc _ ≤ |a.val * (b.val - B)| + |B * (a.val - A)| := abs_add_le _ _
      _ ≤ (MA + ea) * eb + MB * ea := by
          rw [abs_mul, abs_mul]
          exact add_le_add (mul_le_mul ha' h2 (abs_nonneg _) (by linarith [abs_nonneg A]))
            (mul_le_mul h4 h1 (abs_nonneg _) (le_trans (abs_nonneg _) h4))
  refine ⟨hf, ?_, ?_⟩
  · have := abs_sub_le (mul a b).val (a.val * b.val) (A * B)
    linarith
  · rw [abs_mul]; exact mul_le_mul h3 h4 (abs_nonneg _) (le_trans (abs_nonneg _) h3)

lemma addA {a b : F64} (ha : a.isFinite) (hb : b.isFinite) {A B ea eb MA MB E : ℚ}
    (h1 : |a.val - A| ≤ ea) (h2 : |b.val - B| ≤ eb) (h3 : |A| ≤ MA) (h4 : |B| ≤ MB)
    (hM : MA + ea + (MB + eb) ≤ 8) (hE : ep * (MA + ea + (MB + eb)) + (ea + eb) ≤ E) :
    (add a b).isFinite ∧ |(add a b).val - (A + B)| ≤ E ∧ |A + B| ≤ MA + MB := by
  have ha' := tri h1 h3
  have hb' := tri h2 h4
  have hab : |a.val + b.val| ≤ MA + ea + (MB + eb) := le_trans (abs_add_le _ _) (add_le_add ha' hb')
  have hf : (add a b).isFinite := by rw [add_eq_round ha hb]; exact fin_of_le_one (by linarith)
  have e := add_err_res ha hb hf
  simp only [binary64_prec] at e
  rw [show (2 : ℚ) ^ (-((53 : ℕ) : ℤ)) = 1 / 9007199254740992 by norm_num] at e
  have hm := abs_sub_abs_le_abs_sub (add a b).val (a.val + b.val)
  have hr : |(add a b).val - (a.val + b.val)| ≤ ep * (MA + ea + (MB + eb)) := by unfold ep; linarith
  have hd : |a.val + b.val - (A + B)| ≤ ea + eb := by
    rw [show a.val + b.val - (A + B) = (a.val - A) + (b.val - B) by ring]
    exact le_trans (abs_add_le _ _) (add_le_add h1 h2)
  refine ⟨hf, ?_, le_trans (abs_add_le _ _) (add_le_add h3 h4)⟩
  have := abs_sub_le (add a b).val (a.val + b.val) (A + B)
  linarith

lemma subA {a b : F64} (ha : a.isFinite) (hb : b.isFinite) {A B ea eb MA MB E : ℚ}
    (h1 : |a.val - A| ≤ ea) (h2 : |b.val - B| ≤ eb) (h3 : |A| ≤ MA) (h4 : |B| ≤ MB)
    (hM : MA + ea + (MB + eb) ≤ 8) (hE : ep * (MA + ea + (MB + eb)) + (ea + eb) ≤ E) :
    (sub a b).isFinite ∧ |(sub a b).val - (A - B)| ≤ E ∧ |A - B| ≤ MA + MB := by
  have hbn : b.neg.isFinite := by simpa using hb
  have := addA ha hbn (A := A) (B := -B) (MB := MB) h1 (by rw [FP.val_neg]; rw [show -b.val - -B = -(b.val - B) by ring, abs_neg]; exact h2)
    h3 (by rw [abs_neg]; exact h4) hM hE
  rwa [← sub_eq_add_neg] at this

/-- A datum is its own exact value. -/
lemma self_err (a : F64) : |a.val - a.val| ≤ 0 := by simp

end IEEE754.Dmath
