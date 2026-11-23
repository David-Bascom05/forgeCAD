import ForgeCurved.QEdgesSup

/-!
# A cell with no point off the events lies on its triple, and is a vertex sample

* `onTriple`: if no point of a cell with two or more zeros is off the events, every point of it is a
  common point of its (U) triple (`uTriple`). With three zeros that is immediate. With two, a point
  where `critQ ≠ 0` would move off the events (`pair_moves`).
* `vert_sampled`: a common point of a triple in `tri qs ++ critTriples qs` is named by the
  triple's enumeration, so its cell is a vertex sample.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem mem_zeroIdxs {X : Cov} {i : ℕ} : i ∈ zeroIdxs X ↔ X[i]? = some Sgn.zero := by
  simp only [zeroIdxs, List.mem_filter, List.mem_range, beq_iff_eq]
  constructor
  · exact fun h => h.2
  · intro h; exact ⟨(List.getElem?_eq_some_iff.mp h).1, h⟩

theorem zeroIdxs_sorted (X : Cov) : (zeroIdxs X).Pairwise (· < ·) :=
  (List.pairwise_lt_range).filter _

theorem crit_ev (a b : Quad.Quadric) (p : R3) :
    qR (critQ a b) p = gy a p * gz b p - gz a p * gy b p := by
  rw [← evK_real]
  have := critQ_ev realModel a b p
  rw [dyK_real, dzK_real, dyK_real, dzK_real] at this
  exact this

/-- The two-zero case: exactly the positions `i`, `j`. -/
theorem two_only {X : Cov} {i j : ℕ} (h : zeroIdxs X = [i, j]) :
    ∀ m, m ≠ i → m ≠ j → X[m]? ≠ some Sgn.zero := by
  intro m hmi hmj hm
  have := mem_zeroIdxs.mpr hm
  rw [h] at this
  simp at this; omega

/-- **On its triple.** -/
theorem onTriple {qs : List Quad.Quadric} {E : List Num} {X : Cov}
    (hlen : X.length = qs.length) (hnoff : ∀ p, covCellK (qs.map qR) X p → ¬ OffEvents E p)
    {t : Quad.Quadric × Quad.Quadric × Quad.Quadric} (ht : uTriple qs X = some t)
    {p : R3} (hp : covCellK (qs.map qR) X p) :
    qR t.1 p = 0 ∧ qR t.2.1 p = 0 ∧ qR t.2.2 p = 0 := by
  have hz : ∀ i ∈ zeroIdxs X, ∃ hi : i < qs.length, qR qs[i] p = 0 := by
    intro i hi
    have h1 := mem_zeroIdxs.mp hi
    have hl : i < qs.length := by rw [← hlen]; exact (List.getElem?_eq_some_iff.mp h1).1
    exact ⟨hl, sgn_zero_at hp hl h1⟩
  have hgd : ∀ i ∈ zeroIdxs X, qR (qs.getD i qnil) p = 0 := by
    intro i hi
    obtain ⟨hl, h0⟩ := hz i hi
    rw [List.getD_eq_getElem _ _ hl]; exact h0
  unfold uTriple at ht
  split at ht
  · rename_i i j k rest hzs
    cases ht
    exact ⟨hgd i (by rw [hzs]; simp), hgd j (by rw [hzs]; simp), hgd k (by rw [hzs]; simp)⟩
  · rename_i i j hzs
    cases ht
    refine ⟨hgd i (by rw [hzs]; simp), hgd j (by rw [hzs]; simp), ?_⟩
    -- a transverse point would move off the events
    by_contra hc
    obtain ⟨hi, h0i⟩ := hz i (by rw [hzs]; simp)
    obtain ⟨hj, h0j⟩ := hz j (by rw [hzs]; simp)
    have hij : i ≠ j := by
      have := zeroIdxs_sorted X; rw [hzs] at this; simp at this; omega
    rw [List.getD_eq_getElem _ _ hi, List.getD_eq_getElem _ _ hj, crit_ev] at hc
    have hne : ∀ m (hm : m < qs.length), m ≠ i → m ≠ j → qR qs[m] p ≠ 0 := by
      intro m hm hmi hmj h0
      apply two_only hzs m hmi hmj
      rw [← hp, getElem?_covOfK]
      simp [List.getElem?_eq_getElem hm, sgnK_zero_iff.mpr h0]
    obtain ⟨p', -, hoff, hcov⟩ := pair_moves hi hj hij E h0i h0j hne hc
    exact hnoff p' (by show covOfK _ _ = X; rw [hcov]; exact hp) hoff
  · cases ht

