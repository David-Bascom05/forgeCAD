/-
The one new geometric lemma the 2.5D closed-form checker rests on (research/cam/phase1.md, L1):
the signed distance of a ρ-hypograph body {z ∈ Z, ρ ≤ w(z)} is non-decreasing in ρ at fixed z,
and depends only on (ρ, z).

Every tool forge-cam-geom builds (tool.rs: a revolved chain whose side rises) is such a body, with
Z = [z0, z1]; so is every non-cutting part (neck, shank, holder).
-/
import ForgeCam.SignedDist

open Metric Set

namespace ForgeCam

/-- Space, mm. Coordinates 0, 1, 2 are x, y, z. -/
abbrev E3 := EuclideanSpace ℝ (Fin 3)

/-- The point (a, b, c). -/
def mk (a b c : ℝ) : E3 := WithLp.toLp 2 ![a, b, c]

@[simp] theorem mk_0 (a b c : ℝ) : mk a b c 0 = a := by simp [mk]
@[simp] theorem mk_1 (a b c : ℝ) : mk a b c 1 = b := by simp [mk]
@[simp] theorem mk_2 (a b c : ℝ) : mk a b c 2 = c := by simp [mk]

theorem dist3 (p q : E3) :
    dist p q = √((p 0 - q 0) ^ 2 + (p 1 - q 1) ^ 2 + (p 2 - q 2) ^ 2) := by
  rw [EuclideanSpace.dist_eq, Fin.sum_univ_three]; simp [Real.dist_eq, sq_abs]

theorem ext3 {p q : E3} (h0 : p 0 = q 0) (h1 : p 1 = q 1) (h2 : p 2 = q 2) : p = q := by
  ext i; fin_cases i <;> assumption

/-- The distance from the z axis. -/
noncomputable def ρ (p : E3) : ℝ := √(p 0 ^ 2 + p 1 ^ 2)

theorem ρ_nonneg (p : E3) : 0 ≤ ρ p := Real.sqrt_nonneg _

theorem ρ_sq (p : E3) : ρ p ^ 2 = p 0 ^ 2 + p 1 ^ 2 :=
  Real.sq_sqrt (by positivity)

@[simp] theorem ρ_mk (a b c : ℝ) : ρ (mk a b c) = √(a ^ 2 + b ^ 2) := by simp [ρ]

/-- The body {z ∈ Z, ρ ≤ w(z)}: a solid of revolution about the z axis whose slice at each height
of `Z` is the disk of radius w(z) (empty where w(z) < 0). -/
def hyp (Z : Set ℝ) (w : ℝ → ℝ) : Set E3 := {p | p 2 ∈ Z ∧ ρ p ≤ w (p 2)}

/-- Cauchy–Schwarz in the plane, as the reverse triangle inequality for the radii. -/
theorem radial_le (p q : E3) :
    (ρ p - ρ q) ^ 2 ≤ (p 0 - q 0) ^ 2 + (p 1 - q 1) ^ 2 := by
  have hp := ρ_sq p; have hq := ρ_sq q
  have hpq : p 0 * q 0 + p 1 * q 1 ≤ ρ p * ρ q := by
    have h1 : (p 0 * q 0 + p 1 * q 1) ^ 2 ≤ (ρ p * ρ q) ^ 2 := by
      rw [mul_pow, hp, hq]; nlinarith [sq_nonneg (p 0 * q 1 - p 1 * q 0)]
    have h0 := mul_nonneg (ρ_nonneg p) (ρ_nonneg q)
    by_contra hc; push Not at hc; nlinarith
  nlinarith

/-- A point at height c whose distance from the axis is r, on the ray through (x, y): the radial
rescale. -/
noncomputable def onRay (p : E3) (r c : ℝ) : E3 := mk (r / ρ p * p 0) (r / ρ p * p 1) c

theorem ρ_onRay {p : E3} (hp : 0 < ρ p) {r : ℝ} (hr : 0 ≤ r) (c : ℝ) : ρ (onRay p r c) = r := by
  have h := ρ_sq p
  simp only [onRay, ρ_mk]
  rw [show (r / ρ p * p 0) ^ 2 + (r / ρ p * p 1) ^ 2 = r ^ 2 / ρ p ^ 2 * (p 0 ^ 2 + p 1 ^ 2) by
    ring, ← h, div_mul_cancel₀ _ (by positivity), Real.sqrt_sq hr]

