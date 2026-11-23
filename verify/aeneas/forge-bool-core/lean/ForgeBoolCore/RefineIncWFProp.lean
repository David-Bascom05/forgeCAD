import ForgeBoolCore.RefineIncWF

/-!
# Group T: `LedgerWF2` as `ledger_wf`'s checks

`ledger_wf` checks its conditions by position in each table. `LedgerWF2` states them by
membership. This file proves the two forms agree (`wf_iff`), so `ledger_wf_spec` can close with
`LedgerWF2`.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce

/-! ### Windows of a list, by position -/

theorem mem_window {β : Type} {L : List β} {lo hi : Nat} (d : β) (h1 : lo ≤ hi) (h2 : hi ≤ L.length) {x : β} :
    x ∈ (L.drop lo).take (hi - lo) ↔ ∃ t, lo ≤ t ∧ t < hi ∧ L.getD t d = x := by
  constructor
  · intro hx
    obtain ⟨i, hi', rfl⟩ := List.getElem_of_mem hx
    have hil : i < hi - lo := by simp at hi'; omega
    refine ⟨lo + i, by omega, by omega, ?_⟩
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some]
    simp
  · rintro ⟨t, ht1, ht2, rfl⟩
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some]
    have : L[t] = ((L.drop lo).take (hi - lo))[t - lo]'(by simp; omega) := by simp; congr 1; omega
    rw [this]; exact List.getElem_mem _

theorem nodup_iff_count {β : Type} [BEq β] [LawfulBEq β] (L : List β) :
    L.Nodup ↔ ∀ x ∈ L, L.count x = 1 := by
  rw [List.nodup_iff_count_le_one]
  constructor
  · intro h x hx
    have := h x
    have := List.count_pos_iff.mpr hx
    omega
  · intro h x
    by_cases hx : x ∈ L
    · rw [h x hx]
    · rw [List.count_eq_zero_of_not_mem hx]; omega

theorem nodup_map_val (L : List U32) : (L.map (·.val)).Nodup ↔ L.Nodup :=
  List.nodup_map_iff (fun _ _ h => UScalar.eq_of_val_eq h)

theorem count_one_mem {L : List U32} (hnd : L.Nodup) (x : U32) : L.count x = 1 ↔ x ∈ L := by
  constructor
  · intro h; apply List.count_pos_iff.mp; omega
  · intro h; exact (nodup_iff_count L).mp hnd x h

/-! ### Sectors: field equality is equality -/

theorem secEq_iff (y x : Sector) : secEq y x = true ↔ y = x := by
  obtain ⟨f, a, b⟩ := y
  obtain ⟨f', a', b'⟩ := x
  simp only [secEq, decide_eq_true_eq]
  constructor
  · rintro ⟨h1, h2, h3⟩
    rw [UScalar.eq_of_val_eq h1, UScalar.eq_of_val_eq h2, UScalar.eq_of_val_eq h3]
  · intro h; cases h; exact ⟨rfl, rfl, rfl⟩

theorem nodup_iff_countP (L : List Sector) :
    L.Nodup ↔ ∀ x ∈ L, L.countP (secEq · x) = 1 := by
  induction L with
  | nil => simp
  | cons y t ih =>
    rw [List.nodup_cons, ih]
    constructor
    · rintro ⟨hy, ht⟩ x hx
      rw [List.countP_cons]
      rcases List.mem_cons.mp hx with rfl | hx
      · have h0 : t.countP (secEq · x) = 0 := by
          rw [List.countP_eq_zero]
          intro z hz hzx; rw [secEq_iff] at hzx; subst hzx; exact hy hz
        simp [h0, (secEq_iff x x).mpr rfl]
      · have hne : secEq y x = false := by
          rw [Bool.eq_false_iff]; intro h; rw [secEq_iff] at h; subst h; exact hy hx
        simp [ht x hx, hne]
    · intro h
      have hy := h y (List.mem_cons_self ..)
      rw [List.countP_cons, (secEq_iff y y).mpr rfl] at hy
      simp only [ite_true] at hy
      have h0 : t.countP (secEq · y) = 0 := by omega
      refine ⟨fun hm => ?_, fun x hx => ?_⟩
      · rw [List.countP_eq_zero] at h0
        exact h0 y hm ((secEq_iff y y).mpr rfl)
      · have := h x (List.mem_cons_of_mem _ hx)
        rw [List.countP_cons] at this
        have hne : secEq y x = false := by
          rw [Bool.eq_false_iff]; intro hyx; rw [secEq_iff] at hyx; subst hyx
          rw [List.countP_eq_zero] at h0
          exact h0 y hx ((secEq_iff y y).mpr rfl)
        simpa [hne] using this


