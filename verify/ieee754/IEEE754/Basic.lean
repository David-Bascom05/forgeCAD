/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Mathlib.Algebra.Order.Floor.Ring
import Mathlib.Algebra.Order.Floor.Semifield
import Mathlib.Data.Int.Log
import Mathlib.Data.Rat.Floor
import Mathlib.Data.Nat.Sqrt

/-!
# IEEE 754 binary floating point: the values

A binary interchange format is given by its exponent width `ebits` and its precision `prec`
(significand bits, the hidden bit included): binary64 is `⟨11, 53⟩`.

A value (`FP f`) is a signed zero, a signed infinity, NaN, or a finite nonzero value
`(-1)^s · m · 2^e` in *canonical* form (`Canon`): `0 < m < 2^prec`, `qmin ≤ e ≤ qmax`, and either
`m ≥ 2^(prec-1)` (normal) or `e = qmin` (subnormal). Every finite nonzero real number the format
can represent has exactly one canonical form, so equality of `FP` values is equality of IEEE
datums (with one NaN: payloads are not modelled).
-/

namespace IEEE754

/-- A binary interchange format: exponent field width `ebits` and precision `prec`. -/
structure Format where
  ebits : ℕ
  prec : ℕ
  two_le_ebits : 2 ≤ ebits
  two_le_prec : 2 ≤ prec

namespace Format

variable (f : Format)

/-- The largest exponent of a normal number, `2^(ebits-1) - 1` (1023 for binary64). -/
def emax : ℤ := 2 ^ (f.ebits - 1) - 1
/-- The smallest exponent of a normal number, `1 - emax` (-1022 for binary64). -/
def emin : ℤ := 1 - f.emax
/-- The exponent of the least significand unit, `emin - prec + 1` (-1074 for binary64). -/
def qmin : ℤ := f.emin - f.prec + 1
/-- The exponent of the unit of the largest binade, `emax - prec + 1` (971 for binary64). -/
def qmax : ℤ := f.emax - f.prec + 1

/-- The exponent of the unit in the last place for a value whose binade is `[2^E, 2^(E+1))`. -/
def qexp (E : ℤ) : ℤ := max (E - f.prec + 1) f.qmin

/-- The width of the encoding: sign, exponent field, fraction field. -/
def width : ℕ := 1 + f.ebits + (f.prec - 1)

end Format

/-- IEEE 754 binary64 (double). -/
def binary64 : Format := ⟨11, 53, by decide, by decide⟩
/-- IEEE 754 binary32 (single). -/
def binary32 : Format := ⟨8, 24, by decide, by decide⟩
/-- IEEE 754 binary16 (half). -/
def binary16 : Format := ⟨5, 11, by decide, by decide⟩

/-- Canonical significand and exponent of a finite nonzero value. -/
def Canon (f : Format) (m : ℕ) (e : ℤ) : Prop :=
  0 < m ∧ m < 2 ^ f.prec ∧ f.qmin ≤ e ∧ e ≤ f.qmax ∧ (2 ^ (f.prec - 1) ≤ m ∨ e = f.qmin)

instance (f : Format) (m : ℕ) (e : ℤ) : Decidable (Canon f m e) := by
  unfold Canon; infer_instance

/-- An IEEE 754 datum of format `f`. NaN payloads and signs are not modelled. -/
inductive FP (f : Format) where
  /-- `+0` (`s = false`) or `-0` (`s = true`). -/
  | zero (s : Bool)
  /-- `+∞` or `-∞`. -/
  | inf (s : Bool)
  /-- Not a number. -/
  | nan
  /-- `(-1)^s · m · 2^e`, canonical. -/
  | fin (s : Bool) (m : ℕ) (e : ℤ) (h : Canon f m e)

namespace FP

variable {f : Format}

