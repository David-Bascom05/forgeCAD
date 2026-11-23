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

/-! ## Loop 2: the topes, first-seen, with their positions -/

/-- A first-seen table: `Tp` lists the stream `S` without repeats, and `T` holds each id's
position in it, or `NONE`. -/
def TInv (T Tp : List U32) (S : List Nat) (n : Nat) : Prop :=
  T.length = n ∧ Tp.map (·.val) = dedup S ∧ (∀ x ∈ S, x < n) ∧
  ∀ c, c < n → (T.getD c NONE).val = if c ∈ S then (dedup S).idxOf c else U32.max

theorem tinv_none_iff {T Tp : List U32} {S : List Nat} {n p : Nat} (h : TInv T Tp S n) (hp : p < n)
    (hsmall : (dedup S).length < U32.max) : (T.getD p NONE).val = U32.max ↔ p ∉ S := by
  rw [h.2.2.2 p hp]
  by_cases hs : p ∈ S
  · simp only [hs, ite_true, not_true_eq_false, iff_false]
    have := List.idxOf_lt_length_iff.mpr ((mem_dedup _ _).mpr hs); omega
  · simp [hs]

theorem tinv_add_old {T Tp : List U32} {S : List Nat} {n p : Nat} (h : TInv T Tp S n) (hp : p ∈ S) :
    TInv T Tp (S ++ [p]) n := by
  obtain ⟨h1, h2, h3, h4⟩ := h
  refine ⟨h1, by rw [h2, dedup_snoc, if_pos hp, List.append_nil], fun x hx => ?_, fun c hc => ?_⟩
  · rcases List.mem_append.mp hx with hx | hx
    · exact h3 x hx
    · simp at hx; subst hx; exact h3 _ hp
  · rw [h4 c hc]
    have : c ∈ S ++ [p] ↔ c ∈ S := by simp; intro e; subst e; exact hp
    by_cases hcs : c ∈ S
    · rw [if_pos hcs, if_pos (this.mpr hcs), idxOf_dedup_snoc_old hcs]
    · rw [if_neg hcs, if_neg (fun h' => hcs (this.mp h'))]

theorem tinv_add_new {T Tp : List U32} {S : List Nat} {n : Nat} (h : TInv T Tp S n) (p : U32)
    (hp : p.val < n) (hnot : p.val ∉ S) (v : U32) (hv : v.val = (dedup S).length) :
    TInv (T.set p.val v) (Tp ++ [p]) (S ++ [p.val]) n := by
  obtain ⟨h1, h2, h3, h4⟩ := h
  refine ⟨by simp [h1], by rw [List.map_append, h2, dedup_snoc, if_neg hnot]; rfl, fun x hx => ?_, fun c hc => ?_⟩
  · rcases List.mem_append.mp hx with hx | hx
    · exact h3 x hx
    · simp at hx; subst hx; exact hp
  · rw [getD_set_u32 _ _ _ _ (by omega)]
    by_cases e : c = p.val
    · subst e
      rw [if_pos rfl, if_pos (by simp), idxOf_dedup_snoc_new hnot, hv]
    · rw [if_neg e, h4 c hc]
      have : c ∈ S ++ [p.val] ↔ c ∈ S := by simp [e]
      by_cases hcs : c ∈ S
      · rw [if_pos hcs, if_pos (this.mpr hcs), idxOf_dedup_snoc_old hcs]
      · rw [if_neg hcs, if_neg (fun h' => hcs (this.mp h'))]

/-- The sides in the order loop 2 meets them: each facet's plus, then its minus. -/
def stream (P M : List U32) : Nat → List Nat
  | 0 => []
  | k + 1 => stream P M k ++ [P[k]!.val] ++ [M[k]!.val]

theorem stream_length (P M : List U32) : ∀ k, (stream P M k).length = 2 * k
  | 0 => rfl
  | k + 1 => by simp [stream, stream_length P M k]; omega

theorem tinv_small {T Tp : List U32} {S : List Nat} {n : Nat} (h : TInv T Tp S n) :
    Tp.length = (dedup S).length := by
  have := congrArg List.length h.2.1; simpa using this

