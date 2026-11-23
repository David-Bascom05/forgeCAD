/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.AtanConsts
import IEEE754.Dmath.AtanPoly
import IEEE754.Dmath.Ulp

/-!
# `atan`'s polynomial in floating point

`atanPoly t` is `ax * (s1 + s2)` of `dmath::atan`, operation for operation. `atanPoly_eval`: for
finite `|t| ≤ 0.4375` it is finite and within 1.88e-17 of `t − t·katanL(t²)`, its value in
exact arithmetic.
-/

namespace IEEE754.Dmath

open FP

/-- `dmath::atan`'s `ax * (s1 + s2)`. -/
def atanPoly (t : F64) : F64 :=
  let z := mul t t
  let w := mul z z
  let s1 := mul z (add AT0 (mul w (add AT2 (mul w (add AT4 (mul w (add AT6 (mul w (add AT8 (mul w AT10))))))))))
  let s2 := mul w (add AT1 (mul w (add AT3 (mul w (add AT5 (mul w (add AT7 (mul w AT9))))))))
  mul t (add s1 s2)

theorem atanPoly_eval {t : F64} (ht : t.isFinite) (htb : |t.val| ≤ 4375 / 10000) :
    (atanPoly t).isFinite ∧
      |(atanPoly t).val - (t.val - t.val * evalP katanL (t.val ^ 2))| ≤ (47 / 2500000000000000000 : ℚ) ∧
      |t.val - t.val * evalP katanL (t.val ^ 2)| ≤ (319 / 10000 : ℚ) := by
  have fc : AT0.isFinite ∧ AT1.isFinite ∧ AT2.isFinite ∧ AT3.isFinite ∧ AT4.isFinite ∧ AT5.isFinite ∧ AT6.isFinite ∧ AT7.isFinite ∧ AT8.isFinite ∧ AT9.isFinite ∧ AT10.isFinite := by decide
  obtain ⟨fAT0, fAT1, fAT2, fAT3, fAT4, fAT5, fAT6, fAT7, fAT8, fAT9, fAT10⟩ := fc
  have mAT0 : |AT0.val| ≤ (167 / 500 : ℚ) := by rw [AT0_val]; norm_num
  have mAT1 : |AT1.val| ≤ (1 / 5 : ℚ) := by rw [AT1_val]; norm_num
  have mAT2 : |AT2.val| ≤ (143 / 1000 : ℚ) := by rw [AT2_val]; norm_num
  have mAT3 : |AT3.val| ≤ (14 / 125 : ℚ) := by rw [AT3_val]; norm_num
  have mAT4 : |AT4.val| ≤ (91 / 1000 : ℚ) := by rw [AT4_val]; norm_num
  have mAT5 : |AT5.val| ≤ (77 / 1000 : ℚ) := by rw [AT5_val]; norm_num
  have mAT6 : |AT6.val| ≤ (667 / 10000 : ℚ) := by rw [AT6_val]; norm_num
  have mAT7 : |AT7.val| ≤ (73 / 1250 : ℚ) := by rw [AT7_val]; norm_num
  have mAT8 : |AT8.val| ≤ (249 / 5000 : ℚ) := by rw [AT8_val]; norm_num
  have mAT9 : |AT9.val| ≤ (183 / 5000 : ℚ) := by rw [AT9_val]; norm_num
  have mAT10 : |AT10.val| ≤ (163 / 10000 : ℚ) := by rw [AT10_val]; norm_num
  set T := t.val
  obtain ⟨f_z, e_z, m_z0⟩ := mulA ht ht (E := (213 / 10000000000000000000 : ℚ)) (self_err t) (self_err t) htb htb (by norm_num) (by norm_num [ep, tau])
  have m_z : |(T * T)| ≤ (24 / 125 : ℚ) := le_trans m_z0 (by norm_num)
  obtain ⟨f_w, e_w, m_w0⟩ := mulA f_z f_z (E := (123 / 10000000000000000000 : ℚ)) e_z e_z m_z m_z (by norm_num) (by norm_num [ep, tau])
  have m_w : |((T * T) * (T * T))| ≤ (369 / 10000 : ℚ) := le_trans m_w0 (by norm_num)
  obtain ⟨f_wa10, e_wa10, m_wa100⟩ := mulA f_w fAT10 (E := (67 / 250000000000000000000 : ℚ)) e_w (self_err AT10) m_w mAT10 (by norm_num) (by norm_num [ep, tau])
  have m_wa10 : |(((T * T) * (T * T)) * AT10.val)| ≤ (301 / 500000 : ℚ) := le_trans m_wa100 (by norm_num)
  obtain ⟨f_a8, e_a8, m_a80⟩ := addA fAT8 f_wa10 (E := (587 / 100000000000000000000 : ℚ)) (self_err AT8) e_wa10 mAT8 m_wa10 (by norm_num) (by norm_num [ep])
  have m_a8 : |(AT8.val + (((T * T) * (T * T)) * AT10.val))| ≤ (101 / 2000 : ℚ) := le_trans m_a80 (by norm_num)
  obtain ⟨f_wa8, e_wa8, m_wa80⟩ := mulA f_w f_a8 (E := (21 / 20000000000000000000 : ℚ)) e_w e_a8 m_w m_a8 (by norm_num) (by norm_num [ep, tau])
  have m_wa8 : |(((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val)))| ≤ (187 / 100000 : ℚ) := le_trans m_wa80 (by norm_num)
  obtain ⟨f_a6, e_a6, m_a60⟩ := addA fAT6 f_wa8 (E := (867 / 100000000000000000000 : ℚ)) (self_err AT6) e_wa8 mAT6 m_wa8 (by norm_num) (by norm_num [ep])
  have m_a6 : |(AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val))))| ≤ (343 / 5000 : ℚ) := le_trans m_a60 (by norm_num)
  obtain ⟨f_wa6, e_wa6, m_wa60⟩ := mulA f_w f_a6 (E := (29 / 20000000000000000000 : ℚ)) e_w e_a6 m_w m_a6 (by norm_num) (by norm_num [ep, tau])
  have m_wa6 : |(((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val)))))| ≤ (127 / 50000 : ℚ) := le_trans m_wa60 (by norm_num)
  obtain ⟨f_a4, e_a4, m_a40⟩ := addA fAT4 f_wa6 (E := (119 / 10000000000000000000 : ℚ)) (self_err AT4) e_wa6 mAT4 m_wa6 (by norm_num) (by norm_num [ep])
  have m_a4 : |(AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val))))))| ≤ (117 / 1250 : ℚ) := le_trans m_a40 (by norm_num)
  obtain ⟨f_wa4, e_wa4, m_wa40⟩ := mulA f_w f_a4 (E := (99 / 50000000000000000000 : ℚ)) e_w e_a4 m_w m_a4 (by norm_num) (by norm_num [ep, tau])
  have m_wa4 : |(((T * T) * (T * T)) * (AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val)))))))| ≤ (173 / 50000 : ℚ) := le_trans m_wa40 (by norm_num)
  obtain ⟨f_a2, e_a2, m_a20⟩ := addA fAT2 f_wa4 (E := (183 / 10000000000000000000 : ℚ)) (self_err AT2) e_wa4 mAT2 m_wa4 (by norm_num) (by norm_num [ep])
  have m_a2 : |(AT2.val + (((T * T) * (T * T)) * (AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val))))))))| ≤ (147 / 1000 : ℚ) := le_trans m_a20 (by norm_num)
  obtain ⟨f_wa2, e_wa2, m_wa20⟩ := mulA f_w f_a2 (E := (309 / 100000000000000000000 : ℚ)) e_w e_a2 m_w m_a2 (by norm_num) (by norm_num [ep, tau])
  have m_wa2 : |(((T * T) * (T * T)) * (AT2.val + (((T * T) * (T * T)) * (AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val)))))))))| ≤ (543 / 100000 : ℚ) := le_trans m_wa20 (by norm_num)
  obtain ⟨f_a0, e_a0, m_a00⟩ := addA fAT0 f_wa2 (E := (51 / 1250000000000000000 : ℚ)) (self_err AT0) e_wa2 mAT0 m_wa2 (by norm_num) (by norm_num [ep])
  have m_a0 : |(AT0.val + (((T * T) * (T * T)) * (AT2.val + (((T * T) * (T * T)) * (AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val))))))))))| ≤ (17 / 50 : ℚ) := le_trans m_a00 (by norm_num)
  obtain ⟨f_s1, e_s1, m_s10⟩ := mulA f_z f_a0 (E := (7 / 312500000000000000 : ℚ)) e_z e_a0 m_z m_a0 (by norm_num) (by norm_num [ep, tau])
  have m_s1 : |((T * T) * (AT0.val + (((T * T) * (T * T)) * (AT2.val + (((T * T) * (T * T)) * (AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val)))))))))))| ≤ (653 / 10000 : ℚ) := le_trans m_s10 (by norm_num)
  obtain ⟨f_wa9, e_wa9, m_wa90⟩ := mulA f_w fAT9 (E := (601 / 1000000000000000000000 : ℚ)) e_w (self_err AT9) m_w mAT9 (by norm_num) (by norm_num [ep, tau])
  have m_wa9 : |(((T * T) * (T * T)) * AT9.val)| ≤ (17 / 12500 : ℚ) := le_trans m_wa90 (by norm_num)
  obtain ⟨f_a7, e_a7, m_a70⟩ := addA fAT7 f_wa9 (E := (181 / 25000000000000000000 : ℚ)) (self_err AT7) e_wa9 mAT7 m_wa9 (by norm_num) (by norm_num [ep])
  have m_a7 : |(AT7.val + (((T * T) * (T * T)) * AT9.val))| ≤ (299 / 5000 : ℚ) := le_trans m_a70 (by norm_num)
  obtain ⟨f_wa7, e_wa7, m_wa70⟩ := mulA f_w f_a7 (E := (1 / 800000000000000000 : ℚ)) e_w e_a7 m_w m_a7 (by norm_num) (by norm_num [ep, tau])
  have m_wa7 : |(((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val)))| ≤ (221 / 100000 : ℚ) := le_trans m_wa70 (by norm_num)
  obtain ⟨f_a5, e_a5, m_a50⟩ := addA fAT5 f_wa7 (E := (101 / 10000000000000000000 : ℚ)) (self_err AT5) e_wa7 mAT5 m_wa7 (by norm_num) (by norm_num [ep])
  have m_a5 : |(AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val))))| ≤ (793 / 10000 : ℚ) := le_trans m_a50 (by norm_num)
  obtain ⟨f_wa5, e_wa5, m_wa50⟩ := mulA f_w f_a5 (E := (21 / 12500000000000000000 : ℚ)) e_w e_a5 m_w m_a5 (by norm_num) (by norm_num [ep, tau])
  have m_wa5 : |(((T * T) * (T * T)) * (AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val)))))| ≤ (293 / 100000 : ℚ) := le_trans m_wa50 (by norm_num)
  obtain ⟨f_a3, e_a3, m_a30⟩ := addA fAT3 f_wa5 (E := (29 / 2000000000000000000 : ℚ)) (self_err AT3) e_wa5 mAT3 m_wa5 (by norm_num) (by norm_num [ep])
  have m_a3 : |(AT3.val + (((T * T) * (T * T)) * (AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val))))))| ≤ (23 / 200 : ℚ) := le_trans m_a30 (by norm_num)
  obtain ⟨f_wa3, e_wa3, m_wa30⟩ := mulA f_w f_a3 (E := (243 / 100000000000000000000 : ℚ)) e_w e_a3 m_w m_a3 (by norm_num) (by norm_num [ep, tau])
  have m_wa3 : |(((T * T) * (T * T)) * (AT3.val + (((T * T) * (T * T)) * (AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val)))))))| ≤ (17 / 4000 : ℚ) := le_trans m_wa30 (by norm_num)
  obtain ⟨f_a1, e_a1, m_a10⟩ := addA fAT1 f_wa3 (E := (63 / 2500000000000000000 : ℚ)) (self_err AT1) e_wa3 mAT1 m_wa3 (by norm_num) (by norm_num [ep])
  have m_a1 : |(AT1.val + (((T * T) * (T * T)) * (AT3.val + (((T * T) * (T * T)) * (AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val))))))))| ≤ (41 / 200 : ℚ) := le_trans m_a10 (by norm_num)
  obtain ⟨f_s2, e_s2, m_s20⟩ := mulA f_w f_a1 (E := (43 / 10000000000000000000 : ℚ)) e_w e_a1 m_w m_a1 (by norm_num) (by norm_num [ep, tau])
  have m_s2 : |(((T * T) * (T * T)) * (AT1.val + (((T * T) * (T * T)) * (AT3.val + (((T * T) * (T * T)) * (AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val)))))))))| ≤ (757 / 100000 : ℚ) := le_trans m_s20 (by norm_num)
  obtain ⟨f_s, e_s, m_s0⟩ := addA f_s1 f_s2 (E := (87 / 2500000000000000000 : ℚ)) e_s1 e_s2 m_s1 m_s2 (by norm_num) (by norm_num [ep])
  have m_s : |(((T * T) * (AT0.val + (((T * T) * (T * T)) * (AT2.val + (((T * T) * (T * T)) * (AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val))))))))))) + (((T * T) * (T * T)) * (AT1.val + (((T * T) * (T * T)) * (AT3.val + (((T * T) * (T * T)) * (AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val))))))))))| ≤ (729 / 10000 : ℚ) := le_trans m_s0 (by norm_num)
  obtain ⟨f_p, e_p, m_p0⟩ := mulA ht f_s (E := (47 / 2500000000000000000 : ℚ)) (self_err t) e_s htb m_s (by norm_num) (by norm_num [ep, tau])
  have m_p : |(T * (((T * T) * (AT0.val + (((T * T) * (T * T)) * (AT2.val + (((T * T) * (T * T)) * (AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val))))))))))) + (((T * T) * (T * T)) * (AT1.val + (((T * T) * (T * T)) * (AT3.val + (((T * T) * (T * T)) * (AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val)))))))))))| ≤ (319 / 10000 : ℚ) := le_trans m_p0 (by norm_num)
  have key : (T * (((T * T) * (AT0.val + (((T * T) * (T * T)) * (AT2.val + (((T * T) * (T * T)) * (AT4.val + (((T * T) * (T * T)) * (AT6.val + (((T * T) * (T * T)) * (AT8.val + (((T * T) * (T * T)) * AT10.val))))))))))) + (((T * T) * (T * T)) * (AT1.val + (((T * T) * (T * T)) * (AT3.val + (((T * T) * (T * T)) * (AT5.val + (((T * T) * (T * T)) * (AT7.val + (((T * T) * (T * T)) * AT9.val))))))))))) = T - T * evalP katanL (T ^ 2) := by
    simp only [evalP, katanL, AT0_val, AT1_val, AT2_val, AT3_val, AT4_val, AT5_val, AT6_val, AT7_val, AT8_val, AT9_val, AT10_val]; ring
  rw [key] at e_p m_p
  exact ⟨f_p, e_p, m_p⟩

end IEEE754.Dmath
