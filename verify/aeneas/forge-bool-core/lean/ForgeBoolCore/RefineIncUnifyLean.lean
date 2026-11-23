import ForgeBoolCore.RefineIncUnifyEdges

/-!
# Group U (inc), part 6: the Lean `unify`'s inputs, by result position

On an assembled result (`AInv`) whose keep vector agrees with the Lean's keep table (`KeepRel`),
the Lean's result is the result positions read as facets (`result_pos`), its carrier and sense are
the Rust tables' (`carrier_cr`, `orient_cr`), its wings round an edge are `around_e`'s
(`aroundE_pos`), and `joinRel` is "one carrier and sense, round one edge" (`joinRel_iff`). A filtered
range's members are found by position (`fr_get`, `fr_pos`), for the result, the heads and `ecs`.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-! ### A filtered range, by position -/

theorem fr_idxOf (P : Nat → Bool) {h : Nat} (hP : P h = true) :
    ∀ m, ((List.range (h + 1 + m)).filter P).idxOf h = ((List.range h).filter P).length ∧
      ((List.range h).filter P).length < ((List.range (h + 1 + m)).filter P).length := by
  intro m
  induction m with
  | zero =>
    rw [Nat.add_zero, u_filter_range_succ, if_pos hP,
      List.idxOf_append_of_notMem (fun hm => by simp at hm)]
    simp
  | succ m ih =>
    rw [show h + 1 + (m + 1) = (h + 1 + m) + 1 by omega, u_filter_range_succ]
    have hm : h ∈ (List.range (h + 1 + m)).filter P := by simp [hP]; omega
    rw [List.idxOf_append_of_mem hm, ih.1]
    refine ⟨rfl, ?_⟩
    simp only [List.length_append]; have := ih.2; omega

theorem fr_get (P : Nat → Bool) {h n : Nat} (hP : P h = true) (hn : h < n) :
    ∃ hl : ((List.range h).filter P).length < ((List.range n).filter P).length,
      ((List.range n).filter P)[((List.range h).filter P).length] = h := by
  obtain ⟨e1, e2⟩ := fr_idxOf P hP (n - h - 1)
  rw [show h + 1 + (n - h - 1) = n by omega] at e1 e2
  refine ⟨e2, ?_⟩
  have hm : h ∈ (List.range n).filter P := by simp [hP]; omega
  have := List.getElem_idxOf (List.idxOf_lt_length_iff.mpr hm)
  simp only [e1] at this; exact this

theorem fr_pos (P : Nat → Bool) {n i : Nat} (hi : i < ((List.range n).filter P).length) :
    ((List.range n).filter P)[i] < n ∧ P (((List.range n).filter P)[i]) = true ∧
    ((List.range (((List.range n).filter P)[i])).filter P).length = i := by
  have hm := List.getElem_mem hi
  simp only [List.mem_filter, List.mem_range] at hm
  refine ⟨hm.1, hm.2, ?_⟩
  obtain ⟨hl, hg⟩ := fr_get P hm.2 hm.1
  have hnd : ((List.range n).filter P).Nodup := List.nodup_range.filter _
  exact (List.Nodup.getElem_inj_iff hnd).mp hg

theorem fr_len_lt (P : Nat → Bool) {h n : Nat} (hP : P h = true) (hn : h < n) :
    ((List.range h).filter P).length < ((List.range n).filter P).length := (fr_get P hP hn).1

theorem getD_eq_get {L : List Nat} {i : Nat} (h : i < L.length) : L.getD i 0 = L[i] := by
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]; rfl

/-! ### The context -/

/-- What `unify` is given. -/
structure UCtx (l : Ledger) (s : Split) (kv : List Bool) (kt : KeepTabI FCell) (a : Assembled) : Prop where
  hw : LedgerWF2 l
  hs : SplitOK l s
  hk : KeepRel l s kv kt
  hA : AInv l s kv a.result.val a.rpos.val a.orient.val (nfacets l)

