import ForgeSheetCore.Funs
import ForgeSheetCore.SetbackErr
import ForgeSheetCore.RefinePlace
import ForgeSheetCore.RefineJoint
import IEEE754.Dmath.SinCos
open Aeneas Aeneas.Std Result

/-!
# `fold::setback`: the setback through the f64 bridge, with dmath's sine and cosine

build.rs computes `(s, c) = dmath::sin_cos(θ)` and then `setback(ρ, s, c) = ρ ⊗ s ⊘ (1 ⊕ c)`
(shop.rs, op for op). With verify/ieee754's `Dmath.sinCos_err` (the model `Dmath.sinCos` is the
shipped dmath, the aeneas session's equality) and each operation within `u |r| + η`
(`RefinePlace`: `mul_err`, `add_err`, and `div_err` here), `setback_err` gives
(`setback_spec`): for finite `ρ`, finite `θ` with `|θ| ≤ 2²⁰` and `C = 1 + cos θ ≥ 2⁻⁵⁰`, the
computed setback, when finite, is within
`|ρ| (4ε + 4η) / C² + 10 u |ρ| / C + 3η / C + η` (`ε = 2⁻⁵²`) of `ρ sin θ / C = ρ tan(θ / 2)`.
-/

set_option exponentiation.threshold 1100

namespace forge_sheet_core

open IEEE754

set_option maxRecDepth 20000 in
/-- A rounding of a small value is finite. -/
theorem round_fin_q {zs : Bool} {r : ℚ} (h : |r| ≤ 16) : (IEEE754.round binary64 zs r).isFinite := by
  by_cases hr : r = 0
  · subst hr; simp [IEEE754.round, FP.isFinite]
  rw [round_finite_iff (K := ℚ) binary64 hr]
  have h16 : (16 : ℚ) = ((2 ^ 52 : ℕ) : ℚ) * 2 ^ (-48 : ℤ) := by norm_num
  have hex : uround binary64 (16 : ℚ) = 16 := by
    rw [h16, uround_exact binary64 (by positivity) (by simp) (by simp)]
  have hm := uround_mono binary64 (abs_pos.mpr hr) h
  rw [hex] at hm
  have : (16 : ℚ) < 2 ^ (binary64.emax + 1) := by
    simp only [binary64_emax]
    exact lt_of_lt_of_le (by norm_num : (16 : ℚ) < 2 ^ (5 : ℤ)) (two_zpow_le (by norm_num))
  linarith

theorem div_err {a b : IEEE754.F64} (ha : a.isFinite) (hb : b.isFinite) (hb0 : b.val ≠ 0)
    (h : (FP.div a b).isFinite) :
    |(FP.div a b).val - a.val / b.val| ≤ uu * |a.val / b.val| + η := by
  rw [div_eq_round ha hb hb0] at h ⊢; exact round_le h

/-- The real-valued forms. -/
theorem cast_err {x r : ℚ} (h : |x - r| ≤ uu * |r| + η) :
    |(x : ℝ) - r| ≤ (2 ^ (-53 : ℤ) : ℝ) * |(r : ℝ)| + 2 ^ (-1075 : ℤ) := by
  have := (Rat.cast_le (K := ℝ)).mpr h
  push_cast [uu, η] at this; exact this

