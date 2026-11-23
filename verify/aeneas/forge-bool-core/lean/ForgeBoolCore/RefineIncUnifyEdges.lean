import ForgeBoolCore.RefineIncUnifyErel

/-!
# Group U (inc), part 5: the edges loop

With the edge join's roots `Rt` (least positions in `ecs`), `inc::unify`'s last loop numbers the
heads in `ecs` order and fills each edge with its edges in order, naming its faces by their head
facets, `at_e` of its head (`edges_spec`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

theorem ff_spec (fs : alloc.vec.Vec U32) (ff : alloc.vec.Vec FacetId) (k : Usize) (hk : k.val ≤ fs.val.length)
    (hff : ff.val.map (fun (c : FacetId) => (c : U32).val) = nv (fs.val.take k.val)) :
    inc.unify_loop7_loop0 fs ff k ⦃ r => r.val.map (fun (c : FacetId) => (c : U32).val) = nv fs.val ⦄ := by
  unfold inc.unify_loop7_loop0
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec FacetId × Usize) => fs.val.length - st.2.val)
    (fun st => st.2.val ≤ fs.val.length ∧
      st.1.val.map (fun (c : FacetId) => (c : U32).val) = nv (fs.val.take st.2.val))
  · rintro ⟨c, j⟩ ⟨hj, hc⟩
    unfold inc.unify_loop7_loop0.body
    simp only at hj hc ⊢
    split
    · rename_i hlt
      have hjl : j.val < fs.val.length := by scalar_tac
      have hcl : c.val.length = j.val := by
        have := congrArg List.length hc; simp [nv] at this; omega
      step as ⟨x, hx⟩
      have hcl2 : c.val.length < Usize.max := by scalar_tac
      step as ⟨c1, hc1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [hc1, show j1.val = j.val + 1 by scalar_tac, List.map_append, hc, List.take_succ_eq_append_getElem hjl]
      simp only [nv, List.map_append, List.map_cons, List.map_nil, hx]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = fs.val.length := by scalar_tac
      rw [this, List.take_length] at hc; exact hc
  · exact ⟨hk, hff⟩

/-- An edge as the loop writes it at step `p`, headed by `h`. -/
structure EdgeK (ecs : List U32) (A : Nat → List Nat) (Rt : Nat → Nat) (p h : Nat) (E : OutEdge) : Prop where
  cells : E.cells.val.map (fun (c : EdgeId) => (c : U32).val) =
    ((List.range p).filter fun d => Rt d = h).map (idA ecs)
  faces : E.faces.val.map (fun (c : FacetId) => (c : U32).val) = A (idA ecs h)

/-- What the edges loop keeps, after positions `0 .. p-1`. -/
structure EdInv (ecs : List U32) (A : Nat → List Nat) (Rt : Nat → Nat) (m p : Nat)
    (edge_of : List U32) (edges : List OutEdge) : Prop where
  eol : edge_of.length = m
  eo : ∀ i, i < p → (edge_of.getD i NONE).val = (hdL Rt (Rt i)).length
  el : edges.length = (hdL Rt p).length
  ek : ∀ k (hk : k < edges.length), EdgeK ecs A Rt p ((hdL Rt p).getD k 0) edges[k]

theorem hdL_getD {Rt : Nat → Nat} {n k : Nat} (hk : k < (hdL Rt n).length) :
    (hdL Rt n).getD k 0 = (hdL Rt n)[k] := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk]; rfl

theorem filter_succ_ne {Rt : Nat → Nat} {p h : Nat} (hne : Rt p ≠ h) :
    (List.range (p + 1)).filter (fun d => decide (Rt d = h)) = (List.range p).filter (fun d => decide (Rt d = h)) := by
  rw [u_filter_range_succ]; simp [hne]

theorem filter_succ_eq {Rt : Nat → Nat} {p h : Nat} (he : Rt p = h) :
    (List.range (p + 1)).filter (fun d => decide (Rt d = h)) =
      (List.range p).filter (fun d => decide (Rt d = h)) ++ [p] := by
  rw [u_filter_range_succ]; simp [he]

