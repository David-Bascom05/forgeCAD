import ForgeCurved.Transversal
import ForgeCurved.Faces

/-!
# `NoClosedPiece` from the zero loci

A closed piece of a cell `Y` (closed, cut out of `Y` by an open set) is open and closed in the
zero locus `Σ_Y` of the carriers `Y` is zero on, and it avoids every other carrier. So it holds
whole components of `Σ_Y`. **`noClosedPiece_of_locus`**: if every component of `Σ_Y` meets some
other carrier, `Y` has no closed piece. For a facet on a cylinder, `Σ_Y` is the cylinder and its
axial seam meets it; for an edge, `Σ_Y` is the intersection curve and every component of it must
reach a vertex (the enumerator's arcs end at vertices).
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set

variable {P : Type} [TopologicalSpace P]

/-- The zero locus of the carriers a covector is zero on. -/
def zeroLocus (cs : List (Carrier P ℝ)) (Y : Cov) : Set P :=
  {x | ∀ i (h : i < cs.length) (h' : i < Y.length), Y[i] = .zero → cs[i] x = 0}

/-- The points with `Y`'s signs off its zeros. -/
def offSigns (cs : List (Carrier P ℝ)) (Y : Cov) : Set P :=
  {x | ∀ i (h : i < cs.length) (h' : i < Y.length), Y[i] ≠ .zero → sgnK (cs[i] x) = Y[i]}

theorem isOpen_offSigns {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c) (Y : Cov) :
    IsOpen (offSigns cs Y) := by
  rw [isOpen_iff_mem_nhds]
  intro x hx
  filter_upwards [eventually_le hc x] with y hy
  obtain ⟨-, hxy⟩ := le_iff_getElem.mp hy
  intro i h h' hY
  have hxi := hx i h h' hY
  have hxz : (covOfK cs x)[i]'(by rw [length_covOfK]; exact h) ≠ .zero := by
    rw [getElem_covOfK, hxi]; exact hY
  have := (hxy i (by rw [length_covOfK]; exact h) (by rw [length_covOfK]; exact h)).resolve_left hxz
  rw [getElem_covOfK, getElem_covOfK] at this
  rw [← this, hxi]

theorem cell_eq_locus_inter {cs : List (Carrier P ℝ)} {Y : Cov} (hlen : Y.length = cs.length) :
    cell cs Y = zeroLocus cs Y ∩ offSigns cs Y := by
  ext x
  simp only [mem_cell, mem_inter_iff, zeroLocus, offSigns, mem_ofPred_eq]
  constructor
  · rintro rfl
    refine ⟨fun i h h' hz => ?_, fun i h h' _ => ?_⟩
    · rw [getElem_covOfK] at hz; exact sgnK_zero_iff.mp hz
    · rw [getElem_covOfK]
  · rintro ⟨h1, h2⟩
    apply List.ext_getElem (by rw [length_covOfK, hlen])
    intro i hi hi'
    have hic : i < cs.length := by rwa [length_covOfK] at hi
    rw [getElem_covOfK]
    by_cases hz : Y[i] = .zero
    · rw [hz]; exact sgnK_zero_iff.mpr (h1 i hic hi' hz)
    · exact h2 i hic hi' hz

/-- **No closed piece**, when every component of the zero locus meets another carrier. -/
theorem noClosedPiece_of_locus {cs : List (Carrier P ℝ)} (hc : ∀ c ∈ cs, Continuous c) {Y : Cov}
    (hlen : Y.length = cs.length)
    (h : ∀ x ∈ zeroLocus cs Y, ∃ y ∈ connectedComponentIn (zeroLocus cs Y) x,
      ∃ i, ∃ (hi : i < cs.length) (hi' : i < Y.length), Y[i] ≠ .zero ∧ cs[i] y = 0) :
    NoClosedPiece cs Y := by
  intro v hv ⟨x, hxY, hxv⟩ hcl
  set Zl := zeroLocus cs Y
  set O := v ∩ offSigns cs Y
  have hO : IsOpen O := hv.inter (isOpen_offSigns hc Y)
  have hC : cell cs Y ∩ v = Zl ∩ O := by
    rw [cell_eq_locus_inter hlen]; ext; simp only [mem_inter_iff, O]; tauto
  have hxZ : x ∈ Zl := ((cell_eq_locus_inter hlen) ▸ hxY : x ∈ Zl ∩ offSigns cs Y).1
  obtain ⟨y, hyK, i, hi, hi', hYi, hyi⟩ := h x hxZ
  set K := connectedComponentIn Zl x
  have hKZ : K ⊆ Zl := connectedComponentIn_subset _ _
  have hKsub : K ⊆ O ∪ (cell cs Y ∩ v)ᶜ := by
    intro z hz
    by_cases hzC : z ∈ cell cs Y ∩ v
    · rw [hC] at hzC; exact Or.inl hzC.2
    · exact Or.inr hzC
  have hKO : K ⊆ O := by
    rcases sub_or_sub (isPreconnected_connectedComponentIn) hO hcl.isOpen_compl hKsub
      (fun z hz h1 h2 => h2 (by rw [hC]; exact ⟨hKZ hz, h1⟩)) with h | h
    · exact h
    · exact absurd ⟨hxY, hxv⟩ (h (mem_connectedComponentIn hxZ))
  have hyY : y ∈ cell cs Y := by
    have : y ∈ cell cs Y ∩ v := by rw [hC]; exact ⟨hKZ hyK, hKO hyK⟩
    exact this.1
  have := (hKO hyK).2 i hi hi' hYi
  rw [hyi] at this
  exact hYi (this.symm.trans (sgnK_zero_iff.mpr rfl))

end ForgeCurved
