import ForgeMass.Trig

/-!
# Gauss–Legendre on 2 and 3 points (mass.rs:1861–1866, 2015–2022)

`GL2` integrates polynomials of degree ≤ 3 over [−1, 1] exactly, `GL3` those of degree ≤ 5.
`Inner::Poly` (mass.rs:2015–2022) takes `G(a) = ∫_{a0}^{a} F` as `(x/2) Σ w F(a0 + x(1+t)/2)`,
x = a − a0, which is exact for the same F (`gl2_G`, `gl3_G`). The nodes are the real ones,
±1/√3 and ±√(3/5); the kernel's are their nearest doubles.
-/

noncomputable section
open intervalIntegral Polynomial

namespace ForgeMass

/-- ∫₋₁¹ xⁱ -/
theorem int_pow (i : ℕ) : ∫ x in (-1 : ℝ)..1, x ^ i = (1 - (-1) ^ (i + 1)) / (i + 1) := by
  rw [integral_pow]; ring

theorem int_poly {n : ℕ} (p : ℝ[X]) (hp : p.natDegree < n) :
    ∫ x in (-1 : ℝ)..1, p.eval x = ∑ i ∈ Finset.range n, p.coeff i * ((1 - (-1) ^ (i + 1)) / (i + 1)) := by
  simp_rw [eval_eq_sum_range' hp]
  rw [integral_finsetSum (f := fun i x => p.coeff i * x ^ i)
    (fun i _ => (continuous_const.mul (continuous_pow i)).intervalIntegrable _ _)]
  congr 1; funext i
  rw [integral_const_mul, int_pow]

def gl2 (f : ℝ → ℝ) : ℝ := f (-(1 / Real.sqrt 3)) + f (1 / Real.sqrt 3)

def gl3 (f : ℝ → ℝ) : ℝ :=
  5 / 9 * f (-Real.sqrt (3 / 5)) + 8 / 9 * f 0 + 5 / 9 * f (Real.sqrt (3 / 5))

theorem gl2_exact {f : ℝ → ℝ} (hf : PolyR 3 f) : ∫ x in (-1 : ℝ)..1, f x = gl2 f := by
  obtain ⟨p, hp, hpf⟩ := hf
  have e : (fun x => f x) = fun x => p.eval x := funext hpf
  rw [e, int_poly p (n := 4) (by omega), gl2, hpf, hpf, eval_eq_sum_range' (n := 4) (by omega),
    eval_eq_sum_range' (n := 4) (by omega)]
  set t := 1 / Real.sqrt 3
  have t2 : t ^ 2 = 1 / 3 := by rw [div_pow, Real.sq_sqrt (by norm_num)]; norm_num
  simp only [Finset.sum_range_succ, Finset.sum_range_zero]
  push_cast
  linear_combination (-2 * p.coeff 2) * t2

theorem gl3_exact {f : ℝ → ℝ} (hf : PolyR 5 f) : ∫ x in (-1 : ℝ)..1, f x = gl3 f := by
  obtain ⟨p, hp, hpf⟩ := hf
  have e : (fun x => f x) = fun x => p.eval x := funext hpf
  rw [e, int_poly p (n := 6) (by omega), gl3, hpf, hpf, hpf, eval_eq_sum_range' (n := 6) (by omega),
    eval_eq_sum_range' (n := 6) (by omega), eval_eq_sum_range' (n := 6) (by omega)]
  set s := Real.sqrt (3 / 5)
  have s2 : s ^ 2 = 3 / 5 := Real.sq_sqrt (by norm_num)
  simp only [Finset.sum_range_succ, Finset.sum_range_zero]
  push_cast
  linear_combination (-(10 / 9) * p.coeff 2 - (10 / 9) * (s ^ 2 + 3 / 5) * p.coeff 4) * s2

/-- `F` along `[a0, a0 + x]`, pulled back to [−1, 1], keeps its degree. -/
theorem PolyR.affine {d : ℕ} {f : ℝ → ℝ} (hf : PolyR d f) (a0 x : ℝ) :
    PolyR d (fun s => f (a0 + x * (1 + s) / 2)) := by
  obtain ⟨p, hp, hpf⟩ := hf
  refine ⟨p.comp (C (a0 + x / 2) + C (x / 2) * X), ?_, fun s => by simp [hpf]; ring_nf⟩
  calc (p.comp (C (a0 + x / 2) + C (x / 2) * X)).natDegree
      ≤ p.natDegree * (C (a0 + x / 2) + C (x / 2) * X).natDegree := natDegree_comp_le
    _ ≤ d * 1 := by
        apply Nat.mul_le_mul hp
        apply (natDegree_add_le _ _).trans
        simp only [natDegree_C, zero_le, max_le_iff, true_and]
        exact (natDegree_C_mul_le _ _).trans (by simp)
    _ = d := by ring

/-- The substitution `Inner::Poly` makes: ∫ over [a0, a0 + x] is x/2 times ∫ over [−1, 1]. -/
theorem G_pullback (f : ℝ → ℝ) (a0 x : ℝ) :
    ∫ t in a0..a0 + x, f t = x / 2 * ∫ s in (-1 : ℝ)..1, f (a0 + x * (1 + s) / 2) := by
  by_cases hx : x = 0
  · subst hx; simp
  have hc : x / 2 ≠ 0 := by positivity
  have e1 : (fun s => f (a0 + x * (1 + s) / 2)) = fun s => f (x / 2 * s + (a0 + x / 2)) := by
    funext s; congr 1; ring
  rw [e1, intervalIntegral.integral_comp_mul_add (fun t => f t) hc,
    show x / 2 * -1 + (a0 + x / 2) = a0 by ring, show x / 2 * 1 + (a0 + x / 2) = a0 + x by ring,
    smul_eq_mul]
  field_simp

/-- mass.rs:2015–2022 with GL2: exact for F of degree ≤ 3. -/
theorem gl2_G {f : ℝ → ℝ} (hf : PolyR 3 f) (a0 x : ℝ) :
    ∫ t in a0..a0 + x, f t = x / 2 * gl2 (fun s => f (a0 + x * (1 + s) / 2)) := by
  rw [G_pullback, gl2_exact (hf.affine a0 x)]

/-- mass.rs:2015–2022 with GL3: exact for F of degree ≤ 5. -/
theorem gl3_G {f : ℝ → ℝ} (hf : PolyR 5 f) (a0 x : ℝ) :
    ∫ t in a0..a0 + x, f t = x / 2 * gl3 (fun s => f (a0 + x * (1 + s) / 2)) := by
  rw [G_pullback, gl3_exact (hf.affine a0 x)]

end ForgeMass
end
