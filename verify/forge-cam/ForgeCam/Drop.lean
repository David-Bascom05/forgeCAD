/-
3-axis surfacing's checker (kernel/forge-cam-geom/src/drop.rs), L1.

A 3-axis cutter, axis +z, tip at the origin, has a lower profile b on [0, R] (non-decreasing
where the generators need it, never here). Its *column* is

  column R b = {p | ρ p ≤ R ∧ b (ρ p) ≤ p z}.

drop.rs proves a move clear by bounding the signed distance of the part's faces to the column
(`gap_move`), and uses it for every tool body inside the column (the cutter, a shank no wider
than it). The statements here:

* `sd_antitone`, `sd_body_ge_of_column`: a body inside the column is at least as far as the
  column (G1/G2 for those bodies follow from the column's bound);
* `sd_column_ge_vertical`, `sd_column_ge_radial`: the two caps drop.rs takes on the depth into the
  column (straight down to the profile, straight out to the side);
* `drop_clear`: a tip above every face point's y.z − b(ρ) (the drop-cutter's upper bound, `Cl.hi`)
  keeps the column off the face;
* `ring_rho_ge`, `ring_bound`, `line_rho_ge`, `line_bound`: the reduction of a torus about a
  vertical axis (its horizontal rings), a sphere (likewise) and a cylinder with a horizontal axis
  (its lines along the axis) to the 2D problem in the vertical plane through the tool's axis,
  which drop.rs's closed forms (`disk_drop`, `bowl_drop`) solve: every point of a ring or line is
  bounded by the plane's nearest point.

