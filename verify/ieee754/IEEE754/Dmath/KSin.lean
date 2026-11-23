/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.Approx
import IEEE754.Dmath.SinPoly

/-!
# `dmath::k_sin`'s evaluation

`ksin_eval`: for `|x| ≤ 0.78542`, `|y| ≤ 10^-16`, `k_sin(x, y)` is finite and within `1.36·10^-16`
of `x·ksinL(x²) + y·(1 − x²/2)`, its value in exact arithmetic. Each operation goes through the
error calculus (`mulA`, `addA`, `subA`) with the bound written out.
-/

namespace IEEE754.Dmath

open FP

def HALF : F64 := .fin false 4503599627370496 (-53) (by decide)
theorem HALF_bits : HALF.toBits = 0x3fe0000000000000 := by decide
theorem HALF_val : HALF.val = (1 / 2 : ℚ) := by simp only [HALF, val, sgn]; norm_num

/-- `dmath::k_sin`, operation for operation. -/
def ksin (x y : F64) : F64 :=
  let z := mul x x
  let w := mul z z
  let r := add (add S2 (mul z (add S3 (mul z S4)))) (mul (mul z w) (add S5 (mul z S6)))
  let v := mul z x
  sub x (sub (sub (mul z (sub (mul HALF y) (mul v r))) y) (mul v S1))

