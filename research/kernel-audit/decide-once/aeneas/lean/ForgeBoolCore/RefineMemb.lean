import ForgeBoolCore.RefineLedger
open Aeneas Aeneas.Std Result

/-!
# `membership` is the Lean core's `memb`

Loop 0 tables the witness answers (`inside[b][c]`); loop 1 reads, for each body and tope, the
answer at the tope's flood root, which is the Lean's `head` (`flood_spec`, `floodEdges_eq`,
`head_least`).
-/

namespace forge_bool_core

open ForgeExact.DecideOnce ForgeBoolCore.Dedup ForgeBoolCore.UF

/-- The witness answers as `(cell id, body)`. -/
def ipcs (l : Ledger) : List (Nat × Nat) := l.inside.val.map fun q => ((q.1 : U32).val, (q.2 : U32).val)

def M0Inv (l : Ledger) (nb N : Nat) (PO : List (alloc.vec.Vec Bool)) (k : Nat) : Prop :=
  PO.length = nb ∧ (∀ b, b < nb → (PO.getD b default).val.length = N) ∧
  ∀ b, b < nb → ∀ c, c < N → PAt PO b c = ((ipcs l).take k).any (fun q => decide (q.1 = c ∧ q.2 = b))

theorem memb_loop0_spec (l : Ledger) (ext : Ext) (nb : Usize) (hnb : nb.val = l.nb.val)
    (hN : (allCells l ext).length ≤ Usize.max)
    (PO : alloc.vec.Vec (alloc.vec.Vec Bool)) (k : Usize) (hk : k.val ≤ l.inside.val.length)
    (hinv : M0Inv l nb.val (allCells l ext).length PO.val k.val) :
    membership_loop0 l.nb l.face_body l.face_class l.cells l.pieces l.inside l.edges l.verts ext nb PO k ⦃ r =>
      M0Inv l nb.val (allCells l ext).length r.val l.inside.val.length ⦄ := by
  have hpl : (ipcs l).length = l.inside.val.length := by simp [ipcs]
  unfold membership_loop0
  apply loop.spec_decr_nat (fun (q : alloc.vec.Vec (alloc.vec.Vec Bool) × Usize) => l.inside.val.length - q.2.val)
    (fun q => q.2.val ≤ l.inside.val.length ∧ M0Inv l nb.val (allCells l ext).length q.1.val q.2.val)
  · rintro ⟨T, j⟩ ⟨hj, hl1, hl2, hl3⟩
    unfold membership_loop0.body
    simp only at hj hl1 hl2 hl3 ⊢
    split
    · rename_i hlt
      have hjl : j.val < l.inside.val.length := by scalar_tac
      step as ⟨c, b, hcb⟩
      have hpj : (ipcs l)[j.val]'(by omega) = ((c : U32).val, (b : U32).val) := by simp [ipcs, ← hcb]
      have htake : (ipcs l).take (j.val + 1) = (ipcs l).take j.val ++ [((c : U32).val, (b : U32).val)] := by
        rw [List.take_succ, List.getElem?_eq_getElem (by omega), hpj]; rfl
      have hex : ∀ c' b', ((ipcs l).take (j.val + 1)).any (fun q => decide (q.1 = c' ∧ q.2 = b')) =
            (((ipcs l).take j.val).any (fun q => decide (q.1 = c' ∧ q.2 = b')) ||
              decide ((c : U32).val = c' ∧ (b : U32).val = b')) := by
        intro c' b'; rw [htake, List.any_append]; simp
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = (b : U32).val := by rw [hi2]; scalar_tac
      split
      · rename_i hbn
        have hbl : (b : U32).val < nb.val := by scalar_tac
        step as ⟨i3, hi3⟩
        have hi3v : i3.val = (c : U32).val := by rw [hi3]; scalar_tac
        step with ncells_spec l ext hN as ⟨i4, hi4⟩
        split
        · rename_i hcn
          have hcl : (c : U32).val < (allCells l ext).length := by scalar_tac
          step as ⟨i5, hi5⟩
          have hi5v : i5.val = (b : U32).val := by rw [hi5]; scalar_tac
          have hbT : i5.val < T.val.length := by rw [hl1]; omega
          step as ⟨pr, hpr1, hpr2⟩
          obtain ⟨v7, back⟩ := pr
          simp only at hpr1 hpr2 ⊢
          have hv7 : v7 = T.val.getD (b : U32).val default := by
            rw [hpr1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; simp [hi5v]
          have hv7l : (c : U32).val < v7.val.length := by rw [hv7, hl2 _ hbl]; exact hcl
          step as ⟨i6, hi6⟩
          have hi6v : i6.val = (c : U32).val := by rw [hi6]; scalar_tac
          step as ⟨pr1, hq1, hq2⟩
          obtain ⟨w0, back1⟩ := pr1
          simp only at hq1 hq2 ⊢
          step as ⟨j1, hj1⟩
          have hnew : (back (back1 true)).val = T.val.set (b : U32).val (v7.set i6 true) := by
            rw [hpr2, hq2]; simp [hi5v]
          refine ⟨by scalar_tac, ⟨by rw [hnew]; simp [hl1], fun b' hb' => ?_, fun b' hb' c' hc' => ?_⟩, by scalar_tac⟩
          · rw [hnew, getD_set_any _ _ _ _ _ (by omega)]
            split
            · rename_i e; subst e; simp only [alloc.vec.Vec.set_val_eq, List.length_set]; rw [hv7, hl2 _ hbl]
            · exact hl2 b' hb'
          · rw [hj1, hex c' b', ← hl3 b' hb' c' hc', PAt, PAt, hnew, getD_set_any _ _ _ _ _ (by omega)]
            by_cases e : b' = (b : U32).val
            · subst e
              rw [if_pos rfl]
              simp only [alloc.vec.Vec.set_val_eq, hi6v]
              rw [getD_set_any _ _ _ _ _ hv7l, ← hv7]
              by_cases ec : c' = (c : U32).val
              · subst ec; simp
              · have ec' : ¬(c : U32).val = c' := fun h => ec h.symm
                simp [ec, ec']
            · rw [if_neg e]
              have : ¬(b : U32).val = b' := fun h => e h.symm
              simp [this]
        · rename_i hcn
          have hcl : (allCells l ext).length ≤ (c : U32).val := by scalar_tac
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ⟨hl1, hl2, fun b' hb' c' hc' => ?_⟩, by scalar_tac⟩
          rw [hj1, hex c' b', ← hl3 b' hb' c' hc']
          have : ¬(c : U32).val = c' := by omega
          simp [this]
      · rename_i hbn
        have hbl : nb.val ≤ (b : U32).val := by scalar_tac
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ⟨hl1, hl2, fun b' hb' c' hc' => ?_⟩, by scalar_tac⟩
        rw [hj1, hex c' b', ← hl3 b' hb' c' hc']
        have : ¬(b : U32).val = b' := by omega
        simp [this]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = l.inside.val.length := by scalar_tac
      rw [this] at hl3
      exact ⟨hl1, hl2, hl3⟩
  · exact ⟨hk, hinv⟩

/-- Body `i`'s row: at each tope, the witness answer at its flood root. -/
def rowOf (s : Split) (PO : List (alloc.vec.Vec Bool)) (i : Nat) (R : Nat → Nat) (t : Nat) : Bool :=
  PAt PO i (at32 s.topes (R t))

theorem memb_row_spec (s : Split) (PO : alloc.vec.Vec (alloc.vec.Vec Bool)) (i : Usize)
    (hi : i.val < PO.val.length) (N : Nat) (hrow : (PO.val.getD i.val default).val.length = N)
    (htn : ∀ t, t < s.topes.val.length → at32 s.topes t < N)
    (uf0 uf : union_find.Uf) (hg0 : Good (vals uf0.parent))
    (hl0 : uf0.parent.val.length = s.topes.val.length)
    (row : alloc.vec.Vec Bool) (t : Usize) (ht : t.val ≤ s.topes.val.length)
    (hg : Good (vals uf.parent)) (hl : uf.parent.val.length = s.topes.val.length)
    (hr : ∀ y, ufRoot (vals uf.parent) y = ufRoot (vals uf0.parent) y)
    (hrow0 : row.val = (List.range t.val).map (rowOf s PO.val i.val (ufRoot (vals uf0.parent)))) :
    membership_loop1_loop0 s PO i uf row t ⦃ r =>
      ({ facets := r.1, plus := r.2.1, minus := r.2.2.1, topes := r.2.2.2.1, tope_pos := r.2.2.2.2.1,
         first_face := r.2.2.2.2.2.1, piece_of := r.2.2.2.2.2.2.1 } : Split) = s ∧
      r.2.2.2.2.2.2.2.val = (List.range s.topes.val.length).map (rowOf s PO.val i.val (ufRoot (vals uf0.parent))) ⦄ := by
  unfold membership_loop1_loop0
  apply loop.spec_decr_nat (fun (q : union_find.Uf × alloc.vec.Vec Bool × Usize) => s.topes.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ s.topes.val.length ∧ Good (vals q.1.parent) ∧
      q.1.parent.val.length = s.topes.val.length ∧
      (∀ y, ufRoot (vals q.1.parent) y = ufRoot (vals uf0.parent) y) ∧
      q.2.1.val = (List.range q.2.2.val).map (rowOf s PO.val i.val (ufRoot (vals uf0.parent))))
  · rintro ⟨u, rw0, j⟩ ⟨hj, hgu, hlu, hru, hrw⟩
    unfold membership_loop1_loop0.body
    simp only at hj hgu hlu hru hrw ⊢
    split
    · rename_i hlt
      have hjl : j.val < s.topes.val.length := by scalar_tac
      step with find_spec u j (by omega) hgu as ⟨r, u1, hr1, hr2, hr3, hr4⟩
      have hrv : r.val = ufRoot (vals uf0.parent) j.val := by rw [hr1, hru]
      have hrl : r.val < s.topes.val.length := by
        have := root_lt hgu (x := j.val) (by rw [vals_length]; omega)
        rw [vals_length] at this; rw [hr1]; omega
      step as ⟨v, hv⟩
      step as ⟨i2, hi2⟩
      have hi2v : i2.val = at32 s.topes r.val := by rw [at32_getD hrl, hi2]
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = at32 s.topes r.val := by rw [hi3, ← hi2v]; scalar_tac
      have hvl : i3.val < v.val.length := by
        have hv' : v = PO.val.getD i.val default := by
          rw [hv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi]; rfl
        rw [hv', hrow, hi3v]; exact htn _ hrl
      step as ⟨b, hb⟩
      have hrwl : rw0.val.length < Usize.max := by rw [hrw]; simp; scalar_tac
      step as ⟨row1, hrow1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, hr3, by rw [hr2, hlu], fun y => by rw [hr4, hru], ?_, by scalar_tac⟩
      rw [hrow1, hj1, List.range_succ, List.map_append, ← hrw]
      simp only [List.map_cons, List.map_nil]
      congr 1; congr 1
      rw [hb, rowOf, PAt, ← hrv]
      have hv' : v = PO.val.getD i.val default := by
        rw [hv, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi]; rfl
      rw [← hv', ← hi3v, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hvl]; rfl
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = s.topes.val.length := by scalar_tac
      rw [← this]; exact ⟨trivial, hrw⟩
  · exact ⟨ht, hg, hl, hr, hrow0⟩

/-- The rows of the first `i` bodies, each from its flood. -/
def MRows (s : Split) (PO : List (alloc.vec.Vec Bool)) (memb : List (alloc.vec.Vec Bool)) (i : Nat) : Prop :=
  memb.length = i ∧ ∀ b, b < i → ∃ R, Least (floodEdges s b s.facets.val.length) s.topes.val.length R ∧
    (memb.getD b default).val = (List.range s.topes.val.length).map (rowOf s PO b R)

theorem memb_loop1_spec (s : Split) (nb : Usize) (PO : alloc.vec.Vec (alloc.vec.Vec Bool)) (N : Nat)
    (hPO : PO.val.length = nb.val) (hrow : ∀ b, b < nb.val → (PO.val.getD b default).val.length = N)
    (htn : ∀ t, t < s.topes.val.length → at32 s.topes t < N) (hwf : ∀ b, b < nb.val → SplitWF s b)
    (memb : alloc.vec.Vec (alloc.vec.Vec Bool)) (i : Usize) (hi : i.val ≤ nb.val)
    (hm : MRows s PO.val memb.val i.val) :
    membership_loop1 s nb PO memb i ⦃ r => MRows s PO.val r.val nb.val ⦄ := by
  unfold membership_loop1
  apply loop.spec_decr_nat (fun (q : Split × alloc.vec.Vec (alloc.vec.Vec Bool) × Usize) => nb.val - q.2.2.val)
    (fun q => q.1 = s ∧ q.2.2.val ≤ nb.val ∧ MRows s PO.val q.2.1.val q.2.2.val)
  · rintro ⟨s1, m, j⟩ ⟨rfl, hj, hml, hmr⟩
    unfold membership_loop1.body
    simp only at hj hml hmr ⊢
    split
    · rename_i hlt
      have hjl : j.val < nb.val := by scalar_tac
      step with flood_spec s1 j (hwf j.val hjl) as ⟨uf, huf⟩
      obtain ⟨hg, hl, hle⟩ := huf
      rw [vals_length] at hl
      step with memb_row_spec s1 PO j (by omega) N (hrow j.val hjl) htn uf uf hg hl (alloc.vec.Vec.new Bool)
        0#usize (by simp) hg hl (fun _ => rfl) (by simp) as ⟨a1, a2, a3, a4, a5, a6, a7, row, hs1, hrw⟩
      rw [hs1]
      have hml' : m.val.length < Usize.max := by rw [hml]; scalar_tac
      step as ⟨m1, hm1⟩
      step as ⟨j1, hj1⟩
      refine ⟨by scalar_tac, ⟨by rw [hm1]; simp [hml, hj1], fun b hb => ?_⟩, by scalar_tac⟩
      rw [hj1] at hb
      by_cases e : b = j.val
      · subst e
        refine ⟨ufRoot (vals uf.parent), hle, ?_⟩
        rw [hm1, List.getD_eq_getElem?_getD, List.getElem?_append_right (by omega)]
        simp [hml, hrw]
      · obtain ⟨R, hR, hRe⟩ := hmr b (by omega)
        refine ⟨R, hR, ?_⟩
        rw [hm1, List.getD_eq_getElem?_getD, List.getElem?_append_left (by omega), ← List.getD_eq_getElem?_getD, hRe]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = nb.val := by scalar_tac
      rw [← this]; exact ⟨hml, hmr⟩
  · exact ⟨rfl, hi, hm⟩

/-! ## The row is the Lean's `memb` -/

theorem topes_nodup_toL (l : Ledger) : ((toL l).topes).Nodup := nodup_dedup _

theorem crossings_mem (l : Ledger) (i : Nat) :
    ∀ p ∈ (toL l).crossings i, p.1 ∈ (toL l).topes ∧ p.2 ∈ (toL l).topes := by
  intro p hp
  unfold ForgeExact.DecideOnce.Ledger.crossings at hp
  obtain ⟨φ, hφ, rfl⟩ := List.mem_map.mp hp
  have hφ' := (List.mem_filter.mp hφ).1
  unfold ForgeExact.DecideOnce.Ledger.topes
  exact ⟨(mem_dedup _ _).mpr (List.mem_flatMap.mpr ⟨φ, hφ', by simp⟩),
    (mem_dedup _ _).mpr (List.mem_flatMap.mpr ⟨φ, hφ', by simp⟩)⟩

/-- Witness answers name ledger cells. -/
def InsideWF (l : Ledger) : Prop := ∀ q ∈ ipcs l, q.1 < l.cells.val.length

/-- **Body `b`'s row at tope `t` is the Lean's `memb b T`.** -/
theorem memb_eq (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hok0 : ExtOK l e0) (hs : SplitOK l e0 s e)
    (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) (hiw : InsideWF l)
    (PO : List (alloc.vec.Vec Bool))
    (hPO : ∀ b, b < l.nb.val → ∀ c, c < (allCells l e).length →
      PAt PO b c = (ipcs l).any (fun q => decide (q.1 = c ∧ q.2 = b)))
    {b : Nat} (hb : b < l.nb.val) {R : Nat → Nat}
    (hR : Least (floodEdges s b s.facets.val.length) s.topes.val.length R)
    {t : Nat} (ht : t < s.topes.val.length) :
    rowOf s PO b R t = (toL l).memb b ((toL l).topes[t]'(by rw [← topes_toL l e0 s e hok0 hs hpc]; simpa using ht)) := by
  have hTL := topes_toL l e0 s e hok0 hs hpc
  have hlen : (toL l).topes.length = s.topes.val.length := by rw [← hTL]; simp
  rw [floodEdges_eq l e0 s e hok0 hs hpc hb] at hR
  rw [← hlen] at hR ht
  unfold ForgeExact.DecideOnce.Ledger.memb
  simp only
  rw [head_least (topes_nodup_toL l) (crossings_mem l b) hR ht]
  simp only
  obtain ⟨hRl, _, _⟩ := hR t ht
  have hT := hs.topes
  have hidl : at32 s.topes (R t) < (allCells l e).length := by
    rw [at32_getD (by omega)]
    exact hT.2.2.1 _ ((mem_dedup _ _).mp (by rw [← hT.2.1]; exact List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩))
  have hcov : (toL l).topes[R t]'hRl = covE l e (at32 s.topes (R t)) := by
    simp only [← hTL, List.getElem_map]; rw [at32_getD (by omega)]
  rw [hcov, rowOf, hPO b hb _ hidl]
  unfold toL
  apply Bool.eq_iff_iff.mpr
  simp only [List.any_eq_true, decide_eq_true_eq, List.elem_iff, List.mem_map]
  constructor
  · rintro ⟨q, hq, h1, h2⟩
    obtain ⟨q', hq', rfl⟩ := List.mem_map.mp hq
    refine ⟨q', hq', ?_⟩
    simp only [Prod.mk.injEq]
    exact ⟨by rw [← h1, cellL_covAt l e (hiw _ hq)]; rfl, h2⟩
  · rintro ⟨q', hq', he⟩
    simp only [Prod.mk.injEq] at he
    have hq : ((q'.1 : U32).val, (q'.2 : U32).val) ∈ ipcs l := List.mem_map.mpr ⟨q', hq', rfl⟩
    have hql := hiw _ hq
    refine ⟨_, hq, ?_, he.2⟩
    have := he.1
    rw [cellL_covAt l e hql] at this
    exact covE_inj l e hs.ext (by rw [allCells_length]; simp at hql; omega) hidl this

/-- **`membership` is the Lean core's `memb`**, for every body and tope. -/
theorem membership_spec (l : Ledger) (e0 : Ext) (s : Split) (e : Ext) (hok0 : ExtOK l e0)
    (hs : SplitOK l e0 s e) (hpc : ∀ q ∈ pcs l, q.1 < l.cells.val.length) (hiw : InsideWF l)
    (hroom : 2 * s.facets.val.length < U32.max) :
    membership l e s ⦃ m => m.val.length = l.nb.val ∧
      (∀ b, b < l.nb.val → (m.val.getD b default).val.length = s.topes.val.length) ∧
      ∀ b, b < l.nb.val → ∀ t (ht : t < s.topes.val.length),
      (m.val.getD b default).val.getD t false =
        (toL l).memb b ((toL l).topes[t]'(by rw [← topes_toL l e0 s e hok0 hs hpc]; simpa using ht)) ⦄ := by
  unfold membership
  have hN : (allCells l e).length ≤ Usize.max := by
    have := hs.ext.small; have := u32_le_usize; omega
  step as ⟨nb, hnb⟩
  have hnbv : nb.val = l.nb.val := by rw [hnb]; scalar_tac
  step with ncells_spec l e hN as ⟨n, hn⟩
  step as ⟨v, hv1, hv2⟩
  step with alloc.vec.from_elem_spec (core.clone.CloneallocvecVec core.clone.CloneBool) v nb
    (ok_of_spec (clone_vec_bool_spec v)) as ⟨PO, hPO1, hPO2⟩
  step with memb_loop0_spec l e nb hnbv hN PO 0#usize (by simp)
    ⟨by rw [hPO1]; simp, fun b hb => by
        rw [hPO1, List.getD_eq_getElem?_getD, List.getElem?_replicate]; simp [hb, hv1, hn],
      fun b hb c hc => by
        have hb' : b < (List.replicate nb.val v).length := by simp; omega
        simp only [PAt, hPO1, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hb', List.getElem_replicate,
          Option.getD_some, hv1]
        simp⟩
    as ⟨PO1, hPO⟩
  obtain ⟨hPOl, hPOr, hPOp⟩ := hPO
  have hpl : (ipcs l).length = l.inside.val.length := by simp [ipcs]
  rw [← hpl, List.take_length] at hPOp
  have htn : ∀ t, t < s.topes.val.length → at32 s.topes t < (allCells l e).length := by
    intro t ht
    have hT := hs.topes
    rw [at32_getD ht]
    exact hT.2.2.1 _ ((mem_dedup _ _).mp (by rw [← hT.2.1]; exact List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩))
  apply WP.spec_mono (memb_loop1_spec s nb PO1 (allCells l e).length (by omega)
    (fun b hb => hPOr b (by omega)) htn (fun b hb => splitWF_of l e0 s e hs hroom (by omega))
    (alloc.vec.Vec.new (alloc.vec.Vec Bool)) 0#usize (by simp) ⟨by simp, fun b hb => by simp at hb⟩)
  rintro m ⟨hml, hmr⟩
  refine ⟨by omega, fun b hb => ?_, fun b hb t ht => ?_⟩
  · obtain ⟨R, _, hRe⟩ := hmr b (by omega)
    rw [hRe]; simp
  obtain ⟨R, hR, hRe⟩ := hmr b (by omega)
  rw [hRe, List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range ht]
  simp only [Option.map_some, Option.getD_some]
  exact memb_eq l e0 s e hok0 hs hpc hiw PO1 (fun b hb c hc => by rw [← hnbv] at hb; exact hPOp b hb c hc) hb hR ht

end forge_bool_core
