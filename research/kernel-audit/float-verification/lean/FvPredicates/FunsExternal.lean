/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import FvPredicates.Types

/-!
# The `f64` operations, defined by the IEEE 754 model

Aeneas translates every call of an `f64` operation to an opaque function, and its template
(`FunsExternal_Template.lean`) declares each one as an `axiom`. Here each one is instead a
definition, the model's operation. These definitions have no axioms. A translated function
computing with `f64` then has the exact IEEE 754 semantics of the model, which was checked bit
for bit against hardware (verify/ieee754/RESULTS.md). None of these operations panics in Rust,
and none fails here.

Rust's `<`, `<=`, `>`, `>=` and `==` on `f64` are IEEE 754's ordered comparisons. `!=` is the
negation of `==`, so it is true when an operand is NaN.
-/

open Aeneas Aeneas.Std Result


/-- `<f64 as Add>::add` -/
def F64.Insts.CoreOpsArithAddF64F64.add (a b : F64) : Result F64 := ok (IEEE754.FP.add a b)
/-- `<f64 as Sub>::sub` -/
def F64.Insts.CoreOpsArithSubF64F64.sub (a b : F64) : Result F64 := ok (IEEE754.FP.sub a b)
/-- `<f64 as Mul>::mul` -/
def F64.Insts.CoreOpsArithMulF64F64.mul (a b : F64) : Result F64 := ok (IEEE754.FP.mul a b)
/-- `<f64 as Neg>::neg` -/
def F64.Insts.CoreOpsArithNegF64.neg (a : F64) : Result F64 := ok (IEEE754.FP.neg a)
/-- `f64::abs` -/
def core.f64.F64.abs (a : F64) : Result F64 := ok (IEEE754.FP.abs a)
/-- `f64::from_bits` -/
def core.f64.F64.from_bits (b : Std.U64) : Result F64 := ok (IEEE754.FP.ofBits b.val)
/-- `<f64 as PartialEq>::eq` -/
def F64.Insts.CoreCmpPartialEqF64.eq (a b : F64) : Result Bool := ok (IEEE754.FP.eq a b)
/-- `<f64 as PartialEq>::ne` -/
def F64.Insts.CoreCmpPartialEqF64.ne (a b : F64) : Result Bool := ok (!IEEE754.FP.eq a b)
/-- `<f64 as PartialOrd>::lt` -/
def F64.Insts.CoreCmpPartialOrdF64.lt (a b : F64) : Result Bool := ok (IEEE754.FP.lt a b)
/-- `<f64 as PartialOrd>::gt` -/
def F64.Insts.CoreCmpPartialOrdF64.gt (a b : F64) : Result Bool := ok (IEEE754.FP.lt b a)
/-- `<f64 as PartialOrd>::ge` -/
def F64.Insts.CoreCmpPartialOrdF64.ge (a b : F64) : Result Bool := ok (IEEE754.FP.le b a)
/-- `<f64 as Clone>::clone` -/
def F64.Insts.CoreCloneClone.clone (a : F64) : Result F64 := ok a
