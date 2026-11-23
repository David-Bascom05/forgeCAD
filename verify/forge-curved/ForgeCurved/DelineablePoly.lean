import ForgeCurved.Delineable
import Mathlib.Analysis.Polynomial.CauchyBound
import Mathlib.Analysis.Calculus.Deriv.Polynomial
import Mathlib.Topology.Algebra.Polynomial

/-!
# Delineability for polynomial families

The sweep restricts a carrier to a line or curve, giving a polynomial in `y` whose coefficients
are polynomials in the sweep parameter `t`. Here that is `F : ℝ[t][y]` and
`ev F t y = (F.map (eval t)).eval y`.

**`delineable_poly`**: on an open preconnected `I` where the leading coefficient (in `y`) does
not vanish and every real root is simple, the real roots over `I` are `n` continuous, strictly
ordered graphs. The local bound on the roots is Mathlib's Cauchy bound
(`Polynomial.IsRoot.norm_lt_cauchyBound`), uniform near each `t₀` because the leading coefficient
stays away from `0`.
-/

namespace ForgeCurved.Delineable

open Filter Topology Set Polynomial

/-- The family a bivariate polynomial gives: `t` in the coefficients, `y` the variable. -/
noncomputable def ev (F : Polynomial (Polynomial ℝ)) (t y : ℝ) : ℝ :=
  (F.map (evalRingHom t)).eval y

theorem ev_eq_sum (F : Polynomial (Polynomial ℝ)) (t y : ℝ) :
    ev F t y = ∑ i ∈ Finset.range (F.natDegree + 1), (F.coeff i).eval t * y ^ i := by
  rw [ev, eval_eq_sum_range' (lt_of_le_of_lt (natDegree_map_le) (Nat.lt_succ_self _))]
  simp [coeff_map]

theorem smooth_ev (F : Polynomial (Polynomial ℝ)) : Smooth (ev F) (ev (derivative F)) where
  cont := by
    have : Function.uncurry (ev F) =
        fun z : ℝ × ℝ => ∑ i ∈ Finset.range (F.natDegree + 1), (F.coeff i).eval z.1 * z.2 ^ i := by
      funext z; exact ev_eq_sum F z.1 z.2
    rw [this]
    exact continuous_finsetSum _ fun i _ =>
      ((F.coeff i).continuous.comp continuous_fst).mul (continuous_snd.pow i)
  cont' := by
    have : Function.uncurry (ev (derivative F)) = fun z : ℝ × ℝ =>
        ∑ i ∈ Finset.range ((derivative F).natDegree + 1),
          ((derivative F).coeff i).eval z.1 * z.2 ^ i := by
      funext z; exact ev_eq_sum _ z.1 z.2
    rw [this]
    exact continuous_finsetSum _ fun i _ =>
      (((derivative F).coeff i).continuous.comp continuous_fst).mul (continuous_snd.pow i)
  deriv t y := by
    have h := (F.map (evalRingHom t)).hasDerivAt y
    rwa [derivative_map] at h

/-- The roots are bounded near any `t₀` where the leading coefficient is not `0`. -/
theorem roots_bdd (F : Polynomial (Polynomial ℝ)) {t₀ : ℝ} (hlc : F.leadingCoeff.eval t₀ ≠ 0) :
    ∃ R, ∀ᶠ t in 𝓝 t₀, ∀ y, ev F t y = 0 → |y| ≤ R := by
  set n := F.natDegree
  set S : ℝ := ∑ k ∈ Finset.range n, |(F.coeff k).eval t₀| + 1
  set L : ℝ := |F.leadingCoeff.eval t₀| / 2
  have hL : 0 < L := by positivity
  have hSc : Continuous fun t => ∑ k ∈ Finset.range n, |(F.coeff k).eval t| :=
    continuous_finsetSum _ fun k _ => (F.coeff k).continuous.abs
  have hS : ∀ᶠ t in 𝓝 t₀, ∑ k ∈ Finset.range n, |(F.coeff k).eval t| < S :=
    (hSc.tendsto t₀).eventually (gt_mem_nhds (by simp [S]))
  have hLev : ∀ᶠ t in 𝓝 t₀, L < |F.leadingCoeff.eval t| :=
    (F.leadingCoeff.continuous.abs.tendsto t₀).eventually
      (lt_mem_nhds (by simp only [L]; linarith [abs_pos.mpr hlc]))
  refine ⟨S / L + 1, ?_⟩
  filter_upwards [hS, hLev] with t hSt hLt y hy
  have hlt : (evalRingHom t) F.leadingCoeff ≠ 0 := by
    rw [coe_evalRingHom]; intro h; rw [h, abs_zero] at hLt; linarith
  set p := F.map (evalRingHom t)
  have hdeg : p.natDegree = n := natDegree_map_of_leadingCoeff_ne_zero _ hlt
  have hlcp : p.leadingCoeff = F.leadingCoeff.eval t := by
    rw [leadingCoeff_map_of_leadingCoeff_ne_zero _ hlt, coe_evalRingHom]
  have hp0 : p ≠ 0 := by
    intro h; apply hlt; rw [← leadingCoeff_map_of_leadingCoeff_ne_zero _ hlt]; simp [p, h]
  have hb := IsRoot.norm_lt_cauchyBound hp0 (a := y) hy
  have hb' : |y| < (cauchyBound p : ℝ) := by
    have := NNReal.coe_lt_coe.mpr hb
    simpa [Real.norm_eq_abs] using this
  set M : NNReal := (Finset.range p.natDegree).sup fun k => ‖p.coeff k‖₊ with hM
  have hsup : (M : ℝ) ≤ ∑ k ∈ Finset.range n, |(F.coeff k).eval t| := by
    have : M ≤
        ∑ k ∈ Finset.range p.natDegree, ‖p.coeff k‖₊ :=
      Finset.sup_le fun k hk => Finset.single_le_sum (f := fun k => ‖p.coeff k‖₊)
        (fun _ _ => by positivity) hk
    have hc := NNReal.coe_le_coe.mpr this
    rw [NNReal.coe_sum] at hc
    simpa [hdeg, p, coeff_map, Real.norm_eq_abs] using hc
  have hcb : (cauchyBound p : ℝ) ≤ S / L + 1 := by
    simp only [cauchyBound, NNReal.coe_add, NNReal.coe_div, NNReal.coe_one, coe_nnNorm,
      hlcp, Real.norm_eq_abs]
    have hpos : 0 < |F.leadingCoeff.eval t| := by linarith
    have h1 : (M : ℝ) / |F.leadingCoeff.eval t| ≤ S / L := by
      apply div_le_div₀ (by positivity) (hsup.trans hSt.le) hL hLt.le
    linarith
  linarith

/-- **Delineability of a polynomial family.** -/
theorem delineable_poly (F : Polynomial (Polynomial ℝ)) {I : Set ℝ} (hIo : IsOpen I)
    (hIc : IsPreconnected I) (hlc : ∀ t ∈ I, F.leadingCoeff.eval t ≠ 0)
    (hsimple : ∀ t ∈ I, ∀ y, ev F t y = 0 → ev (derivative F) t y ≠ 0) :
    ∃ n, ∃ r : Fin n → ℝ → ℝ, (∀ i, ContinuousOn (r i) I) ∧ (∀ t ∈ I, StrictMono fun i => r i t) ∧
      ∀ t ∈ I, ∀ y, ev F t y = 0 ↔ ∃ i, r i t = y :=
  delineable (smooth_ev F) hIo hIc hsimple fun t ht => roots_bdd F (hlc t ht)

end ForgeCurved.Delineable
