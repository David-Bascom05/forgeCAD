import ForgeBoolCore.RefineIncMemb
import ForgeBoolCore.RefineKeep

/-!
# Group C: the inc classify check and keep

`inc::core`'s first loop checks each piece's two rooms differ in membership (`classifyOk`); its
second applies the rule to each room's column of the membership table (`keep`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- The membership table's entry, from `membership_spec`. -/
theorem memb_get (l : Ledger) (s : Split) {memb : alloc.vec.Vec (alloc.vec.Vec Bool)}
    (hm : memb.val.map (·.val) = membTab l s l.nb.val) {i t : Nat} (hi : i < l.nb.val)
    (ht : t < s.rooms.val.length) :
    ∃ (h1 : i < memb.val.length) (h2 : t < (memb.val[i]).val.length),
      (memb.val[i]).val[t] = membRow l s i t := by
  have hl : memb.val.length = l.nb.val := by
    have := congrArg List.length hm; simpa [membTab] using this
  have hrow : (memb.val[i]'(by omega)).val = (List.range s.rooms.val.length).map (membRow l s i) := by
    have := congrArg (fun L => L[i]?) hm
    simp only [List.getElem?_map, membTab, List.getElem?_range hi, List.getElem?_eq_getElem (show i < memb.val.length by omega),
      Option.map_some, Option.some.injEq] at this
    exact this
  refine ⟨by omega, by rw [hrow]; simpa using ht, ?_⟩
  simp [hrow]

/-- A room's membership at its position is the Lean's, for a room some facet has. -/
theorem membRow_pos (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (i : Nat) {x : Nat}
    (hx : x < l.n_rooms.val) (hm : x ∈ seenRooms l.facet_rooms.val (nfacets l)) :
    membRow l s i (posR s x) = (toLF l).memb i (FCell.room x) := by
  unfold membRow
  have he := ((hs.rinv.2.2.2 x hx).2 hm).1
  change (s.rooms.val.getD (posR s x) 0#u32).val = x at he
  rw [he]

/-- The check for facet `f` and body `i`. -/
def cfi (l : Ledger) (s : Split) (f i : Nat) : Bool :=
  !pieceB l s f i || (membRow l s i (posR s (plusR l f)) != membRow l s i (posR s (minusR l f)))

theorem all_range_false {p : Nat → Bool} {i n : Nat} (hi : i ≤ n) (h : (List.range i).all p = false) :
    (List.range n).all p = false := by
  rw [Bool.eq_false_iff] at h ⊢
  intro hn; apply h
  rw [List.all_eq_true] at hn ⊢
  intro x hx; exact hn x (by simp at hx ⊢; omega)

theorem all_range_succ (p : Nat → Bool) (i : Nat) :
    (List.range (i + 1)).all p = ((List.range i).all p && p i) := by
  simp [List.range_succ, List.all_append]

theorem classify_inner_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s)
    (memb : alloc.vec.Vec (alloc.vec.Vec Bool)) (hm : memb.val.map (·.val) = membTab l s l.nb.val)
    (f : Usize) (hf : f.val < nfacets l) (pp mp : Usize)
    (hpp : pp.val = posR s (plusR l f.val)) (hmp : mp.val = posR s (minusR l f.val))
    (ok : Bool) (i : Usize) (hi : i.val ≤ l.nb.val) (hok : ok = (List.range i.val).all (cfi l s f.val)) :
    core_loop0_loop0 s.piece_of memb (UScalar.cast .Usize l.nb) ok f pp mp i ⦃ r =>
      r = (List.range l.nb.val).all (cfi l s f.val) ⦄ := by
  have hsm := hw.small
  have hnbv : (UScalar.cast UScalarTy.Usize l.nb : Usize).val = l.nb.val := by scalar_tac
  have hfr := facet_rooms_get l hw hf
  have hppl : pp.val < s.rooms.val.length := by rw [hpp]; exact pos_lt l hw s hs hfr.1 (plusR_mem l hf)
  have hmpl : mp.val < s.rooms.val.length := by rw [hmp]; exact pos_lt l hw s hs hfr.2 (minusR_mem l hf)
  unfold core_loop0_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => l.nb.val - q.2.val)
    (fun q => q.2.val ≤ l.nb.val ∧ q.1 = (List.range q.2.val).all (cfi l s f.val))
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold core_loop0_loop0.body
    simp only at hj ho ⊢
    split
    · rename_i ho1
      split
      · rename_i hlt
        have hjl : j.val < l.nb.val := by scalar_tac
        have hkey : f.val * l.nb.val + j.val < nfacets l * l.nb.val := by
          have := Nat.mul_le_mul_right l.nb.val (show f.val + 1 ≤ nfacets l by omega)
          rw [Nat.add_mul, Nat.one_mul] at this; omega
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = f.val * l.nb.val + j.val := by scalar_tac
        have hpl := hs.piece_len
        step as ⟨b, hb⟩
        have hbv : b = pieceB l s f.val j.val := by
          rw [hb, pieceB, ← hi2v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
        obtain ⟨h1, _, e1⟩ := memb_get l s hm hjl hppl
        obtain ⟨_, _, e2⟩ := memb_get l s hm hjl hmpl
        have hcf : cfi l s f.val j.val = (!b || (membRow l s j.val pp.val != membRow l s j.val mp.val)) := by
          rw [cfi, hbv, hpp, hmp]
        split
        · rename_i hb1
          step as ⟨v1, hv1⟩
          subst hv1
          step as ⟨b1, hb1'⟩
          step as ⟨b2, hb2⟩
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by scalar_tac
          refine ⟨by omega, ?_, by omega⟩
          rw [hj1v, all_range_succ, ← ho, ho1, hcf, hb1]
          simp only [Bool.true_and, Bool.not_true, Bool.false_or, hb1', hb2, e1, e2]
        · rename_i hb1
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by scalar_tac
          refine ⟨by omega, ?_, by omega⟩
          have : b = false := by simpa using hb1
          rw [hj1v, all_range_succ, ← ho, ho1, hcf, this]; rfl
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = l.nb.val := by scalar_tac
        rw [ho, this]
    · rename_i ho1
      simp only [WP.spec_ok]
      have : o = false := by simpa using ho1
      subst this
      exact (all_range_false hj ho.symm).symm
  · exact ⟨hi, hok⟩

/-- The check over all the facets. -/
def cAll (l : Ledger) (s : Split) (k : Nat) : Bool :=
  (List.range k).all fun f => (List.range l.nb.val).all (cfi l s f)

theorem core_loop0_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s)
    (memb : alloc.vec.Vec (alloc.vec.Vec Bool)) (hm : memb.val.map (·.val) = membTab l s l.nb.val)
    (ok : Bool) (f : Usize) (hf : f.val ≤ nfacets l) (hok : ok = cAll l s f.val) :
    core_loop0 l.facet_rooms s memb (UScalar.cast .Usize l.nb) (alloc.vec.Vec.len l.facet_class) ok f ⦃ r =>
      r.1 = s ∧ r.2 = cAll l s (nfacets l) ⦄ := by
  have hsm := hw.small
  have hfrl := hw.rooms.1
  have hrpl := hs.rinv.2.1
  unfold core_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => nfacets l - q.2.val)
    (fun q => q.2.val ≤ nfacets l ∧ q.1 = cAll l s q.2.val)
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold core_loop0.body
    simp only at hj ho ⊢
    split
    · rename_i ho1
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
        step as ⟨i, hi⟩
        have hiv : i.val = p.val := by scalar_tac
        step as ⟨i1, hi1⟩
        step as ⟨pp, hpp⟩
        have hppv : pp.val = posR s (plusR l j.val) := by
          rw [hpv, posR, ← hiv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
          simp only [Option.getD_some]; rw [← hi1]; scalar_tac
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = m.val := by scalar_tac
        step as ⟨i3, hi3⟩
        step as ⟨mp, hmp⟩
        have hmpv : mp.val = posR s (minusR l j.val) := by
          rw [hmv, posR, ← hi2v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]
          simp only [Option.getD_some]; rw [← hi3]; scalar_tac
        have ho' : o = true := by simpa using ho1
        step with classify_inner_spec l hw s hs memb hm j hjl pp mp hppv hmpv o 0#usize (by simp)
          (by rw [ho']; simp) as ⟨ok2, hok2⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, ?_, by omega⟩
        rw [hj1v, cAll, all_range_succ, ← cAll, ← ho, ho', hok2]; rfl
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = nfacets l := by unfold nfacets at hj ⊢; scalar_tac
        exact ⟨by simp, by rw [ho, this]⟩
    · rename_i ho1
      simp only [WP.spec_ok]
      have : o = false := by simpa using ho1
      subst this
      exact ⟨by simp, (all_range_false hj ho.symm).symm⟩
  · exact ⟨hf, hok⟩

/-- **The check is `classifyOk`.** -/
theorem cAll_eq (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) :
    cAll l s (nfacets l) = (toLF l).classifyOk := by
  unfold cAll IncLedger.classifyOk
  have hf : (toLF l).facets = (List.range (nfacets l)).map FCell.facet := rfl
  rw [hf, List.all_map]
  apply List.all_congr
  intro f hf'
  have hfl : f < nfacets l := List.mem_range.mp hf'
  simp only [Function.comp]
  apply List.all_congr
  intro i hi'
  have hil : i < l.nb.val := List.mem_range.mp hi'
  have hfr := facet_rooms_get l hw hfl
  rw [cfi, pieceB, hs.piece_of f hfl i hil, membRow_pos l hw s hs i hfr.1 (plusR_mem l hfl),
    membRow_pos l hw s hs i hfr.2 (minusR_mem l hfl)]
  rfl
where
  List.all_congr {β : Type} {L : List β} {p q : β → Bool} (h : ∀ x ∈ L, p x = q x) : L.all p = L.all q := by
    induction L with
    | nil => rfl
    | cons a t ih =>
      simp only [List.all_cons]
      rw [h a (List.mem_cons_self ..), ih fun x hx => h x (List.mem_cons_of_mem _ hx)]

/-! ### Keep -/

/-- Room `t`'s column: its membership in each body. -/
def colI (l : Ledger) (s : Split) (t : Nat) : List Bool := (List.range l.nb.val).map fun i => membRow l s i t

theorem keep_col_specI (l : Ledger) (s : Split)
    (memb : alloc.vec.Vec (alloc.vec.Vec Bool)) (hm : memb.val.map (·.val) = membTab l s l.nb.val)
    (t : Usize) (ht : t.val < s.rooms.val.length)
    (mv : alloc.vec.Vec Bool) (i : Usize) (hi : i.val ≤ l.nb.val) (hmv : mv.val = (colI l s t.val).take i.val) :
    core_loop1_loop0 memb (UScalar.cast .Usize l.nb) t mv i ⦃ r => r.val = colI l s t.val ⦄ := by
  have hnbv : (UScalar.cast UScalarTy.Usize l.nb : Usize).val = l.nb.val := by scalar_tac
  unfold core_loop1_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Bool × Usize) => l.nb.val - q.2.val)
    (fun q => q.2.val ≤ l.nb.val ∧ q.1.val = (colI l s t.val).take q.2.val)
  · rintro ⟨w, j⟩ ⟨hj, hw⟩
    unfold core_loop1_loop0.body
    simp only at hj hw ⊢
    split
    · rename_i hlt
      have hjl : j.val < l.nb.val := by scalar_tac
      obtain ⟨h1, h2, e⟩ := memb_get l s hm hjl ht
      step as ⟨v, hv⟩
      subst hv
      step as ⟨b, hb⟩
      have hwl : w.val.length < Usize.max := by rw [hw]; simp; scalar_tac
      step as ⟨w1, hw1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [hw1, hw, hj1, List.take_succ, List.getElem?_eq_getElem (by simp [colI]; omega)]
      simp only [Option.toList_some]
      congr 1
      simp only [colI, List.getElem_map, List.getElem_range]
      rw [hb, e]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = l.nb.val := by scalar_tac
      rw [hw, this, List.take_of_length_le (by simp [colI])]
  · exact ⟨hi, hmv⟩

theorem core_loop1_spec (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule)
    (memb : alloc.vec.Vec (alloc.vec.Vec Bool)) (hm : memb.val.map (·.val) = membTab l s l.nb.val)
    (keep : alloc.vec.Vec Bool) (t : Usize) (htl : t.val ≤ s.rooms.val.length)
    (hk : keep.val = (List.range t.val).map fun t' => applyL rule (colI l s t')) :
    core_loop1 rule s.rooms memb (UScalar.cast .Usize l.nb) keep t ⦃ r =>
      r.val = (List.range s.rooms.val.length).map fun t' => applyL rule (colI l s t') ⦄ := by
  have hrl := rooms_len_le l s hs
  have hsm := hw.small
  unfold core_loop1
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Bool × Usize) => s.rooms.val.length - q.2.val)
    (fun q => q.2.val ≤ s.rooms.val.length ∧
      q.1.val = (List.range q.2.val).map fun t' => applyL rule (colI l s t'))
  · rintro ⟨kp, j⟩ ⟨hj, hkp⟩
    unfold core_loop1.body
    simp only at hj hkp ⊢
    split
    · rename_i hlt
      have hjl : j.val < s.rooms.val.length := by scalar_tac
      step with keep_col_specI l s memb hm j hjl (alloc.vec.Vec.with_capacity Bool _) 0#usize (by simp)
        (by simp [alloc.vec.Vec.with_capacity]) as ⟨mv, hmv⟩
      step with apply_rule_spec rule _ as ⟨b, hb⟩
      have hkl : kp.val.length < Usize.max := by rw [hkp]; simp; scalar_tac
      step as ⟨kp1, hkp1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      have hj1v : j1.val = j.val + 1 := by scalar_tac
      rw [hkp1, hkp, hj1v, List.range_succ, List.map_append, hb]
      simp [alloc.vec.Vec.deref, hmv]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = s.rooms.val.length := by scalar_tac
      rw [hkp, this]
  · exact ⟨htl, hk⟩

/-- **The keep is the Lean's `keep`**, room by room. -/
theorem keep_eq (l : Ledger) (rule : Rule) (s : Split) (t : Nat) :
    applyL rule (colI l s t) = (toLF l).keep (ruleL l.nb.val rule) (FCell.room (s.rooms.val.getD t 0#u32).val) := by
  unfold IncLedger.keep ruleL colI
  congr 1
  apply List.map_congr_left
  intro i hi
  have : i < l.nb.val := List.mem_range.mp hi
  simp [this, membRow, toLF]

end inc

end forge_bool_core
