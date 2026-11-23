import ForgeBoolCore.RefineSort
open Aeneas Aeneas.Std Result

/-!
# The cells the core reads: the ledger's and the ones it adds

`allCells l ext` is the ledger's cells then the core's (`Ext.extra`). `ExtOK` says the index is
exact over the ledger's cells, no covector is listed twice, and the ids fit `u32` below `NONE`.
`cov` reads it (`cov_spec`), `find` is lookup by covector (`cells_find_spec`), and `intern` adds a covector
only when absent, keeping `ExtOK` (`intern_spec`).
-/

namespace forge_bool_core

/-- Every cell's covector, by id. -/
def allCells (l : Ledger) (ext : Ext) : List (List Sgn) := (l.cells.val ++ ext.extra.val).map (·.val)

def covAt (l : Ledger) (ext : Ext) (id : Nat) : List Sgn := (allCells l ext).getD id []

structure ExtOK (l : Ledger) (ext : Ext) : Prop where
  index : IndexOK ext.index.order (alloc.vec.Vec.deref l.cells)
  nodup : (allCells l ext).Nodup
  small : (allCells l ext).length < U32.max

theorem allCells_length (l : Ledger) (ext : Ext) :
    (allCells l ext).length = l.cells.val.length + ext.extra.val.length := by simp [allCells]

theorem covAt_left {l : Ledger} {ext : Ext} {id : Nat} (h : id < l.cells.val.length) :
    covAt l ext id = l.cells.val[id].val := by
  rw [covAt, allCells, List.map_append, List.getD_eq_getElem?_getD,
    List.getElem?_append_left (by simpa using h)]
  simp [h]

theorem covAt_right {l : Ledger} {ext : Ext} {i : Nat} (h : i < ext.extra.val.length) :
    covAt l ext (l.cells.val.length + i) = ext.extra.val[i].val := by
  simp [covAt, allCells, List.getD_eq_getElem?_getD, List.getElem?_append_right,
    List.getElem?_eq_getElem h]

theorem cellAt_deref (l : Ledger) (ext : Ext) {id : Nat} (h : id < l.cells.val.length) :
    cellAt (alloc.vec.Vec.deref l.cells) id = covAt l ext id := by
  rw [covAt_left h, cellAt_eq (by simpa [alloc.vec.Vec.deref] using h)]; simp [alloc.vec.Vec.deref]

/-- **`cov` reads the cell's covector.** -/
theorem cov_spec (l : Ledger) (ext : Ext) (id : U32) (h : id.val < (allCells l ext).length) :
    cov l ext id ⦃ s => s.val = covAt l ext id.val ⦄ := by
  rw [allCells_length] at h
  unfold cov
  step as ⟨i, hi⟩
  have hiv : i.val = id.val := by rw [hi]; scalar_tac
  split
  · rename_i hlt
    have hl : id.val < l.cells.val.length := by scalar_tac
    step as ⟨i1, hi1⟩
    have : i1.val = id.val := by rw [hi1]; scalar_tac
    step as ⟨v, hv⟩
    rw [covAt_left hl, hv]; simp [alloc.vec.Vec.deref, this]
  · rename_i hge
    have hl : l.cells.val.length ≤ id.val := by scalar_tac
    step as ⟨i1, hi1⟩
    have : i1.val = id.val := by rw [hi1]; scalar_tac
    step as ⟨i2, hi2⟩
    have e : id.val = l.cells.val.length + i2.val := by
      have : (alloc.vec.Vec.len l.cells).val = l.cells.val.length := by simp
      omega
    have hi2l : i2.val < ext.extra.val.length := by omega
    step as ⟨v, hv⟩
    rw [e, covAt_right hi2l, hv]; simp [alloc.vec.Vec.deref]

/-- Lookup over every cell: the id with the key's covector, or `NONE` when none has it. -/
def FindAll (l : Ledger) (ext : Ext) (key : List Sgn) (r : U32) : Prop :=
  (r = NONE → ∀ id, id < (allCells l ext).length → covAt l ext id ≠ key) ∧
  (r ≠ NONE → r.val < (allCells l ext).length ∧ covAt l ext r.val = key)

theorem u32_le_usize : U32.max ≤ Usize.max := by
  rcases Usize.bounds_eq with h | h <;> rw [h] <;> simp [U32.max_eq, U64.max_eq]

