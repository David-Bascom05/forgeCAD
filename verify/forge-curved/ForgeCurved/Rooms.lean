import ForgeCurved.FaceLocal

/-!
# F5, the rooms: connected by the flood, one membership class each, and `R = reg (combine …)`

Route B (curved-design.md §7): the face ledger's rooms are the open connected components of the
region off the faces. The builder finds each room by a flood over pieces (its local cells),
certifying every step. This file gives the facts F5 needs about rooms; `SideLocal`
(FaceLocal.lean) has the local ones at facet points.

* **`room_preconnected`**: a room covered by the closures of preconnected pieces inside it, whose
  flood steps join pieces with closures meeting in the room (a shared split facet, or the end of a
  certified path), is preconnected.
* **`room_class`**: a preconnected room that misses the frontier of `S` lies in `S` or misses it.
  So with every body's boundary on the faces, membership is constant on each room.
* **`res_eq_reg`**: if the rooms are open, connected, cover the region off a nowhere-dense set,
  miss every body's frontier, and `keep` is `op` read at one point of each room, then
  `Res = closure (kept rooms)` is the regularised boolean `reg (combine op A)`. This uses
  `ForgeExact.reg_eq_of_sdiff`.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.Rooms

open ForgeExact ForgeCurved ForgeCurved.FaceLocal Set Topology Relation

variable {P : Type} [TopologicalSpace P]

/-- **A flooded room is connected.** -/
theorem room_preconnected (room : Set P) {ι : Type} (Q : ι → Set P)
    (hQ : ∀ j, IsPreconnected (Q j)) (hsub : ∀ j, Q j ⊆ room) (hcov : room ⊆ ⋃ j, closure (Q j))
    (hstep : ∀ j j', ReflTransGen (fun a b => (closure (Q a) ∩ closure (Q b) ∩ room).Nonempty) j j') :
    IsPreconnected room := by
  have hS : ∀ j, IsPreconnected (closure (Q j) ∩ room) := fun j =>
    (hQ j).subset_closure (fun y hy => ⟨subset_closure hy, hsub j hy⟩) fun y hy => hy.1
  have heq : room = ⋃ j, closure (Q j) ∩ room := by
    ext y
    constructor
    · intro hy
      obtain ⟨j, hj⟩ := mem_iUnion.mp (hcov hy)
      exact mem_iUnion.mpr ⟨j, hj, hy⟩
    · intro hy
      obtain ⟨j, hj⟩ := mem_iUnion.mp hy
      exact hj.2
  rw [heq]
  refine IsPreconnected.iUnion_of_reflTransGen hS fun j j' => ?_
  induction hstep j j' with
  | refl => exact ReflTransGen.refl
  | tail _ hab ih =>
    obtain ⟨y, ⟨ha, hb⟩, hy⟩ := hab
    exact ih.tail ⟨y, ⟨ha, hy⟩, ⟨hb, hy⟩⟩

/-- **One membership class per room.** -/
theorem room_class {room S : Set P} (hconn : IsPreconnected room)
    (hfr : Disjoint room (frontier S)) : room ⊆ S ∨ Disjoint room S := by
  have hcov : room ⊆ interior S ∪ interior Sᶜ := by
    intro y hy
    by_cases hI : y ∈ interior S
    · exact Or.inl hI
    · right
      rw [interior_compl]
      intro hcl
      exact Set.disjoint_left.mp hfr hy ⟨hcl, hI⟩
  have hd : Disjoint (interior S) (interior Sᶜ) :=
    Set.disjoint_left.mpr fun y h1 h2 => interior_subset h2 (interior_subset h1)
  rcases hconn.subset_or_subset isOpen_interior isOpen_interior hd hcov with h | h
  · exact Or.inl (h.trans interior_subset)
  · exact Or.inr (Set.disjoint_left.mpr fun y hy hS => interior_subset (h hy) hS)

/-- **The result is the regularised boolean.** -/
theorem res_eq_reg (G : FaceGeo P) (keep : Nat → Bool) {ι : Type} (op : (ι → Prop) → Prop)
    (A : ι → Pred P) (Z : Set P) (hZ : interior (closure Z) = ∅)
    (hopen : ∀ r, IsOpen (G.room r))
    (hdisj : ∀ r r', keep r ≠ keep r' → Disjoint (G.room r) (G.room r'))
    (hcov : ∀ x, x ∉ Z → ∃ r, x ∈ G.room r) (hconn : ∀ r, IsPreconnected (G.room r))
    (hfr : ∀ r i, Disjoint (G.room r) (frontier {x | A i x}))
    (hkeep : ∀ r, (G.room r).Nonempty → ∃ x₀ ∈ G.room r, (keep r = true ↔ combine op A x₀)) :
    Res G keep = {x | reg (combine op A) x} := by
  -- membership, hence the boolean, is constant on each room
  have hconst : ∀ r, ∀ x ∈ G.room r, (keep r = true ↔ combine op A x) := by
    intro r x hx
    obtain ⟨x₀, hx₀, hk⟩ := hkeep r ⟨x, hx⟩
    have hpat : (fun i => A i x₀) = fun i => A i x := by
      funext i
      apply propext
      rcases room_class (hconn r) (hfr r i) with h | h
      · exact ⟨fun _ => h hx, fun _ => h hx₀⟩
      · exact ⟨fun h' => absurd h' (Set.disjoint_left.mp h hx₀),
          fun h' => absurd h' (Set.disjoint_left.mp h hx)⟩
    rw [hk]
    show op (fun i => A i x₀) ↔ op (fun i => A i x)
    rw [hpat]
  have hKopen : IsOpen (Kept G keep) :=
    isOpen_iUnion fun r => isOpen_iUnion fun _ => hopen r
  -- `Res` is the regularisation of the kept rooms
  have h1 : Res G keep = {x | reg (fun y => y ∈ Kept G keep) x} := by
    ext x
    show x ∈ closure (Kept G keep) ↔ ForgeExact.Topology.closure
      (ForgeExact.Topology.interior fun y => y ∈ Kept G keep) x
    rw [closure_iff]
    have : {y | ForgeExact.Topology.interior (fun y => y ∈ Kept G keep) y} = Kept G keep := by
      rw [interior_eq]; exact hKopen.interior_eq
    rw [this]
  rw [h1]
  have hND : ForgeExact.Topology.NowhereDense fun y => y ∈ Z := by
    rw [nowhereDense_iff]; simpa using hZ
  have h2 := reg_eq_of_sdiff (S := fun y => y ∈ Kept G keep) (T := combine op A) hND (by
    intro x hx
    by_contra hxZ
    obtain ⟨r, hr⟩ := hcov x hxZ
    have hc := hconst r x hr
    rcases hx with ⟨hK, hnc⟩ | ⟨hc', hnK⟩
    · obtain ⟨r', hr'⟩ := mem_iUnion.mp hK
      obtain ⟨hk', hx'⟩ := mem_iUnion.mp hr'
      by_cases hkr : keep r = true
      · exact hnc (hc.mp hkr)
      · exact Set.disjoint_left.mp (hdisj r r' (by rw [hk']; simpa using hkr)) hr hx'
    · exact hnK (room_sub_kept (hc.mpr hc') hr))
  rw [h2]

end ForgeCurved.Rooms
