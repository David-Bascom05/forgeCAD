/-
The kernel's tridiagonal solve (kernel/forge-geom/src/spline.rs, `tridiagonal`), line by line, in
exact arithmetic: on row-dominant rows (`|b_i| > |a_i| + |c_i|`, which the open slope system's are:
`rows_dominant`) every pivot is nonzero and the result satisfies every row (`thomas_solves`). So
the algorithm, not only its output on the oracle's cases, computes the spec's slopes; its rounding
is the business of research/spline-loops/fp.

Rows `i < n` are `a_i x_(i-1) + b_i x_i + c_i x_(i+1) = d_i` with `a_0 = 0` and `c_(n-1) = 0`.
-/
import SplineSpec.Slopes

namespace SplineSpec

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LinearOrder K] [IsStrictOrderedRing K]

/-- The forward sweep: `(c'_i, d'_i)`, row i reduced to `x_i + c'_i x_(i+1) = d'_i`. -/
def sweep (a b c d : ℕ → K) : ℕ → K × K
  | 0 => (c 0 / b 0, d 0 / b 0)
  | i + 1 =>
    let (cp, dp) := sweep a b c d i
    let m := b (i + 1) - a (i + 1) * cp
    (c (i + 1) / m, (d (i + 1) - dp * a (i + 1)) / m)

/-- The back substitution from row n - 1 down: `x_(n-1) = d'_(n-1)`, `x_i = d'_i - c'_i x_(i+1)`. -/
def backSub (a b c d : ℕ → K) (n : ℕ) : ℕ → K
  | k =>
    if h : k + 1 < n then (sweep a b c d k).2 - (sweep a b c d k).1 * backSub a b c d n (k + 1)
    else (sweep a b c d k).2
  termination_by k => n - k

/-- The pivot of row i: `b_0`, then `b_i - a_i c'_(i-1)`. -/
def pivot (a b c d : ℕ → K) : ℕ → K
  | 0 => b 0
  | i + 1 => b (i + 1) - a (i + 1) * (sweep a b c d i).1

/-- Row dominance keeps every reduced `|c'_i| < 1` and every pivot past `|c_i|`, so nonzero. -/
theorem sweep_bound (a b c d : ℕ → K) (n : ℕ) (hdom : ∀ i < n, |a i| + |c i| < |b i|)
    (ha0 : a 0 = 0) : ∀ i < n, |c i| < |pivot a b c d i| ∧ |(sweep a b c d i).1| < 1 ∨ c i = 0 ∧
      |c i| < |pivot a b c d i| := by
  intro i hi
  induction i with
  | zero =>
    have h := hdom 0 hi
    rw [ha0, abs_zero, zero_add] at h
    have hb : 0 < |b 0| := lt_of_le_of_lt (abs_nonneg _) h
    left
    refine ⟨by simpa [pivot] using h, ?_⟩
    simp only [sweep]
    rw [abs_div, div_lt_one hb]; exact h
  | succ k ih =>
    have h := hdom (k + 1) hi
    have hk : |(sweep a b c d k).1| ≤ 1 := by
      rcases ih (by omega) with ⟨_, h1⟩ | ⟨hc, hp⟩
      · exact h1.le
      · -- c'_k = 0 / pivot = 0
        rcases k with _ | k
        · simp [sweep, hc]
        · simp [sweep, hc]
    have hp : |c (k + 1)| < |pivot a b c d (k + 1)| := by
      simp only [pivot]
      have : |a (k + 1) * (sweep a b c d k).1| ≤ |a (k + 1)| := by
        rw [abs_mul]; exact mul_le_of_le_one_right (abs_nonneg _) hk
      have := abs_sub_abs_le_abs_sub (b (k + 1)) (a (k + 1) * (sweep a b c d k).1)
      linarith
    left
    refine ⟨hp, ?_⟩
    have hpos : 0 < |pivot a b c d (k + 1)| := lt_of_le_of_lt (abs_nonneg _) hp
    have e : (sweep a b c d (k + 1)).1 = c (k + 1) / pivot a b c d (k + 1) := by
      simp [sweep, pivot]
    rw [e, abs_div, div_lt_one hpos]; exact hp

theorem pivot_ne (a b c d : ℕ → K) (n : ℕ) (hdom : ∀ i < n, |a i| + |c i| < |b i|) (ha0 : a 0 = 0)
    {i : ℕ} (hi : i < n) : pivot a b c d i ≠ 0 := by
  intro h0
  rcases sweep_bound a b c d n hdom ha0 i hi with ⟨h, _⟩ | ⟨_, h⟩ <;>
    · rw [h0, abs_zero] at h; exact absurd h (not_lt.2 (abs_nonneg _))

