/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeWeldRs.Types

/-!
# The opaque functions of forge-weld's translation (FunsExternal_Template.lean)

`f64::NEG_INFINITY` and `f64::sqrt` come from the float bridge
(`verify/aeneas/bridge/AeneasF64/Std.lean`). The rest are Lean `opaque` constants: no axiom, and
no definition a proof can see, so a theorem whose computation reached one could say nothing about
its result. The theorems here are about sections whose pieces are lines, which never reach them:
- an arc's points and its angle test (`arc_point`, `atan2`, `TAU`, `on_arc`: forge-math's
  `dmath`, kept opaque by translate.conf);
- a spline piece's ends, behind its `Arc`.
-/

open Aeneas Aeneas.Std Result
open forge_weld

opaque alloc.sync.Arc.Insts.CoreOpsDerefDeref.deref {T : Type} (A : Type) :
    alloc.sync.Arc T → Result T := fun _ => fail .panic
opaque forge_geom.spline.PlaneSpline.start : forge_geom.spline.PlaneSpline → Result forge_math.vec.Point2 :=
  fun _ => fail .panic
opaque forge_geom.spline.PlaneSpline.end : forge_geom.spline.PlaneSpline → Result forge_math.vec.Point2 :=
  fun _ => fail .panic
opaque forge_math.dmath.TAU : Result F64 := fail .panic
opaque forge_math.dmath.atan2 : F64 → F64 → Result F64 := fun _ _ => fail .panic
opaque forge_ops.profile.arc_point : forge_math.vec.Point2 → F64 → F64 → Result forge_math.vec.Point2 :=
  fun _ _ _ => fail .panic
opaque section.on_arc : F64 → F64 → F64 → Result Bool := fun _ _ _ => fail .panic
