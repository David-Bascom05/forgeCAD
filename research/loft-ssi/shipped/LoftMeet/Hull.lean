import Mathlib
import LoftMeet.Bernstein

/-!
# loftmeet.rs: exact rational Bezier form of a circle-section span, hull and separation

Models `Poly::new`, `Poly::hull`, `Hull::extent` and `apart` in crates/forge-geom/src/skinned.rs and
loftmeet.rs. Points are `Fin 3 → ℝ`; a separating axis is any linear functional `ℓ`.
-/

namespace LoftSSI

open Finset

/-! ## 1. `Poly::new`: a datum's spline weight in powers of b -/

/-- `Spline::weights`, value row: a = 1 - b, q = h²/6. -/
noncomputable def splineW (yi yj mi mj q b : ℝ) : ℝ :=
  (1 - b) * yi + b * yj + ((((1 - b) ^ 3 - (1 - b)) * mi + (b ^ 3 - b) * mj) * q)

/-- `Poly::new`'s `weight(d)`: `[yi, yj - yi - q(2mi + mj), 3 q mi, q (mj - mi)]` by powers of b. -/
noncomputable def polyW (yi yj mi mj q b : ℝ) : ℝ :=
  yi + (yj - yi - q * (2 * mi + mj)) * b + 3 * q * mi * b ^ 2 + q * (mj - mi) * b ^ 3

/-- The power form the code sums is the spline weight, for ANY second derivatives m: nothing here
uses that m solves the natural (or clamped) spline system, nor that the weights sum to one. -/
theorem polyW_eq (yi yj mi mj q b : ℝ) : polyW yi yj mi mj q b = splineW yi yj mi mj q b := by
  unfold polyW splineW; ring

/-! ## 2. Arcs with one sweep: S = A(b) + B(b) cos a + D(b) sin a -/

/-- One arc rail: origin + r (x cos(st + sw t) + y sin(st + sw t)). -/
noncomputable def arcPt (o x y : Fin 3 → ℝ) (r st sw t : ℝ) : Fin 3 → ℝ :=
  fun c => o c + r * (x c * Real.cos (st + sw * t) + y c * Real.sin (st + sw * t))

/-- `Poly::new` with every rail an arc of the same (start, sweep): summing the weights against the
rails is the code's A + B cos a + D sin a with A = Σ w o, B = Σ w r x, D = Σ w r y. -/
theorem sum_arcs {ι : Type*} (s : Finset ι) (w r : ι → ℝ) (o x y : ι → Fin 3 → ℝ) (st sw t : ℝ) :
    (fun c => ∑ d ∈ s, w d * arcPt (o d) (x d) (y d) (r d) st sw t c) =
      fun c => (∑ d ∈ s, w d * o d c) + (∑ d ∈ s, w d * (r d * x d c)) * Real.cos (st + sw * t) +
        (∑ d ∈ s, w d * (r d * y d c)) * Real.sin (st + sw * t) := by
  funext c
  simp only [arcPt, sum_mul, ← sum_add_distrib]
  refine sum_congr rfl fun d _ => ?_
  ring

/-- Weakening (assumption not needed): equal START is not needed, only equal |sweep|. An arc with
its own start α and sweep σ·ω (σ = ±1) is o + r(x' cos(ωt) + y' sin(ωt)) with the rotated
x' = x cos α + y sin α, y' = σ(y cos α - x sin α); so the span still has the form A + B cos + D sin. -/
theorem arc_any_start (o x y : Fin 3 → ℝ) (r α ω t : ℝ) (σ : ℝ) (hσ : σ = 1 ∨ σ = -1) :
    arcPt o x y r α (σ * ω) t = arcPt o (fun c => x c * Real.cos α + y c * Real.sin α)
      (fun c => σ * (y c * Real.cos α - x c * Real.sin α)) r 0 ω t := by
  funext c
  simp only [arcPt, zero_add]
  rcases hσ with rfl | rfl
  · rw [one_mul, Real.cos_add, Real.sin_add]; ring
  · have e : α + -1 * ω * t = α - ω * t := by ring
    rw [e, Real.cos_sub, Real.sin_sub]; ring

/-! ## 3. `Poly::hull`, the v direction: Bernstein points of a cubic over [b0, b1] -/

noncomputable def cub (c0 c1 c2 c3 b : ℝ) : ℝ := ((c3 * b + c2) * b + c1) * b + c0
noncomputable def dcub (c1 c2 c3 b : ℝ) : ℝ := (c3 * (3 * b) + c2 * 2) * b + c1

