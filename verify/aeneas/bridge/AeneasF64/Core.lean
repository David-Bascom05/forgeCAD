/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import AeneasF64.Std

/-!
# Library functions Aeneas's Lean library leaves out, defined as std defines them

Aeneas keeps these calls into `core` opaque (each crate's `FunsExternal_Template.lean` lists them).
Each one is defined here, once for every translated crate, by the body std gives it at the
pinned toolchain (nightly-2026-09-17; `library/core/src/...`):
- `<[T]>::last`;
- `Option<&T>::copied`;
- `Iterator::sum`'s default method, `Sum::sum(self)`;
- `Sum<&f64> for f64`, `iter.fold(-0.0, |a, b| a + b)`. The fold starts at `-0.0`, so an empty
  sum is `-0.0`.

The fold is Aeneas's `loop`, the same fixed point its translated loops use. It walks the
iterator's own `next`, so it is defined for every iterator and proven about for the slice
iterator in particular.
-/

open Aeneas Aeneas.Std Result ControlFlow

/-- `<[T]>::last` -/
def core.slice.Slice.last {T : Type _} (s : Slice T) : Result (Option T) := ok s.val.getLast?

/-- `Option<&T>::copied` -/
def core.option.OptionShared0T.copied {T : Type _} (_ : core.marker.Copy T) (o : Option T) :
    Result (Option T) := ok o

/-- `Iterator::sum`'s default method: `Sum::sum(self)`. -/
def core.iter.traits.iterator.Iterator.sum.default
    {Self : Type} {S : Type _} {Clause0_Item : Type _}
    (IteratorInst : core.iter.traits.iterator.Iterator Self Clause0_Item)
    (accumSumInst : core.iter.traits.accum.Sum S Clause0_Item) : Self → Result S :=
  fun it => accumSumInst.sum IteratorInst it

/-- One step of `Sum<&f64> for f64`'s fold. -/
def F64.sumStep {I : Type} (inst : core.iter.traits.iterator.Iterator I F64) (st : F64 × I) :
    Result (ControlFlow (F64 × I) F64) := do
  let (o, it') ← inst.next st.2
  match o with
  | none => ok (done st.1)
  | some x => ok (cont (F64.Ops.add st.1 x, it'))

/-- `Sum<&f64> for f64`: `iter.fold(-0.0, |a, b| a + b)`. -/
def F64.Insts.CoreIterTraitsAccumSumSharedAF64.sum {I : Type}
    (inst : core.iter.traits.iterator.Iterator I F64) (it : I) : Result F64 :=
  loop (F64.sumStep inst) (F64.Ops.ofBits 0x8000000000000000, it)

/-- `<f64 as Default>::default`: `0.0` -/
def F64.Insts.CoreDefaultDefault.default : Result F64 := ok (F64.Ops.ofBits 0)

/-- `x.to_bits()`: the encoding. For a NaN the payload is outside the float trusted base (the
model has one NaN, encoded as the quiet NaN), so no theorem may depend on it. -/
def core.f64.F64.to_bits (x : F64) : Result Std.U64 :=
  ok (UScalar.ofNatCore (IEEE754.FP.toBits x) (by
    have := IEEE754.F64.toBits_lt x
    simpa [UScalarTy.numBits] using this))

/-- `<Option<T> as Try>::branch`: `Some v ⇒ Continue v`, `None ⇒ Break None` (the `?` operator). -/
def core.option.Option.Insts.CoreOpsTry_traitTry.branch {T : Type _} :
    Option T → Result (core.ops.control_flow.ControlFlow (Option Never) T)
  | some v => ok (.Continue v)
  | none => ok (.Break none)

/-- `<Option<T> as FromResidual<Option<!>>>::from_residual`: `None`. -/
def core.option.Option.Insts.CoreOpsTry_traitFromResidualOptionNever.from_residual
    (T : Type _) : Option Never → Result (Option T) := fun _ => ok none

/-- `f64::max`: Lean `opaque`. Rust leaves the sign of a zero result unspecified
(`max(-0.0, 0.0)`), and LLVM's `maxnum` lowers differently on x86-64 and wasm32, so no single
definition is the shipped one; a theorem cannot use its value. -/
opaque core.f64.F64.max : F64 → F64 → Result F64 := fun _ _ => fail .panic

/-- `<f64 as Debug>::fmt`: Lean `opaque` (formatting is not part of any feature). -/
opaque F64.Insts.CoreFmtDebug.fmt :
    F64 → core.fmt.Formatter → Result ((core.result.Result Unit core.fmt.Error) × core.fmt.Formatter) :=
  fun _ f => ok (.Ok (), f)

/-- `<f64 as PartialEq>::eq`, called through the trait: IEEE equality. -/
def F64.Insts.CoreCmpPartialEqF64.eq (a b : F64) : Result Bool := ok (IEEE754.FP.eq a b)
/-- `<f64 as PartialEq>::ne` -/
def F64.Insts.CoreCmpPartialEqF64.ne (a b : F64) : Result Bool := ok (!IEEE754.FP.eq a b)
/-- `<f64 as PartialOrd>::partial_cmp`: `None` when either is NaN; `-0` and `+0` are equal. -/
def F64.Insts.CoreCmpPartialOrdF64.partial_cmp (a b : F64) : Result (Option Ordering) :=
  ok (IEEE754.FP.cmp a b)

/-- `RangeInclusive::contains`, std's `RangeBounds::contains`: `start <= item`, and
`item <= end` (or `item < start` once the range is exhausted, so that it contains nothing). -/
def core.ops.range.RangeInclusive.contains {Idx : Type _} {U : Type _}
    (_ : core.cmp.PartialOrd Idx Idx) (inst1 : core.cmp.PartialOrd Idx U)
    (inst2 : core.cmp.PartialOrd U Idx) (r : core.ops.range.RangeInclusive Idx) (item : U) :
    Result Bool := do
  let a ← inst1.le r.start item
  if a then
    if r.exhausted then inst2.lt item r.start else inst2.le item r.«end»
  else ok false

/-- `Vec::clear`: the empty vector (capacity is not modelled). -/
def alloc.vec.Vec.clear {T : Type _} (A : Type _) (_ : alloc.vec.Vec T) : Result (alloc.vec.Vec T) :=
  ok (alloc.vec.Vec.new T)
/-- `Vec::reserve`: the same vector (capacity is not modelled; an allocation failure aborts the
process, outside every theorem). -/
def alloc.vec.Vec.reserve {T : Type _} (A : Type _) (v : alloc.vec.Vec T) (_ : Std.Usize) :
    Result (alloc.vec.Vec T) := ok v
