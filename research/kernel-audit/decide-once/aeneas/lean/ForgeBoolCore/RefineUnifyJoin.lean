import ForgeBoolCore.RefineUnifyClasses

/-!
# Group U, part 3: the join loops (faces)

`unify`'s first loops unite two result positions round one edge cell when they have one carrier
(`zero_idx`) and one sense (`orient`). Their pairs are sound (`JS`: both round one listed edge,
one carrier, one sense) and complete: every such pair round a listed edge is joined
(`u_join_spec`).
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

/-- The carrier of result position `i`. -/
def zR (l : Ledger) (ext : Ext) (a : Assembled) (i : Nat) : Nat := (covAt l ext (resAt a i)).idxOf .Zero

/-- The sense of result position `i`. -/
def oR (a : Assembled) (i : Nat) : Sgn := a.orient.val.getD i .Zero

/-- One carrier and one sense. -/
def JC (l : Ledger) (ext : Ext) (a : Assembled) (i j : Nat) : Prop := zR l ext a i = zR l ext a j ∧ oR a i = oR a j

/-- A sound join pair: two result positions of one carrier and sense, round one listed `around`. -/
def JS (l : Ledger) (ext : Ext) (a : Assembled) (AE : List (List Nat)) (e : Nat × Nat) : Prop :=
  e.1 < a.result.val.length ∧ e.2 < a.result.val.length ∧ JC l ext a e.1 e.2 ∧ ∃ x ∈ AE, e.1 ∈ x ∧ e.2 ∈ x

theorem u_resAt_eq (a : Assembled) {c : Nat} (hc : c < a.result.val.length) :
    resAt a c = (a.result.val[c]).val := by
  unfold resAt; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hc)]; simp

theorem u_oR_eq (a : Assembled) {c : Nat} (hc : c < a.orient.val.length) : oR a c = a.orient.val[c] := by
  unfold oR; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hc]; rfl

theorem u_getD_nv {ar : List U32} {p : Nat} (hp : p < ar.length) : (nv ar).getD p 0 = (ar[p]).val := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa [nv] using hp)]; simp [nv]

theorem u_getD_mem {ar : List U32} {p : Nat} (hp : p < ar.length) : (nv ar).getD p 0 ∈ nv ar := by
  rw [u_getD_nv hp]; exact List.mem_map_of_mem (List.getElem_mem _)

