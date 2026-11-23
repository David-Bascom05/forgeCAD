import ForgeCurved.QOffY

/-!
# A facet point moves off the sweep's events

The facet stage's start. A facet cell's point may sit at an event level `x = e`. Its carrier is
smooth there (`singAll`), and the other carriers keep their signs near it. So:

* `(∂_y q, ∂_z q) ≠ 0`: on every nearby level `x'` the carrier has a zero near the point, by the
  intermediate value theorem along `(0, ∂_y q, ∂_z q)`. One such level is off the events;
* only `∂_x q ≠ 0`: on every nearby vertical line the carrier has a zero near the point, by the
  IVT along `x`. If all of them were at the point's level, the trace on that plane would hold a
  vertical segment (`no_vertical`).

* **`facet_offX`**.
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set Topology Filter

/-- The quadratic part. -/
def qq (q : Quad.Quadric) (v : R3) : ℝ :=
  q.xx * v.1 * v.1 + q.yy * v.2.1 * v.2.1 + q.zz * v.2.2 * v.2.2 + q.xy * v.1 * v.2.1 +
    q.xz * v.1 * v.2.2 + q.yz * v.2.1 * v.2.2

theorem qR_line (q : Quad.Quadric) (p v : R3) (s : ℝ) :
    qR q (p.1 + s * v.1, p.2.1 + s * v.2.1, p.2.2 + s * v.2.2) =
      qR q p + s * (gx q p * v.1 + gy q p * v.2.1 + gz q p * v.2.2) + s * s * qq q v := by
  simp only [qR, gx, gy, gz, qq]; ring

/-- The open set where every carrier but `j` keeps its sign at `p`. -/
theorem signs_nhds3 {qs : List Quad.Quadric} {j : ℕ} {p : R3}
    (hne : ∀ m (hm : m < qs.length), m ≠ j → qR qs[m] p ≠ 0) :
    {z | ∀ m (hm : m < qs.length), m ≠ j → sgnK (qR qs[m] z) = sgnK (qR qs[m] p)} ∈ 𝓝 p := by
  have : {z | ∀ m (hm : m < qs.length), m ≠ j → sgnK (qR qs[m] z) = sgnK (qR qs[m] p)} ⊇
      ⋂ i : Fin qs.length, {z | i.val ≠ j → sgnK (qR qs[i] z) = sgnK (qR qs[i] p)} := by
    intro z hz m hm hmj
    exact (mem_iInter.mp hz ⟨m, hm⟩) hmj
  refine Filter.mem_of_superset ((Filter.iInter_mem).mpr fun i => ?_) this
  by_cases hij : i.val = j
  · simp [hij]
  · have h0 := hne i i.2 hij
    have hcont := qR_cont qs[i]
    rcases lt_or_gt_of_ne h0 with hlt | hgt
    · filter_upwards [hcont.continuousAt.eventually (gt_mem_nhds hlt)] with z hz _
      rw [sgnK_neg hz]; exact (sgnK_neg (show qR qs[i] p < 0 from hlt)).symm
    · filter_upwards [hcont.continuousAt.eventually (lt_mem_nhds hgt)] with z hz _
      rw [sgnK_pos hz]; exact (sgnK_pos (show 0 < qR qs[i] p from hgt)).symm

/-- A nonzero first-order term wins for small `s`. -/
theorem small_sign {a b : ℝ} (ha : 0 < a) : ∃ s0 > 0, ∀ s, 0 < s → s ≤ s0 →
    0 < s * a + s * s * b ∧ s * (-a) + s * s * b < 0 := by
  refine ⟨a / (2 * (|b| + 1)), by positivity, fun s hs hs0 => ?_⟩
  have hb : s * |b| ≤ a / 2 := by
    calc s * |b| ≤ a / (2 * (|b| + 1)) * |b| := mul_le_mul_of_nonneg_right hs0 (abs_nonneg b)
      _ ≤ a / 2 := by
        rw [div_mul_eq_mul_div, div_le_div_iff₀ (by positivity) (by norm_num)]
        nlinarith [abs_nonneg b]
  have h1 : -(s * |b|) ≤ s * b := by nlinarith [neg_abs_le b]
  have h2 : s * b ≤ s * |b| := by nlinarith [le_abs_self b]
  constructor <;> nlinarith

