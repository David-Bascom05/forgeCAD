import ForgeBoolCore.RefineIncCollapse
import ForgeBoolCore.RefineUnifyLean

/-!
# Group U (inc), part 1: the join loops

`inc::unify`'s first loops unite two result positions round one edge when they have one carrier
(`facet_class` of their facets) and one sense (`orient`). As v1's (`RefineUnifyJoin`), the pairs
are sound (`JSi`: both round one listed edge, one carrier, one sense) and complete: every such
pair round a listed edge is joined (`join_spec`). Here the rounds are computed in the loop, by
`around_e`.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- The carrier of result position `i`. -/
def cAt (fc res : List U32) (i : Nat) : Nat := (fc.getD (res.getD i 0#u32).val 0#u32).val

/-- The sense of result position `i`. -/
def oAt (ori : List Sgn) (i : Nat) : Sgn := ori.getD i .Zero

/-- One carrier and one sense. -/
def JCi (fc res : List U32) (ori : List Sgn) (i j : Nat) : Prop :=
  cAt fc res i = cAt fc res j ∧ oAt ori i = oAt ori j

/-- A sound join pair: two result positions of one carrier and sense, round one listed round. -/
def JSi (fc res : List U32) (ori : List Sgn) (AE : List (List Nat)) (e : Nat × Nat) : Prop :=
  e.1 < res.length ∧ e.2 < res.length ∧ JCi fc res ori e.1 e.2 ∧ ∃ x ∈ AE, e.1 ∈ x ∧ e.2 ∈ x

theorem JCi_symm {fc res : List U32} {ori : List Sgn} {i j : Nat} (h : JCi fc res ori i j) :
    JCi fc res ori j i := ⟨h.1.symm, h.2.symm⟩

theorem cAt_eq {fc res : List U32} {c : Nat} (hc : c < res.length) (hf : (res[c]).val < fc.length) :
    cAt fc res c = (fc[(res[c]).val]).val := by
  unfold cAt
  have e1 : res.getD c 0#u32 = res[c] := by rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hc]; rfl
  rw [e1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hf, Option.getD_some]

theorem oAt_eq {ori : List Sgn} {c : Nat} (hc : c < ori.length) : oAt ori c = ori[c] := by
  unfold oAt; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hc]; rfl

/-- The pair `(ar[p], ar[q])`, read and compared by the innermost loop. -/
theorem join_inner_spec (fc res : alloc.vec.Vec U32) (ori : alloc.vec.Vec Sgn)
    (horl : ori.val.length = res.val.length) (hres : ∀ x ∈ res.val, x.val < fc.val.length)
    (hn : res.val.length ≤ U32.max) (AE : List (List Nat))
    (ar : alloc.vec.Vec U32) (har : nv ar.val ∈ AE) (harn : ∀ x ∈ ar.val, x.val < res.val.length)
    (p : Usize) (hp : p.val < ar.val.length) (join : union_find.Uf) (E : List (Nat × Nat)) (q : Usize)
    (hq : p.val < q.val) (hql : q.val ≤ ar.val.length + 1)
    (hu : UInv E res.val.length join) (hs : ∀ e ∈ E, JSi fc.val res.val ori.val AE e)
    (hc : ∀ q', p.val < q' → q' < q.val → q' < ar.val.length →
      JCi fc.val res.val ori.val ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) →
      Eqv E ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0)) :
    inc.unify_loop0_loop0_loop0 fc res ori join ar p q ⦃ r => ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' res.val.length r ∧ (∀ e ∈ E', JSi fc.val res.val ori.val AE e) ∧
      ∀ q', p.val < q' → q' < ar.val.length →
      JCi fc.val res.val ori.val ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) →
      Eqv E' ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) ⦄ := by
  unfold inc.unify_loop0_loop0_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => ar.val.length + 1 - st.2.val)
    (fun st => p.val < st.2.val ∧ st.2.val ≤ ar.val.length + 1 ∧ ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' res.val.length st.1 ∧ (∀ e ∈ E', JSi fc.val res.val ori.val AE e) ∧
      ∀ q', p.val < q' → q' < st.2.val → q' < ar.val.length →
      JCi fc.val res.val ori.val ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) →
      Eqv E' ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0))
  · rintro ⟨u0, j⟩ ⟨hpj, hjl, E0, hE0, hu0, hs0, hc0⟩
    unfold inc.unify_loop0_loop0_loop0.body
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
      have hcn : c.val < res.val.length := by rw [hcv, hx]; exact hpa
      have hdn : d.val < res.val.length := by rw [hdv, hy]; exact hja
      have hpv : (nv ar.val).getD p.val 0 = c.val := by rw [u_getD_nv hp, hcv, hx]
      have hjv : (nv ar.val).getD j.val 0 = d.val := by rw [u_getD_nv hjl', hdv, hy]
      have hfc := hres _ (List.getElem_mem hcn)
      have hfd := hres _ (List.getElem_mem hdn)
      step as ⟨i3, hi3⟩
      step as ⟨i4, hi4⟩
      have hi4v : i4.val = (res.val[c.val]).val := by rw [hi4, hi3]; scalar_tac
      step as ⟨i5, hi5⟩
      step as ⟨i6, hi6⟩
      step as ⟨i7, hi7⟩
      have hi7v : i7.val = (res.val[d.val]).val := by rw [hi7, hi6]; scalar_tac
      step as ⟨i8, hi8⟩
      have hz5 : i5.val = cAt fc.val res.val c.val := by
        rw [cAt_eq hcn hfc, hi5]; simp only [← hi4v]
      have hz8 : i8.val = cAt fc.val res.val d.val := by
        rw [cAt_eq hdn hfd, hi8]; simp only [← hi7v]
      have hkeep : ∀ (E1 : List (Nat × Nat)), (∀ e ∈ E0, e ∈ E1) →
          (JCi fc.val res.val ori.val c.val d.val → Eqv E1 c.val d.val) →
          ∀ q', p.val < q' → q' < j.val + 1 → q' < ar.val.length →
            JCi fc.val res.val ori.val ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) →
            Eqv E1 ((nv ar.val).getD p.val 0) ((nv ar.val).getD q' 0) := by
        intro E1 hE1 hcd q' h1 h2 h3 hj
        by_cases hq' : q' < j.val
        · exact Eqv.mono hE1 (hc0 q' h1 hq' h3 hj)
        · have : q' = j.val := by omega
          subst this; rw [hpv, hjv] at hj ⊢; exact hcd hj
      split
      · rename_i heq
        have hzeq : cAt fc.val res.val c.val = cAt fc.val res.val d.val := by rw [← hz5, ← hz8, heq]
        step as ⟨s2, hs2⟩
        step as ⟨s3, hs3⟩
        rw [sgn_eq]; simp only [Std.bind_ok]
        have ho2 : s2 = oAt ori.val c.val := by rw [oAt_eq (by omega), hs2]
        have ho3 : s3 = oAt ori.val d.val := by rw [oAt_eq (by omega), hs3]
        split
        · rename_i hso
          have hso' : s2 = s3 := by simpa using hso
          step with union_spec u0 hu0 (fun e he => ⟨(hs0 e he).1, (hs0 e he).2.1⟩) hn c d hcn hdn
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
        apply UScalar.eq_of_val_eq; rw [hz5, hz8]; exact hj.1
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨E0, hE0, hu0, hs0, fun q' h1 h3 hj => hc0 q' h1 (by scalar_tac) h3 hj⟩
  · exact ⟨hq, hql, E, fun e he => he, hu, hs, hc⟩

theorem join_mid_spec (fc res : alloc.vec.Vec U32) (ori : alloc.vec.Vec Sgn)
    (horl : ori.val.length = res.val.length) (hres : ∀ x ∈ res.val, x.val < fc.val.length)
    (hn : res.val.length ≤ U32.max) (AE : List (List Nat))
    (ar : alloc.vec.Vec U32) (har : nv ar.val ∈ AE) (harn : ∀ x ∈ ar.val, x.val < res.val.length)
    (join : union_find.Uf) (E : List (Nat × Nat)) (p : Usize) (hp : p.val ≤ ar.val.length)
    (hu : UInv E res.val.length join) (hs : ∀ e ∈ E, JSi fc.val res.val ori.val AE e)
    (hc : ∀ p' q', p' < p.val → p' < q' → q' < ar.val.length →
      JCi fc.val res.val ori.val ((nv ar.val).getD p' 0) ((nv ar.val).getD q' 0) →
      Eqv E ((nv ar.val).getD p' 0) ((nv ar.val).getD q' 0)) :
    inc.unify_loop0_loop0 fc res ori join ar p ⦃ r => ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' res.val.length r ∧ (∀ e ∈ E', JSi fc.val res.val ori.val AE e) ∧
      ∀ i ∈ nv ar.val, ∀ j ∈ nv ar.val, JCi fc.val res.val ori.val i j → Eqv E' i j ⦄ := by
  unfold inc.unify_loop0_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => ar.val.length - st.2.val)
    (fun st => st.2.val ≤ ar.val.length ∧ ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' res.val.length st.1 ∧ (∀ e ∈ E', JSi fc.val res.val ori.val AE e) ∧
      ∀ p' q', p' < st.2.val → p' < q' → q' < ar.val.length →
      JCi fc.val res.val ori.val ((nv ar.val).getD p' 0) ((nv ar.val).getD q' 0) →
      Eqv E' ((nv ar.val).getD p' 0) ((nv ar.val).getD q' 0))
  · rintro ⟨u0, j⟩ ⟨hj, E0, hE0, hu0, hs0, hc0⟩
    unfold inc.unify_loop0_loop0.body
    simp only at hj hE0 hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < ar.val.length := by scalar_tac
      step as ⟨q, hq⟩
      step with join_inner_spec fc res ori horl hres hn AE ar har harn j hjl u0 E0 q (by scalar_tac) (by scalar_tac)
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
      apply u_pairs_all (P := JCi fc.val res.val ori.val) (Q := Eqv E0) (fun _ _ h => JCi_symm h)
        (fun _ _ h => Eqv.symm h) Eqv.refl
      intro p' q' h1 h2 hjc
      exact hc0 p' q' (by simp [nv] at h2; scalar_tac) h1 (by simpa [nv] using h2) hjc
  · exact ⟨hp, E, fun e he => he, hu, hs, hc⟩

/-- The rounds of the edges, as result positions. -/
def AEs (l : Ledger) (a : Assembled) : List (List Nat) :=
  (List.range (nrowsL l.edge_facets)).map fun e => aroundL a (rowL l.edge_facets e)

/-- **The join loops**: sound pairs, and every pair of one carrier and sense round an edge joined. -/
theorem join_spec (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) (hrl : a.rpos.val.length = nfacets l)
    (horl : a.orient.val.length = a.result.val.length)
    (hres : ∀ x ∈ a.result.val, x.val < l.facet_class.val.length)
    (hn : a.result.val.length ≤ U32.max)
    (hrn : ∀ e, e < nrowsL l.edge_facets → ∀ x ∈ aroundL a (rowL l.edge_facets e), x < a.result.val.length)
    (ne : Usize) (hne : ne.val = nrowsL l.edge_facets)
    (join : union_find.Uf) (E : List (Nat × Nat)) (k : Usize) (hk : k.val ≤ ne.val)
    (hu : UInv E a.result.val.length join) (hs : ∀ e ∈ E, JSi l.facet_class.val a.result.val a.orient.val (AEs l a) e)
    (hc : ∀ k', k' < k.val → ∀ i ∈ aroundL a (rowL l.edge_facets k'), ∀ j ∈ aroundL a (rowL l.edge_facets k'),
      JCi l.facet_class.val a.result.val a.orient.val i j → Eqv E i j) :
    inc.unify_loop0 l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids l.vert_sectors
      a.result a.rpos a.orient ne join k ⦃ r => ∃ E',
      UInv E' a.result.val.length r ∧ (∀ e ∈ E', JSi l.facet_class.val a.result.val a.orient.val (AEs l a) e) ∧
      ∀ k', k' < nrowsL l.edge_facets → ∀ i ∈ aroundL a (rowL l.edge_facets k'),
        ∀ j ∈ aroundL a (rowL l.edge_facets k'),
        JCi l.facet_class.val a.result.val a.orient.val i j → Eqv E' i j ⦄ := by
  unfold inc.unify_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => ne.val - st.2.val)
    (fun st => st.2.val ≤ ne.val ∧ ∃ E',
      UInv E' a.result.val.length st.1 ∧ (∀ e ∈ E', JSi l.facet_class.val a.result.val a.orient.val (AEs l a) e) ∧
      ∀ k', k' < st.2.val → ∀ i ∈ aroundL a (rowL l.edge_facets k'), ∀ j ∈ aroundL a (rowL l.edge_facets k'),
        JCi l.facet_class.val a.result.val a.orient.val i j → Eqv E' i j)
  · rintro ⟨u0, j⟩ ⟨hj, E0, hu0, hs0, hc0⟩
    unfold inc.unify_loop0.body
    simp only at hj hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < nrowsL l.edge_facets := by scalar_tac
      step with around_e_spec l hw a hrl j hjl as ⟨ar, har⟩
      have harA : nv ar.val ∈ AEs l a := by
        rw [show nv ar.val = aroundL a (rowL l.edge_facets j.val) from har]
        exact List.mem_map.mpr ⟨j.val, List.mem_range.mpr hjl, rfl⟩
      have harn : ∀ x ∈ ar.val, x.val < a.result.val.length := by
        intro x hx
        exact hrn j.val hjl _ (by rw [← har]; exact List.mem_map_of_mem hx)
      step with join_mid_spec l.facet_class a.result a.orient horl hres hn (AEs l a) ar harA harn u0 E0 0#usize
        (by simp) hu0 hs0 (fun p' _ h1 => by simp at h1) as ⟨u1, E1, hE1, hu1, hs1, hc1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ⟨E1, hu1, hs1, ?_⟩, by scalar_tac⟩
      intro k' hk' i hi j' hj' hjc
      rw [show j1.val = j.val + 1 by scalar_tac] at hk'
      by_cases hkj : k' < j.val
      · exact Eqv.mono hE1 (hc0 k' hkj i hi j' hj' hjc)
      · have : k' = j.val := by omega
        subst this
        rw [← show nv ar.val = aroundL a (rowL l.edge_facets j.val) from har] at hi hj'
        exact hc1 i hi j' hj' hjc
    · rename_i hge
      simp only [WP.spec_ok]
      have hjv : j.val = ne.val := by scalar_tac
      refine ⟨E0, hu0, hs0, ?_⟩
      intro k' hk'
      exact hc0 k' (by omega)
  · exact ⟨hk, E, hu, hs, hc⟩

end inc

end forge_bool_core
