import ForgeBoolApply.Funs
import ForgeBoolProof.KeepOf

/-!
# `keep_of`, translated, refines `keepOf` (D7's code half, D8.sense.flip)

For the two sides a spec has (0: A, 1: B), the translated boolean.rs `keep_of` returns, without
panic, exactly `keepOf` (ForgeBoolProof/KeepOf.lean) on the matching op and class. So
`keepOf_spec` and `written_eq` hold of the shipped code: it keeps exactly `keepRule`'s regions,
and the sense `apply` writes (`sense != flip`) is `writtenSense`.
-/

namespace forge_bool

open Aeneas Aeneas.Std Result

/-- boolean.rs `Op`, as the statements' op. -/
def opOf : boolean.Op → ForgeBoolProof.Op
  | .Union => .union
  | .Subtract => .diff
  | .Intersect => .inter

/-- boolean.rs `Class`, as the statements' class. -/
def classOf : boolean.Class → ForgeBoolProof.Class
  | .In => .inside
  | .Out => .outside
  | .OnSame => .onSame
  | .OnOpposite => .onOpposite

/-- **The translated `keep_of` is `keepOf`.** -/
theorem keep_of_refines (op : boolean.Op) (side : Usize) (hs : side.val < 2) (c : boolean.Class) :
    boolean.keep_of op side c = ok (ForgeBoolProof.keepOf (opOf op) (side.val == 0) (classOf c)) := by
  have h : side.val = 0 ∨ side.val = 1 := by omega
  rcases h with h | h <;> cases op <;> cases c <;>
    simp [boolean.keep_of, h, ForgeBoolProof.keepOf, opOf, classOf]

/-- **The shipped `keep_of` keeps exactly the keep rule's regions, flipping B's in a
difference.** -/
theorem keep_of_spec (op : boolean.Op) (side : Usize) (hs : side.val < 2) (c : boolean.Class) :
    boolean.keep_of op side c = ok (if ForgeBoolProof.keepRule (opOf op) (side.val == 0) (classOf c)
      then some ((opOf op).isDiff && !(side.val == 0)) else none) := by
  rw [keep_of_refines op side hs c, ForgeBoolProof.keepOf_spec]

end forge_bool
