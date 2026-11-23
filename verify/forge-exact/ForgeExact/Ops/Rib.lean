import ForgeExact.Fillet.Basic

/-!
# Rib: the box that holds it

research/ops-spec/design.md §6. A rib's wall is the region between its path and the part,
thickened across the sketch plane (or, for a web, the thickened path run along the plane's normal
to the part). Every point of it lies on a segment from a point near the path (within the
thickness of the path's plane region) to a point of the part. The kernel's `hole::rib_reach`
(forge-model hole.rs) bounds it by the hull of the part's box and the path's box, grown by the
thickness: this file proves that bound sound.

* `Box3`, `mem`: an axis box over an ordered field.
* `segment_mem`: boxes are convex: a point between two points of a box is in it.
* B1 `rib_in_hull`: if every point of a set lies on a segment from a point of box `A` to a point
  of box `B`, the set lies in every box holding both (their hull, as the kernel takes it).
* `grow_mem`: a point within `t` of a box's point on each axis is in the box grown by `t`.
-/

namespace ForgeExact.Ops

open ForgeExact Fillet Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

namespace Rib

/-- An axis box. -/
structure Box3 (K : Type) where
  lo : V K
  hi : V K

def Box3.mem (b : Box3 K) (p : V K) : Prop :=
  b.lo.x ≤ p.x ∧ p.x ≤ b.hi.x ∧ b.lo.y ≤ p.y ∧ p.y ≤ b.hi.y ∧ b.lo.z ≤ p.z ∧ p.z ≤ b.hi.z

/-- Box `a` lies in box `b`. -/
def Box3.Sub (a b : Box3 K) : Prop := ∀ p, a.mem p → b.mem p

/-- The box grown by `t` on every side. -/
def Box3.grow (b : Box3 K) (t : K) : Box3 K :=
  ⟨⟨b.lo.x - t, b.lo.y - t, b.lo.z - t⟩, ⟨b.hi.x + t, b.hi.y + t, b.hi.z + t⟩⟩

/-- The point `p + s (q − p)`, `0 ≤ s ≤ 1`. -/
def lerp (p q : V K) (s : K) : V K := V.add p (V.smul s (V.sub q p))

/-- One coordinate: between two values in `[lo, hi]` stays in it. -/
theorem between {lo hi a b s : K} (ha : lo ≤ a ∧ a ≤ hi) (hb : lo ≤ b ∧ b ≤ hi) (s0 : 0 ≤ s)
    (s1 : s ≤ 1) : lo ≤ a + s * (b - a) ∧ a + s * (b - a) ≤ hi := by
  -- a + s (b − a) = (1 − s) a + s b
  have e : a + s * (b - a) = (1 - s) * a + s * b := by grind
  rw [e]
  have h1 := mul_le_mul_of_nonneg_left' ha.1 (show 0 ≤ 1 - s by grind)
  have h2 := mul_le_mul_of_nonneg_left' hb.1 s0
  have h3 := mul_le_mul_of_nonneg_left' ha.2 (show 0 ≤ 1 - s by grind)
  have h4 := mul_le_mul_of_nonneg_left' hb.2 s0
  constructor <;> grind

/-- **Boxes are convex.** -/
theorem segment_mem {b : Box3 K} {p q : V K} {s : K} (hp : b.mem p) (hq : b.mem q) (s0 : 0 ≤ s)
    (s1 : s ≤ 1) : b.mem (lerp p q s) := by
  obtain ⟨px0, px1, py0, py1, pz0, pz1⟩ := hp
  obtain ⟨qx0, qx1, qy0, qy1, qz0, qz1⟩ := hq
  have bx := between ⟨px0, px1⟩ ⟨qx0, qx1⟩ s0 s1
  have by' := between ⟨py0, py1⟩ ⟨qy0, qy1⟩ s0 s1
  have bz := between ⟨pz0, pz1⟩ ⟨qz0, qz1⟩ s0 s1
  simp only [Box3.mem, lerp, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z, V.sub_x,
    V.sub_y, V.sub_z]
  exact ⟨bx.1, bx.2, by'.1, by'.2, bz.1, bz.2⟩

/-- **B1.** A set every point of which lies on a segment from box `A` to box `B` lies in any box
holding both, their hull among them: the rib's wall, from the thickened path region to the part. -/
theorem rib_in_hull {A B C : Box3 K} {R : Pred (V K)} (hA : A.Sub C) (hB : B.Sub C)
    (h : ∀ x, R x → ∃ p q s, A.mem p ∧ B.mem q ∧ 0 ≤ s ∧ s ≤ 1 ∧ x = lerp p q s) :
    ∀ x, R x → C.mem x := by
  intro x hx
  obtain ⟨p, q, s, hp, hq, s0, s1, rfl⟩ := h x hx
  exact segment_mem (hA p hp) (hB q hq) s0 s1

/-- A point within `t` of a box's point on every axis is in the box grown by `t`: the path's
plane region, thickened across the plane by at most `t`. -/
theorem grow_mem {b : Box3 K} {p q : V K} {t : K} (hp : b.mem p)
    (hx : -t ≤ q.x - p.x ∧ q.x - p.x ≤ t) (hy : -t ≤ q.y - p.y ∧ q.y - p.y ≤ t)
    (hz : -t ≤ q.z - p.z ∧ q.z - p.z ≤ t) : (b.grow t).mem q := by
  obtain ⟨h1, h2, h3, h4, h5, h6⟩ := hp
  simp only [Box3.mem, Box3.grow]
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> grind

end Rib

end ForgeExact.Ops