/-- The code's four points: val(b0), val(b0) + der(b0)·third, val(b1) - der(b1)·third, val(b1). -/
noncomputable def bpts (c0 c1 c2 c3 b0 b1 : ℝ) : ℕ → ℝ
  | 0 => cub c0 c1 c2 c3 b0
  | 1 => cub c0 c1 c2 c3 b0 + dcub c1 c2 c3 b0 * ((b1 - b0) / 3)
  | 2 => cub c0 c1 c2 c3 b1 - dcub c1 c2 c3 b1 * ((b1 - b0) / 3)
  | _ => cub c0 c1 c2 c3 b1

/-- Exact for every interval: the cubic at b0 + (b1 - b0) σ is the Bernstein form of the points. -/
theorem bpts_exact (c0 c1 c2 c3 b0 b1 σ : ℝ) :
    cub c0 c1 c2 c3 (b0 + (b1 - b0) * σ) = bern 3 (bpts c0 c1 c2 c3 b0 b1) σ := by
  simp only [bern, bb, sum_range_succ, sum_range_zero, bpts, cub, dcub]
  norm_num [Nat.choose]
  ring

/-! ## 4. `Poly::hull`, the angle direction: rational quadratic, weights 1, cos h, 1 -/

/-- Homogeneous rational quadratic of the arc centred on angle 0 with half-turn h, in z = 2τ - 1:
weight W and numerator (Nx, Ny), as the code's control points (cos h, ∓sin h), (1/cos h, 0) with
weights 1, cos h, 1 give them (Bernstein degree 2, τ = (1 + z)/2). -/
noncomputable def arcW (h z : ℝ) : ℝ := ((1 + z ^ 2) + (1 - z ^ 2) * Real.cos h) / 2
noncomputable def arcNx (h z : ℝ) : ℝ := ((1 + z ^ 2) * Real.cos h + (1 - z ^ 2)) / 2
noncomputable def arcNy (h z : ℝ) : ℝ := z * Real.sin h

/-- The code's Bernstein sums equal arcW/arcNx/arcNy (the weights are 1, cos h, 1 and the middle
point is (1/cos h, 0), so its weighted coordinate is (1, 0)). -/
theorem arc_bernstein (h τ : ℝ) :
    bern 2 (fun j => if j = 1 then Real.cos h else 1) τ = arcW h (2 * τ - 1) ∧
    bern 2 (fun j => if j = 1 then 1 else Real.cos h) τ = arcNx h (2 * τ - 1) ∧
    bern 2 (fun j => if j = 0 then -Real.sin h else if j = 1 then 0 else Real.sin h) τ =
      arcNy h (2 * τ - 1) := by
  simp only [bern, bb, sum_range_succ, sum_range_zero, arcW, arcNx, arcNy]
  norm_num [Nat.choose]
  refine ⟨by ring, by ring, by ring⟩

theorem arcW_pos {h z : ℝ} (hc : -1 < Real.cos h) (_hz : z ^ 2 ≤ 1) : 0 < arcW h z := by
  unfold arcW
  have := Real.cos_le_one h
  nlinarith [sq_nonneg z, mul_nonneg (sq_nonneg z) (show 0 ≤ 1 - Real.cos h by linarith)]

