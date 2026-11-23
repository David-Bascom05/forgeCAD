import ForgeBoolCore.RefineMemb
open Aeneas Aeneas.Std Result

/-!
# `classify_on`'s check is the Lean core's `classifyOk`
-/

namespace forge_bool_core

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- Facet `k`'s check, over the bodies `i ≥ j`. -/
def ckFrom (v memb : List (alloc.vec.Vec Bool)) (k p m nb j : Nat) : Bool :=
  ((List.range nb).drop j).all fun i => !(PAt v k i && (PAt memb i p == PAt memb i m))

theorem drop_range_succ {n j : Nat} (h : j < n) : (List.range n).drop j = j :: (List.range n).drop (j + 1) := by
  rw [List.drop_eq_getElem_cons (by simpa using h)]; simp

theorem classify_inner_spec (v memb : alloc.vec.Vec (alloc.vec.Vec Bool)) (nb : Usize) (k p m : Usize)
    (hk : k.val < v.val.length) (hrow : nb.val ≤ (v.val.getD k.val default).val.length)
    (hm : nb.val ≤ memb.val.length)
    (hpm : ∀ i, i < nb.val → p.val < (memb.val.getD i default).val.length ∧ m.val < (memb.val.getD i default).val.length)
    (ok : Bool) (i : Usize) (hi : i.val ≤ nb.val) :
    classify_on_loop0_loop0 v memb nb ok k p m i ⦃ r =>
      r = (ok && ckFrom v.val memb.val k.val p.val m.val nb.val i.val) ⦄ := by
  unfold classify_on_loop0_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => nb.val + 1 - q.2.val - (if q.1 then 0 else 1))
    (fun q => q.2.val ≤ nb.val ∧
      (q.1 && ckFrom v.val memb.val k.val p.val m.val nb.val q.2.val) =
        (ok && ckFrom v.val memb.val k.val p.val m.val nb.val i.val))
  · rintro ⟨o, j⟩ ⟨hj, heq⟩
    unfold classify_on_loop0_loop0.body
    simp only at hj heq ⊢
    cases o with
    | false => simp only [Bool.false_eq_true, ite_false, WP.spec_ok]; rw [← heq]; rfl
    | true =>
      simp only [ite_true]
      split
      · rename_i hlt
        have hjl : j.val < nb.val := by scalar_tac
        have hkl : k.val < v.val.length := hk
        step as ⟨v1, hv1⟩
        have hv1' : v1 = v.val.getD k.val default := by
          rw [hv1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hkl]; rfl
        have hjv : j.val < v1.val.length := by rw [hv1']; omega
        step as ⟨b, hb⟩
        have hbv : b = PAt v.val k.val j.val := by
          rw [hb, PAt, ← hv1', List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjv]; rfl
        have hd := drop_range_succ hjl
        split
        · rename_i hbt
          have hjm : j.val < memb.val.length := by omega
          step as ⟨v2, hv2⟩
          have hv2' : v2 = memb.val.getD j.val default := by
            rw [hv2, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjm]; rfl
          obtain ⟨hp, hmm⟩ := hpm j.val hjl
          step as ⟨b2, hb2⟩
          step as ⟨b3, hb3⟩
          have hb2v : b2 = PAt memb.val j.val p.val := by
            rw [hb2, PAt, ← hv2', List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by rw [hv2']; exact hp)]; rfl
          have hb3v : b3 = PAt memb.val j.val m.val := by
            rw [hb3, PAt, ← hv2', List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by rw [hv2']; exact hmm)]; rfl
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, ?_⟩
          · rw [← heq, hj1, ckFrom, ckFrom, hd, List.all_cons, ← hbv, hbt, ← hb2v, ← hb3v]
            by_cases e : b2 = b3 <;> simp [e]
          · by_cases e : b2 = b3 <;> simp [e, hj1] <;> omega
        · rename_i hbf
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by simp [hj1]; omega⟩
          rw [← heq, hj1, ckFrom, ckFrom, hd, List.all_cons, ← hbv]
          simp [hbf]
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = nb.val := by scalar_tac
        rw [← heq, this, ckFrom]; simp
  · exact ⟨hi, rfl⟩

def tpP (s : Split) (k : Nat) : Nat := at32 s.tope_pos (at32 s.plus k)
def tpM (s : Split) (k : Nat) : Nat := at32 s.tope_pos (at32 s.minus k)

