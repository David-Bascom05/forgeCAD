import ForgeBoolCore.RefineUnifyLean

/-!
# Group U, part 9: what `unify` is given

From `AfterSplit` and the assembled result (`AsmOf`): every result cell is a piece's cell
(`u_res_piece`), its carrier fits `u32` when the split is sound (`u_zR_le`), and its first face is
read in range and is the Lean's `origin` (`u_ff_lt`, `u_origin_eq`).
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

/-- The cell id of edge cell number `i` (`ECI` on the lists). -/
def ECI' (es : List CellId) (ecs : List Usize) (i : Nat) : Nat := (nvC es).getD ((nvU ecs).getD i 0) 0

theorem u_res_piece {l : Ledger} {e0 : Ext} {s : Split} {ext : Ext} {keep : List Bool} {a : Assembled}
    (hAS : AfterSplit l e0 s ext) (hasm : AsmOf l ext s keep a) {i : Nat} (hi : i < a.result.val.length) :
    resAt a i ∈ (pcs l).map Prod.fst := by
  have hmem := resAt_mem a hi
  have hTp := hasm.1.2.1
  rw [hTp, mem_dedup] at hmem
  simp only [resIds, List.mem_map, resIdx, List.mem_filter, List.mem_range] at hmem
  obtain ⟨k, ⟨hk, _⟩, e⟩ := hmem
  have hF := hAS.split.facets
  have : at32 s.facets k ∈ s.facets.val.map (·.val) := by
    rw [at32, vals, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hk)]
    simp only [Option.getD_some, List.getElem_map]
    exact List.mem_map_of_mem (List.getElem_mem _)
  rw [hF, mem_dedup] at this
  rw [← e]; exact this

theorem u_res_lt_cells {l : Ledger} {e0 : Ext} {s : Split} {ext : Ext} {keep : List Bool} {a : Assembled}
    (hAS : AfterSplit l e0 s ext) (hasm : AsmOf l ext s keep a) {i : Nat} (hi : i < a.result.val.length) :
    resAt a i < l.cells.val.length := by
  obtain ⟨q, hq, e⟩ := List.mem_map.mp (u_res_piece hAS hasm hi)
  rw [← e]; exact (hAS.wf.pieces q hq).1

theorem u_zR_le {l : Ledger} {e0 : Ext} {s : Split} {ext : Ext} {keep : List Bool} {a : Assembled}
    (hAS : AfterSplit l e0 s ext) (hasm : AsmOf l ext s keep a) (hok : (toL l).splitOk = true)
    {i : Nat} (hi : i < a.result.val.length) : zR l ext a i ≤ U32.max := by
  obtain ⟨q, hq, e⟩ := List.mem_map.mp (u_res_piece hAS hasm hi)
  have hc := (hAS.wf.pieces q hq).1
  unfold ForgeExact.DecideOnce.Ledger.splitOk at hok
  have hp : (cellL l q.1, q.2) ∈ (toL l).pieces := by
    simp only [toL]; exact List.mem_map.mpr ⟨q, hq, rfl⟩
  have := List.all_eq_true.mp hok _ hp
  simp only [Bool.and_eq_true, beq_iff_eq] at this
  have hz := this.1.1.2
  have hzr : zR l ext a i = (cellL l q.1).zeroIdx := by
    rw [zR, ← e, cellL_covAt l ext hc, ForgeExact.DecideOnce.Cov.zeroIdx, idxOf_map_toS]
  rw [hzr, hz]
  unfold ForgeExact.DecideOnce.Ledger.clsOf
  simp only [toL]
  rw [List.getD_eq_getElem?_getD]
  cases h : (l.face_class.val.map (·.val))[q.2]? with
  | none => simp
  | some v =>
    simp only [Option.getD_some]
    obtain ⟨w, _, rfl⟩ := List.mem_map.mp (List.mem_of_getElem? h)
    scalar_tac

