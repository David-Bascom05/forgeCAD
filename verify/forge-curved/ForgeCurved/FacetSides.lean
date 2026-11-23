import ForgeCurved.Chart
import ForgeCurved.FaceLocal

/-!
# F5 at facet points: `SideLocal` from the builder's facet data

`SideLocal` (FaceLocal.lean) is the local half of F5. Here it is derived from what the builder
knows about each facet:
* rooms are open, pairwise disjoint, miss the faces, and cover the region off the faces;
* each facet `f` lies on a carrier `cf f` with a strict derivative `≠ 0` at each of its points.
  The facet is preconnected. Near each of its points, the only face points are the facet's, and
  they are the carrier's zeros there (`alone`);
* one certified sample per side: near some point of the facet, the `+` side is in room `rp f` and
  the `−` side in room `rm f`.

**`sideLocal_of_data`**: then `SideLocal G keep` holds for every `keep`.

The proof: at each facet point, small boxes (`Chart.box_halves`) have connected halves off the
faces, so each half lies in one room. The facet points whose `+` side is in `rp f` are open in
the facet, and so are those whose `+` side is in another room (`side_all`). The facet is
connected and the sample is in the first set, so every point is.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.FacetSides

open Set Filter Topology ForgeCurved.FaceLocal ForgeCurved.Chart

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The builder's facet data. -/
structure FacetData (G : FaceGeo E) (Faces : Set E) (cf : Nat → E → ℝ) : Prop where
  room_open : ∀ r, IsOpen (G.room r)
  room_disj : ∀ r r', r ≠ r' → Disjoint (G.room r) (G.room r')
  room_faces : ∀ r, Disjoint (G.room r) Faces
  room_cover : ∀ z, z ∉ Faces → ∃ r, z ∈ G.room r
  facet_faces : ∀ f, G.facet f ⊆ Faces
  facet_conn : ∀ f, IsPreconnected (G.facet f)
  zero : ∀ f, ∀ y ∈ G.facet f, cf f y = 0
  regular : ∀ f, ∀ y ∈ G.facet f, ∃ L : E →L[ℝ] ℝ, HasStrictFDerivAt (cf f) L y ∧ L ≠ 0
  alone : ∀ f, ∀ y ∈ G.facet f, ∃ W ∈ 𝓝 y, W ∩ Faces ⊆ G.facet f ∧ ∀ z ∈ W, cf f z = 0 → z ∈ G.facet f
  sample_p : ∀ f, (G.facet f).Nonempty →
    ∃ y₀ ∈ G.facet f, ∀ᶠ z in 𝓝 y₀, 0 < cf f z → z ∈ G.room (G.rp f)
  sample_m : ∀ f, (G.facet f).Nonempty →
    ∃ y₀ ∈ G.facet f, ∀ᶠ z in 𝓝 y₀, cf f z < 0 → z ∈ G.room (G.rm f)

variable {G : FaceGeo E} {Faces : Set E} {cf : Nat → E → ℝ}

/-- A preconnected nonempty set off the faces lies in one room. -/
theorem one_room (hd : FacetData G Faces cf) {S : Set E} (hS : IsPreconnected S) (hne : S.Nonempty)
    (hoff : ∀ z ∈ S, z ∉ Faces) : ∃ R, S ⊆ G.room R := by
  obtain ⟨z₀, hz₀⟩ := hne
  obtain ⟨R, hR⟩ := hd.room_cover z₀ (hoff z₀ hz₀)
  refine ⟨R, ?_⟩
  have hcov : S ⊆ G.room R ∪ ⋃ r, ⋃ (_ : r ≠ R), G.room r := by
    intro z hz
    obtain ⟨r, hr⟩ := hd.room_cover z (hoff z hz)
    by_cases h : r = R
    · subst h; exact Or.inl hr
    · exact Or.inr (mem_iUnion.mpr ⟨r, mem_iUnion.mpr ⟨h, hr⟩⟩)
  have hdisj : Disjoint (G.room R) (⋃ r, ⋃ (_ : r ≠ R), G.room r) := by
    rw [Set.disjoint_iUnion_right]; intro r
    rw [Set.disjoint_iUnion_right]; intro h
    exact hd.room_disj R r (Ne.symm h)
  rcases hS.subset_or_subset (hd.room_open R)
      (isOpen_iUnion fun r => isOpen_iUnion fun _ => hd.room_open r) hdisj hcov with h | h
  · exact h
  · exact absurd (h hz₀) (Set.disjoint_left.mp hdisj hR)

/-- Small charts at a facet point: an open box in any neighbourhood, inside the `alone`
neighbourhood, whose halves each lie in one room and reach the point. -/
theorem chart (hd : FacetData G Faces cf) {f : Nat} {y : E} (hy : y ∈ G.facet f) (sg : ℝ → Prop)
    (hsg : sg = (fun a => 0 < a) ∨ sg = (fun a => a < 0)) :
    ∃ N : Set E, IsOpen N ∧ y ∈ N ∧ (∃ R, N ∩ {z | sg (cf f z)} ⊆ G.room R) ∧
      y ∈ closure (N ∩ {z | sg (cf f z)}) := by
  obtain ⟨L, hL, hL0⟩ := hd.regular f y hy
  obtain ⟨W, hW, hWF, -⟩ := hd.alone f y hy
  obtain ⟨N, hNo, hyN, hNW, hp1, hp2, hn1, hn2⟩ := box_halves hL hL0 (hd.zero f y hy) hW
  have hoff : ∀ z ∈ N, cf f z ≠ 0 → z ∉ Faces := fun z hz h0 hzF =>
    h0 (hd.zero f z (hWF ⟨hNW hz, hzF⟩))
  rcases hsg with rfl | rfl
  · refine ⟨N, hNo, hyN, one_room hd hp1 ?_ fun z hz => hoff z hz.1 (ne_of_gt hz.2), hp2⟩
    by_contra h
    rw [not_nonempty_iff_eq_empty] at h
    rw [h, closure_empty] at hp2; exact hp2
  · refine ⟨N, hNo, hyN, one_room hd hn1 ?_ fun z hz => hoff z hz.1 (ne_of_lt hz.2), hn2⟩
    by_contra h
    rw [not_nonempty_iff_eq_empty] at h
    rw [h, closure_empty] at hn2; exact hn2

/-- **One side's room along a connected facet.** -/
theorem side_all (hd : FacetData G Faces cf) (f : Nat) (sg : ℝ → Prop)
    (hsg : sg = (fun a => 0 < a) ∨ sg = (fun a => a < 0)) (R₀ : Nat)
    (hsample : (G.facet f).Nonempty →
      ∃ y₀ ∈ G.facet f, ∀ᶠ z in 𝓝 y₀, sg (cf f z) → z ∈ G.room R₀) :
    ∀ y ∈ G.facet f, ∀ᶠ z in 𝓝 y, sg (cf f z) → z ∈ G.room R₀ := by
  set U := {y | ∀ᶠ z in 𝓝 y, sg (cf f z) → z ∈ G.room R₀}
  set V := ⋃ R, ⋃ (_ : R ≠ R₀), {y | ∀ᶠ z in 𝓝 y, sg (cf f z) → z ∈ G.room R}
  have hU : IsOpen U := isOpen_setOfPred_eventually_nhds
  have hV : IsOpen V := isOpen_iUnion fun _ => isOpen_iUnion fun _ => isOpen_setOfPred_eventually_nhds
  have hcov : G.facet f ⊆ U ∪ V := by
    intro y hy
    obtain ⟨N, hNo, hyN, ⟨R, hR⟩, -⟩ := chart hd hy sg hsg
    have hev : ∀ᶠ z in 𝓝 y, sg (cf f z) → z ∈ G.room R :=
      Filter.mem_of_superset (hNo.mem_nhds hyN) fun z hz hs => hR ⟨hz, hs⟩
    by_cases h : R = R₀
    · subst h; exact Or.inl hev
    · exact Or.inr (mem_iUnion.mpr ⟨R, mem_iUnion.mpr ⟨h, hev⟩⟩)
  have hdisj : G.facet f ∩ (U ∩ V) = ∅ := by
    ext y
    simp only [mem_inter_iff, mem_empty_iff_false, iff_false]
    rintro ⟨hy, hyU, hyV⟩
    obtain ⟨R, hR⟩ := mem_iUnion.mp hyV
    obtain ⟨hRR, hyR⟩ := mem_iUnion.mp hR
    obtain ⟨N, -, -, -, hcl⟩ := chart hd hy sg hsg
    have hfr := mem_closure_iff_frequently.mp hcl
    obtain ⟨z, ⟨⟨-, hzs⟩, hz₀⟩, hzR⟩ := ((hfr.and_eventually hyU).and_eventually hyR).exists
    exact Set.disjoint_left.mp (hd.room_disj R₀ R (Ne.symm hRR)) (hz₀ hzs) (hzR hzs)
  rcases (G.facet f).eq_empty_or_nonempty with he | hne
  · intro y hy; rw [he] at hy; exact absurd hy (notMem_empty y)
  obtain ⟨y₀, hy₀, hy₀U⟩ := hsample hne
  rcases (isPreconnected_iff_subset_of_disjoint.mp (hd.facet_conn f)) U V hU hV hcov hdisj with
    h | h
  · exact fun y hy => h hy
  · have : y₀ ∈ G.facet f ∩ (U ∩ V) := ⟨hy₀, hy₀U, h hy₀⟩
    rw [hdisj] at this; exact absurd this (notMem_empty _)

/-- **F5 at facet points, from the builder's data.** -/
theorem sideLocal_of_data (hd : FacetData G Faces cf) (keep : Nat → Bool) : SideLocal G keep := by
  have hp := fun f => side_all hd f (fun a => 0 < a) (Or.inl rfl) (G.rp f) (hd.sample_p f)
  have hm := fun f => side_all hd f (fun a => a < 0) (Or.inr rfl) (G.rm f) (hd.sample_m f)
  refine ⟨hd.room_open, fun r f => (hd.room_faces r).mono_right (hd.facet_faces f),
    fun r r' h => hd.room_disj r r' fun e => h (e ▸ rfl), fun f y hy => ⟨?_, ?_⟩, fun f y hy => ?_⟩
  · obtain ⟨N, -, -, -, hcl⟩ := chart hd hy (fun a => 0 < a) (Or.inl rfl)
    rw [mem_closure_iff_frequently] at hcl ⊢
    exact (hcl.and_eventually (hp f y hy)).mono fun z ⟨h1, h2⟩ => h2 h1.2
  · obtain ⟨N, -, -, -, hcl⟩ := chart hd hy (fun a => a < 0) (Or.inr rfl)
    rw [mem_closure_iff_frequently] at hcl ⊢
    exact (hcl.and_eventually (hm f y hy)).mono fun z ⟨h1, h2⟩ => h2 h1.2
  · obtain ⟨W, hW, -, hWz⟩ := hd.alone f y hy
    refine ⟨W ∩ {z | 0 < cf f z → z ∈ G.room (G.rp f)} ∩ {z | cf f z < 0 → z ∈ G.room (G.rm f)},
      inter_mem (inter_mem hW (hp f y hy)) (hm f y hy), fun z ⟨⟨hzW, hzp⟩, hzm⟩ => ?_⟩
    rcases lt_trichotomy (cf f z) 0 with h | h | h
    · exact Or.inr (hzm h)
    · exact Or.inl (Or.inl (hWz z hzW h))
    · exact Or.inl (Or.inr (hzp h))

end ForgeCurved.FacetSides
