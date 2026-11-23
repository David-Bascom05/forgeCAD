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
