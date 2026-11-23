import ForgeCurved.QLineStage

/-!
# On a sample plane, a facet point moves off the plane's events

The first step of the in-plane stage. A facet point on the sample plane `x = c` may sit on one of
the plane's `y`-events. Its carrier's trace there has a nonzero gradient `(∂_y q, ∂_z q)`, since
the plane is off the sweep's events. So the trace is a curve through it:

* `∂_z q ≠ 0`: a graph over `y` (LemmaD's `chart`, in `ℝ²`). A nearby level off the events has
  a point of the curve, still in the cell;
* `∂_z q = 0`: a graph `y = h(z)`. `h` is not constant near the point (`no_vertical`), so by the
  intermediate value theorem it takes a value off the events nearby.

* **`plane_offY`**.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set Topology Filter

/-- The open set where every carrier but `j` keeps its sign at `w0`. -/
theorem signs_nhds {qs : List Quad.Quadric} {c : ℝ} {j : ℕ} {w0 : ℝ × ℝ}
    (hne : ∀ m (hm : m < qs.length), m ≠ j → onPlane c qs[m] w0 ≠ 0) :
    {w | ∀ m (hm : m < qs.length), m ≠ j → sgnK (onPlane c qs[m] w) = sgnK (onPlane c qs[m] w0)}
      ∈ 𝓝 w0 := by
  have : {w | ∀ m (hm : m < qs.length), m ≠ j →
      sgnK (onPlane c qs[m] w) = sgnK (onPlane c qs[m] w0)} ⊇
      ⋂ i : Fin qs.length, {w | i.val ≠ j → sgnK (onPlane c qs[i] w) = sgnK (onPlane c qs[i] w0)} := by
    intro w hw m hm hmj
    exact (mem_iInter.mp hw ⟨m, hm⟩) hmj
  refine Filter.mem_of_superset ((Filter.iInter_mem).mpr fun i => ?_) this
  by_cases hij : i.val = j
  · simp [hij]
  · have h0 := hne i i.2 hij
    have hcont := (onPlane_contDiff c qs[i]).continuous
    rcases lt_or_gt_of_ne h0 with hlt | hgt
    · filter_upwards [hcont.continuousAt.eventually (gt_mem_nhds hlt)] with w hw _
      rw [sgnK_neg hw]; exact (sgnK_neg (show onPlane c qs[i] w0 < 0 from hlt)).symm
    · filter_upwards [hcont.continuousAt.eventually (lt_mem_nhds hgt)] with w hw _
      rw [sgnK_pos hw]; exact (sgnK_pos (show 0 < onPlane c qs[i] w0 from hgt)).symm

/-- **A facet point on a sample plane moves off the plane's events, in the cell.** -/
theorem plane_offY {qs : List Quad.Quadric} (hf : frameOk qs = true) {c : ℝ} {j : ℕ}
    (hj : j < qs.length) (Ey : List Num) {w0 : ℝ × ℝ} (h0 : onPlane c qs[j] w0 = 0)
    (hne : ∀ m (hm : m < qs.length), m ≠ j → onPlane c qs[m] w0 ≠ 0)
    (hg : gy qs[j] (emb c w0) ≠ 0 ∨ gz qs[j] (emb c w0) ≠ 0) :
    ∃ w, onPlane c qs[j] w = 0 ∧
      (∀ m (hm : m < qs.length), m ≠ j → sgnK (onPlane c qs[m] w) = sgnK (onPlane c qs[m] w0)) ∧
      ∀ e ∈ Ey, ¬ Denotes realModel e w.1 := by
  set g := onPlane c qs[j]
  have hgC : ∀ f ∈ [g], ContDiff ℝ 1 f := fun f hf => by
    simp only [List.mem_singleton] at hf; subst hf; exact onPlane_contDiff _ _
  have hz0 : ∀ f ∈ [g], f w0 = 0 := fun f hf => by
    simp only [List.mem_singleton] at hf; subst hf; exact h0
  have hN := signs_nhds (j := j) hne
  by_cases hgz : gz qs[j] (emb c w0) ≠ 0
  · -- a graph over `y`
    obtain ⟨U, hU, hw0U, ε, hε, γ, hγc, hγ, huniq⟩ :=
      chart hgC (finrank_R2' g) hz0 (transverseY c qs[j] w0 hgz)
    have hγ0 : γ (ℓy w0) = w0 := ((huniq w0 hw0U hz0).2).symm
    have hcat : ContinuousAt γ (ℓy w0) :=
      hγc.continuousAt (Ioo_mem_nhds (by linarith) (by linarith))
    have hpre : γ ⁻¹' {w | ∀ m (hm : m < qs.length), m ≠ j →
        sgnK (onPlane c qs[m] w) = sgnK (onPlane c qs[m] w0)} ∈ 𝓝 (ℓy w0) :=
      hcat.preimage_mem_nhds (by rw [hγ0]; exact hN)
    obtain ⟨δ, hδ, hball⟩ := Metric.mem_nhds_iff.mp (Filter.inter_mem hpre
      (Ioo_mem_nhds (show ℓy w0 - ε < ℓy w0 by linarith) (show ℓy w0 < ℓy w0 + ε by linarith)))
    obtain ⟨t, ht, hoff⟩ := avoid Ey (show ℓy w0 - δ < ℓy w0 + δ by linarith)
    have htb : t ∈ Metric.ball (ℓy w0) δ := by
      rw [Metric.mem_ball, Real.dist_eq, abs_sub_lt_iff]; constructor <;> linarith [ht.1, ht.2]
    obtain ⟨hts, hti⟩ := hball htb
    obtain ⟨hl, hz⟩ := hγ t hti
    refine ⟨γ t, hz g (List.mem_singleton_self _), hts, ?_⟩
    have : (γ t).1 = t := hl
    rw [this]; exact hoff
  · -- a graph over `z`
    push Not at hgz
    have hgy : gy qs[j] (emb c w0) ≠ 0 := by
      rcases hg with h | h
      · exact h
      · exact absurd hgz h
    obtain ⟨U, hU, hw0U, ε, hε, γ, hγc, hγ, huniq⟩ :=
      chart hgC (finrank_R2' g) hz0 (transverseZ c qs[j] w0 hgy)
    have hγ0 : γ (ℓz w0) = w0 := ((huniq w0 hw0U hz0).2).symm
    have hcat : ContinuousAt γ (ℓz w0) :=
      hγc.continuousAt (Ioo_mem_nhds (by linarith) (by linarith))
    have hpre : γ ⁻¹' {w | ∀ m (hm : m < qs.length), m ≠ j →
        sgnK (onPlane c qs[m] w) = sgnK (onPlane c qs[m] w0)} ∈ 𝓝 (ℓz w0) :=
      hcat.preimage_mem_nhds (by rw [hγ0]; exact hN)
    obtain ⟨δ, hδ, hball⟩ := Metric.mem_nhds_iff.mp (Filter.inter_mem hpre
      (Ioo_mem_nhds (show ℓz w0 - ε < ℓz w0 by linarith) (show ℓz w0 < ℓz w0 + ε by linarith)))
    set I := Ioo (ℓz w0 - δ) (ℓz w0 + δ)
    have hI : ∀ t ∈ I, t ∈ Metric.ball (ℓz w0) δ := fun t ht => by
      rw [Metric.mem_ball, Real.dist_eq, abs_sub_lt_iff]; constructor <;> linarith [ht.1, ht.2]
    -- `h = (γ ·).1` is not constant on `I`
    obtain ⟨t1, ht1, hne1⟩ : ∃ t1 ∈ I, (γ t1).1 ≠ w0.1 := by
      by_contra hall
      push Not at hall
      apply no_vertical hf (List.getElem_mem hj) c w0.1 (show ℓz w0 - δ < ℓz w0 + δ by linarith)
      intro z hz
      obtain ⟨hl, hzz⟩ := hγ z (hball (hI z hz)).2
      have hpt : γ z = (w0.1, z) := Prod.ext (hall z hz) hl
      rw [← hpt]; exact hzz g (List.mem_singleton_self _)
    -- the values between `w0.1` and `h t1` are taken on `[w0.2, t1]`, inside `I`
    have hsub : uIcc (ℓz w0) t1 ⊆ I := by
      intro t ht
      rcases le_total (ℓz w0) t1 with hle | hle
      · rw [uIcc_of_le hle] at ht
        exact ⟨by linarith [ht.1], lt_of_le_of_lt ht.2 ht1.2⟩
      · rw [uIcc_of_ge hle] at ht
        exact ⟨lt_of_lt_of_le ht1.1 ht.1, by linarith [ht.2]⟩
    have hcont : ContinuousOn (fun t => (γ t).1) (uIcc (ℓz w0) t1) :=
      (continuous_fst.comp_continuousOn hγc).mono (fun t ht => (hball (hI t (hsub ht))).2)
    have hiv := intermediate_value_uIcc hcont
    have hγ0' : (γ (ℓz w0)).1 = w0.1 := by rw [hγ0]
    obtain ⟨v, hv, hoff⟩ := avoid Ey (a := min w0.1 (γ t1).1) (b := max w0.1 (γ t1).1)
      (by rcases lt_or_gt_of_ne hne1 with h | h
          · rw [min_eq_right h.le, max_eq_left h.le]; exact h
          · rw [min_eq_left h.le, max_eq_right h.le]; exact h)
    obtain ⟨t, ht, htv⟩ := hiv (by rw [hγ0']; exact Ioo_subset_Icc_self hv)
    obtain ⟨hts, hti⟩ := hball (hI t (hsub ht))
    obtain ⟨-, hz⟩ := hγ t hti
    refine ⟨γ t, hz g (List.mem_singleton_self _), hts, ?_⟩
    simp only at htv
    rw [htv]; exact hoff

end ForgeCurved
