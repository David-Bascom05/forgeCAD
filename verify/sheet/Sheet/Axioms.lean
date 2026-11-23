import Lean
import Sheet.Fold
import Sheet.Rules
import Sheet.Tree
import Sheet.Classes
import Sheet.Region
import Sheet.BranchBound
import Sheet.Build
import Sheet.Float
import Sheet.Exact
import Sheet.Winding
import Sheet.Filter16
import Sheet.Trig
import Sheet.Checker
import Sheet.Oracle

/-!
# Only the standard axioms

`#standard_axioms T₁ T₂ …` fails the build if a named theorem rests on any axiom other than
`propext`, `Classical.choice` and `Quot.sound` (a `sorry` is the axiom `sorryAx`, and fails too;
`native_decide` is `Lean.ofReduceBool`, and fails too). Every main theorem is listed, and this
module is in the default build.
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
  -- the fold of one bend
  Sheet.Bend.s_eq_tan Sheet.Bend.inner_radius Sheet.Bend.neutral_radius Sheet.Bend.wn_mem_iff
  Sheet.Bend.rn_mem_iff Sheet.Bend.φ_BA Sheet.Bend.zone_zero Sheet.Bend.zone_BA
  Sheet.Bend.hasDerivAt_zone_u Sheet.Bend.hasDerivAt_zone_y Sheet.Bend.metricE_eq Sheet.Bend.iso
  Sheet.Bend.neutral_length Sheet.Bend.dir_zero Sheet.Bend.dir_BA Sheet.Bend.deduction
  Sheet.Bend.F_glue Sheet.Bend.refold_b Sheet.Bend.refold_a Sheet.Bend.zone_BA_hem Sheet.Bend.Jat_eq_J
  -- the rules
  Sheet.kOfBA_baOfK Sheet.baOfK_kOfBA Sheet.BA_from_rule Sheet.BA_from_rule_physical
  Sheet.interp_affine Sheet.ktable_physical Sheet.allowance_physical
  Sheet.allowance_physical_bend Sheet.allowance_unphysical Sheet.s_eq_setback
  Sheet.flat_length_consistent Sheet.deduction_is_bd Sheet.ossb_90 Sheet.bd90
  Sheet.outsideReach_eq
  -- the tree
  Sheet.refold_place Sheet.glue_a Sheet.glue_b Sheet.refold
  -- classes and regions
  Sheet.Classes.cls_spec Sheet.Classes.rep_order Sheet.Classes.rep_class Sheet.Classes.rep_unique
  Sheet.Classes.conn_map
  Sheet.Region.within_iff_dist Sheet.Region.carrier_symm Sheet.Region.carrier_reverse
  Sheet.Region.carrier_spec Sheet.Region.vrep_order Sheet.Region.vrep_class
  Sheet.Region.conn_move Sheet.Region.spreadOk_spec Sheet.Region.box_spread
  Sheet.Region.inPoly_rotate Sheet.Region.crosses_reverse Sheet.Region.inPoly_translate
  Sheet.Region.side_swap Sheet.Region.side_ne Sheet.Region.sign_ev Sheet.Region.side_move
  Sheet.Region.disc_move Sheet.Region.closes_of_classes Sheet.Exact.esum_prod Sheet.Exact.filtered_eq
  Sheet.Exact.near_exact Sheet.Exact.disc_exact Sheet.Exact.within_exact Sheet.Exact.esum_n Sheet.Float.filterLt_sound Sheet.Winding.side_pos_iff
  Sheet.Winding.crosses_iff Sheet.Winding.edges_sides Sheet.Winding.inPoly_wind
  Sheet.Winding.inPoly_simple Sheet.Filter16.disc_filter_sound
  Sheet.Filter16.line_gap_filter_sound Sheet.Filter16.rel_mul Sheet.Filter16.rel_add Sheet.Build.oneSegment_iff
  Sheet.Region.inLoop_lines Sheet.Region.owner_unique Sheet.Region.owner_spec
  -- the bend simulator's branch and bound
  Sheet.BB.Encl.comp Sheet.BB.Ival.addOut_mem Sheet.BB.lipschitz_lower Sheet.BB.search_sound
  Sheet.BB.certify_sound
  -- L3: the kernel's joint decisions
  Sheet.Build.decide_ok_iff Sheet.Build.setback_pos_iff Sheet.Build.toBend_physical
  Sheet.Build.accepted_bend Sheet.Build.build_refold
  -- L5: the float filter of the vertex-class predicate
  Sheet.Float.sub_bound Sheet.Float.sq_sum_bounds Sheet.Float.filter_sound
  Sheet.Float.near_filter_sound Sheet.Float.prod_bound Sheet.Float.orient_err
  Sheet.Float.within_filter_sound
  -- enclosures and the checker
  Sheet.Trig.mem_add Sheet.Trig.mem_sub Sheet.Trig.mem_mul Sheet.Trig.mem_coarsen Sheet.Trig.taylor
  Sheet.Trig.taylorI_mem Sheet.Trig.double_mem Sheet.Trig.sinCos_mem Sheet.Trig.sc_mem
  Sheet.Checker.all_spec Sheet.Checker.near_sound Sheet.Checker.dist2I_mem Sheet.Checker.Zone.atI_mem
  Sheet.Checker.Zone.at_mem Sheet.Checker.kappa_sound Sheet.Checker.decide2_pass
  Sheet.Checker.setback_sound Sheet.Checker.refoldVertex_sound
