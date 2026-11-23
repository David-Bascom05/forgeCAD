/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.SinPoly

/-!
# `k_cos`'s polynomial against `cos`

`kcos_poly`: on `|x| ≤ 0.7855`, `|kcosL(x²) − cos x| ≤ 2^-57 + 10^-29`: `cos` against its Taylor
polynomial of degree 24 (`Complex.exp_bound`, real part), and a kernel-checked certificate on 30
intervals of `t = x²` for the difference `t² · qcos(t)`.
-/

namespace IEEE754.Dmath

/-- `k_cos`'s polynomial in `t = x²`: `1 − t/2 + C1 t² + … + C6 t⁷`. -/
def kcosL : List ℚ := [(1 / 1 : ℚ), (-1 / 2 : ℚ), (1501199875790163 / 36028797018963968 : ℚ), (-6405119470031223 / 4611686018427387904 : ℚ), (457508533350745 / 18446744073709551616 : ℚ), (-5205429506036397 / 18889465931478580854784 : ℚ), (1261860039765105 / 604462909807314587353088 : ℚ), (-1757820317994549 / 154742504910672534362390528 : ℚ)]

/-- `cos`'s Taylor polynomial of degree 24, in `t = x²`. -/
def tcosL : List ℚ := [(1 / 1 : ℚ), (-1 / 2 : ℚ), (1 / 24 : ℚ), (-1 / 720 : ℚ), (1 / 40320 : ℚ), (-1 / 3628800 : ℚ), (1 / 479001600 : ℚ), (-1 / 87178291200 : ℚ), (1 / 20922789888000 : ℚ), (-1 / 6402373705728000 : ℚ), (1 / 2432902008176640000 : ℚ), (-1 / 1124000727777607680000 : ℚ), (1 / 620448401733239439360000 : ℚ)]

/-- `(P − T)(x) = t² · qcos(t)`. -/
def qcos : List ℚ := [(-7 / 108086391056891904 : ℚ), (306709 / 207525870829232455680 : ℚ), (-70371197 / 5810724383218508759040 : ℚ), (13046772278989 / 267758179578708883616563200 : ℚ), (-29230257583660337 / 282752637635116581099090739200 : ℚ), (731803394173408997911 / 6587005446347675873284417860403200 : ℚ), (-1 / 20922789888000 : ℚ), (1 / 6402373705728000 : ℚ), (-1 / 2432902008176640000 : ℚ), (1 / 1124000727777607680000 : ℚ), (-1 / 620448401733239439360000 : ℚ)]

def certCos (n : ℕ) (B : ℚ) : Bool := (List.range n).all fun k =>
  let t0 := tMax * k / n
  let t1 := tMax * (k + 1) / n
  decide (evalP ((shiftP ((t0 + t1) / 2) qcos).map (fun a => |a|)) ((t1 - t0) / 2) * (t1 * t1) ≤ B)

theorem certCos_ok : certCos 30 (1 / 2 ^ 57) = true := by decide +kernel

