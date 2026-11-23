import ForgeWeldRs.Funs

/-!
# A straight piece's moments, as the Rust computes them

`section::line_moments` (kernel/forge-weld/src/section.rs) is, operation for operation, the
binary64 evaluation of research/weld-lean's Green's-form terms (ForgeWeld/Section.lean): the area
`(px qy - qx py) / 2`, `lineMx = dy (px² + px qx + qx²) / 6`, `lineMy = -dx (py² + py qy + qy²) / 6`,
in that order of operations. So the moments the kernel sums are Green's form on every straight
piece, never the fan's (`fan_ne_line_on_an_edge`: mixing the two is wrong).
-/

open Aeneas Aeneas.Std Result forge_weld IEEE754

namespace ForgeWeldRs

local notation "two" => F64.Ops.ofBits 4611686018427387904
local notation "six" => F64.Ops.ofBits 4618441417868443648

theorem two_eq : two = FP.fin false (2 ^ 52) (-51) (by decide) := by
  simp only [F64.Ops.ofBits]; decide

theorem six_eq : six = FP.fin false (3 * 2 ^ 51) (-50) (by decide) := by
  simp only [F64.Ops.ofBits]; decide

/-- The literals are 2 and 6, exactly. -/
theorem two_val : (two).isFinite ∧ (two).val = 2 := by
  rw [two_eq]; refine ⟨rfl, ?_⟩; simp only [FP.val, FP.sgn]; norm_num

theorem six_val : (six).isFinite ∧ (six).val = 6 := by
  rw [six_eq]; refine ⟨rfl, ?_⟩; simp only [FP.val, FP.sgn]; norm_num

/-- The area term, in binary64: `(px qy ⊖ qx py) ⊘ 2`. -/
def areaF (p q : forge_math.vec.Point2) : _root_.F64 :=
  FP.div (FP.sub (FP.mul p.x q.y) (FP.mul q.x p.y)) two

/-- `lineMx` in binary64: `dy ⊗ (px² ⊕ px qx ⊕ qx²) ⊘ 6`. -/
def mxF (p q : forge_math.vec.Point2) : _root_.F64 :=
  FP.div (FP.mul (FP.sub q.y p.y)
    (FP.add (FP.add (FP.mul p.x p.x) (FP.mul p.x q.x)) (FP.mul q.x q.x))) six

/-- `lineMy` in binary64: `(⊖dx) ⊗ (py² ⊕ py qy ⊕ qy²) ⊘ 6`. -/
def myF (p q : forge_math.vec.Point2) : _root_.F64 :=
  FP.div (FP.mul (FP.neg (FP.sub q.x p.x))
    (FP.add (FP.add (FP.mul p.y p.y) (FP.mul p.y q.y)) (FP.mul q.y q.y))) six

theorem line_moments_eq (p q : forge_math.vec.Point2) :
    section.line_moments p q = ok (areaF p q, mxF p q, myF p q) := by
  simp [section.line_moments, lift, areaF, mxF, myF]

end ForgeWeldRs
