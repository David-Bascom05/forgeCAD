import ForgeBoolCore.RefineIncUnifyAt

/-!
# Group U (inc), part 4: the edge join (`erel`)

Round each vert that is not a result vert, `inc::unify` unites two of its edges (by their numbers
in `ecs`) when both have two faces and the same two (`same_set`). The pairs are sound (`ESi`: one
pair of faces, the edges round one vert that is not a result vert) and complete: every two such
edges round such a vert are joined (`erel_spec`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- Edge `x`'s number among the edges with two faces, or `NONE`. -/
def ecpA (ep : List U32) (x : Nat) : Nat := (ep.getD x NONE).val

/-- The id at index `j`. -/
def idA (ids : List U32) (j : Nat) : Nat := (ids.getD j 0#u32).val

/-- Two edges, both with two faces, the same two. -/
def PE (A : Nat → List Nat) (ep : List U32) (x y : Nat) : Prop :=
  ecpA ep x ≠ U32.max ∧ ecpA ep y ≠ U32.max ∧ SS (A x) (A y) = true

theorem PE_symm {A : Nat → List Nat} {ep : List U32} {x y : Nat} (h : PE A ep x y) : PE A ep y x :=
  ⟨h.2.1, h.1, by rw [u_SS_symm]; exact h.2.2⟩

/-- A sound edge pair: the numbers of two edges with one pair of faces, related by `G`. -/
def ESi (A : Nat → List Nat) (ep : List U32) (G : Nat → Nat → Prop) (m : Nat) (e : Nat × Nat) : Prop :=
  e.1 < m ∧ e.2 < m ∧ ∃ x y, ecpA ep x = e.1 ∧ ecpA ep y = e.2 ∧ SS (A x) (A y) = true ∧ G x y

/-- What the erel loops read. -/
structure ErCtx (ne m : Nat) (A : Nat → List Nat) (ids : List U32) (at_e : List (alloc.vec.Vec U32))
    (ep : List U32) : Prop where
  ids : ∀ x ∈ ids, x.val < ne
  ate : at_e.map (fun v => nv v.val) = (List.range ne).map A
  epl : ep.length = ne
  epm : ∀ x, x < ne → ecpA ep x ≠ U32.max → ecpA ep x < m
  m : m ≤ U32.max

theorem ErCtx.ate_get {ne m : Nat} {A : Nat → List Nat} {ids : List U32} {at_e : List (alloc.vec.Vec U32)}
    {ep : List U32} (h : ErCtx ne m A ids at_e ep) {x : Nat} (hx : x < ne) :
    ∃ hl : x < at_e.length, nv (at_e[x]).val = A x := by
  have hl : at_e.length = ne := by have := congrArg List.length h.ate; simpa using this
  refine ⟨by omega, ?_⟩
  have h1 := congrArg (fun L => L[x]?) h.ate
  simp only [List.getElem?_map] at h1
  rw [List.getElem?_eq_getElem (by omega), List.getElem?_range hx] at h1
  simpa using h1

theorem ecp_read {ep : List U32} {x : Nat} (hx : x < ep.length) : (ep[x]).val = ecpA ep x := by
  unfold ecpA; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hx]; rfl

theorem id_read {ids : List U32} {x : Nat} (hx : x < ids.length) : (ids[x]).val = idA ids x := by
  unfold idA; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hx]; rfl

theorem none_ne {x : U32} : (x != NONE) = decide (x.val ≠ U32.max) := by
  by_cases h : x.val = U32.max
  · have : x = NONE := UScalar.eq_of_val_eq (by rw [h, inc_none_val])
    subst this; simp [inc_none_val]
  · have : x ≠ NONE := fun e => h (by rw [e, inc_none_val])
    simp [this, h]

