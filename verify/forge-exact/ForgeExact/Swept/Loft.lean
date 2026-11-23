import ForgeExact.Swept.Prism

/-!
# Loft: the ruled solid between two sections (research/sweep-spec/DESIGN.md §3.4)

Lofts are kept as they are for now (the owner's call), so this is L1 and the differential only.

A ruled loft joins each point `q` of the first section to its match `f q` on the second by a
straight line: `Ruled R₀ f = {(1 − s)·q + s·f q : q ∈ R₀, 0 < s < 1}`. The kernel matches segments
by the same fraction along each, which fixes `f` on the boundary; the solid is the region the
walls and the two caps bound, on the domain "that surface is embedded" (no wall meets another off
their shared rails, the normal never vanishes: `LoftError::Twisted`, `SelfIntersecting`). A smooth
loft's walls are research/loft-ssi's Skinned surface (`splineW`, `polyW_eq` in LoftMeet).

* `ruled_translate`: when the second section is the first moved by `t` (`f q = q + t`), the ruled
  loft is the prism along `t` over `(0, 1)`. That is how the kernel builds an extrude along a
  direction (`Extent::Along`, a ruled loft between two translates), and `sweep-diff` finds every
  such extrude equal to its prism.
-/

namespace ForgeExact.Swept

open ForgeExact Ref Flat

/-- The ruled loft from the points of the sketch-frame region `loops` (frame `F`, at `s = 0`) to
their matches `f`. -/
def Ruled (loops : List (List (Rat × Rat))) (F : Frame3) (f : V3 → V3) : Pred V3 :=
  fun p => ∃ x y s, inRegion loops (x, y) = true ∧ 0 < s ∧ s < 1 ∧
    p = ((1 : Rat) - s) * F.pt x y 0 + s * f (F.pt x y 0)

theorem v3_combo (q t : V3) (s : Rat) : ((1 : Rat) - s) * q + s * (q + t) = q + s * t := by
  obtain ⟨a, b, c⟩ := q; obtain ⟨d, e, g⟩ := t
  apply v3_ext <;> simp only [add_x, add_y, add_z, smul_x, smul_y, smul_z] <;> grind

theorem pt_travel (F : Frame3) (x y s : Rat) : F.pt x y 0 + s * F.v = F.pt x y s := by
  obtain ⟨⟨a, b, c⟩, ⟨d, e, g⟩, ⟨h, i, j⟩, ⟨k, l, m⟩⟩ := F
  simp only [Frame3.pt]
  apply v3_ext <;> simp only [add_x, add_y, add_z, smul_x, smul_y, smul_z] <;> grind

/-- **A loft between translates is a prism.** -/
theorem ruled_translate (loops : List (List (Rat × Rat))) (F : Frame3) :
    Ruled loops F (fun q => q + F.v) = Prism loops F 0 1 := by
  funext p; apply propext
  constructor
  · rintro ⟨x, y, s, hr, h0, h1, rfl⟩
    exact ⟨x, y, s, by rw [v3_combo, pt_travel], hr, h0, h1⟩
  · rintro ⟨x, y, s, rfl, hr, h0, h1⟩
    exact ⟨x, y, s, hr, h0, h1, by rw [v3_combo, pt_travel]⟩

end ForgeExact.Swept
