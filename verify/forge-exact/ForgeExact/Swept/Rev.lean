import ForgeExact.Topology

/-!
# Revolve: deciding against a radius exactly (research/sweep-spec/DESIGN.md §3.1)

A revolve's membership at a point asks where the point's distance from the axis, `σ = √s` with
`s` rational, lies against the profile's edges: the sign of `A − σ·B` with `A`, `B` rational. That
sign needs no square root (`lt_mul_sqrt_iff`): so a revolve of a polygon is decided exactly over
ℚ, the same way its solid is a boolean of planes and quadrics (DESIGN.md §3.1, L2).

The axis-side refusal (`CrossesAxis`) is exact in the kernel too (forge-ops `Segment::reach`): an
arc's least distance across the axis is at its ends or where it passes the direction away from
the axis; `reach_arc` below is that fact.
-/

namespace ForgeExact.Swept.Rev

open Lean.Grind

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

theorem le_or_lt' (a b : K) : a ≤ b ∨ b < a := by grind

theorem lt_or_ge' (a b : K) : a < b ∨ b ≤ a := by grind

theorem sq_nonneg' (a : K) : 0 ≤ a * a := by
  rcases le_or_lt' 0 a with h | h
  · exact OrderedRing.mul_nonneg h h
  · have : 0 ≤ -a := by grind
    have := OrderedRing.mul_nonneg this this
    grind

theorem mul_lt_mul_of_pos_left' {a b c : K} (h : a < b) (hc : 0 < c) : c * a < c * b := by
  have := OrderedRing.mul_pos hc (show 0 < b - a by grind)
  grind

theorem mul_self_lt_mul_self' {a b : K} (ha : 0 ≤ a) (h : a < b) : a * a < b * b := by
  have hb : 0 < b := by grind
  have h1 : a * a ≤ a * b := by
    rcases Std.le_iff_lt_or_eq.mp ha with ha' | ha'
    · exact Std.le_of_lt (mul_lt_mul_of_pos_left' h ha')
    · subst ha'; grind
  have h2 : a * b < b * b := by
    have := mul_lt_mul_of_pos_left' h hb
    grind
  grind

/-- For nonnegative `a`, `b`: `a < b` exactly when `a² < b²`. -/
theorem lt_iff_sq_lt {a b : K} (ha : 0 ≤ a) (hb : 0 ≤ b) : a < b ↔ a * a < b * b := by
  constructor
  · exact mul_self_lt_mul_self' ha
  · intro h
    rcases lt_or_ge' a b with h' | h'
    · exact h'
    · have : b * b ≤ a * a := by
        rcases Std.le_iff_lt_or_eq.mp h' with h'' | h''
        · exact Std.le_of_lt (mul_self_lt_mul_self' hb h'')
        · rw [h'']; exact Std.le_refl _
      exact absurd h (by grind)

/-- **The sign against a square root, without one.** With `σ ≥ 0` and `σ² = s`:
`σ·B < A` exactly when `B ≤ 0` and (`A > 0` or `A² < s·B²`), or `B > 0`, `A > 0` and `s·B² < A²`. -/
theorem lt_mul_sqrt_iff (σ s A B : K) (hσ : 0 ≤ σ) (hs : σ * σ = s) :
    σ * B < A ↔ (B ≤ 0 ∧ (0 < A ∨ A * A < s * (B * B))) ∨ (0 < B ∧ 0 < A ∧ s * (B * B) < A * A) := by
  have e : s * (B * B) = (σ * B) * (σ * B) := by rw [← hs]; grind
  rw [e]
  rcases lt_or_ge' 0 B with hB | hB
  · -- B > 0: σB ≥ 0
    have hσB : 0 ≤ σ * B := OrderedRing.mul_nonneg hσ (Std.le_of_lt hB)
    constructor
    · intro h
      have hA : 0 < A := by grind
      exact Or.inr ⟨hB, hA, (lt_iff_sq_lt hσB (Std.le_of_lt hA)).mp h⟩
    · rintro (⟨hB', _⟩ | ⟨_, hA, h⟩)
      · exact absurd hB' (by grind)
      · exact (lt_iff_sq_lt hσB (Std.le_of_lt hA)).mpr h
  · -- B ≤ 0: σB ≤ 0, write it −(σ·(−B))
    have hσB : 0 ≤ σ * -B := OrderedRing.mul_nonneg hσ (by grind)
    have e2 : (σ * B) * (σ * B) = (σ * -B) * (σ * -B) := by grind
    constructor
    · intro h
      refine Or.inl ⟨hB, ?_⟩
      rcases lt_or_ge' 0 A with hA | hA
      · exact Or.inl hA
      · -- σB < A ≤ 0: −A < σ(−B)
        refine Or.inr ?_
        rw [e2, show A * A = (-A) * (-A) by grind]
        exact (lt_iff_sq_lt (by grind) hσB).mp (by grind)
    · rintro (⟨_, hA | h⟩ | ⟨hB', _⟩)
      · grind
      · rw [e2, show A * A = (-A) * (-A) by grind] at h
        rcases lt_or_ge' 0 A with hA | hA
        · grind
        · have := (lt_iff_sq_lt (by grind) hσB).mpr h
          grind
      · exact absurd hB' (by grind)

/-- **An arc's reach along a direction.** On the circle `c + r·(cos θ, sin θ)` the reach along a
unit `u` is `c·u + r cos(θ − θᵤ)`: at least `c·u − r` everywhere, with equality only at the point
facing `−u`. So over an arc the least reach is at an end, or `c·u − r` when the arc passes `−u`
(the kernel's `Segment::reach`). Stated for any point `p` of the circle (`|p − c|² = r²`). -/
theorem reach_arc (cx cy ux uy px py r : K) (hu : ux * ux + uy * uy = 1) (hr : 0 ≤ r)
    (hp : (px - cx) * (px - cx) + (py - cy) * (py - cy) = r * r) :
    cx * ux + cy * uy - r ≤ px * ux + py * uy := by
  -- (p − c)·u ≥ −|p − c| |u| = −r (Cauchy–Schwarz, as a square)
  have cs : ((px - cx) * ux + (py - cy) * uy) * ((px - cx) * ux + (py - cy) * uy) ≤ r * r := by
    have h0 := sq_nonneg' ((px - cx) * uy - (py - cy) * ux)
    -- Lagrange: (a·u)² + (a×u)² = |a|² |u|²
    have lag : ((px - cx) * ux + (py - cy) * uy) * ((px - cx) * ux + (py - cy) * uy) +
        ((px - cx) * uy - (py - cy) * ux) * ((px - cx) * uy - (py - cy) * ux) =
        ((px - cx) * (px - cx) + (py - cy) * (py - cy)) * (ux * ux + uy * uy) := by grind
    rw [hp, hu] at lag
    grind
  rcases lt_or_ge' ((px - cx) * ux + (py - cy) * uy) (-r) with h | h
  · have : r * r < ((px - cx) * ux + (py - cy) * uy) * ((px - cx) * ux + (py - cy) * uy) := by
      have := mul_self_lt_mul_self' hr (show r < -((px - cx) * ux + (py - cy) * uy) by grind)
      grind
    exact absurd cs (by grind)
  · grind

end ForgeExact.Swept.Rev
