/-
**Continuity at a knot**: where the knot `x = u (j+1) = .. = u (j+k)` has multiplicity k ≤ p,
the curve's polynomials on the span before it (j) and after it (j + k) agree at x in value and in
every derivative up to order p - k (`curve_smooth`): the curve is C^(p-k) there.

The proof: the r-th derivative on any span is the same B-spline of degree p - r on that span,
its poles the r-times differenced poles, which do not depend on the span (`iter_curveOn`, from
`hasDerivAt_curve`); and a basis function of degree q ≥ k takes one value at x from either side
(`nb_continuous`): on the left every function but `N_j` is zero there, on the right the same once
q reaches k, and the recurrence keeps both sides alike from there.
-/
import SplineSpec.Deriv
import Mathlib.Analysis.Calculus.IteratedDeriv.Defs

open Finset

namespace SplineSpec

/-- On the span before a knot of multiplicity k, at the knot only `N_j` is nonzero, up to degree k. -/
theorem nb_left_end (u : ℕ → ℝ) (j k : ℕ) (hj : u j < u (j + 1))
    (hk : ∀ l, 1 ≤ l → l ≤ k → u (j + l) = u (j + 1)) :
    ∀ q, q ≤ k → ∀ i, nb u j i q (u (j + 1)) = if i = j then 1 else 0 := by
  intro q
  induction q with
  | zero => intro _ i; simp [nb]
  | succ q ih =>
    intro hq i
    simp only [nb]
    rw [ih (by omega) i, ih (by omega) (i + 1)]
    have hone : omega u j (q + 1) (u (j + 1)) = 1 := by
      rw [omega_eq, hk (q + 1) (by omega) hq]
      exact div_self (sub_ne_zero.2 (ne_of_gt hj))
    by_cases h1 : i = j
    · subst h1; simp [hone]
    · by_cases h2 : i + 1 = j
      · rw [h2, hone]; simp [h1]
      · simp [h1, h2]

/-- On the span after it, at the knot `N_(j+k-q)` alone, below degree k. -/
theorem nb_right_start (u : ℕ → ℝ) (j k : ℕ) (hk : ∀ l, 1 ≤ l → l ≤ k → u (j + l) = u (j + 1)) :
    ∀ q, q < k → ∀ i, nb u (j + k) i q (u (j + 1)) = if i = j + k - q then 1 else 0 := by
  intro q
  induction q with
  | zero => intro _ i; simp [nb]
  | succ q ih =>
    intro hq i
    simp only [nb]
    rw [ih (by omega) i, ih (by omega) (i + 1)]
    have hz : omega u (j + k - q) (q + 1) (u (j + 1)) = 0 := by
      rw [omega_eq, ← hk (k - q) (by omega) (by omega)]
      have : j + (k - q) = j + k - q := by omega
      rw [this, sub_self, zero_div]
    by_cases h1 : i = j + k - q
    · subst h1
      rw [hz]
      have : j + k - q ≠ j + k - (q + 1) := by omega
      have h3 : j + k - q + 1 ≠ j + k - q := by omega
      simp [this, h3]
    · by_cases h2 : i + 1 = j + k - q
      · rw [h2, hz]
        have h4 : i = j + k - (q + 1) := by omega
        have h5 : j + k - (q + 1) ≠ j + k - q := by omega
        simp [h1, h4, h5]
      · have : i ≠ j + k - (q + 1) := by omega
        simp [h1, h2, this]

