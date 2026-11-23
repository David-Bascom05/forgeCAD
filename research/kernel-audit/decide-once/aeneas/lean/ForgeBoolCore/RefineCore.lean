import ForgeBoolCore.Specs

/-!
# Group E: the glue from `core` down to the proven stages
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

theorem new_ext_spec_proof : NewExtSpec := by
  intro l hw
  unfold new_ext
  have hl : (alloc.vec.Vec.deref l.cells).val.length = l.cells.val.length := by simp [alloc.vec.Vec.deref]
  step with index_new_spec (alloc.vec.Vec.deref l.cells) (by rw [hl]; have := hw.room; omega)
    (by rw [hl]; have := hw.sort_room; omega) as ⟨ix, hix⟩
  refine ⟨⟨hix, ?_, ?_⟩, rfl⟩
  · simpa [allCells] using hw.cells_nodup
  · simp [allCells]; have := hw.room; omega

theorem find_map_ids {ids : List Nat} {f : Nat → Bool} {g : ForgeExact.DecideOnce.Cov → Bool}
    {m : Nat → ForgeExact.DecideOnce.Cov} (h : ∀ x ∈ ids, f x = g (m x)) :
    (ids.map m).find? g = (ids.find? f).map m := by
  induction ids with
  | nil => rfl
  | cons x t ih =>
    simp only [List.map_cons, List.find?_cons]
    rw [← h x (by simp)]
    cases f x
    · exact ih (fun y hy => h y (by simp [hy]))
    · rfl