lemma qcos_le {x : ℚ} (hx : |x| ≤ 7855 / 10000) : |(x ^ 2) ^ 2 * evalP qcos (x ^ 2)| ≤ 1 / 2 ^ 57 := by
  have hcert := certCos_ok
  unfold certCos at hcert
  rw [List.all_eq_true] at hcert
  set t := x ^ 2 with htdef
  have ht0 : 0 ≤ t := sq_nonneg x
  have htM : t ≤ tMax := by
    unfold tMax; rw [htdef, ← sq_abs]; exact pow_le_pow_left₀ (abs_nonneg x) hx 2
  have hM : (0 : ℚ) < tMax := by unfold tMax; norm_num
  set k : ℕ := min (⌊t * 30 / tMax⌋₊) 29 with hk
  have hk29 : k < 30 := by omega
  have hc := hcert k (List.mem_range.mpr hk29)
  simp only [decide_eq_true_eq] at hc
  have hkle : (k : ℚ) ≤ t * 30 / tMax := by
    have : (k : ℚ) ≤ ⌊t * 30 / tMax⌋₊ := by exact_mod_cast min_le_left _ _
    exact le_trans this (Nat.floor_le (by positivity))
  have hkge : t * 30 / tMax ≤ k + 1 := by
    rcases lt_or_ge (⌊t * 30 / tMax⌋₊) 29 with h | h
    · have : k = ⌊t * 30 / tMax⌋₊ := by omega
      rw [this]; exact (Nat.lt_floor_add_one _).le
    · have : k = 29 := by omega
      rw [this]
      have : t * 30 / tMax ≤ 30 := by rw [div_le_iff₀ hM]; nlinarith
      push_cast; linarith
  have ht0' : tMax * k / 30 ≤ t := by
    rw [le_div_iff₀ hM] at hkle; rw [div_le_iff₀ (by norm_num)]; nlinarith
  have ht1' : t ≤ tMax * (k + 1) / 30 := by
    rw [div_le_iff₀ hM] at hkge; rw [le_div_iff₀ (by norm_num)]; nlinarith
  set t0 := tMax * k / 30
  set t1 := tMax * (k + 1) / 30
  have hh : 0 ≤ (t1 - t0) / 2 := by linarith
  have hu : |t - (t0 + t1) / 2| ≤ (t1 - t0) / 2 := abs_le.mpr ⟨by linarith, by linarith⟩
  have hq := evalP_abs_le hh (shiftP ((t0 + t1) / 2) qcos) hu
  rw [evalP_shiftP, show (t0 + t1) / 2 + (t - (t0 + t1) / 2) = t by ring] at hq
  have ht2 : |t ^ 2| ≤ t1 * t1 := by
    rw [abs_of_nonneg (sq_nonneg t), sq]; exact mul_le_mul ht1' ht1' ht0 (by linarith)
  rw [abs_mul]
  calc |t ^ 2| * |evalP qcos t| ≤ (t1 * t1) * evalP ((shiftP ((t0 + t1) / 2) qcos).map (fun a => |a|)) ((t1 - t0) / 2) :=
        mul_le_mul ht2 hq (abs_nonneg _) (by nlinarith)
    _ = evalP ((shiftP ((t0 + t1) / 2) qcos).map (fun a => |a|)) ((t1 - t0) / 2) * (t1 * t1) := mul_comm _ _
    _ ≤ 1 / 2 ^ 57 := hc

open Complex in
lemma re_partial (x : ℝ) : ∀ N : ℕ, (∑ m ∈ Finset.range (2 * N + 2), ((x : ℂ) * I) ^ m / (m.factorial : ℂ)).re =
    ∑ i ∈ Finset.range (N + 1), (-1) ^ i * x ^ (2 * i) / ((2 * i).factorial : ℝ)
  | 0 => by simp [Finset.sum_range_succ]
  | N + 1 => by
    rw [show 2 * (N + 1) + 2 = (2 * N + 2) + 1 + 1 by ring]
    conv_lhs => rw [Finset.sum_range_succ, Finset.sum_range_succ]
    conv_rhs => rw [Finset.sum_range_succ]
    rw [Complex.add_re, Complex.add_re, re_partial x N, xI_pow_even,
      show 2 * N + 2 + 1 = 2 * (N + 1) + 1 by ring, xI_pow_odd]
    have e1 : ((((-1) ^ (N + 1) * x ^ (2 * N + 2) : ℝ) : ℂ) / ((2 * N + 2).factorial : ℂ)).re =
        (-1) ^ (N + 1) * x ^ (2 * (N + 1)) / ((2 * (N + 1)).factorial : ℝ) := by
      rw [show ((2 * N + 2).factorial : ℂ) = (((2 * N + 2).factorial : ℝ) : ℂ) by push_cast; rfl,
        Complex.div_ofReal_re, Complex.ofReal_re, show 2 * (N + 1) = 2 * N + 2 by ring]
    have e2 : ((((-1) ^ (N + 1) * x ^ (2 * (N + 1) + 1) : ℝ) : ℂ) * I / ((2 * (N + 1) + 1).factorial : ℂ)).re = 0 := by
      rw [show ((2 * (N + 1) + 1).factorial : ℂ) = (((2 * (N + 1) + 1).factorial : ℝ) : ℂ) by push_cast; rfl,
        Complex.div_ofReal_re, Complex.mul_I_re, Complex.ofReal_im, neg_zero, zero_div]
    rw [e1, e2]; ring