/-- The result's facet at position `i`, as the Lean's cell. -/
def crA (a : Assembled) (i : Nat) : FCell := .facet (rAt a.result.val i)

section
variable {l : Ledger} {s : Split} {kv : List Bool} {kt : KeepTabI FCell} {a : Assembled}

theorem UCtx.len (h : UCtx l s kv kt a) : a.result.val.length = (resL l s kv (nfacets l)).length := by
  have := congrArg List.length h.hA.res; simpa using this

theorem UCtx.rAt_RL (h : UCtx l s kv kt a) (i : Nat) : rAt a.result.val i = (resL l s kv (nfacets l)).getD i 0 := by
  rw [← h.hA.res, rAt]
  by_cases hi : i < a.result.val.length
  · rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi, List.getD_eq_getElem?_getD,
      List.getElem?_eq_getElem (by simpa using hi)]; simp
  · rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (by omega), List.getD_eq_getElem?_getD,
      List.getElem?_eq_none (by simp; omega)]; rfl

theorem UCtx.n_lt (h : UCtx l s kv kt a) : a.result.val.length < U32.max := by
  rw [h.len]
  have := List.length_filter_le (inR l s kv) (List.range (nfacets l))
  have := h.hw.small.1
  simp only [resL, List.length_range] at *; omega

theorem UCtx.rAt_mem (h : UCtx l s kv kt a) {i : Nat} (hi : i < a.result.val.length) :
    rAt a.result.val i < nfacets l ∧ inR l s kv (rAt a.result.val i) = true := by
  rw [h.rAt_RL, getD_eq_get (by rw [← h.len]; exact hi)]
  have := fr_pos (inR l s kv) (n := nfacets l) (i := i) (by rw [← resL, ← h.len]; exact hi)
  exact ⟨this.1, this.2.1⟩

theorem UCtx.rpA_some (h : UCtx l s kv kt a) {f : Nat} (hf : f < nfacets l) (hne : rpA a f ≠ U32.max) :
    rpA a f < a.result.val.length ∧ rAt a.result.val (rpA a f) = f := by
  rw [rpA_eq l s kv a h.hA hf] at hne ⊢
  cases hin : inR l s kv f
  · simp [hin] at hne
  · simp only [if_true]
    obtain ⟨hl, hg⟩ := fr_get (inR l s kv) hin hf
    rw [h.len]
    refine ⟨hl, ?_⟩
    rw [h.rAt_RL]; exact (getD_eq_get hl).trans hg

theorem UCtx.rpA_rAt (h : UCtx l s kv kt a) {i : Nat} (hi : i < a.result.val.length) :
    rpA a (rAt a.result.val i) = i := by
  obtain ⟨hf, hin⟩ := h.rAt_mem hi
  rw [rpA_eq l s kv a h.hA hf, if_pos hin]
  rw [h.rAt_RL, getD_eq_get (by rw [← h.len]; exact hi)]
  exact (fr_pos (inR l s kv) (n := nfacets l) (i := i) (by rw [← resL, ← h.len]; exact hi)).2.2

theorem UCtx.mem_aroundL (h : UCtx l s kv kt a) {xs : List Nat} (hxs : ∀ f ∈ xs, f < nfacets l) {i : Nat} :
    i ∈ aroundL a xs ↔ i < a.result.val.length ∧ rAt a.result.val i ∈ xs := by
  unfold aroundL
  simp only [List.mem_map, List.mem_filter, bne_iff_ne, ne_eq]
  constructor
  · rintro ⟨f, ⟨hf, hne⟩, rfl⟩
    obtain ⟨h1, h2⟩ := h.rpA_some (hxs f hf) hne
    exact ⟨h1, by rw [h2]; exact hf⟩
  · rintro ⟨hi, hm⟩
    refine ⟨_, ⟨hm, ?_⟩, h.rpA_rAt hi⟩
    rw [h.rpA_rAt hi]; have := h.n_lt; omega

