import ForgeBoolCore.RefineUnifyFaces

/-!
# Group U, part 5: the faces round each cell, the vertices, the edge cells

`at_e` / `at_v` are `faces_at` round each listed edge / vertex (`u_at_loop_spec`); a vertex is a
vertex cell with three or more faces (`u_verts_loop_spec`); the edge cells are the edges with two,
numbered in edge order, and `ec_pos` is that numbering by cell id (`u_ecs_loop_spec`).
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

/-- `faces_at fo v`, as numbers. -/
def FA (fo : List U32) (v : List U32) : List Nat := dedup ((nv v).map fun p => (nv fo).getD p 0)

/-- The values of a list of `usize`. -/
def nvU (xs : List Usize) : List Nat := xs.map (·.val)

theorem u_at_loop2_spec (ae : Slice (alloc.vec.Vec U32)) (fo : alloc.vec.Vec U32)
    (hin : ∀ x ∈ ae.val, ∀ y ∈ x.val, y.val < fo.val.length)
    (acc : alloc.vec.Vec (alloc.vec.Vec U32)) (k : Usize) (hk : k.val ≤ ae.val.length)
    (hacc : acc.val.map (fun v => nv v.val) = (ae.val.take k.val).map fun v => FA fo.val v.val) :
    unify_loop2 ae fo acc k ⦃ r => r.val.map (fun v => nv v.val) = ae.val.map fun v => FA fo.val v.val ⦄ := by
  unfold unify_loop2
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => ae.val.length - st.2.val)
    (fun st => st.2.val ≤ ae.val.length ∧
      st.1.val.map (fun v => nv v.val) = (ae.val.take st.2.val).map fun v => FA fo.val v.val)
  · rintro ⟨c, j⟩ ⟨hj, hc⟩
    unfold unify_loop2.body
    simp only at hj hc ⊢
    split
    · rename_i hlt
      have hjl : j.val < ae.val.length := by scalar_tac
      have hcl : c.val.length = j.val := by
        have := congrArg List.length hc; simp at this; omega
      step as ⟨v, hv⟩
      have hvm : v ∈ ae.val := by rw [hv]; exact List.getElem_mem _
      step with faces_at_spec (alloc.vec.Vec.deref fo) (alloc.vec.Vec.deref v)
        (fun y hy => by simpa [alloc.vec.Vec.deref] using hin v hvm y (by simpa [alloc.vec.Vec.deref] using hy))
        as ⟨w, hw⟩
      step as ⟨c1, hc1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [hc1, show j1.val = j.val + 1 by scalar_tac, List.take_succ_eq_append_getElem hjl, List.map_append,
        List.map_append, hc, ← hv]
      simp only [List.map_cons, List.map_nil]
      congr 2
      rw [hw]; simp [FA, alloc.vec.Vec.deref]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = ae.val.length := by scalar_tac
      rw [hc, this, List.take_length]
  · exact ⟨hk, hacc⟩

theorem u_at_loop3_spec (ae : Slice (alloc.vec.Vec U32)) (fo : alloc.vec.Vec U32)
    (hin : ∀ x ∈ ae.val, ∀ y ∈ x.val, y.val < fo.val.length)
    (acc : alloc.vec.Vec (alloc.vec.Vec U32)) (k : Usize) (hk : k.val ≤ ae.val.length)
    (hacc : acc.val.map (fun v => nv v.val) = (ae.val.take k.val).map fun v => FA fo.val v.val) :
    unify_loop3 ae fo acc k ⦃ r => r.val.map (fun v => nv v.val) = ae.val.map fun v => FA fo.val v.val ⦄ := by
  unfold unify_loop3
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec (alloc.vec.Vec U32) × Usize) => ae.val.length - st.2.val)
    (fun st => st.2.val ≤ ae.val.length ∧
      st.1.val.map (fun v => nv v.val) = (ae.val.take st.2.val).map fun v => FA fo.val v.val)
  · rintro ⟨c, j⟩ ⟨hj, hc⟩
    unfold unify_loop3.body
    simp only at hj hc ⊢
    split
    · rename_i hlt
      have hjl : j.val < ae.val.length := by scalar_tac
      have hcl : c.val.length = j.val := by
        have := congrArg List.length hc; simp at this; omega
      step as ⟨v, hv⟩
      have hvm : v ∈ ae.val := by rw [hv]; exact List.getElem_mem _
      step with faces_at_spec (alloc.vec.Vec.deref fo) (alloc.vec.Vec.deref v)
        (fun y hy => by simpa [alloc.vec.Vec.deref] using hin v hvm y (by simpa [alloc.vec.Vec.deref] using hy))
        as ⟨w, hw⟩
      step as ⟨c1, hc1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [hc1, show j1.val = j.val + 1 by scalar_tac, List.take_succ_eq_append_getElem hjl, List.map_append,
        List.map_append, hc, ← hv]
      simp only [List.map_cons, List.map_nil]
      congr 2
      rw [hw]; simp [FA, alloc.vec.Vec.deref]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = ae.val.length := by scalar_tac
      rw [hc, this, List.take_length]
  · exact ⟨hk, hacc⟩