theorem cells_find_loop_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (key : Slice Sgn) (f : U32) (i : Usize)
    (hi : i.val ≤ ext.extra.val.length)
    (hn : f = NONE → (∀ id, id < l.cells.val.length → covAt l ext id ≠ key.val) ∧
      ∀ i', i' < i.val → covAt l ext (l.cells.val.length + i') ≠ key.val)
    (hs : f ≠ NONE → f.val < (allCells l ext).length ∧ covAt l ext f.val = key.val) :
    find_loop l.cells ext.extra key f i ⦃ r => FindAll l ext key.val r ⦄ := by
  have hsmall := hok.small
  rw [allCells_length] at hsmall
  unfold find_loop
  apply loop.spec_decr_nat
    (fun (q : U32 × Usize) => if q.1 = NONE then ext.extra.val.length - q.2.val + 1 else 0)
    (fun q => q.2.val ≤ ext.extra.val.length ∧
      (q.1 = NONE → (∀ id, id < l.cells.val.length → covAt l ext id ≠ key.val) ∧
        ∀ i', i' < q.2.val → covAt l ext (l.cells.val.length + i') ≠ key.val) ∧
      (q.1 ≠ NONE → q.1.val < (allCells l ext).length ∧ covAt l ext q.1.val = key.val))
  · rintro ⟨g, j⟩ ⟨hj, hgn, hgs⟩
    unfold find_loop.body
    simp only at hj hgn hgs ⊢
    split
    · rename_i hg
      obtain ⟨hleft, hdone⟩ := hgn hg
      split
      · rename_i hlt
        have hjl : j.val < ext.extra.val.length := by scalar_tac
        step as ⟨v, hv⟩
        step with same_cov_spec (alloc.vec.Vec.deref v) key as ⟨b, hb⟩
        have hvc : (alloc.vec.Vec.deref v).val = covAt l ext (l.cells.val.length + j.val) := by
          rw [covAt_right hjl, hv]; simp [alloc.vec.Vec.deref]
        rw [hvc] at hb
        split
        · rename_i hbt
          have heq : covAt l ext (l.cells.val.length + j.val) = key.val := by simpa [hbt] using hb.symm
          have h3 : l.cells.val.length + j.val ≤ Usize.max := by have := u32_le_usize; omega
          step as ⟨i3, hi3⟩
          have hcast : (UScalar.cast UScalarTy.U32 i3).val = l.cells.val.length + j.val := by
            have : i3.val = l.cells.val.length + j.val := by rw [hi3]; simp
            rw [UScalar.cast_val_eq, this]
            apply Nat.mod_eq_of_lt
            have := U32.max_eq; simp; omega
          have hne : UScalar.cast UScalarTy.U32 i3 ≠ NONE := by
            intro e; have := congrArg (·.val) e; simp only [hcast, none_val] at this; omega
          step as ⟨j1, hj1⟩
          simp only [hne, ite_false, and_assoc]
          refine ⟨by scalar_tac, fun e => e.elim, fun _ => ⟨by rw [hcast, allCells_length]; omega,
            by rw [hcast]; exact heq⟩, by simp [hg]⟩
        · rename_i hbf
          have hne : covAt l ext (l.cells.val.length + j.val) ≠ key.val := by
            intro e; apply hbf; rw [hb, e]; simp
          step as ⟨j1, hj1⟩
          simp only [hg, ite_true, and_assoc]
          refine ⟨by scalar_tac, fun _ => ⟨hleft, fun i' hi' => ?_⟩, fun e => absurd rfl e, by scalar_tac⟩
          by_cases h' : i' = j.val
          · rw [h']; exact hne
          · exact hdone i' (by scalar_tac)
      · rename_i hge
        simp only [WP.spec_ok]
        refine ⟨fun _ id hid => ?_, fun e => absurd hg e⟩
        rw [allCells_length] at hid
        by_cases h1 : id < l.cells.val.length
        · exact hleft id h1
        · have := hdone (id - l.cells.val.length) (by scalar_tac)
          rwa [show l.cells.val.length + (id - l.cells.val.length) = id by omega] at this
    · rename_i hg
      simp only [WP.spec_ok]
      exact ⟨fun e => absurd e hg, fun _ => hgs hg⟩
  · exact ⟨hi, hn, hs⟩

/-- **`find` is lookup by covector** over every cell. -/
theorem cells_find_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (key : Slice Sgn) :
    find l ext key ⦃ r => FindAll l ext key.val r ⦄ := by
  unfold find
  step with index_find_spec ext.index (alloc.vec.Vec.deref l.cells) key hok.index as ⟨f, hf⟩
  obtain ⟨hf1, hf2⟩ := hf
  have hdl : (alloc.vec.Vec.deref l.cells).val.length = l.cells.val.length := by simp [alloc.vec.Vec.deref]
  apply cells_find_loop_spec l ext hok key f 0#usize (by simp)
  · intro e
    refine ⟨fun id hid => ?_, fun i' h => by simp at h⟩
    rw [← cellAt_deref l ext hid]; exact hf1 e id (by omega)
  · intro e
    obtain ⟨h1, h2⟩ := hf2 e
    refine ⟨by rw [allCells_length]; omega, ?_⟩
    rw [← cellAt_deref l ext (by omega)]; exact h2

theorem mem_allCells_iff {l : Ledger} {ext : Ext} {c : List Sgn} :
    c ∈ allCells l ext ↔ ∃ id, id < (allCells l ext).length ∧ covAt l ext id = c := by
  constructor
  · intro h
    obtain ⟨id, hid, he⟩ := List.mem_iff_getElem.mp h
    exact ⟨id, hid, by simp [covAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hid, he]⟩
  · rintro ⟨id, hid, rfl⟩
    simp only [covAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hid, Option.getD_some]
    exact List.getElem_mem _

theorem ncells_spec (l : Ledger) (ext : Ext) (h : (allCells l ext).length ≤ Usize.max) :
    ncells l ext ⦃ r => r.val = (allCells l ext).length ⦄ := by
  rw [allCells_length] at h
  unfold ncells
  step as ⟨r, hr⟩
  rw [hr, allCells_length]; simp

/-- **`intern`**: the id of `key`, the cells growing by `key` exactly when it was new. -/
theorem intern_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (key : alloc.vec.Vec Sgn)
    (hroom : (allCells l ext).length + 1 < U32.max) :
    intern l ext key ⦃ r =>
      r.2.index = ext.index ∧ ExtOK l r.2 ∧
      allCells l r.2 = allCells l ext ++ (if key.val ∈ allCells l ext then [] else [key.val]) ∧
      r.1.val < (allCells l r.2).length ∧ covAt l r.2 r.1.val = key.val ⦄ := by
  unfold intern
  step with cells_find_spec l ext hok (alloc.vec.Vec.deref key) as ⟨f, hf⟩
  obtain ⟨hf1, hf2⟩ := hf
  have hkv : (alloc.vec.Vec.deref key).val = key.val := by simp [alloc.vec.Vec.deref]
  rw [hkv] at hf1 hf2
  split
  · rename_i hne
    have hne' : f ≠ NONE := by simpa using hne
    obtain ⟨hfl, hfc⟩ := hf2 hne'
    have hin : key.val ∈ allCells l ext := mem_allCells_iff.mpr ⟨f.val, hfl, hfc⟩
    simp only [WP.spec_ok, hin, ite_true, List.append_nil]
    exact ⟨trivial, hok, trivial, hfl, hfc⟩
  · rename_i heq
    have heq' : f = NONE := by simp at heq; exact UScalar.eq_of_val_eq heq
    have hnin : key.val ∉ allCells l ext := by
      intro h; obtain ⟨id, hid, he⟩ := mem_allCells_iff.mp h; exact hf1 heq' id hid he
    have hlen := allCells_length l ext
    have hl1 : ext.extra.val.length + 1 ≤ Usize.max := by have := u32_le_usize; omega
    step as ⟨v, hv⟩
    have hall : allCells l { ext with extra := v } = allCells l ext ++ [key.val] := by
      simp [allCells, hv]
    have hl2 : (allCells l { ext with extra := v }).length = (allCells l ext).length + 1 := by
      rw [hall]; simp
    step with ncells_spec l { ext with extra := v } (by have := u32_le_usize; omega) as ⟨i, hi⟩
    have hiv : i.val = (allCells l ext).length + 1 := by rw [hi, hl2]
    step as ⟨i1, hi1⟩
    step as ⟨f1, hf1v⟩
    have hi1v : i1.val = (allCells l ext).length := by omega
    have hf1' : f1.val = (allCells l ext).length := by
      rw [hf1v, UScalar.cast_val_eq, hi1v]; apply Nat.mod_eq_of_lt
      have := U32.max_eq; simp; omega
    refine ⟨⟨hok.index, ?_, ?_⟩, by rw [hall]; simp [hnin], by rw [hf1', hl2]; omega, ?_⟩
    · rw [hall]; exact List.nodup_append.mpr ⟨hok.nodup, List.nodup_singleton _, fun a ha b hb => by
        simp at hb; subst hb; intro e; subst e; exact hnin ha⟩
    · rw [hl2]; omega
    · rw [hf1']
      simp [covAt, hall, List.getD_eq_getElem?_getD]

end forge_bool_core
