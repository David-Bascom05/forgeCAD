import Lean
import ForgeSheetCore.RefineClasses
import ForgeSheetCore.RefineOwner

/-! Only the standard axioms: each listed theorem about the translated Rust rests on `propext`,
`Classical.choice` and `Quot.sound` alone (a `sorry` or `native_decide` fails the build). -/

open Lean Elab Command

elab "#standard_axioms " ids:ident* : command => do
  for id in ids do
    let n ← liftCoreM (realizeGlobalConstNoOverloadWithInfo id)
    let axs ← liftCoreM (collectAxioms n)
    for a in axs do
      unless a == ``propext || a == ``Classical.choice || a == ``Quot.sound do
        throwError "{n} rests on the axiom {a}"

#standard_axioms
  forge_sheet_core.find_spec forge_sheet_core.union_spec forge_sheet_core.new_spec
  forge_sheet_core.loop0_spec forge_sheet_core.loop1_spec forge_sheet_core.classes_spec
  forge_sheet_core.classList_iff forge_sheet_core.inner_spec forge_sheet_core.owner_spec
