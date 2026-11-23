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

/-! ## Loop 1: each facet's two sides, interned -/

/-- The ledger rebuilt from its fields, as the translation passes it. -/
theorem ledger_eta (l : Ledger) :
    ({ nb := l.nb, face_body := l.face_body, face_class := l.face_class, cells := l.cells,
       pieces := l.pieces, inside := l.inside, edges := l.edges, verts := l.verts } : Ledger) = l := rfl

/-- The side of a covector across its first zero. -/
def sideL (x : List Sgn) (s : Sgn) : List Sgn := x.set (x.idxOf .Zero) s

/-- `ext` grows from `e0`: every earlier cell keeps its covector. -/
def Grows (l : Ledger) (e0 e : Ext) : Prop :=
  (allCells l e0).length ≤ (allCells l e).length ∧
  ∀ id, id < (allCells l e0).length → covAt l e id = covAt l e0 id

theorem grows_refl (l : Ledger) (e : Ext) : Grows l e e := ⟨Nat.le_refl _, fun _ _ => rfl⟩

theorem grows_trans {l : Ledger} {a b c : Ext} (h1 : Grows l a b) (h2 : Grows l b c) : Grows l a c :=
  ⟨Nat.le_trans h1.1 h2.1, fun id h => by rw [h2.2 id (by have := h1.1; omega), h1.2 id h]⟩

