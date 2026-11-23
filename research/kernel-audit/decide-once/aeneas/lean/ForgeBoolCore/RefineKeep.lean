import ForgeBoolCore.RefineClassify
import ForgeBoolCore.RefineRule
open Aeneas Aeneas.Std Result

/-!
# `keep_on`, stages 4–5: the keep table and the result facets

The Rust rule `r` runs as the Lean rule `ruleL nb r`: `applyL r` on the first `nb` bodies'
memberships.
-/

namespace forge_bool_core

open ForgeExact.DecideOnce ForgeBoolCore.Dedup

/-- A Rust rule as the Lean core's rule, for `nb` bodies. -/
def ruleL (nb : Nat) (r : Rule) : ForgeExact.DecideOnce.Rule := fun f => applyL r ((List.range nb).map f)

/-- The memberships of tope `t`, body by body. -/
def colOf (memb : List (alloc.vec.Vec Bool)) (nb t : Nat) : List Bool :=
  (List.range nb).map fun i => (memb.getD i default).val.getD t false

theorem keep_col_spec (memb : Slice (alloc.vec.Vec Bool)) (nb t : Usize) (hnb : nb.val ≤ memb.val.length)
    (ht : ∀ i, i < nb.val → t.val < (memb.val.getD i default).val.length)
    (m : alloc.vec.Vec Bool) (i : Usize) (hi : i.val ≤ nb.val) (hm : m.val = (colOf memb.val nb.val t.val).take i.val) :
    keep_on_loop0_loop0 memb nb t m i ⦃ r => r.val = colOf memb.val nb.val t.val ⦄ := by
  unfold keep_on_loop0_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Bool × Usize) => nb.val - q.2.val)
    (fun q => q.2.val ≤ nb.val ∧ q.1.val = (colOf memb.val nb.val t.val).take q.2.val)
  · rintro ⟨w, j⟩ ⟨hj, hw⟩
    unfold keep_on_loop0_loop0.body
    simp only at hj hw ⊢
    split
    · rename_i hlt
      have hjl : j.val < nb.val := by scalar_tac
      step as ⟨v, hv⟩
      have hv' : v = memb.val.getD j.val default := by
        rw [hv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
      have htv : t.val < v.val.length := by rw [hv']; exact ht j.val hjl
      step as ⟨b, hb⟩
      have hwl : w.val.length < Usize.max := by rw [hw]; simp; scalar_tac
      step as ⟨w1, hw1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [hw1, hw, hj1, List.take_succ, List.getElem?_eq_getElem (by simp [colOf]; omega)]
      simp only [Option.toList_some]
      congr 1
      simp only [colOf, List.getElem_map, List.getElem_range]
      rw [← hv', List.getD_eq_getElem?_getD, List.getElem?_eq_getElem htv, hb]; rfl
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nb.val := by scalar_tac
      rw [hw, this, List.take_of_length_le (by simp [colOf])]
  · exact ⟨hi, hm⟩

theorem keep_loop_spec (s : Split) (rule : Rule) (memb : Slice (alloc.vec.Vec Bool)) (nb : Usize)
    (hnb : nb.val ≤ memb.val.length)
    (ht : ∀ i, i < nb.val → s.topes.val.length ≤ (memb.val.getD i default).val.length)
    (keep : alloc.vec.Vec Bool) (t : Usize) (htl : t.val ≤ s.topes.val.length)
    (hk : keep.val = (List.range t.val).map fun t' => applyL rule (colOf memb.val nb.val t')) :
    keep_on_loop0 s rule memb nb keep t ⦃ r =>
      ({ facets := r.1, plus := r.2.1, minus := r.2.2.1, topes := r.2.2.2.1, tope_pos := r.2.2.2.2.1,
         first_face := r.2.2.2.2.2.1, piece_of := r.2.2.2.2.2.2.1 } : Split) = s ∧
      r.2.2.2.2.2.2.2.val = (List.range s.topes.val.length).map fun t' => applyL rule (colOf memb.val nb.val t') ⦄ := by
  unfold keep_on_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec Bool × Usize) => s.topes.val.length - q.2.val)
    (fun q => q.2.val ≤ s.topes.val.length ∧
      q.1.val = (List.range q.2.val).map fun t' => applyL rule (colOf memb.val nb.val t'))
  · rintro ⟨w, j⟩ ⟨hj, hw⟩
    unfold keep_on_loop0.body
    simp only at hj hw ⊢
    split
    · rename_i hlt
      have hjl : j.val < s.topes.val.length := by scalar_tac
      step with keep_col_spec memb nb j hnb (fun i hi => by have := ht i hi; omega) (alloc.vec.Vec.new Bool)
        0#usize (by simp) (by simp) as ⟨m, hm⟩
      step with apply_rule_spec rule (alloc.vec.Vec.deref m) as ⟨b, hb⟩
      have hwl : w.val.length < Usize.max := by rw [hw]; simp; scalar_tac
      step as ⟨w1, hw1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [hw1, hw, hj1, List.range_succ, List.map_append, hb]
      simp [alloc.vec.Vec.deref, hm]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = s.topes.val.length := by scalar_tac
      exact ⟨trivial, by rw [hw, this]⟩
  · exact ⟨htl, hk⟩

