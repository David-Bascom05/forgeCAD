import ForgeBoolCore.RefineIncAssemble

/-!
# Group K: the touch rule at edges

`inc::around_e l a e` lists, for each facet round edge `e` (once per wing) that is in the result,
its position in the result. `core_k`'s second loop refuses at the first edge with more than two:
`edges.find? badEdge`.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-! ### Csr rows -/

def stAt (c : Csr) (k : Nat) : Nat := (c.start.val.getD k 0#u32).val

theorem csr_mono {c : Csr} {n : Nat} (h : CsrOK c n) {k : Nat} :
    ∀ d, k + d < c.start.val.length → stAt c k ≤ stAt c (k + d)
  | 0, _ => Nat.le_refl _
  | d + 1, hd => by
    have ih := csr_mono h (k := k) d (by omega)
    have := h.mono (k + d) (by omega)
    have e : k + (d + 1) = k + d + 1 := by omega
    rw [e]; exact Nat.le_trans ih this

theorem csr_le_len {c : Csr} {n : Nat} (h : CsrOK c n) {k : Nat} (hk : k < c.start.val.length) :
    stAt c k ≤ c.ids.val.length := by
  have := csr_mono h (k := k) (c.start.val.length - 1 - k) (by omega)
  rw [show k + (c.start.val.length - 1 - k) = c.start.val.length - 1 by omega] at this
  have hl := h.last
  unfold stAt at *; omega

/-- A row's ids, from position `lo` up to `k`. -/
def subL (c : Csr) (lo k : Nat) : List Nat := ((c.ids.val.drop lo).take (k - lo)).map (·.val)

theorem subL_succ (c : Csr) {lo k : Nat} (hlo : lo ≤ k) (hk : k < c.ids.val.length) :
    subL c lo (k + 1) = subL c lo k ++ [(c.ids.val[k]).val] := by
  unfold subL
  rw [show k + 1 - lo = (k - lo) + 1 by omega, List.take_add_one, List.map_append]
  congr 1
  rw [List.getElem?_drop, show lo + (k - lo) = k by omega, List.getElem?_eq_getElem hk]
  rfl

theorem rowL_eq (c : Csr) (k : Nat) : rowL c k = subL c (stAt c k) (stAt c (k + 1)) := rfl

/-! ### `around_e` -/

/-- A facet's position in the result, or `NONE`. -/
def rpA (a : Assembled) (f : Nat) : Nat := (a.rpos.val.getD f NONE).val

def aroundL (a : Assembled) (xs : List Nat) : List Nat :=
  (xs.filter fun f => rpA a f != U32.max).map (rpA a)

theorem around_e_loop_spec (c : Csr) (n : Nat) (hc : CsrOK c n) (a : Assembled) (hrl : n ≤ a.rpos.val.length)
    (lo hi : Usize) (hlh : lo.val ≤ hi.val) (hhi : hi.val ≤ c.ids.val.length)
    (out : alloc.vec.Vec U32) (k : Usize) (hk : lo.val ≤ k.val ∧ k.val ≤ hi.val)
    (ho : out.val.map (·.val) = aroundL a (subL c lo.val k.val)) :
    around_e_loop c a hi out k ⦃ r => r.val.map (·.val) = aroundL a (subL c lo.val hi.val) ⦄ := by
  unfold around_e_loop
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec U32 × Usize) => hi.val - q.2.val)
    (fun q => lo.val ≤ q.2.val ∧ q.2.val ≤ hi.val ∧ q.1.val.map (·.val) = aroundL a (subL c lo.val q.2.val))
  · rintro ⟨o, j⟩ ⟨hj1, hj2, ho'⟩
    unfold around_e_loop.body
    simp only at hj1 hj2 ho' ⊢
    split
    · rename_i hlt
      have hjl : j.val < c.ids.val.length := by scalar_tac
      have hid := hc.ids _ (List.getElem_mem hjl)
      step as ⟨i, hi'⟩
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = (c.ids.val[j.val]).val := by rw [hi1, hi']; scalar_tac
      step as ⟨p, hp⟩
      have hpv : p.val = rpA a (c.ids.val[j.val]).val := by
        rw [rpA, ← hi1v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, hp]
      have hsucc : aroundL a (subL c lo.val (j.val + 1)) =
          aroundL a (subL c lo.val j.val) ++
            (if rpA a (c.ids.val[j.val]).val != U32.max then [rpA a (c.ids.val[j.val]).val] else []) := by
        rw [subL_succ c hj1 hjl, aroundL, aroundL, List.filter_append, List.map_append]
        congr 1
        simp only [List.filter_cons, List.filter_nil]
        split <;> simp
      have hol : o.val.length ≤ j.val := by
        have := congrArg List.length ho'
        simp only [List.length_map, aroundL] at this
        have h2 := List.length_filter_le (fun f => rpA a f != U32.max) (subL c lo.val j.val)
        have h3 : (subL c lo.val j.val).length ≤ j.val := by simp only [subL, List.length_map, List.length_take]; omega
        omega
      split
      · rename_i hne
        have hne' : (rpA a (c.ids.val[j.val]).val != U32.max) = true := by
          rw [← hpv]; have : p ≠ NONE := by simpa using hne
          simp only [bne_iff_ne, ne_eq]; intro e; apply this; apply UScalar.eq_of_val_eq; rw [e, inc_none_val]
        step as ⟨o1, ho1⟩
        step as ⟨j1, hj1'⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        have : j1.val = j.val + 1 := by scalar_tac
        rw [this, hsucc, if_pos hne', ho1, List.map_append, ho']; simp [hpv]
      · rename_i hne
        have hne' : (rpA a (c.ids.val[j.val]).val != U32.max) = false := by
          rw [← hpv]; have : p.val = NONE.val := by simpa using hne
          rw [this, inc_none_val]; simp
        step as ⟨j1, hj1'⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        have : j1.val = j.val + 1 := by scalar_tac
        rw [this, hsucc, hne']; simpa using ho'
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = hi.val := by scalar_tac
      rw [← this]; exact ho'
  · exact ⟨hk.1, hk.2, ho⟩

theorem around_e_spec (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) (hrl : a.rpos.val.length = nfacets l)
    (e : Usize) (he : e.val < nrowsL l.edge_facets) :
    around_e l a e ⦃ r => r.val.map (·.val) = aroundL a (rowL l.edge_facets e.val) ⦄ := by
  have hc := hw.edge_facets
  unfold around_e
  have hk : e.val + 1 < l.edge_facets.start.val.length := by unfold nrowsL at he; omega
  step with row_spec l.edge_facets e hk as ⟨lo, hi, hlo, hhi⟩
  have hm := csr_mono hc (k := e.val) 1 hk
  have hl := csr_le_len hc hk
  unfold stAt at hm hl
  have e1 : rowL l.edge_facets e.val = subL l.edge_facets lo.val hi.val := by
    rw [rowL_eq]; unfold stAt; rw [hlo, hhi]
  rw [e1]
  exact around_e_loop_spec l.edge_facets (nfacets l) hc a (by omega) lo hi (by omega) (by omega)
    (alloc.vec.Vec.new U32) lo ⟨Nat.le_refl _, by omega⟩ (by simp [aroundL, subL])

/-! ### The Lean side -/

theorem rpA_eq (l : Ledger) (s : Split) (keep : List Bool) (a : Assembled)
    (h : AInv l s keep a.result.val a.rpos.val a.orient.val (nfacets l)) {f : Nat} (hf : f < nfacets l) :
    rpA a f = if inR l s keep f = true then (resL l s keep f).length else U32.max := by
  rw [rpA, h.rpos f hf]; simp [hf]

theorem resL_len_lt (l : Ledger) (s : Split) (keep : List Bool) (f : Nat) : (resL l s keep f).length ≤ f := by
  have := List.length_filter_le (inR l s keep) (List.range f); simpa [resL] using this

theorem rpA_ne (l : Ledger) (hw : LedgerWF2 l) (s : Split) (keep : List Bool) (a : Assembled)
    (h : AInv l s keep a.result.val a.rpos.val a.orient.val (nfacets l)) {f : Nat} (hf : f < nfacets l) :
    (rpA a f != U32.max) = inR l s keep f := by
  rw [rpA_eq l s keep a h hf]
  have := resL_len_lt l s keep f
  have := hw.small.1
  cases inR l s keep f <;> simp <;> omega

theorem mem_resL (l : Ledger) (s : Split) (keep : List Bool) {f : Nat} (hf : f < nfacets l) :
    f ∈ resL l s keep (nfacets l) ↔ inR l s keep f = true := by
  simp [resL, hf]

theorem row_ids_lt (l : Ledger) (hw : LedgerWF2 l) {e : Nat} :
    ∀ f ∈ rowL l.edge_facets e, f < nfacets l := by
  intro f hf
  simp only [rowL, List.mem_map] at hf
  obtain ⟨x, hx, rfl⟩ := hf
  exact hw.edge_facets.ids x (List.mem_of_mem_drop (List.mem_of_mem_take hx))

/-- The Lean's result wings round an edge, counted. -/
theorem aroundE_len (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule)
    (a : Assembled) (h : AInv l s (keepVec l s rule) a.result.val a.rpos.val a.orient.val (nfacets l)) (e : Nat) :
    ((toLF l).aroundE ((toLF l).keepTab (ruleL l.nb.val rule)) (FCell.edge e)).length =
      (aroundL a (rowL l.edge_facets e)).length := by
  unfold IncLedger.aroundE aroundL
  rw [result_eq l hw s hs rule, List.length_map]
  have hE : (toLF l).edgeFacets (FCell.edge e) = (rowL l.edge_facets e).map FCell.facet := rfl
  rw [hE, List.filter_map, List.length_map]
  congr 1
  apply List.filter_congr
  intro f hf
  have hfl := row_ids_lt l hw f hf
  rw [rpA_ne l hw s _ a h hfl]
  simp only [Function.comp]
  rw [Bool.eq_iff_iff, List.contains_iff_mem, List.mem_map]
  constructor
  · rintro ⟨g, hg, he⟩; cases he; exact (mem_resL l s _ hfl).mp hg
  · intro hi; exact ⟨f, (mem_resL l s _ hfl).mpr hi, rfl⟩

/-- An edge is bad, by `around_e`'s count. -/
def bE (l : Ledger) (a : Assembled) (e : Nat) : Bool := decide (2 < (aroundL a (rowL l.edge_facets e)).length)

theorem find_range_edges (l : Ledger) (p : FCell → Bool) :
    (toLF l).edges.find? p = ((List.range (nrowsL l.edge_facets)).find? (p ∘ FCell.edge)).map FCell.edge := by
  simp [toLF, List.find?_map]

/-! ### The edge scan -/

/-- What the scan has found over the first `k` edges. -/
def EScan (l : Ledger) (a : Assembled) (bad0 : Outcome) (k : Nat) (bad : Outcome) (found : Bool) : Prop :=
  match (List.range k).find? (bE l a) with
  | some e => found = true ∧ ∃ x : U32, x.val = e ∧ bad = .Refused (.ZeroThicknessEdge x)
  | none => found = false ∧ bad = bad0

theorem find_range_succ (p : Nat → Bool) (k : Nat) :
    (List.range (k + 1)).find? p = ((List.range k).find? p).or (if p k then some k else none) := by
  rw [List.range_succ, List.find?_append]
  simp only [List.find?_cons, List.find?_nil]
  cases (List.range k).find? p <;> cases p k <;> simp

theorem find_range_mono {p : Nat → Bool} {k n : Nat} (hk : k ≤ n) {e : Nat}
    (h : (List.range k).find? p = some e) : (List.range n).find? p = some e := by
  induction n with
  | zero => have : k = 0 := by omega
            subst this; simp at h
  | succ n ih =>
    rcases Nat.lt_or_ge k (n + 1) with hl | hl
    · rw [find_range_succ, ih (by omega)]; rfl
    · have : k = n + 1 := by omega
      subst this; exact h

theorem core_k_loop1_spec (l : Ledger) (hw : LedgerWF2 l) (a : Assembled) (hrl : a.rpos.val.length = nfacets l)
    (ne : Usize) (hne : ne.val = nrowsL l.edge_facets) (bad0 bad : Outcome) (found : Bool) (e : Usize)
    (he : e.val ≤ ne.val) (hI : EScan l a bad0 e.val bad found) :
    core_k_loop1 l.nb l.face_body l.face_class l.n_rooms l.facet_class l.facet_rooms l.pieces l.inside
      l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids l.vert_sectors
      a.result a.rpos a.orient ne bad found e ⦃ r => EScan l a bad0 (nrowsL l.edge_facets) r.1 r.2 ⦄ := by
  have hsm := hw.small
  have hner : nrowsL l.edge_facets < U32.max := hsm.2.2.2.2.2.2.1
  unfold core_k_loop1
  apply loop.spec_decr_nat (fun (q : Outcome × Bool × Usize) => ne.val - q.2.2.val)
    (fun q => q.2.2.val ≤ ne.val ∧ EScan l a bad0 q.2.2.val q.1 q.2.1)
  · rintro ⟨b, fd, j⟩ ⟨hj, hE⟩
    unfold core_k_loop1.body
    simp only at hj hE ⊢
    split
    · rename_i hfd
      simp only [WP.spec_ok]
      unfold EScan at hE ⊢
      split at hE
      · rename_i e0 he0
        rw [find_range_mono (by omega) he0]; exact hE
      · exfalso; rw [hE.1] at hfd; simp at hfd
    · rename_i hfd
      have hfd' : fd = false := by simpa using hfd
      have hnone : (List.range j.val).find? (bE l a) = none := by
        unfold EScan at hE; split at hE
        · exfalso; rw [hE.1] at hfd'; simp at hfd'
        · assumption
      have hb0 : b = bad0 := by unfold EScan at hE; rw [hnone] at hE; exact hE.2
      split
      · rename_i hlt
        have hjl : j.val < nrowsL l.edge_facets := by scalar_tac
        step with around_e_spec l hw a hrl j hjl as ⟨v13, hv13⟩
        have hlen : v13.val.length = (aroundL a (rowL l.edge_facets j.val)).length := by
          have := congrArg List.length hv13; simpa using this
        split
        · rename_i hgt
          step as ⟨i3, hi3⟩
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by scalar_tac
          refine ⟨by omega, ?_, by omega⟩
          unfold EScan
          have hb : bE l a j.val = true := by
            unfold bE; rw [← hlen]; simp only [decide_eq_true_eq]; scalar_tac
          rw [hj1v, find_range_succ, hnone, if_pos hb]
          exact ⟨rfl, i3, by rw [hi3]; scalar_tac, rfl⟩
        · rename_i hgt
          step as ⟨j1, hj1⟩
          have hj1v : j1.val = j.val + 1 := by scalar_tac
          refine ⟨by omega, ?_, by omega⟩
          unfold EScan
          have hb : bE l a j.val = false := by
            unfold bE; rw [← hlen]; simp only [decide_eq_false_iff_not]; scalar_tac
          rw [hj1v, find_range_succ, hnone, hb]
          exact ⟨hfd', hb0⟩
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = nrowsL l.edge_facets := by scalar_tac
        rw [← this]; unfold EScan; rw [hnone]; exact ⟨hfd', hb0⟩
  · exact ⟨he, hI⟩

/-- **The scan's first bad edge is the Lean's.** -/
theorem edges_find_eq (l : Ledger) (hw : LedgerWF2 l) (s : Split) (hs : SplitOK l s) (rule : Rule)
    (a : Assembled) (h : AInv l s (keepVec l s rule) a.result.val a.rpos.val a.orient.val (nfacets l)) :
    (toLF l).edges.find? ((toLF l).badEdge ((toLF l).keepTab (ruleL l.nb.val rule))) =
      ((List.range (nrowsL l.edge_facets)).find? (bE l a)).map FCell.edge := by
  rw [find_range_edges]
  congr 2
  funext e
  simp only [Function.comp, IncLedger.badEdge, bE, aroundE_len l hw s hs rule a h e]

end inc

end forge_bool_core
