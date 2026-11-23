import ForgeWeld.Member

/-!
# The rule at a node gives every pair of members a separating plane

The members meeting at a node, ranked (forge_weld::frame: members passing through first, then
those named in `rank`, then layout order), are `e 0, e 1, .., e (n-1)`. The rule:

* if the first two both end there (`pair`), they meet by the node's corner: a **miter** cuts both
  on the bisecting plane, each `g` back on its own side; a **butt** cuts `e 1` on the plane touching
  `e 0`'s section, `g` past it (and runs `e 0` on to `e 1`'s far side);
* every other member `e k` is **butted** to each member ranked before it.

`cutsOf` is that rule, cut for cut. `node_apart`: for straight members, every pair at the node
lies on the two sides of one plane, so (Member.lean) the members' interiors are pairwise disjoint.
The one pair the rule cannot cut, two members both passing through the node (the later one would
need a cut where it does not end), is the refusal `CrossingMembers`.

Directions here are not normalised (ℚ has no square roots): the miter's `m = u₀ - u₁` and the butt's
`w` (the part of the butted member's direction square to the other's axis) are used as they are,
and the gaps are offsets in their units. forge-weld normalises them and measures the gap in
millimetres; the planes are the same, the offsets scaled by the vectors' lengths.
-/

namespace ForgeWeld

open ForgeExact ForgeExact.Ref ForgeExact.Topology

/-- A straight member at a node: its frame and section, whether it passes through the node, the
direction from the node into it, and its section's support (`sup w` bounds the section in the
direction `(w · ex, w · ey)`). -/
structure Entry where
  F : Frame
  S : Rat → Rat → Prop
  through : Bool
  u : V3
  sup : V3 → Rat
  upper : ∀ w : V3, Upper S (w.dot F.ex) (w.dot F.ey) (sup w)

/-- The part of `u` square to `d` (scaled by `d · d`, to stay in ℚ without dividing). -/
def perp (u d : V3) : V3 := (d.dot d) * u - (u.dot d) * d

theorem perp_dot (u d : V3) : (perp u d).dot d = 0 := by
  show (V3.sub (V3.smul (d.dot d) u) (V3.smul (u.dot d) d)).dot d = 0
  simp only [V3.dot, V3.sub, V3.smul]; grind

/-- The cut butting `b` to `a`: on the plane touching `a`'s section towards `b`, `g` past it. -/
def buttCut (b a : Entry) (g : Rat) : V3 × Rat :=
  let w := perp b.u a.F.d
  (neg w, -(w.dot a.F.a + a.sup w + g))

/-- The run-on cut of `a` in a butt corner with `b`: flush with `b`'s far side. -/
def runonCut (a b : Entry) : V3 × Rat :=
  let w := perp (neg a.u) b.F.d
  (w, w.dot b.F.a + b.sup w)

inductive Corner where
  | miter
  | butt

/-- Each member's cuts at the node (`k` its rank), by the rule. -/
def cutsOf (n : Nat) (e : Nat → Entry) (corner : Corner) (p : V3) (g : Rat) (k : Nat) :
    List (V3 × Rat) :=
  let pair := 2 ≤ n && !(e 0).through && !(e 1).through
  let m := (e 0).u - (e 1).u
  let corner_cuts : List (V3 × Rat) :=
    if pair then
      match corner with
      | .miter =>
        if k = 0 then [(neg m, -(m.dot p + g))] else if k = 1 then [(m, m.dot p - g)] else []
      | .butt =>
        if k = 0 then [runonCut (e 0) (e 1)] else if k = 1 then [buttCut (e 1) (e 0) g] else []
    else []
  let first := if pair then 2 else 1
  corner_cuts ++ (if first ≤ k then (List.range k).map fun i => buttCut (e k) (e i) g else [])

