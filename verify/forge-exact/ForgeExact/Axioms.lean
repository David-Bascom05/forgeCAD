import Lean
import ForgeExact.Regular
import ForgeExact.Flat
import ForgeExact.SnapSym
import ForgeExact.RefFacts
import ForgeExact.DecideOnce.Ledger
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
import ForgeExact.DecideOnce.Curved
import ForgeExact.DecideOnce.CorrectK
import ForgeExact.DecideOnce.LocalK
import ForgeExact.DecideOnce.QForm
import ForgeExact.DecideOnce.FlatK
import ForgeExact.DecideOnce.Classes
import ForgeExact.DecideOnce.QLine
import ForgeExact.DecideOnce.QSep
import ForgeExact.DecideOnce.QEdge
import ForgeExact.DecideOnce.QVert
import ForgeExact.DecideOnce.RootCover
import ForgeExact.DecideOnce.QPropose
import ForgeExact.DecideOnce.SampleLocal
import ForgeExact.JordanLocal
import ForgeExact.DecideOnce.LocalJordan
import ForgeExact.DecideOnce.LocalFinal
import ForgeExact.Alg.Sum
import ForgeExact.Quad.Cert
import ForgeExact.QRef.Octree
import ForgeExact.Poly3.Octree

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
  ForgeExact.DecideOnce.Curved.covCellK_flat
  ForgeExact.DecideOnce.Curved.ExactK.consistent
  ForgeExact.DecideOnce.Curved.correct_k
  ForgeExact.DecideOnce.Curved.correct_ok_k
  ForgeExact.DecideOnce.Curved.zeroThickness_iff_k
  ForgeExact.DecideOnce.Curved.qform_compat
  ForgeExact.DecideOnce.Curved.qform_localCompat
  ForgeExact.DecideOnce.Curved.den_sep
  ForgeExact.DecideOnce.Curved.sep_invariant_reg
  ForgeExact.DecideOnce.Curved.qform_localCompatK
  ForgeExact.DecideOnce.Curved.ExactInK.consistent
  ForgeExact.DecideOnce.Curved.result_iff_in_k
  ForgeExact.DecideOnce.Curved.reg_in_k
  ForgeExact.DecideOnce.Curved.zeroThickness_iff_in_k
  ForgeExact.DecideOnce.Curved.correct_ok_in_k
  ForgeExact.DecideOnce.Curved.vert_unique
  ForgeExact.DecideOnce.Curved.line_in_cell
  ForgeExact.DecideOnce.Curved.isVertK_iff
  ForgeExact.DecideOnce.Curved.isEdgeK_iff
  ForgeExact.DecideOnce.Curved.exactK_of_exact
  ForgeExact.DecideOnce.Curved.flat_correct_k
  ForgeExact.DecideOnce.Curved.noScaled_classesOf
  ForgeExact.DecideOnce.Curved.scaled_classesOf
  ForgeExact.DecideOnce.Curved.flat_correct_classes
  ForgeExact.DecideOnce.Curved.alongLine_ev
  ForgeExact.DecideOnce.Curved.covOn_spec
  ForgeExact.DecideOnce.Curved.axisPlane_on
  ForgeExact.DecideOnce.Curved.axisPlane_nz
  ForgeExact.DecideOnce.Curved.nappePlane_on
  ForgeExact.DecideOnce.Curved.noScaled_seams
  ForgeExact.DecideOnce.Curved.homog_ev
  ForgeExact.DecideOnce.Curved.onCurve_spec
  ForgeExact.DecideOnce.Curved.edgeCov_spec
  ForgeExact.DecideOnce.Curved.restrB_ev
  ForgeExact.DecideOnce.Curved.bezout_spec
  ForgeExact.DecideOnce.Curved.homog3_ev
  ForgeExact.DecideOnce.Curved.onVert_spec
  ForgeExact.DecideOnce.Curved.vertCov_spec
  ForgeExact.DecideOnce.Curved.restrT_ev
  ForgeExact.DecideOnce.Curved.bezout3_spec
  ForgeExact.DecideOnce.Curved.gaps_complete
  ForgeExact.DecideOnce.Curved.cover_complete
  ForgeExact.DecideOnce.Curved.sqfree_roots
  ForgeExact.DecideOnce.Curved.rootsOf_complete
  ForgeExact.DecideOnce.Curved.lin_ev
  ForgeExact.DecideOnce.Curved.proposeEdge_complete
  ForgeExact.DecideOnce.Curved.shear_ev
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
  ForgeExact.Alg.noPosRoot_spec
  ForgeExact.Alg.onePosRoot_spec
  ForgeExact.Alg.moebius_unique
  ForgeExact.Alg.moebius_none
  ForgeExact.Alg.Iso.denotes_of_ok
  ForgeExact.Alg.Denotes.unique
  ForgeExact.Alg.cmpRat_spec
  ForgeExact.Alg.bisect_spec
  ForgeExact.Alg.sign_spec
  ForgeExact.Alg.cmp_spec
  ForgeExact.Alg.neg_spec
  ForgeExact.Alg.addRat_spec
  ForgeExact.Alg.mulPos_spec
  ForgeExact.Alg.ev_redCert
  ForgeExact.Alg.add_spec
  ForgeExact.Alg.mul_spec
  ForgeExact.Quad.box_bound
  ForgeExact.Quad.sign_nbhd
  ForgeExact.Quad.escape
  ForgeExact.Quad.wall_nowhereDense
  ForgeExact.Quad.QForm.den_iff_onSigns
  ForgeExact.Quad.quad_boolean
  ForgeExact.Quad.member_off_wall
  ForgeExact.Quad.member_box
  ForgeExact.Quad.pairCheck_spec
  ForgeExact.Quad.empty_of_patterns
  ForgeExact.QRef.BF.eval3_sound
  ForgeExact.QRef.BF.allFalse_sound
  ForgeExact.QRef.reg_off_wall
  ForgeExact.QRef.in_box
  ForgeExact.QRef.out_box
  ForgeExact.QRef.empty_of_leaves
  ForgeExact.Poly3.zero_of_roots
  ForgeExact.Poly3.escape_one
  ForgeExact.Poly3.box_bound
  ForgeExact.Poly3.sign_nbhd
  ForgeExact.Poly3.escape
  ForgeExact.Poly3.wall_nowhereDense
  ForgeExact.Poly3.reg_off_wall
  ForgeExact.Poly3.in_box
  ForgeExact.Poly3.out_box
  ForgeExact.Poly3.empty_of_leaves
  ForgeExact.Poly3.eval_about
  ForgeExact.Poly3.about_bound
