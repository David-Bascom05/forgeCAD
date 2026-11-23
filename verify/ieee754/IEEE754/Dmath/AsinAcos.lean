/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.Atan2
import IEEE754.Sqrt

/-!
# `dmath::asin`, `acos`: the contract (C3)

`asin_err`, `acos_err`: for finite `x ∈ [−1, 1]`, `dmath::asin(x)` and `dmath::acos(x)` are finite
and within `2^-50` of `arcsin x`, `arccos x`. `sqrtArg_err`: `√((1 − x)(1 + x))` in floating point
is within `2.78·10^-16` (relative) of `√(1 − x²)`; the rest is `atan2_err` and a perturbation of
the angle.
-/

namespace IEEE754.Dmath

open FP Real

/-- `((1.0 - x) * (1.0 + x)).sqrt()`. -/
def sqrtArg (x : F64) : F64 := FP.sqrt (mul (sub ONE x) (add ONE x))

/-- `dmath::asin`: NaN outside `[−1, 1]`. -/
def asin (x : F64) : F64 :=
  if x.isFinite ∧ -1 ≤ x.val ∧ x.val ≤ 1 then atan2 x (sqrtArg x) else .nan

/-- `dmath::acos`: NaN outside `[−1, 1]`. -/
def acos (x : F64) : F64 :=
  if x.isFinite ∧ -1 ≤ x.val ∧ x.val ≤ 1 then atan2 (sqrtArg x) x else .nan

/-! ## The square root's argument -/

lemma mult_pos_ge {v : ℚ} {k : ℤ} (hv : Mult v k) (h : 0 < v) : 2 ^ k ≤ v := by
  by_contra hc; rw [not_le] at hc
  have := mult_small hv (by rw [abs_of_pos h]; exact hc); linarith

/-- Near `1`, `1 − x` is `0` or at least `2^-53`. -/
lemma one_sub_grid {x : F64} (hx : x.isFinite) (h : 1 / 2 ≤ x.val) (h1 : x.val < 1) :
    2 ^ (-53 : ℤ) ≤ 1 - x.val := by
  obtain ⟨e, _, hm, hlt⟩ := mult_val hx
  simp only [binary64_prec] at hlt
  have he : -53 ≤ e := by
    by_contra hc; rw [not_le] at hc
    have : (2 : ℚ) ^ (53 : ℕ) * 2 ^ e ≤ 1 / 2 := by
      rw [← zpow_natCast, ← two_zpow_add]
      exact le_trans (two_zpow_le (show ((53 : ℕ) : ℤ) + e ≤ -1 by omega)) (by norm_num)
    rw [abs_of_pos (by linarith)] at hlt; linarith
  have h1m : Mult 1 (-53) := by have := mult_pow (k := 0) (j := -53) (by norm_num); simpa using this
  exact mult_pos_ge (mult_sub h1m (mult_of_le hm he)) (by linarith)

lemma rel_or_exact_sub {x : F64} (hx : x.isFinite) (h0 : -1 ≤ x.val) (h1 : x.val ≤ 1) :
    (sub ONE x).isFinite ∧ |(sub ONE x).val - (1 - x.val)| ≤ 2 ^ (-53 : ℤ) * (1 - x.val) := by
  have fO : ONE.isFinite := by decide
  by_cases hh : 1 / 2 ≤ x.val
  · obtain ⟨f, v⟩ := sub_of_rep fO hx (rep_sub fO hx (by rw [ONE_val]; linarith) (by rw [ONE_val]; linarith))
    rw [ONE_val] at v
    refine ⟨f, ?_⟩; rw [v, _root_.sub_self, abs_zero]; exact mul_nonneg (two_zpow_pos _).le (by linarith)
  · rw [not_le] at hh
    have f := sub_fin_le fO hx (by rw [ONE_val, abs_le]; constructor <;> linarith)
    refine ⟨f, ?_⟩
    have hn : (2 : ℚ) ^ (-1022 : ℤ) ≤ |ONE.val - x.val| := by
      rw [ONE_val, abs_of_pos (by linarith)]
      exact le_trans (two_zpow_le (show (-1022 : ℤ) ≤ -1 by norm_num)) (by norm_num; linarith)
    have := F64.sub_rel_err fO hx f hn
    rw [ONE_val, abs_of_pos (show (0 : ℚ) < 1 - x.val by linarith)] at this; exact this

lemma rel_or_exact_add {x : F64} (hx : x.isFinite) (h0 : -1 ≤ x.val) (h1 : x.val ≤ 1) :
    (add ONE x).isFinite ∧ |(add ONE x).val - (1 + x.val)| ≤ 2 ^ (-53 : ℤ) * (1 + x.val) := by
  have fO : ONE.isFinite := by decide
  have hn : x.neg.isFinite := by simpa using hx
  have e : add ONE x = sub ONE x.neg := by simp [FP.sub, FP.neg_neg]
  rw [e]
  have := rel_or_exact_sub hn (by rw [FP.val_neg]; linarith) (by rw [FP.val_neg]; linarith)
  rwa [FP.val_neg, sub_neg_eq_add] at this

