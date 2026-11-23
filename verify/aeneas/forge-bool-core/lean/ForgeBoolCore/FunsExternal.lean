/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
-- [forge_bool_core]: the std functions Aeneas leaves opaque (FunsExternal_Template.lean lists
-- them). Each is a definition, as std's body is, or a Lean `opaque` where its value never
-- matters (hashing, formatting); none is an axiom.
import Aeneas
import ForgeBoolCore.Types
open Aeneas Aeneas.Std Result ControlFlow Error
set_option linter.dupNamespace false
set_option linter.unusedVariables false
open forge_bool_core

/-- `<bool as PartialEq>::ne` -/
@[rust_fun "core::cmp::impls::{core::cmp::PartialEq<bool, bool>}::ne"]
def Bool.Insts.CoreCmpPartialEqBool.ne (a b : Bool) : Result Bool := ok (a != b)

/-- `<(U, T) as Debug>::fmt`: output only, never inspected by the core. -/
@[rust_fun "core::fmt::{core::fmt::Debug<(@U, @T)>}::fmt"]
opaque Pair.Insts.CoreFmtDebug.fmt
  {U : Type} {T : Type} (DebugInst : core.fmt.Debug U) (DebugInst1 : core.fmt.Debug T) :
  (U × T) → core.fmt.Formatter → Result ((core.result.Result Unit core.fmt.Error) × core.fmt.Formatter)

/-- `<isize as Hash>::hash`: a hasher's state is not modelled. -/
@[rust_fun "core::hash::impls::{core::hash::Hash<isize>}::hash"]
opaque Isize.Insts.CoreHashHash.hash {H : Type} (HasherInst : core.hash.Hasher H) : Std.Isize → H → Result H

/-- `<u32 as Hash>::hash` -/
@[rust_fun "core::hash::impls::{core::hash::Hash<u32>}::hash"]
opaque U32.Insts.CoreHashHash.hash {H : Type} (HasherInst : core.hash.Hasher H) : Std.U32 → H → Result H

/-- `<(U, T) as PartialEq>::ne`, std's tuple body: `self.0 != other.0 || self.1 != other.1`. -/
@[rust_fun "core::tuple::{core::cmp::PartialEq<(@U, @T), (@U, @T)>}::ne"]
def Pair.Insts.CoreCmpPartialEqPair.ne
    {U : Type _} {T : Type _} (cmpPartialEqInst : core.cmp.PartialEq U U)
    (cmpPartialEqInst1 : core.cmp.PartialEq T T) (a b : U × T) : Result Bool := do
  let x ← cmpPartialEqInst.ne a.1 b.1
  if x then ok true else cmpPartialEqInst1.ne a.2 b.2

/-- `<(U, T) as PartialEq>::eq`, std's tuple body: `self.0 == other.0 && self.1 == other.1`. -/
@[rust_fun "core::tuple::{core::cmp::PartialEq<(@U, @T), (@U, @T)>}::eq"]
def Pair.Insts.CoreCmpPartialEqPair.eq
    {U : Type _} {T : Type _} (cmpPartialEqInst : core.cmp.PartialEq U U)
    (cmpPartialEqInst1 : core.cmp.PartialEq T T) (a b : U × T) : Result Bool := do
  let x ← cmpPartialEqInst.eq a.1 b.1
  if x then cmpPartialEqInst1.eq a.2 b.2 else ok false

/-- `Vec::default()`: the empty vector. -/
@[rust_fun "alloc::vec::{core::default::Default<alloc::vec::Vec<@T>>}::default"]
def alloc.vec.Vec.Insts.CoreDefaultDefault.default (T : Type _) : Result (alloc.vec.Vec T) :=
  ok (alloc.vec.Vec.new T)
