import ForgeCurved.Boundary

/-!
# `xNonManifold_iff_top`: the core's refusal is the result's sheets and fans

`zeroThickness_iff_k` (CorrectK.lean) says the core refuses `ZeroThickness` iff `XNonManifoldK`,
a statement about covectors: three exact boundary facets above an edge covector, or two above a
vertex covector not linked by `Cov.le` through edge covectors. Design §1.3 (iii) shows why that
is not yet geometry: two cylinders tangent along a line have 4 boundary sheets at the line but
2 facets above it. This file states the geometry and proves the two agree on faithful cells.

**The topological statement** (`TopNonManifold`), about the set `R` (L1's regularised result) and
the carriers' strata only:

* the *sheet set* is `frontier R` at points on exactly one carrier; the *fan set* at `v` adds
  the edge-cell points in the closure of a sheet point's cell, and removes `v`;
* `ThreeSheets x`: arbitrarily close to `x` there are three points of the sheet set no two of
  which are joined by a connected set inside it near `x` (three local sheets);
* `TwoFans v`: arbitrarily close to `v` there are two sheet points not joined inside the fan
  set near `v` (two local fans round the vertex);
* `TopNonManifold`: three sheets at a point of an edge cell, or two fans at a vertex cell.

Nothing in it is a covector order or a count of covectors: connectedness, closure and the
frontier of `R` take their places.

**The theorem** (`xNonManifold_iff_top`), for continuous carriers into ℝ: on an exact curved
ledger (`ExactK`) whose listed cells are `Faithful` and satisfy the vertex wing condition
(`VWing`, R5 below), `XNonManifoldK ↔ TopNonManifold`. With `zeroThickness_iff_k`
(`zeroThickness_iff_top`): **the core refuses iff the result has three sheets at an edge or two
fans at a vertex**.

Which faithfulness each half uses:
* combinatorial ⇒ topological: R2 (a face is in the closure: the three facets, or the two, reach
  the point) and R3 (exact boundary facets lie in `frontier R`); distinct facets are never joined
  because facet cells are relatively open in the one-zero stratum, by continuity alone;
* topological ⇒ combinatorial: R4 (the wing condition: one sheet per facet near an edge), R5
  (the same near a vertex) and R2 (the chain of facets round a vertex is joined through its edge
  cells).

**R5, the vertex wing condition** (`VWing`), is R4 at vertices: a listed facet meets small
neighbourhoods of a listed vertex below it in one connected set. Curved.lean's `Faithful` states
R4 at edges only. Without R5 a facet reaching a vertex in two corners (a facet whose boundary
walk passes the vertex twice) would make one combinatorial fan two topological ones. For the
Tier-0 check (design §2.3) this is "no vertex occurs twice in one facet's boundary walk", the
vertex form of its wing condition.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Ledger Filter Topology

variable {P : Type} [TopologicalSpace P]

/-! ## The topological statement -/

/-- Joined in `S`: equal, or both in one connected subset of `S`. -/
def Joined (S : Set P) (y z : P) : Prop := y = z ∨ ∃ C ⊆ S, IsPreconnected C ∧ y ∈ C ∧ z ∈ C

/-- The result's boundary at points on exactly one carrier: its sheets. -/
def sheetSet (cs : List (Carrier P ℝ)) (R : Set P) : Set P :=
  frontier R ∩ {y | (covOfK cs y).isFacet = true}

/-- Round `v`: the sheets, and the edge-cell points in the closure of a sheet point's cell. -/
def fanSet (cs : List (Carrier P ℝ)) (R : Set P) (v : P) : Set P :=
  {y | y ∈ frontier R ∧ y ≠ v ∧ ((covOfK cs y).isFacet = true ∨
    (IsEdgeK cs (covOfK cs y) ∧ ∃ w ∈ sheetSet cs R, y ∈ closure (cell cs (covOfK cs w))))}

/-- Three local sheets of the boundary at `x`. -/
def ThreeSheets (cs : List (Carrier P ℝ)) (R : Set P) (x : P) : Prop :=
  ∃ W₀ ∈ 𝓝 x, ∀ U ∈ 𝓝 x, ∃ y₁ ∈ sheetSet cs R ∩ U, ∃ y₂ ∈ sheetSet cs R ∩ U,
    ∃ y₃ ∈ sheetSet cs R ∩ U, ¬Joined (sheetSet cs R ∩ W₀) y₁ y₂ ∧
      ¬Joined (sheetSet cs R ∩ W₀) y₁ y₃ ∧ ¬Joined (sheetSet cs R ∩ W₀) y₂ y₃

/-- Two local fans of the boundary round `v`. -/
def TwoFans (cs : List (Carrier P ℝ)) (R : Set P) (v : P) : Prop :=
  ∃ W₀ ∈ 𝓝 v, ∀ U ∈ 𝓝 v, ∃ y ∈ sheetSet cs R ∩ U, ∃ z ∈ sheetSet cs R ∩ U,
    ¬Joined (fanSet cs R v ∩ W₀) y z

/-- **The result is not a manifold-with-boundary in the sheet sense**: three local sheets at a
point of an edge cell, or two local fans at a vertex cell. -/
def TopNonManifold (cs : List (Carrier P ℝ)) (R : Set P) : Prop :=
  (∃ x, IsEdgeK cs (covOfK cs x) ∧ ThreeSheets cs R x) ∨
  (∃ v, IsVertK cs (covOfK cs v) ∧ TwoFans cs R v)

/-- **R5, the vertex wing condition**: a listed facet meets small neighbourhoods of a point of a
listed vertex below it in one connected set (R4, `Faithful.wing`, at vertices). -/
def VWing [ForgeExact.Topology P] (cs : List (Carrier P ℝ)) (Listed : Cov → Prop) : Prop :=
  ∀ v φ, Listed v → Listed φ → IsVertK cs v → φ.isFacet = true → Cov.le v φ = true →
    ∀ x, covCellK cs v x → ∀ W : Pred P, ForgeExact.Topology.IsOpen W → W x →
      ∃ U : Pred P, ForgeExact.Topology.IsOpen U ∧ U.Sub W ∧ U x ∧ Connected ((covCellK cs φ).inter U)

/-! ## Tools -/

/-- A function locally constant on `S` is constant on every connected subset of `S`. -/
theorem const_of_loc {α : Type} {S C : Set P} {f : P → α}
    (hloc : ∀ y ∈ S, ∃ N ∈ 𝓝 y, ∀ z ∈ N ∩ S, f z = f y) (hC : IsPreconnected C) (hCS : C ⊆ S)
    {y z : P} (hy : y ∈ C) (hz : z ∈ C) : f y = f z := by
  by_contra hne
  choose N hN hNf using hloc
  have key : ∀ w (hw : w ∈ S), w ∈ interior (N w hw) := fun w hw => mem_interior_iff_mem_nhds.mpr (hN w hw)
  let U : Set P := ⋃ (w : P) (h : w ∈ C) (_ : f w = f y), interior (N w (hCS h))
  let V : Set P := ⋃ (w : P) (h : w ∈ C) (_ : f w ≠ f y), interior (N w (hCS h))
  have hU : IsOpen U := isOpen_iUnion fun _ => isOpen_iUnion fun _ => isOpen_iUnion fun _ => isOpen_interior
  have hV : IsOpen V := isOpen_iUnion fun _ => isOpen_iUnion fun _ => isOpen_iUnion fun _ => isOpen_interior
  have hcov : C ⊆ U ∪ V := by
    intro w hw
    by_cases e : f w = f y
    · exact Or.inl (Set.mem_iUnion₂.mpr ⟨w, hw, Set.mem_iUnion.mpr ⟨e, key w (hCS hw)⟩⟩)
    · exact Or.inr (Set.mem_iUnion₂.mpr ⟨w, hw, Set.mem_iUnion.mpr ⟨e, key w (hCS hw)⟩⟩)
  obtain ⟨p, hpC, hpU, hpV⟩ := hC U V hU hV hcov
    ⟨y, hy, Set.mem_iUnion₂.mpr ⟨y, hy, Set.mem_iUnion.mpr ⟨rfl, key y (hCS hy)⟩⟩⟩
    ⟨z, hz, Set.mem_iUnion₂.mpr ⟨z, hz, Set.mem_iUnion.mpr ⟨Ne.symm hne, key z (hCS hz)⟩⟩⟩
  obtain ⟨w₁, hw₁, hpU⟩ := Set.mem_iUnion₂.mp hpU
  obtain ⟨e₁, hp₁⟩ := Set.mem_iUnion.mp hpU
  obtain ⟨w₂, hw₂, hpV⟩ := Set.mem_iUnion₂.mp hpV
  obtain ⟨e₂, hp₂⟩ := Set.mem_iUnion.mp hpV
  have a₁ := hNf w₁ (hCS hw₁) p ⟨interior_subset hp₁, hCS hpC⟩
  have a₂ := hNf w₂ (hCS hw₂) p ⟨interior_subset hp₂, hCS hpC⟩
  exact e₂ (a₂.symm.trans (a₁.trans e₁))

/-- Finitely many neighbourhoods, chosen at once, and their intersection. -/
theorem nhds_choice {ι : Type} (l : List ι) {x : P} (Q : ι → Set P → Prop)
    (hQ : ∀ i ∈ l, ∃ U ∈ 𝓝 x, Q i U) :
    ∃ g : ι → Set P, (∀ i ∈ l, Q i (g i) ∧ g i ∈ 𝓝 x) ∧ (⋂ i ∈ {i | i ∈ l}, g i) ∈ 𝓝 x := by
  classical
  let g : ι → Set P := fun i => if h : i ∈ l then Classical.choose (hQ i h) else Set.univ
  have hg : ∀ i ∈ l, Q i (g i) ∧ g i ∈ 𝓝 x := fun i hi => by
    simp only [g, hi, dite_true]
    exact ⟨(Classical.choose_spec (hQ i hi)).2, (Classical.choose_spec (hQ i hi)).1⟩
  exact ⟨g, hg, (Filter.biInter_mem (List.finite_toSet l)).mpr fun i hi => (hg i hi).2⟩

variable {L : Ledger} {cs : List (Carrier P ℝ)} {A : Nat → Pred P}

/-- R2, in Mathlib's words: a listed face is in the closure of a listed cell above it. -/
theorem sub_closure_of_le (hf : Faithful cs (Listed L)) {X Y : Cov} (hX : Listed L X)
    (hY : Listed L Y) (hrX : RealisedK cs X) (hrY : RealisedK cs Y) (hle : Cov.le X Y = true) :
    cell cs X ⊆ closure (cell cs Y) := fun x hx => by
  have := (hf.incidence X Y hX hY hrX hrY).mp hle x hx
  rwa [closure_iff, cell_eq] at this

theorem bdry_listed (h : ExactK L cs A) {rule : Rule} {φ : Cov} (hb : XBoundaryK L cs A rule φ) :
    φ ∈ L.facets := (mem_result.mp ((result_iff_k h rule φ).mpr hb)).1

theorem bdry_result (h : ExactK L cs A) {rule : Rule} {φ : Cov} (hb : XBoundaryK L cs A rule φ) :
    φ ∈ L.result (L.keepTab rule) := (result_iff_k h rule φ).mpr hb

theorem edge_listed (h : ExactK L cs A) {rule : Rule} {E φ : Cov} (hr : RealisedK cs E)
    (he : IsEdgeK cs E) (hb : XBoundaryK L cs A rule φ) (hle : Cov.le E φ = true) : E ∈ L.edges :=
  (h.edges E).mpr ⟨hr, he, φ, bdry_listed h hb, hle⟩

theorem vert_listed (h : ExactK L cs A) {rule : Rule} {V φ : Cov} (hr : RealisedK cs V)
    (hv : IsVertK cs V) (hb : XBoundaryK L cs A rule φ) (hle : Cov.le V φ = true) : V ∈ L.verts :=
  (h.verts V).mpr ⟨hr, hv, φ, bdry_listed h hb, hle⟩

theorem sheet_iff (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A) (hf : Faithful cs (Listed L))
    (rule : Rule) {y : P} :
    y ∈ sheetSet cs (Rset L A rule) ↔ XBoundaryK L cs A rule (covOfK cs y) := by
  constructor
  · rintro ⟨hy, hφ⟩; exact (frontier_iff hc h hf rule hφ).mp hy
  · intro hb; exact ⟨(frontier_iff hc h hf rule hb.2.1).mpr hb, hb.2.1⟩

theorem cell_sub_sheet (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) {rule : Rule} {φ : Cov} (hb : XBoundaryK L cs A rule φ) :
    cell cs φ ⊆ sheetSet cs (Rset L A rule) := fun y hy => by
  rw [mem_cell] at hy
  exact (sheet_iff hc h hf rule).mpr (hy ▸ hb)

/-- Distinct facet cells are never joined in the one-zero stratum. -/
theorem facet_const (hc : ∀ c ∈ cs, Continuous c) {C : Set P}
    (hCS : C ⊆ {y | (covOfK cs y).isFacet = true}) (hC : IsPreconnected C) {y z : P}
    (hy : y ∈ C) (hz : z ∈ C) : covOfK cs y = covOfK cs z := by
  refine const_of_loc (S := {y | (covOfK cs y).isFacet = true}) (fun y hy => ?_) hC hCS hy hz
  refine ⟨_, eventually_le hc y, fun z ⟨hzN, hz⟩ => ?_⟩
  exact (facet_eq_of_le hzN hy hz).symm

theorem joined_facet (hc : ∀ c ∈ cs, Continuous c) {S : Set P}
    (hS : S ⊆ {y | (covOfK cs y).isFacet = true}) {y z : P} (hj : Joined S y z) :
    covOfK cs y = covOfK cs z := by
  rcases hj with e | ⟨C, hCS, hC, hy, hz⟩
  · rw [e]
  · exact facet_const hc (hCS.trans hS) hC hy hz

theorem xfan_bdry {rule : Rule} {V a c : Cov} (hx : XFanK L cs A rule V a c)
    (ha : XBoundaryK L cs A rule a) : XBoundaryK L cs A rule c := by
  induction hx with
  | refl => exact ha
  | step _ hb _ _ _ => exact hb

theorem closure_inter_open' {S O : Set P} (hO : IsOpen O) {e : P} (heO : e ∈ O)
    (he : e ∈ closure S) : e ∈ closure (S ∩ O) := by
  rw [mem_closure_iff_nhds] at he ⊢
  intro t ht
  obtain ⟨p, ⟨hpt, hpO⟩, hpS⟩ := he (t ∩ O) (Filter.inter_mem ht (hO.mem_nhds heO))
  exact ⟨p, hpt, hpS, hpO⟩

/-! ## Edges: three facets, three sheets -/

theorem edge_comb_top (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) (rule : Rule) {E : Cov} (hr : RealisedK cs E)
    (hed : IsEdgeK cs E) {a b c : Cov} (hab : a ≠ b) (hac : a ≠ c) (hbc : b ≠ c)
    (ha : XBoundaryK L cs A rule a ∧ Cov.le E a = true) (hb : XBoundaryK L cs A rule b ∧ Cov.le E b = true)
    (hc' : XBoundaryK L cs A rule c ∧ Cov.le E c = true) :
    ∃ x, IsEdgeK cs (covOfK cs x) ∧ ThreeSheets cs (Rset L A rule) x := by
  obtain ⟨x, hx⟩ := hr
  have hx' : covOfK cs x = E := hx
  refine ⟨x, hx' ▸ hed, Set.univ, univ_mem, fun U hU => ?_⟩
  have pick : ∀ φ, XBoundaryK L cs A rule φ ∧ Cov.le E φ = true →
      ∃ y ∈ sheetSet cs (Rset L A rule) ∩ U, covOfK cs y = φ := by
    rintro φ ⟨hφ, hle⟩
    have hEl := edge_listed h ⟨x, hx⟩ hed hφ hle
    have hcl := sub_closure_of_le hf (Or.inr (Or.inr (Or.inl hEl))) (Or.inl (bdry_listed h hφ))
      ⟨x, hx⟩ hφ.1 hle hx'
    obtain ⟨y, hyU, hy⟩ := mem_closure_iff_nhds.mp hcl U hU
    exact ⟨y, ⟨cell_sub_sheet hc h hf hφ hy, hyU⟩, hy⟩
  obtain ⟨y₁, h₁, e₁⟩ := pick a ha
  obtain ⟨y₂, h₂, e₂⟩ := pick b hb
  obtain ⟨y₃, h₃, e₃⟩ := pick c hc'
  have sub : sheetSet cs (Rset L A rule) ∩ Set.univ ⊆ {y | (covOfK cs y).isFacet = true} :=
    fun y hy => hy.1.2
  refine ⟨y₁, h₁, y₂, h₂, y₃, h₃, fun hj => ?_, fun hj => ?_, fun hj => ?_⟩
  · exact hab (e₁ ▸ e₂ ▸ joined_facet hc sub hj)
  · exact hac (e₁ ▸ e₃ ▸ joined_facet hc sub hj)
  · exact hbc (e₂ ▸ e₃ ▸ joined_facet hc sub hj)

theorem edge_top_comb (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) (rule : Rule) {x : P} (hed : IsEdgeK cs (covOfK cs x))
    (h3 : ThreeSheets cs (Rset L A rule) x) :
    ∃ E, RealisedK cs E ∧ IsEdgeK cs E ∧ ∃ a b c, a ≠ b ∧ a ≠ c ∧ b ≠ c ∧
      (XBoundaryK L cs A rule a ∧ Cov.le E a = true) ∧ (XBoundaryK L cs A rule b ∧ Cov.le E b = true) ∧
      (XBoundaryK L cs A rule c ∧ Cov.le E c = true) := by
  obtain ⟨W₀, hW₀, H⟩ := h3
  obtain ⟨N₀, hN₀o, hxN₀, hN₀⟩ := exists_open_le hc x
  set E := covOfK cs x
  set W := N₀ ∩ interior W₀
  have hWo : IsOpen W := hN₀o.inter isOpen_interior
  have hxW : x ∈ W := ⟨hxN₀, mem_interior_iff_mem_nhds.mpr hW₀⟩
  let Q : Cov → Set P → Prop := fun φ V =>
    V ⊆ W ∧ (XBoundaryK L cs A rule φ ∧ Cov.le E φ = true → IsPreconnected (cell cs φ ∩ V))
  have hQ : ∀ φ ∈ L.result (L.keepTab rule), ∃ V ∈ 𝓝 x, Q φ V := by
    intro φ _
    by_cases hφ : XBoundaryK L cs A rule φ ∧ Cov.le E φ = true
    · have hEl := edge_listed h ⟨x, rfl⟩ hed hφ.1 hφ.2
      obtain ⟨V, hVo, hVW, hxV, hVc⟩ := hf.wing E φ (Or.inr (Or.inr (Or.inl hEl)))
        (Or.inl (bdry_listed h hφ.1)) hed hφ.1.2.1 hφ.2 x rfl (fun y => y ∈ W)
        (show IsOpen {y | y ∈ W} from hWo) hxW
      rw [connected_iff] at hVc
      exact ⟨{y | V y}, hVo.mem_nhds hxV, fun y hy => hVW y hy, fun _ => hVc⟩
    · exact ⟨W, hWo.mem_nhds hxW, subset_rfl, fun h' => absurd h' hφ⟩
  obtain ⟨g, hg, hU⟩ := nhds_choice _ Q hQ
  obtain ⟨y₁, ⟨s₁, u₁⟩, y₂, ⟨s₂, u₂⟩, y₃, ⟨s₃, u₃⟩, n₁₂, n₁₃, n₂₃⟩ :=
    H ((⋂ i ∈ {i | i ∈ L.result (L.keepTab rule)}, g i) ∩ W) (Filter.inter_mem hU (hWo.mem_nhds hxW))
  have info : ∀ {y}, y ∈ sheetSet cs (Rset L A rule) →
      y ∈ (⋂ i ∈ {i | i ∈ L.result (L.keepTab rule)}, g i) ∩ W →
      XBoundaryK L cs A rule (covOfK cs y) ∧ Cov.le E (covOfK cs y) = true ∧ y ∈ g (covOfK cs y) := by
    intro y hs hu
    have hb := (sheet_iff hc h hf rule).mp hs
    refine ⟨hb, hN₀ y hu.2.1, ?_⟩
    exact Set.mem_iInter₂.mp hu.1 _ (bdry_result h hb)
  have ne : ∀ {y z}, y ∈ sheetSet cs (Rset L A rule) → z ∈ sheetSet cs (Rset L A rule) →
      y ∈ (⋂ i ∈ {i | i ∈ L.result (L.keepTab rule)}, g i) ∩ W →
      z ∈ (⋂ i ∈ {i | i ∈ L.result (L.keepTab rule)}, g i) ∩ W →
      ¬Joined (sheetSet cs (Rset L A rule) ∩ W₀) y z → covOfK cs y ≠ covOfK cs z := by
    intro y z hy hz uy uz hn e
    obtain ⟨hb, hle, hyg⟩ := info hy uy
    obtain ⟨-, -, hzg⟩ := info hz uz
    have hQ' := (hg _ (bdry_result h hb)).1
    apply hn
    refine Or.inr ⟨cell cs (covOfK cs y) ∩ g (covOfK cs y), fun w hw => ⟨?_, ?_⟩, hQ'.2 ⟨hb, hle⟩,
      ⟨rfl, hyg⟩, ⟨e.symm, e ▸ hzg⟩⟩
    · exact cell_sub_sheet hc h hf hb hw.1
    · exact interior_subset (hQ'.1 hw.2).2
  obtain ⟨b₁, l₁, -⟩ := info s₁ u₁
  obtain ⟨b₂, l₂, -⟩ := info s₂ u₂
  obtain ⟨b₃, l₃, -⟩ := info s₃ u₃
  exact ⟨E, ⟨x, rfl⟩, hed, _, _, _, ne s₁ s₂ u₁ u₂ n₁₂, ne s₁ s₃ u₁ u₃ n₁₃, ne s₂ s₃ u₂ u₃ n₂₃,
    ⟨b₁, l₁⟩, ⟨b₂, l₂⟩, ⟨b₃, l₃⟩⟩

/-! ## Vertices: two fans -/

/-- Not a vertex: a cell with two zeros and more than one point is not a single point. -/
theorem edge_ne_vert {E V : Cov} (he : IsEdgeK cs E) (hv : IsVertK cs V) : E ≠ V := by
  rintro rfl; exact he.2.2 hv

theorem vert_comb_top (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) (rule : Rule) {V : Cov} (hr : RealisedK cs V) (hvx : IsVertK cs V)
    {a b : Cov} (ha : XBoundaryK L cs A rule a ∧ Cov.le V a = true)
    (hb : XBoundaryK L cs A rule b ∧ Cov.le V b = true) (hn : ¬XFanK L cs A rule V a b) :
    ∃ v, IsVertK cs (covOfK cs v) ∧ TwoFans cs (Rset L A rule) v := by
  obtain ⟨v, hv⟩ := hr
  have hv' : covOfK cs v = V := hv
  obtain ⟨N₀, hN₀o, hvN₀, hN₀⟩ := exists_open_le hc v
  refine ⟨v, hv' ▸ hvx, N₀, hN₀o.mem_nhds hvN₀, fun U hU => ?_⟩
  have pick : ∀ φ, XBoundaryK L cs A rule φ ∧ Cov.le V φ = true →
      ∃ y ∈ sheetSet cs (Rset L A rule) ∩ U, covOfK cs y = φ := by
    rintro φ ⟨hφ, hle⟩
    have hVl := vert_listed h ⟨v, hv⟩ hvx hφ hle
    have hcl := sub_closure_of_le hf (Or.inr (Or.inr (Or.inr hVl))) (Or.inl (bdry_listed h hφ))
      ⟨v, hv⟩ hφ.1 hle hv'
    obtain ⟨y, hyU, hy⟩ := mem_closure_iff_nhds.mp hcl U hU
    exact ⟨y, ⟨cell_sub_sheet hc h hf hφ hy, hyU⟩, hy⟩
  obtain ⟨y, hy, ey⟩ := pick a ha
  obtain ⟨z, hz, ez⟩ := pick b hb
  refine ⟨y, hy, z, hz, ?_⟩
  rintro (e | ⟨C, hCS, hC, hyC, hzC⟩)
  · exact hn (ey ▸ ez ▸ e ▸ XFanK.refl)
  -- membership in `a`'s fan, read off a point's cell, is locally constant on the fan set
  let f : P → Prop := fun u => ∃ c, XFanK L cs A rule V a c ∧ Cov.le (covOfK cs u) c = true
  have hloc : ∀ u ∈ fanSet cs (Rset L A rule) v ∩ N₀, ∃ N ∈ 𝓝 u,
      ∀ w ∈ N ∩ (fanSet cs (Rset L A rule) v ∩ N₀), f w = f u := by
    intro u ⟨hu, huN⟩
    refine ⟨_, eventually_le hc u, fun w ⟨hwN, hw, hwN₀⟩ => propext ⟨fun ⟨c, hac, hwc⟩ =>
      ⟨c, hac, le_trans hwN hwc⟩, fun ⟨c, hac, huc⟩ => ?_⟩⟩
    have hcb := xfan_bdry hac ha.1
    have hVu : Cov.le V (covOfK cs u) = true := hv' ▸ hN₀ u huN
    rcases hu.2.2 with hφu | ⟨hed, -⟩
    · -- `u` on a facet: `w` is on the same facet
      have e1 : covOfK cs u = c := facet_eq_of_le huc hφu hcb.2.1
      have e2 : covOfK cs u = covOfK cs w := eq_of_le_of_count hwN (by
        rw [(isFacet_iff _).mp hφu]
        exact count_pos_of_mem (zero_of_frontier hc h rule hw.1))
      exact ⟨c, hac, e2 ▸ e1 ▸ le_refl _⟩
    · -- `u` on an edge: every boundary facet at `w` is in `a`'s fan, through `u`'s edge
      rcases hw.2.2 with hφw | ⟨-, w', hw', hwcl⟩
      · have hbw := (sheet_iff hc h hf rule).mp ⟨hw.1, hφw⟩
        exact ⟨_, XFanK.step hac hbw (le_trans hVu hwN)
          ⟨covOfK cs u, ⟨u, rfl⟩, hed, hVu, huc, hwN⟩, le_refl _⟩
      · have hbd := (sheet_iff hc h hf rule).mp hw'
        have hwd := le_of_mem_closure hc hwcl
        exact ⟨_, XFanK.step hac hbd (le_trans hVu (le_trans hwN hwd))
          ⟨covOfK cs u, ⟨u, rfl⟩, hed, hVu, huc, le_trans hwN hwd⟩, hwd⟩
  have hfy : f y := ⟨a, XFanK.refl, ey ▸ le_refl _⟩
  have := const_of_loc hloc hC hCS hyC hzC
  obtain ⟨c, hac, hzc⟩ : f z := this ▸ hfy
  have e : b = c := facet_eq_of_le (ez ▸ hzc) hb.1.2.1 (xfan_bdry hac ha.1).2.1
  exact hn (e ▸ hac)

theorem vert_top_comb (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) (hvw : VWing cs (Listed L)) (rule : Rule) {v : P}
    (hvx : IsVertK cs (covOfK cs v)) (h2 : TwoFans cs (Rset L A rule) v) :
    ∃ V, RealisedK cs V ∧ IsVertK cs V ∧ ∃ a b,
      (XBoundaryK L cs A rule a ∧ Cov.le V a = true) ∧ (XBoundaryK L cs A rule b ∧ Cov.le V b = true) ∧
      ¬XFanK L cs A rule V a b := by
  obtain ⟨W₀, hW₀, H⟩ := h2
  obtain ⟨N₀, hN₀o, hvN₀, hN₀⟩ := exists_open_le hc v
  set V := covOfK cs v
  set W := N₀ ∩ interior W₀
  have hWo : IsOpen W := hN₀o.inter isOpen_interior
  have hvW : v ∈ W := ⟨hvN₀, mem_interior_iff_mem_nhds.mpr hW₀⟩
  let Q : Cov → Set P → Prop := fun φ U =>
    U ⊆ W ∧ (XBoundaryK L cs A rule φ ∧ Cov.le V φ = true → IsPreconnected (cell cs φ ∩ U))
  have hQ : ∀ φ ∈ L.result (L.keepTab rule), ∃ U ∈ 𝓝 v, Q φ U := by
    intro φ _
    by_cases hφ : XBoundaryK L cs A rule φ ∧ Cov.le V φ = true
    · have hVl := vert_listed h ⟨v, rfl⟩ hvx hφ.1 hφ.2
      obtain ⟨U, hUo, hUW, hvU, hUc⟩ := hvw V φ (Or.inr (Or.inr (Or.inr hVl)))
        (Or.inl (bdry_listed h hφ.1)) hvx hφ.1.2.1 hφ.2 v rfl (fun y => y ∈ W)
        (show IsOpen {y | y ∈ W} from hWo) hvW
      rw [connected_iff] at hUc
      exact ⟨{y | U y}, hUo.mem_nhds hvU, fun y hy => hUW y hy, fun _ => hUc⟩
    · exact ⟨W, hWo.mem_nhds hvW, subset_rfl, fun h' => absurd h' hφ⟩
  obtain ⟨g, hg, hU₀⟩ := nhds_choice _ Q hQ
  set U₀ := ⋂ i ∈ {i | i ∈ L.result (L.keepTab rule)}, g i
  obtain ⟨y, ⟨sy, uy⟩, z, ⟨sz, uz⟩, hn⟩ := H (U₀ ∩ W) (Filter.inter_mem hU₀ (hWo.mem_nhds hvW))
  have hay := (sheet_iff hc h hf rule).mp sy
  have hbz := (sheet_iff hc h hf rule).mp sz
  have lay : Cov.le V (covOfK cs y) = true := hN₀ y uy.2.1
  have lbz : Cov.le V (covOfK cs z) = true := hN₀ z uz.2.1
  refine ⟨V, ⟨v, rfl⟩, hvx, _, _, ⟨hay, lay⟩, ⟨hbz, lbz⟩, fun hx => ?_⟩
  -- `V` is no facet: else both facets are `V`, so `y = z = v`
  have hVnf : V.isFacet ≠ true := by
    intro hVf
    have e1 := facet_eq_of_le lay hVf hay.2.1
    have e2 := facet_eq_of_le lbz hVf hbz.2.1
    exact hn (Or.inl (hvx.2 y z (show covOfK cs y = V from e1.symm) (show covOfK cs z = V from e2.symm)))
  have gU : ∀ φ, XBoundaryK L cs A rule φ → U₀ ⊆ g φ := fun φ hb w hw =>
    Set.mem_iInter₂.mp hw _ (bdry_result h hb)
  have gW : ∀ φ, XBoundaryK L cs A rule φ → g φ ⊆ W := fun φ hb => (hg φ (bdry_result h hb)).1.1
  have piece : ∀ φ, XBoundaryK L cs A rule φ → Cov.le V φ = true →
      cell cs φ ∩ g φ ⊆ fanSet cs (Rset L A rule) v ∩ W₀ := by
    intro φ hb hle w ⟨hw, hwg⟩
    have hfr := cell_sub_sheet hc h hf hb hw
    refine ⟨⟨hfr.1, fun e => ?_, Or.inl hfr.2⟩, interior_subset (gW φ hb hwg).2⟩
    rw [mem_cell] at hw
    have hf' := hb.2.1
    rw [← hw, e] at hf'
    exact hVnf hf'
  -- the claim, along the fan
  have claim : ∀ c, XFanK L cs A rule V (covOfK cs y) c →
      ∃ C ⊆ fanSet cs (Rset L A rule) v ∩ W₀, IsPreconnected C ∧ y ∈ C ∧ cell cs c ∩ g c ⊆ C := by
    intro c hxc
    induction hxc with
    | refl =>
      exact ⟨_, piece _ hay lay, (hg _ (bdry_result h hay)).1.2 ⟨hay, lay⟩,
        ⟨rfl, gU _ hay uy.1⟩, subset_rfl⟩
    | @step b c hab hbc hVc hE ih =>
      obtain ⟨C, hCS, hC, hyC, hbC⟩ := ih
      obtain ⟨E, hEr, hEd, hVE, hEb, hEc⟩ := hE
      have hbb : XBoundaryK L cs A rule b := xfan_bdry hab hay
      have hEl := edge_listed h hEr hEd hbc hEc
      have hVl := vert_listed h ⟨v, rfl⟩ hvx hbc hVc
      have hvE := sub_closure_of_le hf (Or.inr (Or.inr (Or.inr hVl))) (Or.inr (Or.inr (Or.inl hEl)))
        ⟨v, rfl⟩ hEr hVE (show v ∈ cell cs V from rfl)
      obtain ⟨e, heU, heE⟩ := mem_closure_iff_nhds.mp hvE _ (interior_mem_nhds.mpr hU₀)
      have heb := sub_closure_of_le hf (Or.inr (Or.inr (Or.inl hEl))) (Or.inl (bdry_listed h hbb))
        hEr hbb.1 hEb heE
      have hec := sub_closure_of_le hf (Or.inr (Or.inr (Or.inl hEl))) (Or.inl (bdry_listed h hbc))
        hEr hbc.1 hEc heE
      have hgb : interior U₀ ⊆ g b := interior_subset.trans (gU b hbb)
      have hgc : interior U₀ ⊆ g c := interior_subset.trans (gU c hbc)
      have heb' : e ∈ closure (cell cs b ∩ g b) :=
        closure_mono (Set.inter_subset_inter_right _ hgb) (closure_inter_open' isOpen_interior heU heb)
      have hec' : e ∈ closure (cell cs c ∩ g c) :=
        closure_mono (Set.inter_subset_inter_right _ hgc) (closure_inter_open' isOpen_interior heU hec)
      have h1 : IsPreconnected (insert e C) := hC.subset_closure (Set.subset_insert e C)
        (Set.insert_subset_iff.mpr ⟨closure_mono hbC heb', subset_closure⟩)
      have hcc := (hg c (bdry_result h hbc)).1.2 ⟨hbc, hVc⟩
      have h2 : IsPreconnected (insert e (cell cs c ∩ g c)) := hcc.subset_closure
        (Set.subset_insert e _) (Set.insert_subset_iff.mpr ⟨hec', subset_closure⟩)
      obtain ⟨w, hw⟩ := hbb.1
      have hwS := cell_sub_sheet hc h hf hbb (show w ∈ cell cs b from hw)
      have heS : e ∈ fanSet cs (Rset L A rule) v ∩ W₀ := by
        refine ⟨⟨closure_minimal (fun p hp => (cell_sub_sheet hc h hf hbb hp).1) isClosed_frontier heb,
          fun ev => edge_ne_vert hEd hvx (by rw [← heE, ev]), Or.inr ⟨heE ▸ hEd, w, hwS, ?_⟩⟩, ?_⟩
        · rw [show covOfK cs w = b from hw]; exact heb
        · exact interior_subset (gW b hbb (hgb heU)).2
      refine ⟨insert e C ∪ insert e (cell cs c ∩ g c), ?_, h1.union e (Set.mem_insert _ _)
        (Set.mem_insert _ _) h2, Or.inl (Set.mem_insert_of_mem _ hyC),
        fun p hp => Or.inr (Set.mem_insert_of_mem _ hp)⟩
      rintro p ((rfl | hp) | (rfl | hp))
      · exact heS
      · exact hCS hp
      · exact heS
      · exact piece c hbc hVc hp
  obtain ⟨C, hCS, hC, hyC, hbC⟩ := claim _ hx
  exact hn (Or.inr ⟨C, hCS, hC, hyC, hbC ⟨rfl, gU _ hbz uz.1⟩⟩)

/-! ## The theorem -/

/-- **`xNonManifold_iff_top`.** Over continuous carriers into ℝ, on an exact curved ledger whose
listed cells are faithful (R1–R4) and satisfy the vertex wing condition (R5), the covector
non-manifoldness the core refuses is the topological one of the regularised result. -/
theorem xNonManifold_iff_top (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) (hvw : VWing cs (Listed L)) (rule : Rule) :
    XNonManifoldK L cs A rule ↔ TopNonManifold cs (Rset L A rule) := by
  constructor
  · rintro (⟨E, hr, hed, a, b, c, hab, hac, hbc, ha, hb, hc'⟩ | ⟨V, hr, hvx, a, b, ha, hb, hn⟩)
    · exact Or.inl (edge_comb_top hc h hf rule hr hed hab hac hbc ha hb hc')
    · exact Or.inr (vert_comb_top hc h hf rule hr hvx ha hb hn)
  · rintro (⟨x, hed, h3⟩ | ⟨v, hvx, h2⟩)
    · exact Or.inl (edge_top_comb hc h hf rule hed h3)
    · exact Or.inr (vert_top_comb hc h hf hvw rule hvx h2)

/-- **The refusal, topologically**: the core refuses `ZeroThickness` iff the regularised result
has three local sheets at a point of an edge cell, or two local fans at a vertex cell. -/
theorem zeroThickness_iff_top (hc : ∀ c ∈ cs, Continuous c) (h : ExactK L cs A)
    (hf : Faithful cs (Listed L)) (hvw : VWing cs (Listed L)) (rule : Rule) :
    (∃ x, core L rule = .refused (.zeroThickness x)) ↔ TopNonManifold cs (Rset L A rule) :=
  (zeroThickness_iff_k h rule).trans (xNonManifold_iff_top hc h hf hvw rule)

end ForgeCurved