theorem edges_spec (ne : Nat) (A : Nat → List Nat) (ate : alloc.vec.Vec (alloc.vec.Vec U32))
    (hate : ate.val.map (fun v => nv v.val) = (List.range ne).map A)
    (ecs : alloc.vec.Vec U32) (hecs : ∀ x ∈ ecs.val, x.val < ne) (hm : ecs.val.length < U32.max)
    (Rt : Nat → Nat) (hRt : RtOK Rt ecs.val.length)
    (erel : union_find.Uf) (edges : alloc.vec.Vec OutEdge) (edge_of : alloc.vec.Vec U32)
    (p : Usize) (hp : p.val ≤ ecs.val.length) (hu : UfR Rt ecs.val.length erel)
    (hI : EdInv ecs.val A Rt ecs.val.length p.val edge_of.val edges.val) :
    inc.unify_loop7 ate ecs erel edges edge_of p ⦃ r =>
      r.val.length = (hdL Rt ecs.val.length).length ∧
      ∀ k (hk : k < r.val.length), EdgeK ecs.val A Rt ecs.val.length ((hdL Rt ecs.val.length).getD k 0) r.val[k] ⦄ := by
  have hatl : ate.val.length = ne := by have := congrArg List.length hate; simpa using this
  have hA : ∀ x (hx : x < ne), nv (ate.val[x]'(by omega)).val = A x := by
    intro x hx
    have h1 := congrArg (fun L => L[x]?) hate
    simp only [List.getElem?_map] at h1
    rw [List.getElem?_eq_getElem (by omega), List.getElem?_range hx] at h1
    simpa using h1
  unfold inc.unify_loop7
  apply loop.spec_decr_nat
    (fun (st : union_find.Uf × alloc.vec.Vec OutEdge × alloc.vec.Vec U32 × Usize) => ecs.val.length - st.2.2.2.val)
    (fun st => st.2.2.2.val ≤ ecs.val.length ∧ UfR Rt ecs.val.length st.1 ∧
      EdInv ecs.val A Rt ecs.val.length st.2.2.2.val st.2.2.1.val st.2.1.val)
  · rintro ⟨u0, es, eo, j⟩ ⟨hj, hu0, hI0⟩
    unfold inc.unify_loop7.body
    simp only at hj hu0 hI0 ⊢
    split
    · rename_i hlt
      have hjn : j.val < ecs.val.length := by scalar_tac
      obtain ⟨hg0, hl0, hr0⟩ := hu0
      obtain ⟨hR, hRle, hRR⟩ := hRt j.val hjn
      have heol := hI0.eol
      have hesl : es.val.length ≤ j.val := by rw [hI0.el]; exact u_filter_range_len _ _
      have hecj := hecs _ (List.getElem_mem hjn)
      have hidj : (ecs.val[j.val]).val = idA ecs.val j.val := id_read hjn
      step with find_spec u0 j (by omega) hg0 as ⟨r, u1, hr, hl1, hg1, hr1⟩
      have hrv : r.val = Rt j.val := by rw [hr, hr0]
      have hu1 : UfR Rt ecs.val.length u1 := ⟨hg1, by omega, fun y => by rw [hr1, hr0]⟩
      split
      · rename_i hrj
        have hRj : Rt j.val = j.val := by rw [← hrv, hrj]
        have hsucc : hdL Rt (j.val + 1) = hdL Rt j.val ++ [j.val] := by rw [u_hdL_succ, if_pos hRj]
        step as ⟨y0, back, hy0, hback⟩
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = es.val.length := by
          have := u_cast_u32 (x := alloc.vec.Vec.len es) (by simp; omega)
          rw [hi2]; simpa using this
        step as ⟨i3, hi3⟩
        step as ⟨i4, hi4⟩
        have hi4v : i4.val = idA ecs.val j.val := by rw [hi4, hi3, ← hidj]; scalar_tac
        have hi4n : i4.val < ne := by rw [hi4v, ← hidj]; exact hecj
        step as ⟨fs, hfs⟩
        step with ff_spec fs (alloc.vec.Vec.with_capacity FacetId (alloc.vec.Vec.len fs)) 0#usize (by simp)
          (by simp [nv, alloc.vec.Vec.with_capacity]) as ⟨ff1, hff1⟩
        have hff1A : ff1.val.map (fun (c : FacetId) => (c : U32).val) = A (idA ecs.val j.val) := by
          rw [hff1, hfs, ← hi4v]; exact hA _ hi4n
        have hesl2 : es.val.length < Usize.max := by have := u32_le_usize; omega
        step as ⟨es1, hes1⟩
        have heo1 : (back i2).val.length = ecs.val.length := by rw [hback]; simp [heol]
        step as ⟨i6, hi6⟩
        have hi6v : i6.val = es.val.length := by
          simp [hi6, hback, hi2v]
        step as ⟨i7, hi7⟩
        have hi7v : i7.val = es.val.length := by rw [hi7, ← hi6v]; scalar_tac
        have hi7l : i7.val < es1.val.length := by rw [hi7v, hes1]; simp
        step as ⟨oe, back1, hoe, hback1⟩
        have hoe' : oe = { cells := alloc.vec.Vec.new EdgeId, faces := ff1 } := by
          rw [hoe]; simp only [hes1, hi7v]; simp
        step as ⟨i8, hi8⟩
        have hpush : oe.cells.val.length < Usize.max := by
          rw [hoe']; simp
          have := u32_le_usize; have : (0 : Nat) < U32.max := by simp [U32.max_eq]
          omega
        step as ⟨v, hv⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨by omega, hu1, ?_, by omega⟩
        rw [hj1v]
        refine ⟨heo1, ?_, ?_, ?_⟩
        · intro k hk
          rw [hback, alloc.vec.Vec.set_val_eq]
          by_cases hkj : k = j.val
          · subst hkj; rw [getD_set_eq' (by omega), hi2v, hI0.el, hRj]
          · rw [getD_set_ne' hkj, hI0.eo k (by omega)]
        · rw [hback1, hsucc]; simp [hes1, hI0.el]
        · intro k hk
          have hkl : k < es.val.length + 1 := by rw [hback1] at hk; simpa [hes1] using hk
          rw [hsucc]
          simp only [hback1, alloc.vec.Vec.set_val_eq]
          by_cases hke : k = es.val.length
          · subst hke
            have hg : (hdL Rt j.val ++ [j.val]).getD es.val.length 0 = j.val := by
              rw [hI0.el]; simp [List.getD_eq_getElem?_getD]
            rw [hg]
            simp only [hi7v, List.getElem_set_self]
            refine ⟨?_, ?_⟩
            · rw [hv, hoe', filter_succ_eq hRj]
              have : (List.range j.val).filter (fun d => decide (Rt d = j.val)) = [] := by
                rw [List.filter_eq_nil_iff]; intro d hd; simp at hd ⊢
                have := (hRt d (by omega)).2.1; omega
              rw [this]; simp [hi8, hidj]
            · simp only; rw [hoe'] at *; exact hff1A
          · have hkl' : k < es.val.length := by omega
            rw [List.getElem_set_ne (by rw [hi7v]; exact Ne.symm hke)]
            simp only [hes1]
            rw [List.getElem_append_left hkl', u_getD_append_left (by rw [← hI0.el]; exact hkl')]
            have hkI := hI0.ek k hkl'
            have hne : Rt j.val ≠ (hdL Rt j.val).getD k 0 := by
              rw [hRj, hdL_getD (by rw [← hI0.el]; exact hkl')]
              intro he
              have := (u_mem_hdL.mp (List.getElem_mem (l := hdL Rt j.val) (by rw [← hI0.el]; exact hkl'))).1
              omega
            exact ⟨by rw [hkI.cells, filter_succ_ne hne], hkI.faces⟩
      · rename_i hrj
        have hRj : Rt j.val ≠ j.val := by rw [← hrv]; intro h; exact hrj (UScalar.eq_of_val_eq h)
        have hrlt : r.val < j.val := by rw [hrv]; omega
        have hsucc : hdL Rt (j.val + 1) = hdL Rt j.val := by rw [u_hdL_succ, if_neg hRj, List.append_nil]
        obtain ⟨hkl0, hkg0⟩ := hdL_get (n := j.val) hRR (by omega)
        step as ⟨i1, hi1⟩
        have hi1v : i1.val = (hdL Rt (Rt j.val)).length := by
          have := hI0.eo r.val hrlt
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some] at this
          rw [hi1, this, hrv, hRR]
        step as ⟨y0, back, hy0, hback⟩
        have heo1 : (back i1).val.length = ecs.val.length := by rw [hback]; simp [heol]
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = (hdL Rt (Rt j.val)).length := by
          simp [hi2, hback, hi1v]
        step as ⟨i3, hi3⟩
        have hi3v : i3.val = (hdL Rt (Rt j.val)).length := by rw [hi3, ← hi2v]; scalar_tac
        have hi3l : i3.val < es.val.length := by rw [hi3v, hI0.el]; exact hkl0
        step as ⟨oe, back1, hoe, hback1⟩
        step as ⟨i4, hi4⟩
        have hoel : oe.cells.val.length ≤ j.val := by
          have := (hI0.ek i3.val hi3l).cells
          have h2 := congrArg List.length this
          simp only [List.length_map] at h2
          rw [hoe, h2]; exact u_filter_range_len _ _
        have hpush : oe.cells.val.length < Usize.max := by have := u32_le_usize; omega
        step as ⟨v, hv⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨by omega, hu1, ?_, by omega⟩
        rw [hj1v]
        refine ⟨heo1, ?_, ?_, ?_⟩
        · intro k hk
          rw [hback, alloc.vec.Vec.set_val_eq]
          by_cases hkj : k = j.val
          · subst hkj; rw [getD_set_eq' (by omega), hi1v]
          · rw [getD_set_ne' hkj, hI0.eo k (by omega)]
        · rw [hback1, hsucc]; simp [hI0.el]
        · intro k hk
          have hk' : k < es.val.length := by rw [hback1] at hk; simpa using hk
          have hkI := hI0.ek k hk'
          rw [hsucc]
          simp only [hback1, alloc.vec.Vec.set_val_eq]
          by_cases hki : k = i3.val
          · subst hki
            simp only [List.getElem_set_self]
            have hgk : (hdL Rt j.val).getD i3.val 0 = Rt j.val := by
              rw [hdL_getD (by rw [← hI0.el]; exact hk')]; simp only [hi3v]; exact hkg0
            rw [hgk] at hkI ⊢
            rw [← hoe] at hkI
            refine ⟨?_, hkI.faces⟩
            rw [hv, List.map_append, hkI.cells, filter_succ_eq rfl]
            simp [hi4, hidj]
          · rw [List.getElem_set_ne (Ne.symm hki)]
            have hne : Rt j.val ≠ (hdL Rt j.val).getD k 0 := by
              intro he; apply hki
              have hnd := u_hdL_nodup Rt j.val
              rw [hdL_getD (by rw [← hI0.el]; exact hk')] at he
              rw [hi3v]
              exact (List.Nodup.getElem_inj_iff hnd (i := k) (j := (hdL Rt (Rt j.val)).length)
                (hi := by rw [← hI0.el]; exact hk') (hj := hkl0)).mp (by rw [← he, hkg0])
            exact ⟨by rw [hkI.cells, filter_succ_ne hne], hkI.faces⟩
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = ecs.val.length := by scalar_tac
      rw [this] at hI0; exact ⟨hI0.el, hI0.ek⟩
  · exact ⟨hp, hu, hI⟩

end inc

end forge_bool_core
