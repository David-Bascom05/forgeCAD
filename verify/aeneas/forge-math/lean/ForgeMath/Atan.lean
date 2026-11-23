/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMath.Dmath
import IEEE754.Dmath.AtanErr
import IEEE754.Dmath.Atan2
import IEEE754.Dmath.AsinAcos

/-!
# The shipped `atan`, `atan2`, `asin`, `acos` are the ieee session's models

As in `Dmath.lean`: each translated function returns `ok` (no panic, no overflow in `atan2`'s
exponent difference, no out-of-bounds table index) and its value is the model's
(verify/ieee754/IEEE754/Dmath/AtanErr.lean, Atan2.lean, AsinAcos.lean), so the models' contracts
hold for the shipped code: `atan_err`, `atan2_err50` (C2), `asin_err`, `acos_err` (C3).

The tables `ATANHI`, `ATANLO`, `AT` and the literals are checked against the model's constants
by `decide`; float comparisons against finite constants become rational comparisons
(`FP.lt_iff`, `FP.le_iff`), which is how the models state the branches.
-/

open Aeneas Aeneas.Std Result
open forge_math

namespace ForgeMath.Dmath

open IEEE754.Dmath

@[simp] lemma AT_0 : Array.index_usize dmath.atan.AT 0#usize = ok AT0 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_1 : Array.index_usize dmath.atan.AT 1#usize = ok AT1 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_2 : Array.index_usize dmath.atan.AT 2#usize = ok AT2 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_3 : Array.index_usize dmath.atan.AT 3#usize = ok AT3 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_4 : Array.index_usize dmath.atan.AT 4#usize = ok AT4 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_5 : Array.index_usize dmath.atan.AT 5#usize = ok AT5 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_6 : Array.index_usize dmath.atan.AT 6#usize = ok AT6 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_7 : Array.index_usize dmath.atan.AT 7#usize = ok AT7 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_8 : Array.index_usize dmath.atan.AT 8#usize = ok AT8 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_9 : Array.index_usize dmath.atan.AT 9#usize = ok AT9 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma AT_10 : Array.index_usize dmath.atan.AT 10#usize = ok AT10 := by
  unfold dmath.atan.AT; simp [Array.index_usize, Array.make]; decide
@[simp] lemma ATANHI_0 : Array.index_usize dmath.atan.ATANHI 0#usize = ok ATANHI0 := by
  unfold dmath.atan.ATANHI; simp [Array.index_usize, Array.make]; decide
@[simp] lemma ATANHI_1 : Array.index_usize dmath.atan.ATANHI 1#usize = ok ATANHI1 := by
  unfold dmath.atan.ATANHI; simp [Array.index_usize, Array.make]; decide
@[simp] lemma ATANHI_2 : Array.index_usize dmath.atan.ATANHI 2#usize = ok ATANHI2 := by
  unfold dmath.atan.ATANHI; simp [Array.index_usize, Array.make]; decide
@[simp] lemma ATANHI_3 : Array.index_usize dmath.atan.ATANHI 3#usize = ok ATANHI3 := by
  unfold dmath.atan.ATANHI; simp [Array.index_usize, Array.make]; decide
@[simp] lemma ATANLO_0 : Array.index_usize dmath.atan.ATANLO 0#usize = ok ATANLO0 := by
  unfold dmath.atan.ATANLO; simp [Array.index_usize, Array.make]; decide
@[simp] lemma ATANLO_1 : Array.index_usize dmath.atan.ATANLO 1#usize = ok ATANLO1 := by
  unfold dmath.atan.ATANLO; simp [Array.index_usize, Array.make]; decide
@[simp] lemma ATANLO_2 : Array.index_usize dmath.atan.ATANLO 2#usize = ok ATANLO2 := by
  unfold dmath.atan.ATANLO; simp [Array.index_usize, Array.make]; decide
@[simp] lemma ATANLO_3 : Array.index_usize dmath.atan.ATANLO 3#usize = ok ATANLO3 := by
  unfold dmath.atan.ATANLO; simp [Array.index_usize, Array.make]; decide

