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
        let (_, index_mut_back) ← alloc.vec.Vec.index_mut_usize room_pos i3
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

/-- The room ids of the first `f` facets, `[plus, minus]` each. -/
def seenRooms (fr : List (U32 × U32)) (f : Nat) : List Nat :=
  (List.range f).flatMap fun g => [(fr.getD g (0#u32, 0#u32)).1.val, (fr.getD g (0#u32, 0#u32)).2.val]

theorem seenRooms_succ (fr : List (U32 × U32)) (f : Nat) :
    seenRooms fr (f + 1) = seenRooms fr f ++ [(fr.getD f (0#u32, 0#u32)).1.val] ++ [(fr.getD f (0#u32, 0#u32)).2.val] := by
  simp [seenRooms, List.range_succ, List.flatMap_append]

theorem split_loop1_spec (fr : alloc.vec.Vec (U32 × U32)) (nf : Usize) (n : Nat) (hn : n < U32.max)
    (hfr : fr.val.length = nf.val) (hlt : ∀ q ∈ fr.val, q.1.val < n ∧ q.2.val < n)
    (room_pos rooms : alloc.vec.Vec U32) (f : Usize) (hf : f.val ≤ nf.val)
    (h : RInv n room_pos.val rooms.val (seenRooms fr.val f.val)) :
    split_loop1 fr nf room_pos rooms f ⦃ q => RInv n q.1.val q.2.val (seenRooms fr.val nf.val) ⦄ := by
  unfold split_loop1
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) => nf.val - q.2.2.val)
    (fun q => q.2.2.val ≤ nf.val ∧ RInv n q.1.val q.2.1.val (seenRooms fr.val q.2.2.val))
  · rintro ⟨rp, rs, g⟩ ⟨hg, hI⟩
    unfold split_loop1.body
    simp only at hg hI ⊢
    split
    · rename_i hlt'
      have hgl : g.val < fr.val.length := by rw [hfr]; scalar_tac
      step as ⟨p, m, hpm⟩
      have hq : fr.val.getD g.val (0#u32, 0#u32) = (p, m) := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hgl]; simp [hpm]
      have hpn : p.val < n ∧ m.val < n := by
        have := hlt _ (List.getElem_mem hgl); rw [← hpm] at this; exact this
      step as ⟨i, hi⟩
      have hiv : i.val = p.val := by rw [hi]; scalar_tac
      have hil : i.val < rp.val.length := by rw [hiv, hI.2.1]; exact hpn.1
      step as ⟨i1, hi1⟩
      step with ins_spec n hn rp rs _ hI p hpn.1 i1
        (by rw [hi1, List.getD_eq_getElem?_getD, ← hiv, List.getElem?_eq_getElem hil]; rfl) as ⟨rp1, rs1, hq1⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = m.val := by rw [hi2]; scalar_tac
      have hil2 : i2.val < rp1.val.length := by rw [hi2v, hq1.2.1]; exact hpn.2
      step as ⟨i3, hi3⟩
      step with ins_spec n hn rp1 rs1 _ hq1 m hpn.2 i3
        (by rw [hi3, List.getD_eq_getElem?_getD, ← hi2v, List.getElem?_eq_getElem hil2]; rfl) as ⟨rp2, rs2, hq2⟩
      step as ⟨g1, hg1⟩
      have : g1.val = g.val + 1 := by scalar_tac
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [this, seenRooms_succ, hq]; exact hq2
    · rename_i hge
      simp only [WP.spec_ok]
      have : g.val = nf.val := by scalar_tac
      rw [← this]; exact hI
  · exact ⟨hf, h⟩

theorem key_eq {nb c c' b b' : Nat} (hb : b < nb) (hb' : b' < nb) (e : c * nb + b = c' * nb + b') :
    c = c' ∧ b = b' := by
  have hnb : 0 < nb := by omega
  have m1 : (c * nb + b) % nb = b := by rw [Nat.add_comm, Nat.add_mul_mod_self_right, Nat.mod_eq_of_lt hb]
  have m2 : (c' * nb + b') % nb = b' := by rw [Nat.add_comm, Nat.add_mul_mod_self_right, Nat.mod_eq_of_lt hb']
  have d1 : (c * nb + b) / nb = c := by
    rw [Nat.add_comm, Nat.add_mul_div_right _ _ hnb, Nat.div_eq_of_lt hb, Nat.zero_add]
  have d2 : (c' * nb + b') / nb = c' := by
    rw [Nat.add_comm, Nat.add_mul_div_right _ _ hnb, Nat.div_eq_of_lt hb', Nat.zero_add]
  exact ⟨by rw [← d1, ← d2, e], by rw [← m1, ← m2, e]⟩

/-- `piece_of` after the first `k` pieces. -/
def POInv (l : Ledger) (po : List Bool) (k : Nat) : Prop :=
  k ≤ l.pieces.val.length ∧ po.length = nfacets l * l.nb.val ∧
  ∀ c, c < nfacets l → ∀ b, b < l.nb.val →
    (po.getD (c * l.nb.val + b) false = true ↔ ∃ j, j < k ∧ pcF l j = c ∧ (toLF l).bodyOf (pfF l j) = b)

theorem split_loop0_spec (l : Ledger) (hw : LedgerWF2 l) (hs : (toLF l).splitOk = true)
    (po : alloc.vec.Vec Bool) (k : Usize) (h : POInv l po.val k.val) :
    split_loop0 l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets l.vert_edges l.vert_sectors (UScalar.cast .Usize l.nb) po k ⦃ r =>
      POInv l r.val l.pieces.val.length ⦄ := by
  unfold split_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Bool × Usize) => l.pieces.val.length - q.2.val)
    (fun q => POInv l q.1.val q.2.val)
  · rintro ⟨p, j⟩ ⟨hj, hlen, hP⟩
    unfold split_loop0.body
    simp only at hj hlen hP ⊢
    have hnbv : (UScalar.cast UScalarTy.Usize l.nb : Usize).val = l.nb.val := by scalar_tac
    split
    · rename_i hlt
      have hjl : j.val < l.pieces.val.length := by scalar_tac
      step as ⟨c2, f, hcf⟩
      have hpj : l.pieces.val.getD j.val default = (c2, f) := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]; simp [hcf]
      have hc2 : pcF l j.val = c2.val := by unfold pcF; rw [hpj]
      have hf2 : pfF l j.val = (f : U32).val := by unfold pfF; rw [hpj]
      have hok : PieceOK l c2.val (f : U32).val := by
        have := ((splitOk_iff l).mp hs).1 _ (pieces_mem l hjl); rw [hpj] at this; exact this
      step with body_of_spec l f as ⟨i3, hi3⟩
      step as ⟨b, hb⟩
      have hbv : b.val = (toLF l).bodyOf (f : U32).val := by rw [hb, ← hi3]; scalar_tac
      have hbn : b.val < l.nb.val := by rw [hbv]; exact hok.2.2.1
      have hsm := hw.small
      have hkey : c2.val * l.nb.val + b.val < nfacets l * l.nb.val := by
        have : c2.val + 1 ≤ nfacets l := hok.1
        have := Nat.mul_le_mul_right l.nb.val this
        rw [Nat.add_mul, Nat.one_mul] at this; omega
      split
      · rename_i hbl
        step as ⟨i4, hi4⟩
        step as ⟨i5, hi5⟩
        step as ⟨i6, hi6⟩
        have hi6v : i6.val = c2.val * l.nb.val + b.val := by scalar_tac
        have hi6l : i6.val < p.val.length := by rw [hlen, hi6v]; exact hkey
        step as ⟨x0, _, hbk⟩
        obtain ⟨x1, bk⟩ := x0
        simp only at hbk ⊢
        rw [hbk]
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨⟨by omega, by simp [hlen], ?_⟩, by omega⟩
        intro c hc b' hb'
        rw [hj1v]
        simp only [alloc.vec.Vec.set_val_eq]
        by_cases hk : c * l.nb.val + b' = i6.val
        · rw [hi6v] at hk
          obtain ⟨rfl, rfl⟩ := key_eq hb' hbn hk
          rw [hi6v, List.getD_eq_getElem?_getD, List.getElem?_set_self (by rw [← hlen] at hkey; exact hkey)]
          simp only [Option.getD_some, true_iff]
          exact ⟨j.val, by omega, hc2, by rw [hf2, ← hbv]⟩
        · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm hk), ← List.getD_eq_getElem?_getD, hP c hc b' hb']
          constructor
          · rintro ⟨j', hj', h1, h2⟩; exact ⟨j', by omega, h1, h2⟩
          · rintro ⟨j', hj', h1, h2⟩
            refine ⟨j', ?_, h1, h2⟩
            rcases Nat.lt_succ_iff_lt_or_eq.mp hj' with h | h
            · exact h
            · exfalso; apply hk; rw [hi6v, ← h1, ← h2, h, hc2, hf2, hbv]
      · rename_i hbl
        exfalso; apply hbl; scalar_tac

    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = l.pieces.val.length := by scalar_tac
      rw [← this]; exact ⟨hj, hlen, hP⟩
  · exact h

