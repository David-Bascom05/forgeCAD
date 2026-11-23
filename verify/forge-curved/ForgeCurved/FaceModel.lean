import ForgeCurved.FaceLocal
import ForgeCurved.Local

/-!
# F1–F4 from a local carrier model (route B's `EdgeLocal` and `VertLocal`)

The face ledger's facets and edges are pieces of the near faces' carriers. Near a point `x` of a
listed edge (or at a listed vert `v`), the builder knows which local cells of the carriers `cs`
they are:
* the **wings** (or **sectors**) are the realised facet covectors `Φ` above `x`'s, each lying, near
  `x`, in the row's facet for it;
* at a vert, the **bounding edges** are realised edge covectors `E` above `v`'s, each lying in its
  ledger edge, and a sector is bounded by exactly the edges whose covector is below its own.

With the local picture of Local.lean at the point (`LocalGood`: transversal points by
`localGood'_of_transversal`, cone apexes by `localGood_of_homogeneous`), that is F1–F4:
* **`edgeLocal_of_model`**: an `EdgeModel` gives `EdgeLocal` (the wing label is the index of the
  point's covector in `Φ`);
* **`vertLocal_of_model`**: a `VertModel` gives `VertLocal`.

**`obligations_of_models`** packages them into route B's `Obligations` (with F5 from the
rooms), and **`xNonManifoldF_iff_top_of_models`** is route B's theorem from local models.

Nothing global is used: no R1, no Jordan. The obligations left to the builder are the model's
fields, all about the finitely many covectors round the point: the realised facet and edge
covectors above it are listed and are where the row says.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.FaceLocal

open ForgeCurved ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Filter Topology

variable {P : Type} [TopologicalSpace P]

theorem mem_closure_inter_open {s W : Set P} {x : P} (hx : x ∈ closure s) (hW : IsOpen W)
    (hxW : x ∈ W) : x ∈ closure (s ∩ W) := by
  rw [mem_closure_iff] at hx ⊢
  intro o ho hxo
  obtain ⟨y, ⟨hyo, hyW⟩, hys⟩ := hx (o ∩ W) (ho.inter hW) ⟨hxo, hxW⟩
  exact ⟨y, hyo, hys, hyW⟩

/-- Two covectors near each other with the same number of zeros, the later above the earlier,
are equal. -/
theorem eq_of_le_count_eq {X Y : Cov} (h : Cov.le X Y = true)
    (hc : X.count .zero = Y.count .zero) : X = Y := eq_of_le_of_count h hc.le

/-- **The local model at an edge point**: the wings are the facet covectors `Φ`. -/
structure EdgeModel (G : FaceGeo P) (cs : List (Carrier P ℝ)) (wf : List Nat) (x : P) where
  W : Set P
  hWo : IsOpen W
  hxW : x ∈ W
  Φ : List Cov
  len : Φ.length = wf.length
  nodup : Φ.Nodup
  facet : ∀ φ ∈ Φ, φ.count .zero = 1
  above : ∀ φ ∈ Φ, Cov.le (covOfK cs x) φ = true
  real : ∀ φ ∈ Φ, RealisedK cs φ
  /-- the wing's cell lies, near `x`, in the row's facet -/
  sub : ∀ i (h : i < Φ.length), cell cs Φ[i] ∩ W ⊆ G.facet (wf.getD i 0)
  /-- every facet point near `x` is on a listed wing -/
  complete : ∀ y ∈ W, y ∈ FacetPts G → covOfK cs y ∈ Φ

variable {G : FaceGeo P} {cs : List (Carrier P ℝ)}

theorem getElem_idxOf_covOfK {Φ : List Cov} {y : P} (h : covOfK cs y ∈ Φ) :
    Φ[Φ.idxOf (covOfK cs y)]'(List.idxOf_lt_length_iff.mpr h) = covOfK cs y :=
  List.getElem_idxOf _

