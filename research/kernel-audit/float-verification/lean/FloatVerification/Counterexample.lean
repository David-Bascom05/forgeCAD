/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import FloatVerification.Orient2d

/-!
# Without `SLACK`, `orient2d`'s fast path gave a wrong sign when the products underflowed

These points have coordinates near `1e-160`:

    a = (5.394455326111622e-162, 1.6245062719943995e-164)
    b = (1.3762009331530288e-157, 4.1443425003634566e-160)
    c = (-4.608837897596252e-178, 0)

The exact determinant is positive. The two rounded products are subnormal, and they straddle a
rounding midpoint the wrong way round: `left` rounds down to `k·2^-1074` and `right` up to
`(k+1)·2^-1074`, so `det = -2^-1074`. `CCW_BOUND · detsum` underflows to `0`. The kernel's test
before the fix (`bound = CCW_BOUND · detsum`) passed, and `orient2d` answered `-5e-324`.

`SLACK` makes the bound `16 · 2^-1074`, so the test fails and the exact path decides.
`orient2d_fast` proves the test with `SLACK` sound for every finite input.

These facts are checked by evaluation (`native_decide`, which trusts the compiler: the axiom
`Lean.ofReduceBool`), and nothing else depends on them.
-/

namespace FloatVerification

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP
open fv_predicates

def cexA : Point2 := ⟨FP.ofBits 2194214354537152162, FP.ofBits 2156643164839958569⟩
def cexB : Point2 := ⟨FP.ofBits 2260309221917857993, FP.ofBits 2222331156790808471⟩
def cexC : Point2 := ⟨FP.ofBits 11177338095568441929, FP.ofBits 0⟩

/-- The test before the fix: `det > bound || -det > bound` with `bound = CCW_BOUND · detsum`. -/
def orientFastOld (a b c : Point2) : Bool :=
  FP.lt (oBound₀ a b c) (oDet a b c) || FP.lt (oBound₀ a b c) (oDet a b c).neg

/-- The old test passed with `det < 0` though the exact determinant is positive, and the product
`D₁·D₂` underflows. The fixed test does not pass. -/
theorem orient2d_cex :
    orientFastOld cexA cexB cexC = true ∧ (oDet cexA cexB cexC).val = -(2 ^ (-1074 : ℤ)) ∧
      0 < orientExact cexA cexB cexC ∧
      0 < |(sub cexA.x cexC.x).val * (sub cexB.y cexC.y).val| ∧
      |(sub cexA.x cexC.x).val * (sub cexB.y cexC.y).val| < 2 ^ (-1022 : ℤ) ∧
      orientFast cexA cexB cexC = false := by
  native_decide

end FloatVerification
