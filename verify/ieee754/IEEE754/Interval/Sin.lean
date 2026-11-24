/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Interval.Trig
import IEEE754.Dmath.SinCos
import IEEE754.Dmath.AtanConsts
import IEEE754.Dmath.Pi30
import IEEE754.Screen.Gap
import IEEE754.Nearest

/-!
# `Interval::sin` and `Interval::cos` enclose (trace, 80a5f54c)

`trig_at` over `[a, b]` (finite ends, `|a|, |b| ≤ 2^20`):

- `lo = (fa.min(fb) − TRIG_ERR).next_down()` and `hi = (fa.max(fb) + TRIG_ERR).next_up()`,
  where `fa`, `fb` are dmath's values at the ends and `TRIG_ERR = 2^-52`;
- `qa = (a/PI).min(a/PI_UP).next_down()` and `qb = (b/PI).max(b/PI_UP).next_up()`;
- `k` runs over the integers from `(qa − half).ceil()` to `(qb − half).floor()`: `hi = 1` at an
  even one, `lo = −1` at an odd one;
- the result is `[lo.max(−1), hi.min(1)]`.

`next_down(y)` is modelled as any finite float below `y`, and `next_up` likewise; that is all
soundness needs. The facts used:

- `below_round`: a float below a rounding `fl(r)` is at most `r`, because rounding is to nearest.
  `above_round` is the mirror.
- `crit_counted`: an extreme `(k + half)π` in `[a, b]` has `⌈fl(qa − half)⌉ ≤ k ≤ ⌊fl(qb − half)⌋`.
  `qa − half` is not always exact (a negative `qa` can cross a binade), but rounding is monotone and
  `k` is a float, which is enough.
- `sin_encloses`, `cos_encloses`: every `sin x` (resp. `cos x`) with `x ∈ [a, b]` lies in the result.
-/

namespace IEEE754.Interval

open IEEE754.FP Real

/-- `PI_UP`, the float just above π. -/
def PI_UP : F64 := .fin false 7074237752028441 (-51) (by decide)
theorem PI_UP_bits : PI_UP.toBits = 0x400921fb54442d19 := by decide
theorem PI_UP_val : PI_UP.val = (7074237752028441 / 2 ^ 51 : ℚ) := by simp only [PI_UP, val, sgn]; norm_num
/-- `TRIG_ERR = 2^-52`. -/
def TRIG_ERR : F64 := .fin false 4503599627370496 (-104) (by decide)
theorem TRIG_ERR_val : TRIG_ERR.val = (1 / 2 ^ 52 : ℚ) := by simp only [TRIG_ERR, val, sgn]; norm_num
/-- `0.5` and `0.0`. -/
def HALF : F64 := .fin false 4503599627370496 (-53) (by decide)
theorem HALF_val : HALF.val = (1 / 2 : ℚ) := by simp only [HALF, val, sgn]; norm_num
def ZERO : F64 := .zero false
theorem ZERO_val : ZERO.val = (0 : ℚ) := rfl

lemma PI_lt_pi : ((Dmath.PI.val : ℚ) : ℝ) < π := by
  rw [Dmath.PI_val]; have := Dmath.pi_gt_d30; push_cast; linarith [show (884279719003555 / 2 ^ 48 : ℝ) <
    3.141592653589793238462643383279 by norm_num]
lemma pi_lt_PI_UP : π < ((PI_UP.val : ℚ) : ℝ) := by
  rw [PI_UP_val]; have := Dmath.pi_lt_d30; push_cast; linarith [show (3.141592653589793238462643383280 : ℝ) <
    7074237752028441 / 2 ^ 51 by norm_num]
lemma PI_pos : (0 : ℝ) < ((Dmath.PI.val : ℚ) : ℝ) := by rw [Dmath.PI_val]; norm_num

