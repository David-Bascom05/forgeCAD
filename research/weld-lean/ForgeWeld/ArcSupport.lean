import ForgeExact.Alg.Sum

/-!
# An arc's support, exactly (L2 for arcs, step 1)

A section's arcs have rational centres and radii (the library's corners are square), but the
point of an arc farthest in a direction `w` is `c + r w / |w|`: its reach `w · c + r |w|` is in
ℚ(√(w·w)). On the exact-curved session's certified real algebraic numbers
(verify/forge-exact/ForgeExact/Alg: `Num`, `Denotes`, `add_spec`, `mul_spec`, `cmp_spec`), for
every model `K` of the reals:

* `sqrt_denotes`: `sqrtNum d` (the root of `x² - d` isolated in `(0, d + 1)`) denotes the positive
  square root of `d > 0`;
* `circle_le`: on the circle about `c` of radius `r ≥ 0`, `w · p ≤ w · c + r s` where `s = |w|`
  (`s ≥ 0`, `s² = w · w`), with equality at `c + (r / s) w` (`circle_attained`): Cauchy–Schwarz, in
  an ordered field;
* `arcReach_spec`: `arcReach` (`w · c` plus `r` times `sqrtNum (w · w)`, by `Num.add`, `Num.mul`)
  denotes that value whenever it is computed.

So forge_weld's arc tangent candidate is an exact algebraic number, comparable with the others by
`Num.cmp`.
-/

namespace ForgeWeld.Arc

open ForgeExact.Alg Lean.Grind

section

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] (M : Model K)

/-- `x² - d`, low coefficient first. -/
def sqPoly (d : Rat) : Poly := [-d, 0, 1]

/-- The positive square root of `d > 0`, isolated in `(0, d + 1)`. -/
def sqrtNum (d : Rat) : Num := .iso ⟨sqPoly d, 0, d + 1⟩

theorem ev_sqPoly (d : Rat) (x : K) : M.ev (sqPoly d) x = x * x - M.q d := by
  simp only [sqPoly, Model.ev, evalWith_cons, evalWith_nil, M.q_neg, M.q_zero, M.q_one]
  grind

theorem sqrt_denotes (d : Rat) (hd : 0 < d) :
    ∃ s, Denotes M (sqrtNum d) s ∧ 0 < s ∧ s * s = M.q d := by
  have hq : 0 < M.q d := M.q_pos.mpr hd
  have hb : M.q 0 < M.q (d + 1) := M.q_lt.mpr (by grind)
  have hs : M.ev (sqPoly d) (M.q 0) * M.ev (sqPoly d) (M.q (d + 1)) < 0 := by
    rw [M.ev_q, M.ev_q, ← M.q_mul]
    apply M.q_neg_iff.mpr
    simp only [sqPoly, Poly.ev, List.foldr_cons, List.foldr_nil]
    have : 0 < d * d + d + 1 := by
      have := Rat.mul_nonneg (Rat.le_of_lt hd) (Rat.le_of_lt hd); grind
    have e : (-d + 0 * (0 + 0 * (1 + 0 * 0))) * (-d + (d + 1) * (0 + (d + 1) * (1 + (d + 1) * 0))) =
        -(d * (d * d + d + 1)) := by grind
    rw [e]
    have := Rat.mul_pos hd this
    grind
  obtain ⟨s, h1, h2, h3⟩ := M.root_between (sqPoly d) hb hs
  rw [M.q_zero] at h1
  have sq : s * s = M.q d := by rw [ev_sqPoly] at h3; grind
  refine ⟨s, ⟨by rw [M.q_zero]; exact h1, h2, h3, fun y hy1 hy2 hy3 => ?_, ?_⟩, h1, sq⟩
  · -- y² = d = s², both positive: y = s
    rw [M.q_zero] at hy1
    rw [ev_sqPoly] at hy3
    have hy0 : 0 < y := by grind
    have e : y * y = s * s := by grind
    rcases LinearOrder.trichotomy y s with h | h | h
    · have a := OrderedRing.mul_lt_mul_of_pos_left h hy0
      have b := OrderedRing.mul_lt_mul_of_pos_right h h1
      grind
    · exact h
    · have a := OrderedRing.mul_lt_mul_of_pos_left h h1
      have b := OrderedRing.mul_lt_mul_of_pos_right h hy0
      grind
  · simp only [sqPoly, Poly.ev, List.foldr_cons, List.foldr_nil]
    have : 0 < d * d + d + 1 := by
      have := Rat.mul_nonneg (Rat.le_of_lt hd) (Rat.le_of_lt hd); grind
    have e : (-d + 0 * (0 + 0 * (1 + 0 * 0))) * (-d + (d + 1) * (0 + (d + 1) * (1 + (d + 1) * 0))) =
        -(d * (d * d + d + 1)) := by grind
    rw [e]
    have := Rat.mul_pos hd this
    grind

