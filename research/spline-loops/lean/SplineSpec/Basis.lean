/-
The B-spline basis of `SplineSpec.Defs`: on each span the p + 1 functions that can be nonzero sum to
one (for any t: the span's polynomials), are nonnegative on the span, and are zero elsewhere; so a
rational B-spline with positive weights lies in the convex hull of the span's poles
(`eval_between`, what the kernel's boxes and hull tests rely on). With clamped ends the curve starts
at its first pole and ends at its last exactly (`clamped_start`, `clamped_end`), so a loop of such
curves closes exactly.
-/
import SplineSpec.Defs
import Mathlib.Algebra.Order.Field.Basic
import Mathlib.Algebra.BigOperators.Intervals
import Mathlib.Tactic

open Finset

namespace SplineSpec

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LinearOrder K] [IsStrictOrderedRing K]

theorem sumTo_eq (n : ℕ) (f : ℕ → K) : sumTo n f = ∑ i ∈ range n, f i := by
  induction n with
  | zero => rfl
  | succ n ih => simp [sumTo, ih, sum_range_succ]

/-- Only `N_(j-k,k) .. N_(j,k)` can be nonzero on span j: zero right of j. -/
theorem nb_zero_right (u : ℕ → K) (j : ℕ) : ∀ k i (t : K), j < i → nb u j i k t = 0 := by
  intro k
  induction k with
  | zero => intro i t h; simp [nb]; omega
  | succ k ih =>
    intro i t h
    simp only [nb, ih i t h, ih (i + 1) t (by omega), mul_zero, add_zero]

/-- And zero left of j - k. -/
theorem nb_zero_left (u : ℕ → K) (j : ℕ) : ∀ k i (t : K), i + k < j → nb u j i k t = 0 := by
  intro k
  induction k with
  | zero => intro i t h; simp [nb]; omega
  | succ k ih =>
    intro i t h
    simp only [nb, ih i t (by omega), ih (i + 1) t (by omega), mul_zero, add_zero]

/-- **Partition of unity**: on span j the functions sum to one, for any t (they are the span's
polynomials), any degree k ≤ j, over any range of indices past j. Degenerate knot differences
take nothing away: each ratio is added and taken back. -/
theorem nb_sum_one (u : ℕ → K) (j : ℕ) : ∀ k, k ≤ j → ∀ N, j < N → ∀ t : K,
    ∑ i ∈ range N, nb u j i k t = 1 := by
  intro k
  induction k with
  | zero =>
    intro _ N hN t
    simp only [nb]
    rw [sum_ite_eq' (range N) j (fun _ => (1 : K))]
    simp [hN]
  | succ k ih =>
    intro hk N hN t
    simp only [nb]
    rw [sum_add_distrib]
    -- shift the second sum: Σ_(i<N) (1 - ω(i+1)) N(i+1) = Σ_(i<N+1) (1 - ω i) N i - (1 - ω 0) N 0
    have shift : ∑ i ∈ range N, (1 - omega u (i + 1) (k + 1) t) * nb u j (i + 1) k t =
        ∑ i ∈ range (N + 1), (1 - omega u i (k + 1) t) * nb u j i k t
          - (1 - omega u 0 (k + 1) t) * nb u j 0 k t := by
      rw [sum_range_succ']; ring
    rw [shift, sum_range_succ, nb_zero_right u j k N t hN, nb_zero_left u j k 0 t (by omega)]
    simp only [mul_zero, add_zero, sub_zero]
    rw [← sum_add_distrib]
    exact (sum_congr rfl fun i _ => by ring).trans (ih (by omega) N hN t)

/-- **Nonnegativity** on the span `[u j, u (j+1)]` of a nondecreasing knot sequence. -/
theorem nb_nonneg (u : ℕ → K) (hu : Monotone u) (j : ℕ) {t : K} (h0 : u j ≤ t) (h1 : t ≤ u (j + 1)) :
    ∀ k i, 0 ≤ nb u j i k t := by
  intro k
  induction k with
  | zero => intro i; simp only [nb]; split_ifs <;> norm_num
  | succ k ih =>
    intro i
    simp only [nb]
    -- the first term: nonzero only for i ≤ j, where t ≥ u i
    have a : 0 ≤ omega u i (k + 1) t * nb u j i k t := by
      by_cases hij : j < i
      · rw [nb_zero_right u j k i t hij, mul_zero]
      · refine mul_nonneg ?_ (ih i)
        unfold omega
        split_ifs with hd
        · exact le_refl _
        · apply div_nonneg
          · have := hu (show i ≤ j by omega); linarith
          · have := hu (show i ≤ i + (k + 1) by omega); linarith
    -- the second: nonzero only for i + 1 + k ≥ j, where t ≤ u (i + 1 + k + 1)
    have b : 0 ≤ (1 - omega u (i + 1) (k + 1) t) * nb u j (i + 1) k t := by
      by_cases hij : i + 1 + k < j
      · rw [nb_zero_left u j k (i + 1) t hij, mul_zero]
      · refine mul_nonneg ?_ (ih (i + 1))
        unfold omega
        split_ifs with hd
        · norm_num
        · have hlt : u (i + 1) < u (i + 1 + (k + 1)) :=
            lt_of_le_of_ne (hu (by omega)) (Ne.symm hd)
          rw [sub_nonneg, div_le_one (by linarith)]
          have := hu (show j + 1 ≤ i + 1 + (k + 1) by omega)
          linarith
    linarith

/-- The sum over the span's poles: `Σ_(i<n) N_i f_i`. -/
theorem homOn_eq (u : ℕ → K) (p n j : ℕ) (w P : ℕ → K) (t : K) :
    homOn u p n j w P t = ∑ i ∈ range n, nb u j i p t * (w i * P i) := sumTo_eq _ _

theorem weightOn_eq (u : ℕ → K) (p n j : ℕ) (w : ℕ → K) (t : K) :
    weightOn u p n j w t = ∑ i ∈ range n, nb u j i p t * w i := sumTo_eq _ _

/-- The weight on a span is positive: the basis sums to one and is nonnegative there. -/
theorem weightOn_pos (u : ℕ → K) (hu : Monotone u) (p n j : ℕ) (hj : p ≤ j) (hjn : j < n) (w : ℕ → K)
    (hw : ∀ i < n, 0 < w i) {t : K} (h0 : u j ≤ t) (h1 : t ≤ u (j + 1)) :
    0 < weightOn u p n j w t := by
  rw [weightOn_eq]
  -- Σ N w ≥ (min w) Σ N = min w
  obtain ⟨k, hk, hmin⟩ := exists_min_image (range n) w ⟨0, mem_range.2 (by omega)⟩
  have hk' := mem_range.1 hk
  calc 0 < w k := hw k hk'
    _ = ∑ i ∈ range n, nb u j i p t * w k := by
        rw [← sum_mul, nb_sum_one u j p hj n hjn t, one_mul]
    _ ≤ ∑ i ∈ range n, nb u j i p t * w i := by
        refine sum_le_sum fun i hi => mul_le_mul_of_nonneg_left (hmin i hi) (nb_nonneg u hu j h0 h1 p i)

/-- **Convex hull.** With positive weights, a coordinate of the curve on span j lies between any
bounds of that coordinate of the poles. (Poles outside `[j - p, j]` have no weight there.) -/
theorem eval_between (u : ℕ → K) (hu : Monotone u) (p n j : ℕ) (hj : p ≤ j) (hjn : j < n)
    (w P : ℕ → K) (hw : ∀ i < n, 0 < w i) {t : K} (h0 : u j ≤ t) (h1 : t ≤ u (j + 1)) (a b : K)
    (hab : ∀ i < n, a ≤ P i ∧ P i ≤ b) :
    a ≤ homOn u p n j w P t / weightOn u p n j w t ∧ homOn u p n j w P t / weightOn u p n j w t ≤ b := by
  have hW := weightOn_pos u hu p n j hj hjn w hw h0 h1
  rw [le_div_iff₀ hW, div_le_iff₀ hW, homOn_eq, weightOn_eq]
  simp only [mul_sum]
  constructor
  · refine sum_le_sum fun i hi => ?_
    have := nb_nonneg u hu j h0 h1 p i
    have hi' := mem_range.1 hi
    have := mul_le_mul_of_nonneg_left (hab i hi').1 (mul_nonneg this (le_of_lt (hw i hi')))
    linarith
  · refine sum_le_sum fun i hi => ?_
    have := nb_nonneg u hu j h0 h1 p i
    have hi' := mem_range.1 hi
    have := mul_le_mul_of_nonneg_left (hab i hi').2 (mul_nonneg this (le_of_lt (hw i hi')))
    linarith