Hypotheses left open (RESUME.md): that `disk_drop` / `bowl_drop`'s candidates include the 1D
maximum (each piece's stationary points and ends; L6-tested in tests/drop_vs_sampling.rs), the
interval arithmetic (L5), and that each cutter tool.rs builds lies in its column (Column::of's
check, L6-tested in each_cutter_lies_in_its_column).
-/
import ForgeCam.Hypograph

open Metric Set

namespace ForgeCam

/-- The column of a profile `b` up to radius `R`, the tip at the origin. -/
def column (R : ℝ) (b : ℝ → ℝ) : Set E3 := {p | ρ p ≤ R ∧ b (ρ p) ≤ p 2}

/-- The signed distance is antitone in the set (for a nonempty set inside one that is not all of
space). -/
theorem sd_antitone {A B : Set E3} (hAB : A ⊆ B) (hA : A.Nonempty) (hB : Bᶜ.Nonempty) (x : E3) :
    sd B x ≤ sd A x := by
  by_cases hxA : x ∈ A
  · have hxB : x ∈ B := hAB hxA
    rw [sd_of_mem hxA, sd_of_mem hxB, neg_le_neg_iff]
    exact infDist_le_infDist_of_subset (compl_subset_compl.mpr hAB) hB
  · rw [sd_of_not_mem hxA]
    by_cases hxB : x ∈ B
    · exact (sd_nonpos_of_mem hxB).trans infDist_nonneg
    · rw [sd_of_not_mem hxB]
      exact infDist_le_infDist_of_subset hAB hA

/-- The column is not all of space (a point past R is outside it). -/
theorem column_compl_nonempty (R : ℝ) (b : ℝ → ℝ) : (column R b)ᶜ.Nonempty := by
  refine ⟨mk (|R| + 1) 0 0, fun h => ?_⟩
  have h1 := h.1
  simp only [ρ_mk] at h1
  have : √((|R| + 1) ^ 2 + 0 ^ 2) = |R| + 1 := by
    rw [zero_pow two_ne_zero, add_zero, Real.sqrt_sq (by positivity)]
  rw [this] at h1
  linarith [le_abs_self R]

/-- A nonempty body inside the column is at least as far from every point as the column: so a
proven `L ≤ sd column x` is a bound for the body (drop.rs's `Gap::Clear { lower }`, and
ops::certify_with taking it for the cutter and a narrow shank). -/
theorem sd_body_ge_of_column {T : Set E3} {R : ℝ} {b : ℝ → ℝ} (hT : T ⊆ column R b)
    (hTne : T.Nonempty) {x : E3} {L : ℝ} (h : L ≤ sd (column R b) x) : L ≤ sd T x :=
  h.trans (sd_antitone hT hTne (column_compl_nonempty R b) x)

/-- The vertical cap: a point of the column is no deeper than its height above the profile
(straight down leaves it). -/
theorem sd_column_ge_vertical {R : ℝ} {b : ℝ → ℝ} {p : E3} (hp : p ∈ column R b) :
    -(p 2 - b (ρ p)) ≤ sd (column R b) p := by
  rw [sd_of_mem hp, neg_le_neg_iff]
  refine le_of_forall_pos_lt_add fun ε hε => ?_
  -- the point ε/2 below the profile, straight under p, is outside
  have hq : atZ p (b (ρ p) - ε / 2) ∈ (column R b)ᶜ := by
    intro h
    have h2 := h.2
    rw [ρ_atZ] at h2
    simp [atZ] at h2
    linarith
  have hd : dist p (atZ p (b (ρ p) - ε / 2)) = p 2 - b (ρ p) + ε / 2 := by
    rw [dist_atZ, abs_of_nonneg (by linarith [hp.2])]; ring
  have := infDist_le_dist_of_mem hq (x := p)
  linarith

/-- The radial cap: a point of the column is no deeper than its distance to the column's side
(straight out leaves it). -/
theorem sd_column_ge_radial {R : ℝ} {b : ℝ → ℝ} {p : E3} (hp : p ∈ column R b) :
    -(R - ρ p) ≤ sd (column R b) p := by
  rw [sd_of_mem hp, neg_le_neg_iff]
  refine le_of_forall_pos_lt_add fun ε hε => ?_
  have hR : 0 ≤ R := (ρ_nonneg p).trans hp.1
  by_cases h0 : 0 < ρ p
  · -- out along p's own ray to radius R + ε/2
    have hq : onRay p (R + ε / 2) (p 2) ∈ (column R b)ᶜ := by
      intro h
      have h1 := h.1
      rw [ρ_onRay h0 (by linarith) (p 2)] at h1
      linarith
    have hd : dist p (onRay p (R + ε / 2) (p 2)) = R + ε / 2 - ρ p := by
      rw [dist_onRay h0, sub_self, zero_pow two_ne_zero, add_zero, Real.sqrt_sq_eq_abs,
        abs_of_nonpos (by linarith [hp.1])]
      ring
    have := infDist_le_dist_of_mem hq (x := p)
    linarith
  · -- on the axis: out along x
    have hρ : ρ p = 0 := le_antisymm (not_lt.mp h0) (ρ_nonneg p)
    have hxy : p 0 = 0 ∧ p 1 = 0 := by
      have := ρ_sq p
      rw [hρ] at this
      constructor <;> nlinarith [sq_nonneg (p 0), sq_nonneg (p 1)]
    have hq : mk (R + ε / 2) 0 (p 2) ∈ (column R b)ᶜ := by
      intro h
      have h1 := h.1
      simp only [ρ_mk] at h1
      rw [zero_pow two_ne_zero, add_zero, Real.sqrt_sq (by linarith)] at h1
      linarith
    have hd : dist p (mk (R + ε / 2) 0 (p 2)) = R + ε / 2 := by
      rw [dist3]
      simp only [mk_0, mk_1, mk_2, hxy.1, hxy.2, sub_self, zero_pow two_ne_zero, add_zero,
        zero_sub, neg_sq]
      exact Real.sqrt_sq (by linarith)
    have := infDist_le_dist_of_mem hq (x := p)
    rw [hρ]
    linarith

/-- Drop-cutter soundness: if U bounds y.z − b(ρ) over every point y of a face within the
column's footprint (drop.rs's `Cl.hi`, a face's closed form, or a cell's bound), a tip at height
above U keeps every point of the face out of the column. -/
theorem drop_clear {R : ℝ} {b : ℝ → ℝ} {F : Set E3} {U : ℝ}
    (hU : ∀ y ∈ F, ρ y ≤ R → y 2 - b (ρ y) ≤ U) {t : ℝ} (ht : U < t) :
    ∀ y ∈ F, y - mk 0 0 t ∉ column R b := by
  intro y hy h
  have hρ : ρ (y - mk 0 0 t) = ρ y := by simp [ρ]
  have hz : (y - mk 0 0 t) 2 = y 2 - t := by simp
  rw [column, mem_setOf_eq, hρ, hz] at h
  have := hU y hy h.1
  linarith [h.2]

