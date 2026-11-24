/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeNum.Types

/-!
# The opaque functions of forge-num's translation (FunsExternal_Template.lean)

translate.sh lists, in FunsExternal_Template.lean, every function Aeneas left opaque. Each one must be
defined here, in the float bridge (verify/aeneas/bridge), or by a DEP's translation; translate.sh
fails and names any that is not.

What the pinned Aeneas Std (2e3d448) lacks, checked in its source, and the wrapper file in
kernel/forge-num keeps behind one name each:
- `unsigned_abs` (i128 → u128): define it as `if x < 0 then -x else x` on the integer value.
- `trailing_zeros` (u32/u64/u128): define it by its value (the largest k with 2^k ∣ x, and the bit
  width for 0), as a `def` a proof can unfold.
What Std has: I128/U128 arithmetic (overflow-checked: `ok` proves no overflow), checked_add/sub/mul/
div/rem, leading_zeros, wrapping_add/sub/mul/shl/shr, overflowing_*, saturating_*, rotate_*.

Anything kept opaque for speed (iterator chains Aeneas cannot take, and so on) is a Lean `opaque`
here, never an `axiom`: no definition a proof can see, so a theorem holds whatever it answers. Each
one is an open L4 gap; list it in the ledger.
-/

open Aeneas Aeneas.Std Result
open forge_num

set_option linter.unusedVariables false

/-! ## std, as std's body is (or `opaque` where the value never matters) -/

/-- `<bool as PartialEq>::ne` -/
def Bool.Insts.CoreCmpPartialEqBool.ne (a b : Bool) : Result Bool := ok (a != b)

/-- `<Ordering as PartialEq>::eq` (derived: the same variant) -/
def core.cmp.Ordering.Insts.CoreCmpPartialEqOrdering.eq (a b : Ordering) : Result Bool :=
  ok (decide (a = b))

/-- `<u32 as Hash>::hash` and the like: a hasher's state is not modelled (hashing is never part of a
value the kernel computes). -/
opaque U32.Insts.CoreHashHash.hash {H : Type _} (HasherInst : core.hash.Hasher H) : Std.U32 → H → Result H
opaque Isize.Insts.CoreHashHash.hash {H : Type _} (HasherInst : core.hash.Hasher H) : Std.Isize → H → Result H
opaque I128.Insts.CoreHashHash.hash {H : Type _} (HasherInst : core.hash.Hasher H) : Std.I128 → H → Result H
opaque Bool.Insts.CoreHashHash.hash {H : Type _} (HasherInst : core.hash.Hasher H) : Bool → H → Result H
opaque alloc.vec.Vec.Insts.CoreHashHash.hash
  {T : Type _} (A : Type _) {H : Type _} (corehashHashInst : core.hash.Hash T)
  (corehashHasherInst : core.hash.Hasher H) : alloc.vec.Vec T → H → Result H

/-- `Vec::default()`: the empty vector. -/
def alloc.vec.Vec.Insts.CoreDefaultDefault.default (T : Type _) : Result (alloc.vec.Vec T) :=
  ok (alloc.vec.Vec.new T)

/-- `Vec::pop`: the last element, if any, and the vector without it. -/
def alloc.vec.Vec.pop {T : Type _} (A : Type _) (v : alloc.vec.Vec T) :
    Result ((Option T) × (alloc.vec.Vec T)) :=
  ok (v.val.getLast?, alloc.vec.Vec.from v.val.dropLast
    (by have := v.property; simp only [List.length_dropLast]; omega))

/-- `Vec::truncate`: the first `n` elements (all of them if there are fewer). -/
def alloc.vec.Vec.truncate {T : Type _} (A : Type _) (v : alloc.vec.Vec T) (n : Std.Usize) :
    Result (alloc.vec.Vec T) :=
  ok (alloc.vec.Vec.from (v.val.take n.val)
    (by have := v.property; simp only [List.length_take]; omega))

/-- The number of trailing zero bits of `n` in a `w`-bit word (`w` for zero). -/
def tzBits : Nat → Nat → Nat
  | 0, _ => 0
  | w + 1, n => if n % 2 = 0 then (if n = 0 then w + 1 else 1 + tzBits w (n / 2)) else 0

/-- `u32::trailing_zeros` -/
def core.num.U32.trailing_zeros (x : Std.U32) : Result Std.U32 :=
  ok (Std.U32.ofNat (tzBits 32 x.val) (by
    have : tzBits 32 x.val ≤ 32 := by
      generalize x.val = n; generalize 32 = w
      induction w generalizing n with
      | zero => simp [tzBits]
      | succ w ih => unfold tzBits; split <;> (try split) <;> (try have := ih (n / 2)) <;> omega
    simp [UScalar.cMax, UScalar.rMax, U32.rMax]; omega))

/-- `i64::trailing_zeros`: of the two's-complement bits, which for `x` and `-x` are the same (the
magnitude's), and 64 for zero. -/
def core.num.I64.trailing_zeros (x : Std.I64) : Result Std.U32 :=
  ok (Std.U32.ofNat (tzBits 64 x.val.natAbs) (by
    have : tzBits 64 x.val.natAbs ≤ 64 := by
      generalize x.val.natAbs = n; generalize 64 = w
      induction w generalizing n with
      | zero => simp [tzBits]
      | succ w ih => unfold tzBits; split <;> (try split) <;> (try have := ih (n / 2)) <;> omega
    simp [UScalar.cMax, UScalar.rMax, U32.rMax]; omega))

/-- `i128::unsigned_abs`: the magnitude, which fits a u128 (|i128::MIN| = 2^127). -/
def core.num.I128.unsigned_abs (x : Std.I128) : Result Std.U128 :=
  ok (Std.U128.ofNat x.val.natAbs (by
    have h1 := x.hmin; have h2 := x.hmax
    simp [UScalar.cMax, UScalar.rMax, U128.rMax] at *; omega))

namespace forge_num

/-! ## Kept opaque by translate.conf (open L4 gaps) -/

/-- `Big::top64`: Aeneas stops in it ("Could not match the contexts"). -/
opaque int.Big.top64 : int.Big → Result (Std.U64 × Std.I32)

/-- `Big::cmp_big`, `Dy::cmp_dy`: `match x.signum() { 1 => Greater, -1 => Less, _ => Equal }`; Aeneas
writes that as a Lean match on `i32` literals, which Lean cannot compile when one is negative. -/
opaque int.Big.cmp_big : int.Big → int.Big → Result Ordering
opaque dy.Dy.cmp_dy : dy.Dy → dy.Dy → Result Ordering

end forge_num
