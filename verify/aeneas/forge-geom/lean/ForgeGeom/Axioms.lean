/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Lean
import ForgeGeom.Funs
import ForgeGeom.Refine
import ForgeGeom.Thomas
import ForgeGeom.Insert

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

-- the spline's numerics (research/spline-spec): the theorems and the float bounds they rest on
#standard_axioms
  ForgeGeom.lerp_eq ForgeGeom.lerp_spec ForgeGeom.inner_spec ForgeGeom.outer_spec
  ForgeGeom.decasteljau_spec ForgeGeom.arr_ext16 ForgeGeom.levelArr_of
  ForgeGeom.vset_get ForgeGeom.sweep_spec ForgeGeom.back_spec ForgeGeom.tridiagonal_spec
  ForgeGeom.count_spec ForgeGeom.lerps_spec ForgeGeom.clone_H ForgeGeom.efs_spec
  ForgeGeom.insert_knot_spec ForgeGeom.insQ_get
  SplineFP.lerp_err_M SplineFP.dc_step
