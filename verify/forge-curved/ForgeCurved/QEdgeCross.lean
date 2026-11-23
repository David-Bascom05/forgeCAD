import ForgeCurved.QPairChart

/-!
# A cell with two zeros, off the events, crosses its slab

**`edge_slab`**: a cell with two zero carriers, on the bounding carrier's non-positive side, with a
point off the sweep's events, has points at every level of the event-free slab round that point.
That slab holds a sample plane. So the cell is not a point, and it meets a sample plane, where
`edgesOn_complete` makes it an edge sample.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem two_idx : ∀ (X : Cov), 2 ≤ X.count Sgn.zero →
    ∃ a b : ℕ, a ≠ b ∧ X[a]? = some Sgn.zero ∧ X[b]? = some Sgn.zero
  | [], h => by simp at h
  | s :: X, h => by
    by_cases hs : s = Sgn.zero
    · subst hs
      simp only [List.count_cons_self] at h
      have : Sgn.zero ∈ X := List.count_pos_iff.mp (by omega)
      obtain ⟨i, hi, hx⟩ := List.getElem_of_mem this
      exact ⟨0, i + 1, by omega, rfl, by simp [List.getElem?_eq_getElem hi, hx]⟩
    · rw [List.count_cons_of_ne hs] at h
      obtain ⟨a, b, hab, ha, hb⟩ := two_idx X h
      exact ⟨a + 1, b + 1, by omega, by simpa using ha, by simpa using hb⟩

/-- **A two-zero cell off the events crosses its event-free slab.** -/
theorem edge_slab {qs : List Quad.Quadric} {fuel : ℕ} {E : List Num}
    (h : eventsOf qs fuel = some E) {bi : ℕ} (hbi : bi < (qs.map qR).length)
    (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0}) (X : Cov)
    (hXb : X[bi]? = some Sgn.neg ∨ X[bi]? = some Sgn.zero) {j k : ℕ} (hjk : j ≠ k)
    (hXj : X[j]? = some Sgn.zero) (hXk : X[k]? = some Sgn.zero)
    {p : R3} (hp : covCellK (qs.map qR) X p) (hoff : OffEvents E p) :
    ∃ c ∈ planesOf E, ∃ a b : ℝ, a < p.1 ∧ p.1 < b ∧ a < c ∧ c < b ∧
      ∀ t, a < t → t < b → ∃ x, covCellK (qs.map qR) X x ∧ x.1 = t := by
  obtain ⟨c, hc, a, b, hat, htb, hac, hcb, hout⟩ := slab_find (eventsOf_sep h) (eventsOf_den h)
    (t := p.1) hoff
  have hfree : ∀ z ∈ Slab ℓx a b, OffEvents E z := by
    intro z hz e he hd
    rcases hout e he z.1 hd with h1 | h1
    · exact absurd hz.1 (not_lt.mpr (by simpa [ℓx] using h1))
    · exact absurd hz.2 (not_lt.mpr (by simpa [ℓx] using h1))
  obtain ⟨-, ht2, h3⟩ := slab_regular h hfree
  have hC : ∀ c ∈ qs.map qR, ContDiff ℝ 1 c := by
    intro c hc
    obtain ⟨q, -, rfl⟩ := List.mem_map.mp hc
    exact qR_contDiff q
  refine ⟨c, hc, a, b, hat, htb, hac, hcb, fun t hta htb' => ?_⟩
  obtain ⟨x, hx, hl⟩ := crossesAll_edges_bounded finrank_R3 hC ht2 hbi hB h3 X hXb hjk hXj hXk
    ⟨p, ⟨by simpa [ℓx] using hat, by simpa [ℓx] using htb⟩, hp⟩ t hta htb'
  exact ⟨x, hx, by simpa [ℓx] using hl⟩

/-- A sample plane is no event. -/
theorem planes_off {E : List Num} (hs : sepOk E = true) (hd : AllDen E) {c : ℚ}
    (hc : c ∈ planesOf E) : ∀ e ∈ E, ¬ Denotes realModel e (c : ℝ) := by
  intro e he hde
  have hval := valE_eq hde
  cases E with
  | nil => simp at he
  | cons u rest =>
    have hb := bounds u rest hs hd
    simp only [planesOf, List.mem_cons, List.mem_append, List.mem_map] at hc
    rcases hc with (rfl | ⟨⟨p1, p2⟩, hp, hpc⟩) | hc
    · -- below the first
      have := Denotes.lo_le realModel (valE_den (hd u (by simp)))
      simp only [realModel_q] at this
      have h1 := (hb e he).1
      rw [← hval] at h1
      push_cast at h1
      linarith
    · -- between two consecutive
      have hp' : (p1, p2) ∈ (u :: rest).zip (u :: rest).tail := by simpa using hp
      have h2 : p2 ∈ u :: rest := List.mem_of_mem_tail (List.of_mem_zip hp').2
      have hmid := planes_between realModel hs (u := p1) (v := p2) (by simpa using hp')
        (valE_den (hd _ (List.of_mem_zip hp').1)) (valE_den (hd _ h2))
      simp only [realModel_q] at hmid
      rw [← hpc] at hval
      have hg := gap (u :: rest) hs hd (p1, p2) hp' e he
      rw [← hval] at hg
      push_cast at hmid hg
      rcases hg with h | h <;> linarith [hmid.1, hmid.2]
    · -- above the last
      set w := (u :: rest).getLast (by simp)
      have hw : w ∈ u :: rest := List.getLast_mem _
      have := Denotes.le_hi realModel (valE_den (hd w hw))
      simp only [realModel_q] at this
      have h1 := (hb e he).2
      rw [← hval] at h1
      have hc' : c = w.hi + 1 := by
        rcases hc with hc | hc
        · rw [hc]; simp [w, List.getLast?_eq_some_getLast]
        · simp at hc
      rw [hc'] at h1
      push_cast at h1
      linarith

end ForgeCurved