/-- The rule's butt of `e j` to `e i` is among `e j`'s cuts whenever `j` is past the corner pair. -/
theorem butt_mem {n : Nat} {e : Nat → Entry} {corner : Corner} {p : V3} {g : Rat} {i j : Nat}
    (hij : i < j) (hfirst : (if (2 ≤ n && !(e 0).through && !(e 1).through) then 2 else 1) ≤ j) :
    buttCut (e j) (e i) g ∈ cutsOf n e corner p g j := by
  unfold cutsOf
  simp only
  apply List.mem_append_right
  rw [ite_eq_left_iff.mpr (fun h => absurd hfirst h)]
  exact List.mem_map.mpr ⟨i, List.mem_range.mpr hij, rfl⟩

/-- A butt cut separates: `e i` (straight) on the near side, `e j` on the far. -/
theorem buttCut_apart (a b : Entry) {g : Rat} (hg : 0 ≤ g) {cA cB : List (V3 × Rat)}
    (hB : buttCut b a g ∈ cB) :
    Apart (Member a.F a.S cA) (Member b.F b.S cB) (perp b.u a.F.d)
      ((perp b.u a.F.d).dot a.F.a + a.sup (perp b.u a.F.d)) :=
  butt_apart hg (perp_dot _ _) (a.upper _) hB

/-- **The rule separates every pair.** Members `e 0 .. e (n-1)` at a node, cut by `cutsOf` (and
anything else at their other ends: `cs k` holds `cutsOf .. k`), with `g ≥ 0`: for `i < j < n` the
two lie on the two sides of one plane. (The rule butts `e j` to `e i`; where `e j` passes through
the node it cannot be cut there, and forge-weld refuses the frame: `CrossingMembers`.) -/
theorem node_apart {n : Nat} {e : Nat → Entry} {corner : Corner} {p : V3} {g : Rat} (hg : 0 ≤ g)
    {cs : Nat → List (V3 × Rat)} (hcs : ∀ k, cutsOf n e corner p g k ⊆ cs k)
    {i j : Nat} (hij : i < j) (hjn : j < n) :
    ∃ w c, Apart (Member (e i).F (e i).S (cs i)) (Member (e j).F (e j).S (cs j)) w c ∨
      Apart (Member (e j).F (e j).S (cs j)) (Member (e i).F (e i).S (cs i)) w c := by
  by_cases hpair : (2 ≤ n && !(e 0).through && !(e 1).through) = true
  · by_cases h01 : i = 0 ∧ j = 1
    · obtain ⟨rfl, rfl⟩ := h01
      cases corner with
      | miter =>
        let m := (e 0).u - (e 1).u
        refine ⟨m, m.dot p, Or.inr ?_⟩
        have h0 : (neg m, -(m.dot p + g)) ∈ cs 0 := hcs 0 (by
          unfold cutsOf; simp only [hpair, ite_true]; exact List.mem_append_left _ (by simp; exact ⟨rfl, rfl⟩))
        have h1 : (m, m.dot p - g) ∈ cs 1 := hcs 1 (by
          unfold cutsOf; simp only [hpair, ite_true]; exact List.mem_append_left _ (by simp; exact ⟨rfl, rfl⟩))
        refine ⟨fun x hx => ?_, fun x hx => ?_⟩
        · have := member_half h1 hx; grind
        · have := member_half h0 hx; rw [neg_dot] at this; grind
      | butt =>
        have h1 : buttCut (e 1) (e 0) g ∈ cs 1 := hcs 1 (by
          unfold cutsOf; simp only [hpair, ite_true]; exact List.mem_append_left _ (by simp))
        exact ⟨_, _, Or.inl (buttCut_apart (e 0) (e 1) hg h1)⟩
    · -- past the corner pair: `e j` is butted to `e i`
      have hj2 : 2 ≤ j := by omega
      have hm := butt_mem (n := n) (e := e) (corner := corner) (p := p) (g := g) hij (by split <;> omega)
      exact ⟨_, _, Or.inl (buttCut_apart (e i) (e j) hg (hcs j hm))⟩
  · have hm := butt_mem (n := n) (e := e) (corner := corner) (p := p) (g := g) hij
      (by split <;> first | omega | contradiction)
    exact ⟨_, _, Or.inl (buttCut_apart (e i) (e j) hg (hcs j hm))⟩

end ForgeWeld
