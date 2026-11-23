import Aeneas
import ForgeFont.Types
open Aeneas Aeneas.Std Result ControlFlow Error

/-!
# The std items the charstring machine (cff_core.rs) uses that Aeneas leaves opaque

Each is defined as std documents it: `Vec::default` is the empty vector, `Vec::clear` empties
it, `Vec::pop` takes the last element off (none from an empty one), `mem::take` returns the value
and leaves the type's default in its place.
-/

open forge_font

@[rust_fun "alloc::vec::{core::default::Default<alloc::vec::Vec<@T>>}::default"]
def alloc.vec.Vec.Insts.CoreDefaultDefault.default (T : Type _) : Result (alloc.vec.Vec T) :=
  ok (alloc.vec.Vec.new T)

@[rust_fun "alloc::vec::{alloc::vec::Vec<@T>}::clear"]
def alloc.vec.Vec.clear {T : Type _} (_A : Type _) (_v : alloc.vec.Vec T) : Result (alloc.vec.Vec T) :=
  ok (alloc.vec.Vec.new T)

@[rust_fun "alloc::vec::{alloc::vec::Vec<@T>}::pop"]
def alloc.vec.Vec.pop {T : Type _} (_A : Type _) (v : alloc.vec.Vec T) :
    Result ((Option T) × (alloc.vec.Vec T)) :=
  ok (v.val.getLast?, alloc.vec.Vec.from v.val.dropLast
    (Nat.le_trans (by simp) v.property))

@[rust_fun "core::mem::take"]
def core.mem.take {T : Type _} (defaultDefaultInst : core.default.Default T) (x : T) :
    Result (T × T) := do
  let d ← defaultDefaultInst.default
  ok (x, d)
