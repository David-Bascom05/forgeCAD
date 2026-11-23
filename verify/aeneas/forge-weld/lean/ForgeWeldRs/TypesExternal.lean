/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Aeneas

/-!
# The opaque types of forge-weld's translation (TypesExternal_Template.lean)

A section's piece (`forge_ops::profile::Segment`) may be a spline, behind an `Arc`. Neither the
`Arc` nor the spline's evaluator is translated: they are Lean `opaque` types, with no axiom. No
theorem here is about a spline piece.
-/

open Aeneas Aeneas.Std

opaque alloc.sync.Arc (T : Type) : Type
opaque forge_geom.spline.Spline : Type
