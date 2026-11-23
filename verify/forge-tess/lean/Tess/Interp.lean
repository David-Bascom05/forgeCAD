import Mathlib

namespace Sag

open Set

/-- Four zeros of a thrice continuously differentiable function: its third derivative has a zero
between the outer two (Rolle, three times). -/
theorem four_zeros (φ : ℝ → ℝ) (hφ : ContDiff ℝ 3 φ) {a b c d : ℝ} (hab : a < b) (hbc : b < c)
    (hcd : c < d) (ha : φ a = 0) (hb : φ b = 0) (hc : φ c = 0) (hd : φ d = 0) :
    ∃ ξ ∈ Ioo a d, deriv (deriv (deriv φ)) ξ = 0 := by
  have c0 : Continuous φ := hφ.continuous
  have d1 : ContDiff ℝ 2 (deriv φ) :=
    (contDiff_succ_iff_deriv.mp (show ContDiff ℝ (2 + 1) φ from hφ)).2.2
  have c1 : Continuous (deriv φ) := d1.continuous
  have d2 : ContDiff ℝ 1 (deriv (deriv φ)) :=
    (contDiff_succ_iff_deriv.mp (show ContDiff ℝ (1 + 1) (deriv φ) from d1)).2.2
  have c2 : Continuous (deriv (deriv φ)) := d2.continuous
  obtain ⟨p, ⟨pa, pb⟩, hp⟩ := exists_deriv_eq_zero hab c0.continuousOn (by rw [ha, hb])
  obtain ⟨q, ⟨qb, qc⟩, hq⟩ := exists_deriv_eq_zero hbc c0.continuousOn (by rw [hb, hc])
  obtain ⟨r, ⟨rc, rd⟩, hr⟩ := exists_deriv_eq_zero hcd c0.continuousOn (by rw [hc, hd])
  obtain ⟨u, ⟨up, uq⟩, hu⟩ := exists_deriv_eq_zero (pb.trans qb) c1.continuousOn (by rw [hp, hq])
  obtain ⟨v, ⟨vq, vr⟩, hv⟩ := exists_deriv_eq_zero (qc.trans rc) c1.continuousOn (by rw [hq, hr])
  obtain ⟨w, ⟨wu, wv⟩, hw⟩ := exists_deriv_eq_zero (uq.trans vq) c2.continuousOn (by rw [hu, hv])
  exact ⟨w, ⟨pa.trans (up.trans wu), (wv.trans vr).trans rd⟩, hw⟩

/-- The node polynomial of interpolation at 0, ½, 1 is at most √3/36 on [0, 1]. -/
theorem node_le (u : ℝ) (h0 : 0 ≤ u) (h1 : u ≤ 1) : |u * (u - 1 / 2) * (u - 1)| ≤ Real.sqrt 3 / 36 := by
  have s3 : Real.sqrt 3 ^ 2 = 3 := Real.sq_sqrt (by norm_num)
  have s0 : 0 ≤ Real.sqrt 3 := Real.sqrt_nonneg 3
  have s2 : Real.sqrt 3 < 2 := by
    rw [show (2 : ℝ) = Real.sqrt 4 by rw [show (4 : ℝ) = 2 ^ 2 by norm_num, Real.sqrt_sq (by norm_num)]]
    exact Real.sqrt_lt_sqrt (by norm_num) (by norm_num)
  rw [abs_le]
  constructor
  · -- ω + √3/36 = (u − u₂)²(u − u_b), u₂ = ½ + √3/6, u_b = ½ − √3/3 < 0
    have : u * (u - 1 / 2) * (u - 1) + Real.sqrt 3 / 36
        = (u - (1 / 2 + Real.sqrt 3 / 6)) ^ 2 * (u - (1 / 2 - Real.sqrt 3 / 3)) := by
      ring_nf; rw [show Real.sqrt 3 ^ 3 = Real.sqrt 3 ^ 2 * Real.sqrt 3 by ring, s3]; ring
    nlinarith [mul_nonneg (sq_nonneg (u - (1 / 2 + Real.sqrt 3 / 6)))
      (show 0 ≤ u - (1 / 2 - Real.sqrt 3 / 3) by nlinarith)]
  · have : Real.sqrt 3 / 36 - u * (u - 1 / 2) * (u - 1)
        = (u - (1 / 2 - Real.sqrt 3 / 6)) ^ 2 * ((1 / 2 + Real.sqrt 3 / 3) - u) := by
      ring_nf; rw [show Real.sqrt 3 ^ 3 = Real.sqrt 3 ^ 2 * Real.sqrt 3 by ring, s3]; ring
    nlinarith [mul_nonneg (sq_nonneg (u - (1 / 2 - Real.sqrt 3 / 6)))
      (show 0 ≤ (1 / 2 + Real.sqrt 3 / 3) - u by nlinarith)]

