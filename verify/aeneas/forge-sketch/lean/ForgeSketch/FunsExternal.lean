import Aeneas
import ForgeSketch.Types
open Aeneas Aeneas.Std Result ControlFlow Error

/-!
# What the translation of `winding::line` leaves opaque

`f64::min` and `f64::max` are defined as std documents them: the other argument when one is NaN,
else the smaller (larger). Of `+0` and `-0` std may return either; these return the first, and no
comparison in `line` tells them apart.

`orient2d` is forge-math's, translated and proven there (verify/aeneas/forge-math). Here it is an
`opaque` constant: nothing about it is assumed, and the theorems about `line` take what they need
of it (a finite result with the exact determinant's sign) as a hypothesis.
-/

open forge_sketch

@[rust_fun "core::f64::{f64}::max"]
def core.f64.F64.max (a b : F64) : Result F64 :=
  ok (if a.isNaN then b else if b.isNaN then a else if F64.Ops.lt a b then b else a)

@[rust_fun "core::f64::{f64}::min"]
def core.f64.F64.min (a b : F64) : Result F64 :=
  ok (if a.isNaN then b else if b.isNaN then a else if F64.Ops.lt b a then b else a)

@[rust_fun "forge_math::predicates::orient2d"]
opaque forge_math.predicates.orient2d :
    forge_math.vec.Point2 → forge_math.vec.Point2 → forge_math.vec.Point2 → Result F64
