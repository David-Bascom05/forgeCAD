import Mathlib.Analysis.SpecialFunctions.Trigonometric.Bounds
import Mathlib.Analysis.SpecialFunctions.Complex.Circle

/-!
# shape.rs's margin: a screen answers only clear of the boundary (flat G11, class S)

research/decide-once/flat-g11.md classes three of the 24 busiest flat tolerance sites **S** on
shape.rs's margin (kernel/forge-bool/src/shape.rs: "The answer is In or Out only when the point is
clear of the boundary by a margin; nearer, it is Near and the caller asks the B-rep"):

* shape.rs:234 `on_sweep`: an arc whose sweep is within 1e-12 rad of a whole turn is taken whole;
* shape.rs:131 the ray's arc-end graze test, skipped for such an arc;
* nary.rs:1073 `box_out` at `SHAPE_MARGIN`: a new edge whose box the shapes put surely outside the
  result is dropped.

What makes the first two screens sound:

* `screen_same`: two regions that differ only within `ε` of a boundary `B` agree at every point
  farther than `ε` from `B`. The profile taken with the arc whole differs from the true one only in
  the arc's missing piece and the closing segments at its two ends, all within the chord of the gap;
  a point the screen answers In or Out of is farther than the margin from the boundary.
* `arc_chord`: two points of a circle of radius `r` at angles `a` and `b` are at most `r |a - b|`
  apart, so the missing piece of an arc within `g` rad of whole lies within `r g` of its end.
* `gap_within_margin`: for `r` up to the model's half size (`tol::MODEL_HALF_SIZE` = 5e5 mm),
  `r · 1e-12 ≤ 5e-7 < LINEAR` = 1e-5 = `SHAPE_MARGIN`. (Under the input bound InputSize's 1.6e7
  instead, `r · 1e-12 ≤ 1.6e-5`, past the margin: the screen needs the model half size, the bound
  the kernel already holds parts to, research/decide-once ledger G.param.)

And of the third:

* `apart_far`: two intervals apart by more than `m` (`hi < plo - m`) keep every pair of their points
  more than `m` apart. `box_out` tests exactly that, axis by axis, on a box holding the edge and one
  holding the shape; that those boxes hold them (`local_box`, `profile_box` and the margin they add
  for rounding, `1e-9 (1 + diagonal)`) is the code's own obligation, not proven here.

Owed: that shape.rs's parity, computed in floating point, is the exact one at a point farther than
the margin from the boundary (the margin against the evaluation error, under the model half size).
-/

namespace ForgeBoolProof.FlatSites.ShapeMargin

open Complex

/-- **A screen answers alike for two regions near one boundary.** If every point where `R` and `R'`
disagree is within `ε` of `B`, a point farther than `ε` from every point of `B` is in both or in
neither. -/
theorem screen_same {E : Type} [PseudoMetricSpace E] {R R' B : Set E} {ε : ℝ} {p : E}
    (hΔ : ∀ x, (x ∈ R ↔ x ∉ R') → ∃ b ∈ B, dist x b ≤ ε) (hp : ∀ b ∈ B, ε < dist p b) :
    (p ∈ R ↔ p ∈ R') := by
  by_contra h
  have h' : p ∈ R ↔ p ∉ R' := by tauto
  obtain ⟨b, hb, hd⟩ := hΔ p h'
  exact absurd hd (not_le.mpr (hp b hb))

/-- **A circle's chord is no longer than its arc**: points at angles `a` and `b` on a circle of
radius `r ≥ 0` (in the plane, as `ℂ`) are at most `r |a - b|` apart. -/
theorem arc_chord {r : ℝ} (hr : 0 ≤ r) (c : ℂ) (a b : ℝ) :
    ‖(c + r * exp (I * a)) - (c + r * exp (I * b))‖ ≤ r * |a - b| := by
  have hsplit : (c + r * exp (I * a)) - (c + r * exp (I * b))
      = (r * exp (I * b)) * (exp (I * ((a - b : ℝ) : ℂ)) - 1) := by
    have : exp (I * a) = exp (I * b) * exp (I * ((a - b : ℝ) : ℂ)) := by
      rw [← Complex.exp_add]; congr 1; push_cast; ring
    rw [this]; ring
  rw [hsplit, norm_mul, norm_mul, Complex.norm_real, Real.norm_of_nonneg hr]
  have he : ‖exp (I * (b : ℂ))‖ = 1 := by
    rw [mul_comm]; exact Complex.norm_exp_ofReal_mul_I b
  rw [he, mul_one]
  have := Real.norm_exp_I_mul_ofReal_sub_one_le (x := a - b)
  rw [Real.norm_eq_abs] at this
  exact mul_le_mul_of_nonneg_left this hr

/-- **The whole-turn rounding is inside the margin**: an arc of radius at most the model's half
size (5e5 mm) taken whole when within 1e-12 rad of a whole turn moves its boundary by at most
5e-7 mm, under `SHAPE_MARGIN` = `LINEAR` = 1e-5. -/
theorem gap_within_margin {r : ℝ} (hr : r ≤ 5e5) : r * 1e-12 < 1e-5 := by
  nlinarith

/-- **Intervals apart by more than the margin** (`box_out`'s test, per axis): every point of the
first is more than `m` from every point of the second. -/
theorem apart_far {lo hi plo phi m x y : ℝ} (hm : 0 ≤ m) (hx : lo ≤ x ∧ x ≤ hi)
    (hy : plo ≤ y ∧ y ≤ phi) (h : hi < plo - m ∨ lo > phi + m) : m < |x - y| := by
  rcases h with h | h
  · rw [abs_sub_comm, abs_of_pos (by linarith [hx.2, hy.1])]; linarith [hx.2, hy.1]
  · rw [abs_of_pos (by linarith [hx.1, hy.2])]; linarith [hx.1, hy.2]

end ForgeBoolProof.FlatSites.ShapeMargin