/-- At a clamped start, `u 0 = .. = u p < u (p+1)`, only the first function is nonzero at `u p`. -/
theorem nb_clamped_start (u : ℕ → K) (p : ℕ) (hc : ∀ i ≤ p, u i = u p) :
    ∀ k ≤ p, ∀ i, nb u p i k (u p) = if i = p - k then 1 else 0 := by
  intro k
  induction k with
  | zero => intro _ i; simp [nb]
  | succ k ih =>
    intro hk i
    simp only [nb]
    rw [ih (by omega) i, ih (by omega) (i + 1)]
    have hz : ∀ l, l ≤ p → omega u l (k + 1) (u p) = 0 := by
      intro l hl
      unfold omega
      rw [hc l hl]
      split_ifs <;> simp
    by_cases h1 : i = p - k
    · subst h1
      rw [hz (p - k) (by omega)]
      have : p - k ≠ p - (k + 1) := by omega
      have h3 : p - k + 1 ≠ p - k := by omega
      simp [this, h3]
    · by_cases h2 : i + 1 = p - k
      · have hi : i = p - (k + 1) := by omega
        rw [h2, hz (p - k) (by omega)]
        have h4 : p - (k + 1) ≠ p - k := by omega
        simp [h1, hi, h4]
      · have : i ≠ p - (k + 1) := by omega
        simp [h1, h2, this]