/-- The wing pieces are the wings' cells near the point. -/
theorem piece_eq {wf : List Nat} {x : P} (m : EdgeModel G cs wf x) (i : Nat) (hi : i < m.Φ.length) :
    {y | y ∈ m.W ∧ y ∈ G.facet (wf.getD i 0) ∧ m.Φ.idxOf (covOfK cs y) = i} =
      cell cs m.Φ[i] ∩ m.W := by
  ext y
  constructor
  · rintro ⟨hyW, hyf, hyi⟩
    have hm := m.complete y hyW (Set.mem_iUnion.mpr ⟨_, hyf⟩)
    refine ⟨?_, hyW⟩
    rw [mem_cell]
    have := getElem_idxOf_covOfK hm
    simp only [hyi] at this
    exact this.symm
  · rintro ⟨hy, hyW⟩
    rw [mem_cell] at hy
    refine ⟨hyW, m.sub i hi ⟨by rw [mem_cell]; exact hy, hyW⟩, ?_⟩
    rw [hy]
    exact m.nodup.idxOf_getElem i hi

/-- **F1–F4 at an edge point, from the local model and the local picture.** -/
theorem edgeLocal_of_model (hc : ∀ c ∈ cs, Continuous c) {wf : List Nat} {x : P}
    (m : EdgeModel G cs wf x) (hl : LocalGood cs x) : Nonempty (EdgeLocal G wf x) := by
  refine ⟨{
    W₀ := m.W
    hW₀ := m.hWo.mem_nhds m.hxW
    lab := fun y => m.Φ.idxOf (covOfK cs y)
    cov := ?_
    loc := ?_
    near := ?_
    conn := ?_ }⟩
  · intro y hyW hyF
    have hm := m.complete y hyW hyF
    have hlt := List.idxOf_lt_length_iff.mpr hm
    refine ⟨m.len ▸ hlt, m.sub _ hlt ⟨?_, hyW⟩⟩
    rw [mem_cell]; exact (getElem_idxOf_covOfK hm).symm
  · rintro y ⟨hyW, hyF⟩
    refine ⟨_, eventually_le hc y, fun z ⟨hz, hzW, hzF⟩ => ?_⟩
    have e : covOfK cs y = covOfK cs z := eq_of_le_count_eq hz (by
      rw [m.facet _ (m.complete y hyW hyF), m.facet _ (m.complete z hzW hzF)])
    simp only [e]
  · intro i hi
    have hi' : i < m.Φ.length := m.len ▸ hi
    show x ∈ closure {y | y ∈ m.W ∧ y ∈ G.facet (wf.getD i 0) ∧ m.Φ.idxOf (covOfK cs y) = i}
    rw [piece_eq m i hi']
    exact mem_closure_inter_open (hl.near _ (m.above _ (List.getElem_mem hi'))
      (m.real _ (List.getElem_mem hi'))) m.hWo m.hxW
  · intro i hi U hU
    have hi' : i < m.Φ.length := m.len ▸ hi
    obtain ⟨O, hOU, hOo, hxO⟩ := mem_nhds_iff.mp (Filter.inter_mem hU (m.hWo.mem_nhds m.hxW))
    obtain ⟨U'', hU''o, hU''O, hxU'', hcon⟩ := hl.conn O hOo hxO
    refine ⟨U'', hU''o.mem_nhds hxU'', fun y hy => (hOU (hU''O hy)).1, ?_⟩
    show IsPreconnected
      ({y | y ∈ m.W ∧ y ∈ G.facet (wf.getD i 0) ∧ m.Φ.idxOf (covOfK cs y) = i} ∩ U'')
    rw [piece_eq m i hi']
    have : cell cs m.Φ[i] ∩ m.W ∩ U'' = cell cs m.Φ[i] ∩ U'' := by
      ext y; constructor
      · rintro ⟨⟨h1, _⟩, h3⟩; exact ⟨h1, h3⟩
      · rintro ⟨h1, h3⟩; exact ⟨⟨h1, (hOU (hU''O h3)).2⟩, h3⟩
    rw [this]
    exact hcon _ (m.above _ (List.getElem_mem hi'))

/-- **The local model at a vert**: the sectors are the facet covectors `Φ`, the bounding edges
the edge covectors `E` (edge `ea j` for `E[j]`), and a sector is bounded by the edges below it. -/
structure VertModel (G : FaceGeo P) (cs : List (Carrier P ℝ)) (ss : List Sec) (v : P) where
  W : Set P
  hWo : IsOpen W
  hvW : v ∈ W
  v_facet : v ∉ FacetPts G
  v_edge : v ∉ EdgePts G
  Φ : List Cov
  len : Φ.length = ss.length
  nodup : Φ.Nodup
  facet : ∀ φ ∈ Φ, φ.count .zero = 1
  above : ∀ φ ∈ Φ, Cov.le (covOfK cs v) φ = true
  real : ∀ φ ∈ Φ, RealisedK cs φ
  sub : ∀ i (h : i < Φ.length), cell cs Φ[i] ∩ W ⊆ G.facet (ss.getD i (0, 0, 0)).1
  complete : ∀ y ∈ W, y ∈ FacetPts G → covOfK cs y ∈ Φ
  E : List Cov
  ea : Nat → Nat
  edge2 : ∀ ε ∈ E, ε.count .zero = 2
  eabove : ∀ ε ∈ E, Cov.le (covOfK cs v) ε = true
  ereal : ∀ ε ∈ E, RealisedK cs ε
  esub : ∀ j (h : j < E.length), cell cs E[j] ∩ W ⊆ G.edge (ea j)
  /-- a point of a ledger edge near `v` is on one of the listed edge cells of that edge -/
  esharp : ∀ y ∈ W, ∀ a, y ∈ G.edge a → ∃ j, ∃ h : j < E.length, covOfK cs y = E[j] ∧ ea j = a
  ea_inj : ∀ j k, j < E.length → k < E.length → ea j = ea k → j = k
  /-- an edge below a sector bounds it -/
  bounds_of_le : ∀ i (hi : i < Φ.length) j (hj : j < E.length), Cov.le E[j] Φ[i] = true →
    Sec.bounds (ss.getD i (0, 0, 0)) (ea j)
  /-- a sector's bounding edges are listed edges below it -/
  le_of_bounds : ∀ i (hi : i < Φ.length) a, Sec.bounds (ss.getD i (0, 0, 0)) a →
    ∃ j, ∃ hj : j < E.length, ea j = a ∧ Cov.le E[j] Φ[i] = true
  /-- the local picture at the edge points near `v` (transversal there) -/
  local_ : ∀ y ∈ W, LocalGood cs y

/-- The sector pieces are the sectors' cells near the vert. -/
theorem vpiece_eq {ss : List Sec} {v : P} (m : VertModel G cs ss v) (i : Nat) (hi : i < m.Φ.length) :
    {y | y ∈ m.W ∧ y ∈ G.facet (ss.getD i (0, 0, 0)).1 ∧ m.Φ.idxOf (covOfK cs y) = i} =
      cell cs m.Φ[i] ∩ m.W := by
  ext y
  constructor
  · rintro ⟨hyW, hyf, hyi⟩
    have hm := m.complete y hyW (Set.mem_iUnion.mpr ⟨_, hyf⟩)
    refine ⟨?_, hyW⟩
    rw [mem_cell]
    have := getElem_idxOf_covOfK hm
    simp only [hyi] at this
    exact this.symm
  · rintro ⟨hy, hyW⟩
    rw [mem_cell] at hy
    refine ⟨hyW, m.sub i hi ⟨by rw [mem_cell]; exact hy, hyW⟩, ?_⟩
    rw [hy]
    exact m.nodup.idxOf_getElem i hi

theorem edge_mem {ss : List Sec} {v : P} (m : VertModel G cs ss v) {y : P} (hyW : y ∈ m.W)
    (hyE : y ∈ EdgePts G) : covOfK cs y ∈ m.E := by
  obtain ⟨a, ha⟩ := Set.mem_iUnion.mp hyE
  obtain ⟨j, hj, e, -⟩ := m.esharp y hyW a ha
  rw [e]; exact List.getElem_mem hj

/-- **F1–F4 at a vert, from the local model and the local picture.** -/
theorem vertLocal_of_model (hc : ∀ c ∈ cs, Continuous c) {ss : List Sec} {v : P}
    (m : VertModel G cs ss v) : Nonempty (VertLocal G ss v) := by
  have hpiece := vpiece_eq m
  refine ⟨{
    W₀ := m.W
    hW₀ := m.hWo.mem_nhds m.hvW
    slab := fun y => m.Φ.idxOf (covOfK cs y)
    elb := fun y => m.ea (m.E.idxOf (covOfK cs y))
    v_facet := m.v_facet
    v_edge := m.v_edge
    cov := ?_
    loc := ?_
    ecov := ?_
    eloc := ?_
    near := ?_
    conn := ?_
    adj := ?_ }⟩
  · intro y hyW hyF
    have hm := m.complete y hyW hyF
    have hlt := List.idxOf_lt_length_iff.mpr hm
    refine ⟨m.len ▸ hlt, m.sub _ hlt ⟨?_, hyW⟩⟩
    rw [mem_cell]; exact (getElem_idxOf_covOfK hm).symm
  · rintro y ⟨hyW, hyF⟩
    have hy1 := m.facet _ (m.complete y hyW hyF)
    refine ⟨_, Filter.inter_mem (eventually_le hc y) (m.hWo.mem_nhds hyW),
      fun z ⟨hz, hzW, hzF⟩ => ?_, fun z hz hzE => ?_⟩
    · have e : covOfK cs y = covOfK cs z := eq_of_le_count_eq hz.1 (by
        rw [hy1, m.facet _ (m.complete z hzW hzF)])
      simp only [e]
    · -- near a facet point there are no edge points: they would have more zeros
      have h2 := m.edge2 _ (edge_mem m hz.2 hzE)
      have := count_le_of_le hz.1
      omega
  · intro y hyW hyE
    have hm := edge_mem m hyW hyE
    have hlt := List.idxOf_lt_length_iff.mpr hm
    exact m.esub _ hlt ⟨by rw [mem_cell]; exact (getElem_idxOf_covOfK hm).symm, hyW⟩
  · rintro y ⟨hyW, hyE⟩
    have hyE' := edge_mem m hyW hyE
    refine ⟨_, Filter.inter_mem (eventually_le hc y) (m.hWo.mem_nhds hyW),
      fun z ⟨hz, hzW, hzE⟩ => ?_, fun z ⟨hz, hzW, hzF⟩ => ?_⟩
    · have e : covOfK cs y = covOfK cs z := eq_of_le_count_eq hz.1 (by
        rw [m.edge2 _ hyE', m.edge2 _ (edge_mem m hzW hzE)])
      simp only [e]
    · have hzm := m.complete z hzW hzF
      have hiz := List.idxOf_lt_length_iff.mpr hzm
      have hjy := List.idxOf_lt_length_iff.mpr hyE'
      refine m.bounds_of_le _ hiz _ hjy ?_
      rw [getElem_idxOf_covOfK hzm, getElem_idxOf_covOfK hyE']
      exact hz.1
  · intro i hi
    have hi' : i < m.Φ.length := m.len ▸ hi
    show v ∈ closure {y | y ∈ m.W ∧ y ∈ G.facet (ss.getD i (0, 0, 0)).1 ∧
      m.Φ.idxOf (covOfK cs y) = i}
    rw [hpiece i hi']
    exact mem_closure_inter_open ((m.local_ v m.hvW).near _ (m.above _ (List.getElem_mem hi'))
      (m.real _ (List.getElem_mem hi'))) m.hWo m.hvW
  · intro i hi U hU
    have hi' : i < m.Φ.length := m.len ▸ hi
    obtain ⟨O, hOU, hOo, hxO⟩ := mem_nhds_iff.mp (Filter.inter_mem hU (m.hWo.mem_nhds m.hvW))
    obtain ⟨U'', hU''o, hU''O, hxU'', hcon⟩ := (m.local_ v m.hvW).conn O hOo hxO
    refine ⟨U'', hU''o.mem_nhds hxU'', fun y hy => (hOU (hU''O hy)).1, ?_⟩
    show IsPreconnected ({y | y ∈ m.W ∧ y ∈ G.facet (ss.getD i (0, 0, 0)).1 ∧
      m.Φ.idxOf (covOfK cs y) = i} ∩ U'')
    rw [hpiece i hi']
    have : cell cs m.Φ[i] ∩ m.W ∩ U'' = cell cs m.Φ[i] ∩ U'' := by
      ext y; constructor
      · rintro ⟨⟨h1, _⟩, h3⟩; exact ⟨h1, h3⟩
      · rintro ⟨h1, h3⟩; exact ⟨⟨h1, (hOU (hU''O h3)).2⟩, h3⟩
    rw [this]
    exact hcon _ (m.above _ (List.getElem_mem hi'))
  · intro i hi a hb
    have hi' : i < m.Φ.length := m.len ▸ hi
    obtain ⟨j, hj, hea, hle⟩ := m.le_of_bounds i hi' a hb
    have hEm := List.getElem_mem hj
    refine ⟨?_, fun y ⟨hya, hyW⟩ => ?_⟩
    · refine closure_mono (hea ▸ m.esub j hj) ?_
      exact mem_closure_inter_open ((m.local_ v m.hvW).near _ (m.eabove _ hEm) (m.ereal _ hEm))
        m.hWo m.hvW
    · obtain ⟨j', hj', e, hea'⟩ := m.esharp y hyW a hya
      have hjj : j' = j := m.ea_inj j' j hj' hj (hea'.trans hea.symm)
      subst hjj
      show y ∈ closure {y | y ∈ m.W ∧ y ∈ G.facet (ss.getD i (0, 0, 0)).1 ∧
        m.Φ.idxOf (covOfK cs y) = i}
      rw [hpiece i hi']
      refine mem_closure_inter_open ((m.local_ y hyW).near _ ?_ (m.real _ (List.getElem_mem hi')))
        m.hWo hyW
      rw [e]; exact hle

/-- **The builder's obligations from local models**: F5 (the rooms), and at every point of every
listed edge, and at every listed vert, a local carrier model with the local picture (carriers
may differ from point to point: the near faces there). -/
theorem obligations_of_models {keep : Nat → Bool} {L : Rows (P := P)} (hs : SideLocal G keep)
    (hne : ∀ e ∈ L.es, (G.edge e).Nonempty)
    (he : ∀ e ∈ L.es, ∀ x ∈ G.edge e, ∃ cs : List (Carrier P ℝ), (∀ c ∈ cs, Continuous c) ∧
      Nonempty (EdgeModel G cs (L.erow e) x) ∧ LocalGood cs x)
    (hv : ∀ v ∈ L.vs, ∃ cs : List (Carrier P ℝ), (∀ c ∈ cs, Continuous c) ∧
      Nonempty (VertModel G cs (L.vsec v) (L.vpt v))) :
    Obligations G keep L where
  sides := hs
  edge e heL := ⟨hne e heL, fun x hx => by
    obtain ⟨cs, hc, ⟨m⟩, hl⟩ := he e heL x hx
    exact edgeLocal_of_model hc m hl⟩
  vert v hvL := by
    obtain ⟨cs, hc, ⟨m⟩⟩ := hv v hvL
    exact vertLocal_of_model hc m

/-- **Route B's theorem, from local models**: the core's refusal over the face ledger is the
result's non-manifoldness, given F5 and the local carrier models. -/
theorem xNonManifoldF_iff_top_of_models {keep : Nat → Bool} {L : Rows (P := P)}
    (hs : SideLocal G keep) (hne : ∀ e ∈ L.es, (G.edge e).Nonempty)
    (he : ∀ e ∈ L.es, ∀ x ∈ G.edge e, ∃ cs : List (Carrier P ℝ), (∀ c ∈ cs, Continuous c) ∧
      Nonempty (EdgeModel G cs (L.erow e) x) ∧ LocalGood cs x)
    (hv : ∀ v ∈ L.vs, ∃ cs : List (Carrier P ℝ), (∀ c ∈ cs, Continuous c) ∧
      Nonempty (VertModel G cs (L.vsec v) (L.vpt v))) :
    XNonManifoldF G keep L ↔ TopNonManifoldF G (Res G keep) L :=
  xNonManifoldF_iff_top (obligations_of_models hs hne he hv)

end ForgeCurved.FaceLocal
