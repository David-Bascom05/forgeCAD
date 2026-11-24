import ForgeCurved.QEdgeCross

/-!
# Listed edges are edges (the frame)

**`nonvert_edge`**: a sample not tagged vertex, whose covector has two zeros and is on the bounding
carrier's non-positive side, is a realised cell that is not a single point. Its sample sits on a
sample plane, off the events (`samplesAt_located`, `planes_off`). There the cell crosses its slab
(`edge_slab`), so it has a point on another level.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem zeroOn_false {qs : List Quad.Quadric} (hf : frameOk qs = true) :
    ∀ q ∈ qs, ∀ c, zeroOn q c = false := by
  intro q hq c
  have := List.all_eq_true.mp hf q hq
  simp only [Bool.or_eq_true, Bool.and_eq_true, bne_iff_ne, ne_eq] at this
  rcases this with ⟨hp, hlz⟩ | hzz
  · obtain ⟨-, -, -, -, h5, -⟩ := isPlaneQ_iff hp
    simp [zeroOn, restr, Poly.isZero, h5, hlz]
  · simp [zeroOn, restr, Poly.isZero, hzz]

theorem cov_real (qs : List Quad.Quadric) (p : R3) :
    covOfK (qs.map fun q => Quadric.evK realModel q) p = covOfK (qs.map qR) p := by
  have : (qs.map fun q => Quadric.evK realModel q) = qs.map qR := by
    simpa [carriersOf] using carriersOf_real qs
  rw [this]

/-- A sample off the vertex tag: a point on a sample plane, off the events, in its cell. -/
theorem nonvert_point {qs : List Quad.Quadric} (hf : frameOk qs = true) {ss : List QS}
    (h : samplesAt qs = some ss) {E : List Num} (hE : eventsOf qs rootFuel = some E)
    {s : QS} (hs : s ∈ ss) (htag : s.tag ≠ .vert) :
    ∃ c ∈ planesOf E, ∃ p : R3, p.1 = c ∧ OffEvents E p ∧ covCellK (qs.map qR) s.cov p := by
  obtain ⟨E', hE', hloc⟩ := samplesAt_located realModel h (zeroOn_false hf)
  rw [hE] at hE'
  cases hE'
  obtain ⟨c, hc, p, hp1, hcov⟩ := hloc s hs htag
  simp only [realModel_q] at hp1
  refine ⟨c, hc, p, hp1, ?_, ?_⟩
  · intro e he hd
    rw [hp1] at hd
    exact planes_off (eventsOf_sep hE) (eventsOf_den hE) hc e he hd
  · show covOfK _ _ = _
    rw [← cov_real, hcov]

/-- **A listed edge's sample makes a realised cell that is not a point.** -/
theorem nonvert_edge {qs : List Quad.Quadric} (hf : frameOk qs = true) {ss : List QS}
    (h : samplesAt qs = some ss) {E : List Num} (hE : eventsOf qs rootFuel = some E)
    {bi : ℕ} (hbi : bi < (qs.map qR).length) (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0})
    {s : QS} (hs : s ∈ ss) (htag : s.tag ≠ .vert) (h2 : 2 ≤ s.cov.count Sgn.zero)
    (hXb : s.cov[bi]? = some Sgn.neg ∨ s.cov[bi]? = some Sgn.zero) :
    RealisedK (qs.map qR) s.cov ∧ ¬ IsVertK (qs.map qR) s.cov := by
  obtain ⟨c, -, p, -, hoff, hp⟩ := nonvert_point hf h hE hs htag
  refine ⟨⟨p, hp⟩, fun ⟨_, huniq⟩ => ?_⟩
  obtain ⟨j, k, hjk, hj, hk⟩ := two_idx s.cov h2
  obtain ⟨-, -, a, b, hap, hpb, -, -, hall⟩ := edge_slab hE hbi hB s.cov hXb hjk hj hk hp hoff
  obtain ⟨x, hx, hx1⟩ := hall ((a + p.1) / 2) (by linarith) (by linarith)
  have := huniq x p hx hp
  rw [this] at hx1
  linarith

end ForgeCurved
