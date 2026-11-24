/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeMath.Types

/-!
# The opaque functions of forge-math's translation (FunsExternal_Template.lean)

The `f64` functions (`abs`, `is_nan`, `EPSILON`, `clone`, `Sum for f64`) and the std functions
Aeneas leaves out (`last`, `copied`, `Iterator::sum`) come from the bridge,
`verify/aeneas/bridge/AeneasF64/{Std,Core}.lean`. The rest are here:
- the iterator adaptor, and the functions that `translate.conf` keeps opaque, are Lean
  `opaque` constants. An `opaque` constant has no axiom and no definition a proof can see, so a
  theorem whose computation reached one could say nothing about its result. The theorems here are
  about the fast paths, which never reach them.

Keeping them opaque is temporary. verify/aeneas/forge-math/README.md says what makes each one
translatable.
-/

open Aeneas Aeneas.Std Result
open forge_math

/-! ## Opaque: the iterator adaptor of `cross_two`'s `for v in &mut q[..nq]` -/

opaque MutASlice.Insts.CoreIterTraitsCollectIntoIteratorMutATIterMut.into_iter
  {T : Type _} :
  Slice T → Result ((core.slice.iter.IterMut T) × (core.slice.iter.IterMut T → Slice T)) :=
  fun _ => fail .panic

/-! ## Opaque: kept so by translate.conf -/

opaque predicates.cross_big : F64 → F64 → F64 → F64 → Result F64 := fun _ _ _ _ => fail .panic
opaque predicates.sum_into : Slice F64 → Slice F64 → Slice F64 → Result (Std.Usize × (Slice F64)) :=
  fun _ _ _ => fail .panic
opaque predicates.scale_into : Slice F64 → F64 → Slice F64 → Result (Std.Usize × (Slice F64)) :=
  fun _ _ _ => fail .panic
opaque predicates.orient2d_big : vec.Point2 → vec.Point2 → vec.Point2 → Result F64 :=
  fun _ _ _ => fail .panic
opaque predicates.dot3_big : vec.Vec3 → vec.Vec3 → Result F64 := fun _ _ => fail .panic
opaque predicates.exact_diffs {N : Std.Usize} : Array (F64 × F64) N → Result (Option (Array F64 N)) :=
  fun _ => fail .panic
opaque predicates.incircle : vec.Point2 → vec.Point2 → vec.Point2 → vec.Point2 → Result F64 :=
  fun _ _ _ _ => fail .panic
