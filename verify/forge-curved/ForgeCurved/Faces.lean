import ForgeCurved.Local

/-!
# R1 from the faces: no Jordan theorem needed for "at most one component"

R1 asks every sign class to be connected. The hard direction of a Jordan theorem (a curve *does*
separate) is never needed: distinct sides of a curve have distinct covectors anyway. What is needed
is the easy direction, that a sign class has at most one component, and that follows from the
local picture and the faces:

* a component of the cell `Y` reaches its frontier (`NoClosedPiece`: no nonempty piece of `Y` cut
  out by an open set is closed);
* near a frontier point `p` the cell is locally connected (`LocalGood.conn`), so the whole of `Y`
  near `p` lies in one component; along a connected face `X` this side does not change; and
  across an incidence `X ≤ X'` of faces it does not change either;
* so if the proper faces of `Y` are linked by incidence (`FacesLinked`, a combinatorial condition
  on the realised covectors), every component of `Y` contains `Y` near every face, and there is
  one.

**`connected_of_faces`** is the step; **`cellsConnected_of_faces`** is the induction (the proper
faces of a cell have more zeros). Each realised cell is either connected directly (a vertex, a
whole closed surface or curve) or has linked faces and no closed piece. For full cells of `ℝ³`
`NoClosedPiece` is automatic (`noClosedPiece_full`). A spherical shell fails `FacesLinked` (its
two spheres share no face), so a check built on it is conservative there, as design §2.3's
wedge-connectivity check is.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set

variable {P : Type} [TopologicalSpace P]

/-- `X` is a proper realised face of `Y`. -/
def ProperFace (cs : List (Carrier P ℝ)) (Y X : Cov) : Prop :=
  RealisedK cs X ∧ X ≠ Y ∧ Cov.le X Y = true

/-- **The faces of `Y` are linked**: the incidence graph on the proper realised faces of `Y` is
connected (a property of faces closed under incidence holds of all or none). -/
def FacesLinked (cs : List (Carrier P ℝ)) (Y : Cov) : Prop :=
  ∀ A : Cov → Prop,
    (∀ X X', ProperFace cs Y X → ProperFace cs Y X' → Cov.le X X' = true → (A X ↔ A X')) →
    ∀ X X', ProperFace cs Y X → ProperFace cs Y X' → A X → A X'

/-- **No closed piece**: no nonempty part of the cell cut out by an open set is closed. For a
connected cell: the cell is not closed, i.e. it has a frontier. -/
def NoClosedPiece (cs : List (Carrier P ℝ)) (Y : Cov) : Prop :=
  ∀ v : Set P, IsOpen v → (cell cs Y ∩ v).Nonempty → ¬IsClosed (cell cs Y ∩ v)

theorem sub_or_sub {s u v : Set P} (hs : IsPreconnected s) (hu : IsOpen u) (hv : IsOpen v)
    (hsub : s ⊆ u ∪ v) (hd : ∀ y ∈ s, y ∈ u → y ∈ v → False) : s ⊆ u ∨ s ⊆ v := by
  by_contra h
  simp only [not_or, not_subset] at h
  obtain ⟨⟨y, hys, hyu⟩, ⟨z, hzs, hzv⟩⟩ := h
  have hyv : y ∈ v := (hsub hys).resolve_left hyu
  have hzu : z ∈ u := (hsub hzs).resolve_right hzv
  obtain ⟨w, hws, hwu, hwv⟩ := hs u v hu hv hsub ⟨z, hzs, hzu⟩ ⟨y, hys, hyv⟩
  exact hd w hws hwu hwv

/-- The points near which the cell lies in `w`. -/
def sideSet (cs : List (Carrier P ℝ)) (Y : Cov) (w : Set P) : Set P :=
  {p | ∃ U : Set P, _root_.IsOpen U ∧ p ∈ U ∧ cell cs Y ∩ U ⊆ w}

theorem isOpen_sideSet (cs : List (Carrier P ℝ)) (Y : Cov) (w : Set P) :
    IsOpen (sideSet cs Y w) := by
  rw [isOpen_iff_forall_mem_open]
  rintro p ⟨U, hUo, hpU, hU⟩
  exact ⟨U, fun q hq => ⟨U, hUo, hq, hU⟩, hUo, hpU⟩

