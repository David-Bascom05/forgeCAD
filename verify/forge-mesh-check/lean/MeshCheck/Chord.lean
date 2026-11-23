import Mathlib

/-!
# Chord steps on circles (kernel/forge-mesh/src/tessellate.rs:33–64, at 8250ebb)

A chord over angle θ of a circle of radius r sags `r (1 − cos(θ/2))`. `arc_step` takes the step
`2 acos(1 − chord/r)` (π once chord ≥ r), no more than `max_angle`, and no less than 1e-4.
`edge_segments` cuts an edge of angle w into ⌈w / step⌉ equal chords.

Proven:
* `by_chord_sag`: the chord step sags exactly `chord`;
* `segments_sag`: every chord `edge_segments` makes sags at most `chord`, unless the 1e-4 floor
  binds. That happens only where 2 acos(1 − chord/r) < 1e-4, about chord/r < 1.25·10⁻⁹:
  `floor_overshoots` gives an instance;
* `sqrt2_over_half`: on a surface curved both ways, each direction gets the step shortened by
  √2, "half the tolerance each" (tessellate.rs:42–44, research/mesh-bounds). It is a little more
  than half: two such sags add up past the tolerance. The excess is fourth order, below 0.07%
  for steps under `max_angle` = π/12. `interior_steps`' extra 0.9 (tessellate.rs:102) covers it
  inside faces, but `edge_segments` has no such factor.
-/

noncomputable section
open Real

namespace MeshCheck

def sag (r θ : ℝ) : ℝ := r * (1 - cos (θ / 2))

/-- tessellate.rs:33–40 -/
def arcStep (r chord maxAngle : ℝ) : ℝ :=
  max (min (if chord ≥ r then π else 2 * arccos (1 - chord / r)) maxAngle) 1e-4

theorem by_chord_sag {r c : ℝ} (hr : 0 < r) (hc : 0 < c) (hcr : c < r) :
    sag r (2 * arccos (1 - c / r)) = c := by
  have h1 : -1 ≤ 1 - c / r := by
    have : c / r < 1 := (div_lt_one hr).2 hcr
    have : 0 < c / r := div_pos hc hr
    linarith
  have h2 : 1 - c / r ≤ 1 := by have := div_pos hc hr; linarith
  rw [sag, show 2 * arccos (1 - c / r) / 2 = arccos (1 - c / r) by ring, cos_arccos h1 h2]
  field_simp
  ring

theorem sag_mono {r θ₁ θ₂ : ℝ} (hr : 0 ≤ r) (h0 : 0 ≤ θ₁) (h12 : θ₁ ≤ θ₂) (h2 : θ₂ ≤ 2 * π) :
    sag r θ₁ ≤ sag r θ₂ := by
  unfold sag
  apply mul_le_mul_of_nonneg_left _ hr
  have : cos (θ₂ / 2) ≤ cos (θ₁ / 2) :=
    cos_le_cos_of_nonneg_of_le_pi (by linarith) (by linarith) (by linarith)
  linarith