lemma finalize_sign (n : ℕ) (e : ℤ) : (finalize binary64 false n e).sign = false := by
  unfold finalize; dsimp only; split_ifs <;> rfl

lemma round_sign_false {r : ℚ} (h : 0 ≤ r) : (round binary64 false r).sign = false := by
  unfold round
  split_ifs with h0 h1
  · rfl
  · exact finalize_sign _ _
  · exfalso; exact h1 (lt_of_le_of_ne h (Ne.symm h0))

lemma sqrt_sign {p : F64} (hs : p.sign = false) : (FP.sqrt p).sign = false := by
  cases p with
  | zero s => simp_all [FP.sqrt, FP.sign]
  | inf s => cases s <;> simp_all [FP.sqrt, FP.sign]
  | nan => rfl
  | fin s m e h =>
    cases s
    · simp only [FP.sqrt]; exact finalize_sign _ _
    · simp [FP.sign] at hs

/-- The product `(1 − x)(1 + x)`: finite, `+`-signed, within `3.331·10^-16` (relative), and either
zero or at least `2^-55`. -/
lemma prod_err {x : F64} (hx : x.isFinite) (h0 : -1 ≤ x.val) (h1 : x.val ≤ 1) :
    (mul (sub ONE x) (add ONE x)).isFinite ∧ (mul (sub ONE x) (add ONE x)).sign = false ∧
      0 ≤ (mul (sub ONE x) (add ONE x)).val ∧
      ((mul (sub ONE x) (add ONE x)).val = 0 ∨ 1 / 2 ^ 56 ≤ (mul (sub ONE x) (add ONE x)).val) ∧
      |(mul (sub ONE x) (add ONE x)).val - (1 - x.val) * (1 + x.val)| ≤
        3331 / 10 ^ 19 * ((1 - x.val) * (1 + x.val)) := by
  have fO : ONE.isFinite := by decide
  obtain ⟨fa, ea⟩ := rel_or_exact_sub hx h0 h1
  obtain ⟨fb, eb⟩ := rel_or_exact_add hx h0 h1
  set a := sub ONE x
  set b := add ONE x
  have e53 : (2 : ℚ) ^ (-53 : ℤ) = 1 / 2 ^ 53 := by norm_num
  rw [e53] at ea eb
  have hA : 0 ≤ 1 - x.val := by linarith
  have hB : 0 ≤ 1 + x.val := by linarith
  have ha := abs_le.mp ea
  have hb := abs_le.mp eb
  have ha0 : 0 ≤ a.val := by nlinarith
  have hb0 : 0 ≤ b.val := by nlinarith
  have hab2 : a.val * b.val ≤ 5 := by nlinarith
  have sa : a.sign = false := by
    show (sub ONE x).sign = false
    rw [sub_eq_round fO hx]; exact round_sign_false (by rw [ONE_val]; linarith)
  have sb : b.sign = false := by
    show (add ONE x).sign = false
    rw [add_eq_round fO hx]; exact round_sign_false (by rw [ONE_val]; linarith)
  have hpr : mul a b = round binary64 false (a.val * b.val) := by
    rw [mul_eq_round fa fb, sa, sb]; rfl
  have fp : (mul a b).isFinite := by rw [hpr]; exact fin_of_le_one (by rw [abs_of_nonneg (mul_nonneg ha0 hb0)]; linarith)
  have sp : (mul a b).sign = false := by rw [hpr]; exact round_sign_false (mul_nonneg ha0 hb0)
  -- `ab = 0` or `ab ≥ 2^-54`
  have hcase : (1 - x.val) * (1 + x.val) = 0 ∨ 1 / 2 ^ 54 ≤ (1 - x.val) * (1 + x.val) := by
    rcases lt_or_ge x.val (1 / 2) with hl | hl
    · rcases lt_or_ge x.val (-1 / 2) with hl' | hl'
      · rcases eq_or_lt_of_le h0 with he | he
        · left; rw [← he]; ring
        · right
          have hn : x.neg.isFinite := by simpa using hx
          have := one_sub_grid hn (by rw [FP.val_neg]; linarith) (by rw [FP.val_neg]; linarith)
          rw [FP.val_neg, sub_neg_eq_add] at this
          rw [e53] at this
          have := mul_le_mul this (show (1 : ℚ) / 2 ≤ 1 - x.val by linarith) (by norm_num) (by linarith)
          have e : (1 : ℚ) / 2 ^ 54 = 1 / 2 ^ 53 * (1 / 2) := by norm_num
          rw [e]; linarith
      · right
        have e : (1 : ℚ) / 2 ^ 54 ≤ 1 / 2 * (1 / 2) := by norm_num
        have := mul_le_mul (show (1 : ℚ) / 2 ≤ 1 - x.val by linarith) (show (1 : ℚ) / 2 ≤ 1 + x.val by linarith)
          (by norm_num) (by linarith)
        linarith
    · rcases eq_or_lt_of_le h1 with he | he
      · left; rw [he]; ring
      · right
        have := one_sub_grid hx hl he
        rw [e53] at this
        have := mul_le_mul this (show (1 : ℚ) / 2 ≤ 1 + x.val by linarith) (by norm_num) (by linarith)
        have e : (1 : ℚ) / 2 ^ 54 = 1 / 2 ^ 53 * (1 / 2) := by norm_num
        rw [e]; linarith
  -- the computed `ab` against `(1 − x)(1 + x)`
  have hab : |a.val * b.val - (1 - x.val) * (1 + x.val)| ≤
      (2 / 2 ^ 53 + 1 / 2 ^ 106) * ((1 - x.val) * (1 + x.val)) := by
    have e : a.val * b.val - (1 - x.val) * (1 + x.val) =
        (a.val - (1 - x.val)) * (1 + x.val) + (1 - x.val) * (b.val - (1 + x.val)) +
          (a.val - (1 - x.val)) * (b.val - (1 + x.val)) := by ring
    rw [e, abs_le]; constructor <;> nlinarith [mul_nonneg hA hB]
  refine ⟨fp, sp, ?_, ?_, ?_⟩
  · rw [hpr]; exact round_val_nonneg binary64 (mul_nonneg ha0 hb0)
  · rcases hcase with hz | hz
    · -- an exact zero
      left
      have : a.val * b.val = 0 := by
        have := abs_le.mp hab; rw [hz] at this; nlinarith
      rw [hpr, this, round_zero]; rfl
    · right
      have hP : 0 ≤ (1 - x.val) * (1 + x.val) := mul_nonneg hA hB
      have hgt : 1 / 2 ^ 55 ≤ a.val * b.val := by
        have := (abs_le.mp hab).1
        have n : (1 : ℚ) / 2 ^ 55 ≤ (1 - (2 / 2 ^ 53 + 1 / 2 ^ 106)) * (1 / 2 ^ 54) := by norm_num
        nlinarith
      have hn : (2 : ℚ) ^ (-1022 : ℤ) ≤ |a.val * b.val| := by
        rw [abs_of_nonneg (mul_nonneg ha0 hb0)]
        have : (2 : ℚ) ^ (-1022 : ℤ) ≤ 1 / 2 ^ 55 :=
          le_trans (two_zpow_le (show (-1022 : ℤ) ≤ -55 by norm_num)) (by norm_num)
        linarith
      have er := F64.mul_rel_err fa fb fp hn
      rw [e53, abs_of_nonneg (mul_nonneg ha0 hb0)] at er
      have := (abs_le.mp er).1
      have n : (1 : ℚ) / 2 ^ 56 ≤ (1 - 1 / 2 ^ 53) * (1 / 2 ^ 55) := by norm_num
      nlinarith
  · rcases hcase with hz | hz
    · have : a.val * b.val = 0 := by
        have := abs_le.mp hab; rw [hz] at this; nlinarith
      rw [hpr, this, round_zero, hz]; simp [FP.val]
    · have hP : 0 ≤ (1 - x.val) * (1 + x.val) := mul_nonneg hA hB
      have hn : (2 : ℚ) ^ (-1022 : ℤ) ≤ |a.val * b.val| := by
        rw [abs_of_nonneg (mul_nonneg ha0 hb0)]
        have := (abs_le.mp hab).1
        have e : (2 : ℚ) ^ (-1022 : ℤ) ≤ (1 - (2 / 2 ^ 53 + 1 / 2 ^ 106)) * (1 / 2 ^ 54) :=
          le_trans (two_zpow_le (show (-1022 : ℤ) ≤ -55 by norm_num)) (by norm_num)
        nlinarith
      have er := F64.mul_rel_err fa fb fp hn
      rw [e53, abs_of_nonneg (mul_nonneg ha0 hb0)] at er
      have := abs_sub_le (mul a b).val (a.val * b.val) ((1 - x.val) * (1 + x.val))
      have h3 : a.val * b.val ≤ (1 + (2 / 2 ^ 53 + 1 / 2 ^ 106)) * ((1 - x.val) * (1 + x.val)) := by
        have := (abs_le.mp hab).2; linarith
      have n : (1 / 2 ^ 53 : ℚ) * (1 + (2 / 2 ^ 53 + 1 / 2 ^ 106)) + (2 / 2 ^ 53 + 1 / 2 ^ 106) ≤ 3331 / 10 ^ 19 := by
        norm_num
      nlinarith

