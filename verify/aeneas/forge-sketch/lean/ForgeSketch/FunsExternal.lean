import Aeneas
import ForgeSketch.Types
open Aeneas Aeneas.Std Result ControlFlow Error

/-!
# What the translation of `winding::line` leaves opaque

The shared floating-point bridge supplies `f64::min` and `f64::max`.

`orient2d` is forge-math's, translated and proven there (verify/aeneas/forge-math). Here it is an
`opaque` constant: nothing about it is assumed, and the theorems about `line` take what they need
of it (a finite result with the exact determinant's sign) as a hypothesis.
-/

open forge_sketch

@[rust_fun "forge_math::predicates::orient2d"]
opaque forge_math.predicates.orient2d :
    forge_math.vec.Point2 → forge_math.vec.Point2 → forge_math.vec.Point2 → Result F64