/-- **A float below a rounding is at most the rounded value** (rounding to nearest). -/
lemma below_round {zs : Bool} {r : ℚ} {x : F64} (hx : x.isFinite) (hf : (round binary64 zs r).isFinite)
    (h : x.val < (round binary64 zs r).val) : x.val ≤ r := by
  by_contra hn; push Not at hn
  have := round_nearest (f := binary64) hf hx
  rw [abs_of_pos (by linarith : 0 < x.val - r), abs_of_pos (by linarith : 0 < (round binary64 zs r).val - r)] at this
  linarith

lemma above_round {zs : Bool} {r : ℚ} {x : F64} (hx : x.isFinite) (hf : (round binary64 zs r).isFinite)
    (h : (round binary64 zs r).val < x.val) : r ≤ x.val := by
  by_contra hn; push Not at hn
  have := round_nearest (f := binary64) hf hx
  rw [abs_of_neg (by linarith : x.val - r < 0), abs_of_neg (by linarith : (round binary64 zs r).val - r < 0)] at this
  linarith

/-- An integer of at most `2^53` is a float. -/
lemma int_float (k : ℤ) (hk : |k| ≤ 2 ^ 53) : ∃ x : F64, x.isFinite ∧ x.val = k ∧
    ∀ zs, (round binary64 zs (k : ℚ)).isFinite ∧ (round binary64 zs (k : ℚ)).val = k := by
  have h := fun zs => round_exact_q (f := binary64) zs (k := k) (e := 0) (by simpa using hk) (by simp)
    (by
      simp only [zpow_zero, _root_.mul_one, binary64_emax]
      have : |(k : ℚ)| ≤ 2 ^ 53 := by exact_mod_cast hk
      have : (2 : ℚ) ^ 53 < 2 ^ ((1023 : ℤ) + 1) := by
        rw [show ((1023 : ℤ) + 1) = ((1024 : ℕ) : ℤ) by norm_num, zpow_natCast]
        exact pow_lt_pow_right₀ (by norm_num) (by norm_num)
      linarith)
  simp only [zpow_zero, _root_.mul_one] at h
  exact ⟨_, (h false).1, (h false).2, h⟩

/-- `fl(r) ≤ k` whenever `r ≤ k`, for an integer float `k`. -/
lemma round_le_int {zs : Bool} {r : ℚ} {k : ℤ} (hk : |k| ≤ 2 ^ 53) (hf : (round binary64 zs r).isFinite)
    (h : r ≤ k) : (round binary64 zs r).val ≤ k := by
  obtain ⟨x, hx, hxv, hr⟩ := int_float k hk
  by_contra hn; push Not at hn
  have h1 := below_round hx hf (by rw [hxv]; exact hn)
  rw [hxv] at h1
  have he : r = k := le_antisymm h h1
  subst he
  rw [(hr zs).2] at hn
  exact lt_irrefl _ hn

lemma int_le_round {zs : Bool} {r : ℚ} {k : ℤ} (hk : |k| ≤ 2 ^ 53) (hf : (round binary64 zs r).isFinite)
    (h : (k : ℚ) ≤ r) : (k : ℚ) ≤ (round binary64 zs r).val := by
  obtain ⟨x, hx, hxv, hr⟩ := int_float k hk
  by_contra hn; push Not at hn
  have h1 := above_round hx hf (by rw [hxv]; exact hn)
  rw [hxv] at h1
  have he : r = k := le_antisymm h1 h
  subst he
  rw [(hr zs).2] at hn
  exact lt_irrefl _ hn

/-- A float below the f64 `min` of two roundings is at most both rounded values. -/
lemma below_fmin {zs1 zs2 : Bool} {r1 r2 : ℚ} {d : F64} (hd : d.isFinite)
    (h1 : (round binary64 zs1 r1).isFinite) (h2 : (round binary64 zs2 r2).isFinite)
    (h : d.val < (Screen.fmin (round binary64 zs1 r1) (round binary64 zs2 r2)).val) : d.val ≤ r1 ∧ d.val ≤ r2 := by
  rw [Screen.fmin_val] at h
  exact ⟨below_round hd h1 (lt_of_lt_of_le h (min_le_left _ _)),
    below_round hd h2 (lt_of_lt_of_le h (min_le_right _ _))⟩

