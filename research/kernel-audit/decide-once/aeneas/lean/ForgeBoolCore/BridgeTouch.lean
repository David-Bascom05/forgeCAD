import ForgeBoolCore.BridgeResult

/-!
# Group R: the touch rule on ids is the Lean's

`badE` (more than two result facets round an edge, on positions) is the Lean's `badEdge`
(`bad_edge_bridge_proof : BadEdgeBridge`), and `badV` (`bad_vert`'s answer: some facet round the
vertex out of the first one's fan, on indices of `around`) is the Lean's `badVert`
(`bad_vert_bridge_proof : BadVertBridge`).

For the fan test: `around v`'s index `j` reads as the covector `G j`, injectively, so the fan
relation on indices is the Lean's `fanRel` on covectors and the closures correspond.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

open ForgeExact.DecideOnce ForgeBoolCore.Dedup

theorem R_edge_lt {l : Ledger} {e0 s ext} (h : AfterSplit l e0 s ext) {e : Nat} (he : e ∈ idsOf l.edges) :
    e < (allCells l ext).length := by
  simp only [idsOf, List.mem_map] at he
  obtain ⟨x, hx, rfl⟩ := he
  have := h.wf.edges x hx
  rw [allCells_length]; omega

/-- **The edge test on ids is the Lean's `badEdge`.** -/
theorem bad_edge_bridge_proof : BadEdgeBridge := by
  intro l e0 s ext keep a h hkl hA e he
  unfold badE ForgeExact.DecideOnce.Ledger.badEdge
  rw [← around_bridge_proof l e0 s ext keep a h hkl hA e (R_edge_lt h he), List.length_map]

/-! ## Closures along an injective reading -/

section
set_option linter.unusedSectionVars false
variable {α : Type} [DecidableEq α] {R : Nat → Nat → Bool} {r : α → α → Bool} {G : Nat → α} {m : Nat}

theorem R_conn_fwd (hR : ∀ j k, j < m → k < m → R j k = r (G j) (G k)) {j k : Nat} (hj : j < m)
    (h : Conn R (List.range m) j k) : Conn r ((List.range m).map G) (G j) (G k) := by
  induction h with
  | refl => exact Conn.refl
  | @step b c hab hc hbc ih =>
    have hb : b < m := List.mem_range.mp (Conn.mem (List.mem_range.mpr hj) hab)
    have hc' : c < m := List.mem_range.mp hc
    exact Conn.step ih (List.mem_map.mpr ⟨c, hc, rfl⟩) (by rw [← hR b c hb hc']; exact hbc)

theorem R_conn_bwd (hR : ∀ j k, j < m → k < m → R j k = r (G j) (G k)) {j : Nat} (hj : j < m) {y : α}
    (h : Conn r ((List.range m).map G) (G j) y) : ∃ k, k < m ∧ y = G k ∧ Conn R (List.range m) j k := by
  induction h with
  | refl => exact ⟨j, hj, rfl, Conn.refl⟩
  | @step b c _ hc hbc ih =>
    obtain ⟨k, hk, rfl, hjk⟩ := ih
    obtain ⟨k', hk', rfl⟩ := List.mem_map.mp hc
    have hk'' := List.mem_range.mp hk'
    exact ⟨k', hk'', rfl, Conn.step hjk hk' (by rw [hR k k' hk hk'']; exact hbc)⟩

end

/-! ## The fan test -/

section
variable {l : Ledger} {e0 : Ext} {s : Split} {ext : Ext} {keep : List Bool} {a : Assembled}

/-- The covector at index `j` of `around v`. -/
def R_G (l : Ledger) (ext : Ext) (a : Assembled) (ar : List Nat) (j : Nat) : Cov :=
  covE l ext (resAt a (ar.getD j 0))

theorem R_around_G (ar : List Nat) :
    ar.map (fun p => covE l ext (resAt a p)) = (List.range ar.length).map (R_G l ext a ar) := by
  apply List.ext_getElem (by simp)
  intro i h1 h2
  simp only [List.getElem_map, List.getElem_range, R_G]
  rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using h1)]; rfl

/-- Reading positions of `around` as covectors is injective. -/
theorem R_G_inj (h : AfterSplit l e0 s ext) (hA : AsmOf l ext s keep a) (x : List Sgn) {j k : Nat}
    (hj : j < (aroundL l ext a x).length) (hk : k < (aroundL l ext a x).length)
    (e : R_G l ext a (aroundL l ext a x) j = R_G l ext a (aroundL l ext a x) k) : j = k := by
  have hRv := R_res_vals h hA
  have hnd : (a.result.val.map (·.val)).Nodup := by
    rw [hRv]; exact resIds_nodup s keep (R_facets_nodup h) _ (Nat.le_refl _)
  have hlt : ∀ p, p < a.result.val.length → resAt a p < (allCells l ext).length := by
    intro p hp
    have : resAt a p ∈ resIds s keep s.facets.val.length := by
      rw [← hRv, resAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hp)]
      simp only [Option.getD_some]; exact List.getElem_mem _
    have := hA.1.2.2.1 _ this
    exact this
  have hres : ∀ p q, p < a.result.val.length → q < a.result.val.length → resAt a p = resAt a q → p = q := by
    intro p q hp hq hpq
    rw [resAt, resAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hp),
      List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hq), Option.getD_some,
      Option.getD_some] at hpq
    exact (List.Nodup.getElem_inj_iff hnd).mp hpq
  have hand : (aroundL l ext a x).Nodup := List.Nodup.filter _ List.nodup_range
  have harlt : ∀ i (hi : i < (aroundL l ext a x).length), (aroundL l ext a x)[i] < a.result.val.length := by
    intro i hi
    have := List.getElem_mem hi
    simp only [aroundL, List.mem_filter, List.mem_range] at this
    exact this.1
  simp only [R_G, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj, List.getElem?_eq_getElem hk,
    Option.getD_some] at e
  have h1 := covE_inj l ext h.split.ext (hlt _ (harlt j hj)) (hlt _ (harlt k hk)) e
  have h2 := hres _ _ (harlt j hj) (harlt k hk) h1
  exact (List.Nodup.getElem_inj_iff hand).mp h2

