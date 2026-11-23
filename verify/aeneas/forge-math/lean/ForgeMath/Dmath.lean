/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMath.Refine
import IEEE754.Dmath.KSin
import IEEE754.Dmath.KCos
import IEEE754.Dmath.RemPio2
import IEEE754.Dmath.SinCos

/-!
# The shipped `forge_math::dmath` is the ieee session's `IEEE754.Dmath` models

Each lemma says that the translated Rust function returns `ok`, so it cannot panic, and that its
value is the model's function, which is written operation for operation from the same Rust
(verify/ieee754/IEEE754/Dmath). The models' error contracts (`ksin_eval`, `kcos_eval`,
`sinCos_err`, …) then hold for the shipped code by one rewrite.

The constants: each Rust `const` becomes a translated global `F64.Ops.ofBits <rustc's bits>`, and
`decide` checks that it is the model's constant (whose `_bits` theorem states the same pattern).
-/

open Aeneas Aeneas.Std Result
open forge_math

namespace ForgeMath.Dmath

open IEEE754.Dmath

lemma S1_eq : dmath.k_sin.S1 = S1 := by unfold dmath.k_sin.S1; decide
lemma S2_eq : dmath.k_sin.S2 = S2 := by unfold dmath.k_sin.S2; decide
lemma S3_eq : dmath.k_sin.S3 = S3 := by unfold dmath.k_sin.S3; decide
lemma S4_eq : dmath.k_sin.S4 = S4 := by unfold dmath.k_sin.S4; decide
lemma S5_eq : dmath.k_sin.S5 = S5 := by unfold dmath.k_sin.S5; decide
lemma S6_eq : dmath.k_sin.S6 = S6 := by unfold dmath.k_sin.S6; decide
lemma C1_eq : dmath.k_cos.C1 = C1 := by unfold dmath.k_cos.C1; decide
lemma C2_eq : dmath.k_cos.C2 = C2 := by unfold dmath.k_cos.C2; decide
lemma C3_eq : dmath.k_cos.C3 = C3 := by unfold dmath.k_cos.C3; decide
lemma C4_eq : dmath.k_cos.C4 = C4 := by unfold dmath.k_cos.C4; decide
lemma C5_eq : dmath.k_cos.C5 = C5 := by unfold dmath.k_cos.C5; decide
lemma C6_eq : dmath.k_cos.C6 = C6 := by unfold dmath.k_cos.C6; decide
/-- the literal `0.5` -/
lemma half_eq : F64.Ops.ofBits 4602678819172646912 = HALF := by decide
/-- the literal `1.0` -/
lemma one_eq : F64.Ops.ofBits 4607182418800017408 = ONE := by decide

/-- **`k_sin` is the model's `ksin`**, operation for operation. -/
theorem k_sin_eq (x y : F64) : dmath.k_sin x y = ok (ksin x y) := by
  simp only [dmath.k_sin, ksin, lift, bind_ok, S1_eq, S2_eq, S3_eq, S4_eq, S5_eq, S6_eq, half_eq,
    F64.Ops.mul_def, F64.Ops.add_def, F64.Ops.sub_def]

/-- **`k_cos` is the model's `kcos`**, operation for operation. -/
theorem k_cos_eq (x y : F64) : dmath.k_cos x y = ok (kcos x y) := by
  simp only [dmath.k_cos, kcos, lift, bind_ok, C1_eq, C2_eq, C3_eq, C4_eq, C5_eq, C6_eq, half_eq,
    one_eq, F64.Ops.mul_def, F64.Ops.add_def, F64.Ops.sub_def]

