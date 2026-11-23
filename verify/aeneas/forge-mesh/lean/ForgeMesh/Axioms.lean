/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Lean
import ForgeMesh.Funs

/-!
# No axiom in the CDT's translation

`#standard_axioms D₁ D₂ …` fails the build if a named declaration rests on any axiom other
than `propext`, `Classical.choice` and `Quot.sound`. The translated CDT's entry points rest on
none at all beyond those: its external functions (FunsExternal.lean) are definitions or Lean
`opaque` constants, forge-math's `orient2d` is ForgeMath's translation, and the float bridge's
operations are definitions. The theorems about the CDT join this list as they are proved.
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
  forge_mesh.cdt.triangulate_within forge_mesh.cdt.triangulate_as_far forge_mesh.cdt.within
  forge_mesh.cdt.zip
