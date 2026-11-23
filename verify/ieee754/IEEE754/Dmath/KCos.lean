/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.KSin
import IEEE754.Dmath.CosPoly

/-!
# `dmath::k_cos`'s evaluation

`kcos_eval`: for `|x| ≤ 0.78542`, `|y| ≤ 10^-16`, `k_cos(x, y)` is finite and within 1.63e-16 of
`kcosL(x²) − x·y`, its value in exact arithmetic. `0.5·z` is exact up to underflow, `1 − w` is exact
(Sterbenz) and `(1 − w) − hz` is exactly the rounding error of `w`; the rest goes through the error
calculus.
-/

namespace IEEE754.Dmath

open FP

def ONE : F64 := .fin false 4503599627370496 (-52) (by decide)
theorem ONE_bits : ONE.toBits = 0x3ff0000000000000 := by decide
theorem ONE_val : ONE.val = (1 : ℚ) := by simp only [ONE, val, sgn]; norm_num

/-- `dmath::k_cos`, operation for operation. -/
def kcos (x y : F64) : F64 :=
  let z := mul x x
  let w := mul z z
  let r := add (mul z (add C1 (mul z (add C2 (mul z C3))))) (mul (mul w w) (add C4 (mul z (add C5 (mul z C6)))))
  let hz := mul HALF z
  let w := sub ONE hz
  add w (add (sub (sub ONE w) hz) (sub (mul z r) (mul x y)))

/-- `0.5 ⊗ z` is `z/2`, up to underflow. -/
lemma half_mul {z : F64} (hz : z.isFinite) : (mul HALF z).isFinite ∧ |(mul HALF z).val - z.val / 2| ≤ 2 ^ (-1073 : ℤ) := by
  have fH : HALF.isFinite := by decide
  have hm : |HALF.val * z.val| ≤ |z.val| := by rw [HALF_val, abs_mul]; norm_num; linarith [abs_nonneg z.val]
  have hf : (mul HALF z).isFinite := by rw [mul_eq_round fH hz]; exact round_finite_of_le hz hm
  refine ⟨hf, ?_⟩
  have ha : (0:ℚ) < 2 ^ (-1075 : ℤ) := two_zpow_pos _
  have e1022 : (2 : ℚ) ^ (-1021 : ℤ) = 2 ^ 54 * ((2 : ℚ) ^ (-1075 : ℤ)) := by
    rw [show (-1021 : ℤ) = ((54 : ℕ) : ℤ) + (-1075) by norm_num, two_zpow_add, zpow_natCast]
  have e1073 : (2 : ℚ) ^ (-1073 : ℤ) = 4 * ((2 : ℚ) ^ (-1075 : ℤ)) := by
    rw [show (-1073 : ℤ) = ((2 : ℕ) : ℤ) + (-1075) by norm_num, two_zpow_add, zpow_natCast]; norm_num
  rw [e1073]
  by_cases hbig : (2 : ℚ) ^ (-1021 : ℤ) ≤ |z.val|
  · obtain ⟨k, e, hk, he, hv, hcan⟩ := val_repr_canon hz
    have hgt : binary64.qmin < e := by
      rcases lt_or_eq_of_le he with h | h
      · exact h
      · exfalso
        rw [hv, ← h, abs_mul, abs_of_pos (two_zpow_pos (K := ℚ) _)] at hbig
        simp only [binary64_qmin, binary64_prec] at hbig hk
        have : (|(k : ℚ)|) < 2 ^ (53 : ℕ) := by exact_mod_cast hk
        have h2 : (2 : ℚ) ^ (-1021 : ℤ) = 2 ^ (53 : ℕ) * 2 ^ (-1074 : ℤ) := by
          rw [← zpow_natCast, ← two_zpow_add]; norm_num
        rw [h2] at hbig
        have := mul_lt_mul_of_pos_right this (two_zpow_pos (K := ℚ) (-1074))
        linarith
    have hr : Rep binary64 (HALF.val * z.val) := by
      have e2 : HALF.val * z.val = (k : ℚ) * 2 ^ (e - 1) := by
        rw [HALF_val, hv, zpow_sub_one₀ two_ne_zero]; ring
      rw [e2]; refine rep_grid hk.le (by omega) ?_
      rw [← e2]; exact lt_of_le_of_lt hm (FP.abs_val_lt hz)
    obtain ⟨-, h2⟩ := mul_of_rep fH hz hr
    rw [h2, HALF_val, show 1 / 2 * z.val - z.val / 2 = 0 by ring, abs_zero]; linarith
  · rw [not_le] at hbig
    have e := mul_err_abs fH hz hf
    simp only [binary64_prec, binary64_qmin] at e
    rw [show (2 : ℚ) ^ (-((53 : ℕ) : ℤ)) = 1 / 9007199254740992 by norm_num,
      show (-1074 - 1 : ℤ) = -1075 by norm_num, HALF_val] at e
    have := abs_sub_abs_le_abs_sub (mul HALF z).val (1 / 2 * z.val)
    have h3 : |1 / 2 * z.val| ≤ 2 ^ 53 * ((2 : ℚ) ^ (-1075 : ℤ)) := by
      rw [abs_mul, abs_of_pos (by norm_num : (0:ℚ) < 1/2)]; rw [e1022] at hbig
      generalize (2 : ℚ) ^ (-1075 : ℤ) = q at hbig ⊢; norm_num at hbig ⊢; linarith
    rw [show (mul HALF z).val - z.val / 2 = (mul HALF z).val - 1 / 2 * z.val by ring]
    nlinarith

