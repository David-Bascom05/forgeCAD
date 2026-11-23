import Mathlib

/-!
# From the moments' errors to the centroid's and inertia's (mass.rs:111–221)

`mass_rel` sums each moment over the faces (`t`), with error bounds `e`: |tₖ − Tₖ| ≤ eₖ for the
true moments `T` (moments about a point c). Then:
* the centroid offset is `g = (t₁, t₂, t₃) / t₀`;
* the second moments about the centroid are `S_ij = m_ij − t₀ gᵢ gⱼ`;
* the inertia is `I = tr S − S`.

It reports:
* `err[2] = (em + |g| e₀) / |t₀|`, em = max(e₁, e₂, e₃), as a bound on the centroid's
  distance from the true one;
* `err[3] = 2 (es + 2 |g| em + |g|² e₀)`, es the largest second-moment error, as a bound on
  inertia's largest entry.

Proven here:
* `centroid_bound`: |g − G| ≤ (√(e₁² + e₂² + e₃²) + |g| e₀) / (|t₀| − e₀), the rigorous bound.
* `centroid_err_understates`: with em in place of the Euclidean norm of (e₁, e₂, e₃), the form
  `err[2]` had before 2026-11 was not a bound. Three first moments, each off by its whole bound, put the centroid √3 times
  farther off than it says. This is first order, whatever e₀ is.
* `second_moment_bound`: the exact error of one `S_ij`. Its first-order terms are what `err[3]`'s
  `es + 2|g| em + |g|² e₀` covers per entry; the factor 2 covers `I_ii = S_jj + S_kk`. It leaves
  out one second-order term, `(eᵢ + |gᵢ|e₀)(eⱼ + |gⱼ|e₀) / (|t₀| − e₀)`. That term is negligible
  under the kernel's own e₀ ≤ 10⁻³|t₀| (mass.rs:181).
* `inertia_entry_bound`: an entry's error with that term included.

mass.rs now reports `err[2] = (√(e₁² + e₂² + e₃²) + |g| e₀) / (|t₀| − e₀)`, which is
`centroid_bound`. It reports `err[3] = 2 (es + 2|g| em + |g|² e₀ + (em + |g| e₀)² / (|t₀| − e₀))`,
which is twice `inertia_entry_bound`.
-/

noncomputable section

namespace ForgeMass

abbrev E3 := EuclideanSpace ℝ (Fin 3)

theorem T0_pos {T0 t0 e0 : ℝ} (h0 : |t0 - T0| ≤ e0) (he : e0 < |t0|) : |t0| - e0 ≤ |T0| := by
  have := abs_sub_abs_le_abs_sub t0 T0
  linarith

theorem norm_le_of_abs_le (d : E3) (e : Fin 3 → ℝ) (h : ∀ i, |d i| ≤ e i) :
    ‖d‖ ≤ Real.sqrt (∑ i, e i ^ 2) := by
  rw [EuclideanSpace.norm_eq]
  apply Real.sqrt_le_sqrt
  apply Finset.sum_le_sum
  intro i _
  rw [Real.norm_eq_abs, sq_abs]
  exact sq_le_sq' (by linarith [abs_nonneg (d i), abs_le.1 (h i), h i, neg_abs_le (d i)]) (le_trans (le_abs_self _) (h i))

/-- The centroid's error, rigorously. -/
theorem centroid_bound (T0 t0 e0 : ℝ) (T1 t1 : E3) (e : Fin 3 → ℝ) (h0 : |t0 - T0| ≤ e0)
    (h1 : ∀ i, |t1 i - T1 i| ≤ e i) (he : e0 < |t0|) :
    ‖t0⁻¹ • t1 - T0⁻¹ • T1‖ ≤ (Real.sqrt (∑ i, e i ^ 2) + ‖t0⁻¹ • t1‖ * e0) / (|t0| - e0) := by
  have hT := T0_pos h0 he
  have hpos : 0 < |t0| - e0 := by linarith
  have he0 : 0 ≤ e0 := le_trans (abs_nonneg _) h0
  have hT0 : T0 ≠ 0 := by intro h; rw [h, abs_zero] at hT; linarith
  have ht0 : t0 ≠ 0 := by intro h; rw [h, abs_zero] at he; linarith [abs_nonneg (t0 - T0)]
  have id : t0⁻¹ • t1 - T0⁻¹ • T1 = T0⁻¹ • ((t1 - T1) - (t0 - T0) • (t0⁻¹ • t1)) := by
    ext i
    simp only [PiLp.sub_apply, PiLp.smul_apply, smul_eq_mul]
    field_simp
    ring
  rw [id, norm_smul, Real.norm_eq_abs, abs_inv]
  have hd : ‖(t1 - T1) - (t0 - T0) • (t0⁻¹ • t1)‖ ≤ Real.sqrt (∑ i, e i ^ 2) + ‖t0⁻¹ • t1‖ * e0 := by
    calc ‖(t1 - T1) - (t0 - T0) • (t0⁻¹ • t1)‖
        ≤ ‖t1 - T1‖ + ‖(t0 - T0) • (t0⁻¹ • t1)‖ := norm_sub_le _ _
      _ ≤ Real.sqrt (∑ i, e i ^ 2) + ‖t0⁻¹ • t1‖ * e0 := by
          gcongr
          · exact norm_le_of_abs_le _ e (fun i => by simpa using h1 i)
          · rw [norm_smul, Real.norm_eq_abs, mul_comm]
            exact mul_le_mul_of_nonneg_left h0 (norm_nonneg _)
  rw [inv_mul_eq_div]
  exact div_le_div₀ (add_nonneg (Real.sqrt_nonneg _) (mul_nonneg (norm_nonneg _) he0)) hd hpos hT