/-- What `split` returns: the piece table is `isPieceOf`, the rooms are `rooms`, `room_pos` inverts them. -/
structure SplitOK (l : Ledger) (s : Split) : Prop where
  piece_len : s.piece_of.val.length = nfacets l * l.nb.val
  piece_of : ∀ c, c < nfacets l → ∀ b, b < l.nb.val →
    s.piece_of.val.getD (c * l.nb.val + b) false = (toLF l).isPieceOf b (.facet c)
  rinv : RInv l.n_rooms.val s.room_pos.val s.rooms.val (seenRooms l.facet_rooms.val (nfacets l))
  rooms : s.rooms.val.map (fun r => FCell.room r.val) = (toLF l).rooms

theorem isPieceOf_facet (l : Ledger) (c b : Nat) :
    (toLF l).isPieceOf b (.facet c) = true ↔
      ∃ j, j < l.pieces.val.length ∧ pcF l j = c ∧ (toLF l).bodyOf (pfF l j) = b := by
  rw [IncLedger.isPieceOf_iff]
  constructor
  · rintro ⟨f, hm, hb⟩
    simp only [toLF, List.mem_map] at hm
    obtain ⟨q, hq, he⟩ := hm
    obtain ⟨j, hj, rfl⟩ := List.mem_iff_getElem.mp hq
    have hg : l.pieces.val.getD j default = l.pieces.val[j] := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj]; rfl
    simp only [Prod.mk.injEq, FCell.facet.injEq] at he
    refine ⟨j, hj, ?_, ?_⟩
    · unfold pcF; rw [hg]; exact he.1
    · unfold pfF; rw [hg, he.2]; exact hb
  · rintro ⟨j, hj, h1, h2⟩
    refine ⟨pfF l j, ?_, h2⟩
    simp only [toLF, List.mem_map]
    refine ⟨l.pieces.val.getD j default, pieces_mem l hj, ?_⟩
    rw [← h1]; rfl

