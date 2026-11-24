import ForgeCurved.FacetSides
import ForgeCurved.Rooms

/-!
# F6, orientation: a room's membership from the faces' outward sides

Route (iii) (research/decide-once/curved-brep-route.md §2) decides each room's membership in body
`i` without rays. A room next to a piece of a face of `i` is on that face's inside or outside, and
the face's orientation `o_f` says which. Across a facet that is not a piece of `i`, membership in
`i` does not change. Those are the hypotheses of the coordinator's `flood_correct`.

The hypothesis per body is that it is **the solid bounded by its faces**, stated locally:
* `frontier (A i) ⊆ Faces`: the body's boundary is on the faces;
* a facet that is not a piece of `i` misses `frontier (A i)`;
* **orientation at one point** of each piece: near it, the face's inside side (`cf f < 0` when
  `o = true`, `cf f > 0` otherwise) is in `A i` and the other side is not. This is the B-rep's
  validity with outward-oriented faces (`forge_topo::check` and the producing feature), at a
  single point. F6 carries it along the whole connected facet.

With the builder's facet data (`FacetData`, F5) and connected rooms:
* **`f6_piece`**: across a piece, the inside room is in `A i` and the outside room misses it;
* **`f6_off`**: across any other facet, the two rooms are both in `A i` or both miss it;
* **`f6`**: both, as the per-room memberships `flood_correct` consumes.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.F6

open Set Filter Topology ForgeCurved.FaceLocal ForgeCurved.Chart ForgeCurved.FacetSides
  ForgeCurved.Rooms

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]
variable {G : FaceGeo E} {Faces : Set E} {cf : Nat → E → ℝ}

/-- The inside side of a face with orientation `o`: `cf < 0` when `o`, else `cf > 0`. -/
def inSide (o : Bool) (a : ℝ) : Prop := if o then a < 0 else 0 < a

/-- The other side. -/
def outSide (o : Bool) (a : ℝ) : Prop := if o then 0 < a else a < 0

/-- **The face is oriented at `y`**: near `y`, its inside side is in `A`, its outside side not. -/
def Oriented (c : E → ℝ) (A : Set E) (o : Bool) (y : E) : Prop :=
  (∀ᶠ z in 𝓝 y, inSide o (c z) → z ∈ A) ∧ (∀ᶠ z in 𝓝 y, outSide o (c z) → z ∉ A)

/-- The room on the inside of a facet, and on the outside. -/
def inRoom (G : FaceGeo E) (o : Bool) (f : Nat) : Nat := if o then G.rm f else G.rp f
def outRoom (G : FaceGeo E) (o : Bool) (f : Nat) : Nat := if o then G.rp f else G.rm f

theorem room_frontier (hd : FacetData G Faces cf) {A : Set E} (hfr : frontier A ⊆ Faces) (r : Nat) :
    Disjoint (G.room r) (frontier A) := (hd.room_faces r).mono_right hfr

theorem room_in (hd : FacetData G Faces cf) {A : Set E} (hfr : frontier A ⊆ Faces) {r : Nat}
    (hc : IsPreconnected (G.room r)) {z : E} (hz : z ∈ G.room r) (hzA : z ∈ A) : G.room r ⊆ A :=
  (room_class hc (room_frontier hd hfr r)).resolve_right fun h => Set.disjoint_left.mp h hz hzA

theorem room_out (hd : FacetData G Faces cf) {A : Set E} (hfr : frontier A ⊆ Faces) {r : Nat}
    (hc : IsPreconnected (G.room r)) {z : E} (hz : z ∈ G.room r) (hzA : z ∉ A) :
    Disjoint (G.room r) A := by
  rcases room_class hc (room_frontier hd hfr r) with h | h
  · exact absurd (h hz) hzA
  · exact h