/-- **A basis function of degree q ≥ k takes one value at a knot of multiplicity k from either
side.** -/
theorem nb_continuous (u : ℕ → ℝ) (j k : ℕ) (hk1 : 1 ≤ k) (hj : u j < u (j + 1))
    (hk : ∀ l, 1 ≤ l → l ≤ k → u (j + l) = u (j + 1)) :
    ∀ q, k ≤ q → ∀ i, nb u j i q (u (j + 1)) = nb u (j + k) i q (u (j + 1)) := by
  intro q hq
  induction q with
  | zero => omega
  | succ q ih =>
    intro i
    by_cases hqk : q + 1 = k
    · -- the first degree that agrees: both sides are N_j alone
      rw [nb_left_end u j k hj hk (q + 1) (by omega) i]
      simp only [nb]
      rw [nb_right_start u j k hk q (by omega) i, nb_right_start u j k hk q (by omega) (i + 1)]
      have hz : omega u (j + 1) (q + 1) (u (j + 1)) = 0 := by rw [omega_eq, sub_self, zero_div]
      have e : j + k - q = j + 1 := by omega
      rw [e]
      by_cases h1 : i = j
      · subst h1; simp [hz]
      · by_cases h2 : i = j + 1
        · subst h2; simp [hz]
        · have : i + 1 ≠ j + 1 := by omega
          simp [h1, h2, this]
    · simp only [nb]
      rw [ih (by omega) i, ih (by omega) (i + 1)]

/-- The curve on span j, one coordinate: `Σ_(i<n) N_(i,q) P_i`. -/
noncomputable def curveOn (u : ℕ → ℝ) (n j q : ℕ) (P : ℕ → ℝ) (t : ℝ) : ℝ :=
  ∑ i ∈ range n, nb u j i q t * P i

/-- The poles differenced r times, for a curve of degree p: those of its r-th derivative. -/
noncomputable def diffPoles (u : ℕ → ℝ) (p : ℕ) (P : ℕ → ℝ) : ℕ → ℕ → ℝ
  | 0 => P
  | r + 1 => fun i => ((p - r : ℕ) : ℝ) * (diffPoles u p P r i - diffPoles u p P r (i - 1))
      / (u (i + (p - r)) - u i)

/-- The r-th derivative of the curve on a span is the curve of degree p - r on the differenced
poles, on the same span. -/
theorem iter_curveOn (u : ℕ → ℝ) (hu : Monotone u) (p n j : ℕ) (hj : u j < u (j + 1)) (hpj : p ≤ j)
    (hjn : j < n) (P : ℕ → ℝ) :
    ∀ r ≤ p, iteratedDeriv r (curveOn u n j p P) = curveOn u n j (p - r) (diffPoles u p P r) := by
  intro r
  induction r with
  | zero => intro _; simp [diffPoles]
  | succ r ih =>
    intro hr
    rw [iteratedDeriv_succ, ih (by omega)]
    funext t
    have h := hasDerivAt_curve u hu (p - r - 1) n j hj (by omega) hjn (diffPoles u p P r) t
    have e : p - r - 1 + 1 = p - r := by omega
    rw [e] at h
    unfold curveOn
    rw [h.deriv]
    refine sum_congr rfl fun i _ => ?_
    simp only [diffPoles]
    have e2 : p - (r + 1) = p - r - 1 := by omega
    have e3 : i + (p - r - 1) + 1 = i + (p - r) := by omega
    rw [e2, e3]
    congr 1
    push_cast [show r ≤ p by omega, show 1 ≤ p - r by omega]
    ring

/-- **C^(p-k) at a knot of multiplicity k ≤ p**: the polynomials of the spans either side of the
knot agree there in value and in every derivative of order r ≤ p - k. -/
theorem curve_smooth (u : ℕ → ℝ) (hu : Monotone u) (p n j k : ℕ) (hk1 : 1 ≤ k) (hj : u j < u (j + 1))
    (hk : ∀ l, 1 ≤ l → l ≤ k → u (j + l) = u (j + 1)) (hnext : u (j + k) < u (j + k + 1))
    (hpj : p ≤ j) (hjn : j + k < n) (P : ℕ → ℝ) (r : ℕ) (hr : r + k ≤ p) :
    iteratedDeriv r (curveOn u n j p P) (u (j + 1)) =
      iteratedDeriv r (curveOn u n (j + k) p P) (u (j + 1)) := by
  rw [iter_curveOn u hu p n j hj hpj (by omega) P r (by omega),
    iter_curveOn u hu p n (j + k) hnext (by omega) hjn P r (by omega)]
  unfold curveOn
  refine sum_congr rfl fun i _ => ?_
  rw [nb_continuous u j k hk1 hj hk (p - r) (by omega) i]

end SplineSpec