lemma big_eq : F64.Ops.ofBits 4904419994206470144 = BIG66 := by decide
lemma tiny_eq : F64.Ops.ofBits 4485585228861014016 = TINY27 := by decide
lemma two_eq : F64.Ops.ofBits 4611686018427387904 = TWO := by decide
lemma threehalf_eq : F64.Ops.ofBits 4609434218613702656 = THREEHALF := by decide
lemma mone_eq : F64.Ops.ofBits 13830554455654793216 = MONE := by decide

/-- A binary64 constant and its value. -/
lemma cval {c : F64} {m : ℕ} {e : ℤ} {h} (hc : c = .fin false m e h) : c.isFinite ∧ c.val = m * 2 ^ e := by
  subst hc; exact ⟨rfl, by simp [IEEE754.FP.val, IEEE754.FP.sgn]⟩

lemma c7 : (F64.Ops.ofBits 4601552919265804288).isFinite ∧ (F64.Ops.ofBits 4601552919265804288).val = 7 / 16 := by
  obtain ⟨f, v⟩ := cval (show F64.Ops.ofBits 4601552919265804288 = .fin false 7881299347898368 (-54) (by decide) by decide)
  exact ⟨f, by rw [v]; norm_num⟩
lemma c19 : (F64.Ops.ofBits 4608026843730149376).isFinite ∧ (F64.Ops.ofBits 4608026843730149376).val = 19 / 16 := by
  obtain ⟨f, v⟩ := cval (show F64.Ops.ofBits 4608026843730149376 = .fin false 5348024557502464 (-52) (by decide) by decide)
  exact ⟨f, by rw [v]; norm_num⟩
lemma c11 : (F64.Ops.ofBits 4604367669032910848).isFinite ∧ (F64.Ops.ofBits 4604367669032910848).val = 11 / 16 := by
  obtain ⟨f, v⟩ := cval (show F64.Ops.ofBits 4604367669032910848 = .fin false 6192449487634432 (-53) (by decide) by decide)
  exact ⟨f, by rw [v]; norm_num⟩
lemma c39 : (F64.Ops.ofBits 4612671180845875200).isFinite ∧ (F64.Ops.ofBits 4612671180845875200).val = 39 / 16 := by
  obtain ⟨f, v⟩ := cval (show F64.Ops.ofBits 4612671180845875200 = .fin false 5488762045857792 (-51) (by decide) by decide)
  exact ⟨f, by rw [v]; norm_num⟩

lemma flt {a c : F64} (ha : a.isFinite) (hc : c.isFinite) : IEEE754.FP.lt a c = decide (a.val < c.val) := by
  by_cases h : a.val < c.val
  · simp [h, (IEEE754.FP.lt_iff ha hc).mpr h]
  · simp only [h, decide_false]; rw [Bool.eq_false_iff]; exact fun h' => h ((IEEE754.FP.lt_iff ha hc).mp h')
lemma fle {a c : F64} (ha : a.isFinite) (hc : c.isFinite) : IEEE754.FP.le a c = decide (a.val ≤ c.val) := by
  by_cases h : a.val ≤ c.val
  · simp [h, (IEEE754.FP.le_iff ha hc).mpr h]
  · simp only [h, decide_false]; rw [Bool.eq_false_iff]; exact fun h' => h ((IEEE754.FP.le_iff ha hc).mp h')

lemma hcast_0 : IScalar.hcast .Usize 0#i32 = 0#usize := by
  apply UScalar.eq_of_val_eq; simp [IScalar.hcast_val_eq]
lemma hcast_1 : IScalar.hcast .Usize 1#i32 = 1#usize := by
  apply UScalar.eq_of_val_eq; simp [IScalar.hcast_val_eq]
  (rw [Int.emod_eq_of_lt] <;> [rfl; norm_num; (rcases System.Platform.numBits_eq with h | h <;> rw [h] <;> norm_num)])
lemma hcast_2 : IScalar.hcast .Usize 2#i32 = 2#usize := by
  apply UScalar.eq_of_val_eq; simp [IScalar.hcast_val_eq]
  (rw [Int.emod_eq_of_lt] <;> [rfl; norm_num; (rcases System.Platform.numBits_eq with h | h <;> rw [h] <;> norm_num)])
