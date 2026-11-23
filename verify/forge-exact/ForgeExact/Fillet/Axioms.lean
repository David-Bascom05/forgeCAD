import ForgeExact.Axioms
import ForgeExact.Fillet.RevCorner
import ForgeExact.Fillet.Conic
import ForgeExact.Fillet.Overflow
import ForgeExact.Fillet.LocalBall

/-!
# The fillet spec: only the standard axioms

Every main theorem of `ForgeExact.Fillet`, checked by `#standard_axioms` (ForgeExact/Axioms.lean):
the build fails on `sorryAx` or any axiom past `propext`, `Classical.choice` and `Quot.sound`.
-/

#standard_axioms
  ForgeExact.Fillet.closing_extensive
  ForgeExact.Fillet.closing_mono
  ForgeExact.Fillet.closing_idem
  ForgeExact.Fillet.opening_sub
  ForgeExact.Fillet.cut_eq_fill
  ForgeExact.Fillet.closing_local
  ForgeExact.Fillet.PlanePair.free_iff
  ForgeExact.Fillet.PlanePair.closing_iff
  ForgeExact.Fillet.PlanePair.q_eq_cyl
  ForgeExact.Fillet.PlanePair.spring₁
  ForgeExact.Fillet.PlanePair.g1_spring₁
  ForgeExact.Fillet.PlanePair.near_crease
  ForgeExact.Fillet.free_rev
  ForgeExact.Fillet.closing_rev
  ForgeExact.Fillet.boss_closing
  ForgeExact.Fillet.hole_closing
  ForgeExact.Fillet.pivot_sphere
  ForgeExact.Fillet.hole_too_large
  ForgeExact.Fillet.Conic.conic_circle
  ForgeExact.Fillet.Conic.conic_tangent
  ForgeExact.Fillet.quadNonneg_spec
  ForgeExact.Fillet.clear_sphere
  ForgeExact.Fillet.Pick.local_eq_closing
  ForgeExact.Fillet.Pick.fillD_eq_local
  ForgeExact.Fillet.Pick.cutE_both
  ForgeExact.Fillet.Pick.cutE_none
  ForgeExact.Fillet.Pick.cutE_one