/-- The whole check, over facets `k' ≥ k`. -/
def ckAll (s : Split) (memb : List (alloc.vec.Vec Bool)) (nb k : Nat) : Bool :=
  ((List.range s.facets.val.length).drop k).all fun k' =>
    ckFrom s.piece_of.val memb k' (tpP s k') (tpM s k') nb 0

theorem classify_outer_spec (s : Split) (memb : alloc.vec.Vec (alloc.vec.Vec Bool)) (nb : Usize)
    (hwf : SplitWF s 0) (hnb0 : 0 < nb.val → ∀ b, b < nb.val → SplitWF s b)
    (hrowk : ∀ k, k < s.facets.val.length → nb.val ≤ (s.piece_of.val.getD k default).val.length)
    (hm : nb.val ≤ memb.val.length)
    (hmr : ∀ i, i < nb.val → s.topes.val.length ≤ (memb.val.getD i default).val.length)
    (ok : Bool) (k : Usize) (hk : k.val ≤ s.facets.val.length) :
    classify_on_loop0 s memb nb ok k ⦃ r => r.1 = s ∧ r.2 = (ok && ckAll s memb.val nb.val k.val) ⦄ := by
  unfold classify_on_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => s.facets.val.length + 1 - q.2.val - (if q.1 then 0 else 1))
    (fun q => q.2.val ≤ s.facets.val.length ∧ (q.1 && ckAll s memb.val nb.val q.2.val) = (ok && ckAll s memb.val nb.val k.val))
  · rintro ⟨o, j⟩ ⟨hj, heq⟩
    unfold classify_on_loop0.body
    simp only at hj heq ⊢
    cases o with
    | false => simp only [Bool.false_eq_true, ite_false, WP.spec_ok]; exact ⟨trivial, heq⟩
    | true =>
      simp only [ite_true]
      split
      · rename_i hlt
        have hjl : j.val < s.facets.val.length := by scalar_tac
        have hPl := hwf.plus_len; have hMl := hwf.minus_len; have hPOl := hwf.piece_len
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = at32 s.plus j.val := by rw [hi2, at32_getD (by omega), hi1]; scalar_tac
        have hin1 := hwf.plus_in j.val hjl
        step as ⟨i3, hi3⟩
        step as ⟨pp, hpp⟩
        have hppv : pp.val = tpP s j.val := by
          rw [hpp, tpP, at32_getD (by omega)]; simp only [hi2v] at hi3; rw [hi3]; scalar_tac
        step as ⟨i4, hi4⟩
        step as ⟨i5, hi5⟩
        have hi5v : i5.val = at32 s.minus j.val := by rw [hi5, at32_getD (by omega), hi4]; scalar_tac
        have hin2 := hwf.minus_in j.val hjl
        step as ⟨i6, hi6⟩
        step as ⟨mm, hmm⟩
        have hmmv : mm.val = tpM s j.val := by
          rw [hmm, tpM, at32_getD (by omega)]; simp only [hi5v] at hi6; rw [hi6]; scalar_tac
        have hpos1 := hwf.plus_pos j.val hjl
        have hpos2 := hwf.minus_pos j.val hjl
        step with classify_inner_spec s.piece_of memb nb j pp mm (by omega) (hrowk j.val hjl) hm
          (fun i hi => ⟨by have := hmr i hi; rw [hppv]; unfold tpP; omega,
            by have := hmr i hi; rw [hmmv]; unfold tpM; omega⟩) true 0#usize (by simp) as ⟨o2, ho2⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, ?_⟩
        · rw [← heq, hj1, ho2, ckAll, ckAll, drop_range_succ hjl, List.all_cons, hppv, hmmv]; simp
        · cases o2 <;> simp [hj1] <;> omega
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = s.facets.val.length := by scalar_tac
        refine ⟨trivial, ?_⟩
        rw [← heq, this, ckAll]; simp
  · exact ⟨hk, rfl⟩