lemma above_fmax {zs1 zs2 : Bool} {r1 r2 : ℚ} {u : F64} (hu : u.isFinite)
    (h1 : (round binary64 zs1 r1).isFinite) (h2 : (round binary64 zs2 r2).isFinite)
    (h : (Screen.fmax (round binary64 zs1 r1) (round binary64 zs2 r2)).val < u.val) : r1 ≤ u.val ∧ r2 ≤ u.val := by
  rw [Screen.fmax_val] at h
  exact ⟨above_round hu h1 (lt_of_le_of_lt (le_max_left _ _) h),
    above_round hu h2 (lt_of_le_of_lt (le_max_right _ _) h)⟩

lemma PI_ne : Dmath.PI.val ≠ 0 := by rw [Dmath.PI_val]; norm_num
lemma PI_UP_ne : PI_UP.val ≠ 0 := by rw [PI_UP_val]; norm_num

set_option maxHeartbeats 1000000 in
/-- **No extreme is missed.** An extreme `(k + half)π` in `[a, b]` has `k` in the loop's range
`[⌈fl(qa − half)⌉, ⌊fl(qb − half)⌋]`, with `qa` any float below `min(a/PI, a/PI_UP)` (f64) and
`qb` any above `max(b/PI, b/PI_UP)`. -/
theorem crit_counted {a b qa qb half : F64} {h : ℚ} {k : ℤ}
    (ha : a.isFinite) (hb : b.isFinite) (hqa : qa.isFinite) (hqb : qb.isFinite) (hh : half.isFinite)
    (hhv : half.val = h) (hh0 : 0 ≤ h) (hh1 : h ≤ 1 / 2)
    (hab : |a.val| ≤ 1048576 ∧ |b.val| ≤ 1048576)
    (f1 : (div a Dmath.PI).isFinite) (f2 : (div a PI_UP).isFinite)
    (f3 : (div b Dmath.PI).isFinite) (f4 : (div b PI_UP).isFinite)
    (hqa' : qa.val < (Screen.fmin (div a Dmath.PI) (div a PI_UP)).val)
    (hqb' : (Screen.fmax (div b Dmath.PI) (div b PI_UP)).val < qb.val)
    (fsa : (sub qa half).isFinite) (fsb : (sub qb half).isFinite)
    (hin : ((k : ℝ) + h) * π ∈ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ)) :
    ⌈(sub qa half).val⌉ ≤ k ∧ k ≤ ⌊(sub qb half).val⌋ := by
  have fP : Dmath.PI.isFinite := by decide
  have fU : PI_UP.isFinite := by decide
  rw [div_eq_round ha fP PI_ne] at f1 hqa'; rw [div_eq_round ha fU PI_UP_ne] at f2 hqa'
  rw [div_eq_round hb fP PI_ne] at f3 hqb'; rw [div_eq_round hb fU PI_UP_ne] at f4 hqb'
  obtain ⟨l1, l2⟩ := below_fmin hqa f1 f2 hqa'
  obtain ⟨u1, u2⟩ := above_fmax hqb f3 f4 hqb'
  have cq := crit_quotients ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ) ((k : ℝ) + h) ((Dmath.PI.val : ℚ) : ℝ)
    ((PI_UP.val : ℚ) : ℝ) ((qa.val : ℚ) : ℝ) ((qb.val : ℚ) : ℝ) PI_pos PI_lt_pi pi_lt_PI_UP
    (by rw [le_min_iff]; constructor <;> [skip; skip] <;> [have := (Rat.cast_le (K := ℝ)).mpr l1;
          have := (Rat.cast_le (K := ℝ)).mpr l2] <;> push_cast at this ⊢ <;> exact this)
    (by rw [max_le_iff]; constructor <;> [skip; skip] <;> [have := (Rat.cast_le (K := ℝ)).mpr u1;
          have := (Rat.cast_le (K := ℝ)).mpr u2] <;> push_cast at this ⊢ <;> exact this)
    hin
  have cq1 : qa.val ≤ k + h := by have := cq.1; exact_mod_cast this
  have cq2 : (k : ℚ) + h ≤ qb.val := by have := cq.2; exact_mod_cast this
  -- `|k| ≤ 2^53`
  have hk : |k| ≤ 2 ^ 53 := by
    have hpi := Real.pi_gt_three
    have e1 : (-1048576 : ℝ) ≤ ((k : ℝ) + h) * π := by
      have := hin.1; have := (abs_le.mp hab.1).1
      have : (-1048576 : ℝ) ≤ ((a.val : ℚ) : ℝ) := by exact_mod_cast this
      linarith
    have e2 : ((k : ℝ) + h) * π ≤ 1048576 := by
      have := hin.2; have := (abs_le.mp hab.2).2
      have : ((b.val : ℚ) : ℝ) ≤ 1048576 := by exact_mod_cast this
      linarith
    have hhr : (0 : ℝ) ≤ h := by exact_mod_cast hh0
    have hhr1 : (h : ℝ) ≤ 1 / 2 := by have := (Rat.cast_le (K := ℝ)).mpr hh1; push_cast at this; exact this
    have k1 : (k : ℝ) ≤ 1048576 := by
      by_contra hn; push Not at hn; nlinarith
    have k2 : (-1048577 : ℝ) ≤ k := by
      by_contra hn; push Not at hn; nlinarith
    have : (k : ℤ) ≤ 1048576 := by exact_mod_cast k1
    have : (-1048577 : ℤ) ≤ k := by exact_mod_cast k2
    rw [abs_le]; constructor <;> omega
  rw [sub_eq_round hqa hh] at fsa ⊢
  rw [sub_eq_round hqb hh] at fsb ⊢
  constructor
  · apply Int.ceil_le.mpr
    exact round_le_int hk fsa (by rw [hhv]; linarith)
  · apply Int.le_floor.mpr
    exact int_le_round hk fsb (by rw [hhv]; linarith)

