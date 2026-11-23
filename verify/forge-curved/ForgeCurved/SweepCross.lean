import ForgeCurved.Sweep
import ForgeCurved.Thom
import Mathlib.Analysis.Calculus.InverseFunctionTheorem.FDeriv
import Mathlib.Analysis.Calculus.ContDiff.Basic
import Mathlib.Analysis.Calculus.ContDiff.RCLike
import Mathlib.Analysis.Calculus.FDeriv.Prod
import Mathlib.Topology.Algebra.Module.FiniteDimension
import Mathlib.Analysis.Normed.Module.FiniteDimension

/-!
# The sweep's crossings from transversality and properness

`qsampler_covers` (dol-curved) needs `CrossesAll`: every lower-dimensional cell that meets a slab
meets every sweep plane in it. Here its edge half is proven from the critical-set conditions
themselves (`Transverse`, `Proper`, `NoTriple`). The global graph labelling of `ZeroGraphs` is
not needed, only:

* **`chart`** (the inverse function theorem): at a transverse common zero `q` of `ks`, with
  `dim V = |ks| + 1`, the map `x ↦ (ℓ x, (c x)_{c ∈ ks})` is a local homeomorphism. Near `q` the
  common zeros are one continuous graph `γ` over an interval of levels round `ℓ q`;
* **`component_crosses`**: in the slab, the connected component of any common zero meets every
  level. Its levels are open (by the chart) and closed in `(a, b)` (compactness from `Proper`, and
  the chart at a limit point);
* **`crossesAll_edges_of_transverse`**: the edge half of `CrossesAll` for `C¹` carriers on a
  3-dimensional `V` from pairwise transversality and properness and no triple point. The other
  carriers do not vanish on the component, so they keep their signs (`Thom.sgnK_const`).
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.Sweep

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set Filter Topology

variable {V : Type} [NormedAddCommGroup V] [NormedSpace ℝ V] [FiniteDimensional ℝ V]

/-- The sweep level and the carriers' values. -/
def Φ (ℓ : V →L[ℝ] ℝ) (ks : List (V → ℝ)) (x : V) : ℝ × (Fin ks.length → ℝ) :=
  (ℓ x, fun i => ks[i] x)

/-- The derivative of `Φ`. -/
noncomputable def DΦ (ℓ : V →L[ℝ] ℝ) (ks : List (V → ℝ)) (x : V) :
    V →L[ℝ] ℝ × (Fin ks.length → ℝ) :=
  ℓ.prod (ContinuousLinearMap.pi fun i => fderiv ℝ ks[i] x)

theorem phi_strict {ℓ : V →L[ℝ] ℝ} {ks : List (V → ℝ)} (hC : ∀ c ∈ ks, ContDiff ℝ 1 c) (x : V) :
    HasStrictFDerivAt (Φ ℓ ks) (DΦ ℓ ks x) x :=
  ℓ.hasStrictFDerivAt.prodMk (hasStrictFDerivAt_pi.2 fun i =>
    ((hC _ (List.getElem_mem _)).hasStrictFDerivAt (by norm_num)))

/-- A transverse derivative is an isomorphism when `dim V = |ks| + 1`. -/
noncomputable def DΦequiv {ℓ : V →L[ℝ] ℝ} {ks : List (V → ℝ)} {x : V}
    (hdim : Module.finrank ℝ V = ks.length + 1) (ht : Transverse ℓ ks x) :
    V ≃L[ℝ] ℝ × (Fin ks.length → ℝ) :=
  (LinearEquiv.ofBijective (DΦ ℓ ks x).toLinearMap
    ⟨(LinearMap.injective_iff_surjective_of_finrank_eq_finrank (by
      rw [hdim, Module.finrank_prod, Module.finrank_self, Module.finrank_fin_fun]; ring)).mpr ht,
     ht⟩).toContinuousLinearEquiv

