/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import AeneasF64

/-!
# The opaque types of forge-num's translation (TypesExternal_Template.lean)

Every type Aeneas leaves opaque (TypesExternal_Template.lean) is defined here. Typical ones in other
crates: `MaybeUninit<T>` (std's array/vector internals: a value or none). None is expected for
forge-num if its types are plain structs and enums of integers, f64 and Vec.
-/

open Aeneas Aeneas.Std