/-- The ends: `lo` below `fl(min(fa, fb) − 2^-52)` is at most both true values, `hi` above
`fl(max(fa, fb) + 2^-52)` at least both. -/
lemma ends_lo {fa fb lo : F64} {va vb : ℝ} (hfa : fa.isFinite) (hfb : fb.isFinite) (hl : lo.isFinite)
    (ea : |(fa.val : ℝ) - va| ≤ 1 / 2 ^ 52) (eb : |(fb.val : ℝ) - vb| ≤ 1 / 2 ^ 52)
    (hs : (sub (Screen.fmin fa fb) TRIG_ERR).isFinite) (h : lo.val < (sub (Screen.fmin fa fb) TRIG_ERR).val) :
    (lo.val : ℝ) ≤ min va vb := by
  have fT : TRIG_ERR.isFinite := by decide
  have fm := Screen.fmin_fin hfa hfb
  rw [sub_eq_round fm fT] at hs h
  have l := below_round hl hs h
  rw [Screen.fmin_val, TRIG_ERR_val] at l
  have l' : ((lo.val : ℚ) : ℝ) ≤ ((min fa.val fb.val - 1 / 2 ^ 52 : ℚ) : ℝ) := Rat.cast_le.mpr l
  push_cast at l'
  rw [le_min_iff]
  have := (abs_le.mp ea).2; have := (abs_le.mp eb).2
  have := min_le_left (fa.val : ℝ) fb.val; have := min_le_right (fa.val : ℝ) fb.val
  constructor <;> linarith