/-- `exponent` is the 11-bit exponent field, `expo`. -/
@[step]
theorem exponent_spec (x : F64) : dmath.exponent x ⦃ e => e.val = (IEEE754.Dmath.expo x : ℤ) ∧ 0 ≤ e.val ∧ e.val < 2048 ⦄ := by
  unfold dmath.exponent core.f64.F64.to_bits
  step*
  have h1 : i2.val = IEEE754.FP.toBits x / 2 ^ 52 % 2 ^ 11 := by
    rw [i2_post, UScalar.val_and, i1_post, Nat.shiftRight_eq_div_pow]
    have := Nat.and_two_pow_sub_one_eq_mod (IEEE754.FP.toBits x / 2 ^ 52) 11
    norm_num at this ⊢; exact this
  have h2 : i2.val < 2048 := by rw [h1]; omega
  have h3 : (UScalar.hcast IScalarTy.I32 i2).val = (i2.val : ℤ) := by
    rw [UScalar.hcast_val_eq, Int.bmod_eq_of_le] <;> simp [IScalarTy.numBits] <;> omega
  rw [h3, h1, IEEE754.Dmath.expo]; omega

open IEEE754.Dmath in
lemma toint_eq : F64.Ops.div (F64.Ops.ofBits 4609434218613702656) (F64.Ops.ofBits 4372995238176751616) = TOINT := by
  decide +kernel
open IEEE754.Dmath in
lemma consts_eq : dmath.INVPIO2 = INVPIO2 ∧ dmath.PIO2_1 = PIO2_1 ∧ dmath.PIO2_1T = PIO2_1T ∧
    dmath.PIO2_2 = PIO2_2 ∧ dmath.PIO2_2T = PIO2_2T ∧ dmath.PIO2_3 = PIO2_3 ∧ dmath.PIO2_3T = PIO2_3T := by
  unfold dmath.INVPIO2 dmath.PIO2_1 dmath.PIO2_1T dmath.PIO2_2 dmath.PIO2_2T dmath.PIO2_3 dmath.PIO2_3T
  decide

lemma toI32_val (x : F64) :
    (F64.Ops.toIScalar .I32 x).val = IEEE754.FP.toIntSat (-2 ^ 31) (2 ^ 31 - 1) x := by
  simp only [F64.Ops.toIScalar, IScalar.ofInt_val_eq]; norm_num [IScalarTy.numBits]

/-- **`rem_pio2` is the model's `remPio2`**: the quadrant `n` (`f as i32`) and the reduced pair. -/
theorem rem_pio2_spec (x : F64) : dmath.rem_pio2 x ⦃ r => r.1.val = (IEEE754.Dmath.remPio2 x).1 ∧ r.2 = (IEEE754.Dmath.remPio2 x).2 ⦄ := by
  obtain ⟨c1, c2, c3, c4, c5, c6, c7⟩ := consts_eq
  simp only [dmath.rem_pio2, lift, bind_ok, dmath.TOINT, core.f64.F64.EPSILON, toint_eq, c1, c2, c3, c4, c5, c6, c7]
  step*
  split
  · step*
    split
    · step*
      simp only [IEEE754.Dmath.remPio2, toI32_val, F64.Ops.add_def, F64.Ops.sub_def, F64.Ops.mul_def] at *
      split_ifs <;> first | exact ⟨rfl, rfl⟩ | (exfalso; scalar_tac)
    · step*
      simp only [IEEE754.Dmath.remPio2, toI32_val, F64.Ops.add_def, F64.Ops.sub_def, F64.Ops.mul_def] at *
      split_ifs <;> first | exact ⟨rfl, rfl⟩ | (exfalso; scalar_tac)
  · step*
    simp only [IEEE754.Dmath.remPio2, toI32_val, F64.Ops.add_def, F64.Ops.sub_def, F64.Ops.mul_def] at *
    split_ifs <;> first | exact ⟨rfl, rfl⟩ | (exfalso; scalar_tac)

