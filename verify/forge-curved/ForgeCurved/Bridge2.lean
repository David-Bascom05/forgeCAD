import ForgeCurved.FaceLocal
import ForgeExact.Regular
import ForgeCurved.Bridge

/-!
# Route B, bridge (2): the face ledger's result is the regularised boolean

`Res G keep` (FaceLocal) is the closure of the kept rooms. L1's result is `reg (combine op A)`, the
regularised boolean of the bodies `A i`. They are equal when:
* the rooms are open and connected, miss the skeleton (facets, edges, verts: any set `Sk`), and
  cover everything off it;
* the skeleton is nowhere dense, and every body's frontier lies in it;
* each room's keep is the rule on the memberships of one witness point of it (the core's
  `memb`, from its `inside` decisions).

**`room_const_curved`**: a connected room missing a body's frontier lies inside the body or
outside it. This is the curved analogue of RoomConst.lean's `room_const`, which is flat-only
(Jordan along polylines Clear of the faces); connectedness replaces the polylines.
**`res_eq_reg`**: `Res G keep = closure (interior (combine))`, the regularised boolean, by
`reg_eq_of_sdiff`'s argument: the kept rooms are open, and they agree with the boolean off the
skeleton. **`res_eq_reg_forge`** is the same in forge-exact's words (`reg`, `combine`).
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.FaceLocal

open Topology Set

variable {P : Type} [TopologicalSpace P]

/-- **Membership is constant on a room**: a preconnected set missing a body's frontier lies
inside the body or misses it. -/
theorem room_const_curved {R A : Set P} (hR : IsPreconnected R) (hf : Disjoint R (frontier A)) :
    R ⊆ A ∨ Disjoint R A := by
  by_contra h
  simp only [not_or, not_subset, not_disjoint_iff] at h
  obtain ⟨⟨x, hxR, hxA⟩, ⟨y, hyR, hyA⟩⟩ := h
  have hy : y ∈ interior A := by
    by_contra hn
    exact Set.disjoint_left.mp hf hyR ⟨subset_closure hyA, hn⟩
  have hx : x ∈ interior Aᶜ := by
    rw [interior_compl]
    intro hc
    exact Set.disjoint_left.mp hf hxR ⟨hc, fun hi => hxA (interior_subset hi)⟩
  have hcov : R ⊆ interior A ∪ interior Aᶜ := by
    intro w hw
    by_contra hn
    simp only [mem_union, not_or] at hn
    apply Set.disjoint_left.mp hf hw
    refine ⟨?_, hn.1⟩
    by_contra hc
    exact hn.2 (by rw [interior_compl]; exact hc)
  obtain ⟨z, -, hz1, hz2⟩ := hR (interior A) (interior Aᶜ) isOpen_interior isOpen_interior hcov
    ⟨y, hyR, hy⟩ ⟨x, hxR, hx⟩
  exact (interior_subset hz2) (interior_subset hz1)

/-- The boolean of the bodies by a rule on memberships. -/
def combineR (rule : (Nat → Bool) → Bool) (A : Nat → Set P) : Set P :=
  {x | rule (fun i => @decide (x ∈ A i) (Classical.propDecidable _)) = true}

/-- **What bridge (2) needs of the builder and the core.** -/
structure ResData (G : FaceGeo P) (keep : Nat → Bool) (rule : (Nat → Bool) → Bool)
    (A : Nat → Set P) (Sk : Set P) : Prop where
  room_open : ∀ r, IsOpen (G.room r)
  room_conn : ∀ r, IsPreconnected (G.room r)
  room_skel : ∀ r, Disjoint (G.room r) Sk
  cover : ∀ x, x ∉ Sk → ∃ r, x ∈ G.room r
  skel_nd : interior (closure Sk) = ∅
  front : ∀ i, frontier (A i) ⊆ Sk
  /-- each room's keep is the rule at a witness point of it -/
  witness : ∀ r, (G.room r).Nonempty →
    ∃ w ∈ G.room r, keep r = rule (fun i => @decide (w ∈ A i) (Classical.propDecidable _))

