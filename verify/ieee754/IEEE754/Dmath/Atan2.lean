/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.AtanErr

/-!
# `dmath::atan2`: the contract (C2)

`atan2_err50`: for finite `y`, `x`, not both zero (and not `y = −0` with `x < 0`, where the Rust
returns `−π`), `dmath::atan2(y, x)` is finite and within `2^-50` of `atan2R y x`, the angle of
`(x, y)` in `(−π, π]`; `atan2_err` is sharper (`3.56·10^-16` for `x > 0`, `6.9·10^-16` otherwise).
-/

namespace IEEE754.Dmath

open FP Real

def THREE : F64 := .fin false 6755399441055744 (-51) (by decide)
theorem THREE_bits : THREE.toBits = 0x4008000000000000 := by decide
def FOUR : F64 := .fin false 4503599627370496 (-50) (by decide)
theorem FOUR_bits : FOUR.toBits = 0x4010000000000000 := by decide

/-- `dmath::atan2`, branch for branch. `k` is the difference of the raw exponent fields. For a
nonzero, non-NaN operand `v < 0.0` is its sign bit. -/
def atan2 (y x : F64) : F64 :=
  if x.isNaN || y.isNaN then .nan
  else if y.isZero then (if x.sign then (if y.sign then PI.neg else PI) else y)
  else if x.isZero then (if y.sign then FRAC_PI_2.neg else FRAC_PI_2)
  else if x.isInf then
    let q := if y.isInf then (if x.sign then div (mul THREE PI) FOUR else div PI FOUR)
      else if x.sign then PI else .zero false
    if y.sign then q.neg else q
  else if y.isInf then (if y.sign then FRAC_PI_2.neg else FRAC_PI_2)
  else
    let k : ℤ := (expo y : ℤ) - expo x
    let z := if 60 < k then add FRAC_PI_2 (mul HALF PI_LO)
      else if x.sign = true ∧ k < -60 then .zero false
      else atan (div y x).abs
    if x.sign then (if y.sign then sub (sub z PI_LO) PI else sub PI (sub z PI_LO))
    else (if y.sign then z.neg else z)

/-- The angle of `(x, y)`, in `(−π, π]` (`0` at the origin). -/
noncomputable def atan2R (y x : ℝ) : ℝ :=
  if 0 < x then arctan (y / x)
  else if x < 0 then (if 0 ≤ y then arctan (y / x) + π else arctan (y / x) - π)
  else if 0 < y then π / 2 else if y < 0 then -(π / 2) else 0

lemma abs_ge_min {x : F64} (hx : x.isFinite) (h0 : x.val ≠ 0) : 2 ^ (-1074 : ℤ) ≤ |x.val| := by
  obtain ⟨k, e, _, he, hv, ha⟩ := FP.val_repr hx
  simp only [binary64_qmin] at he
  have hk : k ≠ 0 := by rintro rfl; simp at hv; exact h0 hv
  have hk1 : (1 : ℚ) ≤ |(k : ℚ)| := by
    have : (1 : ℤ) ≤ |k| := Int.one_le_abs hk
    exact_mod_cast this
  rw [ha, Int.cast_abs]
  calc (2 : ℚ) ^ (-1074 : ℤ) ≤ 2 ^ e := two_zpow_le he
    _ = 1 * 2 ^ e := (_root_.one_mul _).symm
    _ ≤ |(k : ℚ)| * 2 ^ e := mul_le_mul_of_nonneg_right hk1 (two_zpow_pos _).le

