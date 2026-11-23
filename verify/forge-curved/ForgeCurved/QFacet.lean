import ForgeCurved.QSlabs
import ForgeCurved.Tier0

/-!
# Every facet off the events reaches a sample plane

The first step of `qsampler_covers`. Take a facet cell `X`: one zero, at `j`, and on the negative
side of a bounding carrier `b` (`qledgerB`'s sphere, in the frame). If `X` has a point whose `x`
is no event, then `X` meets one of the rational sample planes `planesOf (eventsOf qs)`.

The point lies in an event-free slab with a sample plane in it (`slab_find`). The slab is regular
(`slab_regular`), so LemmaD's bounded facet crossing (`crossesAll_facets_bounded`) carries the cell
to every level of the slab, the sample plane's among them.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep

theorem eventsOf_den {qs : List Quad.Quadric} {fuel : ℕ} {E : List Num}
    (h : eventsOf qs fuel = some E) : AllDen E := by
  unfold eventsOf at h
  cases hr : rawEvents qs fuel with
  | none => rw [hr] at h; cases h
  | some raw =>
    rw [hr, Option.bind_some] at h
    have hck := pickEvents_spec h
    simp only [checkEvents, Bool.and_eq_true, List.all_eq_true] at hck
    intro e he
    exact denotes_of_isoOkN realModel (hck.1.2 e he)

theorem finrank_R3 : Module.finrank ℝ R3 = 3 := by
  simp [Module.finrank_prod]

/-- **A facet with a point off the events meets a sample plane.** -/
theorem facet_meets_plane {qs : List Quad.Quadric} {fuel : ℕ} {E : List Num}
    (h : eventsOf qs fuel = some E) {bi : ℕ} (hbi : bi < (qs.map qR).length)
    (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0}) (X : Cov)
    (hXb : X[bi]? = some Sgn.neg) {j : ℕ} (hj : j < (qs.map qR).length)
    (hXj : X[j]? = some Sgn.zero) (hXo : ∀ m, m ≠ j → X[m]? ≠ some Sgn.zero)
    {p : R3} (hp : covCellK (qs.map qR) X p) (hoff : OffEvents E p) :
    ∃ c ∈ planesOf E, ∃ p' : R3, covCellK (qs.map qR) X p' ∧ p'.1 = c ∧ OffEvents E p' := by
  obtain ⟨c, hc, a, b, hat, htb, hac, hcb, hout⟩ := slab_find (eventsOf_sep h) (eventsOf_den h)
    (t := p.1) hoff
  have hfree : ∀ z ∈ Slab ℓx a b, OffEvents E z := by
    intro z hz e he hd
    rcases hout e he z.1 hd with h1 | h1
    · exact absurd hz.1 (not_lt.mpr (by simpa [ℓx] using h1))
    · exact absurd hz.2 (not_lt.mpr (by simpa [ℓx] using h1))
  obtain ⟨ht1, ht2, h3⟩ := slab_regular h hfree
  have hC : ∀ c ∈ qs.map qR, ContDiff ℝ 1 c := by
    intro c hc
    obtain ⟨q, -, rfl⟩ := List.mem_map.mp hc
    exact qR_contDiff q
  obtain ⟨p', hp', hl⟩ := crossesAll_facets_bounded finrank_R3 hC ht1 ht2 hbi hB h3 X hXb hj hXj hXo
    ⟨p, ⟨by simpa [ℓx] using hat, by simpa [ℓx] using htb⟩, hp⟩ c hac hcb
  have hl' : p'.1 = c := by simpa [ℓx] using hl
  exact ⟨c, hc, p', hp', hl', hfree p' ⟨by rw [show ℓx p' = p'.1 from rfl, hl']; exact hac,
    by rw [show ℓx p' = p'.1 from rfl, hl']; exact hcb⟩⟩

end ForgeCurved
