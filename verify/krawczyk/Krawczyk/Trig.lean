import Mathlib

/-!
# Interval sin and cos: the range rule

forge_math::Interval::sin over [a, b] takes the values at a and b, then 1 if a maximum
(k + 1/2)π with k even may lie inside, and -1 if a minimum (k odd) may. cos does the same with kπ.
Each bound is then pushed out by an absolute 2^-52 and rounded outward.

This file proves the rule over ℝ. sin on [a, b] is at most max(sin a, sin b) unless an even
(k + 1/2)π lies in [a, b], and at least min(sin a, sin b) unless an odd one does. Same for cos at
kπ. The proof: sin attains its maximum on [a, b] at some c. If c is inside, cos c = 0, so
c = (k + 1/2)π and sin c = (-1)^k. For an even k that contradicts the hypothesis; for an odd k the
maximum is -1, so every value is at most sin a.

The float side has two parts. The kernel's test for a critical point uses an enclosure of
(k + 1/2)π that contains it, so it never misses one (`crit_enclosed`). The endpoint values are
dmath's, within an absolute 2^-52 of the true values for |x| ≤ 2^20 (TRIG_MOST): verify/ieee754's
`Dmath.sin_err` and `Dmath.cos_err` (the IEEE branch). forge-math's `trig_within_the_contract`
also checks this against a double-double reference, measured worst 8.4e-17. Each end is pushed out
by that absolute 2^-52 and then rounded outward (`Krawczyk.nearest_between`), so it encloses the
true value. The join is proven on claude/repo-review-mbdrlj-ieee at 746ecf8 (IEEE754/Interval/Trig.lean,
`sin_encloses` and `cos_encloses`, which use this file's `crit_quotients`).
-/

namespace Krawczyk

open Real Set

/-- sin at an odd multiple of π/2. -/
theorem sin_half_odd (k : ℤ) : sin ((k + 1 / 2) * π) = if Even k then 1 else -1 := by
  have h : ((k : ℝ) + 1 / 2) * π = k * π + π / 2 := by ring
  rw [h, sin_add_pi_div_two]
  rcases Int.even_or_odd k with ⟨m, rfl⟩ | ⟨m, rfl⟩
  · have : ((m + m : ℤ) : ℝ) * π = (m : ℝ) * (2 * π) := by push_cast; ring
    rw [this, cos_int_mul_two_pi, if_pos ⟨m, rfl⟩]
  · have : ((2 * m + 1 : ℤ) : ℝ) * π = (m : ℝ) * (2 * π) + π := by push_cast; ring
    rw [this, cos_add_pi, cos_int_mul_two_pi, if_neg (Int.not_even_iff_odd.mpr ⟨m, rfl⟩)]

/-- Where cos vanishes. -/
theorem cos_zero_half (c : ℝ) (h : cos c = 0) : ∃ k : ℤ, c = (k + 1 / 2) * π := by
  obtain ⟨k, hk⟩ := cos_eq_zero_iff.mp h
  exact ⟨k, by rw [hk]; ring⟩

/-- An interior extreme of sin is at a zero of cos. -/
theorem interior_extreme {a b c : ℝ} (hc : c ∈ Ioo a b)
    (hext : IsMaxOn sin (Icc a b) c ∨ IsMinOn sin (Icc a b) c) :
    ∃ k : ℤ, c = (k + 1 / 2) * π := by
  have hn : Icc a b ∈ nhds c := Icc_mem_nhds hc.1 hc.2
  have hd : deriv sin c = 0 := by
    rcases hext with h | h
    · exact (h.isLocalMax hn).deriv_eq_zero
    · exact (h.isLocalMin hn).deriv_eq_zero
  rw [Real.deriv_sin] at hd
  exact cos_zero_half c hd

/-- **Upper bound.** With no even (k + 1/2)π in [a, b], sin there is at most max(sin a, sin b). -/
theorem sin_le_ends (a b : ℝ) (hab : a ≤ b)
    (hno : ∀ k : ℤ, Even k → ((k : ℝ) + 1 / 2) * π ∉ Icc a b) :
    ∀ x ∈ Icc a b, sin x ≤ max (sin a) (sin b) := by
  obtain ⟨c, hc, hmax⟩ := isCompact_Icc.exists_isMaxOn (nonempty_Icc.mpr hab)
    continuous_sin.continuousOn
  intro x hx
  have hxc : sin x ≤ sin c := hmax hx
  rcases eq_or_lt_of_le hc.1 with h | h
  · rw [← h] at hxc; exact le_trans hxc (le_max_left _ _)
  rcases eq_or_lt_of_le hc.2 with h' | h'
  · rw [h'] at hxc; exact le_trans hxc (le_max_right _ _)
  obtain ⟨k, hk⟩ := interior_extreme ⟨h, h'⟩ (Or.inl hmax)
  by_cases he : Even k
  · exact absurd (hk ▸ hc) (hno k he)
  · have : sin c = -1 := by rw [hk, sin_half_odd, if_neg he]
    have ha : -1 ≤ sin a := neg_one_le_sin a
    linarith [le_max_left (sin a) (sin b)]

/-- **Lower bound.** With no odd (k + 1/2)π in [a, b], sin there is at least min(sin a, sin b). -/
theorem ends_le_sin (a b : ℝ) (hab : a ≤ b)
    (hno : ∀ k : ℤ, ¬ Even k → ((k : ℝ) + 1 / 2) * π ∉ Icc a b) :
    ∀ x ∈ Icc a b, min (sin a) (sin b) ≤ sin x := by
  obtain ⟨c, hc, hmin⟩ := isCompact_Icc.exists_isMinOn (nonempty_Icc.mpr hab)
    continuous_sin.continuousOn
  intro x hx
  have hxc : sin c ≤ sin x := hmin hx
  rcases eq_or_lt_of_le hc.1 with h | h
  · rw [← h] at hxc; exact le_trans (min_le_left _ _) hxc
  rcases eq_or_lt_of_le hc.2 with h' | h'
  · rw [h'] at hxc; exact le_trans (min_le_right _ _) hxc
  obtain ⟨k, hk⟩ := interior_extreme ⟨h, h'⟩ (Or.inr hmin)
  by_cases he : Even k
  · have : sin c = 1 := by rw [hk, sin_half_odd, if_pos he]
    have ha : sin a ≤ 1 := sin_le_one a
    linarith [min_le_left (sin a) (sin b)]
  · exact absurd (hk ▸ hc) (hno k he)

/-- **cos**, from sin by the π/2 shift: its maxima are the even multiples kπ, its minima the odd. -/
theorem cos_le_ends (a b : ℝ) (hab : a ≤ b)
    (hno : ∀ k : ℤ, Even k → (k : ℝ) * π ∉ Icc a b) :
    ∀ x ∈ Icc a b, cos x ≤ max (cos a) (cos b) := by
  intro x hx
  have key := sin_le_ends (a + π / 2) (b + π / 2) (by linarith)
    (fun k he hk => hno k he ⟨by linarith [hk.1], by linarith [hk.2]⟩) (x + π / 2)
    ⟨by linarith [hx.1], by linarith [hx.2]⟩
  simpa only [sin_add_pi_div_two] using key

theorem ends_le_cos (a b : ℝ) (hab : a ≤ b)
    (hno : ∀ k : ℤ, ¬ Even k → (k : ℝ) * π ∉ Icc a b) :
    ∀ x ∈ Icc a b, min (cos a) (cos b) ≤ cos x := by
  intro x hx
  have key := ends_le_sin (a + π / 2) (b + π / 2) (by linarith)
    (fun k he hk => hno k he ⟨by linarith [hk.1], by linarith [hk.2]⟩) (x + π / 2)
    ⟨by linarith [hx.1], by linarith [hx.2]⟩
  simpa only [sin_add_pi_div_two] using key

/-- The kernel's critical-point test never misses one: m·π lies between m·PI and m·PI_UP (the
floats either side of π), so an enclosure [pl, ph] of those products, rounded outward, holds m·π.
If m·π is in [a, b], the enclosure meets [a, b], and the test counts the extreme. -/
theorem crit_enclosed (a b m lo hi pl ph : ℝ) (hlo : lo < π) (hhi : π < hi)
    (hpl : pl ≤ min (m * lo) (m * hi)) (hph : max (m * lo) (m * hi) ≤ ph)
    (hin : m * π ∈ Icc a b) : pl ≤ b ∧ a ≤ ph := by
  have hbetween : min (m * lo) (m * hi) ≤ m * π ∧ m * π ≤ max (m * lo) (m * hi) := by
    rcases le_total 0 m with h | h
    · have h1 : m * lo ≤ m * π := mul_le_mul_of_nonneg_left hlo.le h
      have h2 : m * π ≤ m * hi := mul_le_mul_of_nonneg_left hhi.le h
      exact ⟨le_trans (min_le_left _ _) h1, le_trans h2 (le_max_right _ _)⟩
    · have h1 : m * hi ≤ m * π := mul_le_mul_of_nonpos_left hhi.le h
      have h2 : m * π ≤ m * lo := mul_le_mul_of_nonpos_left hlo.le h
      exact ⟨le_trans (min_le_right _ _) h1, le_trans h2 (le_max_left _ _)⟩
  exact ⟨by linarith [hin.2, hbetween.1], by linarith [hin.1, hbetween.2]⟩

/-- The kernel's critical-point test in the form it is computed: k + half counts when it lies in
[qa, qb], qa at most a's quotients by both floats either side of π, qb at least b's. An extreme
m·π inside [a, b] always has qa ≤ m ≤ qb, so it is never missed. -/
theorem crit_quotients (a b m lo hi qa qb : ℝ) (hlo : 0 < lo) (hlo' : lo < π) (hhi : π < hi)
    (hqa : qa ≤ min (a / lo) (a / hi)) (hqb : max (b / lo) (b / hi) ≤ qb)
    (hin : m * π ∈ Icc a b) : qa ≤ m ∧ m ≤ qb := by
  have hpi : 0 < π := pi_pos
  have hm1 : a / π ≤ m := by rw [div_le_iff₀ hpi]; exact hin.1
  have hm2 : m ≤ b / π := by rw [le_div_iff₀ hpi]; exact hin.2
  -- a / π lies between a / hi and a / lo, whatever a's sign
  have between (x : ℝ) : min (x / lo) (x / hi) ≤ x / π ∧ x / π ≤ max (x / lo) (x / hi) := by
    have h1 : 1 / hi < 1 / π := one_div_lt_one_div_of_lt hpi hhi
    have h2 : 1 / π < 1 / lo := one_div_lt_one_div_of_lt hlo hlo'
    have e : ∀ y : ℝ, x / y = x * (1 / y) := fun y => by ring
    rw [e lo, e hi, e π]
    rcases le_total 0 x with hx | hx
    · exact ⟨le_trans (min_le_right _ _) (mul_le_mul_of_nonneg_left h1.le hx),
        le_trans (mul_le_mul_of_nonneg_left h2.le hx) (le_max_left _ _)⟩
    · exact ⟨le_trans (min_le_left _ _) (mul_le_mul_of_nonpos_left h2.le hx),
        le_trans (mul_le_mul_of_nonpos_left h1.le hx) (le_max_right _ _)⟩
  exact ⟨le_trans hqa (le_trans (between a).1 hm1), le_trans hm2 (le_trans (between b).2 hqb)⟩

end Krawczyk
