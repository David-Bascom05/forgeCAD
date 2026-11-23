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


/-! ### `ledger_wf`'s opening checks, and the loops after them -/

/-- What `ledger_wf`'s opening conjunction checks. -/
structure PreA (l : Ledger) : Prop where
  nb_pos : 0 < l.nb.val
  face_lists : l.face_class.val.length = l.face_body.val.length
  rooms_len : l.facet_rooms.val.length = nfacets l
  small : nfacets l < U32.max ∧ l.n_rooms.val < U32.max ∧ l.pieces.val.length < U32.max ∧
    l.face_body.val.length < U32.max ∧ nfacets l * l.nb.val ≤ Usize.max ∧
    l.n_rooms.val * l.nb.val ≤ Usize.max ∧ nrowsL l.edge_facets < U32.max ∧ nrowsL l.vert_edges < U32.max ∧
    l.vert_sectors.sec.val.length < U32.max ∧ 3 * nfacets l ≤ Usize.max
  ef : CsrOK l.edge_facets (nfacets l)
  ve : CsrOK l.vert_edges (nrowsL l.edge_facets)
  rows : l.vert_sectors.start.val.length = l.vert_edges.start.val.length

theorem preA_of (l : Ledger) (hw : LedgerWF2 l) : PreA l :=
  ⟨hw.nb_pos, hw.face_lists, hw.rooms.1, hw.small, hw.edge_facets, hw.vert_edges, hw.sectors_rows⟩

/-- **Given the opening checks, `LedgerWF2` is the loops' conditions.** -/
theorem wf2_iff (l : Ledger) (hp : PreA l) :
    LedgerWF2 l ↔ SecCsrOK l.vert_sectors ∧ WingsTo l.edge_facets (nrowsL l.edge_facets) ∧
      (∀ w, w < nrowsL l.vert_edges → VertOK l w) ∧ RoomsTo l (nfacets l) ∧
      PiecesTo l l.pieces.val.length ∧ InsideTo l l.inside.val.length ∧
      (∀ f, f < nfacets l → ∃ j, j < l.pieces.val.length ∧ pcF l j = f) := by
  constructor
  · intro hw
    refine ⟨hw.sectors_csr, fun e he => hw.wings e he, fun w hwv => ?_, (rooms_iff l hp.rooms_len).mpr hw.rooms.2,
      (pieces_iff l).mpr hw.pieces, (inside_iff l).mpr hw.inside, (every_iff l).mpr hw.every_piece⟩
    exact (vertOK_iff l hp.ve hw.sectors_csr hp.rows hwv).mpr
      ⟨hw.vert_edges_nodup w hwv, hw.sectors w hwv, hw.sectors_nodup w hwv⟩
  · rintro ⟨hsc, hwg, hv, hr, hpc, hin, hev⟩
    have hvf := fun w hwv => (vertOK_iff l hp.ve hsc hp.rows hwv).mp (hv w hwv)
    exact { nb_pos := hp.nb_pos, edge_facets := hp.ef, vert_edges := hp.ve, sectors_rows := hp.rows,
            sectors_csr := hsc, wings := hwg, vert_edges_nodup := fun w hwv => (hvf w hwv).1,
            sectors := fun w hwv => (hvf w hwv).2.1, sectors_nodup := fun w hwv => (hvf w hwv).2.2,
            rooms := ⟨hp.rooms_len, (rooms_iff l hp.rooms_len).mp hr⟩, pieces := (pieces_iff l).mp hpc,
            inside := (inside_iff l).mp hin, every_piece := (every_iff l).mp hev, face_lists := hp.face_lists,
            small := hp.small }


