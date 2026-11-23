import ForgeCurved.QOnTriple
import ForgeCurved.Cells

/-!
# `ExactK.edges` and `ExactK.verts`, in the sampling frame

Over the frame carriers, with the vertex certificate (U) on every vertex candidate and every piece
inside the bounding carrier:

* **`edges_iff_frame`**: `e ∈ L.edges ↔ RealisedK e ∧ IsEdgeK e ∧ ∃ φ ∈ L.facets, e ≤ φ`;
* **`verts_iff_frame`**: `v ∈ L.verts ↔ RealisedK v ∧ IsVertK v ∧ ∃ φ ∈ L.facets, v ≤ φ`.

The argument:
* a cell with a point off the events is, with two zeros, an edge sample (`twozero_sampled`), and
  is never a point (`edge_slab`, `facet_slab`);
* a cell with no such point lies on its (U) triple (`onTriple`), so it is a vertex sample
  (`vert_sampled`) and, by (U), one point (`vertOneAt_spec`).
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem le_at : ∀ {X Y : Cov}, Cov.le X Y = true → ∀ {i : ℕ} {a b : Sgn}, X[i]? = some a →
    Y[i]? = some b → a = Sgn.zero ∨ a = b := by
  intro X Y h
  rw [le_iff] at h
  induction h with
  | nil => intro i a b ha; simp at ha
  | @cons a' b' X Y hab _ ih =>
    intro i a b ha hb
    cases i with
    | zero =>
      simp only [List.getElem?_cons_zero, Option.some.injEq] at ha hb
      subst ha hb; exact hab
    | succ i => exact ih (by simpa using ha) (by simpa using hb)

theorem le_length {X Y : Cov} (h : Cov.le X Y = true) : X.length = Y.length := by
  simp only [Cov.le, Bool.and_eq_true, beq_iff_eq] at h; exact h.1

/-- Below a piece (negative on `B`), a cell is negative on `B` or on it. -/
theorem le_bi {e φ : Cov} (h : Cov.le e φ = true) {bi : ℕ} (hφ : φ[bi]? = some Sgn.neg) :
    e[bi]? = some Sgn.neg ∨ e[bi]? = some Sgn.zero := by
  have hl : bi < e.length := by
    rw [le_length h]; exact (List.getElem?_eq_some_iff.mp hφ).1
  obtain ⟨a, ha⟩ : ∃ a, e[bi]? = some a := ⟨e[bi], List.getElem?_eq_getElem hl⟩
  rcases le_at h ha hφ with h1 | h1 <;> subst h1
  · right; exact ha
  · left; exact ha

/-- Below a facet, a cell has a zero. -/
theorem le_has_zero {v φ : Cov} (h : Cov.le v φ = true) (hφ : φ.isFacet = true) :
    1 ≤ v.count Sgn.zero := by
  have hz := facet_zero hφ
  have hl : φ.zeroIdx < v.length := by
    rw [le_length h]; exact (List.getElem?_eq_some_iff.mp hz).1
  obtain ⟨a, ha⟩ : ∃ a, v[φ.zeroIdx]? = some a := ⟨_, List.getElem?_eq_getElem hl⟩
  have : a = Sgn.zero := by rcases le_at h ha hz with h1 | h1 <;> exact h1
  subst this
  exact List.count_pos_iff.mpr (List.mem_of_getElem? ha)

/-- A cell below a facet with at most one zero is the facet. -/
theorem le_facet_eq {v φ : Cov} (h : Cov.le v φ = true) (hφ : φ.isFacet = true)
    (h1 : v.count Sgn.zero ≤ 1) : v = φ :=
  eq_of_le_of_count h (by rw [(isFacet_iff φ).mp hφ]; exact h1)

/-- A facet with a point off the events crosses its slab, so it is not a point. -/
theorem facet_not_point {qs : List Quad.Quadric} {E : List Num} (hE : eventsOf qs rootFuel = some E)
    {bi : ℕ} (hbi : bi < (qs.map qR).length) (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0})
    (X : Cov) (hXb : X[bi]? = some Sgn.neg) (hfac : X.isFacet = true)
    {p : R3} (hp : covCellK (qs.map qR) X p) (hoff : OffEvents E p) : ¬ IsVertK (qs.map qR) X := by
  rintro ⟨-, huniq⟩
  have hlen : X.length = (qs.map qR).length := by rw [← hp]; simp [covOfK]
  have hj : X.zeroIdx < (qs.map qR).length := by rw [← hlen]; exact zeroIdx_lt hfac
  obtain ⟨c, -, a, b, hat, htb, -, -, hout⟩ := slab_find (eventsOf_sep hE) (eventsOf_den hE)
    (t := p.1) hoff
  have hfree : ∀ z ∈ Slab ℓx a b, OffEvents E z := by
    intro z hz e he hd
    rcases hout e he z.1 hd with h1 | h1
    · exact absurd hz.1 (not_lt.mpr (by simpa [ℓx] using h1))
    · exact absurd hz.2 (not_lt.mpr (by simpa [ℓx] using h1))
  obtain ⟨ht1, ht2, h3⟩ := slab_regular hE hfree
  have hC : ∀ c ∈ qs.map qR, ContDiff ℝ 1 c := by
    intro c hc; obtain ⟨q, -, rfl⟩ := List.mem_map.mp hc; exact qR_contDiff q
  obtain ⟨x, hx, hl⟩ := crossesAll_facets_bounded finrank_R3 hC ht1 ht2 hbi hB h3 X hXb hj
    (facet_zero hfac) (facet_unique hfac)
    ⟨p, ⟨by simpa [ℓx] using hat, by simpa [ℓx] using htb⟩, hp⟩ ((a + p.1) / 2)
    (by linarith) (by linarith)
  have := huniq x p hx hp
  rw [this] at hl
  simp [ℓx] at hl
  linarith

