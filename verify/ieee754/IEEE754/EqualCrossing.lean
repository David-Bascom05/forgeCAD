/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Theorems
import IEEE754.RoundProps
import IEEE754.TwoProduct
import IEEE754.Sqrt

/-!
# ssi `equal_crossing`'s ellipse: the divisor `|x × a|` is not too small (stage 1: one operation)

kernel/forge-bool/src/ssi.rs `equal_crossing` makes `major = r / |x × a|` with
`x = normalize(a ± b)`, reached only where the f64 `|a × b| > ANGULAR = 1e-11`. This file bounds
the f64 `|x × a|` from below (so `div_pos_finite` gives a finite, positive `major`). Stage 1: the
error of one rounded operation, `|op(x, y) - exact| <= u |exact| + eta`, `u = 2^-prec`,
`eta = 2^(qmin-1)`, for a result at most `2^emax`.
-/

namespace IEEE754

variable (f : Format)

/-- One rounding: finite, and off by at most `u |r| + eta`, for `|r| <= 2^emax`. -/
theorem round_err_le {zs : Bool} {r : ℚ} (hr : |r| ≤ 2 ^ f.emax) :
    (round f zs r).isFinite ∧
      |(round f zs r).val - r| ≤ 2 ^ (-(f.prec : ℤ)) * |r| + 2 ^ (f.qmin - 1) := by
  have hfin : (round f zs r).isFinite := round_finite_of_le_emax hr
  refine ⟨hfin, ?_⟩
  have h := round_err (K := ℚ) f hfin
  have h1 : (0 : ℚ) ≤ 2 ^ (-(f.prec : ℤ)) * |r| := mul_nonneg (two_zpow_pos _).le (abs_nonneg _)
  have h2 : (0 : ℚ) ≤ 2 ^ (f.qmin - 1) := (two_zpow_pos _).le
  exact le_trans h (max_le (by linarith) (by linarith))

theorem add_err_le {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hr : |x.val + y.val| ≤ 2 ^ f.emax) :
    (FP.add x y).isFinite ∧ |(FP.add x y).val - (x.val + y.val)| ≤
      2 ^ (-(f.prec : ℤ)) * |x.val + y.val| + 2 ^ (f.qmin - 1) := by
  rw [add_eq_round hx hy]; exact round_err_le f hr

theorem sub_err_le {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hr : |x.val - y.val| ≤ 2 ^ f.emax) :
    (FP.sub x y).isFinite ∧ |(FP.sub x y).val - (x.val - y.val)| ≤
      2 ^ (-(f.prec : ℤ)) * |x.val - y.val| + 2 ^ (f.qmin - 1) := by
  rw [sub_eq_round hx hy]; exact round_err_le f hr

theorem mul_err_le {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hr : |x.val * y.val| ≤ 2 ^ f.emax) :
    (FP.mul x y).isFinite ∧ |(FP.mul x y).val - x.val * y.val| ≤
      2 ^ (-(f.prec : ℤ)) * |x.val * y.val| + 2 ^ (f.qmin - 1) := by
  rw [mul_eq_round hx hy]; exact round_err_le f hr

theorem div_err_le {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hy0 : y.val ≠ 0)
    (hr : |x.val / y.val| ≤ 2 ^ f.emax) :
    (FP.div x y).isFinite ∧ |(FP.div x y).val - x.val / y.val| ≤
      2 ^ (-(f.prec : ℤ)) * |x.val / y.val| + 2 ^ (f.qmin - 1) := by
  rw [div_eq_round hx hy hy0]; exact round_err_le f hr

/-- A rounding of a non-negative value within `u |r| + eta`, as two one-sided bounds. -/
theorem two_sided {v r u η : ℚ} (hr : 0 ≤ r) (h : |v - r| ≤ u * |r| + η) :
    r - u * r - η ≤ v ∧ v ≤ r + u * r + η := by
  rw [abs_of_nonneg hr] at h
  constructor <;> linarith [abs_le.mp h]

