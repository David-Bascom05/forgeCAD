/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.Ulp

/-!
# Rounding bounds for f64 screens

Every finite rounding is within `ε·|r| + η` of the exact `r` (`ε = 2^-53`, `η = 2^-1075`, the
subnormal floor). `lin3_err`: the left-associated `a0·b0 + a1·b1 + a2·b2 − c` is within
`14ε·M + 9η` of its exact value (crude; the screens' margins are 128ε and up), `M = Σ|a_i·b_i| + |c|`.
-/

namespace IEEE754.Screen

open FP

/-- `2^-53`, as a literal so `linarith` sees a numeral. -/
notation "ε" => (1 / 9007199254740992 : ℚ)
/-- `2^-1075`. -/
noncomputable abbrev η : ℚ := 2 ^ (-1075 : ℤ)

lemma η_pos : 0 < η := two_zpow_pos _

lemma round_rel {zs : Bool} {r : ℚ} (hf : (round binary64 zs r).isFinite) :
    |(round binary64 zs r).val - r| ≤ ε * |r| + η := by
  have h := round_err (K := ℚ) binary64 hf
  rw [Rat.cast_id] at h
  simp only [binary64_prec, binary64_qmin] at h
  rw [show (2 : ℚ) ^ (-((53 : ℕ) : ℤ)) = ε by norm_num, show (-1074 - 1 : ℤ) = -1075 by norm_num] at h
  refine le_trans h (max_le ?_ ?_)
  · linarith [η_pos]
  · have : 0 ≤ ε * |r| := by positivity
    linarith

lemma add_rel {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hf : (add a b).isFinite) :
    |(add a b).val - (a.val + b.val)| ≤ ε * |a.val + b.val| + η := by
  rw [add_eq_round ha hb] at hf ⊢; exact round_rel hf

lemma sub_rel {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hf : (sub a b).isFinite) :
    |(sub a b).val - (a.val - b.val)| ≤ ε * |a.val - b.val| + η := by
  rw [sub_eq_round ha hb] at hf ⊢; exact round_rel hf

lemma mul_rel {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hf : (mul a b).isFinite) :
    |(mul a b).val - a.val * b.val| ≤ ε * |a.val * b.val| + η := by
  rw [mul_eq_round ha hb] at hf ⊢; exact round_rel hf

lemma div_rel {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hb0 : b.val ≠ 0) (hf : (div a b).isFinite) :
    |(div a b).val - a.val / b.val| ≤ ε * |a.val / b.val| + η := by
  rw [div_eq_round ha hb hb0] at hf ⊢; exact round_rel hf

/-- One step of error propagation: a rounded `x ∘ y` whose inputs are within `ex`, `ey` of `X`, `Y`. -/
lemma step_add {x y : F64} {X Y ex ey : ℚ} (hx : x.isFinite) (hy : y.isFinite) (hf : (add x y).isFinite)
    (h1 : |x.val - X| ≤ ex) (h2 : |y.val - Y| ≤ ey) :
    |(add x y).val - (X + Y)| ≤ ε * (|X| + |Y| + ex + ey) + η + (ex + ey) := by
  have e := add_rel hx hy hf
  have a1 : |x.val + y.val| ≤ |X| + |Y| + ex + ey := by
    have := abs_add_le x.val y.val
    have := abs_sub_abs_le_abs_sub x.val X; have := abs_sub_abs_le_abs_sub y.val Y
    linarith
  have := abs_sub_le (add x y).val (x.val + y.val) (X + Y)
  have h3 : |x.val + y.val - (X + Y)| ≤ ex + ey := by
    rw [show x.val + y.val - (X + Y) = (x.val - X) + (y.val - Y) by ring]
    exact le_trans (abs_add_le _ _) (add_le_add h1 h2)
  have : ε * |x.val + y.val| ≤ ε * (|X| + |Y| + ex + ey) := mul_le_mul_of_nonneg_left a1 (by norm_num)
  linarith

lemma step_sub {x y : F64} {X Y ex ey : ℚ} (hx : x.isFinite) (hy : y.isFinite) (hf : (sub x y).isFinite)
    (h1 : |x.val - X| ≤ ex) (h2 : |y.val - Y| ≤ ey) :
    |(sub x y).val - (X - Y)| ≤ ε * (|X| + |Y| + ex + ey) + η + (ex + ey) := by
  have e := sub_rel hx hy hf
  have a1 : |x.val - y.val| ≤ |X| + |Y| + ex + ey := by
    have := abs_sub x.val y.val
    have := abs_sub_abs_le_abs_sub x.val X; have := abs_sub_abs_le_abs_sub y.val Y
    linarith
  have := abs_sub_le (sub x y).val (x.val - y.val) (X - Y)
  have h3 : |x.val - y.val - (X - Y)| ≤ ex + ey := by
    rw [show x.val - y.val - (X - Y) = (x.val - X) - (y.val - Y) by ring]
    exact le_trans (abs_sub _ _) (add_le_add h1 h2)
  have : ε * |x.val - y.val| ≤ ε * (|X| + |Y| + ex + ey) := mul_le_mul_of_nonneg_left a1 (by norm_num)
  linarith

/-- **A three-term dot product minus a constant**, left-associated as in Rust:
`((a0*b0 + a1*b1) + a2*b2) - c`. -/
theorem lin3_err {a0 a1 a2 b0 b1 b2 c : F64} (h0 : a0.isFinite) (h1 : a1.isFinite) (h2 : a2.isFinite)
    (g0 : b0.isFinite) (g1 : b1.isFinite) (g2 : b2.isFinite) (hc : c.isFinite)
    (hf : (sub (add (add (mul a0 b0) (mul a1 b1)) (mul a2 b2)) c).isFinite) :
    |(sub (add (add (mul a0 b0) (mul a1 b1)) (mul a2 b2)) c).val -
        (a0.val * b0.val + a1.val * b1.val + a2.val * b2.val - c.val)| ≤
      14 * ε * (|a0.val * b0.val| + |a1.val * b1.val| + |a2.val * b2.val| + |c.val|) + 9 * η := by
  have f3 := (isFinite_of_sub hf).1
  have f2 := (isFinite_of_add f3).1
  have fm2 := (isFinite_of_add f3).2
  have fm0 := (isFinite_of_add f2).1
  have fm1 := (isFinite_of_add f2).2
  generalize hM : |a0.val * b0.val| + |a1.val * b1.val| + |a2.val * b2.val| + |c.val| = M
  have hP0 := abs_nonneg (a0.val * b0.val); have hP1 := abs_nonneg (a1.val * b1.val)
  have hP2 := abs_nonneg (a2.val * b2.val); have hC := abs_nonneg c.val
  have hη := η_pos
  have m0 : |a0.val * b0.val| ≤ M := by linarith
  have m1 : |a1.val * b1.val| ≤ M := by linarith
  have m2 : |a2.val * b2.val| ≤ M := by linarith
  have mc : |c.val| ≤ M := by linarith
  have e0 : |(mul a0 b0).val - a0.val * b0.val| ≤ ε * M + η := by
    have := mul_rel h0 g0 fm0; have : ε * |a0.val * b0.val| ≤ ε * M := by linarith
    linarith
  have e1 : |(mul a1 b1).val - a1.val * b1.val| ≤ ε * M + η := by
    have := mul_rel h1 g1 fm1; have : ε * |a1.val * b1.val| ≤ ε * M := by linarith
    linarith
  have e2 : |(mul a2 b2).val - a2.val * b2.val| ≤ ε * M + η := by
    have := mul_rel h2 g2 fm2; have : ε * |a2.val * b2.val| ≤ ε * M := by linarith
    linarith
  have s1 := step_add fm0 fm1 f2 e0 e1
  have s1' : |(add (mul a0 b0) (mul a1 b1)).val - (a0.val * b0.val + a1.val * b1.val)| ≤
      41 / 10 * ε * M + 4 * η := by
    have : ε * (|a0.val * b0.val| + |a1.val * b1.val| + (ε * M + η) + (ε * M + η)) ≤
        ε * (2 * M + (ε * M + η) + (ε * M + η)) := by
      apply mul_le_mul_of_nonneg_left _ (by norm_num); linarith
    linarith
  have s2 := step_add f2 fm2 f3 s1' e2
  have hab : |a0.val * b0.val + a1.val * b1.val| ≤ 2 * M := le_trans (abs_add_le _ _) (by linarith)
  have s2' : |(add (add (mul a0 b0) (mul a1 b1)) (mul a2 b2)).val -
      (a0.val * b0.val + a1.val * b1.val + a2.val * b2.val)| ≤ 9 * ε * M + 7 * η := by
    have : ε * (|a0.val * b0.val + a1.val * b1.val| + |a2.val * b2.val| + (41 / 10 * ε * M + 4 * η) +
        (ε * M + η)) ≤ ε * (3 * M + (41 / 10 * ε * M + 4 * η) + (ε * M + η)) := by
      apply mul_le_mul_of_nonneg_left _ (by norm_num); linarith
    linarith
  have s3 := step_sub f3 hc hf s2' (show |c.val - c.val| ≤ 0 by simp)
  have habc : |a0.val * b0.val + a1.val * b1.val + a2.val * b2.val| ≤ 3 * M :=
    le_trans (abs_add_le _ _) (by linarith)
  have : ε * (|a0.val * b0.val + a1.val * b1.val + a2.val * b2.val| + |c.val| + (9 * ε * M + 7 * η) + 0) ≤
      ε * (4 * M + (9 * ε * M + 7 * η)) := by
    apply mul_le_mul_of_nonneg_left _ (by norm_num); linarith
  linarith

end IEEE754.Screen
