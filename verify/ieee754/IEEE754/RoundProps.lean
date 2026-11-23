/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Round
import IEEE754.Ops

/-!
# Properties of `round`

* `round_rel_err`: outside the subnormal range, `|round r - r| ≤ 2^-prec · |r|`;
* `round_abs_err`: inside it, `|round r - r| ≤ 2^(qmin-1)` (half the least subnormal);
* `round_mono`: rounding is monotone, in the IEEE order of the results (overflow included);
* `round_val`: rounding is the identity on representable values;
* `round_exact`: any `k · 2^e` with `|k| ≤ 2^prec`, `e ≥ qmin` and no overflow rounds to itself.
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {K : Type*} [Field K] [LinearOrder K] [IsStrictOrderedRing K] [FloorRing K]
variable (f : Format)

lemma cast_lt_threshold {n : ℕ} {e : ℤ} :
    ((n : ℚ) * 2 ^ e < 2 ^ (f.emax + 1)) ↔ ((n : K) * 2 ^ e < 2 ^ (f.emax + 1)) := by
  rw [← Rat.cast_lt (K := K)]; push_cast; rfl

lemma roundPos_pre {r : K} (hr : 0 < r) :
    (roundPos f r).1 ≤ 2 ^ f.prec ∧ f.qmin ≤ (roundPos f r).2 ∧
      (2 ^ (f.prec - 1) ≤ (roundPos f r).1 ∨ (roundPos f r).2 = f.qmin) := by
  refine ⟨roundPos_le f hr, roundPos_ge_qmin f r, ?_⟩
  rcases eq_or_lt_of_le (roundPos_ge_qmin f r) with h | h
  · exact Or.inr h.symm
  · exact Or.inl (roundPos_normal f hr h).1

lemma val_cast_finalize {s : Bool} {n : ℕ} {e : ℤ} (h : (finalize f s n e).val = sgn s * ((n : ℚ) * 2 ^ e)) :
    ((finalize f s n e).val : K) = sgn s * ((n : K) * 2 ^ e) := by
  rw [h]; cases s <;> simp [sgn]