/-- With `k ≤ 60`, `|y/x| ≤ 2^200`. -/
lemma quot_le {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hx0 : x.val ≠ 0)
    (hk : (expo y : ℤ) - expo x ≤ 60) : |y.val / x.val| ≤ 2 ^ (200 : ℤ) := by
  have hxa : 0 < |x.val| := abs_pos.mpr hx0
  rw [abs_div, div_le_iff₀ hxa]
  have hy' := abs_lt_expo hy
  by_cases h1 : 1 ≤ expo x
  · have hx' := expo_le_abs hx h1
    calc |y.val| ≤ 2 ^ ((expo y : ℤ) - 1022) := hy'.le
      _ ≤ 2 ^ ((200 : ℤ) + ((expo x : ℤ) - 1023)) := two_zpow_le (by omega)
      _ = 2 ^ (200 : ℤ) * 2 ^ ((expo x : ℤ) - 1023) := two_zpow_add _ _
      _ ≤ 2 ^ (200 : ℤ) * |x.val| := mul_le_mul_of_nonneg_left hx' (two_zpow_pos _).le
  · have h0 : expo x = 0 := by omega
    have hm := abs_ge_min hx hx0
    calc |y.val| ≤ 2 ^ ((expo y : ℤ) - 1022) := hy'.le
      _ ≤ 2 ^ ((200 : ℤ) + (-1074)) := two_zpow_le (by omega)
      _ = 2 ^ (200 : ℤ) * 2 ^ (-1074 : ℤ) := two_zpow_add _ _
      _ ≤ 2 ^ (200 : ℤ) * |x.val| := mul_le_mul_of_nonneg_left hm (two_zpow_pos _).le

/-- A relative perturbation `c` of `u ≥ 0` moves `arctan` by at most `c/(2(1−c))`
(`u/(1 + u²) ≤ 1/2`). -/
lemma arctan_rel_gen {d u c : ℝ} (hu : 0 ≤ u) (hc0 : 0 ≤ c) (hc1 : c ≤ 1 / 2) (hd : |d - u| ≤ u * c) :
    |arctan d - arctan u| ≤ c / (2 * (1 - c)) := by
  have hdl : u * (1 - c) ≤ d := by
    have := (abs_le.mp hd).1; nlinarith
  have hm0 : 0 ≤ u * (1 - c) := mul_nonneg hu (by linarith)
  have h := arctan_lip_m (b := u) hm0 hdl (by nlinarith)
  have h2 : |d - u| / (1 + (u * (1 - c)) ^ 2) ≤ u * c / (1 + (u * (1 - c)) ^ 2) :=
    div_le_div_of_nonneg_right hd (by positivity)
  have h3 : u * c / (1 + (u * (1 - c)) ^ 2) ≤ c / (2 * (1 - c)) := by
    rw [div_le_div_iff₀ (by positivity) (by nlinarith)]
    nlinarith [sq_nonneg (u * (1 - c) - 1), mul_nonneg hc0 (sq_nonneg (u * (1 - c) - 1))]
  exact le_trans h (le_trans h2 h3)

set_option linter.constructorNameAsVariable false in
/-- `|arctan d − arctan u|` for `d` within `2^-53·u + η` of `u ≥ 0` (`η ≤ 2^-200`). -/
lemma arctan_rel {d u η : ℝ} (hu : 0 ≤ u) (hη0 : 0 ≤ η) (hη : η ≤ 1 / 2 ^ 200)
    (hd : |d - u| ≤ u / 2 ^ 53 + η) : |arctan d - arctan u| ≤ 1 / 2 ^ 54 + 1 / 2 ^ 100 := by
  -- the constants, opaque to `linarith`
  have k1 : (1 : ℝ) / 2 ^ 200 = (1 / 2 ^ 100) * (1 / 2 ^ 100) := by norm_num
  have k2 : (1 : ℝ) / 2 ^ 100 * (1 / 2 ^ 53) + 1 / 2 ^ 200 ≤ 1 / 2 ^ 54 + 1 / 2 ^ 100 := by norm_num
  have k3 : (1 / 2 ^ 53 + 1 / 2 ^ 100 : ℝ) / (2 * (1 - (1 / 2 ^ 53 + 1 / 2 ^ 100))) ≤ 1 / 2 ^ 54 + 1 / 2 ^ 100 := by
    rw [div_le_iff₀ (by norm_num)]; norm_num
  have k4 : (1 / 2 ^ 53 + 1 / 2 ^ 100 : ℝ) ≤ 1 / 2 := by norm_num
  have k5 : (0 : ℝ) < 1 / 2 ^ 100 := by positivity
  have k6 : (0 : ℝ) < 1 / 2 ^ 53 := by positivity
  rw [k1] at hη
  rw [div_eq_mul_one_div u (2 ^ 53)] at hd
  generalize (1 : ℝ) / 2 ^ 100 = e at *
  generalize (1 : ℝ) / 2 ^ 53 = f at *
  generalize (1 : ℝ) / 2 ^ 54 = g at *
  by_cases hs : u < e
  · have h1 := arctan_lip d u
    have h2 : u * f ≤ e * f := mul_le_mul_of_nonneg_right hs.le k6.le
    have h3 : e * f = e * f := rfl
    nlinarith
  rw [not_lt] at hs
  have hηu : η ≤ u * e := le_trans hη (mul_le_mul_of_nonneg_right hs k5.le)
  have hdc : |d - u| ≤ u * (f + e) := by nlinarith
  exact le_trans (arctan_rel_gen hu (by linarith) k4 hdc) k3

