/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeMath
import ForgeMesh.Types

/-!
# The opaque functions of forge-mesh's CDT translation (FunsExternal_Template.lean)

Everything here is a definition: no axiom. The `f64` constants and functions (`INFINITY`,
`MIN_POSITIVE`, `abs`, `is_finite`, `sqrt`, `clone`) come from the float bridge.

- `forge_math::predicates::orient2d` is verify/aeneas/forge-math's translation of it, whose
  theorems (`ForgeMath/Predicates.lean`) are about that same definition: the CDT's `Point2` is
  its own translation of the same struct, converted field by field.
- The std functions Aeneas's library leaves out (`Vec::pop`, `clear`, `remove`, `split_off`,
  `Default for Vec`, `mem::take`, `f64 == f64`) are written as std documents them, over the
  library's list model of `Vec`.
- `forge_geom::budget::{over, spend_many}` (a thread-local work counter, the shell's) and the
  `Debug` of a `str` and of an `f64` are Lean `opaque` constants: no definition a proof can see, so a theorem
  about the CDT holds whatever the budget answers.
- `cdt::triangulate`, the one shell that names a `dyn Refine`, is kept opaque by translate.conf.

They are in `namespace forge_mesh`, where the generated code looks first, so that they do not
collide with ForgeMath's own names when a proof imports both.
-/

open Aeneas Aeneas.Std Result

namespace forge_mesh

/-! ## forge-math: orient2d, bound to its proven translation -/

/-- The CDT's `Point2` as forge-math's translation has it. -/
def forge_math.vec.Point2.toMath (p : forge_math.vec.Point2) : _root_.forge_math.vec.Point2 :=
  { x := p.x, y := p.y }

def forge_math.predicates.orient2d (a b c : forge_math.vec.Point2) : Result F64 :=
  _root_.forge_math.predicates.orient2d a.toMath b.toMath c.toMath

/-! ## std -/

def F64.Insts.CoreCmpPartialEqF64.eq (a b : F64) : Result Bool := ok (F64.Ops.eq a b)

/-- `Debug for f64` (reached from the derived `Debug` of `Need` and `CdtError`): formatting,
which no theorem about the CDT reads. -/
opaque F64.Insts.CoreFmtDebug.fmt :
    F64 → core.fmt.Formatter → Result ((core.result.Result Unit core.fmt.Error) × core.fmt.Formatter) :=
  fun _ f => ok (.Ok (), f)

opaque Str.Insts.CoreFmtDebug.fmt :
    Str → core.fmt.Formatter → Result ((core.result.Result Unit core.fmt.Error) × core.fmt.Formatter) :=
  fun _ f => ok (.Ok (), f)

/-- `mem::take`: the place's value, and the default left in its stead. -/
def core.mem.take {T : Type _} (defaultDefaultInst : core.default.Default T) (x : T) :
    Result (T × T) := do
  let d ← defaultDefaultInst.default
  ok (x, d)

def alloc.vec.Vec.Insts.CoreDefaultDefault.default (T : Type _) : Result (alloc.vec.Vec T) :=
  ok (alloc.vec.Vec.new T)

def alloc.vec.Vec.clear {T : Type _} (A : Type _) (_ : alloc.vec.Vec T) : Result (alloc.vec.Vec T) :=
  ok (alloc.vec.Vec.new T)

/-- `Vec::pop`: the last element, if any, and the vector without it. -/
def alloc.vec.Vec.pop {T : Type _} (A : Type _) (v : alloc.vec.Vec T) :
    Result ((Option T) × (alloc.vec.Vec T)) :=
  ok (v.val.getLast?, alloc.vec.Vec.from v.val.dropLast
    (by have := v.property; simp only [List.length_dropLast]; omega))

/-- `Vec::remove`: the element at i and the vector without it; a panic past the end. -/
def alloc.vec.Vec.remove {T : Type _} (A : Type _) (v : alloc.vec.Vec T) (i : Std.Usize) :
    Result (T × (alloc.vec.Vec T)) :=
  if h : i.val < v.val.length then
    ok (v.val[i.val], alloc.vec.Vec.from (v.val.eraseIdx i.val)
      (by have := v.property; simp only [List.length_eraseIdx_of_lt h]; omega))
  else fail .panic

/-- `Vec::split_off`: the elements from `at` on, and the vector cut to before it; a panic if
`at` is past the end. -/
def alloc.vec.Vec.split_off {T : Type _} {A : Type _} (_ : core.clone.Clone A)
    (v : alloc.vec.Vec T) (at_ : Std.Usize) : Result ((alloc.vec.Vec T) × (alloc.vec.Vec T)) :=
  if at_.val ≤ v.val.length then
    ok (alloc.vec.Vec.from (v.val.drop at_.val)
          (by have := v.property; simp only [List.length_drop]; omega),
        alloc.vec.Vec.from (v.val.take at_.val)
          (by have := v.property; simp only [List.length_take]; omega))
  else fail .panic

/-! ## The shell: forge_geom's work budget -/

opaque forge_geom.budget.over : Result Bool := ok false

opaque forge_geom.budget.spend_many : Std.U64 → Result Unit := fun _ => ok ()

/-! ## Kept opaque by translate.conf -/

opaque cdt.triangulate :
    Slice forge_math.vec.Point2 → Slice (alloc.vec.Vec Std.U32) → Slice forge_math.vec.Point2 →
      Result (core.result.Result cdt.Triangulation cdt.CdtError) :=
  fun _ _ _ => fail .panic

end forge_mesh
