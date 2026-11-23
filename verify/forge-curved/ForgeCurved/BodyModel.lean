import ForgeCurved.F6

/-!
# A Lean model of a `forge_topo::Body`: `⟦b⟧`, `⟦f⟧`, `ValidSolid`, and the bridge to rooms

old-path-statements.md §0 (the coordinator's, 9cd330bb). Every refinement proof of today's booleans
reads a `Body` through an abstraction. This file is that abstraction's target. The aeneas
session's translated `Body`/`Face`/`Loop`/`Edge` map into `BodyModel` by one function, which
gives each live face:
* its **carrier** `c_f : E → ℝ`: the surface's implicit function (`QFace.quadric` for quadrics; a
  plane's signed distance);
* its **region** `⟦f⟧`: the surface's points inside its trimming loops;
* its **sense** `o_f`, from `Face.same_sense` and the per-kind rule (FaceSense.lean). With
  `o_f = true` the solid is on the side `c_f < 0`.

**`Bounds b S`**: `S` is the solid `b` bounds. `S` is closed, regular and bounded, its frontier
is exactly the faces, and at each face point off the other faces the face is oriented (F6's
`Oriented`). **`ValidSolid b`** is `∃ S, Bounds b S`, and **`⟦b⟧`** is such an `S`.
`forge_topo::check` and the producing feature's validity supply `ValidSolid`, which is a
hypothesis here, as the route's F6 takes it.

**The bridge (c)**, for any arrangement `G` (F5's `FacetData`) whose face set contains `b`'s faces,
with connected rooms:
* `room_in_or_out`: every room of `G` is inside `⟦b⟧` or misses it;
* `denote_eq_closure_rooms`: `⟦b⟧` is the closure of the rooms inside it, so it is a union of
  rooms up to the faces;
* `f6_hyps`: `⟦b⟧` meets F6's hypotheses `hfr` and `hoff`, so F5/F6 apply to it unchanged.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.BodyModel

open Set Filter Topology ForgeCurved.FaceLocal ForgeCurved.FacetSides ForgeCurved.Rooms
  ForgeCurved.F6

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- A live face, abstracted: carrier, region (`⟦f⟧`), sense. -/
structure FaceModel (E : Type) where
  carrier : E → ℝ
  region : Set E
  sense : Bool
  on_carrier : ∀ y ∈ region, carrier y = 0

/-- A body, abstracted: its live faces. -/
structure BodyModel (E : Type) where
  faces : List (FaceModel E)

/-- The faces' points. -/
def Faces (b : BodyModel E) : Set E := ⋃ f ∈ b.faces, f.region

/-- `y` is on face `f` and on no other face of `b`: where `f`'s orientation is read. -/
def Alone (b : BodyModel E) (f : FaceModel E) (y : E) : Prop :=
  y ∈ f.region ∧ ∀ g ∈ b.faces, g ≠ f → y ∉ g.region

/-- **`S` is the solid `b` bounds.** -/
structure Bounds (b : BodyModel E) (S : Set E) : Prop where
  closed : IsClosed S
  regular : closure (interior S) = S
  bounded : Bornology.IsBounded S
  frontier_eq : frontier S = Faces b
  oriented : ∀ f ∈ b.faces, ∀ y, Alone b f y → Oriented f.carrier S f.sense y

/-- **A valid solid**: its faces bound one. -/
def ValidSolid (b : BodyModel E) : Prop := ∃ S, Bounds b S

open Classical in
/-- **`⟦b⟧`, the solid a body bounds** (empty for an invalid body; every statement assumes
`ValidSolid`). -/
noncomputable def denote (b : BodyModel E) : Set E :=
  if h : ValidSolid b then Classical.choose h else ∅

/-- `⟦f⟧`. -/
def denoteFace (f : FaceModel E) : Set E := f.region

theorem bounds_denote {b : BodyModel E} (h : ValidSolid b) : Bounds b (denote b) := by
  unfold denote; rw [dif_pos h]; exact Classical.choose_spec h

variable {G : FaceGeo E} {FacesG : Set E} {cf : Nat → E → ℝ}

/-- **Every room is inside `⟦b⟧` or misses it.** -/
theorem room_in_or_out (hd : FacetData G FacesG cf) (hconn : ∀ r, IsPreconnected (G.room r))
    {b : BodyModel E} (hv : ValidSolid b) (hsub : Faces b ⊆ FacesG) (r : Nat) :
    G.room r ⊆ denote b ∨ Disjoint (G.room r) (denote b) := by
  have hfr : frontier (denote b) ⊆ FacesG := by rw [(bounds_denote hv).frontier_eq]; exact hsub
  exact room_class (hconn r) ((hd.room_faces r).mono_right hfr)

/-- **`⟦b⟧` is the closure of the rooms inside it**, when the faces are nowhere dense (their
interior is empty). -/
theorem denote_eq_closure_rooms (hd : FacetData G FacesG cf) (hconn : ∀ r, IsPreconnected (G.room r))
    {b : BodyModel E} (hv : ValidSolid b) (hsub : Faces b ⊆ FacesG)
    (hnd : interior FacesG = ∅) :
    denote b = closure (⋃ r, ⋃ (_ : G.room r ⊆ denote b), G.room r) := by
  have hB := bounds_denote hv
  apply Set.Subset.antisymm
  · -- the interior is covered by the rooms inside, off the faces; the faces have empty interior
    conv_lhs => rw [← hB.regular]
    apply closure_minimal _ isClosed_closure
    intro x hx
    rw [mem_closure_iff]
    intro o ho hxo
    -- `o ∩ interior ⟦b⟧` is a nonempty open set; it is not inside the faces
    have hU : IsOpen (o ∩ interior (denote b)) := ho.inter isOpen_interior
    have hne : (o ∩ interior (denote b)).Nonempty := ⟨x, hxo, hx⟩
    obtain ⟨z, hz, hzF⟩ : ∃ z ∈ o ∩ interior (denote b), z ∉ FacesG := by
      by_contra hall
      push Not at hall
      have : o ∩ interior (denote b) ⊆ interior FacesG := interior_maximal hall hU
      rw [hnd] at this
      obtain ⟨w, hw⟩ := hne
      exact this hw
    obtain ⟨r, hr⟩ := hd.room_cover z hzF
    have hrin : G.room r ⊆ denote b := by
      rcases room_in_or_out hd hconn hv hsub r with h | h
      · exact h
      · exact absurd (interior_subset hz.2) (Set.disjoint_left.mp h hr)
    exact ⟨z, hz.1, mem_iUnion.mpr ⟨r, mem_iUnion.mpr ⟨hrin, hr⟩⟩⟩
  · exact closure_minimal (iUnion_subset fun r => iUnion_subset fun h => h) hB.closed

/-- **F6's hypotheses hold for `⟦b⟧`.** Its frontier is on the faces, and a facet of `G` that
misses `b`'s faces misses its frontier. -/
theorem f6_hyps {b : BodyModel E} (hv : ValidSolid b) (hsub : Faces b ⊆ FacesG) :
    frontier (denote b) ⊆ FacesG ∧
      ∀ φ : Set E, Disjoint φ (Faces b) → Disjoint φ (frontier (denote b)) := by
  have hfe := (bounds_denote hv).frontier_eq
  exact ⟨hfe ▸ hsub, fun φ hφ => hfe ▸ hφ⟩

/-- **F6's orientation for `⟦b⟧`**: at a point of a face alone, the face is oriented. -/
theorem oriented_denote {b : BodyModel E} (hv : ValidSolid b) {f : FaceModel E} (hf : f ∈ b.faces)
    {y : E} (hy : Alone b f y) : Oriented f.carrier (denote b) f.sense y :=
  (bounds_denote hv).oriented f hf y hy

end ForgeCurved.BodyModel
