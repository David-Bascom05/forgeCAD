import ForgeCurved.ContactModel
import Mathlib.Analysis.SpecialFunctions.Trigonometric.Basic

/-!
# Coaxial equal radii: a sphere in its cylinder, tangent along a circle

The unit sphere and the unit cylinder about the `z` axis touch along the equator circle; its
plane `z = 0` is the contact seam (design §2.3). Near `(1, 0, 0)`, in coordinates
`(u, w, v) = (x - 1, z, y)`, both are graphs over `(w, v)`:
* the cylinder `u = √(1 - v²) - 1`;
* the sphere `u = √(1 - v² - w²) - 1`.
The sphere's graph lies below the cylinder's off the seam and meets it on the seam, so the order
is fixed on each side of the seam and on it.

**`coaxialContact`** is the model there (`ContactAt`). **`coaxial_localConn`** gives local
connectivity at every point of the circle: the carriers are invariant under rotation about the
axis (`localConn_of_invariant`).
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set FaceLocal

/-- The sphere, the cylinder and the equatorial seam. -/
def coaxPair : List (Carrier R3 ℝ) :=
  [fun p => p.1 ^ 2 + p.2.1 ^ 2 + p.2.2 ^ 2 - 1, fun p => p.1 ^ 2 + p.2.1 ^ 2 - 1, fun p => p.2.2]

/-- `(x, y, z) ↦ (x - 1, z, y)`. -/
def coaxT : R3 ≃ₜ R3 where
  toFun p := (p.1 + -1, p.2.2, p.2.1)
  invFun q := (q.1 + 1, q.2.2, q.2.1)
  left_inv p := by simp
  right_inv q := by simp
  continuous_toFun := by fun_prop
  continuous_invFun := by fun_prop

/-- The graphs over `(w, v)`: the sphere, then the cylinder. -/
noncomputable def coaxA : Fin 2 → ℝ × ℝ → ℝ
  | 0 => fun d => Real.sqrt (1 - d.2 ^ 2 - d.1 ^ 2) - 1
  | 1 => fun d => Real.sqrt (1 - d.2 ^ 2) - 1

theorem sgnK_sqrt_sub {a b : ℝ} (ha : 0 ≤ a) (hab : a ≤ b) :
    sgnK (Real.sqrt a - Real.sqrt b) = if a = b then .zero else .neg := by
  split_ifs with h
  · subst h; simp [sgnK]
  · exact sgnK_neg (by
      have := Real.sqrt_lt_sqrt ha (lt_of_le_of_ne hab h); linarith)

