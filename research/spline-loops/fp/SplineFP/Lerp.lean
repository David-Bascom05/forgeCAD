/-
L5 for the spline's evaluation: the kernel's `lerp` (kernel/forge-geom/src/spline.rs,
`a + (b - a) * s`, one subtraction, one product and one sum in binary64, as Rust computes them: no
fused operations) against verify/ieee754, and de Casteljau's levels built from it.

`u = 2^-53` (2^-prec) and `η = 2^(qmin - 1)` (half the least subnormal): every operation of
finite operands with a finite result is within `u |exact| + η` of it (`op_err`, from the library's
relative bound in the normal range and its absolute bound below it). So:

* `lerp_err`: the computed lerp is within `(u |a + m| + η) + (u |d s| + η) + |s| (u |b - a| + η)` of
  `a + (b - a) s`, m and d the computed product and difference; with `|a|, |b| ≤ M`, `0 ≤ s ≤ 1`:
  within `8 u M + 4 η` (`lerp_err_M`).
* `dc_step`: a de Casteljau level from points each within E of the exact ones (all within M) is
  within `E + 8 u (M + E) + 4 η` of the exact level (the exact lerp being a convex combination);
  `p` levels give the evaluation's bound.
-/
import IEEE754

namespace SplineFP

open IEEE754 FP

variable {f : Format}

/-- The unit roundoff 2^-prec. -/
def u (f : Format) : ℚ := 2 ^ (-(f.prec : ℤ))
/-- Half the least subnormal. -/
def η (f : Format) : ℚ := 2 ^ (f.qmin - 1)

theorem u_nonneg : 0 ≤ u f := by unfold u; positivity
theorem η_nonneg : 0 ≤ η f := by unfold η; positivity

/-- **One rounding**, either range: a finite rounding of q is within `u |q| + η` of it. -/
theorem round_err {zs : Bool} {q : ℚ} (hfin : (round f zs q).isFinite) :
    |(round f zs q).val - q| ≤ u f * |q| + η f := by
  rcases le_or_gt (2 ^ f.emin) |q| with h | h
  · have := round_rel_err_q (f := f) hfin h
    have := η_nonneg (f := f)
    unfold u; linarith
  · have := (round_abs_err_q (f := f) (zs := zs) h).2
    have := mul_nonneg (u_nonneg (f := f)) (abs_nonneg q)
    unfold η; linarith

theorem add_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (add x y).isFinite) :
    |(add x y).val - (x.val + y.val)| ≤ u f * |x.val + y.val| + η f := by
  rw [add_eq_round hx hy] at hfin ⊢; exact round_err hfin

theorem sub_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (sub x y).isFinite) :
    |(sub x y).val - (x.val - y.val)| ≤ u f * |x.val - y.val| + η f := by
  rw [sub_eq_round hx hy] at hfin ⊢; exact round_err hfin

theorem mul_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (mul x y).isFinite) :
    |(mul x y).val - x.val * y.val| ≤ u f * |x.val * y.val| + η f := by
  rw [mul_eq_round hx hy] at hfin ⊢; exact round_err hfin

/-- The kernel's lerp, `a + (b - a) * s`, in binary64 operations. -/
def lerp (a b s : FP f) : FP f := add a (mul (sub b a) s)

/-- **The lerp's error**, from its three roundings. -/
theorem lerp_err (a b s : FP f) (ha : a.isFinite) (hb : b.isFinite) (hs : s.isFinite)
    (hd : (sub b a).isFinite) (hm : (mul (sub b a) s).isFinite) (hr : (lerp a b s).isFinite) :
    |(lerp a b s).val - (a.val + (b.val - a.val) * s.val)| ≤
      (u f * |a.val + (mul (sub b a) s).val| + η f) + (u f * |(sub b a).val * s.val| + η f)
        + |s.val| * (u f * |b.val - a.val| + η f) := by
  have e3 := add_err ha hm hr
  have e2 := mul_err hd hs hm
  have e1 := sub_err hb ha hd
  have e1' : |((sub b a).val - (b.val - a.val)) * s.val| ≤ |s.val| * (u f * |b.val - a.val| + η f) := by
    rw [abs_mul, _root_.mul_comm]; exact mul_le_mul_of_nonneg_left e1 (abs_nonneg s.val)
  unfold lerp at hr ⊢
  calc |(add a (mul (sub b a) s)).val - (a.val + (b.val - a.val) * s.val)|
      = |((add a (mul (sub b a) s)).val - (a.val + (mul (sub b a) s).val))
          + ((mul (sub b a) s).val - (sub b a).val * s.val)
          + ((sub b a).val - (b.val - a.val)) * s.val| := by ring_nf
    _ ≤ |(add a (mul (sub b a) s)).val - (a.val + (mul (sub b a) s).val)|
          + |(mul (sub b a) s).val - (sub b a).val * s.val|
          + |((sub b a).val - (b.val - a.val)) * s.val| := abs_add_three _ _ _
    _ ≤ _ := by linarith