set_option maxRecDepth 20000 in
lemma round_fin_real {zs : Bool} {r : ℝ} (h : |r| ≤ 16) : (round binary64 zs r).isFinite := by
  by_cases hr : r = 0
  · subst hr; simp [round, isFinite]
  rw [round_finite_iff (K := ℝ) binary64 hr]
  have h16 : (16 : ℝ) = ((2 ^ 52 : ℕ) : ℝ) * 2 ^ (-48 : ℤ) := by norm_num
  have hex : uround binary64 (16 : ℝ) = 16 := by
    rw [h16, uround_exact binary64 (by positivity) (by simp) (by simp)]
  have hm := uround_mono binary64 (abs_pos.mpr hr) h
  rw [hex] at hm
  have : (16 : ℝ) < 2 ^ (binary64.emax + 1) := by
    simp only [binary64_emax]
    exact lt_of_lt_of_le (by norm_num : (16 : ℝ) < 2 ^ (5 : ℤ)) (two_zpow_le (by norm_num))
  linarith

/-- `√p` against `√P` for `p` within `T·P` of `P`. -/
lemma sqrt_pert {p P T : ℝ} (hP : 0 ≤ P) (hp : 0 ≤ p) (hT0 : 0 ≤ T) (hT1 : T ≤ 1 / 2)
    (h : |p - P| ≤ T * P) : |√p - √P| ≤ T / (2 - T) * √P := by
  rcases eq_or_lt_of_le hP with h0 | h0
  · rw [← h0] at h ⊢
    have h' : |p| ≤ 0 := by simpa using h
    have : p = 0 := abs_nonpos_iff.mp h'
    simp [this]
  have hsP : 0 < √P := Real.sqrt_pos.mpr h0
  have hpl : P * (1 - T) ≤ p := by have := (abs_le.mp h).1; linarith
  have h3 : (0 : ℝ) ≤ 1 - T := by linarith
  have h4 : 1 - T ≤ 1 := by linarith
  have hsq : 1 - T ≤ √(1 - T) := by
    calc 1 - T = √((1 - T) ^ 2) := (Real.sqrt_sq h3).symm
      _ ≤ √(1 - T) := Real.sqrt_le_sqrt (by nlinarith)
  have hsp : √P * (1 - T) ≤ √p := by
    have h1 : √(P * (1 - T)) ≤ √p := Real.sqrt_le_sqrt hpl
    have h2 : √P * (1 - T) ≤ √(P * (1 - T)) := by
      rw [Real.sqrt_mul hP]; exact mul_le_mul_of_nonneg_left hsq hsP.le
    linarith
  have hsum : √P * (2 - T) ≤ √p + √P := by linarith
  have hprod : |√p - √P| * (√p + √P) = |p - P| := by
    rw [← abs_of_pos (show 0 < √p + √P by nlinarith), ← abs_mul,
      show (√p - √P) * (√p + √P) = √p ^ 2 - √P ^ 2 by ring, Real.sq_sqrt hp, Real.sq_sqrt hP]
  have hP' : P = √P * √P := (Real.mul_self_sqrt hP).symm
  rw [div_mul_eq_mul_div, le_div_iff₀ (by linarith)]
  have h5 : |√p - √P| * (√P * (2 - T)) ≤ |√p - √P| * (√p + √P) :=
    mul_le_mul_of_nonneg_left hsum (abs_nonneg _)
  have key : |√p - √P| * (2 - T) * √P ≤ T * √P * √P := by
    have : T * P = T * √P * √P := by rw [mul_assoc, Real.mul_self_sqrt hP]
    nlinarith
  exact le_of_mul_le_mul_right key hsP

