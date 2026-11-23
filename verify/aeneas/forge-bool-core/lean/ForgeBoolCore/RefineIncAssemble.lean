import ForgeBoolCore.RefineIncClassify

/-!
# Group K: the inc result, assembled

`inc::core_k`'s first loop lists the result's facets (those whose two rooms differ in keep), in
facet order, with each one's position (`rpos`) and sense. Here it is `result` and `orient` of the
keep table, `keepTab (ruleL nb r)`.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- The keep of facet `f`'s two rooms, by the keep vector. -/
def kpF (l : Ledger) (s : Split) (keep : List Bool) (f : Nat) : Bool := keep.getD (posR s (plusR l f)) false
def kmF (l : Ledger) (s : Split) (keep : List Bool) (f : Nat) : Bool := keep.getD (posR s (minusR l f)) false

/-- Facet `f` is in the result. -/
def inR (l : Ledger) (s : Split) (keep : List Bool) (f : Nat) : Bool := kpF l s keep f != kmF l s keep f

/-- The result's facets among the first `k`. -/
def resL (l : Ledger) (s : Split) (keep : List Bool) (k : Nat) : List Nat := (List.range k).filter (inR l s keep)

def oriR (l : Ledger) (s : Split) (keep : List Bool) (f : Nat) : Sgn := if kmF l s keep f then .Pos else .Neg

theorem resL_succ (l : Ledger) (s : Split) (keep : List Bool) (k : Nat) :
    resL l s keep (k + 1) = resL l s keep k ++ (if inR l s keep k then [k] else []) := by
  simp only [resL, List.range_succ, List.filter_append, List.filter_cons, List.filter_nil]

theorem resL_lt {l : Ledger} {s : Split} {keep : List Bool} {k x : Nat} (h : x ∈ resL l s keep k) : x < k := by
  simp only [resL, List.mem_filter, List.mem_range] at h; exact h.1

/-- The assembled tables after the first `k` facets. -/
structure AInv (l : Ledger) (s : Split) (keep : List Bool) (res rpos : List U32) (ori : List Sgn) (k : Nat) :
    Prop where
  le : k ≤ nfacets l
  res : res.map (·.val) = resL l s keep k
  rpos_len : rpos.length = nfacets l
  rpos : ∀ g, g < nfacets l → (rpos.getD g NONE).val =
    if g < k ∧ inR l s keep g = true then (resL l s keep g).length else U32.max
  ori : ori = (resL l s keep k).map (oriR l s keep)

