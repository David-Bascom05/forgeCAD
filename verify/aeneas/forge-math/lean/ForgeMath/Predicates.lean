/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMath.Refine
import FloatVerification.CrossSign
import FloatVerification.Orient2d

/-!
# `cross_sign` and `orient2d`: the ieee session's theorems, for the shipped code

research/kernel-audit/float-verification proved the fast paths of `cross_sign` and `orient2d`
correct. It proved them for a copy of the code respelled for Aeneas (`fv_predicates`), because
Aeneas then had no floating point. With the float bridge, `kernel/forge-math` translates as it
is. Here the same theorems are stated for the real functions:
1. `*_of_fast`: when the model's fast test holds, the real function returns exactly the model's
   value (by unfolding the translation, and evaluating the bound constants, which the real code
   computes as `(2 + 8·EPS)·EPS` and so on, with `decide +kernel`);
2. the copy returns the same value (the ieee session's `*_of_fast`);
3. so their theorem's conclusion holds for the real function's result.

The theorems are the fast paths only: the slow paths go through `orient2d_adapt`, whose expansion
arithmetic (`sum_into`, `scale_into`, `estimate`) is opaque in this translation. See
verify/aeneas/forge-math/README.md.
-/

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP
open forge_math

namespace ForgeMath

/-- `CROSS_BOUND = (2 + 8·EPS)·EPS`, as the real code computes it, is the literal the copy used. -/
lemma CROSS_BOUND_eq : predicates.CROSS_BOUND = ok (FP.ofBits 4372995238176751618) := by
  simp only [predicates.CROSS_BOUND, predicates.EPS, core.f64.F64.EPSILON, lift, bind_ok,
    F64.Ops.mul_def, F64.Ops.add_def]
  congr 1; decide +kernel

/-- `CCW_BOUND = (3 + 16·EPS)·EPS` -/
lemma CCW_BOUND_eq : predicates.CCW_BOUND = ok FloatVerification.ccw := by
  simp only [predicates.CCW_BOUND, predicates.EPS, core.f64.F64.EPSILON, lift, bind_ok,
    F64.Ops.mul_def, F64.Ops.add_def, FloatVerification.ccw]
  congr 1; decide +kernel

/-- The real `cross_sign` takes the fast path exactly when the model's `crossFast` holds, and
then returns `a ⊗ b ⊖ c ⊗ d`. -/
lemma cross_sign_of_fast {a b c d : F64} (h : FloatVerification.crossFast a b c d = true) :
    predicates.cross_sign a b c d = ok (sub (mul a b) (mul c d)) := by
  simp only [FloatVerification.crossFast] at h
  simp only [predicates.cross_sign, CROSS_BOUND_eq, lift, bind_ok, core.f64.F64.abs_eq,
    F64.Ops.mul_def, F64.Ops.sub_def, F64.Ops.add_def, F64.Ops.gt_def]
  simp [h]

/-- **`cross_sign`'s fast path is right, for every finite input** (the ieee session's
`cross_sign_fast`, for `kernel/forge-math`'s `cross_sign` itself): if the fast test passes, the
result is finite, nonzero and has the sign of the exact `a·b − c·d`. -/
theorem cross_sign_fast (a b c d : F64) (ha : a.isFinite) (hb : b.isFinite) (hc : c.isFinite)
    (hd : d.isFinite) (h : FloatVerification.crossFast a b c d = true) :
    ∃ v, predicates.cross_sign a b c d = ok v ∧ v.isFinite ∧ v.val ≠ 0 ∧
      (0 < v.val ↔ 0 < a.val * b.val - c.val * d.val) ∧
      (v.val < 0 ↔ a.val * b.val - c.val * d.val < 0) := by
  obtain ⟨v, hv, P⟩ := FloatVerification.cross_sign_fast a b c d ha hb hc hd h
  rw [FloatVerification.cross_sign_of_fast h, Result.ok.injEq] at hv
  subst hv
  exact ⟨_, cross_sign_of_fast h, P⟩

/-- A kernel point as the copy's point (the same two fields). -/
def fv (p : vec.Point2) : fv_predicates.Point2 := ⟨p.x, p.y⟩

/-- The real `orient2d` takes the fast path exactly when the model's `orientFast` holds, and
then returns the rounded determinant. -/
lemma orient2d_of_fast {a b c : vec.Point2}
    (h : FloatVerification.orientFast (fv a) (fv b) (fv c) = true) :
    predicates.orient2d a b c = ok (FloatVerification.oDet (fv a) (fv b) (fv c)) := by
  simp only [FloatVerification.orientFast, FloatVerification.oDet, FloatVerification.oBound,
    FloatVerification.oSum, FloatVerification.oLeft, FloatVerification.oRight, fv,
    Bool.or_eq_true] at h
  simp only [predicates.orient2d, CCW_BOUND_eq, lift, bind_ok, core.f64.F64.abs_eq,
    F64.Ops.mul_def, F64.Ops.sub_def, F64.Ops.add_def, F64.Ops.gt_def, F64.Ops.neg_def,
    FloatVerification.oDet, FloatVerification.oLeft, FloatVerification.oRight, fv]
  rcases h with h | h
  · simp [h]
  · split <;> simp_all

/-- **`orient2d`'s fast path gives the exact sign, under the kernel's precondition** (the ieee
session's `orient2d_fast`, for `kernel/forge-math`'s `orient2d` itself). For finite coordinates
whose three filter products are each exact or normal (`Relative`; `check_filter` in debug
builds), if the fast test passes, the result is finite, nonzero and has the sign of the exact
determinant of the input doubles. -/
theorem orient2d_fast (a b c : vec.Point2)
    (hax : a.x.isFinite) (hay : a.y.isFinite) (hbx : b.x.isFinite) (hby : b.y.isFinite)
    (hcx : c.x.isFinite) (hcy : c.y.isFinite)
    (hP : FloatVerification.Relative (sub a.x c.x) (sub b.y c.y))
    (hQ : FloatVerification.Relative (sub a.y c.y) (sub b.x c.x))
    (hB : FloatVerification.Relative FloatVerification.ccw (FloatVerification.oSum (fv a) (fv b) (fv c)))
    (h : FloatVerification.orientFast (fv a) (fv b) (fv c) = true) :
    ∃ v, predicates.orient2d a b c = ok v ∧ v.isFinite ∧ v.val ≠ 0 ∧
      (0 < v.val ↔ 0 < FloatVerification.orientExact (fv a) (fv b) (fv c)) ∧
      (v.val < 0 ↔ FloatVerification.orientExact (fv a) (fv b) (fv c) < 0) := by
  obtain ⟨v, hv, P⟩ := FloatVerification.orient2d_fast (fv a) (fv b) (fv c)
    hax hay hbx hby hcx hcy hP hQ hB h
  rw [FloatVerification.orient2d_of_fast h, Result.ok.injEq] at hv
  subst hv
  exact ⟨_, orient2d_of_fast h, P⟩

end ForgeMath