theorem grows_intern {l : Ledger} {e e' : Ext} (key : List Sgn)
    (h : allCells l e' = allCells l e ++ (if key ∈ allCells l e then [] else [key])) : Grows l e e' := by
  refine ⟨by rw [h]; simp, fun id hid => ?_⟩
  simp only [covAt, h, List.getD_eq_getElem?_getD, List.getElem?_append_left hid]

theorem allCells_grow {l : Ledger} {e e' : Ext} (key : List Sgn)
    (h : allCells l e' = allCells l e ++ (if key ∈ allCells l e then [] else [key])) :
    (allCells l e').length ≤ (allCells l e).length + 1 := by
  rw [h]; split <;> simp

theorem to_vec_sgn_spec (x : Slice Sgn) :
    alloc.slice.Slice.to_vec Sgn.Insts.CoreCloneClone x ⦃ v => v.val = x.val ⦄ := by
  have hclone : ∀ y ∈ x.val, Sgn.Insts.CoreCloneClone.clone y = ok y := by intro y _; cases y <;> rfl
  step as ⟨v, hv⟩
  rw [hv]; rfl

/-- Loop 1's invariant at `j`: the first `j` facets' sides interned. -/
def L1Inv (l : Ledger) (e0 : Ext) (F : List U32) (e : Ext) (P M : List U32) (j : Nat) : Prop :=
  j ≤ F.length ∧ ExtOK l e ∧ Grows l e0 e ∧ (allCells l e).length ≤ (allCells l e0).length + 2 * j ∧
  P.length = j ∧ M.length = j ∧
  ∀ k (hk : k < j),
    P[k]!.val < (allCells l e).length ∧ covAt l e P[k]!.val = sideL (covAt l e0 F[k]!.val) .Pos ∧
    M[k]!.val < (allCells l e).length ∧ covAt l e M[k]!.val = sideL (covAt l e0 F[k]!.val) .Neg

theorem split_loop1_spec (l : Ledger) (e0 : Ext) (hok : ExtOK l e0) (F : alloc.vec.Vec U32)
    (hF : ∀ x ∈ F.val, x.val < l.cells.val.length)
    (hroom : (allCells l e0).length + 2 * F.val.length + 1 < U32.max)
    (e : Ext) (P M : alloc.vec.Vec U32) (j : Usize) (hinv : L1Inv l e0 F.val e P.val M.val j.val) :
    split_loop1 l.nb l.face_body l.face_class l.cells l.pieces l.inside l.edges l.verts e F P M j ⦃ r =>
      L1Inv l e0 F.val r.1 r.2.1.val r.2.2.val F.val.length ⦄ := by
  unfold split_loop1
  apply loop.spec_decr_nat
    (fun (q : Ext × alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) => F.val.length - q.2.2.2.val)
    (fun q => L1Inv l e0 F.val q.1 q.2.1.val q.2.2.1.val q.2.2.2.val)
  · rintro ⟨e1, P1, M1, i⟩ ⟨hi, hok1, hgr, hlen, hPl, hMl, hPM⟩
    unfold split_loop1.body
    simp only at hi hok1 hgr hlen hPl hMl hPM ⊢
    split
    · rename_i hlt
      have hil : i.val < F.val.length := by scalar_tac
      step as ⟨c, hc⟩
      have hcF : c ∈ F.val := by rw [hc]; exact List.getElem_mem _
      have hcl : c.val < l.cells.val.length := hF c hcF
      have hcl0 : c.val < (allCells l e0).length := by rw [allCells_length]; omega
      have hcl1 : c.val < (allCells l e1).length := by have := hgr.1; omega
      step with cov_spec l e1 c hcl1 as ⟨sx, hsx⟩
      have hsx0 : sx.val = covAt l e0 c.val := by rw [hsx, hgr.2 _ hcl0]
      step with to_vec_sgn_spec sx as ⟨x, hx⟩
      have hxd : (alloc.vec.Vec.deref x).val = covAt l e0 c.val := by simp [alloc.vec.Vec.deref, hx, hsx0]
      step with zero_idx_spec (alloc.vec.Vec.deref x) as ⟨z, hz⟩
      have hzv : z.val = (covAt l e0 c.val).idxOf .Zero := by
        rw [hz]; simp only [covOf, ForgeExact.DecideOnce.Cov.zeroIdx, idxOf_map_toS, hxd]
      step with with_set_spec (alloc.vec.Vec.deref x) z Sgn.Pos as ⟨vp, hvp⟩
      have hvp' : vp.val = sideL (covAt l e0 c.val) .Pos := by rw [hvp, hxd, hzv]; rfl
      step with intern_spec l e1 hok1 vp (by omega) as ⟨ip, e2, _, hok2, hall2, hipl, hipc⟩
      have hg12 := grows_intern vp.val hall2
      have hl2 := allCells_grow vp.val hall2
      step as ⟨P2, hP2⟩
      step with with_set_spec (alloc.vec.Vec.deref x) z Sgn.Neg as ⟨vm, hvm⟩
      have hvm' : vm.val = sideL (covAt l e0 c.val) .Neg := by rw [hvm, hxd, hzv]; rfl
      step with intern_spec l e2 hok2 vm (by omega) as ⟨im, e3, _, hok3, hall3, himl, himc⟩
      have hg23 := grows_intern vm.val hall3
      have hl3 := allCells_grow vm.val hall3
      step as ⟨M2, hM2⟩
      step as ⟨i1, hi1⟩
      have hFi : F.val[i.val]!.val = c.val := by rw [getElem!_pos F.val i.val hil, hc]
      refine ⟨⟨by scalar_tac, hok3, grows_trans hgr (grows_trans hg12 hg23), by rw [hi1]; omega,
        by rw [hP2, hi1]; simp [hPl], by rw [hM2, hi1]; simp [hMl], fun k hk => ?_⟩, by scalar_tac⟩
      rw [hi1] at hk
      by_cases hki : k < i.val
      · obtain ⟨a1, a2, a3, a4⟩ := hPM k hki
        have hPk : P2.val[k]! = P1.val[k]! := by
          rw [hP2, getElem!_pos _ k (by simp; omega), getElem!_pos _ k (by omega)]
          simp [List.getElem_append_left (by omega : k < P1.val.length)]
        have hMk : M2.val[k]! = M1.val[k]! := by
          rw [hM2, getElem!_pos _ k (by simp; omega), getElem!_pos _ k (by omega)]
          simp [List.getElem_append_left (by omega : k < M1.val.length)]
        rw [hPk, hMk]
        have g := grows_trans hg12 hg23
        refine ⟨by have := g.1; omega, by rw [g.2 _ a1]; exact a2, by have := g.1; omega, by rw [g.2 _ a3]; exact a4⟩
      · have hk' : k = i.val := by omega
        subst hk'
        have hPk : P2.val[i.val]! = ip := by
          rw [hP2, getElem!_pos _ i.val (by simp; omega)]; simp [hPl]
        have hMk : M2.val[i.val]! = im := by
          rw [hM2, getElem!_pos _ i.val (by simp; omega)]; simp [hMl]
        rw [hPk, hMk, hFi]
        refine ⟨by have := hg23.1; omega, ?_, himl, by rw [himc, hvm']⟩
        rw [hg23.2 _ hipl, hipc, hvp']
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = F.val.length := by scalar_tac
      rw [← this]; exact ⟨hi, hok1, hgr, hlen, hPl, hMl, hPM⟩
  · exact hinv

end forge_bool_core
