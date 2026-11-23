import ForgeCurved.QOffX

/-!
# Every facet is sampled

**`facet_sampled`**: in a frame that passes `frameOk` and `singAll`, where `samplesAt` succeeds,
every realised facet cell on the negative side of a bounding carrier is a sample's covector.

The chain:
1. its carrier is smooth at its points (`singAll_spec`);
2. a point off the sweep's events (`facet_offX`);
3. a point on a sample plane, off the events (`facet_meets_plane`);
4. a point there off the plane's events (`plane_offY`);
5. a point on a sample line (`plane_to_line`);
6. a facet sample with its covector (`lineSamples_complete`).
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem dxK_real (q : Quad.Quadric) (p : R3) : dxK realModel q p = gx q p := by
  simp only [dxK, gx, realModel_q]; ring

/-- Off the events, a carrier's trace on a level has a nonzero gradient. -/
theorem off_grad {qs : List Quad.Quadric} {fuel : ℕ} {E : List Num} (h : eventsOf qs fuel = some E)
    {p : R3} (hp : OffEvents E p) {q : Quad.Quadric} (hq : q ∈ qs) (h0 : qR q p = 0) :
    gy q p ≠ 0 ∨ gz q p ≠ 0 := by
  by_contra hn
  push Not at hn
  apply off_triple h hp (a := q) (b := gradPlaneY q) (c := gradPlaneZ q)
  · simp only [eventTriples, List.mem_append, List.mem_map]
    right; exact ⟨q, hq, rfl⟩
  · exact h0
  · rw [← evK_eq, gradPlaneY_ev, dyK_real]; exact hn.1
  · rw [← evK_eq, gradPlaneZ_ev, dzK_real]; exact hn.2

/-- A facet's signs at a point. -/
theorem facet_signs {qs : List Quad.Quadric} {X : Cov} {p : R3} (hp : covCellK (qs.map qR) X p)
    {j : ℕ} (hj : j < qs.length) (hXj : X[j]? = some Sgn.zero)
    (hXo : ∀ m, m ≠ j → X[m]? ≠ some Sgn.zero) :
    qR qs[j] p = 0 ∧ ∀ m (hm : m < qs.length), m ≠ j → qR qs[m] p ≠ 0 := by
  have hsig : ∀ m (hm : m < qs.length), X[m]? = some (sgnK (qR qs[m] p)) := by
    intro m hm
    rw [← hp, getElem?_covOfK]; simp [hm]
  refine ⟨?_, fun m hm hmj h0 => hXo m hmj ?_⟩
  · have := hsig j hj; rw [hXj] at this; exact sgnK_zero_iff.mp (Option.some.inj this).symm
  · rw [hsig m hm, sgnK_zero_iff.mpr h0]

