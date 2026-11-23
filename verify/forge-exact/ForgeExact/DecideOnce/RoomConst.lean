import ForgeExact.JordanLocal

/-!
# A8 L4: membership is constant across a room (`room_const`)

face-arrangement.md §2.3. A room of the face arrangement is a radial class of facet-sides: sides
joined round edges. The builder decides a room's membership in each body once, with one witness. It
is right for every facet-side of the room when consecutive sides are joined by polylines each of
whose segments is `Clear` of every face. A `Clear` segment meets each face's plane, if at all,
outside the face.

* `ClearPath F p q`: a polyline from `p` to `q`, every segment `Clear` of every face in `F`.
* `room_const`: along a `ClearPath` over a list holding a closed flat-faced solid's faces, the
  solid's ray-parity membership is constant (`Jordan.contains_const_local`, segment by segment).
* The wedge lemma, which belongs to the builder's L3 (R3), supplies the paths. For consecutive
  facets `φ`, `ψ` round an edge, points just off `φ` and just off `ψ`, on the joined sides, are
  joined by a `ClearPath`; chaining the joins reaches the room's witness. `room_const` then gives
  `Refines`' rooms condition for the builder: each facet-side's membership is its room witness's.
-/

namespace ForgeExact.DecideOnce

open ForgeExact ForgeExact.Ref Jordan

/-- A polyline whose every segment is `Clear` of every face of `F`. -/
inductive ClearPath (F : List Ref.Face) : V3 → V3 → Prop
  | refl (p : V3) : ClearPath F p p
  | step {p q r : V3} : ClearPath F p q → (∀ f ∈ F, Clear f q r) → ClearPath F p r

namespace ClearPath

variable {F : List Face}

theorem trans {p q r : V3} (h1 : ClearPath F p q) (h2 : ClearPath F q r) : ClearPath F p r := by
  induction h2 with
  | refl => exact h1
  | step _ hc ih => exact ClearPath.step ih hc

theorem symm {p q : V3} (h : ClearPath F p q) : ClearPath F q p := by
  induction h with
  | refl => exact ClearPath.refl _
  | step _ hc ih =>
    exact (ClearPath.step (ClearPath.refl _) fun f hf => clear_symm (hc f hf)).trans ih

theorem mono {G : List Ref.Face} (hFG : ∀ f ∈ G, f ∈ F) {p q : V3} (h : ClearPath F p q) : ClearPath G p q := by
  induction h with
  | refl => exact ClearPath.refl _
  | step _ hc ih => exact ClearPath.step ih fun f hf => hc f (hFG f hf)

end ClearPath

/-- **Membership is constant along a clear polyline** (a room's facet-sides share their membership). -/
theorem room_const (S : Solid) (hflat : FacesFlat S) (hcl : Closed S) {F : List Ref.Face}
    (hF : ∀ f ∈ facesL S, f ∈ F) {p q : V3} (h : ClearPath F p q) : S.contains p = S.contains q := by
  have h' := h.mono hF
  clear h
  induction h' with
  | refl => rfl
  | step _ hc ih => exact Eq.trans ih (contains_const_local S hflat hcl _ _ hc)

/-- For several bodies: every body's membership is constant along a clear polyline over all faces. -/
theorem room_const_all (S : Nat → Solid) (hflat : ∀ i, FacesFlat (S i)) (hcl : ∀ i, Closed (S i))
    {F : List Ref.Face} (hF : ∀ i, ∀ f ∈ facesL (S i), f ∈ F) {p q : V3} (h : ClearPath F p q) (i : Nat) :
    (S i).contains p = (S i).contains q :=
  room_const (S i) (hflat i) (hcl i) (hF i) h

end ForgeExact.DecideOnce
