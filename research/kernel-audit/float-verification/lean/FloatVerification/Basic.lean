/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import FvPredicates

/-!
# Facts about the model used by the predicate proofs

* the constants of the translated code, decoded;
* the sign of a rounded difference is the sign of the exact difference (or zero);
* rounded products keep the order of the exact products;
* a comparison `B < X` that holds rules out NaN and the wrong infinities, and when `B` is
  `c · (|l| + |r|)` it forces `l`, `r` and `l - r` to be finite.
-/

set_option linter.unusedSectionVars false

namespace FloatVerification

open IEEE754 hiding F64
open IEEE754.FP

/-- `ε = 2^-53`, the unit roundoff of binary64. -/
abbrev ε : ℚ := 2 ^ (-53 : ℤ)

lemma ε_pos : 0 < ε := by positivity
lemma ε_le : ε ≤ 1 / 1000 := by norm_num [ε]

/-! ## The constants -/

/-- `CCW_BOUND = (3 + 16ε)ε`. -/
lemma ccw_bound_decode : (FP.ofBits 4375247037990436868 : F64) =
    .fin false 6755399441055748 (-104) (by decide) := by decide

lemma ccw_bound_val : (FP.ofBits 4375247037990436868 : F64).val = (3 + 16 * ε) * ε := by
  rw [ccw_bound_decode]; simp [FP.val, FP.sgn, ε]; norm_num

/-- `CROSS_BOUND = (2 + 8ε)ε`. -/
lemma cross_bound_decode : (FP.ofBits 4372995238176751618 : F64) =
    .fin false 4503599627370498 (-104) (by decide) := by decide

lemma cross_bound_val : (FP.ofBits 4372995238176751618 : F64).val = (2 + 8 * ε) * ε := by
  rw [cross_bound_decode]; simp [FP.val, FP.sgn, ε]; norm_num

/-! ## Signs and order -/

/-- A rounded difference has the sign of the exact one, or is zero. -/
lemma sub_sign {x y : F64} (hx : x.isFinite) (hy : y.isFinite) :
    (0 < (sub x y).val → 0 < x.val - y.val) ∧ ((sub x y).val < 0 → x.val - y.val < 0) := by
  rw [sub_eq_round hx hy]
  constructor
  · intro h; by_contra h'; exact absurd (IEEE754.round_val_nonpos binary64 (K := ℚ) (not_lt.mp h')) (not_le.mpr h)
  · intro h; by_contra h'; exact absurd (IEEE754.round_val_nonneg binary64 (K := ℚ) (not_lt.mp h')) (not_le.mpr h)

/-- Rounded products keep the order of the exact products. -/
lemma mul_mono {a b c d : F64} (ha : a.isFinite) (hb : b.isFinite) (hc : c.isFinite) (hd : d.isFinite)
    (h1 : (mul a b).isFinite) (h2 : (mul c d).isFinite) (h : a.val * b.val ≤ c.val * d.val) :
    (mul a b).val ≤ (mul c d).val := by
  have := IEEE754.round_mono binary64 (s₁ := a.sign ^^ b.sign) (s₂ := c.sign ^^ d.sign) h
  rw [← mul_eq_round ha hb, ← mul_eq_round hc hd] at this
  exact (le_iff h1 h2).mp this

/-! ## What a comparison that holds rules out -/

lemma lt_cases {x y : F64} (h : FP.lt x y = true) :
    x.isNaN = false ∧ y.isNaN = false ∧ x ≠ .inf false ∧ y ≠ .inf true := by
  -- (whether `simp_all` leaves the `if` of two infinities or rewrites it away depends on the simp
  -- set the imports bring; both are closed)
  cases x <;> cases y <;> simp_all [FP.lt, FP.cmp, isNaN] <;> (try split_ifs at h) <;> (try simp_all) <;>
    (revert h; rename_i a b; cases a <;> cases b <;> simp)

