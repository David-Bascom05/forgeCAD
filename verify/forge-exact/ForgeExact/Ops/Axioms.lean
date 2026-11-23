import ForgeExact.Axioms
import ForgeExact.Ops

/-!
# The ops spec: only the standard axioms

Every main theorem of `ForgeExact.Ops`, checked by `#standard_axioms` (ForgeExact/Axioms.lean):
the build fails on `sorryAx` or any axiom past `propext`, `Classical.choice` and `Quot.sound`.
-/

#standard_axioms
  ForgeExact.Ops.interior_img
  ForgeExact.Ops.closure_img
  ForgeExact.Ops.reg_img
  ForgeExact.Ops.solid_img
  ForgeExact.Ops.reg_diff_reg
  ForgeExact.Ops.reg_union_reg
  ForgeExact.Ops.full_diff
  ForgeExact.Ops.steps_eq
  ForgeExact.Ops.cuts_perm
  ForgeExact.Ops.adds_perm
  ForgeExact.Ops.pattern_cut
  ForgeExact.Ops.pattern_add
  ForgeExact.Ops.bigUnion_dedup
  ForgeExact.Ops.no_target_iff
  ForgeExact.Ops.wall_mem_iff
  ForgeExact.Ops.thru_finite
  ForgeExact.Ops.thru_enclosed
  ForgeExact.Ops.profile_wf
  ForgeExact.Ops.Chamfer.contacts_on_plane
  ForgeExact.Ops.Chamfer.contact_dist
  ForgeExact.Ops.Chamfer.spring_level
  ForgeExact.Ops.Chamfer.spring_dist
  ForgeExact.Ops.Chamfer.q_lt_of_past_chord
  ForgeExact.Ops.Chamfer.fill_sub_chamfer
  ForgeExact.Ops.Draft.h_sq
  ForgeExact.Ops.Draft.h_zero_iff
  ForgeExact.Ops.Draft.Drafted.on_neutral_line
  ForgeExact.Ops.Draft.Drafted.drafted_angle
  ForgeExact.Ops.Draft.beta_ne_zero