theorem mem_val (L : List U32) (x : U32) : x ∈ L ↔ x.val ∈ L.map (·.val) := by
  constructor
  · intro h; exact List.mem_map.mpr ⟨x, h, rfl⟩
  · intro h
    obtain ⟨y, hy, e⟩ := List.mem_map.mp h
    rw [← UScalar.eq_of_val_eq e]; exact hy

theorem one_le_count_iff (L : List U32) (x : U32) : 1 ≤ L.count x ↔ x ∈ L := by
  rw [← List.count_pos_iff]; omega

/-- **The checks at a vert are LedgerWF2's three vert fields.** -/
theorem vertOK_iff (l : Ledger) (hve : CsrOK l.vert_edges (nrowsL l.edge_facets)) (hsc : SecCsrOK l.vert_sectors)
    (hrows : l.vert_sectors.start.val.length = l.vert_edges.start.val.length) {v : Nat} (hv : v < nrowsL l.vert_edges) :
    VertOK l v ↔ (rowL l.vert_edges v).Nodup ∧
      (∀ s ∈ secRowL l.vert_sectors v, s.facet.val < nfacets l ∧ s.a.val ∈ rowL l.vert_edges v ∧
        s.b.val ∈ rowL l.vert_edges v ∧ s.facet.val ∈ rowL l.edge_facets s.a.val ∧
        s.facet.val ∈ rowL l.edge_facets s.b.val) ∧
      (secRowL l.vert_sectors v).Nodup := by
  have hk : v + 1 < l.vert_edges.start.val.length := by unfold nrowsL at hv; omega
  have hsk : v + 1 < l.vert_sectors.start.val.length := by rw [hrows]; exact hk
  have hm := csr_mono hve (k := v) 1 hk
  have hl := csr_le_len hve hk
  have hsm := sec_mono' l hsc (k := v) 1 hsk
  have hsl := sec_le_len' l hsc hsk
  set lo := stAt l.vert_edges v
  set hi := stAt l.vert_edges (v + 1)
  set slo := sstAt l v
  set shi := sstAt l (v + 1)
  have hrowL : rowL l.vert_edges v = (rowU l.vert_edges.ids.val lo hi).map (·.val) := rfl
  have hsec : secRowL l.vert_sectors v = secP.rowU' l.vert_sectors.sec.val slo shi := rfl
  have hef : ∀ a : U32, rowL l.edge_facets a.val =
      (rowU l.edge_facets.ids.val (stAt l.edge_facets a.val) (stAt l.edge_facets (a.val + 1))).map (·.val) :=
    fun _ => rfl
  unfold VertOK
  rw [hrowL, hsec, nodup_map_val, nodup_iff_count, nodup_iff_countP]
  -- turn the positions into members
  have hpos : (∀ t, slo ≤ t → t < shi → secP l lo hi slo shi (l.vert_sectors.sec.val.getD t default)) ↔
      ∀ x ∈ secP.rowU' l.vert_sectors.sec.val slo shi, secP l lo hi slo shi x := by
    constructor
    · intro h x hx
      obtain ⟨t, h1, h2, rfl⟩ := (mem_window default hsm hsl).mp hx
      exact h t h1 h2
    · intro h t h1 h2
      exact h _ ((mem_window default hsm hsl).mpr ⟨t, h1, h2, rfl⟩)
  rw [hpos]
  constructor
  · rintro ⟨hnd, hsx⟩
    have hnd' : (rowU l.vert_edges.ids.val lo hi).Nodup := (nodup_iff_count _).mpr hnd
    refine ⟨hnd, fun x hx => ?_, fun x hx => (hsx x hx).1⟩
    obtain ⟨_, h2, h3, h4, h5, h6⟩ := hsx x hx
    refine ⟨h2, ?_, ?_, ?_, ?_⟩
    · rw [← mem_val]; exact (count_one_mem hnd' _).mp h3
    · rw [← mem_val]; exact (count_one_mem hnd' _).mp h4
    · rw [hef, ← mem_val]; exact (one_le_count_iff _ _).mp h5
    · rw [hef, ← mem_val]; exact (one_le_count_iff _ _).mp h6
  · rintro ⟨hnd, hsx, hsn⟩
    have hnd' : (rowU l.vert_edges.ids.val lo hi).Nodup := (nodup_iff_count _).mpr hnd
    refine ⟨hnd, fun x hx => ?_⟩
    obtain ⟨h2, h3, h4, h5, h6⟩ := hsx x hx
    refine ⟨hsn x hx, h2, ?_, ?_, ?_, ?_⟩
    · rw [count_one_mem hnd', mem_val]; exact h3
    · rw [count_one_mem hnd', mem_val]; exact h4
    · rw [one_le_count_iff, mem_val, ← hef]; exact h5
    · rw [one_le_count_iff, mem_val, ← hef]; exact h6


theorem forall_idx {β : Type} (L : List β) (d : β) (P : β → Prop) :
    (∀ j, j < L.length → P (L.getD j d)) ↔ ∀ x ∈ L, P x := by
  constructor
  · intro h x hx
    obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hx
    have := h j hj
    rwa [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj, Option.getD_some] at this
  · intro h j hj
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj, Option.getD_some]
    exact h _ (List.getElem_mem _)

theorem exists_idx {β : Type} (L : List β) (d : β) (P : β → Prop) :
    (∃ j, j < L.length ∧ P (L.getD j d)) ↔ ∃ x ∈ L, P x := by
  constructor
  · rintro ⟨j, hj, h⟩
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj, Option.getD_some] at h
    exact ⟨_, List.getElem_mem _, h⟩
  · rintro ⟨x, hx, h⟩
    obtain ⟨j, hj, rfl⟩ := List.getElem_of_mem hx
    exact ⟨j, hj, by rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj, Option.getD_some]; exact h⟩

theorem rooms_iff (l : Ledger) (hl : l.facet_rooms.val.length = nfacets l) :
    RoomsTo l (nfacets l) ↔ ∀ p ∈ l.facet_rooms.val, p.1.val < l.n_rooms.val ∧ p.2.val < l.n_rooms.val ∧ p.1 ≠ p.2 := by
  unfold RoomsTo
  rw [← hl, ← forall_idx _ (0#u32, 0#u32)]
  constructor
  · intro h j hj; obtain ⟨a, b, c⟩ := h j hj; exact ⟨b, c, a⟩
  · intro h j hj; obtain ⟨a, b, c⟩ := h j hj; exact ⟨c, a, b⟩

theorem pieces_iff (l : Ledger) :
    PiecesTo l l.pieces.val.length ↔ ∀ p ∈ l.pieces.val, (p.1 : U32).val < nfacets l ∧
      (p.2 : U32).val < l.face_body.val.length ∧
      l.face_class.val.getD (p.2 : U32).val 0#u32 = l.facet_class.val.getD (p.1 : U32).val 0#u32 := by
  unfold PiecesTo pcF pfF
  exact forall_idx _ default (fun (p : FacetId × FaceId) => (p.1 : U32).val < nfacets l ∧ (p.2 : U32).val < l.face_body.val.length ∧
      l.face_class.val.getD (p.2 : U32).val 0#u32 = l.facet_class.val.getD (p.1 : U32).val 0#u32)

theorem inside_iff (l : Ledger) :
    InsideTo l l.inside.val.length ↔ ∀ p ∈ l.inside.val, (p.1 : U32).val < l.n_rooms.val ∧ (p.2 : U32).val < l.nb.val := by
  unfold InsideTo
  exact forall_idx _ default (fun (p : RoomId × BodyId) => (p.1 : U32).val < l.n_rooms.val ∧ (p.2 : U32).val < l.nb.val)

theorem every_iff (l : Ledger) :
    (∀ f, f < nfacets l → ∃ j, j < l.pieces.val.length ∧ pcF l j = f) ↔
      ∀ φ, φ < nfacets l → ∃ p ∈ l.pieces.val, (p.1 : U32).val = φ := by
  apply forall_congr'; intro f; apply imp_congr_right; intro _
  unfold pcF
  exact exists_idx _ default (fun (p : FacetId × FaceId) => (p.1 : U32).val = f)

end inc

end forge_bool_core
