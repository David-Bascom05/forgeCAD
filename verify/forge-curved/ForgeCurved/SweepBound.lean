import ForgeCurved.SweepCross
import ForgeCurved.SweepFacet

/-!
# The sweep's crossings, bounded: no properness needed

`crossesAll_edges_of_transverse` and `crossesAll_facets_of_transverse` (LemmaD) need `Proper`.
That is false for every plane not perpendicular to the sweep and for every cylinder, cone and
seam, so as stated they would cover no CAD input with a plane or a seam. Here the same proofs
run with compactness from a **bounding carrier** instead: a carrier `c_b` whose sublevel set
`{c_b ≤ 0}` is bounded, with the cell on its negative side (`X[b] = neg`). dol-curved's
`qledgerB` appends a rational sphere `B` and certifies every body inside it (`certOk_inside`),
so every cell the ledger reads is of this kind.

* `component_crossesS`, `limit_pointS`: LemmaD's lemmas with the compact set cut down to a
  closed `S` that holds the component;
* **`crossesAll_edges_bounded`**, **`crossesAll_facets_bounded`**: the two halves of
  `CrossesAll` with the bounding carrier in place of `Proper`. Pairwise transversality,
  transversality of single carriers, and `NoTriple` are as before, and they cover `B` like any
  other carrier.

The proofs are LemmaD's, with the compact set changed. The originals stay as they are.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.Sweep

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set Filter Topology

variable {V : Type} [NormedAddCommGroup V] [NormedSpace ℝ V] [FiniteDimensional ℝ V]