/-- **The loops after the opening checks decide `LedgerWF2`.** -/
theorem wf_main (l : Ledger) (hp : PreA l) (ne nv : Usize) (hne : ne.val = nrowsL l.edge_facets)
    (hnv : nv.val = nrowsL l.vert_edges) :
    (do
      let i ← alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice U32) l.vert_sectors.start 0#usize
      let ok2 ←
        if i = 0#u32 then do
          let i1 := alloc.vec.Vec.len l.vert_sectors.start
          let i2 ← i1 - 1#usize
          let i3 ← alloc.vec.Vec.index (core.slice.index.SliceIndexUsizeSlice U32) l.vert_sectors.start i2
          let i4 ← lift (UScalar.cast .Usize i3)
          let i5 := alloc.vec.Vec.len l.vert_sectors.sec
          ok (decide (i4 = i5))
        else ok false
      let ok3 ← ledger_wf_loop0 l.vert_sectors.start ok2 1#usize
      let ok4 ← ledger_wf_loop1 l.edge_facets.start l.edge_facets.ids ne ok3 0#usize
      let ok5 ← ledger_wf_loop2 l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids
        l.vert_sectors.start l.vert_sectors.sec (alloc.vec.Vec.len l.facet_class) nv ok4 0#usize
      let ok6 ← ledger_wf_loop3 l.n_rooms l.facet_rooms (alloc.vec.Vec.len l.facet_class) ok5 0#usize
      let is_piece ← alloc.vec.from_elem core.clone.CloneBool false (alloc.vec.Vec.len l.facet_class)
      let (ok7, is_piece1) ← ledger_wf_loop4 l.face_body l.face_class l.facet_class l.pieces
        (alloc.vec.Vec.len l.facet_class) ok6 is_piece 0#usize
      let ok8 ← ledger_wf_loop5 l.nb l.n_rooms l.inside ok7 0#usize
      ledger_wf_loop6 (alloc.vec.Vec.len l.facet_class) ok8 is_piece1 0#usize) ⦃ r => r = true ↔ LedgerWF2 l ⦄ := by
  have hnf : (alloc.vec.Vec.len l.facet_class).val = nfacets l := by simp [nfacets]
  have hne0 : l.vert_edges.start.val ≠ [] := hp.ve.nonempty
  have hs0 : 0 < l.vert_sectors.start.val.length := by
    rw [hp.rows]; exact List.length_pos_iff.mpr hne0
  rw [wf2_iff l hp]
  step as ⟨i, hi⟩
  -- ok2: the first and last starts
  have hok2 : ∀ (ok2 : Bool), (ok2 = true ↔ (l.vert_sectors.start.val.getD 0 0#u32).val = 0 ∧
      (l.vert_sectors.start.val.getD (l.vert_sectors.start.val.length - 1) 0#u32).val = l.vert_sectors.sec.val.length) →
      (do
        let ok3 ← ledger_wf_loop0 l.vert_sectors.start ok2 1#usize
        let ok4 ← ledger_wf_loop1 l.edge_facets.start l.edge_facets.ids ne ok3 0#usize
        let ok5 ← ledger_wf_loop2 l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids
          l.vert_sectors.start l.vert_sectors.sec (alloc.vec.Vec.len l.facet_class) nv ok4 0#usize
        let ok6 ← ledger_wf_loop3 l.n_rooms l.facet_rooms (alloc.vec.Vec.len l.facet_class) ok5 0#usize
        let is_piece ← alloc.vec.from_elem core.clone.CloneBool false (alloc.vec.Vec.len l.facet_class)
        let (ok7, is_piece1) ← ledger_wf_loop4 l.face_body l.face_class l.facet_class l.pieces
          (alloc.vec.Vec.len l.facet_class) ok6 is_piece 0#usize
        let ok8 ← ledger_wf_loop5 l.nb l.n_rooms l.inside ok7 0#usize
        ledger_wf_loop6 (alloc.vec.Vec.len l.facet_class) ok8 is_piece1 0#usize) ⦃ r => r = true ↔
          SecCsrOK l.vert_sectors ∧ WingsTo l.edge_facets (nrowsL l.edge_facets) ∧
          (∀ w, w < nrowsL l.vert_edges → VertOK l w) ∧ RoomsTo l (nfacets l) ∧
          PiecesTo l l.pieces.val.length ∧ InsideTo l l.inside.val.length ∧
          (∀ f, f < nfacets l → ∃ j, j < l.pieces.val.length ∧ pcF l j = f) ⦄ := by
    intro ok2 h2
    rw [wf_loop0_eq]
    step with csr_ok_loop0_spec l.vert_sectors.start ok2 ok2 1#usize (by simp)
      ⟨fun h => ⟨h, fun a ha _ => by simp at ha⟩, fun h => h.1⟩ as ⟨ok3, h3⟩
    have hsec : ok3 = true ↔ SecCsrOK l.vert_sectors := by
      rw [h3, h2]
      constructor
      · rintro ⟨⟨hz, hl⟩, hm⟩
        exact ⟨List.length_pos_iff.mp hs0, hz, fun k hk => hm k hk hk, hl⟩
      · intro h; exact ⟨⟨h.zero, h.last⟩, fun k hk _ => h.mono k hk⟩
    step with wf_loop1_spec l.edge_facets (nfacets l) ne hne ok3 ok3 0#usize (fun _ => hp.ef) (by simp)
      ⟨fun h => ⟨h, fun e he => by simp at he⟩, fun h => h.1⟩ as ⟨ok4, h4⟩
    rw [hne] at h4
    step with wf_loop2_spec l _ nv hnf hnv ok4 ok4 0#usize
      (fun h => ⟨hp.ef, hp.ve, hsec.mp ((h4.mp h).1), hp.rows⟩) (by simp)
      ⟨fun h => ⟨h, fun w hw => by simp at hw⟩, fun h => h.1⟩ as ⟨ok5, h5⟩
    rw [hnv] at h5
    step with wf_loop3_spec l _ ok5 ok5 0#usize (fun _ => by rw [hnf]; exact hp.rooms_len) (by simp)
      ⟨fun h => ⟨h, fun f hf => by simp at hf⟩, fun h => h.1⟩ as ⟨ok6, h6⟩
    rw [hnf] at h6
    step with alloc.vec.from_elem_spec core.clone.CloneBool false (alloc.vec.Vec.len l.facet_class) (by rfl)
      as ⟨ip, hip1, hip2⟩
    have hipl : ip.val.length = nfacets l := by rw [hip1, List.length_replicate, hnf]
    step with wf_loop4_spec l ok6 ok6 ip 0#usize (fun _ => hp.face_lists) (by simp) hipl
      ⟨fun h => ⟨h, fun j hj => by simp at hj⟩, fun h => h.1⟩
      (fun _ f hf => by
        rw [hip1, List.getD_eq_getElem?_getD, List.getElem?_replicate]
        constructor
        · intro h; exfalso; revert h; split <;> simp
        · rintro ⟨j, hj, _⟩; simp at hj) as ⟨ok7, ip1, h7, hl7, hm7⟩
    step with wf_loop5_spec l ok7 ok7 0#usize (by simp)
      ⟨fun h => ⟨h, fun j hj => by simp at hj⟩, fun h => h.1⟩ as ⟨ok8, h8⟩
    apply WP.spec_mono (wf_loop6_spec _ ip1 ok8 ok8 0#usize (fun _ => by rw [hl7, hnf]) (by simp)
      ⟨fun h => ⟨h, fun g hg => by simp at hg⟩, fun h => h.1⟩)
    intro r hr
    rw [hr, hnf]
    constructor
    · rintro ⟨h8t, hall⟩
      obtain ⟨h7t, hin⟩ := h8.mp h8t
      obtain ⟨h6t, hpc⟩ := h7.mp h7t
      obtain ⟨h5t, hrm⟩ := h6.mp h6t
      obtain ⟨h4t, hvx⟩ := h5.mp h5t
      obtain ⟨h3t, hwg⟩ := h4.mp h4t
      refine ⟨hsec.mp h3t, hwg, hvx, hrm, hpc, hin, fun f hf => (hm7 h7t f hf).mp (hall f hf)⟩
    · rintro ⟨hsc, hwg, hvx, hrm, hpc, hin, hev⟩
      have h3t := hsec.mpr hsc
      have h4t := h4.mpr ⟨h3t, hwg⟩
      have h5t := h5.mpr ⟨h4t, hvx⟩
      have h6t := h6.mpr ⟨h5t, hrm⟩
      have h7t := h7.mpr ⟨h6t, hpc⟩
      have h8t := h8.mpr ⟨h7t, hin⟩
      exact ⟨h8t, fun f hf => (hm7 h7t f hf).mpr (hev f hf)⟩
  have hi0 : (l.vert_sectors.start.val.getD 0 0#u32) = i := by
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hs0, Option.getD_some, hi]
  split
  · rename_i hiz
    step as ⟨i2, hi2⟩
    step as ⟨i3, hi3⟩
    step as ⟨i4, hi4⟩
    try simp only [Std.bind_ok]
    apply hok2
    have hlast : i4.val = (l.vert_sectors.start.val.getD (l.vert_sectors.start.val.length - 1) 0#u32).val := by
      have : i2.val = l.vert_sectors.start.val.length - 1 := by scalar_tac
      rw [hi4, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, hi3]
      simp only [this]; scalar_tac
    rw [hi0, hiz, decide_eq_true_eq]
    constructor
    · intro e; exact ⟨rfl, by rw [← hlast, e]; simp⟩
    · rintro ⟨_, e⟩; apply UScalar.eq_of_val_eq; rw [hlast, e]; simp
  · rename_i hiz
    simp only [Std.bind_ok]
    apply hok2
    rw [hi0]
    constructor
    · intro h; cases h
    · rintro ⟨e, _⟩; exact absurd (UScalar.eq_of_val_eq (by rw [e]; rfl)) hiz

end inc

end forge_bool_core
