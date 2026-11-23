import ForgeCurved.SweepCross
import Mathlib.Analysis.Normed.Module.HahnBanach
import Mathlib.LinearAlgebra.FiniteDimensional.Lemmas

/-!
# The facet half of `CrossesAll`

A facet cell (one zero, at carrier `j`) that meets the slab meets every sweep plane in it. The
argument is `component_crosses`' (levels open and closed in `(a, b)`), on the surface `Z(c_j)`:
* **`aux_functional`**: at a point where `ℓ` and `∇c_j` are independent, some linear `μ` completes
  them to a chart (`dim V = 3`);
* **`sheet`** (the inverse function theorem for `(ℓ, c_j, g)`): near `q ∈ Z(c_j)` the points of
  `Z(c_j)` with `g ∈ J` form a connected sheet over an interval of levels round `ℓ q`, inside any
  given neighbourhood. `g = μ` with `J = ℝ` is a whole neighbourhood in the surface. `g = c_k`
  with `J = (0, ∞)` or `(-∞, 0)` is one side of the curve `Z(c_j) ∩ Z(c_k)`;
* **`crossesAll_facets_of_transverse`**: at a limit level, the limit point is interior to the
  facet or on exactly one bounding curve (no triple point). Either way a sheet joins it to the
  component.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.Sweep

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Set Filter Topology

variable {V : Type} [NormedAddCommGroup V] [NormedSpace ℝ V] [FiniteDimensional ℝ V]

