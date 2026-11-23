/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import AeneasF64.Ops

/-!
# Library functions on `f64`

Aeneas keeps calls into `core` opaque and lists each one, with the type it expects, in a crate's
`FunsExternal_Template.lean`. The ones on `f64` are defined here, once, for every translated
crate, as the IEEE 754 model's functions. Their names and types are the ones Aeneas generates.
Add a function here when a translated crate first needs it, and `tools/aeneas/translate.sh` will
check the template against the definitions.

Each is total and cannot fail, as in Rust.
-/

open Aeneas Aeneas.Std Result

/-- `f64::EPSILON` = 2^-52 -/
def core.f64.F64.EPSILON : Result F64 := ok (F64.Ops.ofBits 0x3CB0000000000000)
/-- `f64::MIN_POSITIVE` = 2^-1022 -/
def core.f64.F64.MIN_POSITIVE : Result F64 := ok (F64.Ops.ofBits 0x0010000000000000)
/-- `f64::MAX` -/
def core.f64.F64.MAX : Result F64 := ok (F64.Ops.ofBits 0x7FEFFFFFFFFFFFFF)
/-- `f64::INFINITY` -/
def core.f64.F64.INFINITY : Result F64 := ok (F64.Ops.ofBits 0x7FF0000000000000)
/-- `f64::NEG_INFINITY` -/
def core.f64.F64.NEG_INFINITY : Result F64 := ok (F64.Ops.ofBits 0xFFF0000000000000)
/-- `f64::NAN` -/
def core.f64.F64.NAN : Result F64 := ok IEEE754.FP.nan

/-- `x.abs()`: the sign bit cleared -/
def core.f64.F64.abs (x : F64) : Result F64 := ok (IEEE754.FP.abs x)
/-- `x.sqrt()`: correctly rounded (IEEE 754 requires it; `IEEE754.FP.sqrt_eq_round`) -/
def core.f64.F64.sqrt (x : F64) : Result F64 := ok (IEEE754.FP.sqrt x)
/-- The same, as Charon names it when it comes from `std` (a method called through `std`, as
`Vec2::length` does): one function. -/
def std.f64.F64.sqrt (x : F64) : Result F64 := ok (IEEE754.FP.sqrt x)
/-- `x.is_nan()` -/
def core.f64.F64.is_nan (x : F64) : Result Bool := ok x.isNaN
/-- `x.is_finite()` -/
def core.f64.F64.is_finite (x : F64) : Result Bool := ok x.isFinite
/-- `x.is_infinite()` -/
def core.f64.F64.is_infinite (x : F64) : Result Bool := ok x.isInf
/-- `f64::from_bits(b)` -/
def core.f64.F64.from_bits (b : Std.U64) : Result F64 := ok (F64.Ops.ofBits b.val)
/-- `x.clone()` -/
def F64.Insts.CoreCloneClone.clone (x : F64) : Result F64 := ok x
