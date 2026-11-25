import Mathlib.Analysis.SpecialFunctions.Trigonometric.Basic

/-!
# The points of an ellipse square to a point (forge-geom `curve.rs` `ellipse_square`)

The ellipse (a cos t, b sin t) is square to the point (x, y) at t where the distance's slope
`slope a b x y (cos t) (sin t)` is zero; the nearest point of an arc is at an end or at such a t.
`Ellipse::closest` takes the nearest of the arc's ends and every such t in the arc, so it is right
when `ellipse_square` gives every such t. This file proves the mathematics it rests on; with
d = a² − b², A = (a x)², B = (b y)²:

* `square_iff`, `root_of_square` (Joachimsthal): off the axes (a x ≠ 0, b y ≠ 0), (c, s) on the
  unit circle is square to the point iff c (d + e) = a x and s e = b y for some e ≠ 0, −d, and that
  e is a root of `G d A B e = A / (d + e)² + B / e² − 1`. Each root fixes (c, s), so the angles
  square to the point are those of G's roots.
* G's roots lie strictly beyond both near ends (`root_far_from_poles`: A < (d + e)², B < e²).
* On e > 0, G is strictly decreasing (`G_anti_pos`), ≥ 0 at e = √B (`G_nonneg_of_sq_le_B`) and
  ≤ 0 at e = √(A + B) (`G_nonpos_pos`): one root, in the code's bracket [V, hypot(U, V)].
* On d + e < 0, strictly increasing (`G_mono_neg`), ≤ 0 at d + e = −√(A + B) (`G_nonpos_neg`),
  ≥ 0 at d + e = −√A (`G_nonneg_of_sq_le_A`): one root, in [−hypot(U, V), −U].
