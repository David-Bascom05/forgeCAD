import ForgeBoolProof.FlatSites.Where

/-!
# G13: `face_point`'s probe stands for its face

old-path-statements.md §G, G13. `face_point(f)` steps inward from the middle of `f`'s first edge,
by `min(0.01·len, 0.05)`, and its two callers classify the point against the other solid to class
a whole face no new edge touches (boolean.rs `general`'s untouched faces; `local`'s first loop):
* the step was unchecked: a face narrower than the step there puts the probe off the face;
* with proof/flat-sites-g13, `face_point` keeps a planar face's probe only when it is inside the
  face (`classify::probe_inside`: `point_in_face(f, q) = Inside`, or a cheap sufficient test cross-
  checked against it in debug builds), halving the step up to four times, else `None` (the callers
  take the general path); a curved face's probe stays unchecked (G13.curved, open).

What makes the class of the probe the class of the face:
* `point_in_face = Inside` gives `q ∈ relint ⟦f⟧` (1a.pif);
* the face (its relative interior, a connected set) meets no point of the other solid's boundary
  (it is untouched: no new edge, imprint or coplanar partner);
* **`const_off_frontier`**: a preconnected set that misses `frontier S` lies wholly in `interior S`
  or wholly outside `closure S`. So `whereR S` takes one value on it, the probe's
  (**`where_const`**).

Measured before the check (FLAT-SITES, gate parts seq + nary1, fuzz 1–40 seq): 7 of 6,675 probes
were not `Inside` their own face (6 curved, 1 planar; fuzz 19, 20, 21, 30, 40). Whether today's
answer there was right is UNKNOWN (the parts are curved, outside the exact reference), so the check
closes the gap rather than judging those cases.
-/

namespace ForgeBoolProof.FlatSites.G13

open Set

variable {E : Type} [TopologicalSpace E]

/-- A preconnected set off a set's frontier lies in its interior or off its closure. -/
theorem const_off_frontier {S R : Set E} (hR : IsPreconnected R) (hoff : Disjoint R (frontier S)) :
    R ⊆ interior S ∨ R ⊆ (closure S)ᶜ := by
  have hcover : R ⊆ interior S ∪ (closure S)ᶜ := by
    intro x hx
    by_contra h
    simp only [mem_union, mem_compl_iff, not_or, not_not] at h
    exact Set.disjoint_left.mp hoff hx ⟨h.2, h.1⟩
  have hdisj : Disjoint (interior S) (closure S)ᶜ :=
    Set.disjoint_left.mpr fun x hi hc => hc (subset_closure (interior_subset hi))
  rcases hR.subset_or_subset isOpen_interior isClosed_closure.isOpen_compl hdisj hcover with h | h
  · exact Or.inl h
  · exact Or.inr h

end ForgeBoolProof.FlatSites.G13

namespace ForgeBoolProof.FlatSites.G13

open Set ForgeBoolProof.FlatSites

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- **The probe's class is the face's.** For a preconnected face interior `R` off the other
solid's frontier, any two of its points get the same exact answer. -/
theorem where_const {S R : Set E} (hR : IsPreconnected R) (hoff : Disjoint R (frontier S))
    {q x : E} (hq : q ∈ R) (hx : x ∈ R) : whereR S x = whereR S q := by
  have key : ∀ y ∈ R, y ∉ frontier S := fun y hy hf => Set.disjoint_left.mp hoff hy hf
  rcases const_off_frontier hR hoff with h | h
  · rw [(whereR_inside_iff).mpr (h hx), (whereR_inside_iff).mpr (h hq)]
  · have hout : ∀ y ∈ R, whereR S y = .outside := by
      intro y hy
      have hy1 : y ∉ interior S := fun hi => h hy (subset_closure (interior_subset hi))
      unfold whereR
      simp [hy1, key y hy]
    rw [hout x hx, hout q hq]

end ForgeBoolProof.FlatSites.G13
