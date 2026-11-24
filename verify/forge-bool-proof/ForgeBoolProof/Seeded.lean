import ForgeBoolProof.Assembly

/-!
# D8.seeded: the result's faces are `Seeded`

`Seeded Rs`: every connected open set off the result's faces is unbounded, or reaches a point
where one face is alone. Take the component `C` of a point's complement of the faces. If `C` is
bounded, its frontier is on the result's faces, which lie on the arrangement's faces. It is not
all on their edges and vertices (`OffFacets`): those have empty interior and do not cut small
neighbourhoods (`Thin`, row D5.thin), so they do not cut space either (`compl_preconnected`).
Then `C` and the outside of its closure, both met off them, would split a connected set. So the
frontier of `C` meets a facet, and there a result face is alone (`LoneOnFacets`: `apply` writes
each kept facet into one face).

The arrangement's faces are closed: since D5.zone's ruling Z2 each body's faces are its solid's
frontier (`faces_eq_frontier`).
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof

open Set Filter Topology ForgeCurved.BodyModel ForgeCurved.FaceLocal ForgeCurved.FacetSides

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- **Thin sets do not cut space.** In a preconnected space, a set with empty interior that does
not cut small neighbourhoods of its points leaves a preconnected complement. -/
theorem compl_preconnected [PreconnectedSpace E] {N : Set E} (hN : interior N = ∅)
    (hcut : ∀ x ∈ N, ∃ V : Set E, IsOpen V ∧ x ∈ V ∧ IsPreconnected (V \ N)) :
    IsPreconnected Nᶜ := by
  rw [isPreconnected_iff_subset_of_disjoint]
  intro u v hu hv hcov hdis
  set A := {x | ∀ᶠ z in 𝓝 x, z ∉ N → z ∈ u}
  set B := {x | ∀ᶠ z in 𝓝 x, z ∉ N → z ∈ v}
  have hAo : IsOpen A := isOpen_setOfPred_eventually_nhds
  have hBo : IsOpen B := isOpen_setOfPred_eventually_nhds
  -- every point is in `A` or `B`
  have hAB : (univ : Set E) ⊆ A ∪ B := by
    rintro x -
    by_cases hx : x ∈ N
    · obtain ⟨V, hVo, hxV, hVc⟩ := hcut x hx
      have hsub : V \ N ⊆ u ∪ v := fun z hz => hcov hz.2
      have hdis' : V \ N ∩ (u ∩ v) = ∅ := by
        rw [eq_empty_iff_forall_notMem]
        intro z ⟨hz, hzuv⟩
        have : z ∈ Nᶜ ∩ (u ∩ v) := ⟨hz.2, hzuv⟩
        rw [hdis] at this; exact this
      rcases (isPreconnected_iff_subset_of_disjoint.mp hVc) u v hu hv hsub hdis' with h | h
      · exact Or.inl (Filter.mem_of_superset (hVo.mem_nhds hxV) fun z hz hzN => h ⟨hz, hzN⟩)
      · exact Or.inr (Filter.mem_of_superset (hVo.mem_nhds hxV) fun z hz hzN => h ⟨hz, hzN⟩)
    · rcases hcov hx with h | h
      · exact Or.inl (Filter.mem_of_superset (hu.mem_nhds h) fun z hz _ => hz)
      · exact Or.inr (Filter.mem_of_superset (hv.mem_nhds h) fun z hz _ => hz)
  -- and in not both
  have hABd : (univ : Set E) ∩ (A ∩ B) = ∅ := by
    rw [eq_empty_iff_forall_notMem]
    rintro x ⟨-, hxA, hxB⟩
    obtain ⟨W, hW, hWo, hxW⟩ := mem_nhds_iff.mp (hxA.and hxB)
    obtain ⟨z, ⟨hzW, hzN⟩⟩ : (W \ N).Nonempty :=
      diff_nonempty_of_open hWo ⟨x, hxW⟩ hN
    obtain ⟨h1, h2⟩ := hW hzW
    have : z ∈ Nᶜ ∩ (u ∩ v) := ⟨hzN, h1 hzN, h2 hzN⟩
    rw [hdis] at this; exact this
  rcases (isPreconnected_iff_subset_of_disjoint.mp isPreconnected_univ) A B hAo hBo hAB hABd with
    h | h
  · exact Or.inl fun x hx => (h (mem_univ x)).self_of_nhds hx
  · exact Or.inr fun x hx => (h (mem_univ x)).self_of_nhds hx

/-- The component of a point in an open set is open (balls are connected). -/
theorem isOpen_component {U : Set E} (hU : IsOpen U) (x : E) : IsOpen (connectedComponentIn U x) := by
  rw [Metric.isOpen_iff]
  intro y hy
  obtain ⟨ε, hε, hball⟩ := Metric.isOpen_iff.mp hU y (connectedComponentIn_subset U x hy)
  refine ⟨ε, hε, ?_⟩
  have hc := IsPreconnected.union y (Metric.mem_ball_self hε) hy (convex_ball y ε).isPreconnected
    (isPreconnected_connectedComponentIn (x := x) (F := U))
  have hx : x ∈ Metric.ball y ε ∪ connectedComponentIn U x :=
    Or.inr (mem_connectedComponentIn (connectedComponentIn_nonempty_iff.mp ⟨y, hy⟩))
  exact (subset_union_left.trans
    (hc.subset_connectedComponentIn hx (union_subset hball (connectedComponentIn_subset U x))))

