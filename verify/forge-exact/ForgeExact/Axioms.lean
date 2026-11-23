import Lean
import ForgeExact.Regular
import ForgeExact.Flat
import ForgeExact.SnapSym
import ForgeExact.RefFacts
import ForgeExact.DecideOnce.Ledger
import ForgeExact.DecideOnce.CoreInc
import ForgeExact.DecideOnce.Nef
import ForgeExact.DecideOnce.Rungs
import ForgeExact.Jordan
import ForgeExact.DecideOnce.BRep
import ForgeExact.DecideOnce.Witness
import ForgeExact.DecideOnce.BoxesExact
import ForgeExact.DecideOnce.Sample
import ForgeExact.DecideOnce.Sampler
import ForgeExact.DecideOnce.Frontier
import ForgeExact.DecideOnce.Local
import ForgeExact.DecideOnce.SampleLocal
import ForgeExact.JordanLocal
import ForgeExact.DecideOnce.LocalJordan
import ForgeExact.DecideOnce.LocalFinal

/-!
# Only the standard axioms

`#standard_axioms T₁ T₂ …` fails the build if any named theorem rests on an axiom other than
`propext`, `Classical.choice` and `Quot.sound` (`sorry` is the axiom `sorryAx`, so it fails too).
Every main theorem of the package is listed here, and this module is part of the default build.
-/

open Lean Elab Command

elab "#standard_axioms " ids:ident* : command => do
  for id in ids do
    let n ← liftCoreM (realizeGlobalConstNoOverloadWithInfo id)
    let axs ← liftCoreM (collectAxioms n)
    for a in axs do
      unless a == ``propext || a == ``Classical.choice || a == ``Quot.sound do
        throwError "{n} rests on the axiom {a}"
    logInfo m!"{n}: {axs.toList}"

