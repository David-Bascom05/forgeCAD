import ForgeBoolCore.RefineIncUnifyFaces

/-!
# Group U (inc), part 3: the faces round each edge and vert, the verts, the edges between two faces

`at_e[e]` is `faces_at head_of (around_e e)` (`ate_spec`); a vert is one with three or more faces
round it, its sectors' facets read as result positions (`verts_spec`); `ecs` lists the edges with
two faces, in edge order, and `ec_pos` is that numbering by edge (`ecs_spec`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- The faces round edge `k`, by head facet, as `faces_at` reads them. -/
def AtEf (l : Ledger) (a : Assembled) (ho : List U32) (k : Nat) : List Nat :=
  dedup ((aroundL a (rowL l.edge_facets k)).map fun p => (nv ho).getD p 0)

theorem ate_spec (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) (hrl : a.rpos.val.length = nfacets l)
    (head_of : alloc.vec.Vec U32)
    (hrn : ∀ e, e < nrowsL l.edge_facets → ∀ x ∈ aroundL a (rowL l.edge_facets e), x < head_of.val.length)
    (ne : Usize) (hne : ne.val = nrowsL l.edge_facets)
    (at_e : alloc.vec.Vec (alloc.vec.Vec U32)) (e : Usize) (he : e.val ≤ ne.val)
    (hI : at_e.val.map (fun v => nv v.val) = (List.range e.val).map (AtEf l a head_of.val)) :
    inc.unify_loop3 l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids l.vert_sectors
      a.result a.rpos a.orient ne head_of at_e e ⦃ r =>
      r.val.map (fun v => nv v.val) = (List.range ne.val).map (AtEf l a head_of.val) ⦄ := by
  have hsm := hw.small
  unfold inc.unify_loop3
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => ne.val - st.2.val)
    (fun st => st.2.val ≤ ne.val ∧
      st.1.val.map (fun v => nv v.val) = (List.range st.2.val).map (AtEf l a head_of.val))
  · rintro ⟨c, j⟩ ⟨hj, hc⟩
    unfold inc.unify_loop3.body
    simp only at hj hc ⊢
    split
    · rename_i hlt
      have hjl : j.val < nrowsL l.edge_facets := by scalar_tac
      have hcl : c.val.length = j.val := by
        have := congrArg List.length hc; simp at this; omega
      step with around_e_spec l hw a hrl j hjl as ⟨ar, har⟩
      step with faces_at_spec (alloc.vec.Vec.deref head_of) (alloc.vec.Vec.deref ar)
        (fun y hy => by
          have hy' : y ∈ ar.val := by simpa [alloc.vec.Vec.deref] using hy
          have : y.val ∈ aroundL a (rowL l.edge_facets j.val) := by
            rw [← show nv ar.val = _ from har]; exact List.mem_map_of_mem hy'
          simpa [alloc.vec.Vec.deref] using hrn j.val hjl _ this)
        as ⟨w, hw'⟩
      have hcl2 : c.val.length < Usize.max := by have := u32_le_usize; omega
      step as ⟨c1, hc1⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by omega
      refine ⟨by omega, ?_, by omega⟩
      rw [hc1, hj1v, List.range_succ, List.map_append, List.map_append, hc]
      simp only [List.map_cons, List.map_nil]
      congr 2
      rw [hw']; simp [AtEf, alloc.vec.Vec.deref, ← har]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = ne.val := by scalar_tac
      rw [this] at hc; exact hc
  · exact ⟨he, hI⟩

/-- A vert's result sectors' facets, as result positions. -/
def PSf (l : Ledger) (a : Assembled) (v : Nat) : List Nat :=
  (avL l a (sstAt l v) (sstAt l (v + 1))).map fun x => rpA a (secAt l x).facet.val

/-- The faces round vert `v`, by head facet. -/
def AtVf (l : Ledger) (a : Assembled) (ho : List U32) (v : Nat) : List Nat :=
  dedup ((PSf l a v).map fun p => (nv ho).getD p 0)

theorem ps_spec (sc : SecCsr) (rp : alloc.vec.Vec U32) (ar : alloc.vec.Vec Usize)
    (har : ∀ x ∈ ar.val, x.val < sc.sec.val.length)
    (hfr : ∀ x ∈ ar.val, (sc.sec.val.getD x.val default).facet.val < rp.val.length)
    (ps : alloc.vec.Vec U32) (k : Usize) (hk : k.val ≤ ar.val.length)
    (hps : nv ps.val = ((ar.val.take k.val).map (·.val)).map fun x => (rp.val.getD (sc.sec.val.getD x default).facet.val NONE).val) :
    inc.unify_loop4_loop0 sc rp ar ps k ⦃ r => r.1 = sc ∧
      nv r.2.val = (ar.val.map (·.val)).map fun x => (rp.val.getD (sc.sec.val.getD x default).facet.val NONE).val ⦄ := by
  unfold inc.unify_loop4_loop0
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec U32 × Usize) => ar.val.length - st.2.val)
    (fun st => st.2.val ≤ ar.val.length ∧
      nv st.1.val = ((ar.val.take st.2.val).map (·.val)).map fun x => (rp.val.getD (sc.sec.val.getD x default).facet.val NONE).val)
  · rintro ⟨c, j⟩ ⟨hj, hc⟩
    unfold inc.unify_loop4_loop0.body
    simp only at hj hc ⊢
    split
    · rename_i hlt
      have hjl : j.val < ar.val.length := by scalar_tac
      have hcl : c.val.length = j.val := by
        have := congrArg List.length hc; simp [nv] at this; omega
      have hxm := List.getElem_mem hjl
      have hxs := har _ hxm
      have hxf := hfr _ hxm
      step as ⟨x, hx⟩
      step as ⟨sv, hsv⟩
      have hsv' : sv = sc.sec.val.getD (ar.val[j.val]).val default := by
        simp [hsv, hx, List.getD_eq_getElem?_getD, hxs]
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = sv.facet.val := by rw [hi2]; scalar_tac
      step as ⟨i3, hi3⟩
      have hcl2 : c.val.length < Usize.max := by scalar_tac
      step as ⟨c1, hc1⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by omega
      refine ⟨by omega, ?_, by omega⟩
      rw [hj1v, List.take_succ_eq_append_getElem hjl, List.map_append, List.map_append, ← hc, hc1]
      simp only [nv, List.map_append, List.map_cons, List.map_nil]
      congr 2
      have hi2l : i2.val < rp.val.length := by rw [hi2v, hsv']; exact hxf
      rw [hi3, ← hsv', List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by rw [← hi2v]; exact hi2l)]
      simp [hi2v]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = ar.val.length := by scalar_tac
      rw [this, List.take_length] at hc; exact ⟨by simp, hc⟩
  · exact ⟨hk, hps⟩

theorem mem_avL_lt (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) {v x : Nat} (hv : v < nrowsL l.vert_edges)
    (hx : x ∈ avL l a (sstAt l v) (sstAt l (v + 1))) :
    x < l.vert_sectors.sec.val.length ∧ (secAt l x).facet.val < nfacets l ∧ rpA a (secAt l x).facet.val ≠ U32.max := by
  have hsr := hw.sectors_rows
  have hvl : v + 1 < l.vert_sectors.start.val.length := by unfold nrowsL at hv; omega
  have hlen := sec_le_len l hw hvl
  have hm := mem_avL hx
  have hrow := secRowL_eq l hlen
  refine ⟨by omega, ?_, ?_⟩
  · have hmem : secAt l x ∈ secRowL l.vert_sectors v := by
      rw [hrow]; exact List.mem_map.mpr ⟨x, by simp [List.mem_range'_1]; omega, rfl⟩
    exact (hw.sectors v hv _ hmem).1
  · simp only [avL, List.mem_filter, bne_iff_ne, ne_eq, decide_eq_true_eq] at hx
    exact hx.2

theorem verts_spec (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) (hrl : a.rpos.val.length = nfacets l)
    (head_of : alloc.vec.Vec U32)
    (hpsn : ∀ v, v < nrowsL l.vert_edges → ∀ p ∈ PSf l a v, p < head_of.val.length)
    (nv' : Usize) (hnv : nv'.val = nrowsL l.vert_edges)
    (io : alloc.vec.Vec Bool) (out : alloc.vec.Vec VertId) (k : Usize) (hk : k.val ≤ nv'.val)
    (hio : io.val.length = nv'.val)
    (hout : out.val.map (fun (c : VertId) => (c : U32).val) =
      (List.range k.val).filter fun i => 3 ≤ (AtVf l a head_of.val i).length)
    (hiov : ∀ i, i < k.val → io.val.getD i false = decide (3 ≤ (AtVf l a head_of.val i).length))
    (hiof : ∀ i, k.val ≤ i → io.val.getD i false = false) :
    inc.unify_loop4 l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids l.vert_sectors
      a.result a.rpos a.orient nv' head_of io out k ⦃ r =>
      r.2.val.map (fun (c : VertId) => (c : U32).val) =
        (List.range nv'.val).filter (fun i => 3 ≤ (AtVf l a head_of.val i).length) ∧
      r.1.val.length = nv'.val ∧
      ∀ i, i < nv'.val → r.1.val.getD i false = decide (3 ≤ (AtVf l a head_of.val i).length) ⦄ := by
  have hsm := hw.small
  unfold inc.unify_loop4
  apply loop.spec_decr_nat
    (fun (st : SecCsr × alloc.vec.Vec Bool × alloc.vec.Vec VertId × Usize) => nv'.val - st.2.2.2.val)
    (fun st => st.1 = l.vert_sectors ∧ st.2.2.2.val ≤ nv'.val ∧ st.2.1.val.length = nv'.val ∧
      st.2.2.1.val.map (fun (c : VertId) => (c : U32).val) =
        ((List.range st.2.2.2.val).filter fun i => 3 ≤ (AtVf l a head_of.val i).length) ∧
      (∀ i, i < st.2.2.2.val → st.2.1.val.getD i false = decide (3 ≤ (AtVf l a head_of.val i).length)) ∧
      ∀ i, st.2.2.2.val ≤ i → st.2.1.val.getD i false = false)
  · rintro ⟨sc0, b, o, j⟩ ⟨hsc0, hj, hbl, ho, hb, hf⟩
    subst hsc0
    unfold inc.unify_loop4.body
    simp only at hj hbl ho hb hf ⊢
    split
    · rename_i hlt
      have hjl : j.val < nrowsL l.vert_edges := by scalar_tac
      have hol : o.val.length ≤ j.val := by
        have := congrArg List.length ho; simp only [List.length_map] at this; rw [this]
        exact u_filter_range_len _ _
      step with around_v_spec l hw a (by omega) j hjl as ⟨ar, har⟩
      have harm : ∀ x ∈ ar.val, x.val ∈ avL l a (sstAt l j.val) (sstAt l (j.val + 1)) := by
        intro x hx; rw [← har]; exact List.mem_map_of_mem hx
      step with ps_spec l.vert_sectors a.rpos ar (fun x hx => (mem_avL_lt l hw a hjl (harm x hx)).1)
        (fun x hx => by rw [hrl]; exact (mem_avL_lt l hw a hjl (harm x hx)).2.1)
        (alloc.vec.Vec.with_capacity U32 (alloc.vec.Vec.len ar)) 0#usize (by simp)
        (by simp [nv, alloc.vec.Vec.with_capacity]) as ⟨sc1, ps, hsc1, hps⟩
      have hpsv : nv ps.val = PSf l a j.val := by
        rw [hps, PSf, ← har]; rfl
      step with faces_at_spec (alloc.vec.Vec.deref head_of) (alloc.vec.Vec.deref ps)
        (fun y hy => by
          have hy' : y ∈ ps.val := by simpa [alloc.vec.Vec.deref] using hy
          have : y.val ∈ PSf l a j.val := by rw [← hpsv]; exact List.mem_map_of_mem hy'
          simpa [alloc.vec.Vec.deref] using hpsn j.val hjl _ this)
        as ⟨w, hw'⟩
      have hwv : nv w.val = AtVf l a head_of.val j.val := by
        rw [hw']; simp [AtVf, alloc.vec.Vec.deref, ← hpsv]
      have hwl : w.val.length = (AtVf l a head_of.val j.val).length := by
        rw [← hwv]; simp [nv]
      have hfilt := u_filter_range_succ (fun i => decide (3 ≤ (AtVf l a head_of.val i).length)) j.val
      split
      · rename_i h3
        have h3' : 3 ≤ (AtVf l a head_of.val j.val).length := by rw [← hwl]; scalar_tac
        step as ⟨w0, hw0, hback0⟩
        obtain ⟨_, back0⟩ := w0
        try simp only at hback0 ⊢
        subst hback0
        step as ⟨ci, hci⟩
        have hol2 : o.val.length < Usize.max := by have := u32_le_usize; omega
        step as ⟨o1, ho1⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨hsc1, by omega, by simp [hbl], ?_, ?_, ?_, by omega⟩
        · rw [hj1v, hfilt, if_pos (by simpa using h3'), ho1, List.map_append, ho]
          have : ci.val = j.val := by rw [hci]; scalar_tac
          simp [this]
        · intro i hi
          rw [hj1v] at hi
          simp only [alloc.vec.Vec.set_val_eq]
          by_cases e : i = j.val
          · subst e
            rw [decide_eq_true h3', List.getD_eq_getElem?_getD, List.getElem?_set_self (by omega)]; rfl
          · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm e), ← List.getD_eq_getElem?_getD]
            exact hb i (by omega)
        · intro i hi
          rw [hj1v] at hi
          simp only [alloc.vec.Vec.set_val_eq]
          rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (by omega), ← List.getD_eq_getElem?_getD]
          exact hf i (by omega)
      · rename_i h3
        have h3' : ¬ 3 ≤ (AtVf l a head_of.val j.val).length := by rw [← hwl]; scalar_tac
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨hsc1, by omega, hbl, ?_, ?_, fun i hi => hf i (by omega), by omega⟩
        · rw [hj1v, hfilt, if_neg (by simpa using h3'), List.append_nil]; exact ho
        · intro i hi
          rw [hj1v] at hi
          by_cases e : i = j.val
          · subst e
            rw [decide_eq_false h3']
            exact hf _ (le_refl _)
          · exact hb i (by omega)
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nv'.val := by scalar_tac
      rw [this] at ho hb
      exact ⟨ho, hbl, hb⟩
  · exact ⟨rfl, hk, hio, hout, hiov, hiof⟩

/-- The edges with two faces among the first `k`. -/
def ECSf (A : Nat → List Nat) (k : Nat) : List Nat := (List.range k).filter fun e => (A e).length = 2

theorem ecs_spec (ne : Usize) (hne : ne.val < U32.max) (A : Nat → List Nat)
    (at_e : alloc.vec.Vec (alloc.vec.Vec U32))
    (hate : at_e.val.map (fun v => nv v.val) = (List.range ne.val).map A)
    (ecs ec_pos : alloc.vec.Vec U32) (e : Usize) (he : e.val ≤ ne.val)
    (hecs : nv ecs.val = ECSf A e.val) (hepl : ec_pos.val.length = ne.val)
    (hep : ∀ k, k < ne.val → (ec_pos.val.getD k NONE).val =
      if k < e.val ∧ (A k).length = 2 then (ECSf A k).length else U32.max) :
    inc.unify_loop5 ne at_e ecs ec_pos e ⦃ r => nv r.1.val = ECSf A ne.val ∧ r.2.val.length = ne.val ∧
      ∀ k, k < ne.val → (r.2.val.getD k NONE).val = if (A k).length = 2 then (ECSf A k).length else U32.max ⦄ := by
  have hatl : at_e.val.length = ne.val := by
    have := congrArg List.length hate; simpa using this
  unfold inc.unify_loop5
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec U32 × alloc.vec.Vec U32 × Usize) => ne.val - st.2.2.val)
    (fun st => st.2.2.val ≤ ne.val ∧ nv st.1.val = ECSf A st.2.2.val ∧ st.2.1.val.length = ne.val ∧
      ∀ k, k < ne.val → (st.2.1.val.getD k NONE).val =
        if k < st.2.2.val ∧ (A k).length = 2 then (ECSf A k).length else U32.max)
  · rintro ⟨c, p, j⟩ ⟨hj, hc, hpl, hp⟩
    unfold inc.unify_loop5.body
    simp only at hj hc hpl hp ⊢
    split
    · rename_i hlt
      have hjl : j.val < ne.val := by scalar_tac
      have hcl : c.val.length ≤ j.val := by
        have := congrArg List.length hc; simp only [nv, List.length_map] at this; rw [this]
        exact u_filter_range_len _ _
      have hcl' : c.val.length = (ECSf A j.val).length := by
        have := congrArg List.length hc; simpa [nv] using this
      step as ⟨v, hv⟩
      have hvA : (A j.val).length = v.val.length := by
        have h1 := congrArg (fun L => L[j.val]?) hate
        simp only [List.getElem?_map] at h1
        rw [List.getElem?_eq_getElem (by omega), List.getElem?_range hjl] at h1
        simp only [Option.map_some, Option.some.injEq] at h1
        rw [← h1, ← hv]; simp [nv]
      have hfilt : ECSf A (j.val + 1) = ECSf A j.val ++ (if (A j.val).length = 2 then [j.val] else []) := by
        unfold ECSf; rw [u_filter_range_succ]; simp
      split
      · rename_i h2
        have h2' : (A j.val).length = 2 := by rw [hvA]; scalar_tac
        step as ⟨w0, hw0, hback0⟩
        obtain ⟨_, back0⟩ := w0
        try simp only at hback0 ⊢
        subst hback0
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = c.val.length := by rw [hi2]; scalar_tac
        step as ⟨i3, hi3⟩
        have hi3v : i3.val = j.val := by rw [hi3]; scalar_tac
        have hcl2 : c.val.length < Usize.max := by have := u32_le_usize; omega
        step as ⟨c1, hc1⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨by omega, ?_, by simp [hpl], ?_, by omega⟩
        · rw [hj1v, hfilt, if_pos h2', hc1, ← hc]; simp [nv, hi3v]
        · intro k hk
          simp only [alloc.vec.Vec.set_val_eq]
          by_cases e : k = j.val
          · subst e
            rw [List.getD_eq_getElem?_getD, List.getElem?_set_self (by omega)]
            simp [hi2v, hcl', h2', hj1v]
          · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm e), ← List.getD_eq_getElem?_getD,
              hp k hk, hj1v]
            have : (k < j.val + 1) ↔ k < j.val := by omega
            simp only [this]
      · rename_i h2
        have h2' : ¬ (A j.val).length = 2 := by rw [hvA]; scalar_tac
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨by omega, ?_, hpl, ?_, by omega⟩
        · rw [hj1v, hfilt, if_neg h2', List.append_nil]; exact hc
        · intro k hk
          rw [hp k hk, hj1v]
          by_cases e : k = j.val
          · subst e; simp [h2']
          · have : (k < j.val + 1) ↔ k < j.val := by omega
            simp only [this]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = ne.val := by scalar_tac
      rw [this] at hc hp
      refine ⟨hc, hpl, fun k hk => ?_⟩
      rw [hp k hk]; simp [hk]
  · exact ⟨he, hecs, hepl, hep⟩

end inc

end forge_bool_core