/-- What the sweep means: each row, reduced, is `x_i + c'_i x_(i+1) = d'_i` for any x solving the
rows up to i. -/
theorem sweep_reduces (a b c d : ℕ → K) (n : ℕ) (hdom : ∀ i < n, |a i| + |c i| < |b i|) (ha0 : a 0 = 0)
    (x : ℕ → K) : ∀ i < n,
      (∀ j ≤ i, a j * x (j - 1) + b j * x j + c j * x (j + 1) = d j) →
      x i + (sweep a b c d i).1 * x (i + 1) = (sweep a b c d i).2 := by
  intro i hi
  induction i with
  | zero =>
    intro hrow
    have h := hrow 0 le_rfl
    have hb := pivot_ne a b c d n hdom ha0 (i := 0) hi
    simp only [pivot] at hb
    simp only [sweep]
    rw [ha0, zero_mul, zero_add] at h
    field_simp
    linear_combination h
  | succ k ih =>
    intro hrow
    have hk := ih (by omega) (fun j hj => hrow j (by omega))
    have h := hrow (k + 1) le_rfl
    have hm := pivot_ne a b c d n hdom ha0 (i := k + 1) hi
    simp only [pivot] at hm
    simp only [sweep]
    simp only [Nat.add_sub_cancel] at h
    field_simp
    -- x_k = d'_k - c'_k x_(k+1): put it into row k + 1
    have hx : x k = (sweep a b c d k).2 - (sweep a b c d k).1 * x (k + 1) := by linear_combination hk
    rw [hx] at h
    linear_combination h

/-- The back substitution solves the reduced rows: `x_i + c'_i x_(i+1) = d'_i` for i < n - 1 and
`x_(n-1) = d'_(n-1)`. -/
theorem backSub_reduced (a b c d : ℕ → K) (n : ℕ) (i : ℕ) (hi : i + 1 < n) :
    backSub a b c d n i + (sweep a b c d i).1 * backSub a b c d n (i + 1) = (sweep a b c d i).2 := by
  rw [backSub]
  simp only [hi, dite_true]
  ring

theorem backSub_last (a b c d : ℕ → K) (n : ℕ) (hn : 0 < n) :
    backSub a b c d n (n - 1) = (sweep a b c d (n - 1)).2 := by
  rw [backSub]
  have : ¬ (n - 1 + 1 < n) := by omega
  simp only [this, dite_false]

/-- **The kernel's solve solves the system**: on row-dominant rows with `a_0 = 0` and `c_(n-1) = 0`,
the back-substituted sweep satisfies every row. -/
theorem thomas_solves (a b c d : ℕ → K) (n : ℕ) (hn : 0 < n) (hdom : ∀ i < n, |a i| + |c i| < |b i|)
    (ha0 : a 0 = 0) (hcn : c (n - 1) = 0) :
    ∀ i < n, a i * backSub a b c d n (i - 1) + b i * backSub a b c d n i
      + c i * backSub a b c d n (i + 1) = d i := by
  set x := backSub a b c d n
  -- the reduced rows, all of them (the last with c_(n-1) = 0)
  have red : ∀ i < n, x i + (sweep a b c d i).1 * x (i + 1) = (sweep a b c d i).2 := by
    intro i hi
    by_cases h : i + 1 < n
    · exact backSub_reduced a b c d n i h
    · have hi' : i = n - 1 := by omega
      subst hi'
      have hc : (sweep a b c d (n - 1)).1 = 0 := by
        rcases Nat.eq_zero_or_pos (n - 1) with h0 | h0
        · rw [h0] at hcn ⊢; simp [sweep, hcn]
        · obtain ⟨k, hk⟩ : ∃ k, n - 1 = k + 1 := ⟨n - 2, by omega⟩
          rw [hk] at hcn ⊢; simp [sweep, hcn]
      rw [hc, zero_mul, add_zero]
      exact backSub_last a b c d n hn
  -- each row from its reduced form and the one before
  intro i hi
  induction i with
  | zero =>
    have h := red 0 hi
    have hb := pivot_ne a b c d n hdom ha0 (i := 0) hi
    simp only [pivot] at hb
    simp only [sweep] at h
    rw [ha0, zero_mul, zero_add]
    field_simp at h
    linear_combination h
  | succ k _ =>
    have h := red (k + 1) hi
    have hk := red k (by omega)
    have hm := pivot_ne a b c d n hdom ha0 (i := k + 1) hi
    simp only [pivot] at hm
    simp only [sweep] at h
    simp only [Nat.add_sub_cancel]
    field_simp at h
    have hx : x k = (sweep a b c d k).2 - (sweep a b c d k).1 * x (k + 1) := by linear_combination hk
    rw [hx]
    linear_combination h

end SplineSpec
