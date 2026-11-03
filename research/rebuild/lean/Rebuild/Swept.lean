/-
Where can two values of a drag differ? Only where the part's boundary passed. This is why a box
around where each moved thing was and is covers everything that changed, and what that needs.
-/
import Mathlib.Topology.Order.IntermediateValue
import Mathlib.Topology.Instances.Real.Lemmas

namespace Rebuild
open Set

variable {α : Type*} [TopologicalSpace α]

/-- A drag at a point: for each value of the parameter the part `B p`. Continuous at `x`: a point
strictly inside stays so for near values, and likewise a point strictly outside. Nothing else is
assumed: no smoothness, no monotony, no shape of the part. -/
structure ContinuousAt' (B : ℝ → Set α) (x : α) : Prop where
  inside : IsOpen {p | x ∈ interior (B p)}
  outside : IsOpen {p | x ∉ closure (B p)}

/-- If `x` is in the part at one value and not at another, at some value between it was on the
boundary. So what differs between two values lies in what the boundary swept. Hypothesis needed:
continuity at `x`. It fails on jumps: a selector picking other edges, a refused value skipped; then
nothing is swept, and the code falls back to one box around everything that changed. -/
theorem swept {B : ℝ → Set α} {x : α} (hc : ContinuousAt' B x) {p q : ℝ} (hpq : p ≤ q)
    (hp : x ∈ interior (B p)) (hq : x ∉ closure (B q)) :
    ∃ r ∈ Icc p q, x ∈ frontier (B r) := by
  by_contra hno
  push_neg at hno
  have hcover : Icc p q ⊆ {r | x ∈ interior (B r)} ∪ {r | x ∉ closure (B r)} := by
    intro r hr
    by_contra hn
    simp only [mem_union, mem_setOf_eq, not_or, not_not] at hn
    exact hno r hr ⟨hn.2, hn.1⟩
  have hconn : IsPreconnected (Icc p q) := isPreconnected_Icc
  have hne1 : (Icc p q ∩ {r | x ∈ interior (B r)}).Nonempty := ⟨p, ⟨le_refl _, hpq⟩, hp⟩
  have hne2 : (Icc p q ∩ {r | x ∉ closure (B r)}).Nonempty := ⟨q, ⟨hpq, le_refl _⟩, hq⟩
  obtain ⟨r, _, h1, h2⟩ := hconn _ _ hc.inside hc.outside hcover hne1 hne2
  exact h2 (subset_closure (interior_subset h1))

end Rebuild