lemma cos_taylor {x : ℝ} (hx : |x| ≤ 1) :
    |Real.cos x - ∑ i ∈ Finset.range 13, (-1) ^ i * x ^ (2 * i) / ((2 * i).factorial : ℝ)| ≤
      |x| ^ 26 * (27 * ((26).factorial * 26 : ℝ)⁻¹) := by
  have hn : ‖(x : ℂ) * Complex.I‖ = |x| := by simp
  have hb := Complex.exp_bound (x := (x : ℂ) * Complex.I) (by rw [hn]; exact hx) (n := 26) (by norm_num)
  rw [hn] at hb
  have hre : (Complex.exp ((x : ℂ) * Complex.I) -
      ∑ m ∈ Finset.range 26, ((x : ℂ) * Complex.I) ^ m / (m.factorial : ℂ)).re =
      Real.cos x - ∑ i ∈ Finset.range 13, (-1) ^ i * x ^ (2 * i) / ((2 * i).factorial : ℝ) := by
    rw [Complex.sub_re, Complex.exp_ofReal_mul_I_re, show (26 : ℕ) = 2 * 12 + 2 from rfl, re_partial]
  rw [← hre]
  refine le_trans (Complex.abs_re_le_norm _) ?_
  simpa using hb

lemma tcos_sum (x : ℚ) : evalP tcosL (x ^ 2) =
    ∑ i ∈ Finset.range 13, (-1) ^ i * x ^ (2 * i) / ((2 * i).factorial : ℚ) := by
  simp only [evalP, tcosL, Finset.sum_range_succ, Finset.sum_range_zero, Nat.factorial]
  push_cast; ring

lemma kcos_tcos (x : ℚ) : evalP kcosL (x ^ 2) - evalP tcosL (x ^ 2) = (x ^ 2) ^ 2 * evalP qcos (x ^ 2) := by
  simp only [evalP, kcosL, tcosL, qcos]; ring

/-- **`k_cos`'s polynomial.** On `|x| ≤ 0.7855`, `|kcosL(x²) − cos x| ≤ 2^-57 + 10^-29`. -/
theorem kcos_poly {x : ℚ} (hx : |x| ≤ 7855 / 10000) :
    |((evalP kcosL (x ^ 2) : ℚ) : ℝ) - Real.cos x| ≤ 1 / 2 ^ 57 + 1 / 10 ^ 29 := by
  have h1 := qcos_le hx
  have h1r : |(((x ^ 2) ^ 2 * evalP qcos (x ^ 2) : ℚ) : ℝ)| ≤ 1 / 2 ^ 57 := by
    have h := (Rat.cast_le (K := ℝ)).mpr h1; push_cast at h ⊢; exact h
  have hxr : |(x : ℝ)| ≤ 1 := by
    have h := (Rat.cast_le (K := ℝ)).mpr (le_trans hx (by norm_num : (7855 / 10000 : ℚ) ≤ 1)); push_cast at h; exact h
  have h2 := cos_taylor hxr
  have hT : ((evalP tcosL (x ^ 2) : ℚ) : ℝ) =
      ∑ i ∈ Finset.range 13, (-1) ^ i * (x : ℝ) ^ (2 * i) / ((2 * i).factorial : ℝ) := by
    rw [tcos_sum]; push_cast; rfl
  have h3 : |(x : ℝ)| ^ 26 * (27 * ((26).factorial * 26 : ℝ)⁻¹) ≤ 1 / 10 ^ 29 := by
    have : |(x : ℝ)| ≤ 7855 / 10000 := by
      have h := (Rat.cast_le (K := ℝ)).mpr hx; push_cast at h; exact h
    have hp : |(x : ℝ)| ^ 26 ≤ (7855 / 10000) ^ 26 := pow_le_pow_left₀ (abs_nonneg _) this 26
    calc |(x : ℝ)| ^ 26 * (27 * ((26).factorial * 26 : ℝ)⁻¹) ≤ (7855 / 10000) ^ 26 * (27 * ((26).factorial * 26 : ℝ)⁻¹) :=
          mul_le_mul_of_nonneg_right hp (by positivity)
      _ ≤ 1 / 10 ^ 29 := by norm_num [Nat.factorial]
  have key : ((evalP kcosL (x ^ 2) : ℚ) : ℝ) - Real.cos x =
      (((x ^ 2) ^ 2 * evalP qcos (x ^ 2) : ℚ) : ℝ) - (Real.cos x - ((evalP tcosL (x ^ 2) : ℚ) : ℝ)) := by
    rw [← kcos_tcos]; push_cast; ring
  rw [key]
  have := abs_sub (((x ^ 2) ^ 2 * evalP qcos (x ^ 2) : ℚ) : ℝ) (Real.cos x - ((evalP tcosL (x ^ 2) : ℚ) : ℝ))
  rw [hT] at this ⊢
  linarith

end IEEE754.Dmath