/-- Loop 2's minus half and step, from the table after the plus half. -/
theorem split_loop2_spec (F P M T Tp : alloc.vec.Vec U32) (n : Nat) (i : Usize)
    (hP : P.val.length = F.val.length) (hM : M.val.length = F.val.length)
    (hin : ∀ k, k < F.val.length → P.val[k]!.val < n ∧ M.val[k]!.val < n)
    (hroom : 2 * F.val.length < U32.max) (hi : i.val ≤ F.val.length)
    (hinv : TInv T.val Tp.val (stream P.val M.val i.val) n) :
    split_loop2 F P M T Tp i ⦃ r => TInv r.1.val r.2.val (stream P.val M.val F.val.length) n ⦄ := by
  unfold split_loop2
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) => F.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ F.val.length ∧ TInv q.1.val q.2.1.val (stream P.val M.val q.2.2.val) n)
  · rintro ⟨T0, Tp0, j⟩ ⟨hj, hti⟩
    unfold split_loop2.body
    simp only at hj hti ⊢
    split
    · rename_i hlt
      have hjl : j.val < F.val.length := by scalar_tac
      obtain ⟨hpn, hmn⟩ := hin j.val hjl
      have hS := stream_length P.val M.val j.val
      have hd0 : (dedup (stream P.val M.val j.val)).length < U32.max := by
        have := dedup_length_le (stream P.val M.val j.val); omega
      have hT0 := hti.1
      step as ⟨p, hp⟩
      have hpv : P.val[j.val]! = p := by rw [getElem!_pos _ _ (by omega), hp]
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = p.val := by rw [hi2]; scalar_tac
      rw [hpv] at hpn
      step as ⟨g, hg⟩
      have hgv : (T0.val.getD p.val NONE) = g := by
        simp only [hi2v] at hg
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), hg]; rfl
      split
      · rename_i hnone
        have hnot : p.val ∉ stream P.val M.val j.val :=
          (tinv_none_iff hti hpn hd0).mp (by rw [hgv, hnone, none_val])
        step as ⟨i5, hi5⟩
        have hi5v : i5.val = p.val := by rw [hi5]; scalar_tac
        step as ⟨w, hw, hback⟩
        obtain ⟨w1, back⟩ := w
        simp only at hw hback ⊢
        have hTpl := tinv_small hti
        step as ⟨i6, hi6⟩
        have hi6v : i6.val = (dedup (stream P.val M.val j.val)).length := by
          rw [hi6, UScalar.cast_val_eq]; simp only [alloc.vec.Vec.len_val]
          rw [Nat.mod_eq_of_lt (by have := U32.max_eq; simp; omega)]; simpa using hTpl
        step as ⟨Tp1, hTp1⟩
        have h1 : TInv (back i6).val Tp1.val (stream P.val M.val j.val ++ [P.val[j.val]!.val]) n := by
          rw [hback, alloc.vec.Vec.set_val_eq, hTp1, hi5v, hpv]
          exact tinv_add_new hti p hpn hnot i6 hi6v
        try simp only [Std.bind_ok]
        generalize back i6 = T1 at h1 ⊢
        have hti1 := h1
        have hS := stream_length P.val M.val j.val
        have hd1 : (dedup (stream P.val M.val j.val ++ [P.val[j.val]!.val])).length < U32.max := by
          have h1 := dedup_length_le (stream P.val M.val j.val ++ [P.val[j.val]!.val])
          have h2 : (stream P.val M.val j.val ++ [P.val[j.val]!.val]).length = 2 * j.val + 1 := by simp [hS]
          omega
        have hT1 := hti1.1
        step as ⟨m, hm⟩
        have hmv : M.val[j.val]! = m := by rw [getElem!_pos _ _ (by omega), hm]
        rw [hmv] at hmn
        step as ⟨i4, hi4⟩
        have hi4v : i4.val = m.val := by rw [hi4]; scalar_tac
        step as ⟨g, hg⟩
        have hgv : (T1.val.getD m.val NONE) = g := by
          simp only [hi4v] at hg
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), hg]; rfl
        have hnext : stream P.val M.val (j.val + 1) =
            stream P.val M.val j.val ++ [P.val[j.val]!.val] ++ [m.val] := by simp only [stream, hmv]
        split
        · rename_i hnone
          have hnot : m.val ∉ stream P.val M.val j.val ++ [P.val[j.val]!.val] :=
            (tinv_none_iff hti1 hmn hd1).mp (by rw [hgv, hnone, none_val])
          step as ⟨i7, hi7⟩
          have hi7v : i7.val = m.val := by rw [hi7]; scalar_tac
          step as ⟨w, hw, hback⟩
          obtain ⟨w1, back⟩ := w
          simp only at hw hback ⊢
          have hTpl := tinv_small hti1
          step as ⟨i8, hi8⟩
          have hi8v : i8.val = (dedup (stream P.val M.val j.val ++ [P.val[j.val]!.val])).length := by
            rw [hi8, UScalar.cast_val_eq]; simp only [alloc.vec.Vec.len_val]
            rw [Nat.mod_eq_of_lt (by have := U32.max_eq; simp; omega)]; simpa using hTpl
          step as ⟨Tp2, hTp2⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hj1, hnext, hback, alloc.vec.Vec.set_val_eq, hTp2, hi7v]
          exact tinv_add_new hti1 m hmn hnot i8 hi8v
        · rename_i hsome
          have hin : m.val ∈ stream P.val M.val j.val ++ [P.val[j.val]!.val] := by
            by_contra hn
            exact hsome (UScalar.eq_of_val_eq (by rw [← hgv, (tinv_none_iff hti1 hmn hd1).mpr hn, none_val]))
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hj1, hnext]
          exact tinv_add_old hti1 hin

      · rename_i hsome
        have hin' : p.val ∈ stream P.val M.val j.val := by
          by_contra hn
          exact hsome (UScalar.eq_of_val_eq (by rw [← hgv, (tinv_none_iff hti hpn hd0).mpr hn, none_val]))
        have h1 : TInv T0.val Tp0.val (stream P.val M.val j.val ++ [P.val[j.val]!.val]) n := by
          rw [hpv]; exact tinv_add_old hti hin'
        simp only [Std.bind_ok]
        generalize T0 = T1 at h1 ⊢
        generalize Tp0 = Tp1 at h1 ⊢
        have hti1 := h1
        have hS := stream_length P.val M.val j.val
        have hd1 : (dedup (stream P.val M.val j.val ++ [P.val[j.val]!.val])).length < U32.max := by
          have h1 := dedup_length_le (stream P.val M.val j.val ++ [P.val[j.val]!.val])
          have h2 : (stream P.val M.val j.val ++ [P.val[j.val]!.val]).length = 2 * j.val + 1 := by simp [hS]
          omega
        have hT1 := hti1.1
        step as ⟨m, hm⟩
        have hmv : M.val[j.val]! = m := by rw [getElem!_pos _ _ (by omega), hm]
        rw [hmv] at hmn
        step as ⟨i4, hi4⟩
        have hi4v : i4.val = m.val := by rw [hi4]; scalar_tac
        step as ⟨g, hg⟩
        have hgv : (T1.val.getD m.val NONE) = g := by
          simp only [hi4v] at hg
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), hg]; rfl
        have hnext : stream P.val M.val (j.val + 1) =
            stream P.val M.val j.val ++ [P.val[j.val]!.val] ++ [m.val] := by simp only [stream, hmv]
        split
        · rename_i hnone
          have hnot : m.val ∉ stream P.val M.val j.val ++ [P.val[j.val]!.val] :=
            (tinv_none_iff hti1 hmn hd1).mp (by rw [hgv, hnone, none_val])
          step as ⟨i7, hi7⟩
          have hi7v : i7.val = m.val := by rw [hi7]; scalar_tac
          step as ⟨w, hw, hback⟩
          obtain ⟨w1, back⟩ := w
          simp only at hw hback ⊢
          have hTpl := tinv_small hti1
          step as ⟨i8, hi8⟩
          have hi8v : i8.val = (dedup (stream P.val M.val j.val ++ [P.val[j.val]!.val])).length := by
            rw [hi8, UScalar.cast_val_eq]; simp only [alloc.vec.Vec.len_val]
            rw [Nat.mod_eq_of_lt (by have := U32.max_eq; simp; omega)]; simpa using hTpl
          step as ⟨Tp2, hTp2⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hj1, hnext, hback, alloc.vec.Vec.set_val_eq, hTp2, hi7v]
          exact tinv_add_new hti1 m hmn hnot i8 hi8v
        · rename_i hsome
          have hin : m.val ∈ stream P.val M.val j.val ++ [P.val[j.val]!.val] := by
            by_contra hn
            exact hsome (UScalar.eq_of_val_eq (by rw [← hgv, (tinv_none_iff hti1 hmn hd1).mpr hn, none_val]))
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          rw [hj1, hnext]
          exact tinv_add_old hti1 hin

    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = F.val.length := by scalar_tac
      rw [← this]; exact hti
  · exact ⟨hi, hinv⟩