/-- The pair `(ar[p], ar[q])` as positions read and compared by the loop body. -/
theorem u_join_inner_spec (l : Ledger) (ext : Ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (horl : a.orient.val.length = a.result.val.length) (AE : List (List Nat))
    (ar : alloc.vec.Vec U32) (har : nv ar.val ∈ AE) (harn : ∀ x ∈ ar.val, x.val < a.result.val.length)
    (p : Usize) (hp : p.val < ar.val.length) (join : union_find.Uf) (E : List (Nat × Nat)) (q : Usize)
    (hq : p.val < q.val) (hql : q.val ≤ ar.val.length + 1)
    (hu : UInv E a.result.val.length join) (hs : ∀ e ∈ E, JS l ext a AE e)
    (hc : ∀ q', p.val < q' → q' < q.val → q' < ar.val.length →
      JC l ext a ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) →
      Eqv E ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0)) :
    unify_loop0_loop0_loop0 l ext a.result a.orient join ar p q ⦃ r => ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' a.result.val.length r ∧ (∀ e ∈ E', JS l ext a AE e) ∧
      ∀ q', p.val < q' → q' < ar.val.length →
      JC l ext a ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) →
      Eqv E' ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) ⦄ := by
  have hsmall := h.small
  unfold unify_loop0_loop0_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => ar.val.length + 1 - st.2.val)
    (fun st => p.val < st.2.val ∧ st.2.val ≤ ar.val.length + 1 ∧ ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' a.result.val.length st.1 ∧ (∀ e ∈ E', JS l ext a AE e) ∧
      ∀ q', p.val < q' → q' < st.2.val → q' < ar.val.length →
      JC l ext a ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) →
      Eqv E' ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0))
  · rintro ⟨u0, j⟩ ⟨hpj, hjl, E0, hE0, hu0, hs0, hc0⟩
    unfold unify_loop0_loop0_loop0.body
    simp only at hpj hjl hE0 hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hjl' : j.val < ar.val.length := by scalar_tac
      have hpa := harn _ (List.getElem_mem hp)
      have hja := harn _ (List.getElem_mem hjl')
      step as ⟨x, hx⟩
      step as ⟨c, hcx⟩
      have hcv : c.val = x.val := by rw [hcx]; scalar_tac
      step as ⟨y, hy⟩
      step as ⟨d, hdy⟩
      have hdv : d.val = y.val := by rw [hdy]; scalar_tac
      have hcn : c.val < a.result.val.length := by rw [hcv, hx]; exact hpa
      have hdn : d.val < a.result.val.length := by rw [hdv, hy]; exact hja
      have hpv : (nv ar.val).getD p.val 0 = c.val := by rw [u_getD_nv hp, hcv, hx]
      have hjv : (nv ar.val).getD j.val 0 = d.val := by rw [u_getD_nv hjl', hdv, hy]
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = resAt a c.val := by rw [u_resAt_eq a hcn, hi3]
      have hi3l : i3.val < (allCells l ext).length := by rw [hi3v]; exact resAt_lt h hcn
      step with cov_spec l ext i3 hi3l as ⟨s, hs'⟩
      step with zero_idx_spec s as ⟨i4, hi4⟩
      step as ⟨i5, hi5⟩
      have hi5v : i5.val = resAt a d.val := by rw [u_resAt_eq a hdn, hi5]
      have hi5l : i5.val < (allCells l ext).length := by rw [hi5v]; exact resAt_lt h hdn
      step with cov_spec l ext i5 hi5l as ⟨s1, hs1⟩
      step with zero_idx_spec s1 as ⟨i6, hi6⟩
      have hz4 : i4.val = zR l ext a c.val := by
        rw [hi4, zR, ← hi3v, ← hs', covOf, ForgeExact.DecideOnce.Cov.zeroIdx, idxOf_map_toS]
      have hz6 : i6.val = zR l ext a d.val := by
        rw [hi6, zR, ← hi5v, ← hs1, covOf, ForgeExact.DecideOnce.Cov.zeroIdx, idxOf_map_toS]
      have hjS : j.val + 1 ≤ ar.val.length + 1 := by omega
      -- the step for every branch: q' = j is either joined or not related
      have hkeep : ∀ (E1 : List (Nat × Nat)), (∀ e ∈ E0, e ∈ E1) →
          (JC l ext a c.val d.val → Eqv E1 c.val d.val) →
          ∀ q', p.val < q' → q' < j.val + 1 → q' < ar.val.length →
            JC l ext a ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) →
            Eqv E1 ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) := by
        intro E1 hE1 hcd q' h1 h2 h3 hj
        by_cases hq' : q' < j.val
        · exact Eqv.mono hE1 (hc0 q' h1 hq' h3 hj)
        · have : q' = j.val := by omega
          subst this; rw [hpv, hjv] at hj ⊢; exact hcd hj
      split
      · rename_i heq
        have hzeq : zR l ext a c.val = zR l ext a d.val := by rw [← hz4, ← hz6, heq]
        step as ⟨s2, hs2⟩
        step as ⟨s3, hs3⟩
        rw [sgn_eq]; simp only [Std.bind_ok]
        have ho2 : s2 = oR a c.val := by rw [u_oR_eq a (by omega), hs2]
        have ho3 : s3 = oR a d.val := by rw [u_oR_eq a (by omega), hs3]
        split
        · rename_i hso
          have hso' : s2 = s3 := by simpa using hso
          have hl := uinv_len hu0
          step with union_spec u0 hu0 (fun e he => ⟨(hs0 e he).1, (hs0 e he).2.1⟩) (by omega) c d hcn hdn
            as ⟨u1, hu1⟩
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, by scalar_tac, ⟨E0 ++ [(c.val, d.val)],
            fun e he => List.mem_append_left _ (hE0 e he), hu1, ?_, ?_⟩, by scalar_tac⟩
          · intro e he
            rcases List.mem_append.mp he with he | he
            · exact hs0 e he
            · simp at he; rw [he]
              refine ⟨hcn, hdn, ⟨hzeq, by rw [← ho2, ← ho3, hso']⟩, nv ar.val, har, ?_, ?_⟩
              · rw [← hpv]; exact u_getD_mem hp
              · rw [← hjv]; exact u_getD_mem hjl'
          · rw [show j1.val = j.val + 1 by scalar_tac]
            exact hkeep _ (fun e he => List.mem_append_left _ he)
              (fun _ => Eqv.edge (e := (c.val, d.val)) (by simp))
        · rename_i hso
          have hso' : ¬ s2 = s3 := by simpa using hso
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, by scalar_tac, ⟨E0, hE0, hu0, hs0, ?_⟩, by scalar_tac⟩
          rw [show j1.val = j.val + 1 by scalar_tac]
          exact hkeep _ (fun e he => he) (fun hj => absurd (by rw [ho2, ho3]; exact hj.2) hso')
      · rename_i hne
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, by scalar_tac, ⟨E0, hE0, hu0, hs0, ?_⟩, by scalar_tac⟩
        rw [show j1.val = j.val + 1 by scalar_tac]
        refine hkeep _ (fun e he => he) (fun hj => absurd ?_ hne)
        apply UScalar.eq_of_val_eq; rw [hz4, hz6]; exact hj.1
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨E0, hE0, hu0, hs0, fun q' h1 h3 hj => hc0 q' h1 (by scalar_tac) h3 hj⟩
  · exact ⟨hq, hql, E, fun e he => he, hu, hs, hc⟩

/-- Pairs at increasing indices cover every pair of members, for symmetric relations. -/
theorem u_pairs_all {L : List Nat} {P Q : Nat → Nat → Prop} (hP : ∀ x y, P x y → P y x)
    (hQ : ∀ x y, Q x y → Q y x) (hQr : ∀ x, Q x x)
    (h : ∀ p q, p < q → q < L.length → P (L.getD p 0) (L.getD q 0) → Q (L.getD p 0) (L.getD q 0)) :
    ∀ i ∈ L, ∀ j ∈ L, P i j → Q i j := by
  intro i hi j hj hij
  obtain ⟨p, hp, rfl⟩ := List.getElem_of_mem hi
  obtain ⟨q, hq, rfl⟩ := List.getElem_of_mem hj
  have ep : L.getD p 0 = L[p] := by rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hp]; rfl
  have eq : L.getD q 0 = L[q] := by rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hq]; rfl
  rcases Nat.lt_trichotomy p q with hpq | hpq | hpq
  · have := h p q hpq hq; rw [ep, eq] at this; exact this hij
  · subst hpq; exact hQr _
  · have := h q p hpq hp; rw [ep, eq] at this; exact hQ _ _ (this (hP _ _ hij))

theorem u_JC_symm {l : Ledger} {ext : Ext} {a : Assembled} {i j : Nat} (h : JC l ext a i j) : JC l ext a j i :=
  ⟨h.1.symm, h.2.symm⟩

theorem u_join_mid_spec (l : Ledger) (ext : Ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (horl : a.orient.val.length = a.result.val.length) (AE : List (List Nat))
    (ar : alloc.vec.Vec U32) (har : nv ar.val ∈ AE) (harn : ∀ x ∈ ar.val, x.val < a.result.val.length)
    (join : union_find.Uf) (E : List (Nat × Nat)) (p : Usize) (hp : p.val ≤ ar.val.length)
    (hu : UInv E a.result.val.length join) (hs : ∀ e ∈ E, JS l ext a AE e)
    (hc : ∀ p' q', p' < p.val → p' < q' → q' < ar.val.length →
      JC l ext a ((nv ar.val).getD p' 0) ((nv ar.val).getD q' 0) →
      Eqv E ((nv ar.val).getD p' 0) ((nv ar.val).getD q' 0)) :
    unify_loop0_loop0 l ext a.result a.orient join ar p ⦃ r => ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' a.result.val.length r ∧ (∀ e ∈ E', JS l ext a AE e) ∧
      ∀ i ∈ nv ar.val, ∀ j ∈ nv ar.val, JC l ext a i j → Eqv E' i j ⦄ := by
  have hsmall := h.small
  unfold unify_loop0_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => ar.val.length - st.2.val)
    (fun st => st.2.val ≤ ar.val.length ∧ ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' a.result.val.length st.1 ∧ (∀ e ∈ E', JS l ext a AE e) ∧
      ∀ p' q', p' < st.2.val → p' < q' → q' < ar.val.length →
      JC l ext a ((nv ar.val).getD p' 0) ((nv ar.val).getD q' 0) →
      Eqv E' ((nv ar.val).getD p' 0) ((nv ar.val).getD q' 0))
  · rintro ⟨u0, j⟩ ⟨hj, E0, hE0, hu0, hs0, hc0⟩
    unfold unify_loop0_loop0.body
    simp only at hj hE0 hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < ar.val.length := by scalar_tac
      step as ⟨q, hq⟩
      step with u_join_inner_spec l ext a W h horl AE ar har harn j hjl u0 E0 q (by scalar_tac) (by scalar_tac)
        hu0 hs0 (fun q' h1 h2 _ _ => by exfalso; scalar_tac) as ⟨u1, E1, hE1, hu1, hs1, hc1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ⟨E1, fun e he => hE1 e (hE0 e he), hu1, hs1, ?_⟩, by scalar_tac⟩
      intro p' q' h1 h2 h3 hjc
      rw [show j1.val = j.val + 1 by scalar_tac] at h1
      by_cases hpj : p' < j.val
      · exact Eqv.mono hE1 (hc0 p' q' hpj h2 h3 hjc)
      · have : p' = j.val := by omega
        subst this; exact hc1 q' h2 h3 hjc
    · rename_i hge
      simp only [WP.spec_ok]
      refine ⟨E0, hE0, hu0, hs0, ?_⟩
      apply u_pairs_all (P := JC l ext a) (Q := Eqv E0) (fun _ _ h => u_JC_symm h) (fun _ _ h => Eqv.symm h) Eqv.refl
      intro p' q' h1 h2 hjc
      exact hc0 p' q' (by simp [nv] at h2; scalar_tac) h1 (by simpa [nv] using h2) hjc
  · exact ⟨hp, E, fun e he => he, hu, hs, hc⟩

/-- **The join loops**: sound pairs, and every pair of one carrier and sense round a listed edge
joined. -/
theorem u_join_spec (l : Ledger) (ext : Ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (horl : a.orient.val.length = a.result.val.length) (ae : Slice (alloc.vec.Vec U32))
    (haen : ∀ x ∈ ae.val, ∀ y ∈ x.val, y.val < a.result.val.length)
    (join : union_find.Uf) (E : List (Nat × Nat)) (k : Usize) (hk : k.val ≤ ae.val.length)
    (hu : UInv E a.result.val.length join) (hs : ∀ e ∈ E, JS l ext a (ae.val.map fun v => nv v.val) e)
    (hc : ∀ k', k' < k.val → ∀ x, ae.val[k']? = some x → ∀ i ∈ nv x.val, ∀ j ∈ nv x.val,
      JC l ext a i j → Eqv E i j) :
    unify_loop0 l ext a.result a.orient ae join k ⦃ r => ∃ E',
      UInv E' a.result.val.length r ∧ (∀ e ∈ E', JS l ext a (ae.val.map fun v => nv v.val) e) ∧
      ∀ x ∈ ae.val, ∀ i ∈ nv x.val, ∀ j ∈ nv x.val, JC l ext a i j → Eqv E' i j ⦄ := by
  unfold unify_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => ae.val.length - st.2.val)
    (fun st => st.2.val ≤ ae.val.length ∧ ∃ E',
      UInv E' a.result.val.length st.1 ∧ (∀ e ∈ E', JS l ext a (ae.val.map fun v => nv v.val) e) ∧
      ∀ k', k' < st.2.val → ∀ x, ae.val[k']? = some x → ∀ i ∈ nv x.val, ∀ j ∈ nv x.val,
        JC l ext a i j → Eqv E' i j)
  · rintro ⟨u0, j⟩ ⟨hj, E0, hu0, hs0, hc0⟩
    unfold unify_loop0.body
    simp only at hj hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < ae.val.length := by scalar_tac
      step as ⟨ar, har⟩
      have harm : ar ∈ ae.val := by rw [har]; exact List.getElem_mem _
      step with u_join_mid_spec l ext a W h horl (ae.val.map fun v => nv v.val) ar
        (List.mem_map_of_mem harm) (fun y hy => haen ar harm y hy) u0 E0 0#usize (by simp) hu0 hs0
        (fun p' _ h1 => by simp at h1) as ⟨u1, E1, hE1, hu1, hs1, hc1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ⟨E1, hu1, hs1, ?_⟩, by scalar_tac⟩
      intro k' hk' x hx i hi j' hj' hjc
      rw [show j1.val = j.val + 1 by scalar_tac] at hk'
      by_cases hkj : k' < j.val
      · exact Eqv.mono hE1 (hc0 k' hkj x hx i hi j' hj' hjc)
      · have : k' = j.val := by omega
        subst this
        rw [List.getElem?_eq_getElem hjl] at hx
        have : x = ar := by rw [har]; exact (Option.some.inj hx).symm
        subst this; exact hc1 i hi j' hj' hjc
    · rename_i hge
      simp only [WP.spec_ok]
      refine ⟨E0, hu0, hs0, fun x hx => ?_⟩
      obtain ⟨k', hk', rfl⟩ := List.getElem_of_mem hx
      exact hc0 k' (by scalar_tac) _ (List.getElem?_eq_getElem hk')
  · exact ⟨hk, E, hu, hs, hc⟩

end forge_bool_core
