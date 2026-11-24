/-
The derivative of a B-spline is a B-spline of one degree less (`hasDerivAt_curve`): on a span,
`d/dt Σ N_(i,p) P_i = Σ N_(i,p-1) · p (P_i - P_(i-1)) / (u_(i+p) - u_i)`, from the basis's
derivative `N'_(i,k+1) = (k+1) (N_(i,k) / (u_(i+k+1) - u_i) - N_(i+1,k) / (u_(i+k+2) - u_(i+1)))`
(`hasDerivAt_nb`). With the convex hull (`eval_between`) this bounds a curve's derivative by its
differenced poles, which is what `derivative_bounds` and the hull tests take.
-/
import SplineSpec.Basis
import Mathlib.Analysis.Calculus.Deriv.Add
import Mathlib.Analysis.Calculus.Deriv.Mul
import Mathlib.Analysis.Calculus.Deriv.Inv

open Finset

namespace SplineSpec

/-- The blending ratio is the plain quotient (a zero knot difference gives zero either way). -/
theorem omega_eq (u : ℕ → ℝ) (i k : ℕ) (t : ℝ) : omega u i k t = (t - u i) / (u (i + k) - u i) := by
  unfold omega; split_ifs with h
  · rw [h, sub_self, div_zero]
  · rfl

/-- A basis function on a span is zero wherever its own support has no length. -/
theorem nb_zero_degenerate (u : ℕ → ℝ) (hu : Monotone u) (j : ℕ) (hj : u j < u (j + 1)) (i k : ℕ)
    (hd : u i = u (i + k + 1)) (t : ℝ) : nb u j i k t = 0 := by
  by_cases h1 : j < i
  · exact nb_zero_right u j k i t h1
  by_cases h2 : i + k < j
  · exact nb_zero_left u j k i t h2
  exfalso
  have a := hu (show i ≤ j by omega)
  have b := hu (show j + 1 ≤ i + k + 1 by omega)
  linarith

/-- The basis's derivative, as a formula. -/
noncomputable def dnb (u : ℕ → ℝ) (j : ℕ) : ℕ → ℕ → ℝ → ℝ
  | _, 0, _ => 0
  | i, k + 1, t => (k + 1) * (nb u j i k t / (u (i + k + 1) - u i)
      - nb u j (i + 1) k t / (u (i + k + 2) - u (i + 1)))

