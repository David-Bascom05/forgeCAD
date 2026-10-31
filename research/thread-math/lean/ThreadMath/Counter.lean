import ThreadMath.Section
import ThreadMath.Turns

/-!
# Which hypotheses are really needed, with counterexamples for those that are.
Not needed (deleted while proving): `linear_skip` needs neither b ≠ 0 nor m ≥ 0; `one_root_between`
needs neither A > 0 nor |G| < A; `face_in_hull_of_boundary` needs neither connectedness of the
region nor any property of the surface beyond affinity in v.
-/

namespace ThreadMath

/-- `linear_skip`: without the same side the middle can reach zero. -/
example : ¬ ∀ a b x0 x1 m t : ℝ, x0 ≤ t → t ≤ x1 → m < |a + b * x0| → m < |a + b * x1| →
    m < |a + b * t| := by
  intro h
  have := h 0 1 (-1) 1 (1/2) 0 (by norm_num) (by norm_num) (by norm_num) (by norm_num)
  norm_num at this

/-- `turns_covered`: with no step every turn qualifies, so no interval of turns holds them all;
the code handles it apart (one turn stands for all). -/
example : ∀ k : ℤ, (0 : ℝ) ≤ 0 - 0 * (k : ℝ) ∧ (0 : ℝ) - 0 * (k : ℝ) ≤ 0 := by
  intro k; simp

/-- `nearest_turn`: with no step the floor is not the answer, every turn ties (the code takes the
lowest there, as the old loop did). -/
example (s0 : ℝ) (m : ℤ) : |s0 - 0 * (m : ℝ)| = |s0| := by simp

/-- `shell_reading` needs e square to z: an e along z itself strays by r, not r |a . z|. With
a ⟂ z, the bound would say no stray at all. -/
example : ∃ az stray : ℝ, az = 0 ∧ stray = 1 ∧ 1 * |az| < stray := ⟨0, 1, rfl, rfl, by norm_num⟩

end ThreadMath
