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

/-- The result facet at position `i`, as the Lean's covector. -/
def crF (l : Ledger) (ext : Ext) (a : Assembled) (i : Nat) : Cov := covE l ext (resAt a i)

theorem u_getD_mem' {L : List Nat} {f : Nat} (hf : f < L.length) : L.getD f 0 ∈ L := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hf]; exact List.getElem_mem _

theorem u_beq_dec {α : Type} [BEq α] [LawfulBEq α] [DecidableEq α] (x y : α) : (x == y) = decide (x = y) := by
  by_cases h : x = y <;> simp [h]

theorem u_contains_dec {α : Type} [BEq α] [LawfulBEq α] [DecidableEq α] (L : List α) (x : α) :
    L.contains x = decide (x ∈ L) := by
  by_cases h : x ∈ L <;> simp [h]

theorem u_mem_aroundL' {l : Ledger} {ext : Ext} {a : Assembled} {x p : Nat} :
    p ∈ aroundL l ext a (covAt l ext x) ↔ p < a.result.val.length ∧ Cov.le (covE l ext x) (crF l ext a p) = true := by
  simp [aroundL, covE, crF]

/-- The ledger's edges (or verts) as the Lean reads them, by index. -/
theorem u_toL_cells {l : Ledger} (ext : Ext) {vs : alloc.vec.Vec CellId}
    (hvs : ∀ v ∈ vs.val, (v : U32).val < l.cells.val.length) :
    vs.val.map (fun c => cellL l (c : U32).val) =
      (List.range vs.val.length).map fun k => covE l ext ((nvC vs.val).getD k 0) := by
  apply List.ext_getElem (by simp)
  intro i h1 h2
  simp only [List.getElem_map, List.getElem_range]
  rw [cellL_covAt l ext (hvs _ (List.getElem_mem _)), covE]
  have hi : i < vs.val.length := by simpa using h1
  simp [nvC, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi]

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
  -- the result by positions
  have hxs : (toL l).result (ktOf l keep.val) = (List.range a.result.val.length).map (crF l ext a) := by
    rw [← hres]
    apply List.ext_getElem (by simp)
    intro i h1 h2
    simp only [List.getElem_map, List.getElem_range, crF]
    rw [u_resAt_eq a (by simpa using h1)]
  have hinj : ∀ i j, i < a.result.val.length → j < a.result.val.length → crF l ext a i = crF l ext a j → i = j := by
    intro i j hi hj h
    have := covE_inj l ext hok (resAt_lt hW hi) (resAt_lt hW hj) h
    exact u_nodup_getD hW.nodup (by simpa using hi) (by simpa using hj) this
  have horient : ∀ i, i < a.result.val.length →
      ForgeExact.DecideOnce.Ledger.orient (ktOf l keep.val) (crF l ext a i) = toS (oR a i) := by
    intro i hi
    have h1 := congrArg (fun L => L[i]?) hor
    simp only [List.getElem?_map, hxs] at h1
    rw [List.getElem?_eq_getElem (by omega), List.getElem?_eq_getElem (by simpa using hi)] at h1
    simp only [Option.map_some, Option.some.injEq, List.getElem_map, List.getElem_range] at h1
    rw [← h1, u_oR_eq a (by omega)]
  have hzero : ∀ i, (crF l ext a i).zeroIdx = zR l ext a i := by
    intro i; simp [crF, covE, zR, ForgeExact.DecideOnce.Cov.zeroIdx, idxOf_map_toS]
  have hcells : ∀ v ∈ l.edges.val, (v : U32).val < l.cells.val.length := hAS.wf.edges
  have hLE : (toL l).edges = (List.range l.edges.val.length).map fun k => covE l ext ((nvC l.edges.val).getD k 0) :=
    u_toL_cells ext hAS.wf.edges
  have hLV : (toL l).verts = (List.range l.verts.val.length).map fun k => covE l ext (vAt l k) :=
    u_toL_cells ext hAS.wf.verts
  have heAt : ∀ k, k < l.edges.val.length → (nvC l.edges.val).getD k 0 < (allCells l ext).length := by
    intro k hk; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa [nvC] using hk)]
    simp only [nvC, List.getElem_map, Option.getD_some]; exact hev _ (List.getElem_mem _)
  have hvAt : ∀ k, k < l.verts.val.length → vAt l k < (allCells l ext).length := by
    intro k hk; rw [u_vAt_eq l hk]; exact hvv _ (List.getElem_mem _)
  -- the face join is the Lean's `joinRel`
  have hJsymm : ∀ x y, (toL l).joinRel (ktOf l keep.val) x y = (toL l).joinRel (ktOf l keep.val) y x := by
    intro x y
    unfold ForgeExact.DecideOnce.Ledger.joinRel
    rw [Bool.beq_comm (a := x.zeroIdx), Bool.beq_comm (a := ForgeExact.DecideOnce.Ledger.orient _ x)]
    congr 1; congr 1; funext e; rw [Bool.and_comm]
  have haeK : ∀ k, k < l.edges.val.length → ae.val.getD k default ∈ ae.val := by
    intro k hk; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; exact List.getElem_mem _
  have hJs : ∀ p ∈ EJ, p.1 < a.result.val.length ∧ p.2 < a.result.val.length ∧
      (toL l).joinRel (ktOf l keep.val) (crF l ext a p.1) (crF l ext a p.2) = true := by
    intro p hp
    obtain ⟨h1, h2, ⟨hz, ho⟩, x, hx, hm1, hm2⟩ := hsJ p hp
    refine ⟨h1, h2, ?_⟩
    obtain ⟨v, hv, rfl⟩ := List.mem_map.mp hx
    obtain ⟨k, hk, rfl⟩ := List.getElem_of_mem hv
    have hk' : k < l.edges.val.length := by omega
    have hA := haeA k hk'
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hk, Option.getD_some] at hA
    rw [hA] at hm1 hm2
    unfold ForgeExact.DecideOnce.Ledger.joinRel
    simp only [Bool.and_eq_true, beq_iff_eq, List.any_eq_true]
    refine ⟨⟨by rw [hzero, hzero, hz], by rw [horient _ h1, horient _ h2, ho]⟩,
      covE l ext ((nvC l.edges.val).getD k 0), ?_, (u_mem_aroundL'.mp hm1).2, (u_mem_aroundL'.mp hm2).2⟩
    rw [hLE]; exact List.mem_map.mpr ⟨k, List.mem_range.mpr hk', rfl⟩
  have hJc : ∀ i j, i < a.result.val.length → j < a.result.val.length →
      (toL l).joinRel (ktOf l keep.val) (crF l ext a i) (crF l ext a j) = true → Eqv EJ i j := by
    intro i j hi hj h
    unfold ForgeExact.DecideOnce.Ledger.joinRel at h
    simp only [Bool.and_eq_true, beq_iff_eq, List.any_eq_true] at h
    obtain ⟨⟨hz, ho⟩, e, he, hle1, hle2⟩ := h
    rw [hLE] at he
    obtain ⟨k, hk, rfl⟩ := List.mem_map.mp he
    have hk' := List.mem_range.mp hk
    have hA := haeA k hk'
    have hJC : JC l ext a i j := ⟨by rw [← hzero, ← hzero, hz], toS_inj (by rw [← horient _ hi, ← horient _ hj, ho])⟩
    refine hcJ _ (haeK k hk') i ?_ j ?_ hJC
    · rw [hA]; exact u_mem_aroundL'.mpr ⟨hi, hle1⟩
    · rw [hA]; exact u_mem_aroundL'.mpr ⟨hj, hle2⟩
  have hiffJ := fun i j hi hj => u_eqv_iff (crF l ext a) a.result.val.length _ hJsymm EJ hJs hJc hinj (i := i) (j := j) hi hj
  obtain ⟨hHeads, hFaceOf⟩ := u_cls_filter hlJ hiffJ hinj
  unfold bodyL ForgeExact.DecideOnce.Ledger.unify
  lift_lets
  intro hs fk key fa at_ outV ecsL erel
  have hhs : hs = (hdL (ufRoot (vals join1.parent)) a.result.val.length).map (crF l ext a) := by
    show (toL l).heads (ktOf l keep.val) = _
    unfold ForgeExact.DecideOnce.Ledger.heads; rw [hxs]
    have := hHeads; simp only [u_beq_dec] at this ⊢; exact this
  have hFO : ∀ t, t < a.result.val.length → ufRoot (vals join1.parent) t = t →
      (toL l).faceOf (ktOf l keep.val) (crF l ext a t) =
        ((List.range a.result.val.length).filter fun d => ufRoot (vals join1.parent) d = t).map (crF l ext a) := by
    intro t ht htt
    unfold ForgeExact.DecideOnce.Ledger.faceOf; rw [hxs]
    have := hFaceOf t ht; simp only [u_contains_dec] at this ⊢; rw [htt] at this
    convert this using 3
    exact decide_eq_decide.mpr Iff.rfl
  have hkey : ∀ p, p < a.result.val.length → key (crF l ext a p) = crF l ext a (ufRoot (vals join1.parent) p) := by
    intro p hp
    show (Option.map Prod.snd (List.find? (fun e => e.1 == crF l ext a p)
      (List.flatMap (fun h => List.map (fun c => (c, h)) ((toL l).faceOf (ktOf l keep.val) h)) hs))).getD _ = _
    rw [hhs]
    have := u_key hRt hinj _ hFO hp; simp only [u_beq_dec] at this ⊢; exact this
  -- the faces round a cell
  have hFAt : ∀ x, x < (allCells l ext).length → (toL l).facesAt (ktOf l keep.val) key (covE l ext x) =
      (dedup ((aroundL l ext a (covAt l ext x)).map fun p =>
        (hdL (ufRoot (vals join1.parent)) a.result.val.length).idxOf (ufRoot (vals join1.parent) p))).map
        fun k => crF l ext a ((hdL (ufRoot (vals join1.parent)) a.result.val.length).getD k 0) := by
    intro x hx
    unfold ForgeExact.DecideOnce.Ledger.facesAt
    rw [← hAB l e0 s ext keep.val a hAS hkeep hasm x hx, List.map_map]
    have hlt : ∀ p ∈ aroundL l ext a (covAt l ext x), p < a.result.val.length := fun p hp => (u_mem_aroundL'.mp hp).1
    have : (aroundL l ext a (covAt l ext x)).map (key ∘ fun p => covE l ext (resAt a p)) =
        (aroundL l ext a (covAt l ext x)).map fun p => crF l ext a (ufRoot (vals join1.parent) p) := by
      apply List.map_congr_left; intro p hp; exact hkey p (hlt p hp)
    rw [this]
    exact u_dedup_names hRt hinj hlt
  have hat : ∀ x ∈ (toL l).edges ++ (toL l).verts, at_ x = (toL l).facesAt (ktOf l keep.val) key x := by
    intro x hx
    show (Option.map Prod.snd (List.find? (fun e => e.1 == x) fa)).getD [] = _
    simp only [fa]; rw [find?_pair _ _ _ hx]; rfl
  have hFAr : ∀ (v : alloc.vec.Vec U32), (∀ y ∈ v.val, y.val < a.result.val.length) →
      FA face_of1.val v.val = dedup ((nv v.val).map fun p =>
        (hdL (ufRoot (vals join1.parent)) a.result.val.length).idxOf (ufRoot (vals join1.parent) p)) := by
    intro v hv
    unfold FA; congr 1
    apply List.map_congr_left; intro p hp
    have : ∃ y ∈ v.val, y.val = p := by simpa [nv] using hp
    obtain ⟨y, hy, rfl⟩ := this
    exact hfnum _ (hv y hy)
  have hatvG : ∀ i, i < l.verts.val.length →
      nv (atv.val.getD i default).val = FA face_of1.val (av.val.getD i default).val := by
    intro i hi
    have h1 := congrArg (fun L => L[i]?) hatv
    simp only [List.getElem?_map] at h1
    rw [List.getElem?_eq_getElem (by omega), List.getElem?_eq_getElem (by omega)] at h1
    simp only [Option.map_some, Option.some.injEq] at h1
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem (by omega)]
    exact h1
  have havK : ∀ k, k < l.verts.val.length → av.val.getD k default ∈ av.val := by
    intro k hk; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; exact List.getElem_mem _
  have hAtE : ∀ k, k < l.edges.val.length → at_ (covE l ext ((nvC l.edges.val).getD k 0)) =
      (nv (ate.val.getD k default).val).map
        fun f => crF l ext a ((hdL (ufRoot (vals join1.parent)) a.result.val.length).getD f 0) := by
    intro k hk
    rw [hat _ (List.mem_append_left _ (by rw [hLE]; exact List.mem_map.mpr ⟨k, List.mem_range.mpr hk, rfl⟩)),
      hFAt _ (heAt k hk), hateG k hk, hFAr _ (haen _ (haeK k hk)), haeA k hk]
  have hAtV : ∀ k, k < l.verts.val.length → at_ (covE l ext (vAt l k)) =
      (nv (atv.val.getD k default).val).map
        fun f => crF l ext a ((hdL (ufRoot (vals join1.parent)) a.result.val.length).getD f 0) := by
    intro k hk
    rw [hat _ (List.mem_append_right _ (by rw [hLV]; exact List.mem_map.mpr ⟨k, List.mem_range.mpr hk, rfl⟩)),
      hFAt _ (hvAt k hk), hatvG k hk, hFAr _ (havn _ (havK k hk)), havA k hk]
    rfl
  simp only [ForgeExact.DecideOnce.Body.mk.injEq]
  refine ⟨?_, ?_, ?_⟩
  · -- the faces
    rw [hhs, List.map_map]
    apply List.ext_getElem (by simp [hF.fl])
    intro f h1 h2
    have hf : f < faces.val.length := by simpa using h1
    have hfh : f < (hdL (ufRoot (vals join1.parent)) a.result.val.length).length := by rw [← hF.fl]; exact hf
    have hfo := hF.fc f hf
    have hg : (hdL (ufRoot (vals join1.parent)) a.result.val.length).getD f 0 =
        (hdL (ufRoot (vals join1.parent)) a.result.val.length)[f] := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hfh]; rfl
    rw [hg] at hfo
    obtain ⟨hhn, hhR⟩ := u_mem_hdL.mp (List.getElem_mem hfh)
    simp only [List.getElem_map, Function.comp]
    unfold faceL
    rw [ForgeExact.DecideOnce.OutFace.mk.injEq]
    refine ⟨?_, ?_, ?_, ?_⟩
    · have e1 : (faces.val[f]).facets.val.map (fun c => covE l ext (c : U32).val) =
          (nvC (faces.val[f]).facets.val).map (covE l ext) := by simp [nvC]
      rw [e1, hfo.facets, List.map_map, hFO _ hhn hhR]; rfl
    · rw [hfo.plane, hzero]
    · rw [hfo.orient, horient _ hhn]
    · rw [hfo.origin]; exact (u_ff hAS hasm hhn).2
  rotate_left
  · -- the vertices
    show _ = (toL l).verts.filter fun v => decide (3 ≤ (at_ v).length)
    rw [hLV, u_filter_range_map]
    have e1 : verts.val.map (fun c => covE l ext (c : U32).val) = (nvC verts.val).map (covE l ext) := by simp [nvC]
    rw [e1, hverts, List.map_map]
    congr 1
    apply List.filter_congr
    intro i hi
    have hi' := List.mem_range.mp hi
    rw [hAtV i hi']; simp [nv]
  -- the edges
  have hRtn : ∀ f, f < (hdL (ufRoot (vals join1.parent)) a.result.val.length).length →
      (hdL (ufRoot (vals join1.parent)) a.result.val.length).getD f 0 < a.result.val.length := by
    intro f hf
    have := u_getD_mem' hf
    exact (u_mem_hdL.mp this).1
  have hgInj : ∀ x y, x < (hdL (ufRoot (vals join1.parent)) a.result.val.length).length →
      y < (hdL (ufRoot (vals join1.parent)) a.result.val.length).length →
      crF l ext a ((hdL (ufRoot (vals join1.parent)) a.result.val.length).getD x 0) =
        crF l ext a ((hdL (ufRoot (vals join1.parent)) a.result.val.length).getD y 0) → x = y := by
    intro x y hx hy h
    exact u_nodup_getD (u_hdL_nodup _ _) hx hy (hinj _ _ (hRtn x hx) (hRtn y hy) h)
  have hecsL : ecsL = (List.range ecs.val.length).map fun x => covE l ext (ECI l ecs.val x) := by
    show (toL l).edges.filter (fun e => (at_ e).length == 2) = _
    rw [hLE, u_filter_range_map]
    have e1 : ((List.range l.edges.val.length).filter fun k =>
        (at_ (covE l ext ((nvC l.edges.val).getD k 0))).length == 2) = nvU ecs.val := by
      rw [hecs]; apply List.filter_congr; intro k hk
      rw [hAtE k (List.mem_range.mp hk)]; simp only [nv, List.length_map]; exact u_beq_dec _ _
    rw [e1]
    apply List.ext_getElem (by simp [nvU])
    intro x h1 h2
    simp only [List.getElem_map, List.getElem_range, ECI]
    congr 2
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa [nvU] using h2)]; simp [nvU]
  have hinj2 : ∀ x y, x < ecs.val.length → y < ecs.val.length →
      covE l ext (ECI l ecs.val x) = covE l ext (ECI l ecs.val y) → x = y := by
    intro x y hx hy h
    have := covE_inj l ext hok (hEI x hx).1 (hEI y hy).1 h
    rw [← (hEI x hx).2, ← (hEI y hy).2]; exact congrArg _ this
  have hecsLt : ∀ x, x < ecs.val.length → (nvU ecs.val).getD x 0 < l.edges.val.length := by
    intro x hx
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simp [nvU]; omega)]
    simp only [nvU, List.getElem_map, Option.getD_some]; exact hecsv _ (List.getElem_mem _)
  have hAt2 : ∀ x, x < ecs.val.length → at_ (covE l ext (ECI l ecs.val x)) =
      (FE ate.val ecs.val x).map
        fun f => crF l ext a ((hdL (ufRoot (vals join1.parent)) a.result.val.length).getD f 0) := by
    intro x hx; exact hAtE _ (hecsLt x hx)
  have hFEl : ∀ x, x < ecs.val.length → ∀ f ∈ FE ate.val ecs.val x,
      f < (hdL (ufRoot (vals join1.parent)) a.result.val.length).length := by
    intro x hx f hf; rw [← hheadsL]; exact hfs x hx f hf
  have hSS : ∀ x y, x < ecs.val.length → y < ecs.val.length →
      Ledger.sameSet (at_ (covE l ext (ECI l ecs.val x))) (at_ (covE l ext (ECI l ecs.val y))) =
        SS (FE ate.val ecs.val x) (FE ate.val ecs.val y) := by
    intro x y hx hy
    rw [hAt2 x hx, hAt2 y hy]
    apply u_sameSet_map
    intro u hu v hv huv
    have hu' : u < (hdL (ufRoot (vals join1.parent)) a.result.val.length).length := by
      rcases List.mem_append.mp hu with hu | hu
      · exact hFEl x hx u hu
      · exact hFEl y hy u hu
    have hv' : v < (hdL (ufRoot (vals join1.parent)) a.result.val.length).length := by
      rcases List.mem_append.mp hv with hv | hv
      · exact hFEl x hx v hv
      · exact hFEl y hy v hv
    exact hgInj u v hu' hv' huv
  have hErelSymm : ∀ x y, erel x y = erel y x := by
    intro x y
    show (Ledger.sameSet (at_ x) (at_ y) && _) = (Ledger.sameSet (at_ y) (at_ x) && _)
    unfold ForgeExact.DecideOnce.Ledger.sameSet
    rw [Bool.and_comm (List.all (at_ x) _)]
    congr 1; congr 1; funext v; rw [Bool.and_assoc, Bool.and_assoc, Bool.and_left_comm (Cov.le v x)]
  -- the outV test
  have hOut : ∀ k, k < l.verts.val.length →
      (outV.contains (covE l ext (vAt l k)) = false ↔ io.val.getD k false = false) := by
    intro k hk
    rw [hio k hk, u_contains_dec, decide_eq_false_iff_not, decide_eq_false_iff_not]
    show (covE l ext (vAt l k) ∉ (toL l).verts.filter fun v => decide (3 ≤ (at_ v).length)) ↔ _
    rw [List.mem_filter, hAtV k hk]
    simp only [nv, List.length_map, decide_eq_true_eq, not_and]
    have : covE l ext (vAt l k) ∈ (toL l).verts := by rw [hLV]; exact List.mem_map.mpr ⟨k, List.mem_range.mpr hk, rfl⟩
    exact ⟨fun h => h this, fun h _ => h⟩
  have hvMem : ∀ k, k < l.verts.val.length → covE l ext (vAt l k) ∈ (toL l).verts := by
    intro k hk; rw [hLV]; exact List.mem_map.mpr ⟨k, List.mem_range.mpr hk, rfl⟩
  have hEs : ∀ p ∈ E2, p.1 < ecs.val.length ∧ p.2 < ecs.val.length ∧
      erel (covE l ext (ECI l ecs.val p.1)) (covE l ext (ECI l ecs.val p.2)) = true := by
    intro p hp
    obtain ⟨h1, h2, hEC, k, hk, hiok, hle1, hle2⟩ := hs2 p hp
    refine ⟨h1, h2, ?_⟩
    show (Ledger.sameSet _ _ && _) = true
    rw [hSS _ _ h1 h2, Bool.and_eq_true]
    refine ⟨hEC, List.any_eq_true.mpr ⟨covE l ext (vAt l k), hvMem k hk, ?_⟩⟩
    simp only [Bool.and_eq_true, Bool.not_eq_true']
    exact ⟨⟨(leS_iff _ _).mpr hle1, (leS_iff _ _).mpr hle2⟩, (hOut k hk).mpr hiok⟩
  have hCv : ∀ k x, k < l.verts.val.length → x < ecs.val.length →
      Cov.le (covE l ext (vAt l k)) (covE l ext (ECI l ecs.val x)) = true →
      CvR l ext a isE.val av.val (ECI l ecs.val) k x := by
    intro k x hk hx hle
    -- the edge cell has a facet round it
    have h2 := hec2 x hx
    have hFE : FE ate.val ecs.val x = dedup ((aroundL l ext a (covAt l ext (ECI l ecs.val x))).map fun p =>
        (hdL (ufRoot (vals join1.parent)) a.result.val.length).idxOf (ufRoot (vals join1.parent) p)) := by
      unfold FE; rw [hateG _ (hecsLt x hx), hFAr _ (haen _ (haeK _ (hecsLt x hx))), haeA _ (hecsLt x hx)]; rfl
    have hne : FE ate.val ecs.val x ≠ [] := by
      intro e
      have h3 : (nv (ate.val.getD ((nvU ecs.val).getD x 0) default).val).length = 2 := by simp only [nv, List.length_map]; exact h2
      unfold FE at e; rw [e] at h3; simp at h3
    obtain ⟨p, hp⟩ : ∃ p, p ∈ aroundL l ext a (covAt l ext (ECI l ecs.val x)) := by
      rw [hFE] at hne
      cases hA : aroundL l ext a (covAt l ext (ECI l ecs.val x)) with
      | nil => rw [hA] at hne; simp [dedup, dedupAux] at hne
      | cons q t => exact ⟨q, by simp⟩
    obtain ⟨hpn, hple⟩ := u_mem_aroundL'.mp hp
    have hvp : p ∈ nv (av.val.getD k default).val := by
      rw [havA k hk]; exact u_mem_aroundL'.mpr ⟨hpn, ForgeExact.DecideOnce.Decisions.le_trans' hle hple⟩
    obtain ⟨j', hj', hjp⟩ := List.getElem_of_mem hvp
    refine ⟨j', by simpa [nv] using hj', ?_⟩
    have hjv : (nv (av.val.getD k default).val).getD j' 0 = p := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj', hjp]; rfl
    rw [hjv]
    have hEIx := (hEI x hx).1
    refine ⟨by rw [hNe]; exact hEIx, ?_, (leS_iff _ _).mp hle, (leS_iff _ _).mp hple⟩
    rw [hmk.2 _ (by rw [hmk.1]; exact hEIx)]
    simp only [decide_eq_true_eq]
    have hm := (u_ecs_facts l.edges.val ate.val hateL _ hsmall hev hend' ecs.val ep.val hecs hep).2.2.1 x hx
    show ECI' l.edges.val ecs.val x ∈ idsOf l.edges
    rw [hm]
    have hxl : x < (ecL l.edges.val ate.val l.edges.val.length).length := by
      have := congrArg List.length hecs; simp only [nvU, List.length_map] at this
      unfold ecL; rw [List.length_map, ← this]; exact hx
    obtain ⟨i, hi, e⟩ := u_ecL_sub (u_getD_mem' hxl)
    rw [← e]; exact u_getD_mem' (by simpa [nvC, idsOf] using hi)
  have hEc : ∀ x y, x < ecs.val.length → y < ecs.val.length →
      erel (covE l ext (ECI l ecs.val x)) (covE l ext (ECI l ecs.val y)) = true → Eqv E2 x y := by
    intro x y hx hy h
    have h' : (Ledger.sameSet _ _ && _) = true := h
    rw [hSS _ _ hx hy, Bool.and_eq_true] at h'
    obtain ⟨hEC, hany⟩ := h'
    obtain ⟨v, hv, hvv'⟩ := List.any_eq_true.mp hany
    simp only [Bool.and_eq_true, Bool.not_eq_true'] at hvv'
    obtain ⟨⟨hle1, hle2⟩, hnot⟩ := hvv'
    rw [hLV] at hv
    obtain ⟨k, hk, rfl⟩ := List.mem_map.mp hv
    have hk' := List.mem_range.mp hk
    exact hc2 k hk' ((hOut k hk').mp hnot) x y hx hy (hCv k x hk' hx hle1) (hCv k y hk' hy hle2) hEC
  have hiff2 := fun i j hi hj => u_eqv_iff (fun x => covE l ext (ECI l ecs.val x)) ecs.val.length erel hErelSymm E2
    hEs hEc hinj2 (i := i) (j := j) hi hj
  obtain ⟨hHeads2, hCls2⟩ := u_cls_filter hlE hiff2 hinj2
  rw [hecsL]
  have hH := hHeads2
  simp only [u_beq_dec] at hH ⊢
  rw [hH, List.map_map]
  apply List.ext_getElem (by simp [hEd.fl])
  intro f h1 h2
  have hf : f < edges.val.length := by simpa using h1
  have hfh : f < (hdL (ufRoot (vals erel1.parent)) ecs.val.length).length := by rw [← hEd.fl]; exact hf
  have heo := hEd.fc f hf
  have hg : (hdL (ufRoot (vals erel1.parent)) ecs.val.length).getD f 0 =
      (hdL (ufRoot (vals erel1.parent)) ecs.val.length)[f] := by
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hfh]; rfl
  rw [hg] at heo
  obtain ⟨hhn, hhR⟩ := u_mem_hdL.mp (List.getElem_mem hfh)
  simp only [List.getElem_map, Function.comp]
  unfold edgeL
  rw [ForgeExact.DecideOnce.OutEdge.mk.injEq]
  refine ⟨?_, ?_⟩
  · have e1 : (edges.val[f]).cells.val.map (fun c => covE l ext (c : U32).val) =
        (nvC (edges.val[f]).cells.val).map (covE l ext) := by simp [nvC]
    rw [e1, heo.cells, List.map_map]
    have := hCls2 _ hhn
    simp only [u_contains_dec] at this
    rw [hhR] at this
    convert this.symm using 3
    all_goals first | rfl | exact decide_eq_decide.mpr Iff.rfl | (rw [u_contains_dec]; exact decide_eq_decide.mpr Iff.rfl)
  · have e1 : (edges.val[f]).faces.val.map (fun c => covE l ext (c : U32).val) =
        (nvC (edges.val[f]).faces.val).map (covE l ext) := by simp [nvC]
    rw [e1, heo.faces, List.map_map, hAt2 _ hhn]
    apply List.map_congr_left
    intro u hu
    simp only [Function.comp]
    have hul := hFEl _ hhn u hu
    rw [hF.hds, u_getD_map (d := 0) hul]; rfl


end forge_bool_core