/-- Each of the ⌈w/step⌉ equal chords of an edge of angle w sags at most `chord`, where the
1e-4 floor does not bind (and max_angle ≤ 2π). -/
theorem segments_sag {r c maxAngle w : ℝ} (hr : 0 < r) (hc : 0 < c) (hm : 0 < maxAngle)
    (hm2 : maxAngle ≤ 2 * π) (hw : 0 ≤ w)
    (hfloor : 1e-4 ≤ min (if c ≥ r then π else 2 * arccos (1 - c / r)) maxAngle) :
    sag r (w / ⌈w / arcStep r c maxAngle⌉₊) ≤ c := by
  set s := arcStep r c maxAngle with hs
  have hs' : s = min (if c ≥ r then π else 2 * arccos (1 - c / r)) maxAngle := by
    rw [hs, arcStep, max_eq_left hfloor]
  have spos : 0 < s := by rw [hs']; linarith [hfloor]
  -- the chord's angle is at most the step
  have hθ : w / ⌈w / s⌉₊ ≤ s := by
    rcases eq_or_lt_of_le hw with h | h
    · rw [← h]; simp; exact spos.le
    · have hn : (0 : ℝ) < ⌈w / s⌉₊ := by
        have : 0 < w / s := div_pos h spos
        exact_mod_cast Nat.ceil_pos.2 this
      rw [div_le_iff₀ hn]
      have := Nat.le_ceil (w / s)
      rw [div_le_iff₀ spos] at this
      linarith
  have hθ0 : 0 ≤ w / ⌈w / s⌉₊ := div_nonneg hw (Nat.cast_nonneg _)
  have hs2 : s ≤ 2 * π := by rw [hs']; exact (min_le_right _ _).trans hm2
  calc sag r (w / ⌈w / s⌉₊) ≤ sag r s := sag_mono hr.le hθ0 hθ hs2
    _ ≤ c := by
      by_cases hcr : c ≥ r
      · have : s ≤ π := by rw [hs', if_pos hcr]; exact min_le_left _ _
        calc sag r s ≤ sag r π := sag_mono hr.le spos.le this (by linarith [pi_pos])
          _ = r := by simp [sag, cos_pi_div_two]
          _ ≤ c := hcr
      · push_neg at hcr
        have : s ≤ 2 * arccos (1 - c / r) := by rw [hs', if_neg (not_le.2 hcr)]; exact min_le_left _ _
        have harc : 2 * arccos (1 - c / r) ≤ 2 * π := by linarith [arccos_le_pi (1 - c / r)]
        calc sag r s ≤ sag r (2 * arccos (1 - c / r)) := sag_mono hr.le spos.le this harc
          _ = c := by_chord_sag hr hc hcr

/-- Where the floor binds the chord sags past `chord`: radius 1, chord 10⁻¹², step 10⁻⁴. -/
theorem floor_overshoots :
    2 * arccos (1 - 1e-12 / 1) < 1e-4 ∧ (1e-12 : ℝ) < sag 1 1e-4 := by
  have hb := Real.cos_bound (x := 5e-5) (by rw [abs_of_pos (by norm_num)]; norm_num)
  rw [abs_le] at hb
  have hcos : cos (5e-5 : ℝ) < 1 - 1e-12 := by
    have := hb.2
    rw [abs_of_pos (show (0 : ℝ) < 5e-5 by norm_num)] at this
    nlinarith
  constructor
  · have : arccos (1 - 1e-12 / 1) < arccos (cos 5e-5) :=
      arccos_lt_arccos (by linarith [neg_one_le_cos (5e-5 : ℝ)]) (by norm_num; linarith) (by norm_num)
    rw [arccos_cos (by norm_num) (by linarith [pi_gt_three])] at this
    linarith
  · unfold sag
    rw [show (1e-4 : ℝ) / 2 = 5e-5 by norm_num]
    linarith

/-- √2 sin(y/√2) > sin y on (0, π/2]: what makes the √2-shortened step sag more than half. -/
theorem sqrt2_sin (y : ℝ) (hy : 0 < y) (hy2 : y ≤ π / 2) : sin y < √2 * sin (y / √2) := by
  have h2 : (1 : ℝ) < √2 := by
    rw [show (1 : ℝ) = √1 by simp]; exact Real.sqrt_lt_sqrt (by norm_num) (by norm_num)
  have hs2 : 0 < √2 := by positivity
  let g := fun t : ℝ => √2 * sin (t / √2) - sin t
  have hd : ∀ t, HasDerivAt g (cos (t / √2) - cos t) t := by
    intro t
    have h1 := ((hasDerivAt_id t).div_const √2).sin
    have := (h1.const_mul √2).sub (hasDerivAt_sin t)
    convert this using 1
    · ext s; simp only [g, Pi.sub_apply, id]
    · simp only [id]; field_simp
  have mono : StrictMonoOn g (Set.Icc 0 (π / 2)) := by
    apply strictMonoOn_of_deriv_pos (convex_Icc 0 (π / 2))
    · exact fun t _ => (hd t).continuousAt.continuousWithinAt
    · intro t ht
      rw [interior_Icc] at ht
      rw [(hd t).deriv]
      have ht0 : 0 < t := ht.1
      have hlt : t / √2 < t := by rw [div_lt_iff₀ hs2]; nlinarith
      have := cos_lt_cos_of_nonneg_of_le_pi_div_two (by positivity) (by linarith [ht.2]) hlt
      linarith
  have := mono ⟨le_refl 0, by positivity⟩ ⟨hy.le, hy2⟩ hy
  simp only [g, zero_div, sin_zero, mul_zero, sub_zero] at this
  linarith

/-- Two chords each at the √2-shortened step sag more than one chord at the full step. -/
theorem sqrt2_over_half (r x : ℝ) (hr : 0 < r) (hx : 0 < x) (hxπ : x ≤ π) :
    sag r x < 2 * sag r (x / √2) := by
  unfold sag
  have hs2 : 0 < √2 := by positivity
  have e : ∀ w, 1 - cos (2 * w) = 2 * sin w ^ 2 := by
    intro w; rw [cos_two_mul]; nlinarith [sin_sq_add_cos_sq w]
  set z := x / 4
  have hz : 0 < z := by positivity
  have hz2 : z ≤ π / 2 := by simp only [z]; linarith [pi_pos]
  rw [show x / 2 = 2 * z by simp only [z]; ring, show x / √2 / 2 = 2 * (z / √2) by
    simp only [z]; field_simp; ring, e, e]
  have key := sqrt2_sin z hz hz2
  have hsz : 0 < sin z := sin_pos_of_pos_of_lt_pi hz (by linarith [pi_pos])
  have sq : sin z ^ 2 < 2 * sin (z / √2) ^ 2 := by
    have h := pow_lt_pow_left₀ key hsz.le (by norm_num : (2 : ℕ) ≠ 0)
    rw [mul_pow, Real.sq_sqrt (by norm_num)] at h
    exact h
  nlinarith

end MeshCheck
end