/-- **`sq3_err`**: the f64 `x*x + y*y + z*z` (Rust's `length_sq`, left to right) is within
`4u S + 6 eta` of `S = x² + y² + z²`, for `4 S + 8 <= 2^emax` and `u <= 1/8`. -/
theorem sq3_err {x y z : FP f} (hx : x.isFinite) (hy : y.isFinite) (hz : z.isFinite)
    (hu : (2 : ℚ) ^ (-(f.prec : ℤ)) ≤ 1 / 8) (hη : (2 : ℚ) ^ (f.qmin - 1) ≤ 1)
    (hS : 4 * (x.val * x.val + y.val * y.val + z.val * z.val) + 8 ≤ 2 ^ f.emax) :
    (FP.add (FP.add (FP.mul x x) (FP.mul y y)) (FP.mul z z)).isFinite ∧
      |(FP.add (FP.add (FP.mul x x) (FP.mul y y)) (FP.mul z z)).val -
          (x.val * x.val + y.val * y.val + z.val * z.val)| ≤
        4 * 2 ^ (-(f.prec : ℤ)) * (x.val * x.val + y.val * y.val + z.val * z.val) +
          6 * 2 ^ (f.qmin - 1) := by
  have sx := mul_self_nonneg x.val; have sy := mul_self_nonneg y.val
  have sz := mul_self_nonneg z.val
  have hS0 : 0 ≤ x.val * x.val + y.val * y.val + z.val * z.val := by linarith
  have bnd : ∀ t : ℚ, 0 ≤ t →
      t ≤ 2 * (x.val * x.val + y.val * y.val + z.val * z.val) + 6 → |t| ≤ 2 ^ f.emax :=
    fun t ht h => by rw [abs_of_nonneg ht]; linarith
  obtain ⟨f1, e1⟩ := mul_err_le f hx hx (bnd _ sx (by linarith))
  obtain ⟨f2, e2⟩ := mul_err_le f hy hy (bnd _ sy (by linarith))
  obtain ⟨f3, e3⟩ := mul_err_le f hz hz (bnd _ sz (by linarith))
  have p1n : 0 ≤ (FP.mul x x).val := by rw [mul_eq_round hx hx]; exact round_val_nonneg f sx
  have p2n : 0 ≤ (FP.mul y y).val := by rw [mul_eq_round hy hy]; exact round_val_nonneg f sy
  have p3n : 0 ≤ (FP.mul z z).val := by rw [mul_eq_round hz hz]; exact round_val_nonneg f sz
  have u0 : (0 : ℚ) ≤ 2 ^ (-(f.prec : ℤ)) := (two_zpow_pos _).le
  have η0 : (0 : ℚ) ≤ 2 ^ (f.qmin - 1) := (two_zpow_pos _).le
  generalize hS_def : x.val * x.val + y.val * y.val + z.val * z.val = S at *
  generalize hu_eq : (2 : ℚ) ^ (-(f.prec : ℤ)) = u at *
  generalize hη_eq : (2 : ℚ) ^ (f.qmin - 1) = η at *
  obtain ⟨l1, r1⟩ := two_sided sx e1
  obtain ⟨l2, r2⟩ := two_sided sy e2
  obtain ⟨l3, r3⟩ := two_sided sz e3
  -- A = p1 + p2 + p3 is within u S + 3 eta of S
  have uS_split : u * S = u * (x.val * x.val) + u * (y.val * y.val) + u * (z.val * z.val) := by
    rw [← hS_def]; ring
  have hAhi : (FP.mul x x).val + (FP.mul y y).val + (FP.mul z z).val ≤ S + u * S + 3 * η := by linarith
  have uS : u * S ≤ S / 8 := by
    have := mul_le_mul_of_nonneg_right hu (by rw [← hS_def]; linarith : (0:ℚ) ≤ S); linarith
  have uη : u * η ≤ η / 8 := by
    have := mul_le_mul_of_nonneg_right hu η0; linarith
  have hA : (FP.mul x x).val + (FP.mul y y).val + (FP.mul z z).val ≤ 9 / 8 * S + 3 * η := by linarith
  -- the first sum
  have hp12 : (FP.mul x x).val + (FP.mul y y).val ≤ 2 * S + 6 := by linarith
  obtain ⟨fs, es⟩ := add_err_le f f1 f2 (bnd _ (by linarith) hp12)
  rw [hu_eq, hη_eq] at es
  have sn : 0 ≤ (FP.add (FP.mul x x) (FP.mul y y)).val := by
    rw [add_eq_round f1 f2]; exact round_val_nonneg f (by linarith)
  obtain ⟨ls, rs⟩ := two_sided (by linarith) es
  have u12 : u * ((FP.mul x x).val + (FP.mul y y).val) ≤ u * (9 / 8 * S + 3 * η) :=
    mul_le_mul_of_nonneg_left (by linarith) u0
  have hsp_lin : (FP.add (FP.mul x x) (FP.mul y y)).val + (FP.mul z z).val ≤ 81 / 64 * S + 35 / 8 * η := by
    have := mul_le_mul_of_nonneg_left uS (by norm_num : (0:ℚ) ≤ 9 / 8)
    linarith
  -- the second
  have hsp : (FP.add (FP.mul x x) (FP.mul y y)).val + (FP.mul z z).val ≤ 2 * S + 6 := by linarith
  obtain ⟨fq, eq⟩ := add_err_le f fs f3 (bnd _ (by linarith) hsp)
  rw [hu_eq, hη_eq] at eq
  obtain ⟨lq, rq⟩ := two_sided (by linarith) eq
  have u23 : u * ((FP.add (FP.mul x x) (FP.mul y y)).val + (FP.mul z z).val) ≤ u * (81 / 64 * S + 35 / 8 * η) :=
    mul_le_mul_of_nonneg_left hsp_lin u0
  have hgoal : |(FP.add (FP.add (FP.mul x x) (FP.mul y y)) (FP.mul z z)).val - S| ≤
      4 * u * S + 6 * η := by
    have k1 : u * (9 / 8 * S + 3 * η) = 9 / 8 * (u * S) + 3 * (u * η) := by ring
    have k2 : u * (81 / 64 * S + 35 / 8 * η) = 81 / 64 * (u * S) + 35 / 8 * (u * η) := by ring
    have k3 : 4 * u * S = 4 * (u * S) := by ring
    have uS0 : 0 ≤ u * S := mul_nonneg u0 (by rw [← hS_def]; linarith)
    have uη0 : 0 ≤ u * η := mul_nonneg u0 η0
    rw [k3]
    apply abs_le.mpr
    constructor <;> linarith
  exact ⟨fq, hgoal⟩

end IEEE754
