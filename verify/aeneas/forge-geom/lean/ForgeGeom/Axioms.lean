/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Lean
import ForgeGeom.Funs

/-!
# No axiom in forge-geom's translation

`#standard_axioms D₁ D₂ …` fails the build if a named declaration rests on any axiom other than
`propext`, `Classical.choice` and `Quot.sound`. The opaque items (FunsExternal.lean,
TypesExternal.lean) are definitions or Lean `opaque` constants; forge-math's are its translation.
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
  forge_geom.intersect.plane_surface forge_geom.intersect.plane_plane
  forge_geom.intersect.planes_coincident