lemma sqrt_of_val_zero {p : F64} (hp : p.isFinite) (h : p.val = 0) :
    (FP.sqrt p).isFinite ∧ (FP.sqrt p).val = 0 := by
  cases p with
  | zero s => simp [FP.sqrt, FP.isFinite, FP.val]
  | fin s m e hc => exact absurd h (val_ne_zero hp rfl)
  | _ => simp_all [FP.isFinite]

lemma sqrt_err_pos {p : F64} (hp : p.isFinite) (sp : p.sign = false) (hlo : 1 / 2 ^ 56 ≤ p.val)
    (hhi : p.val ≤ 5) :
    (FP.sqrt p).isFinite ∧ 0 ≤ (FP.sqrt p).val ∧
      |((FP.sqrt p).val : ℝ) - √(p.val : ℝ)| ≤ 1 / 2 ^ 53 * √(p.val : ℝ) := by
  cases p with
  | fin s m e hc =>
    cases s
    · have hpr : (0 : ℝ) ≤ ((FP.fin false m e hc : F64).val : ℝ) := by
        have : (0 : ℚ) ≤ (FP.fin false m e hc : F64).val := le_trans (by positivity) hlo
        exact_mod_cast this
      have hs3 : √(((FP.fin false m e hc : F64).val : ℚ) : ℝ) ≤ 3 := by
        rw [Real.sqrt_le_left (by norm_num)]
        have : ((FP.fin false m e hc : F64).val : ℝ) ≤ 5 := by exact_mod_cast hhi
        linarith
      have hfin : (FP.fin false m e hc : F64).sqrt.isFinite := by
        rw [sqrt_eq_round]; exact round_fin_real (by rw [abs_of_nonneg (Real.sqrt_nonneg _)]; linarith)
      have hn : (2 : ℝ) ^ binary64.emin ≤ √(((FP.fin false m e hc : F64).val : ℚ) : ℝ) := by
        simp only [binary64_emin]
        have h1 : (2 : ℝ) ^ (-1022 : ℤ) ≤ 1 / 2 ^ 28 :=
          le_trans (two_zpow_le (show (-1022 : ℤ) ≤ -28 by norm_num)) (by norm_num)
        have h2 : (1 : ℝ) / 2 ^ 28 ≤ √(((FP.fin false m e hc : F64).val : ℚ) : ℝ) := by
          rw [Real.le_sqrt (by positivity) hpr]
          have : ((1 / 2 ^ 56 : ℚ) : ℝ) ≤ ((FP.fin false m e hc : F64).val : ℝ) := by exact_mod_cast hlo
          push_cast at this; norm_num at this ⊢; linarith
        linarith
      have e := sqrt_rel_err (f := binary64) hc hfin hn
      simp only [binary64_prec] at e
      rw [show (2 : ℝ) ^ (-((53 : ℕ) : ℤ)) = 1 / 2 ^ 53 by norm_num] at e
      refine ⟨hfin, ?_, e⟩
      rw [sqrt_eq_round]; exact round_val_nonneg binary64 (Real.sqrt_nonneg _)
    · simp [FP.sign] at sp
  | zero s => simp [FP.val] at hlo; linarith [show (0 : ℚ) < 1 / 2 ^ 56 by positivity]
  | _ => simp_all [FP.isFinite]

