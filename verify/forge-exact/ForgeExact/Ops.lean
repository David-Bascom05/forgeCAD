import ForgeExact.Ops.Pattern
import ForgeExact.Ops.Hole
import ForgeExact.Ops.Chamfer
import ForgeExact.Ops.Draft
import ForgeExact.Ops.Shell
import ForgeExact.Ops.DraftCone

/-!
# The specification of shell, draft, hole, rib, chamfer and pattern

research/ops-spec/design.md. Pattern (P1–P3), hole (H1, H3, H4), chamfer on a plane pair (C1, C2)
planar draft (D1–D3), drafted cylinders as frusta, and the shell at a convex edge (S1) so far; `Ops/Axioms.lean` checks the axioms.
-/
