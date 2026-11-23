/-
Soft jaws (design.md §7b). A jaw is its blank less the pocket: the held body H grown by the
clearance k, cut to the grip slab (the depths down to grip_depth). In a setup that uses the jaws
they are the fixture F, and the corrected G2 applies to them unchanged (ForgeCam/Prism.lean,
`G2_of_closedForm`). Cutting the jaws is an ordinary setup (the blank as stock, the vise as F),
so G1–G5 cover it with no new theorem.
-/
import ForgeCam.Spec

open Metric Set

namespace ForgeCam

/-- A soft jaw: the blank less (H ⊕ B_k) within the grip slab. -/
noncomputable def jaw (blank H slab : Set E3) (k : ℝ) : Set E3 := blank \ (cthickening k H ∩ slab)

/-- **Seating.** The held body meets no point of the jaw (so no point of its interior),
provided H meets the blank only within the grip slab: the pocket is no shallower than the part
sits. Any clearance k will do, k ≥ 0 included (H ⊆ H ⊕ B_k; for k < 0 Mathlib's cthickening is
the closure). Without the proviso it fails (`jaw_seat_needs_slab`): a part reaching below the
grip depth runs into the jaw's floor. -/
theorem jaw_seat {blank H slab : Set E3} {k : ℝ} (hslab : H ∩ blank ⊆ slab) :
    Disjoint H (jaw blank H slab k) :=
  Set.disjoint_left.mpr fun _ hx ⟨hb, hn⟩ =>
    hn ⟨self_subset_cthickening H hx, hslab ⟨hx, hb⟩⟩

theorem jaw_seat_interior {blank H slab : Set E3} {k : ℝ} (hslab : H ∩ blank ⊆ slab) :
    Disjoint H (interior (jaw blank H slab k)) :=
  (jaw_seat hslab).mono_right interior_subset

/-- The proviso is needed: a body reaching outside the slab inside the blank is not seated. -/
theorem jaw_seat_needs_slab {blank H slab : Set E3} {k : ℝ} {x : E3} (hx : x ∈ H)
    (hb : x ∈ blank) (hs : x ∉ slab) : ¬ Disjoint H (jaw blank H slab k) :=
  Set.not_disjoint_iff.mpr ⟨x, hx, hb, fun h => hs h.2⟩

end ForgeCam
