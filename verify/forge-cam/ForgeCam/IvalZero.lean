import Mathlib.Data.Real.Basic
import Mathlib.Tactic

/-!
# CAM's intervals: an exact zero operand (kernel/forge-num/src/ival.rs `Ival::add`, `Ival::mul`)

An `Ival` [lo, hi] encloses a real when lo ≤ x ≤ hi. Every operation must return an enclosure of
every result of its operands' enclosed reals (soundness). The general rules round outward one ulp
each side; where one operand is the exact point 0 the result is exact instead:
* `add`: x + 0 = x, so the other operand's interval itself encloses every sum;
* `mul`: x · 0 = 0, so the point [0, 0] encloses every product (the other operand any real in a
  finite interval: the code keeps the general rule for unbounded or undefined operands).
Both are sound, and tighter than the outward-rounded rule (which contains them).
-/

namespace ForgeCam.IvalZero

/-- An interval of reals. -/
structure Ival where
  lo : ℝ
  hi : ℝ

/-- x lies in the interval. -/
def Ival.mem (I : Ival) (x : ℝ) : Prop := I.lo ≤ x ∧ x ≤ I.hi

/-- The exact point 0. -/
def zero : Ival := ⟨0, 0⟩

theorem mem_zero {y : ℝ} (h : zero.mem y) : y = 0 :=
  le_antisymm h.2 h.1

/-- **The sum with an exact zero is the other operand**: every x + y with x ∈ I and y ∈ [0, 0]
lies in I (`Ival::add` returns I). -/
theorem add_zero_encl (I : Ival) {x y : ℝ} (hx : I.mem x) (hy : zero.mem y) : I.mem (x + y) := by
  rw [mem_zero hy, add_zero]; exact hx

/-- The same with the zero first. -/
theorem zero_add_encl (I : Ival) {x y : ℝ} (hx : zero.mem x) (hy : I.mem y) : I.mem (x + y) := by
  rw [mem_zero hx, zero_add]; exact hy

/-- **The product with an exact zero is exactly zero**: every x · y with x ∈ I (any interval of
reals) and y ∈ [0, 0] lies in [0, 0] (`Ival::mul` returns [0, 0]). -/
theorem mul_zero_encl (I : Ival) {x y : ℝ} (_hx : I.mem x) (hy : zero.mem y) :
    zero.mem (x * y) := by
  rw [mem_zero hy, mul_zero]; exact ⟨le_rfl, le_rfl⟩

/-- The same with the zero first. -/
theorem zero_mul_encl (I : Ival) {x y : ℝ} (hx : zero.mem x) (_hy : I.mem y) :
    zero.mem (x * y) := by
  rw [mem_zero hx, zero_mul]; exact ⟨le_rfl, le_rfl⟩

/-- Tighter: any interval that encloses some product (or sum) contains the exact result, so the
exact-zero result is inside every sound enclosure the general rule could give. -/
theorem exact_inside {J : Ival} {v : ℝ} (hv : J.mem v) (P : Ival) (hP : ∀ z, P.mem z → z = v) :
    ∀ z, P.mem z → J.mem z := by
  intro z hz; rw [hP z hz]; exact hv

end ForgeCam.IvalZero
