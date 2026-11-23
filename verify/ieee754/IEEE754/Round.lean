/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Basic
import Mathlib.Tactic.Linarith
import Mathlib.Tactic.Positivity
import Mathlib.Tactic.FieldSimp
import Mathlib.Tactic.Ring

/-!
# Rounding: the error bound, monotonicity, exactness

All over any linearly ordered field with a floor (`ℚ` for the arithmetic operations, `ℝ` for the
square root).
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

/-! ## Facts about the format's constants -/

namespace Format

variable (f : Format)

lemma one_le_emax : 1 ≤ f.emax := by
  unfold emax
  have : (2 : ℤ) ≤ 2 ^ (f.ebits - 1) := by
    have h := f.two_le_ebits
    calc (2 : ℤ) = 2 ^ 1 := by norm_num
      _ ≤ 2 ^ (f.ebits - 1) := pow_le_pow_right₀ (by norm_num) (by omega)
  omega

lemma prec_sub_one : ((f.prec - 1 : ℕ) : ℤ) = f.prec - 1 := by
  have := f.two_le_prec; omega

lemma qmin_le_qmax : f.qmin ≤ f.qmax := by
  have := f.one_le_emax; unfold qmin qmax emin; omega

lemma qmax_add_prec : f.qmax + f.prec = f.emax + 1 := by unfold qmax; ring

lemma qmin_add_prec : f.qmin + f.prec - 1 = f.emin := by unfold qmin; ring

lemma qmin_le_zero : f.qmin ≤ 0 := by
  have := f.one_le_emax; have := f.two_le_prec; unfold qmin emin; omega

lemma le_qexp (E : ℤ) : E - f.prec + 1 ≤ f.qexp E := le_max_left _ _

lemma qmin_le_qexp (E : ℤ) : f.qmin ≤ f.qexp E := le_max_right _ _

lemma qexp_mono {E E' : ℤ} (h : E ≤ E') : f.qexp E ≤ f.qexp E' := by
  unfold qexp; omega

end Format

section Field

variable {K : Type*} [Field K] [LinearOrder K] [IsStrictOrderedRing K] [FloorRing K]

/-! ## Powers of two -/

lemma two_zpow_pos (e : ℤ) : (0 : K) < 2 ^ e := zpow_pos (by norm_num) e

lemma two_zpow_le {a b : ℤ} (h : a ≤ b) : (2 : K) ^ a ≤ 2 ^ b :=
  zpow_le_zpow_right₀ (by norm_num) h

lemma two_zpow_lt {a b : ℤ} (h : a < b) : (2 : K) ^ a < 2 ^ b :=
  zpow_lt_zpow_right₀ (by norm_num) h

lemma two_zpow_lt_iff {a b : ℤ} : (2 : K) ^ a < 2 ^ b ↔ a < b :=
  zpow_lt_zpow_iff_right₀ (by norm_num)

lemma two_zpow_le_iff {a b : ℤ} : (2 : K) ^ a ≤ 2 ^ b ↔ a ≤ b :=
  zpow_le_zpow_iff_right₀ (by norm_num)

lemma two_zpow_add (a b : ℤ) : (2 : K) ^ (a + b) = 2 ^ a * 2 ^ b := zpow_add₀ two_ne_zero a b

lemma cast_two_pow (n : ℕ) : ((2 ^ n : ℕ) : K) = (2 : K) ^ (n : ℤ) := by
  rw [zpow_natCast]; push_cast; rfl

/-- The binade of `r > 0`. -/
lemma log_spec {r : K} (hr : 0 < r) : 2 ^ Int.log 2 r ≤ r ∧ r < 2 ^ (Int.log 2 r + 1) := by
  have h1 := Int.zpow_log_le_self (b := 2) (by norm_num) hr
  have h2 := Int.lt_zpow_succ_log_self (b := 2) (by norm_num) r
  push_cast at h1 h2
  exact ⟨h1, h2⟩

lemma log_lt_of_lt {r : K} (hr : 0 < r) {k : ℤ} (h : r < 2 ^ k) : Int.log 2 r < k :=
  two_zpow_lt_iff.mp (lt_of_le_of_lt (log_spec hr).1 h)