/-! ## `z ≈ arctan |y/x|` on each branch -/

lemma zA {u : ℝ} (hu : 2 ^ 60 ≤ u) :
    (add FRAC_PI_2 (mul HALF PI_LO)).isFinite ∧
      |((add FRAC_PI_2 (mul HALF PI_LO)).val : ℝ) - arctan u| ≤ 356 / 10 ^ 18 := by
  have fH : HALF.isFinite := by decide
  have fL : PI_LO.isFinite := by decide
  have fF : FRAC_PI_2.isFinite := by decide
  obtain ⟨fh, eh⟩ := mul_ulp fH fL (k := -53) (by rw [HALF_val, PI_LO_val]; norm_num) (by norm_num) (by norm_num)
  rw [HALF_val, PI_LO_val] at eh
  have eh2 : |(mul HALF PI_LO).val - 1 / 2 * (4967757600021511 / 2 ^ 105)| ≤ 1 / 10 ^ 30 :=
    le_trans eh (le_trans (two_zpow_le (show (-53 - 54 : ℤ) ≤ -100 by norm_num)) (by norm_num))
  clear eh
  have hh := abs_le.mp eh2
  obtain ⟨fz, ez⟩ := add_ulp fF fh (k := 1) (by rw [FRAC_PI_2_val, abs_lt]; constructor <;> norm_num <;> linarith)
    (by norm_num) (by norm_num)
  rw [show (2 : ℚ) ^ (1 - 54 : ℤ) = 1 / 2 ^ 53 by norm_num] at ez
  refine ⟨fz, ?_⟩
  -- z against (PI + PI_LO)/2
  have e1 : |(add FRAC_PI_2 (mul HALF PI_LO)).val - (PI.val + PI_LO.val) / 2| ≤ 1 / 2 ^ 53 + 1 / 10 ^ 30 := by
    have hF : FRAC_PI_2.val = PI.val / 2 := by rw [FRAC_PI_2_val, PI_val]; norm_num
    rw [hF] at ez
    rw [PI_LO_val]
    have := abs_le.mp ez
    rw [abs_le]; constructor <;> linarith
  have e1r := qcast e1
  have hc := c_pi
  push_cast at e1r hc
  have hu0 : (0 : ℝ) < u := lt_of_lt_of_le (by positivity) hu
  have h3 := red3 hu0
  have hb := arctan_abs_le (-1 / u)
  have hia : |-1 / u| ≤ 1 / 10 ^ 18 := by
    rw [abs_div, abs_neg, abs_one, abs_of_pos hu0]
    exact le_trans (one_div_le_one_div_of_le (by positivity) hu) (by norm_num)
  rw [h3]
  clear hh
  have := abs_le.mp e1r; have := abs_le.mp hc; have := abs_le.mp hb; have := abs_le.mp hia
  rw [abs_le]; constructor <;> linarith

