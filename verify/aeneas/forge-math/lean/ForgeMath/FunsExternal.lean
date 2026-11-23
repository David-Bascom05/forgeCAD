/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeMath.Types

/-!
# The opaque functions of forge-math's translation (FunsExternal_Template.lean)

The `f64` functions (`abs`, `is_nan`, `EPSILON`, `clone`, `Sum for f64`) and the std functions
Aeneas leaves out (`last`, `copied`, `Iterator::sum`) come from the bridge,
`verify/aeneas/bridge/AeneasF64/{Std,Core}.lean`. The four functions below are the cold integer
fallback (`Big`), run only when a slow path's expansion arithmetic overflows to NaN; translate.conf
keeps them opaque. They are Lean `opaque` constants: no axiom, and no definition a proof can
see, so a theorem whose computation reached one could say nothing about its result.
-/

open Aeneas Aeneas.Std Result
open forge_math

/-! ## Opaque: kept so by translate.conf -/

opaque predicates.cross_big : F64 → F64 → F64 → F64 → Result F64 := fun _ _ _ _ => fail .panic
opaque predicates.orient2d_big : vec.Point2 → vec.Point2 → vec.Point2 → Result F64 :=
  fun _ _ _ => fail .panic
opaque predicates.dot3_big : vec.Vec3 → vec.Vec3 → Result F64 := fun _ _ => fail .panic
opaque predicates.incircle_big : vec.Point2 → vec.Point2 → vec.Point2 → vec.Point2 → Result F64 :=
  fun _ _ _ _ => fail .panic