theorem ksin_eval {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hxb : |x.val| ≤ 78542 / 100000)
    (hyb : |y.val| ≤ 1 / 10 ^ 16) :
    (ksin x y).isFinite ∧
      |(ksin x y).val - (x.val * evalP ksinL (x.val ^ 2) + y.val * (1 - x.val ^ 2 / 2))| ≤ (17 / 125000000000000000 : ℚ) := by
  have fc : S1.isFinite ∧ S2.isFinite ∧ S3.isFinite ∧ S4.isFinite ∧ S5.isFinite ∧ S6.isFinite ∧ HALF.isFinite := by decide
  obtain ⟨fS1, fS2, fS3, fS4, fS5, fS6, fHALF⟩ := fc
  have mS1 : |S1.val| ≤ (167 / 1000 : ℚ) := by rw [S1_val]; norm_num
  have mS2 : |S2.val| ≤ (417 / 50000 : ℚ) := by rw [S2_val]; norm_num
  have mS3 : |S3.val| ≤ (199 / 1000000 : ℚ) := by rw [S3_val]; norm_num
  have mS4 : |S4.val| ≤ (69 / 25000000 : ℚ) := by rw [S4_val]; norm_num
  have mS5 : |S5.val| ≤ (251 / 10000000000 : ℚ) := by rw [S5_val]; norm_num
  have mS6 : |S6.val| ≤ (159 / 1000000000000 : ℚ) := by rw [S6_val]; norm_num
  have mHALF : |HALF.val| ≤ 1 / 2 := by rw [HALF_val]; norm_num
  set X := x.val
  set Y := y.val

  obtain ⟨f_z, e_z, m_z0⟩ := mulA hx hx (E := (137 / 2000000000000000000 : ℚ)) (self_err x) (self_err x) hxb hxb (by norm_num) (by norm_num [ep, tau])
  have m_z : |(X * X)| ≤ (617 / 1000 : ℚ) := le_trans m_z0 (by norm_num)
  obtain ⟨f_w, e_w, m_w0⟩ := mulA f_z f_z (E := (127 / 1000000000000000000 : ℚ)) e_z e_z m_z m_z (by norm_num) (by norm_num [ep, tau])
  have m_w : |((X * X) * (X * X))| ≤ (381 / 1000 : ℚ) := le_trans m_w0 (by norm_num)
  obtain ⟨f_zS4, e_zS4, m_zS40⟩ := mulA f_z fS4 (E := (379 / 1000000000000000000000000 : ℚ)) e_z (self_err S4) m_z mS4 (by norm_num) (by norm_num [ep, tau])
  have m_zS4 : |((X * X) * S4.val)| ≤ (171 / 100000000 : ℚ) := le_trans m_zS40 (by norm_num)
  obtain ⟨f_s3, e_s3, m_s30⟩ := addA fS3 f_zS4 (E := (227 / 10000000000000000000000 : ℚ)) (self_err S3) e_zS4 mS3 m_zS4 (by norm_num) (by norm_num [ep, tau])
  have m_s3 : |(S3.val + ((X * X) * S4.val))| ≤ (201 / 1000000 : ℚ) := le_trans m_s30 (by norm_num)
  obtain ⟨f_zs3, e_zs3, m_zs30⟩ := mulA f_z f_s3 (E := (13 / 312500000000000000000 : ℚ)) e_z e_s3 m_z m_s3 (by norm_num) (by norm_num [ep, tau])
  have m_zs3 : |((X * X) * (S3.val + ((X * X) * S4.val)))| ≤ (1 / 8000 : ℚ) := le_trans m_zs30 (by norm_num)
  obtain ⟨f_s2, e_s2, m_s20⟩ := addA fS2 f_zs3 (E := (491 / 500000000000000000000 : ℚ)) (self_err S2) e_zs3 mS2 m_zs3 (by norm_num) (by norm_num [ep, tau])
  have m_s2 : |(S2.val + ((X * X) * (S3.val + ((X * X) * S4.val))))| ≤ (847 / 100000 : ℚ) := le_trans m_s20 (by norm_num)
  obtain ⟨f_zw, e_zw, m_zw0⟩ := mulA f_z f_w (E := (131 / 1000000000000000000 : ℚ)) e_z e_w m_z m_w (by norm_num) (by norm_num [ep, tau])
  have m_zw : |((X * X) * ((X * X) * (X * X)))| ≤ (59 / 250 : ℚ) := le_trans m_zw0 (by norm_num)
  obtain ⟨f_zS6, e_zS6, m_zS60⟩ := mulA f_z fS6 (E := (109 / 5000000000000000000000000000 : ℚ)) e_z (self_err S6) m_z mS6 (by norm_num) (by norm_num [ep, tau])
  have m_zS6 : |((X * X) * S6.val)| ≤ (491 / 5000000000000 : ℚ) := le_trans m_zS60 (by norm_num)
  obtain ⟨f_s5, e_s5, m_s50⟩ := addA fS5 f_zS6 (E := (141 / 50000000000000000000000000 : ℚ)) (self_err S5) e_zS6 mS5 m_zS6 (by norm_num) (by norm_num [ep, tau])
  have m_s5 : |(S5.val + ((X * X) * S6.val))| ≤ (63 / 2500000000 : ℚ) := le_trans m_s50 (by norm_num)
  obtain ⟨f_t2, e_t2, m_t20⟩ := mulA f_zw f_s5 (E := (463 / 100000000000000000000000000 : ℚ)) e_zw e_s5 m_zw m_s5 (by norm_num) (by norm_num [ep, tau])
  have m_t2 : |(((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val)))| ≤ (119 / 20000000000 : ℚ) := le_trans m_t20 (by norm_num)
  obtain ⟨f_r, e_r, m_r0⟩ := addA f_s2 f_t2 (E := (193 / 100000000000000000000 : ℚ)) e_s2 e_t2 m_s2 m_t2 (by norm_num) (by norm_num [ep, tau])
  have m_r : |((S2.val + ((X * X) * (S3.val + ((X * X) * S4.val)))) + (((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val))))| ≤ (53 / 6250 : ℚ) := le_trans m_r0 (by norm_num)
  obtain ⟨f_v, e_v, m_v0⟩ := mulA f_z hx (E := (27 / 250000000000000000 : ℚ)) e_z (self_err x) m_z hxb (by norm_num) (by norm_num [ep, tau])
  have m_v : |((X * X) * X)| ≤ (97 / 200 : ℚ) := le_trans m_v0 (by norm_num)
  obtain ⟨f_hy, e_hy, m_hy0⟩ := mulA fHALF hy (E := (159 / 100000000000000000000000000000000 : ℚ)) (self_err HALF) (self_err y) mHALF hyb (by norm_num) (by norm_num [ep, tau])
  have m_hy : |(HALF.val * Y)| ≤ (1 / 20000000000000000 : ℚ) := le_trans m_hy0 (by norm_num)
  obtain ⟨f_vr, e_vr, m_vr0⟩ := mulA f_v f_r (E := (231 / 100000000000000000000 : ℚ)) e_v e_r m_v m_r (by norm_num) (by norm_num [ep, tau])
  have m_vr : |(((X * X) * X) * ((S2.val + ((X * X) * (S3.val + ((X * X) * S4.val)))) + (((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val)))))| ≤ (103 / 25000 : ℚ) := le_trans m_vr0 (by norm_num)
  obtain ⟨f_d1, e_d1, m_d10⟩ := subA f_hy f_vr (E := (277 / 100000000000000000000 : ℚ)) e_hy e_vr m_hy m_vr (by norm_num) (by norm_num [ep, tau])
  have m_d1 : |((HALF.val * Y) - (((X * X) * X) * ((S2.val + ((X * X) * (S3.val + ((X * X) * S4.val)))) + (((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val))))))| ≤ (413 / 100000 : ℚ) := le_trans m_d10 (by norm_num)
  obtain ⟨f_zd, e_zd, m_zd0⟩ := mulA f_z f_d1 (E := (57 / 25000000000000000000 : ℚ)) e_z e_d1 m_z m_d1 (by norm_num) (by norm_num [ep, tau])
  have m_zd : |((X * X) * ((HALF.val * Y) - (((X * X) * X) * ((S2.val + ((X * X) * (S3.val + ((X * X) * S4.val)))) + (((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val)))))))| ≤ (51 / 20000 : ℚ) := le_trans m_zd0 (by norm_num)
  obtain ⟨f_d2, e_d2, m_d20⟩ := subA f_zd hy (E := (257 / 100000000000000000000 : ℚ)) e_zd (self_err y) m_zd hyb (by norm_num) (by norm_num [ep, tau])
  have m_d2 : |(((X * X) * ((HALF.val * Y) - (((X * X) * X) * ((S2.val + ((X * X) * (S3.val + ((X * X) * S4.val)))) + (((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val))))))) - Y)| ≤ (8 / 3125 : ℚ) := le_trans m_d20 (by norm_num)
  obtain ⟨f_vs, e_vs, m_vs0⟩ := mulA f_v fS1 (E := (271 / 10000000000000000000 : ℚ)) e_v (self_err S1) m_v mS1 (by norm_num) (by norm_num [ep, tau])
  have m_vs : |(((X * X) * X) * S1.val)| ≤ (81 / 1000 : ℚ) := le_trans m_vs0 (by norm_num)
  obtain ⟨f_d3, e_d3, m_d30⟩ := subA f_d2 f_vs (E := (39 / 1000000000000000000 : ℚ)) e_d2 e_vs m_d2 m_vs (by norm_num) (by norm_num [ep, tau])
  have m_d3 : |((((X * X) * ((HALF.val * Y) - (((X * X) * X) * ((S2.val + ((X * X) * (S3.val + ((X * X) * S4.val)))) + (((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val))))))) - Y) - (((X * X) * X) * S1.val))| ≤ (209 / 2500 : ℚ) := le_trans m_d30 (by norm_num)
  obtain ⟨f_res, e_res, m_res0⟩ := subA hx f_d3 (E := (17 / 125000000000000000 : ℚ)) (self_err x) e_d3 hxb m_d3 (by norm_num) (by norm_num [ep, tau])
  have m_res : |(X - ((((X * X) * ((HALF.val * Y) - (((X * X) * X) * ((S2.val + ((X * X) * (S3.val + ((X * X) * S4.val)))) + (((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val))))))) - Y) - (((X * X) * X) * S1.val)))| ≤ (87 / 100 : ℚ) := le_trans m_res0 (by norm_num)
  refine ⟨f_res, ?_⟩
  have key : (X - ((((X * X) * ((HALF.val * Y) - (((X * X) * X) * ((S2.val + ((X * X) * (S3.val + ((X * X) * S4.val)))) + (((X * X) * ((X * X) * (X * X))) * (S5.val + ((X * X) * S6.val))))))) - Y) - (((X * X) * X) * S1.val))) = X * evalP ksinL (X ^ 2) + Y * (1 - X ^ 2 / 2) := by
    simp only [evalP, ksinL, S1_val, S2_val, S3_val, S4_val, S5_val, S6_val, HALF_val]; ring
  rw [← key]; exact e_res

end IEEE754.Dmath
