import Mathlib.Analysis.SpecialFunctions.Trigonometric.Basic
import Mathlib.Analysis.InnerProductSpace.Basic

/-!
# A screw face read the other way along its axis (forge-geom `screw/pair.rs` `meet_screw`)

A screw face's point at turn angle u and section point (r, h), in the frame (o; x, y, z), is

  o + (r + a u) (cos u x + sin u y) + (h + b u) z.

Read in the frame (o; x, −y, −z) with drift −a, rise b, section point (r, −h) and angle −u, it is
the same point (`flip_point`). So a face about the axis the other way (a nut on a stud) is a face
about the same way, with its angles reversed: `meet_screw`'s closed form for faces about one axis
the same way applies to it. The code's map is negations only (frame axes, a, h, the angle range),
which IEEE arithmetic carries exactly; `pair.rs`'s `a_face_read_the_other_way_is_the_same_to_the_bit`
checks the f64 formula agrees to the bit (dmath's sin_cos is odd/even exactly).

`flip_range`: the angle range carries over as U ↦ −U, endpoints swapped.
-/

namespace ForgeCurved.ScrewFlip

variable {V : Type*} [AddCommGroup V] [Module ℝ V]

/-- A screw face's point. -/
noncomputable def point (o x y z : V) (a b r h u : ℝ) : V :=
  o + (r + a * u) • (Real.cos u • x + Real.sin u • y) + (h + b * u) • z

/-- **The flip.** Read about the axis the other way, with drift −a, rise b, section height −h and
angle −u, the point is the same. -/
theorem flip_point (o x y z : V) (a b r h u : ℝ) :
    point o x (-y) (-z) (-a) b r (-h) (-u) = point o x y z a b r h u := by
  simp only [point, Real.cos_neg, Real.sin_neg]
  have e1 : r + -a * -u = r + a * u := by ring
  have e2 : -h + b * -u = -(h + b * u) := by ring
  rw [e1, e2, smul_neg, neg_smul, smul_neg, neg_neg, neg_smul, neg_neg]

/-- The angle range reversed: u ∈ [lo, hi] iff −u ∈ [−hi, −lo]. -/
theorem flip_range (lo hi u : ℝ) : u ∈ Set.Icc lo hi ↔ -u ∈ Set.Icc (-hi) (-lo) := by
  simp only [Set.mem_Icc, neg_le_neg_iff]
  exact And.comm

/-- The same face as a set: every point of one reading is a point of the other, angles reversed. -/
theorem flip_face (o x y z : V) (a b r h lo hi : ℝ) (p : V) :
    (∃ u ∈ Set.Icc lo hi, p = point o x y z a b r h u) ↔
      ∃ u ∈ Set.Icc (-hi) (-lo), p = point o x (-y) (-z) (-a) b r (-h) u := by
  constructor
  · rintro ⟨u, hu, rfl⟩
    exact ⟨-u, (flip_range lo hi u).mp hu, (flip_point o x y z a b r h u).symm⟩
  · rintro ⟨u, hu, rfl⟩
    refine ⟨-u, ?_, ?_⟩
    · have := (flip_range lo hi (-u)).mpr (by simpa using hu)
      exact this
    · have := flip_point o x y z a b r h (-u)
      simp only [neg_neg] at this
      exact this

end ForgeCurved.ScrewFlip
