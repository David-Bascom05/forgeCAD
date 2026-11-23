/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Basic

/-!
# The operations

Each arithmetic operation handles NaN, infinities and the sign of zero as IEEE 754 prescribes,
and on finite operands is `round` of the exact rational result (`add_eq_round` and the rest in
`IEEE754.Theorems`). Square root is computed with an integer square root (`sqrtPos`) and is proven
equal to `round` of the real square root (`sqrt_eq_round`).
-/

namespace IEEE754

namespace FP

variable {f : Format}

/-- Addition. -/
def add : FP f → FP f → FP f
  | nan, _ | _, nan => nan
  | inf a, inf b => if a = b then inf a else nan
  | inf a, _ => inf a
  | _, inf b => inf b
  | x, y => round f (x.sign && y.sign) (x.val + y.val)

/-- Subtraction, `x + (-y)`. -/
def sub (x y : FP f) : FP f := add x (neg y)

/-- Multiplication. -/
def mul : FP f → FP f → FP f
  | nan, _ | _, nan => nan
  | inf a, y => if y.isZero then nan else inf (a ^^ y.sign)
  | x, inf b => if x.isZero then nan else inf (x.sign ^^ b)
  | x, y => round f (x.sign ^^ y.sign) (x.val * y.val)

/-- Division. -/
def div : FP f → FP f → FP f
  | nan, _ | _, nan => nan
  | inf _, inf _ => nan
  | inf a, y => inf (a ^^ y.sign)
  | x, inf b => zero (x.sign ^^ b)
  | x, y =>
    if y.isZero then (if x.isZero then nan else inf (x.sign ^^ y.sign))
    else round f (x.sign ^^ y.sign) (x.val / y.val)

/-- Fused multiply-add `x * y + z` with a single rounding. -/
def fma (x y z : FP f) : FP f :=
  if x.isNaN || y.isNaN || z.isNaN then nan
  else if (x.isInf && y.isZero) || (x.isZero && y.isInf) then nan
  else if x.isInf || y.isInf then
    if z.isInf && z.sign != (x.sign ^^ y.sign) then nan else inf (x.sign ^^ y.sign)
  else if z.isInf then z
  else round f ((x.sign ^^ y.sign) && z.sign) (x.val * y.val + z.val)

instance : Add (FP f) := ⟨add⟩
instance : Sub (FP f) := ⟨sub⟩
instance : Mul (FP f) := ⟨mul⟩
instance : Div (FP f) := ⟨div⟩

end FP

variable (f : Format)

/-- `roundPos f (√q)` for rational `q > 0`, computed exactly: the binade of `√q` is half that of
`q`, and the scaled root `√N` is compared with `k + 1/2` by squaring. -/
def sqrtPos (q : ℚ) : ℕ × ℤ :=
  let e := f.qexp (Int.log 2 q / 2)
  let N : ℚ := q / 2 ^ (2 * e)
  let k := Nat.sqrt ⌊N⌋₊
  let t : ℚ := ((k : ℚ) + 1 / 2) ^ 2
  (if N < t then k else if t < N then k + 1 else if k % 2 = 0 then k else k + 1, e)

namespace FP

variable {f : Format}

/-- Square root. -/
def sqrt : FP f → FP f
  | zero s => zero s
  | inf false => inf false
  | inf true => nan
  | nan => nan
  | fin true .. => nan
  | fin false m e _ => finalize f false (sqrtPos f ((m : ℚ) * 2 ^ e)).1 (sqrtPos f ((m : ℚ) * 2 ^ e)).2

/-! ## Comparisons -/

/-- IEEE 754 comparison: `none` when unordered (a NaN operand); `-0` and `+0` are equal. -/
def cmp : FP f → FP f → Option Ordering
  | nan, _ | _, nan => none
  | inf a, inf b => some (if a = b then .eq else if a then .lt else .gt)
  | inf a, _ => some (if a then .lt else .gt)
  | _, inf b => some (if b then .gt else .lt)
  | x, y => some (compare x.val y.val)

/-- `x == y` (IEEE `compareQuietEqual`). -/
def eq (x y : FP f) : Bool := cmp x y == some .eq
/-- `x < y`. -/
def lt (x y : FP f) : Bool := cmp x y == some .lt
/-- `x ≤ y`. -/
def le (x y : FP f) : Bool := cmp x y == some .lt || cmp x y == some .eq

/-! ## Conversions -/

/-- Conversion from an integer, rounded to nearest (an exact zero is `+0`). -/
def ofInt (n : ℤ) : FP f := round f false (n : ℚ)

/-- Rounding of a rational toward zero. -/
def truncQ (q : ℚ) : ℤ := if 0 ≤ q then ⌊q⌋ else ⌈q⌉

/-- Conversion to an integer by truncation toward zero; `none` for infinities and NaN. -/
def toInt? (x : FP f) : Option ℤ := if x.isFinite then some (truncQ x.val) else none

/-- Truncating conversion that saturates at `[lo, hi]` and sends NaN to `0`: Rust's `as` cast
(for `i64`, `lo = -2^63` and `hi = 2^63 - 1`). -/
def toIntSat (lo hi : ℤ) : FP f → ℤ
  | nan => 0
  | inf s => if s then lo else hi
  | x => max lo (min hi (truncQ x.val))

end FP

end IEEE754
