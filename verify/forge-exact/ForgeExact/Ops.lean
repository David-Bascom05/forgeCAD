import ForgeExact.Ops.Pattern
import ForgeExact.Ops.Hole
import ForgeExact.Ops.Chamfer
import ForgeExact.Ops.Draft
import ForgeExact.Ops.Shell
import ForgeExact.Ops.DraftCone
import ForgeExact.Ops.ChamferCorner
import ForgeExact.Ops.Rib

/-!
# The specification of shell, draft, hole, rib, chamfer and pattern

research/ops-spec/design.md. Pattern (P1–P3), hole (H1, H3, H4), chamfer on a plane pair (C1, C2) and its corner rule (point or patch)
planar draft (D1–D3), drafted cylinders as frusta, the shell (S1, S2) and the rib's box (B1) so far; `Ops/Axioms.lean` checks the axioms.
-/