lemma le_log_of_le {r : K} (hr : 0 < r) {k : ℤ} (h : 2 ^ k ≤ r) : k ≤ Int.log 2 r := by
  have := two_zpow_lt_iff.mp (lt_of_le_of_lt h (log_spec hr).2); omega

lemma log_eq {r : K} {E : ℤ} (h1 : 2 ^ E ≤ r) (h2 : r < 2 ^ (E + 1)) : Int.log 2 r = E := by
  have hr : 0 < r := lt_of_lt_of_le (two_zpow_pos E) h1
  have := log_lt_of_lt hr h2
  have := le_log_of_le hr h1
  omega

/-! ## Rounding to the nearest natural number -/

lemma rne_cases (y : K) : rne y = ⌊y⌋₊ ∨ rne y = ⌊y⌋₊ + 1 := by
  unfold rne; split_ifs <;> simp

lemma rne_err {y : K} (hy : 0 ≤ y) : |(rne y : K) - y| ≤ 1 / 2 := by
  have h1 := Nat.floor_le hy
  have h2 := Nat.lt_floor_add_one y
  unfold rne
  split_ifs with ha hb
  · rw [abs_le]; constructor <;> linarith
  · rw [abs_le]; push_cast; constructor <;> linarith
  · rw [abs_le]; constructor <;> linarith
  · rw [abs_le]; push_cast; constructor <;> linarith

lemma floor_le_rne (y : K) : ⌊y⌋₊ ≤ rne y := by
  rcases rne_cases y with h | h <;> omega

lemma rne_le_floor_succ (y : K) : rne y ≤ ⌊y⌋₊ + 1 := by
  rcases rne_cases y with h | h <;> omega

lemma rne_natCast (n : ℕ) : rne (n : K) = n := by
  unfold rne; simp

lemma le_rne {y : K} {n : ℕ} (h : (n : K) ≤ y) : n ≤ rne y :=
  le_trans (Nat.le_floor h) (floor_le_rne y)