theorem u_ff {l : Ledger} {e0 : Ext} {s : Split} {ext : Ext} {keep : List Bool} {a : Assembled}
    (hAS : AfterSplit l e0 s ext) (hasm : AsmOf l ext s keep a) {i : Nat} (hi : i < a.result.val.length) :
    resAt a i < s.first_face.val.length ∧
    origOf s a i = (toL l).origin (covE l ext (resAt a i)) := by
  have hpc := u_res_piece hAS hasm hi
  have hc := u_res_lt_cells hAS hasm hi
  have hc0 : resAt a i < (allCells l e0).length := by rw [allCells_length, hAS.extra0]; simpa using hc
  have hfirst := hAS.split.first _ hc0
  obtain ⟨f, hf⟩ : ∃ f, firstF (pcs l) (resAt a i) = some f := by
    cases h : firstF (pcs l) (resAt a i) with
    | none => exact absurd hpc ((firstF_none _ _).mp h)
    | some f => exact ⟨f, rfl⟩
  have hfl : f < U32.max := (hAS.wf.pieces _ (firstF_mem hf)).2
  rw [hf, Option.getD_some] at hfirst
  have hlt : resAt a i < s.first_face.val.length := by
    by_contra hge
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (by omega), Option.getD_none, none_val] at hfirst
    omega
  refine ⟨hlt, ?_⟩
  unfold origOf
  rw [hfirst, if_neg (by omega)]
  unfold ForgeExact.DecideOnce.Ledger.origin
  simp only [toL]
  rw [List.find?_map]
  have hcv : covE l ext (resAt a i) = cellL l (resAt a i) := by rw [covE, cellL_covAt l ext hc]
  rw [hcv]
  have hcongr : ((pcs l).find? ((fun p => p.1 == cellL l (resAt a i)) ∘ fun q => (cellL l q.1, q.2))) =
      (pcs l).find? (·.1 == resAt a i) := by
    apply find?_congr
    intro q hq
    simp only [Function.comp]
    apply Bool.eq_iff_iff.mpr
    simp only [beq_iff_eq]
    exact ⟨fun h => cellL_inj l ext hAS.split.ext (hAS.wf.pieces q hq).1 hc h, fun h => by rw [h]⟩
  rw [hcongr]
  unfold firstF at hf
  cases h : (pcs l).find? (·.1 == resAt a i) with
  | none => rw [h] at hf; simp at hf
  | some q => rw [h] at hf; simp at hf ⊢; exact hf.symm

theorem u_getD_map {α β : Type} {f : α → β} {L : List α} {x : Nat} {d : α} {d' : β} (hx : x < L.length) :
    (L.map f).getD x d' = f (L.getD x d) := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_eq_getElem hx, List.getD_eq_getElem?_getD,
    List.getElem?_eq_getElem hx]; rfl

