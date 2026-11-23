/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import IEEE754

/-!
# The float bridge: Rust's `f64` is `verify/ieee754`'s binary64

Aeneas, patched by `tools/aeneas/patches/aeneas-f64.patch`, translates the `f64` operations of
the Rust it is given to calls of the functions below:

| Rust | Lean (generated) | defined as |
|---|---|---|
| `a + b`, `a - b`, `a * b`, `a / b` | `F64.Ops.add a b` … `F64.Ops.div a b` | `IEEE754.FP.add` … (round to nearest, ties to even) |
| `-a` | `F64.Ops.neg a` | `IEEE754.FP.neg` (the sign bit flipped) |
| `a == b`, `a != b` | `F64.Ops.eq a b`, `F64.Ops.ne a b` | IEEE 754 equality (`NaN ≠ NaN`, `-0 = +0`), and its negation |
| `a < b`, `a <= b`, `a > b`, `a >= b` | `F64.Ops.lt` … `F64.Ops.ge` | IEEE 754's ordered comparisons (false on NaN) |
| a literal `1.5` | `F64.Ops.ofBits 4609434218613702656` | the encoding rustc computed (Charon's `float_bits`) |
| `n as f64` | `F64.Ops.ofIScalar n`, `F64.Ops.ofUScalar n` | `IEEE754.FP.ofInt`, rounded to nearest |
| `x as i64` … | `F64.Ops.toIScalar .I64 x` … | `IEEE754.FP.toIntSat`: truncation, saturating, NaN to 0 |

They live in `F64.Ops`, not `F64`, so that dot notation on a float (`x.neg`, `x.abs`, `x.val`)
still finds the model's `IEEE754.FP` functions.

None of them fails: an `f64` operation cannot panic in Rust, so the generated code calls them
outside the `Result` monad. Calls of library functions on `f64` (`abs`, `sqrt`, `to_bits`, …)
are opaque to Aeneas; `AeneasF64/Std.lean` defines each one used by a translated crate.

These are definitions, not axioms. What they assume about the machine is one stated trusted base,
in `verify/aeneas/README.md` ("the float trusted base"): the compiled code performs these
binary64 operations, rounding to nearest, with no contraction and no reassociation.
-/

open Aeneas Aeneas.Std

/-- Rust's `f64`: the IEEE 754 model's binary64 datum. -/
abbrev F64 := IEEE754.F64

namespace F64.Ops



/-- `a + b` -/
def add (a b : F64) : F64 := IEEE754.FP.add a b
/-- `a - b` -/
def sub (a b : F64) : F64 := IEEE754.FP.sub a b
/-- `a * b` -/
def mul (a b : F64) : F64 := IEEE754.FP.mul a b
/-- `a / b` -/
def div (a b : F64) : F64 := IEEE754.FP.div a b
/-- `-a` -/
def neg (a : F64) : F64 := IEEE754.FP.neg a

/-- `a == b` -/
def eq (a b : F64) : Bool := IEEE754.FP.eq a b
/-- `a != b`: true when either is NaN -/
def ne (a b : F64) : Bool := !IEEE754.FP.eq a b
/-- `a < b` -/
def lt (a b : F64) : Bool := IEEE754.FP.lt a b
/-- `a <= b` -/
def le (a b : F64) : Bool := IEEE754.FP.le a b
/-- `a > b` -/
def gt (a b : F64) : Bool := IEEE754.FP.lt b a
/-- `a >= b` -/
def ge (a b : F64) : Bool := IEEE754.FP.le b a

/-- A literal, by its encoding. -/
def ofBits (n : Nat) : F64 := IEEE754.FP.ofBits n

/-- `n as f64` for a signed `n`. -/
def ofIScalar {ty : IScalarTy} (x : IScalar ty) : F64 := IEEE754.FP.ofInt x.val
/-- `n as f64` for an unsigned `n`. -/
def ofUScalar {ty : UScalarTy} (x : UScalar ty) : F64 := IEEE754.FP.ofInt (x.val : ℤ)

lemma toIntSat_mem {lo hi : ℤ} (hlo : lo ≤ 0) (hhi : 0 ≤ hi) (x : F64) :
    lo ≤ IEEE754.FP.toIntSat lo hi x ∧ IEEE754.FP.toIntSat lo hi x ≤ hi := by
  unfold IEEE754.FP.toIntSat
  split
  · exact ⟨hlo, hhi⟩
  · split <;> omega
  · constructor
    · exact le_max_left _ _
    · exact max_le (le_trans hlo hhi) (min_le_left _ _)

/-- `x as i64` (and the other signed types): truncation toward zero, saturating at the type's
bounds, NaN to 0. -/
def toIScalar (ty : IScalarTy) (x : F64) : IScalar ty :=
  IScalar.ofIntCore (IEEE754.FP.toIntSat (-2 ^ (ty.numBits - 1)) (2 ^ (ty.numBits - 1) - 1) x) (by
    have hp : (1 : ℤ) ≤ 2 ^ (ty.numBits - 1) := one_le_pow₀ (by norm_num)
    have h := toIntSat_mem (lo := -2 ^ (ty.numBits - 1)) (hi := 2 ^ (ty.numBits - 1) - 1)
      (by omega) (by omega) x
    omega)

/-- `x as u64` (and the other unsigned types): truncation toward zero, saturating at `0` and the
type's maximum, NaN to 0. -/
def toUScalar (ty : UScalarTy) (x : F64) : UScalar ty :=
  UScalar.ofNatCore (IEEE754.FP.toIntSat 0 (2 ^ ty.numBits - 1) x).toNat (by
    have hp : (1 : ℤ) ≤ 2 ^ ty.numBits := one_le_pow₀ (by norm_num)
    have h := toIntSat_mem (lo := 0) (hi := 2 ^ ty.numBits - 1) le_rfl (by omega) x
    have hc : ((2 ^ ty.numBits : ℕ) : ℤ) = (2 : ℤ) ^ ty.numBits := by push_cast; rfl
    omega)

end F64.Ops
