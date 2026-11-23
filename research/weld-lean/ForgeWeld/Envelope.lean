import ForgeWeld.Node

/-!
# The frame's envelope, and its members a disjoint decomposition of it

The envelope `E` of the members at a node, as the rule intends it (stated explicitly, not the
uncut prisms' union): each member's uncut prism restricted by the half-spaces the rule gives it
(`cutsOf`: the miter's or butt's plane at the node, each gap kept back on its member's own side,
and a butt's run-on to the other's far side), and by its cuts elsewhere (`more k`). So the gap
slabs are left out of `E` and a run-on is in it, by definition.

* `union_eq_envelope`: a point is in `E` iff it is in some member.
* `envelope_disjoint`: every two members meet at most on a plane (nowhere dense), so the members
  are `E` cut into pieces with disjoint interiors: the regularised union is `E` itself.
-/

namespace ForgeWeld

open ForgeExact ForgeExact.Ref ForgeExact.Topology

/-- The envelope of members `0 .. n-1` at a node: their prisms, each restricted by the rule's
half-spaces and its other cuts. -/
def Envelope (n : Nat) (e : Nat → Entry) (corner : Corner) (p : V3) (g : Rat)
    (more : Nat → List (V3 × Rat)) : Pred V3 :=
  fun x => ∃ k, k < n ∧ Prism (e k).F (e k).S x ∧
    ∀ c ∈ cutsOf n e corner p g k ++ more k, Half c.1 c.2 x

/-- Member `k` as the rule makes it. -/
def NodeMember (n : Nat) (e : Nat → Entry) (corner : Corner) (p : V3) (g : Rat)
    (more : Nat → List (V3 × Rat)) (k : Nat) : Pred V3 :=
  Member (e k).F (e k).S (cutsOf n e corner p g k ++ more k)

theorem union_eq_envelope (n : Nat) (e : Nat → Entry) (corner : Corner) (p : V3) (g : Rat)
    (more : Nat → List (V3 × Rat)) (x : V3) :
    Envelope n e corner p g more x ↔ ∃ k, k < n ∧ NodeMember n e corner p g more k x :=
  ⟨fun ⟨k, hk, hp, hc⟩ => ⟨k, hk, hp, hc⟩, fun ⟨k, hk, hp, hc⟩ => ⟨k, hk, hp, hc⟩⟩

/-- Every two members of the envelope meet at most on one plane: nowhere dense, given the plane's
normal is not zero (the butt's `w` not zero is the refusal `ButtParallel`; the miter's `m` not
zero is `MiterTooSharp`). -/
theorem envelope_disjoint {n : Nat} {e : Nat → Entry} {corner : Corner} {p : V3} {g : Rat}
    (hg : 0 ≤ g) (more : Nat → List (V3 × Rat)) {i j : Nat} (hij : i < j) (hjn : j < n) :
    ∃ w c, ((NodeMember n e corner p g more i).inter (NodeMember n e corner p g more j)).Sub
      (Flat.wall [⟨w, c⟩]) := by
  obtain ⟨w, c, h | h⟩ := node_apart (e := e) (corner := corner) (p := p) hg
    (cs := fun k => cutsOf n e corner p g k ++ more k)
    (fun k => List.subset_append_left _ _) hij hjn
  · exact ⟨w, c, apart_inter_wall h⟩
  · exact ⟨w, c, fun x ⟨hi, hj⟩ => apart_inter_wall h x ⟨hj, hi⟩⟩

theorem envelope_disjoint_nowhereDense {n : Nat} {e : Nat → Entry} {corner : Corner} {p : V3}
    {g : Rat} (more : Nat → List (V3 × Rat)) {i j : Nat} {w : V3} {c : Rat} (hw : w ≠ ⟨0, 0, 0⟩)
    (h : ((NodeMember n e corner p g more i).inter (NodeMember n e corner p g more j)).Sub
      (Flat.wall [⟨w, c⟩])) :
    NowhereDense ((NodeMember n e corner p g more i).inter (NodeMember n e corner p g more j)) :=
  nowhereDense_mono
    (Flat.wall_nowhereDense [⟨w, c⟩] fun q hq => by rw [List.mem_singleton.mp hq]; exact hw) h

end ForgeWeld
