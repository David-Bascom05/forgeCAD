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

/-! ## `core::f64::consts` (the encodings rustc gives the constants) -/

def core.f64.consts.PI : Result F64 := ok (F64.Ops.ofBits 0x400921FB54442D18)
def core.f64.consts.FRAC_PI_2 : Result F64 := ok (F64.Ops.ofBits 0x3FF921FB54442D18)
def core.f64.consts.FRAC_PI_4 : Result F64 := ok (F64.Ops.ofBits 0x3FE921FB54442D18)
def core.f64.consts.TAU : Result F64 := ok (F64.Ops.ofBits 0x401921FB54442D18)
def core.f64.consts.SQRT_2 : Result F64 := ok (F64.Ops.ofBits 0x3FF6A09E667F3BCD)
def core.f64.consts.LN_2 : Result F64 := ok (F64.Ops.ofBits 0x3FE62E42FEFA39EF)
def core.f64.consts.E : Result F64 := ok (F64.Ops.ofBits 0x4005BF0A8B145769)

/-- `x.is_sign_negative()`: the sign bit. For a NaN it is outside the float trusted base (Rust
does not fix a NaN's sign; the model has one NaN). -/
def core.f64.F64.is_sign_negative (x : F64) : Result Bool := ok x.sign

/-- `x.clamp(min, max)`, std's body: it panics unless `min <= max` (so a NaN bound panics), then
`if x < min { x = min }; if x > max { x = max }`. -/
def core.f64.F64.clamp (x lo hi : F64) : Result F64 :=
  if IEEE754.FP.le lo hi then
    let x := if IEEE754.FP.lt x lo then lo else x
    ok (if IEEE754.FP.lt hi x then hi else x)
  else fail .panic

/-- IEEE 754's roundToIntegralTowardNegative, `x.floor()`. An integer (zeros, infinities and NaN
included) is its own floor. Otherwise `⌊x⌋` is an integer below `2^52` in magnitude, so exactly
representable, and an exact zero result is `+0` (only `x ∈ (0, 1)` gives it). Not part of
verify/ieee754/check's hardware comparison: the trusted base T is what covers it. -/
def F64.Ops.floor (x : F64) : F64 :=
  if x.isFinite && x.val.den != 1 then IEEE754.FP.ofInt ⌊x.val⌋ else x
/-- `x.floor()`, as Charon names it through `std` -/
def std.f64.F64.floor (x : F64) : Result F64 := ok (F64.Ops.floor x)
def core.f64.F64.floor (x : F64) : Result F64 := ok (F64.Ops.floor x)