/-- **`k_cos`'s evaluation.** -/
theorem kcos_eval {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hxb : |x.val| ≤ 78542 / 100000)
    (hyb : |y.val| ≤ 1 / 10 ^ 16) :
    (kcos x y).isFinite ∧ |(kcos x y).val - (evalP kcosL (x.val ^ 2) - x.val * y.val)| ≤ (163 / 1000000000000000000 : ℚ) := by
  have fc : C1.isFinite ∧ C2.isFinite ∧ C3.isFinite ∧ C4.isFinite ∧ C5.isFinite ∧ C6.isFinite ∧ ONE.isFinite := by decide
  obtain ⟨fC1, fC2, fC3, fC4, fC5, fC6, fONE⟩ := fc
  have mC1 : |C1.val| ≤ (417 / 10000 : ℚ) := by rw [C1_val]; norm_num
  have mC2 : |C2.val| ≤ (139 / 100000 : ℚ) := by rw [C2_val]; norm_num
  have mC3 : |C3.val| ≤ (249 / 10000000 : ℚ) := by rw [C3_val]; norm_num
  have mC4 : |C4.val| ≤ (69 / 250000000 : ℚ) := by rw [C4_val]; norm_num
  have mC5 : |C5.val| ≤ (209 / 100000000000 : ℚ) := by rw [C5_val]; norm_num
  have mC6 : |C6.val| ≤ (57 / 5000000000000 : ℚ) := by rw [C6_val]; norm_num
  set X := x.val
  set Y := y.val
  obtain ⟨f_z, e_z, m_z0⟩ := mulA hx hx (E := (137 / 2000000000000000000 : ℚ)) (self_err x) (self_err x) hxb hxb (by norm_num) (by norm_num [ep, tau])
  have m_z : |(X * X)| ≤ (617 / 1000 : ℚ) := le_trans m_z0 (by norm_num)
  obtain ⟨f_w, e_w, m_w0⟩ := mulA f_z f_z (E := (127 / 1000000000000000000 : ℚ)) e_z e_z m_z m_z (by norm_num) (by norm_num [ep, tau])
  have m_w : |((X * X) * (X * X))| ≤ (381 / 1000 : ℚ) := le_trans m_w0 (by norm_num)
  obtain ⟨f_zC3, e_zC3, m_zC30⟩ := mulA f_z fC3 (E := (171 / 50000000000000000000000 : ℚ)) e_z (self_err C3) m_z mC3 (by norm_num) (by norm_num [ep, tau])
  have m_zC3 : |((X * X) * C3.val)| ≤ (77 / 5000000 : ℚ) := le_trans m_zC30 (by norm_num)
  obtain ⟨f_c2, e_c2, m_c20⟩ := addA fC2 f_zC3 (E := (1 / 6250000000000000000 : ℚ)) (self_err C2) e_zC3 mC2 m_zC3 (by norm_num) (by norm_num [ep, tau])
  have m_c2 : |(C2.val + ((X * X) * C3.val))| ≤ (141 / 100000 : ℚ) := le_trans m_c20 (by norm_num)
  obtain ⟨f_zc2, e_zc2, m_zc20⟩ := mulA f_z f_c2 (E := (73 / 250000000000000000000 : ℚ)) e_z e_c2 m_z m_c2 (by norm_num) (by norm_num [ep, tau])
  have m_zc2 : |((X * X) * (C2.val + ((X * X) * C3.val)))| ≤ (87 / 100000 : ℚ) := le_trans m_zc20 (by norm_num)
  obtain ⟨f_c1, e_c1, m_c10⟩ := addA fC1 f_zc2 (E := (251 / 50000000000000000000 : ℚ)) (self_err C1) e_zc2 mC1 m_zc2 (by norm_num) (by norm_num [ep, tau])
  have m_c1 : |(C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))| ≤ (213 / 5000 : ℚ) := le_trans m_c10 (by norm_num)
  obtain ⟨f_t1, e_t1, m_t10⟩ := mulA f_z f_c1 (E := (447 / 50000000000000000000 : ℚ)) e_z e_c1 m_z m_c1 (by norm_num) (by norm_num [ep, tau])
  have m_t1 : |((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val)))))| ≤ (263 / 10000 : ℚ) := le_trans m_t10 (by norm_num)
  obtain ⟨f_ww, e_ww, m_ww0⟩ := mulA f_w f_w (E := (113 / 1000000000000000000 : ℚ)) e_w e_w m_w m_w (by norm_num) (by norm_num [ep, tau])
  have m_ww : |(((X * X) * (X * X)) * ((X * X) * (X * X)))| ≤ (73 / 500 : ℚ) := le_trans m_ww0 (by norm_num)
  obtain ⟨f_zC6, e_zC6, m_zC60⟩ := mulA f_z fC6 (E := (157 / 100000000000000000000000000000 : ℚ)) e_z (self_err C6) m_z mC6 (by norm_num) (by norm_num [ep, tau])
  have m_zC6 : |((X * X) * C6.val)| ≤ (11 / 1562500000000 : ℚ) := le_trans m_zC60 (by norm_num)
  obtain ⟨f_c5, e_c5, m_c50⟩ := addA fC5 f_zC6 (E := (47 / 200000000000000000000000000 : ℚ)) (self_err C5) e_zC6 mC5 m_zC6 (by norm_num) (by norm_num [ep, tau])
  have m_c5 : |(C5.val + ((X * X) * C6.val))| ≤ (21 / 10000000000 : ℚ) := le_trans m_c50 (by norm_num)
  obtain ⟨f_zc5, e_zc5, m_zc50⟩ := mulA f_z f_c5 (E := (433 / 1000000000000000000000000000 : ℚ)) e_z e_c5 m_z m_c5 (by norm_num) (by norm_num [ep, tau])
  have m_zc5 : |((X * X) * (C5.val + ((X * X) * C6.val)))| ≤ (13 / 10000000000 : ℚ) := le_trans m_zc50 (by norm_num)
  obtain ⟨f_c4, e_c4, m_c40⟩ := addA fC4 f_zc5 (E := (313 / 10000000000000000000000000 : ℚ)) (self_err C4) e_zc5 mC4 m_zc5 (by norm_num) (by norm_num [ep, tau])
  have m_c4 : |(C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))| ≤ (139 / 500000000 : ℚ) := le_trans m_c40 (by norm_num)
  obtain ⟨f_t2, e_t2, m_t20⟩ := mulA f_ww f_c4 (E := (81 / 2000000000000000000000000 : ℚ)) e_ww e_c4 m_ww m_c4 (by norm_num) (by norm_num [ep, tau])
  have m_t2 : |((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val)))))| ≤ (203 / 5000000000 : ℚ) := le_trans m_t20 (by norm_num)
  obtain ⟨f_r, e_r, m_r0⟩ := addA f_t1 f_t2 (E := (119 / 10000000000000000000 : ℚ)) e_t1 e_t2 m_t1 m_t2 (by norm_num) (by norm_num [ep, tau])
  have m_r : |(((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))| ≤ (33 / 1250 : ℚ) := le_trans m_r0 (by norm_num)
  obtain ⟨f_zr, e_zr, m_zr0⟩ := mulA f_z f_r (E := (11 / 1000000000000000000 : ℚ)) e_z e_r m_z m_r (by norm_num) (by norm_num [ep, tau])
  have m_zr : |((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val)))))))| ≤ (163 / 10000 : ℚ) := le_trans m_zr0 (by norm_num)
  obtain ⟨f_xy, e_xy, m_xy0⟩ := mulA hx hy (E := (159 / 100000000000000000000000000000000 : ℚ)) (self_err x) (self_err y) hxb hyb (by norm_num) (by norm_num [ep, tau])
  have m_xy : |(X * Y)| ≤ (393 / 5000000000000000000 : ℚ) := le_trans m_xy0 (by norm_num)
  obtain ⟨f_t, e_t, m_t0⟩ := subA f_zr f_xy (E := (129 / 10000000000000000000 : ℚ)) e_zr e_xy m_zr m_xy (by norm_num) (by norm_num [ep, tau])
  have m_t : |(((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))) - (X * Y))| ≤ (41 / 2500 : ℚ) := le_trans m_t0 (by norm_num)

  obtain ⟨f_hz, e_hz0⟩ := half_mul f_z
  have h1073 : (2 : ℚ) ^ (-1073 : ℤ) ≤ 1 / 2 ^ 100 := le_trans (two_zpow_le (show (-1073 : ℤ) ≤ -100 by norm_num)) (by rw [zpow_neg, zpow_ofNat]; norm_num)
  have e_hz : |(mul HALF (mul x x)).val - X * X / 2| ≤ (343 / 10000000000000000000 : ℚ) := by
    have h1 := abs_sub_le (mul HALF (mul x x)).val ((mul x x).val / 2) (X * X / 2)
    have h2 : |(mul x x).val / 2 - X * X / 2| = |(mul x x).val - X * X| / 2 := by
      rw [← sub_div, abs_div]; norm_num
    rw [h2] at h1; linarith
  have m_hz : |X * X / 2| ≤ (617 / 2000 : ℚ) := by rw [abs_div, abs_two]; linarith
  have hONE : |ONE.val - 1| ≤ 0 := by rw [ONE_val]; norm_num
  obtain ⟨f_w', e_w', -⟩ := subA fONE f_hz (MA := 1) (E := (9 / 50000000000000000 : ℚ)) hONE e_hz (by norm_num) m_hz (by norm_num) (by norm_num [ep])
  have hzz : 0 ≤ X * X := mul_self_nonneg X
  have hzz' : X * X ≤ (617 / 1000 : ℚ) := le_trans (le_abs_self _) m_z
  have hw1 : (sub ONE (mul HALF (mul x x))).val ≤ 1 + (9 / 50000000000000000 : ℚ) := by have := (abs_le.mp e_w').2; linarith
  have hw2 : (34574999999999991 / 50000000000000000 : ℚ) ≤ (sub ONE (mul HALF (mul x x))).val := by have := (abs_le.mp e_w').1; linarith
  obtain ⟨f_a1, v_a1⟩ := sub_of_rep fONE f_w' (rep_sub fONE f_w' (by rw [ONE_val]; linarith) (by rw [ONE_val]; linarith))
  have r2 : Rep binary64 ((sub ONE (sub ONE (mul HALF (mul x x)))).val - (mul HALF (mul x x)).val) := by
    have hn : ((mul HALF (mul x x))).neg.isFinite := by simpa using f_hz
    have := add_err_representable (f := binary64) fONE hn f_w' false
    have e : (sub ONE (sub ONE (mul HALF (mul x x)))).val - (mul HALF (mul x x)).val = ONE.val + ((mul HALF (mul x x))).neg.val - (add ONE ((mul HALF (mul x x))).neg).val := by
      rw [v_a1, FP.val_neg]; show _ = _ - (sub ONE (mul HALF (mul x x))).val; ring
    rw [e]; exact rep_of_round this
  obtain ⟨f_a2, v_a2⟩ := sub_of_rep f_a1 f_hz r2
  have m_a2 : |(sub (sub ONE (sub ONE (mul HALF (mul x x)))) (mul HALF (mul x x))).val| ≤ (43 / 200000000000000000 : ℚ) := by
    rw [v_a2, v_a1, ONE_val]
    have := abs_le.mp e_w'; have := abs_le.mp e_hz; rw [abs_le]; constructor <;> linarith
  obtain ⟨f_s, e_s, -⟩ := addA f_a2 f_t (E := (37 / 2500000000000000000 : ℚ)) (self_err _) e_t m_a2 m_t (by norm_num) (by norm_num [ep])
  have m_wp : |(sub ONE (mul HALF (mul x x))).val| ≤ (1000000000000001 / 1000000000000000 : ℚ) := by rw [abs_le]; constructor <;> linarith
  obtain ⟨f_res, e_res, -⟩ := addA f_w' f_s (E := (1 / 7812500000000000 : ℚ)) (self_err _) e_s m_wp (le_trans (abs_add_le _ _) (add_le_add m_a2 m_t)) (by norm_num) (by norm_num [ep])
  refine ⟨f_res, ?_⟩
  have key : (sub ONE (mul HALF (mul x x))).val + ((sub (sub ONE (sub ONE (mul HALF (mul x x)))) (mul HALF (mul x x))).val + (((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))) - (X * Y))) = (1 - (mul HALF (mul x x)).val) + (((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))) - (X * Y)) := by
    rw [v_a2, v_a1, ONE_val]; ring
  have key2 : evalP kcosL (X ^ 2) - X * Y = (1 - X * X / 2) + (((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))) - (X * Y)) := by
    simp only [evalP, kcosL, C1_val, C2_val, C3_val, C4_val, C5_val, C6_val]; ring
  rw [key] at e_res
  show |(add (sub ONE (mul HALF (mul x x))) (add (sub (sub ONE (sub ONE (mul HALF (mul x x)))) (mul HALF (mul x x))) (sub (mul (mul x x) (add (mul (mul x x) (add C1 (mul (mul x x) (add C2 (mul (mul x x) C3))))) (mul (mul (mul (mul x x) (mul x x)) (mul (mul x x) (mul x x))) (add C4 (mul (mul x x) (add C5 (mul (mul x x) C6))))))) (mul x y)))).val - _| ≤ _
  rw [key2]
  have h3 := abs_sub_le (add (sub ONE (mul HALF (mul x x))) (add (sub (sub ONE (sub ONE (mul HALF (mul x x)))) (mul HALF (mul x x))) (sub (mul (mul x x) (add (mul (mul x x) (add C1 (mul (mul x x) (add C2 (mul (mul x x) C3))))) (mul (mul (mul (mul x x) (mul x x)) (mul (mul x x) (mul x x))) (add C4 (mul (mul x x) (add C5 (mul (mul x x) C6))))))) (mul x y)))).val ((1 - (mul HALF (mul x x)).val) + (((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))) - (X * Y))) ((1 - X * X / 2) + (((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))) - (X * Y)))
  have h4 : (1 - (mul HALF (mul x x)).val) + (((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))) - (X * Y)) - ((1 - X * X / 2) + (((X * X) * (((X * X) * (C1.val + ((X * X) * (C2.val + ((X * X) * C3.val))))) + ((((X * X) * (X * X)) * ((X * X) * (X * X))) * (C4.val + ((X * X) * (C5.val + ((X * X) * C6.val))))))) - (X * Y))) = -((mul HALF (mul x x)).val - X * X / 2) := by ring
  rw [h4, abs_neg] at h3
  linarith

end IEEE754.Dmath