/-! ## Loop 3: each facet's position -/

theorem idxOf_take_succ {l : List Nat} (hnd : l.Nodup) {j : Nat} (hj : j < l.length) :
    l.idxOf l[j] = j := List.Nodup.idxOf_getElem hnd j hj

theorem split_loop3_spec (F FP0 FP : alloc.vec.Vec U32) (i : Usize)
    (hnd : (F.val.map (·.val)).Nodup) (hin : ∀ x ∈ F.val, x.val < FP0.val.length)
    (hsmall : F.val.length < U32.max) (hi : i.val ≤ F.val.length)
    (hlen : FP.val.length = FP0.val.length)
    (hinv : ∀ c, c < FP0.val.length → (FP.val.getD c NONE).val =
      if c ∈ (F.val.take i.val).map (·.val) then (F.val.map (·.val)).idxOf c else (FP0.val.getD c NONE).val) :
    split_loop3 F FP i ⦃ r => r.val.length = FP0.val.length ∧ ∀ c, c < FP0.val.length →
      (r.val.getD c NONE).val = if c ∈ F.val.map (·.val) then (F.val.map (·.val)).idxOf c
        else (FP0.val.getD c NONE).val ⦄ := by
  unfold split_loop3
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × Usize) => F.val.length - q.2.val)
    (fun q => q.2.val ≤ F.val.length ∧ q.1.val.length = FP0.val.length ∧ ∀ c, c < FP0.val.length →
      (q.1.val.getD c NONE).val = if c ∈ (F.val.take q.2.val).map (·.val) then (F.val.map (·.val)).idxOf c
        else (FP0.val.getD c NONE).val)
  · rintro ⟨T, j⟩ ⟨hj, hTl, hT⟩
    unfold split_loop3.body
    simp only at hj hTl hT ⊢
    split
    · rename_i hlt
      have hjl : j.val < F.val.length := by scalar_tac
      step as ⟨x, hx⟩
      have hxl : x.val < FP0.val.length := hin x (by rw [hx]; exact List.getElem_mem _)
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = x.val := by rw [hi3]; scalar_tac
      step as ⟨w0, back, hw0⟩
      rename_i hback
      step as ⟨i4, hi4⟩
      have hi4v : i4.val = j.val := by
        rw [hi4, UScalar.cast_val_eq]; apply Nat.mod_eq_of_lt; have := U32.max_eq; simp; omega
      step as ⟨j1, hj1⟩
      have hxj : (F.val.map (·.val))[j.val]'(by simp; omega) = x.val := by simp [hx]
      have htk : (F.val.take (j.val + 1)).map (·.val) = (F.val.take j.val).map (·.val) ++ [x.val] := by
        rw [List.take_succ, List.getElem?_eq_getElem hjl, ← hx]; simp
      refine ⟨by scalar_tac, by rw [hback]; simp [hTl], fun c hc => ?_, by scalar_tac⟩
      rw [hback, alloc.vec.Vec.set_val_eq, hi3v, getD_set_u32 _ _ _ _ (by omega), hj1, htk]
      by_cases e : c = x.val
      · subst e
        rw [if_pos rfl, if_pos (by simp), hi4v, ← hxj, idxOf_take_succ hnd]
      · rw [if_neg e, hT c hc]
        have : c ∈ (F.val.take j.val).map (·.val) ++ [x.val] ↔ c ∈ (F.val.take j.val).map (·.val) := by simp [e]
        by_cases h1 : c ∈ (F.val.take j.val).map (·.val)
        · rw [if_pos h1, if_pos (this.mpr h1)]
        · rw [if_neg h1, if_neg (fun h' => h1 (this.mp h'))]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = F.val.length := by scalar_tac
      rw [this, List.take_length] at hT
      exact ⟨hTl, hT⟩
  · exact ⟨hi, hlen, hinv⟩