/-- The algebra of the derivative's induction step: coefficient by coefficient of the three
functions of the degree below, a rational identity in t and the knots; where the knots of `N1` or
`N2`'s support coincide, that function is zero. -/
theorem step_alg (A B C D E F t N0 N1 N2 K : ℝ) (hAB : A ≤ B) (hBC : B ≤ C) (hBE : B ≤ E)
    (hED : E ≤ D) (hCD : C ≤ D) (hN1 : B = C → N1 = 0) (hN2 : E = D → N2 = 0) :
    (t - A) / (C - A) * (K * (N0 / (F - A) - N1 / (C - B)))
      + (1 - (t - B) / (D - B)) * (K * (N1 / (C - B) - N2 / (D - E)))
    = K * (((t - A) / (F - A) * N0 + (1 - (t - B) / (C - B)) * N1) / (C - A)
      - ((t - B) / (C - B) * N1 + (1 - (t - E) / (D - E)) * N2) / (D - B)) := by
  by_cases hfa : F - A = 0
  · rw [hfa]; simp only [div_zero, zero_mul, zero_add, zero_sub]
    by_cases hbc : B = C
    · rw [hN1 hbc, hbc, sub_self]; simp only [div_zero, zero_div, sub_zero, mul_zero, zero_mul, add_zero, zero_add, zero_sub, neg_zero]
      by_cases hed : E = D
      · rw [hN2 hed]; simp
      · have h1 : D - E ≠ 0 := sub_ne_zero.2 (Ne.symm hed)
        have h2 : D - C ≠ 0 := by intro h; apply hed; linarith [hbc]
        field_simp; ring
    · have h1 : C - B ≠ 0 := sub_ne_zero.2 (Ne.symm hbc)
      have h2 : C - A ≠ 0 := by intro h; apply hbc; linarith [lt_of_le_of_ne hBC hbc]
      have h3 : D - B ≠ 0 := by intro h; apply hbc; linarith [lt_of_le_of_ne hBC hbc]
      by_cases hed : E = D
      · rw [hN2 hed, hed, sub_self]; simp only [div_zero, zero_div, sub_zero, mul_zero, zero_mul, add_zero]
        field_simp; ring
      · have h4 : D - E ≠ 0 := sub_ne_zero.2 (Ne.symm hed)
        field_simp; ring
  · by_cases hbc : B = C
    · rw [hN1 hbc, hbc, sub_self]; simp only [div_zero, zero_div, sub_zero, mul_zero, zero_mul, add_zero, zero_add, zero_sub, neg_zero, sub_zero]
      by_cases hed : E = D
      · rw [hN2 hed]; simp only [mul_zero, zero_div, sub_zero, add_zero, div_zero, zero_mul]
        by_cases hca : C - A = 0
        · rw [hca]; simp
        · field_simp; ring
      · have h1 : D - E ≠ 0 := sub_ne_zero.2 (Ne.symm hed)
        have h2 : D - C ≠ 0 := by intro h; apply hed; linarith [hbc]
        by_cases hca : C - A = 0
        · rw [hca]; simp only [div_zero, zero_mul, zero_add, zero_sub, mul_zero]; field_simp; ring
        · field_simp; ring
    · have h1 : C - B ≠ 0 := sub_ne_zero.2 (Ne.symm hbc)
      have h2 : C - A ≠ 0 := by intro h; apply hbc; linarith [lt_of_le_of_ne hBC hbc]
      have h3 : D - B ≠ 0 := by intro h; apply hbc; linarith [lt_of_le_of_ne hBC hbc]
      by_cases hed : E = D
      · rw [hN2 hed, hed, sub_self]; simp only [div_zero, zero_div, sub_zero, mul_zero, zero_mul, add_zero]
        field_simp; ring
      · have h4 : D - E ≠ 0 := sub_ne_zero.2 (Ne.symm hed)
        field_simp; ring

/-- The step of the derivative's induction: the ratios carry the lower derivative into the
formula. -/
theorem dnb_step (u : ℕ → ℝ) (hu : Monotone u) (j : ℕ) (hj : u j < u (j + 1)) (i k : ℕ) (t : ℝ) :
    omega u i (k + 1) t * dnb u j i k t + (1 - omega u (i + 1) (k + 1) t) * dnb u j (i + 1) k t
      = k * (nb u j i k t / (u (i + k + 1) - u i) - nb u j (i + 1) k t / (u (i + k + 2) - u (i + 1))) := by
  cases k with
  | zero => simp [dnb]
  | succ k =>
    simp only [dnb, nb, omega_eq]
    have e := step_alg (u i) (u (i + 1)) (u (i + k + 2)) (u (i + k + 3)) (u (i + 2)) (u (i + k + 1)) t
      (nb u j i k t) (nb u j (i + 1) k t) (nb u j (i + 2) k t) ((k : ℝ) + 1)
      (hu (by omega)) (hu (by omega)) (hu (by omega)) (hu (by omega)) (hu (by omega))
      (fun h => nb_zero_degenerate u hu j hj (i + 1) k (by rw [h]; ring_nf) t)
      (fun h => nb_zero_degenerate u hu j hj (i + 2) k (by rw [h]; ring_nf) t)
    have i1 : i + (k + 1 + 1) = i + k + 2 := by ring
    have i2 : i + 1 + (k + 1 + 1) = i + k + 3 := by ring
    have i3 : i + (k + 1) = i + k + 1 := by ring
    have i4 : i + 1 + k + 1 = i + k + 2 := by ring
    have i5 : i + 1 + (k + 1) = i + k + 2 := by ring
    have i6 : i + 1 + 1 = i + 2 := by ring
    have i7 : i + 1 + 1 + (k + 1) = i + k + 3 := by ring
    have i8 : i + 1 + k + 2 = i + k + 3 := by ring
    have i9 : i + (k + 1) + 1 = i + k + 2 := by ring
    have i10 : i + 2 + k + 1 = i + k + 3 := by ring
    simp only [i1, i2, i3, i4, i5, i6, i7, i8, i9, i10, Nat.cast_add, Nat.cast_one] at e ⊢
    linear_combination e

