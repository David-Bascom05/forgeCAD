/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.AtanEval
import IEEE754.Dmath.AtanFacts
import IEEE754.Dmath.SinCos

/-!
# `dmath::atan`: the contract

`atan_err`: for finite `x`, `dmath::atan(x)` is finite and within `3·10^-16` of `arctan x`.
-/

namespace IEEE754.Dmath

open FP Real

/-- The common tail of `atan`'s reduced branches: `hi − (((t·(s1+s2)) − lo) − t)`. If the reduced
`t` is within `δ` of `te` and `arctan a = c + arctan te` with `hi + lo ≈ c`, the result is within
`2^(k-54) + δ + 6·10^-17` of `arctan a` (`k` bounds the result's binade). -/
lemma reduced_err {hi lo t : F64} (fhi : hi.isFinite) (flo : lo.isFinite) (ft : t.isFinite)
    {a te c δ : ℝ} {k : ℤ} (hk : -1 ≤ k) (hk2 : k ≤ 1)
    (htb : |t.val| ≤ 4375 / 10000) (hte : |(t.val : ℝ) - te| ≤ δ) (hlo : |lo.val| ≤ 1 / 10 ^ 16)
    (hc : |((hi.val + lo.val : ℚ) : ℝ) - c| ≤ 1 / 10 ^ 20) (hred : arctan a = c + arctan te)
    (hroom : |arctan a| + (δ + 1 / 10 ^ 16) < 2 ^ k) :
    (sub hi (sub (sub (atanPoly t) lo) t)).isFinite ∧
      |((sub hi (sub (sub (atanPoly t) lo) t)).val : ℝ) - arctan a| ≤ 2 ^ (k - 54) + δ + 6 / 10 ^ 17 := by
  obtain ⟨fp, ep, mp⟩ := atanPoly_eval ft htb
  set PT : ℚ := t.val - t.val * evalP katanL (t.val ^ 2)
  -- q1 = p − lo
  have h1 : |(atanPoly t).val - lo.val| < 2 ^ (-4 : ℤ) := by
    have := abs_sub (atanPoly t).val lo.val
    have := abs_sub_abs_le_abs_sub (atanPoly t).val PT
    rw [show (2 : ℚ) ^ (-4 : ℤ) = 1 / 16 by norm_num]; linarith
  obtain ⟨fq1, eq1⟩ := sub_ulp fp flo h1 (by norm_num) (by norm_num)
  set q1 := sub (atanPoly t) lo
  -- q2 = q1 − t
  have h2 : |q1.val - t.val| < 2 ^ (-1 : ℤ) := by
    have := abs_sub q1.val t.val
    have := abs_sub_abs_le_abs_sub q1.val ((atanPoly t).val - lo.val)
    have := abs_sub (atanPoly t).val lo.val
    have := abs_sub_abs_le_abs_sub (atanPoly t).val PT
    rw [show (2 : ℚ) ^ (-1 : ℤ) = 1 / 2 by norm_num] at *; norm_num at eq1; linarith
  obtain ⟨fq2, eq2⟩ := sub_ulp fq1 ft h2 (by norm_num) (by norm_num)
  set q2 := sub q1 t
  -- the error of q2 against PT − lo − t
  have eq2' : |q2.val - (PT - lo.val - t.val)| ≤ 51 / 10 ^ 18 := by
    have e1 : (2 : ℚ) ^ (-1 - 54 : ℤ) = 1 / 2 ^ 55 := by norm_num
    have e2 : (2 : ℚ) ^ (-4 - 54 : ℤ) = 1 / 2 ^ 58 := by norm_num
    rw [e1] at eq2; rw [e2] at eq1
    have hsplit : q2.val - (PT - lo.val - t.val) = (q2.val - (q1.val - t.val)) +
        ((q1.val - ((atanPoly t).val - lo.val)) + ((atanPoly t).val - PT)) := by ring
    rw [hsplit]
    have a1 := abs_add_le (q2.val - (q1.val - t.val))
      ((q1.val - ((atanPoly t).val - lo.val)) + ((atanPoly t).val - PT))
    have a2 := abs_add_le (q1.val - ((atanPoly t).val - lo.val)) ((atanPoly t).val - PT)
    have n : (1 : ℚ) / 2 ^ 55 + 1 / 2 ^ 58 + 47 / 2500000000000000000 ≤ 51 / 10 ^ 18 := by norm_num
    linarith
  -- in ℝ: hi − q2 against arctan a
  have hp := katan_poly htb
  have hl := arctan_lip (t.val : ℝ) te
  have eq2r := qcast eq2'
  push_cast at eq2r hc
  have hcore : |((hi.val : ℝ) - q2.val) - arctan a| ≤ δ + 6 / 10 ^ 17 := by
    rw [hred]
    have e : ((hi.val : ℝ) - q2.val) - (c + arctan te) =
        -((q2.val : ℝ) - (PT - lo.val - t.val)) + (((hi.val : ℝ) + lo.val) - c) +
          ((t.val : ℝ) * ((evalP katanL (t.val ^ 2) : ℚ) : ℝ) - arctan t.val) + (arctan t.val - arctan te) := by
      simp only [PT]; push_cast; ring
    rw [e]
    set X1 := -((q2.val : ℝ) - (PT - lo.val - t.val))
    set X2 := ((hi.val : ℝ) + lo.val) - c
    set X3 := (t.val : ℝ) * ((evalP katanL (t.val ^ 2) : ℚ) : ℝ) - arctan t.val
    set X4 := arctan (t.val : ℝ) - arctan te
    have b1 : |X1| ≤ 51 / 10 ^ 18 := by simp only [X1]; rw [abs_neg]; exact eq2r
    have b3 : |X3| ≤ 1 / 2 ^ 57 + 1 / 10 ^ 19 := hp
    have b4 : |X4| ≤ δ := le_trans hl hte
    have a1 := abs_add_le (X1 + X2 + X3) X4
    have a2 := abs_add_le (X1 + X2) X3
    have a3 := abs_add_le X1 X2
    have n : (51 : ℝ) / 10 ^ 18 + 1 / 10 ^ 20 + (1 / 2 ^ 57 + 1 / 10 ^ 19) ≤ 6 / 10 ^ 17 := by norm_num
    linarith
  have h3r : |((hi.val : ℝ) - q2.val)| < 2 ^ k := by
    have := abs_sub_abs_le_abs_sub ((hi.val : ℝ) - q2.val) (arctan a)
    linarith
  have h3 : |hi.val - q2.val| < 2 ^ k := by
    have : ((|hi.val - q2.val| : ℚ) : ℝ) < ((2 ^ k : ℚ) : ℝ) := by push_cast; exact h3r
    exact_mod_cast this
  obtain ⟨fr, er⟩ := sub_ulp fhi fq2 h3 (by omega)
    (le_trans (two_zpow_le (show k ≤ 1 by omega)) (by norm_num))
  refine ⟨fr, ?_⟩
  have err := qcast er
  push_cast at err
  have := abs_sub_le ((sub hi q2).val : ℝ) ((hi.val : ℝ) - q2.val) (arctan a)
  linarith

/-! ## The reductions -/

lemma rep_two_mul {x : F64} (hx : x.isFinite) (hb : |x.val| ≤ 4) : Rep binary64 (2 * x.val) := by
  obtain ⟨e, he, ⟨n, hn⟩, hlt⟩ := mult_val hx
  simp only [binary64_qmin, binary64_prec] at he hlt
  refine rep_of_mult ⟨n, by rw [hn, zpow_add_one₀ two_ne_zero]; ring⟩ (by simp only [binary64_qmin]; omega) ?_ ?_
  · simp only [binary64_prec]
    rw [abs_mul, abs_two, zpow_add_one₀ two_ne_zero]; nlinarith [two_zpow_pos (K := ℚ) e]
  · simp only [binary64_emax]
    rw [abs_mul, abs_two]
    exact lt_of_le_of_lt (by linarith) (lt_of_lt_of_le (by norm_num : (8 : ℚ) < 2 ^ (4 : ℤ)) (two_zpow_le (by norm_num)))

/-- `u/d` against `u/D`. -/
lemma quot_pert {u d D umax e m : ℚ} (hm : 0 < m) (hd : m ≤ d) (hD : m ≤ D) (hu : |u| ≤ umax)
    (he : |d - D| ≤ e) : |u / d - u / D| ≤ umax * e / (m * m) := by
  have hd0 : 0 < d := lt_of_lt_of_le hm hd
  have hD0 : 0 < D := lt_of_lt_of_le hm hD
  rw [div_sub_div _ _ hd0.ne' hD0.ne', show u * D - d * u = u * (D - d) by ring, abs_div, abs_mul,
    abs_of_pos (mul_pos hd0 hD0), abs_sub_comm]
  apply div_le_div₀ (mul_nonneg (le_trans (abs_nonneg _) hu) (le_trans (abs_nonneg _) he))
    (mul_le_mul hu he (abs_nonneg _) (le_trans (abs_nonneg _) hu)) (mul_pos hm hm)
    (mul_le_mul hd hD hm.le hd0.le)

/-- The `id = 0` reduction: `t = (2a − 1)/(2 + a)` within `3·10^-17`. -/
lemma red_t0 {ax : F64} (hx : ax.isFinite) (h1 : 7 / 16 ≤ ax.val) (h2 : ax.val < 11 / 16) :
    (div (sub (mul TWO ax) ONE) (add TWO ax)).isFinite ∧
      |(div (sub (mul TWO ax) ONE) (add TWO ax)).val| ≤ 4375 / 10000 ∧
      |(div (sub (mul TWO ax) ONE) (add TWO ax)).val - (2 * ax.val - 1) / (2 + ax.val)| ≤ 3 / 10 ^ 17 := by
  have fT : TWO.isFinite := by decide
  have fO : ONE.isFinite := by decide
  obtain ⟨fm, vm⟩ := mul_of_rep fT hx (by rw [TWO_val]; exact rep_two_mul hx (by rw [abs_le]; constructor <;> linarith))
  rw [TWO_val] at vm
  obtain ⟨fn, vn⟩ := sub_of_rep fm fO (rep_sub fm fO (by rw [vm, ONE_val]; linarith) (by rw [vm, ONE_val]; linarith))
  rw [vm, ONE_val] at vn
  obtain ⟨fd, ed⟩ := add_ulp fT hx (k := 2) (by rw [TWO_val, abs_lt]; constructor <;> norm_num <;> linarith)
    (by norm_num) (by norm_num)
  rw [TWO_val, show (2 : ℚ) ^ (2 - 54 : ℤ) = 1 / 2 ^ 52 by norm_num] at ed
  have hdl : 2 + 7 / 16 - 1 / 2 ^ 52 ≤ (add TWO ax).val := by have := (abs_le.mp ed).1; linarith
  have hd0 : (add TWO ax).val ≠ 0 := by intro h; rw [h] at hdl; norm_num at hdl
  have hq : |(sub (mul TWO ax) ONE).val / (add TWO ax).val| < 2 ^ (-2 : ℤ) := by
    rw [vn, abs_div, abs_of_pos (show (0 : ℚ) < (add TWO ax).val by linarith), div_lt_iff₀ (by linarith)]
    rw [show (2 : ℚ) ^ (-2 : ℤ) = 1 / 4 by norm_num, abs_lt]; constructor <;> nlinarith
  obtain ⟨ft, et⟩ := div_ulp fn fd hd0 hq (by norm_num) (by norm_num)
  rw [show (2 : ℚ) ^ (-2 - 54 : ℤ) = 1 / 2 ^ 56 by norm_num] at et
  have hp := quot_pert (u := 2 * ax.val - 1) (umax := 3 / 8) (e := 1 / 2 ^ 52) (m := 2 + 7 / 16 - 1 / 2 ^ 52)
    (by norm_num) hdl (by linarith) (by rw [abs_le]; constructor <;> linarith) ed
  rw [vn] at et
  have tot : |(div (sub (mul TWO ax) ONE) (add TWO ax)).val - (2 * ax.val - 1) / (2 + ax.val)| ≤ 3 / 10 ^ 17 := by
    have := abs_sub_le (div (sub (mul TWO ax) ONE) (add TWO ax)).val ((2 * ax.val - 1) / (add TWO ax).val)
      ((2 * ax.val - 1) / (2 + ax.val))
    have n : (1 : ℚ) / 2 ^ 56 + 3 / 8 * (1 / 2 ^ 52) / ((2 + 7 / 16 - 1 / 2 ^ 52) * (2 + 7 / 16 - 1 / 2 ^ 52)) ≤ 3 / 10 ^ 17 := by
      norm_num
    linarith
  refine ⟨ft, ?_, tot⟩
  have hte : |(2 * ax.val - 1) / (2 + ax.val)| ≤ 1 / 4 := by
    rw [abs_div, abs_of_pos (show (0 : ℚ) < 2 + ax.val by linarith), div_le_iff₀ (by linarith), abs_le]
    constructor <;> linarith
  have := abs_sub_abs_le_abs_sub (div (sub (mul TWO ax) ONE) (add TWO ax)).val ((2 * ax.val - 1) / (2 + ax.val))
  linarith

/-- The `id = 1` reduction: `t = (a − 1)/(a + 1)` within `4·10^-17`. -/
lemma red_t1 {ax : F64} (hx : ax.isFinite) (h1 : 11 / 16 ≤ ax.val) (h2 : ax.val < 19 / 16) :
    (div (sub ax ONE) (add ax ONE)).isFinite ∧ |(div (sub ax ONE) (add ax ONE)).val| ≤ 4375 / 10000 ∧
      |(div (sub ax ONE) (add ax ONE)).val - (ax.val - 1) / (ax.val + 1)| ≤ 4 / 10 ^ 17 := by
  have fO : ONE.isFinite := by decide
  obtain ⟨fn, vn⟩ := sub_of_rep hx fO (rep_sub hx fO (by rw [ONE_val]; linarith) (by rw [ONE_val]; linarith))
  rw [ONE_val] at vn
  obtain ⟨fd, ed⟩ := add_ulp hx fO (k := 2) (by rw [ONE_val, abs_lt]; constructor <;> norm_num <;> linarith)
    (by norm_num) (by norm_num)
  rw [ONE_val, show (2 : ℚ) ^ (2 - 54 : ℤ) = 1 / 2 ^ 52 by norm_num] at ed
  have hdl : 11 / 16 + 1 - 1 / 2 ^ 52 ≤ (add ax ONE).val := by have := (abs_le.mp ed).1; linarith
  have hd0 : (add ax ONE).val ≠ 0 := by intro h; rw [h] at hdl; norm_num at hdl
  have hq : |(sub ax ONE).val / (add ax ONE).val| < 2 ^ (-2 : ℤ) := by
    rw [vn, abs_div, abs_of_pos (show (0 : ℚ) < (add ax ONE).val by linarith), div_lt_iff₀ (by linarith)]
    rw [show (2 : ℚ) ^ (-2 : ℤ) = 1 / 4 by norm_num, abs_lt]; constructor <;> nlinarith
  obtain ⟨ft, et⟩ := div_ulp fn fd hd0 hq (by norm_num) (by norm_num)
  rw [show (2 : ℚ) ^ (-2 - 54 : ℤ) = 1 / 2 ^ 56 by norm_num, vn] at et
  have hp := quot_pert (u := ax.val - 1) (umax := 5 / 16) (e := 1 / 2 ^ 52) (m := 11 / 16 + 1 - 1 / 2 ^ 52)
    (by norm_num) hdl (by linarith) (by rw [abs_le]; constructor <;> linarith) ed
  have tot : |(div (sub ax ONE) (add ax ONE)).val - (ax.val - 1) / (ax.val + 1)| ≤ 4 / 10 ^ 17 := by
    have := abs_sub_le (div (sub ax ONE) (add ax ONE)).val ((ax.val - 1) / (add ax ONE).val) ((ax.val - 1) / (ax.val + 1))
    have n : (1 : ℚ) / 2 ^ 56 + 5 / 16 * (1 / 2 ^ 52) / ((11 / 16 + 1 - 1 / 2 ^ 52) * (11 / 16 + 1 - 1 / 2 ^ 52)) ≤ 4 / 10 ^ 17 := by
      norm_num
    linarith
  refine ⟨ft, ?_, tot⟩
  have hte : |(ax.val - 1) / (ax.val + 1)| ≤ 1 / 4 := by
    rw [abs_div, abs_of_pos (show (0 : ℚ) < ax.val + 1 by linarith), div_le_iff₀ (by linarith), abs_le]
    constructor <;> linarith
  have := abs_sub_abs_le_abs_sub (div (sub ax ONE) (add ax ONE)).val ((ax.val - 1) / (ax.val + 1))
  linarith

/-- The `id = 2` reduction: `t = (a − 3/2)/(1 + 3/2·a)` within `1.1·10^-16`. -/
lemma red_t2 {ax : F64} (hx : ax.isFinite) (h1 : 19 / 16 ≤ ax.val) (h2 : ax.val < 39 / 16) :
    (div (sub ax THREEHALF) (add ONE (mul THREEHALF ax))).isFinite ∧
      |(div (sub ax THREEHALF) (add ONE (mul THREEHALF ax))).val| ≤ 4375 / 10000 ∧
      |(div (sub ax THREEHALF) (add ONE (mul THREEHALF ax))).val - (ax.val - 3 / 2) / (1 + 3 / 2 * ax.val)| ≤
        11 / 10 ^ 17 := by
  have fO : ONE.isFinite := by decide
  have fH : THREEHALF.isFinite := by decide
  obtain ⟨fn, vn⟩ := sub_of_rep hx fH (rep_sub hx fH (by rw [THREEHALF_val]; linarith) (by rw [THREEHALF_val]; linarith))
  rw [THREEHALF_val] at vn
  obtain ⟨fv1, ev1⟩ := mul_ulp fH hx (k := 2) (by rw [THREEHALF_val, abs_lt]; constructor <;> norm_num <;> linarith)
    (by norm_num) (by norm_num)
  rw [THREEHALF_val, show (2 : ℚ) ^ (2 - 54 : ℤ) = 1 / 2 ^ 52 by norm_num] at ev1
  have hv1 := abs_le.mp ev1
  obtain ⟨fv2, ev2⟩ := add_ulp fO fv1 (k := 3) (by rw [ONE_val, abs_lt]; constructor <;> norm_num <;> linarith)
    (by norm_num) (by norm_num)
  rw [ONE_val, show (2 : ℚ) ^ (3 - 54 : ℤ) = 2 / 2 ^ 52 by norm_num] at ev2
  have hv2 := abs_le.mp ev2
  set d := add ONE (mul THREEHALF ax)
  have ed : |d.val - (1 + 3 / 2 * ax.val)| ≤ 3 / 2 ^ 52 := by
    rw [abs_le]; constructor <;> linarith
  have hdl : 1 + 3 / 2 * (19 / 16) - 3 / 2 ^ 52 ≤ d.val := by linarith
  have hd0 : d.val ≠ 0 := by intro h; rw [h] at hdl; norm_num at hdl
  have hq : |(sub ax THREEHALF).val / d.val| < 2 ^ (-1 : ℤ) := by
    rw [vn, abs_div, abs_of_pos (show (0 : ℚ) < d.val by linarith), div_lt_iff₀ (by linarith)]
    rw [show (2 : ℚ) ^ (-1 : ℤ) = 1 / 2 by norm_num, abs_lt]; constructor <;> nlinarith
  obtain ⟨ft, et⟩ := div_ulp fn fv2 hd0 hq (by norm_num) (by norm_num)
  rw [show (2 : ℚ) ^ (-1 - 54 : ℤ) = 1 / 2 ^ 55 by norm_num, vn] at et
  have hp := quot_pert (u := ax.val - 3 / 2) (umax := 15 / 16) (e := 3 / 2 ^ 52) (m := 1 + 3 / 2 * (19 / 16) - 3 / 2 ^ 52)
    (by norm_num) hdl (by linarith) (by rw [abs_le]; constructor <;> linarith) ed
  have tot : |(div (sub ax THREEHALF) d).val - (ax.val - 3 / 2) / (1 + 3 / 2 * ax.val)| ≤ 11 / 10 ^ 17 := by
    have := abs_sub_le (div (sub ax THREEHALF) d).val ((ax.val - 3 / 2) / d.val) ((ax.val - 3 / 2) / (1 + 3 / 2 * ax.val))
    have n : (1 : ℚ) / 2 ^ 55 + 15 / 16 * (3 / 2 ^ 52) /
        ((1 + 3 / 2 * (19 / 16) - 3 / 2 ^ 52) * (1 + 3 / 2 * (19 / 16) - 3 / 2 ^ 52)) ≤ 11 / 10 ^ 17 := by
      norm_num
    linarith
  refine ⟨ft, ?_, tot⟩
  have hte : |(ax.val - 3 / 2) / (1 + 3 / 2 * ax.val)| ≤ 34 / 100 := by
    rw [abs_div, abs_of_pos (show (0 : ℚ) < 1 + 3 / 2 * ax.val by linarith), div_le_iff₀ (by linarith), abs_le]
    constructor <;> linarith
  have := abs_sub_abs_le_abs_sub (div (sub ax THREEHALF) d).val ((ax.val - 3 / 2) / (1 + 3 / 2 * ax.val))
  linarith

/-- The `id = 3` reduction: `t = −1/a` within `3·10^-17`. -/
lemma red_t3 {ax : F64} (hx : ax.isFinite) (h1 : 39 / 16 ≤ ax.val) :
    (div MONE ax).isFinite ∧ |(div MONE ax).val| ≤ 4375 / 10000 ∧
      |(div MONE ax).val - (-1 / ax.val)| ≤ 3 / 10 ^ 17 := by
  have fM : MONE.isFinite := by decide
  have h0 : ax.val ≠ 0 := by intro h; rw [h] at h1; norm_num at h1
  have hte : |(-1 / ax.val)| ≤ 16 / 39 := by
    rw [abs_div, abs_neg, abs_one, abs_of_pos (show (0 : ℚ) < ax.val by linarith), div_le_iff₀ (by linarith)]
    linarith
  have hq : |MONE.val / ax.val| < 2 ^ (-1 : ℤ) := by
    rw [MONE_val, show (2 : ℚ) ^ (-1 : ℤ) = 1 / 2 by norm_num]; linarith
  obtain ⟨ft, et⟩ := div_ulp fM hx h0 hq (by norm_num) (by norm_num)
  rw [show (2 : ℚ) ^ (-1 - 54 : ℤ) = 1 / 2 ^ 55 by norm_num, MONE_val] at et
  have n : (1 : ℚ) / 2 ^ 55 ≤ 3 / 10 ^ 17 := by norm_num
  refine ⟨ft, ?_, le_trans et n⟩
  have := abs_sub_abs_le_abs_sub (div MONE ax).val (-1 / ax.val)
  linarith

/-! ## The model and the contract -/

/-- `ATANHI[i] − ((ax·(s1+s2) − ATANLO[i]) − ax)` at the reduced `ax = t`. -/
def atanTail (hi lo t : F64) : F64 := sub hi (sub (sub (atanPoly t) lo) t)

/-- `dmath::atan` on a finite argument, branch for branch (`ax >= 7.3786976294838206e19` is
`2^66 ≤ |x|`, `ax < 7.450580596923828e-9` is `|x| < 2^-27`; comparisons of finite values are exact). -/
def atanR (x : F64) : F64 :=
  let ax := x.abs
  if BIG66.val ≤ ax.val then (if x.sign then (add ATANHI3 ATANLO3).neg else add ATANHI3 ATANLO3)
  else if ax.val < 7 / 16 then
    if ax.val < TINY27.val then x
    else (if x.sign then (sub ax (atanPoly ax)).neg else sub ax (atanPoly ax))
  else
    let r := if ax.val < 19 / 16 then
        (if ax.val < 11 / 16 then atanTail ATANHI0 ATANLO0 (div (sub (mul TWO ax) ONE) (add TWO ax))
         else atanTail ATANHI1 ATANLO1 (div (sub ax ONE) (add ax ONE)))
      else if ax.val < 39 / 16 then
        atanTail ATANHI2 ATANLO2 (div (sub ax THREEHALF) (add ONE (mul THREEHALF ax)))
      else atanTail ATANHI3 ATANLO3 (div MONE ax)
    if x.sign then r.neg else r

/-- `dmath::atan`. -/
def atan (x : F64) : F64 :=
  if x.isNaN then x
  else if x.isFinite then atanR x
  else if x.sign then (add ATANHI3 ATANLO3).neg else add ATANHI3 ATANLO3

lemma sign_val {x : F64} : (x.sign = true → x.val = -|x.val|) ∧ (x.sign = false → x.val = |x.val|) := by
  cases x with
  | fin s m e h =>
    have : (0 : ℚ) ≤ m * 2 ^ e := mul_nonneg (Nat.cast_nonneg _) (two_zpow_pos _).le
    cases s <;> simp [FP.sign, FP.val, sgn, abs_of_nonneg this]
  | _ => simp [FP.sign, FP.val]

/-- The sign step: from `r ≈ arctan |x|` to `±r ≈ arctan x`. -/
lemma signed {x r : F64} {E : ℝ} (hr : r.isFinite) (h : |(r.val : ℝ) - arctan (|(x.val : ℝ)|)| ≤ E) :
    (if x.sign then r.neg else r).isFinite ∧
      |((if x.sign then r.neg else r).val : ℝ) - arctan x.val| ≤ E := by
  obtain ⟨hn, hp⟩ := @sign_val x
  cases hs : x.sign
  · simp only [Bool.false_eq_true, if_false]
    refine ⟨hr, ?_⟩
    have := hp hs
    have e : (x.val : ℝ) = |(x.val : ℝ)| := by rw [← Rat.cast_abs, ← this]
    rw [e]; exact h
  · simp only [if_true]
    refine ⟨by simpa using hr, ?_⟩
    have := hn hs
    have e : (x.val : ℝ) = -|(x.val : ℝ)| := by rw [← Rat.cast_abs, ← Rat.cast_neg, ← this]
    rw [e, arctan_neg, FP.val_neg]; push_cast
    rw [show -(r.val : ℝ) - -arctan |(x.val : ℝ)| = -((r.val : ℝ) - arctan (|(x.val : ℝ)|)) by ring, abs_neg]
    exact h

lemma arctan_abs_le (a : ℝ) : |arctan a| ≤ |a| := by
  have := arctan_lip a 0; rwa [arctan_zero, _root_.sub_zero, _root_.sub_zero] at this

/-- The large branch: `ATANHI[3] + ATANLO[3]` against `arctan a`, `a ≥ 2^66`. -/
lemma big_err {a : ℝ} (ha : 73786976294838206464 ≤ a) :
    (add ATANHI3 ATANLO3).isFinite ∧ |((add ATANHI3 ATANLO3).val : ℝ) - arctan a| ≤ 3 / 10 ^ 16 := by
  have f3 : ATANHI3.isFinite := by decide
  have fl : ATANLO3.isFinite := by decide
  obtain ⟨fz, ez⟩ := add_ulp f3 fl (k := 1) (by rw [ATANHI3_val, ATANLO3_val]; norm_num) (by norm_num) (by norm_num)
  refine ⟨fz, ?_⟩
  rw [show (2 : ℚ) ^ (1 - 54 : ℤ) = 1 / 2 ^ 53 by norm_num] at ez
  have ezr := qcast ez
  have hc := c_pio2
  push_cast at ezr hc
  have h3 := red3 (a := a) (by linarith)
  have hb := arctan_abs_le (-1 / a)
  have hia : |-1 / a| ≤ 1 / 10 ^ 19 := by
    rw [abs_div, abs_neg, abs_one, abs_of_pos (by linarith), div_le_iff₀ (by linarith)]; linarith
  rw [h3]
  have := abs_le.mp ezr; have := abs_le.mp hc; have := abs_le.mp hb
  rw [abs_le]; constructor <;> nlinarith

/-- The `id = -1` branch: `ax − ax·(s1+s2)` against `arctan a`, `2^-27 ≤ a < 7/16`. -/
lemma small_err {ax : F64} (hx : ax.isFinite) (h0 : 0 ≤ ax.val) (h1 : ax.val < 7 / 16) :
    (sub ax (atanPoly ax)).isFinite ∧ |((sub ax (atanPoly ax)).val : ℝ) - arctan ax.val| ≤ 3 / 10 ^ 16 := by
  have hb : |ax.val| ≤ 4375 / 10000 := by rw [abs_of_nonneg h0]; linarith
  obtain ⟨fp, ep, mp⟩ := atanPoly_eval hx hb
  have h2 : |ax.val - (atanPoly ax).val| < 2 ^ (-1 : ℤ) := by
    have := abs_sub ax.val (atanPoly ax).val
    have := abs_sub_abs_le_abs_sub (atanPoly ax).val (ax.val - ax.val * evalP katanL (ax.val ^ 2))
    rw [show (2 : ℚ) ^ (-1 : ℤ) = 1 / 2 by norm_num]; linarith
  obtain ⟨fr, er⟩ := sub_ulp hx fp h2 (by norm_num) (by norm_num)
  refine ⟨fr, ?_⟩
  rw [show (2 : ℚ) ^ (-1 - 54 : ℤ) = 1 / 2 ^ 55 by norm_num] at er
  have hk := katan_poly hb
  have e1 : |(sub ax (atanPoly ax)).val - ax.val * evalP katanL (ax.val ^ 2)| ≤ 1 / 2 ^ 55 + 47 / 2500000000000000000 := by
    have := abs_sub_le (sub ax (atanPoly ax)).val (ax.val - (atanPoly ax).val) (ax.val * evalP katanL (ax.val ^ 2))
    rw [show ax.val - (atanPoly ax).val - ax.val * evalP katanL (ax.val ^ 2) =
      -((atanPoly ax).val - (ax.val - ax.val * evalP katanL (ax.val ^ 2))) by ring, abs_neg] at this
    linarith
  have e1r := qcast e1
  push_cast at e1r hk
  have := abs_sub_le ((sub ax (atanPoly ax)).val : ℝ) ((ax.val : ℝ) * ((evalP katanL (ax.val ^ 2) : ℚ) : ℝ)) (arctan ax.val)
  have n : ((1 : ℝ) / 2 ^ 55 + 47 / 2500000000000000000) + (1 / 2 ^ 57 + 1 / 10 ^ 19) ≤ 3 / 10 ^ 16 := by norm_num
  linarith

/-- The tiny branch: `x` itself, `|x| < 2^-27`. -/
lemma tiny_err {a : ℝ} (ha : |a| < 1 / 2 ^ 27) : |a - arctan a| ≤ 3 / 10 ^ 16 := by
  have h := atan_taylor 1 a
  have h' : |arctan a - a| ≤ |a| ^ 3 := by
    have e : ∑ k ∈ Finset.range 1, (-1 : ℝ) ^ k * a ^ (2 * k + 1) / (2 * k + 1) = a := by simp
    rw [e] at h; exact h
  have h3 : |a| ^ 3 ≤ (1 / 2 ^ 27) ^ 3 := pow_le_pow_left₀ (abs_nonneg a) ha.le 3
  rw [abs_sub_comm]
  have : (1 / 2 ^ 27 : ℝ) ^ 3 ≤ 3 / 10 ^ 16 := by norm_num
  linarith

lemma arctan_abs_nonneg (a : ℝ) : 0 ≤ arctan |a| := arctan_nonneg.mpr (abs_nonneg a)

/-- **`dmath::atan`.** For finite `x`, `atan(x)` is finite and within `3·10^-16` of `arctan x`. -/
theorem atan_err {x : F64} (hx : x.isFinite) :
    (atan x).isFinite ∧ |((atan x).val : ℝ) - arctan x.val| ≤ 3 / 10 ^ 16 := by
  have hnan : x.isNaN = false := by cases x <;> simp_all [FP.isFinite, FP.isNaN]
  unfold atan atanR
  rw [if_neg (by simp [hnan]), if_pos hx]
  have fa : x.abs.isFinite := by simpa using hx
  have va : x.abs.val = |x.val| := abs_val x
  have va0 : 0 ≤ x.abs.val := by rw [va]; exact abs_nonneg _
  have var : ((x.abs.val : ℚ) : ℝ) = |(x.val : ℝ)| := by rw [va, Rat.cast_abs]
  set a := x.abs.val
  by_cases hbig : BIG66.val ≤ a
  · rw [if_pos hbig]
    rw [BIG66_val] at hbig
    have hr : (73786976294838206464 : ℝ) ≤ |(x.val : ℝ)| := by
      have h' := (Rat.cast_le (K := ℝ)).mpr hbig; push_cast at h'; rw [← var]; exact h'
    obtain ⟨f, e⟩ := big_err hr
    exact signed f e
  rw [if_neg hbig]; rw [BIG66_val, not_le] at hbig
  by_cases h716 : a < 7 / 16
  · rw [if_pos h716]
    by_cases htiny : a < TINY27.val
    · rw [if_pos htiny]
      refine ⟨hx, ?_⟩
      rw [TINY27_val] at htiny
      have : |(x.val : ℝ)| < 1 / 2 ^ 27 := by
        have h' := (Rat.cast_lt (K := ℝ)).mpr htiny; push_cast at h'; rw [← var]; exact h'
      exact tiny_err this
    · rw [if_neg htiny]
      obtain ⟨f, e⟩ := small_err fa va0 h716
      rw [var] at e
      exact signed f e
  rw [if_neg h716]; rw [not_lt] at h716
  have key : ∀ r : F64, r.isFinite ∧ |(r.val : ℝ) - arctan (|(x.val : ℝ)|)| ≤ 3 / 10 ^ 16 →
      (if x.sign then r.neg else r).isFinite ∧
        |((if x.sign then r.neg else r).val : ℝ) - arctan x.val| ≤ 3 / 10 ^ 16 := fun r h => signed h.1 h.2
  apply key
  have ha0r : (0 : ℝ) ≤ a := by exact_mod_cast va0
  -- the reduced branches, unsigned
  have har : (a : ℝ) = |(x.val : ℝ)| := var
  have hpi := Real.pi_lt_d2
  have hpi' := Real.pi_gt_three
  by_cases h1916 : a < 19 / 16
  · rw [if_pos h1916]
    by_cases h1116 : a < 11 / 16
    · rw [if_pos h1116]
      obtain ⟨ft, htb, hte⟩ := red_t0 fa h716 h1116
      have hter := qcast hte
      push_cast at hter
      have hred : arctan (a : ℝ) = arctan (1 / 2) + arctan ((2 * (a : ℝ) - 1) / (2 + a)) :=
        red0 (by linarith)
      have hroom : |arctan (a : ℝ)| + (3 / 10 ^ 17 + 1 / 10 ^ 16) < 2 ^ (0 : ℤ) := by
        have := arctan_abs_le (a : ℝ)
        have : (a : ℝ) < 11 / 16 := by
          have h' := (Rat.cast_lt (K := ℝ)).mpr h1116; push_cast at h'; exact h'
        rw [abs_of_nonneg (by exact_mod_cast va0 : (0 : ℝ) ≤ a)] at *
        norm_num; linarith
      obtain ⟨fr, er⟩ := reduced_err (by decide) (by decide) ft (by norm_num) (by norm_num) htb hter
        (by rw [ATANLO0_val]; norm_num) c_half hred hroom
      rw [← har]
      refine ⟨fr, le_trans er (by norm_num)⟩
    · rw [if_neg h1116]; rw [not_lt] at h1116
      obtain ⟨ft, htb, hte⟩ := red_t1 fa h1116 h1916
      have hter := qcast hte
      push_cast at hter
      have ha0 : (11 / 16 : ℝ) ≤ a := by
        have h' := (Rat.cast_le (K := ℝ)).mpr h1116; push_cast at h'; exact h'
      have ha1 : (a : ℝ) < 19 / 16 := by
        have h' := (Rat.cast_lt (K := ℝ)).mpr h1916; push_cast at h'; exact h'
      have hred : arctan (a : ℝ) = π / 4 + arctan (((a : ℝ) - 1) / (a + 1)) := red1 (by linarith)
      have hroom : |arctan (a : ℝ)| + (4 / 10 ^ 17 + 1 / 10 ^ 16) < 2 ^ (0 : ℤ) := by
        have hte5 : |((a : ℝ) - 1) / (a + 1)| ≤ 5 / 27 := by
          rw [abs_div, abs_of_pos (show (0 : ℝ) < a + 1 by linarith), div_le_iff₀ (by linarith), abs_le]
          constructor <;> linarith
        have := arctan_abs_le (((a : ℝ) - 1) / (a + 1))
        have hnn : 0 ≤ arctan (a : ℝ) := arctan_nonneg.mpr (by linarith)
        rw [abs_of_nonneg hnn, hred]
        have := (abs_le.mp (le_trans this hte5)).2
        norm_num; linarith
      obtain ⟨fr, er⟩ := reduced_err (by decide) (by decide) ft (by norm_num) (by norm_num) htb hter
        (by rw [ATANLO1_val]; norm_num) c_quarter hred hroom
      rw [← har]
      refine ⟨fr, le_trans er (by norm_num)⟩
  · rw [if_neg h1916]; rw [not_lt] at h1916
    have hroom2 : ∀ δ : ℝ, δ ≤ 11 / 10 ^ 17 → |arctan (a : ℝ)| + (δ + 1 / 10 ^ 16) < 2 ^ (1 : ℤ) := by
      intro δ hδ
      have hnn : 0 ≤ arctan (a : ℝ) := arctan_nonneg.mpr (by exact_mod_cast va0)
      have := arctan_lt_pi_div_two (a : ℝ)
      rw [abs_of_nonneg hnn]; norm_num; linarith
    by_cases h3916 : a < 39 / 16
    · rw [if_pos h3916]
      obtain ⟨ft, htb, hte⟩ := red_t2 fa h1916 h3916
      have hter := qcast hte
      push_cast at hter
      have hred : arctan (a : ℝ) = arctan (3 / 2) + arctan (((a : ℝ) - 3 / 2) / (1 + 3 / 2 * a)) :=
        red2 (by exact_mod_cast va0)
      obtain ⟨fr, er⟩ := reduced_err (by decide) (by decide) ft (by norm_num) (by norm_num) htb hter
        (by rw [ATANLO2_val]; norm_num) c_threehalf hred (hroom2 _ le_rfl)
      rw [← har]
      refine ⟨fr, le_trans er (by norm_num)⟩
    · rw [if_neg h3916]; rw [not_lt] at h3916
      obtain ⟨ft, htb, hte⟩ := red_t3 fa h3916
      have hter := qcast hte
      push_cast at hter
      have h39 : (39 / 16 : ℝ) ≤ a := by
        have h' := (Rat.cast_le (K := ℝ)).mpr h3916; push_cast at h'; exact h'
      have hred : arctan (a : ℝ) = π / 2 + arctan (-1 / (a : ℝ)) := red3 (by linarith)
      obtain ⟨fr, er⟩ := reduced_err (by decide) (by decide) ft (by norm_num) (by norm_num) htb hter
        (by rw [ATANLO3_val]; norm_num) c_pio2 hred (hroom2 _ (by norm_num))
      rw [← har]
      refine ⟨fr, le_trans er (by norm_num)⟩

end IEEE754.Dmath
