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

/-- `Iterator::all` on a slice iterator, as std's `try_fold` runs it: each element in turn,
stopping after the first `false` (that element consumed). -/
def core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.all {T F : Type _}
    (inst : core.ops.function.FnMut F T Bool) (it : core.slice.iter.Iter T) (f : F) :
    Result (Bool × core.slice.iter.Iter T) :=
  if h : it.i < it.slice.val.length then do
    let (b, f) ← inst.call_mut f it.slice.val[it.i]
    if b then
      core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.all inst
        { it with i := it.i + 1 } f
    else ok (false, { it with i := it.i + 1 })
  else ok (true, it)
termination_by it.slice.val.length - it.i
decreasing_by all_goals (simp_wf; omega)

/-- One step of `Iterator::all`'s default method. -/
def core.iter.traits.iterator.Iterator.allStep {Self F Item : Type _}
    (inst : core.iter.traits.iterator.Iterator Self Item)
    (fm : core.ops.function.FnMut F Item Bool)
    (st : Self × F) : Result (ControlFlow (Self × F) (Bool × Self)) := do
  let (x, it) ← inst.next st.1
  match x with
  | none => ok (done (true, it))
  | some x =>
    let (b, f) ← fm.call_mut st.2 x
    if b then ok (cont (it, f)) else ok (done (false, it))

/-- `Iterator::all`'s default method, over the iterator's own `next` (Aeneas's `loop`). -/
def core.iter.traits.iterator.Iterator.all.default {Self F Item : Type _}
    (inst : core.iter.traits.iterator.Iterator Self Item)
    (fm : core.ops.function.FnMut F Item Bool) :
    Self → F → Result (Bool × Self) :=
  fun it f => loop (core.iter.traits.iterator.Iterator.allStep inst fm) (it, f)

/-- `f64::next_up`, std's body over the bits (`library/core/src/num/f64.rs`). -/
def core.f64.F64.next_up (x : F64) : Result F64 :=
  let bits := IEEE754.FP.toBits x
  if x.isNaN || bits == 0x7ff0000000000000 then ok x
  else
    let abs := bits % 2 ^ 63
    let next := if abs == 0 then 1 else if bits == abs then bits + 1 else bits - 1
    ok (F64.Ops.ofBits next)

/-- `f64::next_down`, std's body over the bits. -/
def core.f64.F64.next_down (x : F64) : Result F64 :=
  let bits := IEEE754.FP.toBits x
  if x.isNaN || bits == 0xfff0000000000000 then ok x
  else
    let abs := bits % 2 ^ 63
    let next :=
      if abs == 0 then 0x8000000000000001 else if bits == abs then bits - 1 else bits + 1
    ok (F64.Ops.ofBits next)

/-- `x.ceil()`: `-(-x).floor()`, exact in IEEE 754 (a result `0` from `x ∈ (-1, 0]` is `-0`, as
roundToIntegralTowardPositive gives). -/
def std.f64.F64.ceil (x : F64) : Result F64 :=
  ok (F64.Ops.neg (F64.Ops.floor (F64.Ops.neg x)))
def core.f64.F64.ceil (x : F64) : Result F64 := std.f64.F64.ceil x

/-- `i64::rem_euclid`, std's body: `%` (which fails on a zero divisor and on `MIN % -1`), then
the divisor's magnitude added to a negative remainder. -/
def core.num.I64.rem_euclid (x y : Std.I64) : Result Std.I64 := do
  let r ← x % y
  if r < 0#i64 then (if y < 0#i64 then r - y else r + y) else ok r

/-- `[T; N]::map`'s walk: each element in order through the closure, which carries its state. -/
def core.array.mapList {T F U : Type _} (inst : core.ops.function.FnMut F T U) :
    List T → F → Result (List U × F)
  | [], f => ok ([], f)
  | x :: xs, f => do
    let (y, f) ← inst.call_mut f x
    let (ys, f) ← core.array.mapList inst xs f
    ok (y :: ys, f)

/-- `[T; N]::map(f)`, std's body: a new array of `f` applied to each element in order. (The walk
gives one element per element, so the length check never fails; a proof that uses `map` shows
that once.) -/
def core.array.Array.map {T F U : Type _} {N : Std.Usize} (inst : core.ops.function.FnMut F T U)
    (a : Array T N) (f : F) : Result (Array U N) := do
  let (l, _) ← core.array.mapList inst a.val f
  if h : l.length = N.val then ok (Array.from l h) else fail .panic

/-- `Iterator::any` on a slice iterator, as std's `try_fold` runs it: each element in turn,
stopping after the first `true` (that element consumed). -/
def core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.any {T F : Type _}
    (inst : core.ops.function.FnMut F T Bool) (it : core.slice.iter.Iter T) (f : F) :
    Result (Bool × core.slice.iter.Iter T) :=
  if h : it.i < it.slice.val.length then do
    let (b, f) ← inst.call_mut f it.slice.val[it.i]
    if b then ok (true, { it with i := it.i + 1 })
    else
      core.slice.iter.Iter.Insts.CoreIterTraitsIteratorIteratorSharedAT.any inst
        { it with i := it.i + 1 } f
  else ok (false, it)
termination_by it.slice.val.length - it.i
decreasing_by all_goals (simp_wf; omega)

/-- `f64::MIN` = `-f64::MAX` -/
def core.f64.F64.MIN : Result F64 := ok (F64.Ops.ofBits 0xFFEFFFFFFFFFFFFF)

universe u in
/-- `core::array::IntoIter<T, N>`, `[T; N]`'s by-value iterator: the elements not yet taken, in
order (std keeps the array and a live range of it). -/
structure core.array.iter.IntoIter (T : Type u) (N : Std.Usize) : Type u where
  rest : List T

/-- `<[T; N] as IntoIterator>::into_iter`: every element, in order. -/
def Array.Insts.CoreIterTraitsCollectIntoIteratorTIntoIter.into_iter {T : Type _} {N : Std.Usize}
    (a : Array T N) : Result (core.array.iter.IntoIter T N) := ok ⟨a.val⟩

/-- `<array::IntoIter<T, N> as Iterator>::next`: the first element not yet taken. -/
def core.array.iter.IntoIter.Insts.CoreIterTraitsIteratorIterator.next {T : Type _}
    {N : Std.Usize} (it : core.array.iter.IntoIter T N) :
    Result ((Option T) × (core.array.iter.IntoIter T N)) :=
  match it.rest with
  | [] => ok (none, it)
  | x :: xs => ok (some x, ⟨xs⟩)