/-- `err[2]` as mass.rs:217 forms it is not a bound: three first moments off by their whole
bounds put the centroid √3 em away, where it says em. -/
theorem centroid_err_understates :
    ∃ (T0 t0 e0 : ℝ) (T1 t1 : E3) (e : Fin 3 → ℝ), |t0 - T0| ≤ e0 ∧ (∀ i, |t1 i - T1 i| ≤ e i) ∧
      e0 < |t0| ∧
      (max (max (e 0) (e 1)) (e 2) + ‖t0⁻¹ • t1‖ * e0) / |t0| < ‖t0⁻¹ • t1 - T0⁻¹ • T1‖ := by
  refine ⟨1, 1, 0, 0, (WithLp.equiv 2 (Fin 3 → ℝ)).symm (fun _ => 1), fun _ => 1, by simp,
    fun i => by simp, by simp, ?_⟩
  simp only [inv_one, one_smul, sub_zero, max_self, mul_zero, add_zero, abs_one, div_one]
  rw [EuclideanSpace.norm_eq]
  simp only [WithLp.equiv_symm_apply, Real.norm_eq_abs, abs_one, one_pow, Finset.sum_const,
    Finset.card_univ, Fintype.card_fin, nsmul_eq_mul, mul_one]
  rw [show ((3 : ℕ) : ℝ) = 3 by norm_num]
  rw [Real.lt_sqrt (by norm_num)]
  norm_num

/-- The exact error of one second moment about the centroid, `m − tᵢtⱼ/t₀`, and its bound. -/
theorem second_moment_bound (T0 t0 e0 Ti ti ei Tj tj ej M m es : ℝ) (h0 : |t0 - T0| ≤ e0)
    (hi : |ti - Ti| ≤ ei) (hj : |tj - Tj| ≤ ej) (hm : |m - M| ≤ es) (he : e0 < |t0|) :
    |(m - ti * tj / t0) - (M - Ti * Tj / T0)| ≤
      es + |ti / t0 * (tj / t0)| * e0 + |ti / t0| * ej + |tj / t0| * ei +
        (ei + |ti / t0| * e0) * (ej + |tj / t0| * e0) / (|t0| - e0) := by
  have hT := T0_pos h0 he
  have hpos : 0 < |t0| - e0 := by linarith
  have hT0 : T0 ≠ 0 := by intro h; rw [h, abs_zero] at hT; linarith
  have ht0 : t0 ≠ 0 := by intro h; rw [h, abs_zero] at he; linarith [abs_nonneg (t0 - T0)]
  have he0 : 0 ≤ e0 := le_trans (abs_nonneg _) h0
  have hei : 0 ≤ ei := le_trans (abs_nonneg _) hi
  have hej : 0 ≤ ej := le_trans (abs_nonneg _) hj
  set gi := ti / t0
  set gj := tj / t0
  set d0 := t0 - T0
  set di := ti - Ti
  set dj := tj - Tj
  have id : (m - ti * tj / t0) - (M - Ti * Tj / T0) =
      (m - M) + d0 * gi * gj - gi * dj - gj * di + (di - gi * d0) * (dj - gj * d0) / T0 := by
    simp only [gi, gj, d0, di, dj]
    field_simp
    ring
  rw [id]
  have q : |(di - gi * d0) * (dj - gj * d0) / T0| ≤
      (ei + |gi| * e0) * (ej + |gj| * e0) / (|t0| - e0) := by
    rw [abs_div, abs_mul]
    have a1 : |di - gi * d0| ≤ ei + |gi| * e0 := by
      calc |di - gi * d0| ≤ |di| + |gi * d0| := abs_sub _ _
        _ ≤ ei + |gi| * e0 := by rw [abs_mul]; gcongr
    have a2 : |dj - gj * d0| ≤ ej + |gj| * e0 := by
      calc |dj - gj * d0| ≤ |dj| + |gj * d0| := abs_sub _ _
        _ ≤ ej + |gj| * e0 := by rw [abs_mul]; gcongr
    have p1 : 0 ≤ ei + |gi| * e0 := add_nonneg hei (mul_nonneg (abs_nonneg _) he0)
    have p2 : 0 ≤ ej + |gj| * e0 := add_nonneg hej (mul_nonneg (abs_nonneg _) he0)
    exact div_le_div₀ (mul_nonneg p1 p2) (mul_le_mul a1 a2 (abs_nonneg _) p1) hpos hT
  calc |(m - M) + d0 * gi * gj - gi * dj - gj * di + (di - gi * d0) * (dj - gj * d0) / T0|
      ≤ |m - M| + |d0 * gi * gj| + |gi * dj| + |gj * di| + |(di - gi * d0) * (dj - gj * d0) / T0| := by
        have := abs_add_le ((m - M) + d0 * gi * gj - gi * dj - gj * di) ((di - gi * d0) * (dj - gj * d0) / T0)
        have := abs_sub ((m - M) + d0 * gi * gj - gi * dj) (gj * di)
        have := abs_sub ((m - M) + d0 * gi * gj) (gi * dj)
        have := abs_add_le (m - M) (d0 * gi * gj)
        linarith
    _ ≤ es + |gi * gj| * e0 + |gi| * ej + |gj| * ei +
          (ei + |gi| * e0) * (ej + |gj| * e0) / (|t0| - e0) := by
        rw [show d0 * gi * gj = d0 * (gi * gj) by ring, abs_mul d0, abs_mul gi dj, abs_mul gj di]
        have f2 : |d0| * |gi * gj| ≤ |gi * gj| * e0 := by
          rw [mul_comm]; exact mul_le_mul_of_nonneg_left h0 (abs_nonneg _)
        have f3 : |gi| * |dj| ≤ |gi| * ej := mul_le_mul_of_nonneg_left hj (abs_nonneg _)
        have f4 : |gj| * |di| ≤ |gj| * ei := mul_le_mul_of_nonneg_left hi (abs_nonneg _)
        linarith