lemma ends_hi {fa fb hi : F64} {va vb : ℝ} (hfa : fa.isFinite) (hfb : fb.isFinite) (hh : hi.isFinite)
    (ea : |(fa.val : ℝ) - va| ≤ 1 / 2 ^ 52) (eb : |(fb.val : ℝ) - vb| ≤ 1 / 2 ^ 52)
    (hs : (add (Screen.fmax fa fb) TRIG_ERR).isFinite) (h : (add (Screen.fmax fa fb) TRIG_ERR).val < hi.val) :
    max va vb ≤ (hi.val : ℝ) := by
  have fT : TRIG_ERR.isFinite := by decide
  have fm := Screen.fmax_fin hfa hfb
  rw [add_eq_round fm fT] at hs h
  have l := above_round hh hs h
  rw [Screen.fmax_val, TRIG_ERR_val] at l
  have l' : ((max fa.val fb.val + 1 / 2 ^ 52 : ℚ) : ℝ) ≤ ((hi.val : ℚ) : ℝ) := Rat.cast_le.mpr l
  push_cast at l'
  rw [max_le_iff]
  have := (abs_le.mp ea).1; have := (abs_le.mp eb).1
  have := le_max_left (fa.val : ℝ) fb.val; have := le_max_right (fa.val : ℝ) fb.val
  constructor <;> linarith

