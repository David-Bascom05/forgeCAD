import Mathlib

namespace Sag

/-- Walking from a point with weights `l` (one of them negative) towards one with weights `m`
(all ≥ 0), all summing to one: at the last of the times its negative weights reach zero, every
weight is ≥ 0 and one is 0. -/
theorem walk_to_edge (l₁ l₂ l₃ m₁ m₂ m₃ : ℝ)
    (q₁ : 0 ≤ m₁) (q₂ : 0 ≤ m₂) (q₃ : 0 ≤ m₃) (neg : l₁ < 0 ∨ l₂ < 0 ∨ l₃ < 0) :
    ∃ t : ℝ, 0 ≤ t ∧ t ≤ 1 ∧
      0 ≤ (1 - t) * l₁ + t * m₁ ∧ 0 ≤ (1 - t) * l₂ + t * m₂ ∧ 0 ≤ (1 - t) * l₃ + t * m₃ ∧
      ((1 - t) * l₁ + t * m₁ = 0 ∨ (1 - t) * l₂ + t * m₂ = 0 ∨ (1 - t) * l₃ + t * m₃ = 0) := by
  -- each weight's time to reach zero (0 for one already ≥ 0)
  let r : ℝ → ℝ → ℝ := fun l m => if l < 0 then l / (l - m) else 0
  have r_mem : ∀ l m : ℝ, 0 ≤ m → 0 ≤ r l m ∧ r l m ≤ 1 := by
    intro l m hm
    simp only [r]
    split_ifs with h
    · have hd : 0 < m - l := by linarith
      rw [show l / (l - m) = (-l) / (m - l) by rw [← neg_div_neg_eq]; ring_nf]
      exact ⟨div_nonneg (by linarith) hd.le, (div_le_one hd).mpr (by linarith)⟩
    · exact ⟨le_refl _, zero_le_one⟩
  -- past its time, a weight is ≥ 0; at its time, a negative one is 0
  have past : ∀ l m t : ℝ, 0 ≤ m → r l m ≤ t → t ≤ 1 → 0 ≤ (1 - t) * l + t * m := by
    intro l m t hm hr ht
    simp only [r] at hr
    split_ifs at hr with h
    · have hd : 0 < m - l := by linarith
      rw [show l / (l - m) = (-l) / (m - l) by rw [← neg_div_neg_eq]; ring_nf] at hr
      rw [div_le_iff₀ hd] at hr
      nlinarith
    · nlinarith
  have at_time : ∀ l m : ℝ, l < 0 → 0 ≤ m → (1 - r l m) * l + r l m * m = 0 := by
    intro l m h hm
    simp only [r, h, ite_true]
    have hd : l - m ≠ 0 := by linarith
    field_simp
    ring
  set a := r l₁ m₁
  set b := r l₂ m₂
  set c := r l₃ m₃
  obtain ⟨a0, a1⟩ := r_mem l₁ m₁ q₁
  obtain ⟨b0, b1⟩ := r_mem l₂ m₂ q₂
  obtain ⟨c0, c1⟩ := r_mem l₃ m₃ q₃
  -- a weight that is ≥ 0 at the start has time 0, below any negative one's (which is > 0)
  have pos : ∀ l m : ℝ, l < 0 → 0 ≤ m → 0 < r l m := by
    intro l m h hm
    simp only [r, h, ite_true]
    exact div_pos_of_neg_of_neg h (by linarith)
  have zero : ∀ l m : ℝ, ¬ l < 0 → r l m = 0 := by
    intro l m h; simp only [r, h, ite_false]
  -- the latest of the three times
  rcases le_total a b with hab | hab <;> rcases le_total b c with hbc | hbc <;>
    rcases le_total a c with hac | hac
  all_goals first
    | (refine ⟨c, c0, c1, past _ _ _ q₁ (by linarith) c1, past _ _ _ q₂ (by linarith) c1,
        past _ _ _ q₃ le_rfl c1, Or.inr (Or.inr (at_time _ _ ?_ q₃))⟩
       by_contra hn
       have := zero l₃ m₃ hn
       rcases neg with h | h | h
       · linarith [pos l₁ m₁ h q₁]
       · linarith [pos l₂ m₂ h q₂]
       · exact hn h)
    | (refine ⟨b, b0, b1, past _ _ _ q₁ (by linarith) b1, past _ _ _ q₂ le_rfl b1,
        past _ _ _ q₃ (by linarith) b1, Or.inr (Or.inl (at_time _ _ ?_ q₂))⟩
       by_contra hn
       have := zero l₂ m₂ hn
       rcases neg with h | h | h
       · linarith [pos l₁ m₁ h q₁]
       · exact hn h
       · linarith [pos l₃ m₃ h q₃])
    | (refine ⟨a, a0, a1, past _ _ _ q₁ le_rfl a1, past _ _ _ q₂ (by linarith) a1,
        past _ _ _ q₃ (by linarith) a1, Or.inl (at_time _ _ ?_ q₁)⟩
       by_contra hn
       have := zero l₁ m₁ hn
       rcases neg with h | h | h
       · exact hn h
       · linarith [pos l₂ m₂ h q₂]
       · linarith [pos l₃ m₃ h q₃])

end Sag