/-- The distance from p to the point on its own ray at radius r and height c. -/
theorem dist_onRay {p : E3} (hp : 0 < ρ p) (r c : ℝ) :
    dist p (onRay p r c) = √((ρ p - r) ^ 2 + (p 2 - c) ^ 2) := by
  rw [dist3]; congr 1
  have h := ρ_sq p
  simp only [onRay, mk_0, mk_1, mk_2]
  have e : (p 0 - r / ρ p * p 0) ^ 2 + (p 1 - r / ρ p * p 1) ^ 2
      = (1 - r / ρ p) ^ 2 * (p 0 ^ 2 + p 1 ^ 2) := by ring
  rw [e, ← h]
  field_simp

/-- A distance is at least its vertical part. -/
theorem abs_z_le_dist (p q : E3) : |p 2 - q 2| ≤ dist p q := by
  rw [dist3, ← Real.sqrt_sq_eq_abs]
  exact Real.sqrt_le_sqrt (by nlinarith [sq_nonneg (p 0 - q 0), sq_nonneg (p 1 - q 1)])

/-- A distance is at least its radial part. -/
theorem radial_dist_le (p q : E3) : √((ρ p - ρ q) ^ 2 + (p 2 - q 2) ^ 2) ≤ dist p q := by
  rw [dist3]; exact Real.sqrt_le_sqrt (by nlinarith [radial_le p q])

/-- The point above or below p at height c. -/
def atZ (p : E3) (c : ℝ) : E3 := mk (p 0) (p 1) c

theorem ρ_atZ (p : E3) (c : ℝ) : ρ (atZ p c) = ρ p := by simp [atZ, ρ]

theorem dist_atZ (p : E3) (c : ℝ) : dist p (atZ p c) = |p 2 - c| := by
  rw [dist3]; simp [atZ, Real.sqrt_sq_eq_abs]

