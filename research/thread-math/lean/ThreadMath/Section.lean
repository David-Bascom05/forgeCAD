import Mathlib.Analysis.SpecialFunctions.Trigonometric.Deriv
import Mathlib.Analysis.Calculus.Deriv.MeanValue
import Mathlib.Tactic

/-!
# The closed forms of `kernel/forge-geom/src/screw/along.rs`

A point of the curve, read in turn k's half plane of a screw motion (a, b):
  q_k(t) = (r0 + a2 t - a u, h0 + b2 t - b u),  u = u0 + w t + 2 pi k.
-/

set_option linter.style.header false
set_option linter.style.longLine false
set_option linter.unusedSimpArgs false

namespace ThreadMath

/-- Linear in t, turn k shifted by a fixed step: `Moving`, `each_turn` and the fence's loop. -/
theorem section_along_line (nx ny c r0 h0 u0 a b a2 b2 w K t : ℝ) :
    let u := u0 + w * t + K
    nx * (r0 + a2 * t - a * u) + ny * (h0 + b2 * t - b * u) - c
      = (nx * (r0 - a * u0) + ny * (h0 - b * u0) - c)
        + (nx * (a2 - a * w) + ny * (b2 - b * w)) * t - (a * nx + b * ny) * K := by
  intro u; simp only [u]; ring

/-- A curve of the surface's own motion (a2 = a, b2 = b, w = 1) has no slope: it never crosses,
whatever the number of turns. This one case was two thirds of the old search's work. -/
theorem own_motion_is_flat (nx ny a b : ℝ) : nx * (a - a * 1) + ny * (b - b * 1) = 0 := by ring

/-- `linear`, the skip: a line value outside the band at both ends, same side, is outside it all
the way between. Needs: the same side. Not needed: b ≠ 0, m ≥ 0 (see `Counter`). -/
theorem linear_skip {a b x0 x1 m t : ℝ} (ht : x0 ≤ t ∧ t ≤ x1)
    (h : (m < a + b * x0 ∧ m < a + b * x1) ∨ (a + b * x0 < -m ∧ a + b * x1 < -m)) :
    m < |a + b * t| := by
  have hx : x0 ≤ x1 := ht.1.trans ht.2
  rcases eq_or_lt_of_le hx with e | hlt
  · have : t = x0 := le_antisymm (e ▸ ht.2) ht.1
    subst this
    rcases h with h | h
    · exact lt_of_lt_of_le h.1 (le_abs_self _)
    · exact lt_of_lt_of_le (by linarith [h.1]) (neg_le_abs _)
  · set l := (t - x0) / (x1 - x0)
    have hl0 : 0 ≤ l := div_nonneg (by linarith [ht.1]) (by linarith)
    have hl1 : l ≤ 1 := (div_le_one (by linarith)).mpr (by linarith [ht.2])
    have e : a + b * t = (1 - l) * (a + b * x0) + l * (a + b * x1) := by
      have : t = x0 + l * (x1 - x0) := by simp only [l]; field_simp; ring
      rw [this]; ring
    rcases h with h | h
    · have hm : m < min (a + b * x0) (a + b * x1) := lt_min h.1 h.2
      have h0 := mul_nonneg (sub_nonneg.mpr hl1)
        (sub_nonneg.mpr (min_le_left (a + b * x0) (a + b * x1)))
      have h1 := mul_nonneg hl0 (sub_nonneg.mpr (min_le_right (a + b * x0) (a + b * x1)))
      have : m < a + b * t := by rw [e]; nlinarith
      exact lt_of_lt_of_le this (le_abs_self _)
    · have hm : max (a + b * x0) (a + b * x1) < -m := max_lt h.1 h.2
      have h0 := mul_nonneg (sub_nonneg.mpr hl1)
        (sub_nonneg.mpr (le_max_left (a + b * x0) (a + b * x1)))
      have h1 := mul_nonneg hl0 (sub_nonneg.mpr (le_max_right (a + b * x0) (a + b * x1)))
      have : a + b * t < -m := by rw [e]; nlinarith
      exact lt_of_lt_of_le (by linarith) (neg_le_abs _)

