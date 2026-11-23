import ForgeSketch.Line

/-!
# The hull subdivision's rounding (winding.rs `bezier`), on verify/ieee754

`bezier` splits a piece at its parameter's middle by de Casteljau in f64: every new point is
`(a + b) / 2` of two points of the level before. These are the bounds its margin rests on.
-/

open IEEE754

namespace ForgeSketch

/-- **One midpoint's rounding**: `(a ⊕ b) ⊘ 2` against the exact midpoint `(x + y)/2` of the
points `a`, `b` stand for (each within `E` of its own), when both operations stay finite. The
error grows by one rounding of the sum and one of the halving (relative to their results), and
half the least subnormal each. -/
theorem mid_err (a b two : IEEE754.F64) (x y E : ℚ) (ha : a.isFinite) (hb : b.isFinite)
    (h2 : two.isFinite) (h2v : two.val = 2) (hs : (FP.add a b).isFinite)
    (hz : (FP.div (FP.add a b) two).isFinite) (hax : |a.val - x| ≤ E) (hby : |b.val - y| ≤ E) :
    |(FP.div (FP.add a b) two).val - (x + y) / 2| ≤
      E + 2 ^ (-53 : ℤ) * |(FP.div (FP.add a b) two).val| + 2 ^ (-54 : ℤ) * |(FP.add a b).val| +
        2 ^ (-1074 : ℤ) := by
  have e1 := IEEE754.round_err_abs (f := binary64) (zs := a.sign && b.sign) (q := a.val + b.val)
    (by rw [← IEEE754.add_eq_round ha hb]; exact hs)
  rw [← IEEE754.add_eq_round ha hb] at e1
  have h20 : two.val ≠ 0 := by rw [h2v]; norm_num
  have e2 := IEEE754.round_err_abs (f := binary64) (zs := (FP.add a b).sign ^^ two.sign)
    (q := (FP.add a b).val / two.val) (by rw [← IEEE754.div_eq_round hs h2 h20]; exact hz)
  rw [← IEEE754.div_eq_round hs h2 h20] at e2
  rw [h2v] at e2
  have hq : binary64.qmin - 1 = -1075 := by decide
  rw [hq] at e1 e2
  generalize (FP.div (FP.add a b) two).val = z at e2 ⊢
  generalize (FP.add a b).val = s at e1 e2 ⊢
  generalize a.val = A at e1 hax
  generalize b.val = B at e1 hby
  have p53 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) = 2 ^ (-53 : ℤ) := by simp [binary64]
  rw [p53] at e1 e2
  have p54 : (2 : ℚ) ^ (-54 : ℤ) = 2 ^ (-53 : ℤ) / 2 := by
    rw [show (-54 : ℤ) = -53 + -1 by norm_num, zpow_add₀ (by norm_num)]; norm_num
  have p1074 : (2 : ℚ) ^ (-1074 : ℤ) = 2 * 2 ^ (-1075 : ℤ) := by
    rw [show (-1074 : ℤ) = 1 + -1075 by norm_num, zpow_add₀ (by norm_num)]; norm_num
  rw [p54, p1074]
  have hn53 : (0 : ℚ) ≤ 2 ^ (-53 : ℤ) := by positivity
  have hn1075 : (0 : ℚ) ≤ 2 ^ (-1075 : ℤ) := by positivity
  generalize (2 : ℚ) ^ (-53 : ℤ) = c at e1 e2 hn53 ⊢
  generalize (2 : ℚ) ^ (-1075 : ℤ) = t at e1 e2 hn1075 ⊢
  generalize c * |z| = cz at e1 e2 ⊢
  have hcs : c / 2 * |s| = (c * |s|) / 2 := by ring
  rw [hcs]
  generalize c * |s| = cs at e1 e2 ⊢
  rw [abs_le] at e1 e2 hax hby ⊢
  constructor <;> linarith [e1.1, e1.2, e2.1, e2.2, hax.1, hax.2, hby.1, hby.2]

/-- **One projection's rounding**: `x̃ ⊘ w̃` against the exact `x / w`, where `x̃`, `w̃` are within
`E` of `x`, `w` and both weights are at least `wmin > 0`. The error is one rounding of the result
(and half the least subnormal), plus the carried error over the weight, plus the carried weight
error scaled by the point's own size. -/
theorem proj_err (a w : IEEE754.F64) (x v E wmin : ℚ) (ha : a.isFinite) (hw : w.isFinite)
    (hwm : wmin ≤ w.val) (hvm : wmin ≤ v) (h0 : 0 < wmin) (hz : (FP.div a w).isFinite)
    (hax : |a.val - x| ≤ E) (hwv : |w.val - v| ≤ E) :
    |(FP.div a w).val - x / v| ≤
      2 ^ (-53 : ℤ) * |(FP.div a w).val| + 2 ^ (-1075 : ℤ) + E / wmin + |x / v| * E / wmin := by
  have hw0 : w.val ≠ 0 := by linarith
  have e := IEEE754.round_err_abs (f := binary64) (zs := a.sign ^^ w.sign) (q := a.val / w.val)
    (by rw [← IEEE754.div_eq_round ha hw hw0]; exact hz)
  rw [← IEEE754.div_eq_round ha hw hw0] at e
  have hq : binary64.qmin - 1 = -1075 := by decide
  have p53 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) = 2 ^ (-53 : ℤ) := by simp [binary64]
  rw [hq, p53] at e
  -- the exact quotients: a/w - x/v = ((a - x) + (x/v)(v - w)) / w
  have hwpos : 0 < w.val := by linarith
  have hv0 : v ≠ 0 := by intro h; linarith
  have key : a.val / w.val - x / v = ((a.val - x) + (x / v) * (v - w.val)) / w.val := by
    field_simp
    ring
  have h2 : |a.val / w.val - x / v| ≤ E / wmin + |x / v| * E / wmin := by
    rw [key, abs_div, abs_of_pos hwpos]
    have hnum : |(a.val - x) + (x / v) * (v - w.val)| ≤ E + |x / v| * E := by
      calc |(a.val - x) + (x / v) * (v - w.val)| ≤ |a.val - x| + |x / v| * |v - w.val| := by
            rw [← abs_mul]; exact abs_add_le _ _
        _ ≤ E + |x / v| * E := by
            gcongr
            rw [abs_sub_comm]; exact hwv
    have hE : 0 ≤ E := le_trans (abs_nonneg _) hax
    calc |(a.val - x) + (x / v) * (v - w.val)| / w.val ≤ (E + |x / v| * E) / w.val := by
          gcongr
      _ ≤ (E + |x / v| * E) / wmin := by
          apply div_le_div_of_nonneg_left (by positivity) h0 hwm
      _ = E / wmin + |x / v| * E / wmin := by ring
  calc |(FP.div a w).val - x / v| ≤ |(FP.div a w).val - a.val / w.val| + |a.val / w.val - x / v| :=
        abs_sub_le _ _ _
    _ ≤ _ := by linarith

end ForgeSketch
