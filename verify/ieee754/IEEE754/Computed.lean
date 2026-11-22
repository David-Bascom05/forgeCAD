/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Identities
import IEEE754.Compare

/-!
# Error relative to the computed result, and where infinities and NaN come from

* `round_rel_err_res`: outside the subnormal range, `|round r − r| ≤ 2^-prec · |round r|` (the
  error relative to the *result*, as forward error analyses of floating-point predicates use it);
* `add_err_res`, `sub_err_res`: for a sum or difference of two finite data, that bound holds
  always (in the subnormal range the operation is exact);
* `mul_err_res`: for a product it holds unless the exact product is nonzero and below `2^emin`;
* finiteness: a finite sum, product or absolute value has finite operands.
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {f : Format}

/-- The magnitude of a finite rounding is the unbounded rounding of the magnitude. -/
lemma abs_round_val {K : Type*} [Field K] [LinearOrder K] [IsStrictOrderedRing K] [FloorRing K]
    {zs : Bool} {r : K} (hr : r ≠ 0) (hfin : (round f zs r).isFinite) :
    |((round f zs r).val : K)| = uround f |r| := by
  have hu := (round_finite_iff f hr).mp hfin
  rcases lt_or_gt_of_ne hr with h | h
  · rw [abs_of_neg h] at hu ⊢
    rw [(round_neg' f h hu).2.1, abs_neg, abs_of_nonneg (uround_nonneg f _)]
  · rw [abs_of_pos h] at hu ⊢
    rw [(round_pos f h hu).2.1, abs_of_nonneg (uround_nonneg f _)]

/-- **Relative error, relative to the result.** -/
theorem round_rel_err_res {K : Type*} [Field K] [LinearOrder K] [IsStrictOrderedRing K] [FloorRing K]
    {zs : Bool} {r : K} (hfin : (round f zs r).isFinite) (hr : 2 ^ f.emin ≤ |r|) :
    |((round f zs r).val : K) - r| ≤ 2 ^ (-(f.prec : ℤ)) * |((round f zs r).val : K)| := by
  have hr0 : r ≠ 0 := by
    intro h; rw [h, abs_zero] at hr; linarith [two_zpow_pos (K := K) f.emin]
  have ha : 0 < |r| := abs_pos.mpr hr0
  rw [round_val_abs f hr0 hfin, abs_round_val hr0 hfin]
  refine le_trans (uround_err f ha) ?_
  -- the quantum exponent is `log |r| - p + 1 > qmin`, so the significand is at least `2^(p-1)`
  have hE := le_log_of_le ha hr
  have he : (roundPos f |r|).2 = Int.log 2 |r| - f.prec + 1 := by
    have := roundPos_snd f |r|; have := f.qmin_add_prec; omega
  have hq : f.qmin < (roundPos f |r|).2 ∨ (roundPos f |r|).2 = f.qmin := by
    have := roundPos_ge_qmin f |r|; omega
  have hn : 2 ^ (f.prec - 1) ≤ (roundPos f |r|).1 := by
    rcases hq with hq | hq
    · exact (roundPos_normal f ha hq).1
    · -- `e = qmin` and `e = log |r| - p + 1 ≥ emin - p + 1 = qmin`: still normal
      have hE' : Int.log 2 |r| = f.emin := by have := f.qmin_add_prec; omega
      rw [roundPos_fst]
      apply le_rne
      rw [cast_two_pow, le_div_iff₀ (two_zpow_pos _), ← two_zpow_add, hq, f.prec_sub_one]
      rw [show ((f.prec : ℤ) - 1 + f.qmin) = f.emin by have := f.qmin_add_prec; omega]
      exact hr
  have : (2 : K) ^ ((f.prec : ℤ) - 1 + (roundPos f |r|).2) ≤ uround f |r| := by
    simp only [uround]
    rw [two_zpow_add]
    apply mul_le_mul_of_nonneg_right _ (two_zpow_pos _).le
    rw [← f.prec_sub_one, ← cast_two_pow]; exact_mod_cast hn
  calc (2 : K) ^ ((roundPos f |r|).2 - 1)
      = 2 ^ (-(f.prec : ℤ)) * 2 ^ ((f.prec : ℤ) - 1 + (roundPos f |r|).2) := by
        rw [← two_zpow_add]; congr 1; ring
    _ ≤ 2 ^ (-(f.prec : ℤ)) * uround f |r| :=
        mul_le_mul_of_nonneg_left this (two_zpow_pos _).le

lemma round_rel_err_res_q {zs : Bool} {q : ℚ} (hfin : (round f zs q).isFinite) (hq : 2 ^ f.emin ≤ |q|) :
    |(round f zs q).val - q| ≤ 2 ^ (-(f.prec : ℤ)) * |(round f zs q).val| := by
  simpa using round_rel_err_res (K := ℚ) hfin hq

theorem add_err_res {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (add x y).isFinite) :
    |(add x y).val - (x.val + y.val)| ≤ 2 ^ (-(f.prec : ℤ)) * |(add x y).val| := by
  rcases lt_or_ge |x.val + y.val| (2 ^ f.emin) with h | h
  · rw [add_exact_of_small hx hy h, _root_.sub_self, abs_zero]
    exact mul_nonneg (two_zpow_pos _).le (abs_nonneg _)
  · rw [add_eq_round hx hy] at hfin ⊢; exact round_rel_err_res_q hfin h

theorem sub_err_res {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (sub x y).isFinite) :
    |(sub x y).val - (x.val - y.val)| ≤ 2 ^ (-(f.prec : ℤ)) * |(sub x y).val| := by
  have hy' : y.neg.isFinite := by simpa using hy
  have := add_err_res hx hy' hfin
  simpa [sub, sub_eq_add_neg] using this

theorem mul_err_res {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (mul x y).isFinite)
    (hn : x.val * y.val = 0 ∨ 2 ^ f.emin ≤ |x.val * y.val|) :
    |(mul x y).val - x.val * y.val| ≤ 2 ^ (-(f.prec : ℤ)) * |(mul x y).val| := by
  rw [mul_eq_round hx hy] at hfin ⊢
  rcases hn with h | h
  · rw [h, round_zero]; simp
  · exact round_rel_err_res_q hfin h

/-! ## Finiteness -/

lemma isFinite_of_add {x y : FP f} (h : (add x y).isFinite) : x.isFinite ∧ y.isFinite := by
  cases x <;> cases y <;> simp_all [add, isFinite] <;> split_ifs at h <;> simp_all [isFinite]

lemma isFinite_of_mul {x y : FP f} (h : (mul x y).isFinite) : x.isFinite ∧ y.isFinite := by
  cases x <;> cases y <;> simp_all [mul, isFinite, isZero] <;> split_ifs at h <;> simp_all [isFinite]

lemma isFinite_of_sub {x y : FP f} (h : (sub x y).isFinite) : x.isFinite ∧ y.isFinite := by
  have := isFinite_of_add (x := x) (y := y.neg) h
  simpa using this

@[simp] lemma isFinite_abs (x : FP f) : x.abs.isFinite = x.isFinite := by cases x <;> rfl

@[simp] lemma isNaN_abs (x : FP f) : x.abs.isNaN = x.isNaN := by cases x <;> rfl

lemma val_abs_nonneg (x : FP f) : 0 ≤ x.abs.val := by rw [abs_val]; exact abs_nonneg _

/-- A datum is finite, infinite or NaN. -/
lemma trichotomy (x : FP f) : x.isFinite ∨ (∃ s, x = .inf s) ∨ x = .nan := by
  cases x <;> simp [isFinite]

end IEEE754
