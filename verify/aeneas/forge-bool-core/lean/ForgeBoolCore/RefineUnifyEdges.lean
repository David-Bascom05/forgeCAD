import ForgeBoolCore.RefineUnifyErel

/-!
# Group U, part 7: the edges loop

With the edge join's roots, `unify`'s last loop numbers the heads in edge-cell order and fills each
edge with its cells in order, naming its faces by their heads (`u_edges_loop_spec`). Its
bookkeeping is the faces loop's (`u_einv_head`, `u_einv_non`, `u_einv_tail`).
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce ForgeBoolCore.UF

namespace forge_bool_core

/-- The cell id of edge cell number `i`. -/
def ECI (l : Ledger) (ecs : List Usize) (i : Nat) : Nat := (nvC l.edges.val).getD ((nvU ecs).getD i 0) 0

/-- An edge as the loop writes it at step `p`, headed by `h`. -/
structure EdgeOK (l : Ledger) (ecs : List Usize) (ate : List (alloc.vec.Vec U32)) (hds : List U32)
    (Rt : Nat → Nat) (p h : Nat) (X : OutEdge) : Prop where
  cells : nvC X.cells.val = ((List.range p).filter fun i => Rt i = h).map (ECI l ecs)
  faces : nvC X.faces.val = (FE ate ecs h).map fun f => (nv hds).getD f 0

/-- What the edges loop keeps (as `FInv`). -/
structure EInv (l : Ledger) (ecs : List Usize) (ate : List (alloc.vec.Vec U32)) (hds : List U32)
    (Rt : Nat → Nat) (n pf pq : Nat) (edge_of : List U32) (edges : List OutEdge) : Prop where
  fol : edge_of.length = n
  fo : ∀ i, i < pq → (edge_of.getD i NONE).val = (hdL Rt pq).idxOf (Rt i)
  fl : edges.length = (hdL Rt pq).length
  fc : ∀ f (hf : f < edges.length), EdgeOK l ecs ate hds Rt pf ((hdL Rt pq).getD f 0) edges[f]