/-- `linear`, the clear crossing: opposite signs at the ends, the one root is -a / b. -/
theorem linear_root {a b x0 x1 : ℝ} (h0 : a + b * x0 < 0) (h1 : 0 < a + b * x1) (hx : x0 < x1) :
    0 < b ∧ x0 < -a / b ∧ -a / b < x1 ∧ ∀ t, a + b * t = 0 ↔ t = -a / b := by
  have hb : 0 < b := by nlinarith
  refine ⟨hb, ?_, ?_, ?_⟩
  · rw [lt_div_iff₀ hb]; nlinarith
  · rw [div_lt_iff₀ hb]; nlinarith
  · intro t; constructor
    · intro e; field_simp; linarith
    · intro e; rw [e]; field_simp; ring

/-- The search's reach: a value more than `reach` from zero everywhere leaves no leaf, so no hit.
With rounding: if the computed value is within E of the exact one and past 1.6 tolerances, the
exact value is past 1.5 tolerances as long as E ≤ tol / 10, the code's `MISS` margin. -/
theorem miss_margin {s shat tol E : ℝ} (ht : 0 < tol) (herr : |shat - s| ≤ E) (hE : E ≤ tol / 10)
    (hmiss : 1.6 * tol < |shat|) : 1.5 * tol < |s| := by
  have : |shat| ≤ |s| + |shat - s| := by
    have := abs_add_le s (shat - s); simpa using this
  linarith

/-- What fits in that margin: `coaxial` lets the axes differ by 1e-9 mm and 1e-13 rad, which over the
model's 1e6 mm lever is 1.01e-7, and the section value's dozen products and sums over terms of at most
4e6 mm round to at most 12 * 2^-53 * 4e6 < 6e-9. Together under tol / 10 = 1e-6. -/
theorem margin_budget : (1e-9 + 1e-13 * 1e6 + 12 * (1 / 2 ^ 53) * 4e6 : ℝ) ≤ 1e-5 / 10 := by
  norm_num

/-! ## An arc of the track -/

/-- `arc_piece`: the squared distance from the circle's centre along the moving point. -/
theorem arc_square (rx ry vx vy t : ℝ) (hv : 0 < vx ^ 2 + vy ^ 2) :
    (rx + vx * t) ^ 2 + (ry + vy * t) ^ 2
      = (rx * vy - ry * vx) ^ 2 / (vx ^ 2 + vy ^ 2)
        + (vx ^ 2 + vy ^ 2) * (t + (rx * vx + ry * vy) / (vx ^ 2 + vy ^ 2)) ^ 2 := by
  field_simp; ring

/-- `arc_piece`, the skip: a line whose nearest approach h to the centre is past r + m stays more
than m off the circle: its distance from the centre never falls below h. -/
theorem arc_skip {rx ry vx vy t r m : ℝ} (hv : 0 < vx ^ 2 + vy ^ 2) (hr : 0 ≤ r) (hm : 0 ≤ m)
    (hh : (r + m) ^ 2 < (rx * vy - ry * vx) ^ 2 / (vx ^ 2 + vy ^ 2)) :
    (r + m) ^ 2 < (rx + vx * t) ^ 2 + (ry + vy * t) ^ 2 := by
  rw [arc_square rx ry vx vy t hv]
  have : 0 ≤ (vx ^ 2 + vy ^ 2) * (t + (rx * vx + ry * vy) / (vx ^ 2 + vy ^ 2)) ^ 2 := by positivity
  linarith

/-- `arc_piece`, the roots: at t0 ± sqrt(r² - h²) / speed the point is on the circle. -/
theorem arc_roots {rx ry vx vy r : ℝ} (hv : 0 < vx ^ 2 + vy ^ 2) (s : ℝ)
    (hs : s = 1 ∨ s = -1)
    (hh : (rx * vy - ry * vx) ^ 2 / (vx ^ 2 + vy ^ 2) ≤ r ^ 2) :
    let vv := vx ^ 2 + vy ^ 2
    let h2 := (rx * vy - ry * vx) ^ 2 / vv
    let t := -(rx * vx + ry * vy) / vv + s * Real.sqrt (r ^ 2 - h2) / Real.sqrt vv
    (rx + vx * t) ^ 2 + (ry + vy * t) ^ 2 = r ^ 2 := by
  intro vv h2 t
  have e := arc_square rx ry vx vy t hv
  have hs2 : s ^ 2 = 1 := by rcases hs with rfl | rfl <;> norm_num
  have ht : t + (rx * vx + ry * vy) / vv = s * Real.sqrt (r ^ 2 - h2) / Real.sqrt vv := by
    simp only [t]; ring
  have hsq : (Real.sqrt (r ^ 2 - h2)) ^ 2 = r ^ 2 - h2 := Real.sq_sqrt (by simp only [h2, vv] at *; linarith)
  have hsv : (Real.sqrt vv) ^ 2 = vv := Real.sq_sqrt hv.le
  have hne : Real.sqrt vv ≠ 0 := (Real.sqrt_pos.mpr hv).ne'
  rw [e]
  change h2 + vv * (t + (rx * vx + ry * vy) / vv) ^ 2 = r ^ 2
  rw [ht, div_pow, mul_pow, hs2, hsq, hsv]
  have : vv ≠ 0 := hv.ne'
  field_simp; ring