lemma lt_finite {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (h : FP.lt x y = true) :
    x.val < y.val := (lt_iff hx hy).mp h

/-- `c · s` with `c` finite and positive is `+∞` when `s = +∞`, and NaN when `s` is. -/
lemma mul_pos_left_cases {c s : F64} (hc : c.isFinite) (hc0 : 0 < c.val) :
    (s = .inf false → mul c s = .inf false) ∧ (s = .nan → mul c s = .nan) := by
  have hcs : c.sign = false ∧ c.isZero = false := by
    cases c with
    | fin s' m e h => cases s' <;> simp_all [val, sgn, sign, isZero]; linarith [show (0:ℚ) ≤ m * 2 ^ e by positivity]
    | zero s' => simp [val] at hc0
    | _ => simp [isFinite] at hc
  constructor
  · rintro rfl; cases c <;> simp_all [mul, isFinite]
  · rintro rfl; cases c <;> simp_all [mul, isFinite]

/-- A finite difference whose rounding overflows makes the rounded sum of magnitudes overflow too. -/
lemma sum_abs_inf_of_sub_inf {l r : F64} (hl : l.isFinite) (hr : r.isFinite)
    (hd : (sub l r).isFinite = false) : (add l.abs r.abs).isFinite = false := by
  have hla : l.abs.isFinite := by simpa using hl
  have hra : r.abs.isFinite := by simpa using hr
  rw [sub_eq_round hl hr] at hd
  rw [add_eq_round hla hra, abs_val, abs_val]
  have hne : l.val - r.val ≠ 0 := by
    intro h0; rw [h0] at hd; simp [isFinite] at hd
  have hd' := hd
  rw [Bool.eq_false_iff, ne_eq, IEEE754.round_finite_iff binary64 hne] at hd'
  have hpos : 0 < |l.val - r.val| := abs_pos.mpr hne
  have hle : |l.val - r.val| ≤ |l.val| + |r.val| := abs_sub _ _
  have hsum : |l.val| + |r.val| ≠ 0 := by linarith
  rw [Bool.eq_false_iff, ne_eq, IEEE754.round_finite_iff binary64 hsum,
    abs_of_nonneg (by positivity : (0 : ℚ) ≤ |l.val| + |r.val|)]
  intro hlt
  exact hd' (lt_of_le_of_lt (IEEE754.uround_mono binary64 hpos hle) hlt)

/-- Rounding a nonnegative number never gives `-∞`. -/
lemma round_ne_neg_inf {zs : Bool} {r : ℚ} (hr : 0 ≤ r) : round binary64 zs r ≠ .inf true := by
  rcases eq_or_lt_of_le hr with h | h
  · subst h; simp
  by_cases hu : uround binary64 r < 2 ^ (binary64.emax + 1)
  · have := (round_pos binary64 (zs := zs) h hu).1
    intro h'; rw [h'] at this; simp [isFinite] at this
  · rw [round_pos_ovf binary64 h (not_lt.mp hu)]; simp

/-- A sum of magnitudes is never `-∞`. -/
lemma add_abs_ne_neg_inf (x y : F64) : add x.abs y.abs ≠ .inf true := by
  rcases trichotomy x with hx | ⟨s, rfl⟩ | rfl <;> rcases trichotomy y with hy | ⟨t, rfl⟩ | rfl
  · rw [add_eq_round (by simpa using hx) (by simpa using hy)]
    exact round_ne_neg_inf (add_nonneg (val_abs_nonneg x) (val_abs_nonneg y))
  all_goals (first
    | (cases x <;> simp_all [isFinite, add, FP.abs]; done)
    | (cases y <;> simp_all [isFinite, add, FP.abs]; done)
    | simp [add, FP.abs])

/-- If `c · (|l| + |r|) < X` holds for `c` finite and positive, then `l`, `r`, `|l| + |r|`,
`c · (|l| + |r|)` and `l - r` are finite. -/
lemma finite_of_bound {c l r X : F64} (hc : c.isFinite) (hc0 : 0 < c.val)
    (h : FP.lt (mul c (add l.abs r.abs)) X = true) :
    l.isFinite ∧ r.isFinite ∧ (add l.abs r.abs).isFinite ∧ (mul c (add l.abs r.abs)).isFinite ∧
      (sub l r).isFinite := by
  obtain ⟨hB, _, hB', _⟩ := lt_cases h
  have hS : (add l.abs r.abs).isFinite := by
    rcases trichotomy (add l.abs r.abs) with hS | ⟨s, hS⟩ | hS
    · exact hS
    · cases s
      · exact absurd ((mul_pos_left_cases hc hc0).1 hS) hB'
      · exact absurd hS (add_abs_ne_neg_inf l r)
    · rw [(mul_pos_left_cases hc hc0).2 hS] at hB; simp [isNaN] at hB
  obtain ⟨hl, hr⟩ := isFinite_of_add hS
  have hl' : l.isFinite := by simpa using hl
  have hr' : r.isFinite := by simpa using hr
  have hBf : (mul c (add l.abs r.abs)).isFinite := by
    rcases trichotomy (mul c (add l.abs r.abs)) with hm | ⟨s, hm⟩ | hm
    · exact hm
    · cases s
      · exact absurd hm hB'
      · rw [mul_eq_round hc hS] at hm
        refine absurd hm (round_ne_neg_inf (mul_nonneg hc0.le ?_))
        rw [add_eq_round hl hr]
        exact round_val_nonneg binary64 (add_nonneg (val_abs_nonneg l) (val_abs_nonneg r))
    · rw [hm] at hB; simp [isNaN] at hB
  refine ⟨hl', hr', hS, hBf, ?_⟩
  by_contra hd
  have := sum_abs_inf_of_sub_inf hl' hr' (Bool.eq_false_iff.mpr hd)
  rw [hS] at this; exact absurd this (by decide)

end FloatVerification