/-- The Lean tope at facet `k`'s plus (minus) position is its plus (minus) side. -/
theorem tp_sides (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hok0 : ExtOK l e0) (hs : SplitOK l e0 s e)
    (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) {k : Nat} (hk : k < s.facets.val.length) :
    ∃ (h1 : tpP s k < (toL l).topes.length) (h2 : tpM s k < (toL l).topes.length),
      (toL l).topes[tpP s k] = Cov.plus (cellL l (s.facets.val[k]'hk).val) ∧
      (toL l).topes[tpM s k] = Cov.minus (cellL l (s.facets.val[k]'hk).val) := by
  have hTL := topes_toL l e0 s e hok0 hs hpc
  obtain ⟨_, _, _, _, hPl, hMl, hPM⟩ := hs.sides
  have hT := hs.topes
  have hF : (s.facets.val[k]'hk).val < l.cells.val.length := by
    have : (s.facets.val[k]'hk).val ∈ s.facets.val.map (·.val) := List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩
    rw [hs.facets] at this
    obtain ⟨q, hq, e⟩ := List.mem_map.mp ((mem_dedup _ _).mp this)
    rw [← e]; exact hpc q hq
  have hsm := stream_mem s.plus s.minus s.facets.val.length (by omega) (by omega) k hk
  -- position of a stream id among the Lean topes
  have hpos : ∀ c ∈ stream s.plus.val s.minus.val s.facets.val.length,
      ∃ h : at32 s.tope_pos c < (toL l).topes.length, (toL l).topes[at32 s.tope_pos c] = covE l e c := by
    intro c hc
    have hcn := hT.2.2.1 c hc
    have hcT : c ∈ s.topes.val.map (·.val) := by rw [hT.2.1]; exact (mem_dedup _ _).mpr hc
    have hidx : at32 s.tope_pos c = (s.topes.val.map (·.val)).idxOf c := by
      have := hT.2.2.2 c hcn
      rw [if_pos hc, ← hT.2.1] at this
      rw [at32_getD (by rw [hT.1]; exact hcn), ← this, List.getD_eq_getElem?_getD,
        List.getElem?_eq_getElem (by rw [hT.1]; exact hcn)]; rfl
    have hlt : (s.topes.val.map (·.val)).idxOf c < (s.topes.val.map (·.val)).length :=
      List.idxOf_lt_length_iff.mpr hcT
    have hlen : (toL l).topes.length = s.topes.val.length := by rw [← hTL]; simp
    refine ⟨by rw [hidx, hlen]; simpa using hlt, ?_⟩
    simp only [← hTL, List.getElem_map]
    have e1 : (s.topes.val.map (·.val))[(s.topes.val.map (·.val)).idxOf c] = c := List.getElem_idxOf hlt
    simp only [hidx]
    rw [List.getElem_map] at e1; rw [e1]
  obtain ⟨h1, e1⟩ := hpos _ hsm.1
  obtain ⟨h2, e2⟩ := hpos _ hsm.2
  obtain ⟨_, a2, _, a4⟩ := hPM k hk
  rw [getElem!_u32 (show k < s.facets.val.length from hk)] at a2 a4
  refine ⟨h1, h2, ?_, ?_⟩
  · simp only [tpP]; rw [e1, at32_getD (by omega), covE, ← getElem!_u32 (show k < s.plus.val.length by omega), a2,
      side_plus, cellL_covAt l e0 hF]
  · simp only [tpM]; rw [e2, at32_getD (by omega), covE, ← getElem!_u32 (show k < s.minus.val.length by omega), a4,
      side_minus, cellL_covAt l e0 hF]

/-- **The check is the Lean's `classifyOk`.** -/
theorem ckAll_eq (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hok0 : ExtOK l e0) (hs : SplitOK l e0 s e)
    (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) (M : List (alloc.vec.Vec Bool))
    (hM : ∀ b, b < l.nb.val → ∀ t (ht : t < s.topes.val.length),
      (M.getD b default).val.getD t false =
        (toL l).memb b ((toL l).topes[t]'(by rw [← topes_toL l e0 s e hok0 hs hpc]; simpa using ht))) :
    ckAll s M l.nb.val 0 = (toL l).classifyOk := by
  have hFL := facets_toL l e0 hok0 hpc s.facets.val hs.facets
  have hTL := topes_toL l e0 s e hok0 hs hpc
  have hlenT : (toL l).topes.length = s.topes.val.length := by rw [← hTL]; simp
  unfold ckAll ForgeExact.DecideOnce.Ledger.classifyOk
  rw [← hFL]
  apply Bool.eq_iff_iff.mpr
  simp only [List.drop_zero, List.all_eq_true, List.mem_range, List.mem_map, forall_exists_index, and_imp,
    forall_apply_eq_imp_iff₂]
  have hnb : (toL l).nb = l.nb.val := rfl
  rw [hnb]
  constructor
  · intro h x hx i hi
    obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hx
    have := h k hk
    unfold ckFrom at this
    simp only [List.drop_zero, List.all_eq_true, List.mem_range] at this
    have hc := this i hi
    obtain ⟨h1, h2, ep, em⟩ := tp_sides l e0 s e hok0 hs hpc hk
    rw [piece_toL l e0 s e hok0 hs hpc hk hi] at hc
    rw [PAt, PAt, hM i hi _ (by rw [← hlenT]; exact h1), hM i hi _ (by rw [← hlenT]; exact h2)] at hc
    simp only [ep, em] at hc
    revert hc
    cases (toL l).isPieceOf i _ <;> cases (toL l).memb i _ <;> cases (toL l).memb i _ <;> simp
  · intro h k hk
    unfold ckFrom
    simp only [List.drop_zero, List.all_eq_true, List.mem_range]
    intro i hi
    have hc := h _ (List.getElem_mem hk) i hi
    obtain ⟨h1, h2, ep, em⟩ := tp_sides l e0 s e hok0 hs hpc hk
    rw [piece_toL l e0 s e hok0 hs hpc hk hi, PAt, PAt, hM i hi _ (by rw [← hlenT]; exact h1),
      hM i hi _ (by rw [← hlenT]; exact h2)]
    simp only [ep, em]
    revert hc
    cases (toL l).isPieceOf i _ <;> cases (toL l).memb i _ <;> cases (toL l).memb i _ <;> simp

end forge_bool_core