/-- **The fan relation on indices is the Lean's `fanRel`.** -/
theorem R_fanR_eq (h : AfterSplit l e0 s ext) {isE : List Bool} (hE : MarksEdges l ext isE) (v : Nat)
    (ar : List Nat) (j k : Nat) :
    fanR l ext a isE (covAt l ext v) ar j k =
      (toL l).fanRel (covE l ext v) (R_G l ext a ar j) (R_G l ext a ar k) := by
  apply Bool.eq_iff_iff.mpr
  unfold fanR shareR ForgeExact.DecideOnce.Ledger.fanRel
  simp only [List.any_eq_true, List.mem_range, Bool.and_eq_true, ebB, decide_eq_true_eq]
  have hcs : ∀ i, (csAt l ext a ar i).map toS = R_G l ext a ar i := fun _ => rfl
  simp only [hcs]
  constructor
  · rintro ⟨e, _, ⟨⟨⟨⟨_, hie⟩, hv⟩, hej⟩, ⟨⟨⟨_, _⟩, _⟩, hek⟩⟩⟩
    have hmem : e ∈ idsOf l.edges := by
      have := hE.2 e (by omega); rw [hie] at this; simpa using this.symm
    simp only [idsOf, List.mem_map] at hmem
    obtain ⟨x, hx, rfl⟩ := hmem
    refine ⟨cellL l x.val, List.mem_map.mpr ⟨x, hx, rfl⟩, ?_⟩
    rw [cellL_covAt l ext (h.wf.edges x hx)]
    exact ⟨⟨hv, hej⟩, hek⟩
  · rintro ⟨c, hc, ⟨hv, hcj⟩, hck⟩
    simp only [toL, List.mem_map] at hc
    obtain ⟨x, hx, rfl⟩ := hc
    have hxl := h.wf.edges x hx
    have hxN : x.val < isE.length := by rw [hE.1, allCells_length]; omega
    have hie : isE.getD x.val false = true := by
      rw [hE.2 _ hxN]; exact decide_eq_true (List.mem_map.mpr ⟨x, hx, rfl⟩)
    rw [cellL_covAt l ext hxl] at hv hcj hck
    exact ⟨x.val, hxN, ⟨⟨⟨⟨hxN, hie⟩, hv⟩, hcj⟩, ⟨⟨⟨hxN, hie⟩, hv⟩, hck⟩⟩⟩