/-- **The model at `(1, 0, 0)`.** -/
noncomputable def coaxialContact : ContactAt coaxPair (1, 0, 0) where
  T := coaxT
  hT := by simp [coaxT]
  m := 2
  a := coaxA
  ha k := by fin_cases k <;> (simp only [coaxA]; fun_prop)
  ha0 k := by fin_cases k <;> simp [coaxA]
  kind i := if i.1 = 0 then some 0 else if i.1 = 1 then some 1 else none
  fl _ := false
  V := {p | 1 / 2 < p.1 ∧ |p.2.1| < 1 / 2 ∧ |p.2.2| < 1 / 2}
  hV := by
    have h1 : IsOpen {p : R3 | (1 / 2 : ℝ) < p.1} := isOpen_lt continuous_const continuous_fst
    have h2 : IsOpen {p : R3 | |p.2.1| < 1 / 2} := isOpen_lt (by fun_prop) continuous_const
    have h3 : IsOpen {p : R3 | |p.2.2| < 1 / 2} := isOpen_lt (by fun_prop) continuous_const
    exact h1.inter (h2.inter h3)
  hxV := by norm_num
  hm i _ y hy := by
    obtain ⟨h1, h2, h3⟩ := hy
    have hy2 : y.2.1 ^ 2 < 1 / 4 := by nlinarith [abs_lt.mp h2, sq_abs y.2.1]
    have hz2 : y.2.2 ^ 2 < 1 / 4 := by nlinarith [abs_lt.mp h3, sq_abs y.2.2]
    fin_cases i
    · set s := Real.sqrt (1 - y.2.1 ^ 2 - y.2.2 ^ 2)
      have hs : s ^ 2 = 1 - y.2.1 ^ 2 - y.2.2 ^ 2 := Real.sq_sqrt (by linarith)
      have hs0 : 0 ≤ s := Real.sqrt_nonneg _
      have : y.1 ^ 2 + y.2.1 ^ 2 + y.2.2 ^ 2 - 1 = (y.1 - s) * (y.1 + s) := by nlinarith [hs]
      show sgnK (y.1 ^ 2 + y.2.1 ^ 2 + y.2.2 ^ 2 - 1) =
        fS false (sgnK (y.1 + -1 - (Real.sqrt (1 - y.2.1 ^ 2 - y.2.2 ^ 2) - 1)))
      rw [this, sgnK_mul_pos' (by linarith), show y.1 + -1 - (s - 1) = y.1 - s by ring]; rfl
    · set s := Real.sqrt (1 - y.2.1 ^ 2)
      have hs : s ^ 2 = 1 - y.2.1 ^ 2 := Real.sq_sqrt (by linarith)
      have hs0 : 0 ≤ s := Real.sqrt_nonneg _
      have : y.1 ^ 2 + y.2.1 ^ 2 - 1 = (y.1 - s) * (y.1 + s) := by nlinarith [hs]
      show sgnK (y.1 ^ 2 + y.2.1 ^ 2 - 1) = fS false (sgnK (y.1 + -1 - (Real.sqrt (1 - y.2.1 ^ 2) - 1)))
      rw [this, sgnK_mul_pos' (by linarith), show y.1 + -1 - (s - 1) = y.1 - s by ring]; rfl
    · rfl
  used o := by
    rcases o with _ | k
    · exact ⟨⟨2, by simp [coaxPair]⟩, show (0 : ℝ) = 0 from rfl, by simp⟩
    · fin_cases k
      · exact ⟨⟨0, by simp [coaxPair]⟩,
          show (1 : ℝ) ^ 2 + (0 : ℝ) ^ 2 + (0 : ℝ) ^ 2 - 1 = 0 by norm_num, by simp⟩
      · exact ⟨⟨1, by simp [coaxPair]⟩, show (1 : ℝ) ^ 2 + (0 : ℝ) ^ 2 - 1 = 0 by norm_num, by simp⟩
  δ₀ := 1 / 2
  hδ₀ := by norm_num
  ord k l d d' hd hd' hs := by
    have hb : ∀ e : ℝ × ℝ, ‖e‖ < 1 / 2 → e.1 ^ 2 < 1 / 4 ∧ e.2 ^ 2 < 1 / 4 := by
      intro e he
      have h1 := (norm_fst_le e).trans_lt he
      have h2 := (norm_snd_le e).trans_lt he
      rw [Real.norm_eq_abs] at h1 h2
      exact ⟨by nlinarith [abs_lt.mp h1, sq_abs e.1], by nlinarith [abs_lt.mp h2, sq_abs e.2]⟩
    have key : ∀ e : ℝ × ℝ, ‖e‖ < 1 / 2 →
        sgnK (coaxA 0 e - coaxA 1 e) = if e.1 = 0 then .zero else .neg := by
      intro e he
      obtain ⟨h1, h2⟩ := hb e he
      have : coaxA 0 e - coaxA 1 e = Real.sqrt (1 - e.2 ^ 2 - e.1 ^ 2) - Real.sqrt (1 - e.2 ^ 2) := by
        simp only [coaxA]; ring
      rw [this, sgnK_sqrt_sub (by linarith) (by nlinarith)]
      by_cases h0 : e.1 = 0
      · simp [h0]
      · have : (1 - e.2 ^ 2 - e.1 ^ 2 = 1 - e.2 ^ 2) ↔ False := by
          constructor
          · intro h; apply h0; nlinarith
          · exact False.elim
        simp [this, h0]
    have hz : ∀ e e' : ℝ × ℝ, sgnK e.1 = sgnK e'.1 → (e.1 = 0 ↔ e'.1 = 0) := by
      intro e e' h
      rw [← sgnK_zero_iff, ← sgnK_zero_iff (q := e'.1), h]
    have key' : ∀ e : ℝ × ℝ, ‖e‖ < 1 / 2 →
        sgnK (coaxA 1 e - coaxA 0 e) = fS true (sgnK (coaxA 0 e - coaxA 1 e)) := by
      intro e _
      rw [show coaxA 1 e - coaxA 0 e = (coaxA 0 e - coaxA 1 e) * (-1) by ring,
        sgnK_mul_neg (by norm_num)]
    fin_cases k <;> fin_cases l
    · simp
    · simp only [Fin.zero_eta, Fin.isValue, Fin.mk_one]
      rw [key d hd, key d' hd', if_congr (hz d d' hs) rfl rfl]
    · simp only [Fin.zero_eta, Fin.isValue, Fin.mk_one]
      rw [key' d hd, key' d' hd', key d hd, key d' hd', if_congr (hz d d' hs) rfl rfl]
    · simp

theorem coaxPair_cont : ∀ c ∈ coaxPair, Continuous c := by
  intro c hc
  simp only [coaxPair, List.mem_cons, List.not_mem_nil, or_false] at hc
  rcases hc with rfl | rfl | rfl <;> fun_prop

/-- **Invariance**: a homeomorphism keeping every carrier carries local connectivity. -/
theorem localConn_of_invariant {cs : List (Carrier R3 ℝ)} (R : R3 ≃ₜ R3)
    (hR : ∀ c ∈ cs, ∀ p, c (R p) = c p) {x : R3} (h : LocalConn cs x) : LocalConn cs (R x) := by
  have hcov : ∀ p, covOfK cs (R p) = covOfK cs p := by
    intro p; simp only [covOfK]; exact List.map_congr_left fun c hc => by rw [hR c hc]
  intro W hW hxW
  obtain ⟨U, hUo, hUW, hxU, hc⟩ := h (R ⁻¹' W) (hW.preimage R.continuous) hxW
  refine ⟨R.symm ⁻¹' U, hUo.preimage R.symm.continuous, fun y hy => ?_, ?_, fun Y hY => ?_⟩
  · have := hUW hy; simpa using this
  · simpa using hxU
  · rw [hcov] at hY
    have e : cell cs Y ∩ R.symm ⁻¹' U = R '' (cell cs Y ∩ U) := by
      ext y
      constructor
      · rintro ⟨h1, h2⟩
        refine ⟨R.symm y, ⟨?_, h2⟩, by simp⟩
        rw [mem_cell] at h1 ⊢; rw [← hcov, R.apply_symm_apply]; exact h1
      · rintro ⟨p, ⟨h1, h2⟩, rfl⟩
        refine ⟨?_, by simpa using h2⟩
        rw [mem_cell] at h1 ⊢; rw [hcov]; exact h1
    rw [e]
    exact (hc Y hY).image _ R.continuous.continuousOn

/-- Rotation about the `z` axis. -/
noncomputable def rotZ (θ : ℝ) : R3 ≃ₜ R3 where
  toFun p := (Real.cos θ * p.1 - Real.sin θ * p.2.1, Real.sin θ * p.1 + Real.cos θ * p.2.1, p.2.2)
  invFun p := (Real.cos θ * p.1 + Real.sin θ * p.2.1, -Real.sin θ * p.1 + Real.cos θ * p.2.1, p.2.2)
  left_inv p := by
    have := Real.sin_sq_add_cos_sq θ
    ext <;> simp <;> first | linear_combination p.1 * this | linear_combination p.2.1 * this
  right_inv p := by
    have := Real.sin_sq_add_cos_sq θ
    ext <;> simp <;> first | linear_combination p.1 * this | linear_combination p.2.1 * this
  continuous_toFun := by fun_prop
  continuous_invFun := by fun_prop

/-- **Local connectivity at every point of the contact circle.** -/
theorem coaxial_localConn (θ : ℝ) :
    LocalConn coaxPair (Real.cos θ, Real.sin θ, 0) := by
  have h := localConn_of_contact coaxPair_cont coaxialContact
  have hx : rotZ θ (1, 0, 0) = (Real.cos θ, Real.sin θ, 0) := by simp [rotZ]
  rw [← hx]
  refine localConn_of_invariant (rotZ θ) (fun c hc p => ?_) h
  have := Real.sin_sq_add_cos_sq θ
  simp only [coaxPair, List.mem_cons, List.not_mem_nil, or_false] at hc
  rcases hc with rfl | rfl | rfl <;> simp only [rotZ, Homeomorph.homeomorph_mk_coe, Equiv.coe_fn_mk] <;>
    nlinarith [this]

end ForgeCurved