lemma hcast_3 : IScalar.hcast .Usize 3#i32 = 3#usize := by
  apply UScalar.eq_of_val_eq; simp [IScalar.hcast_val_eq]
  (rw [Int.emod_eq_of_lt] <;> [rfl; norm_num; (rcases System.Platform.numBits_eq with h | h <;> rw [h] <;> norm_num)])

theorem atan_eq (x : F64) : dmath.atan x = ok (IEEE754.Dmath.atan x) := by
  unfold dmath.atan
  simp only [core.f64.F64.is_nan, core.f64.F64.is_sign_negative, core.f64.F64.abs, lift, bind_ok,
    AT_0, AT_1, AT_2, AT_3, AT_4, AT_5, AT_6, AT_7, AT_8, AT_9, AT_10, ATANHI_3, ATANLO_3,
    big_eq, tiny_eq, two_eq, threehalf_eq, mone_eq, one_eq,
    F64.Ops.mul_def, F64.Ops.add_def, F64.Ops.sub_def, F64.Ops.div_def, F64.Ops.neg_def,
    F64.Ops.lt_def, F64.Ops.ge_def]
  by_cases hn : x.isNaN
  · simp [hn, IEEE754.Dmath.atan]
  simp only [hn, Bool.false_eq_true, ite_false]
  by_cases hf : x.isFinite
  · have fa : (IEEE754.FP.abs x).isFinite := by simpa using hf
    obtain ⟨f7, v7⟩ := c7
    obtain ⟨f19, v19⟩ := c19
    obtain ⟨f11, v11⟩ := c11
    obtain ⟨f39, v39⟩ := c39
    rw [fle (by decide) fa, flt fa f7, flt fa (by decide), flt fa f19, flt fa f11, flt fa f39, v7, v19, v11, v39]
    simp only [IEEE754.Dmath.atan, IEEE754.Dmath.atanR, hn, hf, decide_eq_true_eq, Bool.false_eq_true, ite_false, ite_true]
    have hm1 : (-1)#i32 < 0#i32 := by decide
    simp only [hm1, ite_true]
    split_ifs <;> simp [hcast_0, hcast_1, hcast_2, hcast_3, IEEE754.Dmath.atanTail,
      IEEE754.Dmath.atanPoly]
  · -- `±∞`: `|x| = +∞ ≥ 2^66`
    cases x with
    | inf s =>
      have hb : IEEE754.FP.le BIG66 (IEEE754.FP.abs (.inf s : F64)) = true := by cases s <;> decide
      simp [hb, IEEE754.Dmath.atan, IEEE754.FP.isNaN, IEEE754.FP.isFinite, IEEE754.FP.sign]
      cases s <;> rfl
    | _ => simp_all [IEEE754.FP.isNaN, IEEE754.FP.isFinite]

/-! ## `atan2` -/

lemma zero_eq : F64.Ops.ofBits 0 = .zero false := by decide
lemma four_eq : F64.Ops.ofBits 4616189618054758400 = FOUR := by decide
lemma three_eq : F64.Ops.ofBits 4613937818241073152 = THREE := by decide
lemma pi_lo_eq : dmath.atan2.PI_LO = PI_LO := by unfold dmath.atan2.PI_LO; decide
lemma pi_eq : dmath.PI = ok PI := by
  unfold dmath.PI core.f64.consts.PI; congr 1
lemma frac_pi_2_eq : dmath.FRAC_PI_2 = ok FRAC_PI_2 := by
  unfold dmath.FRAC_PI_2 core.f64.consts.FRAC_PI_2; congr 1

