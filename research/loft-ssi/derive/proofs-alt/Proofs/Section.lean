import Mathlib

/-!
# Loft x loft: the exact section conditions

A volute-class wall span is `S(s, phi) = P(s) + cos phi * A(s) - sin phi * rho(s) * e_z` with
`P, A` horizontal (in R^2, written as `x, y` pairs) and `rho` scalar. Everything here is pointwise
in the section parameters, so `P, A, rho` are plain reals: the lemmas hold for any spline.

`cross u v = u.x v.y - u.y v.x`, `dot u v = u.x v.x + u.y v.y`.
-/

namespace LoftSSI

/-- The 2D cross product. -/
def cross (ux uy vx vy : ℝ) : ℝ := ux * vy - uy * vx

/-- The plane condition `g = A ^ (h - P)`. -/
def g (Ax Ay Px Py hx hy : ℝ) : ℝ := cross Ax Ay (hx - Px) (hy - Py)

/-- The in-plane ellipse condition `Psi = rho^2 ((h - P).A)^2 + |A|^4 (z^2 - rho^2)`. -/
def psi (Ax Ay Px Py rho hx hy z : ℝ) : ℝ :=
  rho ^ 2 * ((hx - Px) * Ax + (hy - Py) * Ay) ^ 2 + (Ax ^ 2 + Ay ^ 2) ^ 2 * (z ^ 2 - rho ^ 2)

/-- A point lies on the section ellipse iff `g = 0` and `Psi = 0` (A and rho nonzero). -/
theorem on_section_iff (Ax Ay Px Py rho hx hy z : ℝ) (hA : Ax ^ 2 + Ay ^ 2 ≠ 0) (hr : rho ≠ 0) :
    (∃ c s : ℝ, c ^ 2 + s ^ 2 = 1 ∧ hx = Px + c * Ax ∧ hy = Py + c * Ay ∧ z = -(s * rho)) ↔
      (g Ax Ay Px Py hx hy = 0 ∧ psi Ax Ay Px Py rho hx hy z = 0) := by
  constructor
  · rintro ⟨c, s, hcs, rfl, rfl, rfl⟩
    refine ⟨by unfold g cross; ring, ?_⟩
    have : psi Ax Ay Px Py rho (Px + c * Ax) (Py + c * Ay) (-(s * rho)) =
        (Ax ^ 2 + Ay ^ 2) ^ 2 * rho ^ 2 * (c ^ 2 + s ^ 2 - 1) := by unfold psi; ring
    rw [this, hcs]; ring
  · rintro ⟨hg, hp⟩
    set n := Ax ^ 2 + Ay ^ 2 with hn
    set c := ((hx - Px) * Ax + (hy - Py) * Ay) / n
    refine ⟨c, -(z / rho), ?_, ?_, ?_, ?_⟩
    · -- Psi = n^2 rho^2 (c^2 + s^2 - 1)
      have key : psi Ax Ay Px Py rho hx hy z = n ^ 2 * rho ^ 2 * (c ^ 2 + (-(z / rho)) ^ 2 - 1) := by
        unfold psi; simp only [c]; field_simp; ring
      rw [key] at hp
      have : n ^ 2 * rho ^ 2 ≠ 0 := by positivity
      have := (mul_eq_zero.mp hp).resolve_left this
      linarith
    · -- (h - P).x n = A.x (h - P).A - A.y g
      have e : (hx - Px) * n = Ax * ((hx - Px) * Ax + (hy - Py) * Ay) - Ay * g Ax Ay Px Py hx hy := by
        unfold g cross; rw [hn]; ring
      rw [hg] at e
      simp only [c]; field_simp; linarith
    · have e : (hy - Py) * n = Ay * ((hx - Px) * Ax + (hy - Py) * Ay) + Ax * g Ax Ay Px Py hx hy := by
        unfold g cross; rw [hn]; ring
      rw [hg] at e
      simp only [c]; field_simp; linarith
    · field_simp