/-! ## Stage 5: the result facets -/

def keepAt (keep : List Bool) (t : Nat) : Bool := keep.getD t false

/-- Facet `k` is in the result: its two sides kept differently. -/
def inRes (s : Split) (keep : List Bool) (k : Nat) : Bool := keepAt keep (tpP s k) != keepAt keep (tpM s k)

def resIdx (s : Split) (keep : List Bool) (k : Nat) : List Nat := (List.range k).filter (inRes s keep)

def resIds (s : Split) (keep : List Bool) (k : Nat) : List Nat := (resIdx s keep k).map (at32 s.facets)

def orientOf (s : Split) (keep : List Bool) (k : Nat) : Sgn := if keepAt keep (tpM s k) then .Pos else .Neg

theorem resIdx_succ (s : Split) (keep : List Bool) (k : Nat) :
    resIdx s keep (k + 1) = resIdx s keep k ++ (if inRes s keep k then [k] else []) := by
  unfold resIdx; rw [List.range_succ, List.filter_append]; simp [List.filter_cons]

theorem resIds_nodup (s : Split) (keep : List Bool) (hnd : (s.facets.val.map (·.val)).Nodup) :
    ∀ k, k ≤ s.facets.val.length → (resIds s keep k).Nodup := by
  intro k hk
  unfold resIds resIdx
  apply List.Nodup.map_on
  · intro a ha b hb h
    simp only [List.mem_filter, List.mem_range] at ha hb
    rw [at32_getD (by omega), at32_getD (by omega)] at h
    have := (List.Nodup.getElem_inj_iff hnd (i := a) (j := b) (hi := by simp; omega) (hj := by simp; omega)).mp
      (by simpa using h)
    exact this
  · exact List.Nodup.filter _ List.nodup_range