/-- **`setback` refines `ρ tan(θ/2)`**, with dmath's sine and cosine. -/
theorem setback_spec (rho theta : IEEE754.F64) (hr : rho.isFinite) (hθ : theta.isFinite)
    (hb : |theta.val| ≤ 1048576) (hC : (2 : ℝ) ^ (-50 : ℤ) ≤ 1 + Real.cos theta.val) :
    fold.setback rho (Dmath.sinCos theta).1 (Dmath.sinCos theta).2 ⦃ r =>
      r.isFinite →
        let C := 1 + Real.cos theta.val
        |(r.val : ℝ) - rho.val * Real.sin theta.val / C| ≤
          |(rho.val : ℝ)| * (4 * (2 ^ (-52 : ℤ)) + 4 * 2 ^ (-1075 : ℤ)) / C ^ 2 +
          10 * 2 ^ (-53 : ℤ) * |(rho.val : ℝ)| / C + 3 * 2 ^ (-1075 : ℤ) / C + 2 ^ (-1075 : ℤ) ⦄ := by
  obtain ⟨fs, fc, es, ec⟩ := Dmath.sinCos_err hθ hb
  set s := (Dmath.sinCos theta).1
  set c := (Dmath.sinCos theta).2
  unfold fold.setback
  simp only [lift, Std.bind_ok, WP.spec_ok, F64.Ops.add_def, F64.Ops.mul_def, F64.Ops.div_def]
  intro hfin
  set C := 1 + Real.cos theta.val with hCdef
  have one_f := one_fin
  -- the denominator: finite (a small value), and at least C / 2
  have dfin : (FP.add (F64.Ops.ofBits 4607182418800017408) c).isFinite := by
    rw [add_eq_round one_f fc]; apply round_fin_q
    have : |c.val| ≤ 2 := by
      have hc' : |(c.val : ℝ)| ≤ 2 := by
        have := abs_sub_abs_le_abs_sub (c.val : ℝ) (Real.cos theta.val)
        have := Real.abs_cos_le_one theta.val
        have : (1 : ℝ) / 2 ^ 52 ≤ 1 := by norm_num
        linarith
      exact_mod_cast hc'
    rw [one_val]
    calc |1 + c.val| ≤ |(1 : ℚ)| + |c.val| := abs_add_le _ _
      _ ≤ 16 := by rw [abs_one]; linarith
  have hε : (2 : ℝ) ^ (-52 : ℤ) = 1 / 2 ^ 52 := by norm_num
  have U0 : (0 : ℝ) ≤ 2 ^ (-53 : ℤ) := by positivity
  have U8 : (2 : ℝ) ^ (-53 : ℤ) ≤ 1 / 8 := by norm_num
  have h50 : (2 : ℝ) ^ (-50 : ℤ) = 1 / 2 ^ 50 := by norm_num
  rw [h50] at hC
  have hεC : (2 : ℝ) ^ (-52 : ℤ) ≤ C / 4 := by
    rw [hε]; have : (1 : ℝ) / 2 ^ 52 * 4 = 1 / 2 ^ 50 := by norm_num
    linarith
  have hηC : (2 : ℝ) ^ (-1075 : ℤ) ≤ C / 8 := by
    have : (2 : ℝ) ^ (-1075 : ℤ) * 8 ≤ 1 / 2 ^ 50 := by norm_num
    linarith
  have ed := cast_err (add_err dfin)
  rw [one_val] at ed; push_cast at ed
  have ec' : |(c.val : ℝ) - Real.cos theta.val| ≤ 2 ^ (-52 : ℤ) := by rw [hε]; exact ec
  have es' : |(s.val : ℝ) - Real.sin theta.val| ≤ 2 ^ (-52 : ℤ) := by rw [hε]; exact es
  have hCp : 0 < C := lt_of_lt_of_le (by positivity) hC
  have dpos := ForgeSheetCore.Setback.den_pos U0 U8 hCp hεC hηC ec' ed
  have d0 : (FP.add (F64.Ops.ofBits 4607182418800017408) c).val ≠ 0 := by
    intro h0
    have : ((FP.add (F64.Ops.ofBits 4607182418800017408) c).val : ℝ) = 0 := by exact_mod_cast h0
    linarith
  -- the numerator: finite, since the quotient is
  have mfin : (FP.mul rho s).isFinite := by
    rcases IEEE754.trichotomy (FP.mul rho s) with h | ⟨sg, h⟩ | h
    · exact h
    · exfalso; rw [h] at hfin
      cases hd : FP.add (F64.Ops.ofBits 4607182418800017408) c <;>
        simp_all [FP.div, FP.isFinite]
    · exfalso; rw [h] at hfin; simp [FP.div, FP.isFinite] at hfin
  have em := cast_err (mul_err mfin)
  have er := cast_err (div_err mfin dfin d0 hfin)
  push_cast at em er
  exact ForgeSheetCore.Setback.setback_err U0 U8 (by positivity) hCp hεC hηC es' ec' em ed er

end forge_sheet_core
