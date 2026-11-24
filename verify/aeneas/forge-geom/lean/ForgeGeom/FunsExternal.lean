/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeGeom.Types

/-!
# The opaque functions of forge-geom's translation (FunsExternal_Template.lean)

forge-math's functions are forge-math's translation (translate.conf `DEPS`); `f64`'s and std's
are the float bridge's (verify/aeneas/bridge). What is left is an open L4 gap, a Lean `opaque`
constant: no definition a proof can see, so a theorem holds whatever it answers.
- `intersect::plane_cylinder` (curved; not on the flat route): Aeneas stops in it
  ("Unreachable", at a join of its branches).
-/

open Aeneas Aeneas.Std Result
open forge_geom

namespace forge_geom

/-! ## forge-math's constants

Aeneas reads another crate's `const` as an opaque `Result` global; forge-math's translation defines
each as its value. These bind the one to the other. -/

def forge_math.tol.LINEAR : Result F64 := ok _root_.forge_math.tol.LINEAR
def forge_math.tol.ANGULAR : Result F64 := ok _root_.forge_math.tol.ANGULAR
def forge_math.tol.MIN_LENGTH : Result F64 := ok _root_.forge_math.tol.MIN_LENGTH
def forge_math.vec.Vec3.Z : Result _root_.forge_math.vec.Vec3 := ok _root_.forge_math.vec.Vec3.Z

/-- `forge_geom::intersect::plane_cylinder`: an open L4 gap. -/
opaque intersect.plane_cylinder : surface.Plane → surface.Cylinder → Result intersect.SurfaceSurface

end forge_geom