theorem core_k_spec_of (hR : ResultBridge) (hE : BadEdgeBridge) (hV : BadVertBridge) (hU : UnifySpec) :
    CoreKSpec := by
  intro l e0 s ext keep hA hkl
  have hw := hA.wf
  have hs := hA.split
  have hok := hs.ext
  have hsmall := hok.small
  have hN : (allCells l ext).length ≤ Usize.max := by have := u32_le_usize; omega
  unfold core_k
  step with ncells_spec l ext hN as ⟨n, hn⟩
  step as ⟨RP, hRP, hRPl⟩
  have hpcs : ∀ q ∈ pcs l, q.1 < l.cells.val.length := fun q hq => (hw.pieces q hq).1
  have hFl : s.facets.val.length ≤ l.pieces.val.length := by
    have := congrArg List.length hs.facets; simp only [List.length_map] at this; rw [this]
    have := ForgeBoolCore.Dedup.dedup_length_le ((pcs l).map Prod.fst); simp [pcs] at this ⊢; omega
  have hroom : 2 * s.facets.val.length < U32.max := by have := hw.room; omega
  have hwf : SplitWF s 0 := splitWF_of l e0 s ext hs hroom hw.nb_pos
  have hnd : (s.facets.val.map (·.val)).Nodup := by rw [hs.facets]; exact ForgeExact.DecideOnce.nodup_dedup _
  have hFN : ∀ x ∈ s.facets.val, x.val < (allCells l ext).length := by
    intro x hx
    have : x.val ∈ s.facets.val.map (·.val) := List.mem_map_of_mem hx
    rw [hs.facets, ForgeExact.DecideOnce.mem_dedup] at this
    simp only [List.mem_map] at this
    obtain ⟨q, hq, e⟩ := this
    rw [← e, allCells_length]; have := hpcs q hq; omega
  have hkl' : s.topes.val.length ≤ keep.val.length := by omega
  have h0 : resIds s keep.val 0 = [] := by simp [resIds, resIdx]
  have h0' : resIdx s keep.val 0 = [] := by simp [resIdx]
  have hT0 : TInv RP.val (alloc.vec.Vec.new U32).val (resIds s keep.val 0) (allCells l ext).length := by
    rw [h0]
    refine ⟨by rw [hRP, List.length_replicate, hn], by simp [ForgeExact.DecideOnce.dedup, ForgeExact.DecideOnce.dedupAux],
      by simp, fun c hc => ?_⟩
    rw [hRP]; simp [List.getD_eq_getElem?_getD, hn, hc, none_val]
  step with keep_res_spec s keep (allCells l ext).length hwf hnd hFN (by omega) hkl' (alloc.vec.Vec.new U32)
    (alloc.vec.Vec.new Sgn) RP 0#usize (by simp) ⟨hT0, by simp [resIdx]⟩
    as ⟨f1, p1, m1, t1, tp1, ff1, po1, R, O, RPos, hf1, hp1, hm1, ht1, htp1, hff1, hpo1, hTI, hO⟩
  subst hf1 hp1 hm1 ht1 htp1 hff1 hpo1
  have hasm : AsmOf l ext s keep.val { result := R, orient := O, rpos := RPos } := ⟨hTI, hO⟩
  obtain ⟨hres, hor, W, hRok⟩ := hR l e0 s ext keep.val _ hA hkl hasm
  step with ncells_spec l ext hN as ⟨n2, hn2⟩
  step as ⟨E0, hE0, hE0l⟩
  step with is_edge_spec l E0 0#usize (allCells l ext).length (by simp) (by rw [hE0, List.length_replicate, hn2])
    (by intro c hc; rw [hE0]; simp [List.getD_eq_getElem?_getD, hn2, hc])
    as ⟨q1, q2, q3, q4, q5, q6, q7, q8, isE, e1, e2, e3, e4, e5, e6, e7, e8, hisl, hise⟩
  subst e1 e2 e3 e4 e5 e6 e7 e8
  have hmarks : MarksEdges l ext isE.val := ⟨hisl, fun c hc => by rw [hise c (by omega)]; rfl⟩
  have hEdges : ∀ x ∈ l.edges.val, (x : U32).val < (allCells l ext).length := by
    intro x hx; rw [allCells_length]; have := hw.edges x hx; omega
  have hVerts : ∀ x ∈ l.verts.val, (x : U32).val < (allCells l ext).length := by
    intro x hx; rw [allCells_length]; have := hw.verts x hx; omega
  step with edge_scan_spec l ext hok _ W hRok hEdges hw.lists.1 as ⟨bad, ae, hscan⟩
  step with vert_scan_spec l ext hok _ W hRok isE hisl hVerts hw.lists.2 bad as ⟨bad1, av, hv1, hv2⟩
  -- the Lean's edges and verts are the ids read as covectors
  have hcov : ∀ (v : alloc.vec.Vec CellId), (∀ x ∈ v.val, (x : U32).val < l.cells.val.length) →
      v.val.map (fun c => cellL l (c : U32).val) = (idsOf v).map (covE l ext) := by
    intro v hv
    simp only [idsOf, List.map_map]
    apply List.map_congr_left
    intro x hx; simp only [Function.comp]; rw [cellL_covAt l ext (hv x hx)]; rfl
  set kt := ktOf l keep.val
  set a : Assembled := { result := R, orient := O, rpos := RPos }
  have hfE : (toL l).edges.find? ((toL l).badEdge kt) = ((idsOf l.edges).find? (badE l ext a)).map (covE l ext) := by
    show (l.edges.val.map fun c => cellL l (c : U32).val).find? _ = _
    rw [hcov _ hw.edges]
    exact find_map_ids (fun x hx => hE l e0 s ext keep.val a hA hkl hasm x hx)
  have hfV : (toL l).verts.find? ((toL l).badVert kt) = ((idsOf l.verts).find? (badV l ext a isE.val)).map (covE l ext) := by
    show (l.verts.val.map fun c => cellL l (c : U32).val).find? _ = _
    rw [hcov _ hw.verts]
    exact find_map_ids (fun x hx => hV l e0 s ext keep.val a isE.val hA hkl hasm hmarks x hx)
  unfold ForgeExact.DecideOnce.Ledger.coreK
  by_cases hb : bad = NONE
  · obtain ⟨hEn, hae⟩ := hscan.1 hb
    rw [hfE, hEn]
    simp only [Option.map_none]
    have hSV := hv2 hb
    by_cases hb1 : bad1 = NONE
    · obtain ⟨hVn, hav⟩ := hSV.1 hb1
      rw [hfV, hVn]
      simp only [Option.map_none]
      split
      · rename_i h; exact absurd hb1 (by simpa using h)
      · step as ⟨emp, hemp⟩
        have hRe : (toL l).result kt = [] ↔ R.val = [] := by
          rw [← hres]; simp; rfl
        split
        · rename_i he
          simp only [WP.spec_ok]
          have : R.val = [] := by simpa [hemp] using he
          rw [if_pos (hRe.mpr this)]; rfl
        · rename_i he
          have hne : R.val ≠ [] := by simpa [hemp] using he
          rw [if_neg (fun h => hne (hRe.mp h))]
          step with hU l e0 s ext keep a (alloc.vec.Vec.deref isE) (alloc.vec.Vec.deref ae) (alloc.vec.Vec.deref av)
            hA hkl hasm hne (by simpa [alloc.vec.Vec.deref] using hmarks)
            (by simpa [alloc.vec.Vec.deref] using hae) (by simpa [alloc.vec.Vec.deref] using hav)
            (by rw [hfE, hEn]; rfl) (by rw [hfV, hVn]; rfl) as ⟨body, hbody⟩
          simp only [WP.spec_ok, outcomeL]
          rw [hbody]
    · have hVs := hSV.2 hb1
      rw [hfV, hVs]
      simp only [Option.map_some]
      split
      · simp only [WP.spec_ok]; rfl
      · rename_i h; exact absurd (UScalar.eq_of_val_eq (by simpa using h)) hb1
  · have hEs := hscan.2 hb
    have hb1 := hv1 hb
    subst hb1
    rw [hfE, hEs]
    simp only [Option.map_some]
    split
    · simp only [WP.spec_ok]; rfl
    · rename_i h; exact absurd (UScalar.eq_of_val_eq (by simpa using h)) hb