/-- Distances to a set shrink if every point of the set can be traded for one at least as near. -/
theorem infDist_le_of_trade {S : Set E3} {p q : E3}
    (h : ∀ c ∈ S, ∃ c' ∈ S, dist q c' ≤ dist p c) : infDist q S ≤ infDist p S := by
  rcases S.eq_empty_or_nonempty with rfl | hne
  · simp
  · refine (le_infDist hne).mpr fun c hc => ?_
    obtain ⟨c', hc', hle⟩ := h c hc
    exact (infDist_le_dist_of_mem hc').trans hle

section mono
variable {Z : Set ℝ} {w : ℝ → ℝ} {p q : E3}

/-- Inside is closed under moving toward the axis. -/
theorem hyp_mem_of_le (hz : p 2 = q 2) (hρ : ρ p ≤ ρ q) (hq : q ∈ hyp Z w) : p ∈ hyp Z w :=
  ⟨hz ▸ hq.1, hz ▸ hρ.trans hq.2⟩

/-- Toward the axis, the body is no farther. -/
theorem infDist_hyp_le (hz : p 2 = q 2) (hρ : ρ p ≤ ρ q) :
    infDist p (hyp Z w) ≤ infDist q (hyp Z w) := by
  refine infDist_le_of_trade fun b hb => ?_
  by_cases hpb : ρ p ≤ ρ b
  · refine ⟨atZ p (b 2), ⟨by simpa [atZ] using hb.1, ?_⟩, ?_⟩
    · simp only [atZ, mk_2]; rw [← atZ, ρ_atZ]; exact hpb.trans hb.2
    · rw [dist_atZ, hz]; exact abs_z_le_dist q b
  · push Not at hpb
    have hp0 : 0 < ρ p := lt_of_le_of_lt (ρ_nonneg b) hpb
    refine ⟨onRay p (ρ b) (b 2), ⟨by simpa [onRay] using hb.1, ?_⟩, ?_⟩
    · have : (onRay p (ρ b) (b 2)) 2 = b 2 := by simp [onRay]
      rw [this, ρ_onRay hp0 (ρ_nonneg b)]; exact hb.2
    · rw [dist_onRay hp0, hz]
      refine le_trans (Real.sqrt_le_sqrt ?_) (radial_dist_le q b)
      nlinarith [ρ_nonneg b]

/-- Toward the axis, the outside is no nearer. -/
theorem infDist_hyp_compl_le (hz : p 2 = q 2) (hρ : ρ p ≤ ρ q) :
    infDist q (hyp Z w)ᶜ ≤ infDist p (hyp Z w)ᶜ := by
  refine infDist_le_of_trade fun c hc => ?_
  have hc' : c 2 ∉ Z ∨ w (c 2) < ρ c := by
    by_contra h; push Not at h; exact hc ⟨h.1, h.2⟩
  -- the point of q's vertical line at c's height, if it is outside
  have vert : (c 2 ∉ Z ∨ w (c 2) < ρ q) →
      ∃ c' ∈ (hyp Z w)ᶜ, dist q c' ≤ dist p c := by
    intro h
    refine ⟨atZ q (c 2), ?_, ?_⟩
    · rintro ⟨h1, h2⟩
      simp only [atZ, mk_2] at h1 h2
      rw [← atZ, ρ_atZ] at h2
      rcases h with h | h
      · exact h h1
      · linarith
    · rw [dist_atZ, ← hz]; exact abs_z_le_dist p c
  rcases hc' with h | h
  · exact vert (Or.inl h)
  by_cases hqc : ρ c ≤ ρ q
  · exact vert (Or.inr (lt_of_lt_of_le h hqc))
  push Not at hqc
  rcases (ρ_nonneg q).eq_or_lt with hq0 | hq0
  · -- q is on the axis, and so is p: they are the same point
    have hp0 : ρ p = 0 := le_antisymm (hq0 ▸ hρ) (ρ_nonneg p)
    have z0 : ∀ r : E3, ρ r = 0 → r 0 = 0 ∧ r 1 = 0 := by
      intro r hr
      have := ρ_sq r; rw [hr] at this
      constructor <;> nlinarith [sq_nonneg (r 0), sq_nonneg (r 1)]
    have hpq : p = q := ext3 (by rw [(z0 p hp0).1, (z0 q hq0.symm).1])
      (by rw [(z0 p hp0).2, (z0 q hq0.symm).2]) hz
    exact ⟨c, hc, hpq ▸ le_rfl⟩
  · refine ⟨onRay q (ρ c) (c 2), ?_, ?_⟩
    · rintro ⟨_, h2⟩
      have : (onRay q (ρ c) (c 2)) 2 = c 2 := by simp [onRay]
      rw [this, ρ_onRay hq0 (ρ_nonneg c)] at h2
      linarith
    · rw [dist_onRay hq0, ← hz]
      refine le_trans (Real.sqrt_le_sqrt ?_) ((radial_dist_le c p).trans (dist_comm c p).le)
      nlinarith [ρ_nonneg p]

/-- **The lemma.** At a fixed height, the signed distance of a ρ-hypograph body is
non-decreasing in ρ. -/
theorem sd_hyp_mono (hz : p 2 = q 2) (hρ : ρ p ≤ ρ q) : sd (hyp Z w) p ≤ sd (hyp Z w) q := by
  by_cases hq : q ∈ hyp Z w
  · have hp := hyp_mem_of_le hz hρ hq
    rw [sd_of_mem hp, sd_of_mem hq]
    exact neg_le_neg (infDist_hyp_compl_le hz hρ)
  · rw [sd_of_not_mem hq]
    by_cases hp : p ∈ hyp Z w
    · exact (sd_nonpos_of_mem hp).trans infDist_nonneg
    · rw [sd_of_not_mem hp]; exact infDist_hyp_le hz hρ

/-- So it depends only on (ρ, z). -/
theorem sd_hyp_eq (hz : p 2 = q 2) (hρ : ρ p = ρ q) : sd (hyp Z w) p = sd (hyp Z w) q :=
  le_antisymm (sd_hyp_mono hz hρ.le) (sd_hyp_mono hz.symm hρ.ge)

end mono

/-- The signed distance of the body as a function of (ρ, z): its value at (ρ, 0, z). -/
noncomputable def sdRZ (Z : Set ℝ) (w : ℝ → ℝ) (r z : ℝ) : ℝ := sd (hyp Z w) (mk r 0 z)

theorem sd_hyp_eq_sdRZ (Z : Set ℝ) (w : ℝ → ℝ) (p : E3) : sd (hyp Z w) p = sdRZ Z w (ρ p) (p 2) :=
  sd_hyp_eq (by simp) (by simp [Real.sqrt_sq (ρ_nonneg p)])

/-- The form the checker uses: a lower bound D on a point's distance from the axis bounds its
signed distance below by the value at D. -/
theorem sdRZ_le_sd {Z : Set ℝ} {w : ℝ → ℝ} {p : E3} {D : ℝ} (hD : 0 ≤ D) (hle : D ≤ ρ p) :
    sdRZ Z w D (p 2) ≤ sd (hyp Z w) p :=
  sd_hyp_mono (by simp) (by simpa [Real.sqrt_sq hD] using hle)

theorem sdRZ_mono {Z : Set ℝ} {w : ℝ → ℝ} {r r' z : ℝ} (hr : 0 ≤ r) (h : r ≤ r') :
    sdRZ Z w r z ≤ sdRZ Z w r' z :=
  sd_hyp_mono (by simp) (by simp [Real.sqrt_sq hr, Real.sqrt_sq (hr.trans h), h])

end ForgeCam
