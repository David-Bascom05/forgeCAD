import ForgeCurved.QPlane
import ForgeExact.DecideOnce.QCover

/-!
# On a sample plane, a facet reaches a sample line

The second stage of `qsampler_covers`. On the sample plane `x = c`, a facet cell's point whose `y`
is none of the plane's events lies on a curve of its carrier's trace. In the event-free `y`-slab
round it, that curve meets no other carrier, since a crossing would be an event
(`edgesOn_complete`). It is transverse to the level lines of `y`, since a turn would be an event
(`turnsOn_complete`). So its component stays in the cell (`Thom.sgnK_const`) and reaches every
level of the slab (`component_crossesS`, in `ℝ²`), the sample line's among them.

* **`plane_to_line`**.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem activeOn_eq {qs : List Quad.Quadric} (hf : frameOk qs = true) (c : ℚ) :
    activeOn qs c = qs := by
  unfold activeOn
  apply List.filter_eq_self.mpr
  intro q hq
  have := List.all_eq_true.mp hf q hq
  simp only [Bool.or_eq_true, Bool.and_eq_true, bne_iff_ne, ne_eq] at this
  rcases this with ⟨hp, hlz⟩ | hzz
  · obtain ⟨-, -, -, -, h5, -⟩ := isPlaneQ_iff hp
    simp [zeroOn, restr, Poly.isZero, h5, hlz]
  · simp [zeroOn, restr, Poly.isZero, hzz]

theorem noXPlane_eq {qs : List Quad.Quadric} (hf : frameOk qs = true) :
    (qs.filter fun q => !isXPlane q) = qs := by
  apply List.filter_eq_self.mpr
  intro q hq
  have := List.all_eq_true.mp hf q hq
  simp only [Bool.or_eq_true, Bool.and_eq_true, bne_iff_ne, ne_eq] at this
  rcases this with ⟨-, hlz⟩ | hzz
  · simp [isXPlane, hlz]
  · simp [isXPlane, isPlaneQ, hzz]

theorem den_cast {e : Num} {t : ℝ} : Denotes realModel e t ↔ Denotes realModel e t := Iff.rfl

/-- A pair of distinct indices, in `pairs` one way round. -/
theorem pair_of {qs : List Quad.Quadric} {j m : ℕ} (hj : j < qs.length) (hm : m < qs.length)
    (hjm : j ≠ m) : (qs[j], qs[m]) ∈ pairs qs ∨ (qs[m], qs[j]) ∈ pairs qs := by
  rcases Nat.lt_or_gt_of_ne hjm with h | h
  · exact Or.inl (mem_pairs hj hm h)
  · exact Or.inr (mem_pairs hm hj h)

theorem evK_plane (q : Quad.Quadric) (c : ℚ) (w : ℝ × ℝ) :
    Quadric.evK realModel q (realModel.q c, w.1, w.2) = onPlane c q w := by
  rw [realModel_q, evK_real]; rfl

theorem dzK_plane_real (q : Quad.Quadric) (c : ℚ) (w : ℝ × ℝ) :
    dzK realModel q (realModel.q c, w.1, w.2) = gz q (emb c w) := by
  rw [dzK_real]; rfl