/-- What mass.rs's `err[3]` takes per entry of S: with every |gᵢ| ≤ G and eᵢ ≤ em, an entry's error
is at most es + 2 G em + G² e₀ + (em + G e₀)² / (|t₀| − e₀). An inertia entry is an entry of S
(off the diagonal) or a sum of two (on it), so twice this bounds them all. -/
theorem inertia_entry_bound (T0 t0 e0 Ti ti ei Tj tj ej M m es G em : ℝ) (h0 : |t0 - T0| ≤ e0)
    (hi : |ti - Ti| ≤ ei) (hj : |tj - Tj| ≤ ej) (hm : |m - M| ≤ es) (he : e0 < |t0|)
    (hgi : |ti / t0| ≤ G) (hgj : |tj / t0| ≤ G) (hei : ei ≤ em) (hej : ej ≤ em) :
    |(m - ti * tj / t0) - (M - Ti * Tj / T0)| ≤
      es + 2 * G * em + G * G * e0 + (em + G * e0) * (em + G * e0) / (|t0| - e0) := by
  have base := second_moment_bound T0 t0 e0 Ti ti ei Tj tj ej M m es h0 hi hj hm he
  have he0 : 0 ≤ e0 := le_trans (abs_nonneg _) h0
  have hei0 : 0 ≤ ei := le_trans (abs_nonneg _) hi
  have hej0 : 0 ≤ ej := le_trans (abs_nonneg _) hj
  have hpos : 0 < |t0| - e0 := by linarith
  have hG : 0 ≤ G := le_trans (abs_nonneg _) hgi
  have p1 : |ti / t0 * (tj / t0)| * e0 ≤ G * G * e0 := by
    rw [abs_mul]; exact mul_le_mul_of_nonneg_right (mul_le_mul hgi hgj (abs_nonneg _) hG) he0
  have p2 : |ti / t0| * ej ≤ G * em := mul_le_mul hgi hej hej0 hG
  have p3 : |tj / t0| * ei ≤ G * em := mul_le_mul hgj hei hei0 hG
  have p4 : (ei + |ti / t0| * e0) * (ej + |tj / t0| * e0) / (|t0| - e0) ≤
      (em + G * e0) * (em + G * e0) / (|t0| - e0) := by
    apply div_le_div_of_nonneg_right _ hpos.le
    apply mul_le_mul
    · linarith [mul_le_mul_of_nonneg_right hgi he0]
    · linarith [mul_le_mul_of_nonneg_right hgj he0]
    · exact add_nonneg hej0 (mul_nonneg (abs_nonneg _) he0)
    · linarith [mul_nonneg hG he0, le_trans hei0 hei]
  linarith

end ForgeMass
end
