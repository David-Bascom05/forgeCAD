/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMath.Refine

/-!
# `estimate`, the real one, with `e.iter().sum()`

The shipped `estimate` sums an expansion with `e.iter().sum()`. Aeneas leaves `Iterator::sum` and
`Sum for f64` opaque; the bridge defines them as std does (`AeneasF64/Core.lean`). Here the sum
over a slice is shown to be the left fold from `-0.0` of the model's `add`, so `estimate` is
`if sign(sum) = sign(top) then sum else top` on the exact IEEE values.
-/

open Aeneas Aeneas.Std Result ControlFlow
open forge_math

namespace ForgeMath

/-- `-0.0` -/
abbrev negZero : F64 := F64.Ops.ofBits 0x8000000000000000

lemma step_lt (s : Slice F64) (i : Nat) (acc : F64) (h : i < s.val.length) :
    F64.sumStep (core.iter.traits.iterator.IteratorSliceIter F64) (acc, ⟨s, i⟩) =
      ok (cont (F64.Ops.add acc s.val[i], ⟨s, i + 1⟩)) := by
  have h' : i < s.len.val := by simpa [Slice.len] using h
  simp [F64.sumStep, core.slice.iter.IteratorSliceIter.next, h']
  rw [dif_pos h]
  simp only [bind_ok]
  rfl

lemma step_ge (s : Slice F64) (i : Nat) (acc : F64) (h : ¬ i < s.val.length) :
    F64.sumStep (core.iter.traits.iterator.IteratorSliceIter F64) (acc, ⟨s, i⟩) = ok (done acc) := by
  have h' : ¬ i < s.len.val := by simpa [Slice.len] using h
  simp [F64.sumStep, core.slice.iter.IteratorSliceIter.next, h']
  rw [dif_neg h]
  simp

lemma sum_loop (s : Slice F64) :
    ∀ n i (acc : F64), s.val.length - i = n →
      loop (F64.sumStep (core.iter.traits.iterator.IteratorSliceIter F64)) (acc, ⟨s, i⟩) =
        ok ((s.val.drop i).foldl F64.Ops.add acc) := by
  intro n
  induction n with
  | zero =>
    intro i acc h
    rw [loop, step_ge s i acc (by omega), bind_ok, List.drop_of_length_le (by omega)]
    rfl
  | succ n ih =>
    intro i acc h
    have hi : i < s.val.length := by omega
    rw [loop, step_lt s i acc hi, bind_ok]
    simp only []
    rw [ih (i + 1) _ (by omega), List.drop_eq_getElem_cons hi, List.foldl_cons]

/-- The sum of a slice is the left fold of the model's `add` from `-0.0`, as std's `Sum`. -/
theorem slice_sum (s : Slice F64) :
    core.iter.traits.iterator.Iterator.sum.default (core.iter.traits.iterator.IteratorSliceIter F64)
      F64.Insts.CoreIterTraitsAccumSumSharedAF64 ⟨s, 0⟩ = ok (s.val.foldl F64.Ops.add negZero) := by
  simp only [core.iter.traits.iterator.Iterator.sum.default, F64.Insts.CoreIterTraitsAccumSumSharedAF64,
    F64.Insts.CoreIterTraitsAccumSumSharedAF64.sum]
  rw [sum_loop s _ 0 _ rfl]
  simp

/-- **The shipped `estimate`**: the fold from `-0.0` when its sign agrees with the last component's,
the last component otherwise (`0` for an empty expansion). -/
theorem estimate_eq (e : Slice F64) :
    predicates.estimate e =
      let top := e.val.getLast?.getD (F64.Ops.ofBits 0)
      let sum := e.val.foldl F64.Ops.add negZero
      if F64.Ops.gt sum (F64.Ops.ofBits 0) = F64.Ops.gt top (F64.Ops.ofBits 0) then
        if F64.Ops.lt sum (F64.Ops.ofBits 0) = F64.Ops.lt top (F64.Ops.ofBits 0) then ok sum
        else ok top
      else ok top := by
  simp only [predicates.estimate, core.slice.Slice.last, core.option.OptionShared0T.copied,
    core.slice.Slice.iter, bind_ok, lift, slice_sum]
  cases h : e.val.getLast? <;> rfl

end ForgeMath