/-- **`√((1 − x)(1 + x))` in floating point.** Within `2.78·10^-16` (relative) of `√(1 − x²)`,
`+`-signed and nonnegative. -/
theorem sqrtArg_err {x : F64} (hx : x.isFinite) (h0 : -1 ≤ x.val) (h1 : x.val ≤ 1) :
    (sqrtArg x).isFinite ∧ (sqrtArg x).sign = false ∧ 0 ≤ (sqrtArg x).val ∧
      |((sqrtArg x).val : ℝ) - √(1 - (x.val : ℝ) ^ 2)| ≤ 278 / 10 ^ 18 * √(1 - (x.val : ℝ) ^ 2) := by
  obtain ⟨fp, sp, p0, pcase, ep⟩ := prod_err hx h0 h1
  unfold sqrtArg
  set p := mul (sub ONE x) (add ONE x)
  have hPq : 0 ≤ (1 - x.val) * (1 + x.val) := mul_nonneg (by linarith) (by linarith)
  have hP : ((((1 - x.val) * (1 + x.val) : ℚ) : ℝ)) = 1 - (x.val : ℝ) ^ 2 := by push_cast; ring
  have ss := sqrt_sign sp
  rcases pcase with hz | hpos
  · obtain ⟨f, v⟩ := sqrt_of_val_zero fp hz
    have hP0 : (1 - x.val) * (1 + x.val) = 0 := by
      rw [hz] at ep
      have := abs_le.mp ep
      nlinarith
    have hR : 1 - (x.val : ℝ) ^ 2 = 0 := by rw [← hP, hP0]; simp
    refine ⟨f, ss, by rw [v], ?_⟩
    rw [v, hR]; simp
  · have hhi : p.val ≤ 5 := by
      have := (abs_le.mp ep).2
      have : (1 - x.val) * (1 + x.val) ≤ 1 := by nlinarith [sq_nonneg x.val]
      linarith
    obtain ⟨f, nn, e⟩ := sqrt_err_pos fp sp hpos hhi
    refine ⟨f, ss, nn, ?_⟩
    have epr := qcast ep
    push_cast at epr
    have hPr : (0 : ℝ) ≤ 1 - (x.val : ℝ) ^ 2 := by rw [← hP]; exact_mod_cast hPq
    have epr' : |(p.val : ℝ) - (1 - (x.val : ℝ) ^ 2)| ≤ 3331 / 10 ^ 19 * (1 - (x.val : ℝ) ^ 2) := by
      have e2 : (1 - (x.val : ℝ)) * (1 + x.val) = 1 - (x.val : ℝ) ^ 2 := by ring
      rw [e2] at epr; exact epr
    have hp0 : (0 : ℝ) ≤ p.val := by exact_mod_cast p0
    have sq := sqrt_pert hPr hp0 (by norm_num) (by norm_num) epr'
    set S := √(1 - (x.val : ℝ) ^ 2)
    have hS0 : 0 ≤ S := Real.sqrt_nonneg _
    have hsq := abs_le.mp sq
    have := abs_sub_le (((FP.sqrt p).val : ℚ) : ℝ) √(p.val : ℝ) S
    have hsp : √(p.val : ℝ) ≤ S + 3331 / 10 ^ 19 / (2 - 3331 / 10 ^ 19) * S := by linarith
    have h5 : 1 / 2 ^ 53 * √(p.val : ℝ) ≤ 1 / 2 ^ 53 * (S + 3331 / 10 ^ 19 / (2 - 3331 / 10 ^ 19) * S) :=
      mul_le_mul_of_nonneg_left hsp (by norm_num)
    have n : (1 / 2 ^ 53 : ℝ) * (1 + 3331 / 10 ^ 19 / (2 - 3331 / 10 ^ 19)) + 3331 / 10 ^ 19 / (2 - 3331 / 10 ^ 19) ≤
        278 / 10 ^ 18 := by norm_num
    nlinarith

