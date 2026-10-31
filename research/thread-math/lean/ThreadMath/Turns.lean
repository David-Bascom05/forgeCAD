import Mathlib.Tactic
import Mathlib.Algebra.Order.Floor.Ring

/-!
# Which turns can be met: `turns_between`, and the fence's nearest turn (`Fence::value`)
-/

set_option linter.style.header false
set_option linter.style.longLine false
set_option linter.unusedSimpArgs false

namespace ThreadMath

/-- `turns_between`: every turn k where base - step k lands in the band, for some value in the
range base lies in, is in the interval the code loops over. So no turn is left out, however many
turns the surface has. Needs step ≠ 0 (with no step every turn or none qualifies: `Counter`). -/
theorem turns_covered {lo hi blo bhi step y : ℝ} {k : ℤ} (hstep : step ≠ 0)
    (hy : lo ≤ y ∧ y ≤ hi) (hb : blo ≤ y - step * k ∧ y - step * k ≤ bhi) :
    let x := (lo - bhi) / step
    let z := (hi - blo) / step
    ⌈min x z⌉ ≤ k ∧ k ≤ ⌊max x z⌋ := by
  intro x z
  rcases lt_or_gt_of_ne hstep with hs | hs
  · -- step < 0: k ≥ (hi - blo)/step, k ≤ (lo - bhi)/step
    have h1 : z ≤ (k : ℝ) := by
      simp only [z]; rw [div_le_iff_of_neg hs]; nlinarith [hy.2, hb.1]
    have h2 : (k : ℝ) ≤ x := by
      simp only [x]; rw [le_div_iff_of_neg hs]; nlinarith [hy.1, hb.2]
    exact ⟨Int.ceil_le.mpr ((min_le_right _ _).trans h1), Int.le_floor.mpr (h2.trans (le_max_left _ _))⟩
  · have h1 : x ≤ (k : ℝ) := by
      simp only [x]; rw [div_le_iff₀ hs]; nlinarith [hy.1, hb.2]
    have h2 : (k : ℝ) ≤ z := by
      simp only [z]; rw [le_div_iff₀ hs]; nlinarith [hy.2, hb.1]
    exact ⟨Int.ceil_le.mpr ((min_le_left _ _).trans h1), Int.le_floor.mpr (h2.trans (le_max_right _ _))⟩

/-- The fence's value: over all integers, |s0 - step m| is smallest at ⌊s0 / step⌋ or one above,
so the old loop over every turn can stop at two. Needs step ≠ 0; with step = 0 every turn ties. -/
theorem nearest_turn (s0 step : ℝ) (hs : step ≠ 0) (m : ℤ) :
    let f := ⌊s0 / step⌋
    min |s0 - step * f| |s0 - step * (f + 1 : ℤ)| ≤ |s0 - step * m| := by
  intro f
  set x := s0 / step
  have e0 : s0 = step * x := by simp only [x]; field_simp
  have hf := Int.floor_le x
  have hf' := Int.lt_floor_add_one x
  have scale : ∀ n : ℤ, |s0 - step * n| = |step| * |x - n| := by
    intro n; rw [e0, ← mul_sub, abs_mul]
  rw [scale m, scale f, scale (f + 1), ← mul_min_of_nonneg _ _ (abs_nonneg _)]
  apply mul_le_mul_of_nonneg_left _ (abs_nonneg _)
  rcases le_or_gt (m : ℤ) f with h | h
  · apply (min_le_left _ _).trans
    have : (m : ℝ) ≤ f := by exact_mod_cast h
    rw [abs_of_nonneg (by linarith), abs_of_nonneg (by linarith)]; linarith
  · apply (min_le_right _ _).trans
    have : (f : ℝ) + 1 ≤ m := by exact_mod_cast h
    push_cast
    rw [abs_of_neg (by linarith), abs_of_neg (by linarith)]; linarith

/-- The same with the turn clamped to the surface's [lo, hi], as the code clamps it: over the turns
that exist the smallest is at the clamped floor or one above it, kept within hi. -/
theorem nearest_turn_clamped (s0 step : ℝ) (hs : step ≠ 0) (lo hi m : ℤ) (hm : lo ≤ m ∧ m ≤ hi) :
    let c := max lo (min ⌊s0 / step⌋ hi)
    min |s0 - step * c| |s0 - step * (min (c + 1) hi : ℤ)| ≤ |s0 - step * m| := by
  intro c
  set x := s0 / step
  have e0 : s0 = step * x := by simp only [x]; field_simp
  have scale : ∀ n : ℤ, |s0 - step * n| = |step| * |x - n| := by
    intro n; rw [e0, ← mul_sub, abs_mul]
  rw [scale m, scale c, scale (min (c + 1) hi), ← mul_min_of_nonneg _ _ (abs_nonneg _)]
  apply mul_le_mul_of_nonneg_left _ (abs_nonneg _)
  have hf := Int.floor_le x
  have hf' := Int.lt_floor_add_one x
  have hlh : lo ≤ hi := hm.1.trans hm.2
  rcases le_total ⌊x⌋ lo with h1 | h1
  · -- floor at or below lo: c = lo, x below lo + 1
    have hc : c = lo := by
      simp only [c]; rw [max_eq_left]; exact (min_le_left _ _).trans h1
    have hx : x < (lo : ℝ) + 1 := by
      have : (⌊x⌋ : ℝ) ≤ lo := by exact_mod_cast h1
      linarith
    rcases eq_or_lt_of_le hm.1 with e | hlt
    · apply (min_le_left _ _).trans; rw [hc, ← e]
    · have hml : (lo : ℝ) + 1 ≤ m := by exact_mod_cast hlt
      have hlo1 : min (lo + 1) hi = lo + 1 := by
        apply min_eq_left; omega
      apply (min_le_right _ _).trans; rw [hc, hlo1]; push_cast
      rw [abs_of_neg (by linarith), abs_sub_comm]
      rcases le_or_gt ((m : ℝ)) x with h | h
      · exfalso; linarith
      · rw [abs_of_pos (by linarith)]; linarith
  · rcases le_total hi ⌊x⌋ with h2 | h2
    · have hc : c = hi := by
        simp only [c]; rw [min_eq_right h2]; exact max_eq_right hlh
      apply (min_le_left _ _).trans; rw [hc]
      have hhx : (hi : ℝ) ≤ x := by
        have : (hi : ℝ) ≤ ⌊x⌋ := by exact_mod_cast h2
        linarith
      have hmh : (m : ℝ) ≤ hi := by exact_mod_cast hm.2
      rw [abs_of_nonneg (by linarith), abs_of_nonneg (by linarith)]; linarith
    · have hc : c = ⌊x⌋ := by
        simp only [c]; rw [min_eq_left h2]; exact max_eq_right h1
      rcases le_or_gt m ⌊x⌋ with h | h
      · apply (min_le_left _ _).trans; rw [hc]
        have : (m : ℝ) ≤ ⌊x⌋ := by exact_mod_cast h
        rw [abs_of_nonneg (by linarith), abs_of_nonneg (by linarith)]; linarith
      · have hup : ⌊x⌋ + 1 ≤ hi := by omega
        apply (min_le_right _ _).trans; rw [hc, min_eq_left hup]; push_cast
        have : (⌊x⌋ : ℝ) + 1 ≤ m := by exact_mod_cast h
        rw [abs_of_neg (by linarith), abs_of_neg (by linarith)]; linarith

end ThreadMath