/-- R2: two volute-class sections meet iff wall 2's section parameter `c2` in `[-1, 1]` solves
`g = Psi = 0` with `h = P2 + c2 A2`, `z^2 = rho2^2 (1 - c2^2)`. -/
theorem sections_meet_iff (A1x A1y P1x P1y r1 A2x A2y P2x P2y r2 : ℝ)
    (hA : A1x ^ 2 + A1y ^ 2 ≠ 0) (hr : r1 ≠ 0) :
    (∃ c1 s1 c2 s2 : ℝ, c1 ^ 2 + s1 ^ 2 = 1 ∧ c2 ^ 2 + s2 ^ 2 = 1 ∧
        P1x + c1 * A1x = P2x + c2 * A2x ∧ P1y + c1 * A1y = P2y + c2 * A2y ∧ s1 * r1 = s2 * r2) ↔
      (∃ c2 : ℝ, c2 ^ 2 ≤ 1 ∧ g A1x A1y P1x P1y (P2x + c2 * A2x) (P2y + c2 * A2y) = 0 ∧
        r1 ^ 2 * (((P2x + c2 * A2x) - P1x) * A1x + ((P2y + c2 * A2y) - P1y) * A1y) ^ 2 +
          (A1x ^ 2 + A1y ^ 2) ^ 2 * (r2 ^ 2 * (1 - c2 ^ 2) - r1 ^ 2) = 0) := by
  constructor
  · rintro ⟨c1, s1, c2, s2, h1, h2, hx, hy, hz⟩
    refine ⟨c2, by nlinarith [sq_nonneg s2], ?_⟩
    have := (on_section_iff A1x A1y P1x P1y r1 (P2x + c2 * A2x) (P2y + c2 * A2y) (-(s2 * r2)) hA hr).mp
      ⟨c1, s1, h1, hx.symm, hy.symm, by rw [hz]⟩
    refine ⟨this.1, ?_⟩
    have h := this.2
    unfold psi at h
    have : (-(s2 * r2)) ^ 2 = r2 ^ 2 * (1 - c2 ^ 2) := by
      have : s2 ^ 2 = 1 - c2 ^ 2 := by linarith
      rw [neg_sq, mul_pow, this]; ring
    rw [this] at h
    linarith
  · rintro ⟨c2, hc, hg, hp⟩
    set s2 := Real.sqrt (1 - c2 ^ 2)
    have hs2 : s2 ^ 2 = 1 - c2 ^ 2 := Real.sq_sqrt (by linarith)
    have hp' : psi A1x A1y P1x P1y r1 (P2x + c2 * A2x) (P2y + c2 * A2y) (-(s2 * r2)) = 0 := by
      unfold psi
      rw [neg_sq, mul_pow, hs2]
      linarith
    obtain ⟨c1, s1, h1, hx, hy, hz⟩ :=
      (on_section_iff A1x A1y P1x P1y r1 _ _ _ hA hr).mpr ⟨hg, hp'⟩
    exact ⟨c1, s1, c2, s2, h1, by linarith, hx.symm, hy.symm, by linarith⟩

