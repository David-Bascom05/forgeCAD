import ForgeWeld.Member

/-!
# The cut list is the half-spaces' extent

A straight member's points are `a + s d + u ex + v ey`, `(u, v)` in its section. An end cut
`(n, c)` (keeping `n · x ≤ c`) facing along the member (`n · d > 0`) bounds `s`:

* `reach_le`: every point of the member has `s (n·d) ≤ c - n·a + H`, where `H` bounds the section in
  the direction `(-(n·ex), -(n·ey))` (its support there);
* `reach_attained`: where the section reaches `H` at `(u*, v*)` and the other cuts keep that point,
  the member has a point with `s (n·d) = c - n·a + H`.

So the far end of the member along its axis (the long point) is `s = (c - n·a + H) / (n·d)`:
forge_weld::frame's `crossings` (`far = (n·(q - a) + support(-n)) / n·d`), and the cut list's
`overall` is that at the end less the same at the start. The path's own crossing is the case
`(u, v) = (0, 0)` (`path_crossing`): the cut list's `length`.
-/

namespace ForgeWeld

open ForgeExact ForgeExact.Ref ForgeExact.Topology

/-- The axial coordinate `s` of every point of the member is bounded by an end cut. -/
theorem reach_le {F : Frame} {S : Rat → Rat → Prop} {cuts : List (V3 × Rat)} {n : V3} {c H : Rat}
    (hk : (n, c) ∈ cuts) (hS : Upper S (-(n.dot F.ex)) (-(n.dot F.ey)) H)
    {s u v : Rat} (huv : S u v) (hx : Member F S cuts (F.a + s * F.d + u * F.ex + v * F.ey)) :
    s * n.dot F.d ≤ c - n.dot F.a + H := by
  have h1 := member_half hk hx
  have h2 := hS u v huv
  simp only [dot_add, dot_smul] at h1
  grind

/-- And reached: at a section point where the support is attained, the point on the cut's plane
is in the member when the other cuts keep it. -/
theorem reach_attained {F : Frame} {S : Rat → Rat → Prop} {cuts : List (V3 × Rat)} {n : V3}
    {c H s u v : Rat} (huv : S u v) (hH : -(n.dot F.ex) * u + -(n.dot F.ey) * v = H)
    (hs : s * n.dot F.d = c - n.dot F.a + H)
    (hother : ∀ k ∈ cuts, Half k.1 k.2 (F.a + s * F.d + u * F.ex + v * F.ey)) :
    Member F S cuts (F.a + s * F.d + u * F.ex + v * F.ey) ∧
      n.dot (F.a + s * F.d + u * F.ex + v * F.ey) = c := by
  refine ⟨⟨⟨s, u, v, huv, rfl⟩, hother⟩, ?_⟩
  simp only [dot_add, dot_smul]
  grind

/-- So the long point is unique: two axial coordinates both reaching the bound are equal (the
cut faces along the member: `n · d ≠ 0`). -/
theorem long_point_unique {n d : V3} {R s t : Rat} (hnd : n.dot d ≠ 0) (hs : s * n.dot d = R)
    (ht : t * n.dot d = R) : s = t := by
  have : (s - t) * n.dot d = 0 := by grind
  rcases Rat.mul_eq_zero.mp this with h | h
  · grind
  · exact absurd h hnd

/-- The path (the section's origin) crosses the cut's plane where `s (n·d) = c - n·a`: the cut
list's `length` is between the two ends' crossings. -/
theorem path_crossing {F : Frame} {n : V3} {c s : Rat} (hs : s * n.dot F.d = c - n.dot F.a) :
    n.dot (F.a + s * F.d) = c := by
  simp only [dot_add, dot_smul]
  grind

end ForgeWeld