* On (−d, 0), in p = d + e, q = −e (p + q = d), with A = cu³, B = cv³ (cu, cv > 0, the code's
  cbrt(U)², cbrt(V)²): G ≥ (cu + cv)³ / d² − 1 (`evolute_bound`), so no root when
  (cu + cv)³ > d²; G strictly decreasing for p ≤ p₀ = d cu / (cu + cv) (`mid_anti`) and strictly
  increasing for p ≥ p₀ (`mid_mono`): at most one root each side, in the code's brackets
  p ∈ [U, p₀] and q ∈ [V, q₀] (q₀ = d cv / (cu + cv), the code's e ∈ [e*, −V]).
* On the axes, at the centre and for b = 0, the slope factors (`slope_major_axis`,
  `slope_minor_axis`, `slope_centre`, `slope_flat`): the code's closed forms.

Open (ledger C.efoot): the f64 side. `held_newton` returns a double inside each bracket, not the
root; `atan2` and the arithmetic round. Checked by forge-geom's tests: every angle square to the
point to a cosine of 1e-7, no sample of 20 001 nearer, never farther than the old sampler.
-/

namespace ForgeCurved.EllipseFoot

/-- The distance's slope (half the derivative of its square, in t) from (x, y) to the ellipse at
the point (a c, b s), c = cos t, s = sin t. -/
def slope (a b x y c s : ℝ) : ℝ := a * x * s - b * y * c - (a ^ 2 - b ^ 2) * s * c

/-- It is the derivative's: (P − p) · P′ with P = (a cos t, b sin t). -/
theorem slope_eq (a b x y t : ℝ) :
    slope a b x y (Real.cos t) (Real.sin t) =
      (a * Real.cos t - x) * (-(a * Real.sin t)) + (b * Real.sin t - y) * (b * Real.cos t) := by
  unfold slope; ring

/-- Joachimsthal's function, in e = μ + b². -/
noncomputable def G (d A B e : ℝ) : ℝ := A / (d + e) ^ 2 + B / e ^ 2 - 1

/-- **Square iff a root's point.** -/
theorem square_iff {a b x y c s : ℝ} (hx : a * x ≠ 0) (hy : b * y ≠ 0) (hcs : c ^ 2 + s ^ 2 = 1) :
    slope a b x y c s = 0 ↔
      ∃ e : ℝ, e ≠ 0 ∧ (a ^ 2 - b ^ 2) + e ≠ 0 ∧ c * ((a ^ 2 - b ^ 2) + e) = a * x ∧ s * e = b * y := by
  constructor
  · intro h
    unfold slope at h
    have hs : s ≠ 0 := by
      intro hs0
      have hc : c ^ 2 = 1 := by rw [hs0] at hcs; linarith
      have hc0 : c ≠ 0 := by intro h0; rw [h0] at hc; norm_num at hc
      rw [hs0] at h
      have : b * y * c = 0 := by linarith
      rcases mul_eq_zero.mp this with h1 | h1
      · exact hy h1
      · exact hc0 h1
    have hc : c ≠ 0 := by
      intro hc0
      rw [hc0] at h
      have : a * x * s = 0 := by linarith
      rcases mul_eq_zero.mp this with h1 | h1
      · exact hx h1
      · exact hs h1
    refine ⟨b * y / s, div_ne_zero hy hs, ?_, ?_, ?_⟩
    · intro h0
      have : c * ((a ^ 2 - b ^ 2) + b * y / s) = a * x := by
        field_simp
        linear_combination (-1 : ℝ) * h
      rw [h0, mul_zero] at this
      exact hx this.symm
    · field_simp
      linear_combination (-1 : ℝ) * h
    · field_simp
  · rintro ⟨e, _, _, h1, h2⟩
    unfold slope
    rw [← h1, ← h2]
    ring

/-- A root's point is on the ellipse, so its e is a root of G. -/
theorem root_of_square {a b x y c s e : ℝ} (he : e ≠ 0) (hde : (a ^ 2 - b ^ 2) + e ≠ 0)
    (h1 : c * ((a ^ 2 - b ^ 2) + e) = a * x) (h2 : s * e = b * y) (hcs : c ^ 2 + s ^ 2 = 1) :
    G (a ^ 2 - b ^ 2) ((a * x) ^ 2) ((b * y) ^ 2) e = 0 := by
  unfold G
  rw [← h1, ← h2]
  have r1 : (c * ((a ^ 2 - b ^ 2) + e)) ^ 2 / ((a ^ 2 - b ^ 2) + e) ^ 2 = c ^ 2 := by
    rw [mul_pow, mul_div_assoc, div_self (pow_ne_zero 2 hde), mul_one]
  have r2 : (s * e) ^ 2 / e ^ 2 = s ^ 2 := by
    rw [mul_pow, mul_div_assoc, div_self (pow_ne_zero 2 he), mul_one]
  rw [r1, r2]
  linarith

/-- And conversely a root gives a point on the unit circle (c, s) = (a x / (d + e), b y / e). -/
theorem circle_of_root {d A B e ax by_ : ℝ} (hA : ax ^ 2 = A)
    (hB : by_ ^ 2 = B) (h : G d A B e = 0) : (ax / (d + e)) ^ 2 + (by_ / e) ^ 2 = 1 := by
  unfold G at h
  rw [div_pow, div_pow, hA, hB]
  linarith

/-- A root lies strictly beyond both near ends: A < (d + e)² and B < e². -/
theorem root_far_from_poles {d A B e : ℝ} (hA : 0 < A) (hB : 0 < B) (he : e ≠ 0) (hde : d + e ≠ 0)
    (h : G d A B e = 0) : A < (d + e) ^ 2 ∧ B < e ^ 2 := by
  unfold G at h
  have p1 : 0 < (d + e) ^ 2 := by positivity
  have p2 : 0 < e ^ 2 := by positivity
  have q1 : 0 < A / (d + e) ^ 2 := div_pos hA p1
  have q2 : 0 < B / e ^ 2 := div_pos hB p2
  constructor
  · have : A / (d + e) ^ 2 < 1 := by linarith
    rwa [div_lt_one p1] at this
  · have : B / e ^ 2 < 1 := by linarith
    rwa [div_lt_one p2] at this

/-- G ≥ 0 where e² ≤ B (the bracket's end by the pole at 0). -/
theorem G_nonneg_of_sq_le_B {d A B e : ℝ} (hA : 0 ≤ A) (he : e ≠ 0) (h : e ^ 2 ≤ B) :
    0 ≤ G d A B e := by
  unfold G
  have p2 : 0 < e ^ 2 := by positivity
  have : 1 ≤ B / e ^ 2 := by rwa [one_le_div p2]
  have : 0 ≤ A / (d + e) ^ 2 := by positivity
  linarith

/-- G ≥ 0 where (d + e)² ≤ A (the bracket's end by the pole at −d). -/
theorem G_nonneg_of_sq_le_A {d A B e : ℝ} (hB : 0 ≤ B) (hde : d + e ≠ 0) (h : (d + e) ^ 2 ≤ A) :
    0 ≤ G d A B e := by
  unfold G
  have p1 : 0 < (d + e) ^ 2 := by positivity
  have : 1 ≤ A / (d + e) ^ 2 := by rwa [one_le_div p1]
  have : 0 ≤ B / e ^ 2 := by positivity
  linarith

/-- G ≤ 0 at e > 0 with e² ≥ A + B (the nearest point's far bracket end, e = hypot(U, V)). -/
theorem G_nonpos_pos {d A B e : ℝ} (hd : 0 ≤ d) (hA : 0 ≤ A) (he : 0 < e)
    (h : A + B ≤ e ^ 2) : G d A B e ≤ 0 := by
  unfold G
  have p2 : 0 < e ^ 2 := by positivity
  have le : e ^ 2 ≤ (d + e) ^ 2 := by nlinarith
  have t1 : A / (d + e) ^ 2 ≤ A / e ^ 2 := div_le_div_of_nonneg_left hA p2 le
  have t2 : (A + B) / e ^ 2 ≤ 1 := by rwa [div_le_one p2]
  have : A / e ^ 2 + B / e ^ 2 = (A + B) / e ^ 2 := by ring
  linarith

/-- G ≤ 0 at d + e < 0 with (d + e)² ≥ A + B (the farthest point's bracket end). -/
theorem G_nonpos_neg {d A B e : ℝ} (hd : 0 ≤ d) (hB : 0 ≤ B) (hde : d + e < 0)
    (h : A + B ≤ (d + e) ^ 2) : G d A B e ≤ 0 := by
  unfold G
  have p1 : 0 < (d + e) ^ 2 := by nlinarith
  have le : (d + e) ^ 2 ≤ e ^ 2 := by nlinarith
  have t1 : B / e ^ 2 ≤ B / (d + e) ^ 2 := div_le_div_of_nonneg_left hB p1 le
  have t2 : (A + B) / (d + e) ^ 2 ≤ 1 := by rwa [div_le_one p1]
  have : A / (d + e) ^ 2 + B / (d + e) ^ 2 = (A + B) / (d + e) ^ 2 := by ring
  linarith

/-- On e > 0, G strictly decreases: one root there at most. -/
theorem G_anti_pos {d A B e₁ e₂ : ℝ} (hd : 0 ≤ d) (hA : 0 ≤ A) (hB : 0 < B) (h1 : 0 < e₁)
    (h12 : e₁ < e₂) : G d A B e₂ < G d A B e₁ := by
  unfold G
  have p1 : 0 < (d + e₁) ^ 2 := pow_pos (by linarith) 2
  have q1 : 0 < e₁ ^ 2 := by positivity
  have t1 : A / (d + e₂) ^ 2 ≤ A / (d + e₁) ^ 2 :=
    div_le_div_of_nonneg_left hA p1 (by nlinarith)
  have t2 : B / e₂ ^ 2 < B / e₁ ^ 2 := div_lt_div_of_pos_left hB q1 (by nlinarith)
  linarith

/-- On d + e < 0, G strictly increases: one root there at most. -/
theorem G_mono_neg {d A B e₁ e₂ : ℝ} (hd : 0 ≤ d) (hA : 0 < A) (hB : 0 ≤ B) (h2 : d + e₂ < 0)
    (h12 : e₁ < e₂) : G d A B e₁ < G d A B e₂ := by
  unfold G
  have p2 : 0 < (d + e₂) ^ 2 := by nlinarith
  have q2 : 0 < e₂ ^ 2 := by nlinarith
  have t1 : A / (d + e₁) ^ 2 < A / (d + e₂) ^ 2 := div_lt_div_of_pos_left hA p2 (by nlinarith)
  have t2 : B / e₁ ^ 2 ≤ B / e₂ ^ 2 := div_le_div_of_nonneg_left hB q2 (by nlinarith)
  linarith

/-- The tangent line of c / t² (convex on t > 0) lies below it. -/
theorem tangent {c t t₀ : ℝ} (hc : 0 ≤ c) (ht : 0 < t) (ht₀ : 0 < t₀) :
    c / t₀ ^ 2 - 2 * (c / t₀ ^ 3) * (t - t₀) ≤ c / t ^ 2 := by
  have key : c / t ^ 2 - (c / t₀ ^ 2 - 2 * (c / t₀ ^ 3) * (t - t₀)) =
      c * (t - t₀) ^ 2 * (2 * t + t₀) / (t ^ 2 * t₀ ^ 3) := by
    field_simp
    ring
  have : 0 ≤ c * (t - t₀) ^ 2 * (2 * t + t₀) / (t ^ 2 * t₀ ^ 3) := by positivity
  linarith

/-- Strictly, off the point of contact. -/
theorem tangent_strict {c t t₀ : ℝ} (hc : 0 < c) (ht : 0 < t) (ht₀ : 0 < t₀) (hne : t ≠ t₀) :
    c / t₀ ^ 2 - 2 * (c / t₀ ^ 3) * (t - t₀) < c / t ^ 2 := by
  have key : c / t ^ 2 - (c / t₀ ^ 2 - 2 * (c / t₀ ^ 3) * (t - t₀)) =
      c * (t - t₀) ^ 2 * (2 * t + t₀) / (t ^ 2 * t₀ ^ 3) := by
    field_simp
    ring
  have hsq : 0 < (t - t₀) ^ 2 := by
    have : t - t₀ ≠ 0 := sub_ne_zero.mpr hne
    positivity
  have : 0 < c * (t - t₀) ^ 2 * (2 * t + t₀) / (t ^ 2 * t₀ ^ 3) := by positivity
  linarith

/-- At the least point: A / p₀³ = B / q₀³ = (cu + cv)³ / d³. -/
theorem at_least {d cu cv : ℝ} (hd : 0 < d) (hu : 0 < cu) (hv : 0 < cv) :
    cu ^ 3 / (d * cu / (cu + cv)) ^ 3 = (cu + cv) ^ 3 / d ^ 3 ∧
      cv ^ 3 / (d * cv / (cu + cv)) ^ 3 = (cu + cv) ^ 3 / d ^ 3 := by
  have hs : 0 < cu + cv := by linarith
  constructor <;> field_simp

/-- **The evolute.** On (−d, 0), in p = d + e > 0, q = −e > 0: G ≥ (cu + cv)³ / d² − 1. -/
theorem evolute_bound {d cu cv p q : ℝ} (hd : 0 < d) (hu : 0 < cu) (hv : 0 < cv) (hp : 0 < p)
    (hq : 0 < q) (hpq : p + q = d) :
    (cu + cv) ^ 3 / d ^ 2 ≤ cu ^ 3 / p ^ 2 + cv ^ 3 / q ^ 2 := by
  have hs : 0 < cu + cv := by linarith
  set p₀ := d * cu / (cu + cv) with hp₀
  set q₀ := d * cv / (cu + cv) with hq₀
  have hp₀pos : 0 < p₀ := by positivity
  have hq₀pos : 0 < q₀ := by positivity
  have t1 := tangent (c := cu ^ 3) (by positivity) hp hp₀pos
  have t2 := tangent (c := cv ^ 3) (by positivity) hq hq₀pos
  obtain ⟨e1, e2⟩ := at_least hd hu hv
  rw [← hp₀] at e1
  rw [← hq₀] at e2
  have pq₀ : p₀ + q₀ = d := by
    rw [hp₀, hq₀, ← add_div, ← mul_add, mul_div_assoc, div_self hs.ne', mul_one]
  have sum0 : (p - p₀) + (q - q₀) = 0 := by linarith
  have v0 : cu ^ 3 / p₀ ^ 2 + cv ^ 3 / q₀ ^ 2 = (cu + cv) ^ 3 / d ^ 2 := by
    rw [hp₀, hq₀]; field_simp
  rw [e1] at t1
  rw [e2] at t2
  have h0 : 2 * ((cu + cv) ^ 3 / d ^ 3) * (p - p₀) + 2 * ((cu + cv) ^ 3 / d ^ 3) * (q - q₀) = 0 := by
    rw [← mul_add, sum0, mul_zero]
  linarith

/-- So no root on (−d, 0) outside the evolute. -/
theorem no_mid_root {d cu cv p q : ℝ} (hd : 0 < d) (hu : 0 < cu) (hv : 0 < cv) (hp : 0 < p)
    (hq : 0 < q) (hpq : p + q = d) (hout : d ^ 2 < (cu + cv) ^ 3) :
    1 < cu ^ 3 / p ^ 2 + cv ^ 3 / q ^ 2 := by
  have := evolute_bound hd hu hv hp hq hpq
  have : 1 < (cu + cv) ^ 3 / d ^ 2 := by rw [one_lt_div (by positivity)]; exact hout
  linarith

/-- On the side p ≤ p₀ (by the pole at −d) G strictly decreases in p. -/
theorem mid_anti {d cu cv p₁ p₂ : ℝ} (hd : 0 < d) (hu : 0 < cu) (hv : 0 < cv) (h1 : 0 < p₁)
    (h12 : p₁ < p₂) (h2 : p₂ ≤ d * cu / (cu + cv)) :
    cu ^ 3 / p₂ ^ 2 + cv ^ 3 / (d - p₂) ^ 2 < cu ^ 3 / p₁ ^ 2 + cv ^ 3 / (d - p₁) ^ 2 := by
  have hs : 0 < cu + cv := by linarith
  have hp₂ : 0 < p₂ := by linarith
  have hlt : d * cu / (cu + cv) < d := by
    rw [div_lt_iff₀ hs]; nlinarith
  have hq₂ : 0 < d - p₂ := by linarith
  have hq₁ : 0 < d - p₁ := by linarith
  -- the tangent lines at p₂ and q₂ = d − p₂
  have t1 := tangent_strict (c := cu ^ 3) (by positivity) h1 hp₂ (ne_of_lt h12)
  have t2 := tangent (c := cv ^ 3) (by positivity) hq₁ hq₂
  -- the slopes: A / p₂³ ≥ (cu + cv)³ / d³ ≥ B / q₂³
  obtain ⟨e1, e2⟩ := at_least hd hu hv
  have s1 : (cu + cv) ^ 3 / d ^ 3 ≤ cu ^ 3 / p₂ ^ 3 := by
    rw [← e1]
    exact div_le_div_of_nonneg_left (by positivity) (by positivity) (pow_le_pow_left₀ hp₂.le h2 3)
  have hq₀ : d * cv / (cu + cv) ≤ d - p₂ := by
    have : d * cv / (cu + cv) = d - d * cu / (cu + cv) := by field_simp; ring
    linarith
  have s2 : cv ^ 3 / (d - p₂) ^ 3 ≤ (cu + cv) ^ 3 / d ^ 3 := by
    rw [← e2]
    exact div_le_div_of_nonneg_left (by positivity) (by positivity) (pow_le_pow_left₀ (by positivity) hq₀ 3)
  have hΔ : 0 < p₂ - p₁ := by linarith
  have : (d - p₁) - (d - p₂) = p₂ - p₁ := by ring
  nlinarith

/-- On the side p ≥ p₀ (by the pole at 0) G strictly increases in p. -/
theorem mid_mono {d cu cv p₁ p₂ : ℝ} (hd : 0 < d) (hu : 0 < cu) (hv : 0 < cv)
    (h1 : d * cu / (cu + cv) ≤ p₁) (h12 : p₁ < p₂) (h2 : p₂ < d) :
    cu ^ 3 / p₁ ^ 2 + cv ^ 3 / (d - p₁) ^ 2 < cu ^ 3 / p₂ ^ 2 + cv ^ 3 / (d - p₂) ^ 2 := by
  -- the mirror of `mid_anti`, in q = d − p, with cu and cv exchanged
  have hs : 0 < cu + cv := by linarith
  have hq : d * cv / (cv + cu) ≥ d - p₁ := by
    have : d * cv / (cv + cu) = d - d * cu / (cu + cv) := by field_simp; ring
    linarith
  have := mid_anti (d := d) (cu := cv) (cv := cu) (p₁ := d - p₂) (p₂ := d - p₁) hd hv hu
    (by linarith) (by linarith) hq
  have r1 : d - (d - p₁) = p₁ := by ring
  have r2 : d - (d - p₂) = p₂ := by ring
  rw [r1, r2] at this
  linarith

/-- On the major axis (y = 0): s = 0, or d c = a x. -/
theorem slope_major_axis (a b x c s : ℝ) :
    slope a b x 0 c s = s * (a * x - (a ^ 2 - b ^ 2) * c) := by
  unfold slope; ring

/-- On the minor axis (x = 0): c = 0, or d s = −b y. -/
theorem slope_minor_axis (a b y c s : ℝ) :
    slope a b 0 y c s = -c * (b * y + (a ^ 2 - b ^ 2) * s) := by
  unfold slope; ring

/-- At the centre: s = 0 or c = 0 (or every t, for a circle). -/
theorem slope_centre (a b c s : ℝ) : slope a b 0 0 c s = -((a ^ 2 - b ^ 2) * s * c) := by
  unfold slope; ring

/-- Flat (b = 0, a segment run twice): s = 0, or a c = x. -/
theorem slope_flat (a x y c s : ℝ) : slope a 0 x y c s = a * s * (x - a * c) := by
  unfold slope; ring

end ForgeCurved.EllipseFoot