/-- **Every component of the common zeros in the slab meets every level.** -/
theorem component_crossesS {ℓ : V →L[ℝ] ℝ} {ks : List (V → ℝ)} {a b : ℝ}
    (hC : ∀ c ∈ ks, ContDiff ℝ 1 c) (hdim : Module.finrank ℝ V = ks.length + 1)
    (ht : ∀ x ∈ Zs ℓ ks a b, Transverse ℓ ks x) {S : Set V} (hSc : IsClosed S)
    (hPS : ∀ a' b', a < a' → b' < b →
      Bornology.IsBounded {z | a' ≤ ℓ z ∧ ℓ z ≤ b' ∧ (∀ c ∈ ks, c z = 0) ∧ z ∈ S})
    {x : V} (hx : x ∈ Zs ℓ ks a b) (hDS : connectedComponentIn (Zs ℓ ks a b) x ⊆ S) :
    ∀ c ∈ Ioo a b, ∃ y ∈ connectedComponentIn (Zs ℓ ks a b) x, ℓ y = c := by
  set Z := Zs ℓ ks a b
  set D := connectedComponentIn Z x
  have hDZ : D ⊆ Z := connectedComponentIn_subset _ _
  have hxD : x ∈ D := mem_connectedComponentIn hx
  set I := ℓ '' D
  have hIab : I ⊆ Ioo a b := by rintro _ ⟨y, hy, rfl⟩; exact (hDZ hy).1
  -- the local graph at a common zero, as a preconnected set of common zeros round it
  have hloc : ∀ q ∈ Z, ∃ U : Set V, IsOpen U ∧ q ∈ U ∧ ∃ ε > 0,
      Ioo (ℓ q - ε) (ℓ q + ε) ⊆ Ioo a b ∧ ∃ Γ : Set V, IsPreconnected Γ ∧ Γ ⊆ Z ∧ q ∈ Γ ∧
        Ioo (ℓ q - ε) (ℓ q + ε) ⊆ ℓ '' Γ ∧ ∀ z ∈ U, z ∈ Z → z ∈ Γ := by
    intro q hq
    obtain ⟨U, hU, hqU, ε₀, hε₀, γ, hγc, hγ, huniq⟩ := chart hC hdim hq.2 (ht q hq)
    set ε := min ε₀ (min (ℓ q - a) (b - ℓ q))
    have hε : 0 < ε := lt_min hε₀ (lt_min (by linarith [hq.1.1]) (by linarith [hq.1.2]))
    have hsub : Ioo (ℓ q - ε) (ℓ q + ε) ⊆ Ioo (ℓ q - ε₀) (ℓ q + ε₀) := fun t ht =>
      ⟨by linarith [ht.1, min_le_left ε₀ (min (ℓ q - a) (b - ℓ q))],
       by linarith [ht.2, min_le_left ε₀ (min (ℓ q - a) (b - ℓ q))]⟩
    have hab : Ioo (ℓ q - ε) (ℓ q + ε) ⊆ Ioo a b := fun t ht =>
      ⟨by linarith [ht.1, min_le_right ε₀ (min (ℓ q - a) (b - ℓ q)),
          min_le_left (ℓ q - a) (b - ℓ q)],
       by linarith [ht.2, min_le_right ε₀ (min (ℓ q - a) (b - ℓ q)),
          min_le_right (ℓ q - a) (b - ℓ q)]⟩
    refine ⟨U ∩ ℓ ⁻¹' Ioo (ℓ q - ε) (ℓ q + ε), hU.inter (isOpen_Ioo.preimage ℓ.continuous),
      ⟨hqU, by constructor <;> linarith⟩, ε, hε, hab, γ '' Ioo (ℓ q - ε) (ℓ q + ε),
      isPreconnected_Ioo.image _ (hγc.mono hsub), ?_, ?_, ?_, ?_⟩
    · rintro _ ⟨t, htt, rfl⟩
      obtain ⟨h1, h2⟩ := hγ t (hsub htt)
      exact ⟨by show a < ℓ (γ t) ∧ ℓ (γ t) < b; rw [h1]; exact hab htt, h2⟩
    · refine ⟨ℓ q, ⟨by linarith, by linarith⟩, ?_⟩
      exact ((huniq q hqU hq.2).2).symm
    · intro t htt
      exact ⟨γ t, ⟨t, htt, rfl⟩, (hγ t (hsub htt)).1⟩
    · rintro z ⟨hzU, hzl⟩ hzZ
      obtain ⟨-, hz⟩ := huniq z hzU hzZ.2
      exact ⟨ℓ z, hzl, hz.symm⟩
  -- a local graph meeting `D` lies in `D`
  have hjoin : ∀ Γ : Set V, IsPreconnected Γ → Γ ⊆ Z → (Γ ∩ D).Nonempty → Γ ⊆ D := by
    rintro Γ hΓ hΓZ ⟨z, hzΓ, hzD⟩
    show Γ ⊆ connectedComponentIn Z x
    rw [connectedComponentIn_eq hzD]
    exact hΓ.subset_connectedComponentIn hzΓ hΓZ
  -- the levels of `D` are open
  have hIopen : IsOpen I := by
    rw [Metric.isOpen_iff]
    rintro _ ⟨q, hqD, rfl⟩
    obtain ⟨U, -, -, ε, hε, -, Γ, hΓ, hΓZ, hqΓ, hΓl, -⟩ := hloc q (hDZ hqD)
    refine ⟨ε, hε, fun t htb => ?_⟩
    have htt : t ∈ Ioo (ℓ q - ε) (ℓ q + ε) := by
      rw [Metric.mem_ball, Real.dist_eq, abs_sub_lt_iff] at htb
      constructor <;> linarith
    obtain ⟨y, hyΓ, rfl⟩ := hΓl htt
    exact ⟨y, hjoin Γ hΓ hΓZ ⟨q, hqΓ, hqD⟩ hyΓ, rfl⟩
  -- and closed in `(a, b)`
  have hIclosed : ∀ t ∈ Ioo a b, t ∈ closure I → t ∈ I := by
    intro t htab htI
    set a' := (a + t) / 2
    set b' := (t + b) / 2
    have ha' : a < a' := by simp only [a']; linarith [htab.1]
    have hb' : b' < b := by simp only [b']; linarith [htab.2]
    set K := {z : V | a' ≤ ℓ z ∧ ℓ z ≤ b' ∧ (∀ c ∈ ks, c z = 0) ∧ z ∈ S}
    have hKc : IsClosed K := by
      have : K = (ℓ ⁻¹' Icc a' b' ∩ ⋂ i : Fin ks.length, (ks[i]) ⁻¹' {0}) ∩ S := by
        ext z; simp only [K, mem_ofPred_eq, mem_inter_iff, mem_preimage, mem_Icc, mem_iInter,
          mem_singleton_iff, zero_iff, and_assoc]
      rw [this]
      exact ((isClosed_Icc.preimage ℓ.continuous).inter (isClosed_iInter fun i =>
        isClosed_singleton.preimage ((hC _ (List.getElem_mem _)).continuous))).inter hSc
    have hK : IsCompact K := Metric.isCompact_of_isClosed_isBounded hKc (hPS a' b' ha' hb')
    set A := D ∩ K
    have hAc : IsCompact (closure A) := hK.closure_of_subset inter_subset_right
    have hcl : t ∈ ℓ '' closure A := by
      have himg : IsClosed (ℓ '' closure A) := (hAc.image ℓ.continuous).isClosed
      refine himg.closure_subset_iff.mpr (image_mono subset_closure) ?_
      rw [mem_closure_iff] at htI ⊢
      intro O hO htO
      have htO' : t ∈ O ∩ Ioo a' b' :=
        ⟨htO, by simp only [a']; linarith [htab.1], by simp only [b']; linarith [htab.2]⟩
      obtain ⟨_, ⟨hsO, hsab⟩, ⟨y, hyD, rfl⟩⟩ := htI (O ∩ Ioo a' b') (hO.inter isOpen_Ioo) htO'
      exact ⟨ℓ y, hsO, y, ⟨hyD, hsab.1.le, hsab.2.le, (hDZ hyD).2, hDS hyD⟩, rfl⟩
    obtain ⟨q, hqA, rfl⟩ := hcl
    have hqK : q ∈ K := hK.isClosed.closure_subset_iff.mpr inter_subset_right hqA
    have hqZ : q ∈ Z := ⟨htab, hqK.2.2.1⟩
    obtain ⟨U, hU, hqU, ε, -, -, Γ, hΓ, hΓZ, hqΓ, -, hΓU⟩ := hloc q hqZ
    obtain ⟨z, hzU, hzA⟩ := mem_closure_iff.mp hqA U hU hqU
    have hzΓ := hΓU z hzU (hDZ hzA.1)
    exact ⟨q, hjoin Γ hΓ hΓZ ⟨z, hzΓ, hzA.1⟩ hqΓ, rfl⟩
  -- so the levels are all of `(a, b)`
  have hall : Ioo a b ⊆ I := by
    rcases (isPreconnected_iff_subset_of_disjoint.mp isPreconnected_Ioo) I (closure I)ᶜ hIopen
        isClosed_closure.isOpen_compl (fun t ht => by
          by_cases h : t ∈ closure I
          · exact Or.inl (hIclosed t ht h)
          · exact Or.inr h)
        (by ext t; simp only [mem_inter_iff, mem_compl_iff, mem_empty_iff_false, iff_false];
            rintro ⟨-, h1, h2⟩; exact h2 (subset_closure h1)) with h | h
    · exact h
    · exact absurd (subset_closure (show ℓ x ∈ I from ⟨x, hxD, rfl⟩)) (h hx.1)
  intro c hc
  obtain ⟨y, hy, rfl⟩ := hall hc
  exact ⟨y, hy, rfl⟩

/-- **The edge half of `CrossesAll`, bounded.** The cell is on the bounding carrier's negative
side or on it (an edge of `B` itself). -/
theorem crossesAll_edges_bounded {cs : List (V → ℝ)} {ℓ : V →L[ℝ] ℝ} {a b : ℝ}
    (hdim : Module.finrank ℝ V = 3) (hC : ∀ c ∈ cs, ContDiff ℝ 1 c)
    (ht : ∀ j k (hj : j < cs.length) (hk : k < cs.length), j ≠ k →
      ∀ x ∈ Zs ℓ [cs[j], cs[k]] a b, Transverse ℓ [cs[j], cs[k]] x)
    {bi : ℕ} (hbi : bi < cs.length) (hB : Bornology.IsBounded {z | cs[bi] z ≤ 0})
    (h3 : NoTriple ℓ cs a b) (X : Cov) (hXb : X[bi]? = some Sgn.neg ∨ X[bi]? = some Sgn.zero)
    {j k : ℕ} (hjk : j ≠ k)
    (hXj : X[j]? = some Sgn.zero) (hXk : X[k]? = some Sgn.zero)
    (hx : ∃ x ∈ Slab ℓ a b, covCellK cs X x) :
    ∀ c, a < c → c < b → ∃ x, covCellK cs X x ∧ ℓ x = c := by
  obtain ⟨x, hxS, hxX⟩ := hx
  have hxX' : covOfK cs x = X := hxX
  have hzero : ∀ m, X[m]? = some Sgn.zero → ∃ hm : m < cs.length, cs[m] x = 0 := by
    intro m hm
    rw [← hxX', getElem?_covOfK] at hm
    rcases h : cs[m]? with _ | c
    · rw [h] at hm; cases hm
    · rw [h] at hm
      have hlt : m < cs.length := (List.getElem?_eq_some_iff.mp h).1
      refine ⟨hlt, ?_⟩
      rw [(List.getElem?_eq_some_iff.mp h).2]
      exact sgnK_zero_iff.mp (Option.some.inj hm)
  obtain ⟨hj, hxj⟩ := hzero j hXj
  obtain ⟨hk, hxk⟩ := hzero k hXk
  set ks := [cs[j], cs[k]]
  have hCk : ∀ c ∈ ks, ContDiff ℝ 1 c := by
    intro c hc
    simp only [ks, List.mem_cons, List.not_mem_nil, or_false] at hc
    rcases hc with rfl | rfl <;> exact hC _ (List.getElem_mem _)
  have hxZ : x ∈ Zs ℓ ks a b := ⟨hxS, by simp [ks, hxj, hxk]⟩
  set D := connectedComponentIn (Zs ℓ ks a b) x
  have hDZ : D ⊆ Zs ℓ ks a b := connectedComponentIn_subset _ _
  have hD : IsPreconnected D := isPreconnected_connectedComponentIn
  -- every carrier keeps its sign on the component
  have hsign : ∀ y ∈ D, covOfK cs y = X := by
    intro y hy
    rw [← hxX']
    apply List.ext_getElem (by simp [covOfK])
    intro m hm1 hm2
    simp only [covOfK, List.getElem_map]
    have hm : m < cs.length := by simpa [covOfK] using hm1
    have hon : ∀ z ∈ D, cs[j] z = 0 ∧ cs[k] z = 0 := fun z hz => by
      have := (hDZ hz).2; simp only [ks, List.mem_cons, List.not_mem_nil, or_false,
        forall_eq_or_imp, forall_eq] at this; exact this
    by_cases hmj : m = j
    · subst hmj; rw [(hon y hy).1, hxj]
    by_cases hmk : m = k
    · subst hmk; rw [(hon y hy).2, hxk]
    have hne : ∀ z ∈ D, cs[m] z ≠ 0 := fun z hz h0 =>
      h3 z (hDZ hz).1 j k m hj hk hm hjk (Ne.symm hmk) (Ne.symm hmj) ⟨(hon z hz).1, (hon z hz).2, h0⟩
    exact Thom.sgnK_const hD (hC _ (List.getElem_mem hm)).continuous.continuousOn hne hy
      (mem_connectedComponentIn hxZ)
  intro c hac hcb
  have hSc : IsClosed {z : V | cs[bi] z ≤ 0} :=
    isClosed_le (hC _ (List.getElem_mem hbi)).continuous continuous_const
  have hDS : D ⊆ {z : V | cs[bi] z ≤ 0} := by
    intro y hy
    rcases hXb with h | h
    · rw [← hsign y hy, getElem?_covOfK] at h
      simp only [hbi, getElem?_pos, Option.map_some, Option.some.injEq] at h
      rcases lt_trichotomy (cs[bi] y) 0 with h' | h' | h'
      · exact h'.le
      · rw [sgnK_zero_iff.mpr h'] at h; cases h
      · rw [sgnK_pos h'] at h; cases h
    · rw [← hsign y hy, getElem?_covOfK] at h
      simp only [hbi, getElem?_pos, Option.map_some, Option.some.injEq] at h
      exact (sgnK_zero_iff.mp h).le
  obtain ⟨y, hyD, hyc⟩ := component_crossesS hCk (by rw [hdim]; rfl)
    (ht j k hj hk hjk) hSc (fun a' b' _ _ => hB.subset fun z hz => hz.2.2.2) hxZ hDS c ⟨hac, hcb⟩
  exact ⟨y, hsign y hyD, hyc⟩

/-- A limit level of a set of zeros in the slab is attained at a zero in its closure. -/
theorem limit_pointS {ℓ : V →L[ℝ] ℝ} {ks : List (V → ℝ)} {a b : ℝ}
    (hC : ∀ c ∈ ks, Continuous c) {S : Set V} (hSc : IsClosed S)
    (hPS : ∀ a' b', a < a' → b' < b →
      Bornology.IsBounded {z | a' ≤ ℓ z ∧ ℓ z ≤ b' ∧ (∀ c ∈ ks, c z = 0) ∧ z ∈ S}) {D : Set V}
    (hD : ∀ z ∈ D, ∀ c ∈ ks, c z = 0) (hDS : D ⊆ S) {t : ℝ} (htab : t ∈ Ioo a b) (ht : t ∈ closure (ℓ '' D)) :
    ∃ q ∈ closure D, ℓ q = t ∧ ∀ c ∈ ks, c q = 0 := by
  set a' := (a + t) / 2
  set b' := (t + b) / 2
  have ha' : a < a' := by simp only [a']; linarith [htab.1]
  have hb' : b' < b := by simp only [b']; linarith [htab.2]
  set K := {z : V | a' ≤ ℓ z ∧ ℓ z ≤ b' ∧ (∀ c ∈ ks, c z = 0) ∧ z ∈ S}
  have hKc : IsClosed K := by
    have : K = (ℓ ⁻¹' Icc a' b' ∩ ⋂ i : Fin ks.length, (ks[i]) ⁻¹' {0}) ∩ S := by
      ext z; simp only [K, mem_ofPred_eq, mem_inter_iff, mem_preimage, mem_Icc, mem_iInter,
        mem_singleton_iff, zero_iff, and_assoc]
    rw [this]
    exact ((isClosed_Icc.preimage ℓ.continuous).inter (isClosed_iInter fun i =>
      isClosed_singleton.preimage (hC _ (List.getElem_mem _)))).inter hSc
  have hK : IsCompact K := Metric.isCompact_of_isClosed_isBounded hKc (hPS a' b' ha' hb')
  set A := D ∩ K
  have hAc : IsCompact (closure A) := hK.closure_of_subset inter_subset_right
  have hcl : t ∈ ℓ '' closure A := by
    have himg : IsClosed (ℓ '' closure A) := (hAc.image ℓ.continuous).isClosed
    refine himg.closure_subset_iff.mpr (image_mono subset_closure) ?_
    rw [mem_closure_iff] at ht ⊢
    intro O hO htO
    have htO' : t ∈ O ∩ Ioo a' b' :=
      ⟨htO, by simp only [a']; linarith [htab.1], by simp only [b']; linarith [htab.2]⟩
    obtain ⟨_, ⟨hsO, hsab⟩, ⟨y, hyD, rfl⟩⟩ := ht (O ∩ Ioo a' b') (hO.inter isOpen_Ioo) htO'
    exact ⟨ℓ y, hsO, y, ⟨hyD, hsab.1.le, hsab.2.le, hD y hyD, hDS hyD⟩, rfl⟩
  obtain ⟨q, hqA, rfl⟩ := hcl
  have hqK : q ∈ K := hK.isClosed.closure_subset_iff.mpr inter_subset_right hqA
  exact ⟨q, closure_mono inter_subset_left hqA, rfl, hqK.2.2.1⟩

/-- **The facet half of `CrossesAll`, bounded.** -/
theorem crossesAll_facets_bounded {cs : List (V → ℝ)} {ℓ : V →L[ℝ] ℝ} {a b : ℝ}
    (hdim : Module.finrank ℝ V = 3) (hC : ∀ c ∈ cs, ContDiff ℝ 1 c)
    (ht1 : ∀ j (hj : j < cs.length), ∀ x ∈ Zs ℓ [cs[j]] a b, Transverse ℓ [cs[j]] x)
    (ht2 : ∀ j k (hj : j < cs.length) (hk : k < cs.length), j ≠ k →
      ∀ x ∈ Zs ℓ [cs[j], cs[k]] a b, Transverse ℓ [cs[j], cs[k]] x)
    {bi : ℕ} (hbi : bi < cs.length) (hB : Bornology.IsBounded {z | cs[bi] z ≤ 0})
    (h3 : NoTriple ℓ cs a b) (X : Cov) (hXb : X[bi]? = some Sgn.neg) {j : ℕ} (hj : j < cs.length)
    (hXj : X[j]? = some Sgn.zero) (hXo : ∀ m, m ≠ j → X[m]? ≠ some Sgn.zero)
    (hx : ∃ x ∈ Slab ℓ a b, covCellK cs X x) :
    ∀ c, a < c → c < b → ∃ x, covCellK cs X x ∧ ℓ x = c := by
  classical
  obtain ⟨x, hxS, hxX⟩ := hx
  have hcont : ∀ m (hm : m < cs.length), Continuous cs[m] := fun m hm =>
    (hC _ (List.getElem_mem hm)).continuous
  -- the cell in the slab, and the component of `x`
  set Fc := {y | y ∈ Slab ℓ a b ∧ covOfK cs y = X}
  have hxF : x ∈ Fc := ⟨hxS, hxX⟩
  set D := connectedComponentIn Fc x
  have hDF : D ⊆ Fc := connectedComponentIn_subset _ _
  have hxD : x ∈ D := mem_connectedComponentIn hxF
  -- on the cell: `c_j` vanishes and the others do not
  have hcell : ∀ y, covOfK cs y = X → cs[j] y = 0 ∧ ∀ m (hm : m < cs.length), m ≠ j → cs[m] y ≠ 0 := by
    intro y hy
    have hget : ∀ m (hm : m < cs.length), X[m]? = some (sgnK (cs[m] y)) := fun m hm => by
      rw [← hy, getElem?_covOfK]; simp [hm]
    refine ⟨sgnK_zero_iff.mp (Option.some.inj ((hget j hj).symm.trans hXj)), fun m hm hmj h0 => ?_⟩
    exact hXo m hmj (by rw [hget m hm, sgnK_zero_iff.mpr h0])
  -- a preconnected piece of `Z(c_j)` in the slab, off the other carriers, meeting `D`, lies in `D`
  have hjoin : ∀ S : Set V, IsPreconnected S → S ⊆ Slab ℓ a b → (∀ z ∈ S, cs[j] z = 0) →
      (∀ z ∈ S, ∀ m (hm : m < cs.length), m ≠ j → cs[m] z ≠ 0) → (S ∩ D).Nonempty → S ⊆ D := by
    rintro S hS hSs hSj hSo ⟨z₀, hz₀S, hz₀D⟩
    have hSF : S ⊆ Fc := by
      intro z hz
      refine ⟨hSs hz, ?_⟩
      rw [← (hDF hz₀D).2]
      apply List.ext_getElem (by simp [covOfK])
      intro m hm1 _
      simp only [covOfK, List.getElem_map]
      have hm : m < cs.length := by simpa [covOfK] using hm1
      by_cases hmj : m = j
      · subst hmj; rw [hSj z hz, hSj z₀ hz₀S]
      · exact Thom.sgnK_const hS (hcont m hm).continuousOn (fun w hw => hSo w hw m hm hmj) hz hz₀S
    show S ⊆ connectedComponentIn Fc x
    rw [connectedComponentIn_eq hz₀D]
    exact hS.subset_connectedComponentIn hz₀S hSF
  -- the open set where the carriers other than `j` (and `k`) do not vanish
  have hoff : ∀ (p : ℕ → Prop) (q : V), (∀ m (hm : m < cs.length), p m → cs[m] q ≠ 0) →
      {z | z ∈ Slab ℓ a b ∧ ∀ m (hm : m < cs.length), p m → cs[m] z ≠ 0} ∈ 𝓝 q ∨
        q ∉ Slab ℓ a b := by
    intro p q hq
    by_cases hqS : q ∈ Slab ℓ a b
    · left
      have hSo : IsOpen (Slab ℓ a b) :=
        (isOpen_lt continuous_const ℓ.continuous).inter (isOpen_lt ℓ.continuous continuous_const)
      have : {z | z ∈ Slab ℓ a b ∧ ∀ m (hm : m < cs.length), p m → cs[m] z ≠ 0} =
          Slab ℓ a b ∩ ⋂ i : Fin cs.length, {z | p i → cs[i] z ≠ 0} := by
        ext z; simp only [mem_ofPred_eq, mem_inter_iff, mem_iInter]
        constructor
        · rintro ⟨h1, h2⟩; exact ⟨h1, fun i => h2 i i.2⟩
        · rintro ⟨h1, h2⟩; exact ⟨h1, fun m hm => h2 ⟨m, hm⟩⟩
      rw [this]
      refine (hSo.inter (isOpen_iInter_of_finite fun i => ?_)).mem_nhds
        ⟨hqS, mem_iInter.mpr fun i hpi => hq i i.2 hpi⟩
      by_cases hpi : p i
      · simp only [hpi, true_implies]; exact isOpen_ne_fun (hcont i i.2) continuous_const
      · simp [hpi]
    · exact Or.inr hqS
  set I := ℓ '' D
  have hIab : I ⊆ Ioo a b := by rintro _ ⟨y, hy, rfl⟩; exact (hDF hy).1
  -- a whole surface neighbourhood of an interior point `q` of the cell's closure
  have hinner : ∀ q, q ∈ Slab ℓ a b → cs[j] q = 0 → (∀ m (hm : m < cs.length), m ≠ j → cs[m] q ≠ 0) →
      q ∈ closure D → ∃ ε > 0, ∀ t ∈ Ioo (ℓ q - ε) (ℓ q + ε), t ∈ I := by
    intro q hqS hqj hqo hqD
    obtain ⟨μ, hμ⟩ := aux_functional hdim (ht1 j hj q ⟨hqS, by simp [hqj]⟩)
    have hN₀ := (hoff (fun m => m ≠ j) q hqo).resolve_right (not_not.mpr hqS)
    obtain ⟨ε, hε, S, hS, hSN, hSz, hSl, N, hN, hNS⟩ := sheet hdim (hC _ (List.getElem_mem hj))
      (by fun_prop : ContDiff ℝ 1 fun y => μ y) hqj hμ (J := univ) isOpen_univ ordConnected_univ
      (by simp) hN₀
    obtain ⟨z, hzN, hzD⟩ := mem_closure_iff_nhds.mp hqD N hN
    have hSD : S ⊆ D := hjoin S hS (fun z hz => (hSN hz).1) (fun z hz => (hSz z hz).1)
      (fun z hz => (hSN hz).2) ⟨z, hNS z hzN (hcell z (hDF hzD).2).1 trivial, hzD⟩
    refine ⟨ε, hε, fun t ht => ?_⟩
    obtain ⟨w, hwS, rfl⟩ := hSl t ht
    exact ⟨w, hSD hwS, rfl⟩
  have hIopen : IsOpen I := by
    rw [Metric.isOpen_iff]
    rintro _ ⟨q, hqD, rfl⟩
    obtain ⟨hqS, hqX⟩ := hDF hqD
    obtain ⟨hqj, hqo⟩ := hcell q hqX
    obtain ⟨ε, hε, hε'⟩ := hinner q hqS hqj hqo (subset_closure hqD)
    refine ⟨ε, hε, fun t htb => hε' t ?_⟩
    rw [Metric.mem_ball, Real.dist_eq, abs_sub_lt_iff] at htb
    constructor <;> linarith
  have hIclosed : ∀ t ∈ Ioo a b, t ∈ closure I → t ∈ I := by
    intro t htab htI
    have hSc : IsClosed {z : V | cs[bi] z ≤ 0} := isClosed_le (hcont bi hbi) continuous_const
    have hDS : D ⊆ {z : V | cs[bi] z ≤ 0} := by
      intro y hy
      have h := hXb
      rw [← (hDF hy).2, getElem?_covOfK] at h
      simp only [hbi, getElem?_pos, Option.map_some, Option.some.injEq] at h
      rcases lt_trichotomy (cs[bi] y) 0 with h' | h' | h'
      · exact h'.le
      · rw [sgnK_zero_iff.mpr h'] at h; cases h
      · rw [sgnK_pos h'] at h; cases h
    obtain ⟨q, hqD, hqt, hqz⟩ := limit_pointS (ks := [cs[j]])
      (fun c hc => by rw [List.mem_singleton] at hc; subst hc; exact hcont j hj)
      hSc (fun a' b' _ _ => hB.subset fun z hz => hz.2.2.2)
      (fun z hz => by simp [(hcell z (hDF hz).2).1]) hDS htab htI
    have hqj : cs[j] q = 0 := by simpa using hqz
    have hqS : q ∈ Slab ℓ a b := by show a < ℓ q ∧ ℓ q < b; rw [hqt]; exact htab
    by_cases hk : ∃ k, ∃ hk : k < cs.length, k ≠ j ∧ cs[k] q = 0
    · -- on one bounding curve: the side of `c_k` that the cell is on
      obtain ⟨k, hk, hkj, hqk⟩ := hk
      have hqo : ∀ m (hm : m < cs.length), (m ≠ j ∧ m ≠ k) → cs[m] q ≠ 0 := fun m hm ⟨h1, h2⟩ h0 =>
        h3 q hqS j k m hj hk hm (Ne.symm hkj) (Ne.symm h2) (Ne.symm h1) ⟨hqj, hqk, h0⟩
      have hN₀ := (hoff (fun m => m ≠ j ∧ m ≠ k) q hqo).resolve_right (not_not.mpr hqS)
      -- the sign of `c_k` on the cell
      rcases hXk : X[k]? with _ | σ
      · exfalso
        have := (hcell _ (hDF hxD).2)
        rw [← (hDF hxD).2, getElem?_covOfK] at hXk; simp [hk] at hXk
      set J : Set ℝ := if σ = Sgn.pos then Ioi 0 else Iio 0
      have hσ : σ ≠ Sgn.zero := fun h => hXo k hkj (by rw [hXk, h])
      have hJ : ∀ z, covOfK cs z = X → cs[k] z ∈ J := by
        intro z hz
        have h := hXk
        rw [← hz, getElem?_covOfK] at h
        simp only [hk, getElem?_pos, Option.map_some, Option.some.injEq] at h
        rcases lt_trichotomy (cs[k] z) 0 with h' | h' | h'
        · rw [sgnK_neg h'] at h; subst h; simp [J, h']
        · rw [sgnK_zero_iff.mpr h'] at h; exact absurd h.symm hσ
        · rw [sgnK_pos h'] at h; subst h; simp [J, h']
      have hJo : IsOpen J := by
        simp only [J]; split_ifs; exacts [isOpen_Ioi, isOpen_Iio]
      have hJc : J.OrdConnected := by
        simp only [J]; split_ifs; exacts [ordConnected_Ioi, ordConnected_Iio]
      have hJq : cs[k] q ∈ closure J := by
        rw [hqk]; simp only [J]; split_ifs <;> simp
      obtain ⟨ε, hε, S, hS, hSN, hSz, hSl, N, hN, hNS⟩ := sheet hdim (hC _ (List.getElem_mem hj))
        (hC _ (List.getElem_mem hk)) hqj (ht2 j k hj hk (Ne.symm hkj) q ⟨hqS, by simp [hqj, hqk]⟩)
        hJo hJc hJq hN₀
      obtain ⟨z, hzN, hzD⟩ := mem_closure_iff_nhds.mp hqD N hN
      have hSD : S ⊆ D := by
        refine hjoin S hS (fun z hz => (hSN hz).1) (fun z hz => (hSz z hz).1) (fun z hz m hm hmj => ?_)
          ⟨z, hNS z hzN (hcell z (hDF hzD).2).1 (hJ z (hDF hzD).2), hzD⟩
        by_cases hmk : m = k
        · subst hmk
          have := (hSz z hz).2.1
          simp only [J] at this; split_ifs at this
          · exact ne_of_gt this
          · exact ne_of_lt this
        · exact (hSN hz).2 m hm ⟨hmj, hmk⟩
      obtain ⟨w, hwS, hwt⟩ := hSl (ℓ q) ⟨by linarith, by linarith⟩
      exact ⟨w, hSD hwS, by rw [hwt, hqt]⟩
    · push Not at hk
      obtain ⟨ε, hε, hε'⟩ := hinner q hqS hqj (fun m hm hmj => hk m hm hmj) hqD
      rw [← hqt]; exact hε' _ ⟨by linarith, by linarith⟩
  intro c hac hcb
  obtain ⟨y, hy, rfl⟩ := levels_all hIab hIopen hIclosed ⟨ℓ x, x, hxD, rfl⟩ ⟨hac, hcb⟩
  exact ⟨y, (hDF hy).2, rfl⟩

end ForgeCurved.Sweep
