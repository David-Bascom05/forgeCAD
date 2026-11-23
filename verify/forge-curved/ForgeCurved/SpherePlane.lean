import ForgeCurved.CoaxialContact
import ForgeCurved.FaceModel

/-!
# Sphere–plane point contact

A sphere resting on a plane touches it at one point, an isolated vertex of the face ledger with
no edges: each facet (the plane round the point, the sphere round it) is one sector with no
bounding edges (FaceLocal's `Sec` allows the empty list).

* **`graphCell_preconnected'`**: bands between graphs over a connected base are connected when
  every fibre over the base is nonempty. No order is assumed, so a point contact, where the graphs
  meet at one point of the base, is covered.
* **`punctured_preconnected`**: a sup-norm ball in `ℝ²` minus its centre is connected.
* **`spherePlane_localConn`**: local connectivity at the contact point. Near it, in coordinates
  `(z, x, y)`, the sphere is `z = a(x, y) = 1 - √(1 - x² - y²)`, with `a > 0` off the origin and
  `a(0) = 0`. A cell's base is the disc, the punctured disc, the centre, or empty, depending only on
  whether `d = 0`.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set FaceLocal

/-- **Bands are connected when no fibre is empty.** -/
theorem graphCell_preconnected' {m : Nat} {a : Fin m → ℝ × ℝ → ℝ} (ha : ∀ k, Continuous (a k))
    {D : Set (ℝ × ℝ)} (hD : IsPreconnected D) {δ : ℝ} (σ : Fin m → Sgn)
    (hfib : ∀ d ∈ D, ∃ u, |u| < δ ∧ ∀ k, sgnK (u - a k d) = σ k) :
    IsPreconnected (graphCell a D δ σ) := by
  rcases D.eq_empty_or_nonempty with he | ⟨d0, hd0⟩
  · have : graphCell a D δ σ = ∅ := by
      ext p; simp only [graphCell, he, mem_empty_iff_false, false_and, mem_setOf_eq]
    rw [this]; exact isPreconnected_empty
  obtain ⟨s, hs, hsD⟩ : ∃ s : ℝ × ℝ → ℝ, Continuous s ∧ ∀ d ∈ D,
      |s d| < δ ∧ ∀ k, sgnK (s d - a k d) = σ k := by
    by_cases hc : ∃ c, σ c = .zero
    · obtain ⟨c, hc⟩ := hc
      refine ⟨a c, ha c, fun d hd => ?_⟩
      obtain ⟨u, hu, huσ⟩ := hfib d hd
      have : u = a c d := by have := huσ c; rw [hc] at this; linarith [sgnK_zero_iff.mp this]
      subst this; exact ⟨hu, huσ⟩
    · push Not at hc
      set lpos := (List.finRange m).filter fun k => σ k = .pos
      set lneg := (List.finRange m).filter fun k => σ k = .neg
      set lo := fun d => lmax (fun k => a k d) (-δ) lpos
      set hi := fun d => lmin (fun k => a k d) δ lneg
      have mpos : ∀ k, k ∈ lpos ↔ σ k = .pos := fun k => by simp [lpos]
      have mneg : ∀ k, k ∈ lneg ↔ σ k = .neg := fun k => by simp [lneg]
      have hu : ∀ d ∈ D, ∃ u, lo d < u ∧ u < hi d := by
        intro d hd
        obtain ⟨u, hu, huσ⟩ := hfib d hd
        refine ⟨u, lmax_lt (by linarith [(abs_lt.mp hu).1]) fun k hk => ?_,
          lt_lmin (abs_lt.mp hu).2 fun k hk => ?_⟩
        · have := huσ k; rw [(mpos k).mp hk] at this
          rcases lt_trichotomy (u - a k d) 0 with h | h | h
          · rw [sgnK_neg h] at this; cases this
          · rw [h] at this; simp [sgnK] at this
          · linarith
        · have := huσ k; rw [(mneg k).mp hk] at this
          rcases lt_trichotomy (u - a k d) 0 with h | h | h
          · linarith
          · rw [h] at this; simp [sgnK] at this
          · rw [sgnK_pos h] at this; cases this
      refine ⟨fun d => (lo d + hi d) / 2,
        ((continuous_lmax ha (-δ) lpos).add (continuous_lmin ha δ lneg)).div_const 2,
        fun d hd => ⟨?_, fun k => ?_⟩⟩
      · obtain ⟨u, h1, h2⟩ := hu d hd
        have h3 := (le_lmax (f := fun k => a k d) (b := -δ) (l := lpos)).1
        have h4 := (lmin_le (f := fun k => a k d) (b := δ) (l := lneg)).1
        rw [abs_lt]; constructor <;> simp only [lo, hi] at h1 h2 h3 h4 ⊢ <;> linarith
      · obtain ⟨u, h1, h2⟩ := hu d hd
        rcases hk : σ k with _ | _ | _
        · have h5 := (lmin_le (f := fun k => a k d) (b := δ) (l := lneg)).2 k ((mneg k).mpr hk)
          exact sgnK_neg (by simp only [lo, hi] at h1 h2 h5 ⊢; linarith)
        · exact absurd hk (hc k)
        · have h5 := (le_lmax (f := fun k => a k d) (b := -δ) (l := lpos)).2 k ((mpos k).mpr hk)
          exact sgnK_pos (by simp only [lo, hi] at h1 h2 h5 ⊢; linarith)
  have hF : ∀ d, Convex ℝ {u : ℝ | |u| < δ ∧ ∀ k, sgnK (u - a k d) = σ k} := by
    intro d
    have : {u : ℝ | |u| < δ ∧ ∀ k, sgnK (u - a k d) = σ k} =
        Ioo (-δ) δ ∩ ⋂ k, {u : ℝ | sgnK (u - a k d) = σ k} := by
      ext u; simp [abs_lt]
    rw [this]
    exact (convex_Ioo _ _).inter (convex_iInter fun k => convex_sgnK_sub _ _)
  have hsec : IsPreconnected ((fun d => (s d, d)) '' D) :=
    hD.image _ ((hs.prodMk continuous_id).continuousOn)
  refine isPreconnected_of_forall (s d0, d0) fun y hy => ?_
  obtain ⟨hyD, hyδ, hyσ⟩ := hy
  set fib := (fun u : ℝ => ((u, y.2) : R3)) '' {u : ℝ | |u| < δ ∧ ∀ k, sgnK (u - a k y.2) = σ k}
  refine ⟨fib ∪ (fun d => (s d, d)) '' D, ?_, Or.inr ⟨d0, hd0, rfl⟩,
    Or.inl ⟨y.1, ⟨hyδ, hyσ⟩, rfl⟩, ?_⟩
  · rintro q (⟨u, ⟨hu1, hu2⟩, rfl⟩ | ⟨d, hd, rfl⟩)
    · exact ⟨hyD, hu1, hu2⟩
    · exact ⟨hd, (hsD d hd).1, (hsD d hd).2⟩
  · refine IsPreconnected.union (s y.2, y.2) ⟨s y.2, (hsD y.2 hyD), rfl⟩ ⟨y.2, hyD, rfl⟩
      ((hF y.2).isPreconnected.image _
        (by fun_prop : Continuous fun u : ℝ => ((u, y.2) : R3)).continuousOn) hsec

/-- **A punctured sup-norm disc is connected.** -/
theorem punctured_preconnected {η : ℝ} (hη : 0 < η) :
    IsPreconnected (Metric.ball (0 : ℝ × ℝ) η \ {0}) := by
  set B := Metric.ball (0 : ℝ × ℝ) η
  have hB : Convex ℝ B := convex_ball _ _
  have half : ∀ (f : ℝ × ℝ →ₗ[ℝ] ℝ) (s : Sgn), Convex ℝ (B ∩ f ⁻¹' {r | sgnK r = s}) :=
    fun f s => hB.inter ((convex_sgnK s).linear_preimage f)
  set H1 := B ∩ (LinearMap.fst ℝ ℝ ℝ) ⁻¹' {r | sgnK r = .pos}
  set H2 := B ∩ (LinearMap.snd ℝ ℝ ℝ) ⁻¹' {r | sgnK r = .pos}
  set H3 := B ∩ (LinearMap.fst ℝ ℝ ℝ) ⁻¹' {r | sgnK r = .neg}
  set H4 := B ∩ (LinearMap.snd ℝ ℝ ℝ) ⁻¹' {r | sgnK r = .neg}
  have hε : 0 < η / 2 := half_pos hη
  have inB : ∀ a b : ℝ, |a| = η / 2 → |b| = η / 2 → ((a, b) : ℝ × ℝ) ∈ B := by
    intro a b ha hb
    rw [Metric.mem_ball, dist_zero_right, Prod.norm_def, Real.norm_eq_abs, Real.norm_eq_abs, ha, hb,
      max_self]
    linarith
  have pe : |η / 2| = η / 2 := abs_of_pos hε
  have ne : |-(η / 2)| = η / 2 := by rw [abs_neg, pe]
  have sp : sgnK (η / 2) = .pos := sgnK_pos hε
  have sn : sgnK (-(η / 2)) = .neg := sgnK_neg (by linarith)
  have e : B \ {0} = ((H1 ∪ H2) ∪ H3) ∪ H4 := by
    ext d
    simp only [mem_diff, mem_singleton_iff, mem_union, mem_inter_iff, mem_preimage,
      LinearMap.fst_apply, LinearMap.snd_apply, mem_setOf_eq, H1, H2, H3, H4]
    constructor
    · rintro ⟨hd, hne⟩
      rcases lt_trichotomy d.1 0 with h1 | h1 | h1
      · exact Or.inl (Or.inr ⟨hd, sgnK_neg h1⟩)
      · rcases lt_trichotomy d.2 0 with h2 | h2 | h2
        · exact Or.inr ⟨hd, sgnK_neg h2⟩
        · exact absurd (Prod.ext h1 h2) hne
        · exact Or.inl (Or.inl (Or.inr ⟨hd, sgnK_pos h2⟩))
      · exact Or.inl (Or.inl (Or.inl ⟨hd, sgnK_pos h1⟩))
    · rintro (((⟨hd, h⟩ | ⟨hd, h⟩) | ⟨hd, h⟩) | ⟨hd, h⟩) <;> refine ⟨hd, fun h0 => ?_⟩ <;>
        rw [h0] at h <;> simp [sgnK] at h
  rw [e]
  refine IsPreconnected.union (-(η / 2), -(η / 2)) ?_ ?_ ?_ (half _ _).isPreconnected
  · exact Or.inr ⟨inB _ _ ne ne, by simpa using sn⟩
  · exact ⟨inB _ _ ne ne, by simpa using sn⟩
  refine IsPreconnected.union (-(η / 2), η / 2) ?_ ?_ ?_ (half _ _).isPreconnected
  · exact Or.inr ⟨inB _ _ ne pe, by simpa using sp⟩
  · exact ⟨inB _ _ ne pe, by simpa using sn⟩
  refine IsPreconnected.union (η / 2, η / 2) ⟨inB _ _ pe pe, by simpa using sp⟩
    ⟨inB _ _ pe pe, by simpa using sp⟩ (half _ _).isPreconnected (half _ _).isPreconnected

/-! ## The contact point -/

/-- The unit sphere resting on the plane `z = 0`. -/
def spherePlane : List (Carrier R3 ℝ) :=
  [fun p => p.1 ^ 2 + p.2.1 ^ 2 + (p.2.2 - 1) ^ 2 - 1, fun p => p.2.2]

theorem covOfK_spherePlane (p : R3) : covOfK spherePlane p =
    [sgnK (p.1 ^ 2 + p.2.1 ^ 2 + (p.2.2 - 1) ^ 2 - 1), sgnK p.2.2] := rfl

/-- The sphere's lower cap over the plane, and the plane. -/
noncomputable def spA : Fin 2 → ℝ × ℝ → ℝ
  | 0 => fun d => 1 - Real.sqrt (1 - d.1 ^ 2 - d.2 ^ 2)
  | 1 => fun _ => 0

theorem spA_cont : ∀ k, Continuous (spA k) := by
  intro k; fin_cases k <;> (simp only [spA]; fun_prop)

theorem norm_small {d : ℝ × ℝ} (hd : ‖d‖ < 1 / 4) : d.1 ^ 2 + d.2 ^ 2 < 1 / 8 := by
  have h1 := (norm_fst_le d).trans_lt hd
  have h2 := (norm_snd_le d).trans_lt hd
  rw [Real.norm_eq_abs] at h1 h2
  nlinarith [abs_lt.mp h1, abs_lt.mp h2, sq_abs d.1, sq_abs d.2]

theorem spA0_nonneg {d : ℝ × ℝ} (hd : ‖d‖ < 1 / 4) : 0 ≤ spA 0 d := by
  have := norm_small hd
  simp only [spA]
  have : Real.sqrt (1 - d.1 ^ 2 - d.2 ^ 2) ≤ 1 := by
    rw [Real.sqrt_le_one]; nlinarith [sq_nonneg d.1, sq_nonneg d.2]
  linarith

theorem spA0_pos {d : ℝ × ℝ} (hd : ‖d‖ < 1 / 4) (h0 : d ≠ 0) : 0 < spA 0 d := by
  have hs := norm_small hd
  have hp : 0 < d.1 ^ 2 + d.2 ^ 2 := by
    by_contra hn
    apply h0
    have h1 : d.1 = 0 := by nlinarith [sq_nonneg d.1, sq_nonneg d.2]
    have h2 : d.2 = 0 := by nlinarith [sq_nonneg d.1, sq_nonneg d.2]
    exact Prod.ext h1 h2
  simp only [spA]
  have : Real.sqrt (1 - d.1 ^ 2 - d.2 ^ 2) < 1 := by
    rw [Real.sqrt_lt' one_pos]; linarith
  linarith

/-- Off the centre, whether a fibre is empty does not depend on the point. -/
theorem fib_transfer {a b δ : ℝ} (ha : 0 < a) (hb : 0 < b) (hbδ : b < δ) {τ₀ τ₁ : Sgn}
    (h : ∃ u, |u| < δ ∧ sgnK (u - a) = τ₀ ∧ sgnK (u - 0) = τ₁) :
    ∃ u, |u| < δ ∧ sgnK (u - b) = τ₀ ∧ sgnK (u - 0) = τ₁ := by
  obtain ⟨u, hu, h0, h1⟩ := h
  simp only [sub_zero] at h1 ⊢
  have hδ := abs_lt.mp hu
  rcases lt_trichotomy u 0 with hu0 | hu0 | hu0
  · refine ⟨u, hu, ?_, h1⟩
    rw [← h0, sgnK_neg (by linarith), sgnK_neg (by linarith)]
  · subst hu0
    refine ⟨0, by simpa using hu, ?_, h1⟩
    rw [← h0, sgnK_neg (by linarith), sgnK_neg (by linarith)]
  · rcases lt_trichotomy (u - a) 0 with h2 | h2 | h2
    · refine ⟨b / 2, by rw [abs_lt]; constructor <;> linarith, ?_, ?_⟩
      · rw [← h0, sgnK_neg h2, sgnK_neg (by linarith)]
      · rw [← h1, sgnK_pos hu0, sgnK_pos (by linarith)]
    · refine ⟨b, by rw [abs_lt]; constructor <;> linarith, ?_, ?_⟩
      · rw [← h0, h2, sub_self]
      · rw [← h1, sgnK_pos hu0, sgnK_pos hb]
    · refine ⟨(b + δ) / 2, by rw [abs_lt]; constructor <;> linarith, ?_, ?_⟩
      · rw [← h0, sgnK_pos h2, sgnK_pos (by linarith)]
      · rw [← h1, sgnK_pos hu0, sgnK_pos (by linarith)]

/-- `(x, y, z) ↦ (z, x, y)`. -/
def spT : R3 ≃ₜ R3 where
  toFun p := (p.2.2, p.1, p.2.1)
  invFun q := (q.2.1, q.2.2, q.1)
  left_inv p := by simp
  right_inv q := by simp
  continuous_toFun := by fun_prop
  continuous_invFun := by fun_prop

/-- **Local connectivity at the contact point of a sphere and a plane.** -/
theorem spherePlane_localConn : LocalConn spherePlane (0, 0, 0) := by
  classical
  intro W hW hxW
  obtain ⟨δ1, hδ1, hball⟩ := Metric.isOpen_iff.mp hW _ hxW
  set δ := min δ1 (1 / 4)
  have hδ : 0 < δ := lt_min hδ1 (by norm_num)
  -- the cap stays below `δ` near the centre
  have hev : ∀ᶠ d in 𝓝 (0 : ℝ × ℝ), spA 0 d < δ :=
    (spA_cont 0).continuousAt.eventually_lt continuousAt_const (by simp [spA, hδ])
  obtain ⟨η1, hη1, hη⟩ := Metric.eventually_nhds_iff.mp hev
  set η := min η1 δ
  have hηpos : 0 < η := lt_min hη1 hδ
  have hηδ : η ≤ δ := min_le_right _ _
  have hδq : δ ≤ 1 / 4 := min_le_right _ _
  set U : Set R3 := {p | |p.2.2| < δ ∧ ‖((p.1, p.2.1) : ℝ × ℝ)‖ < η}
  have hUo : IsOpen U := by
    have h1 : IsOpen {p : R3 | |p.2.2| < δ} := isOpen_lt (by fun_prop) continuous_const
    have h2 : IsOpen {p : R3 | ‖((p.1, p.2.1) : ℝ × ℝ)‖ < η} := isOpen_lt (by fun_prop) continuous_const
    exact h1.inter h2
  refine ⟨U, hUo, fun p hp => hball ?_, ⟨by simp [hδ], by simp [hηpos]⟩, fun Y hY => ?_⟩
  · rw [Metric.mem_ball, show (((0 : ℝ), (0 : ℝ), (0 : ℝ)) : R3) = 0 from rfl, dist_zero_right,
      Prod.norm_def, Prod.norm_def]
    have h2 := hp.2
    rw [Prod.norm_def] at h2
    refine max_lt (lt_of_lt_of_le (lt_of_le_of_lt (le_max_left _ _) h2) (hηδ.trans
      (min_le_left _ _))) (max_lt (lt_of_lt_of_le (lt_of_le_of_lt (le_max_right _ _) h2)
      (hηδ.trans (min_le_left _ _))) ?_)
    rw [Real.norm_eq_abs]; exact lt_of_lt_of_le hp.1 (min_le_left _ _)
  -- the cell's two signs
  have hlen : Y.length = 2 := by
    have := length_eq_of_le hY; rw [length_covOfK] at this; simpa [spherePlane] using this.symm
  obtain ⟨Y0, Y1, rfl⟩ : ∃ Y0 Y1, Y = [Y0, Y1] := by
    match Y, hlen with
    | [a, b], _ => exact ⟨a, b, rfl⟩
  set σ : Fin 2 → Sgn := fun k => if k = 0 then fS true Y0 else Y1
  set D : Set (ℝ × ℝ) := {d | ‖d‖ < η ∧ ∃ u, |u| < δ ∧ ∀ k, sgnK (u - spA k d) = σ k}
  -- the sphere's sign, in the model
  have hsph : ∀ p ∈ U, sgnK (p.1 ^ 2 + p.2.1 ^ 2 + (p.2.2 - 1) ^ 2 - 1) =
      fS true (sgnK (p.2.2 - spA 0 (p.1, p.2.1))) := by
    intro p hp
    have hn := norm_small (lt_of_lt_of_le hp.2 (hηδ.trans hδq))
    simp only at hn
    set s := Real.sqrt (1 - p.1 ^ 2 - p.2.1 ^ 2)
    have hs : s ^ 2 = 1 - p.1 ^ 2 - p.2.1 ^ 2 := Real.sq_sqrt (by linarith)
    have hs0 : 0 ≤ s := Real.sqrt_nonneg _
    have hz := abs_lt.mp hp.1
    have : p.1 ^ 2 + p.2.1 ^ 2 + (p.2.2 - 1) ^ 2 - 1 = (p.2.2 - (1 - s)) * (p.2.2 - (1 + s)) := by
      nlinarith [hs]
    rw [this, sgnK_mul_neg (by linarith)]
    rfl
  have hcell : cell spherePlane [Y0, Y1] ∩ U = spT ⁻¹' graphCell spA D δ σ := by
    ext p
    constructor
    · rintro ⟨hpY, hpU⟩
      rw [mem_cell, covOfK_spherePlane] at hpY
      simp only [List.cons.injEq, and_true] at hpY
      have hsg : ∀ k, sgnK (p.2.2 - spA k (p.1, p.2.1)) = σ k := by
        intro k
        fin_cases k
        · show sgnK (p.2.2 - spA 0 (p.1, p.2.1)) = fS true Y0
          rw [← hpY.1, hsph p hpU, fS_fS]
        · show sgnK (p.2.2 - 0) = Y1
          rw [sub_zero, hpY.2]
      exact ⟨⟨hpU.2, p.2.2, hpU.1, hsg⟩, hpU.1, hsg⟩
    · rintro ⟨⟨hd, -⟩, hu, hsg⟩
      have hpU : p ∈ U := ⟨hu, hd⟩
      refine ⟨?_, hpU⟩
      rw [mem_cell, covOfK_spherePlane, hsph p hpU]
      have h0 := hsg 0
      have h1 := hsg 1
      simp only [σ, if_pos rfl] at h0
      simp only [σ, show (1 : Fin 2) ≠ 0 by decide, if_false] at h1
      change sgnK (p.2.2 - spA 0 (p.1, p.2.1)) = fS true Y0 at h0
      change sgnK (p.2.2 - 0) = Y1 at h1
      rw [h0, fS_fS, ← h1, sub_zero]
  rw [hcell, ← spT.image_symm]
  refine IsPreconnected.image ?_ _ spT.symm.continuous.continuousOn
  refine graphCell_preconnected' spA_cont ?_ σ fun d hd => hd.2
  -- the base: a disc, a punctured disc, the centre, or empty
  have hfib : ∀ d, ‖d‖ < η →
      ((∃ u, |u| < δ ∧ ∀ k, sgnK (u - spA k d) = σ k) ↔
        (∃ u, |u| < δ ∧ sgnK (u - spA 0 d) = σ 0 ∧ sgnK (u - 0) = σ 1)) := by
    intro d _
    constructor
    · rintro ⟨u, hu, h⟩; exact ⟨u, hu, h 0, h 1⟩
    · rintro ⟨u, hu, h0, h1⟩; exact ⟨u, hu, fun k => by fin_cases k; exact h0; exact h1⟩
  by_cases hnz : ∃ d ∈ D, d ≠ 0
  · obtain ⟨d1, hd1, hd10⟩ := hnz
    have hη4 : η ≤ 1 / 4 := hηδ.trans hδq
    have hall : Metric.ball (0 : ℝ × ℝ) η \ {0} ⊆ D := by
      rintro d ⟨hd, hd0⟩
      rw [Metric.mem_ball, dist_zero_right] at hd
      refine ⟨hd, (hfib d hd).mpr ?_⟩
      have hd1' := (hfib d1 hd1.1).mp hd1.2
      exact fib_transfer (spA0_pos (lt_of_lt_of_le hd1.1 hη4) hd10)
        (spA0_pos (lt_of_lt_of_le hd hη4) hd0)
        (@hη d (by rw [dist_zero_right]; exact lt_of_lt_of_le hd (min_le_left _ _))) hd1'
    by_cases h0 : (0 : ℝ × ℝ) ∈ D
    · have : D = Metric.ball 0 η := by
        ext d
        constructor
        · intro hd; rw [Metric.mem_ball, dist_zero_right]; exact hd.1
        · intro hd
          by_cases hd0 : d = 0
          · rw [hd0]; exact h0
          · exact hall ⟨hd, hd0⟩
      rw [this]; exact (convex_ball _ _).isPreconnected
    · have : D = Metric.ball 0 η \ {0} := by
        ext d
        constructor
        · intro hd
          refine ⟨by rw [Metric.mem_ball, dist_zero_right]; exact hd.1, fun e => h0 ?_⟩
          rw [mem_singleton_iff] at e; rw [← e]; exact hd
        · intro hd; exact hall hd
      rw [this]; exact punctured_preconnected hηpos
  · push Not at hnz
    exact (Set.subsingleton_of_forall_eq 0 fun d hd => hnz d hd).isPreconnected

/-! ## The vert: two sectors, no bounding edges -/

/-- The neighbourhood of the contact point used. -/
def spW : Set R3 := {p | |p.1| < 1 / 4 ∧ |p.2.1| < 1 / 4 ∧ |p.2.2| < 1 / 4}

theorem isOpen_spW : IsOpen spW := by
  have h1 : IsOpen {p : R3 | |p.1| < 1 / 4} := isOpen_lt (by fun_prop) continuous_const
  have h2 : IsOpen {p : R3 | |p.2.1| < 1 / 4} := isOpen_lt (by fun_prop) continuous_const
  have h3 : IsOpen {p : R3 | |p.2.2| < 1 / 4} := isOpen_lt (by fun_prop) continuous_const
  exact h1.inter (h2.inter h3)

/-- The two sectors' covectors: the sphere above the plane, the plane outside the sphere. -/
def spWings : List Cov := [[.zero, .pos], [.pos, .zero]]

theorem sp_on_sphere {y : R3} (h : y.1 ^ 2 + y.2.1 ^ 2 + (y.2.2 - 1) ^ 2 - 1 = 0) (hy : y ≠ 0) :
    covOfK spherePlane y = [.zero, .pos] := by
  have hz : 0 < y.2.2 := by
    by_contra hn
    push Not at hn
    apply hy
    have h3 : y.2.2 = 0 := by nlinarith [sq_nonneg y.1, sq_nonneg y.2.1]
    have h1 : y.1 = 0 := by nlinarith [sq_nonneg y.1, sq_nonneg y.2.1]
    have h2 : y.2.1 = 0 := by nlinarith [sq_nonneg y.1, sq_nonneg y.2.1]
    exact Prod.ext h1 (Prod.ext h2 h3)
  rw [covOfK_spherePlane, h, sgnK_pos hz]; simp [sgnK]

theorem sp_on_plane {y : R3} (h : y.2.2 = 0) (hy : y ≠ 0) :
    covOfK spherePlane y = [.pos, .zero] := by
  have hp : 0 < y.1 ^ 2 + y.2.1 ^ 2 := by
    by_contra hn
    apply hy
    have h1 : y.1 = 0 := by nlinarith [sq_nonneg y.1, sq_nonneg y.2.1]
    have h2 : y.2.1 = 0 := by nlinarith [sq_nonneg y.1, sq_nonneg y.2.1]
    exact Prod.ext h1 (Prod.ext h2 h)
  rw [covOfK_spherePlane, h, sgnK_pos (by nlinarith)]; simp [sgnK]

theorem sp_near : ∀ φ ∈ spWings, ((0 : ℝ), (0 : ℝ), (0 : ℝ)) ∈ closure (cell spherePlane φ) := by
  intro φ hφ
  simp only [spWings, List.mem_cons, List.not_mem_nil, or_false] at hφ
  rcases hφ with rfl | rfl
  · -- along the sphere
    set γ : ℝ → R3 := fun t => (t, 0, 1 - Real.sqrt (1 - t ^ 2))
    have hc : Continuous γ := by simp only [γ]; fun_prop
    have h0 : γ 0 = (0, 0, 0) := by simp [γ]
    rw [← h0]
    refine mem_closure_of_tendsto ((hc.tendsto 0).mono_left (nhdsWithin_le_nhds (s := Ioi 0))) ?_
    filter_upwards [Ioo_mem_nhdsGT (by norm_num : (0 : ℝ) < 1 / 2)] with t ht
    have ht2 : t ^ 2 < 1 := by nlinarith [ht.1, ht.2]
    have hs : Real.sqrt (1 - t ^ 2) ^ 2 = 1 - t ^ 2 := Real.sq_sqrt (by linarith)
    refine sp_on_sphere ?_ (fun e => by have := congrArg Prod.fst e; simp [γ] at this; linarith [ht.1])
    simp only [γ]; nlinarith [hs]
  · -- along the plane
    set γ : ℝ → R3 := fun t => (t, 0, 0)
    have hc : Continuous γ := by simp only [γ]; fun_prop
    have h0 : γ 0 = (0, 0, 0) := rfl
    rw [← h0]
    refine mem_closure_of_tendsto ((hc.tendsto 0).mono_left (nhdsWithin_le_nhds (s := Ioi 0))) ?_
    filter_upwards [self_mem_nhdsWithin] with t (ht : 0 < t)
    exact sp_on_plane rfl (fun e => by have := congrArg Prod.fst e; simp [γ] at this; linarith)

/-- **The local model at the contact point**: sectors `(f_s, [])` and `(f_p, [])`, no edges. -/
noncomputable def spModel {G : FaceGeo R3} {fs fp : Nat}
    (hvF : ((0 : ℝ), (0 : ℝ), (0 : ℝ)) ∉ FacetPts G) (hvE : ((0 : ℝ), (0 : ℝ), (0 : ℝ)) ∉ EdgePts G)
    (hF : ∀ y ∈ spW, y ∈ FacetPts G →
      (y.1 ^ 2 + y.2.1 ^ 2 + (y.2.2 - 1) ^ 2 - 1 = 0 ∨ y.2.2 = 0) ∧ y ≠ 0)
    (hE : ∀ y ∈ spW, ∀ a, y ∉ G.edge a)
    (h₁ : ∀ y ∈ spW, y.1 ^ 2 + y.2.1 ^ 2 + (y.2.2 - 1) ^ 2 - 1 = 0 → y ≠ 0 → y ∈ G.facet fs)
    (h₂ : ∀ y ∈ spW, y.2.2 = 0 → y ≠ 0 → y ∈ G.facet fp) :
    VertModel G spherePlane [(fs, []), (fp, [])] (0, 0, 0) where
  W := spW
  hWo := isOpen_spW
  hvW := by norm_num [spW]
  v_facet := hvF
  v_edge := hvE
  Φ := spWings
  len := rfl
  nodup := by decide
  facet := by decide
  above := by
    have : covOfK spherePlane ((0 : ℝ), (0 : ℝ), (0 : ℝ)) = [.zero, .zero] := by
      rw [covOfK_spherePlane]; norm_num [sgnK]
    rw [this]; decide
  near := sp_near
  sub i hi := by
    rintro y ⟨hy, hyW⟩
    rw [mem_cell] at hy
    simp only [spWings, List.length_cons, List.length_nil] at hi
    rw [covOfK_spherePlane] at hy
    interval_cases i <;>
      simp only [spWings, List.getElem_cons_zero, List.getElem_cons_succ, List.cons.injEq,
        and_true] at hy
    · have hy0 : y ≠ 0 := fun e => by rw [e] at hy; simp [sgnK] at hy
      exact h₁ y hyW (sgnK_zero_iff.mp hy.1) hy0
    · have hy0 : y ≠ 0 := fun e => by rw [e] at hy; norm_num [sgnK] at hy
      exact h₂ y hyW (sgnK_zero_iff.mp hy.2) hy0
  complete y hyW hyF := by
    obtain ⟨h | h, hy0⟩ := hF y hyW hyF
    · rw [sp_on_sphere h hy0]; simp [spWings]
    · rw [sp_on_plane h hy0]; simp [spWings]
  E := []
  ea := id
  edge2 := by simp
  eabove := by simp
  enear := by simp
  esub j h := absurd h (by simp)
  esharp y hy a ha := absurd ha (hE y hy a)
  ea_inj j k hj := absurd hj (by simp)
  bounds_of_le i _ j hj := absurd hj (by simp)
  le_of_bounds i hi a hb := by
    simp only [spWings, List.length_cons, List.length_nil] at hi
    interval_cases i <;> simp [Sec.bounds] at hb
  conn := spherePlane_localConn
  inc y _ i _ j hj := absurd hj (by simp)

theorem spherePlane_cont : ∀ c ∈ spherePlane, Continuous c := by
  intro c hc
  simp only [spherePlane, List.mem_cons, List.not_mem_nil, or_false] at hc
  rcases hc with rfl | rfl <;> fun_prop

/-- **F1–F4 at the contact point of a sphere and a plane.** -/
theorem spherePlane_vertLocal {G : FaceGeo R3} {fs fp : Nat}
    (hvF : ((0 : ℝ), (0 : ℝ), (0 : ℝ)) ∉ FacetPts G) (hvE : ((0 : ℝ), (0 : ℝ), (0 : ℝ)) ∉ EdgePts G)
    (hF : ∀ y ∈ spW, y ∈ FacetPts G →
      (y.1 ^ 2 + y.2.1 ^ 2 + (y.2.2 - 1) ^ 2 - 1 = 0 ∨ y.2.2 = 0) ∧ y ≠ 0)
    (hE : ∀ y ∈ spW, ∀ a, y ∉ G.edge a)
    (h₁ : ∀ y ∈ spW, y.1 ^ 2 + y.2.1 ^ 2 + (y.2.2 - 1) ^ 2 - 1 = 0 → y ≠ 0 → y ∈ G.facet fs)
    (h₂ : ∀ y ∈ spW, y.2.2 = 0 → y ≠ 0 → y ∈ G.facet fp) :
    Nonempty (VertLocal G [(fs, []), (fp, [])] (0, 0, 0)) :=
  vertLocal_of_model spherePlane_cont (spModel hvF hvE hF hE h₁ h₂)

/-- **The vert is refused iff both facets bound the result** (two sectors sharing no edge are two
fans), and then the result has two local fans there. -/
theorem spherePlane_bad_iff {keep : Nat → Bool} (G : FaceGeo R3) (fs fp : Nat) :
    BadVertS G keep [(fs, []), (fp, [])] ↔ Bd G keep fs ∧ Bd G keep fp := by
  have hno : ∀ i j, ¬FanRel G keep [(fs, []), (fp, [])] i j := by
    rintro i j ⟨hi, hj, -, -, a, ha, -⟩
    simp only [List.length_cons, List.length_nil] at hi
    interval_cases i <;> simp [Sec.bounds] at ha
  have hrt : ∀ i j, Relation.ReflTransGen (FanRel G keep [(fs, []), (fp, [])]) i j → i = j := by
    intro i j h
    induction h with
    | refl => rfl
    | tail _ h _ => exact absurd h (hno _ _)
  constructor
  · rintro ⟨i, j, hi, hj, bi, bj, hn⟩
    simp only [List.length_cons, List.length_nil] at hi hj
    interval_cases i <;> interval_cases j
    · exact absurd Relation.ReflTransGen.refl hn
    · exact ⟨bi, bj⟩
    · exact ⟨bj, bi⟩
    · exact absurd Relation.ReflTransGen.refl hn
  · rintro ⟨b1, b2⟩
    exact ⟨0, 1, by simp, by simp, b1, b2, fun h => absurd (hrt _ _ h) (by decide)⟩

end ForgeCurved