/-! ## A helix against any plane: `helix_plane` -/

/-- Where G t + D alone is past A + m, the plane is more than m away: only those turns can hold a
root. Needs: 0 ≤ A is not even needed for the cosine's bound, only |A cos| ≤ |A|. -/
theorem slab {A th G D m t : ℝ} (h : |A| + m < |G * t + D|) :
    m < |A * Real.cos (t - th) + G * t + D| := by
  have hc : |A * Real.cos (t - th)| ≤ |A| := by
    rw [abs_mul]; exact mul_le_of_le_one_right (abs_nonneg _) (Real.abs_cos_le_one _)
  have : |G * t + D| ≤ |A * Real.cos (t - th) + G * t + D| + |A * Real.cos (t - th)| := by
    have e : G * t + D = (A * Real.cos (t - th) + G * t + D) - A * Real.cos (t - th) := by ring
    calc |G * t + D| = |(A * Real.cos (t - th) + G * t + D) - A * Real.cos (t - th)| := by rw [← e]
      _ ≤ _ := abs_sub _ _
  linarith

theorem helix_plane_deriv (A th G D t : ℝ) :
    HasDerivAt (fun t => A * Real.cos (t - th) + G * t + D) (-A * Real.sin (t - th) + G) t := by
  have h1 : HasDerivAt (fun t : ℝ => t - th) 1 t := (hasDerivAt_id t).sub_const th
  have h2 := (Real.hasDerivAt_cos (t - th)).comp t h1
  have h3 := (h2.const_mul A).add ((hasDerivAt_id t).const_mul G)
  have h4 := h3.add_const D
  convert h4 using 1
  simp; ring

/-- Between two neighbouring turning points f' keeps its sign, so a root there is alone: the code
needs one root per bracket. Needs only that f' does not vanish on the open interval; A, G any. -/
theorem one_root_between {A th G D x0 x1 : ℝ}
    (hne : ∀ t ∈ Set.Ioo x0 x1, -A * Real.sin (t - th) + G ≠ 0) :
    Set.InjOn (fun t => A * Real.cos (t - th) + G * t + D) (Set.Icc x0 x1) := by
  intro p hp q hq hpq
  by_contra hne'
  wlog hlt : p < q generalizing p q
  · exact this hq hp hpq.symm (Ne.symm hne') (lt_of_le_of_ne (not_lt.mp hlt) (Ne.symm hne'))
  have hsub : Set.Icc p q ⊆ Set.Icc x0 x1 := Set.Icc_subset_Icc hp.1 hq.2
  obtain ⟨c, hc, hd⟩ := exists_hasDerivAt_eq_zero (f := fun t => A * Real.cos (t - th) + G * t + D)
    (f' := fun t => -A * Real.sin (t - th) + G) hlt
    ((by fun_prop : Continuous fun t => A * Real.cos (t - th) + G * t + D).continuousOn)
    hpq (fun t _ => helix_plane_deriv A th G D t)
  exact hne c ⟨lt_of_le_of_lt hp.1 hc.1, lt_of_lt_of_le hc.2 hq.2⟩ hd

/-- The turning points are where the code says: sin(t - th) = G / A. -/
theorem turning_points {A th G t : ℝ} (hA : A ≠ 0) :
    -A * Real.sin (t - th) + G = 0 ↔ Real.sin (t - th) = G / A := by
  constructor
  · intro h; field_simp; linarith
  · intro h; rw [h]; field_simp; ring

end ThreadMath