/-- Near every facet point, its sides are its rooms, and each side is met arbitrarily close. -/
theorem side_room (hd : FacetData G Faces cf) {f : Nat} {y : E} (hy : y ∈ G.facet f) (s : Bool) :
    (∀ᶠ z in 𝓝 y, (if s then cf f z < 0 else 0 < cf f z) →
      z ∈ G.room (if s then G.rm f else G.rp f)) ∧
    ∃ᶠ z in 𝓝 y, (if s then cf f z < 0 else 0 < cf f z) := by
  cases s
  · refine ⟨side_all hd f (fun a => 0 < a) (Or.inl rfl) (G.rp f) (hd.sample_p f) y hy, ?_⟩
    obtain ⟨N, -, -, -, hcl⟩ := chart hd hy (fun a => 0 < a) (Or.inl rfl)
    exact (mem_closure_iff_frequently.mp hcl).mono fun z hz => hz.2
  · refine ⟨side_all hd f (fun a => a < 0) (Or.inr rfl) (G.rm f) (hd.sample_m f) y hy, ?_⟩
    obtain ⟨N, -, -, -, hcl⟩ := chart hd hy (fun a => a < 0) (Or.inr rfl)
    exact (mem_closure_iff_frequently.mp hcl).mono fun z hz => hz.2

/-- A point of the side `s` near `y`, in its room, with property `Q` eventually near `y`. -/
theorem side_point (hd : FacetData G Faces cf) {f : Nat} {y : E} (hy : y ∈ G.facet f) (s : Bool)
    {Q : E → Prop} (hQ : ∀ᶠ z in 𝓝 y, (if s then cf f z < 0 else 0 < cf f z) → Q z) :
    ∃ z, z ∈ G.room (if s then G.rm f else G.rp f) ∧ Q z := by
  obtain ⟨hev, hfr⟩ := side_room hd hy s
  obtain ⟨z, ⟨hs, h1⟩, h2⟩ := ((hfr.and_eventually hev).and_eventually hQ).exists
  exact ⟨z, h1 hs, h2 hs⟩

/-- **F6 across a piece**: the inside room is in the body, the outside room misses it. -/
theorem f6_piece (hd : FacetData G Faces cf) (hconn : ∀ r, IsPreconnected (G.room r)) {A : Set E}
    (hfr : frontier A ⊆ Faces) {f : Nat} {o : Bool} {y : E} (hy : y ∈ G.facet f)
    (hor : Oriented (cf f) A o y) :
    G.room (inRoom G o f) ⊆ A ∧ Disjoint (G.room (outRoom G o f)) A := by
  obtain ⟨hin, hout⟩ := hor
  constructor
  · obtain ⟨z, hz, hzA⟩ := side_point hd hy o (Q := fun z => z ∈ A)
      (hin.mono fun z h hs => h (by cases o <;> simpa [inSide] using hs))
    exact room_in hd hfr (hconn _) (by cases o <;> simpa [inRoom] using hz) hzA
  · obtain ⟨z, hz, hzA⟩ := side_point hd hy (!o) (Q := fun z => z ∉ A)
      (hout.mono fun z h hs => h (by cases o <;> simpa [outSide] using hs))
    exact room_out hd hfr (hconn _) (by cases o <;> simpa [outRoom] using hz) hzA

