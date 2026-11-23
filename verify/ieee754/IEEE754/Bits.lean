/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Basic

/-!
# The bit-level encoding

`toBits` and `ofBits` between `FP f` and the natural numbers below `2^f.width`: sign bit, biased
exponent field, fraction field. NaN encodes as the canonical quiet NaN (exponent all ones,
fraction `100…0`); every NaN encoding decodes to the one `nan`.
-/

namespace IEEE754

namespace FP

variable {f : Format}

/-- The sign bit's weight. -/
def signBit (f : Format) : ℕ := 2 ^ (f.ebits + (f.prec - 1))

/-- The all-ones exponent field, in place. -/
def expOnes (f : Format) : ℕ := (2 ^ f.ebits - 1) * 2 ^ (f.prec - 1)

/-- The encoding. -/
def toBits : FP f → ℕ
  | zero s => if s then signBit f else 0
  | inf s => (if s then signBit f else 0) + expOnes f
  | nan => expOnes f + 2 ^ (f.prec - 2)
  | fin s m e _ =>
    (if s then signBit f else 0) +
      (if m < 2 ^ (f.prec - 1) then m
       else (e - f.qmin + 1).toNat * 2 ^ (f.prec - 1) + (m - 2 ^ (f.prec - 1)))

/-- The decoding (bits above the width are ignored). -/
def ofBits (b : ℕ) : FP f :=
  let frac := b % 2 ^ (f.prec - 1)
  let bexp := b / 2 ^ (f.prec - 1) % 2 ^ f.ebits
  let s := b / signBit f % 2 == 1
  if bexp = 0 then
    if frac = 0 then zero s else if h : Canon f frac f.qmin then fin s frac f.qmin h else nan
  else if bexp = 2 ^ f.ebits - 1 then (if frac = 0 then inf s else nan)
  else
    if h : Canon f (2 ^ (f.prec - 1) + frac) (bexp + f.qmin - 1) then
      fin s (2 ^ (f.prec - 1) + frac) (bexp + f.qmin - 1) h
    else nan

end FP

end IEEE754