/-- **The basis's derivative** on a nonempty span, for nondecreasing knots. -/
theorem hasDerivAt_nb (u : ℕ → ℝ) (hu : Monotone u) (j : ℕ) (hj : u j < u (j + 1)) :
    ∀ k i t, HasDerivAt (nb u j i k) (dnb u j i k t) t := by
  intro k
  induction k with
  | zero => intro i t; simp only [nb, dnb]; exact hasDerivAt_const t _
  | succ k ih =>
    intro i t
    have hw : ∀ l, HasDerivAt (fun s => omega u l (k + 1) s) (1 / (u (l + (k + 1)) - u l)) t := by
      intro l
      simp only [omega_eq]
      simpa using ((hasDerivAt_id t).sub_const (u l)).div_const (u (l + (k + 1)) - u l)
    have h := ((hw i).mul (ih i t)).add (((hasDerivAt_const t (1 : ℝ)).sub (hw (i + 1))).mul (ih (i + 1) t))
    have hf : (nb u j i (k + 1)) = fun s => omega u i (k + 1) s * nb u j i k s
        + (1 - omega u (i + 1) (k + 1) s) * nb u j (i + 1) k s := by
      funext s; rfl
    rw [hf]
    convert h using 1
    have st := dnb_step u hu j hj i k t
    have lhs : dnb u j i (k + 1) t = ((k : ℝ) + 1) * (nb u j i k t / (u (i + k + 1) - u i)
        - nb u j (i + 1) k t / (u (i + k + 2) - u (i + 1))) := rfl
    rw [lhs]
    have i1 : i + (k + 1) = i + k + 1 := by ring
    have i2 : i + 1 + (k + 1) = i + k + 2 := by ring
    simp only [i1, i2, Pi.sub_apply]
    linear_combination -st