lemma tiny_spec : (F64.Ops.div ONE (F64.Ops.ofBits 4728779608739020800)).isFinite ∧
    (F64.Ops.div ONE (F64.Ops.ofBits 4728779608739020800)).val = 1 / 134217728 := by
  have : F64.Ops.div ONE (F64.Ops.ofBits 4728779608739020800) =
      .fin false 4503599627370496 (-79) (by decide) := by decide +kernel
  rw [this]; refine ⟨rfl, ?_⟩; simp [IEEE754.FP.val, IEEE754.FP.sgn]; norm_num

lemma bv_and3 (x : BitVec 32) : (x &&& 3#32).toInt = x.toInt % 4 := by
  have h1 : (x &&& 3#32).toNat = x.toNat % 4 := by
    rw [BitVec.toNat_and]; have := Nat.and_two_pow_sub_one_eq_mod x.toNat 2; simpa using this
  rw [BitVec.toInt_eq_toNat_bmod, BitVec.toInt_eq_toNat_bmod, h1]
  have hx := x.isLt
  simp only [Int.bmod]; split_ifs <;> omega

lemma and3_val (n : I32) : (n &&& 3#i32).val = n.val % 4 := by
  simp only [IScalar.val, IScalar.bv_and]; exact bv_and3 _

/-- **`sin_cos` is the model's `sinCos`**: the tiny-argument shortcut, the reduction, the
quadrant dispatch on `n & 3` (`n % 4`), and NaN for non-finite input. -/
theorem sin_cos_spec (x : F64) : dmath.sin_cos x ⦃ r => r = sinCos x ⦄ := by
  unfold dmath.sin_cos
  simp only [core.f64.F64.is_finite, core.f64.F64.abs, lift, bind_ok, core.f64.F64.NAN, one_eq]
  by_cases hf : x.isFinite
  · simp only [hf, ite_true]
    obtain ⟨hcf, hcv⟩ := tiny_spec
    by_cases ht : |x.val| < 1 / 134217728
    · have hlt : F64.Ops.lt (IEEE754.FP.abs x) (F64.Ops.div ONE (F64.Ops.ofBits 4728779608739020800)) = true := by
        rw [F64.Ops.lt_def, IEEE754.FP.lt_iff (by simpa using hf) hcf, IEEE754.FP.abs_val, hcv]; exact ht
      simp only [hlt, ite_true, WP.spec_ok, sinCos, hf, ht]
    · have hlt : F64.Ops.lt (IEEE754.FP.abs x) (F64.Ops.div ONE (F64.Ops.ofBits 4728779608739020800)) = false := by
        rw [Bool.eq_false_iff, ne_eq, F64.Ops.lt_def, IEEE754.FP.lt_iff (by simpa using hf) hcf, IEEE754.FP.abs_val, hcv]; exact ht
      simp only [hlt, Bool.false_eq_true, ite_false]
      apply WP.spec_bind (rem_pio2_spec x)
      rintro ⟨n, y0, y1⟩ ⟨hn, hy⟩
      simp only [k_sin_eq, k_cos_eq, bind_ok]
      have h4 := and3_val n
      have hm : (IEEE754.Dmath.remPio2 x).2 = (y0, y1) := hy.symm
      simp only [sinCos, hf, ht, ite_true, ite_false, hm, ← hn, F64.Ops.neg_def]
      show (match n &&& 3#i32 with
        | 0#32#iscalar => ok (ksin y0 y1, kcos y0 y1)
        | 1#32#iscalar => ok (kcos y0 y1, IEEE754.FP.neg (ksin y0 y1))
        | 2#32#iscalar => ok (IEEE754.FP.neg (ksin y0 y1), IEEE754.FP.neg (kcos y0 y1))
        | _ => ok (IEEE754.FP.neg (kcos y0 y1), ksin y0 y1)) ⦃ r => r = _ ⦄
      generalize n &&& 3#i32 = k at h4 ⊢
      have hr : 0 ≤ (n : ℤ) % 4 ∧ (n : ℤ) % 4 < 4 := ⟨Int.emod_nonneg _ (by norm_num), Int.emod_lt_of_pos _ (by norm_num)⟩
      have hk : k = 0#i32 ∨ k = 1#i32 ∨ k = 2#i32 ∨ k = 3#i32 := by
        simp only [IScalar.eq_equiv]
        have e0 : (0#i32 : I32).val = 0 := rfl
        have e1 : (1#i32 : I32).val = 1 := rfl
        have e2 : (2#i32 : I32).val = 2 := rfl
        have e3 : (3#i32 : I32).val = 3 := rfl
        rw [e0, e1, e2, e3]; omega
      rcases hk with rfl | rfl | rfl | rfl
      · have : (n : ℤ) % 4 = 0 := by rw [← h4]; rfl
        simp only [this]; exact (WP.spec_ok _).mpr (by simp)
      · have : (n : ℤ) % 4 = 1 := by rw [← h4]; rfl
        simp only [this]; exact (WP.spec_ok _).mpr (by simp)
      · have : (n : ℤ) % 4 = 2 := by rw [← h4]; rfl
        simp only [this]; exact (WP.spec_ok _).mpr (by simp)
      · have : (n : ℤ) % 4 = 3 := by rw [← h4]; rfl
        simp only [this]; exact (WP.spec_ok _).mpr (by simp)
  · simp [hf, sinCos]

/-- **`sin` is the model's `sin`.** -/
theorem sin_spec (x : F64) : dmath.sin x ⦃ r => r = IEEE754.Dmath.sin x ⦄ := by
  unfold dmath.sin
  apply WP.spec_bind (sin_cos_spec x)
  rintro ⟨s, c⟩ h
  simp [IEEE754.Dmath.sin, ← h]

/-- **`cos` is the model's `cos`.** -/
theorem cos_spec (x : F64) : dmath.cos x ⦃ r => r = IEEE754.Dmath.cos x ⦄ := by
  unfold dmath.cos
  apply WP.spec_bind (sin_cos_spec x)
  rintro ⟨s, c⟩ h
  simp [IEEE754.Dmath.cos, ← h]

/-! ## The contracts, for the shipped code -/

/-- **Contract C1 for the shipped `dmath::sin`**: for finite `|x| ≤ 2^20` it returns a finite value
within `2^-52` of `sin x` (the ieee session's `sin_err`, through `sin_spec`). -/
theorem sin_err (x : F64) (hx : x.isFinite) (hb : |x.val| ≤ 1048576) :
    dmath.sin x ⦃ r => r.isFinite ∧ |(r.val : ℝ) - Real.sin x.val| ≤ 1 / 2 ^ 52 ⦄ :=
  WP.spec_mono (sin_spec x) (fun r h => h ▸ IEEE754.Dmath.sin_err hx hb)

/-- **Contract C1 for the shipped `dmath::cos`**. -/
theorem cos_err (x : F64) (hx : x.isFinite) (hb : |x.val| ≤ 1048576) :
    dmath.cos x ⦃ r => r.isFinite ∧ |(r.val : ℝ) - Real.cos x.val| ≤ 1 / 2 ^ 52 ⦄ :=
  WP.spec_mono (cos_spec x) (fun r h => h ▸ IEEE754.Dmath.cos_err hx hb)

/-- **Contract C1 for the shipped `dmath::sin_cos`**. -/
theorem sin_cos_err (x : F64) (hx : x.isFinite) (hb : |x.val| ≤ 1048576) :
    dmath.sin_cos x ⦃ r => r.1.isFinite ∧ r.2.isFinite ∧ |(r.1.val : ℝ) - Real.sin x.val| ≤ 1 / 2 ^ 52 ∧
      |(r.2.val : ℝ) - Real.cos x.val| ≤ 1 / 2 ^ 52 ⦄ :=
  WP.spec_mono (sin_cos_spec x) (fun r h => h ▸ IEEE754.Dmath.sinCos_err hx hb)

end ForgeMath.Dmath