/-! ## Perturbing the angle -/

/-- A relative perturbation `c` of `Y ≥ 0` moves `atan2R Y X` by at most `c/(2(1−c))`. -/
lemma pert_y {X Y Y' c : ℝ} (hY : 0 ≤ Y) (hY' : 0 ≤ Y') (hc0 : 0 ≤ c) (hc1 : c ≤ 1 / 2)
    (h : |Y' - Y| ≤ c * Y) : |atan2R Y' X - atan2R Y X| ≤ c / (2 * (1 - c)) := by
  have hb0 : 0 ≤ c / (2 * (1 - c)) := div_nonneg hc0 (by linarith)
  unfold atan2R
  rcases lt_trichotomy X 0 with hX | hX | hX
  · rw [if_neg (not_lt.mpr hX.le), if_neg (not_lt.mpr hX.le), if_pos hX, if_pos hX, if_pos hY', if_pos hY]
    have hnX : 0 < -X := by linarith
    have e1 : Y' / X = -(Y' / -X) := by rw [div_neg, _root_.neg_neg]
    have e2 : Y / X = -(Y / -X) := by rw [div_neg, _root_.neg_neg]
    rw [e1, e2, arctan_neg, arctan_neg,
      show -arctan (Y' / -X) + π - (-arctan (Y / -X) + π) = -(arctan (Y' / -X) - arctan (Y / -X)) by ring, abs_neg]
    apply arctan_rel_gen (div_nonneg hY hnX.le) hc0 hc1
    rw [div_sub_div_same, abs_div, abs_of_pos hnX, div_le_iff₀ hnX]
    calc |Y' - Y| ≤ c * Y := h
      _ = Y / -X * c * -X := by rw [mul_right_comm, div_mul_cancel₀ Y hnX.ne', _root_.mul_comm]
  · subst hX
    rw [if_neg (lt_irrefl 0), if_neg (lt_irrefl 0), if_neg (lt_irrefl 0), if_neg (lt_irrefl 0)]
    rcases eq_or_lt_of_le hY with h0 | h0
    · rw [← h0] at h ⊢
      have : Y' = 0 := by
        have h' : |Y'| ≤ 0 := by simpa using h
        exact abs_nonpos_iff.mp h'
      rw [this]; simp; exact hb0
    · have hYl : Y * (1 - c) ≤ Y' := by have := (abs_le.mp h).1; nlinarith
      have : 0 < Y' := lt_of_lt_of_le (by nlinarith) hYl
      rw [if_pos this, if_pos h0]; simp; exact hb0
  · rw [if_pos hX, if_pos hX]
    apply arctan_rel_gen (div_nonneg hY hX.le) hc0 hc1
    rw [div_sub_div_same, abs_div, abs_of_pos hX, div_le_iff₀ hX]
    have hXne : X ≠ 0 := hX.ne'
    calc |Y' - Y| ≤ c * Y := h
      _ = Y / X * c * X := by field_simp

/-- A relative perturbation `c` of `X > 0` moves `atan2R Y X` by at most `c'/(2(1−c'))`,
`c' = c/(1−c)`. -/
lemma pert_x {X X' Y c : ℝ} (hX : 0 < X) (hX' : 0 < X') (hc0 : 0 ≤ c) (hc1 : c ≤ 1 / 4)
    (h : |X' - X| ≤ c * X) :
    |atan2R Y X' - atan2R Y X| ≤ c / (1 - c) / (2 * (1 - c / (1 - c))) := by
  unfold atan2R
  rw [if_pos hX', if_pos hX]
  have hc' : c / (1 - c) ≤ 1 / 2 := by rw [div_le_iff₀ (by linarith)]; linarith
  have hc'0 : 0 ≤ c / (1 - c) := div_nonneg hc0 (by linarith)
  have hXl : X * (1 - c) ≤ X' := by have := (abs_le.mp h).1; nlinarith
  have key : |(|Y| / X') - |Y| / X| ≤ |Y| / X * (c / (1 - c)) := by
    rw [div_sub_div _ _ hX'.ne' hX.ne', abs_div, abs_of_pos (mul_pos hX' hX),
      show |Y| * X - X' * |Y| = |Y| * (X - X') by ring, abs_mul, abs_abs, abs_sub_comm]
    rw [div_le_iff₀ (mul_pos hX' hX)]
    have h1 : |Y| * |X' - X| ≤ |Y| * (c * X) := mul_le_mul_of_nonneg_left h (abs_nonneg _)
    have h2 : |Y| / X * (c / (1 - c)) * (X' * X) = |Y| * c * X * (X' / (X * (1 - c))) := by
      field_simp
    rw [h2]
    have h3 : 1 ≤ X' / (X * (1 - c)) := by rw [le_div_iff₀ (by nlinarith)]; linarith
    have h4 : 0 ≤ |Y| * c * X := by positivity
    nlinarith
  have g := arctan_rel_gen (d := |Y| / X') (u := |Y| / X) (div_nonneg (abs_nonneg _) hX.le) hc'0 hc' key
  rcases le_or_gt 0 Y with hy | hy
  · rw [abs_of_nonneg hy] at g; exact g
  · rw [abs_of_neg hy] at g
    rw [show Y / X' = -(-Y / X') by ring, show Y / X = -(-Y / X) by ring, arctan_neg, arctan_neg,
      show -arctan (-Y / X') - -arctan (-Y / X) = -(arctan (-Y / X') - arctan (-Y / X)) by ring, abs_neg]
    exact g

/-! ## `arcsin`, `arccos` as angles -/

lemma arccos_eq_atan2R {x : ℝ} (h0 : -1 ≤ x) (h1 : x ≤ 1) : arccos x = atan2R (√(1 - x ^ 2)) x := by
  unfold atan2R
  rcases lt_trichotomy x 0 with hx | hx | hx
  · rw [if_neg (not_lt.mpr hx.le), if_pos hx, if_pos (Real.sqrt_nonneg _)]
    have hnx : 0 < -x := by linarith
    have := arccos_eq_arctan hnx
    rw [neg_sq] at this
    have e : arccos x = π - arccos (-x) := by rw [← arccos_neg, _root_.neg_neg]
    rw [e, this, show √(1 - x ^ 2) / -x = -(√(1 - x ^ 2) / x) by rw [div_neg], arctan_neg]; ring
  · subst hx
    rw [if_neg (lt_irrefl 0), if_neg (lt_irrefl 0)]
    simp
  · rw [if_pos hx]; exact arccos_eq_arctan hx

lemma arcsin_eq_atan2R {x : ℝ} (h0 : -1 ≤ x) (h1 : x ≤ 1) : arcsin x = atan2R x (√(1 - x ^ 2)) := by
  unfold atan2R
  rcases eq_or_lt_of_le (Real.sqrt_nonneg (1 - x ^ 2)) with hs | hs
  · rw [← hs, if_neg (lt_irrefl 0), if_neg (lt_irrefl 0)]
    have h2 : 1 - x ^ 2 ≤ 0 := Real.sqrt_eq_zero'.mp hs.symm
    have hx2 : x ^ 2 = 1 := by nlinarith
    rcases lt_trichotomy x 0 with hx | hx | hx
    · have : x = -1 := by nlinarith
      rw [if_neg (not_lt.mpr hx.le), if_pos hx, this, arcsin_neg_one]
    · subst hx; norm_num at hx2
    · have : x = 1 := by nlinarith
      rw [if_pos hx, this, arcsin_one]
  · rw [if_pos hs]
    have h2 : 0 < 1 - x ^ 2 := Real.sqrt_pos.mp hs
    exact arcsin_eq_arctan ⟨by nlinarith, by nlinarith⟩

/-! ## The contracts -/

/-- **Contract C3, `acos`.** For finite `x ∈ [−1, 1]`, `dmath::acos(x)` is finite and within
`2^-50` of `arccos x`. -/
theorem acos_err {x : F64} (hx : x.isFinite) (h0 : -1 ≤ x.val) (h1 : x.val ≤ 1) :
    (acos x).isFinite ∧ |((acos x).val : ℝ) - arccos x.val| ≤ 1 / 2 ^ 50 := by
  unfold acos
  rw [if_pos ⟨hx, h0, h1⟩]
  obtain ⟨fs, ss, s0, es⟩ := sqrtArg_err hx h0 h1
  set s := sqrtArg x
  have hr0 : (-1 : ℝ) ≤ x.val := by exact_mod_cast h0
  have hr1 : (x.val : ℝ) ≤ 1 := by exact_mod_cast h1
  have hne : x.val ≠ 0 ∨ s.val ≠ 0 := by
    by_cases hx0 : x.val = 0
    · right
      have e1 : |(s.val : ℝ) - 1| ≤ 278 / 10 ^ 18 := by
        have := es; rw [hx0] at this; simpa using this
      intro hs0; rw [hs0] at e1; norm_num at e1
    · left; exact hx0
  obtain ⟨fa, ea⟩ := atan2_err hx fs hne (fun _ => Or.inl ss)
  refine ⟨fa, ?_⟩
  have ea' : |((atan2 s x).val : ℝ) - atan2R s.val x.val| ≤ 69 / 10 ^ 17 :=
    le_trans ea (by split_ifs <;> norm_num)
  have hS0 : 0 ≤ √(1 - (x.val : ℝ) ^ 2) := Real.sqrt_nonneg _
  have hp := pert_y (X := (x.val : ℝ)) hS0 (by exact_mod_cast s0) (by norm_num) (by norm_num) es
  rw [arccos_eq_atan2R hr0 hr1]
  have := abs_sub_le ((atan2 s x).val : ℝ) (atan2R s.val x.val) (atan2R (√(1 - (x.val : ℝ) ^ 2)) x.val)
  have n : (69 : ℝ) / 10 ^ 17 + 278 / 10 ^ 18 / (2 * (1 - 278 / 10 ^ 18)) ≤ 1 / 2 ^ 50 := by norm_num
  linarith

/-- **Contract C3, `asin`.** For finite `x ∈ [−1, 1]`, `dmath::asin(x)` is finite and within
`2^-50` of `arcsin x`. -/
theorem asin_err {x : F64} (hx : x.isFinite) (h0 : -1 ≤ x.val) (h1 : x.val ≤ 1) :
    (asin x).isFinite ∧ |((asin x).val : ℝ) - arcsin x.val| ≤ 1 / 2 ^ 50 := by
  unfold asin
  rw [if_pos ⟨hx, h0, h1⟩]
  obtain ⟨fs, ss, s0, es⟩ := sqrtArg_err hx h0 h1
  set s := sqrtArg x
  have hr0 : (-1 : ℝ) ≤ x.val := by exact_mod_cast h0
  have hr1 : (x.val : ℝ) ≤ 1 := by exact_mod_cast h1
  set S := √(1 - (x.val : ℝ) ^ 2)
  have hS0 : 0 ≤ S := Real.sqrt_nonneg _
  rw [arcsin_eq_atan2R hr0 hr1]
  have hc : (278 : ℝ) / 10 ^ 18 < 1 := by norm_num
  rcases eq_or_lt_of_le s0 with hs | hs
  · -- `s = 0`, so `S = 0` and `x = ±1`
    have e0 : ((s.val : ℚ) : ℝ) = 0 := by rw [← hs]; simp
    have hS : S = 0 := by
      rw [e0, zero_sub, abs_neg, abs_of_nonneg hS0] at es
      nlinarith
    have hx0 : x.val ≠ 0 := by
      intro h
      have : S = 1 := by simp only [S, h]; simp
      rw [this] at hS; norm_num at hS
    obtain ⟨fa, ea⟩ := atan2_err fs hx (Or.inr hx0) (fun h => absurd h hx0)
    refine ⟨fa, ?_⟩
    rw [e0, ← hS] at ea
    exact le_trans ea (by split_ifs <;> norm_num)
  · have hsr : (0 : ℝ) < s.val := by exact_mod_cast hs
    have hSp : 0 < S := by
      rcases eq_or_lt_of_le hS0 with h | h
      · rw [← h] at es; simp at es; exfalso
        have : |((s.val : ℚ) : ℝ)| ≤ 0 := by simpa using es
        have := abs_nonpos_iff.mp this; linarith
      · exact h
    obtain ⟨fa, ea⟩ := atan2_err fs hx (Or.inl hs.ne') (fun _ => Or.inr ss)
    refine ⟨fa, ?_⟩
    rw [if_pos hsr] at ea
    have hp := pert_x (Y := (x.val : ℝ)) hSp hsr (by norm_num) (by norm_num) es
    have := abs_sub_le ((atan2 x s).val : ℝ) (atan2R x.val s.val) (atan2R x.val S)
    have n : (356 : ℝ) / 10 ^ 18 + 278 / 10 ^ 18 / (1 - 278 / 10 ^ 18) / (2 * (1 - 278 / 10 ^ 18 / (1 - 278 / 10 ^ 18)))
        ≤ 1 / 2 ^ 50 := by norm_num
    linarith

end IEEE754.Dmath