/-- The edge cells' numbering, both ways. -/
theorem u_ecs_facts (es : List CellId) (ate : List (alloc.vec.Vec U32)) (hl : ate.length = es.length)
    (N : Nat) (hN : N < U32.max) (hes : ∀ e ∈ es, (e : U32).val < N) (hnd : (nvC es).Nodup)
    (ecs : List Usize) (ep : List U32)
    (hecs : nvU ecs = (List.range es.length).filter fun i => (ate.getD i default).val.length = 2)
    (hep : ∀ c, c < N → (ep.getD c NONE).val =
      if c ∈ ecL es ate es.length then (ecL es ate es.length).idxOf c else U32.max) :
    ecs.length < U32.max ∧ (∀ x ∈ ecs, x.val < es.length) ∧
    (∀ x, x < ecs.length → ECI' es ecs x = (ecL es ate es.length).getD x 0) ∧
    (∀ e, e < N → epAt ep e ≠ U32.max → epAt ep e < ecs.length ∧ ECI' es ecs (epAt ep e) = e) ∧
    (∀ x, x < ecs.length → ECI' es ecs x < N ∧ epAt ep (ECI' es ecs x) = x) ∧
    (∀ x, x < ecs.length → (ate.getD ((nvU ecs).getD x 0) default).val.length = 2) := by
  have hnvl : (nvC es).length = es.length := by simp [nvC]
  have hesl : es.length ≤ N := by
    have : nvC es ⊆ List.range N := by
      intro x hx; simp only [nvC, List.mem_map] at hx; obtain ⟨e, he, rfl⟩ := hx
      exact List.mem_range.mpr (hes e he)
    have := List.Nodup.length_le_of_subset hnd this; simpa [nvC] using this
  have hM : ecs.length = (ecL es ate es.length).length := by
    have := congrArg List.length hecs; simp only [nvU, List.length_map] at this; simp [ecL, this]
  have hMl : ecs.length ≤ es.length := by
    have := congrArg List.length hecs; simp only [nvU, List.length_map] at this; rw [this]
    exact u_filter_range_len _ _
  have hecsv : ∀ x ∈ ecs, x.val < es.length := by
    intro x hx
    have : x.val ∈ nvU ecs := List.mem_map_of_mem hx
    rw [hecs] at this; exact List.mem_range.mp (List.mem_filter.mp this).1
  have hget : ∀ x, x < ecs.length → (nvU ecs).getD x 0 =
      ((List.range es.length).filter fun i => (ate.getD i default).val.length = 2).getD x 0 := by
    intro x _; rw [hecs]
  have hE2 : ∀ x, x < ecs.length → ECI' es ecs x = (ecL es ate es.length).getD x 0 := by
    intro x hx
    unfold ECI' ecL
    rw [hget x hx]
    have hxl : x < ((List.range es.length).filter fun i => (ate.getD i default).val.length = 2).length := by
      have := congrArg List.length hecs; simp only [nvU, List.length_map] at this; omega
    rw [u_getD_map hxl]
  have hnd2 : (ecL es ate es.length).Nodup := by
    unfold ecL
    apply List.Nodup.map_on _ (List.nodup_range.filter _)
    intro i hi j hj h
    simp only [List.mem_filter, List.mem_range] at hi hj
    exact u_nodup_getD hnd (by omega) (by omega) h
  have hmemN : ∀ c ∈ ecL es ate es.length, c < N := by
    intro c hc
    obtain ⟨i, hi, rfl⟩ := u_ecL_sub hc
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]
    simp only [nvC, List.getElem_map, Option.getD_some]
    exact hes _ (List.getElem_mem _)
  refine ⟨by omega, hecsv, hE2, ?_, ?_, ?_⟩
  · intro e he hne
    unfold epAt at hne ⊢
    rw [hep e he] at hne ⊢
    by_cases hm : e ∈ ecL es ate es.length
    · rw [if_pos hm]
      refine ⟨by rw [hM]; exact u_idxOf_lt hm, ?_⟩
      rw [hE2 _ (by rw [hM]; exact u_idxOf_lt hm), u_getD_idxOf hm]
    · rw [if_neg hm] at hne; exact absurd rfl hne
  · intro x hx
    rw [hE2 x hx]
    have hxl : x < (ecL es ate es.length).length := by omega
    have hmx : (ecL es ate es.length).getD x 0 ∈ ecL es ate es.length := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hxl]; exact List.getElem_mem _
    refine ⟨hmemN _ hmx, ?_⟩
    unfold epAt
    rw [hep _ (hmemN _ hmx), if_pos hmx, u_idxOf_getD hnd2 hxl]
  · intro x hx
    rw [hget x hx]
    have hxl : x < ((List.range es.length).filter fun i => (ate.getD i default).val.length = 2).length := by
      have := congrArg List.length hecs; simp only [nvU, List.length_map] at this; omega
    have hy : ((List.range es.length).filter fun i => (ate.getD i default).val.length = 2).getD x 0 ∈
        ((List.range es.length).filter fun i => (ate.getD i default).val.length = 2) := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hxl]; exact List.getElem_mem _
    simpa using (List.mem_filter.mp hy).2

end forge_bool_core
