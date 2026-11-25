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
  farther than `ε` from `B`: a screen that answers In or Out only clear of the boundary by the
  margin answers the true region wherever the region it computes differs from it only within the
  margin of the boundary.
* What governs the whole-turn branch is not the model's size: no arc a shape holds is within
  1e-12 rad of whole unless it is whole. Every `Seg2::Arc` of a shape comes from a profile
  `normalized` by forge-ops (extrude, revolve, holes, coils; patterns and mirrors keep `|sweep|`),
  and `Segment::degenerate` (forge-ops profile.rs) refuses an arc within `ANGULAR` = 1e-11 rad of
  a whole turn, at any radius; a whole turn is a circle, sweep exactly `2π` and alone in its loop
  (`CircleNotAlone`). So the branch fires only where `R = R'` (ε = 0 in `screen_same`). The
  condition matters: with a closing segment, the branch would count the arc across its gap and
  the segment both, and the disc's centre would read Out at any radius (1 mm as at 16 km); tests
  kernel/forge-bool/tests/shape_whole_turn.rs and interface/forge-lang/tests/whole_turn_size.rs
  hold the refusal at 1 mm, 5e5, 1.2e7 and `InputSize`'s 1.6e7, and the exact answer just past it.
* `arc_chord` and `gap_within_margin` bound the boundary's move if an arc within `g` of whole
  taken whole on its own (its gap closed by nothing): `r g`, under the margin for `r` up to the
  model half size. Not needed by the code as it stands (above), kept as the bound a whole-turn
  tolerance on a lone arc would owe.

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
