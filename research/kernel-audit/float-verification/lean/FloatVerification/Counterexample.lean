/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import FloatVerification.Orient2d

/-!
# Without the precondition, `orient2d`'s fast path gives a wrong sign

These points have coordinates near `1e-160`:

    a = (5.394455326111622e-162, 1.6245062719943995e-164)
    b = (1.3762009331530288e-157, 4.1443425003634566e-160)
    c = (-4.608837897596252e-178, 0)

The exact determinant is positive. The two rounded products are subnormal and straddle a rounding
midpoint the wrong way round, so `det = -2^-1074`, and `bound` underflows to 0: the test
passes and `orient2d` answers `-5e-324`. The product `D₁·D₂` is neither exact nor normal: the
kernel's precondition (`Relative`, its debug check `check_filter`) fails here, and the kernel's
test `orient2d_underflow_counterexample_is_caught` checks that the debug check catches it.

Checked by evaluation (`native_decide`, which trusts the compiler); nothing depends on it.
-/

namespace FloatVerification

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP
open fv_predicates

def cexA : Point2 := ⟨FP.ofBits 2194214354537152162, FP.ofBits 2156643164839958569⟩
def cexB : Point2 := ⟨FP.ofBits 2260309221917857993, FP.ofBits 2222331156790808471⟩
def cexC : Point2 := ⟨FP.ofBits 11177338095568441929, FP.ofBits 0⟩

/-- The test passes with `det < 0` though the exact determinant is positive, and the product
`D₁·D₂` breaks the precondition. -/
theorem orient2d_cex :
    orientFast cexA cexB cexC = true ∧ (oDet cexA cexB cexC).val = -(2 ^ (-1074 : ℤ)) ∧
      0 < orientExact cexA cexB cexC ∧
      0 < |(sub cexA.x cexC.x).val * (sub cexB.y cexC.y).val| ∧
      |(sub cexA.x cexC.x).val * (sub cexB.y cexC.y).val| < 2 ^ (-1022 : ℤ) ∧
      ¬ Relative (sub cexA.x cexC.x) (sub cexB.y cexC.y) := by
  native_decide

end FloatVerification