/-- **On a circle, a direction's reach is at most the centre's plus `r |w|`** (Cauchy–Schwarz in an
ordered field): for `|p - c|² = r²`, `s ≥ 0`, `s² = w · w`, `r ≥ 0`. -/
theorem circle_le (cx cy px py wx wy r s : K) (hr : 0 ≤ r) (hs : 0 ≤ s)
    (hss : s * s = wx * wx + wy * wy)
    (hp : (px - cx) * (px - cx) + (py - cy) * (py - cy) = r * r) :
    wx * px + wy * py ≤ wx * cx + wy * cy + r * s := by
  -- (w·v)² ≤ |w|²|v|² = (r s)², with r s ≥ 0, so w·v ≤ r s
  let vx := px - cx
  let vy := py - cy
  have cs : (wx * vx + wy * vy) * (wx * vx + wy * vy) ≤ (r * s) * (r * s) := by
    have key : (r * s) * (r * s) - (wx * vx + wy * vy) * (wx * vx + wy * vy) =
        (wx * vy - wy * vx) * (wx * vy - wy * vx) := by
      have : (r * s) * (r * s) = (vx * vx + vy * vy) * (wx * wx + wy * wy) := by
        rw [show (r * s) * (r * s) = (r * r) * (s * s) by grind, hss, ← hp]
      rw [this]; grind
    have : 0 ≤ (wx * vy - wy * vx) * (wx * vy - wy * vx) := by
      have := OrderedRing.sq_nonneg (a := wx * vy - wy * vx); grind
    grind
  have hrs : 0 ≤ r * s := OrderedRing.mul_nonneg hr hs
  have : wx * vx + wy * vy ≤ r * s := by
    apply Classical.byContradiction; intro hn
    have hlt : r * s < wx * vx + wy * vy := by grind
    have := OrderedRing.mul_lt_mul_of_pos_left hlt (show 0 < wx * vx + wy * vy by grind)
    have := OrderedRing.mul_le_mul_of_nonneg_right (show r * s ≤ wx * vx + wy * vy by grind) hrs
    grind
  grind

/-- And reached at `c + (r / s) w` (for `s > 0`). -/
theorem circle_attained (cx cy wx wy r s : K) (hs : 0 < s) (hss : s * s = wx * wx + wy * wy) :
    wx * (cx + r / s * wx) + wy * (cy + r / s * wy) = wx * cx + wy * cy + r * s ∧
      (r / s * wx) * (r / s * wx) + (r / s * wy) * (r / s * wy) = r * r := by
  have hs0 : s ≠ 0 := by grind
  have inv : s * s⁻¹ = 1 := Field.mul_inv_cancel hs0
  have e : r / s * (wx * wx + wy * wy) = r * s := by
    rw [← hss, Field.div_eq_mul_inv]
    calc r * s⁻¹ * (s * s) = r * s * (s * s⁻¹) := by grind
      _ = r * s := by rw [inv]; grind
  constructor
  · grind
  · have : (r / s) * (r / s) * (wx * wx + wy * wy) = r * r := by
      rw [← hss, Field.div_eq_mul_inv]
      calc r * s⁻¹ * (r * s⁻¹) * (s * s) = r * r * ((s * s⁻¹) * (s * s⁻¹)) := by grind
        _ = r * r := by rw [inv]; grind
    grind

end

/-- The reach `w · c + r |w|` of an arc's farthest point, as an algebraic number (when the
certified sum and product succeed within `fuel`). -/
def arcReach (fuel : Nat) (cx cy wx wy r : Rat) : Option Num :=
  match Num.mul fuel (.rat r) (sqrtNum (wx * wx + wy * wy)) with
  | some rs => Num.add fuel (.rat (wx * cx + wy * cy)) rs
  | none => none

theorem arcReach_spec {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
    [OrderedRing K] (M : Model K) (fuel : Nat) (cx cy wx wy r : Rat) (hw : 0 < wx * wx + wy * wy)
    {z : Num} (h : arcReach fuel cx cy wx wy r = some z) :
    ∃ s, 0 < s ∧ s * s = M.q (wx * wx + wy * wy) ∧
      Denotes M z (M.q (wx * cx + wy * cy) + M.q r * s) := by
  obtain ⟨s, hs, hpos, hsq⟩ := sqrt_denotes M _ hw
  unfold arcReach at h
  split at h
  · rename_i rs hrs
    have hm := mul_spec M fuel (x := M.q r) (y := s) (show Denotes M (.rat r) (M.q r) from rfl) hs hrs
    exact ⟨s, hpos, hsq, add_spec M fuel (x := M.q (wx * cx + wy * cy))
      (show Denotes M (.rat (wx * cx + wy * cy)) (M.q (wx * cx + wy * cy)) from rfl) hm h⟩
  · cases h

end ForgeWeld.Arc
