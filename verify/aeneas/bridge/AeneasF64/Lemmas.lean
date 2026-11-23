/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import AeneasF64.Std

/-!
# Unfolding the bridge

`simp` lemmas that turn a translated function's float steps back into the model's operations, so
a proof about generated code can use `verify/ieee754`'s theorems directly.
-/

open Aeneas Aeneas.Std Result

namespace F64.Ops

@[simp] lemma add_def (a b : F64) : add a b = IEEE754.FP.add a b := rfl
@[simp] lemma sub_def (a b : F64) : sub a b = IEEE754.FP.sub a b := rfl
@[simp] lemma mul_def (a b : F64) : mul a b = IEEE754.FP.mul a b := rfl
@[simp] lemma div_def (a b : F64) : div a b = IEEE754.FP.div a b := rfl
@[simp] lemma neg_def (a : F64) : neg a = IEEE754.FP.neg a := rfl
@[simp] lemma eq_def' (a b : F64) : eq a b = IEEE754.FP.eq a b := rfl
@[simp] lemma ne_def' (a b : F64) : ne a b = !IEEE754.FP.eq a b := rfl
@[simp] lemma lt_def (a b : F64) : lt a b = IEEE754.FP.lt a b := rfl
@[simp] lemma le_def (a b : F64) : le a b = IEEE754.FP.le a b := rfl
@[simp] lemma gt_def (a b : F64) : gt a b = IEEE754.FP.lt b a := rfl
@[simp] lemma ge_def (a b : F64) : ge a b = IEEE754.FP.le b a := rfl

end F64.Ops

@[simp] lemma core.f64.F64.abs_eq (x : F64) : core.f64.F64.abs x = ok (IEEE754.FP.abs x) := rfl
@[simp] lemma core.f64.F64.is_nan_eq (x : F64) : core.f64.F64.is_nan x = ok x.isNaN := rfl
@[simp] lemma core.f64.F64.EPSILON_eq :
    core.f64.F64.EPSILON = ok (F64.Ops.ofBits 0x3CB0000000000000) := rfl