/-- **A clamped curve starts at its first pole**, exactly. -/
theorem clamped_start (u : ℕ → K) (p n : ℕ) (hpn : p < n) (hc : ∀ i ≤ p, u i = u p) (w P : ℕ → K)
    (hw : 0 < w 0) : homOn u p n p w P (u p) / weightOn u p n p w (u p) = P 0 := by
  rw [homOn_eq, weightOn_eq]
  simp only [nb_clamped_start u p hc p le_rfl, Nat.sub_self, ite_mul, one_mul, zero_mul]
  rw [sum_ite_eq' (range n) 0, sum_ite_eq' (range n) 0]
  simp only [mem_range, show 0 < n by omega, if_true, one_mul]
  field_simp

/-- At a clamped end, `u (n-1) < u n = .. = u (n+p)`, only the last function is nonzero at `u n`
on the last span. -/
theorem nb_clamped_end (u : ℕ → K) (p n : ℕ) (hn : 0 < n) (hc : ∀ i, n ≤ i → i ≤ n + p → u i = u n)
    (hlast : u (n - 1) < u n) :
    ∀ k ≤ p, ∀ i, nb u (n - 1) i k (u n) = if i = n - 1 then 1 else 0 := by
  intro k
  induction k with
  | zero => intro _ i; simp [nb]
  | succ k ih =>
    intro hk i
    simp only [nb]
    rw [ih (by omega) i, ih (by omega) (i + 1)]
    have hone : omega u (n - 1) (k + 1) (u n) = 1 := by
      unfold omega
      have e1 : u (n - 1 + (k + 1)) = u n := hc _ (by omega) (by omega)
      rw [e1]
      have hne : u n ≠ u (n - 1) := ne_of_gt hlast
      rw [if_neg hne, div_self (sub_ne_zero.2 hne)]
    by_cases h1 : i = n - 1
    · subst h1
      have : n - 1 + 1 ≠ n - 1 := by omega
      simp [hone, this]
    · by_cases h2 : i + 1 = n - 1
      · rw [h2, hone]; simp [h1]
      · simp [h1, h2]

/-- **A clamped curve ends at its last pole**, exactly. -/
theorem clamped_end (u : ℕ → K) (p n : ℕ) (hpn : p < n) (hc : ∀ i, n ≤ i → i ≤ n + p → u i = u n)
    (hlast : u (n - 1) < u n) (w P : ℕ → K) (hw : 0 < w (n - 1)) :
    homOn u p n (n - 1) w P (u n) / weightOn u p n (n - 1) w (u n) = P (n - 1) := by
  rw [homOn_eq, weightOn_eq]
  simp only [nb_clamped_end u p n (by omega) hc hlast p le_rfl, ite_mul, one_mul, zero_mul]
  rw [sum_ite_eq' (range n) (n - 1), sum_ite_eq' (range n) (n - 1)]
  simp only [mem_range, show n - 1 < n by omega, if_true]
  field_simp

end SplineSpec