/-- **F6 across any other facet**: off the body's frontier, both rooms agree. -/
theorem f6_off (hd : FacetData G Faces cf) (hconn : ∀ r, IsPreconnected (G.room r)) {A : Set E}
    (hfr : frontier A ⊆ Faces) {f : Nat} (hoff : Disjoint (G.facet f) (frontier A)) {y : E}
    (hy : y ∈ G.facet f) :
    (G.room (G.rp f) ⊆ A ∧ G.room (G.rm f) ⊆ A) ∨
      (Disjoint (G.room (G.rp f)) A ∧ Disjoint (G.room (G.rm f)) A) := by
  have hyf : y ∉ frontier A := Set.disjoint_left.mp hoff hy
  by_cases hcl : y ∈ closure A
  · -- inside: a neighbourhood lies in `A`
    have hint : y ∈ interior A := by
      by_contra h; exact hyf ⟨hcl, h⟩
    have hev : ∀ᶠ z in 𝓝 y, z ∈ A := mem_interior_iff_mem_nhds.mp hint
    left
    obtain ⟨z, hz, hzA⟩ := side_point hd hy false (Q := fun z => z ∈ A) (hev.mono fun z h _ => h)
    obtain ⟨z', hz', hzA'⟩ := side_point hd hy true (Q := fun z => z ∈ A) (hev.mono fun z h _ => h)
    exact ⟨room_in hd hfr (hconn _) hz hzA, room_in hd hfr (hconn _) hz' hzA'⟩
  · -- outside: a neighbourhood misses `A`
    have hev : ∀ᶠ z in 𝓝 y, z ∉ A := by
      have : (closure A)ᶜ ∈ 𝓝 y := isClosed_closure.isOpen_compl.mem_nhds hcl
      exact Filter.mem_of_superset this fun z hz hzA => hz (subset_closure hzA)
    right
    obtain ⟨z, hz, hzA⟩ := side_point hd hy false (Q := fun z => z ∉ A) (hev.mono fun z h _ => h)
    obtain ⟨z', hz', hzA'⟩ := side_point hd hy true (Q := fun z => z ∉ A) (hev.mono fun z h _ => h)
    exact ⟨room_out hd hfr (hconn _) hz hzA, room_out hd hfr (hconn _) hz' hzA'⟩

/-- **F6.** For bodies `A i` bounded by their faces, with each piece `piece i f` oriented (`o i f`)
at one of its points: across a piece, membership in `i` is the orientation's; across any other
facet it is unchanged. Rooms are nonempty next to a facet (F5's `sides`), so the memberships
`G.room r ⊆ A i` are the per-room memberships `flood_correct` consumes. -/
theorem f6 (hd : FacetData G Faces cf) (hconn : ∀ r, IsPreconnected (G.room r))
    (A : Nat → Set E) (piece : Nat → Nat → Prop) (o : Nat → Nat → Bool)
    (hfr : ∀ i, frontier (A i) ⊆ Faces)
    (hoff : ∀ i f, ¬ piece i f → Disjoint (G.facet f) (frontier (A i)))
    (hor : ∀ i f, piece i f → ∃ y ∈ G.facet f, Oriented (cf f) (A i) (o i f) y)
    (i f : Nat) (hne : (G.facet f).Nonempty) :
    (piece i f → G.room (inRoom G (o i f) f) ⊆ A i ∧ Disjoint (G.room (outRoom G (o i f) f)) (A i)) ∧
    (¬ piece i f → (G.room (G.rp f) ⊆ A i ↔ G.room (G.rm f) ⊆ A i)) := by
  refine ⟨fun hp => ?_, fun hnp => ?_⟩
  · obtain ⟨y, hy, ho⟩ := hor i f hp
    exact f6_piece hd hconn (hfr i) hy ho
  · obtain ⟨y, hy⟩ := hne
    -- both rooms are nonempty: they reach `y`
    have hnp' : ∀ s : Bool, (G.room (if s then G.rm f else G.rp f)).Nonempty := fun s => by
      obtain ⟨z, hz, -⟩ := side_point hd hy s (Q := fun _ => True) (Eventually.of_forall fun _ _ => trivial)
      exact ⟨z, hz⟩
    obtain ⟨zp, hzp⟩ := hnp' false
    obtain ⟨zm, hzm⟩ := hnp' true
    rcases f6_off hd hconn (hfr i) (hoff i f hnp) hy with ⟨h1, h2⟩ | ⟨h1, h2⟩
    · exact ⟨fun _ => h2, fun _ => h1⟩
    · exact ⟨fun h => absurd (h hzp) (Set.disjoint_left.mp h1 hzp),
        fun h => absurd (h hzm) (Set.disjoint_left.mp h2 hzm)⟩

end ForgeCurved.F6
