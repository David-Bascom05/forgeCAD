import ForgeBoolCore.RefineIncUnifyLean

/-!
# Group U (inc): `inc::unify` refines `IncLedger.unify` (`unify_specI`)

On an assembled result that passed the touch rule, the translated `unify` writes the Lean
`unify`'s body. Its loops' specs (parts 1–5) give the faces by join root, the faces round each edge
and vert by head facet, the verts, the edges with two faces and the edge runs by `erel` root; the
Lean side (part 6) names the same things by `head`/`cls` over the result. So `UnifySpec` holds on
every well-formed ledger and sound split (`unify_specI`), with no hypotheses beyond `UnifySpec`'s,
and `IncCoreSpec` follows (`incCoreSpec_proof`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

theorem nv_getD {L : List U32} {p : Nat} (hp : p < L.length) : (nv L).getD p 0 = (L.getD p NONE).val := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa [nv] using hp),
    List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hp]; simp [nv]

theorem beq_dec' {α : Type} [BEq α] [LawfulBEq α] [DecidableEq α] (x y : α) : (x == y) = decide (x = y) := by
  by_cases h : x = y <;> simp [h]

theorem contains_dec' {α : Type} [BEq α] [LawfulBEq α] [DecidableEq α] (L : List α) (x : α) :
    L.contains x = decide (x ∈ L) := by
  by_cases h : x ∈ L <;> simp [h]

theorem facet_inj : ∀ a b : Nat, FCell.facet a = FCell.facet b → a = b := fun _ _ h => by cases h; rfl

theorem edge_inj : ∀ a b : Nat, FCell.edge a = FCell.edge b → a = b := fun _ _ h => by cases h; rfl