theorem UCtx.aroundL_lt (h : UCtx l s kv kt a) {xs : List Nat} (hxs : ∀ f ∈ xs, f < nfacets l) :
    ∀ i ∈ aroundL a xs, i < a.result.val.length := fun _ hi => ((h.mem_aroundL hxs).mp hi).1

theorem UCtx.result_pos (h : UCtx l s kv kt a) :
    (toLF l).result kt = (List.range a.result.val.length).map (crA a) := by
  rw [result_eq l h.hw s h.hs kv kt h.hk]
  apply List.ext_getElem (by simp [h.len])
  intro i h1 h2
  simp only [List.getElem_map, List.getElem_range, crA]
  rw [h.rAt_RL, getD_eq_get (by simpa using h1)]

theorem UCtx.crA_inj (h : UCtx l s kv kt a) {i j : Nat} (hi : i < a.result.val.length) (hj : j < a.result.val.length)
    (he : crA a i = crA a j) : i = j := by
  simp only [crA, FCell.facet.injEq] at he
  rw [← h.rpA_rAt hi, ← h.rpA_rAt hj, he]

theorem carrier_cr (a : Assembled) (i : Nat) :
    (toLF l).carrier (crA a i) = cAt l.facet_class.val a.result.val i := rfl

theorem UCtx.orient_cr (h : UCtx l s kv kt a) {i : Nat} (hi : i < a.result.val.length) :
    (toLF l).orient kt (crA a i) = toS (oAt a.orient.val i) := by
  obtain ⟨hf, _⟩ := h.rAt_mem hi
  rw [crA, orient_eq l h.hw s h.hs kv kt h.hk hf]
  congr 1
  have hol : a.orient.val.length = a.result.val.length := by
    rw [h.hA.ori, List.length_map, h.len]
  rw [oAt_eq (by omega)]
  simp only [h.hA.ori, List.getElem_map]
  rw [h.rAt_RL, getD_eq_get (by rw [← h.len]; exact hi)]

theorem UCtx.aroundE_pos (h : UCtx l s kv kt a) (k : Nat) :
    (toLF l).aroundE kt (.edge k) = (aroundL a (rowL l.edge_facets k)).map (crA a) := by
  have hxs := row_ids_lt l h.hw (e := k)
  unfold IncLedger.aroundE aroundL
  have hE : (toLF l).edgeFacets (FCell.edge k) = (rowL l.edge_facets k).map FCell.facet := rfl
  rw [hE, List.filter_map, List.map_map]
  have e1 : (rowL l.edge_facets k).filter ((fun φ => (toLF l).result kt |>.contains φ) ∘ FCell.facet) =
      (rowL l.edge_facets k).filter (fun f => rpA a f != U32.max) := by
    apply List.filter_congr
    intro f hf
    rw [rpA_ne l h.hw s _ a h.hA (hxs f hf)]
    simp only [Function.comp]
    rw [result_eq l h.hw s h.hs kv kt h.hk, Bool.eq_iff_iff, List.contains_iff_mem, List.mem_map]
    constructor
    · rintro ⟨g, hg, he⟩; cases he; exact (mem_resL l s _ (hxs f hf)).mp hg
    · intro hi; exact ⟨f, (mem_resL l s _ (hxs f hf)).mpr hi, rfl⟩
  rw [e1]
  apply List.map_congr_left
  intro f hf
  simp only [List.mem_filter, bne_iff_ne, ne_eq] at hf
  simp only [Function.comp, crA]
  rw [(h.rpA_some (hxs f hf.1) hf.2).2]