/-- Within a room, the keep is the boolean. -/
theorem keep_iff {G : FaceGeo P} {keep : Nat → Bool} {rule : (Nat → Bool) → Bool}
    {A : Nat → Set P} {Sk : Set P} (h : ResData G keep rule A Sk) {r : Nat} {x : P}
    (hx : x ∈ G.room r) : keep r = true ↔ x ∈ combineR rule A := by
  obtain ⟨w, hw, hk⟩ := h.witness r ⟨x, hx⟩
  have hm : (fun i => @decide (w ∈ A i) (Classical.propDecidable _)) =
      fun i => @decide (x ∈ A i) (Classical.propDecidable _) := by
    funext i
    have hd : Disjoint (G.room r) (frontier (A i)) :=
      Set.disjoint_of_subset_right (h.front i) (h.room_skel r)
    rcases room_const_curved (h.room_conn r) hd with hs | hs
    · simp [hs hw, hs hx]
    · simp [Set.disjoint_left.mp hs hw, Set.disjoint_left.mp hs hx]
  rw [hk, hm]; rfl

/-- **Bridge (2)**: the face ledger's result is the regularised boolean. -/
theorem res_eq_reg {G : FaceGeo P} {keep : Nat → Bool} {rule : (Nat → Bool) → Bool}
    {A : Nat → Set P} {Sk : Set P} (h : ResData G keep rule A Sk) :
    Res G keep = closure (interior (combineR rule A)) := by
  have hKo : IsOpen (Kept G keep) :=
    isOpen_iUnion fun r => isOpen_iUnion fun _ => h.room_open r
  -- the kept rooms and the boolean agree off the skeleton
  have hagree : ∀ x, x ∉ Sk → (x ∈ Kept G keep ↔ x ∈ combineR rule A) := by
    intro x hx
    obtain ⟨r, hr⟩ := h.cover x hx
    constructor
    · intro hK
      obtain ⟨r', hr'⟩ := Set.mem_iUnion.mp hK
      obtain ⟨hk, hx'⟩ := Set.mem_iUnion.mp hr'
      exact (keep_iff h hx').mp hk
    · intro hc
      exact room_sub_kept ((keep_iff h hr).mpr hc) hr
  -- the regularisation argument: equal off a nowhere-dense set
  have hsub : ∀ {S T : Set P}, (∀ x, x ∉ Sk → x ∈ S → x ∈ T) →
      closure (interior S) ⊆ closure (interior T) := by
    intro S T hST
    refine closure_minimal (fun x hx => ?_) isClosed_closure
    rw [mem_closure_iff]
    intro U hU hxU
    -- U ∩ interior S is a nonempty open set, so it is not inside closure Sk
    have hW : IsOpen (U ∩ interior S) := hU.inter isOpen_interior
    have : ¬(U ∩ interior S ⊆ closure Sk) := fun hc => by
      have := interior_maximal hc hW
      rw [h.skel_nd] at this
      exact this ⟨hxU, hx⟩
    obtain ⟨y, ⟨hyU, hyS⟩, hyZ⟩ := not_subset.mp this
    -- a neighbourhood of y off Sk, inside U ∩ interior S, lies in T
    have hV : IsOpen (U ∩ interior S ∩ (closure Sk)ᶜ) := hW.inter isClosed_closure.isOpen_compl
    refine ⟨y, hyU, interior_maximal (fun w (hw : w ∈ U ∩ interior S ∩ (closure Sk)ᶜ) =>
      hST w (fun hs => hw.2 (subset_closure hs)) (interior_subset hw.1.2)) hV ⟨⟨hyU, hyS⟩, hyZ⟩⟩
  unfold Res
  rw [← hKo.interior_eq]
  apply Subset.antisymm
  · exact hsub fun x hx hK => (hagree x hx).mp hK
  · exact hsub fun x hx hc => (hagree x hx).mpr hc

/-- **Bridge (2), in forge-exact's words**: `Res` is `reg (combine (ruleOp rule) A)`. -/
theorem res_eq_reg_forge {G : FaceGeo P} {keep : Nat → Bool} {rule : (Nat → Bool) → Bool}
    {A : Nat → Set P} {Sk : Set P} (h : ResData G keep rule A Sk) :
    Res G keep = {x | ForgeExact.reg (ForgeExact.combine
      (fun m : Nat → Prop => rule (fun i => @decide (m i) (Classical.propDecidable _)) = true)
      (fun i (y : P) => y ∈ A i)) x} := by
  rw [res_eq_reg h]
  ext x
  simp only [ForgeExact.reg, mem_setOf_eq]
  rw [ForgeCurved.closure_iff, ← ForgeCurved.interior_eq]
  rfl

end ForgeCurved.FaceLocal
