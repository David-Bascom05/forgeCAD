/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import FloatVerification.Orient2d

/-!
# `orient2d`'s fast path gives a wrong sign when the products underflow

These points have coordinates near `1e-160`:

    a = (5.394455326111622e-162, 1.6245062719943995e-164)
    b = (1.3762009331530288e-157, 4.1443425003634566e-160)
    c = (-4.608837897596252e-178, 0)

The exact determinant is positive. The two rounded products are subnormal, and they straddle
a rounding midpoint in the wrong order: `left` rounds down to `k·2^-1074` and `right` rounds up
to `(k+1)·2^-1074`. So `det = -2^-1074`. Meanwhile `bound = CCW_BOUND · detsum` underflows to
`0`, the test `-det > bound` passes, and `orient2d` answers `-5e-324`.
`predicates/src/lib.rs` checks this against the kernel's own `orient2d`
(`orient2d_underflow_counterexample`).

The witness is checked by evaluation (`native_decide`, which trusts the compiler: the axiom
`Lean.ofReduceBool`). It shows a hypothesis of `orient2d_fast` cannot be dropped, and nothing
else depends on it. The no-underflow hypothesis `hP` fails here.
-/

namespace FloatVerification

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP
open fv_predicates

def cexA : Point2 := ⟨FP.ofBits 2194214354537152162, FP.ofBits 2156643164839958569⟩
def cexB : Point2 := ⟨FP.ofBits 2260309221917857993, FP.ofBits 2222331156790808471⟩
def cexC : Point2 := ⟨FP.ofBits 11177338095568441929, FP.ofBits 0⟩

/-- The test passes, `det` is negative, and the exact determinant is positive. The product
`D₁·D₂` is nonzero and below `2^-1022` (it underflows). -/
theorem orient2d_cex_facts :
    orientFast cexA cexB cexC = true ∧ (oDet cexA cexB cexC).val = -(2 ^ (-1074 : ℤ)) ∧
      0 < orientExact cexA cexB cexC ∧
      0 < |(sub cexA.x cexC.x).val * (sub cexB.y cexC.y).val| ∧
      |(sub cexA.x cexC.x).val * (sub cexB.y cexC.y).val| < 2 ^ (-1022 : ℤ) := by
  native_decide

/-- **The fast path's sign is wrong here**: `orient2d` returns a negative value though the exact
determinant is positive. -/
theorem orient2d_wrong_sign :
    ∃ v, orient2d cexA cexB cexC = ok v ∧ v.val < 0 ∧ 0 < orientExact cexA cexB cexC := by
  obtain ⟨h, hv, hex, _⟩ := orient2d_cex_facts
  refine ⟨_, orient2d_of_fast h, ?_, hex⟩
  rw [hv]; norm_num

end FloatVerification
