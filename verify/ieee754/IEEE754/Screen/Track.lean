/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Gap

/-!
# Tracked errors

`Tr v X M k`: the f64 `v` is finite and within `k·ε·M` of the exact `X`, where `|X| ≤ M`. The
lemmas `tr_mul`, `tr_add` and `tr_sub` propagate it through one rounded operation.
- Addition and subtraction of floats never underflow, so their rounding is relative (`add_rel0`).
- Multiplication needs its exact product to be `0` or normal (`NZ`).
-/

namespace IEEE754.Screen

open FP

/-- Zero or normal: the product rounds with a relative error only. -/
def NZ (x : ℚ) : Prop := x = 0 ∨ (2 : ℚ) ^ (-1022 : ℤ) ≤ |x|

structure Tr (v : F64) (X M k : ℚ) : Prop where
  fin : v.isFinite
  err : |v.val - X| ≤ k * ε * M
  mag : |X| ≤ M

lemma add_rel0 {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hf : (add a b).isFinite) :
    |(add a b).val - (a.val + b.val)| ≤ ε * |a.val + b.val| := by
  rcases lt_or_ge |a.val + b.val| (2 ^ (-1022 : ℤ)) with h | h
  · obtain ⟨ea, hea, hma, -⟩ := mult_val ha
    obtain ⟨eb, heb, hmb, -⟩ := mult_val hb
    simp only [binary64_qmin] at hea heb
    have hm : Mult (a.val + b.val) (-1074) := mult_add (mult_of_le hma hea) (mult_of_le hmb heb)
    have hr : Rep binary64 (a.val + b.val) := rep_of_mult hm (by simp) (by
      simp only [binary64_prec]
      refine le_trans h.le ?_
      rw [← zpow_natCast, ← two_zpow_add]; exact two_zpow_le (by norm_num))
      (lt_of_lt_of_le h (two_zpow_le (by simp)))
    rw [(add_of_rep ha hb hr).2, _root_.sub_self, abs_zero]; positivity
  · have := F64.add_rel_err ha hb hf h
    rwa [show (2 : ℚ) ^ (-53 : ℤ) = 1 / 9007199254740992 by norm_num] at this

lemma sub_rel0 {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hf : (sub a b).isFinite) :
    |(sub a b).val - (a.val - b.val)| ≤ ε * |a.val - b.val| := by
  have hbn : b.neg.isFinite := by simpa using hb
  have := add_rel0 ha hbn hf
  rwa [FP.val_neg, ← sub_eq_add_neg] at this

lemma mul_rel0 {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hf : (mul a b).isFinite)
    (hz : NZ (a.val * b.val)) : |(mul a b).val - a.val * b.val| ≤ ε * |a.val * b.val| := by
  rcases hz with h | h
  · rw [mul_eq_round ha hb, h, round_zero]; simp [FP.val]
  · have := F64.mul_rel_err ha hb hf h
    rwa [show (2 : ℚ) ^ (-53 : ℤ) = 1 / 9007199254740992 by norm_num] at this

lemma div_rel0 {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hb0 : b.val ≠ 0) (hf : (div a b).isFinite)
    (hz : NZ (a.val / b.val)) : |(div a b).val - a.val / b.val| ≤ ε * |a.val / b.val| := by
  rcases hz with h | h
  · rw [div_eq_round ha hb hb0, h, round_zero]; simp [FP.val]
  · have := F64.div_rel_err ha hb hb0 hf h
    rwa [show (2 : ℚ) ^ (-53 : ℤ) = 1 / 9007199254740992 by norm_num] at this

lemma Tr.of_input {v : F64} {X : ℚ} (hv : v.isFinite) (h : |v.val - X| ≤ 2 * ε * |X|) : Tr v X |X| 2 :=
  ⟨hv, by linarith, le_rfl⟩

lemma Tr.exact {v : F64} (hv : v.isFinite) : Tr v v.val |v.val| 0 := ⟨hv, by simp, le_rfl⟩

lemma Tr.abs {v : F64} {X M k : ℚ} (h : Tr v X M k) : Tr v.abs |X| M k :=
  ⟨by simpa using h.fin, by rw [FP.abs_val]; exact le_trans (abs_abs_sub_abs_le_abs_sub _ _) h.err,
    by rw [abs_abs]; exact h.mag⟩

lemma Tr.weaken {v : F64} {X M k k' : ℚ} (h : Tr v X M k) (hk : k ≤ k') (hM : 0 ≤ M) : Tr v X M k' :=
  ⟨h.fin, le_trans h.err (by nlinarith), h.mag⟩

lemma Tr.M_nonneg {v : F64} {X M k : ℚ} (h : Tr v X M k) : 0 ≤ M := le_trans (abs_nonneg _) h.mag

lemma Tr.abs_le {v : F64} {X M k : ℚ} (h : Tr v X M k) : |v.val| ≤ (1 + k * ε) * M := by
  have := abs_sub_abs_le_abs_sub v.val X
  have := h.err; have := h.mag
  nlinarith