/-- The frontier of a component of an open set is off the set. -/
theorem frontier_component {U : Set E} (hU : IsOpen U) (x : E) :
    frontier (connectedComponentIn U x) ⊆ Uᶜ := by
  intro y hy hyU
  obtain ⟨ε, hε, hball⟩ := Metric.isOpen_iff.mp hU y hyU
  obtain ⟨w, hwb, hwC⟩ := mem_closure_iff.mp hy.1 _ Metric.isOpen_ball (Metric.mem_ball_self hε)
  have hc := (convex_ball y ε).isPreconnected.union w hwb hwC
    (isPreconnected_connectedComponentIn (x := x) (F := U))
  have hxC : x ∈ connectedComponentIn U x :=
    mem_connectedComponentIn (connectedComponentIn_nonempty_iff.mp ⟨w, hwC⟩)
  have hsub := hc.subset_connectedComponentIn (Or.inr hxC)
    (union_subset hball (connectedComponentIn_subset U x))
  have hyC : y ∈ connectedComponentIn U x := hsub (Or.inl (Metric.mem_ball_self hε))
  exact hy.2 (by rw [(isOpen_component hU x).interior_eq]; exact hyC)

/-- Since D5.zone's ruling Z2, a valid body's faces are its solid's frontier. -/
theorem faces_eq_frontier {b : BodyModel E} (hv : ValidExact b) : Faces b = frontier (denoteExact b) :=
  Subset.antisymm (bounds_denoteExact hv).faces_sub (bounds_denoteExact hv).frontier_sub

/-- **`apply` writes each kept facet into one face**: a result face point on a facet is a point
where one result face is alone. -/
def LoneOnFacets (a : Arr E) (Rs : BodyModel E) : Prop :=
  ∀ y ∈ Faces Rs, y ∈ FacetPts a.G → ∃ F ∈ Rs.faces, Alone Rs F y

/-- **D8.seeded.** -/
theorem seeded_result [Nontrivial E] {Aex Bex Rs : BodyModel E} {a : Arr E}
    (hvA : ValidExact Aex) (hvB : ValidExact Bex) (ha : ArrOK Aex Bex a) (ht : Thin a)
    {k : Nat → Bool} (hw : WritesKept a k Rs) (hlone : LoneOnFacets a Rs) : Seeded Rs := by
  -- the arrangement's faces are closed, and the result's lie on them
  have hFG : IsClosed a.FacesG := by
    rw [ha.faces, faces_eq_frontier hvA, faces_eq_frontier hvB]
    exact isClosed_frontier.union isClosed_frontier
  have hRcl : IsClosed (Faces Rs) := by rw [hw.faces_eq]; exact isClosed_closure
  have hRG : Faces Rs ⊆ a.FacesG := by
    rw [hw.faces_eq]
    refine closure_minimal ?_ hFG
    intro y hy
    simp only [KeptFacets, mem_iUnion] at hy
    obtain ⟨f, -, -, hy⟩ := hy
    exact ha.data.facet_faces _ hy
  intro x hx
  set U := (Faces Rs)ᶜ
  have hUo : IsOpen U := hRcl.isOpen_compl
  set C := connectedComponentIn U x
  have hxC : x ∈ C := mem_connectedComponentIn hx
  have hCo : IsOpen C := isOpen_component hUo x
  refine ⟨C, hCo, isPreconnected_connectedComponentIn, hxC,
    Set.disjoint_left.mpr fun z hz hzF => connectedComponentIn_subset U x hz hzF, ?_⟩
  by_cases hb : Bornology.IsBounded C
  swap
  · exact Or.inl hb
  right
  -- a point of `C`'s closure on a facet and on the result's faces
  by_contra hno
  have hoff : frontier C ⊆ OffFacets a := by
    intro y hy
    have hyR : y ∈ Faces Rs := by
      have := frontier_component hUo x hy
      simpa [U] using this
    refine ⟨hRG hyR, fun hyP => hno ?_⟩
    obtain ⟨F, hF, hal⟩ := hlone y hyR hyP
    exact ⟨F, hF, y, hal, hy.1⟩
  have hpre := compl_preconnected (E := E) ht.no_interior fun z hz => by
    obtain ⟨V, -, hVo, hzV, hVc⟩ := ht.uncut z hz.1 univ univ_mem
    exact ⟨V, hVo, hzV, hVc⟩
  -- `C` and the outside of its closure, both met off the edges, split the complement
  have hA : (C \ OffFacets a).Nonempty := diff_nonempty_of_open hCo ⟨x, hxC⟩ ht.no_interior
  have hBne : (closure C)ᶜ.Nonempty := by
    by_contra h
    rw [not_nonempty_iff_eq_empty, compl_empty_iff] at h
    exact NormedSpace.unbounded_univ ℝ E (h ▸ hb.closure)
  have hB : ((closure C)ᶜ \ OffFacets a).Nonempty :=
    diff_nonempty_of_open isClosed_closure.isOpen_compl hBne ht.no_interior
  have hcov : (OffFacets a)ᶜ ⊆ C ∪ (closure C)ᶜ := by
    intro z hz
    by_cases hzC : z ∈ C
    · exact Or.inl hzC
    · right; intro hzcl
      exact hz (hoff ⟨hzcl, by rwa [hCo.interior_eq]⟩)
  have hdis : (OffFacets a)ᶜ ∩ (C ∩ (closure C)ᶜ) = ∅ := by
    rw [eq_empty_iff_forall_notMem]
    rintro z ⟨-, hzC, hzn⟩
    exact hzn (subset_closure hzC)
  rcases (isPreconnected_iff_subset_of_disjoint.mp hpre) C (closure C)ᶜ hCo
      isClosed_closure.isOpen_compl hcov hdis with h | h
  · obtain ⟨z, hz, hzN⟩ := hB
    exact hz (subset_closure (h hzN))
  · obtain ⟨z, hz, hzN⟩ := hA
    exact h hzN (subset_closure hz)

end ForgeBoolProof
