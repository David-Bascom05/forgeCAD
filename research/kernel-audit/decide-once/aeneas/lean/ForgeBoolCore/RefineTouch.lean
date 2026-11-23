import ForgeBoolCore.RefineAround

/-!
# The touch rule in `keep_on`

`is_edge` marks the edge cells (`is_edge_spec`); the edge scan finds the first edge with more than
two result facets round it, or none, and keeps every edge's `around` (`edge_scan_spec`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

theorem is_edge_spec (v : alloc.vec.Vec CellId) (E : alloc.vec.Vec Bool) (k : Usize) (N : Nat)
    (hk : k.val ≤ v.val.length) (hl : E.val.length = N)
    (hE : ∀ c, c < N → E.val.getD c false = decide (c ∈ (v.val.take k.val).map (fun (x : U32) => x.val))) :
    keep_on_loop2 v E k ⦃ r => r.val.length = N ∧
      ∀ c, c < N → r.val.getD c false = decide (c ∈ v.val.map (fun (x : U32) => x.val)) ⦄ := by
  unfold keep_on_loop2
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec Bool × Usize) => v.val.length - q.2.val)
    (fun q => q.2.val ≤ v.val.length ∧ q.1.val.length = N ∧
      ∀ c, c < N → q.1.val.getD c false = decide (c ∈ (v.val.take q.2.val).map (fun (x : U32) => x.val)))
  · rintro ⟨E0, j⟩ ⟨hj, hl0, hE0⟩
    unfold keep_on_loop2.body
    simp only at hj hl0 hE0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < v.val.length := by scalar_tac
      have htake : (v.val.take (j.val + 1)).map (fun (x : U32) => x.val) =
          (v.val.take j.val).map (fun (x : U32) => x.val) ++ [(v.val[j.val]'hjl : U32).val] := by
        rw [List.take_succ_eq_append_getElem hjl, List.map_append, List.map_singleton]
      step as ⟨ci, hci⟩
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = (v.val[j.val]'hjl : U32).val := by rw [hi1, hci]; scalar_tac
      split
      · rename_i hin
        step as ⟨i3, hi3⟩
        step as ⟨pr, h1, h2⟩
        obtain ⟨w, back⟩ := pr
        simp only at h1 h2 ⊢
        subst h2
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, by simp [hl0], ?_, by scalar_tac⟩
        intro c hc
        have hi3v : i3.val = (v.val[j.val]'hjl : U32).val := by rw [hi3, hci]; scalar_tac
        rw [hj1, htake, alloc.vec.Vec.set_val_eq, List.getD_eq_getElem?_getD]
        by_cases hce : c = i3.val
        · subst hce; rw [List.getElem?_set_self (by omega)]; simp [hi3v]
        · rw [List.getElem?_set_ne (Ne.symm hce), ← List.getD_eq_getElem?_getD, hE0 c hc]
          simp only [List.mem_append, List.mem_singleton]; rw [← hi3v]; simp [hce]
      · rename_i hout
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, hl0, ?_, by scalar_tac⟩
        intro c hc
        rw [hj1, htake, hE0 c hc]
        have : c ≠ (v.val[j.val]'hjl : U32).val := by
          intro e; apply hout
          have : i1.val < E0.val.length := by rw [hi1v, ← e]; omega
          scalar_tac
        simp [this]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = v.val.length := by scalar_tac
      refine ⟨hl0, fun c hc => ?_⟩
      rw [hE0 c hc, this, List.take_length]
  · exact ⟨hk, hl, hE⟩

theorem find_take_succ {f : Nat → Bool} {L : List Nat} {j : Nat} (hj : j < L.length)
    (h : (L.take j).find? f = none) :
    (L.take (j + 1)).find? f = if f L[j] then some L[j] else none := by
  rw [List.take_succ_eq_append_getElem hj, List.find?_append, h]; simp [List.find?_cons]; split <;> simp_all

theorem find_of_take {f : Nat → Bool} {L : List Nat} {j : Nat} {x : Nat} (h : (L.take j).find? f = some x) :
    L.find? f = some x := by
  rw [← List.take_append_drop j L, List.find?_append, h]; rfl

/-- An edge (by id) has more than two result facets round it. -/
def badE (l : Ledger) (ext : Ext) (a : Assembled) (e : Nat) : Bool := decide (2 < (aroundL l ext a (covAt l ext e)).length)

def idsOf (v : alloc.vec.Vec CellId) : List Nat := v.val.map fun (x : U32) => x.val

/-- What the edge scan leaves: no bad edge and every edge's `around`, or the first bad edge. -/
def ScanOut (l : Ledger) (ext : Ext) (a : Assembled) (es : List Nat) (bad : U32) (ae : List (List Nat)) : Prop :=
  (bad = NONE → es.find? (badE l ext a) = none ∧ ae = es.map fun e => aroundL l ext a (covAt l ext e)) ∧
  (bad ≠ NONE → es.find? (badE l ext a) = some bad.val)

theorem edge_scan_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (hE : ∀ e ∈ l.edges.val, (e : U32).val < (allCells l ext).length) (hEs : l.edges.val.length < U32.max) :
    keep_on_loop3 l.nb l.face_body l.face_class l.cells l.pieces l.inside l.edges l.verts ext
      a.result a.orient a.rpos NONE (alloc.vec.Vec.new (alloc.vec.Vec U32)) 0#usize ⦃ r =>
      ScanOut l ext a (idsOf l.edges) r.1 (r.2.val.map fun v => v.val.map (·.val)) ⦄ := by
  unfold keep_on_loop3
  apply loop.spec_decr_nat
    (fun (q : U32 × alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => l.edges.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ l.edges.val.length ∧ q.2.1.val.length = q.2.2.val ∧
      (q.1 = NONE → ((idsOf l.edges).take q.2.2.val).find? (badE l ext a) = none) ∧
      (q.1 = NONE → q.2.1.val.map (fun v => v.val.map (·.val)) =
        ((idsOf l.edges).take q.2.2.val).map fun e => aroundL l ext a (covAt l ext e)) ∧
      (q.1 ≠ NONE → (idsOf l.edges).find? (badE l ext a) = some q.1.val))
  · rintro ⟨b0, ae0, j⟩ ⟨hj, hael, hb1, hb2, hb3⟩
    unfold keep_on_loop3.body
    simp only at hj hael hb1 hb2 hb3 ⊢
    split
    · rename_i hnone
      split
      · rename_i hlt
        have hjl : j.val < l.edges.val.length := by scalar_tac
        have hil : j.val < (idsOf l.edges).length := by simp [idsOf]; omega
        have hget : (idsOf l.edges)[j.val]'hil = (l.edges.val[j.val]'hjl : U32).val := by simp [idsOf]
        have hft := find_take_succ (f := badE l ext a) hil (hb1 hnone)
        have htake : (idsOf l.edges).take (j.val + 1) = (idsOf l.edges).take j.val ++ [(idsOf l.edges)[j.val]'hil] :=
          List.take_succ_eq_append_getElem hil
        step as ⟨ci, hci⟩
        have hcil : ci.val < (allCells l ext).length := by rw [hci]; exact hE _ (List.getElem_mem _)
        have ha : ({ result := a.result, orient := a.orient, rpos := a.rpos } : Assembled) = a := rfl
        change (do let ar ← around l ext a ci; _) ⦃ _ ⦄
        step with around_spec l ext hok a W h ci hcil as ⟨ar, har⟩
        have hcv : ci.val = (idsOf l.edges)[j.val]'hil := by rw [hget, hci]
        have hael' : ae0.val.length < Usize.max := by
          have := u32_le_usize; have := U32.max_eq; scalar_tac
        have hbadv : badE l ext a ((idsOf l.edges)[j.val]'hil) = decide (2 < ar.val.length) := by
          unfold badE; rw [← hcv, ← har]; simp
        split
        · rename_i hgt
          have hbad : badE l ext a ((idsOf l.edges)[j.val]'hil) = true := by rw [hbadv]; simpa using hgt
          rw [if_pos hbad] at hft
          have hcn : ci ≠ NONE := by
            intro e; have := h.small; have := hok.small; rw [e, none_val] at hcil; omega
          step as ⟨ae1, hae1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, by simp [hae1, hael, hj1], fun e => absurd e hcn, fun e => absurd e hcn,
            fun _ => ?_, by scalar_tac⟩
          rw [hcv]; exact find_of_take hft
        · rename_i hgt
          have hbad : badE l ext a ((idsOf l.edges)[j.val]'hil) = false := by rw [hbadv]; simpa using hgt
          rw [if_neg (by simp [hbad])] at hft
          step as ⟨ae1, hae1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, by simp [hae1, hael, hj1], fun _ => by rw [hj1]; exact hft, fun _ => ?_,
            fun e => absurd hnone e, by scalar_tac⟩
          rw [hj1, hae1, htake, List.map_append, List.map_append, hb2 hnone, ← hcv]; simp [har]
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = l.edges.val.length := by scalar_tac
        have ht : (idsOf l.edges).take j.val = idsOf l.edges := List.take_of_length_le (by simp [idsOf]; omega)
        rw [ht] at hb1 hb2
        exact ⟨fun e => ⟨hb1 e, hb2 e⟩, hb3⟩
    · rename_i hsome
      simp only [WP.spec_ok]
      exact ⟨fun e => absurd e hsome, hb3⟩
  · refine ⟨by simp, by simp, fun _ => by simp, fun _ => by simp, fun h => absurd rfl h⟩

end forge_bool_core
