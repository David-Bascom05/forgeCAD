import ForgeBoolCore.RefineUnifyAt

/-!
# Group U, part 6: the edge join (`erel`)

Round each vertex cell that is not a vertex, `unify` collects the edge cells between it and its
facets (`round`, by edge-cell number) and unites two of them when they join one pair of faces
(`same_set`). The pairs are sound (`ES`: one pair of faces, a common non-vertex below both) and
complete (`u_erel_spec`).
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

/-- `sameSet` on numbers. -/
def SS (A B : List Nat) : Bool := A.all (fun x => decide (x ∈ B)) && B.all (fun x => decide (x ∈ A))

theorem u_SS_symm (A B : List Nat) : SS A B = SS B A := by unfold SS; rw [Bool.and_comm]

/-- The faces round edge cell number `x`. -/
def FE (ate : List (alloc.vec.Vec U32)) (ecs : List Usize) (x : Nat) : List Nat :=
  nv (ate.getD ((nvU ecs).getD x 0) default).val

/-- The `same_set` test on edge cells `x`, `y`. -/
def EC (ate : List (alloc.vec.Vec U32)) (ecs : List Usize) (x y : Nat) : Prop :=
  SS (FE ate ecs x) (FE ate ecs y) = true

theorem u_EC_symm {ate : List (alloc.vec.Vec U32)} {ecs : List Usize} {x y : Nat}
    (h : EC ate ecs x y) : EC ate ecs y x := by unfold EC at *; rw [u_SS_symm]; exact h

/-- A sound edge pair: both edge cells, one pair of faces, and related by `Wr`. -/
def ES (ate : List (alloc.vec.Vec U32)) (ecs : List Usize) (Wr : Nat → Nat → Prop) (e : Nat × Nat) : Prop :=
  e.1 < ecs.length ∧ e.2 < ecs.length ∧ EC ate ecs e.1 e.2 ∧ Wr e.1 e.2