theorem keep_res_spec (s : Split) (keep : Slice Bool) (N : Nat)
    (hwf : SplitWF s 0) (hnd : (s.facets.val.map (·.val)).Nodup)
    (hFN : ∀ x ∈ s.facets.val, x.val < N) (hsmall : s.facets.val.length < U32.max)
    (hkl : s.topes.val.length ≤ keep.val.length)
    (R : alloc.vec.Vec U32) (O : alloc.vec.Vec Sgn) (RP : alloc.vec.Vec U32) (k : Usize)
    (hk : k.val ≤ s.facets.val.length)
    (hinv : TInv RP.val R.val (resIds s keep.val k.val) N ∧
      O.val = (resIdx s keep.val k.val).map (orientOf s keep.val)) :
    core_k_loop0 s keep R O RP k ⦃ r =>
      r.1 = s.facets ∧ r.2.1 = s.plus ∧ r.2.2.1 = s.minus ∧ r.2.2.2.1 = s.topes ∧ r.2.2.2.2.1 = s.tope_pos ∧
      r.2.2.2.2.2.1 = s.first_face ∧ r.2.2.2.2.2.2.1 = s.piece_of ∧
      TInv r.2.2.2.2.2.2.2.2.2.val r.2.2.2.2.2.2.2.1.val (resIds s keep.val s.facets.val.length) N ∧
      r.2.2.2.2.2.2.2.2.1.val = (resIdx s keep.val s.facets.val.length).map (orientOf s keep.val) ⦄ := by
  unfold core_k_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × alloc.vec.Vec Sgn × alloc.vec.Vec U32 × Usize) => s.facets.val.length - q.2.2.2.val)
    (fun q => q.2.2.2.val ≤ s.facets.val.length ∧ TInv q.2.2.1.val q.1.val (resIds s keep.val q.2.2.2.val) N ∧
      q.2.1.val = (resIdx s keep.val q.2.2.2.val).map (orientOf s keep.val))
  · rintro ⟨R0, O0, RP0, j⟩ ⟨hj, hti, ho⟩
    unfold core_k_loop0.body
    simp only at hj hti ho ⊢
    split
    · rename_i hlt
      have hjl : j.val < s.facets.val.length := by scalar_tac
      have hPl := hwf.plus_len; have hMl := hwf.minus_len
      have hp1 := hwf.plus_in j.val hjl; have hp2 := hwf.plus_pos j.val hjl
      have hm1 := hwf.minus_in j.val hjl; have hm2 := hwf.minus_pos j.val hjl
      step as ⟨i1, hi1⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = at32 s.plus j.val := by rw [hi2, at32_getD (by omega), hi1]; scalar_tac
      step as ⟨i3, hi3⟩
      step as ⟨i4, hi4⟩
      have hi4v : i4.val = tpP s j.val := by
        rw [hi4, tpP, at32_getD (by omega)]; simp only [hi2v] at hi3; rw [hi3]; scalar_tac
      have hi4l : i4.val < keep.val.length := by rw [hi4v]; unfold tpP; omega
      step as ⟨kp, hkp⟩
      have hkpv : kp = keepAt keep.val (tpP s j.val) := by
        rw [hkp, keepAt, List.getD_eq_getElem?_getD, ← hi4v, List.getElem?_eq_getElem hi4l]; rfl
      step as ⟨i5, hi5⟩
      step as ⟨i6, hi6⟩
      have hi6v : i6.val = at32 s.minus j.val := by rw [hi6, at32_getD (by omega), hi5]; scalar_tac
      step as ⟨i7, hi7⟩
      step as ⟨i8, hi8⟩
      have hi8v : i8.val = tpM s j.val := by
        rw [hi8, tpM, at32_getD (by omega)]; simp only [hi6v] at hi7; rw [hi7]; scalar_tac
      have hi8l : i8.val < keep.val.length := by rw [hi8v]; unfold tpM; omega
      step as ⟨km, hkm⟩
      have hkmv : km = keepAt keep.val (tpM s j.val) := by
        rw [hkm, keepAt, List.getD_eq_getElem?_getD, ← hi8v, List.getElem?_eq_getElem hi8l]; rfl
      have hsucc := resIdx_succ s keep.val j.val
      have hRidx : resIds s keep.val (j.val + 1) = resIds s keep.val j.val ++
          (if inRes s keep.val j.val then [at32 s.facets j.val] else []) := by
        unfold resIds; rw [hsucc]; split <;> simp
      split
      · rename_i hne
        have hin : inRes s keep.val j.val = true := by unfold inRes; rw [← hkpv, ← hkmv]; simpa using hne
        step as ⟨x, hx⟩
        have hxv : x.val = at32 s.facets j.val := by rw [at32_getD hjl, hx]
        have hxN : x.val < N := hFN x (by rw [hx]; exact List.getElem_mem _)
        have hnot : x.val ∉ resIds s keep.val j.val := by
          intro h
          have hnd2 := resIds_nodup s keep.val hnd (j.val + 1) (by omega)
          rw [hRidx, if_pos hin, ← hxv] at hnd2
          exact (List.nodup_append.mp hnd2).2.2 x.val h x.val (List.mem_singleton_self _) rfl
        have hRP := hti.1
        step as ⟨i11, hi11⟩
        have hi11v : i11.val = x.val := by rw [hi11]; scalar_tac
        step as ⟨pr, hpr1, hpr2⟩
        obtain ⟨w, back⟩ := pr
        simp only at hpr1 hpr2 ⊢
        have hback := hpr2
        have hRl := tinv_small hti
        have hResl : (resIds s keep.val j.val).length ≤ j.val := by
          unfold resIds resIdx; simp; exact (List.length_filter_le _ _).trans (by simp)
        step as ⟨i12, hi12⟩
        have hi12v : i12.val = (dedup (resIds s keep.val j.val)).length := by
          rw [hi12, UScalar.cast_val_eq]; simp only [alloc.vec.Vec.len_val]
          rw [Nat.mod_eq_of_lt (by have := U32.max_eq; simp; rw [hRl, dedup_of_nodup (resIds_nodup s keep.val hnd j.val (by omega))]; omega)]
          exact hRl
        have hR0l : R0.val.length < Usize.max := by
          rw [hRl, dedup_of_nodup (resIds_nodup s keep.val hnd j.val (by omega))]; scalar_tac
        step as ⟨R1, hR1⟩
        have hti' : TInv (RP0.val.set x.val i12) (R0.val ++ [x]) (resIds s keep.val j.val ++ [x.val]) N := by
          have := tinv_add_new hti x hxN hnot i12 hi12v
          exact this
        have hO0l : O0.val.length < Usize.max := by
          have h1 : (resIdx s keep.val j.val).length ≤ j.val := by
            unfold resIdx; exact (List.length_filter_le _ _).trans (by simp)
          have := u32_le_usize
          rw [ho, List.length_map]; omega
        split
        · rename_i hkmt
          step as ⟨O1, hO1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
          · rw [hj1, hRidx, if_pos hin, ← hxv, hR1, hback, alloc.vec.Vec.set_val_eq, hi11v]; exact hti'
          · rw [hO1, ho, hj1, hsucc, if_pos hin]; simp [orientOf, ← hkmv, hkmt]
        · rename_i hkmt
          step as ⟨O1, hO1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
          · rw [hj1, hRidx, if_pos hin, ← hxv, hR1, hback, alloc.vec.Vec.set_val_eq, hi11v]; exact hti'
          · rw [hO1, ho, hj1, hsucc, if_pos hin]; simp [orientOf, ← hkmv, hkmt]
      · rename_i hne
        have hin : inRes s keep.val j.val = false := by unfold inRes; rw [← hkpv, ← hkmv]; simpa using hne
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
        · rw [hj1, hRidx, hin]; simpa using hti
        · rw [ho, hj1, hsucc, hin]; simp
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = s.facets.val.length := by scalar_tac
      rw [← this]; exact ⟨trivial, trivial, trivial, trivial, trivial, trivial, trivial, hti, ho⟩
  · exact ⟨hk, hinv⟩

end forge_bool_core