/-- A new head at `p`: a new edge, numbered after the others. -/
theorem u_einv_head {l : Ledger} {ecs : List Usize} {ate : List (alloc.vec.Vec U32)} {hds : List U32} {Rt : Nat → Nat} {n p : Nat}
    (hRt : ∀ i, i < n → Rt i ≤ i ∧ Rt (Rt i) = Rt i) (hpn : p < n)
    {fo : List U32} {fs : List OutEdge} (hI : EInv l ecs ate hds Rt n p p fo fs) (hRp : Rt p = p)
    (F0 : OutEdge) (hF0f : nvC F0.cells.val = [])
    (hF0g : nvC F0.faces.val = (FE ate ecs p).map fun f => (nv hds).getD f 0)
    (i1 : U32) (hi1 : i1.val = fs.length) :
    EInv l ecs ate hds Rt n p (p + 1) (fo.set p i1) (fs ++ [F0]) := by
  have hsucc : hdL Rt (p + 1) = hdL Rt p ++ [p] := by rw [u_hdL_succ, if_pos hRp]
  have hpn' : p ∉ hdL Rt p := fun h => by have := (u_mem_hdL.mp h).1; omega
  have hfo := hI.fol
  refine ⟨by simp [hI.fol], ?_, ?_, ?_⟩
  · intro i hi
    rw [hsucc]
    by_cases hip : i = p
    · subst hip
      rw [u_getD_set_eq (by omega), hi1, hI.fl, hRp, List.idxOf_append_of_notMem hpn']; simp
    · have hil : i < p := by omega
      rw [u_getD_set_ne hip, hI.fo i hil, List.idxOf_append_of_mem (u_root_mem_hdL hRt (by omega) hil)]
  · rw [hsucc]; simp [hI.fl]
  · intro f hf
    rw [hsucc]
    by_cases hfl : f < fs.length
    · rw [List.getElem_append_left hfl, u_getD_append_left (by rw [← hI.fl]; exact hfl)]
      exact hI.fc f hfl
    · have : f = fs.length := by simp at hf; omega
      subst this
      rw [List.getElem_append_right (le_refl _)]
      simp only [Nat.sub_self, List.getElem_cons_zero]
      have hg : (hdL Rt p ++ [p]).getD fs.length 0 = p := by
        rw [hI.fl]; simp [List.getD_eq_getElem?_getD]
      rw [hg]
      refine ⟨?_, hF0g⟩
      rw [hF0f]
      symm; rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
      intro i hi; simp at hi ⊢
      have := (hRt i (by omega)).1; omega

/-- A position that is not a head: the face number of its root. -/
theorem u_einv_non {l : Ledger} {ecs : List Usize} {ate : List (alloc.vec.Vec U32)} {hds : List U32} {Rt : Nat → Nat} {n p : Nat}
    (hRt : ∀ i, i < n → Rt i ≤ i ∧ Rt (Rt i) = Rt i) (hpn : p < n)
    {fo : List U32} {fs : List OutEdge} (hI : EInv l ecs ate hds Rt n p p fo fs) (hRp : Rt p ≠ p)
    (i1 : U32) (hi1 : i1.val = (fo.getD (Rt p) NONE).val) :
    EInv l ecs ate hds Rt n p (p + 1) (fo.set p i1) fs := by
  have hsucc : hdL Rt (p + 1) = hdL Rt p := by rw [u_hdL_succ, if_neg hRp]; simp
  have hlt : Rt p < p := by have := (hRt p hpn).1; omega
  refine ⟨by simp [hI.fol], ?_, by rw [hsucc]; exact hI.fl, by rw [hsucc]; exact hI.fc⟩
  intro i hi
  rw [hsucc]
  by_cases hip : i = p
  · subst hip
    rw [u_getD_set_eq (by rw [hI.fol]; exact hpn), hi1, hI.fo _ hlt, (hRt i hpn).2]
  · rw [u_getD_set_ne hip, hI.fo i (by omega)]

/-- The step's tail: the edge cell joins its edge. -/
theorem u_einv_tail {l : Ledger} {ecs : List Usize} {ate : List (alloc.vec.Vec U32)} {hds : List U32} {Rt : Nat → Nat} {n p : Nat}
    (hRt : ∀ i, i < n → Rt i ≤ i ∧ Rt (Rt i) = Rt i) (hpn : p < n)
    {fo : List U32} {fs : List OutEdge} (hI : EInv l ecs ate hds Rt n p (p + 1) fo fs) :
    (fo.getD p NONE).val < fs.length ∧
    ∀ (F' : OutEdge) (hf : (fo.getD p NONE).val < fs.length),
      nvC F'.cells.val = nvC (fs[(fo.getD p NONE).val]'hf).cells.val ++ [ECI l ecs p] →
      F'.faces = (fs[(fo.getD p NONE).val]'hf).faces →
      EInv l ecs ate hds Rt n (p + 1) (p + 1) fo (fs.set (fo.getD p NONE).val F') := by
  have hmem := u_root_mem_hdL hRt hpn (Nat.lt_succ_self p)
  have hf0 : (fo.getD p NONE).val = (hdL Rt (p + 1)).idxOf (Rt p) := hI.fo p (by omega)
  have hflt : (fo.getD p NONE).val < fs.length := by rw [hf0, hI.fl]; exact u_idxOf_lt hmem
  refine ⟨hflt, fun F' hf hF1 hF2 => ?_⟩
  refine ⟨hI.fol, hI.fo, by simp [hI.fl], ?_⟩
  intro f hfl
  have hfl' : f < fs.length := by simpa using hfl
  have hfac := hI.fc f hfl'
  rw [List.getElem_set]
  have hupd : ∀ hh, ((List.range (p + 1)).filter fun i => Rt i = hh) =
      ((List.range p).filter fun i => Rt i = hh) ++ (if Rt p = hh then [p] else []) := by
    intro hh; rw [u_filter_range_succ]; by_cases e : Rt p = hh <;> simp [e]
  split
  · rename_i e
    subst e
    have hg : (hdL Rt (p + 1)).getD (fo.getD p NONE).val 0 = Rt p := by rw [hf0]; exact u_getD_idxOf hmem
    have hfac' := hI.fc _ hf
    rw [hg] at hfac' ⊢
    refine ⟨?_, by rw [hF2]; exact hfac'.faces⟩
    rw [hF1, hfac'.cells, hupd, if_pos rfl]; simp
  · rename_i e
    have hne : Rt p ≠ (hdL Rt (p + 1)).getD f 0 := by
      intro h'; apply e
      rw [hf0, h', u_idxOf_getD (u_hdL_nodup _ _) (by rw [← hI.fl]; exact hfl')]
    refine ⟨?_, hfac.faces⟩
    rw [hfac.cells, hupd, if_neg hne]; simp

theorem u_fcs_loop_spec (heads fs : alloc.vec.Vec U32) (hfs : ∀ f ∈ fs.val, f.val < heads.val.length)
    (fcs : alloc.vec.Vec CellId) (q : Usize) (hq : q.val ≤ fs.val.length)
    (hacc : nvC fcs.val = ((nv fs.val).take q.val).map fun f => (nv heads.val).getD f 0) :
    unify_loop7_loop0 heads fs fcs q ⦃ r => nvC r.val = (nv fs.val).map fun f => (nv heads.val).getD f 0 ⦄ := by
  unfold unify_loop7_loop0
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec CellId × Usize) => fs.val.length - st.2.val)
    (fun st => st.2.val ≤ fs.val.length ∧ nvC st.1.val = ((nv fs.val).take st.2.val).map fun f => (nv heads.val).getD f 0)
  · rintro ⟨c, j⟩ ⟨hj, hc⟩
    unfold unify_loop7_loop0.body
    simp only at hj hc ⊢
    split
    · rename_i hlt
      have hjl : j.val < fs.val.length := by scalar_tac
      have hjl' : j.val < (nv fs.val).length := by simp [nv]; omega
      have hcl : c.val.length ≤ j.val := by
        have := congrArg List.length hc; simp only [nvC, nv, List.length_map, List.length_take] at this; omega
      step as ⟨x, hx⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = x.val := by rw [hi2]; scalar_tac
      have hxl : x.val < heads.val.length := by rw [hx]; exact hfs _ (List.getElem_mem _)
      step as ⟨y, hy⟩
      step as ⟨c1, hc1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [show j1.val = j.val + 1 by scalar_tac, List.take_succ_eq_append_getElem hjl', List.map_append, ← hc, hc1]
      have hxl' : (fs.val[j.val]).val < heads.val.length := hx ▸ hxl
      simp [nvC, nv, hy, hi2v, hx, List.getD_eq_getElem?_getD, hxl']
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = fs.val.length := by scalar_tac
      rw [hc, this, ← List.length_map (f := fun (u : U32) => u.val), List.take_length]
  · exact ⟨hq, hacc⟩

/-- **The edges loop.** -/
theorem u_edges_loop_spec (l : Ledger) (heads : alloc.vec.Vec U32) (ate : alloc.vec.Vec (alloc.vec.Vec U32))
    (ecs : alloc.vec.Vec Usize) (hM : ecs.val.length < U32.max)
    (hecs : ∀ x ∈ ecs.val, x.val < ate.val.length) (hecsl : ∀ x ∈ ecs.val, x.val < l.edges.val.length)
    (hfs : ∀ x, x < ecs.val.length → ∀ f ∈ FE ate.val ecs.val x, f < heads.val.length)
    (Rt : Nat → Nat)
    (hRt : ∀ i, i < ecs.val.length → Rt i < ecs.val.length ∧ Rt i ≤ i ∧ Rt (Rt i) = Rt i)
    (uf : union_find.Uf) (edges : alloc.vec.Vec OutEdge) (edge_of : alloc.vec.Vec U32)
    (p : Usize) (hp : p.val ≤ ecs.val.length) (hu : UfR Rt ecs.val.length uf)
    (hI : EInv l ecs.val ate.val heads.val Rt ecs.val.length p.val p.val edge_of.val edges.val) :
    unify_loop7 l.edges heads ate ecs uf edges edge_of p ⦃ r =>
      ∃ eo : List U32, EInv l ecs.val ate.val heads.val Rt ecs.val.length ecs.val.length ecs.val.length eo r.val ⦄ := by
  have hRt' : ∀ i, i < ecs.val.length → Rt i ≤ i ∧ Rt (Rt i) = Rt i := fun i hi => (hRt i hi).2
  unfold unify_loop7
  apply loop.spec_decr_nat
    (fun (st : union_find.Uf × alloc.vec.Vec OutEdge × alloc.vec.Vec U32 × Usize) => ecs.val.length - st.2.2.2.val)
    (fun st => st.2.2.2.val ≤ ecs.val.length ∧ UfR Rt ecs.val.length st.1 ∧
      EInv l ecs.val ate.val heads.val Rt ecs.val.length st.2.2.2.val st.2.2.2.val st.2.2.1.val st.2.1.val)
  · rintro ⟨u0, es0, eo0, j⟩ ⟨hj, ⟨hg0, hl0, hr0⟩, hI0⟩
    unfold unify_loop7.body
    simp only at hj hI0 hg0 hl0 hr0 ⊢
    split
    · rename_i hlt
      have hjn : j.val < ecs.val.length := by scalar_tac
      obtain ⟨hRn, hRle, hRR⟩ := hRt j.val hjn
      step with find_spec u0 j (by omega) hg0 as ⟨r, u1, hr, hl1, hg1, hr1⟩
      have hrv : r.val = Rt j.val := by rw [hr, hr0]
      have hu1 : UfR Rt ecs.val.length u1 := ⟨hg1, by omega, fun y => by rw [hr1, hr0]⟩
      have hfol := hI0.fol
      have hesl : es0.val.length ≤ j.val := by
        rw [hI0.fl]; exact (List.length_filter_le _ _).trans (by simp)
      have hjfo : j.val < eo0.val.length := by omega
      split
      · rename_i hrj
        have hRj : Rt j.val = j.val := by rw [← hrv, hrj]
        have hesl' : (alloc.vec.Vec.len es0).val = es0.val.length := by simp
        step as ⟨w0, back0, hw0, hback0⟩
        subst hback0
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = es0.val.length := by rw [hi2, u_cast_u32 (by omega), hesl']
        step as ⟨i3, hi3⟩
        have hi3l : i3.val < ate.val.length := by rw [hi3]; exact hecs _ (List.getElem_mem _)
        step as ⟨fs, hfsv⟩
        have hfsn : nv fs.val = FE ate.val ecs.val j.val := by
          rw [hfsv, FE, List.getD_eq_getElem?_getD,
            show (nvU ecs.val).getD j.val 0 = i3.val by simp [nvU, List.getD_eq_getElem?_getD, hjn, hi3],
            List.getElem?_eq_getElem hi3l]; rfl
        step with u_fcs_loop_spec heads fs (fun f hf => hfs j.val hjn f.val (by rw [← hfsn]; exact List.mem_map_of_mem hf))
          (alloc.vec.Vec.new CellId) 0#usize (by simp) (by simp [nvC]) as ⟨fcs, hfcs⟩
        have hesl2 : es0.val.length < Usize.max := by have := u32_le_usize; omega
        step as ⟨ES, hES⟩
        have hM : EInv l ecs.val ate.val heads.val Rt ecs.val.length j.val (j.val + 1) (eo0.val.set j.val i2) ES.val := by
          rw [hES]
          exact u_einv_head hRt' hjn hI0 hRj _ (by simp [nvC]) (by rw [hfcs, hfsn]) i2 hi2v
        rw [← alloc.vec.Vec.set_val_eq] at hM
        obtain ⟨hflt, hupd⟩ := u_einv_tail hRt' hjn hM
        have hfo1 := hM.fol
        step as ⟨fi, hfi⟩
        have hfiv : fi.val = ((eo0.set j i2).val.getD j.val NONE).val := by
          rw [hfi, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
        step as ⟨fi1, hfi1⟩
        have hfi1v : fi1.val = fi.val := by rw [hfi1]; scalar_tac
        have hfl : fi1.val < ES.val.length := by rw [hfi1v, hfiv]; exact hflt
        step as ⟨oe, back, hoe, hback⟩
        step as ⟨i6, hi6⟩
        have hi6l : i6.val < l.edges.val.length := by rw [hi6]; exact hecsl _ (List.getElem_mem _)
        step as ⟨ci, hci⟩
        have hciv : (ci : U32).val = ECI l ecs.val j.val := by
          rw [hci, ECI, List.getD_eq_getElem?_getD,
            show (nvU ecs.val).getD j.val 0 = i6.val by simp [nvU, List.getD_eq_getElem?_getD, hjn, hi6]]
          simp [nvC, hi6l]
        have hoee : oe = ES.val[((eo0.set j i2).val.getD j.val NONE).val]'hflt := by
          rw [hoe]; simp [hfi1v, hfiv]
        have hoel : oe.cells.val.length < Usize.max := by
          have hfc := congrArg List.length (hM.fc _ hflt).cells
          simp only [nvC, List.length_map] at hfc
          rw [hoee, hfc]
          exact lt_of_le_of_lt (u_filter_range_len _ _) (by have := u32_le_usize; omega)
        step as ⟨v2, hv2⟩
        step as ⟨j1, hj1⟩
        refine ⟨by rw [hj1]; omega, hu1, ?_, by rw [hj1]; omega⟩
        rw [hj1, hback, alloc.vec.Vec.set_val_eq _ fi1, hfi1v, hfiv]
        apply hupd _ hflt
        · rw [← hoee]; simp [nvC, hv2, hciv]
        · rw [← hoee]
      · rename_i hrj
        have hRj : Rt j.val ≠ j.val := by intro e; apply hrj; apply UScalar.eq_of_val_eq; rw [hrv, e]
        have hrl : r.val < eo0.val.length := by omega
        step as ⟨i0, hi0⟩
        step as ⟨w0, back0, hw0, hback0⟩
        subst hback0
        have hM : EInv l ecs.val ate.val heads.val Rt ecs.val.length j.val (j.val + 1) (eo0.val.set j.val i0) es0.val := by
          refine u_einv_non hRt' hjn hI0 hRj i0 ?_
          rw [hi0, ← hrv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hrl]; rfl
        rw [← alloc.vec.Vec.set_val_eq] at hM
        obtain ⟨hflt, hupd⟩ := u_einv_tail hRt' hjn hM
        have hfo1 := hM.fol
        step as ⟨fi, hfi⟩
        have hfiv : fi.val = ((eo0.set j i0).val.getD j.val NONE).val := by
          rw [hfi, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; rfl
        step as ⟨fi1, hfi1⟩
        have hfi1v : fi1.val = fi.val := by rw [hfi1]; scalar_tac
        have hfl : fi1.val < es0.val.length := by rw [hfi1v, hfiv]; exact hflt
        step as ⟨oe, back, hoe, hback⟩
        step as ⟨i6, hi6⟩
        have hi6l : i6.val < l.edges.val.length := by rw [hi6]; exact hecsl _ (List.getElem_mem _)
        step as ⟨ci, hci⟩
        have hciv : (ci : U32).val = ECI l ecs.val j.val := by
          rw [hci, ECI, List.getD_eq_getElem?_getD,
            show (nvU ecs.val).getD j.val 0 = i6.val by simp [nvU, List.getD_eq_getElem?_getD, hjn, hi6]]
          simp [nvC, hi6l]
        have hoee : oe = es0.val[((eo0.set j i0).val.getD j.val NONE).val]'hflt := by
          rw [hoe]; simp [hfi1v, hfiv]
        have hoel : oe.cells.val.length < Usize.max := by
          have hfc := congrArg List.length (hM.fc _ hflt).cells
          simp only [nvC, List.length_map] at hfc
          rw [hoee, hfc]
          exact lt_of_le_of_lt (u_filter_range_len _ _) (by have := u32_le_usize; omega)
        step as ⟨v2, hv2⟩
        step as ⟨j1, hj1⟩
        refine ⟨by rw [hj1]; omega, hu1, ?_, by rw [hj1]; omega⟩
        rw [hj1, hback, alloc.vec.Vec.set_val_eq _ fi1, hfi1v, hfiv]
        apply hupd _ hflt
        · rw [← hoee]; simp [nvC, hv2, hciv]
        · rw [← hoee]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = ecs.val.length := by scalar_tac
      rw [this] at hI0; exact ⟨eo0.val, hI0⟩
  · exact ⟨hp, hu, hI⟩

end forge_bool_core
