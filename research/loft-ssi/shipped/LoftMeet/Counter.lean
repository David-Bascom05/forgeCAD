import LoftMeet.Patch

/-!
# Counterexamples: which assumptions of the hull test are needed
-/

namespace LoftSSI

open Finset

/-- The unit circle in the plane as a span with constant data: x = cos θ (B = 1, A = D = 0). -/
def unitB : ℕ → ℝ := fun k => if k = 0 then 1 else 0

/-- **A negative weight breaks the hull.** For a piece turning more than half a turn
(π/2 < h < π, weight cos h < 0) the conversion is still exact (`patch_exact` needs only h < π),
but every control point has x < 0 while the arc passes through x = 1 (θ = xm = 0). -/
theorem neg_weight_counterexample {h : ℝ} (h1 : Real.pi / 2 < h) (h2 : h < Real.pi) :
    (∀ r < 4, ∀ j < 3, cpt (fun _ => 0) unitB (fun _ => 0) 0 1 0 h r j < 0) ∧
      |(0 : ℝ) - 0| ≤ |h| ∧ spanV (fun _ => 0) unitB (fun _ => 0) (0 + (1 - 0) * 0) 0 = 1 := by
  have hc : Real.cos h < 0 := Real.cos_neg_of_pi_div_two_lt_of_lt h1 (by linarith)
  refine ⟨fun r hr j hj => ?_, by simp, ?_⟩
  · have hb : bp unitB 0 1 r = 1 := by
      interval_cases r <;> simp [bp, bpts, cub, dcub, unitB]
    have ha : bp (fun _ => 0) 0 1 r = 0 := by
      interval_cases r <;> simp [bp, bpts, cub, dcub]
    simp only [cpt, ha, hb]
    interval_cases j <;> simp [hc, aw, Real.cos_neg, LoftSSI.hc, div_neg_of_pos_of_neg]
  · simp [spanV, cubv, cub, unitB]

/-- **A zero weight (exactly half a turn) has no finite middle point.** At h = π/2 the weight
cos h is 0 and the homogeneous middle point (cos xm, sin xm) = (1, 0) is a direction: the two
finite control points (cos ∓h, sin ∓h) have x = 0 while the arc reaches x = 1. The code refuses
|h| ≥ 0.49π (`Poly::hull`) and its pieces turn at most a quarter turn (h ≤ π/4). -/
theorem zero_weight_counterexample :
    aw (Real.pi / 2) 1 = 0 ∧ Real.cos (0 - Real.pi / 2) = 0 ∧ Real.cos (0 + Real.pi / 2) = 0 ∧
      Real.cos 0 = 1 := by
  simp [aw]

/-- Abstractly, for a rational Bezier form with FINITE control points, nonnegative weights and a
positive denominator suffice for the hull property (a zero weight is allowed). -/
theorem rational_hull {ι : Type*} (s : Finset ι) (a x : ι → ℝ) (M : ℝ) (ha : ∀ i ∈ s, 0 ≤ a i)
    (hpos : 0 < ∑ i ∈ s, a i) (hx : ∀ i ∈ s, x i ≤ M) :
    (∑ i ∈ s, a i * x i) / (∑ i ∈ s, a i) ≤ M := by
  rw [div_le_iff₀ hpos, mul_sum]
  exact sum_le_sum fun i hi => by rw [mul_comm M]; exact mul_le_mul_of_nonneg_left (hx i hi) (ha i hi)

end LoftSSI