#standard_axioms
  ForgeExact.reg_sub_of_diff_sub
  ForgeExact.reg_eq_of_sdiff
  ForgeExact.reg_kept
  ForgeExact.reg_kept_sampled
  ForgeExact.reg_kept_bool
  ForgeExact.reg_reg
  ForgeExact.Flat.wall_nowhereDense
  ForgeExact.Flat.escape
  ForgeExact.Flat.HForm.den_iff_onSigns
  ForgeExact.Flat.flat_boolean
  ForgeExact.Flat.flat_bool
  ForgeExact.Snap.tolEq_symm
  ForgeExact.Snap.cls_spec
  ForgeExact.Snap.rep_order
  ForgeExact.Snap.rep_class
  ForgeExact.Snap.onIn_class
  ForgeExact.Snap.gap_carried
  ForgeExact.Snap.vertexAt_on
  ForgeExact.Snap.snap_result
  ForgeExact.Snap.tolEq_sym
  ForgeExact.Snap.rep_sym
  ForgeExact.RefFacts.step_keeps_sign
  ForgeExact.RefFacts.step_leaves_plane
  ForgeExact.DecideOnce.cls_spec
  ForgeExact.DecideOnce.head_eq
  ForgeExact.DecideOnce.Table.get?_decideAll
  ForgeExact.DecideOnce.Table.decideAll_once
  ForgeExact.DecideOnce.Decisions.ask_build
  ForgeExact.DecideOnce.Decisions.oneVertex_symm
  ForgeExact.DecideOnce.Decisions.oneVertex_trans
  ForgeExact.DecideOnce.Decisions.oneSurface_symm
  ForgeExact.DecideOnce.Decisions.oneSurface_trans
  ForgeExact.DecideOnce.Decisions.side_of_oneSurface
  ForgeExact.DecideOnce.Decisions.onEdge_of_atVertex
  ForgeExact.DecideOnce.Decisions.on_of_onEdge
  ForgeExact.DecideOnce.Decisions.along_iff
  ForgeExact.DecideOnce.sideIn_class
  ForgeExact.DecideOnce.Ledger.memb_eq
  ForgeExact.DecideOnce.Ledger.splitOk_of
  ForgeExact.DecideOnce.Ledger.classifyOk_of
  ForgeExact.DecideOnce.Ledger.unify_valid
  ForgeExact.DecideOnce.Ledger.coverage
  ForgeExact.DecideOnce.Ledger.no_retry
  ForgeExact.DecideOnce.Exact.consistent
  ForgeExact.DecideOnce.result_iff
  ForgeExact.DecideOnce.correct
  ForgeExact.DecideOnce.correct_ok
  ForgeExact.DecideOnce.zeroThickness_iff
  ForgeExact.DecideOnce.nef_compat
  ForgeExact.DecideOnce.correct_bool
  ForgeExact.DecideOnce.Ledger.consistent_of_check
  ForgeExact.DecideOnce.rung_covered
  ForgeExact.DecideOnce.rung_exact
  ForgeExact.DecideOnce.instances_covered
  ForgeExact.Jordan.Aff.sweep3
  ForgeExact.Jordan.seg_identity
  ForgeExact.Jordan.face_step
  ForgeExact.Jordan.xor_even
  ForgeExact.Jordan.move_rise
  ForgeExact.Jordan.move_level
  ForgeExact.Jordan.crosses_eq
  ForgeExact.Jordan.pierced_eq
  ForgeExact.Jordan.contains_eq
  ForgeExact.Jordan.contains_const
  ForgeExact.Jordan.chi_out
  ForgeExact.Jordan.face_rise
  ForgeExact.Jordan.face_level'
  ForgeExact.Jordan.contains_eq'
  ForgeExact.Jordan.contains_const_local
  ForgeExact.DecideOnce.brep_compat
  ForgeExact.DecideOnce.Ledger.coverageK
  ForgeExact.DecideOnce.Ledger.collapse_local
  ForgeExact.DecideOnce.Ledger.collapse_uniform
  ForgeExact.DecideOnce.Ledger.collapse_refuse
  ForgeExact.DecideOnce.Ledger.collapse_comp
  ForgeExact.DecideOnce.Ledger.collapse_idem
  ForgeExact.DecideOnce.Ledger.collapse_subset
  ForgeExact.DecideOnce.Ledger.coverageM
  ForgeExact.DecideOnce.correct_off_thin
  ForgeExact.DecideOnce.side_point
  ForgeExact.DecideOnce.witness_of_samples
  ForgeExact.DecideOnce.BoxEx.box_compat
  ForgeExact.DecideOnce.BoxEx.box_boundary
  ForgeExact.DecideOnce.BoxEx.box_complete
  ForgeExact.DecideOnce.BoxEx.box_edges
  ForgeExact.DecideOnce.BoxEx.box_verts
  ForgeExact.DecideOnce.BoxEx.box_exact
  ForgeExact.DecideOnce.BoxEx.box_correct
  ForgeExact.DecideOnce.BoxEx.box_refused_iff
  ForgeExact.DecideOnce.Sample.side_cells
  ForgeExact.DecideOnce.Sample.flip_iff
  ForgeExact.DecideOnce.Sample.s_witness
  ForgeExact.DecideOnce.Sample.s_boundary
  ForgeExact.DecideOnce.Sample.s_complete
  ForgeExact.DecideOnce.Sample.s_low
  ForgeExact.DecideOnce.Sample.sample_exact
  ForgeExact.DecideOnce.Sample.samples_of_covers
  ForgeExact.DecideOnce.Sample.sample_correct
  ForgeExact.DecideOnce.Sampler.cand1_ok
  ForgeExact.DecideOnce.Sampler.emb
  ForgeExact.DecideOnce.Sampler.cover2
  ForgeExact.DecideOnce.Sampler.lift_proj
  ForgeExact.DecideOnce.Sampler.coef_ev
  ForgeExact.DecideOnce.Sampler.sampler_covers
  ForgeExact.DecideOnce.Sampler.flat_exact
  ForgeExact.DecideOnce.Sampler.flat_correct
  ForgeExact.DecideOnce.Frontier.reg_local
  ForgeExact.DecideOnce.Frontier.far_unchanged
  ForgeExact.DecideOnce.Frontier.frontier_local
  ForgeExact.DecideOnce.Frontier.split_at_region
  ForgeExact.DecideOnce.result_iff_in
  ForgeExact.DecideOnce.reg_in
  ForgeExact.DecideOnce.zeroThickness_iff_in
  ForgeExact.DecideOnce.correct_ok_in
  ForgeExact.DecideOnce.Sample.local_exact
  ForgeExact.DecideOnce.Sample.local_correct
  ForgeExact.DecideOnce.Sample.wall_region
  ForgeExact.DecideOnce.Sample.wall_up
  ForgeExact.DecideOnce.Sample.local_correct_walls
  ForgeExact.DecideOnce.Sample.local_correct_sampler
  ForgeExact.DecideOnce.covCell_conv
  ForgeExact.DecideOnce.local_compat
  ForgeExact.DecideOnce.Sample.localCompat_of
  ForgeExact.DecideOnce.Sample.local_correct_final
  ForgeExact.DecideOnce.Frontier.stitch_reg
  ForgeExact.DecideOnce.cls_map
  ForgeExact.DecideOnce.Ledger.core_inc
