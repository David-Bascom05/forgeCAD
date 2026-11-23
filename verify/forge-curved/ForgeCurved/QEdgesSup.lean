import ForgeCurved.QEdgesSub

/-!
# A two-zero cell off the events is an edge sample

**`twozero_sampled`**: a cell with two zero carriers, on the bounding carrier's non-positive side,
with a point off the events, reaches a sample plane (`edge_slab`). There it is a crossing of its
two carriers (`edgesOn_complete2`), so an edge sample of the plane, with the cell's covector.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem sgn_zero_at {qs : List Quad.Quadric} {X : Cov} {x : R3} (hx : covCellK (qs.map qR) X x)
    {j : ℕ} (hj : j < qs.length) (hXj : X[j]? = some Sgn.zero) : qR qs[j] x = 0 := by
  have := hXj
  rw [← hx, getElem?_covOfK] at this
  simp only [List.getElem?_map, List.getElem?_eq_getElem hj, Option.map_some,
    Option.some.injEq] at this
  exact sgnK_zero_iff.mp this

/-- **A two-zero cell off the events is an edge sample.** -/
theorem twozero_sampled {qs : List Quad.Quadric} (hf : frameOk qs = true) {ss : List QS}
    (h : samplesAt qs = some ss) {E : List Num} (hE : eventsOf qs rootFuel = some E)
    {bi : ℕ} (hbi : bi < (qs.map qR).length) (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0})
    (X : Cov) (hXb : X[bi]? = some Sgn.neg ∨ X[bi]? = some Sgn.zero) {j k : ℕ}
    (hj : j < qs.length) (hk : k < qs.length) (hjk : j ≠ k)
    (hXj : X[j]? = some Sgn.zero) (hXk : X[k]? = some Sgn.zero)
    {p : R3} (hp : covCellK (qs.map qR) X p) (hoff : OffEvents E p) :
    ∃ s ∈ ss, s.tag ≠ .vert ∧ s.cov = X := by
  obtain ⟨c, hc, a, b, -, -, hac, hcb, hall⟩ := edge_slab hE hbi hB X hXb hjk hXj hXk hp hoff
  obtain ⟨x, hx, hx1⟩ := hall c hac hcb
  obtain ⟨E', hE', hplanes⟩ := samplesAt_parts h
  rw [hE] at hE'; cases hE'
  obtain ⟨ps, hps, hpsm⟩ := hplanes c hc
  obtain ⟨es, hes, hedge⟩ := planeSamples_edge hps
  rw [activeOn_eq hf, noXPlane_eq hf] at hes
  have h0j := sgn_zero_at hx hj hXj
  have h0k := sgn_zero_at hx hk hXk
  have hxe : x = ((c : ℝ), x.2.1, x.2.2) := by
    obtain ⟨u, v, w⟩ := x; simp only at hx1; simp [hx1]
  have ev : ∀ q : Quad.Quadric, qR q x = 0 →
      Quadric.evK realModel q (realModel.q c, x.2.1, x.2.2) = 0 := by
    intro q hq; rw [realModel_q, evK_real, ← hxe]; exact hq
  obtain ⟨e, he, hd, hok, hpt⟩ : ∃ e ∈ es, Denotes realModel e.y x.2.1 ∧ e.dOk signFuel = true ∧
      planePt realModel e.c e.N e.D x.2.1 = (realModel.q c, x.2.1, x.2.2) := by
    rcases pair_of hj hk hjk with hpr | hpr
    · exact edgesOn_complete2 realModel hes hpr (ev _ h0j) (ev _ h0k)
    · exact edgesOn_complete2 realModel hes hpr (ev _ h0k) (ev _ h0j)
  obtain ⟨s, hs, hse⟩ := hedge e he
  refine ⟨s, hpsm s hs, ?_, ?_⟩
  · unfold edgeSample at hse
    split at hse
    · simp only [Option.map_eq_some_iff] at hse
      obtain ⟨X', -, rfl⟩ := hse
      simp
    · cases hse
  · unfold edgeSample at hse
    split at hse
    · simp only [Option.map_eq_some_iff] at hse
      obtain ⟨X', hX', rfl⟩ := hse
      have := edgeCov_spec realModel hd hok qs X' hX'
      rw [hpt, realModel_q] at this
      show X' = X
      rw [this, cov_real, ← hxe]
      exact hx
    · cases hse

end ForgeCurved