theorem erel_inner_spec (ne m : Nat) (A : Nat → List Nat) (G : Nat → Nat → Prop)
    (v : alloc.vec.Vec U32) (ate : alloc.vec.Vec (alloc.vec.Vec U32))
    (epv : alloc.vec.Vec U32) (hC : ErCtx ne m A v.val ate.val epv.val)
    (hi : Usize) (hhi : hi.val ≤ v.val.length) (e1 : Usize) (he1 : e1.val < ne) (i0 : Nat)
    (hG : ∀ j', i0 < j' → j' < hi.val → G e1.val (idA v.val j'))
    (erel : union_find.Uf) (E : List (Nat × Nat)) (j : Usize) (hj : i0 < j.val) (hjl : j.val ≤ hi.val + 1)
    (hu : UInv E m erel) (hs : ∀ e ∈ E, ESi A epv.val G m e)
    (hc : ∀ j', i0 < j' → j' < j.val → j' < hi.val → PE A epv.val e1.val (idA v.val j') →
      Eqv E (ecpA epv.val e1.val) (ecpA epv.val (idA v.val j'))) :
    inc.unify_loop6_loop0_loop0 v ate epv erel hi e1 j ⦃ r => ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' m r ∧ (∀ e ∈ E', ESi A epv.val G m e) ∧
      ∀ j', i0 < j' → j' < hi.val → PE A epv.val e1.val (idA v.val j') →
        Eqv E' (ecpA epv.val e1.val) (ecpA epv.val (idA v.val j')) ⦄ := by
  have hepl := hC.epl
  have hml := hC.m
  unfold inc.unify_loop6_loop0_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => hi.val + 1 - st.2.val)
    (fun st => i0 < st.2.val ∧ st.2.val ≤ hi.val + 1 ∧ ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' m st.1 ∧ (∀ e ∈ E', ESi A epv.val G m e) ∧
      ∀ j', i0 < j' → j' < st.2.val → j' < hi.val → PE A epv.val e1.val (idA v.val j') →
        Eqv E' (ecpA epv.val e1.val) (ecpA epv.val (idA v.val j')))
  · rintro ⟨u0, q⟩ ⟨hq, hql, E0, hE0, hu0, hs0, hc0⟩
    unfold inc.unify_loop6_loop0_loop0.body
    simp only at hq hql hE0 hu0 hs0 hc0 ⊢
    -- the step for any branch that joins nothing: `q` is not related
    have hkeep : ∀ (E1 : List (Nat × Nat)), (∀ e ∈ E0, e ∈ E1) →
        (PE A epv.val e1.val (idA v.val q.val) → Eqv E1 (ecpA epv.val e1.val) (ecpA epv.val (idA v.val q.val))) →
        ∀ j', i0 < j' → j' < q.val + 1 → j' < hi.val → PE A epv.val e1.val (idA v.val j') →
          Eqv E1 (ecpA epv.val e1.val) (ecpA epv.val (idA v.val j')) := by
      intro E1 hE1 hq1 j' h1 h2 h3 h4
      by_cases hj' : j' < q.val
      · exact Eqv.mono hE1 (hc0 j' h1 hj' h3 h4)
      · have : j' = q.val := by omega
        subst this; exact hq1 h4
    split
    · rename_i hlt
      have hqi : q.val < v.val.length := by scalar_tac
      have he2l := hC.ids _ (List.getElem_mem hqi)
      step as ⟨x, hx⟩
      step as ⟨e2, he2⟩
      have he2v : e2.val = idA v.val q.val := by rw [he2, ← id_read hqi, hx]; scalar_tac
      have he2n : e2.val < ne := by rw [he2, hx]; scalar_tac
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = ecpA epv.val e1.val := by rw [hi1, ecp_read (by omega)]
      rw [none_ne]
      split
      · rename_i hn1
        have hn1' : ecpA epv.val e1.val ≠ U32.max := by rw [← hi1v]; simpa using hn1
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = ecpA epv.val e2.val := by rw [hi2, ecp_read (by omega)]
        rw [none_ne]
        split
        · rename_i hn2
          have hn2' : ecpA epv.val e2.val ≠ U32.max := by rw [← hi2v]; simpa using hn2
          obtain ⟨hl1, hA1⟩ := hC.ate_get he1
          obtain ⟨hl2, hA2⟩ := hC.ate_get he2n
          step as ⟨w1, hw1⟩
          step as ⟨w2, hw2⟩
          step with same_set_spec (alloc.vec.Vec.deref w1) (alloc.vec.Vec.deref w2) as ⟨b, hb⟩
          have hbv : b = SS (A e1.val) (A e2.val) := by
            rw [hb, ← hA1, ← hA2, hw1, hw2]; simp [SS, alloc.vec.Vec.deref]
          split
          · rename_i hbt
            have hss : SS (A e1.val) (A e2.val) = true := by rw [← hbv]; exact hbt
            step as ⟨i3, hi3⟩
            have hi3v : i3.val = ecpA epv.val e1.val := by rw [hi3, ← hi1v]; scalar_tac
            step as ⟨i4, hi4⟩
            have hi4v : i4.val = ecpA epv.val e2.val := by rw [hi4, ← hi2v]; scalar_tac
            have hm1 := hC.epm _ he1 hn1'
            have hm2 := hC.epm _ he2n hn2'
            step with union_spec u0 hu0 (fun e he => ⟨(hs0 e he).1, (hs0 e he).2.1⟩) hml i3 i4
              (by omega) (by omega) as ⟨u1, hu1⟩
            step as ⟨q1, hq1⟩
            have hq1v : q1.val = q.val + 1 := by scalar_tac
            refine ⟨by scalar_tac, by scalar_tac, ⟨E0 ++ [(i3.val, i4.val)],
              fun e he => List.mem_append_left _ (hE0 e he), hu1, ?_, ?_⟩, by scalar_tac⟩
            · intro e he
              rcases List.mem_append.mp he with he | he
              · exact hs0 e he
              · simp at he; rw [he]
                refine ⟨by omega, by omega, e1.val, e2.val, hi3v.symm, hi4v.symm, hss, ?_⟩
                rw [he2v]; exact hG q.val hq (by scalar_tac)
            · rw [hq1v]
              refine hkeep _ (fun e he => List.mem_append_left _ he) (fun _ => ?_)
              rw [← hi3v, ← he2v, ← hi4v]; exact Eqv.edge (e := (i3.val, i4.val)) (by simp)
          · rename_i hbt
            step as ⟨q1, hq1⟩
            have hq1v : q1.val = q.val + 1 := by scalar_tac
            refine ⟨by scalar_tac, by scalar_tac, ⟨E0, hE0, hu0, hs0, ?_⟩, by scalar_tac⟩
            rw [hq1v]
            refine hkeep _ (fun e he => he) (fun hp => absurd ?_ hbt)
            rw [hbv, he2v]; exact hp.2.2
        · rename_i hn2
          step as ⟨q1, hq1⟩
          have hq1v : q1.val = q.val + 1 := by scalar_tac
          refine ⟨by scalar_tac, by scalar_tac, ⟨E0, hE0, hu0, hs0, ?_⟩, by scalar_tac⟩
          rw [hq1v]
          refine hkeep _ (fun e he => he) (fun hp => absurd ?_ hn2)
          simp only [decide_eq_true_eq]; rw [hi2v, he2v]; exact hp.2.1
      · rename_i hn1
        step as ⟨q1, hq1⟩
        have hq1v : q1.val = q.val + 1 := by scalar_tac
        refine ⟨by scalar_tac, by scalar_tac, ⟨E0, hE0, hu0, hs0, ?_⟩, by scalar_tac⟩
        rw [hq1v]
        refine hkeep _ (fun e he => he) (fun hp => absurd ?_ hn1)
        simp only [decide_eq_true_eq]; rw [hi1v]; exact hp.1
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨E0, hE0, hu0, hs0, fun j' h1 h3 h4 => hc0 j' h1 (by scalar_tac) h3 h4⟩
  · exact ⟨hj, hjl, E, fun e he => he, hu, hs, hc⟩

theorem erel_mid_spec (ne m : Nat) (A : Nat → List Nat) (G : Nat → Nat → Prop)
    (v : alloc.vec.Vec U32) (ate : alloc.vec.Vec (alloc.vec.Vec U32))
    (epv : alloc.vec.Vec U32) (hC : ErCtx ne m A v.val ate.val epv.val)
    (lo hi : Usize) (hhi : hi.val ≤ v.val.length)
    (hG : ∀ i j, lo.val ≤ i → i < hi.val → lo.val ≤ j → j < hi.val → G (idA v.val i) (idA v.val j))
    (erel : union_find.Uf) (E : List (Nat × Nat)) (i : Usize) (hlo : lo.val ≤ i.val) (hil : i.val ≤ hi.val)
    (hu : UInv E m erel) (hs : ∀ e ∈ E, ESi A epv.val G m e)
    (hc : ∀ i' j', lo.val ≤ i' → i' < i.val → i' < j' → j' < hi.val → PE A epv.val (idA v.val i') (idA v.val j') →
      Eqv E (ecpA epv.val (idA v.val i')) (ecpA epv.val (idA v.val j'))) :
    inc.unify_loop6_loop0 v ate epv erel hi i ⦃ r => ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' m r ∧ (∀ e ∈ E', ESi A epv.val G m e) ∧
      ∀ i' j', lo.val ≤ i' → i' < j' → j' < hi.val → PE A epv.val (idA v.val i') (idA v.val j') →
        Eqv E' (ecpA epv.val (idA v.val i')) (ecpA epv.val (idA v.val j')) ⦄ := by
  unfold inc.unify_loop6_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => hi.val - st.2.val)
    (fun st => lo.val ≤ st.2.val ∧ st.2.val ≤ hi.val ∧ ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' m st.1 ∧ (∀ e ∈ E', ESi A epv.val G m e) ∧
      ∀ i' j', lo.val ≤ i' → i' < st.2.val → i' < j' → j' < hi.val → PE A epv.val (idA v.val i') (idA v.val j') →
        Eqv E' (ecpA epv.val (idA v.val i')) (ecpA epv.val (idA v.val j')))
  · rintro ⟨u0, q⟩ ⟨hq1, hq2, E0, hE0, hu0, hs0, hc0⟩
    unfold inc.unify_loop6_loop0.body
    simp only at hq1 hq2 hE0 hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hqi : q.val < v.val.length := by scalar_tac
      step as ⟨x, hx⟩
      step as ⟨e1, he1⟩
      have he1v : e1.val = idA v.val q.val := by
        rw [he1, ← id_read hqi]; simp only [hx]; scalar_tac
      have he1n : e1.val < ne := by rw [he1v, ← id_read hqi]; exact hC.ids _ (List.getElem_mem hqi)
      step as ⟨j, hj⟩
      step with erel_inner_spec ne m A G v ate epv hC hi hhi e1 he1n q.val
        (fun j' h1 h2 => by rw [he1v]; exact hG q.val j' hq1 (by scalar_tac) (by omega) h2)
        u0 E0 j (by scalar_tac) (by scalar_tac) hu0 hs0 (fun j' h1 h2 _ _ => by exfalso; scalar_tac)
        as ⟨u1, E1, hE1, hu1, hs1, hc1⟩
      step as ⟨q1, hq1'⟩
      have hq1v : q1.val = q.val + 1 := by scalar_tac
      refine ⟨by scalar_tac, by scalar_tac, ⟨E1, fun e he => hE1 e (hE0 e he), hu1, hs1, ?_⟩, by scalar_tac⟩
      intro i' j' h1 h2 h3 h4 hp
      rw [hq1v] at h2
      by_cases hiq : i' < q.val
      · exact Eqv.mono hE1 (hc0 i' j' h1 hiq h3 h4 hp)
      · have : i' = q.val := by omega
        subst this; rw [← he1v] at hp ⊢; exact hc1 j' h3 h4 hp
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨E0, hE0, hu0, hs0, fun i' j' h1 h3 h4 hp => hc0 i' j' h1 (by scalar_tac) h3 h4 hp⟩
  · exact ⟨hlo, hil, E, fun e he => he, hu, hs, hc⟩

/-- Two edges round one vert that is not a result vert. -/
def GV (l : Ledger) (io : List Bool) (x y : Nat) : Prop :=
  ∃ v, v < nrowsL l.vert_edges ∧ io.getD v false = false ∧ x ∈ rowL l.vert_edges v ∧ y ∈ rowL l.vert_edges v

theorem subL_get (c : Csr) {lo hi : Nat} (hhi : hi ≤ c.ids.val.length) {i : Nat} (h1 : lo ≤ i) (h2 : i < hi) :
    idA c.ids.val i ∈ subL c lo hi := by
  unfold subL
  rw [List.mem_map]
  refine ⟨c.ids.val[i]'(by omega), ?_, id_read (by omega)⟩
  rw [List.mem_iff_getElem]
  refine ⟨i - lo, by simp; omega, ?_⟩
  simp [show lo + (i - lo) = i by omega]

theorem subL_getD (c : Csr) {lo hi : Nat} (hhi : hi ≤ c.ids.val.length) {p : Nat} (hp : p < (subL c lo hi).length) :
    (subL c lo hi).getD p 0 = idA c.ids.val (lo + p) := by
  have hpl : p < hi - lo := by simp [subL] at hp; omega
  unfold subL
  rw [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_take, if_pos hpl, List.getElem?_drop,
    List.getElem?_eq_getElem (by omega)]
  simp [← id_read (show lo + p < c.ids.val.length by omega)]

theorem erel_spec (l : Ledger) (hw : LedgerWF2 l) (m : Nat) (A : Nat → List Nat)
    (ate : alloc.vec.Vec (alloc.vec.Vec U32)) (ep : alloc.vec.Vec U32)
    (hC : ErCtx (nrowsL l.edge_facets) m A l.vert_edges.ids.val ate.val ep.val)
    (nv' : Usize) (hnv : nv'.val = nrowsL l.vert_edges) (io : alloc.vec.Vec Bool) (hiol : io.val.length = nv'.val)
    (erel : union_find.Uf) (E : List (Nat × Nat)) (k : Usize) (hk : k.val ≤ nv'.val)
    (hu : UInv E m erel) (hs : ∀ e ∈ E, ESi A ep.val (GV l io.val) m e)
    (hc : ∀ v, v < k.val → io.val.getD v false = false → ∀ x ∈ rowL l.vert_edges v, ∀ y ∈ rowL l.vert_edges v,
      PE A ep.val x y → Eqv E (ecpA ep.val x) (ecpA ep.val y)) :
    inc.unify_loop6 l.vert_edges.start l.vert_edges.ids nv' ate io ep erel k ⦃ r => ∃ E',
      UInv E' m r ∧ (∀ e ∈ E', ESi A ep.val (GV l io.val) m e) ∧
      ∀ v, v < nrowsL l.vert_edges → io.val.getD v false = false → ∀ x ∈ rowL l.vert_edges v,
        ∀ y ∈ rowL l.vert_edges v, PE A ep.val x y → Eqv E' (ecpA ep.val x) (ecpA ep.val y) ⦄ := by
  have hc' := hw.vert_edges
  unfold inc.unify_loop6
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => nv'.val - st.2.val)
    (fun st => st.2.val ≤ nv'.val ∧ ∃ E', UInv E' m st.1 ∧ (∀ e ∈ E', ESi A ep.val (GV l io.val) m e) ∧
      ∀ v, v < st.2.val → io.val.getD v false = false → ∀ x ∈ rowL l.vert_edges v, ∀ y ∈ rowL l.vert_edges v,
        PE A ep.val x y → Eqv E' (ecpA ep.val x) (ecpA ep.val y))
  · rintro ⟨u0, q⟩ ⟨hq, E0, hu0, hs0, hc0⟩
    unfold inc.unify_loop6.body
    simp only at hq hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hqn : q.val < nrowsL l.vert_edges := by scalar_tac
      have hio : q.val < io.val.length := by omega
      step as ⟨b, hb⟩
      have hbv : io.val.getD q.val false = b := by
        rw [hb, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by scalar_tac)]; rfl
      split
      · rename_i hbt
        step as ⟨q1, hq1⟩
        have hq1v : q1.val = q.val + 1 := by scalar_tac
        refine ⟨by omega, ⟨E0, hu0, hs0, ?_⟩, by omega⟩
        intro v hv hiv
        by_cases hvq : v < q.val
        · exact hc0 v hvq hiv
        · have : v = q.val := by omega
          subst this; rw [hbv, hbt] at hiv; simp at hiv
      · rename_i hbt
        have hkq : q.val + 1 < l.vert_edges.start.val.length := by unfold nrowsL at hqn; omega
        step with row_spec { start := l.vert_edges.start, ids := l.vert_edges.ids } q hkq as ⟨lo, hi, hlo, hhi⟩
        have hlo' : lo.val = stAt l.vert_edges q.val := hlo
        have hhi' : hi.val = stAt l.vert_edges (q.val + 1) := hhi
        have hmono := csr_mono hc' (k := q.val) 1 hkq
        have hlen := csr_le_len hc' hkq
        unfold stAt at hmono hlen
        have hrow : rowL l.vert_edges q.val = subL l.vert_edges lo.val hi.val := by
          rw [rowL_eq, hlo', hhi']
        have hhl : hi.val ≤ l.vert_edges.ids.val.length := by rw [hhi']; exact hlen
        have hbf : io.val.getD q.val false = false := by rw [hbv]; simpa using hbt
        step with erel_mid_spec (nrowsL l.edge_facets) m A (GV l io.val)
          l.vert_edges.ids ate ep hC lo hi hhl
          (fun i j h1 h2 h3 h4 => ⟨q.val, hqn, hbf, by rw [hrow]; exact subL_get _ hhl h1 h2,
            by rw [hrow]; exact subL_get _ hhl h3 h4⟩)
          u0 E0 lo (le_refl _) (by rw [hlo', hhi']; exact hmono) hu0 hs0 (fun i' j' h1 h2 => by omega)
          as ⟨u1, E1, hE1, hu1, hs1, hc1⟩
        step as ⟨q1, hq1⟩
        have hq1v : q1.val = q.val + 1 := by scalar_tac
        refine ⟨by omega, ⟨E1, hu1, hs1, ?_⟩, by omega⟩
        intro v hv hiv
        by_cases hvq : v < q.val
        · intro x hx y hy hp; exact Eqv.mono hE1 (hc0 v hvq hiv x hx y hy hp)
        · have : v = q.val := by omega
          subst this
          rw [hrow]
          apply u_pairs_all (P := PE A ep.val) (Q := fun x y => Eqv E1 (ecpA ep.val x) (ecpA ep.val y))
            (fun _ _ h => PE_symm h) (fun _ _ h => Eqv.symm h) (fun _ => Eqv.refl _)
          intro p r hpr hr hp
          have hp' : p < (subL l.vert_edges lo.val hi.val).length := by omega
          rw [subL_getD _ hhl hp', subL_getD _ hhl hr] at hp ⊢
          have hrl : r < hi.val - lo.val := by simp [subL] at hr; omega
          exact hc1 _ _ (by omega) (by omega) (by omega) hp
    · rename_i hge
      simp only [WP.spec_ok]
      have : q.val = nv'.val := by scalar_tac
      refine ⟨E0, hu0, hs0, fun v hv => hc0 v (by omega)⟩
  · exact ⟨hk, E, hu, hs, hc⟩

end inc

end forge_bool_core