/-- R1: the section-pair projection. With `W = A1 ^ A2 ≠ 0`, `D = P2 - P1`, `a1 = D ^ A1`,
`a2 = D ^ A2`: the sections meet iff `F = rho1^2 (W^2 - a2^2) - rho2^2 (W^2 - a1^2) = 0` and
`W^2 >= a2^2`; then `c1 = a2 / W`, `c2 = a1 / W`. -/
theorem pair_iff (A1x A1y P1x P1y r1 A2x A2y P2x P2y r2 : ℝ)
    (hW : cross A1x A1y A2x A2y ≠ 0) (hr1 : r1 ≠ 0) (hr2 : r2 ≠ 0) :
    let W := cross A1x A1y A2x A2y
    let a1 := cross (P2x - P1x) (P2y - P1y) A1x A1y
    let a2 := cross (P2x - P1x) (P2y - P1y) A2x A2y
    (∃ c1 s1 c2 s2 : ℝ, c1 ^ 2 + s1 ^ 2 = 1 ∧ c2 ^ 2 + s2 ^ 2 = 1 ∧
        P1x + c1 * A1x = P2x + c2 * A2x ∧ P1y + c1 * A1y = P2y + c2 * A2y ∧ s1 * r1 = s2 * r2) ↔
      (r1 ^ 2 * (W ^ 2 - a2 ^ 2) - r2 ^ 2 * (W ^ 2 - a1 ^ 2) = 0 ∧ a2 ^ 2 ≤ W ^ 2) := by
  intro W a1 a2
  have hW' : W ≠ 0 := hW
  constructor
  · rintro ⟨c1, s1, c2, s2, h1, h2, hx, hy, hz⟩
    -- the linear solve: c1 W = a2, c2 W = a1
    have e1 : c1 * W = a2 := by
      simp only [W, a2, cross]
      have : P2x - P1x = c1 * A1x - c2 * A2x := by linarith
      have h' : P2y - P1y = c1 * A1y - c2 * A2y := by linarith
      rw [this, h']; ring
    have e2 : c2 * W = a1 := by
      simp only [W, a1, cross]
      have : P2x - P1x = c1 * A1x - c2 * A2x := by linarith
      have h' : P2y - P1y = c1 * A1y - c2 * A2y := by linarith
      rw [this, h']; ring
    have hs1 : W ^ 2 - a2 ^ 2 = W ^ 2 * s1 ^ 2 := by
      rw [← e1]; have : s1 ^ 2 = 1 - c1 ^ 2 := by linarith
      rw [this]; ring
    have hs2 : W ^ 2 - a1 ^ 2 = W ^ 2 * s2 ^ 2 := by
      rw [← e2]; have : s2 ^ 2 = 1 - c2 ^ 2 := by linarith
      rw [this]; ring
    refine ⟨?_, ?_⟩
    · rw [hs1, hs2]
      have : (s1 * r1) ^ 2 = (s2 * r2) ^ 2 := by rw [hz]
      nlinarith [this]
    · nlinarith [sq_nonneg (W * s1), hs1]
  · rintro ⟨hF, hG⟩
    set c1 := a2 / W
    set c2 := a1 / W
    have hc1 : c1 * W = a2 := by simp only [c1]; field_simp
    have hc2 : c2 * W = a1 := by simp only [c2]; field_simp
    have h1le : c1 ^ 2 ≤ 1 := by
      have : c1 ^ 2 * W ^ 2 ≤ 1 * W ^ 2 := by rw [← mul_pow, hc1]; linarith
      exact le_of_mul_le_mul_right this (by positivity)
    set s1 := Real.sqrt (1 - c1 ^ 2)
    have hs1 : s1 ^ 2 = 1 - c1 ^ 2 := Real.sq_sqrt (by linarith)
    set s2 := s1 * r1 / r2
    refine ⟨c1, s1, c2, s2, by linarith, ?_, ?_, ?_, ?_⟩
    · -- r2^2 (1 - c2^2) = r1^2 (1 - c1^2) from F = 0 divided by W^2
      have hF' : W ^ 2 * (r1 ^ 2 * (1 - c1 ^ 2) - r2 ^ 2 * (1 - c2 ^ 2)) = 0 := by
        have : W ^ 2 * (r1 ^ 2 * (1 - c1 ^ 2) - r2 ^ 2 * (1 - c2 ^ 2)) =
            r1 ^ 2 * (W ^ 2 - (c1 * W) ^ 2) - r2 ^ 2 * (W ^ 2 - (c2 * W) ^ 2) := by ring
        rw [this, hc1, hc2]; exact hF
      have hW2 : W ^ 2 ≠ 0 := pow_ne_zero 2 hW'
      have h := (mul_eq_zero.mp hF').resolve_left hW2
      simp only [s2]
      field_simp
      rw [hs1]
      linear_combination h
    · have key : (P1x + c1 * A1x) * W = (P2x + c2 * A2x) * W := by
        have e : (P1x + c1 * A1x) * W = P1x * W + (c1 * W) * A1x := by ring
        have e' : (P2x + c2 * A2x) * W = P2x * W + (c2 * W) * A2x := by ring
        rw [e, e', hc1, hc2]; simp only [W, a1, a2, cross]; ring
      exact mul_right_cancel₀ hW' key
    · have key : (P1y + c1 * A1y) * W = (P2y + c2 * A2y) * W := by
        have e : (P1y + c1 * A1y) * W = P1y * W + (c1 * W) * A1y := by ring
        have e' : (P2y + c2 * A2y) * W = P2y * W + (c2 * W) * A2y := by ring
        rw [e, e', hc1, hc2]; simp only [W, a1, a2, cross]; ring
      exact mul_right_cancel₀ hW' key
    · simp only [s2]; field_simp

end LoftSSI