/-- Finitely many events leave a punctured interval round any level free. -/
theorem isolate (E : List Num) (x : ℝ) : ∃ η > 0, ∀ e ∈ E, ∀ v, Denotes realModel e v →
    v = x ∨ η ≤ |v - x| := by
  induction E with
  | nil => exact ⟨1, one_pos, fun e he => by simp at he⟩
  | cons e E ih =>
    obtain ⟨η, hη, hE⟩ := ih
    by_cases hx : valE e = x
    · refine ⟨η, hη, fun e' he' v hv => ?_⟩
      rcases List.mem_cons.mp he' with rfl | he'
      · left; rw [valE_eq hv]; exact hx
      · exact hE e' he' v hv
    · have hpos : 0 < |valE e - x| := abs_pos.mpr (sub_ne_zero.mpr hx)
      refine ⟨min η |valE e - x|, lt_min hη hpos, fun e' he' v hv => ?_⟩
      rcases List.mem_cons.mp he' with rfl | he'
      · right; rw [valE_eq hv]; exact min_le_right _ _
      · rcases hE e' he' v hv with h | h
        · left; exact h
        · right; exact _root_.le_trans (min_le_left _ _) h

/-- **A facet point moves off the sweep's events, in its cell.** -/
theorem facet_offX {qs : List Quad.Quadric} (hf : frameOk qs = true) {j : ℕ} (hj : j < qs.length)
    (E : List Num) {p : R3} (h0 : qR qs[j] p = 0)
    (hne : ∀ m (hm : m < qs.length), m ≠ j → qR qs[m] p ≠ 0)
    (hg : gx qs[j] p ≠ 0 ∨ gy qs[j] p ≠ 0 ∨ gz qs[j] p ≠ 0) :
    ∃ p', qR qs[j] p' = 0 ∧
      (∀ m (hm : m < qs.length), m ≠ j → sgnK (qR qs[m] p') = sgnK (qR qs[m] p)) ∧
      ∀ e ∈ E, ¬ Denotes realModel e p'.1 := by
  set q := qs[j]
  obtain ⟨r, hr, hball⟩ := Metric.mem_nhds_iff.mp (signs_nhds3 (j := j) hne)
  by_cases hyz : gy q p ≠ 0 ∨ gz q p ≠ 0
  · -- along `d = (0, ∂_y q, ∂_z q)`, on nearby levels
    set a := gy q p * gy q p + gz q p * gz q p
    have ha : 0 < a := by
      rcases hyz with h | h
      · have := mul_self_pos.mpr h; nlinarith [mul_self_nonneg (gz q p)]
      · have := mul_self_pos.mpr h; nlinarith [mul_self_nonneg (gy q p)]
    set d : R3 := (0, gy q p, gz q p)
    obtain ⟨s0, hs0, hsg⟩ := small_sign (b := qq q d) ha
    -- keep the segment inside the ball
    set s1 := min s0 (r / (2 * (|gy q p| + |gz q p| + 1)))
    have hs1 : 0 < s1 := lt_min hs0 (by positivity)
    obtain ⟨hpos, hneg⟩ := hsg s1 hs1 (min_le_left _ _)
    have hval : ∀ s, qR q (p.1 + s * d.1, p.2.1 + s * d.2.1, p.2.2 + s * d.2.2) =
        s * a + s * s * qq q d := fun s => by
      rw [qR_line, h0]; simp only [d]; ring
    have hpos' : 0 < qR q (p.1, p.2.1 + s1 * gy q p, p.2.2 + s1 * gz q p) := by
      have := hval s1; simp only [d, mul_zero, add_zero] at this; rw [this]; exact hpos
    have hneg' : qR q (p.1, p.2.1 + (-s1) * gy q p, p.2.2 + (-s1) * gz q p) < 0 := by
      have := hval (-s1); simp only [d, mul_zero, add_zero] at this; rw [this]
      have : -s1 * a + -s1 * -s1 * qq q d = s1 * (-a) + s1 * s1 * qq q d := by ring
      rw [this]; exact hneg
    -- on nearby levels the signs persist
    have hc1 : ContinuousAt (fun x' => qR q (x', p.2.1 + s1 * gy q p, p.2.2 + s1 * gz q p)) p.1 :=
      ((qR_cont q).comp (by fun_prop)).continuousAt
    have hc2 : ContinuousAt (fun x' => qR q (x', p.2.1 + (-s1) * gy q p, p.2.2 + (-s1) * gz q p)) p.1 :=
      ((qR_cont q).comp (by fun_prop)).continuousAt
    obtain ⟨δ, hδ, hδb⟩ := Metric.mem_nhds_iff.mp (Filter.inter_mem
      (hc1.eventually (lt_mem_nhds hpos')) (hc2.eventually (gt_mem_nhds hneg')))
    set δ' := min δ (r / 2)
    obtain ⟨x', hx', hoff⟩ := avoid E (show p.1 - δ' < p.1 + δ' by
      have : 0 < δ' := lt_min hδ (by linarith); linarith)
    have hx'b : x' ∈ Metric.ball p.1 δ := by
      rw [Metric.mem_ball, Real.dist_eq, abs_sub_lt_iff]
      constructor <;> linarith [hx'.1, hx'.2, min_le_left δ (r / 2)]
    obtain ⟨hp1, hn1⟩ := hδb hx'b
    -- the IVT along the segment
    have hcs : ContinuousOn (fun s => qR q (x', p.2.1 + s * gy q p, p.2.2 + s * gz q p))
        (uIcc (-s1) s1) := ((qR_cont q).comp (by fun_prop)).continuousOn
    obtain ⟨s, hs, hs0'⟩ := intermediate_value_uIcc hcs
      (show (0 : ℝ) ∈ uIcc _ _ from mem_uIcc.mpr (Or.inl ⟨hn1.le, hp1.le⟩))
    refine ⟨(x', p.2.1 + s * gy q p, p.2.2 + s * gz q p), hs0', fun m hm hmj => hball ?_ m hm hmj,
      hoff⟩
    -- the point is in the ball
    have hsb : |s| ≤ s1 := by
      rw [uIcc_of_le (by linarith)] at hs; exact abs_le.mpr hs
    have hs1r : s1 * (|gy q p| + |gz q p| + 1) ≤ r / 2 := by
      have := min_le_right s0 (r / (2 * (|gy q p| + |gz q p| + 1)))
      calc s1 * (|gy q p| + |gz q p| + 1) ≤ r / (2 * (|gy q p| + |gz q p| + 1)) *
            (|gy q p| + |gz q p| + 1) := mul_le_mul_of_nonneg_right this (by positivity)
        _ = r / 2 := by field_simp
    rw [Metric.mem_ball, Prod.dist_eq, Prod.dist_eq, max_lt_iff, max_lt_iff, Real.dist_eq,
      Real.dist_eq, Real.dist_eq]
    refine ⟨by rw [abs_sub_lt_iff]; constructor <;> linarith [hx'.1, hx'.2, min_le_right δ (r / 2)],
      ?_, ?_⟩
    · have : |p.2.1 + s * gy q p - p.2.1| = |s| * |gy q p| := by
        rw [show p.2.1 + s * gy q p - p.2.1 = s * gy q p by ring, abs_mul]
      rw [this]
      nlinarith [abs_nonneg s, abs_nonneg (gy q p), abs_nonneg (gz q p)]
    · have : |p.2.2 + s * gz q p - p.2.2| = |s| * |gz q p| := by
        rw [show p.2.2 + s * gz q p - p.2.2 = s * gz q p by ring, abs_mul]
      rw [this]
      nlinarith [abs_nonneg s, abs_nonneg (gy q p), abs_nonneg (gz q p)]
  · -- only `∂_x q ≠ 0`: along `x`, on nearby vertical lines
    push Not at hyz
    have hgx : gx q p ≠ 0 := by
      rcases hg with h | h | h
      · exact h
      · exact absurd hyz.1 h
      · exact absurd hyz.2 h
    set a := gx q p * gx q p
    have ha : 0 < a := mul_self_pos.mpr hgx
    set d : R3 := (gx q p, 0, 0)
    obtain ⟨s0, hs0, hsg⟩ := small_sign (b := qq q d) ha
    obtain ⟨η, hη, hiso⟩ := isolate E p.1
    -- keep the segment inside the ball and inside the punctured interval
    set s1 := min s0 (min (r / (2 * (|gx q p| + 1))) (η / (2 * (|gx q p| + 1))))
    have hs1 : 0 < s1 := lt_min hs0 (lt_min (by positivity) (by positivity))
    obtain ⟨hpos, hneg⟩ := hsg s1 hs1 (min_le_left _ _)
    have hval : ∀ s, qR q (p.1 + s * d.1, p.2.1 + s * d.2.1, p.2.2 + s * d.2.2) =
        s * a + s * s * qq q d := fun s => by
      rw [qR_line, h0, hyz.1, hyz.2]; simp only [d]; ring
    have hpos' : 0 < qR q (p.1 + s1 * gx q p, p.2.1, p.2.2) := by
      have := hval s1; simp only [d, mul_zero, add_zero] at this; rw [this]; exact hpos
    have hneg' : qR q (p.1 + (-s1) * gx q p, p.2.1, p.2.2) < 0 := by
      have := hval (-s1); simp only [d, mul_zero, add_zero] at this; rw [this]
      have : -s1 * a + -s1 * -s1 * qq q d = s1 * (-a) + s1 * s1 * qq q d := by ring
      rw [this]; exact hneg
    -- on nearby vertical lines the signs persist
    have hc1 : ContinuousAt (fun w => qR q (p.1 + s1 * gx q p, p.2.1, p.2.2 + w)) 0 :=
      ((qR_cont q).comp (by fun_prop)).continuousAt
    have hc2 : ContinuousAt (fun w => qR q (p.1 + (-s1) * gx q p, p.2.1, p.2.2 + w)) 0 :=
      ((qR_cont q).comp (by fun_prop)).continuousAt
    obtain ⟨δ, hδ, hδb⟩ := Metric.mem_nhds_iff.mp (Filter.inter_mem
      (hc1.eventually (lt_mem_nhds (by simpa using hpos')))
      (hc2.eventually (gt_mem_nhds (by simpa using hneg'))))
    set δ' := min δ (r / 2)
    have hδ' : 0 < δ' := lt_min hδ (by linarith)
    have hgs : s1 * |gx q p| < r / 2 ∧ s1 * |gx q p| < η := by
      have h1 := min_le_left (r / (2 * (|gx q p| + 1))) (η / (2 * (|gx q p| + 1)))
      have h2 := min_le_right (r / (2 * (|gx q p| + 1))) (η / (2 * (|gx q p| + 1)))
      have h3 := min_le_right s0 (min (r / (2 * (|gx q p| + 1))) (η / (2 * (|gx q p| + 1))))
      have hg0 : 0 ≤ |gx q p| := abs_nonneg _
      constructor
      · calc s1 * |gx q p| ≤ r / (2 * (|gx q p| + 1)) * |gx q p| :=
              mul_le_mul_of_nonneg_right (_root_.le_trans h3 h1) hg0
          _ < r / 2 := by
            rw [div_mul_eq_mul_div, div_lt_div_iff₀ (by positivity) (by norm_num)]; nlinarith
      · calc s1 * |gx q p| ≤ η / (2 * (|gx q p| + 1)) * |gx q p| :=
              mul_le_mul_of_nonneg_right (_root_.le_trans h3 h2) hg0
          _ < η := by
            rw [div_mul_eq_mul_div, div_lt_iff₀ (by positivity)]; nlinarith
    by_contra hnot
    push Not at hnot
    apply no_vertical hf (List.getElem_mem hj) p.1 p.2.1 (show p.2.2 < p.2.2 + δ' by linarith)
    intro z hz
    set w := z - p.2.2
    have hwb : w ∈ Metric.ball (0 : ℝ) δ := by
      rw [Metric.mem_ball, Real.dist_eq, sub_zero, abs_lt]
      constructor <;> linarith [hz.1, hz.2, min_le_left δ (r / 2)]
    obtain ⟨hp1, hn1⟩ := hδb hwb
    simp only [mem_ofPred_eq] at hp1 hn1
    have hcs : ContinuousOn (fun s => qR q (p.1 + s * gx q p, p.2.1, p.2.2 + w)) (uIcc (-s1) s1) :=
      ((qR_cont q).comp (by fun_prop)).continuousOn
    obtain ⟨s, hs, hs0'⟩ := intermediate_value_uIcc hcs
      (show (0 : ℝ) ∈ uIcc _ _ from mem_uIcc.mpr (Or.inl ⟨hn1.le, hp1.le⟩))
    simp only at hs0'
    have hsb : |s| ≤ s1 := by
      rw [uIcc_of_le (by linarith)] at hs; exact abs_le.mpr hs
    have hzw : p.2.2 + w = z := by simp only [w]; ring
    by_cases hs0 : s = 0
    · rw [hs0, zero_mul, add_zero, hzw] at hs0'
      exact hs0'
    · exfalso
      -- otherwise the point is off the events and in the cell
      have hin : (p.1 + s * gx q p, p.2.1, p.2.2 + w) ∈ Metric.ball p r := by
        rw [Metric.mem_ball, Prod.dist_eq, Prod.dist_eq, max_lt_iff, max_lt_iff, Real.dist_eq,
          Real.dist_eq, Real.dist_eq]
        refine ⟨?_, by simp; linarith, ?_⟩
        · rw [show p.1 + s * gx q p - p.1 = s * gx q p by ring, abs_mul]
          nlinarith [abs_nonneg s, abs_nonneg (gx q p), hgs.1]
        · rw [show p.2.2 + w - p.2.2 = w by ring, abs_lt]
          constructor <;> linarith [hz.1, hz.2, min_le_right δ (r / 2)]
      obtain ⟨e, he, hd⟩ := hnot _ hs0' (fun m hm hmj => hball hin m hm hmj)
      rcases hiso e he _ hd with h | h
      · simp only at h
        have : s * gx q p = 0 := by linarith
        rcases mul_eq_zero.mp this with h' | h'
        · exact hs0 h'
        · exact hgx h'
      · simp only [show p.1 + s * gx q p - p.1 = s * gx q p by ring, abs_mul] at h
        nlinarith [abs_nonneg s, abs_nonneg (gx q p), hgs.2]

end ForgeCurved
