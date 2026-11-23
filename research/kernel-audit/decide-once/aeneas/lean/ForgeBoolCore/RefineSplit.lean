import ForgeBoolCore.RefineExt
import ForgeBoolCore.RefineUF
import ForgeBoolCore.DedupModel
open Aeneas Aeneas.Std Result

/-!
# `split`, stage by stage

Loop 0: the facets are the pieces' cells, first occurrences in order (`dedup`), and `first_face`
holds each one's first face.
-/

namespace forge_bool_core

open ForgeExact.DecideOnce ForgeBoolCore.Dedup

/-- The pieces as `(cell id, face id)`. -/
def pcs (l : Ledger) : List (Nat × Nat) := l.pieces.val.map fun p => ((p.1 : U32).val, (p.2 : U32).val)

/-- A cell's first face among some pieces. -/
def firstF (ps : List (Nat × Nat)) (c : Nat) : Option Nat := (ps.find? (·.1 == c)).map (·.2)

theorem firstF_snoc (ps : List (Nat × Nat)) (p : Nat × Nat) (c : Nat) :
    firstF (ps ++ [p]) c = (firstF ps c).or (if p.1 = c then some p.2 else none) := by
  unfold firstF
  rw [List.find?_append]
  cases h : ps.find? (·.1 == c) <;> simp [List.find?_cons]
  split <;> simp_all

theorem firstF_none (ps : List (Nat × Nat)) (c : Nat) : firstF ps c = none ↔ c ∉ ps.map Prod.fst := by
  unfold firstF
  simp only [Option.map_eq_none_iff, List.find?_eq_none, beq_iff_eq, List.mem_map]
  constructor
  · rintro h ⟨q, hq, e⟩; exact h q hq e
  · intro h q hq e; exact h ⟨q, hq, e⟩

theorem firstF_mem {ps : List (Nat × Nat)} {c v : Nat} (h : firstF ps c = some v) : (c, v) ∈ ps := by
  unfold firstF at h
  obtain ⟨q, hq, rfl⟩ := Option.map_eq_some_iff.mp h
  have := List.find?_some hq
  have hm := List.mem_of_find?_eq_some hq
  simp at this; rw [← this]; exact hm