theorem DΦequiv_coe {ℓ : V →L[ℝ] ℝ} {ks : List (V → ℝ)} {x : V}
    (hdim : Module.finrank ℝ V = ks.length + 1) (ht : Transverse ℓ ks x) :
    ((DΦequiv hdim ht : V ≃L[ℝ] ℝ × (Fin ks.length → ℝ)) : V →L[ℝ] ℝ × (Fin ks.length → ℝ)) =
      DΦ ℓ ks x := by
  ext v <;> rfl

theorem zero_iff {ks : List (V → ℝ)} {z : V} :
    (∀ c ∈ ks, c z = 0) ↔ ∀ i : Fin ks.length, ks[i] z = 0 := by
  constructor
  · intro h i; exact h _ (List.getElem_mem _)
  · intro h c hc
    obtain ⟨i, hi, rfl⟩ := List.mem_iff_getElem.mp hc
    exact h ⟨i, hi⟩

/-- **The chart at a transverse common zero.** -/
theorem chart {ℓ : V →L[ℝ] ℝ} {ks : List (V → ℝ)} (hC : ∀ c ∈ ks, ContDiff ℝ 1 c)
    (hdim : Module.finrank ℝ V = ks.length + 1) {q : V} (hq : ∀ c ∈ ks, c q = 0)
    (ht : Transverse ℓ ks q) :
    ∃ U : Set V, IsOpen U ∧ q ∈ U ∧ ∃ ε > 0, ∃ γ : ℝ → V,
      ContinuousOn γ (Ioo (ℓ q - ε) (ℓ q + ε)) ∧
      (∀ t ∈ Ioo (ℓ q - ε) (ℓ q + ε), ℓ (γ t) = t ∧ ∀ c ∈ ks, c (γ t) = 0) ∧
      ∀ z ∈ U, (∀ c ∈ ks, c z = 0) → ℓ z ∈ Ioo (ℓ q - ε) (ℓ q + ε) ∧ z = γ (ℓ z) := by
  have hf : HasStrictFDerivAt (Φ ℓ ks)
      ((DΦequiv hdim ht : V ≃L[ℝ] ℝ × (Fin ks.length → ℝ)) : V →L[ℝ] ℝ × (Fin ks.length → ℝ)) q := by
    rw [DΦequiv_coe]; exact phi_strict hC q
  set e := hf.toOpenPartialHomeomorph (Φ ℓ ks)
  have hqs : q ∈ e.source := hf.mem_toOpenPartialHomeomorph_source
  have hΦq : Φ ℓ ks q = (ℓ q, 0) := by
    simp only [Φ, Prod.mk.injEq, true_and]; funext i; exact (zero_iff.mp hq) i
  have htq : (ℓ q, (0 : Fin ks.length → ℝ)) ∈ e.target := by
    rw [← hΦq]; exact hf.image_mem_toOpenPartialHomeomorph_target
  obtain ⟨ε, hε, hball⟩ := Metric.isOpen_iff.mp e.open_target _ htq
  have hlev : ∀ t ∈ Ioo (ℓ q - ε) (ℓ q + ε), (t, (0 : Fin ks.length → ℝ)) ∈ e.target := by
    intro t ht
    apply hball
    rw [Metric.mem_ball, Prod.dist_eq, dist_self, max_eq_left dist_nonneg, Real.dist_eq,
      abs_sub_lt_iff]
    constructor <;> linarith [ht.1, ht.2]
  set γ : ℝ → V := fun t => e.symm (t, 0)
  have hγΦ : ∀ t ∈ Ioo (ℓ q - ε) (ℓ q + ε), Φ ℓ ks (γ t) = (t, 0) := fun t ht =>
    e.right_inv (hlev t ht)
  refine ⟨e.source ∩ ℓ ⁻¹' Ioo (ℓ q - ε) (ℓ q + ε),
    e.open_source.inter (isOpen_Ioo.preimage ℓ.continuous),
    ⟨hqs, by constructor <;> linarith⟩, ε, hε, γ, ?_, fun t ht => ?_, ?_⟩
  · exact e.continuousOn_symm.comp (Continuous.continuousOn (by fun_prop)) fun t ht => hlev t ht
  · have h := hγΦ t ht
    simp only [Φ, Prod.mk.injEq] at h
    refine ⟨h.1, zero_iff.mpr fun i => ?_⟩
    exact congrFun h.2 i
  · rintro z ⟨hzs, hzl⟩ hz
    refine ⟨hzl, ?_⟩
    have hΦz : Φ ℓ ks z = (ℓ z, 0) := by
      simp only [Φ, Prod.mk.injEq, true_and]; funext i; exact (zero_iff.mp hz) i
    have := e.left_inv hzs
    rw [show e z = Φ ℓ ks z from rfl, hΦz] at this
    exact this.symm

