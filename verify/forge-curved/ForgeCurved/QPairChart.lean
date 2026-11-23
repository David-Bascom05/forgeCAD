import ForgeCurved.QExact

/-!
# A transverse point of two carriers lies on a curve of its cell

At a point `p` of `q_j = q_k = 0` with `critQ ≠ 0` (the pair transverse to the sweep), the common
zeros near `p` are a graph over `x` (LemmaD's `chart`, with `ks = [q_j, q_k]`). The other carriers
keep their signs near `p`. So the cell of `p` has points at every level near `p.1`, among them one
off any finite list of events and other than `p.1` (**`pair_moves`**).

The edges and verts fields read this twice. A two-zero vertex candidate's points are all turning
points, and a point cell with two zeros is a turning point.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set Topology Filter

/-- **A transverse point of two carriers moves, in its cell, to a level off the events.** -/
theorem pair_moves {qs : List Quad.Quadric} {j k : ℕ} (hj : j < qs.length) (hk : k < qs.length)
    (hjk : j ≠ k) (E : List Num) {p : R3} (h0j : qR qs[j] p = 0) (h0k : qR qs[k] p = 0)
    (hne : ∀ m (hm : m < qs.length), m ≠ j → m ≠ k → qR qs[m] p ≠ 0)
    (ht : gy qs[j] p * gz qs[k] p - gz qs[j] p * gy qs[k] p ≠ 0) :
    ∃ p', p'.1 ≠ p.1 ∧ (∀ e ∈ E, ¬ Denotes realModel e p'.1) ∧
      covOfK (qs.map qR) p' = covOfK (qs.map qR) p := by
  have hC : ∀ c ∈ [qR qs[j], qR qs[k]], ContDiff ℝ 1 c := by
    intro c hc
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hc
    rcases hc with rfl | rfl <;> exact qR_contDiff _
  have hz : ∀ c ∈ [qR qs[j], qR qs[k]], c p = 0 := by
    intro c hc
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hc
    rcases hc with rfl | rfl
    · exact h0j
    · exact h0k
  obtain ⟨U, hU, hpU, ε, hε, γ, hγc, hγ, huniq⟩ :=
    chart hC (by rw [finrank_R3]; rfl) hz (transverse2 qs[j] qs[k] p ht)
  have hγ0 : γ (ℓx p) = p := ((huniq p hpU hz).2).symm
  have hcat : ContinuousAt γ (ℓx p) := hγc.continuousAt (Ioo_mem_nhds (by linarith) (by linarith))
  -- the other carriers keep their signs near `p`
  have hN : {z | ∀ m (hm : m < qs.length), m ≠ j → m ≠ k →
      sgnK (qR qs[m] z) = sgnK (qR qs[m] p)} ∈ 𝓝 p := by
    have : {z | ∀ m (hm : m < qs.length), m ≠ j → m ≠ k →
        sgnK (qR qs[m] z) = sgnK (qR qs[m] p)} ⊇
        ⋂ i : Fin qs.length, {z | i.val ≠ j → i.val ≠ k → sgnK (qR qs[i] z) = sgnK (qR qs[i] p)} := by
      intro z hz m hm hmj hmk
      exact (mem_iInter.mp hz ⟨m, hm⟩) hmj hmk
    refine Filter.mem_of_superset ((Filter.iInter_mem).mpr fun i => ?_) this
    by_cases hij : i.val = j ∨ i.val = k
    · rcases hij with h | h <;> simp [h]
    · push Not at hij
      have h0 := hne i i.2 hij.1 hij.2
      have hcont := qR_cont qs[i]
      rcases lt_or_gt_of_ne h0 with hlt | hgt
      · filter_upwards [hcont.continuousAt.eventually (gt_mem_nhds hlt)] with z hz _ _
        rw [sgnK_neg hz]; exact (sgnK_neg (show qR qs[i] p < 0 from hlt)).symm
      · filter_upwards [hcont.continuousAt.eventually (lt_mem_nhds hgt)] with z hz _ _
        rw [sgnK_pos hz]; exact (sgnK_pos (show 0 < qR qs[i] p from hgt)).symm
  have hpre := hcat.preimage_mem_nhds (by rw [hγ0]; exact hN)
  obtain ⟨δ, hδ, hball⟩ := Metric.mem_nhds_iff.mp (Filter.inter_mem hpre
    (Ioo_mem_nhds (show ℓx p - ε < ℓx p by linarith) (show ℓx p < ℓx p + ε by linarith)))
  obtain ⟨t, ht', hoff⟩ := avoid E (show ℓx p < ℓx p + δ by linarith)
  have htb : t ∈ Metric.ball (ℓx p) δ := by
    rw [Metric.mem_ball, Real.dist_eq, abs_sub_lt_iff]; constructor <;> linarith [ht'.1, ht'.2]
  obtain ⟨hts, hti⟩ := hball htb
  obtain ⟨hl, hzz⟩ := hγ t hti
  have hl' : (γ t).1 = t := hl
  refine ⟨γ t, by rw [hl']; exact ne_of_gt ht'.1, by rw [hl']; exact hoff, ?_⟩
  apply List.ext_getElem?
  intro m
  rw [getElem?_covOfK, getElem?_covOfK]
  by_cases hm : m < qs.length
  · simp only [List.getElem?_map, List.getElem?_eq_getElem hm, Option.map_some]
    by_cases hmj : m = j
    · subst hmj
      rw [hzz (qR qs[m]) (by simp), h0j]
    by_cases hmk : m = k
    · subst hmk
      rw [hzz (qR qs[m]) (by simp), h0k]
    rw [hts m hm hmj hmk]
  · simp [List.getElem?_eq_none (not_lt.mp hm)]

end ForgeCurved