/-- Exactness, surface ⊆ patch: for a piece turning less than a full turn (|h| < π; the weight
cos h may be zero or negative here) every angle φ in [-|h|, |h|] is hit at some z ∈ [-1, 1]:
W·(cos φ, sin φ) = (Nx, Ny) with W > 0. -/
theorem arc_hits {h φ : ℝ} (hh : |h| < Real.pi) (hφ : |φ| ≤ |h|) :
    ∃ z, z ^ 2 ≤ 1 ∧ 0 < arcW h z ∧
      arcW h z * Real.cos φ = arcNx h z ∧ arcW h z * Real.sin φ = arcNy h z := by
  have hpi := Real.pi_pos
  rcases eq_or_ne h 0 with rfl | h0
  · refine ⟨0, by norm_num, arcW_pos (by simp) (by norm_num), ?_⟩
    have : φ = 0 := by simpa using hφ
    subst this; simp [arcW, arcNx, arcNy]
  -- half angles: T = tan(h/2) ≠ 0, t = tan(φ/2), z = t / T
  have ch2 : 0 < Real.cos (h / 2) := Real.cos_pos_of_mem_Ioo ⟨by
    linarith [neg_abs_le h], by linarith [le_abs_self h]⟩
  have cp2 : 0 < Real.cos (φ / 2) := Real.cos_pos_of_mem_Ioo ⟨by
    linarith [neg_abs_le φ, abs_nonneg h], by linarith [le_abs_self φ]⟩
  have hc : -1 < Real.cos h := by
    rw [show h = 2 * (h / 2) by ring, Real.cos_two_mul]; nlinarith
  set C := Real.cos (h / 2); set S := Real.sin (h / 2)
  set c := Real.cos (φ / 2); set s := Real.sin (φ / 2)
  have hS : S ≠ 0 := by
    intro hS0
    have hsin : Real.sin h = 0 := by
      rw [show h = 2 * (h / 2) by ring, Real.sin_two_mul]; simp [S] at hS0; simp [hS0]
    rcases Real.sin_eq_zero_iff_of_lt_of_lt (by linarith [neg_abs_le h]) (by
      linarith [le_abs_self h]) |>.mp hsin with h' ; exact h0 h'
  -- |tan(φ/2)| ≤ |tan(h/2)|, via cos(φ/2) ≥ cos(h/2) > 0
  have p1 : c ^ 2 + s ^ 2 = 1 := Real.cos_sq_add_sin_sq _
  have p2 : C ^ 2 + S ^ 2 = 1 := Real.cos_sq_add_sin_sq _
  have cle : C ≤ c := by
    simp only [C, c]
    rw [← Real.cos_abs (φ / 2), ← Real.cos_abs (h / 2)]
    apply Real.cos_le_cos_of_nonneg_of_le_pi (abs_nonneg _) (by
      rw [abs_div]; norm_num; linarith)
    rw [abs_div, abs_div]; norm_num; linarith
  have key : (s / c) ^ 2 ≤ (S / C) ^ 2 := by
    rw [div_pow, div_pow, div_le_div_iff₀ (by positivity) (by positivity)]
    have : C ^ 2 ≤ c ^ 2 := by nlinarith
    nlinarith
  have e1 : Real.cos φ = c ^ 2 - s ^ 2 := by
    rw [show φ = 2 * (φ / 2) by ring, Real.cos_two_mul, ← Real.cos_sq_add_sin_sq (φ / 2)]; ring
  have e2 : Real.sin φ = 2 * s * c := by
    rw [show φ = 2 * (φ / 2) by ring, Real.sin_two_mul]
  have e3 : Real.cos h = C ^ 2 - S ^ 2 := by
    rw [show h = 2 * (h / 2) by ring, Real.cos_two_mul, ← Real.cos_sq_add_sin_sq (h / 2)]; ring
  have e4 : Real.sin h = 2 * S * C := by
    rw [show h = 2 * (h / 2) by ring, Real.sin_two_mul]
  set z := (s / c) / (S / C) with hzdef
  have hz : z * (c * S) = s * C := by rw [hzdef]; field_simp
  refine ⟨z, ?_, ?_, ?_, ?_⟩
  · rw [hzdef, div_pow, div_le_one (by positivity)]; exact key
  · apply arcW_pos hc; rw [hzdef, div_pow, div_le_one (by have : C ≠ 0 := ch2.ne'; positivity)]; exact key
  · unfold arcW arcNx; rw [e1, e3]
    apply mul_left_cancel₀ (show c ^ 2 * S ^ 2 ≠ 0 by positivity)
    linear_combination ((C*s + S*c*z)*(C^2 - S^2 - 1)*(-c^2 + s^2 - 1)/2) * hz
      + (-(C^4*s^2 - C^2*S^2*c^2 + C^2*S^2*s^2 - C^2*s^2 + S^4*c^2 - 2*S^4*s^2 - S^2*c^2
        + 2*S^2*s^2)/2) * p1 + (s^2*(C - S)*(C + S)*(s - 1)*(s + 1)) * p2
  · unfold arcW arcNy; rw [e2, e3, e4]
    apply mul_left_cancel₀ (show c * S ^ 2 ≠ 0 by positivity)
    linear_combination (-C^3*s^2 - C^2*S*c*s*z + C*S^2*s^2 - 2*C*S^2 + C*s^2 + S^3*c*s*z
      + S*c*s*z) * hz + (S^2*s*(C^2 - S^2 + 1)) * p1 + (-s*(C^2*s^2 - S^2*s^2 + S^2)) * p2

end LoftSSI