/-- The common zeros of `ks` in the slab. -/
def Zs (ℓ : V →L[ℝ] ℝ) (ks : List (V → ℝ)) (a b : ℝ) : Set V :=
  {x | x ∈ Slab ℓ a b ∧ ∀ c ∈ ks, c x = 0}

/-- **Every component of the common zeros in the slab meets every level.** -/
theorem component_crosses {ℓ : V →L[ℝ] ℝ} {ks : List (V → ℝ)} {a b : ℝ}
    (hC : ∀ c ∈ ks, ContDiff ℝ 1 c) (hdim : Module.finrank ℝ V = ks.length + 1)
    (ht : ∀ x ∈ Zs ℓ ks a b, Transverse ℓ ks x) (hP : Proper ℓ ks a b)
    {x : V} (hx : x ∈ Zs ℓ ks a b) :
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
    set K := {z : V | a' ≤ ℓ z ∧ ℓ z ≤ b' ∧ ∀ c ∈ ks, c z = 0}
    have hKc : IsClosed K := by
      have : K = ℓ ⁻¹' Icc a' b' ∩ ⋂ i : Fin ks.length, (ks[i]) ⁻¹' {0} := by
        ext z; simp only [K, mem_ofPred_eq, mem_inter_iff, mem_preimage, mem_Icc, mem_iInter,
          mem_singleton_iff, zero_iff, and_assoc]
      rw [this]
      exact (isClosed_Icc.preimage ℓ.continuous).inter (isClosed_iInter fun i =>
        isClosed_singleton.preimage ((hC _ (List.getElem_mem _)).continuous))
    have hK : IsCompact K := Metric.isCompact_of_isClosed_isBounded hKc (hP a' b' ha' hb')
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
      exact ⟨ℓ y, hsO, y, ⟨hyD, hsab.1.le, hsab.2.le, (hDZ hyD).2⟩, rfl⟩
    obtain ⟨q, hqA, rfl⟩ := hcl
    have hqK : q ∈ K := hK.isClosed.closure_subset_iff.mpr inter_subset_right hqA
    have hqZ : q ∈ Z := ⟨htab, hqK.2.2⟩
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

/-- **The edge half of `CrossesAll`, from the critical-set conditions.** On a 3-dimensional `V`,
with `C¹` carriers, every pair transverse to the sweep at its common zeros in the slab and proper
there, and no triple point: every cell with two zeros that meets the slab meets every sweep plane
in it. -/
theorem crossesAll_edges_of_transverse {cs : List (V → ℝ)} {ℓ : V →L[ℝ] ℝ} {a b : ℝ}
    (hdim : Module.finrank ℝ V = 3) (hC : ∀ c ∈ cs, ContDiff ℝ 1 c)
    (ht : ∀ j k (hj : j < cs.length) (hk : k < cs.length), j ≠ k →
      ∀ x ∈ Zs ℓ [cs[j], cs[k]] a b, Transverse ℓ [cs[j], cs[k]] x)
    (hP : ∀ j k (hj : j < cs.length) (hk : k < cs.length), j ≠ k → Proper ℓ [cs[j], cs[k]] a b)
    (h3 : NoTriple ℓ cs a b) (X : Cov) {j k : ℕ} (hjk : j ≠ k)
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
  obtain ⟨y, hyD, hyc⟩ := component_crosses hCk (by rw [hdim]; rfl)
    (ht j k hj hk hjk) (hP j k hj hk hjk) hxZ c ⟨hac, hcb⟩
  exact ⟨y, hsign y hyD, hyc⟩

end ForgeCurved.Sweep
