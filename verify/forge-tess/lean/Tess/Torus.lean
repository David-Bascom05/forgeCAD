import Mathlib
import Tess.Interp

namespace Sag

/-- The distance from the axis along a chord, ρ(s) = √(α + 2βs + γs²) (α, β, γ from the shadows
of the chord's start and step on the plane square to the axis), when the shadows are not on
one line through the axis (αγ − β² > 0, so the quadratic has no zero): its derivatives. -/
theorem shadow_derivs (α β γ : ℝ) (γ0 : 0 < γ) (hd : 0 < α * γ - β ^ 2) :
    let Q := fun s : ℝ => α + 2 * β * s + γ * s ^ 2
    (∀ s, 0 < Q s) ∧
    deriv (fun s => Real.sqrt (Q s)) = (fun s => (β + γ * s) / Real.sqrt (Q s)) ∧
    deriv (deriv (fun s => Real.sqrt (Q s))) = (fun s => (α * γ - β ^ 2) / Real.sqrt (Q s) ^ 3) ∧
    deriv (deriv (deriv (fun s => Real.sqrt (Q s))))
      = (fun s => -3 * (α * γ - β ^ 2) * (β + γ * s) / Real.sqrt (Q s) ^ 5) := by
  intro Q
  have hQ : ∀ s, 0 < Q s := by
    intro s; simp only [Q]
    -- γ·Q = (γs + β)² + (αγ − β²) > 0
    have : γ * (α + 2 * β * s + γ * s ^ 2) = (γ * s + β) ^ 2 + (α * γ - β ^ 2) := by ring
    nlinarith [sq_nonneg (γ * s + β)]
  have hq : ∀ s, HasDerivAt Q (2 * β + 2 * γ * s) s := by
    intro s
    have := (((hasDerivAt_id s).const_mul (2 * β)).const_add α).add (((hasDerivAt_id s).pow 2).const_mul γ)
    convert this using 1
    · funext y; simp only [Q, id, Pi.add_apply, Pi.pow_apply]
    · simp only [id]; ring
  have sq : ∀ s, 0 < Real.sqrt (Q s) := fun s => Real.sqrt_pos.mpr (hQ s)
  have hr : ∀ s, HasDerivAt (fun s => Real.sqrt (Q s)) ((β + γ * s) / Real.sqrt (Q s)) s := by
    intro s
    have := (hq s).sqrt (hQ s).ne'
    convert this using 1
    field_simp
  have e1 : deriv (fun s => Real.sqrt (Q s)) = fun s => (β + γ * s) / Real.sqrt (Q s) := by
    funext s; exact (hr s).deriv
  have hr2 : ∀ s, HasDerivAt (fun s => (β + γ * s) / Real.sqrt (Q s))
      ((α * γ - β ^ 2) / Real.sqrt (Q s) ^ 3) s := by
    intro s
    have := (((hasDerivAt_id s).const_mul γ).const_add β).div (hr s) (sq s).ne'
    convert this using 1
    · funext y; simp only [id, Pi.div_apply]
    · have h2 : Real.sqrt (Q s) ^ 2 = Q s := Real.sq_sqrt (hQ s).le
      have hs := (sq s).ne'
      simp only [id, mul_one]
      field_simp
      rw [h2]; simp only [Q]; ring
  have e2 : deriv (deriv (fun s => Real.sqrt (Q s))) = fun s => (α * γ - β ^ 2) / Real.sqrt (Q s) ^ 3 := by
    rw [e1]; funext s; exact (hr2 s).deriv
  have hr3 : ∀ s, HasDerivAt (fun s => (α * γ - β ^ 2) / Real.sqrt (Q s) ^ 3)
      (-3 * (α * γ - β ^ 2) * (β + γ * s) / Real.sqrt (Q s) ^ 5) s := by
    intro s
    have hs := (sq s).ne'
    convert (hasDerivAt_const s (α * γ - β ^ 2)).div ((hr s).pow 3) (pow_ne_zero 3 hs) using 1
    simp only [Pi.pow_apply]
    field_simp
    ring
  refine ⟨hQ, e1, e2, ?_⟩
  rw [e2]; funext s; exact (hr3 s).deriv

/-- The third derivative of ρ along the chord, in size, on [0, 1]: at most
3(αγ − β²)·max(|β|, |β + γ|)/ρ⁵ for ρ ≥ ρ₀ > 0 there. -/
theorem shadow_third_le (α β γ ρ₀ : ℝ) (γ0 : 0 < γ) (hd : 0 < α * γ - β ^ 2) (hρ : 0 < ρ₀)
    (hmin : ∀ s ∈ Set.Icc (0 : ℝ) 1, ρ₀ ≤ Real.sqrt (α + 2 * β * s + γ * s ^ 2))
    (s : ℝ) (hs : s ∈ Set.Icc (0 : ℝ) 1) :
    |deriv (deriv (deriv (fun s => Real.sqrt (α + 2 * β * s + γ * s ^ 2)))) s|
      ≤ 3 * (α * γ - β ^ 2) * max |β| |β + γ| / ρ₀ ^ 5 := by
  obtain ⟨_, _, _, e3⟩ := shadow_derivs α β γ γ0 hd
  rw [e3]
  simp only
  have r := hmin s hs
  have r5 : ρ₀ ^ 5 ≤ Real.sqrt (α + 2 * β * s + γ * s ^ 2) ^ 5 := pow_le_pow_left₀ hρ.le r 5
  have lin : |β + γ * s| ≤ max |β| |β + γ| := by
    obtain ⟨s0, s1⟩ := hs
    rw [abs_le]
    constructor <;> rcases le_total 0 γ with g | g <;>
      nlinarith [le_max_left |β| |β + γ|, le_max_right |β| |β + γ|, neg_abs_le β, neg_abs_le (β + γ),
        le_abs_self β, le_abs_self (β + γ)]
  have rp : 0 < Real.sqrt (α + 2 * β * s + γ * s ^ 2) := lt_of_lt_of_le hρ r
  rw [abs_div, abs_mul, abs_mul, abs_of_pos (pow_pos rp 5), abs_of_pos hd,
    show |(-3 : ℝ)| = 3 by norm_num]
  calc 3 * (α * γ - β ^ 2) * |β + γ * s| / Real.sqrt (α + 2 * β * s + γ * s ^ 2) ^ 5
      ≤ 3 * (α * γ - β ^ 2) * max |β| |β + γ| / Real.sqrt (α + 2 * β * s + γ * s ^ 2) ^ 5 := by
        gcongr
    _ ≤ 3 * (α * γ - β ^ 2) * max |β| |β + γ| / ρ₀ ^ 5 := by
        gcongr

/-- A point h from the core circle (h ≥ 0) whose F = h² − r² is below c·(2r − c) in size lies
within c of the torus: |h − r| < c (0 < c ≤ r). -/
theorem F_sag (h r c : ℝ) (h0 : 0 ≤ h) (hc : 0 < c) (hcr : c ≤ r) (hF : |h ^ 2 - r ^ 2| < c * (2 * r - c)) :
    |h - r| < c := by
  rw [abs_lt] at hF ⊢
  constructor <;> by_contra k <;> push Not at k <;> nlinarith

/-- **The torus's chord bound.** Along a chord, G(s) = a₀ + a₁s + a₂s² − 2R·√(α + 2βs + γs²)
(F at the chord's point s: the polynomial part |p|² + R² − r², the root ρ), zero at both ends,
the shadows not on a line through the axis (γ > 0, αγ − β² > 0) and ρ ≥ ρ₀ > 0 on the chord:
|G(s)| ≤ |G(½)| + √3·R·(αγ − β²)·max(|β|, |β + γ|)/(36ρ₀⁵) there. -/
theorem stray_bound (a₀ a₁ a₂ R α β γ ρ₀ : ℝ) (hR : 0 ≤ R) (γ0 : 0 < γ) (hd : 0 < α * γ - β ^ 2)
    (hρ : 0 < ρ₀) (hmin : ∀ s ∈ Set.Icc (0 : ℝ) 1, ρ₀ ≤ Real.sqrt (α + 2 * β * s + γ * s ^ 2))
    (G0 : a₀ + a₁ * 0 + a₂ * 0 ^ 2 - 2 * R * Real.sqrt (α + 2 * β * 0 + γ * 0 ^ 2) = 0)
    (G1 : a₀ + a₁ * 1 + a₂ * 1 ^ 2 - 2 * R * Real.sqrt (α + 2 * β * 1 + γ * 1 ^ 2) = 0)
    (s : ℝ) (hs : s ∈ Set.Icc (0 : ℝ) 1) :
    |a₀ + a₁ * s + a₂ * s ^ 2 - 2 * R * Real.sqrt (α + 2 * β * s + γ * s ^ 2)|
      ≤ |a₀ + a₁ * (1 / 2) + a₂ * (1 / 2) ^ 2 - 2 * R * Real.sqrt (α + 2 * β * (1 / 2) + γ * (1 / 2) ^ 2)|
        + Real.sqrt 3 * R * (α * γ - β ^ 2) * max |β| |β + γ| / (36 * ρ₀ ^ 5) := by
  obtain ⟨hQ, e1, e2, e3⟩ := shadow_derivs α β γ γ0 hd
  set ρ : ℝ → ℝ := fun s => Real.sqrt (α + 2 * β * s + γ * s ^ 2)
  set G : ℝ → ℝ := fun s => a₀ + a₁ * s + a₂ * s ^ 2 - 2 * R * ρ s
  -- G is C³ (the quadratic under the root never vanishes)
  have hρc : ContDiff ℝ 3 ρ := by
    refine ContDiff.sqrt ?_ (fun x => (hQ x).ne')
    exact (contDiff_const.add (contDiff_const.mul contDiff_id)).add (contDiff_const.mul (contDiff_id.pow 2))
  have hG : ContDiff ℝ 3 G :=
    ((contDiff_const.add (contDiff_const.mul contDiff_id)).add (contDiff_const.mul (contDiff_id.pow 2))).sub
      (contDiff_const.mul hρc)
  -- its third derivative is −2R·ρ‴
  have dρ : Differentiable ℝ ρ := hρc.differentiable (by norm_num)
  have h1 : deriv G = fun s => a₁ + 2 * a₂ * s - 2 * R * deriv ρ s := by
    funext x
    have := ((((hasDerivAt_id x).const_mul a₁).const_add a₀).add (((hasDerivAt_id x).pow 2).const_mul a₂)).sub
      ((dρ x).hasDerivAt.const_mul (2 * R))
    have hd' : HasDerivAt G (a₁ + 2 * a₂ * x - 2 * R * deriv ρ x) x := by
      convert this using 1
      · funext y; simp only [G, id, Pi.add_apply, Pi.sub_apply, Pi.pow_apply]
      · simp only [id]; ring
    exact hd'.deriv
  have ρ1 : ContDiff ℝ 2 (deriv ρ) := (contDiff_succ_iff_deriv.mp (show ContDiff ℝ (2 + 1) ρ from hρc)).2.2
  have dρ1 : Differentiable ℝ (deriv ρ) := ρ1.differentiable (by norm_num)
  have h2 : deriv (deriv G) = fun s => 2 * a₂ - 2 * R * deriv (deriv ρ) s := by
    rw [h1]; funext x
    have := ((((hasDerivAt_id x).const_mul (2 * a₂)).const_add a₁)).sub ((dρ1 x).hasDerivAt.const_mul (2 * R))
    have hd' : HasDerivAt (fun s => a₁ + 2 * a₂ * s - 2 * R * deriv ρ s) (2 * a₂ - 2 * R * deriv (deriv ρ) x) x := by
      convert this using 1
      · funext y; simp only [id, Pi.sub_apply]
      · ring
    exact hd'.deriv
  have ρ2 : ContDiff ℝ 1 (deriv (deriv ρ)) := (contDiff_succ_iff_deriv.mp (show ContDiff ℝ (1 + 1) (deriv ρ) from ρ1)).2.2
  have dρ2 : Differentiable ℝ (deriv (deriv ρ)) := ρ2.differentiable one_ne_zero
  have h3 : deriv (deriv (deriv G)) = fun s => -(2 * R * deriv (deriv (deriv ρ)) s) := by
    rw [h2]; funext x
    have := (hasDerivAt_const x (2 * a₂)).sub ((dρ2 x).hasDerivAt.const_mul (2 * R))
    have hd' : HasDerivAt (fun s => 2 * a₂ - 2 * R * deriv (deriv ρ) s) (-(2 * R * deriv (deriv (deriv ρ)) x)) x := by
      convert this using 1
      simp
    exact hd'.deriv
  -- the remainder, with M = 2R·3(αγ − β²)·max(|β|, |β + γ|)/ρ₀⁵
  set M := 2 * R * (3 * (α * γ - β ^ 2) * max |β| |β + γ| / ρ₀ ^ 5)
  have hM : ∀ x ∈ Set.Icc (0 : ℝ) 1, |deriv (deriv (deriv G)) x| ≤ M := by
    intro x hx
    rw [h3]; simp only
    rw [abs_neg, abs_mul, abs_of_nonneg (by positivity : (0 : ℝ) ≤ 2 * R)]
    exact mul_le_mul_of_nonneg_left (shadow_third_le α β γ ρ₀ γ0 hd hρ hmin x hx) (by positivity)
  have rem := interp_remainder G hG M (by simpa [G, ρ] using G0) (by simpa [G, ρ] using G1) hM s hs.1 hs.2
  obtain ⟨s0, s1⟩ := hs
  have w : |4 * s * (1 - s) * G (1 / 2)| ≤ |G (1 / 2)| := by
    rw [abs_mul]
    have : |4 * s * (1 - s)| ≤ 1 := by
      rw [abs_le]; constructor <;> nlinarith [sq_nonneg (2 * s - 1)]
    nlinarith [abs_nonneg (G (1 / 2))]
  have tri := abs_sub_abs_le_abs_sub (G s) (4 * s * (1 - s) * G (1 / 2))
  have eM : M * Real.sqrt 3 / 216
      = Real.sqrt 3 * R * (α * γ - β ^ 2) * max |β| |β + γ| / (36 * ρ₀ ^ 5) := by
    simp only [M]; field_simp; ring
  show |G s| ≤ |G (1 / 2)| + _
  linarith

end Sag
