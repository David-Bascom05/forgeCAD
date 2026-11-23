import ForgeBoolCore.RefineUnifyFacts

/-!
# Group U: `unify` refines `Ledger.unify`

The translated `unify`, past the touch rule, writes the Lean `unify`'s body (`unify_spec'`). It
assumes group R's `ResultBridge` and `AroundBridge`, and the two hypotheses `UnifySpec` lacks
(`UnifySpec'`): the ledger's edges are distinct, and the split is sound (`splitOk`, so a carrier
fits the `u32` plane).
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

/-- `UnifySpec` with the two hypotheses it needs: distinct edges, and a sound split. -/
def UnifySpec' : Prop :=
  ∀ (l : Ledger) (e0 : Ext) (s : Split) (ext : Ext) (keep : Slice Bool) (a : Assembled)
    (isE : Slice Bool) (ae av : Slice (alloc.vec.Vec U32)),
    AfterSplit l e0 s ext → keep.val.length = s.topes.val.length → AsmOf l ext s keep.val a →
    a.result.val ≠ [] →
    MarksEdges l ext isE.val →
    ae.val.map (fun v => v.val.map (·.val)) = (idsOf l.edges).map (fun e => aroundL l ext a (covAt l ext e)) →
    av.val.map (fun v => v.val.map (·.val)) = (idsOf l.verts).map (fun v => aroundL l ext a (covAt l ext v)) →
    (toL l).edges.find? ((toL l).badEdge (ktOf l keep.val)) = none →
    (toL l).verts.find? ((toL l).badVert (ktOf l keep.val)) = none →
    (idsOf l.edges).Nodup → (toL l).splitOk = true →
    unify l ext s a isE ae av ⦃ b => bodyL l ext b = (toL l).unify (ktOf l keep.val) ⦄

