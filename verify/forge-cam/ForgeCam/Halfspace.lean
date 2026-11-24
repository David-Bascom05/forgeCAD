/-
The half-space bound (kernel/forge-cam-geom/src/shadow.rs, `halfspace_bound`): a plane face lies
in its plane {x | ⟪N, x⟫ = c} (N a unit vector). If the moved tool lies in {x | D ≤ ⟪N, x⟫ − c},
its signed distance from every point of the plane is at least D, whichever the sign of D: a
plane point outside the tool is at least D from it (D > 0), and a plane point inside it is
within −D of the half-space the tool keeps out of (D ≤ 0). The checker takes the better of the
two senses of N and the least of ⟪N, x⟫ − c over the tool's profile and the move's piece.
-/
import ForgeCam.SignedDist
import ForgeCam.Hypograph

open Metric Set

namespace ForgeCam

theorem inner_sub_le_dist {N x y : E3} (hN : ‖N‖ = 1) : inner ℝ N (x - y) ≤ dist x y := by
  calc inner ℝ N (x - y) ≤ ‖N‖ * ‖x - y‖ := real_inner_le_norm N (x - y)
    _ = dist x y := by rw [hN, one_mul, dist_eq_norm]

/-- **The half-space bound.** -/
theorem sd_ge_of_halfspace {B : Set E3} (hne : B.Nonempty) {N P : E3} (hN : ‖N‖ = 1) {c D : ℝ}
    (hB : ∀ x ∈ B, D ≤ inner ℝ N x - c) (hP : inner ℝ N P = c) : D ≤ sd B P := by
  by_cases hPB : P ∈ B
  · -- P inside: D ≤ 0, and P + (D − ε) N is outside B, |D − ε| from P
    have hD : D ≤ 0 := by have := hB P hPB; linarith
    rw [sd_of_mem hPB]
    have key : ∀ ε > 0, infDist P Bᶜ ≤ -D + ε := by
      intro ε hε
      set y := P + (D - ε) • N
      have hy : y ∉ B := by
        intro hyB
        have h := hB y hyB
        have : inner ℝ N y = c + (D - ε) := by
          simp only [y, inner_add_right, inner_smul_right, hP, real_inner_self_eq_norm_sq, hN]
          ring
        linarith
      have hd : dist P y = -D + ε := by
        simp only [y, dist_eq_norm, sub_add_cancel_left, norm_neg, norm_smul, hN, mul_one,
          Real.norm_eq_abs]
        rw [abs_of_neg (by linarith)]; ring
      calc infDist P Bᶜ ≤ dist P y := infDist_le_dist_of_mem hy
        _ = -D + ε := hd
    have : infDist P Bᶜ ≤ -D := le_of_forall_pos_le_add fun ε hε => key ε hε
    linarith
  · -- P outside: every point of B is at least D from P
    rw [sd_of_not_mem hPB]
    rcases le_or_gt D 0 with hD | hD
    · exact hD.trans infDist_nonneg
    · refine (le_infDist hne).mpr fun x hx => ?_
      have h1 := hB x hx
      have h2 : inner ℝ N (x - P) = inner ℝ N x - c := by rw [inner_sub_right, hP]
      have h3 := inner_sub_le_dist (x := x) (y := P) hN
      rw [dist_comm] at h3
      linarith

end ForgeCam