/-- **A completing coordinate.** -/
theorem aux_functional {ℓ : V →L[ℝ] ℝ} {c : V → ℝ} (hdim : Module.finrank ℝ V = 3) {q : V}
    (ht : Transverse ℓ [c] q) : ∃ μ : V →L[ℝ] ℝ, Transverse ℓ [c, fun y => μ y] q := by
  set D2 := (DΦ ℓ [c] q).toLinearMap
  have hsurj : Function.Surjective D2 := ht
  have hker : Module.finrank ℝ (LinearMap.ker D2) = 1 := by
    have h := LinearMap.finrank_range_add_finrank_ker D2
    rw [LinearMap.range_eq_top.mpr hsurj, finrank_top, Module.finrank_prod, Module.finrank_self,
      Module.finrank_fin_fun, hdim] at h
    simp only [List.length_cons, List.length_nil] at h
    omega
  obtain ⟨w, hw0, hw⟩ := finrank_eq_one_iff'.mp hker
  have hw0' : (w : V) ≠ 0 := fun h => hw0 (Subtype.ext h)
  obtain ⟨μ, -, hμw⟩ := exists_dual_vector ℝ (w : V) (norm_ne_zero_iff.mpr hw0')
  refine ⟨μ, ?_⟩
  have hdim' : Module.finrank ℝ V = [c, fun y => μ y].length + 1 := by simp [hdim]
  have hinj : Function.Injective (DΦ ℓ [c, fun y => μ y] q).toLinearMap := by
    rw [← LinearMap.ker_eq_bot, LinearMap.ker_eq_bot']
    intro v hv
    have h1 : ℓ v = 0 := congrArg Prod.fst hv
    have h2 : fderiv ℝ c q v = 0 := congrFun (congrArg Prod.snd hv) ⟨0, by simp⟩
    have h3 : μ v = 0 := by
      have := congrFun (congrArg Prod.snd hv) ⟨1, by simp⟩
      simpa [DΦ, ContinuousLinearMap.fderiv] using this
    have hvK : v ∈ LinearMap.ker D2 := by
      rw [LinearMap.mem_ker]
      refine Prod.ext h1 (funext fun i => ?_)
      obtain ⟨i, hi⟩ := i
      have : i = 0 := by simp only [List.length_cons, List.length_nil] at hi; omega
      subst this; exact h2
    obtain ⟨a, ha⟩ := hw ⟨v, hvK⟩
    have hav : a • (w : V) = v := congrArg Subtype.val ha
    rw [← hav, map_smul, hμw, smul_eq_mul, mul_eq_zero] at h3
    rcases h3 with h | h
    · rw [← hav, h, zero_smul]
    · exact absurd (norm_eq_zero.mp h) hw0'
  exact (LinearMap.injective_iff_surjective_of_finrank_eq_finrank (by
    rw [hdim', Module.finrank_prod, Module.finrank_self, Module.finrank_fin_fun]; ring)).mp hinj

/-- The point of the `(t, 0, s)` plane in chart coordinates. -/
def ι2 (p : ℝ × ℝ) : ℝ × (Fin 2 → ℝ) := (p.1, ![0, p.2])

theorem Φ_pair {ℓ : V →L[ℝ] ℝ} {c g : V → ℝ} {z : V} (hz : c z = 0) :
    Φ ℓ [c, g] z = ι2 (ℓ z, g z) := by
  simp only [Φ, ι2, Prod.mk.injEq, true_and]
  funext i
  fin_cases i <;> simp [hz]

/-- **A connected sheet of `Z(c)` with `g ∈ J`, over levels round `ℓ q`.** -/
theorem sheet {ℓ : V →L[ℝ] ℝ} {c g : V → ℝ} (hdim : Module.finrank ℝ V = 3)
    (hc : ContDiff ℝ 1 c) (hg : ContDiff ℝ 1 g) {q : V} (hq : c q = 0)
    (ht : Transverse ℓ [c, g] q) {J : Set ℝ} (hJo : IsOpen J) (hJc : J.OrdConnected)
    (hJq : g q ∈ closure J) {N₀ : Set V} (hN₀ : N₀ ∈ 𝓝 q) :
    ∃ ε > 0, ∃ S : Set V, IsPreconnected S ∧ S ⊆ N₀ ∧
      (∀ z ∈ S, c z = 0 ∧ g z ∈ J ∧ ℓ z ∈ Ioo (ℓ q - ε) (ℓ q + ε)) ∧
      (∀ t ∈ Ioo (ℓ q - ε) (ℓ q + ε), ∃ z ∈ S, ℓ z = t) ∧
      ∃ N ∈ 𝓝 q, ∀ z ∈ N, c z = 0 → g z ∈ J → z ∈ S := by
  have hC : ∀ c' ∈ [c, g], ContDiff ℝ 1 c' := by
    intro c' hc'; simp only [List.mem_cons, List.not_mem_nil, or_false] at hc'
    rcases hc' with rfl | rfl <;> assumption
  have hdim' : Module.finrank ℝ V = [c, g].length + 1 := by simp [hdim]
  have hf : HasStrictFDerivAt (Φ ℓ [c, g])
      ((DΦequiv hdim' ht : V ≃L[ℝ] ℝ × (Fin [c, g].length → ℝ)) :
        V →L[ℝ] ℝ × (Fin [c, g].length → ℝ)) q := by
    rw [DΦequiv_coe]; exact phi_strict hC q
  set e := hf.toOpenPartialHomeomorph (Φ ℓ [c, g])
  have hqs : q ∈ e.source := hf.mem_toOpenPartialHomeomorph_source
  have hΦq : Φ ℓ [c, g] q = ι2 (ℓ q, g q) := Φ_pair hq
  have hnear : e.target ∩ e.symm ⁻¹' N₀ ∈ 𝓝 (ι2 (ℓ q, g q)) := by
    have htq : ι2 (ℓ q, g q) ∈ e.target := by
      rw [← hΦq]; exact hf.image_mem_toOpenPartialHomeomorph_target
    have hsq : e.symm (ι2 (ℓ q, g q)) = q := by rw [← hΦq]; exact e.left_inv hqs
    refine inter_mem (e.open_target.mem_nhds htq) ?_
    exact e.continuousAt_symm htq (by rw [hsq]; exact hN₀)
  obtain ⟨ε, hε, hball⟩ := Metric.mem_nhds_iff.mp hnear
  have hι : ∀ t s, |t - ℓ q| < ε → |s - g q| < ε → ι2 (t, s) ∈ Metric.ball (ι2 (ℓ q, g q)) ε := by
    intro t s h1 h2
    rw [Metric.mem_ball, Prod.dist_eq, max_lt_iff, Real.dist_eq]
    refine ⟨h1, (dist_pi_lt_iff hε).mpr fun i => ?_⟩
    fin_cases i <;> simp [ι2, Real.dist_eq, h2, hε]
  set Jε := J ∩ Ioo (g q - ε) (g q + ε)
  set B := Ioo (ℓ q - ε) (ℓ q + ε) ×ˢ Jε
  have hBt : ∀ p ∈ B, ι2 p ∈ e.target ∩ e.symm ⁻¹' N₀ := by
    rintro ⟨t, s⟩ ⟨ht', hs', hs''⟩
    exact hball (hι t s (abs_sub_lt_iff.mpr ⟨by linarith [ht'.2], by linarith [ht'.1]⟩)
      (abs_sub_lt_iff.mpr ⟨by linarith [hs''.2], by linarith [hs''.1]⟩))
  have hJε : Jε.Nonempty := by
    obtain ⟨s, hs, hsJ⟩ := mem_closure_iff.mp hJq (Ioo (g q - ε) (g q + ε)) isOpen_Ioo
      ⟨by linarith, by linarith⟩
    exact ⟨s, hsJ, hs⟩
  set S := e.symm '' (ι2 '' B)
  have hSprop : ∀ p ∈ B, Φ ℓ [c, g] (e.symm (ι2 p)) = ι2 p := fun p hp => e.right_inv (hBt p hp).1
  refine ⟨ε, hε, S, ?_, ?_, ?_, ?_, ?_⟩
  · have hB : IsPreconnected B := isPreconnected_Ioo.prod (hJc.inter ordConnected_Ioo).isPreconnected
    refine (hB.image ι2 (Continuous.continuousOn (by unfold ι2; fun_prop))).image _
      (e.continuousOn_symm.mono ?_)
    rintro _ ⟨p, hp, rfl⟩; exact (hBt p hp).1
  · rintro _ ⟨_, ⟨p, hp, rfl⟩, rfl⟩; exact (hBt p hp).2
  · rintro _ ⟨_, ⟨⟨t, s⟩, hp, rfl⟩, rfl⟩
    have h := hSprop _ hp
    simp only [Φ, ι2, Prod.mk.injEq] at h
    obtain ⟨h1, h2⟩ := h
    have h0 : c (e.symm (ι2 (t, s))) = 0 := by
      exact congrFun h2 0
    have hs1 : g (e.symm (ι2 (t, s))) = s := by
      exact congrFun h2 1
    refine ⟨h0, ?_, ?_⟩
    · rw [hs1]; exact hp.2.1
    · show ℓ (e.symm (t, ![0, s])) ∈ _
      rw [h1]; exact hp.1
  · intro t ht'
    obtain ⟨s, hs⟩ := hJε
    refine ⟨e.symm (ι2 (t, s)), ⟨ι2 (t, s), ⟨(t, s), ⟨ht', hs⟩, rfl⟩, rfl⟩, ?_⟩
    have h := hSprop (t, s) ⟨ht', hs⟩
    simp only [Φ, ι2, Prod.mk.injEq] at h
    exact h.1
  · refine ⟨e.source ∩ Φ ℓ [c, g] ⁻¹' Metric.ball (ι2 (ℓ q, g q)) ε,
      inter_mem (e.open_source.mem_nhds hqs) ((Metric.isOpen_ball.preimage
        (by unfold Φ; exact ℓ.continuous.prodMk (continuous_pi fun i =>
          (hC _ (List.getElem_mem _)).continuous))).mem_nhds
          (show Φ ℓ [c, g] q ∈ _ by rw [hΦq]; exact Metric.mem_ball_self hε)),
      fun z ⟨hzs, hzb⟩ hz0 hzJ => ?_⟩
    rw [mem_preimage, Φ_pair hz0] at hzb
    simp only [ι2, Metric.mem_ball, Prod.dist_eq, max_lt_iff, Real.dist_eq] at hzb
    have h2 : |g z - g q| < ε := by
      have := (dist_pi_lt_iff hε).mp hzb.2 1
      simpa [ι2, Real.dist_eq] using this
    have hzB : (ℓ z, g z) ∈ B := ⟨⟨by linarith [(abs_sub_lt_iff.mp hzb.1).2],
        by linarith [(abs_sub_lt_iff.mp hzb.1).1]⟩, hzJ,
      by linarith [(abs_sub_lt_iff.mp h2).2], by linarith [(abs_sub_lt_iff.mp h2).1]⟩
    refine ⟨ι2 (ℓ z, g z), ⟨_, hzB, rfl⟩, ?_⟩
    have := e.left_inv hzs
    rw [show e z = Φ ℓ [c, g] z from rfl, Φ_pair hz0] at this
    exact this

/-- Levels that are open and closed in `(a, b)` and not empty are all of `(a, b)`. -/
theorem levels_all {I : Set ℝ} {a b : ℝ} (hI : I ⊆ Ioo a b) (ho : IsOpen I)
    (hcl : ∀ t ∈ Ioo a b, t ∈ closure I → t ∈ I) (hne : I.Nonempty) : Ioo a b ⊆ I := by
  rcases (isPreconnected_iff_subset_of_disjoint.mp isPreconnected_Ioo) I (closure I)ᶜ ho
      isClosed_closure.isOpen_compl (fun t ht => by
        by_cases h : t ∈ closure I
        · exact Or.inl (hcl t ht h)
        · exact Or.inr h)
      (by ext t; simp only [mem_inter_iff, mem_compl_iff, mem_empty_iff_false, iff_false];
          rintro ⟨-, h1, h2⟩; exact h2 (subset_closure h1)) with h | h
  · exact h
  · obtain ⟨t, ht⟩ := hne
    exact absurd (subset_closure ht) (h (hI ht))

/-- A limit level of a set of zeros in the slab is attained at a zero in its closure. -/
theorem limit_point {ℓ : V →L[ℝ] ℝ} {ks : List (V → ℝ)} {a b : ℝ}
    (hC : ∀ c ∈ ks, Continuous c) (hP : Proper ℓ ks a b) {D : Set V}
    (hD : ∀ z ∈ D, ∀ c ∈ ks, c z = 0) {t : ℝ} (htab : t ∈ Ioo a b) (ht : t ∈ closure (ℓ '' D)) :
    ∃ q ∈ closure D, ℓ q = t ∧ ∀ c ∈ ks, c q = 0 := by
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
      isClosed_singleton.preimage (hC _ (List.getElem_mem _)))
  have hK : IsCompact K := Metric.isCompact_of_isClosed_isBounded hKc (hP a' b' ha' hb')
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
    exact ⟨ℓ y, hsO, y, ⟨hyD, hsab.1.le, hsab.2.le, hD y hyD⟩, rfl⟩
  obtain ⟨q, hqA, rfl⟩ := hcl
  have hqK : q ∈ K := hK.isClosed.closure_subset_iff.mpr inter_subset_right hqA
  exact ⟨q, closure_mono inter_subset_left hqA, rfl, hqK.2.2⟩

/-- **The facet half of `CrossesAll`, from the critical-set conditions.** On a 3-dimensional `V`,
with `C¹` carriers, each carrier transverse to the sweep at its zeros in the slab and proper
there, each pair transverse at its common zeros, and no triple point: a cell with one zero (at
`j`) that meets the slab meets every sweep plane in it. -/
theorem crossesAll_facets_of_transverse {cs : List (V → ℝ)} {ℓ : V →L[ℝ] ℝ} {a b : ℝ}
    (hdim : Module.finrank ℝ V = 3) (hC : ∀ c ∈ cs, ContDiff ℝ 1 c)
    (ht1 : ∀ j (hj : j < cs.length), ∀ x ∈ Zs ℓ [cs[j]] a b, Transverse ℓ [cs[j]] x)
    (ht2 : ∀ j k (hj : j < cs.length) (hk : k < cs.length), j ≠ k →
      ∀ x ∈ Zs ℓ [cs[j], cs[k]] a b, Transverse ℓ [cs[j], cs[k]] x)
    (hP : ∀ j (hj : j < cs.length), Proper ℓ [cs[j]] a b)
    (h3 : NoTriple ℓ cs a b) (X : Cov) {j : ℕ} (hj : j < cs.length)
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
    obtain ⟨q, hqD, hqt, hqz⟩ := limit_point (ks := [cs[j]])
      (fun c hc => by rw [List.mem_singleton] at hc; subst hc; exact hcont j hj)
      (hP j hj) (fun z hz => by simp [(hcell z (hDF hz).2).1]) htab htI
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
