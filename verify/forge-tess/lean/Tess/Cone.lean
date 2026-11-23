import Mathlib

namespace Sag

/-- A function convex on [0, 1] and zero at both ends dips at most twice as deep anywhere in it
as at its middle: f(s) ≥ 2·f(1/2) (so a chord, on which a cone's signed distance is convex,
sags at most twice its middle's sag). -/
theorem convex_dip (f : ℝ → ℝ) (hf : ConvexOn ℝ (Set.Icc 0 1) f) (h0 : f 0 = 0) (h1 : f 1 = 0)
    (s : ℝ) (hs : s ∈ Set.Icc (0 : ℝ) 1) : 2 * f (1 / 2) ≤ f s := by
  obtain ⟨s0, s1⟩ := hs
  have hm : f (1 / 2) ≤ 0 := by
    have := hf.2 (show (0 : ℝ) ∈ Set.Icc 0 1 by simp) (show (1 : ℝ) ∈ Set.Icc 0 1 by simp)
      (show (0 : ℝ) ≤ 1 / 2 by norm_num) (show (0 : ℝ) ≤ 1 / 2 by norm_num) (by norm_num)
    simp only [smul_eq_mul, h0, h1] at this
    norm_num at this
    linarith
  rcases le_total s (1 / 2) with h | h
  · -- 1/2 lies between s and 1: 1/2 = a·s + b·1 with a = 1/(2(1 − s))
    have hs1 : 0 < 1 - s := by linarith
    have a0 : 0 ≤ 1 / (2 * (1 - s)) := by positivity
    have b0 : 0 ≤ 1 - 1 / (2 * (1 - s)) := by
      rw [sub_nonneg, div_le_one (by linarith)]; linarith
    have := hf.2 (show s ∈ Set.Icc (0 : ℝ) 1 from ⟨s0, s1⟩) (show (1 : ℝ) ∈ Set.Icc 0 1 by simp)
      a0 b0 (by ring)
    simp only [smul_eq_mul, h1, mul_zero, add_zero] at this
    have e : 1 / (2 * (1 - s)) * s + (1 - 1 / (2 * (1 - s))) * 1 = 1 / 2 := by
      field_simp; ring
    rw [e] at this
    -- f(1/2) ≤ f(s)/(2(1 − s)), so 2(1 − s)·f(1/2) ≤ f(s), and 2(1 − s) ≤ 2 with f(1/2) ≤ 0
    have k : 2 * (1 - s) * f (1 / 2) ≤ f s := by
      have := mul_le_mul_of_nonneg_left this (show 0 ≤ 2 * (1 - s) by linarith)
      rwa [← mul_assoc, mul_one_div_cancel (by linarith), one_mul] at this
    nlinarith
  · have hs0 : 0 < s := by linarith
    have a0 : 0 ≤ 1 / (2 * s) := by positivity
    have b0 : 0 ≤ 1 - 1 / (2 * s) := by
      rw [sub_nonneg, div_le_one (by linarith)]; linarith
    have := hf.2 (show s ∈ Set.Icc (0 : ℝ) 1 from ⟨s0, s1⟩) (show (0 : ℝ) ∈ Set.Icc 0 1 by simp)
      a0 b0 (by ring)
    simp only [smul_eq_mul, h0, mul_zero, add_zero] at this
    have e : 1 / (2 * s) * s = 1 / 2 := by
      field_simp
    rw [e] at this
    have k : 2 * s * f (1 / 2) ≤ f s := by
      have := mul_le_mul_of_nonneg_left this (show 0 ≤ 2 * s by linarith)
      rwa [← mul_assoc, mul_one_div_cancel (by linarith), one_mul] at this
    nlinarith

/-- Along a chord a + s·d, the norm of a linear image plus an affine term is convex: the cone's
signed distance (ρ − R₀)·cos α − z·sin α (ρ the norm of the point's shadow on the plane square
to the axis, z its height, both linear) is convex along any chord. -/
theorem chord_convex {E F : Type*} [NormedAddCommGroup E] [NormedSpace ℝ E]
    [NormedAddCommGroup F] [NormedSpace ℝ F] (L : E →L[ℝ] F) (a d : E) (α β γ : ℝ) (hα : 0 ≤ α) :
    ConvexOn ℝ Set.univ (fun s : ℝ => α * ‖L (a + s • d)‖ + β * s + γ) := by
  have hn : ConvexOn ℝ Set.univ (fun s : ℝ => ‖L a + s • L d‖) := by
    refine ⟨convex_univ, fun x _ y _ p q hp hq hpq => ?_⟩
    simp only [smul_eq_mul]
    have : L a + (p * x + q * y) • L d = p • (L a + x • L d) + q • (L a + y • L d) := by
      rw [show q = 1 - p by linarith]; module
    rw [this]
    calc ‖p • (L a + x • L d) + q • (L a + y • L d)‖
        ≤ ‖p • (L a + x • L d)‖ + ‖q • (L a + y • L d)‖ := norm_add_le _ _
      _ = p * ‖L a + x • L d‖ + q * ‖L a + y • L d‖ := by
        rw [norm_smul, norm_smul, Real.norm_of_nonneg hp, Real.norm_of_nonneg hq]
  have e : (fun s : ℝ => α * ‖L (a + s • d)‖ + β * s + γ)
      = fun s => α * ‖L a + s • L d‖ + (β * s + γ) := by
    funext s; rw [map_add, map_smul]; ring
  rw [e]
  have ha : ConvexOn ℝ Set.univ (fun s : ℝ => β * s + γ) := by
    refine ⟨convex_univ, fun x _ y _ p q _ _ hpq => le_of_eq ?_⟩
    simp only [smul_eq_mul]
    rw [show q = 1 - p by linarith]; ring
  exact (hn.smul hα).add ha

end Sag
