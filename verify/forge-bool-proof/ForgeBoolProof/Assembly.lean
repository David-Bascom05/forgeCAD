import ForgeBoolProof.Compose

/-!
# D8.math: the assembly boundary lemma (ledger row D8.math)

D8 owes `BoundsFaces` of the result `R` against `S`, the closure of the kept rooms. Once `apply`'s
output is read as faces, what it writes is the kept regions: the faces' points are the closure of
the kept facets (`WritesKept`, the code's half of D8). The mathematics' half is here:

* **`faces_sub_frontier`**: the kept facets lie on `frontier S` (`KeptIsBd`: a kept region's facet
  is a boundary facet; F5: a boundary facet is on the frontier), and the frontier is closed, so
  the result's faces lie on it: `BoundsFaces.faces_sub`, zones or not.
* **`frontier_on`**: a frontier point of `S` on the arrangement's faces is on the result's faces.
  On a facet it is on a boundary facet (F5), whose region is kept (`KeptIsBd`). Off the facets (on
  the arrangement's edges and vertices) it is a limit of such points when the edges and vertices
  do not separate space locally (`Thin`): a small connected neighbourhood minus them meets both
  the kept rooms and the outside of `S`, so it meets the frontier off the edges, on a facet.

`Thin` is a hypothesis on the arrangement (ledger row D5.thin, open): the faces' points off the
facets (curves and points, in 3D) have empty interior and do not cut small neighbourhoods. The
other fields of `BoundsFaces` (`local_formula`, `oriented`, `seeded`) stay with D8 (rows D8.orient,
D8.formula, D8.seeded, open).
-/

set_option linter.unusedSectionVars false

namespace ForgeBoolProof

open Set Filter Topology ForgeCurved.BodyModel ForgeCurved.FaceLocal ForgeCurved.FacetSides

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]

/-- The kept facets: the facets of the regions whose kept flag is set. -/
def KeptFacets (a : Arr E) (k : Nat → Bool) : Set E :=
  ⋃ f ∈ a.regions, ⋃ (_ : k f = true), a.G.facet (a.gf f)

/-- **What `apply` writes, read as faces** (the code's half of D8): the result's face points are
the closure of the kept facets. -/
structure WritesKept (a : Arr E) (k : Nat → Bool) (Rs : BodyModel E) : Prop where
  faces_eq : Faces Rs = closure (KeptFacets a k)

/-- The arrangement's face points off the facets: its edges and vertices. -/
def OffFacets (a : Arr E) : Set E := a.FacesG \ FacetPts a.G

/-- **The edges and vertices are thin** (row D5.thin, a hypothesis on the arrangement): they have
empty interior, and every face point has arbitrarily small open neighbourhoods that stay
connected once they are removed. -/
structure Thin (a : Arr E) : Prop where
  no_interior : interior (OffFacets a) = ∅
  uncut : ∀ x ∈ a.FacesG, ∀ U ∈ 𝓝 x, ∃ V ⊆ U, IsOpen V ∧ x ∈ V ∧ IsPreconnected (V \ OffFacets a)

/-- An open nonempty set is not inside a set with empty interior. -/
theorem diff_nonempty_of_open {O N : Set E} (hO : IsOpen O) (hne : O.Nonempty)
    (hN : interior N = ∅) : (O \ N).Nonempty := by
  by_contra h
  rw [not_nonempty_iff_eq_empty, diff_eq_empty] at h
  obtain ⟨x, hx⟩ := hne
  have : x ∈ interior N := interior_maximal h hO hx
  rw [hN] at this; exact this

section

variable {Aex Bex : BodyModel E} {a : Arr E} (ha : ArrOK Aex Bex a) {k keepR : Nat → Bool}
  (hK : KeptIsBd a k keepR)
include ha hK

/-- A kept facet is on the frontier of the kept rooms' closure. -/
theorem keptFacets_sub_frontier : KeptFacets a k ⊆ frontier (Res a.G keepR) := by
  intro y hy
  simp only [KeptFacets, mem_iUnion] at hy
  obtain ⟨f, hf, hk, hy⟩ := hy
  have hbd : Bd a.G keepR (a.gf f) := (hK.1 (a.gf f) ⟨y, hy⟩).mpr ⟨f, hf, rfl, hk⟩
  exact frontier_of_bd (sideLocal_of_data ha.data keepR) hbd hy

/-- The result's faces lie on the frontier (`BoundsFaces.faces_sub`, zones included). -/
theorem faces_sub_frontier {Rs : BodyModel E} (hw : WritesKept a k Rs) :
    Faces Rs ⊆ frontier (Res a.G keepR) := by
  rw [hw.faces_eq]
  exact closure_minimal (keptFacets_sub_frontier ha hK) isClosed_frontier

/-- A frontier point on a facet is on a kept facet. -/
theorem sheet_sub_keptFacets : frontier (Res a.G keepR) ∩ FacetPts a.G ⊆ KeptFacets a k := by
  rintro y ⟨hfr, hy⟩
  simp only [FacetPts, mem_iUnion] at hy
  obtain ⟨g, hy⟩ := hy
  have hbd := bd_of_frontier (sideLocal_of_data ha.data keepR) hy hfr
  obtain ⟨f, hf, hg, hk⟩ := (hK.1 g ⟨y, hy⟩).mp hbd
  simp only [KeptFacets, mem_iUnion]
  exact ⟨f, hf, hk, hg ▸ hy⟩

end

/-- **Off the facets, the frontier is a limit of frontier points on facets**, when the edges and
vertices are thin. -/
theorem frontier_sub_closure_sheet {Aex Bex : BodyModel E} {a : Arr E} (ha : ArrOK Aex Bex a)
    (ht : Thin a) (keepR : Nat → Bool) :
    frontier (Res a.G keepR) ⊆ closure (frontier (Res a.G keepR) ∩ FacetPts a.G) := by
  set S := Res a.G keepR
  have hKo : IsOpen (Kept a.G keepR) :=
    isOpen_iUnion fun r => isOpen_iUnion fun _ => ha.data.room_open r
  intro x hx
  have hxF : x ∈ a.FacesG := ForgeBoolProof.frontier_res_sub ha.data keepR hx
  rw [mem_closure_iff_nhds]
  intro U hU
  obtain ⟨V, hVU, hVo, hxV, hVc⟩ := ht.uncut x hxF U hU
  -- a kept point in `V` off the edges
  have h1 : (V ∩ Kept a.G keepR).Nonempty :=
    mem_closure_iff.mp (isClosed_closure.frontier_subset hx) V hVo hxV
  obtain ⟨p, ⟨hpV, hpK⟩, hpN⟩ :=
    diff_nonempty_of_open (hVo.inter hKo) h1 ht.no_interior
  -- a point of `V` outside `S`, off the edges
  have h2 : (V ∩ Sᶜ).Nonempty := by
    by_contra h
    rw [not_nonempty_iff_eq_empty, ← disjoint_iff_inter_eq_empty, disjoint_compl_right_iff_subset] at h
    exact hx.2 (interior_maximal h hVo hxV)
  obtain ⟨q, ⟨hqV, hqS⟩, hqN⟩ :=
    diff_nonempty_of_open (hVo.inter isClosed_closure.isOpen_compl) h2 ht.no_interior
  have hpI : p ∈ interior S := interior_maximal subset_closure hKo hpK
  obtain ⟨z, ⟨hzV, hzN⟩, hzfr⟩ : ((V \ OffFacets a) ∩ frontier S).Nonempty := by
    by_contra h
    rw [not_nonempty_iff_eq_empty] at h
    have hcov : V \ OffFacets a ⊆ interior S ∪ Sᶜ := by
      intro w hw
      by_cases hwI : w ∈ interior S
      · exact Or.inl hwI
      · right; intro hwS
        have : w ∈ frontier S := ⟨subset_closure hwS, hwI⟩
        exact (eq_empty_iff_forall_notMem.mp h) w ⟨hw, this⟩
    obtain ⟨w, -, hwI, hwS⟩ := hVc _ _ isOpen_interior isClosed_closure.isOpen_compl hcov
      ⟨p, ⟨hpV, hpN⟩, hpI⟩ ⟨q, ⟨hqV, hqN⟩, hqS⟩
    exact hwS (interior_subset hwI)
  have hzF : z ∈ a.FacesG := ForgeBoolProof.frontier_res_sub ha.data keepR hzfr
  have hzP : z ∈ FacetPts a.G := by
    by_contra h; exact hzN ⟨hzF, h⟩
  exact ⟨z, hVU hzV, hzfr, hzP⟩

/-- **D8.math, the boundary half**: from the keep rule's `KeptIsBd`, `apply` writing the kept
regions (`WritesKept`) and thin edges (`Thin`), the result's faces satisfy `BoundsFaces`'s
`frontier_on` and `faces_sub` against the closure of the kept rooms. -/
theorem boundsFaces_boundary {Aex Bex Rs : BodyModel E} {a : Arr E} (ha : ArrOK Aex Bex a)
    (ht : Thin a) {k keepR : Nat → Bool} (hK : KeptIsBd a k keepR) (hw : WritesKept a k Rs) :
    a.FacesG ∩ frontier (Res a.G keepR) ⊆ Faces Rs ∧
      Faces Rs ⊆ frontier (Res a.G keepR) := by
  refine ⟨fun x hx => ?_, faces_sub_frontier ha hK hw⟩
  rw [hw.faces_eq]
  exact closure_mono (sheet_sub_keptFacets ha hK) (frontier_sub_closure_sheet ha ht keepR hx.2)

/-- **D8.math, assembled**: with the other three fields (`local_formula`, `oriented`, `seeded`:
rows D8.formula, D8.orient, D8.seeded), `BoundsFaces` holds. -/
theorem boundsFaces_of_writes {Aex Bex Rs : BodyModel E} {a : Arr E} (ha : ArrOK Aex Bex a)
    (ht : Thin a) {k keepR : Nat → Bool} (hK : KeptIsBd a k keepR) (hw : WritesKept a k Rs)
    (hlf : ∀ u Φ z, Rs.formulas[u]? = some Φ → Rs.zones[u]? = some z → ∀ x ∈ z,
      (∀ f ∈ Rs.faces, IncV Rs u f → f.carrier x ≠ 0) → (x ∈ Res a.G keepR ↔ Φ.den (carriers Rs) x))
    (hor : ∀ f ∈ Rs.faces, ∀ y, Alone Rs f y → ForgeCurved.F6.Oriented f.carrier (Res a.G keepR) f.sense y)
    (hseed : Seeded Rs) :
    BoundsFaces Rs a.FacesG (Res a.G keepR) :=
  have h := boundsFaces_boundary ha ht hK hw
  ⟨h.1, h.2, hlf, hor, hseed⟩

end ForgeBoolProof
