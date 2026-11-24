/-
What makes the oracle's answer for a control spline a proof (SplineSpec/Oracle.lean, `checkControl`).

On a span `[a, a + h]` the homogeneous curve `homOn` (and its weight `weightOn`) is a polynomial of
degree at most p in t (`nb_poly`). The oracle finds Bernstein coefficients c with
`Σ c_l B_l(k/p) = homOn(a + h k/p)` at the p + 1 points k = 0..p, checked exactly; two polynomials of
degree p that agree at p + 1 points are one (`span_bernstein`), so `Σ c_l B_l((t - a)/h)` *is* the
spec's curve on the span. Then the kernel's piece, with points ĉ, differs from it by at most
`max |ĉ_l - c_l|` at every t of the span (`bern_close`, the Bernstein basis being nonnegative and
summing to one there), which is the number the oracle bounds.
-/
import SplineSpec.Deriv
import SplineLoops.Certs
import Mathlib.LinearAlgebra.Lagrange

open Finset Polynomial

namespace SplineSpec

/-- The basis function as a polynomial: the same recurrence on `Polynomial ℝ`. -/
noncomputable def nbP (u : ℕ → ℝ) (j : ℕ) : ℕ → ℕ → Polynomial ℝ
  | i, 0 => C (if i = j then 1 else 0)
  | i, k + 1 => C (1 / (u (i + (k + 1)) - u i)) * (X - C (u i)) * nbP u j i k
      + (1 - C (1 / (u (i + 1 + (k + 1)) - u (i + 1))) * (X - C (u (i + 1)))) * nbP u j (i + 1) k

theorem nbP_eval (u : ℕ → ℝ) (j : ℕ) : ∀ k i t, (nbP u j i k).eval t = nb u j i k t := by
  intro k
  induction k with
  | zero => intro i t; simp only [nbP, nb]; split_ifs <;> simp
  | succ k ih =>
    intro i t
    simp only [nbP, nb, eval_add, eval_mul, eval_C, eval_sub, eval_X, eval_one, ih, omega_eq]
    ring

theorem nbP_degree (u : ℕ → ℝ) (j : ℕ) : ∀ k i, (nbP u j i k).natDegree ≤ k := by
  intro k
  induction k with
  | zero => intro i; simp only [nbP]; split_ifs <;> simp
  | succ k ih =>
    intro i
    simp only [nbP]
    refine (natDegree_add_le _ _).trans (max_le ?_ ?_)
    · refine (natDegree_mul_le).trans ?_
      have : (C (1 / (u (i + (k + 1)) - u i)) * (X - C (u i))).natDegree ≤ 1 :=
        (natDegree_mul_le).trans (by
          have := natDegree_X_sub_C_le (u i)
          simp only [natDegree_C, zero_add]; exact this)
      have := ih i
      omega
    · refine (natDegree_mul_le).trans ?_
      have : (1 - C (1 / (u (i + 1 + (k + 1)) - u (i + 1))) * (X - C (u (i + 1)))).natDegree ≤ 1 := by
        refine (natDegree_sub_le _ _).trans (max_le (by simp) ?_)
        refine (natDegree_mul_le).trans ?_
        have := natDegree_X_sub_C_le (u (i + 1))
        simp only [natDegree_C, zero_add]; exact this
      have := ih (i + 1)
      omega

/-- The homogeneous curve (one coordinate, or the weight with `P = 1`) on a span is a polynomial of
degree at most p. -/
theorem homOn_poly (u : ℕ → ℝ) (p n j : ℕ) (w P : ℕ → ℝ) :
    ∃ f : Polynomial ℝ, f.natDegree ≤ p ∧ ∀ t, f.eval t = homOn u p n j w P t := by
  refine ⟨∑ i ∈ range n, nbP u j i p * C (w i * P i), ?_, fun t => ?_⟩
  · refine natDegree_sum_le_of_forall_le _ _ fun i _ => ?_
    exact (natDegree_mul_C_le _ _).trans (nbP_degree u j p i)
  · rw [homOn_eq, eval_finsetSum]
    simp [nbP_eval]

/-- The Bernstein form over `[a, a + h]` as a polynomial in t. -/
noncomputable def bernP (p : ℕ) (c : ℕ → ℝ) (a h : ℝ) : Polynomial ℝ :=
  ∑ l ∈ range (p + 1), C (c l * (p.choose l : ℝ)) * (C (1 / h) * (X - C a)) ^ l *
    (1 - C (1 / h) * (X - C a)) ^ (p - l)

theorem bernP_eval (p : ℕ) (c : ℕ → ℝ) (a h t : ℝ) :
    (bernP p c a h).eval t = SplineLoops.bern p c ((t - a) / h) := by
  simp only [bernP, SplineLoops.bern, SplineLoops.bb, eval_finsetSum, eval_mul, eval_C, eval_pow,
    eval_sub, eval_X, eval_one]
  refine sum_congr rfl fun l _ => ?_
  ring