/-- A point with the facet's zero and its other signs is in the facet. -/
theorem cell_of_signs {qs : List Quad.Quadric} {X : Cov} {p p' : R3}
    (hp : covCellK (qs.map qR) X p) {j : ℕ} (hj : j < qs.length) (h0 : qR qs[j] p' = 0)
    (hXj : X[j]? = some Sgn.zero)
    (hs : ∀ m (hm : m < qs.length), m ≠ j → sgnK (qR qs[m] p') = sgnK (qR qs[m] p)) :
    covCellK (qs.map qR) X p' := by
  show covOfK (qs.map qR) p' = X
  apply List.ext_getElem?
  intro m
  rw [getElem?_covOfK]
  by_cases hm : m < qs.length
  · simp only [List.getElem?_map, List.getElem?_eq_getElem hm, Option.map_some]
    by_cases hmj : m = j
    · subst hmj; rw [hXj, sgnK_zero_iff.mpr h0]
    · rw [hs m hm hmj, ← hp, getElem?_covOfK]; simp [hm]
  · have hm' : qs.length ≤ m := not_lt.mp hm
    simp only [List.getElem?_map, List.getElem?_eq_none hm', Option.map_none]
    have hlen : X.length = qs.length := by rw [← hp]; simp [covOfK]
    exact (List.getElem?_eq_none (by omega)).symm

/-- **Every facet is sampled.** -/
theorem facet_sampled {qs : List Quad.Quadric} (hf : frameOk qs = true) (hsing : singAll qs = true)
    {ss : List QS} (h : samplesAt qs = some ss) {bi : ℕ} (hbi : bi < (qs.map qR).length)
    (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0}) (X : Cov)
    (hXb : X[bi]? = some Sgn.neg) {j : ℕ} (hj : j < (qs.map qR).length)
    (hXj : X[j]? = some Sgn.zero) (hXo : ∀ m, m ≠ j → X[m]? ≠ some Sgn.zero)
    (hX : RealisedK (qs.map qR) X) : ∃ s ∈ ss, s.cov = X := by
  have hjq : j < qs.length := by simpa using hj
  obtain ⟨p, hp⟩ := hX
  obtain ⟨h0, hne⟩ := facet_signs hp hjq hXj hXo
  -- 1. smooth at `p`
  have hg : gx qs[j] p ≠ 0 ∨ gy qs[j] p ≠ 0 ∨ gz qs[j] p ≠ 0 := by
    by_contra hn
    push Not at hn
    obtain ⟨m, hmj, r, hr, hr0⟩ := singAll_spec realModel hsing (j := j) (q := qs[j])
      (List.getElem?_eq_getElem hjq) (p := p)
      ⟨by rw [dxK_real]; exact hn.1, by rw [dyK_real]; exact hn.2.1, by rw [dzK_real]; exact hn.2.2⟩
      (by rw [evK_real]; exact h0)
    have hm : m < qs.length := (List.getElem?_eq_some_iff.mp hr).1
    have : qs[m] = r := (List.getElem?_eq_some_iff.mp hr).2
    exact hne m hm hmj (by rw [this, ← evK_real]; exact hr0)
  -- 2. off the sweep's events
  obtain ⟨E, hE, hplanes⟩ := samplesAt_parts h
  obtain ⟨p1, h01, hs1, hoff1⟩ := facet_offX hf hjq E h0 hne hg
  have hp1 := cell_of_signs hp hjq h01 hXj hs1
  -- 3. on a sample plane, off the events
  obtain ⟨c, hc, p2, hp2, hp2c, hoff2⟩ := facet_meets_plane hE hbi hB X hXb hj hXj hXo hp1 hoff1
  obtain ⟨ps, hps, hpsm⟩ := hplanes c hc
  obtain ⟨es, turns, Ey, hes, hturns, hEy, hlines⟩ := planeSamples_parts hps
  obtain ⟨h02, hne2⟩ := facet_signs hp2 hjq hXj hXo
  have hg2 := off_grad hE hoff2 (List.getElem_mem hjq) h02
  -- 4. off the plane's events
  have hp2e : p2 = emb (c : ℝ) (p2.2.1, p2.2.2) := by
    obtain ⟨x, y, z⟩ := p2; simp only at hp2c; simp [emb, hp2c]
  obtain ⟨w, hw0, hws, hwoff⟩ := plane_offY hf hjq Ey (c := (c : ℝ)) (w0 := (p2.2.1, p2.2.2))
    (by show qR qs[j] (emb c _) = 0; rw [← hp2e]; exact h02)
    (fun m hm hmj => by show qR qs[m] (emb c _) ≠ 0; rw [← hp2e]; exact hne2 m hm hmj)
    (by rw [← hp2e]; exact hg2)
  have hp3 : covCellK (qs.map qR) X (emb c w) :=
    cell_of_signs hp2 hjq hw0 hXj (fun m hm hmj => by
      have := hws m hm hmj
      rw [hp2e]
      exact this)
  -- 5. on a sample line
  obtain ⟨c', hc', t, hpt⟩ := plane_to_line hf hes hturns hEy hbi hB X hXb hj hXj hXo hp3 rfl hwoff
  -- 6. a facet sample
  obtain ⟨ls, hls, hlsm⟩ := hlines c' hc'
  obtain ⟨h0t, -⟩ := facet_signs hpt hjq hXj hXo
  obtain ⟨s, hs, hscov⟩ := lineSamples_complete realModel hls
    (q := qs[j]) (by rw [activeOn_eq hf]; exact List.getElem_mem hjq) (t := t)
    (by rw [realModel_q, realModel_q, evK_real]; exact h0t)
  refine ⟨s, hpsm s (hlsm s hs), ?_⟩
  rw [hscov, realModel_q, realModel_q]
  have : (qs.map fun q => Quadric.evK realModel q) = qs.map qR := by
    simpa [carriersOf] using carriersOf_real qs
  rw [this]
  exact hpt

end ForgeCurved