lemma zB {u : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 2 ^ 60) :
    (FP.zero false : F64).isFinite ∧ |((FP.zero false : F64).val : ℝ) - arctan u| ≤ 356 / 10 ^ 18 := by
  refine ⟨rfl, ?_⟩
  have := arctan_abs_le u
  simp only [FP.val, Rat.cast_zero, zero_sub, abs_neg]
  rw [abs_of_nonneg hu0] at this
  have n : (1 : ℝ) / 2 ^ 60 ≤ 356 / 10 ^ 18 := by norm_num
  linarith

lemma zC {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hx0 : x.val ≠ 0) (hq : |y.val / x.val| ≤ 2 ^ (200 : ℤ)) :
    (atan (div y x).abs).isFinite ∧
      |((atan (div y x).abs).val : ℝ) - arctan (|(y.val : ℝ) / x.val|)| ≤ 356 / 10 ^ 18 := by
  have hdr := div_eq_round hy hx hx0
  have fd : (div y x).isFinite := by
    rw [hdr]; exact round_finite_of_le_emax (le_trans hq (two_zpow_le (by simp)))
  have ed := round_err (K := ℚ) binary64 (zs := y.sign ^^ x.sign) (r := y.val / x.val) (by rw [← hdr]; exact fd)
  rw [← hdr] at ed
  simp only [binary64_prec, binary64_qmin] at ed
  rw [show (-1074 - 1 : ℤ) = -1075 by norm_num, show (2 : ℚ) ^ (-((53 : ℕ) : ℤ)) = 1 / 2 ^ 53 by norm_num] at ed
  have hη1 : (2 : ℚ) ^ (-1075 : ℤ) ≤ 1 / 2 ^ 200 :=
    le_trans (two_zpow_le (show (-1075 : ℤ) ≤ -200 by norm_num)) (by norm_num)
  have hη0 : (0 : ℚ) < 2 ^ (-1075 : ℤ) := two_zpow_pos _
  generalize (2 : ℚ) ^ (-1075 : ℤ) = η at ed hη1 hη0
  set Q := y.val / x.val
  have ed' : |(div y x).val - Q| ≤ |Q| / 2 ^ 53 + η := by
    refine le_trans ed (max_le ?_ ?_)
    · have : 1 / 2 ^ 53 * |Q| = |Q| / 2 ^ 53 := by ring
      linarith
    · have : (0 : ℚ) ≤ |Q| / 2 ^ 53 := by positivity
      linarith
  have fa : (div y x).abs.isFinite := by simpa using fd
  have va : (div y x).abs.val = |(div y x).val| := abs_val _
  have e2 : |(div y x).abs.val - (|Q|)| ≤ |Q| / 2 ^ 53 + η := by
    rw [va]; exact le_trans (abs_abs_sub_abs_le_abs_sub _ _) ed'
  have e2r := qcast e2
  push_cast at e2r
  have hηr : (η : ℝ) ≤ 1 / 2 ^ 200 := by
    have := (Rat.cast_le (K := ℝ)).mpr hη1; push_cast at this; exact this
  have hQ : ((Q : ℚ) : ℝ) = (y.val : ℝ) / x.val := by simp only [Q]; push_cast; rfl
  rw [hQ] at e2r
  have hrel := arctan_rel (d := (((div y x).abs.val : ℚ) : ℝ)) (u := |(y.val : ℝ) / x.val|)
    (η := (η : ℝ)) (abs_nonneg _) (by exact_mod_cast hη0.le) hηr e2r
  obtain ⟨fz, ez⟩ := atan_err fa
  refine ⟨fz, ?_⟩
  have := abs_sub_le ((atan (div y x).abs).val : ℝ) (arctan ((div y x).abs.val : ℝ)) (arctan (|(y.val : ℝ) / x.val|))
  have n : (3 : ℝ) / 10 ^ 16 + (1 / 2 ^ 54 + 1 / 2 ^ 100) ≤ 356 / 10 ^ 18 := by norm_num
  linarith

/-! ## The quadrants -/

lemma qcast_le {a c : ℚ} (h : (a : ℝ) ≤ (c : ℝ)) : a ≤ c := by exact_mod_cast h

