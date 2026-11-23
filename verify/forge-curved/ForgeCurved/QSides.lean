import ForgeCurved.QWall

/-!
# Both sides of a facet are realised

Under `singAll`, a facet's carrier has a nonzero gradient at the facet's points. A small step
along `±∇q` from such a point changes `q`'s sign and no other carrier's. So `X.set j pos` and
`X.set j neg`, the facet's two side cells, are realised (`facet_sides`). This holds in any frame,
and the frame map carries it to the input carriers (`realised_unframe`).
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeCurved.Sweep
  Set Topology Filter

theorem realised_unframe (f : Frame) {qs : List Quad.Quadric} {X : Cov}
    (h : RealisedK ((qs.map (inFrame f)).map qR) X) : RealisedK (qs.map qR) X := by
  obtain ⟨p, hp⟩ := h
  refine ⟨frameT f p, ?_⟩
  show covOfK _ _ = X
  rw [← cov_frame]; exact hp

/-- A point with the facet's other signs and the sign `s` at `j` is in the side cell. -/
theorem cell_set {qs : List Quad.Quadric} {X : Cov} {p p' : R3}
    (hp : covCellK (qs.map qR) X p) {j : ℕ} (hj : j < qs.length) {s : Sgn}
    (hsj : sgnK (qR qs[j] p') = s)
    (hs : ∀ m (hm : m < qs.length), m ≠ j → sgnK (qR qs[m] p') = sgnK (qR qs[m] p)) :
    covCellK (qs.map qR) (X.set j s) p' := by
  show covOfK (qs.map qR) p' = X.set j s
  apply List.ext_getElem?
  intro m
  rw [getElem?_covOfK]
  have hlen : X.length = qs.length := by rw [← hp]; simp [covOfK]
  by_cases hm : m < qs.length
  · simp only [List.getElem?_map, List.getElem?_eq_getElem hm, Option.map_some]
    by_cases hmj : m = j
    · subst hmj; rw [List.getElem?_set_self (by omega), hsj]
    · rw [List.getElem?_set_ne (Ne.symm hmj), hs m hm hmj, ← hp, getElem?_covOfK]; simp [hm]
  · have hm' : qs.length ≤ m := not_lt.mp hm
    simp only [List.getElem?_map, List.getElem?_eq_none hm', Option.map_none]
    exact (List.getElem?_eq_none (by simp; omega)).symm

/-- **Both sides of a realised facet are realised.** -/
theorem facet_sides {qs : List Quad.Quadric} (hsing : singAll qs = true) {X : Cov} {j : ℕ}
    (hj : j < qs.length) (hXj : X[j]? = some Sgn.zero) (hXo : ∀ m, m ≠ j → X[m]? ≠ some Sgn.zero)
    (hX : RealisedK (qs.map qR) X) :
    RealisedK (qs.map qR) (X.set j .pos) ∧ RealisedK (qs.map qR) (X.set j .neg) := by
  obtain ⟨p, hp⟩ := hX
  obtain ⟨h0, hne⟩ := facet_signs hp hj hXj hXo
  have hg : gx qs[j] p ≠ 0 ∨ gy qs[j] p ≠ 0 ∨ gz qs[j] p ≠ 0 := by
    by_contra hn
    push Not at hn
    obtain ⟨m, hmj, r, hr, hr0⟩ := singAll_spec realModel hsing (j := j) (q := qs[j])
      (List.getElem?_eq_getElem hj) (p := p)
      ⟨by rw [dxK_real]; exact hn.1, by rw [dyK_real]; exact hn.2.1, by rw [dzK_real]; exact hn.2.2⟩
      (by rw [evK_real]; exact h0)
    have hm : m < qs.length := (List.getElem?_eq_some_iff.mp hr).1
    have : qs[m] = r := (List.getElem?_eq_some_iff.mp hr).2
    exact hne m hm hmj (by rw [this, ← evK_real]; exact hr0)
  set q := qs[j]
  set v : R3 := (gx q p, gy q p, gz q p)
  set a := gx q p * gx q p + gy q p * gy q p + gz q p * gz q p
  have ha : 0 < a := by
    rcases hg with h | h | h
    · have := mul_self_pos.mpr h; nlinarith [mul_self_nonneg (gy q p), mul_self_nonneg (gz q p)]
    · have := mul_self_pos.mpr h; nlinarith [mul_self_nonneg (gx q p), mul_self_nonneg (gz q p)]
    · have := mul_self_pos.mpr h; nlinarith [mul_self_nonneg (gx q p), mul_self_nonneg (gy q p)]
  obtain ⟨r, hr, hball⟩ := Metric.mem_nhds_iff.mp (signs_nhds3 (j := j) hne)
  obtain ⟨s0, hs0, hsg⟩ := small_sign (b := qq q v) ha
  set s1 := min s0 (r / (2 * (‖v‖ + 1)))
  have hs1 : 0 < s1 := lt_min hs0 (by positivity)
  obtain ⟨hpos, hneg⟩ := hsg s1 hs1 (min_le_left _ _)
  have hval : ∀ s, qR q (p.1 + s * v.1, p.2.1 + s * v.2.1, p.2.2 + s * v.2.2) =
      s * a + s * s * qq q v := fun s => by rw [qR_line, h0]; simp only [v, a]; ring
  have hin : ∀ s, |s| ≤ s1 → (p.1 + s * v.1, p.2.1 + s * v.2.1, p.2.2 + s * v.2.2) ∈ Metric.ball p r := by
    intro s hs
    rw [Metric.mem_ball, dist_eq_norm]
    have : (p.1 + s * v.1, p.2.1 + s * v.2.1, p.2.2 + s * v.2.2) - p = s • v := by
      obtain ⟨a, b, c⟩ := p; obtain ⟨x, y, z⟩ := v
      simp only [Prod.mk_sub_mk, Prod.smul_mk, smul_eq_mul, Prod.mk.injEq]
      refine ⟨by ring, by ring, by ring⟩
    rw [this, norm_smul, Real.norm_eq_abs]
    have h2 : s1 * (‖v‖ + 1) ≤ r / 2 := by
      have := min_le_right s0 (r / (2 * (‖v‖ + 1)))
      calc s1 * (‖v‖ + 1) ≤ r / (2 * (‖v‖ + 1)) * (‖v‖ + 1) :=
            mul_le_mul_of_nonneg_right this (by positivity)
        _ = r / 2 := by field_simp
    nlinarith [norm_nonneg v, abs_nonneg s]
  constructor
  · refine ⟨_, cell_set hp hj ?_ (fun m hm hmj => hball (hin s1 (by rw [abs_of_pos hs1])) m hm hmj)⟩
    rw [hval]; exact sgnK_pos hpos
  · refine ⟨_, cell_set hp hj ?_ (fun m hm hmj => hball (hin (-s1) (by rw [abs_neg, abs_of_pos hs1])) m hm hmj)⟩
    rw [hval]
    have : -s1 * a + -s1 * -s1 * qq q v = s1 * (-a) + s1 * s1 * qq q v := by ring
    rw [this]; exact sgnK_neg hneg

end ForgeCurved
