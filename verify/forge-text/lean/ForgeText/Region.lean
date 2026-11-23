import ForgeText.Seg

/-!
# A text's region (interface/forge-lang/src/sketch_text.rs, kernel/forge-sketch: `profiles_in_groups`)

Over an abstract plane: a region is a set of points. A glyph part's region is its outside
contours' insides less its counters' (each by the winding rule); a glyph's region is the union of
its parts' (a composite's components may overlap: Å is A and a ring); the text's is the union of
its glyphs', refused when two glyphs' boundaries share a point.
-/

namespace ForgeText

abbrev Region (P : Type) := P → Prop

def union {P : Type} (rs : List (Region P)) : Region P := fun p => ∃ r ∈ rs, r p

/-- Two glyphs meet when their boundaries share a point (they cross or touch). -/
def Meet {P : Type} (bA bB : Region P) : Prop := ∃ p, bA p ∧ bB p

/-- The text's region: the union of every glyph's, each the union of its parts'. -/
def textRegion {P : Type} (glyphs : List (List (Region P))) : Region P :=
  union (glyphs.map union)

/-- A point is in the text iff it is in some part of some glyph: the order the parts and
glyphs are taken in (the boolean's order) does not matter. -/
theorem mem_textRegion {P : Type} (glyphs : List (List (Region P))) (p : P) :
    textRegion glyphs p ↔ ∃ g ∈ glyphs, ∃ r ∈ g, r p := by
  simp only [textRegion, union, List.mem_map]
  constructor
  · rintro ⟨r, ⟨g, hg, rfl⟩, ⟨r', hr', hp⟩⟩
    exact ⟨g, hg, r', hr', hp⟩
  · rintro ⟨g, hg, r, hr, hp⟩
    exact ⟨union g, ⟨g, hg, rfl⟩, r, hr, hp⟩

/-- The refusal's claim: a text is set only if no two different glyphs' boundaries meet. -/
def RefusesMeeting {P : Type} (boundaries : List (Region P)) (set : Bool) : Prop :=
  set = true → ∀ i j, i ≠ j → (hi : i < boundaries.length) → (hj : j < boundaries.length) →
    ¬ Meet (boundaries[i]'hi) (boundaries[j]'hj)

end ForgeText
