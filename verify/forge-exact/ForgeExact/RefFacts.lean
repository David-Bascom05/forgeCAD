import ForgeExact.Flat

/-!
# Facts about the executable reference itself

* `step_keeps_sign` : the step off plane `P` that `Ref.boundary` takes, `m ± δ n_P` with
  `0 < δ ≤ |Q(m)| / (2 (|n_P · n_Q| + 1))` for every other plane `Q`, crosses no other plane. So
  the two points classified for a 2-D cell lie in the two 3-D cells beside it, one each side of
  `P` (`step_leaves_plane`), and `Flat.flat_boolean` applies to them as sample points.

The reference's former assumption about its input, that a closed planar B-rep's ray-parity
membership (`Ref.Solid.contains`) is constant on every cell of the arrangement of its planes (the
Jordan–Brouwer property of a closed polyhedral surface), is now a theorem:
`ForgeExact.Jordan.contains_const`, for solids whose loops lie in their face planes and whose
edges are each used an even number of times. The kernel's own `check` certifies both of the tools
handed over.
-/

namespace ForgeExact.RefFacts

open Ref Flat

theorem qabs_eq_or (a : Rat) : qabs a = a ∨ qabs a = -a := by unfold qabs; split <;> simp

/-- The step keeps the sign of every other plane. -/
theorem step_keeps_sign (P Q : Plane) (m : V3) (δ : Rat) (hm : Q.eval m ≠ 0) (hδ : 0 < δ)
    (hle : δ ≤ qabs (Q.eval m) / (2 * (qabs (P.n.dot Q.n) + 1))) (s : Rat) (hs : s = 1 ∨ s = -1) :
    (Q.eval m > 0 ↔ Q.eval (m + (s * δ) * P.n) > 0) ∧
      (Q.eval m < 0 ↔ Q.eval (m + (s * δ) * P.n) < 0) := by
  rw [eval_step]
  have hcomm : Q.n.dot P.n = P.n.dot Q.n := by simp only [V3.dot]; grind
  rw [hcomm]
  have hb : qabs (s * P.n.dot Q.n) = qabs (P.n.dot Q.n) := by
    rcases hs with rfl | rfl
    · simp
    · rcases qabs_eq_or (P.n.dot Q.n) with h | h <;> rcases qabs_eq_or (-1 * P.n.dot Q.n) with h' | h' <;>
        unfold qabs at * <;> split at * <;> split at * <;> grind
  have e : s * δ * P.n.dot Q.n = δ * (s * P.n.dot Q.n) := by grind
  rw [e]
  rcases qabs_eq_or (Q.eval m) with ha | ha
  · -- Q(m) > 0
    have hpos : 0 < Q.eval m := by
      have := qabs_nonneg (Q.eval m); rw [ha] at this; grind
    have := pos_of_small hpos hδ (b := s * P.n.dot Q.n) (by rw [hb]; rw [ha] at hle; exact hle)
    constructor <;> constructor <;> intro h <;> grind
  · -- Q(m) < 0
    have hneg : 0 < -Q.eval m := by
      have := qabs_nonneg (Q.eval m); rw [ha] at this; grind
    have := pos_of_small hneg hδ (b := -(s * P.n.dot Q.n)) (by
      have : qabs (-(s * P.n.dot Q.n)) = qabs (s * P.n.dot Q.n) := by
        unfold qabs; split <;> split <;> grind
      rw [this, hb]; rw [ha] at hle; exact hle)
    constructor <;> constructor <;> intro h <;> grind

/-- And the step leaves `P` itself, on the side asked for. -/
theorem step_leaves_plane (P : Plane) (hP : P.n ≠ ⟨0, 0, 0⟩) (m : V3) (hm : P.eval m = 0) (δ : Rat)
    (hδ : 0 < δ) : P.eval (m + δ * P.n) > 0 ∧ P.eval (m + (-1 * δ) * P.n) < 0 := by
  rw [eval_step, eval_step, hm]
  have h := Rat.mul_pos hδ (norm2_pos hP)
  constructor <;> grind

end ForgeExact.RefFacts
