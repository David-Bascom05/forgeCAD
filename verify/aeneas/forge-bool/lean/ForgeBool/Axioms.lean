/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Lean
import ForgeBool.Funs
import ForgeBool.D2Flat

/-!
# No axiom in forge-bool's translation

`#standard_axioms D₁ D₂ …` fails the build if a named declaration rests on any axiom other than
`propext`, `Classical.choice` and `Quot.sound`. The opaque items are definitions or Lean `opaque`
constants (FunsExternal.lean), or the dependencies' translations. The theorems about the
translation join this list as they are proved.
-/

open Lean Elab Command

elab "#standard_axioms " ids:ident* : command => do
  for id in ids do
    let n ← liftCoreM (realizeGlobalConstNoOverloadWithInfo id)
    let axs ← liftCoreM (collectAxioms n)
    for a in axs do
      unless a == ``propext || a == ``Classical.choice || a == ``Quot.sound do
        throwError "{n} rests on the axiom {a}"

#standard_axioms
  forge_bool.ssi.surfaces_in forge_bool.ssi.closed_or_flush forge_bool.ssi.closed_form
  forge_bool.ssi.nearly_flush forge_bool.ssi.turned_flush forge_bool.ssi.spread_apart
  forge_bool.d2.plane_plane_only forge_bool.d2.parallel_in_only forge_bool.d2.nearly_flush_planes_only
  forge_bool.d2.spindle_in_planes forge_bool.d2.closed_or_flush_planes_only
  forge_bool.d2.surfaces_in_planes
