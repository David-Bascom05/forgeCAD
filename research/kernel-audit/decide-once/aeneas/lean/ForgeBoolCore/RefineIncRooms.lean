import ForgeBoolCore.RefineIncSplit
import ForgeBoolCore.DedupModel

/-!
# inc refinement, group S: `split`'s rooms, first seen (`room_pos`, `rooms`)

The facet loop lists each room once, in first-seen order over `[plus, minus]` per facet, which is
`IncLedger.rooms` of `toLF`. `room_pos` maps a room to its position. One insert is
`ins_spec`; the loop applies it twice per facet.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup

/-- The rooms table after seeing the room ids `xs`, each below `n`. -/
def RInv (n : Nat) (rp rs : List U32) (xs : List Nat) : Prop :=
  rs.map (·.val) = ForgeExact.DecideOnce.dedup xs ∧ rp.length = n ∧ (∀ x ∈ xs, x < n) ∧
  ∀ r, r < n → (rp.getD r NONE = NONE ↔ r ∉ xs) ∧
    (r ∈ xs → ((rs.getD (rp.getD r NONE).val 0#u32).val = r ∧ (rp.getD r NONE).val < rs.length))

theorem rinv_len {n : Nat} {rp rs : List U32} {xs : List Nat} (h : RInv n rp rs xs) : rs.length ≤ n := by
  have hnd : (rs.map (·.val)).Nodup := by rw [h.1]; exact nodup_dedup _
  have hsub : rs.map (·.val) ⊆ List.range n := by
    intro y hy; rw [h.1, mem_dedup] at hy; exact List.mem_range.mpr (h.2.2.1 y hy)
  have := hnd.length_le_of_subset hsub
  simpa using this

theorem ins_spec (n : Nat) (hn : n < U32.max) (room_pos rooms : alloc.vec.Vec U32) (xs : List Nat)
    (h : RInv n room_pos.val rooms.val xs) (p : U32) (hp : p.val < n) (i1 : U32)
    (hi1 : i1 = room_pos.val.getD p.val NONE) :
    (if i1 = inc.NONE
      then
        do
        let i2 := alloc.vec.Vec.len rooms
        let i3 ← lift (UScalar.cast .Usize p)
        let (_, index_mut_back) ←
          alloc.vec.Vec.index_mut (core.slice.index.SliceIndexUsizeSlice
            Std.U32) room_pos i3
        let i4 ← lift (UScalar.cast .U32 i2)
        let rooms2 ← alloc.vec.Vec.push rooms p
        let room_pos2 := index_mut_back i4
        ok (room_pos2, rooms2)
      else ok (room_pos, rooms)) ⦃ q => RInv n q.1.val q.2.val (xs ++ [p.val]) ⦄ := by
  obtain ⟨h1, h2, h3, h4⟩ := h
  have hpl : p.val < room_pos.val.length := by rw [h2]; exact hp
  split
  · rename_i hnone
    have hpx : p.val ∉ xs := ((h4 p.val hp).1).mp (by rw [← hi1]; exact hnone)
    have hlen := rinv_len ⟨h1, h2, h3, h4⟩
    step as ⟨i3, hi3⟩
    step as ⟨x0, bk, hbk, hx0⟩
    step as ⟨i4, hi4⟩
    step as ⟨rs2, hrs2⟩
    have hi3v : i3.val = p.val := by rw [hi3]; scalar_tac
    have hi4v : i4.val = rooms.val.length := by rw [hi4]; scalar_tac
    have hset : (bk i4).val = room_pos.val.set p.val i4 := by
      rw [hx0]; simp [alloc.vec.Vec.set_val_eq, hi3v]
    have hg : ∀ r, (bk i4).val.getD r NONE = if r = p.val then i4 else room_pos.val.getD r NONE := by
      intro r; rw [hset]; exact getD_set_any _ _ _ _ _ hpl
    refine ⟨?_, by rw [hset, List.length_set]; exact h2, fun x hx => ?_, fun r hr => ⟨?_, fun hrx => ?_⟩⟩
    · rw [hrs2, List.map_append, h1, dedup_snoc]; simp [hpx]
    · rcases List.mem_append.mp hx with hx | hx
      · exact h3 x hx
      · simp at hx; omega
    · rw [hg]
      by_cases e : r = p.val
      · subst e; simp only [ite_true]
        constructor
        · intro h0; exfalso
          have : i4.val = NONE.val := by rw [h0]
          rw [inc_none_val, hi4v] at this; omega
        · intro hn; exact absurd (List.mem_append_right _ (List.mem_singleton_self _)) hn
      · simp only [e, ite_false, List.mem_append, List.mem_singleton, or_false]
        exact (h4 r hr).1
    · rw [hg]
      by_cases e : r = p.val
      · subst e; simp only [ite_true]
        rw [hrs2, hi4v]
        refine ⟨?_, by simp⟩
        rw [List.getD_eq_getElem?_getD, List.getElem?_append_right (le_refl _)]; simp
      · simp only [e, ite_false]
        have hrx' : r ∈ xs := by
          rcases List.mem_append.mp hrx with h | h
          · exact h
          · simp at h; exact absurd h e
        obtain ⟨a1, a2⟩ := (h4 r hr).2 hrx'
        rw [hrs2]
        refine ⟨?_, by rw [List.length_append]; exact Nat.lt_of_lt_of_le a2 (Nat.le_add_right _ _)⟩
        rw [List.getD_eq_getElem?_getD, List.getElem?_append_left a2, ← List.getD_eq_getElem?_getD]; exact a1
  · rename_i hsome
    simp only [WP.spec_ok]
    have hpx : p.val ∈ xs := by
      by_contra hc; exact hsome (by rw [hi1]; exact ((h4 p.val hp).1).mpr hc)
    have e : xs ++ [p.val] = xs ++ [p.val] := rfl
    refine ⟨by rw [h1, dedup_snoc]; simp [hpx], h2, fun x hx => ?_, fun r hr => ⟨?_, fun hrx => ?_⟩⟩
    · rcases List.mem_append.mp hx with hx | hx
      · exact h3 x hx
      · simp at hx; omega
    · rw [(h4 r hr).1]; simp only [List.mem_append, List.mem_singleton]
      constructor
      · intro hn; exact fun h => h.elim hn (fun e => hn (e ▸ hpx))
      · intro hn h'; exact hn (Or.inl h')
    · have : r ∈ xs := by
        rcases List.mem_append.mp hrx with h | h
        · exact h
        · simp at h; exact h ▸ hpx
      exact (h4 r hr).2 this

end inc

end forge_bool_core