/-- **Interpolation at 0, ½, 1.** A thrice continuously differentiable function zero at 0 and
1, its third derivative at most M in size on [0, 1], strays from the parabola through its ends
and middle by at most M·√3/216 there. -/
theorem interp_remainder (f : ℝ → ℝ) (hf : ContDiff ℝ 3 f) (M : ℝ) (h0 : f 0 = 0) (h1 : f 1 = 0)
    (hM : ∀ x ∈ Icc (0 : ℝ) 1, |deriv (deriv (deriv f)) x| ≤ M) (s : ℝ) (hs0 : 0 ≤ s) (hs1 : s ≤ 1) :
    |f s - 4 * s * (1 - s) * f (1 / 2)| ≤ M * Real.sqrt 3 / 216 := by
  have M0 : 0 ≤ M := le_trans (abs_nonneg _) (hM 0 ⟨le_rfl, zero_le_one⟩)
  set c := f (1 / 2)
  set ω : ℝ → ℝ := fun u => u * (u - 1 / 2) * (u - 1)
  by_cases hw : ω s = 0
  · -- at a node the parabola is exact
    have : s = 0 ∨ s = 1 / 2 ∨ s = 1 := by
      simp only [ω, mul_eq_zero, sub_eq_zero] at hw; tauto
    have hp : 0 ≤ M * Real.sqrt 3 / 216 := by positivity
    rcases this with rfl | rfl | rfl <;> simp only [h0, h1, c] <;> norm_num <;> exact hp
  set K := (f s - 4 * s * (1 - s) * c) / ω s
  set φ : ℝ → ℝ := fun u => f u - (4 * u * (1 - u) * c + K * ω u)
  -- the subtracted cubic and its derivatives
  have q1 : ∀ u, HasDerivAt (fun u => 4 * u * (1 - u) * c + K * ω u)
      (4 * (1 - 2 * u) * c + K * (3 * u ^ 2 - 3 * u + 1 / 2)) u := by
    intro u
    have := ((((hasDerivAt_id u).const_mul 4).mul ((hasDerivAt_const u 1).sub (hasDerivAt_id u))).mul_const c).add
      ((((hasDerivAt_id u).mul ((hasDerivAt_id u).sub_const (1 / 2))).mul ((hasDerivAt_id u).sub_const 1)).const_mul K)
    convert this using 1
    · funext y; simp [ω]
    · simp; ring
  have q2 : ∀ u, HasDerivAt (fun u => 4 * (1 - 2 * u) * c + K * (3 * u ^ 2 - 3 * u + 1 / 2))
      (-8 * c + K * (6 * u - 3)) u := by
    intro u
    have := ((((hasDerivAt_const u 1).sub ((hasDerivAt_id u).const_mul 2)).const_mul 4).mul_const c).add
      (((((hasDerivAt_id u).pow 2).const_mul 3).sub ((hasDerivAt_id u).const_mul 3)).add_const (1 / 2)
        |>.const_mul K)
    convert this using 1
    · funext y; simp
    · simp; ring
  have q3 : ∀ u, HasDerivAt (fun u => -8 * c + K * (6 * u - 3)) (6 * K) u := by
    intro u
    have := ((((hasDerivAt_id u).const_mul 6).sub_const 3).const_mul K).const_add (-8 * c)
    convert this using 1
    · funext y; simp
    · simp; ring
  have df : Differentiable ℝ f := hf.differentiable (by norm_num)
  have f1 : ContDiff ℝ 2 (deriv f) := (contDiff_succ_iff_deriv.mp (show ContDiff ℝ (2 + 1) f from hf)).2.2
  have df1 : Differentiable ℝ (deriv f) := f1.differentiable (by norm_num)
  have f2 : ContDiff ℝ 1 (deriv (deriv f)) :=
    (contDiff_succ_iff_deriv.mp (show ContDiff ℝ (1 + 1) (deriv f) from f1)).2.2
  have df2 : Differentiable ℝ (deriv (deriv f)) := f2.differentiable one_ne_zero
  have e1 : deriv φ = fun u => deriv f u - (4 * (1 - 2 * u) * c + K * (3 * u ^ 2 - 3 * u + 1 / 2)) := by
    funext u; exact ((df u).hasDerivAt.sub (q1 u)).deriv
  have e2 : deriv (deriv φ) = fun u => deriv (deriv f) u - (-8 * c + K * (6 * u - 3)) := by
    rw [e1]; funext u; exact ((df1 u).hasDerivAt.sub (q2 u)).deriv
  have e3 : deriv (deriv (deriv φ)) = fun u => deriv (deriv (deriv f)) u - 6 * K := by
    rw [e2]; funext u; exact ((df2 u).hasDerivAt.sub (q3 u)).deriv
  have hφ : ContDiff ℝ 3 φ := by
    refine hf.sub (ContDiff.add ?_ ?_)
    · exact ((contDiff_const.mul contDiff_id).mul (contDiff_const.sub contDiff_id)).mul contDiff_const
    · exact contDiff_const.mul ((contDiff_id.mul (contDiff_id.sub contDiff_const)).mul (contDiff_id.sub contDiff_const))
  -- its zeros: 0, ½, 1 and s
  have z0 : φ 0 = 0 := by simp [φ, ω, h0]
  have zh : φ (1 / 2) = 0 := by simp [φ, ω, c]; ring
  have z1 : φ 1 = 0 := by simp [φ, ω, h1]
  have zs : φ s = 0 := by simp only [φ, K]; field_simp; ring
  have s0 : s ≠ 0 := by rintro rfl; simp [ω] at hw
  have sh : s ≠ 1 / 2 := by rintro rfl; simp [ω] at hw
  have s1 : s ≠ 1 := by rintro rfl; simp [ω] at hw
  obtain ⟨ξ, ⟨ξ0, ξ1⟩, hξ⟩ : ∃ ξ ∈ Ioo (0 : ℝ) 1, deriv (deriv (deriv φ)) ξ = 0 := by
    rcases lt_or_gt_of_ne sh with h | h
    · exact four_zeros φ hφ (lt_of_le_of_ne hs0 (Ne.symm s0)) h (by norm_num) z0 zs zh z1
    · exact four_zeros φ hφ (by norm_num) h (lt_of_le_of_ne hs1 s1) z0 zh zs z1
  rw [e3] at hξ
  simp only at hξ
  have hK : K = deriv (deriv (deriv f)) ξ / 6 := by linarith
  have hfs : f s - 4 * s * (1 - s) * c = K * ω s := by simp only [K]; field_simp
  rw [hfs, abs_mul, hK, abs_div]
  have := hM ξ ⟨ξ0.le, ξ1.le⟩
  have hn := node_le s hs0 hs1
  calc |deriv (deriv (deriv f)) ξ| / |(6 : ℝ)| * |ω s| ≤ M / 6 * (Real.sqrt 3 / 36) := by
        rw [show |(6 : ℝ)| = 6 by norm_num]
        exact mul_le_mul (div_le_div_of_nonneg_right this (by norm_num)) hn (abs_nonneg _) (by positivity)
    _ = M * Real.sqrt 3 / 216 := by ring

end Sag