theorem toLF_rooms (l : Ledger) (hw : LedgerWF2 l) :
    (toLF l).rooms = (ForgeExact.DecideOnce.dedup (seenRooms l.facet_rooms.val (nfacets l))).map FCell.room := by
  have e : ((toLF l).facets.flatMap fun φ => [(toLF l).plus φ, (toLF l).minus φ]) =
      (seenRooms l.facet_rooms.val (nfacets l)).map FCell.room := by
    simp [toLF, seenRooms, List.flatMap_map, List.map_flatMap, facetNum]
  unfold IncLedger.rooms; rw [e]
  exact dedup_map _ _ (fun a _ b _ h => by cases h; rfl)

theorem split_spec (l : Ledger) (hw : LedgerWF2 l) (hs : (toLF l).splitOk = true) :
    split l ⦃ s => SplitOK l s ⦄ := by
  unfold split
  have hsm := hw.small
  step as ⟨nb, hnb⟩
  have hnbv : nb.val = l.nb.val := by rw [hnb]; scalar_tac
  have hmul : (alloc.vec.Vec.len l.facet_class).val * nb.val ≤ Usize.max := by
    rw [hnbv]; simpa [nfacets] using hsm.2.2.2.2
  step as ⟨i, hi⟩
  step with alloc.vec.from_elem_spec core.clone.CloneBool false i (by rfl) as ⟨po, hp1, hp2⟩
  have h0 : POInv l po.val (0#usize).val := by
    refine ⟨by simp, by rw [hp1, List.length_replicate, hi, hnbv]; simp [nfacets], ?_⟩
    intro c hc b hb
    rw [hp1, List.getD_eq_getElem?_getD, List.getElem?_replicate]
    constructor
    · intro h; exfalso; revert h; split <;> simp
    · rintro ⟨j, hj, _⟩; simp at hj
  rw [hnb]
  step with split_loop0_spec l hw hs po 0#usize h0 as ⟨po1, hpo1⟩
  step as ⟨i1, hi1⟩
  step with alloc.vec.from_elem_spec core.clone.CloneU32 NONE i1 (by rfl) as ⟨rp, hr1, hr2⟩
  have hR : RInv l.n_rooms.val rp.val (alloc.vec.Vec.new U32).val
      (seenRooms l.facet_rooms.val (0#usize).val) := by
    have hi1v : i1.val = l.n_rooms.val := by scalar_tac
    refine ⟨by simp [seenRooms, ForgeExact.DecideOnce.dedup, dedupAux], by rw [hr1, List.length_replicate, hi1v],
      by simp [seenRooms], fun r hr => ⟨?_, by simp [seenRooms]⟩⟩
    rw [hr1, List.getD_eq_getElem?_getD, List.getElem?_replicate]
    have e0 : seenRooms l.facet_rooms.val (0#usize).val = [] := by simp [seenRooms]
    rw [e0]; simp only [List.not_mem_nil, not_false_eq_true, iff_true]
    split <;> rfl
  step with split_loop1_spec l.facet_rooms (alloc.vec.Vec.len l.facet_class) l.n_rooms.val hsm.2.1
    (by simpa [nfacets] using hw.rooms.1) (fun q hq => ⟨(hw.rooms.2 q hq).1, (hw.rooms.2 q hq).2.1⟩)
    rp _ 0#usize (by simp) hR as ⟨rp1, rs, hq⟩
  have hlen : (alloc.vec.Vec.len l.facet_class).val = nfacets l := by simp [nfacets]
  rw [hlen] at hq
  refine ⟨hpo1.2.1, fun c hc b hb => ?_, hq, ?_⟩
  · rw [Bool.eq_iff_iff, hpo1.2.2 c hc b hb, isPieceOf_facet]
  · rw [toLF_rooms l hw, ← hq.1, List.map_map]; rfl

end inc

end forge_bool_core
