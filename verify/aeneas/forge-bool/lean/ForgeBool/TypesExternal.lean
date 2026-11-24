/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas
import ForgeGeom

/-!
# The opaque types of forge-bool's translation (TypesExternal_Template.lean)

Every one is defined elsewhere, so this file only imports them:
- `core::array::IntoIter` is the float bridge's (AeneasF64/Core.lean);
- forge-geom's opaque types (`ProcCurve`, `Proc`, `IntCurve`, open L4 gaps) are forge-geom's
  translation's (verify/aeneas/forge-geom, TypesExternal.lean).
-/