lemma tr_mul {a b : F64} {Xa Ma ka Xb Mb kb : ℚ} (ha : Tr a Xa Ma ka) (hb : Tr b Xb Mb kb)
    (hka : 0 ≤ ka) (hka' : ka ≤ 1000) (hkb : 0 ≤ kb) (hkb' : kb ≤ 1000)
    (hf : (mul a b).isFinite) (hz : NZ (a.val * b.val)) :
    Tr (mul a b) (Xa * Xb) (Ma * Mb) (ka + kb + 2) := by
  have hMa := ha.M_nonneg; have hMb := hb.M_nonneg
  have e := mul_rel0 ha.fin hb.fin hf hz
  have a1 := ha.abs_le
  have hXb := hb.mag
  have d : |a.val * b.val - Xa * Xb| ≤ ((1 + ka * ε) * Ma) * (kb * ε * Mb) + Mb * (ka * ε * Ma) := by
    rw [show a.val * b.val - Xa * Xb = a.val * (b.val - Xb) + Xb * (a.val - Xa) by ring]
    refine le_trans (abs_add_le _ _) (add_le_add ?_ ?_)
    · rw [abs_mul]; exact mul_le_mul a1 hb.err (abs_nonneg _) (by positivity)
    · rw [abs_mul]; exact mul_le_mul hXb ha.err (abs_nonneg _) hMb
  have r : ε * |a.val * b.val| ≤ ε * (((1 + ka * ε) * Ma) * ((1 + kb * ε) * Mb)) := by
    rw [abs_mul]; exact mul_le_mul_of_nonneg_left (mul_le_mul a1 hb.abs_le (abs_nonneg _) (by positivity))
      (by norm_num)
  refine ⟨hf, ?_, by rw [abs_mul]; exact mul_le_mul ha.mag hXb (abs_nonneg _) hMa⟩
  have := abs_sub_le (mul a b).val (a.val * b.val) (Xa * Xb)
  have hMM : 0 ≤ Ma * Mb := mul_nonneg hMa hMb
  -- `(1 + kε)` factors are within `1 + 10^-12`
  have k1 : ka * ε ≤ 1 / 10 ^ 12 := by nlinarith
  have k2 : kb * ε ≤ 1 / 10 ^ 12 := by nlinarith
  have k0 : 0 ≤ ka * ε := by positivity
  have k0' : 0 ≤ kb * ε := by positivity
  nlinarith [mul_nonneg (mul_nonneg k0 k0') hMM, mul_nonneg k0 hMM, mul_nonneg k0' hMM]

lemma tr_add {a b : F64} {Xa Ma Xb Mb K : ℚ} (ha : Tr a Xa Ma K) (hb : Tr b Xb Mb K)
    (hK : 0 ≤ K) (hK' : K ≤ 1000) (hf : (add a b).isFinite) :
    Tr (add a b) (Xa + Xb) (Ma + Mb) (K + 2) := by
  have hMa := ha.M_nonneg; have hMb := hb.M_nonneg
  have e := add_rel0 ha.fin hb.fin hf
  have a1 := ha.abs_le; have b1 := hb.abs_le
  refine ⟨hf, ?_, le_trans (abs_add_le _ _) (add_le_add ha.mag hb.mag)⟩
  have d : |a.val + b.val - (Xa + Xb)| ≤ K * ε * Ma + K * ε * Mb := by
    rw [show a.val + b.val - (Xa + Xb) = (a.val - Xa) + (b.val - Xb) by ring]
    exact le_trans (abs_add_le _ _) (add_le_add ha.err hb.err)
  have s : |a.val + b.val| ≤ (1 + K * ε) * Ma + (1 + K * ε) * Mb := le_trans (abs_add_le _ _) (add_le_add a1 b1)
  have := abs_sub_le (add a b).val (a.val + b.val) (Xa + Xb)
  have k1 : K * ε ≤ 1 / 10 ^ 12 := by nlinarith
  have : ε * |a.val + b.val| ≤ ε * ((1 + K * ε) * Ma + (1 + K * ε) * Mb) := mul_le_mul_of_nonneg_left s (by norm_num)
  nlinarith

lemma tr_sub {a b : F64} {Xa Ma Xb Mb K : ℚ} (ha : Tr a Xa Ma K) (hb : Tr b Xb Mb K)
    (hK : 0 ≤ K) (hK' : K ≤ 1000) (hf : (sub a b).isFinite) :
    Tr (sub a b) (Xa - Xb) (Ma + Mb) (K + 2) := by
  have hbn : Tr b.neg (-Xb) Mb K :=
    ⟨by simpa using hb.fin, by rw [FP.val_neg, show -b.val - -Xb = -(b.val - Xb) by ring, abs_neg]; exact hb.err,
      by rw [abs_neg]; exact hb.mag⟩
  have := tr_add ha hbn hK hK' hf
  rwa [← sub_eq_add_neg] at this

/-- A tracked value of a nonnegative quantity (`X = M`) is at least `(1 − kε)·M`. -/
lemma Tr.lower {v : F64} {M k : ℚ} (h : Tr v M M k) : (1 - k * ε) * M ≤ v.val := by
  have := (_root_.abs_le.mp h.err).1; nlinarith

end IEEE754.Screen