/-! ## Loop 4: which bodies each facet is a piece of -/

/-- `Ledger.bodyOf`: the face's body, 0 past the end. -/
def bodyF (l : Ledger) (f : Nat) : Nat := ((l.face_body.val.getD f (0#u32 : U32)) : U32).val

theorem body_of_spec (l : Ledger) (f : FaceId) : body_of l f ⦃ r => r.val = bodyF l (f : U32).val ⦄ := by
  unfold body_of
  step as ⟨i, hi⟩
  have hiv : i.val = (f : U32).val := by rw [hi]; scalar_tac
  split
  · rename_i hlt
    have hl : (f : U32).val < l.face_body.val.length := by scalar_tac
    step as ⟨i2, hi2⟩
    have : i2.val = (f : U32).val := by rw [hi2]; scalar_tac
    step as ⟨r, hr⟩
    rw [hr]; simp [bodyF, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl, this]
  · rename_i hge
    have hl : l.face_body.val.length ≤ (f : U32).val := by scalar_tac
    simp only [WP.spec_ok]
    simp [bodyF, List.getD_eq_getElem?_getD, List.getElem?_eq_none hl]

theorem getD_set_any {α : Type} (t : List α) (i j : Nat) (v d : α) (hi : i < t.length) :
    (t.set i v).getD j d = if j = i then v else t.getD j d := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_set, List.getD_eq_getElem?_getD]
  by_cases h : i = j
  · subst h; simp [hi]
  · simp [h, Ne.symm h]

def PAt (PO : List (alloc.vec.Vec Bool)) (p b : Nat) : Bool := (PO.getD p default).val.getD b false

def posAt (FP : alloc.vec.Vec U32) (c : Nat) : Nat := (FP.val.getD c NONE).val

/-- Loop 4's invariant over the first `k` pieces. -/
def L4Inv (l : Ledger) (FP : alloc.vec.Vec U32) (Fl nb : Nat) (PO : List (alloc.vec.Vec Bool)) (k : Nat) : Prop :=
  PO.length = Fl ∧ (∀ p, p < Fl → (PO.getD p default).val.length = nb) ∧
  ∀ p, p < Fl → ∀ b, b < nb →
    PAt PO p b = ((pcs l).take k).any (fun q => decide (posAt FP q.1 = p ∧ bodyF l q.2 = b))

theorem split_loop4_spec (l : Ledger) (FP : alloc.vec.Vec U32) (Fl : Nat)
    (hin : ∀ q ∈ pcs l, q.1 < FP.val.length ∧ posAt FP q.1 < Fl)
    (PO : alloc.vec.Vec (alloc.vec.Vec Bool)) (k : Usize) (hk : k.val ≤ l.pieces.val.length)
    (hinv : L4Inv l FP Fl l.nb.val PO.val k.val) :
    split_loop4 l.nb l.face_body l.face_class l.cells l.pieces l.inside l.edges l.verts FP PO k ⦃ r =>
      L4Inv l FP Fl l.nb.val r.val l.pieces.val.length ⦄ := by
  have hpl : (pcs l).length = l.pieces.val.length := by simp [pcs]
  unfold split_loop4
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec (alloc.vec.Vec Bool) × Usize) => l.pieces.val.length - q.2.val)
    (fun q => q.2.val ≤ l.pieces.val.length ∧ L4Inv l FP Fl l.nb.val q.1.val q.2.val)
  · rintro ⟨T, j⟩ ⟨hj, hl1, hl2, hl3⟩
    unfold split_loop4.body
    simp only at hj hl1 hl2 hl3 ⊢
    split
    · rename_i hlt
      have hjl : j.val < l.pieces.val.length := by scalar_tac
      step as ⟨c, f, hcf⟩
      have hpj : (pcs l)[j.val]'(by omega) = ((c : U32).val, (f : U32).val) := by simp [pcs, ← hcf]
      have hmem : ((c : U32).val, (f : U32).val) ∈ pcs l := by rw [← hpj]; exact List.getElem_mem _
      obtain ⟨hcl, hpos⟩ := hin _ hmem
      have htake : (pcs l).take (j.val + 1) = (pcs l).take j.val ++ [((c : U32).val, (f : U32).val)] := by
        rw [List.take_succ, List.getElem?_eq_getElem (by omega), hpj]; rfl
      step with body_of_spec l f as ⟨bi, hbi⟩
      step as ⟨b, hb⟩
      have hbv : b.val = bodyF l (f : U32).val := by rw [hb]; scalar_tac
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = (c : U32).val := by rw [hi3]; scalar_tac
      step as ⟨i4, hi4⟩
      have hi4v : (i4 : U32).val = posAt FP (c : U32).val := by
        simp only [hi3v] at hi4
        rw [posAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hcl, hi4]; rfl
      step as ⟨pp, hpp⟩
      have hppv : pp.val = posAt FP (c : U32).val := by rw [hpp, ← hi4v]; scalar_tac
      step as ⟨nb5, hnb5⟩
      have hnb5v : nb5.val = l.nb.val := by rw [hnb5]; scalar_tac
      have hex : ∀ p' b', ((pcs l).take (j.val + 1)).any (fun q => decide (posAt FP q.1 = p' ∧ bodyF l q.2 = b')) =
            (((pcs l).take j.val).any (fun q => decide (posAt FP q.1 = p' ∧ bodyF l q.2 = b')) ||
              decide (posAt FP (c : U32).val = p' ∧ bodyF l (f : U32).val = b')) := by
        intro p' b'
        rw [htake, List.any_append]; simp
      split
      · rename_i hbn
        have hbl : b.val < l.nb.val := by scalar_tac
        have hppl : pp.val < T.val.length := by rw [hl1, hppv]; exact hpos
        step as ⟨pr, hpr1, hpr2⟩
        obtain ⟨v7, back⟩ := pr
        simp only at hpr1 hpr2 ⊢
        have hv7l : b.val < v7.val.length := by
          have := hl2 pp.val (by omega)
          rw [hpr1]; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hppl] at this; simp at this; omega
        step as ⟨pr1, hq1, hq2⟩
        obtain ⟨w0, back1⟩ := pr1
        simp only at hq1 hq2 ⊢
        step as ⟨j1, hj1⟩
        have hnew : (back (back1 true)).val = T.val.set pp.val (v7.set b true) := by
          rw [hpr2, hq2]; simp
        refine ⟨by scalar_tac, ⟨by rw [hnew]; simp [hl1], fun p' hp' => ?_, fun p' hp' b' hb' => ?_⟩, by scalar_tac⟩
        · rw [hnew, getD_set_any _ _ _ _ _ hppl]
          split
          · rename_i e; subst e
            have := hl2 pp.val hp'
            rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hppl] at this
            simp [hpr1] at this ⊢; exact this
          · exact hl2 p' hp'
        · rw [hj1, hex p' b', ← hl3 p' hp' b' hb', PAt, PAt, hnew, getD_set_any _ _ _ _ _ hppl]
          by_cases e : p' = pp.val
          · subst e
            rw [if_pos rfl]
            have hv7' : v7 = T.val.getD pp.val default := by
              rw [hpr1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hppl]; rfl
            simp only [alloc.vec.Vec.set_val_eq]
            rw [getD_set_any _ _ _ _ _ hv7l, ← hv7', hppv, hbv]
            by_cases eb : b' = bodyF l (f : U32).val
            · subst eb; simp
            · have eb' : ¬bodyF l (f : U32).val = b' := fun h => eb h.symm
              simp [eb, eb']
          · rw [if_neg e]
            have : ¬posAt FP (c : U32).val = p' := by rw [← hppv]; exact fun h => e h.symm
            simp [this]
      · rename_i hbn
        have hbl : l.nb.val ≤ b.val := by scalar_tac
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ⟨hl1, hl2, fun p' hp' b' hb' => ?_⟩, by scalar_tac⟩
        rw [hj1, hex p' b', ← hl3 p' hp' b' hb']
        have : ¬bodyF l (f : U32).val = b' := by rw [← hbv]; omega
        simp [this]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = l.pieces.val.length := by scalar_tac
      rw [this] at hl3
      exact ⟨hl1, hl2, hl3⟩
  · exact ⟨hk, hinv⟩

end forge_bool_core