/-- **The derivative of a B-spline is a B-spline of one degree less**: on a nonempty span j with
`p ≤ j < n`, `d/dt Σ_(i<n) N_(i,p+1) P_i = Σ_(i<n) N_(i,p) Q_i`, `Q_i = (p+1)(P_i - P_(i-1)) /
(u_(i+p+1) - u_i)` (`Q_0` is never weighed there). One coordinate; the homogeneous form of a
rational curve the same, coordinate by coordinate. -/
theorem hasDerivAt_curve (u : ℕ → ℝ) (hu : Monotone u) (p n j : ℕ) (hj : u j < u (j + 1))
    (hpj : p + 1 ≤ j) (hjn : j < n) (P : ℕ → ℝ) (t : ℝ) :
    HasDerivAt (fun s => ∑ i ∈ range n, nb u j i (p + 1) s * P i)
      (∑ i ∈ range n, nb u j i p t * ((p + 1) * (P i - P (i - 1)) / (u (i + p + 1) - u i))) t := by
  have h := HasDerivAt.sum (u := range n) (A := fun i s => nb u j i (p + 1) s * P i)
    (A' := fun i => dnb u j i (p + 1) t * P i) (x := t)
    (fun i _ => (hasDerivAt_nb u hu j hj (p + 1) i t).mul_const (P i))
  convert h using 1
  · funext s; simp [Finset.sum_apply]
  have lhs : ∀ i, dnb u j i (p + 1) t = ((p : ℝ) + 1) * (nb u j i p t / (u (i + p + 1) - u i)
      - nb u j (i + 1) p t / (u (i + p + 2) - u (i + 1))) := fun i => rfl
  simp only [lhs]
  -- Σ (p+1)(N_i/c_i - N_(i+1)/c_(i+1)) P_i = Σ N_i (p+1)(P_i - P_(i-1))/c_i, the ends zero
  have hz0 : nb u j 0 p t = 0 := nb_zero_left u j p 0 t (by omega)
  have hzn : nb u j n p t = 0 := nb_zero_right u j p n t hjn
  have shift : ∑ i ∈ range n, nb u j (i + 1) p t / (u (i + p + 2) - u (i + 1)) * P i
      = ∑ i ∈ range n, nb u j i p t / (u (i + p + 1) - u i) * P (i - 1) := by
    obtain ⟨n', rfl⟩ : ∃ n', n = n' + 1 := ⟨n - 1, by omega⟩
    rw [sum_range_succ, sum_range_succ' (fun i => nb u j i p t / (u (i + p + 1) - u i) * P (i - 1)),
      hzn, hz0]
    simp only [zero_div, zero_mul, add_zero]
    refine sum_congr rfl fun i _ => ?_
    have a : i + 1 + p + 1 = i + p + 2 := by ring
    simp only [a, Nat.add_sub_cancel]
  have e1 : ∑ i ∈ range n, ((p : ℝ) + 1) * (nb u j i p t / (u (i + p + 1) - u i)
        - nb u j (i + 1) p t / (u (i + p + 2) - u (i + 1))) * P i
      = ((p : ℝ) + 1) * (∑ i ∈ range n, nb u j i p t / (u (i + p + 1) - u i) * P i
        - ∑ i ∈ range n, nb u j (i + 1) p t / (u (i + p + 2) - u (i + 1)) * P i) := by
    rw [mul_sub, mul_sum, mul_sum, ← sum_sub_distrib]
    refine sum_congr rfl fun i _ => by ring
  have i1 : ∀ i, i + (p + 1) + 1 = i + p + 2 := fun i => by ring
  have i2 : ∀ i, i + (p + 1) = i + p + 1 := fun i => by ring
  try simp only [i1, i2, Nat.cast_add, Nat.cast_one]
  rw [e1, shift, ← sum_sub_distrib, mul_sum]
  refine sum_congr rfl fun i _ => by ring

/-- **A rational curve's derivative**, `C = A / W` with `A = Σ N w P` and `W = Σ N w`:
`C' = (A' - C W') / W`, where A' and W' are the B-splines of degree p on the differenced
homogeneous poles (`hasDerivAt_curve`). This is what bounds a rational curve's derivative:
A', W' by their poles (`eval_between`), W from below by the least weight. -/
theorem hasDerivAt_rational (u : ℕ → ℝ) (hu : Monotone u) (p n j : ℕ) (hj : u j < u (j + 1))
    (hpj : p + 1 ≤ j) (hjn : j < n) (w P : ℕ → ℝ) (t : ℝ) (hW : weightOn u (p + 1) n j w t ≠ 0) :
    HasDerivAt (fun s => homOn u (p + 1) n j w P s / weightOn u (p + 1) n j w s)
      ((∑ i ∈ range n, nb u j i p t * ((p + 1) * (w i * P i - w (i - 1) * P (i - 1)) / (u (i + p + 1) - u i))
        - homOn u (p + 1) n j w P t / weightOn u (p + 1) n j w t *
          ∑ i ∈ range n, nb u j i p t * ((p + 1) * (w i - w (i - 1)) / (u (i + p + 1) - u i)))
        / weightOn u (p + 1) n j w t) t := by
  have hA := hasDerivAt_curve u hu p n j hj hpj hjn (fun i => w i * P i) t
  have hB := hasDerivAt_curve u hu p n j hj hpj hjn w t
  have eA : (fun s => homOn u (p + 1) n j w P s) = fun s => ∑ i ∈ range n, nb u j i (p + 1) s * (w i * P i) := by
    funext s; exact homOn_eq _ _ _ _ _ _ _
  have eB : (fun s => weightOn u (p + 1) n j w s) = fun s => ∑ i ∈ range n, nb u j i (p + 1) s * w i := by
    funext s; exact weightOn_eq _ _ _ _ _ _
  rw [← eA] at hA
  rw [← eB] at hB
  have h := hA.div hB hW
  convert h using 1
  generalize (∑ i ∈ range n, nb u j i p t * ((↑p + 1) * (w i * P i - w (i - 1) * P (i - 1)) / (u (i + p + 1) - u i))) = A'
  generalize (∑ i ∈ range n, nb u j i p t * ((↑p + 1) * (w i - w (i - 1)) / (u (i + p + 1) - u i))) = W'
  generalize homOn u (p + 1) n j w P t = A at *
  generalize weightOn u (p + 1) n j w t = W at *
  field_simp

end SplineSpec
