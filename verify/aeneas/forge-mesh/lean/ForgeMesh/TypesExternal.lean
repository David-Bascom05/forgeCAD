/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas

/-!
# The opaque types of forge-mesh's CDT translation (TypesExternal_Template.lean)

`MaybeUninit` comes in with std's array and vector internals; no translated function of the CDT
reads one. It is defined as what it can hold: a value or none.
-/

open Aeneas Aeneas.Std

namespace forge_mesh

def core.mem.maybe_uninit.MaybeUninit (T : Type _) : Type _ := Option T

end forge_mesh