/-- Rounding a positive number that does not overflow. -/
lemma round_pos {zs : Bool} {r : K} (hr : 0 < r) (hu : uround f r < 2 ^ (f.emax + 1)) :
    (round f zs r).isFinite ∧ ((round f zs r).val : K) = uround f r ∧ (round f zs r).sign = false := by
  have hround : round f zs r = finalize f false (roundPos f r).1 (roundPos f r).2 := by
    simp [round, hr.ne', hr]
  obtain ⟨h1, h2, h3⟩ := roundPos_pre f hr
  obtain ⟨a, b, c⟩ := finalize_of_lt f (s := false) h1 h2 h3 ((cast_lt_threshold f).mpr hu)
  rw [hround]
  refine ⟨a, ?_, c⟩
  rw [val_cast_finalize f b]; simp [sgn, uround]

lemma round_pos_ovf {zs : Bool} {r : K} (hr : 0 < r) (hu : 2 ^ (f.emax + 1) ≤ uround f r) :
    round f zs r = .inf false := by
  have hround : round f zs r = finalize f false (roundPos f r).1 (roundPos f r).2 := by
    simp [round, hr.ne', hr]
  rw [hround]
  apply finalize_of_ge f (roundPos_le f hr)
  rw [← Rat.cast_le (K := K)]; push_cast; exact hu

lemma round_neg' {zs : Bool} {r : K} (hr : r < 0) (hu : uround f (-r) < 2 ^ (f.emax + 1)) :
    (round f zs r).isFinite ∧ ((round f zs r).val : K) = -uround f (-r) ∧ (round f zs r).sign = true := by
  have hr' : 0 < -r := neg_pos.mpr hr
  have hround : round f zs r = finalize f true (roundPos f (-r)).1 (roundPos f (-r)).2 := by
    simp [round, hr.ne, not_lt.mpr hr.le]
  obtain ⟨h1, h2, h3⟩ := roundPos_pre f hr'
  obtain ⟨a, b, c⟩ := finalize_of_lt f (s := true) h1 h2 h3 ((cast_lt_threshold f).mpr hu)
  rw [hround]
  refine ⟨a, ?_, c⟩
  rw [val_cast_finalize f b]; simp [sgn, uround]

lemma round_neg_ovf {zs : Bool} {r : K} (hr : r < 0) (hu : 2 ^ (f.emax + 1) ≤ uround f (-r)) :
    round f zs r = .inf true := by
  have hr' : 0 < -r := neg_pos.mpr hr
  have hround : round f zs r = finalize f true (roundPos f (-r)).1 (roundPos f (-r)).2 := by
    simp [round, hr.ne, not_lt.mpr hr.le]
  rw [hround]
  apply finalize_of_ge f (roundPos_le f hr')
  rw [← Rat.cast_le (K := K)]; push_cast; exact hu

@[simp] lemma round_zero (zs : Bool) : round f zs (0 : K) = .zero zs := by simp [round]

/-- A finite rounding comes from a value that does not overflow. -/
lemma round_finite_iff {zs : Bool} {r : K} (hr : r ≠ 0) :
    (round f zs r).isFinite ↔ uround f |r| < 2 ^ (f.emax + 1) := by
  rcases lt_or_gt_of_ne hr with h | h
  · rw [abs_of_neg h]
    by_cases hu : uround f (-r) < 2 ^ (f.emax + 1)
    · simp [hu, (round_neg' f (zs := zs) h hu).1]
    · rw [round_neg_ovf f h (not_lt.mp hu)]; simp [hu, isFinite]
  · rw [abs_of_pos h]
    by_cases hu : uround f r < 2 ^ (f.emax + 1)
    · simp [hu, (round_pos f (zs := zs) h hu).1]
    · rw [round_pos_ovf f h (not_lt.mp hu)]; simp [hu, isFinite]

/-- The value of a finite rounding is `±uround |r|`. -/
lemma round_val_abs {zs : Bool} {r : K} (hr : r ≠ 0) (hfin : (round f zs r).isFinite) :
    |((round f zs r).val : K) - r| = abs (uround f |r| - |r|) := by
  have hu := (round_finite_iff f hr).mp hfin
  rcases lt_or_gt_of_ne hr with h | h
  · rw [abs_of_neg h] at hu ⊢
    rw [(round_neg' f h hu).2.1, ← abs_neg]; congr 1; ring
  · rw [abs_of_pos h] at hu ⊢
    rw [(round_pos f h hu).2.1]

/-- **Relative error.** Outside the subnormal range, a finite rounding is within `2^-prec`
(half an ulp, relative) of the exact value. -/
theorem round_rel_err {zs : Bool} {r : K} (hfin : (round f zs r).isFinite)
    (hr : 2 ^ f.emin ≤ |r|) : |((round f zs r).val : K) - r| ≤ 2 ^ (-(f.prec : ℤ)) * |r| := by
  have hr0 : r ≠ 0 := by
    intro h; rw [h, abs_zero] at hr; linarith [two_zpow_pos (K := K) f.emin]
  have ha : 0 < |r| := abs_pos.mpr hr0
  rw [round_val_abs f hr0 hfin]
  refine le_trans (uround_err f ha) ?_
  have hE := le_log_of_le ha hr
  have he : (roundPos f |r|).2 = Int.log 2 |r| - f.prec + 1 := by
    have := roundPos_snd f |r|; have := f.qmin_add_prec; omega
  rw [he, show Int.log 2 |r| - f.prec + 1 - 1 = -(f.prec : ℤ) + Int.log 2 |r| by ring, two_zpow_add]
  exact mul_le_mul_of_nonneg_left (log_spec ha).1 (two_zpow_pos _).le

/-- In the subnormal range rounding never overflows. -/
lemma uround_small {r : K} (hr0 : 0 < r) (hr : r < 2 ^ f.emin) :
    (roundPos f r).2 = f.qmin ∧ uround f r < 2 ^ (f.emax + 1) := by
  have hE := log_lt_of_lt hr0 hr
  have he : (roundPos f r).2 = f.qmin := by
    have := roundPos_snd f r; have := f.qmin_add_prec; omega
  refine ⟨he, ?_⟩
  have h1 : uround f r ≤ 2 ^ ((f.prec : ℤ) + f.qmin) := by
    simp only [uround]; rw [two_zpow_add]
    apply mul_le_mul _ (by rw [he]) (two_zpow_pos _).le (two_zpow_pos _).le
    rw [← cast_two_pow]; exact_mod_cast roundPos_le f hr0
  refine lt_of_le_of_lt h1 (two_zpow_lt ?_)
  have := f.qmin_add_prec; have := f.one_le_emax; unfold Format.emin at *; omega

/-- **Absolute error in the subnormal range**: at most half the least subnormal, and never an
overflow. -/
theorem round_abs_err {zs : Bool} {r : K} (hr : |r| < 2 ^ f.emin) :
    (round f zs r).isFinite ∧ |((round f zs r).val : K) - r| ≤ 2 ^ (f.qmin - 1) := by
  by_cases hr0 : r = 0
  · subst hr0; simp only [round_zero]; refine ⟨rfl, ?_⟩; simp only [val, Rat.cast_zero, sub_zero, abs_zero]
    exact (two_zpow_pos _).le
  have ha : 0 < |r| := abs_pos.mpr hr0
  obtain ⟨he, hu⟩ := uround_small f ha hr
  have hfin := (round_finite_iff f (zs := zs) hr0).mpr hu
  refine ⟨hfin, ?_⟩
  rw [round_val_abs f hr0 hfin, ← he]
  exact uround_err f ha

/-- Both bounds in one: `|round r - r| ≤ max (2^-prec · |r|) 2^(qmin-1)`. -/
theorem round_err {zs : Bool} {r : K} (hfin : (round f zs r).isFinite) :
    |((round f zs r).val : K) - r| ≤ max (2 ^ (-(f.prec : ℤ)) * |r|) (2 ^ (f.qmin - 1)) := by
  rcases lt_or_ge |r| (2 ^ f.emin) with h | h
  · exact le_trans (round_abs_err f (zs := zs) h).2 (le_max_right _ _)
  · exact le_trans (round_rel_err f hfin h) (le_max_left _ _)

/-! ## Monotonicity -/

lemma le_of_val {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (h : x.val ≤ y.val) :
    x.le y = true := by
  cases x <;> cases y <;> simp_all [isFinite, FP.le, FP.cmp, compare_lt_iff_lt,
    le_iff_lt_or_eq]

lemma le_inf_pos {x : FP f} (hx : x.isNaN = false) : x.le (.inf false) = true := by
  cases x <;> simp_all [isNaN, FP.le, FP.cmp]

lemma inf_neg_le {y : FP f} (hy : y.isNaN = false) : (FP.inf true).le y = true := by
  cases y <;> simp_all [isNaN, FP.le, FP.cmp]

lemma round_not_nan (zs : Bool) (r : K) : (round f zs r).isNaN = false := by
  by_cases h0 : r = 0
  · subst h0; simp [isNaN]
  rcases lt_or_gt_of_ne h0 with h | h
  · by_cases hu : uround f (-r) < 2 ^ (f.emax + 1)
    · have := (round_neg' f (zs := zs) h hu).1
      revert this; cases round f zs r <;> simp [isFinite, isNaN]
    · rw [round_neg_ovf f h (not_lt.mp hu)]; rfl
  · by_cases hu : uround f r < 2 ^ (f.emax + 1)
    · have := (round_pos f (zs := zs) h hu).1
      revert this; cases round f zs r <;> simp [isFinite, isNaN]
    · rw [round_pos_ovf f h (not_lt.mp hu)]; rfl

/-- The value of a rounding of a nonnegative number is nonnegative, and of a nonpositive one
nonpositive. -/
lemma round_val_nonneg {zs : Bool} {r : K} (hr : 0 ≤ r) : 0 ≤ (round f zs r).val := by
  rcases eq_or_lt_of_le hr with h | h
  · subst h; simp [val]
  by_cases hu : uround f r < 2 ^ (f.emax + 1)
  · have := (round_pos f (zs := zs) h hu).2.1
    have h2 : (0 : K) ≤ ((round f zs r).val : K) := by rw [this]; exact uround_nonneg f r
    exact_mod_cast h2
  · rw [round_pos_ovf f h (not_lt.mp hu)]; rfl

lemma round_val_nonpos {zs : Bool} {r : K} (hr : r ≤ 0) : (round f zs r).val ≤ 0 := by
  rcases eq_or_lt_of_le hr with h | h
  · subst h; simp [val]
  by_cases hu : uround f (-r) < 2 ^ (f.emax + 1)
  · have := (round_neg' f (zs := zs) h hu).2.1
    have h2 : ((round f zs r).val : K) ≤ 0 := by rw [this]; linarith [uround_nonneg f (-r)]
    exact_mod_cast h2
  · rw [round_neg_ovf f h (not_lt.mp hu)]; rfl

/-- **Monotonicity of rounding**, in the IEEE order of the results: `r₁ ≤ r₂` gives
`round r₁ ≤ round r₂`, infinities included. -/
theorem round_mono {s₁ s₂ : Bool} {r₁ r₂ : K} (h : r₁ ≤ r₂) :
    (round f s₁ r₁).le (round f s₂ r₂) = true := by
  have n1 := round_not_nan f s₁ r₁
  have n2 := round_not_nan f s₂ r₂
  -- results of rounding are finite or infinite
  have fin_or {zs : Bool} {r : K} : (round f zs r).isFinite = true ∨ (round f zs r) = .inf false ∨
      (round f zs r) = .inf true := by
    have := round_not_nan f zs r
    revert this; cases round f zs r <;> simp [isFinite, isNaN]
  -- an infinite rounding has the sign of the number rounded
  have inf_pos {zs : Bool} {r : K} (hi : round f zs r = .inf false) : 0 < r := by
    rcases lt_trichotomy r 0 with h | h | h
    · by_cases hu : uround f (-r) < 2 ^ (f.emax + 1)
      · have := (round_neg' f (zs := zs) h hu).2.2; rw [hi] at this; simp [sign] at this
      · rw [round_neg_ovf f h (not_lt.mp hu)] at hi; cases hi
    · subst h; simp at hi
    · exact h
  have inf_neg {zs : Bool} {r : K} (hi : round f zs r = .inf true) : r < 0 := by
    rcases lt_trichotomy r 0 with h | h | h
    · exact h
    · subst h; simp at hi
    · by_cases hu : uround f r < 2 ^ (f.emax + 1)
      · have := (round_pos f (zs := zs) h hu).2.2; rw [hi] at this; simp [sign] at this
      · rw [round_pos_ovf f h (not_lt.mp hu)] at hi; cases hi
  rcases fin_or (zs := s₂) (r := r₂) with h2 | h2 | h2
  · rcases fin_or (zs := s₁) (r := r₁) with h1 | h1 | h1
    · apply le_of_val f h1 h2
      -- both finite: compare values
      rcases le_or_gt r₂ 0 with hb | hb
      · -- both nonpositive
        rcases eq_or_lt_of_le hb with hb0 | hb0
        · rw [hb0]; simp only [round_zero, val]; exact round_val_nonpos f (le_trans h hb0.le)
        have ha0 : r₁ < 0 := lt_of_le_of_lt h hb0
        have hu1 := (round_finite_iff f ha0.ne).mp h1
        have hu2 := (round_finite_iff f hb0.ne).mp h2
        rw [abs_of_neg ha0] at hu1; rw [abs_of_neg hb0] at hu2
        have e1 := (round_neg' f (zs := s₁) ha0 hu1).2.1
        have e2 := (round_neg' f (zs := s₂) hb0 hu2).2.1
        have : ((round f s₁ r₁).val : K) ≤ ((round f s₂ r₂).val : K) := by
          rw [e1, e2]; exact neg_le_neg (uround_mono f (neg_pos.mpr hb0) (neg_le_neg h))
        exact_mod_cast this
      · rcases le_or_gt r₁ 0 with ha | ha
        · exact le_trans (round_val_nonpos f ha) (round_val_nonneg f hb.le)
        have hu1 := (round_finite_iff f ha.ne').mp h1
        have hu2 := (round_finite_iff f hb.ne').mp h2
        rw [abs_of_pos ha] at hu1; rw [abs_of_pos hb] at hu2
        have e1 := (round_pos f (zs := s₁) ha hu1).2.1
        have e2 := (round_pos f (zs := s₂) hb hu2).2.1
        have : ((round f s₁ r₁).val : K) ≤ ((round f s₂ r₂).val : K) := by
          rw [e1, e2]; exact uround_mono f ha h
        exact_mod_cast this
    · -- `round r₁ = +∞` forces `round r₂ = +∞`
      exfalso
      have ha := inf_pos h1
      have hb : 0 < r₂ := lt_of_lt_of_le ha h
      have hu2 := (round_finite_iff f hb.ne').mp h2
      rw [abs_of_pos hb] at hu2
      have hu1 : uround f r₁ < 2 ^ (f.emax + 1) := lt_of_le_of_lt (uround_mono f ha h) hu2
      have := (round_pos f (zs := s₁) ha hu1).1
      rw [h1] at this; simp [isFinite] at this
    · rw [h1]; exact inf_neg_le f n2
  · rw [h2]; exact le_inf_pos f n1
  · -- `round r₂ = -∞` forces `round r₁ = -∞`
    have hb := inf_neg h2
    have ha : r₁ < 0 := lt_of_le_of_lt h hb
    have hu : 2 ^ (f.emax + 1) ≤ uround f (-r₁) := by
      by_contra hc
      have hu1 := lt_of_not_ge hc
      have hu2 : uround f (-r₂) < 2 ^ (f.emax + 1) :=
        lt_of_le_of_lt (uround_mono f (neg_pos.mpr hb) (neg_le_neg h)) hu1
      have := (round_neg' f (zs := s₂) hb hu2).1
      rw [h2] at this; simp [isFinite] at this
    rw [round_neg_ovf f ha hu]; exact inf_neg_le f n2

/-! ## Exactness -/

/-- **Rounding is the identity on representable values.** -/
theorem round_val (zs : Bool) {s : Bool} {m : ℕ} {e : ℤ} (h : Canon f m e) :
    round f zs ((FP.fin s m e h).val : K) = .fin s m e h := by
  obtain ⟨hm0, hmp, he1, he2, hnorm⟩ := id h
  have hr : (0 : K) < m * 2 ^ e := mul_pos (by exact_mod_cast hm0) (two_zpow_pos e)
  -- the quantum exponent of `m · 2^e` is `e`
  have hq : (roundPos f ((m : K) * 2 ^ e)).2 = e := by
    rw [roundPos_snd]
    by_cases hn : 2 ^ (f.prec - 1) ≤ m
    · have : Int.log 2 ((m : K) * 2 ^ e) = e + (f.prec - 1) := by
        apply log_eq
        · rw [add_comm, two_zpow_add]
          apply mul_le_mul_of_nonneg_right _ (two_zpow_pos _).le
          rw [← f.prec_sub_one, ← cast_two_pow]; exact_mod_cast hn
        · rw [show e + (f.prec - 1) + 1 = (f.prec : ℤ) + e by ring, two_zpow_add]
          apply mul_lt_mul_of_pos_right _ (two_zpow_pos _)
          rw [← cast_two_pow]; exact_mod_cast hmp
      rw [this]; omega
    · have hq : e = f.qmin := by omega
      have : Int.log 2 ((m : K) * 2 ^ e) < e + (f.prec - 1) := by
        apply log_lt_of_lt hr
        rw [add_comm, two_zpow_add]
        apply mul_lt_mul_of_pos_right _ (two_zpow_pos _)
        rw [← f.prec_sub_one, ← cast_two_pow]; exact_mod_cast lt_of_not_ge hn
      omega
  have hfst : (roundPos f ((m : K) * 2 ^ e)).1 = m := by
    rw [roundPos_fst, hq, mul_div_assoc, div_self (two_zpow_pos e).ne', mul_one, rne_natCast]
  have hfin : ∀ s', finalize f s' m e = .fin s' m e h := by
    intro s'; unfold finalize
    simp only [show m ≠ 0 by omega, show m ≠ 2 ^ f.prec by omega, ite_false,
      show ¬ f.qmax < e by omega]
    rw [dif_pos h]
  cases s
  · have hv : ((FP.fin false m e h).val : K) = m * 2 ^ e := by simp [val, sgn]
    rw [hv]; unfold round; rw [if_neg hr.ne', if_pos hr, hfst, hq, hfin]
  · have hv : ((FP.fin true m e h).val : K) = -(m * 2 ^ e) := by simp [val, sgn]
    rw [hv]; unfold round
    rw [if_neg (neg_ne_zero.mpr hr.ne'), if_neg (not_lt.mpr (neg_nonpos.mpr hr.le)), neg_neg,
      hfst, hq, hfin]

/-- **Exactness**: `k · 2^e` with `|k| ≤ 2^prec`, `e ≥ qmin` and no overflow is representable,
so it rounds to itself. -/
theorem round_exact (zs : Bool) {k : ℤ} {e : ℤ} (hk : |k| ≤ 2 ^ f.prec) (he : f.qmin ≤ e)
    (hlt : |(k : K) * 2 ^ e| < 2 ^ (f.emax + 1)) :
    (round f zs ((k : K) * 2 ^ e)).isFinite ∧ ((round f zs ((k : K) * 2 ^ e)).val : K) = k * 2 ^ e := by
  rcases lt_trichotomy k 0 with h | h | h
  · set n := (-k).toNat
    have hn : (n : ℤ) = -k := Int.toNat_of_nonneg (by omega)
    have hnK : (n : K) = -k := by exact_mod_cast hn
    have hr : (k : K) * 2 ^ e < 0 := mul_neg_of_neg_of_pos (by exact_mod_cast h) (two_zpow_pos e)
    have hneg : -((k : K) * 2 ^ e) = n * 2 ^ e := by rw [hnK]; ring
    have hux : uround f (-((k : K) * 2 ^ e)) = -((k : K) * 2 ^ e) := by
      rw [hneg]; apply uround_exact f (by omega) he
      have : (n : ℤ) ≤ 2 ^ f.prec := by rw [hn]; rw [abs_of_neg h] at hk; exact hk
      exact_mod_cast this
    have hu : uround f (-((k : K) * 2 ^ e)) < 2 ^ (f.emax + 1) := by
      rw [hux]; rwa [abs_of_neg hr] at hlt
    obtain ⟨a, b, _⟩ := round_neg' f (zs := zs) hr hu
    exact ⟨a, by rw [b, hux, neg_neg]⟩
  · subst h; simp [isFinite, val]
  · set n := k.toNat
    have hn : (n : ℤ) = k := Int.toNat_of_nonneg (by omega)
    have hnK : (n : K) = k := by exact_mod_cast hn
    have hr : 0 < (k : K) * 2 ^ e := mul_pos (by exact_mod_cast h) (two_zpow_pos e)
    have hux : uround f ((k : K) * 2 ^ e) = (k : K) * 2 ^ e := by
      rw [← hnK]; apply uround_exact f (by omega) he
      have : (n : ℤ) ≤ 2 ^ f.prec := by rw [hn]; rw [abs_of_pos h] at hk; exact hk
      exact_mod_cast this
    have hu : uround f ((k : K) * 2 ^ e) < 2 ^ (f.emax + 1) := by
      rw [hux]; rwa [abs_of_pos hr] at hlt
    obtain ⟨a, b, _⟩ := round_pos f (zs := zs) hr hu
    exact ⟨a, by rw [b, hux]⟩

end IEEE754