/-- A ring: the points at height h on the horizontal circle of radius r0 about the vertical axis
through (ox, oy). The horizontal distance from a tool axis at (x, y) to any of them is at least
| D − r0 |, D the axes' distance (the reverse triangle inequality). -/
theorem ring_rho_ge {ox oy x y r0 : ℝ} (hr0 : 0 ≤ r0) {q : E3}
    (hq : √((q 0 - ox) ^ 2 + (q 1 - oy) ^ 2) = r0) :
    |√((x - ox) ^ 2 + (y - oy) ^ 2) - r0| ≤ ρ (q - mk x y 0) := by
  have e : ρ (q - mk x y 0) = √((q 0 - x) ^ 2 + (q 1 - y) ^ 2) := by simp [ρ]
  rw [e, ← hq]
  -- | |a| − |c| | ≤ |a − c| in the plane, a = X − o, c = q − o
  set a0 := x - ox; set a1 := y - oy; set c0 := q 0 - ox; set c1 := q 1 - oy
  have h0 : q 0 - x = c0 - a0 := by simp [a0, c0]
  have h1 : q 1 - y = c1 - a1 := by simp [a1, c1]
  rw [h0, h1]
  have ha := Real.sqrt_nonneg (a0 ^ 2 + a1 ^ 2)
  have hc := Real.sqrt_nonneg (c0 ^ 2 + c1 ^ 2)
  have sa := Real.sq_sqrt (by positivity : (0 : ℝ) ≤ a0 ^ 2 + a1 ^ 2)
  have sc := Real.sq_sqrt (by positivity : (0 : ℝ) ≤ c0 ^ 2 + c1 ^ 2)
  have sd' := Real.sq_sqrt (by positivity : (0 : ℝ) ≤ (c0 - a0) ^ 2 + (c1 - a1) ^ 2)
  have hd := Real.sqrt_nonneg ((c0 - a0) ^ 2 + (c1 - a1) ^ 2)
  -- Cauchy–Schwarz: a·c ≤ |a| |c|
  have cs : a0 * c0 + a1 * c1 ≤ √(a0 ^ 2 + a1 ^ 2) * √(c0 ^ 2 + c1 ^ 2) := by
    have h2 : (a0 * c0 + a1 * c1) ^ 2 ≤ (√(a0 ^ 2 + a1 ^ 2) * √(c0 ^ 2 + c1 ^ 2)) ^ 2 := by
      rw [mul_pow, sa, sc]; nlinarith [sq_nonneg (a0 * c1 - a1 * c0)]
    have h3 := mul_nonneg ha hc
    by_contra hcon; push_neg at hcon; nlinarith
  rw [abs_le]
  constructor <;> nlinarith [sq_nonneg (√(a0 ^ 2 + a1 ^ 2) - √(c0 ^ 2 + c1 ^ 2))]

/-- So a ring's every point within reach is bounded by the plane's nearest one: with b
non-decreasing on [0, R], y.z − b(ρ) ≤ h − b(|D − r0|). drop.rs's torus (about a vertical axis)
and sphere closed forms take the max of the right side over the tube (or the meridian). -/
theorem ring_bound {R : ℝ} {b : ℝ → ℝ} (hb : MonotoneOn b (Icc 0 R)) {ox oy x y r0 h : ℝ}
    (hr0 : 0 ≤ r0) {q : E3} (hq : √((q 0 - ox) ^ 2 + (q 1 - oy) ^ 2) = r0) (hz : q 2 = h)
    (hin : ρ (q - mk x y 0) ≤ R) :
    q 2 - b (ρ (q - mk x y 0)) ≤ h - b |√((x - ox) ^ 2 + (y - oy) ^ 2) - r0| := by
  have hle := ring_rho_ge hr0 hq (x := x) (y := y)
  have h1 : b |√((x - ox) ^ 2 + (y - oy) ^ 2) - r0| ≤ b (ρ (q - mk x y 0)) :=
    hb ⟨abs_nonneg _, hle.trans hin⟩ ⟨ρ_nonneg _, hin⟩ hle
  rw [hz]; linarith

/-- A line along a horizontal axis direction (a0, a1) (unit), at lateral place e across it and
height h: the horizontal distance from a tool axis whose lateral place is d is at least |d − e|
(the component across the axis). -/
theorem line_rho_ge {a0 a1 : ℝ} (ha : a0 ^ 2 + a1 ^ 2 = 1) {x y : ℝ} {q : E3} :
    |(x * a1 - y * a0) - (q 0 * a1 - q 1 * a0)| ≤ ρ (q - mk x y 0) := by
  have e : ρ (q - mk x y 0) = √((q 0 - x) ^ 2 + (q 1 - y) ^ 2) := by simp [ρ]
  rw [e, ← Real.sqrt_sq_eq_abs]
  apply Real.sqrt_le_sqrt
  -- |v × a|² ≤ |v|² |a|² = |v|²
  nlinarith [sq_nonneg ((q 0 - x) * a0 + (q 1 - y) * a1)]

/-- So a cylinder with a horizontal axis is bounded line by line by the plane across it:
y.z − b(ρ) ≤ h − b(|d − e|) (drop.rs's cylinder closed forms). -/
theorem line_bound {R : ℝ} {b : ℝ → ℝ} (hb : MonotoneOn b (Icc 0 R)) {a0 a1 : ℝ}
    (ha : a0 ^ 2 + a1 ^ 2 = 1) {x y h : ℝ} {q : E3} (hz : q 2 = h) (hin : ρ (q - mk x y 0) ≤ R) :
    q 2 - b (ρ (q - mk x y 0)) ≤ h - b |(x * a1 - y * a0) - (q 0 * a1 - q 1 * a0)| := by
  have hle := line_rho_ge ha (x := x) (y := y) (q := q)
  have h1 := hb ⟨abs_nonneg _, hle.trans hin⟩ ⟨ρ_nonneg _, hin⟩ hle
  rw [hz]; linarith

end ForgeCam