theorem getD_set_u32 (t : List U32) (c c' : Nat) (v : U32) (hc : c < t.length) :
    (t.set c v).getD c' NONE = if c' = c then v else t.getD c' NONE := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_set, List.getD_eq_getElem?_getD]
  by_cases h : c = c'
  · subst h; simp [hc]
  · simp [h, Ne.symm h]

theorem split_loop0_spec (l : Ledger) (ff fac : alloc.vec.Vec U32) (k : Usize)
    (hk : k.val ≤ l.pieces.val.length)
    (hin : ∀ p ∈ pcs l, p.1 < ff.val.length) (hf : ∀ p ∈ pcs l, p.2 < U32.max)
    (hfac : fac.val.map (·.val) = dedup (((pcs l).take k.val).map Prod.fst))
    (hff : ∀ c, c < ff.val.length → (ff.val.getD c NONE).val = ((firstF ((pcs l).take k.val) c).getD U32.max)) :
    split_loop0 l ff fac k ⦃ r =>
      r.1 = l.nb ∧ r.2.1 = l.face_body ∧ r.2.2.1 = l.face_class ∧ r.2.2.2.1 = l.cells ∧
      r.2.2.2.2.1 = l.pieces ∧ r.2.2.2.2.2.1 = l.inside ∧ r.2.2.2.2.2.2.1 = l.edges ∧
      r.2.2.2.2.2.2.2.1 = l.verts ∧
      r.2.2.2.2.2.2.2.2.1.val.length = ff.val.length ∧
      (∀ c, c < ff.val.length → (r.2.2.2.2.2.2.2.2.1.val.getD c NONE).val = ((firstF (pcs l) c).getD U32.max)) ∧
      r.2.2.2.2.2.2.2.2.2.val.map (·.val) = dedup ((pcs l).map Prod.fst) ⦄ := by
  have hpl : (pcs l).length = l.pieces.val.length := by simp [pcs]
  unfold split_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) => l.pieces.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ l.pieces.val.length ∧ q.1.val.length = ff.val.length ∧
      q.2.1.val.map (·.val) = dedup (((pcs l).take q.2.2.val).map Prod.fst) ∧
      ∀ c, c < ff.val.length → (q.1.val.getD c NONE).val = ((firstF ((pcs l).take q.2.2.val) c).getD U32.max))
  · rintro ⟨t, F, j⟩ ⟨hj, htl, hF, ht⟩
    unfold split_loop0.body
    simp only at hj htl hF ht ⊢
    split
    · rename_i hlt
      have hjl : j.val < l.pieces.val.length := by scalar_tac
      step as ⟨c, f, hp⟩
      have hpj : (pcs l)[j.val]'(by omega) = ((c : U32).val, (f : U32).val) := by
        simp [pcs, ← hp]
      have hmem : ((c : U32).val, (f : U32).val) ∈ pcs l := by rw [← hpj]; exact List.getElem_mem _
      have hcin : (c : U32).val < ff.val.length := hin _ hmem
      have hfl : (f : U32).val < U32.max := hf _ hmem
      have htake : (pcs l).take (j.val + 1) = (pcs l).take j.val ++ [((c : U32).val, (f : U32).val)] := by
        rw [List.take_succ, List.getElem?_eq_getElem (by omega), hpj]; rfl
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = (c : U32).val := by rw [hi1]; scalar_tac
      step as ⟨g, hg⟩
      have hgv : g.val = ((firstF ((pcs l).take j.val) (c : U32).val).getD U32.max) := by
        rw [← ht _ hcin]
        simp only [hi1v] at hg
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), hg]; rfl
      have hFl : F.val.length ≤ j.val := by
        have := congrArg List.length hF
        simp only [List.length_map] at this
        rw [this]
        have h1 := dedup_length_le (((pcs l).take j.val).map Prod.fst)
        have h2 : ((pcs l).take j.val).length ≤ j.val := List.length_take_le _ _
        rw [List.length_map] at h1; omega
      split
      · rename_i hnone
        have hnew : firstF ((pcs l).take j.val) (c : U32).val = none := by
          cases h : firstF ((pcs l).take j.val) (c : U32).val with
          | none => rfl
          | some v =>
            exfalso
            rw [h] at hgv
            have hv := hf _ (List.mem_of_mem_take (firstF_mem h))
            have : g.val = U32.max := by rw [hnone, none_val]
            simp at hgv; omega
        have hnot := (firstF_none _ _).mp hnew
        step as ⟨i3, hi3⟩
        have hi3v : i3.val = (c : U32).val := by rw [hi3]; scalar_tac
        step as ⟨w, hw, hback⟩
        obtain ⟨w1, back⟩ := w
        simp only at hw hback ⊢
        step as ⟨F1, hF1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, by rw [hback]; simp [htl], ?_, ?_, by scalar_tac⟩
        · rw [hj1, htake, List.map_append, List.map_singleton, dedup_snoc, ← hF, hF1, if_neg hnot]; simp
        · intro c' hc'
          rw [hj1, htake, hback, alloc.vec.Vec.set_val_eq, hi3v, getD_set_u32 _ _ _ _ (by omega),
            firstF_snoc]
          by_cases e : c' = (c : U32).val
          · subst e; simp [hnew]
          · rw [if_neg e, ht c' hc']
            have : ¬(c : U32).val = c' := fun h => e h.symm
            simp [this]
      · rename_i hsome
        have hold : (c : U32).val ∈ ((pcs l).take j.val).map Prod.fst := by
          by_contra hn
          have := (firstF_none _ _).mpr hn
          rw [this] at hgv; simp at hgv
          exact hsome (UScalar.eq_of_val_eq (by rw [hgv, none_val]))
        obtain ⟨v, hv⟩ : ∃ v, firstF ((pcs l).take j.val) (c : U32).val = some v := by
          cases h : firstF ((pcs l).take j.val) (c : U32).val with
          | none => exact absurd ((firstF_none _ _).mp h) (fun h' => h' hold)
          | some v => exact ⟨v, rfl⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, htl, ?_, ?_, by scalar_tac⟩
        · rw [hj1, htake, List.map_append, List.map_singleton, dedup_snoc, ← hF, if_pos hold]; simp
        · intro c' hc'
          rw [hj1, htake, firstF_snoc, ht c' hc']
          by_cases e : c' = (c : U32).val
          · subst e; simp [hv]
          · have : ¬(c : U32).val = c' := fun h => e h.symm
            simp [this]
    · rename_i hge
      have : j.val = l.pieces.val.length := by scalar_tac
      rw [this, ← hpl, List.take_length] at hF ht
      simp only [WP.spec_ok, true_and]
      exact ⟨htl, ht, hF⟩
  · exact ⟨hk, rfl, hfac, hff⟩

end forge_bool_core