theorem face_sub_closure {cs : List (Carrier P ℝ)} (hl : ∀ x, LocalGood cs x) {X Y : Cov}
    (hY : RealisedK cs Y) (hle : Cov.le X Y = true) : cell cs X ⊆ closure (cell cs Y) := by
  intro q hq
  have hq' : covOfK cs q = X := hq
  obtain ⟨y, hy⟩ := hY
  exact (hl q).near Y (by rw [hq']; exact hle) ⟨y, hy⟩

/-- The piece of `Y` in `w'` is closed when every proper face lies on the `w` side. -/
theorem piece_closed {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c) {Y : Cov}
    {w w' : Set P} (hw : IsOpen w) (hsub : cell cs Y ⊆ w ∪ w')
    (hd : ∀ y ∈ cell cs Y, y ∈ w → y ∈ w' → False)
    (hf : ∀ X, ProperFace cs Y X → cell cs X ⊆ sideSet cs Y w) : IsClosed (cell cs Y ∩ w') := by
  rw [← closure_subset_iff_isClosed]
  intro b hb
  have hbY : b ∈ closure (cell cs Y) := closure_mono inter_subset_left hb
  have hle := le_of_mem_closure hc hbY
  by_cases hX : covOfK cs b = Y
  · have hbY' : b ∈ cell cs Y := hX
    rcases hsub hbY' with h | h
    · obtain ⟨z, hzw, hzY, hzw'⟩ := mem_closure_iff.mp hb w hw h
      exact (hd z hzY hzw hzw').elim
    · exact ⟨hbY', h⟩
  · obtain ⟨U, hUo, hbU, hU⟩ := hf _ ⟨⟨b, rfl⟩, hX, hle⟩ (show b ∈ cell cs (covOfK cs b) from rfl)
    obtain ⟨z, hzU, hzY, hzw'⟩ := mem_closure_iff.mp hb U hUo hbU
    exact (hd z hzY (hU ⟨hzY, hzU⟩) hzw').elim

/-- **The step: a cell with connected, linked faces and no closed piece is connected.** -/
theorem connected_of_faces {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c)
    (hl : ∀ x, LocalGood cs x) {Y : Cov} (hY : RealisedK cs Y)
    (hfaces : ∀ X, ProperFace cs Y X → IsPreconnected (cell cs X))
    (hlink : FacesLinked cs Y) (hF : NoClosedPiece cs Y) : IsPreconnected (cell cs Y) := by
  intro u v hu hv hsub ⟨a, haY, hau⟩ ⟨b, hbY, hbv⟩
  by_contra hne
  have hd : ∀ y ∈ cell cs Y, y ∈ u → y ∈ v → False := fun y hy h1 h2 => hne ⟨y, hy, h1, h2⟩
  set Su := sideSet cs Y u
  set Sv := sideSet cs Y v
  -- every point of the closure is on one side, and not on both
  have hside : closure (cell cs Y) ⊆ Su ∪ Sv := by
    intro p hp
    obtain ⟨U, hUo, -, hpU, hconn⟩ := (hl p).conn univ isOpen_univ (mem_univ p)
    rcases sub_or_sub (hconn Y (le_of_mem_closure hc hp)) hu hv
      (inter_subset_left.trans hsub) (fun y hy => hd y hy.1) with h | h
    · exact Or.inl ⟨U, hUo, hpU, h⟩
    · exact Or.inr ⟨U, hUo, hpU, h⟩
  have hboth : ∀ p ∈ closure (cell cs Y), p ∈ Su → p ∈ Sv → False := by
    rintro p hp ⟨U, hUo, hpU, hU⟩ ⟨U', hUo', hpU', hU'⟩
    obtain ⟨z, ⟨hzU, hzU'⟩, hzY⟩ := mem_closure_iff.mp hp (U ∩ U') (hUo.inter hUo') ⟨hpU, hpU'⟩
    exact hd z hzY (hU ⟨hzY, hzU⟩) (hU' ⟨hzY, hzU'⟩)
  -- each proper face is on one side
  have hface : ∀ X, ProperFace cs Y X → cell cs X ⊆ Su ∨ cell cs X ⊆ Sv := fun X hX =>
    sub_or_sub (hfaces X hX) (isOpen_sideSet _ _ _) (isOpen_sideSet _ _ _)
      ((face_sub_closure hl hY hX.2.2).trans hside)
      (fun q hq => hboth q (face_sub_closure hl hY hX.2.2 hq))
  -- and incident faces are on the same side
  have hinc : ∀ (S S' : Set P), (S = Su ∧ S' = Sv) ∨ (S = Sv ∧ S' = Su) →
      ∀ X X', ProperFace cs Y X → ProperFace cs Y X' → Cov.le X X' = true →
        (cell cs X ⊆ S ↔ cell cs X' ⊆ S) := by
    intro S S' hSS' X X' hX hX' hle
    have hSo : IsOpen S := by rcases hSS' with ⟨rfl, -⟩ | ⟨rfl, -⟩ <;> exact isOpen_sideSet _ _ _
    have hS'o : IsOpen S' := by
      rcases hSS' with ⟨-, rfl⟩ | ⟨-, rfl⟩ <;> exact isOpen_sideSet _ _ _
    have hbo : ∀ p ∈ closure (cell cs Y), p ∈ S → p ∈ S' → False := by
      rcases hSS' with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · exact hboth
      · exact fun p hp h1 h2 => hboth p hp h2 h1
    have hor : ∀ Z, ProperFace cs Y Z → cell cs Z ⊆ S ∨ cell cs Z ⊆ S' := by
      intro Z hZ
      rcases hSS' with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩
      · exact hface Z hZ
      · exact (hface Z hZ).symm
    have hXX' : cell cs X ⊆ closure (cell cs X') := face_sub_closure hl hX'.1 hle
    obtain ⟨p, hp⟩ := hX.1
    have hp' : p ∈ cell cs X := hp
    constructor
    · intro hXS
      rcases hor X' hX' with h | h
      · exact h
      · obtain ⟨q, hqS, hqX'⟩ := mem_closure_iff.mp (hXX' hp') S hSo (hXS hp')
        exact (hbo q (face_sub_closure hl hY hX'.2.2 hqX') hqS (h hqX')).elim
    · intro hX'S
      rcases hor X hX with h | h
      · exact h
      · obtain ⟨q, hqS', hqX'⟩ := mem_closure_iff.mp (hXX' hp') S' hS'o (h hp')
        exact (hbo q (face_sub_closure hl hY hX'.2.2 hqX') (hX'S hqX') hqS').elim
  -- all faces on the `u` side, or all on the `v` side
  by_cases hall : ∀ X, ProperFace cs Y X → cell cs X ⊆ Su
  · exact hF v hv ⟨b, hbY, hbv⟩ (piece_closed hc hu hsub hd hall)
  · push Not at hall
    obtain ⟨X0, hX0, hX0u⟩ := hall
    have hX0v : cell cs X0 ⊆ Sv := (hface X0 hX0).resolve_left hX0u
    have hallv : ∀ X, ProperFace cs Y X → cell cs X ⊆ Sv := fun X hX =>
      hlink (fun Z => cell cs Z ⊆ Sv) (hinc Sv Su (Or.inr ⟨rfl, rfl⟩)) X0 X hX0 hX hX0v
    exact hF u hu ⟨a, haY, hau⟩
      (piece_closed hc hv (by rw [union_comm]; exact hsub) (fun y hy h1 h2 => hd y hy h2 h1) hallv)

/-- The number of zeros of a covector. -/
def nz (X : Cov) : Nat := X.count .zero

theorem nz_lt_of_proper {cs : List (Carrier P ℝ)} {Y X : Cov} (h : ProperFace cs Y X) :
    nz Y < nz X := by
  have h1 := count_le_of_le h.2.2
  rcases Nat.lt_or_ge (nz Y) (nz X) with h2 | h2
  · exact h2
  · exact absurd (eq_of_le_of_count h.2.2 (by unfold nz at h2; omega)) h.2.1

/-- What a cell needs for R1: connected outright (a vertex, a closed surface or curve), or linked
faces and no closed piece. -/
def FaceCert (cs : List (Carrier P ℝ)) (Y : Cov) : Prop :=
  IsPreconnected (cell cs Y) ∨ (FacesLinked cs Y ∧ NoClosedPiece cs Y)

/-- **R1 from the faces**, by induction on the number of nonzero entries. -/
theorem cellsConnected_of_faces {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c)
    (hl : ∀ x, LocalGood cs x) (hcert : ∀ Y, RealisedK cs Y → FaceCert cs Y) :
    CellsConnected cs := by
  intro Y hY
  induction h : cs.length + 1 - nz Y using Nat.strong_induction_on generalizing Y with
  | _ n ih =>
    rcases hcert Y hY with h1 | ⟨hlink, hF⟩
    · exact h1
    · refine connected_of_faces hc hl hY (fun X hX => ?_) hlink hF
      have hlt := nz_lt_of_proper hX
      have hle : nz X ≤ cs.length := by
        obtain ⟨x, hx⟩ := hX.1
        have hx' : covOfK cs x = X := hx
        have hlen : X.length = cs.length := by rw [← hx', length_covOfK]
        have := List.count_le_length (a := Sgn.zero) (l := X)
        unfold nz; omega
      exact ih _ (by omega) X hX.1 rfl

/-- **Full cells have no closed piece**, in a preconnected space with a wall point. -/
theorem noClosedPiece_full [PreconnectedSpace P] {cs : List (Carrier P ℝ)}
    (hc : ∀ c ∈ cs, Continuous c) (hw : ∃ x, wallK cs x) {T : Cov} (hT : Sgn.zero ∉ T) :
    NoClosedPiece cs T := by
  intro v hv hne hcl
  have hop : IsOpen (cell cs T ∩ v) := (full_open hc hT).inter hv
  rcases isClopen_iff.mp ⟨hcl, hop⟩ with h | h
  · exact hne.ne_empty h
  · obtain ⟨x, hx⟩ := hw
    have hxT : x ∈ cell cs T := (h ▸ mem_univ x : x ∈ cell cs T ∩ v).1
    have hx' : covOfK cs x = T := hxT
    exact hT (hx' ▸ (wallK_iff cs x).mp hx)

end ForgeCurved