/-- The triple is one the vertex samples enumerate. -/
theorem uTriple_mem {qs : List Quad.Quadric} {X : Cov} (hlen : X.length = qs.length)
    {t : Quad.Quadric × Quad.Quadric × Quad.Quadric} (ht : uTriple qs X = some t) :
    t ∈ tri qs ++ critTriples qs := by
  have hs := zeroIdxs_sorted X
  have hl : ∀ i ∈ zeroIdxs X, i < qs.length := by
    intro i hi
    rw [← hlen]; exact (List.getElem?_eq_some_iff.mp (mem_zeroIdxs.mp hi)).1
  unfold uTriple at ht
  split at ht
  · rename_i i j k rest hzs
    cases ht
    rw [hzs] at hs hl
    simp only [List.pairwise_cons, List.mem_cons] at hs
    have hi := hl i (by simp); have hj := hl j (by simp); have hk := hl k (by simp)
    rw [List.getD_eq_getElem _ _ hi, List.getD_eq_getElem _ _ hj, List.getD_eq_getElem _ _ hk]
    exact List.mem_append_left _ (mem_tri hi hj hk (hs.1 j (by simp)) (hs.2.1 k (by simp)))
  · rename_i i j hzs
    cases ht
    rw [hzs] at hs hl
    simp only [List.pairwise_cons, List.mem_cons] at hs
    have hi := hl i (by simp); have hj := hl j (by simp)
    rw [List.getD_eq_getElem _ _ hi, List.getD_eq_getElem _ _ hj]
    exact List.mem_append_right _ (List.mem_map.mpr ⟨(qs[i], qs[j]), mem_pairs hi hj
      (hs.1 j (by simp)), rfl⟩)
  · cases ht

/-- **A common point of an enumerated triple makes its cell a vertex sample.** -/
theorem vert_sampled {qs : List Quad.Quadric} {ss : List QS} (h : samplesAt qs = some ss)
    {t : Quad.Quadric × Quad.Quadric × Quad.Quadric} (htm : t ∈ tri qs ++ critTriples qs)
    {p : R3} (h1 : qR t.1 p = 0) (h2 : qR t.2.1 p = 0) (h3 : qR t.2.2 p = 0) :
    ∃ s ∈ ss, s.tag = .vert ∧ s.cov = covOfK (qs.map qR) p := by
  unfold samplesAt at h
  simp only [Option.bind_eq_bind, Option.pure_def, Option.bind_eq_some_iff,
    Option.some.injEq] at h
  obtain ⟨E, -, ps, -, es, -, vs, hvs, rfl⟩ := h
  unfold vertSamples at hvs
  simp only [Option.map_eq_some_iff] at hvs
  obtain ⟨ls, hls, rfl⟩ := hvs
  obtain ⟨l, hl, hlm⟩ := mapM_mem hls t htm
  unfold enumSamples at hl
  simp only [Option.bind_eq_some_iff] at hl
  obtain ⟨ds, hds, hdl⟩ := hl
  obtain ⟨x, y, z⟩ := p
  obtain ⟨d, hd, hat⟩ := enumTriple_complete realModel hds (x := x) (y := y) (z := z)
    (by rw [evK_real]; exact h1) (by rw [evK_real]; exact h2) (by rw [evK_real]; exact h3)
  obtain ⟨s, hs, hsl⟩ := mapM_mem hdl d hd
  simp only [Option.map_eq_some_iff] at hs
  obtain ⟨X, hX, rfl⟩ := hs
  refine ⟨⟨.vert, X⟩, List.mem_append_right _ (List.mem_flatten.mpr ⟨l, hlm, hsl⟩), rfl, ?_⟩
  show X = _
  rw [PtD.cov_spec realModel hX hat, cov_real]

end ForgeCurved