lemma quad {x y z : F64} (hx : x.isFinite) (hy : y.isFinite) (fz : z.isFinite) (hx0 : x.val ≠ 0)
    (hy0 : y.val ≠ 0) (hz : |(z.val : ℝ) - arctan (|(y.val : ℝ) / x.val|)| ≤ 356 / 10 ^ 18) :
    (if x.sign then (if y.sign then sub (sub z PI_LO) PI else sub PI (sub z PI_LO))
      else (if y.sign then z.neg else z)).isFinite ∧
    |(((if x.sign then (if y.sign then sub (sub z PI_LO) PI else sub PI (sub z PI_LO))
      else (if y.sign then z.neg else z)).val : ℚ) : ℝ) - atan2R y.val x.val| ≤ (if (0 : ℝ) < x.val then 356 / 10 ^ 18 else 69 / 10 ^ 17) := by
  set u := |(y.val : ℝ) / x.val|
  have hu0 : 0 ≤ u := abs_nonneg _
  have ha0 : 0 ≤ arctan u := arctan_nonneg.mpr hu0
  have ha1 := arctan_lt_pi_div_two u
  have hpi := Real.pi_lt_d4
  have hzl := abs_le.mp hz
  have fL : PI_LO.isFinite := by decide
  have fP : PI.isFinite := by decide
  -- the range of z
  have hzr1 : ((z.val : ℚ) : ℝ) ≤ ((15709 / 10000 : ℚ) : ℝ) := by push_cast; linarith
  have hzr2 : ((-1 / 10 ^ 15 : ℚ) : ℝ) ≤ ((z.val : ℚ) : ℝ) := by push_cast; linarith
  have hz1 := qcast_le hzr1
  have hz2 := qcast_le hzr2
  -- w = z − PI_LO and the two closing subtractions
  have hw : |z.val - PI_LO.val| < 2 ^ (1 : ℤ) := by
    rw [PI_LO_val, abs_lt]; constructor <;> norm_num <;> linarith
  obtain ⟨fw, ew⟩ := sub_ulp fz fL hw (by norm_num) (by norm_num)
  rw [show (2 : ℚ) ^ (1 - 54 : ℤ) = 1 / 2 ^ 53 by norm_num] at ew
  have hwl := abs_le.mp ew
  have hr1 : |PI.val - (sub z PI_LO).val| < 2 ^ (2 : ℤ) := by
    rw [PI_val, abs_lt]; rw [PI_LO_val] at hwl; constructor <;> norm_num <;> linarith
  have hr2 : |(sub z PI_LO).val - PI.val| < 2 ^ (2 : ℤ) := by rw [abs_sub_comm]; exact hr1
  obtain ⟨fr1, er1⟩ := sub_ulp fP fw hr1 (by norm_num) (by norm_num)
  obtain ⟨fr2, er2⟩ := sub_ulp fw fP hr2 (by norm_num) (by norm_num)
  rw [show (2 : ℚ) ^ (2 - 54 : ℤ) = 1 / 2 ^ 52 by norm_num] at er1 er2
  have ewr := qcast ew
  have er1r := qcast er1
  have er2r := qcast er2
  have hc := c_pi
  push_cast at ewr er1r er2r hc
  have hcl := abs_le.mp hc
  have hwlr := abs_le.mp ewr
  have n : (356 : ℝ) / 10 ^ 18 + 1 / 2 ^ 53 + 1 / 2 ^ 52 + 2 / 10 ^ 20 ≤ 69 / 10 ^ 17 := by norm_num
  have n0 : (356 : ℝ) / 10 ^ 18 ≤ 1 / 2 ^ 50 := by norm_num
  obtain ⟨sxn, sxp⟩ := @sign_val x
  obtain ⟨syn, syp⟩ := @sign_val y
  have hX : (x.val : ℝ) ≠ 0 := by exact_mod_cast hx0
  have hY : (y.val : ℝ) ≠ 0 := by exact_mod_cast hy0
  cases hxs : x.sign <;> cases hys : y.sign
  · -- x > 0, y > 0
    have ex := sxp hxs; have ey := syp hys
    have hxp : (0 : ℝ) < x.val := by
      have : (0 : ℚ) < x.val := by rw [ex]; exact abs_pos.mpr hx0
      exact_mod_cast this
    have hyp : (0 : ℝ) < y.val := by
      have : (0 : ℚ) < y.val := by rw [ey]; exact abs_pos.mpr hy0
      exact_mod_cast this
    simp only [Bool.false_eq_true, if_false]
    refine ⟨fz, ?_⟩
    rw [if_pos hxp]
    unfold atan2R; rw [if_pos hxp]
    have : (y.val : ℝ) / x.val = u := (abs_of_pos (div_pos hyp hxp)).symm
    rw [this]; exact hz
  · -- x > 0, y < 0
    have ex := sxp hxs; have ey := syn hys
    have hxp : (0 : ℝ) < x.val := by
      have : (0 : ℚ) < x.val := by rw [ex]; exact abs_pos.mpr hx0
      exact_mod_cast this
    have hyn : (y.val : ℝ) < 0 := by
      have : y.val < (0 : ℚ) := by rw [ey]; exact neg_neg_of_pos (abs_pos.mpr hy0)
      exact_mod_cast this
    simp only [Bool.false_eq_true, if_false, if_true]
    refine ⟨by simpa using fz, ?_⟩
    rw [if_pos hxp]
    unfold atan2R; rw [if_pos hxp]
    have : (y.val : ℝ) / x.val = -u := by
      simp only [u]; rw [abs_of_neg (div_neg_of_neg_of_pos hyn hxp)]; ring
    rw [this, arctan_neg, FP.val_neg]; push_cast
    rw [show -(z.val : ℝ) - -arctan u = -((z.val : ℝ) - arctan u) by ring, abs_neg]
    exact hz
  · -- x < 0, y > 0
    have ex := sxn hxs; have ey := syp hys
    have hxn : (x.val : ℝ) < 0 := by
      have : x.val < (0 : ℚ) := by rw [ex]; exact neg_neg_of_pos (abs_pos.mpr hx0)
      exact_mod_cast this
    have hyp : (0 : ℝ) < y.val := by
      have : (0 : ℚ) < y.val := by rw [ey]; exact abs_pos.mpr hy0
      exact_mod_cast this
    simp only [Bool.false_eq_true, if_false, if_true]
    refine ⟨fr1, ?_⟩
    rw [if_neg (not_lt.mpr hxn.le)]
    unfold atan2R; rw [if_neg (not_lt.mpr hxn.le), if_pos hxn, if_pos hyp.le]
    have : (y.val : ℝ) / x.val = -u := by
      simp only [u]; rw [abs_of_neg (div_neg_of_pos_of_neg hyp hxn)]; ring
    rw [this, arctan_neg]
    have := abs_le.mp er1r
    rw [abs_le]; constructor <;> linarith
  · -- x < 0, y < 0
    have ex := sxn hxs; have ey := syn hys
    have hxn : (x.val : ℝ) < 0 := by
      have : x.val < (0 : ℚ) := by rw [ex]; exact neg_neg_of_pos (abs_pos.mpr hx0)
      exact_mod_cast this
    have hyn : (y.val : ℝ) < 0 := by
      have : y.val < (0 : ℚ) := by rw [ey]; exact neg_neg_of_pos (abs_pos.mpr hy0)
      exact_mod_cast this
    simp only [if_true]
    refine ⟨fr2, ?_⟩
    rw [if_neg (not_lt.mpr hxn.le)]
    unfold atan2R; rw [if_neg (not_lt.mpr hxn.le), if_pos hxn, if_neg (not_le.mpr hyn)]
    have : (y.val : ℝ) / x.val = u := (abs_of_pos (div_pos_of_neg_of_neg hyn hxn)).symm
    rw [this]
    have := abs_le.mp er2r
    rw [abs_le]; constructor <;> linarith