theorem u_around_mem {l : Ledger} {ext : Ext} {a : Assembled} {vs : alloc.vec.Vec CellId}
    {ae : Slice (alloc.vec.Vec U32)}
    (hae : ae.val.map (fun v => v.val.map (·.val)) = (idsOf vs).map (fun e => aroundL l ext a (covAt l ext e))) :
    ae.val.length = vs.val.length ∧
    (∀ x ∈ ae.val, ∀ y ∈ x.val, y.val < a.result.val.length) ∧
    ∀ k, k < vs.val.length → nv (ae.val.getD k default).val = aroundL l ext a (covAt l ext ((nvC vs.val).getD k 0)) := by
  have hl : ae.val.length = vs.val.length := by
    have := congrArg List.length hae; simpa [idsOf] using this
  refine ⟨hl, ?_, ?_⟩
  · intro x hx y hy
    have : nv x.val ∈ (idsOf vs).map (fun e => aroundL l ext a (covAt l ext e)) := by
      rw [← hae]; exact List.mem_map_of_mem hx
    obtain ⟨e, _, he⟩ := List.mem_map.mp this
    have hy' : y.val ∈ aroundL l ext a (covAt l ext e) := by rw [he]; exact List.mem_map_of_mem hy
    exact (u_mem_aroundL.mp hy').1
  · intro k hk
    have h1 := congrArg (fun L => L[k]?) hae
    simp only [List.getElem?_map] at h1
    rw [List.getElem?_eq_getElem (by omega), List.getElem?_eq_getElem (by simpa [idsOf] using hk)] at h1
    simp only [Option.map_some, Option.some.injEq] at h1
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some]
    rw [show nv (ae.val[k]'(by omega)).val = _ from h1]
    simp [idsOf, nvC, List.getD_eq_getElem?_getD, hk]

theorem unify_spec' (hRB : ResultBridge) (hAB : AroundBridge) : UnifySpec' := by
  intro l e0 s ext keep a isE ae av hAS hkeep hasm _ hmk hae hav _ _ hend hsok
  obtain ⟨hres, hor, W, hW⟩ := hRB l e0 s ext keep.val a hAS hkeep hasm
  have hok := hAS.split.ext
  have hsmall := hok.small
  have hn := hW.small
  have horl : a.orient.val.length = a.result.val.length := by
    have h1 := congrArg List.length hres; have h2 := congrArg List.length hor
    simp only [List.length_map] at h1 h2; omega
  obtain ⟨haeL, haen, haeA⟩ := u_around_mem hae
  obtain ⟨havL, havn, havA⟩ := u_around_mem hav
  have hNe : isE.val.length = (allCells l ext).length := hmk.1
  have hev : ∀ v ∈ l.edges.val, (v : U32).val < (allCells l ext).length := by
    intro v hv; rw [allCells_length]; have := hAS.wf.edges v hv; omega
  have hvv : ∀ v ∈ l.verts.val, (v : U32).val < (allCells l ext).length := by
    intro v hv; rw [allCells_length]; have := hAS.wf.verts v hv; omega
  unfold unify
  step with new_spec (alloc.vec.Vec.len a.result) (by simp; omega) as ⟨join, hjoin⟩
  have hu0 : UInv [] a.result.val.length join := by unfold UInv; rw [hjoin]; simpa using inv_range a.result.val.length
  step with u_join_spec l ext a W hW horl ae haen join [] 0#usize (by simp) hu0 (by simp) (fun k' h => by simp at h)
    as ⟨join1, EJ, huJ, hsJ, hcJ⟩
  -- the faces
  have hlJ := huJ.2.2
  have hRt := u_least_facts hlJ
  have hlenJ := uinv_len huJ
  step as ⟨fo, hfo⟩
  have hfoL : fo.val = List.replicate a.result.val.length NONE := by rw [hfo]; simp
  step with u_faces_loop_spec l ext a W hW horl s (ufRoot (vals join1.parent)) hRt
    (fun i hi => u_zR_le hAS hasm hsok hi) (fun i hi => (u_ff hAS hasm hi).1) (alloc.vec.Vec.len a.result) (by simp)
    join1 fo (alloc.vec.Vec.new OutFace) (alloc.vec.Vec.new U32) 0#usize (by simp)
    ⟨huJ.1, hlenJ, fun _ => rfl⟩
    ⟨by simp [hfoL], fun i hi => by simp at hi, by simp [nv, hdL], by simp [hdL], fun f hf => by simp at hf⟩
    as ⟨face_of1, faces, heads, hF⟩
  have hfoL1 := hF.fol
  -- the faces round each edge and vertex cell
  step with u_at_loop2_spec ae face_of1 (fun x hx y hy => by rw [hfoL1]; exact haen x hx y hy)
    (alloc.vec.Vec.new _) 0#usize (by simp) (by simp) as ⟨ate, hate⟩
  step with u_at_loop3_spec av face_of1 (fun x hx y hy => by rw [hfoL1]; exact havn x hx y hy)
    (alloc.vec.Vec.new _) 0#usize (by simp) (by simp) as ⟨atv, hatv⟩
  have hateL : ate.val.length = l.edges.val.length := by
    have := congrArg List.length hate; simp only [List.length_map] at this; omega
  have hatvL : atv.val.length = l.verts.val.length := by
    have := congrArg List.length hatv; simp only [List.length_map] at this; omega
  -- the vertices
  step as ⟨io0, hio0⟩
  step with u_verts_loop_spec l.verts atv hatvL (alloc.vec.Vec.new CellId) io0 0#usize (by simp)
    (by rw [hio0]; simp) (by simp [nvC]) (fun i hi => by simp at hi)
    (fun i _ => by rw [hio0]; simp [List.getD_eq_getElem?_getD, List.getElem?_replicate]; split <;> rfl)
    as ⟨verts, io, hverts, hioL, hio⟩
  -- the edge cells
  step with ncells_spec l ext (by have := u32_le_usize; omega) as ⟨nc, hnc⟩
  step as ⟨ep0, hep0⟩
  have hend' : (nvC l.edges.val).Nodup := hend
  step with u_ecs_loop_spec l.edges ate hateL (allCells l ext).length hsmall hev hend'
    (alloc.vec.Vec.new Usize) ep0 0#usize (by simp) (by rw [hep0]; simp [hnc]) (by simp [nvU])
    (fun c hc => by
      rw [hep0]; simp [ecL, List.getD_eq_getElem?_getD, List.getElem?_replicate, hnc, hc, none_val])
    as ⟨ecs, ep, hecs, hepL, hep⟩
  obtain ⟨hM, hecsv, hECI, hepE, hEI, hec2⟩ := u_ecs_facts l.edges.val ate.val hateL _ hsmall hev hend'
    ecs.val ep.val hecs hep
  -- the edge join
  step with new_spec (alloc.vec.Vec.len ecs) (by simp; omega) as ⟨erel, herel⟩
  have hu2 : UInv [] ecs.val.length erel := by unfold UInv; rw [herel]; simpa using inv_range ecs.val.length
  step with u_erel_spec l ext hok a W hW isE hNe hvv av havL (fun k hk => havA k hk) ate ecs
    (fun x hx => by rw [hateL]; exact hecsv x hx) hM io hioL ep hepL (ECI l ecs.val) hepE hEI
    erel [] 0#usize (by simp) hu2 (by simp) (fun k' h => by simp at h)
    as ⟨erel1, E2, hu2', hs2, hc2⟩
  -- the edges
  have hfnum : ∀ p, p < a.result.val.length →
      (nv face_of1.val).getD p 0 = (hdL (ufRoot (vals join1.parent)) a.result.val.length).idxOf (ufRoot (vals join1.parent) p) := by
    intro p hp
    rw [← hF.fo p hp, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simp [nv]; omega),
      List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]
    simp [nv]
  have hheadsL : heads.val.length = (hdL (ufRoot (vals join1.parent)) a.result.val.length).length := by
    have := congrArg List.length hF.hds; simpa [nv] using this
  have hFA : ∀ v ∈ ae.val, ∀ f ∈ FA face_of1.val v.val, f < heads.val.length := by
    intro v hv f hf
    simp only [FA, mem_dedup, List.mem_map] at hf
    obtain ⟨p, hp, rfl⟩ := hf
    have hpn : p < a.result.val.length := by
      have : ∃ y ∈ v.val, y.val = p := by simpa [nv] using hp
      obtain ⟨y, hy, rfl⟩ := this; exact haen v hv y hy
    rw [hfnum p hpn, hheadsL]
    exact u_idxOf_lt (u_mem_hdL.mpr ⟨(hRt p hpn).1, (hRt p hpn).2.2⟩)
  have hateG : ∀ i, i < l.edges.val.length →
      nv (ate.val.getD i default).val = FA face_of1.val (ae.val.getD i default).val := by
    intro i hi
    have h1 := congrArg (fun L => L[i]?) hate
    simp only [List.getElem?_map] at h1
    rw [List.getElem?_eq_getElem (by omega), List.getElem?_eq_getElem (by omega)] at h1
    simp only [Option.map_some, Option.some.injEq] at h1
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem (by omega)]
    exact h1
  have hfs : ∀ x, x < ecs.val.length → ∀ f ∈ FE ate.val ecs.val x, f < heads.val.length := by
    intro x hx f hf
    have hix : (nvU ecs.val).getD x 0 < l.edges.val.length := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simp [nvU]; omega)]
      simp only [nvU, List.getElem_map, Option.getD_some]; exact hecsv _ (List.getElem_mem _)
    unfold FE at hf; rw [hateG _ hix] at hf
    refine hFA _ ?_ f hf
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; exact List.getElem_mem _
  have hlE := hu2'.2.2
  have hRt2 := u_least_facts hlE
  have hlen2 := uinv_len hu2'
  step as ⟨eo0, heo0⟩
  step with u_edges_loop_spec l heads ate ecs hM (fun x hx => by rw [hateL]; exact hecsv x hx) hecsv hfs
    (ufRoot (vals erel1.parent)) hRt2 erel1 (alloc.vec.Vec.new OutEdge) eo0 0#usize (by simp)
    ⟨hu2'.1, hlen2, fun _ => rfl⟩
    ⟨by rw [heo0]; simp, fun i hi => by simp at hi, by simp [hdL], fun f hf => by simp at hf⟩
    as ⟨edges, eo, hEd⟩
  sorry

end forge_bool_core
