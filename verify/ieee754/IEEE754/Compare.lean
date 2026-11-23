/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Theorems

/-!
# Comparisons

On finite data the IEEE predicates are the order of the exact values (so `-0 == +0`); any NaN
operand makes every predicate false.
-/

namespace IEEE754

namespace FP

variable {f : Format}

private lemma fin_or {x : FP f} (hx : x.isFinite) : (∃ s, x = zero s) ∨ ∃ s m e h, x = fin s m e h := by
  cases x <;> simp_all [isFinite]

lemma cmp_finite {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) :
    cmp x y = some (compare x.val y.val) := by
  rcases fin_or hx with ⟨s, rfl⟩ | ⟨s, m, e, h, rfl⟩ <;>
  rcases fin_or hy with ⟨s', rfl⟩ | ⟨s', m', e', h', rfl⟩ <;> rfl

theorem lt_iff {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) : x.lt y = true ↔ x.val < y.val := by
  simp [FP.lt, cmp_finite hx hy, compare_lt_iff_lt]

theorem le_iff {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) : x.le y = true ↔ x.val ≤ y.val := by
  simp [FP.le, cmp_finite hx hy, compare_lt_iff_lt, le_iff_lt_or_eq]

theorem eq_iff {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) : x.eq y = true ↔ x.val = y.val := by
  simp [FP.eq, cmp_finite hx hy]

theorem nan_lt (y : FP f) : nan.lt y = false := by cases y <;> rfl
theorem nan_le (y : FP f) : nan.le y = false := by cases y <;> rfl
theorem nan_eq (y : FP f) : nan.eq y = false := by cases y <;> rfl
theorem eq_nan (x : FP f) : x.eq nan = false := by cases x <;> rfl

/-- `-0 == +0`. -/
theorem zero_eq_zero (s t : Bool) : (zero s : FP f).eq (zero t) = true := by
  simp [FP.eq, cmp]

end FP

end IEEE754
