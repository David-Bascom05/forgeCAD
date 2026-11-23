/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import AeneasF64
import ForgeGeom.Types

/-!
# The opaque functions of forge-geom's translation (FunsExternal_Template.lean)

forge-math's vector operations, which Charon leaves opaque from a dependency crate: each is
`vec_ops!` (kernel/forge-math/src/vec.rs, lines 239-256) spelled out, field by field, with the
bridge's binary64 operations: `$V { $($f: self.$f op s),+ }`. `F64`'s `clone` is the bridge's.
-/

open Aeneas Aeneas.Std Result
open forge_geom

namespace forge_geom

/-- `#[derive(Clone, Copy)]`: the value itself. -/
def forge_math.vec.Vec3.Insts.CoreCloneClone.clone (v : forge_math.vec.Vec3) :
    Result forge_math.vec.Vec3 := ok v

/-- `Vec3::ZERO = Vec3 { x: 0.0, y: 0.0, z: 0.0 }`. -/
def forge_math.vec.Vec3.ZERO : Result forge_math.vec.Vec3 :=
  ok { x := F64.Ops.ofBits 0, y := F64.Ops.ofBits 0, z := F64.Ops.ofBits 0 }

/-- `impl Div<f64> for Vec3`: each field over s. -/
def forge_math.vec.Vec3.Insts.CoreOpsArithDivF64Vec3.div (v : forge_math.vec.Vec3) (s : F64) :
    Result forge_math.vec.Vec3 :=
  ok { x := F64.Ops.div v.x s, y := F64.Ops.div v.y s, z := F64.Ops.div v.z s }

/-- `impl Mul<f64> for Vec3`: each field times s. -/
def forge_math.vec.Vec3.Insts.CoreOpsArithMulF64Vec3.mul (v : forge_math.vec.Vec3) (s : F64) :
    Result forge_math.vec.Vec3 :=
  ok { x := F64.Ops.mul v.x s, y := F64.Ops.mul v.y s, z := F64.Ops.mul v.z s }

/-- `impl Sub for Vec3`: field by field. -/
def forge_math.vec.Vec3.Insts.CoreOpsArithSubVec3Vec3.sub (v o : forge_math.vec.Vec3) :
    Result forge_math.vec.Vec3 :=
  ok { x := F64.Ops.sub v.x o.x, y := F64.Ops.sub v.y o.y, z := F64.Ops.sub v.z o.z }

end forge_geom