/-- **The lerp's error in the points' size**: with `|a|, |b| ≤ M`, `0 ≤ s ≤ 1` and `u ≤ 1/16`
(binary64's is 2^-53), within `8 u M + 4 η`. -/
theorem lerp_err_M (a b s : FP f) (ha : a.isFinite) (hb : b.isFinite) (hs : s.isFinite)
    (hd : (sub b a).isFinite) (hm : (mul (sub b a) s).isFinite) (hr : (lerp a b s).isFinite)
    (M : ℚ) (haM : |a.val| ≤ M) (hbM : |b.val| ≤ M) (hs0 : 0 ≤ s.val) (hs1 : s.val ≤ 1)
    (hu : u f ≤ 1 / 16) (hηM : η f ≤ M) :
    |(lerp a b s).val - (a.val + (b.val - a.val) * s.val)| ≤ 8 * u f * M + 4 * η f := by
  have e := lerp_err a b s ha hb hs hd hm hr
  have e1 := sub_err hb ha hd
  have e2 := mul_err hd hs hm
  have hu0 := u_nonneg (f := f)
  have hη0 := η_nonneg (f := f)
  have hsa : |s.val| ≤ 1 := by rw [abs_of_nonneg hs0]; exact hs1
  have hba : |b.val - a.val| ≤ 2 * M := by
    calc |b.val - a.val| ≤ |b.val| + |a.val| := abs_sub _ _
      _ ≤ 2 * M := by linarith
  -- the computed difference and product are near their exact values, so of size about 2M
  have hdv : |(sub b a).val| ≤ 2 * M + u f * (2 * M) + η f := by
    have := abs_sub_abs_le_abs_sub (sub b a).val (b.val - a.val)
    have := mul_le_mul_of_nonneg_left hba hu0
    linarith
  have hds : |(sub b a).val * s.val| ≤ 2 * M + u f * (2 * M) + η f := by
    rw [abs_mul]
    calc |(sub b a).val| * |s.val| ≤ |(sub b a).val| * 1 :=
          mul_le_mul_of_nonneg_left hsa (abs_nonneg _)
      _ ≤ _ := by linarith
  have hmv : |(mul (sub b a) s).val| ≤ (2 * M + u f * (2 * M) + η f) * (1 + u f) + η f := by
    have := abs_sub_abs_le_abs_sub (mul (sub b a) s).val ((sub b a).val * s.val)
    have := mul_le_mul_of_nonneg_left hds hu0
    nlinarith
  have ham : |a.val + (mul (sub b a) s).val| ≤ M + ((2 * M + u f * (2 * M) + η f) * (1 + u f) + η f) := by
    calc |a.val + (mul (sub b a) s).val| ≤ |a.val| + |(mul (sub b a) s).val| := abs_add_le _ _
      _ ≤ _ := by linarith
  have t1 := mul_le_mul_of_nonneg_left ham hu0
  have t2 := mul_le_mul_of_nonneg_left hds hu0
  have t3 : |s.val| * (u f * |b.val - a.val| + η f) ≤ 1 * (u f * (2 * M) + η f) := by
    apply mul_le_mul hsa _ (by positivity) (by norm_num)
    have := mul_le_mul_of_nonneg_left hba hu0
    linarith
  have hM : 0 ≤ M := le_trans (abs_nonneg _) haM
  -- u ≤ 1/16 and η ≤ M make the second-order terms fit
  have q1 : u f * u f ≤ u f / 16 := by nlinarith
  have q2 : u f * η f ≤ η f / 16 := by nlinarith
  nlinarith [mul_nonneg hu0 hM, mul_nonneg hu0 hη0, mul_nonneg (mul_nonneg hu0 hu0) hM,
    mul_nonneg (mul_nonneg hu0 hu0) hη0]

/-- **A de Casteljau level**: from computed points each within E of the exact ones (the exact
ones within M), the computed level is within `E + 8 u (M + E) + 4 η` of the exact level. -/
theorem dc_step (a b s : FP f) (ha : a.isFinite) (hb : b.isFinite) (hs : s.isFinite)
    (hd : (sub b a).isFinite) (hm : (mul (sub b a) s).isFinite) (hr : (lerp a b s).isFinite)
    (x y M E : ℚ) (hxM : |x| ≤ M) (hyM : |y| ≤ M) (hax : |a.val - x| ≤ E) (hby : |b.val - y| ≤ E)
    (hs0 : 0 ≤ s.val) (hs1 : s.val ≤ 1) (hu : u f ≤ 1 / 16) (hηM : η f ≤ M + E) :
    |(lerp a b s).val - (x + (y - x) * s.val)| ≤ E + (8 * u f * (M + E) + 4 * η f) := by
  have haM : |a.val| ≤ M + E := by
    have := abs_sub_abs_le_abs_sub a.val x; linarith
  have hbM : |b.val| ≤ M + E := by
    have := abs_sub_abs_le_abs_sub b.val y; linarith
  have e := lerp_err_M a b s ha hb hs hd hm hr (M + E) haM hbM hs0 hs1 hu hηM
  -- the exact lerp is a convex combination: points within E give a lerp within E
  have conv : |(a.val + (b.val - a.val) * s.val) - (x + (y - x) * s.val)| ≤ E := by
    have : (a.val + (b.val - a.val) * s.val) - (x + (y - x) * s.val)
        = (1 - s.val) * (a.val - x) + s.val * (b.val - y) := by ring
    rw [this]
    calc |(1 - s.val) * (a.val - x) + s.val * (b.val - y)|
        ≤ |(1 - s.val) * (a.val - x)| + |s.val * (b.val - y)| := abs_add_le _ _
      _ = (1 - s.val) * |a.val - x| + s.val * |b.val - y| := by
          rw [abs_mul, abs_mul, abs_of_nonneg (by linarith), abs_of_nonneg hs0]
      _ ≤ (1 - s.val) * E + s.val * E := by
          apply add_le_add
          · exact mul_le_mul_of_nonneg_left hax (by linarith)
          · exact mul_le_mul_of_nonneg_left hby hs0
      _ = E := by ring
  calc |(lerp a b s).val - (x + (y - x) * s.val)|
      ≤ |(lerp a b s).val - (a.val + (b.val - a.val) * s.val)|
        + |(a.val + (b.val - a.val) * s.val) - (x + (y - x) * s.val)| := abs_sub_le _ _ _
    _ ≤ (8 * u f * (M + E) + 4 * η f) + E := add_le_add e conv
    _ = _ := by ring

end SplineFP