theorem unify_specI : ∀ l s, LedgerWF2 l → SplitOK l s → UnifySpec l s := by
  intro l s hw hs kv kt a hk hA _ _ _
  have h : UCtx l s kv kt a := ⟨hw, hs, hk, hA⟩
  have hsm := hw.small
  have hrl : a.rpos.val.length = nfacets l := hA.rpos_len
  have hn := h.n_lt
  have horl : a.orient.val.length = a.result.val.length := by rw [hA.ori, List.length_map, h.len]
  have hres : ∀ x ∈ a.result.val, x.val < l.facet_class.val.length := by
    intro x hx
    obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem hx
    have := (h.rAt_mem hi).1
    rw [rAt_eq hi] at this; exact this
  have hrn : ∀ e, e < nrowsL l.edge_facets → ∀ x ∈ aroundL a (rowL l.edge_facets e), x < a.result.val.length :=
    fun e _ => h.aroundL_lt (row_ids_lt l hw (e := e))
  unfold inc.unify
  step with rows_spec _ as ⟨ne, hne⟩
  step with rows_spec _ as ⟨nv', hnv⟩
  have hnev : ne.val = nrowsL l.edge_facets := by rw [hne]; simp [nrowsL, alloc.vec.Vec.deref]
  have hnvv : nv'.val = nrowsL l.vert_edges := by rw [hnv]; simp [nrowsL, alloc.vec.Vec.deref]
  have hner : nrowsL l.edge_facets < U32.max := hsm.2.2.2.2.2.2.1
  have hnvr : nrowsL l.vert_edges < U32.max := hsm.2.2.2.2.2.2.2.1
  -- the face join
  step with new_spec (alloc.vec.Vec.len a.result) (by simp; omega) as ⟨join, hjoin⟩
  have hu0 : UInv [] a.result.val.length join := by
    unfold UInv; rw [hjoin]; simpa using inv_range a.result.val.length
  step with join_spec l hw a hrl horl hres (by omega) hrn ne hnev join [] 0#usize (by simp) hu0 (by simp)
    (fun k' h => by simp at h) as ⟨join1, EJ, huJ, hsJ, hcJ⟩
  have hlJ := huJ.2.2
  have hRt : RtOK (ufRoot (vals join1.parent)) a.result.val.length := u_least_facts hlJ
  -- the faces
  step as ⟨fo, hfo⟩
  have hfoL : fo.val = List.replicate a.result.val.length NONE := by rw [hfo]; simp
  step with faces1_spec l.facet_class a.result a.orient horl hres hn (alloc.vec.Vec.len a.result) (by simp)
    (ufRoot (vals join1.parent)) hRt join1 fo (alloc.vec.Vec.new OutFace) fo 0#usize (by simp)
    ⟨huJ.1, uinv_len huJ, fun _ => rfl⟩
    ⟨by simp [hfoL], fun i hi => by simp at hi, by simp [hfoL], fun i hi => by simp at hi, by simp [hdL],
      fun k hk => by simp at hk⟩
    as ⟨face_of1, faces, head_of, hF1⟩
  step with faces2_spec l.facet_class a.result a.orient (alloc.vec.Vec.len a.result) (by simp)
    (ufRoot (vals join1.parent)) hRt face_of1 hF1.fol hF1.fo faces 0#usize (by simp) hF1.fl
    (fun k hk => ⟨by simp [(hF1.fc k hk).facets], (hF1.fc k hk).plane, (hF1.fc k hk).orient⟩)
    as ⟨faces1, hfl1, hF2⟩
  have hHO : ∀ p, p < a.result.val.length →
      (nv head_of.val).getD p 0 = rAt a.result.val (ufRoot (vals join1.parent) p) := by
    intro p hp; rw [nv_getD (by rw [hF1.hol]; exact hp), hF1.ho p hp]
  -- the faces round each edge
  step with ate_spec l hw a hrl head_of (fun e he x hx => by rw [hF1.hol]; exact hrn e he x hx) ne hnev
    (alloc.vec.Vec.with_capacity (alloc.vec.Vec U32) ne) 0#usize (by simp)
    (by simp [alloc.vec.Vec.with_capacity]) as ⟨at_e1, hate⟩
  -- the verts
  have hpsn : ∀ v, v < nrowsL l.vert_edges → ∀ p ∈ PSf l a v, p < a.result.val.length := by
    intro v hv p hp
    simp only [PSf, List.mem_map] at hp
    obtain ⟨x, hx, rfl⟩ := hp
    obtain ⟨_, hf, hne'⟩ := mem_avL_lt l hw a hv hx
    exact (h.rpA_some hf hne').1
  step as ⟨io0, hio0⟩
  step with verts_spec l hw a hrl head_of (fun v hv p hp => by rw [hF1.hol]; exact hpsn v hv p hp) nv' hnvv io0
    (alloc.vec.Vec.new VertId) 0#usize (by simp) (by rw [hio0]; simp) (by simp)
    (fun i hi => by simp at hi)
    (fun i _ => by rw [hio0]; simp [List.getD_eq_getElem?_getD, List.getElem?_replicate]; split <;> rfl)
    as ⟨io1, verts, hverts, hiol, hio⟩
  -- the edges with two faces
  step as ⟨ep0, hep0⟩
  step with ecs_spec ne (by omega) (AtEf l a head_of.val) at_e1 hate (alloc.vec.Vec.new U32) ep0 0#usize
    (by simp) (by simp [ECSf, nv]) (by rw [hep0]; simp)
    (fun k hk => by rw [hep0]; simp [List.getD_eq_getElem?_getD, List.getElem?_replicate, hk, inc_none_val])
    as ⟨ecs, ep, hecs, hepl, hep⟩
  set A := AtEf l a head_of.val with hAdef
  have hecsl : ecs.val.length = (ECSf A ne.val).length := by
    have := congrArg List.length hecs; simpa [nv] using this
  have hecsv : ∀ i (hi : i < ecs.val.length), idA ecs.val i = (ECSf A ne.val)[i]'(lt_of_lt_of_eq hi hecsl) := by
    intro i hi
    have := congrArg (fun L => L[i]?) hecs
    simp only [nv, List.getElem?_map] at this
    rw [List.getElem?_eq_getElem hi, List.getElem?_eq_getElem (by omega)] at this
    simp only [Option.map_some, Option.some.injEq] at this
    rw [← id_read hi, this]
  have hm : ecs.val.length < U32.max := by
    rw [hecsl]; have := u_filter_range_len (fun e => decide ((A e).length = 2)) ne.val
    unfold ECSf; omega
  have hecsn : ∀ x ∈ ecs.val, x.val < nrowsL l.edge_facets := by
    intro x hx
    obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem hx
    rw [id_read hi, hecsv i hi, ← hnev]
    exact (fr_pos (fun e => decide ((A e).length = 2)) (n := ne.val) (i := i) (lt_of_lt_of_eq hi hecsl)).1
  -- the edge join
  step with new_spec (alloc.vec.Vec.len ecs) (by simp; omega) as ⟨erel, herel⟩
  have hu2 : UInv [] ecs.val.length erel := by
    unfold UInv; rw [herel]; simpa using inv_range ecs.val.length
  have hC : ErCtx (nrowsL l.edge_facets) ecs.val.length A l.vert_edges.ids.val at_e1.val ep.val := by
    refine ⟨hw.vert_edges.ids, by rw [hate, hnev], by rw [hepl, hnev], ?_, by omega⟩
    intro x hx hne'
    have hx' : x < ne.val := by omega
    have := hep x hx'
    unfold ecpA at hne' ⊢
    rw [this] at hne' ⊢
    by_cases h2 : (A x).length = 2
    · rw [if_pos h2, hecsl]
      exact fr_len_lt (fun e => decide ((A e).length = 2)) (by simpa using h2) hx'
    · rw [if_neg h2] at hne'; exact absurd rfl hne'
  step with erel_spec l hw ecs.val.length A at_e1 ep hC nv' hnvv io1 hiol erel [] 0#usize (by simp) hu2 (by simp)
    (fun v h => by simp at h) as ⟨erel1, E2, hu2', hs2, hc2⟩
  have hlE := hu2'.2.2
  have hRt2 : RtOK (ufRoot (vals erel1.parent)) ecs.val.length := u_least_facts hlE
  -- the edges
  step as ⟨eo0, heo0⟩
  step with edges_spec ne.val A at_e1 hate ecs (fun x hx => by rw [hnev]; exact hecsn x hx) hm
    (ufRoot (vals erel1.parent)) hRt2 erel1 (alloc.vec.Vec.new OutEdge) eo0 0#usize (by simp)
    ⟨hu2'.1, uinv_len hu2', fun _ => rfl⟩
    ⟨by rw [heo0]; simp, fun i hi => by simp at hi, by simp [hdL], fun k hk => by simp at hk⟩
    as ⟨edges, hel, hEd⟩
  -- the Lean side: the face join is `joinRel`
  have hJs : ∀ p ∈ EJ, p.1 < a.result.val.length ∧ p.2 < a.result.val.length ∧
      (toLF l).joinRel kt (crA a p.1) (crA a p.2) = true := by
    intro p hp
    obtain ⟨h1, h2, hjc, x, hx, m1, m2⟩ := hsJ p hp
    obtain ⟨k, hk, rfl⟩ := List.mem_map.mp hx
    exact ⟨h1, h2, (h.joinRel_iff h1 h2).mpr ⟨hjc, k, List.mem_range.mp hk, m1, m2⟩⟩
  have hJc : ∀ i j, i < a.result.val.length → j < a.result.val.length →
      (toLF l).joinRel kt (crA a i) (crA a j) = true → Eqv EJ i j := by
    intro i j hi hj hr
    obtain ⟨hjc, k, hk, m1, m2⟩ := (h.joinRel_iff hi hj).mp hr
    exact hcJ k hk i m1 j m2 hjc
  have hinj : ∀ i j, i < a.result.val.length → j < a.result.val.length → crA a i = crA a j → i = j :=
    fun i j hi hj he => h.crA_inj hi hj he
  have hiffJ := fun i j hi hj => u_eqv_iff (crA a) a.result.val.length ((toLF l).joinRel kt)
    (joinRel_symm _ _) EJ hJs hJc hinj (i := i) (j := j) hi hj
  obtain ⟨hHeads, hFaceOf⟩ := u_cls_filter hlJ hiffJ hinj
  have hV : (toLF l).verts = (List.range (nrowsL l.vert_edges)).map FCell.vert := rfl
  have hE : (toLF l).edges = (List.range (nrowsL l.edge_facets)).map FCell.edge := rfl
  unfold bodyL IncLedger.unify
  lift_lets
  intro hsL fk key atE atV outV ecsL erelL
  have hhs : hsL = (hdL (ufRoot (vals join1.parent)) a.result.val.length).map (crA a) := by
    show (toLF l).heads kt = _
    unfold IncLedger.heads; rw [h.result_pos]
    have := hHeads; simp only [beq_dec'] at this ⊢; exact this
  have hFO : ∀ t, t < a.result.val.length → ufRoot (vals join1.parent) t = t →
      (toLF l).faceOf kt (crA a t) =
        ((List.range a.result.val.length).filter fun d => ufRoot (vals join1.parent) d = t).map (crA a) := by
    intro t ht htt
    unfold IncLedger.faceOf; rw [h.result_pos]
    have := hFaceOf t ht; simp only [contains_dec'] at this ⊢; rw [htt] at this
    exact this
  have hkey : ∀ p, p < a.result.val.length → key (crA a p) = crA a (ufRoot (vals join1.parent) p) := by
    intro p hp
    show (Option.map Prod.snd (List.find? (fun e => e.1 == crA a p)
      (List.flatMap (fun h => List.map (fun c => (c, h)) ((toLF l).faceOf kt h)) hsL))).getD _ = _
    rw [hhs]
    have := u_key hRt hinj _ hFO hp; simp only [beq_dec'] at this ⊢; exact this
  -- the faces round an edge and a vert
  have hAtE : ∀ k, atE (.edge k) = (A k).map FCell.facet := by
    intro k
    show (toLF l).facesAtE kt key (.edge k) = _
    unfold IncLedger.facesAtE
    rw [h.aroundE_pos, List.map_map]
    have hlt := h.aroundL_lt (row_ids_lt l hw (e := k))
    have e1 : (aroundL a (rowL l.edge_facets k)).map (key ∘ crA a) =
        ((aroundL a (rowL l.edge_facets k)).map fun p => (nv head_of.val).getD p 0).map FCell.facet := by
      rw [List.map_map]; apply List.map_congr_left; intro p hp
      simp only [Function.comp]; rw [hkey p (hlt p hp), hHO p (hlt p hp)]; rfl
    rw [e1, dedup_map _ _ (fun x _ y _ hxy => facet_inj x y hxy)]; rfl
  have hAtVl : ∀ v, v < nrowsL l.vert_edges → (atV (.vert v)).length = (AtVf l a head_of.val v).length := by
    intro v hv
    show ((toLF l).facesAtV kt key (.vert v)).length = _
    unfold IncLedger.facesAtV
    rw [aroundV_eq l hw s hs kv kt hk a hA hv, List.map_map]
    have e1 : (avL l a (sstAt l v) (sstAt l (v + 1))).map ((fun s => key s.facet) ∘ fun k => toSec (secAt l k)) =
        ((PSf l a v).map fun p => (nv head_of.val).getD p 0).map FCell.facet := by
      rw [PSf, List.map_map, List.map_map]; apply List.map_congr_left; intro x hx
      obtain ⟨_, hf, hne'⟩ := mem_avL_lt l hw a hv hx
      obtain ⟨hpn, hpr⟩ := h.rpA_some hf hne'
      simp only [Function.comp, toSec]
      have : FCell.facet (secAt l x).facet.val = crA a (rpA a (secAt l x).facet.val) := by rw [crA, hpr]
      rw [this, hkey _ hpn, hHO _ hpn]; rfl
    rw [e1, dedup_map _ _ (fun x _ y _ hxy => facet_inj x y hxy), List.length_map]; rfl
  have hOutV : outV = ((List.range nv'.val).filter fun v => 3 ≤ (AtVf l a head_of.val v).length).map FCell.vert := by
    show (toLF l).verts.filter (fun v => decide (3 ≤ (atV v).length)) = _
    rw [hV, List.filter_map, hnvv]; congr 1; apply List.filter_congr; intro v hv
    simp only [Function.comp]; rw [hAtVl v (List.mem_range.mp hv)]
  -- the edges with two faces, by position
  have hecsL : ecsL = (List.range ecs.val.length).map fun i => FCell.edge (idA ecs.val i) := by
    show (toLF l).edges.filter (fun e => (atE e).length == 2) = _
    rw [hE, List.filter_map]
    have e1 : (List.range (nrowsL l.edge_facets)).filter ((fun e => (atE e).length == 2) ∘ FCell.edge) =
        ECSf A ne.val := by
      rw [hnev]; unfold ECSf; apply List.filter_congr; intro k _
      simp only [Function.comp]; rw [hAtE k, List.length_map]; exact beq_dec' _ _
    rw [e1]
    apply List.ext_getElem (by simp [hecsl])
    intro i h1 h2
    simp only [List.getElem_map, List.getElem_range]
    rw [hecsv i (by simpa using h2)]
  have hpos1 : ∀ i, i < ecs.val.length → idA ecs.val i < ne.val ∧ (A (idA ecs.val i)).length = 2 ∧
      ecpA ep.val (idA ecs.val i) = i := by
    intro i hi
    have hf : (ECSf A ne.val)[i]'(lt_of_lt_of_eq hi hecsl) < ne.val ∧
        decide ((A ((ECSf A ne.val)[i]'(lt_of_lt_of_eq hi hecsl))).length = 2) = true ∧
        (ECSf A ((ECSf A ne.val)[i]'(lt_of_lt_of_eq hi hecsl))).length = i :=
      fr_pos (fun e => decide ((A e).length = 2)) (n := ne.val) (i := i) (lt_of_lt_of_eq hi hecsl)
    rw [hecsv i hi]
    have h2 : (A ((ECSf A ne.val)[i]'(lt_of_lt_of_eq hi hecsl))).length = 2 := of_decide_eq_true hf.2.1
    refine ⟨hf.1, h2, ?_⟩
    unfold ecpA; rw [hep _ hf.1, if_pos h2]; exact hf.2.2
  have hpos2 : ∀ x, x < ne.val → ecpA ep.val x ≠ U32.max → idA ecs.val (ecpA ep.val x) = x := by
    intro x hx hne'
    unfold ecpA at hne' ⊢
    rw [hep x hx] at hne' ⊢
    by_cases h2 : (A x).length = 2
    · rw [if_pos h2]
      obtain ⟨hl, hg⟩ := fr_get (fun e => decide ((A e).length = 2)) (by simpa using h2) hx
      rw [hecsv _ (by rw [hecsl]; exact hl)]; exact hg
    · rw [if_neg h2] at hne'; exact absurd rfl hne'
  have hinj2 : ∀ i j, i < ecs.val.length → j < ecs.val.length →
      FCell.edge (idA ecs.val i) = FCell.edge (idA ecs.val j) → i = j := by
    intro i j hi hj he
    have := edge_inj _ _ he
    rw [← (hpos1 i hi).2.2, ← (hpos1 j hj).2.2, this]
  -- the edge join is `erel`
  have hOutC : ∀ v, v < nrowsL l.vert_edges → (outV.contains (.vert v) = false ↔ io1.val.getD v false = false) := by
    intro v hv
    rw [hio v (by omega), hOutV, contains_dec', decide_eq_false_iff_not, decide_eq_false_iff_not, List.mem_map]
    constructor
    · intro hn' h3; exact hn' ⟨v, List.mem_filter.mpr ⟨List.mem_range.mpr (by omega), by simpa using h3⟩, rfl⟩
    · rintro hn' ⟨w, hw', e⟩
      cases e
      exact hn' (by simpa using (List.mem_filter.mp hw').2)
  have hErel : ∀ i j, i < ecs.val.length → j < ecs.val.length →
      (erelL (.edge (idA ecs.val i)) (.edge (idA ecs.val j)) = true ↔
        SS (A (idA ecs.val i)) (A (idA ecs.val j)) = true ∧ GV l io1.val (idA ecs.val i) (idA ecs.val j)) := by
    intro i j hi hj
    show (IncLedger.sameSetI (atE (.edge (idA ecs.val i))) (atE (.edge (idA ecs.val j))) &&
      (toLF l).verts.any fun v => ((toLF l).vertEdges v).contains (.edge (idA ecs.val i)) &&
        ((toLF l).vertEdges v).contains (.edge (idA ecs.val j)) && !outV.contains v) = true ↔ _
    rw [hAtE, hAtE, sameSetI_map facet_inj, Bool.and_eq_true, List.any_eq_true]
    apply and_congr Iff.rfl
    have hVE : ∀ v x, ((toLF l).vertEdges (.vert v)).contains (.edge x) = true ↔ x ∈ rowL l.vert_edges v := by
      intro v x
      show ((rowL l.vert_edges v).map FCell.edge).contains (.edge x) = true ↔ _
      rw [List.contains_iff_mem, List.mem_map]
      exact ⟨fun ⟨y, hy, e⟩ => edge_inj _ _ e ▸ hy, fun hx => ⟨x, hx, rfl⟩⟩
    constructor
    · rintro ⟨w, hw', hc⟩
      rw [hV] at hw'
      obtain ⟨v, hv, rfl⟩ := List.mem_map.mp hw'
      have hv' := List.mem_range.mp hv
      simp only [Bool.and_eq_true, Bool.not_eq_true'] at hc
      obtain ⟨⟨c1, c2⟩, c3⟩ := hc
      exact ⟨v, hv', (hOutC v hv').mp c3, (hVE v _).mp c1, (hVE v _).mp c2⟩
    · rintro ⟨v, hv, hio', hx, hy⟩
      refine ⟨.vert v, by rw [hV]; exact List.mem_map.mpr ⟨v, List.mem_range.mpr hv, rfl⟩, ?_⟩
      simp only [Bool.and_eq_true, Bool.not_eq_true']
      exact ⟨⟨(hVE v _).mpr hx, (hVE v _).mpr hy⟩, (hOutC v hv).mpr hio'⟩
  have hErelSymm : ∀ x y, erelL x y = erelL y x := by
    intro x y
    show (IncLedger.sameSetI (atE x) (atE y) && _) = (IncLedger.sameSetI (atE y) (atE x) && _)
    rw [sameSetI_symm (atE x)]
    congr 1; congr 1; funext v; rw [Bool.and_comm (((toLF l).vertEdges v).contains x)]
  have hveL : ∀ v x, x ∈ rowL l.vert_edges v → x < nrowsL l.edge_facets := by
    intro v x hx
    simp only [rowL, List.mem_map] at hx
    obtain ⟨z, hz, rfl⟩ := hx
    exact hw.vert_edges.ids z (List.mem_of_mem_drop (List.mem_of_mem_take hz))
  have hEs : ∀ p ∈ E2, p.1 < ecs.val.length ∧ p.2 < ecs.val.length ∧
      erelL (.edge (idA ecs.val p.1)) (.edge (idA ecs.val p.2)) = true := by
    intro p hp
    obtain ⟨h1, h2, x, y, ex, ey, hss, hG⟩ := hs2 p hp
    obtain ⟨v, hv, hio', hx, hy⟩ := hG
    have hxn : x < ne.val := by rw [hnev]; exact hveL v x hx
    have hyn : y < ne.val := by rw [hnev]; exact hveL v y hy
    have ex' : idA ecs.val p.1 = x := by rw [← ex]; exact hpos2 x hxn (by rw [ex]; omega)
    have ey' : idA ecs.val p.2 = y := by rw [← ey]; exact hpos2 y hyn (by rw [ey]; omega)
    refine ⟨h1, h2, (hErel _ _ h1 h2).mpr ?_⟩
    rw [ex', ey']; exact ⟨hss, v, hv, hio', hx, hy⟩
  have hEc : ∀ i j, i < ecs.val.length → j < ecs.val.length →
      erelL (.edge (idA ecs.val i)) (.edge (idA ecs.val j)) = true → Eqv E2 i j := by
    intro i j hi hj hr
    obtain ⟨hss, v, hv, hio', hx, hy⟩ := (hErel i j hi hj).mp hr
    have := hc2 v hv hio' _ hx _ hy
      ⟨by rw [(hpos1 i hi).2.2]; omega, by rw [(hpos1 j hj).2.2]; omega, hss⟩
    rw [(hpos1 i hi).2.2, (hpos1 j hj).2.2] at this; exact this
  have hiff2 := fun i j hi hj => u_eqv_iff (fun i => FCell.edge (idA ecs.val i)) ecs.val.length erelL hErelSymm E2
    hEs hEc hinj2 (i := i) (j := j) hi hj
  obtain ⟨hHeads2, hCls2⟩ := u_cls_filter hlE hiff2 hinj2
  simp only [BodyI.mk.injEq]
  refine ⟨?_, ?_, ?_⟩
  · -- the faces
    rw [hhs, List.map_map]
    apply List.ext_getElem (by simp [hfl1])
    intro f h1 h2
    have hf : f < faces1.val.length := by simpa using h1
    have hfh : f < (hdL (ufRoot (vals join1.parent)) a.result.val.length).length := by rw [← hfl1]; exact hf
    have hfo := hF2 f hf
    rw [hdL_getD hfh] at hfo
    obtain ⟨hhn, hhR⟩ := u_mem_hdL.mp (List.getElem_mem hfh)
    simp only [List.getElem_map, Function.comp]
    unfold faceL
    rw [OutFaceI.mk.injEq]
    refine ⟨?_, ?_, ?_⟩
    · have e1 : (faces1.val[f]).facets.val.map (fun (c : FacetId) => FCell.facet (c : U32).val) =
          ((faces1.val[f]).facets.val.map (fun (c : FacetId) => (c : U32).val)).map FCell.facet := by simp
      rw [e1, hfo.facets, List.map_map, hFO _ hhn hhR]; rfl
    · rw [hfo.plane]; rfl
    · rw [hfo.orient, h.orient_cr hhn]
  · -- the edges
    rw [hecsL]
    have hH := hHeads2
    simp only [beq_dec'] at hH ⊢
    rw [hH, List.map_map]
    apply List.ext_getElem (by simp [hel])
    intro f h1 h2
    have hf : f < edges.val.length := by simpa using h1
    have hfh : f < (hdL (ufRoot (vals erel1.parent)) ecs.val.length).length := by rw [← hel]; exact hf
    have heo := hEd f hf
    rw [hdL_getD hfh] at heo
    obtain ⟨hhn, hhR⟩ := u_mem_hdL.mp (List.getElem_mem hfh)
    simp only [List.getElem_map, Function.comp]
    unfold edgeL
    rw [OutEdgeI.mk.injEq]
    refine ⟨?_, ?_⟩
    · have e1 : (edges.val[f]).cells.val.map (fun (c : EdgeId) => FCell.edge (c : U32).val) =
          ((edges.val[f]).cells.val.map (fun (c : EdgeId) => (c : U32).val)).map FCell.edge := by simp
      rw [e1, heo.cells, List.map_map]
      have := hCls2 _ hhn
      simp only [contains_dec'] at this ⊢
      rw [hhR] at this
      rw [this]; rfl
    · have e1 : (edges.val[f]).faces.val.map (fun (c : FacetId) => FCell.facet (c : U32).val) =
          ((edges.val[f]).faces.val.map (fun (c : FacetId) => (c : U32).val)).map FCell.facet := by simp
      rw [e1, heo.faces, hAtE]
  · -- the verts
    rw [hOutV]
    have e1 : verts.val.map (fun (v : VertId) => FCell.vert (v : U32).val) =
        (verts.val.map (fun (c : VertId) => (c : U32).val)).map FCell.vert := by simp
    rw [e1, hverts]


/-- **The inc core refines `coreMI`** (`IncCoreSpec`), with no open obligations. -/
theorem incCoreSpec_proof : IncCoreSpec := incCoreSpec_of_unify unify_specI

/-- **The new boolean's end-to-end theorem** (`EndToEndF`), for no thin rooms. -/
theorem endToEndF_proof : EndToEndF := endToEndF_of incCoreSpec_proof

end inc

end forge_bool_core