theorem bernP_degree (p : ℕ) (c : ℕ → ℝ) (a h : ℝ) : (bernP p c a h).natDegree ≤ p := by
  refine natDegree_sum_le_of_forall_le _ _ fun l hl => ?_
  have hl' := mem_range.1 hl
  have h1 : (C (1 / h) * (X - C a)).natDegree ≤ 1 :=
    (natDegree_mul_le).trans (by have := natDegree_X_sub_C_le a; simp only [natDegree_C, zero_add]; exact this)
  have h2 : (1 - C (1 / h) * (X - C a)).natDegree ≤ 1 :=
    (natDegree_sub_le _ _).trans (max_le (by simp) h1)
  have e1 : (C (c l * (p.choose l : ℝ)) * (C (1 / h) * (X - C a)) ^ l).natDegree ≤ l :=
    (natDegree_C_mul_le _ _).trans (natDegree_pow_le.trans (by nlinarith))
  have e2 : ((1 - C (1 / h) * (X - C a)) ^ (p - l)).natDegree ≤ p - l :=
    natDegree_pow_le.trans (by nlinarith)
  refine (natDegree_mul_le).trans ?_
  omega

/-- **The oracle's coefficients are the spec's curve on the span**: Bernstein coefficients that
match the span's polynomial at the p + 1 points `a + h k / p` match it at every t. -/
theorem span_bernstein (u : ℕ → ℝ) (p n j : ℕ) (hp : 1 ≤ p) (w P c : ℕ → ℝ) (a h : ℝ) (hh : h ≠ 0)
    (hc : ∀ k ≤ p, SplineLoops.bern p c ((k : ℝ) / p) = homOn u p n j w P (a + h * ((k : ℝ) / p))) :
    ∀ t, SplineLoops.bern p c ((t - a) / h) = homOn u p n j w P t := by
  obtain ⟨f, hf, hfe⟩ := homOn_poly u p n j w P
  have hpos : (0 : ℝ) < p := by exact_mod_cast hp
  let pt : ℕ → ℝ := fun k => a + h * ((k : ℝ) / p)
  have hinj : Set.InjOn pt (range (p + 1) : Set ℕ) := by
    intro x _ y _ hxy
    simp only [pt] at hxy
    have : (x : ℝ) / p = (y : ℝ) / p := by
      have := mul_left_cancel₀ hh (add_left_cancel hxy); exact this
    have : (x : ℝ) = y := by field_simp at this; exact this
    exact_mod_cast this
  set S := (range (p + 1)).image pt
  have hS : S.card = p + 1 := by rw [card_image_of_injOn hinj, card_range]
  have heq : bernP p c a h = f := by
    apply Polynomial.eq_of_degree_sub_lt_of_eval_finset_eq S
    · rw [hS]
      refine lt_of_le_of_lt (degree_sub_le _ _) ?_
      refine max_lt ?_ ?_
      · exact lt_of_le_of_lt (degree_le_of_natDegree_le (bernP_degree p c a h))
          (WithBot.coe_lt_coe.2 (Nat.lt_succ_self p))
      · exact lt_of_le_of_lt (degree_le_of_natDegree_le hf) (WithBot.coe_lt_coe.2 (Nat.lt_succ_self p))
    · intro x hx
      obtain ⟨k, hk, rfl⟩ := mem_image.1 hx
      rw [bernP_eval, hfe]
      have : (pt k - a) / h = (k : ℝ) / p := by simp only [pt]; field_simp; ring
      rw [this]
      exact hc k (Nat.lt_succ_iff.1 (mem_range.1 hk))
  intro t
  rw [← bernP_eval, heq, hfe]

/-- **Two Bernstein forms with coefficients within e of each other are within e** on [0, 1]. -/
theorem bern_close (p : ℕ) (c d : ℕ → ℝ) (e : ℝ) (hcd : ∀ l ≤ p, |c l - d l| ≤ e) {s : ℝ}
    (h0 : 0 ≤ s) (h1 : s ≤ 1) : |SplineLoops.bern p c s - SplineLoops.bern p d s| ≤ e := by
  have hsub : SplineLoops.bern p c s - SplineLoops.bern p d s = SplineLoops.bern p (fun l => c l - d l) s := by
    simp only [SplineLoops.bern, ← sum_sub_distrib]
    refine sum_congr rfl fun l _ => by ring
  rw [hsub, abs_le]
  constructor
  · exact SplineLoops.bern_ge p _ (-e) (fun l hl => by have := hcd l hl; rw [abs_le] at this; linarith) h0 h1
  · have := SplineLoops.bern_ge p (fun l => -(c l - d l)) (-e)
      (fun l hl => by have := hcd l hl; rw [abs_le] at this; linarith) h0 h1
    have hneg : SplineLoops.bern p (fun l => -(c l - d l)) s = -SplineLoops.bern p (fun l => c l - d l) s := by
      simp only [SplineLoops.bern, ← sum_neg_distrib]
      refine sum_congr rfl fun l _ => by ring
    linarith

end SplineSpec
