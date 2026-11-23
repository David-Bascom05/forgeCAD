/-
Soft jaws (design.md §7b). A jaw is its blank less the pocket: the held body H grown by the
clearance k, cut to the grip slab (the depths down to grip_depth). In a setup that uses the jaws
they are the fixture F, and the corrected G2 applies to them unchanged (ForgeCam/Vise.lean,
`G2open_of_closedForm`). Cutting the jaws is an ordinary setup (the blank as stock, the vise as F),
so G1–G5 cover it with no new theorem.
-/
import ForgeCam.Prism

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

/-! ### The soft jaws as forge-cam cuts them (fixture.rs `soft_jaws`, 5b32f0a)

The pocket is a prism: the region M (M = (G ⊖ r) ⊕ (r + fit), plus reliefs) cut from the jaws'
top down to the seat. The held body H is the material the earlier setup left, turned over. The
band [seat, top] is split at every flat face height of the part and every stock level, so within
each sub-band H is a prism over one section G k. forge-cam refuses the jaws unless every G k ⊆ M
(exact seating, `DoesNotSeat`). -/

/-- The jaws: the blank less the pocket M × [seat, top]. -/
def softJaw (blank : Set E3) (M : Set (ℝ × ℝ)) (seat top : ℝ) : Set E3 :=
  blank \ prism M (Icc seat top)

/-- **Seating, for forge-cam's soft jaws.** Within the grip, H is a stack of prisms over the
sections G k at heights [z k, z (k+1)] inside [seat, top], each section inside the pocket's
region M. If H meets the blank only within the grip heights, H meets no point of the jaws. -/
theorem softJaw_seat {blank H : Set E3} {M : Set (ℝ × ℝ)} {seat top : ℝ} {n : ℕ}
    (z : ℕ → ℝ) (G : ℕ → Set (ℝ × ℝ))
    (hstack : ∀ p ∈ H, seat ≤ p 2 → p 2 ≤ top →
      ∃ k < n, p ∈ prism (G k) (Icc (z k) (z (k + 1))))
    (hband : ∀ k < n, seat ≤ z k ∧ z (k + 1) ≤ top)
    (hseat : ∀ k < n, G k ⊆ M)
    (hgrip : ∀ p ∈ H, p ∈ blank → seat ≤ p 2 ∧ p 2 ≤ top) :
    Disjoint H (softJaw blank M seat top) := by
  refine Set.disjoint_left.mpr fun p hp ⟨hb, hn⟩ => hn ?_
  obtain ⟨h0, h1⟩ := hgrip p hp hb
  obtain ⟨k, hk, hxy, hz⟩ := hstack p hp h0 h1
  exact ⟨hseat k hk hxy, (hband k hk).1.trans hz.1, hz.2.trans (hband k hk).2⟩

theorem softJaw_seat_interior {blank H : Set E3} {M : Set (ℝ × ℝ)} {seat top : ℝ} {n : ℕ}
    (z : ℕ → ℝ) (G : ℕ → Set (ℝ × ℝ))
    (hstack : ∀ p ∈ H, seat ≤ p 2 → p 2 ≤ top →
      ∃ k < n, p ∈ prism (G k) (Icc (z k) (z (k + 1))))
    (hband : ∀ k < n, seat ≤ z k ∧ z (k + 1) ≤ top)
    (hseat : ∀ k < n, G k ⊆ M)
    (hgrip : ∀ p ∈ H, p ∈ blank → seat ≤ p 2 ∧ p 2 ≤ top) :
    Disjoint H (interior (softJaw blank M seat top)) :=
  (softJaw_seat z G hstack hband hseat hgrip).mono_right interior_subset

/-- The grip proviso is needed here too: held material inside the blank below the seat (top
stock the earlier setup did not face off, turned over) is not seated. -/
theorem softJaw_needs_grip {blank H : Set E3} {M : Set (ℝ × ℝ)} {seat top : ℝ} {p : E3}
    (hp : p ∈ H) (hb : p ∈ blank) (hlow : p 2 < seat) :
    ¬ Disjoint H (softJaw blank M seat top) :=
  Set.not_disjoint_iff.mpr ⟨p, hp, hb, fun h => by linarith [h.2.1]⟩

/-- The two models agree: forge-cam's pocket region holds the held body's sections grown by
nothing more than M needs. If H's grip part is inside M × [seat, top], it is in the earlier model's
(H ⊕ B_k) ∩ slab for any k (`jaw_seat`). -/
theorem softJaw_subset_jaw {blank H : Set E3} {M : Set (ℝ × ℝ)} {seat top k : ℝ}
    (hM : prism M (Icc seat top) ⊇ cthickening k H ∩ {p | seat ≤ p 2 ∧ p 2 ≤ top}) :
    softJaw blank M seat top ⊆ jaw blank H {p | seat ≤ p 2 ∧ p 2 ≤ top} k :=
  fun p ⟨hb, hn⟩ => ⟨hb, fun h => hn (hM h)⟩

end ForgeCam