lemma rne_le {y : K} (hy : 0 ≤ y) {n : ℕ} (h : y ≤ n) : rne y ≤ n := by
  have hk : ⌊y⌋₊ ≤ n := Nat.floor_le_of_le h
  rcases Nat.lt_or_eq_of_le hk with hlt | heq
  · exact le_trans (rne_le_floor_succ y) hlt
  · have h1 := Nat.floor_le hy
    rw [heq] at h1
    have hy' : y = n := le_antisymm h h1
    rw [hy', rne_natCast]

lemma rne_mono {y₁ y₂ : K} (_h0 : 0 ≤ y₁) (h : y₁ ≤ y₂) : rne y₁ ≤ rne y₂ := by
  have hk : ⌊y₁⌋₊ ≤ ⌊y₂⌋₊ := Nat.floor_mono h
  rcases Nat.lt_or_eq_of_le hk with hlt | heq
  · exact le_trans (rne_le_floor_succ y₁) (le_trans hlt (floor_le_rne y₂))
  · unfold rne
    simp only [heq]
    split_ifs <;> first | omega | (exfalso; linarith)

/-! ## The unbounded rounding of a positive number -/

variable (f : Format)

lemma roundPos_fst (r : K) : (roundPos f r).1 = rne (r / 2 ^ (roundPos f r).2) := rfl
lemma roundPos_snd (r : K) : (roundPos f r).2 = max (Int.log 2 r - f.prec + 1) f.qmin := rfl

/-- The binade facts for `roundPos`. -/
lemma roundPos_scaled {r : K} (hr : 0 < r) :
    r / 2 ^ (roundPos f r).2 < 2 ^ (f.prec : ℤ) ∧ 0 ≤ r / 2 ^ (roundPos f r).2 := by
  obtain ⟨_, h2⟩ := log_spec hr
  set E := Int.log 2 r
  have he : E - f.prec + 1 ≤ f.qexp E := f.le_qexp E
  have hpos := two_zpow_pos (K := K) (f.qexp E)
  refine ⟨?_, div_nonneg hr.le hpos.le⟩
  simp only [roundPos]
  rw [div_lt_iff₀ hpos, ← two_zpow_add]
  exact lt_of_lt_of_le h2 (two_zpow_le (by omega))

/-- The rounded significand is at most `2^prec`. -/
lemma roundPos_le {r : K} (hr : 0 < r) : (roundPos f r).1 ≤ 2 ^ f.prec := by
  obtain ⟨h1, h2⟩ := roundPos_scaled f hr
  have : (roundPos f r).1 = rne (r / 2 ^ (roundPos f r).2) := rfl
  rw [this]
  apply rne_le h2
  rw [cast_two_pow]; exact h1.le

lemma roundPos_ge_qmin (r : K) : f.qmin ≤ (roundPos f r).2 := f.qmin_le_qexp _

/-- Above the subnormal range the rounded significand is at least `2^(prec-1)`. -/
lemma roundPos_normal {r : K} (hr : 0 < r) (hq : f.qmin < (roundPos f r).2) :
    2 ^ (f.prec - 1) ≤ (roundPos f r).1 ∧ (roundPos f r).2 = Int.log 2 r - f.prec + 1 := by
  obtain ⟨h1, _⟩ := log_spec hr
  have he : (roundPos f r).2 = Int.log 2 r - f.prec + 1 := by
    have := roundPos_snd f r; omega
  refine ⟨?_, he⟩
  rw [roundPos_fst]
  apply le_rne
  rw [cast_two_pow, le_div_iff₀ (two_zpow_pos _), ← two_zpow_add, he, f.prec_sub_one]
  convert h1 using 2; ring

/-- The unbounded rounded value `n · 2^e` of `r > 0`. -/
def uround (r : K) : K := ((roundPos f r).1 : K) * 2 ^ (roundPos f r).2

lemma uround_err {r : K} (hr : 0 < r) :
    |uround f r - r| ≤ 2 ^ ((roundPos f r).2 - 1) := by
  obtain ⟨_, h0⟩ := roundPos_scaled f hr
  have he := rne_err h0
  set e := (roundPos f r).2
  have hpos := two_zpow_pos (K := K) e
  have : uround f r - r = ((rne (r / 2 ^ e) : K) - r / 2 ^ e) * 2 ^ e := by
    simp only [uround]; rw [roundPos_fst, sub_mul, div_mul_cancel₀ _ hpos.ne']
  rw [this, abs_mul, abs_of_pos hpos]
  calc _ ≤ 1 / 2 * 2 ^ e := mul_le_mul_of_nonneg_right he hpos.le
    _ = 2 ^ (e - 1) := by rw [zpow_sub₀ two_ne_zero, zpow_one]; ring

lemma uround_nonneg (r : K) : 0 ≤ uround f r :=
  mul_nonneg (Nat.cast_nonneg _) (two_zpow_pos _).le

/-- `uround` is monotone. -/
lemma uround_mono {r₁ r₂ : K} (h0 : 0 < r₁) (h : r₁ ≤ r₂) : uround f r₁ ≤ uround f r₂ := by
  have h0' : 0 < r₂ := lt_of_lt_of_le h0 h
  have hE : Int.log 2 r₁ ≤ Int.log 2 r₂ := Int.log_mono_right h0 h
  have he : (roundPos f r₁).2 ≤ (roundPos f r₂).2 := f.qexp_mono hE
  rcases lt_or_eq_of_le he with hlt | heq
  · -- different binades: `uround r₁ ≤ 2^(p + e₁) ≤ 2^(p - 1 + e₂) ≤ uround r₂`
    have hn := roundPos_normal f h0' (lt_of_le_of_lt (roundPos_ge_qmin f r₁) hlt)
    have a1 : uround f r₁ ≤ 2 ^ ((f.prec : ℤ) + (roundPos f r₁).2) := by
      simp only [uround]
      rw [two_zpow_add]
      apply mul_le_mul_of_nonneg_right _ (two_zpow_pos _).le
      rw [← cast_two_pow]; exact_mod_cast roundPos_le f h0
    have a2 : (2 : K) ^ ((f.prec : ℤ) - 1 + (roundPos f r₂).2) ≤ uround f r₂ := by
      simp only [uround]
      rw [two_zpow_add]
      apply mul_le_mul_of_nonneg_right _ (two_zpow_pos _).le
      rw [← f.prec_sub_one, ← cast_two_pow]; exact_mod_cast hn.1
    exact le_trans a1 (le_trans (two_zpow_le (by omega)) a2)
  · simp only [uround, roundPos_fst f r₁, roundPos_fst f r₂, heq]
    apply mul_le_mul_of_nonneg_right _ (two_zpow_pos _).le
    have := rne_mono (div_nonneg h0.le (two_zpow_pos _).le)
      (div_le_div_of_nonneg_right h (two_zpow_pos (K := K) (roundPos f r₂).2).le)
    exact_mod_cast this

/-- With a quantum exponent at most `e`, a multiple of `2^e` is its own rounding. -/
lemma uround_exact {n : ℕ} {e : ℤ} (hn : 0 < n) (he : f.qmin ≤ e)
    (hp : n ≤ 2 ^ f.prec) : uround f ((n : K) * 2 ^ e) = n * 2 ^ e := by
  set r : K := n * 2 ^ e
  have hr : 0 < r := mul_pos (by exact_mod_cast hn) (two_zpow_pos e)
  have hrle : r ≤ 2 ^ ((f.prec : ℤ) + e) := by
    rw [two_zpow_add]
    exact mul_le_mul_of_nonneg_right (by rw [← cast_two_pow]; exact_mod_cast hp) (two_zpow_pos _).le
  -- the quantum exponent `e'` is at most `e + 1`, and at most `e` unless `r = 2^(p+e)`
  set e' := (roundPos f r).2
  have hE := log_spec hr
  have hlog : Int.log 2 r ≤ f.prec + e := by
    have := lt_of_le_of_lt hE.1 (lt_of_le_of_lt hrle (two_zpow_lt (lt_add_one _)))
    have := two_zpow_lt_iff.mp this; omega
  -- `r / 2^e'` is a natural number
  obtain ⟨N, hN⟩ : ∃ N : ℕ, r / 2 ^ e' = N := by
    by_cases hle : e' ≤ e
    · refine ⟨n * 2 ^ (e - e').toNat, ?_⟩
      push_cast
      rw [← zpow_natCast, Int.toNat_of_nonneg (by omega)]
      simp only [r]
      rw [mul_div_assoc, ← zpow_sub₀ two_ne_zero]
    · -- then `e' = log r - p + 1 > e`, so `log r = p + e` and `r = 2^(p+e)`, `n = 2^p`
      have he' : e' = Int.log 2 r - f.prec + 1 := by
        have := roundPos_snd f r; have := f.qmin_le_qmax; omega
      have hl : Int.log 2 r = f.prec + e := by omega
      have : r = 2 ^ ((f.prec : ℤ) + e) := by
        apply le_antisymm hrle; rw [← hl]; exact hE.1
      refine ⟨2 ^ (f.prec - 1), ?_⟩
      rw [this, cast_two_pow, ← zpow_sub₀ two_ne_zero, f.prec_sub_one, he', hl]
      congr 1; ring
  have : (roundPos f r).1 = N := by
    show rne (r / 2 ^ e') = N
    rw [hN, rne_natCast]
  simp only [uround, this]
  show (N : K) * 2 ^ e' = r
  rw [← hN, div_mul_cancel₀ _ (two_zpow_pos _).ne']

end Field

/-! ## From the unbounded rounding to a datum -/

section Finalize

variable (f : Format)

/-- The threshold `2^(emax+1)`: unbounded rounded values at or above it overflow. -/
lemma finalize_of_lt {s : Bool} {n : ℕ} {e : ℤ} (hn : n ≤ 2 ^ f.prec) (he : f.qmin ≤ e)
    (hnorm : 2 ^ (f.prec - 1) ≤ n ∨ e = f.qmin)
    (hlt : (n : ℚ) * 2 ^ e < 2 ^ (f.emax + 1)) :
    (finalize f s n e).isFinite ∧ (finalize f s n e).val = sgn s * ((n : ℚ) * 2 ^ e) ∧
      (finalize f s n e).sign = s := by
  unfold finalize
  by_cases h0 : n = 0
  · subst h0; simp [isFinite, val, sign]
  simp only [h0, ite_false]
  have hp1 := f.prec_sub_one
  have hp2 := f.two_le_prec
  have hq := f.qmax_add_prec
  by_cases hc : n = 2 ^ f.prec
  · simp only [hc, ite_true]
    -- `2^p · 2^e < 2^(qmax + p)` so `e + 1 ≤ qmax`
    have : e < f.qmax := by
      rw [hc, cast_two_pow, ← two_zpow_add, ← hq] at hlt
      have := two_zpow_lt_iff.mp hlt; omega
    have hcan : Canon f (2 ^ (f.prec - 1)) (e + 1) := by
      refine ⟨by positivity, Nat.pow_lt_pow_right (by norm_num) (by omega), by omega, by omega,
        Or.inl le_rfl⟩
    simp only [show ¬ f.qmax < e + 1 by omega, ite_false, hcan, dite_true]
    refine ⟨rfl, ?_, rfl⟩
    simp only [val]
    congr 1
    rw [cast_two_pow, cast_two_pow, ← two_zpow_add, ← two_zpow_add, hp1]
    congr 1; ring
  · simp only [hc, ite_false]
    have hlt' : n < 2 ^ f.prec := lt_of_le_of_ne hn hc
    have heq : e ≤ f.qmax := by
      by_contra hgt
      have hgt := lt_of_not_ge hgt
      have hnorm' : 2 ^ (f.prec - 1) ≤ n := by
        rcases hnorm with h | h
        · exact h
        · have := f.qmin_le_qmax; omega
      have : (2 : ℚ) ^ (f.emax + 1) ≤ n * 2 ^ e := by
        calc (2 : ℚ) ^ (f.emax + 1) = 2 ^ ((f.prec : ℤ) - 1 + (f.qmax + 1)) := by
              congr 1; omega
          _ = ((2 ^ (f.prec - 1) : ℕ) : ℚ) * 2 ^ (f.qmax + 1) := by
              rw [two_zpow_add, cast_two_pow, hp1]
          _ ≤ n * 2 ^ e := by
              apply mul_le_mul (by exact_mod_cast hnorm') (two_zpow_le (by omega))
                (two_zpow_pos _).le (Nat.cast_nonneg _)
      linarith
    have hcan : Canon f n e := ⟨by omega, hlt', he, heq, hnorm⟩
    simp only [show ¬ f.qmax < e by omega, ite_false, hcan, dite_true]
    exact ⟨rfl, rfl, rfl⟩

lemma finalize_of_ge {s : Bool} {n : ℕ} {e : ℤ} (hn : n ≤ 2 ^ f.prec)
    (hge : (2 : ℚ) ^ (f.emax + 1) ≤ (n : ℚ) * 2 ^ e) : finalize f s n e = .inf s := by
  have hp1 := f.prec_sub_one
  have hq := f.qmax_add_prec
  unfold finalize
  have h0 : n ≠ 0 := by
    rintro rfl; simp at hge; linarith [two_zpow_pos (K := ℚ) (f.emax + 1)]
  simp only [h0, ite_false]
  by_cases hc : n = 2 ^ f.prec
  · simp only [hc, ite_true]
    rw [hc, cast_two_pow, ← two_zpow_add, ← hq] at hge
    have := two_zpow_le_iff.mp hge
    simp only [show f.qmax < e + 1 by omega, ite_true]
  · simp only [hc, ite_false]
    have hlt' : (n : ℚ) < 2 ^ (f.prec : ℤ) := by
      rw [← cast_two_pow]; exact_mod_cast lt_of_le_of_ne hn hc
    have : (2 : ℚ) ^ (f.emax + 1) < 2 ^ ((f.prec : ℤ) + e) := by
      rw [two_zpow_add (f.prec : ℤ) e]
      exact lt_of_le_of_lt hge (mul_lt_mul_of_pos_right hlt' (two_zpow_pos _))
    have := two_zpow_lt_iff.mp this
    simp only [show f.qmax < e by omega, ite_true]

end Finalize

end IEEE754