/-- A two-zero cell with a point off the events is not a point. -/
theorem two_not_point {qs : List Quad.Quadric} {E : List Num} (hE : eventsOf qs rootFuel = some E)
    {bi : ℕ} (hbi : bi < (qs.map qR).length) (hB : Bornology.IsBounded {z | (qs.map qR)[bi] z ≤ 0})
    (X : Cov) (hXb : X[bi]? = some Sgn.neg ∨ X[bi]? = some Sgn.zero) {j k : ℕ} (hjk : j ≠ k)
    (hXj : X[j]? = some Sgn.zero) (hXk : X[k]? = some Sgn.zero)
    {p : R3} (hp : covCellK (qs.map qR) X p) (hoff : OffEvents E p) : ¬ IsVertK (qs.map qR) X := by
  rintro ⟨-, huniq⟩
  obtain ⟨-, -, a, b, hap, hpb, -, -, hall⟩ := edge_slab hE hbi hB X hXb hjk hXj hXk hp hoff
  obtain ⟨x, hx, hx1⟩ := hall ((a + p.1) / 2) (by linarith) (by linarith)
  have := huniq x p hx hp
  rw [this] at hx1
  linarith

end ForgeCurved

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set

theorem mem_edges {nc : ℕ} {forms : List SForm} {ss : List QS} {e : Cov} :
    e ∈ (curvedLedger nc forms ss).edges ↔
      (∃ s ∈ ss, s.tag ≠ .vert ∧ 2 ≤ s.cov.count Sgn.zero ∧ s.cov = e) ∧
        ∃ φ ∈ (curvedLedger nc forms ss).facets, Cov.le e φ = true := by
  simp only [curvedLedger, List.mem_filter, mem_dedup, List.mem_map, List.any_eq_true,
    bne_iff_ne, ne_eq, Bool.and_eq_true, decide_eq_true_eq]
  constructor
  · rintro ⟨⟨s, ⟨hs, htag, h2⟩, rfl⟩, φ, hφ, hle⟩
    exact ⟨⟨s, hs, htag, h2, rfl⟩, φ, hφ, hle⟩
  · rintro ⟨⟨s, hs, htag, h2, rfl⟩, φ, hφ, hle⟩
    exact ⟨⟨s, ⟨hs, htag, h2⟩, rfl⟩, φ, hφ, hle⟩

theorem mem_verts {nc : ℕ} {forms : List SForm} {ss : List QS} {v : Cov} :
    v ∈ (curvedLedger nc forms ss).verts ↔
      (∃ s ∈ ss, s.tag = .vert ∧ s.cov = v) ∧ (¬ ∃ s ∈ ss, s.tag ≠ .vert ∧ s.cov = v) ∧
        ∃ φ ∈ (curvedLedger nc forms ss).facets, Cov.le v φ = true := by
  simp only [curvedLedger, List.mem_filter, mem_dedup, List.mem_map, List.any_eq_true,
    bne_iff_ne, ne_eq, Bool.and_eq_true, Bool.not_eq_true', beq_iff_eq, decide_eq_true_eq]
  constructor
  · rintro ⟨⟨s, ⟨hs, htag⟩, rfl⟩, hnot, φ, hφ, hle⟩
    refine ⟨⟨s, hs, htag, rfl⟩, ?_, φ, hφ, hle⟩
    rintro ⟨s', hs', htag', hcov⟩
    simp only [List.any_eq_false, Bool.and_eq_true, bne_iff_ne, ne_eq, beq_iff_eq, not_and] at hnot
    exact hnot s' hs' htag' hcov
  · rintro ⟨⟨s, hs, htag, rfl⟩, hnot, φ, hφ, hle⟩
    refine ⟨⟨s, ⟨hs, htag⟩, rfl⟩, ?_, φ, hφ, hle⟩
    simp only [List.any_eq_false, Bool.and_eq_true, bne_iff_ne, ne_eq, beq_iff_eq, not_and]
    intro s' hs' htag' hcov
    exact hnot ⟨s', hs', htag', hcov⟩

end ForgeCurved