/-- **On a sample plane, a facet point off the plane's events reaches a sample line.** -/
theorem plane_to_line {qs : List Quad.Quadric} (hf : frameOk qs = true) {c : ℚ}
    {es : List ESample} {turns Ey : List Num}
    (hes : edgesOn ((activeOn qs c).filter fun q => !isXPlane q) c = some es)
    (hturns : turnsOn (activeOn qs c) c = some turns)
    (hEy : pickEvents (turns ++ es.map ESample.y) = some Ey)
    {bi : ℕ} (hbi : bi < (qs.map qR).length)
    (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0}) (X : Cov)
    (hXb : X[bi]? = some Sgn.neg) {j : ℕ} (hj : j < (qs.map qR).length)
    (hXj : X[j]? = some Sgn.zero) (hXo : ∀ m, m ≠ j → X[m]? ≠ some Sgn.zero)
    {p : R3} (hp : covCellK (qs.map qR) X p) (hpc : p.1 = c)
    (hoff : ∀ e ∈ Ey, ¬ Denotes realModel e p.2.1) :
    ∃ c' ∈ planesOf Ey, ∃ t : ℝ, covCellK (qs.map qR) X ((c : ℝ), (c' : ℝ), t) := by
  rw [activeOn_eq hf, noXPlane_eq hf] at hes
  rw [activeOn_eq hf] at hturns
  have hjq : j < qs.length := by simpa using hj
  have hbq : bi < qs.length := by simpa using hbi
  have hck := pickEvents_spec hEy
  simp only [checkEvents, Bool.and_eq_true, List.all_eq_true] at hck
  have hden : AllDen Ey := fun e he => denotes_of_isoOkN realModel (hck.1.2 e he)
  obtain ⟨c', hc', a2, b2, hat, htb, hac, hcb, hout⟩ := slab_find hck.2 hden (t := p.2.1) hoff
  -- a level of the slab is no event
  have hfree : ∀ t ∈ Ioo a2 b2, ∀ e ∈ Ey, ¬ Denotes realModel e t := fun t ht e he hd => by
    rcases hout e he t hd with h | h
    · exact absurd ht.1 (not_lt.mpr h)
    · exact absurd ht.2 (not_lt.mpr h)
  set g := fun m : Fin qs.length => onPlane (c : ℝ) qs[m]
  set gj := onPlane (c : ℝ) qs[j]
  set Z := Zs ℓy [gj] a2 b2
  set w0 : ℝ × ℝ := (p.2.1, p.2.2)
  have hpw : p = emb c w0 := by
    obtain ⟨x, y, z⟩ := p; simp only at hpc; simp [emb, w0, hpc]
  -- the cell's signs at `p`
  have hsig : ∀ m (hm : m < qs.length), X[m]? = some (sgnK (qR qs[m] p)) := by
    intro m hm
    rw [← hp, getElem?_covOfK]; simp [hm]
  have hgj0 : gj w0 = 0 := by
    have := hsig j hjq
    rw [hXj] at this
    have h0 := sgnK_zero_iff.mp (Option.some.inj this).symm
    rw [hpw] at h0; exact h0
  have hw0 : w0 ∈ Z := ⟨⟨hat, htb⟩, by simp [gj, hgj0]⟩
  -- on `Z`, no other carrier vanishes (a crossing would be an event)
  have hnz : ∀ w ∈ Z, ∀ m (hm : m < qs.length), m ≠ j → onPlane (c : ℝ) qs[m] w ≠ 0 := by
    intro w hw m hm hmj h0
    have hwj : onPlane (c : ℝ) qs[j] w = 0 := by
      have := hw.2; simpa [gj] using this
    have hev : ∃ e ∈ es, Denotes realModel e.y w.1 := by
      rcases pair_of hjq hm (Ne.symm hmj) with hpr | hpr
      · exact edgesOn_complete realModel hes hpr (by rw [evK_plane]; exact hwj)
          (by rw [evK_plane]; exact h0)
      · exact edgesOn_complete realModel hes hpr (by rw [evK_plane]; exact h0)
          (by rw [evK_plane]; exact hwj)
    obtain ⟨e, he, hd⟩ := hev
    obtain ⟨e', he', hd'⟩ := pickEvents_cover realModel hEy
      (List.mem_append_right _ (List.mem_map.mpr ⟨e, he, rfl⟩)) hd
    exact hfree w.1 hw.1 e' he' hd'
  -- on `Z`, the trace is transverse to the level lines of `y` (a turn would be an event)
  have ht : ∀ w ∈ Z, Transverse ℓy [gj] w := by
    intro w hw
    apply transverseY
    intro hz
    have hwj : onPlane (c : ℝ) qs[j] w = 0 := by
      have := hw.2; simpa [gj] using this
    obtain ⟨n, hn, hd⟩ := turnsOn_complete realModel hturns (List.getElem_mem hjq)
      (by rw [evK_plane]; exact hwj) (by rw [dzK_plane_real]; exact hz)
    obtain ⟨e', he', hd'⟩ := pickEvents_cover realModel hEy (List.mem_append_left _ hn) hd
    exact hfree w.1 hw.1 e' he' hd'
  -- the component keeps every other carrier's sign
  set D := connectedComponentIn Z w0
  have hDZ : D ⊆ Z := connectedComponentIn_subset _ _
  have hDc : IsPreconnected D := isPreconnected_connectedComponentIn
  have hw0D : w0 ∈ D := mem_connectedComponentIn hw0
  have hkeep : ∀ w ∈ D, ∀ m (hm : m < qs.length), m ≠ j →
      sgnK (onPlane (c : ℝ) qs[m] w) = sgnK (onPlane (c : ℝ) qs[m] w0) := by
    intro w hw m hm hmj
    exact Thom.sgnK_const hDc (onPlane_contDiff _ _).continuous.continuousOn
      (fun u hu => hnz u (hDZ hu) m hm hmj) hw hw0D
  have hbj : bi ≠ j := by intro h; subst h; rw [hXj] at hXb; cases hXb
  -- the bounding carrier stays negative, so the component is bounded
  set S := {w : ℝ × ℝ | onPlane (c : ℝ) qs[bi] w ≤ 0}
  have hSc : IsClosed S := isClosed_le (onPlane_contDiff _ _).continuous continuous_const
  have hB0 : sgnK (onPlane (c : ℝ) qs[bi] w0) = Sgn.neg := by
    have := hsig bi hbq
    rw [hXb, hpw] at this
    exact (Option.some.inj this).symm
  have hDS : D ⊆ S := by
    intro w hw
    have := hkeep w hw bi hbq hbj
    rw [hB0] at this
    show onPlane (c : ℝ) qs[bi] w ≤ 0
    rcases lt_trichotomy (onPlane (c : ℝ) qs[bi] w) 0 with h | h | h
    · exact h.le
    · rw [sgnK_zero_iff.mpr h] at this; cases this
    · rw [sgnK_pos h] at this; cases this
  have hSb : Bornology.IsBounded S := by
    obtain ⟨C, hC⟩ := (Metric.isBounded_iff_subset_closedBall 0).mp hB
    refine (Metric.isBounded_iff_subset_closedBall 0).mpr ⟨C, fun w hw => ?_⟩
    have h3 := hC (show emb c w ∈ {z | (qs.map qR)[bi] z ≤ 0} by
      simpa [S, onPlane, emb] using hw)
    simp only [Metric.mem_closedBall, dist_zero_right] at h3 ⊢
    calc ‖w‖ ≤ ‖emb c w‖ := by simp [emb, Prod.norm_def]
      _ ≤ C := h3
  -- the component reaches the sample line
  obtain ⟨w, hwD, hwl⟩ := component_crossesS (ks := [gj]) (fun g hg => by
      simp only [List.mem_singleton] at hg; subst hg; exact onPlane_contDiff _ _)
    (finrank_R2' gj) ht hSc (fun _ _ _ _ => hSb.subset fun z hz => hz.2.2.2) hw0 hDS c' ⟨hac, hcb⟩
  refine ⟨c', hc', w.2, ?_⟩
  have hw1 : w.1 = c' := by simpa [ℓy] using hwl
  -- its covector is the cell's
  show covOfK (qs.map qR) ((c : ℝ), (c' : ℝ), w.2) = X
  apply List.ext_getElem?
  intro m
  rw [getElem?_covOfK]
  by_cases hm : m < qs.length
  · simp only [List.getElem?_map, List.getElem?_eq_getElem hm, Option.map_some]
    by_cases hmj : m = j
    · subst hmj
      rw [hXj]
      have := (hDZ hwD).2
      simp only [List.mem_singleton, forall_eq, gj] at this
      congr 1
      apply sgnK_zero_iff.mpr
      rw [← hw1]; exact this
    · rw [hsig m hm]
      congr 1
      have := hkeep w hwD m hm hmj
      rw [← hw1]
      rw [hpw]
      exact this
  · have hm' : qs.length ≤ m := not_lt.mp hm
    simp only [List.getElem?_map, List.getElem?_eq_none hm', Option.map_none]
    have hlen : X.length = qs.length := by rw [← hp]; simp [covOfK]
    exact (List.getElem?_eq_none (by omega)).symm

end ForgeCurved