/-- The vertices: the vertex cells with three or more faces round them. -/
theorem u_verts_loop_spec (vs : alloc.vec.Vec CellId) (atv : alloc.vec.Vec (alloc.vec.Vec U32))
    (hl : atv.val.length = vs.val.length)
    (out : alloc.vec.Vec CellId) (io : alloc.vec.Vec Bool) (k : Usize) (hk : k.val ≤ vs.val.length)
    (hio : io.val.length = vs.val.length)
    (hout : nvC out.val = ((List.range k.val).filter fun i => 3 ≤ (atv.val.getD i default).val.length).map
      fun i => (nvC vs.val).getD i 0)
    (hiov : ∀ i, i < k.val → io.val.getD i false = decide (3 ≤ (atv.val.getD i default).val.length))
    (hiof : ∀ i, k.val ≤ i → io.val.getD i false = false) :
    unify_loop4 vs atv out io k ⦃ r =>
      nvC r.1.val = ((List.range vs.val.length).filter fun i => 3 ≤ (atv.val.getD i default).val.length).map
        (fun i => (nvC vs.val).getD i 0) ∧ r.2.val.length = vs.val.length ∧
      ∀ i, i < vs.val.length → r.2.val.getD i false = decide (3 ≤ (atv.val.getD i default).val.length) ⦄ := by
  unfold unify_loop4
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec CellId × alloc.vec.Vec Bool × Usize) => vs.val.length - st.2.2.val)
    (fun st => st.2.2.val ≤ vs.val.length ∧ st.2.1.val.length = vs.val.length ∧
      nvC st.1.val = ((List.range st.2.2.val).filter fun i => 3 ≤ (atv.val.getD i default).val.length).map
        (fun i => (nvC vs.val).getD i 0) ∧
      (∀ i, i < st.2.2.val → st.2.1.val.getD i false = decide (3 ≤ (atv.val.getD i default).val.length)) ∧
      ∀ i, st.2.2.val ≤ i → st.2.1.val.getD i false = false)
  · rintro ⟨o, b, j⟩ ⟨hj, hbl, ho, hb, hf⟩
    unfold unify_loop4.body
    simp only at hj hbl ho hb hf ⊢
    split
    · rename_i hlt
      have hjl : j.val < vs.val.length := by scalar_tac
      have hol : o.val.length ≤ j.val := by
        have := congrArg List.length ho; simp only [nvC, List.length_map] at this; rw [this]
        exact u_filter_range_len _ _
      step as ⟨v1, hv1⟩
      have hv1d : atv.val.getD j.val default = v1 := by
        rw [hv1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
      have hfilt := u_filter_range_succ (fun i => decide (3 ≤ (atv.val.getD i default).val.length)) j.val
      split
      · rename_i h3
        have h3' : 3 ≤ v1.val.length := by scalar_tac
        step as ⟨ci, hci⟩
        step as ⟨o1, ho1⟩
        step as ⟨w0, hw0, hback0⟩
        obtain ⟨_, back0⟩ := w0
        simp only at hback0 ⊢
        subst hback0
        step as ⟨j1, hj1⟩
        refine ⟨by rw [hj1]; omega, by simp [hbl], ?_, ?_, ?_, by rw [hj1]; omega⟩
        · rw [hj1, hfilt, if_pos (by rw [hv1d]; simpa using h3'), List.map_append, ← ho, ho1]
          simp [nvC, hci, List.getD_eq_getElem?_getD, hjl]
        · intro i hi
          rw [hj1] at hi
          simp only [alloc.vec.Vec.set_val_eq]
          by_cases e : i = j.val
          · subst e
            rw [hv1d, decide_eq_true h3', List.getD_eq_getElem?_getD, List.getElem?_set_self (by omega)]; rfl
          · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm e), ← List.getD_eq_getElem?_getD]
            exact hb i (by omega)
        · intro i hi
          rw [hj1] at hi
          simp only [alloc.vec.Vec.set_val_eq]
          rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (by omega), ← List.getD_eq_getElem?_getD]
          exact hf i (by omega)
      · rename_i h3
        have h3' : ¬ 3 ≤ v1.val.length := by scalar_tac
        step as ⟨j1, hj1⟩
        refine ⟨by rw [hj1]; omega, hbl, ?_, ?_, fun i hi => hf i (by rw [hj1] at hi; omega), by rw [hj1]; omega⟩
        · rw [hj1, hfilt, if_neg (by rw [hv1d]; simpa using h3'), List.append_nil]; exact ho
        · intro i hi
          rw [hj1] at hi
          by_cases e : i = j.val
          · subst e
            rw [hv1d, decide_eq_false h3']
            exact hf _ (le_refl _)
          · exact hb i (by omega)
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = vs.val.length := by scalar_tac
      rw [this] at ho hb
      exact ⟨ho, hbl, hb⟩
  · exact ⟨hk, hio, hout, hiov, hiof⟩

/-- The edge cells (edges with two faces) among the first `k` edges, by cell id. -/
def ecL (es : List CellId) (ate : List (alloc.vec.Vec U32)) (k : Nat) : List Nat :=
  ((List.range k).filter fun i => (ate.getD i default).val.length = 2).map fun i => (nvC es).getD i 0

theorem u_ecL_succ (es : List CellId) (ate : List (alloc.vec.Vec U32)) (k : Nat) :
    ecL es ate (k + 1) = ecL es ate k ++
      (if (ate.getD k default).val.length = 2 then [(nvC es).getD k 0] else []) := by
  unfold ecL; rw [u_filter_range_succ, List.map_append]
  by_cases h : (ate.getD k default).val.length = 2
  · rw [if_pos (by simpa using h), if_pos h]; rfl
  · rw [if_neg (by simpa using h), if_neg h]; rfl

theorem u_ecL_sub {es : List CellId} {ate : List (alloc.vec.Vec U32)} {k c : Nat} (h : c ∈ ecL es ate k) :
    ∃ i, i < k ∧ (nvC es).getD i 0 = c := by
  unfold ecL at h; simp only [List.mem_map, List.mem_filter, List.mem_range] at h
  obtain ⟨i, ⟨hi, _⟩, e⟩ := h; exact ⟨i, hi, e⟩

theorem u_nodup_getD {L : List Nat} (hnd : L.Nodup) {i j : Nat} (hi : i < L.length) (hj : j < L.length)
    (h : L.getD i 0 = L.getD j 0) : i = j := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi, List.getD_eq_getElem?_getD,
    List.getElem?_eq_getElem hj] at h
  exact (List.Nodup.getElem_inj_iff hnd).mp h

/-- **The edge cells**, numbered in edge order, and their numbers by cell id. -/
theorem u_ecs_loop_spec (es : alloc.vec.Vec CellId) (ate : alloc.vec.Vec (alloc.vec.Vec U32))
    (hl : ate.val.length = es.val.length) (N : Nat) (hN : N < U32.max)
    (hes : ∀ e ∈ es.val, (e : U32).val < N) (hnd : (nvC es.val).Nodup)
    (ecs : alloc.vec.Vec Usize) (ep : alloc.vec.Vec U32) (k : Usize) (hk : k.val ≤ es.val.length)
    (hepl : ep.val.length = N)
    (hecs : nvU ecs.val = (List.range k.val).filter fun i => (ate.val.getD i default).val.length = 2)
    (hep : ∀ c, c < N → (ep.val.getD c NONE).val =
      if c ∈ ecL es.val ate.val k.val then (ecL es.val ate.val k.val).idxOf c else U32.max) :
    unify_loop5 es ate ecs ep k ⦃ r =>
      nvU r.1.val = ((List.range es.val.length).filter fun i => (ate.val.getD i default).val.length = 2) ∧
      r.2.val.length = N ∧
      ∀ c, c < N → (r.2.val.getD c NONE).val =
        if c ∈ ecL es.val ate.val es.val.length then (ecL es.val ate.val es.val.length).idxOf c else U32.max ⦄ := by
  have hesl : es.val.length ≤ N := by
    have : nvC es.val ⊆ List.range N := by
      intro x hx; simp only [nvC, List.mem_map] at hx; obtain ⟨e, he, rfl⟩ := hx
      exact List.mem_range.mpr (hes e he)
    have := List.Nodup.length_le_of_subset hnd this; simpa [nvC] using this
  have hnvl : (nvC es.val).length = es.val.length := by simp [nvC]
  unfold unify_loop5
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec Usize × alloc.vec.Vec U32 × Usize) => es.val.length - st.2.2.val)
    (fun st => st.2.2.val ≤ es.val.length ∧ st.2.1.val.length = N ∧
      nvU st.1.val = ((List.range st.2.2.val).filter fun i => (ate.val.getD i default).val.length = 2) ∧
      ∀ c, c < N → (st.2.1.val.getD c NONE).val =
        if c ∈ ecL es.val ate.val st.2.2.val then (ecL es.val ate.val st.2.2.val).idxOf c else U32.max)
  · rintro ⟨o, b, j⟩ ⟨hj, hbl, ho, hb⟩
    unfold unify_loop5.body
    simp only at hj hbl ho hb ⊢
    split
    · rename_i hlt
      have hjl : j.val < es.val.length := by scalar_tac
      have hol : o.val.length ≤ j.val := by
        have := congrArg List.length ho; simp only [nvU, List.length_map] at this; rw [this]
        exact u_filter_range_len _ _
      have hecl : (ecL es.val ate.val j.val).length = o.val.length := by
        have := congrArg List.length ho; simp only [nvU, List.length_map] at this; rw [this]; simp [ecL]
      step as ⟨v1, hv1⟩
      have hv1d : ate.val.getD j.val default = v1 := by
        rw [hv1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
      split
      · rename_i h2
        have h2' : v1.val.length = 2 := by scalar_tac
        step as ⟨ci, hci⟩
        step as ⟨i3, hi3⟩
        have hciN : (ci : U32).val < N := by rw [hci]; exact hes _ (List.getElem_mem _)
        have hi3v : i3.val = (ci : U32).val := by rw [hi3]; scalar_tac
        have hcid : (nvC es.val).getD j.val 0 = (ci : U32).val := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; simp [nvC, hci]
        step as ⟨w0, hw0, hback0⟩
        obtain ⟨_, back0⟩ := w0
        simp only at hback0 ⊢
        subst hback0
        step as ⟨i4, hi4⟩
        have hi4v : i4.val = o.val.length := by
          rw [hi4, UScalar.cast_val_eq]; apply Nat.mod_eq_of_lt; have := U32.max_eq; simp; omega
        step as ⟨o1, ho1⟩
        step as ⟨j1, hj1⟩
        have hnot : (ci : U32).val ∉ ecL es.val ate.val j.val := by
          intro hm
          obtain ⟨i, hi, e⟩ := u_ecL_sub hm
          rw [← hcid] at e
          have := u_nodup_getD hnd (by omega) (by omega) e; omega
        have hE : ecL es.val ate.val (j.val + 1) = ecL es.val ate.val j.val ++ [(ci : U32).val] := by
          rw [u_ecL_succ, if_pos (by rw [hv1d]; exact h2'), hcid]
        have hF : ((List.range (j.val + 1)).filter fun i => (ate.val.getD i default).val.length = 2) =
            ((List.range j.val).filter fun i => (ate.val.getD i default).val.length = 2) ++ [j.val] := by
          rw [u_filter_range_succ, if_pos (by rw [hv1d]; simpa using h2')]
        refine ⟨by rw [hj1]; omega, by simp [hbl], ?_, ?_, by rw [hj1]; omega⟩
        · rw [hj1, hF, ho1, ← ho]; simp [nvU]
        · intro c hc
          rw [hj1, hE]
          simp only [alloc.vec.Vec.set_val_eq]
          by_cases e : c = (ci : U32).val
          · subst e
            rw [List.getD_eq_getElem?_getD, ← hi3v, List.getElem?_set_self (by rw [hbl]; omega)]
            simp only [Option.getD_some, List.mem_append, List.mem_singleton, or_true, if_true, hi3v]
            rw [List.idxOf_append_of_notMem hnot]; simp [hi4v, hecl]
          · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (by rw [hi3v]; exact Ne.symm e),
              ← List.getD_eq_getElem?_getD, hb c hc]
            have hm : c ∈ ecL es.val ate.val j.val ++ [(ci : U32).val] ↔ c ∈ ecL es.val ate.val j.val := by
              simp [e]
            by_cases hcm : c ∈ ecL es.val ate.val j.val
            · rw [if_pos hcm, if_pos (hm.mpr hcm), List.idxOf_append_of_mem hcm]
            · rw [if_neg hcm, if_neg (fun h => hcm (hm.mp h))]
      · rename_i h2
        have h2' : ¬ v1.val.length = 2 := by scalar_tac
        step as ⟨j1, hj1⟩
        have hE : ecL es.val ate.val (j.val + 1) = ecL es.val ate.val j.val := by
          rw [u_ecL_succ, if_neg (by rw [hv1d]; exact h2'), List.append_nil]
        have hF : ((List.range (j.val + 1)).filter fun i => (ate.val.getD i default).val.length = 2) =
            ((List.range j.val).filter fun i => (ate.val.getD i default).val.length = 2) := by
          rw [u_filter_range_succ, if_neg (by rw [hv1d]; simpa using h2'), List.append_nil]
        refine ⟨by rw [hj1]; omega, hbl, ?_, ?_, by rw [hj1]; omega⟩
        · rw [hj1, hF]; exact ho
        · intro c hc
          rw [hj1, hE]; exact hb c hc
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = es.val.length := by scalar_tac
      rw [this] at ho hb
      exact ⟨ho, hbl, hb⟩
  · exact ⟨hk, hepl, hecs, hep⟩

end forge_bool_core