theorem u_ec_read (ate : alloc.vec.Vec (alloc.vec.Vec U32)) (ecs : alloc.vec.Vec Usize)
    (hecs : ∀ x ∈ ecs.val, x.val < ate.val.length) {x : Usize} (hx : x.val < ecs.val.length) :
    (ecs.val[x.val]).val < ate.val.length ∧
    ∀ (hl : (ecs.val[x.val]).val < ate.val.length),
      nv (ate.val[(ecs.val[x.val]).val]'hl).val = FE ate.val ecs.val x.val := by
  refine ⟨hecs _ (List.getElem_mem _), fun hl => ?_⟩
  unfold FE
  have e1 : (nvU ecs.val).getD x.val 0 = (ecs.val[x.val]).val := by
    simp [nvU, List.getD_eq_getElem?_getD, hx]
  rw [e1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl]; rfl

theorem u_erel_inner_spec (ate : alloc.vec.Vec (alloc.vec.Vec U32)) (ecs : alloc.vec.Vec Usize)
    (hecs : ∀ x ∈ ecs.val, x.val < ate.val.length) (hM : ecs.val.length ≤ U32.max)
    (Wr : Nat → Nat → Prop) (rd : alloc.vec.Vec U32) (hrd : ∀ x ∈ rd.val, x.val < ecs.val.length)
    (hW : ∀ x ∈ nv rd.val, ∀ y ∈ nv rd.val, Wr x y)
    (p : Usize) (hp : p.val < rd.val.length) (uf : union_find.Uf) (E : List (Nat × Nat)) (q : Usize)
    (hq : p.val < q.val) (hql : q.val ≤ rd.val.length + 1)
    (hu : UInv E ecs.val.length uf) (hs : ∀ e ∈ E, ES ate.val ecs.val Wr e)
    (hc : ∀ q', p.val < q' → q' < q.val → q' < rd.val.length →
      EC ate.val ecs.val ((nv rd.val).getD p.val 0) ((nv rd.val).getD q' 0) →
      Eqv E ((nv rd.val).getD p.val 0) ((nv rd.val).getD q' 0)) :
    unify_loop6_loop1_loop0 ate ecs uf rd p q ⦃ r => ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' ecs.val.length r ∧ (∀ e ∈ E', ES ate.val ecs.val Wr e) ∧
      ∀ q', p.val < q' → q' < rd.val.length →
      EC ate.val ecs.val ((nv rd.val).getD p.val 0) ((nv rd.val).getD q' 0) →
      Eqv E' ((nv rd.val).getD p.val 0) ((nv rd.val).getD q' 0) ⦄ := by
  unfold unify_loop6_loop1_loop0
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => rd.val.length + 1 - st.2.val)
    (fun st => p.val < st.2.val ∧ st.2.val ≤ rd.val.length + 1 ∧ ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' ecs.val.length st.1 ∧ (∀ e ∈ E', ES ate.val ecs.val Wr e) ∧
      ∀ q', p.val < q' → q' < st.2.val → q' < rd.val.length →
      EC ate.val ecs.val ((nv rd.val).getD p.val 0) ((nv rd.val).getD q' 0) →
      Eqv E' ((nv rd.val).getD p.val 0) ((nv rd.val).getD q' 0))
  · rintro ⟨u0, j⟩ ⟨hpj, hjl, E0, hE0, hu0, hs0, hc0⟩
    unfold unify_loop6_loop1_loop0.body
    simp only at hpj hjl hE0 hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hjl' : j.val < rd.val.length := by scalar_tac
      have hpa := hrd _ (List.getElem_mem hp)
      have hja := hrd _ (List.getElem_mem hjl')
      step as ⟨x, hx⟩
      step as ⟨e1, he1⟩
      have he1v : e1.val = x.val := by rw [he1]; scalar_tac
      step as ⟨y, hy⟩
      step as ⟨e2, he2⟩
      have he2v : e2.val = y.val := by rw [he2]; scalar_tac
      have h1n : e1.val < ecs.val.length := by rw [he1v, hx]; exact hpa
      have h2n : e2.val < ecs.val.length := by rw [he2v, hy]; exact hja
      have hpv : (nv rd.val).getD p.val 0 = e1.val := by rw [u_getD_nv hp, he1v, hx]
      have hjv : (nv rd.val).getD j.val 0 = e2.val := by rw [u_getD_nv hjl', he2v, hy]
      obtain ⟨hr1, hr1'⟩ := u_ec_read ate ecs hecs h1n
      obtain ⟨hr2, hr2'⟩ := u_ec_read ate ecs hecs h2n
      step as ⟨i3, hi3⟩
      step as ⟨v, hv⟩
      step as ⟨i4, hi4⟩
      step as ⟨v1, hv1⟩
      have hvf : nv v.val = FE ate.val ecs.val e1.val := by rw [← hr1' (by rw [← hi3]; exact hi3 ▸ hr1)]; simp [hv, hi3]
      have hv1f : nv v1.val = FE ate.val ecs.val e2.val := by rw [← hr2' (by rw [← hi4]; exact hi4 ▸ hr2)]; simp [hv1, hi4]
      step with same_set_spec (alloc.vec.Vec.deref v) (alloc.vec.Vec.deref v1) as ⟨b, hb⟩
      have hbE : b = true ↔ EC ate.val ecs.val e1.val e2.val := by
        rw [hb]; unfold EC SS; rw [← hvf, ← hv1f]; simp [alloc.vec.Vec.deref]
      have hkeep : ∀ (E1 : List (Nat × Nat)), (∀ e ∈ E0, e ∈ E1) →
          (EC ate.val ecs.val e1.val e2.val → Eqv E1 e1.val e2.val) →
          ∀ q', p.val < q' → q' < j.val + 1 → q' < rd.val.length →
            EC ate.val ecs.val ((nv rd.val).getD p.val 0) ((nv rd.val).getD q' 0) →
            Eqv E1 ((nv rd.val).getD p.val 0) ((nv rd.val).getD q' 0) := by
        intro E1 hE1 hcd q' h1 h2 h3 hj
        by_cases hq' : q' < j.val
        · exact Eqv.mono hE1 (hc0 q' h1 hq' h3 hj)
        · have : q' = j.val := by omega
          subst this; rw [hpv, hjv] at hj ⊢; exact hcd hj
      split
      · rename_i hbt
        have hl := uinv_len hu0
        step with union_spec u0 hu0 (fun e he => ⟨(hs0 e he).1, (hs0 e he).2.1⟩) hM e1 e2 h1n h2n
          as ⟨u1, hu1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, by scalar_tac, ⟨E0 ++ [(e1.val, e2.val)],
          fun e he => List.mem_append_left _ (hE0 e he), hu1, ?_, ?_⟩, by scalar_tac⟩
        · intro e he
          rcases List.mem_append.mp he with he | he
          · exact hs0 e he
          · simp at he; rw [he]
            refine ⟨h1n, h2n, hbE.mp hbt, hW _ ?_ _ ?_⟩
            · rw [← hpv]; exact u_getD_mem hp
            · rw [← hjv]; exact u_getD_mem hjl'
        · rw [show j1.val = j.val + 1 by scalar_tac]
          exact hkeep _ (fun e he => List.mem_append_left _ he)
            (fun _ => Eqv.edge (e := (e1.val, e2.val)) (by simp))
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, by scalar_tac, ⟨E0, hE0, hu0, hs0, ?_⟩, by scalar_tac⟩
        rw [show j1.val = j.val + 1 by scalar_tac]
        exact hkeep _ (fun e he => he) (fun hj => absurd (hbE.mpr hj) hbf)
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨E0, hE0, hu0, hs0, fun q' h1 h3 hj => hc0 q' h1 (by scalar_tac) h3 hj⟩
  · exact ⟨hq, hql, E, fun e he => he, hu, hs, hc⟩

theorem u_erel_mid_spec (ate : alloc.vec.Vec (alloc.vec.Vec U32)) (ecs : alloc.vec.Vec Usize)
    (hecs : ∀ x ∈ ecs.val, x.val < ate.val.length) (hM : ecs.val.length ≤ U32.max)
    (Wr : Nat → Nat → Prop) (rd : alloc.vec.Vec U32) (hrd : ∀ x ∈ rd.val, x.val < ecs.val.length)
    (hW : ∀ x ∈ nv rd.val, ∀ y ∈ nv rd.val, Wr x y)
    (uf : union_find.Uf) (E : List (Nat × Nat)) (p : Usize) (hp : p.val ≤ rd.val.length)
    (hu : UInv E ecs.val.length uf) (hs : ∀ e ∈ E, ES ate.val ecs.val Wr e)
    (hc : ∀ p' q', p' < p.val → p' < q' → q' < rd.val.length →
      EC ate.val ecs.val ((nv rd.val).getD p' 0) ((nv rd.val).getD q' 0) →
      Eqv E ((nv rd.val).getD p' 0) ((nv rd.val).getD q' 0)) :
    unify_loop6_loop1 ate ecs uf rd p ⦃ r => ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' ecs.val.length r ∧ (∀ e ∈ E', ES ate.val ecs.val Wr e) ∧
      ∀ i ∈ nv rd.val, ∀ j ∈ nv rd.val, EC ate.val ecs.val i j → Eqv E' i j ⦄ := by
  unfold unify_loop6_loop1
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => rd.val.length - st.2.val)
    (fun st => st.2.val ≤ rd.val.length ∧ ∃ E', (∀ e ∈ E, e ∈ E') ∧
      UInv E' ecs.val.length st.1 ∧ (∀ e ∈ E', ES ate.val ecs.val Wr e) ∧
      ∀ p' q', p' < st.2.val → p' < q' → q' < rd.val.length →
      EC ate.val ecs.val ((nv rd.val).getD p' 0) ((nv rd.val).getD q' 0) →
      Eqv E' ((nv rd.val).getD p' 0) ((nv rd.val).getD q' 0))
  · rintro ⟨u0, j⟩ ⟨hj, E0, hE0, hu0, hs0, hc0⟩
    unfold unify_loop6_loop1.body
    simp only at hj hE0 hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < rd.val.length := by scalar_tac
      step as ⟨q, hq⟩
      step with u_erel_inner_spec ate ecs hecs hM Wr rd hrd hW j hjl u0 E0 q (by scalar_tac) (by scalar_tac)
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
      apply u_pairs_all (P := EC ate.val ecs.val) (Q := Eqv E0) (fun _ _ h => u_EC_symm h)
        (fun _ _ h => Eqv.symm h) Eqv.refl
      intro p' q' h1 h2 hjc
      exact hc0 p' q' (by simp [nv] at h2; scalar_tac) h1 (by simpa [nv] using h2) hjc
  · exact ⟨hp, E, fun e he => he, hu, hs, hc⟩

/-- What a `round` holds: distinct edge-cell numbers, each `Good`. -/
def RdOK (M : Nat) (Good : Nat → Prop) (rd : List U32) : Prop :=
  (nv rd).Nodup ∧ ∀ x ∈ nv rd, x < M ∧ Good x

theorem u_rd_len {M : Nat} {Good : Nat → Prop} {rd : List U32} (h : RdOK M Good rd) : rd.length ≤ M := by
  have : nv rd ⊆ List.range M := fun x hx => List.mem_range.mpr (h.2 x hx).1
  have := List.Nodup.length_le_of_subset h.1 this; simpa [nv] using this

/-- `ec_pos` read at `e`. -/
def epAt (ep : List U32) (e : Nat) : Nat := (ep.getD e NONE).val

theorem u_round_inner_spec (ep : alloc.vec.Vec U32) (es : alloc.vec.Vec U32) (M : Nat) (hM : M < U32.max)
    (Good : Nat → Prop) (hes : ∀ e ∈ es.val, e.val < ep.val.length)
    (hgood : ∀ e ∈ nv es.val, epAt ep.val e ≠ U32.max → epAt ep.val e < M ∧ Good (epAt ep.val e))
    (rd : alloc.vec.Vec U32) (hrd : RdOK M Good rd.val) (q : Usize) (hq : q.val ≤ es.val.length) :
    unify_loop6_loop0_loop0 ep rd es q ⦃ r => RdOK M Good r.val ∧ (∀ x ∈ nv rd.val, x ∈ nv r.val) ∧
      ∀ q', q.val ≤ q' → q' < es.val.length → epAt ep.val ((nv es.val).getD q' 0) ≠ U32.max →
        epAt ep.val ((nv es.val).getD q' 0) ∈ nv r.val ⦄ := by
  have hu := u32_le_usize
  unfold unify_loop6_loop0_loop0
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec U32 × Usize) => es.val.length - st.2.val)
    (fun st => q.val ≤ st.2.val ∧ st.2.val ≤ es.val.length ∧ RdOK M Good st.1.val ∧
      (∀ x ∈ nv rd.val, x ∈ nv st.1.val) ∧
      ∀ q', q.val ≤ q' → q' < st.2.val → epAt ep.val ((nv es.val).getD q' 0) ≠ U32.max →
        epAt ep.val ((nv es.val).getD q' 0) ∈ nv st.1.val)
  · rintro ⟨r0, j⟩ ⟨hqj, hj, hr0, hsub, hc⟩
    unfold unify_loop6_loop0_loop0.body
    simp only at hqj hj hr0 hsub hc ⊢
    split
    · rename_i hlt
      have hjl : j.val < es.val.length := by scalar_tac
      have hrl := u_rd_len hr0
      step as ⟨x, hx⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = x.val := by rw [hi2]; scalar_tac
      have hxl : x.val < ep.val.length := by rw [hx]; exact hes _ (List.getElem_mem _)
      step as ⟨e, hev⟩
      have hej : (nv es.val).getD j.val 0 = x.val := by rw [u_getD_nv hjl, hx]
      have hepe : epAt ep.val ((nv es.val).getD j.val 0) = e.val := by
        rw [hej, epAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hxl]; simp [hev, hi2v]
      have hgj := hgood _ (u_getD_mem hjl)
      rw [hepe] at hgj
      have hstep : ∀ r1 : alloc.vec.Vec U32, RdOK M Good r1.val → (∀ y ∈ nv r0.val, y ∈ nv r1.val) →
          (e.val ≠ U32.max → e.val ∈ nv r1.val) →
          ∀ q', q.val ≤ q' → q' < j.val + 1 → epAt ep.val ((nv es.val).getD q' 0) ≠ U32.max →
            epAt ep.val ((nv es.val).getD q' 0) ∈ nv r1.val := by
        intro r1 _ hs1 he1 q' h1 h2 h3
        by_cases hqj' : q' < j.val
        · exact hs1 _ (hc q' h1 hqj' h3)
        · have : q' = j.val := by omega
          subst this; rw [hepe] at h3 ⊢; exact he1 h3
      split
      · rename_i hne
        have hne' : e.val ≠ U32.max := by
          intro h; simp only [bne_iff_ne, ne_eq] at hne; apply hne; apply UScalar.eq_of_val_eq; rw [h, none_val]
        step with has_spec (alloc.vec.Vec.deref r0) e as ⟨b, hb⟩
        split
        · rename_i hbt
          step as ⟨j1, hj1⟩
          have hm : e.val ∈ nv r0.val := by
            have := hb ▸ hbt; simpa [alloc.vec.Vec.deref] using this
          refine ⟨by scalar_tac, by scalar_tac, hr0, hsub, ?_, by scalar_tac⟩
          rw [show j1.val = j.val + 1 by scalar_tac]
          exact hstep r0 hr0 (fun y hy => hy) (fun _ => hm)
        · rename_i hbf
          have hnm : e.val ∉ nv r0.val := by
            intro h; apply hbf; rw [hb]; simpa [alloc.vec.Vec.deref] using h
          have hr0l : r0.val.length < Usize.max := by omega
          step as ⟨r1, hr1⟩
          step as ⟨j1, hj1⟩
          have hr1v : nv r1.val = nv r0.val ++ [e.val] := by rw [hr1]; simp [nv]
          have hok1 : RdOK M Good r1.val := by
            refine ⟨?_, ?_⟩
            · rw [hr1v]; exact List.nodup_append.mpr ⟨hr0.1, by simp, fun a ha b hb => by
                simp at hb; rintro rfl; exact hnm (hb ▸ ha)⟩
            · intro y hy; rw [hr1v] at hy
              rcases List.mem_append.mp hy with hy | hy
              · exact hr0.2 y hy
              · simp at hy; rw [hy]; exact hgj hne'
          refine ⟨by scalar_tac, by scalar_tac, hok1, fun y hy => ?_, ?_, by scalar_tac⟩
          · rw [hr1v]; exact List.mem_append_left _ (hsub y hy)
          · rw [show j1.val = j.val + 1 by scalar_tac]
            exact hstep r1 hok1 (fun y hy => by rw [hr1v]; exact List.mem_append_left _ hy)
              (fun _ => by rw [hr1v]; simp)
      · rename_i heq
        have heq' : e.val = U32.max := by
          simp only [bne_iff_ne, ne_eq, Decidable.not_not] at heq; rw [heq, none_val]
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, by scalar_tac, hr0, hsub, ?_, by scalar_tac⟩
        rw [show j1.val = j.val + 1 by scalar_tac]
        exact hstep r0 hr0 (fun y hy => hy) (fun h => absurd heq' h)
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hr0, hsub, fun q' h1 h2 h3 => hc q' h1 (by scalar_tac) h3⟩
  · exact ⟨le_refl _, hq, hrd, fun x hx => hx, fun q' h1 h2 => by simp only at h2; omega⟩

theorem u_mem_aroundL {l : Ledger} {ext : Ext} {a : Assembled} {x : List Sgn} {p : Nat} :
    p ∈ aroundL l ext a x ↔ p < a.result.val.length ∧ LeS x (covAt l ext (resAt a p)) := by
  simp [aroundL, leS_iff]

theorem u_round_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (isE : Slice Bool) (hN : isE.val.length = (allCells l ext).length)
    (ep : alloc.vec.Vec U32) (hepl : ep.val.length = (allCells l ext).length) (M : Nat) (hM : M < U32.max)
    (EI : Nat → Nat)
    (hep : ∀ e, e < (allCells l ext).length → epAt ep.val e ≠ U32.max → epAt ep.val e < M ∧ EI (epAt ep.val e) = e)
    (v : U32) (hv : v.val < (allCells l ext).length) (z : alloc.vec.Vec Usize)
    (hz : nvU z.val = zerosL (covAt l ext v.val))
    (ar : alloc.vec.Vec U32) (har : nv ar.val = aroundL l ext a (covAt l ext v.val))
    (rd : alloc.vec.Vec U32)
    (hrd : RdOK M (fun x => LeS (covAt l ext v.val) (covAt l ext (EI x))) rd.val)
    (j : Usize) (hj : j.val ≤ ar.val.length) :
    unify_loop6_loop0 l.nb l.face_body l.face_class l.cells l.pieces l.inside l.edges l.verts ext a.result isE ep v z
      rd ar j ⦃ r => RdOK M (fun x => LeS (covAt l ext v.val) (covAt l ext (EI x))) r.val ∧
        ∀ j', j.val ≤ j' → j' < ar.val.length → ∀ e,
          EdgeBetween l ext isE.val (covAt l ext v.val) (covAt l ext (resAt a ((nv ar.val).getD j' 0))) e →
          epAt ep.val e ≠ U32.max → epAt ep.val e ∈ nv r.val ⦄ := by
  have harn : ∀ x ∈ ar.val, x.val < a.result.val.length ∧ LeS (covAt l ext v.val) (covAt l ext (resAt a x.val)) := by
    intro x hx
    have : x.val ∈ aroundL l ext a (covAt l ext v.val) := by rw [← har]; exact List.mem_map_of_mem hx
    exact u_mem_aroundL.mp this
  unfold unify_loop6_loop0
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec U32 × Usize) => ar.val.length - st.2.val)
    (fun st => j.val ≤ st.2.val ∧ st.2.val ≤ ar.val.length ∧
      RdOK M (fun x => LeS (covAt l ext v.val) (covAt l ext (EI x))) st.1.val ∧
      ∀ j', j.val ≤ j' → j' < st.2.val → ∀ e,
        EdgeBetween l ext isE.val (covAt l ext v.val) (covAt l ext (resAt a ((nv ar.val).getD j' 0))) e →
        epAt ep.val e ≠ U32.max → epAt ep.val e ∈ nv st.1.val)
  · rintro ⟨r0, i⟩ ⟨hji, hi, hr0, hc⟩
    unfold unify_loop6_loop0.body
    simp only at hji hi hr0 hc ⊢
    split
    · rename_i hlt
      have hil : i.val < ar.val.length := by scalar_tac
      obtain ⟨hxn, hxle⟩ := harn _ (List.getElem_mem hil)
      step as ⟨x, hx⟩
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = x.val := by rw [hi3]; scalar_tac
      step as ⟨phi, hphi⟩
      have hxn' : x.val < a.result.val.length := by rw [hx]; exact hxn
      have hphiv : phi.val = resAt a x.val := by rw [u_resAt_eq a hxn', hphi]; simp [hi3v]
      have hphil : phi.val < (allCells l ext).length := by rw [hphiv]; exact resAt_lt h hxn'
      have hfac : (covAt l ext phi.val).count .Zero = 1 := by rw [hphiv]; exact resAt_facet h hxn'
      have hle : LeS (covAt l ext v.val) (covAt l ext phi.val) := by rw [hphiv, hx]; exact hxle
      have hxi : (nv ar.val).getD i.val 0 = x.val := by rw [u_getD_nv hil, hx]
      step with edges_between_spec l ext hok isE v phi (alloc.vec.Vec.deref z) hN hv hphil
        (by simpa [alloc.vec.Vec.deref, nvU] using hz) hfac hle as ⟨es, hes⟩
      have hesl : ∀ e ∈ es.val, e.val < ep.val.length := by
        intro e he
        have := ((hes e.val).mp (List.mem_map_of_mem he)).1; omega
      have hgood : ∀ e ∈ nv es.val, epAt ep.val e ≠ U32.max →
          epAt ep.val e < M ∧ LeS (covAt l ext v.val) (covAt l ext (EI (epAt ep.val e))) := by
        intro e he hne
        have heb := (hes e).mp he
        obtain ⟨h1, h2⟩ := hep e (by rw [← hN]; exact heb.1) hne
        exact ⟨h1, by rw [h2]; exact heb.2.2.1⟩
      step with u_round_inner_spec ep es M hM _ hesl hgood r0 hr0 0#usize (by simp) as ⟨r1, hr1, hsub, hc1⟩
      step as ⟨i1, hi1⟩
      refine ⟨by scalar_tac, by scalar_tac, hr1, ?_, by scalar_tac⟩
      intro j' h1 h2 e heb hne
      rw [show i1.val = i.val + 1 by scalar_tac] at h2
      by_cases hj' : j' < i.val
      · exact hsub _ (hc j' h1 hj' e heb hne)
      · have : j' = i.val := by omega
        subst this
        rw [hxi, ← hphiv] at heb
        have hem : e ∈ nv es.val := (hes e).mpr heb
        obtain ⟨q, hq, hqe⟩ := List.getElem_of_mem hem
        have hql : q < es.val.length := by simpa [nv] using hq
        have := hc1 q (Nat.zero_le _) hql
        rw [show (nv es.val).getD q 0 = e by rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hq, hqe]; rfl] at this
        exact this hne
    · rename_i hge
      simp only [WP.spec_ok]
      exact ⟨hr0, fun j' h1 h2 => hc j' h1 (by scalar_tac)⟩
  · exact ⟨le_refl _, hj, hrd, fun j' h1 h2 => by simp only at h2; omega⟩

/-- The vertex cell at index `k`. -/
def vAt (l : Ledger) (k : Nat) : Nat := (nvC l.verts.val).getD k 0

/-- Edge cell number `x` lies between vertex `k` and one of its facets. -/
def CvR (l : Ledger) (ext : Ext) (a : Assembled) (isE : List Bool) (av : List (alloc.vec.Vec U32)) (EI : Nat → Nat)
    (k x : Nat) : Prop :=
  ∃ j', j' < (av.getD k default).val.length ∧
    EdgeBetween l ext isE (covAt l ext (vAt l k)) (covAt l ext (resAt a ((nv (av.getD k default).val).getD j' 0))) (EI x)

/-- Two edge cells with a common vertex cell, not a vertex, below both. -/
def WrR (l : Ledger) (ext : Ext) (io : List Bool) (EI : Nat → Nat) (x y : Nat) : Prop :=
  ∃ k, k < l.verts.val.length ∧ io.getD k false = false ∧
    LeS (covAt l ext (vAt l k)) (covAt l ext (EI x)) ∧ LeS (covAt l ext (vAt l k)) (covAt l ext (EI y))

theorem u_vAt_eq (l : Ledger) {k : Nat} (hk : k < l.verts.val.length) : vAt l k = (l.verts.val[k] : U32).val := by
  simp [vAt, nvC, List.getD_eq_getElem?_getD, hk]

/-- **The edge join**: sound pairs, and every pair of edge cells joining one pair of faces round a
non-vertex joined. -/
theorem u_erel_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (isE : Slice Bool) (hN : isE.val.length = (allCells l ext).length)
    (hvl : ∀ v ∈ l.verts.val, (v : U32).val < (allCells l ext).length)
    (av : Slice (alloc.vec.Vec U32)) (havl : av.val.length = l.verts.val.length)
    (hav : ∀ k, k < l.verts.val.length → nv (av.val.getD k default).val = aroundL l ext a (covAt l ext (vAt l k)))
    (ate : alloc.vec.Vec (alloc.vec.Vec U32)) (ecs : alloc.vec.Vec Usize)
    (hecs : ∀ x ∈ ecs.val, x.val < ate.val.length) (hM : ecs.val.length < U32.max)
    (io : alloc.vec.Vec Bool) (hio : io.val.length = l.verts.val.length)
    (ep : alloc.vec.Vec U32) (hepl : ep.val.length = (allCells l ext).length) (EI : Nat → Nat)
    (hep : ∀ e, e < (allCells l ext).length → epAt ep.val e ≠ U32.max →
      epAt ep.val e < ecs.val.length ∧ EI (epAt ep.val e) = e)
    (hEI : ∀ x, x < ecs.val.length → EI x < (allCells l ext).length ∧ epAt ep.val (EI x) = x)
    (uf : union_find.Uf) (E : List (Nat × Nat)) (k : Usize) (hk : k.val ≤ l.verts.val.length)
    (hu : UInv E ecs.val.length uf) (hs : ∀ e ∈ E, ES ate.val ecs.val (WrR l ext io.val EI) e)
    (hc : ∀ k', k' < k.val → io.val.getD k' false = false → ∀ x y, x < ecs.val.length → y < ecs.val.length →
      CvR l ext a isE.val av.val EI k' x → CvR l ext a isE.val av.val EI k' y → EC ate.val ecs.val x y → Eqv E x y) :
    unify_loop6 l.nb l.face_body l.face_class l.cells l.pieces l.inside l.edges l.verts ext a.result isE av ate io
      ecs ep uf k ⦃ r => ∃ E', UInv E' ecs.val.length r ∧ (∀ e ∈ E', ES ate.val ecs.val (WrR l ext io.val EI) e) ∧
      ∀ k', k' < l.verts.val.length → io.val.getD k' false = false → ∀ x y, x < ecs.val.length → y < ecs.val.length →
        CvR l ext a isE.val av.val EI k' x → CvR l ext a isE.val av.val EI k' y → EC ate.val ecs.val x y →
        Eqv E' x y ⦄ := by
  unfold unify_loop6
  apply loop.spec_decr_nat (fun (st : union_find.Uf × Usize) => l.verts.val.length - st.2.val)
    (fun st => st.2.val ≤ l.verts.val.length ∧ ∃ E', UInv E' ecs.val.length st.1 ∧
      (∀ e ∈ E', ES ate.val ecs.val (WrR l ext io.val EI) e) ∧
      ∀ k', k' < st.2.val → io.val.getD k' false = false → ∀ x y, x < ecs.val.length → y < ecs.val.length →
        CvR l ext a isE.val av.val EI k' x → CvR l ext a isE.val av.val EI k' y → EC ate.val ecs.val x y →
        Eqv E' x y)
  · rintro ⟨u0, j⟩ ⟨hj, E0, hu0, hs0, hc0⟩
    unfold unify_loop6.body
    simp only at hj hu0 hs0 hc0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < l.verts.val.length := by scalar_tac
      step as ⟨b, hb⟩
      have hbd : io.val.getD j.val false = b := by
        rw [hb, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
      split
      · rename_i hbt
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ⟨E0, hu0, hs0, ?_⟩, by scalar_tac⟩
        intro k' hk' hio' x y hx hy h1 h2 h3
        rw [show j1.val = j.val + 1 by scalar_tac] at hk'
        by_cases hkj : k' < j.val
        · exact hc0 k' hkj hio' x y hx hy h1 h2 h3
        · have : k' = j.val := by omega
          subst this; rw [hbd] at hio'; simp [hio'] at hbt
      · rename_i hbf
        have hbf' : b = false := by simpa using hbf
        step as ⟨ci, hci⟩
        have hciv : (ci : U32).val = vAt l j.val := by rw [u_vAt_eq l hjl, hci]
        have hcil : (ci : U32).val < (allCells l ext).length := by rw [hci]; exact hvl _ (List.getElem_mem _)
        step with cov_spec l ext ci hcil as ⟨sv, hsv⟩
        step with zeros_spec sv as ⟨z, hz⟩
        step as ⟨ar, har⟩
        have hard : av.val.getD j.val default = ar := by
          rw [har, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
        have har' : nv ar.val = aroundL l ext a (covAt l ext (ci : U32).val) := by
          rw [← hard, hav j.val hjl, hciv]
        step with u_round_spec l ext hok a W h isE hN ep hepl ecs.val.length hM EI hep ci hcil z
          (by rw [nvU, hz, hsv]) ar har' (alloc.vec.Vec.new U32) ⟨by simp [nv], by simp [nv]⟩ 0#usize (by simp)
          as ⟨rd, hrd, hrdc⟩
        have hrdl : ∀ x ∈ rd.val, x.val < ecs.val.length := fun x hx => (hrd.2 x.val (List.mem_map_of_mem hx)).1
        have hW : ∀ x ∈ nv rd.val, ∀ y ∈ nv rd.val, WrR l ext io.val EI x y := by
          intro x hx y hy
          refine ⟨j.val, hjl, by rw [hbd, hbf'], ?_, ?_⟩
          · rw [← hciv]; exact (hrd.2 x hx).2
          · rw [← hciv]; exact (hrd.2 y hy).2
        step with u_erel_mid_spec ate ecs hecs (by omega) (WrR l ext io.val EI) rd hrdl hW u0 E0 0#usize (by simp)
          hu0 hs0 (fun p' q' h1 => by simp at h1) as ⟨u1, E1, hE1, hu1, hs1, hc1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by rw [hj1]; omega, ⟨E1, hu1, hs1, ?_⟩, by rw [hj1]; omega⟩
        intro k' hk' hio' x y hx hy h1 h2 h3
        rw [hj1] at hk'
        by_cases hkj : k' < j.val
        · exact Eqv.mono hE1 (hc0 k' hkj hio' x y hx hy h1 h2 h3)
        · have : k' = j.val := by omega
          subst this
          have hin : ∀ z, z < ecs.val.length → CvR l ext a isE.val av.val EI j.val z → z ∈ nv rd.val := by
            intro z hz ⟨j', hj', heb⟩
            rw [hard] at hj' heb
            rw [← hciv] at heb
            have := hrdc j' (Nat.zero_le _) hj' (EI z) heb (by rw [(hEI z hz).2]; omega)
            rwa [(hEI z hz).2] at this
          exact hc1 x (hin x hx h1) y (hin y hy h2) h3
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = l.verts.val.length := by scalar_tac
      rw [this] at hc0
      exact ⟨E0, hu0, hs0, hc0⟩
  · exact ⟨hk, E, hu, hs, hc⟩

end forge_bool_core