/-- The keep vector `keep_on` builds is the Lean keep table. -/
theorem keepTab_eq (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hok0 : ExtOK l e0) (hs : SplitOK l e0 s e)
    (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) (M : List (alloc.vec.Vec Bool))
    (hM : ∀ b, b < l.nb.val → ∀ t (ht : t < s.topes.val.length),
      (M.getD b default).val.getD t false =
        (toL l).memb b ((toL l).topes[t]'(by rw [← topes_toL l e0 s e hok0 hs hpc]; simpa using ht)))
    (r : Rule) :
    (toL l).keepTab (ruleL l.nb.val r) =
      ktOf l ((List.range s.topes.val.length).map fun t => applyL r (colOf M l.nb.val t)) := by
  have hlenT : (toL l).topes.length = s.topes.val.length := by
    rw [← topes_toL l e0 s e hok0 hs hpc]; simp
  unfold ForgeExact.DecideOnce.Ledger.keepTab ktOf
  apply List.ext_getElem (by simp [hlenT])
  intro i h1 h2
  simp only [List.getElem_map, List.getElem_zip, List.getElem_range]
  congr 1
  unfold ForgeExact.DecideOnce.Ledger.keep ruleL colOf
  congr 1
  apply List.map_congr_left
  intro b hb
  simp only [List.mem_range] at hb
  have hi : i < s.topes.val.length := by simp at h1; omega
  rw [hM b (by simpa [toL] using hb) i hi]
  simp [toL] at hb ⊢; simp [hb]

theorem keep_on_lemma (hK : CollapseSpec) (hC : CoreKSpec) (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext)
    (r : Rule) (memb : Slice (alloc.vec.Vec Bool)) (m : MetricLedger) (hA : AfterSplit l e0 s ext) (hm : MetricWF l m)
    (hml : memb.val.length = l.nb.val)
    (hmr : ∀ b, b < l.nb.val → (memb.val.getD b default).val.length = s.topes.val.length)
    (hmv : ∀ b, b < l.nb.val → ∀ t (ht : t < s.topes.val.length),
      (memb.val.getD b default).val.getD t false =
        (toL l).memb b ((toL l).topes[t]'(by
          rw [← topes_toL l e0 s ext hA.ext0 hA.split (fun q hq => (hA.wf.pieces q hq).1)]; simpa using ht))) :
    keep_on l ext s r memb m ⦃ o => outcomeL l ext o =
      match (toL l).collapse ((toL l).keepTab (ruleL l.nb.val r)) (thinL l m) (enclosedL l m) with
      | .refuse T => .refused (.zeroThickness T)
      | .ok kt' => (toL l).coreK kt' ⦄ := by
  have hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length := fun q hq => (hA.wf.pieces q hq).1
  unfold keep_on
  step as ⟨nb, hnb⟩
  have hnbv : nb.val = l.nb.val := by rw [hnb]; scalar_tac
  step with keep_loop_spec s r memb nb (by omega) (fun i hi => by rw [hmr i (by omega)])
    (alloc.vec.Vec.new Bool) 0#usize (by simp) (by simp) as ⟨f, p, mi, t, tp, ff, po, keep, hsplit, hkeep⟩
  rw [hsplit]
  have hkt := keepTab_eq l e0 s ext hA.ext0 hA.split hpc memb.val hmv r
  rw [hnbv] at hkeep
  rw [← hkeep] at hkt
  have hkl : keep.val.length = s.topes.val.length := by rw [hkeep]; simp
  simp only [alloc.vec.Vec.deref_mut, lift]
  step with hK l e0 s ext keep.slice m hA hm hkl as ⟨bad, keep2, hcol⟩
  rw [hkt]
  have hks : (keep.slice).val = keep.val := rfl
  rw [hks] at hcol
  cases hc : (toL l).collapse (ktOf l keep.val) (thinL l m) (enclosedL l m) with
  | refuse T =>
    rw [hc] at hcol
    obtain ⟨hbn, _, hbT⟩ := hcol
    rw [if_pos (by simpa using hbn)]
    simp only [WP.spec_ok, outcomeL, hbT]
  | ok kt' =>
    rw [hc] at hcol
    obtain ⟨hbn, hk2l, hk2⟩ := hcol
    rw [if_neg (by simp [hbn])]
    have : (alloc.vec.Vec.deref ({ slice := keep2 } : alloc.vec.Vec Bool)).val = keep2.val := by
      simp only [alloc.vec.Vec.deref, Slice.from_val]; rfl
    apply WP.spec_mono (hC l e0 s ext (alloc.vec.Vec.deref { slice := keep2 }) hA (by rw [this]; exact hk2l))
    intro o ho
    rw [ho]
    have : (alloc.vec.Vec.deref ({ slice := keep2 } : alloc.vec.Vec Bool)).val = keep2.val := by
      simp only [alloc.vec.Vec.deref, Slice.from_val]; rfl
    rw [this, hk2]

theorem classify_on_spec_of (hK : CollapseSpec) (hC : CoreKSpec) : ClassifyOnSpec := by
  intro l e0 s ext r m hA hm
  have hw := hA.wf
  have hs := hA.split
  have hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length := fun q hq => (hw.pieces q hq).1
  have hFl : s.facets.val.length ≤ l.pieces.val.length := by
    have := congrArg List.length hs.facets; simp only [List.length_map] at this; rw [this]
    have := ForgeBoolCore.Dedup.dedup_length_le ((pcs l).map Prod.fst); simp [pcs] at this ⊢; omega
  have hroom : 2 * s.facets.val.length < U32.max := by have := hw.room; omega
  unfold classify_on
  step with membership_spec l e0 s ext hA.ext0 hs hpc hw.inside hroom as ⟨memb, hml, hmr, hmv⟩
  step as ⟨nb, hnb⟩
  have hnbv : nb.val = l.nb.val := by rw [hnb]; scalar_tac
  step with classify_outer_spec s memb nb (splitWF_of l e0 s ext hs hroom hw.nb_pos)
    (fun _ b hb => splitWF_of l e0 s ext hs hroom (by omega))
    (fun k hk => by rw [hs.piece_row k hk]; omega) (by omega)
    (fun i hi => by rw [hmr i (by omega)]) true 0#usize (by simp) as ⟨s1, ok1, hs1, hok1⟩
  rw [hs1]
  have hck := ckAll_eq l e0 s ext hA.ext0 hs hpc memb.val hmv
  rw [hnbv, hck] at hok1
  simp only [Bool.true_and] at hok1
  split
  · rename_i hok
    rw [hok1] at hok
    rw [if_neg (by simp [hok])]
    exact keep_on_lemma hK hC l e0 s ext r (alloc.vec.Vec.deref memb) m hA hm
      (by simpa [alloc.vec.Vec.deref] using hml) (by simpa [alloc.vec.Vec.deref] using hmr)
      (by simpa [alloc.vec.Vec.deref] using hmv)
  · rename_i hok
    rw [hok1] at hok
    rw [if_pos (by simpa using hok)]
    simp only [WP.spec_ok]; rfl

theorem core_spec_of (hS : SplitOkSpec) (hCl : ClassifyOnSpec) : CoreSpec := by
  intro l r m hw hm
  unfold core
  step with new_ext_spec_proof l hw as ⟨e0, hok0, hx0⟩
  step with hS l e0 hw hok0 hx0 as ⟨b, hb⟩
  have hlen0 : (allCells l e0).length = l.cells.val.length := by rw [allCells_length, hx0]; simp
  unfold ForgeExact.DecideOnce.Ledger.coreM
  split
  · rename_i hbt
    rw [← hb, hbt]
    simp only [Bool.not_true, Bool.false_eq_true, ite_false]
    step with split_spec l e0 hok0 hw.pieces (by rw [hlen0]; exact hw.room) as ⟨s, ext, hsp⟩
    apply WP.spec_mono (hCl l e0 s ext r m ⟨hw, hok0, hx0, hsp⟩ hm)
    intro o ho
    exact ⟨ext, hsp.ext, ho⟩
  · rename_i hbf
    have : (toL l).splitOk = false := by rw [← hb]; simpa using hbf
    rw [this]
    simp only [WP.spec_ok, Bool.not_false, ite_true]
    exact ⟨e0, hok0, rfl⟩

end forge_bool_core