theorem core_k_loop0_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s)
    (keep : Slice Bool) (hk : keep.val.length = s.rooms.val.length)
    (res rpos : alloc.vec.Vec U32) (ori : alloc.vec.Vec Sgn) (f : Usize)
    (h : AInv l s keep.val res.val rpos.val ori.val f.val) :
    core_k_loop0 l.facet_rooms s keep (alloc.vec.Vec.len l.facet_class) res rpos ori f ⦃ r =>
      AInv l s keep.val r.1.val r.2.1.val r.2.2.val (nfacets l) ⦄ := by
  have hsm := hw.small
  have hfrl := hw.rooms.1
  have hrpl := hs.rinv.2.1
  unfold core_k_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × alloc.vec.Vec U32 × alloc.vec.Vec Sgn × Usize) => nfacets l - q.2.2.2.val)
    (fun q => AInv l s keep.val q.1.val q.2.1.val q.2.2.1.val q.2.2.2.val)
  · rintro ⟨rs, rp, os, j⟩ hI
    have hj := hI.le
    unfold core_k_loop0.body
    simp only at hI hj ⊢
    split
    · rename_i hlt
      have hjl : j.val < nfacets l := by simp [nfacets]; scalar_tac
      have hjf : j.val < l.facet_rooms.val.length := by rw [hfrl]; exact hjl
      step as ⟨p, m, hpm⟩
      have hpmg : l.facet_rooms.val.getD j.val (0#u32, 0#u32) = (p, m) := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjf]; simp [hpm]
      clear hpm
      have hfr := facet_rooms_get l hw hjl
      have hpv : plusR l j.val = p.val := by unfold plusR; rw [hpmg]
      have hmv : minusR l j.val = m.val := by unfold minusR; rw [hpmg]
      clear hpmg
      have hpl := pos_lt l hw s hs hfr.1 (plusR_mem l hjl)
      have hml := pos_lt l hw s hs hfr.2 (minusR_mem l hjl)
      step as ⟨i, hi⟩
      have hiv : i.val = p.val := by scalar_tac
      step as ⟨i1, hi1⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = posR s (plusR l j.val) := by
        rw [hpv, posR, ← hiv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
        simp only [Option.getD_some]; rw [← hi1]; scalar_tac
      step as ⟨kp, hkp⟩
      have hkpv : kp = kpF l s keep.val j.val := by
        rw [hkp, kpF, ← hi2v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = m.val := by scalar_tac
      step as ⟨i4, hi4⟩
      step as ⟨i5, hi5⟩
      have hi5v : i5.val = posR s (minusR l j.val) := by
        rw [hmv, posR, ← hi3v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
        simp only [Option.getD_some]; rw [← hi4]; scalar_tac
      step as ⟨km, hkm⟩
      have hkmv : km = kmF l s keep.val j.val := by
        rw [hkm, kmF, ← hi5v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
      have hrl : rs.val.length = (resL l s keep.val j.val).length := by
        have := congrArg List.length hI.res; simpa using this
      have hrlt : (resL l s keep.val j.val).length ≤ j.val := by
        have := List.length_filter_le (inR l s keep.val) (List.range j.val); simpa [resL] using this
      split
      · rename_i hne
        have hin : inR l s keep.val j.val = true := by
          rw [inR, ← hkpv, ← hkmv]; simpa using hne
        have hrpj : j.val < rp.val.length := by rw [hI.rpos_len]; exact hjl
        step as ⟨x0, _, hbk⟩
        obtain ⟨x1, bk⟩ := x0
        simp only at hbk ⊢
        step as ⟨i7, hi7⟩
        have hi7v : i7.val = (resL l s keep.val j.val).length := by
          rw [← hrl]; scalar_tac
        step as ⟨i8, hi8⟩
        have hi8v : i8.val = j.val := by scalar_tac
        have hrsl : rs.val.length < Usize.max := by scalar_tac
        step as ⟨rs1, hrs1⟩
        have hpost : ∀ (os1 : alloc.vec.Vec Sgn), os1.val = os.val ++ [oriR l s keep.val j.val] →
            ∀ (j1 : Usize), j1.val = j.val + 1 →
            AInv l s keep.val rs1.val (bk i7).val os1.val j1.val := by
          intro os1 hos1 j1 hj1v
          refine ⟨by omega, ?_, ?_, ?_, ?_⟩
          · rw [hrs1, hj1v, resL_succ, if_pos hin, List.map_append, hI.res]; simp [hi8v]
          · rw [hbk]; simp [hI.rpos_len]
          · intro g hg
            rw [hbk, alloc.vec.Vec.set_val_eq, hj1v]
            by_cases hgj : g = j.val
            · subst hgj
              rw [List.getD_eq_getElem?_getD, List.getElem?_set_self hrpj]
              simp [hin, hi7v]
            · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm hgj), ← List.getD_eq_getElem?_getD,
                hI.rpos g hg]
              have : (g < j.val + 1) ↔ g < j.val := by omega
              simp only [this]
          · rw [hos1, hI.ori, hj1v, resL_succ, if_pos hin]; simp
        split
        · rename_i hkm1
          have hpush : os.val.length < Usize.max := by
            have := congrArg List.length hI.ori; simp at this; scalar_tac
          step as ⟨os1, hos1⟩
          step as ⟨j1, hj1⟩
          refine ⟨hpost os1 (by rw [hos1, oriR, ← hkmv, hkm1]; simp) j1 (by scalar_tac), by scalar_tac⟩
        · rename_i hkm1
          have hpush : os.val.length < Usize.max := by
            have := congrArg List.length hI.ori; simp at this; scalar_tac
          step as ⟨os1, hos1⟩
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by clear hkm1; scalar_tac
          have : km = false := by simpa using hkm1
          refine ⟨hpost os1 (by rw [hos1, oriR, ← hkmv, this]; simp) j1 hj1v, by omega⟩
      · rename_i hne
        have hin : inR l s keep.val j.val = false := by
          rw [inR, ← hkpv, ← hkmv]; simpa using hne
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨⟨by omega, ?_, hI.rpos_len, ?_, ?_⟩, by omega⟩
        · rw [hj1v, resL_succ, if_neg (by simp [hin]), List.append_nil]; exact hI.res
        · intro g hg
          rw [hI.rpos g hg, hj1v]
          by_cases hgj : g = j.val
          · subst hgj; simp [hin]
          · have : (g < j.val + 1) ↔ g < j.val := by omega
            simp only [this]
        · rw [hj1v, resL_succ, if_neg (by simp [hin]), List.append_nil]; exact hI.ori
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nfacets l := by unfold nfacets at hj ⊢; scalar_tac
      rw [← this]; exact hI
  · exact h

/-! ### The Lean side -/

/-- The keep table's entry for a room it lists is the room's keep. -/
theorem keepInI_tab {α : Type} [DecidableEq α] [BEq α] [LawfulBEq α] (L : IncLedger α) (rule : ForgeExact.DecideOnce.Rule)
    {T : α} (hT : T ∈ L.rooms) : keepInI (L.keepTab rule) T = L.keep rule T := by
  unfold keepInI IncLedger.keepTab
  cases hf : (L.rooms.map fun T => (T, L.keep rule T)).find? (fun e => e.1 == T) with
  | none =>
    exfalso
    have := List.find?_eq_none.mp hf (T, L.keep rule T) (List.mem_map.mpr ⟨T, hT, rfl⟩)
    simp at this
  | some e =>
    have hm := List.mem_of_find?_eq_some hf
    have he := List.find?_some hf
    obtain ⟨x, _, rfl⟩ := List.mem_map.mp hm
    simp only [beq_iff_eq] at he
    subst he; rfl

/-- The keep vector `core_loop1` builds. -/
def keepVec (l : Ledger) (s : Split) (rule : Rule) : List Bool :=
  (List.range s.rooms.val.length).map fun t => applyL rule (colI l s t)

theorem room_mem (l : Ledger) (hw : LedgerWF2 l) {x : Nat}
    (hm : x ∈ seenRooms l.facet_rooms.val (nfacets l)) : FCell.room x ∈ (toLF l).rooms := by
  rw [toLF_rooms l hw, List.mem_map]
  exact ⟨x, (mem_dedup _ _).mpr hm, rfl⟩

/-- A room's keep, by the vector, is the Lean's keep-table entry. -/
theorem keepVec_room (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule) {x : Nat}
    (hx : x < l.n_rooms.val) (hm : x ∈ seenRooms l.facet_rooms.val (nfacets l)) :
    (keepVec l s rule).getD (posR s x) false =
      keepInI ((toLF l).keepTab (ruleL l.nb.val rule)) (FCell.room x) := by
  have hp := pos_lt l hw s hs hx hm
  rw [keepInI_tab _ _ (room_mem l hw hm), keepVec, List.getD_eq_getElem?_getD,
    List.getElem?_map, List.getElem?_range hp, Option.map_some, Option.getD_some, keep_eq]
  have he := ((hs.rinv.2.2.2 x hx).2 hm).1
  change (s.rooms.val.getD (posR s x) 0#u32).val = x at he
  rw [he]

/-- A keep vector, by room position, agrees with a Lean keep table on the rooms. -/
def KeepRel (l : Ledger) (s : Split) (kv : List Bool) (kt : KeepTabI FCell) : Prop :=
  ∀ x, x < l.n_rooms.val → x ∈ seenRooms l.facet_rooms.val (nfacets l) →
    kv.getD (posR s x) false = keepInI kt (FCell.room x)

theorem keepVec_rel (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule) :
    KeepRel l s (keepVec l s rule) ((toLF l).keepTab (ruleL l.nb.val rule)) :=
  fun _ hx hm => keepVec_room l hw s hs rule hx hm

theorem kpF_eq (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (kv : List Bool) (kt : KeepTabI FCell) (hk : KeepRel l s kv kt) {f : Nat}
    (hf : f < nfacets l) :
    kpF l s (kv) f = keepInI (kt) ((toLF l).plus (FCell.facet f)) :=
  hk _ (facet_rooms_get l hw hf).1 (plusR_mem l hf)

theorem kmF_eq (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (kv : List Bool) (kt : KeepTabI FCell) (hk : KeepRel l s kv kt) {f : Nat}
    (hf : f < nfacets l) :
    kmF l s (kv) f = keepInI (kt) ((toLF l).minus (FCell.facet f)) :=
  hk _ (facet_rooms_get l hw hf).2 (minusR_mem l hf)

/-- **The assembled facets are the Lean's `result`.** -/
theorem result_eq (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (kv : List Bool) (kt : KeepTabI FCell) (hk : KeepRel l s kv kt) :
    (toLF l).result (kt) =
      (resL l s (kv) (nfacets l)).map FCell.facet := by
  unfold IncLedger.result resL
  have hf : (toLF l).facets = (List.range (nfacets l)).map FCell.facet := rfl
  rw [hf, List.filter_map]
  congr 1
  apply List.filter_congr
  intro f hf'
  have hfl := List.mem_range.mp hf'
  simp only [Function.comp, inR, kpF_eq l hw s hs kv kt hk hfl, kmF_eq l hw s hs kv kt hk hfl]

/-- **The senses are the Lean's `orient`.** -/
theorem orient_eq (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (kv : List Bool) (kt : KeepTabI FCell) (hk : KeepRel l s kv kt) {f : Nat}
    (hf : f < nfacets l) :
    (toLF l).orient (kt) (FCell.facet f) = toS (oriR l s (kv) f) := by
  unfold IncLedger.orient oriR
  rw [← kmF_eq l hw s hs kv kt hk hf]
  cases kmF l s (kv) f <;> rfl

end inc

end forge_bool_core
