/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import AeneasF64.Lemmas

/-!
# One error bound per operation, for translated code

For finite operands and a finite result, each basic operation is within `2⁻⁵³` of the exact
value relatively, plus `2⁻¹⁰⁷⁵` absolutely (the subnormal range):

  `|(F64.Ops.add a b).val − (a.val + b.val)| ≤ 2⁻⁵³·|a.val + b.val| + 2⁻¹⁰⁷⁵`

and the same for `sub`, `mul` and `div` (`b.val ≠ 0`). This is `IEEE754.round_err` (its `max`
bounded by the sum), through `add_eq_round` and the others. These are the forms forward error
analyses of translated float code use. The sharper forms (relative only, outside the subnormal
range; relative to the result) are verify/ieee754's `F64.add_rel_err`, `add_err_res` and so on.
-/

namespace F64.Ops

open IEEE754 hiding F64

/-- `2⁻⁵³` -/
abbrev u : ℚ := 2 ^ (-53 : ℤ)
/-- `2⁻¹⁰⁷⁵` -/
abbrev η : ℚ := 2 ^ (-1075 : ℤ)

lemma round_err' {zs : Bool} {r : ℚ} (h : (round binary64 zs r).isFinite) :
    |(round binary64 zs r).val - r| ≤ u * |r| + η := by
  have e := IEEE754.round_err binary64 (zs := zs) (K := ℚ) h
  have h1 : (0 : ℚ) ≤ u * |r| := by positivity
  have h2 : (0 : ℚ) ≤ η := by positivity
  have hp : (2 : ℚ) ^ (-(binary64.prec : ℤ)) = u := rfl
  have hq : (2 : ℚ) ^ (binary64.qmin - 1) = η := rfl
  rw [hp, hq] at e
  exact le_trans e (max_le (by linarith) (by linarith))

theorem add_err {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (h : (add a b).isFinite) :
    |(add a b).val - (a.val + b.val)| ≤ u * |a.val + b.val| + η := by
  simp only [add_def, IEEE754.add_eq_round ha hb] at h ⊢; exact round_err' h

theorem sub_err {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (h : (sub a b).isFinite) :
    |(sub a b).val - (a.val - b.val)| ≤ u * |a.val - b.val| + η := by
  simp only [sub_def, IEEE754.sub_eq_round ha hb] at h ⊢; exact round_err' h

theorem mul_err {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (h : (mul a b).isFinite) :
    |(mul a b).val - (a.val * b.val)| ≤ u * |a.val * b.val| + η := by
  simp only [mul_def, IEEE754.mul_eq_round ha hb] at h ⊢; exact round_err' h

theorem div_err {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hb0 : b.val ≠ 0)
    (h : (div a b).isFinite) :
    |(div a b).val - (a.val / b.val)| ≤ u * |a.val / b.val| + η := by
  simp only [div_def, IEEE754.div_eq_round ha hb hb0] at h ⊢; exact round_err' h

end F64.Ops
