import ForgeBoolCore.Specs

/-!
# Group R: the result table and `around`, from positions and ids to covectors

Pure Lean, no `step`. After `split` and `core_k`'s first loop (`AsmOf`):

* the result cells, read as covectors, are the Lean's `result` (`R_result_eq`), and the
  orientations are the Lean's `orient` (`R_orient_eq`);
* `aroundL` (positions in the result) read as covectors is the Lean's `around`
  (`around_bridge_proof : AroundBridge`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

open ForgeExact.DecideOnce ForgeBoolCore.Dedup

/-! ## Looking a side up in the keep table -/

theorem R_keepIn_zip : ∀ (ks : List Cov) (vs : List Bool) (T : Cov), T ∈ ks → ks.length ≤ vs.length →
    keepIn (ks.zip vs) T = vs.getD (ks.idxOf T) false
  | [], _, _, h, _ => nomatch h
  | _ :: _, [], _, _, hl => by simp at hl
  | k :: ks, v :: vs, T, hT, hl => by
    unfold keepIn
    simp only [List.zip_cons_cons, List.find?_cons, List.idxOf_cons]
    by_cases e : k = T
    · subst e; simp
    · have hT' : T ∈ ks := by
        rcases List.mem_cons.mp hT with h | h
        · exact absurd h.symm e
        · exact h
      have ih := R_keepIn_zip ks vs T hT' (by simp at hl; omega)
      unfold keepIn at ih
      have e1 : (k == T) = false := by simpa using e
      simp only [e1]; simp [ih]

/-! ## The facts `AfterSplit` gives -/

section
variable {l : Ledger} {e0 : Ext} {s : Split} {ext : Ext}

theorem R_hpc (h : AfterSplit l e0 s ext) : ∀ q ∈ pcs l, q.1 < l.cells.val.length :=
  fun q hq => (h.wf.pieces q hq).1

theorem R_facets_nodup (h : AfterSplit l e0 s ext) : (s.facets.val.map (·.val)).Nodup := by
  rw [h.split.facets]; exact nodup_dedup _

theorem R_facet_lt (h : AfterSplit l e0 s ext) : ∀ x ∈ s.facets.val, x.val < l.cells.val.length := by
  intro x hx
  have : x.val ∈ s.facets.val.map (·.val) := List.mem_map.mpr ⟨x, hx, rfl⟩
  rw [h.split.facets] at this
  obtain ⟨q, hq, e⟩ := List.mem_map.mp ((mem_dedup _ _).mp this)
  rw [← e]; exact R_hpc h q hq

theorem R_facets_len (h : AfterSplit l e0 s ext) : s.facets.val.length ≤ l.pieces.val.length := by
  have := congrArg List.length h.split.facets
  simp only [List.length_map] at this
  have h2 := dedup_length_le ((pcs l).map Prod.fst)
  rw [List.length_map] at h2
  have : (pcs l).length = l.pieces.val.length := by simp [pcs]
  omega

/-- A side's position in the Lean's topes is its `tope_pos`. -/
theorem R_tope (h : AfterSplit l e0 s ext) {c : Nat}
    (hc : c ∈ stream s.plus.val s.minus.val s.facets.val.length) :
    covE l ext c ∈ (toL l).topes ∧ ((toL l).topes).idxOf (covE l ext c) = at32 s.tope_pos c := by
  have hs := h.split
  have hT := hs.topes
  have hTL := topes_toL l e0 s ext h.ext0 hs (R_hpc h)
  have hcn := hT.2.2.1 c hc
  have hmap : s.topes.val.map (fun x => covE l ext x.val) = (s.topes.val.map (·.val)).map (covE l ext) := by
    simp [List.map_map]
  refine ⟨?_, ?_⟩
  · rw [← hTL, hmap, hT.2.1]
    exact List.mem_map.mpr ⟨c, (mem_dedup _ _).mpr hc, rfl⟩
  · rw [← hTL, hmap, idxOf_map_inj (covE l ext) _ c (fun x hx e => covE_inj l ext hs.ext
      (by rw [hT.2.1] at hx; exact hT.2.2.1 x ((mem_dedup _ _).mp hx)) hcn e), hT.2.1]
    have := hT.2.2.2 c hcn
    rw [if_pos hc] at this
    rw [at32_getD (by rw [hT.1]; exact hcn), ← this, List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem (by rw [hT.1]; exact hcn)]; rfl

/-- The facet at position `k`, as the Lean's. -/
def R_fk (l : Ledger) (s : Split) (k : Nat) : Cov := cellL l (at32 s.facets k)

/-- **Each facet's two sides keep as the keep vector says.** -/
theorem R_keep_side (h : AfterSplit l e0 s ext) {keep : List Bool} (hkl : keep.length = s.topes.val.length)
    {k : Nat} (hk : k < s.facets.val.length) :
    keepIn (ktOf l keep) (Cov.plus (R_fk l s k)) = keepAt keep (tpP s k) ∧
    keepIn (ktOf l keep) (Cov.minus (R_fk l s k)) = keepAt keep (tpM s k) := by
  have hs := h.split
  obtain ⟨_, _, _, _, hPl, hMl, hPM⟩ := hs.sides
  obtain ⟨_, h2, _, h4⟩ := hPM k hk
  have hc : (s.facets.val[k]'hk).val < l.cells.val.length := R_facet_lt h _ (List.getElem_mem _)
  rw [getElem!_u32 hk] at h2 h4
  have hfk : R_fk l s k = cellL l (s.facets.val[k]'hk).val := by rw [R_fk, at32_getD hk]
  have ep : Cov.plus (R_fk l s k) = covE l ext (at32 s.plus k) := by
    rw [hfk, at32_getD (by omega), covE, ← getElem!_u32 (show k < s.plus.val.length by omega), h2, side_plus,
      cellL_covAt l e0 hc]
  have em : Cov.minus (R_fk l s k) = covE l ext (at32 s.minus k) := by
    rw [hfk, at32_getD (by omega), covE, ← getElem!_u32 (show k < s.minus.val.length by omega), h4, side_minus,
      cellL_covAt l e0 hc]
  have hsm := stream_mem s.plus s.minus s.facets.val.length (by omega) (by omega) k hk
  have hTL := topes_toL l e0 s ext h.ext0 hs (R_hpc h)
  have hlen : (toL l).topes.length ≤ keep.length := by rw [← hTL, hkl]; simp
  obtain ⟨hp1, hp2⟩ := R_tope h hsm.1
  obtain ⟨hm1, hm2⟩ := R_tope h hsm.2
  refine ⟨?_, ?_⟩
  · rw [ep, ktOf, R_keepIn_zip _ _ _ hp1 hlen, hp2]; rfl
  · rw [em, ktOf, R_keepIn_zip _ _ _ hm1 hlen, hm2]; rfl

/-- The Lean's facets, by position. -/
theorem R_facets_range (h : AfterSplit l e0 s ext) :
    (toL l).facets = (List.range s.facets.val.length).map (R_fk l s) := by
  rw [← facets_toL l e0 h.ext0 (R_hpc h) s.facets.val h.split.facets]
  apply List.ext_getElem (by simp)
  intro i h1 h2
  simp only [List.getElem_map, List.getElem_range, R_fk]
  rw [at32_getD (by simpa using h1)]

/-- The Lean's result, by position. -/
theorem R_lean_result (h : AfterSplit l e0 s ext) {keep : List Bool} (hkl : keep.length = s.topes.val.length) :
    (toL l).result (ktOf l keep) = (resIdx s keep s.facets.val.length).map (R_fk l s) := by
  unfold ForgeExact.DecideOnce.Ledger.result resIdx
  rw [R_facets_range h, List.filter_map]
  congr 1
  apply List.filter_congr
  intro k hk
  rw [List.mem_range] at hk
  obtain ⟨e1, e2⟩ := R_keep_side h hkl hk
  simp only [Function.comp, e1, e2, inRes]

/-- The result cells, by id, are the result facets' ids. -/
theorem R_res_vals (h : AfterSplit l e0 s ext) {keep : List Bool} {a : Assembled} (hA : AsmOf l ext s keep a) :
    a.result.val.map (·.val) = resIds s keep s.facets.val.length := by
  rw [hA.1.2.1, dedup_of_nodup (resIds_nodup s keep (R_facets_nodup h) _ (Nat.le_refl _))]

/-- **The result cells read as covectors are the Lean's result.** -/
theorem R_result_eq (h : AfterSplit l e0 s ext) {keep : List Bool} (hkl : keep.length = s.topes.val.length)
    {a : Assembled} (hA : AsmOf l ext s keep a) :
    a.result.val.map (fun r => covE l ext r.val) = (toL l).result (ktOf l keep) := by
  have : a.result.val.map (fun r => covE l ext r.val) = (a.result.val.map (·.val)).map (covE l ext) := by
    simp [List.map_map]
  rw [this, R_res_vals h hA, R_lean_result h hkl, resIds, List.map_map]
  apply List.map_congr_left
  intro k hk
  simp only [resIdx, List.mem_filter, List.mem_range] at hk
  have hk' := hk.1
  have hc := R_facet_lt h _ (List.getElem_mem hk')
  simp only [Function.comp, R_fk]
  rw [at32_getD hk', covE, ← cellL_covAt l ext hc]

/-- **The orientations are the Lean's.** -/
theorem R_orient_eq (h : AfterSplit l e0 s ext) {keep : List Bool} (hkl : keep.length = s.topes.val.length)
    {a : Assembled} (hA : AsmOf l ext s keep a) :
    a.orient.val.map toS =
      ((toL l).result (ktOf l keep)).map (ForgeExact.DecideOnce.Ledger.orient (ktOf l keep)) := by
  rw [hA.2, R_lean_result h hkl, List.map_map, List.map_map]
  apply List.map_congr_left
  intro k hk
  simp only [resIdx, List.mem_filter, List.mem_range] at hk
  obtain ⟨_, e2⟩ := R_keep_side h hkl hk.1
  simp only [Function.comp, orientOf, ForgeExact.DecideOnce.Ledger.orient, e2]
  split <;> rfl

/-- The result read `around` needs: `ResOK` (every piece is a facet, by `splitOk`). -/
theorem R_resOK_of (h : AfterSplit l e0 s ext) {keep : List Bool} {a : Assembled} (hA : AsmOf l ext s keep a)
    (W : Nat) : ResOK l ext a W := by
  have hsok := h.ok
  have hRv := R_res_vals h hA
  have hT := hA.1
  have hd : dedup (resIds s keep s.facets.val.length) = a.result.val.map (·.val) := by
    rw [hRv, dedup_of_nodup (resIds_nodup s keep (R_facets_nodup h) _ (Nat.le_refl _))]
  -- a result id is a facet id
  have hmemF : ∀ r ∈ a.result.val, ∃ k, k < s.facets.val.length ∧ r.val = at32 s.facets k := by
    intro r hr
    have : r.val ∈ resIds s keep s.facets.val.length := by rw [← hRv]; exact List.mem_map_of_mem hr
    simp only [resIds, resIdx, List.mem_map, List.mem_filter, List.mem_range] at this
    obtain ⟨k, ⟨hk, _⟩, e⟩ := this
    exact ⟨k, hk, e.symm⟩
  have hcovr : ∀ r ∈ a.result.val, ∃ k, k < s.facets.val.length ∧
      (covAt l ext r.val).map toS = R_fk l s k := by
    intro r hr
    obtain ⟨k, hk, e⟩ := hmemF r hr
    refine ⟨k, hk, ?_⟩
    have hc := R_facet_lt h _ (List.getElem_mem hk)
    rw [R_fk, ← e, cellL_covAt l ext (by rw [e, at32_getD hk]; exact hc)]
  have hfkF : ∀ k, k < s.facets.val.length → R_fk l s k ∈ (toL l).facets := by
    intro k hk; rw [R_facets_range h]; exact List.mem_map.mpr ⟨k, List.mem_range.mpr hk, rfl⟩
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · rw [hRv]; exact resIds_nodup s keep (R_facets_nodup h) _ (Nat.le_refl _)
  · intro r hr
    have : r.val ∈ a.result.val.map (·.val) := List.mem_map_of_mem hr
    rw [← hd] at this
    exact hT.2.2.1 _ ((mem_dedup _ _).mp this)
  · have h1 : (a.result.val.map (·.val)).length ≤ s.facets.val.length := by
      rw [hRv]; unfold resIds resIdx; simp only [List.length_map]
      exact (List.length_filter_le _ _).trans (by simp)
    have := R_facets_len h
    have := h.wf.room
    simp only [List.length_map] at h1; omega
  · exact hT.1
  · intro c hc
    rw [hT.2.2.2 c hc, hd]
    have : c ∈ resIds s keep s.facets.val.length ↔ c ∈ a.result.val.map (·.val) := by
      rw [← hd, mem_dedup]
    by_cases hm : c ∈ a.result.val.map (·.val)
    · rw [if_pos (this.mpr hm), if_pos hm]
    · rw [if_neg (fun h' => hm (this.mp h')), if_neg hm]
  · intro r hr
    obtain ⟨k, hk, e⟩ := hcovr r hr
    obtain ⟨p, hp⟩ := ForgeExact.DecideOnce.Ledger.mem_facets.mp (hfkF k hk)
    unfold ForgeExact.DecideOnce.Ledger.splitOk at hsok
    have := List.all_eq_true.mp hsok _ hp
    simp only [Bool.and_eq_true, Cov.isFacet, beq_iff_eq] at this
    have hcnt := this.1.1.1
    rw [← e, show ForgeExact.DecideOnce.Sgn.zero = toS forge_bool_core.Sgn.Zero from rfl,
      List.count_map_of_injective _ _ (fun a b h => toS_inj h)] at hcnt
    exact hcnt

end

/-- **The result table is the Lean's result**, oriented, and `around` may read it. -/
theorem result_bridge_proof : ResultBridge := by
  intro l e0 s ext keep a h hkl hA
  exact ⟨R_result_eq h hkl hA, R_orient_eq h hkl hA, 0, R_resOK_of h hA 0⟩

/-- **`around` on positions, read as covectors, is the Lean's `around`.** -/
theorem around_bridge_proof : AroundBridge := by
  intro l e0 s ext keep a h hkl hA x _
  unfold ForgeExact.DecideOnce.Ledger.around
  rw [← R_result_eq h hkl hA]
  have : a.result.val.map (fun r => covE l ext r.val) =
      (List.range a.result.val.length).map (fun p => covE l ext (resAt a p)) := by
    apply List.ext_getElem (by simp)
    intro i h1 h2
    simp only [List.getElem_map, List.getElem_range, resAt]
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using h1)]
    simp
  rw [this, List.filter_map, aroundL]
  rfl

end forge_bool_core
