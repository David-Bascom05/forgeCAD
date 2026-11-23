/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMesh.Funs
import AeneasF64.Lemmas

/-!
# The CDT's small helpers, against their meaning

Each lemma is about the translated function itself: it returns `ok` (no panic, overflow or
divergence), with the value said. The first stones of `Spec.Ends` and `Spec.Correct`.
-/

open Aeneas Aeneas.Std Result

namespace forge_mesh

/-- `nx`: the next corner round a triangle. -/
theorem nx_spec (i : Std.Usize) (h : i.val < 3) :
    cdt.nx i ⦃ j => j.val = (i.val + 1) % 3 ⦄ := by
  unfold cdt.nx
  step*

/-- `pv`: the corner before. -/
theorem pv_spec (i : Std.Usize) (h : i.val < 3) :
    cdt.pv i ⦃ j => j.val = (i.val + 2) % 3 ⦄ := by
  unfold cdt.pv
  step*

/-- `slot`: where x is among a triangle's three ids, if it is. -/
theorem slot_spec (a : Array Std.U32 3#usize) (x : Std.U32) :
    cdt.slot a x ⦃ r => ∀ k, r = some k → k.val < 3 ∧ a.val[k.val]! = x ⦄ := by
  unfold cdt.slot
  step*
  all_goals (intro k hk; cases hk; simp_all)

/-- `least` is the smaller of two finite doubles. -/
theorem least_spec (a b : F64) (ha : a.isFinite) (hb : b.isFinite) :
    ∃ c, cdt.least a b = ok c ∧ c.val = min a.val b.val ∧ c.isFinite := by
  unfold cdt.least
  by_cases h : IEEE754.FP.lt a b = true
  · have := (IEEE754.FP.lt_iff ha hb).mp h
    simp [F64.Ops.lt, h, min_eq_left this.le, ha]
  · have : ¬ a.val < b.val := fun h' => h ((IEEE754.FP.lt_iff ha hb).mpr h')
    simp [F64.Ops.lt, h, min_eq_right (not_lt.mp this), hb]

/-- `most` is the larger of two finite doubles. -/
theorem most_spec (a b : F64) (ha : a.isFinite) (hb : b.isFinite) :
    ∃ c, cdt.most a b = ok c ∧ c.val = max a.val b.val ∧ c.isFinite := by
  unfold cdt.most
  by_cases h : IEEE754.FP.lt b a = true
  · have := (IEEE754.FP.lt_iff hb ha).mp h
    simp [F64.Ops.gt, h, max_eq_left this.le, ha]
  · have : ¬ b.val < a.val := fun h' => h ((IEEE754.FP.lt_iff hb ha).mpr h')
    simp [F64.Ops.gt, h, max_eq_right (not_lt.mp this), hb]

/-- `same_chord`: l-r and a-b are one chord, either way round. -/
theorem same_chord_spec (l r a b : Std.U32) :
    cdt.same_chord l r a b = ok (decide ((l = a ∧ r = b) ∨ (l = b ∧ r = a))) := by
  unfold cdt.same_chord
  by_cases h1 : l = a <;> by_cases h2 : r = b <;> by_cases h3 : l = b <;> by_cases h4 : r = a <;>
    simp_all

end forge_mesh