lemma val_zero_of_isZero {x : F64} (h : x.isZero = true) : x.val = 0 := by
  cases x <;> simp_all [FP.isZero, FP.val]

lemma val_ne_zero {x : F64} (hx : x.isFinite) (h : x.isZero = false) : x.val ≠ 0 := by
  cases x with
  | fin s m e hc =>
    have : (0 : ℚ) < m := by exact_mod_cast hc.1
    simp only [FP.val]
    exact mul_ne_zero (by cases s <;> simp [sgn]) (mul_ne_zero this.ne' (two_zpow_pos _).ne')
  | _ => simp_all [FP.isFinite, FP.isZero]

/-- **`dmath::atan2`.** For finite `y`, `x`, not both zero, and not `y = −0` with `x < 0`,
`atan2(y, x)` is finite and within `3.56·10^-16` of `atan2R y x` when `x > 0`, `6.9·10^-16`
otherwise. -/
theorem atan2_err {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hne : x.val ≠ 0 ∨ y.val ≠ 0)
    (hsz : y.val = 0 → y.sign = false ∨ x.sign = false) :
    (atan2 y x).isFinite ∧ |((atan2 y x).val : ℝ) - atan2R y.val x.val| ≤ (if (0 : ℝ) < x.val then 356 / 10 ^ 18 else 69 / 10 ^ 17) := by
  have hxn : x.isNaN = false := by cases x <;> simp_all [FP.isFinite, FP.isNaN]
  have hyn : y.isNaN = false := by cases y <;> simp_all [FP.isFinite, FP.isNaN]
  have hxi : x.isInf = false := by cases x <;> simp_all [FP.isFinite, FP.isInf]
  have hyi : y.isInf = false := by cases y <;> simp_all [FP.isFinite, FP.isInf]
  obtain ⟨sxn, sxp⟩ := @sign_val x
  obtain ⟨syn, syp⟩ := @sign_val y
  unfold atan2
  rw [if_neg (by simp [hxn, hyn])]
  cases hyz : y.isZero
  · -- y ≠ 0
    rw [if_neg (by simp)]
    have hy0 := val_ne_zero hy hyz
    cases hxz : x.isZero
    · rw [if_neg (by simp), if_neg (by simp [hxi]), if_neg (by simp [hyi])]
      have hx0 := val_ne_zero hx hxz
      dsimp only
      have big : 60 < (expo y : ℤ) - expo x → (2 : ℝ) ^ 60 ≤ |(y.val : ℝ) / x.val| := by
        intro h60
        have hg := expo_gap hy hx (k := 60) (by norm_num) h60
        have hxa : 0 < |x.val| := abs_pos.mpr hx0
        have : (2 : ℚ) ^ (60 : ℕ) ≤ |y.val / x.val| := by
          rw [abs_div, le_div_iff₀ hxa]
          have e : (2 : ℚ) ^ (60 : ℕ) * 2 ^ (-60 : ℤ) = 1 := by norm_num
          calc (2 : ℚ) ^ (60 : ℕ) * |x.val| ≤ 2 ^ (60 : ℕ) * (2 ^ (-60 : ℤ) * |y.val|) :=
                mul_le_mul_of_nonneg_left hg.le (by positivity)
            _ = |y.val| := by rw [← mul_assoc, e, _root_.one_mul]
        have := (Rat.cast_le (K := ℝ)).mpr this; rw [Rat.cast_abs] at this; push_cast at this; exact this
      have small : 60 < (expo x : ℤ) - expo y → |(y.val : ℝ) / x.val| ≤ 1 / 2 ^ 60 := by
        intro h60
        have hg := expo_gap hx hy (k := 60) (by norm_num) h60
        have hxa : 0 < |x.val| := abs_pos.mpr hx0
        have : |y.val / x.val| ≤ 1 / 2 ^ (60 : ℕ) := by
          rw [abs_div, div_le_iff₀ hxa]
          have : (2 : ℚ) ^ (-60 : ℤ) = 1 / 2 ^ (60 : ℕ) := by norm_num
          rw [this] at hg; linarith
        have := (Rat.cast_le (K := ℝ)).mpr this; rw [Rat.cast_abs] at this; push_cast at this; exact this
      refine quad hx hy ?_ hx0 hy0 ?_ <;> split_ifs with h60 h60'
      · exact (zA (big h60)).1
      · rfl
      · exact (zC hx hy hx0 (quot_le hx hy hx0 (by omega))).1
      · exact (zA (big h60)).2
      · exact (zB (abs_nonneg _) (small (by omega))).2
      · exact (zC hx hy hx0 (quot_le hx hy hx0 (by omega))).2
    · -- x = 0, y ≠ 0
      rw [if_pos rfl]
      have hx0 := val_zero_of_isZero hxz
      have hF := c_FRAC_PI_2
      have fF : FRAC_PI_2.isFinite := by decide
      have n : (62 : ℝ) / 10 ^ 18 ≤ 69 / 10 ^ 17 := by norm_num
      have hb : (if (0 : ℝ) < x.val then (356 : ℝ) / 10 ^ 18 else 69 / 10 ^ 17) = 69 / 10 ^ 17 := by
        rw [if_neg]; rw [hx0]; simp
      rw [hb]
      unfold atan2R
      rw [hx0, Rat.cast_zero, if_neg (lt_irrefl 0), if_neg (lt_irrefl 0)]
      cases hys : y.sign
      · have ey := syp hys
        have hyp : (0 : ℝ) < y.val := by
          have : (0 : ℚ) < y.val := by rw [ey]; exact abs_pos.mpr hy0
          exact_mod_cast this
        simp only [Bool.false_eq_true, if_false]
        rw [if_pos hyp]; exact ⟨fF, le_trans hF n⟩
      · have ey := syn hys
        have hyn' : (y.val : ℝ) < 0 := by
          have : y.val < (0 : ℚ) := by rw [ey]; exact neg_neg_of_pos (abs_pos.mpr hy0)
          exact_mod_cast this
        simp only [if_true]
        rw [if_neg (not_lt.mpr hyn'.le), if_pos hyn']
        refine ⟨by simpa using fF, ?_⟩
        have := neg_err hF
        rw [show -(π / 2) = -(π / 2) from rfl]; exact le_trans this n
  · -- y = 0
    rw [if_pos rfl]
    have hy0 := val_zero_of_isZero hyz
    have hx0 : x.val ≠ 0 := by rcases hne with h | h; exact h; exact absurd hy0 h
    unfold atan2R
    rw [hy0, Rat.cast_zero, zero_div, arctan_zero]
    cases hxs : x.sign
    · have ex := sxp hxs
      have hxp : (0 : ℝ) < x.val := by
        have : (0 : ℚ) < x.val := by rw [ex]; exact abs_pos.mpr hx0
        exact_mod_cast this
      simp only [Bool.false_eq_true, if_false]
      rw [if_pos hxp, hy0]; refine ⟨hy, ?_⟩; simp only [Rat.cast_zero, sub_zero, abs_zero]
      split_ifs <;> norm_num
    · have ex := sxn hxs
      have hxn' : (x.val : ℝ) < 0 := by
        have : x.val < (0 : ℚ) := by rw [ex]; exact neg_neg_of_pos (abs_pos.mpr hx0)
        exact_mod_cast this
      have hys : y.sign = false := by
        rcases hsz hy0 with h | h; exact h; rw [hxs] at h; exact absurd h (by decide)
      simp only [if_true, hys, Bool.false_eq_true, if_false]
      rw [if_neg (not_lt.mpr hxn'.le), if_pos hxn', if_pos le_rfl, zero_add]
      rw [if_neg (not_lt.mpr hxn'.le)]
      refine ⟨by decide, le_trans c_PI (by norm_num)⟩

/-- **Contract C2**, as stated: within `2^-50`. -/
theorem atan2_err50 {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hne : x.val ≠ 0 ∨ y.val ≠ 0)
    (hsz : y.val = 0 → y.sign = false ∨ x.sign = false) :
    (atan2 y x).isFinite ∧ |((atan2 y x).val : ℝ) - atan2R y.val x.val| ≤ 1 / 2 ^ 50 := by
  obtain ⟨f, e⟩ := atan2_err hx hy hne hsz
  refine ⟨f, le_trans e ?_⟩
  split_ifs <;> norm_num

end IEEE754.Dmath