instance : DecidableEq (FP f) := fun x y =>
  match x, y with
  | zero a, zero b => if h : a = b then isTrue (h ▸ rfl) else isFalse (by intro h'; cases h'; exact h rfl)
  | inf a, inf b => if h : a = b then isTrue (h ▸ rfl) else isFalse (by intro h'; cases h'; exact h rfl)
  | nan, nan => isTrue rfl
  | fin s m e _, fin s' m' e' _ =>
    if h : s = s' ∧ m = m' ∧ e = e' then isTrue (by obtain ⟨rfl, rfl, rfl⟩ := h; rfl)
    else isFalse (by intro h'; cases h'; exact h ⟨rfl, rfl, rfl⟩)
  | zero _, inf _ | zero _, nan | zero _, fin .. | inf _, zero _ | inf _, nan | inf _, fin ..
  | nan, zero _ | nan, inf _ | nan, fin .. | fin .., zero _ | fin .., inf _ | fin .., nan =>
    isFalse (by intro h; cases h)

/-- Finite: a zero or a finite nonzero value. -/
def isFinite : FP f → Bool
  | zero _ | fin .. => true
  | _ => false

def isNaN : FP f → Bool
  | nan => true
  | _ => false

def isInf : FP f → Bool
  | inf _ => true
  | _ => false

def isZero : FP f → Bool
  | zero _ => true
  | _ => false

/-- The sign bit (`false` for NaN). -/
def sign : FP f → Bool
  | zero s | inf s | fin s .. => s
  | nan => false

/-- The sign `±1` as a number. -/
def sgn {K : Type*} [Ring K] (s : Bool) : K := if s then -1 else 1

/-- The exact rational value of a finite datum (`0` for infinities and NaN). -/
def val : FP f → ℚ
  | fin s m e _ => sgn s * ((m : ℚ) * 2 ^ e)
  | _ => 0

/-- Negation: flips the sign (IEEE `negate`). -/
def neg : FP f → FP f
  | zero s => zero !s
  | inf s => inf !s
  | nan => nan
  | fin s m e h => fin (!s) m e h

/-- Absolute value: clears the sign (IEEE `abs`). -/
def abs : FP f → FP f
  | zero _ => zero false
  | inf _ => inf false
  | nan => nan
  | fin _ m e h => fin false m e h

instance : Neg (FP f) := ⟨neg⟩

end FP

/-! ## Rounding to nearest, ties to even -/

section Round

variable {K : Type*} [Field K] [LinearOrder K] [IsStrictOrderedRing K] [FloorRing K]

/-- The nearest natural number to `y ≥ 0`, ties to even. -/
def rne (y : K) : ℕ :=
  if y < ⌊y⌋₊ + 1 / 2 then ⌊y⌋₊
  else if (⌊y⌋₊ : K) + 1 / 2 < y then ⌊y⌋₊ + 1
  else if ⌊y⌋₊ % 2 = 0 then ⌊y⌋₊ else ⌊y⌋₊ + 1

variable (f : Format)

/-- Turn an unbounded rounded magnitude `n · 2^e` into a datum: carry into the next binade when
`n = 2^prec`, overflow to infinity past the largest binade. -/
def finalize (s : Bool) (n : ℕ) (e : ℤ) : FP f :=
  if n = 0 then .zero s else
  let p : ℕ × ℤ := if n = 2 ^ f.prec then (2 ^ (f.prec - 1), e + 1) else (n, e)
  if f.qmax < p.2 then .inf s else
  if h : Canon f p.1 p.2 then .fin s p.1 p.2 h else .nan

/-- Round `r > 0` with unbounded exponent range: the significand and the quantum exponent. -/
def roundPos (r : K) : ℕ × ℤ :=
  let e := f.qexp (Int.log 2 r)
  (rne (r / 2 ^ e), e)

/-- Round to nearest, ties to even, onto format `f`. An exact zero becomes `zero zs` (the sign
an operation gives an exact zero result); otherwise the sign is that of `r`, also when the
magnitude underflows to zero. -/
def round (zs : Bool) (r : K) : FP f :=
  if r = 0 then .zero zs
  else if 0 < r then finalize f false (roundPos f r).1 (roundPos f r).2
  else finalize f true (roundPos f (-r)).1 (roundPos f (-r)).2

end Round

end IEEE754
