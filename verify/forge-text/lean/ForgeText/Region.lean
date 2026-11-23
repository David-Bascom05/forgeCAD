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

/-- The check `profiles_in_groups` makes before a text is set: over the loops `0..n`, each with
its glyph (`whole`), every pair `i < j` of different glyphs is asked whether they meet (crossing or
touching); the text is set only if none does. -/
def apart (n : Nat) (whole : Nat → Nat) (meets : Nat → Nat → Bool) : Bool :=
  (List.range n).all fun i => (List.range n).all fun j => !(decide (i < j) && decide (whole i ≠ whole j)) || !meets i j

/-- The refusal's claim: a set text has no two loops of different glyphs that meet (`meets`
being symmetric, as two loops sharing a point is). -/
theorem refuses_meeting (n : Nat) (whole : Nat → Nat) (meets : Nat → Nat → Bool)
    (symm : ∀ a b, meets a b = meets b a) (h : apart n whole meets = true) :
    ∀ i j, i < n → j < n → whole i ≠ whole j → meets i j = false := by
  intro i j hi hj hw
  simp only [apart, List.all_eq_true, List.mem_range] at h
  have hij : i ≠ j := fun e => hw (e ▸ rfl)
  rcases Nat.lt_or_gt_of_ne hij with lt | gt
  · have := h i hi j hj
    simp [lt, hw] at this
    simpa using this
  · have := h j hj i hi
    have hw' : whole j ≠ whole i := fun e => hw e.symm
    simp [gt, hw'] at this
    rw [symm]; simpa using this

end ForgeText