end

/-- **The fan test on ids is the Lean's `badVert`.** -/
theorem bad_vert_bridge_proof : BadVertBridge := by
  intro l e0 s ext keep a isE h hkl hA hE v _
  set ar := aroundL l ext a (covAt l ext v) with har
  set G := R_G l ext a ar with hG
  have hAG : (toL l).around (ktOf l keep) (covE l ext v) = (List.range ar.length).map G := by
    have hv : v < (allCells l ext).length := by
      rename_i hv'
      simp only [idsOf, List.mem_map] at hv'
      obtain ⟨x, hx, rfl⟩ := hv'
      have := h.wf.verts x hx
      rw [allCells_length]; omega
    rw [← around_bridge_proof l e0 s ext keep a h hkl hA v hv, ← har, R_around_G]
  have hR : ∀ j k, j < ar.length → k < ar.length →
      fanR l ext a isE (covAt l ext v) ar j k = (toL l).fanRel (covE l ext v) (G j) (G k) :=
    fun j k _ _ => R_fanR_eq h hE v ar j k
  have hinj : ∀ j k, j < ar.length → k < ar.length → G j = G k → j = k :=
    fun j k hj hk e => R_G_inj h hA (covAt l ext v) hj hk e
  apply Bool.eq_iff_iff.mpr
  rw [badV_iff, badVert_iff, hAG, ← har]
  constructor
  · rintro ⟨k, hk1, hk, hn⟩
    refine ⟨G 0, List.mem_map.mpr ⟨0, List.mem_range.mpr (by omega), rfl⟩,
      G k, List.mem_map.mpr ⟨k, List.mem_range.mpr hk, rfl⟩, fun hc => hn ?_⟩
    obtain ⟨k', hk', e, hc'⟩ := R_conn_bwd hR (by omega) hc
    rw [hinj k k' hk hk' e]; exact hc'
  · rintro ⟨c, hc, d, hd, hn⟩
    obtain ⟨j, hj, rfl⟩ := List.mem_map.mp hc
    obtain ⟨k, hk, rfl⟩ := List.mem_map.mp hd
    have hj' := List.mem_range.mp hj
    have hk' := List.mem_range.mp hk
    apply Classical.byContradiction
    intro hall
    have hall' : ∀ i, i < ar.length → Conn (fanR l ext a isE (covAt l ext v) ar) (List.range ar.length) 0 i := by
      intro i hi
      by_cases h0 : i = 0
      · subst h0; exact Conn.refl
      · apply Classical.byContradiction
        intro hni
        exact hall ⟨i, by omega, hi, hni⟩
    have h0 : 0 < ar.length := by omega
    have c0 := R_conn_fwd hR h0 (hall' j hj')
    have c1 := R_conn_fwd hR h0 (hall' k hk')
    have hmem0 : G 0 ∈ (List.range ar.length).map G := List.mem_map.mpr ⟨0, List.mem_range.mpr h0, rfl⟩
    exact hn ((Conn.symm (ForgeExact.DecideOnce.Ledger.fanRel_symm (toL l) (covE l ext v)) hmem0 c0).trans c1)

end forge_bool_core