theorem UCtx.joinRel_iff (h : UCtx l s kv kt a) {i j : Nat} (hi : i < a.result.val.length) (hj : j < a.result.val.length) :
    (toLF l).joinRel kt (crA a i) (crA a j) = true ↔
      JCi l.facet_class.val a.result.val a.orient.val i j ∧
      ∃ k, k < nrowsL l.edge_facets ∧ i ∈ aroundL a (rowL l.edge_facets k) ∧ j ∈ aroundL a (rowL l.edge_facets k) := by
  unfold IncLedger.joinRel
  simp only [Bool.and_eq_true, beq_iff_eq, List.any_eq_true]
  rw [carrier_cr, carrier_cr, h.orient_cr hi, h.orient_cr hj]
  have hE : (toLF l).edges = (List.range (nrowsL l.edge_facets)).map FCell.edge := rfl
  rw [hE]
  constructor
  · rintro ⟨⟨hc, ho⟩, e, he, h1, h2⟩
    obtain ⟨k, hk, rfl⟩ := List.mem_map.mp he
    have hxs := row_ids_lt l h.hw (e := k)
    have hE' : (toLF l).edgeFacets (FCell.edge k) = (rowL l.edge_facets k).map FCell.facet := rfl
    rw [hE', List.contains_iff_mem, List.mem_map] at h1 h2
    obtain ⟨f1, hf1, e1⟩ := h1
    obtain ⟨f2, hf2, e2⟩ := h2
    simp only [crA, FCell.facet.injEq] at e1 e2
    refine ⟨⟨hc, toS_inj ho⟩, k, List.mem_range.mp hk, (h.mem_aroundL hxs).mpr ⟨hi, ?_⟩,
      (h.mem_aroundL hxs).mpr ⟨hj, ?_⟩⟩
    · rw [← e1]; exact hf1
    · rw [← e2]; exact hf2
  · rintro ⟨⟨hc, ho⟩, k, hk, h1, h2⟩
    have hxs := row_ids_lt l h.hw (e := k)
    refine ⟨⟨hc, by rw [ho]⟩, .edge k, List.mem_map.mpr ⟨k, List.mem_range.mpr hk, rfl⟩, ?_, ?_⟩
    · have hE' : (toLF l).edgeFacets (FCell.edge k) = (rowL l.edge_facets k).map FCell.facet := rfl
      rw [hE', List.contains_iff_mem, List.mem_map]
      exact ⟨_, ((h.mem_aroundL hxs).mp h1).2, rfl⟩
    · have hE' : (toLF l).edgeFacets (FCell.edge k) = (rowL l.edge_facets k).map FCell.facet := rfl
      rw [hE', List.contains_iff_mem, List.mem_map]
      exact ⟨_, ((h.mem_aroundL hxs).mp h2).2, rfl⟩

end

theorem joinRel_symm (L : IncLedger FCell) (kt : KeepTabI FCell) (x y : FCell) :
    L.joinRel kt x y = L.joinRel kt y x := by
  unfold IncLedger.joinRel
  rw [Bool.beq_comm (a := L.carrier x), Bool.beq_comm (a := L.orient kt x)]
  congr 1; congr 1; funext e; rw [Bool.and_comm]

/-- `sameSetI` of lists named injectively is `SS` of the numbers. -/
theorem sameSetI_map {g : Nat → FCell} (hg : ∀ a b, g a = g b → a = b) (A B : List Nat) :
    IncLedger.sameSetI (A.map g) (B.map g) = SS A B := by
  unfold IncLedger.sameSetI SS
  rw [List.all_map, List.all_map]
  have h1 : A.all ((fun x => (B.map g).contains x) ∘ g) = A.all (fun x => decide (x ∈ B)) := by
    apply u_all_congr; intro x _; simp only [Function.comp]
    exact u_contains_map (fun a _ e => hg a x e)
  have h2 : B.all ((fun x => (A.map g).contains x) ∘ g) = B.all (fun x => decide (x ∈ A)) := by
    apply u_all_congr; intro x _; simp only [Function.comp]
    exact u_contains_map (fun a _ e => hg a x e)
  rw [h1, h2]

theorem sameSetI_symm (A B : List FCell) : IncLedger.sameSetI A B = IncLedger.sameSetI B A := by
  unfold IncLedger.sameSetI; rw [Bool.and_comm]

end inc

end forge_bool_core