set_option maxHeartbeats 1000000 in
/-- **`Interval::sin` encloses.** Every `sin x`, `x ∈ [a, b]`, lies in `[lo.max(−1), hi.min(1)]`. -/
theorem sin_encloses {a b lo0 hi0 qa qb lo hi : F64}
    (ha : a.isFinite) (hb : b.isFinite) (hab : a.val ≤ b.val) (hr : |a.val| ≤ 1048576 ∧ |b.val| ≤ 1048576)
    (hl0 : lo0.isFinite) (hs0 : (sub (Screen.fmin (Dmath.sin a) (Dmath.sin b)) TRIG_ERR).isFinite)
    (hlo0 : lo0.val < (sub (Screen.fmin (Dmath.sin a) (Dmath.sin b)) TRIG_ERR).val)
    (hh0 : hi0.isFinite) (ht0 : (add (Screen.fmax (Dmath.sin a) (Dmath.sin b)) TRIG_ERR).isFinite)
    (hhi0 : (add (Screen.fmax (Dmath.sin a) (Dmath.sin b)) TRIG_ERR).val < hi0.val)
    (hqa : qa.isFinite) (hqb : qb.isFinite)
    (f1 : (div a Dmath.PI).isFinite) (f2 : (div a PI_UP).isFinite)
    (f3 : (div b Dmath.PI).isFinite) (f4 : (div b PI_UP).isFinite)
    (hqa' : qa.val < (Screen.fmin (div a Dmath.PI) (div a PI_UP)).val)
    (hqb' : (Screen.fmax (div b Dmath.PI) (div b PI_UP)).val < qb.val)
    (fsa : (sub qa HALF).isFinite) (fsb : (sub qb HALF).isFinite)
    (hlo : (∃ k : ℤ, ⌈(sub qa HALF).val⌉ ≤ k ∧ k ≤ ⌊(sub qb HALF).val⌋ ∧ ¬ Even k) → lo.val = -1)
    (hlo' : ¬ (∃ k : ℤ, ⌈(sub qa HALF).val⌉ ≤ k ∧ k ≤ ⌊(sub qb HALF).val⌋ ∧ ¬ Even k) → lo.val = lo0.val)
    (hhi : (∃ k : ℤ, ⌈(sub qa HALF).val⌉ ≤ k ∧ k ≤ ⌊(sub qb HALF).val⌋ ∧ Even k) → hi.val = 1)
    (hhi' : ¬ (∃ k : ℤ, ⌈(sub qa HALF).val⌉ ≤ k ∧ k ≤ ⌊(sub qb HALF).val⌋ ∧ Even k) → hi.val = hi0.val) :
    ∀ x ∈ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ),
      max ((lo.val : ℚ) : ℝ) (-1) ≤ Real.sin x ∧ Real.sin x ≤ min ((hi.val : ℚ) : ℝ) 1 := by
  intro x hx
  obtain ⟨fa, ea⟩ := Dmath.sin_err ha hr.1
  obtain ⟨fb, eb⟩ := Dmath.sin_err hb hr.2
  have hab' : ((a.val : ℚ) : ℝ) ≤ ((b.val : ℚ) : ℝ) := Rat.cast_le.mpr hab
  have fh : (HALF).isFinite := by decide
  have cc := fun (k : ℤ) (hin : ((k : ℝ) + ((1 / 2) : ℚ)) * π ∈ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ)) =>
    crit_counted (k := k) ha hb hqa hqb fh HALF_val (by norm_num) (by norm_num) hr f1 f2 f3 f4 hqa' hqb' fsa fsb hin
  constructor
  · refine max_le ?_ (by linarith [Real.neg_one_le_sin x])
    by_cases E : ∃ k : ℤ, ⌈(sub qa HALF).val⌉ ≤ k ∧ k ≤ ⌊(sub qb HALF).val⌋ ∧ ¬ Even k
    · rw [hlo E]; push_cast; exact Real.neg_one_le_sin x
    · rw [hlo' E]
      have hno : ∀ k : ℤ, ¬ Even k → ((k : ℝ) + 1 / 2) * π ∉ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ) := by
        intro k hk hin
        exact E ⟨k, (cc k (by push_cast; simpa using hin)).1, (cc k (by push_cast; simpa using hin)).2, hk⟩
      have := ends_le_sin _ _ hab' hno x hx
      exact le_trans (ends_lo (va := Real.sin ((a.val : ℚ) : ℝ)) (vb := Real.sin ((b.val : ℚ) : ℝ)) fa fb hl0 ea eb hs0 hlo0) this
  · refine le_min ?_ (Real.sin_le_one x)
    by_cases E : ∃ k : ℤ, ⌈(sub qa HALF).val⌉ ≤ k ∧ k ≤ ⌊(sub qb HALF).val⌋ ∧ Even k
    · rw [hhi E]; push_cast; exact Real.sin_le_one x
    · rw [hhi' E]
      have hno : ∀ k : ℤ, Even k → ((k : ℝ) + 1 / 2) * π ∉ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ) := by
        intro k hk hin
        exact E ⟨k, (cc k (by push_cast; simpa using hin)).1, (cc k (by push_cast; simpa using hin)).2, hk⟩
      have := sin_le_ends _ _ hab' hno x hx
      exact le_trans this (ends_hi (va := Real.sin ((a.val : ℚ) : ℝ)) (vb := Real.sin ((b.val : ℚ) : ℝ)) fa fb hh0 ea eb ht0 hhi0)

set_option maxHeartbeats 1000000 in
/-- **`Interval::cos` encloses.** Every `cos x`, `x ∈ [a, b]`, lies in `[lo.max(−1), hi.min(1)]`. -/
theorem cos_encloses {a b lo0 hi0 qa qb lo hi : F64}
    (ha : a.isFinite) (hb : b.isFinite) (hab : a.val ≤ b.val) (hr : |a.val| ≤ 1048576 ∧ |b.val| ≤ 1048576)
    (hl0 : lo0.isFinite) (hs0 : (sub (Screen.fmin (Dmath.cos a) (Dmath.cos b)) TRIG_ERR).isFinite)
    (hlo0 : lo0.val < (sub (Screen.fmin (Dmath.cos a) (Dmath.cos b)) TRIG_ERR).val)
    (hh0 : hi0.isFinite) (ht0 : (add (Screen.fmax (Dmath.cos a) (Dmath.cos b)) TRIG_ERR).isFinite)
    (hhi0 : (add (Screen.fmax (Dmath.cos a) (Dmath.cos b)) TRIG_ERR).val < hi0.val)
    (hqa : qa.isFinite) (hqb : qb.isFinite)
    (f1 : (div a Dmath.PI).isFinite) (f2 : (div a PI_UP).isFinite)
    (f3 : (div b Dmath.PI).isFinite) (f4 : (div b PI_UP).isFinite)
    (hqa' : qa.val < (Screen.fmin (div a Dmath.PI) (div a PI_UP)).val)
    (hqb' : (Screen.fmax (div b Dmath.PI) (div b PI_UP)).val < qb.val)
    (fsa : (sub qa ZERO).isFinite) (fsb : (sub qb ZERO).isFinite)
    (hlo : (∃ k : ℤ, ⌈(sub qa ZERO).val⌉ ≤ k ∧ k ≤ ⌊(sub qb ZERO).val⌋ ∧ ¬ Even k) → lo.val = -1)
    (hlo' : ¬ (∃ k : ℤ, ⌈(sub qa ZERO).val⌉ ≤ k ∧ k ≤ ⌊(sub qb ZERO).val⌋ ∧ ¬ Even k) → lo.val = lo0.val)
    (hhi : (∃ k : ℤ, ⌈(sub qa ZERO).val⌉ ≤ k ∧ k ≤ ⌊(sub qb ZERO).val⌋ ∧ Even k) → hi.val = 1)
    (hhi' : ¬ (∃ k : ℤ, ⌈(sub qa ZERO).val⌉ ≤ k ∧ k ≤ ⌊(sub qb ZERO).val⌋ ∧ Even k) → hi.val = hi0.val) :
    ∀ x ∈ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ),
      max ((lo.val : ℚ) : ℝ) (-1) ≤ Real.cos x ∧ Real.cos x ≤ min ((hi.val : ℚ) : ℝ) 1 := by
  intro x hx
  obtain ⟨fa, ea⟩ := Dmath.cos_err ha hr.1
  obtain ⟨fb, eb⟩ := Dmath.cos_err hb hr.2
  have hab' : ((a.val : ℚ) : ℝ) ≤ ((b.val : ℚ) : ℝ) := Rat.cast_le.mpr hab
  have fh : (ZERO).isFinite := by decide
  have cc := fun (k : ℤ) (hin : ((k : ℝ) + ((0) : ℚ)) * π ∈ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ)) =>
    crit_counted (k := k) ha hb hqa hqb fh ZERO_val (by norm_num) (by norm_num) hr f1 f2 f3 f4 hqa' hqb' fsa fsb hin
  constructor
  · refine max_le ?_ (by linarith [Real.neg_one_le_cos x])
    by_cases E : ∃ k : ℤ, ⌈(sub qa ZERO).val⌉ ≤ k ∧ k ≤ ⌊(sub qb ZERO).val⌋ ∧ ¬ Even k
    · rw [hlo E]; push_cast; exact Real.neg_one_le_cos x
    · rw [hlo' E]
      have hno : ∀ k : ℤ, ¬ Even k → (k : ℝ) * π ∉ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ) := by
        intro k hk hin
        exact E ⟨k, (cc k (by push_cast; simpa using hin)).1, (cc k (by push_cast; simpa using hin)).2, hk⟩
      have := ends_le_cos _ _ hab' hno x hx
      exact le_trans (ends_lo (va := Real.cos ((a.val : ℚ) : ℝ)) (vb := Real.cos ((b.val : ℚ) : ℝ)) fa fb hl0 ea eb hs0 hlo0) this
  · refine le_min ?_ (Real.cos_le_one x)
    by_cases E : ∃ k : ℤ, ⌈(sub qa ZERO).val⌉ ≤ k ∧ k ≤ ⌊(sub qb ZERO).val⌋ ∧ Even k
    · rw [hhi E]; push_cast; exact Real.cos_le_one x
    · rw [hhi' E]
      have hno : ∀ k : ℤ, Even k → (k : ℝ) * π ∉ Set.Icc ((a.val : ℚ) : ℝ) ((b.val : ℚ) : ℝ) := by
        intro k hk hin
        exact E ⟨k, (cc k (by push_cast; simpa using hin)).1, (cc k (by push_cast; simpa using hin)).2, hk⟩
      have := cos_le_ends _ _ hab' hno x hx
      exact le_trans this (ends_hi (va := Real.cos ((a.val : ℚ) : ℝ)) (vb := Real.cos ((b.val : ℚ) : ℝ)) fa fb hh0 ea eb ht0 hhi0)

end IEEE754.Interval