lemma fin_val_ne {s : Bool} {m : ℕ} {e : ℤ} {h} : (IEEE754.FP.fin s m e h : F64).val ≠ 0 := by
  have hm : 0 < m := h.1
  simp only [IEEE754.FP.val, IEEE754.FP.sgn]
  have : (0 : ℚ) < m * 2 ^ e := mul_pos (by exact_mod_cast hm) (zpow_pos (by norm_num) _)
  cases s <;> simp [this.ne']

/-- `y == 0.0` is `y`'s being a zero. -/
lemma eqz (y : F64) : IEEE754.FP.eq y (.zero false) = y.isZero := by
  cases y with
  | fin s m e h =>
    have := @fin_val_ne s m e h
    simp only [IEEE754.FP.eq, IEEE754.FP.cmp, IEEE754.FP.isZero]
    simpa [IEEE754.FP.val] using this
  | inf s => cases s <;> simp [IEEE754.FP.eq, IEEE754.FP.cmp, IEEE754.FP.isZero]
  | _ => simp [IEEE754.FP.eq, IEEE754.FP.cmp, IEEE754.FP.isZero, IEEE754.FP.val]

lemma fin_pos {m : ℕ} {e : ℤ} (hm : 0 < m) : (0 : ℚ) < m * 2 ^ e :=
  mul_pos (by exact_mod_cast hm) (zpow_pos (by norm_num) _)

/-- `y < 0.0` is the sign bit of a nonzero `y`. -/
lemma ltz (y : F64) : IEEE754.FP.lt y (.zero false) = (y.sign && !y.isZero) := by
  cases y with
  | fin s m e h =>
    have := fin_pos (e := e) h.1
    cases s <;> simp [IEEE754.FP.lt, IEEE754.FP.cmp, IEEE754.FP.isZero, IEEE754.FP.val, IEEE754.FP.sgn,
      IEEE754.FP.sign, compare_lt_iff_lt, this, this.le]
  | inf s => cases s <;> simp [IEEE754.FP.lt, IEEE754.FP.cmp, IEEE754.FP.isZero, IEEE754.FP.sign]
  | _ => simp [IEEE754.FP.lt, IEEE754.FP.cmp, IEEE754.FP.isZero, IEEE754.FP.val, IEEE754.FP.sign]

/-- `x > 0.0`: a positive `x`. -/
lemma gtz (x : F64) : IEEE754.FP.lt (.zero false) x = (!x.sign && !x.isZero && !x.isNaN) := by
  cases x with
  | fin s m e h =>
    have := fin_pos (e := e) h.1
    cases s <;> simp [IEEE754.FP.lt, IEEE754.FP.cmp, IEEE754.FP.isZero, IEEE754.FP.val, IEEE754.FP.sgn,
      IEEE754.FP.sign, IEEE754.FP.isNaN, compare_lt_iff_lt, this, this.le]
  | inf s => cases s <;> simp [IEEE754.FP.lt, IEEE754.FP.cmp, IEEE754.FP.isZero, IEEE754.FP.sign, IEEE754.FP.isNaN]
  | _ => simp [IEEE754.FP.lt, IEEE754.FP.cmp, IEEE754.FP.isZero, IEEE754.FP.val, IEEE754.FP.sign, IEEE754.FP.isNaN]

/-- **`atan2` is the model's `atan2`**: the special cases in order, then the exponent gap `k`. -/
theorem atan2_spec (y x : F64) : dmath.atan2 y x ⦃ r => r = IEEE754.Dmath.atan2 y x ⦄ := by
  unfold dmath.atan2
  simp only [core.f64.F64.is_nan, core.f64.F64.is_sign_negative, core.f64.F64.is_infinite,
    core.f64.F64.abs, core.f64.F64.NAN, lift, bind_ok, pi_eq, frac_pi_2_eq, pi_lo_eq, zero_eq,
    four_eq, three_eq, half_eq, F64.Ops.mul_def, F64.Ops.add_def, F64.Ops.sub_def, F64.Ops.div_def,
    F64.Ops.neg_def, F64.Ops.lt_def, F64.Ops.gt_def, F64.Ops.eq_def', eqz, ltz, gtz, atan_eq]
  by_cases hxn : x.isNaN
  · simp [hxn, IEEE754.Dmath.atan2]
  rw [Bool.not_eq_true] at hxn
  by_cases hyn : y.isNaN
  · simp [hxn, hyn, IEEE754.Dmath.atan2]
  rw [Bool.not_eq_true] at hyn
  by_cases hyz : y.isZero
  · simp only [hxn, hyn, hyz, IEEE754.Dmath.atan2]; split_ifs <;> simp_all
  rw [Bool.not_eq_true] at hyz
  by_cases hxz : x.isZero
  · simp only [hxn, hyn, hyz, hxz, IEEE754.Dmath.atan2]; split_ifs <;> simp_all
  rw [Bool.not_eq_true] at hxz
  by_cases hxi : x.isInf
  · simp only [hxn, hyn, hyz, hxz, hxi, IEEE754.Dmath.atan2]; split_ifs <;> simp_all
  rw [Bool.not_eq_true] at hxi
  by_cases hyi : y.isInf
  · simp only [hxn, hyn, hyz, hxz, hxi, hyi, IEEE754.Dmath.atan2]; split_ifs <;> simp_all
  rw [Bool.not_eq_true] at hyi
  simp only [hxn, hyn, hyz, hxz, hxi, hyi, Bool.false_eq_true, ite_false, Bool.not_false, Bool.and_true,
]
  step*
  have hk : (k : ℤ) = (expo y : ℤ) - expo x := by rw [k_post, i_post, i1_post]
  simp only [IEEE754.Dmath.atan2, hxn, hyn, hyz, hxz, hxi, hyi, Bool.false_eq_true, ite_false, Bool.or_false]
  have g60 : (k > 60#i32) = ((60 : ℤ) < (expo y : ℤ) - expo x) := by
    rw [← hk]; apply propext; simp only [gt_iff_lt, IScalar.lt_equiv]; rfl
  have l60 : (k < (-60)#i32) = ((expo y : ℤ) - expo x < -60) := by
    rw [← hk]; apply propext; simp only [IScalar.lt_equiv]; rfl
  simp only [g60, l60]
  by_cases c1 : (60 : ℤ) < (expo y : ℤ) - expo x
  · simp only [c1, ite_true, bind_ok]; split_ifs <;> simp
  · simp only [c1, ite_false]
    by_cases c2 : (expo y : ℤ) - expo x < -60 <;> by_cases sx : x.sign <;> by_cases sy : y.sign <;>
      simp [c2, sx, sy]

/-! ## `asin`, `acos` -/

/-- `(-1.0..=1.0).contains(&x)`: `-1 <= x && x <= 1`, false for NaN. -/
lemma contains_eq (x : F64) :
    core.ops.range.RangeInclusive.contains F64.Insts.CoreCmpPartialOrdF64 F64.Insts.CoreCmpPartialOrdF64
      F64.Insts.CoreCmpPartialOrdF64 ⟨MONE, ONE, false⟩ x =
      ok (decide (x.isFinite ∧ -1 ≤ x.val ∧ x.val ≤ 1)) := by
  simp only [core.ops.range.RangeInclusive.contains, core.cmp.PartialOrd.le_body,
    F64.Insts.CoreCmpPartialOrdF64.partial_cmp]
  cases x with
  | fin s m e h =>
    have hf : (IEEE754.FP.fin s m e h : F64).isFinite := rfl
    have hm : MONE.isFinite := by decide
    have ho : ONE.isFinite := by decide
    have l1 := IEEE754.FP.le_iff hm hf
    have l2 := IEEE754.FP.le_iff hf ho
    simp only [IEEE754.FP.le, beq_iff_eq, Bool.or_eq_true] at l1 l2
    have vm : MONE.val = -1 := by simp [MONE, IEEE754.FP.val, IEEE754.FP.sgn]; norm_num
    have vo : ONE.val = 1 := by simp [ONE, IEEE754.FP.val, IEEE754.FP.sgn]; norm_num
    rw [vm] at l1; rw [vo] at l2
    by_cases a : -1 ≤ (IEEE754.FP.fin s m e h : F64).val <;>
      by_cases b : (IEEE754.FP.fin s m e h : F64).val ≤ 1 <;> simp_all
  | zero s =>
    have : (IEEE754.FP.zero s : F64).val = 0 := rfl
    simp [IEEE754.FP.cmp, MONE, ONE, IEEE754.FP.val, IEEE754.FP.sgn, IEEE754.FP.isFinite]
    norm_num [compare_lt_iff_lt, compare_eq_iff_eq]
  | inf s => cases s <;> simp [IEEE754.FP.cmp, MONE, ONE, IEEE754.FP.isFinite]
  | nan => simp [IEEE754.FP.cmp, IEEE754.FP.isFinite]

lemma sqrt_eq (x : F64) : std.f64.F64.sqrt x = ok (IEEE754.FP.sqrt x) := rfl

/-- **`asin` is the model's `asin`.** -/
theorem asin_spec (x : F64) : dmath.asin x ⦃ r => r = IEEE754.Dmath.asin x ⦄ := by
  unfold dmath.asin
  simp only [core.ops.range.RangeInclusive.new, bind_ok, mone_eq, one_eq, contains_eq, lift, sqrt_eq,
    F64.Ops.mul_def, F64.Ops.add_def, F64.Ops.sub_def, core.f64.F64.NAN]
  by_cases h : x.isFinite ∧ -1 ≤ x.val ∧ x.val ≤ 1
  · simp only [h, IEEE754.Dmath.asin, IEEE754.Dmath.sqrtArg]
    exact atan2_spec _ _
  · simp [h, IEEE754.Dmath.asin]

/-- **`acos` is the model's `acos`.** -/
theorem acos_spec (x : F64) : dmath.acos x ⦃ r => r = IEEE754.Dmath.acos x ⦄ := by
  unfold dmath.acos
  simp only [core.ops.range.RangeInclusive.new, bind_ok, mone_eq, one_eq, contains_eq, lift, sqrt_eq,
    F64.Ops.mul_def, F64.Ops.add_def, F64.Ops.sub_def, core.f64.F64.NAN]
  by_cases h : x.isFinite ∧ -1 ≤ x.val ∧ x.val ≤ 1
  · simp only [h, IEEE754.Dmath.acos, IEEE754.Dmath.sqrtArg]
    exact atan2_spec _ _
  · simp [h, IEEE754.Dmath.acos]

/-! ## The contracts, for the shipped code -/

/-- **`dmath::atan` as shipped**: for finite `x`, finite and within `3·10^-16` of `arctan x`. -/
theorem atan_err (x : F64) (hx : x.isFinite) :
    dmath.atan x ⦃ r => r.isFinite ∧ |(r.val : ℝ) - Real.arctan x.val| ≤ 3 / 10 ^ 16 ⦄ := by
  rw [atan_eq]; exact (WP.spec_ok _).mpr (IEEE754.Dmath.atan_err hx)

/-- **Contract C2 for the shipped `dmath::atan2`**: finite `y`, `x`, not both zero, and not
`y = −0` with `x < 0`: finite and within `2^-50` of the angle of `(x, y)`. -/
theorem atan2_err50 (y x : F64) (hx : x.isFinite) (hy : y.isFinite) (hne : x.val ≠ 0 ∨ y.val ≠ 0)
    (hsz : y.val = 0 → y.sign = false ∨ x.sign = false) :
    dmath.atan2 y x ⦃ r => r.isFinite ∧ |(r.val : ℝ) - atan2R y.val x.val| ≤ 1 / 2 ^ 50 ⦄ :=
  WP.spec_mono (atan2_spec y x) (fun _ h => h ▸ IEEE754.Dmath.atan2_err50 hx hy hne hsz)

/-- **Contract C3 for the shipped `dmath::asin`**: within `2^-50` of `arcsin x` on `[−1, 1]`. -/
theorem asin_err (x : F64) (hx : x.isFinite) (h0 : -1 ≤ x.val) (h1 : x.val ≤ 1) :
    dmath.asin x ⦃ r => r.isFinite ∧ |(r.val : ℝ) - Real.arcsin x.val| ≤ 1 / 2 ^ 50 ⦄ :=
  WP.spec_mono (asin_spec x) (fun _ h => h ▸ IEEE754.Dmath.asin_err hx h0 h1)

/-- **Contract C3 for the shipped `dmath::acos`**: within `2^-50` of `arccos x` on `[−1, 1]`. -/
theorem acos_err (x : F64) (hx : x.isFinite) (h0 : -1 ≤ x.val) (h1 : x.val ≤ 1) :
    dmath.acos x ⦃ r => r.isFinite ∧ |(r.val : ℝ) - Real.arccos x.val| ≤ 1 / 2 ^ 50 ⦄ :=
  WP.spec_mono (acos_spec x) (fun _ h => h ▸ IEEE754.Dmath.acos_err hx h0 h1)

end ForgeMath.Dmath
