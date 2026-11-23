import Lean
import ForgeSheetCore.RefineClasses
import ForgeSheetCore.RefineOwner
import ForgeSheetCore.RefineChain
import ForgeSheetCore.RefinePlace
import ForgeSheetCore.DevelopErr
import ForgeSheetCore.JointErr
import ForgeSheetCore.RefineJoint
import ForgeSheetCore.SetbackErr
import ForgeSheetCore.RefineSetback
import ForgeSheetCore.ZoneErr
import ForgeSheetCore.RefineZone
import ForgeSheetCore.RefineExact
import ForgeSheetCore.ExactModel
import ForgeSheetCore.RefineDisc
import ForgeSheetCore.ExpansionInst
import ForgeSheetCore.RefineLineGap
import ForgeSheetCore.LineGapExact

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
  ForgeSheetCore.Setback.den_pos ForgeSheetCore.Setback.setback_err
  forge_sheet_core.round_fin_q forge_sheet_core.div_err forge_sheet_core.cast_err
  forge_sheet_core.setback_spec
  ForgeSheetCore.Zone.mul_approx ForgeSheetCore.Zone.add_approx ForgeSheetCore.Zone.trig_approx
  forge_sheet_core.coord_err forge_sheet_core.ptw_spec forge_sheet_core.phi_spec
  forge_sheet_core.round_local_spec forge_sheet_core.zone_spec
  forge_sheet_core.two_sum_eq forge_sheet_core.grow_spec forge_sheet_core.two_product_eq
  forge_sheet_core.exp_of_spec forge_sheet_core.exp_diff_spec forge_sheet_core.exp_neg_spec
  forge_sheet_core.exp_add_spec forge_sheet_core.exp_mul_spec forge_sheet_core.exp_sign_spec
  forge_sheet_core.round_mult forge_sheet_core.rel_mult forge_sheet_core.twoSum_good
  forge_sheet_core.fold_grow_good forge_sheet_core.twoProduct_good forge_sheet_core.growL_good
  forge_sheet_core.growMany_good forge_sheet_core.mulL_eq forge_sheet_core.prodTerms_good
  forge_sheet_core.diff_facts forge_sheet_core.mul_facts forge_sheet_core.add_facts
  forge_sheet_core.disc_model forge_sheet_core.FILTER_eq forge_sheet_core.disc_filter
  forge_sheet_core.disc_spec
  IEEE754.NO_sum IEEE754.NO_sublist IEEE754.NO_sign IEEE754.two_sum_facts IEEE754.grow_step
  IEEE754.grow_chain_NO IEEE754.grow_chain_sum IEEE754.grow_nonoverlapping
  forge_sheet_core.growL_eq forge_sheet_core.chain_ok forge_sheet_core.signL_exp
  forge_sheet_core.grow_exp forge_sheet_core.disc_exact
  forge_sheet_core.line_gap_filter forge_sheet_core.add_gen forge_sheet_core.mul_gen
  forge_sheet_core.line_model forge_sheet_core.line_gap_spec forge_sheet_core.line_gap_exact
