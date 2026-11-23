import Lean
import ForgeSheetCore.RefineClasses
import ForgeSheetCore.RefineOwner
import ForgeSheetCore.RefineChain
import ForgeSheetCore.RefinePlace
import ForgeSheetCore.DevelopErr
import ForgeSheetCore.JointErr
import ForgeSheetCore.RefineJoint

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
  forge_sheet_core.fo_block forge_sheet_core.in_block forge_sheet_core.loop0_chain_spec
  forge_sheet_core.unchained_block forge_sheet_core.pinch_block forge_sheet_core.loop1_chain_spec
  ForgeSheetCore.Walk.walk_closes ForgeSheetCore.Walk.closed_add forge_sheet_core.succ_perm
  forge_sheet_core.walk_spec forge_sheet_core.walks_spec forge_sheet_core.chain_spec
  forge_sheet_core.round_le forge_sheet_core.add_err forge_sheet_core.mul_err forge_sheet_core.sub_err
  forge_sheet_core.three_round forge_sheet_core.two_round forge_sheet_core.apply_spec
  forge_sheet_core.vec_spec forge_sheet_core.then_spec forge_sheet_core.diff_prod
  forge_sheet_core.two_round' forge_sheet_core.local_spec forge_sheet_core.inverse_spec
  ForgeSheetCore.Develop.develop_err
  ForgeSheetCore.Joint.norm_err ForgeSheetCore.Joint.len_err ForgeSheetCore.Joint.axis_err
  ForgeSheetCore.Joint.joint_err forge_sheet_core.round_nz forge_sheet_core.sub_nz
  forge_sheet_core.add_nz forge_sheet_core.mul_nz forge_sheet_core.div_nz forge_sheet_core.sqrt_nz
  forge_sheet_core.one_val forge_sheet_core.joint_real forge_sheet_core.joint_spec
