/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.SinPoly
import Mathlib.Analysis.SpecialFunctions.Trigonometric.ArctanDeriv
import Mathlib.Analysis.Calculus.MeanValue

/-!
# `atan`'s polynomial against `arctan`

`katan_poly`: on `|t| ≤ 0.4375`, the polynomial `dmath::atan` evaluates (in exact arithmetic),
`t − t·(s1 + s2)`, is within `2^-57 + 10^-19` of `arctan t`. The proof: `arctan` against its Taylor
polynomial of degree 53 by the mean value theorem (`atan_taylor`: the derivative of the remainder is
`(−s²)^N/(1+s²)`), and that polynomial against `atan`'s by a kernel-checked certificate on 30
intervals of `z = t²`.
-/

namespace IEEE754.Dmath
/-- `arctan` against its Taylor polynomial: the remainder is at most `|t|^(2N+1)`. -/
lemma atan_taylor (N : ℕ) (t : ℝ) :
    |Real.arctan t - ∑ k ∈ Finset.range N, (-1) ^ k * t ^ (2 * k + 1) / (2 * k + 1)| ≤ |t| ^ (2 * N + 1) := by
  set g : ℝ → ℝ := fun s => Real.arctan s - ∑ k ∈ Finset.range N, (-1) ^ k * s ^ (2 * k + 1) / (2 * k + 1)
  have hd : ∀ s : ℝ, HasDerivAt g ((-s ^ 2) ^ N / (1 + s ^ 2)) s := by
    intro s
    have h1 := Real.hasDerivAt_arctan s
    have h2 : HasDerivAt (fun s : ℝ => ∑ k ∈ Finset.range N, (-1 : ℝ) ^ k * s ^ (2 * k + 1) / (2 * (k : ℝ) + 1))
        (∑ k ∈ Finset.range N, (-s ^ 2 : ℝ) ^ k) s := by
      apply HasDerivAt.fun_sum
      intro k _
      have := ((hasDerivAt_pow (2 * k + 1) s).const_mul ((-1 : ℝ) ^ k)).div_const (2 * k + 1 : ℝ)
      convert this using 1
      have hk : (2 * (k : ℝ) + 1) ≠ 0 := by positivity
      rw [neg_pow, ← pow_mul, Nat.add_sub_cancel]; push_cast; field_simp
    convert h1.sub h2 using 1
    have hq : (-s ^ 2 : ℝ) ≠ 1 := by nlinarith [sq_nonneg s]
    have hp : (1 + s ^ 2 : ℝ) ≠ 0 := by positivity
    rw [geom_sum_eq hq]
    have hm : (-s ^ 2 - 1 : ℝ) ≠ 0 := by nlinarith [sq_nonneg s]
    field_simp; ring
  have hb : ∀ s ∈ Set.uIcc 0 t, ‖(-s ^ 2) ^ N / (1 + s ^ 2)‖ ≤ |t| ^ (2 * N) := by
    intro s hs
    have hst : |s| ≤ |t| := by
      rcases Set.mem_uIcc.mp hs with ⟨a, b⟩ | ⟨a, b⟩
      · rw [abs_of_nonneg a, abs_of_nonneg (le_trans a b)]; exact b
      · rw [abs_of_nonpos b, abs_of_nonpos (le_trans a b)]; linarith
    rw [Real.norm_eq_abs, abs_div, abs_pow, abs_neg, abs_of_pos (by positivity : (0 : ℝ) < 1 + s ^ 2)]
    rw [div_le_iff₀ (by positivity), abs_pow, ← pow_mul]
    have : |s| ^ (2 * N) ≤ |t| ^ (2 * N) := pow_le_pow_left₀ (abs_nonneg s) hst _
    nlinarith [pow_nonneg (abs_nonneg t) (2 * N), sq_nonneg s]
  have := Convex.norm_image_sub_le_of_norm_hasDerivWithin_le (fun s _ => (hd s).hasDerivWithinAt) hb
    (convex_uIcc 0 t) Set.left_mem_uIcc Set.right_mem_uIcc
  have g0 : g 0 = 0 := by simp [g]
  rw [g0, sub_zero, sub_zero, Real.norm_eq_abs, Real.norm_eq_abs] at this
  rw [pow_succ]; exact this

/-- `atan`'s polynomial, in `z = t²`: `P(t) = t · katanL(t²)`, with `katanL = 1 − Σ AT[k] z^(k+1)`. -/
def katanL : List ℚ := [(1 / 1 : ℚ), (-6004799503160589 / 18014398509481984 : ℚ), (1801439850937073 / 9007199254740992 : ℚ), (-5146970997949439 / 36028797018963968 : ℚ), (8006398829074033 / 72057594037927936 : ℚ), (-3275337272528951 / 36028797018963968 : ℚ), (5542580929731181 / 72057594037927936 : ℚ), (-4799809039908177 / 72057594037927936 : ℚ), (4203530284924621 / 72057594037927936 : ℚ), (-7172437082246635 / 144115188075855872 : ℚ), (5264754476739631 / 144115188075855872 : ℚ), (-4694068057790993 / 288230376151711744 : ℚ)]

/-- `arctan`'s Taylor polynomial of degree 53, in `z = t²`. -/
def tatanL : List ℚ := [(1 / 1 : ℚ), (-1 / 3 : ℚ), (1 / 5 : ℚ), (-1 / 7 : ℚ), (1 / 9 : ℚ), (-1 / 11 : ℚ), (1 / 13 : ℚ), (-1 / 15 : ℚ), (1 / 17 : ℚ), (-1 / 19 : ℚ), (1 / 21 : ℚ), (-1 / 23 : ℚ), (1 / 25 : ℚ), (-1 / 27 : ℚ), (1 / 29 : ℚ), (-1 / 31 : ℚ), (1 / 33 : ℚ), (-1 / 35 : ℚ), (1 / 37 : ℚ), (-1 / 39 : ℚ), (1 / 41 : ℚ), (-1 / 43 : ℚ), (1 / 45 : ℚ), (-1 / 47 : ℚ), (1 / 49 : ℚ), (-1 / 51 : ℚ), (1 / 53 : ℚ)]

/-- `katanL − tatanL`. -/
def qatan : List ℚ := [(0 / 1 : ℚ), (217 / 54043195528445952 : ℚ), (-55627 / 45035996273704960 : ℚ), (33317895 / 252201579132747776 : ℚ), (-4576261639 / 648518346341351424 : ℚ), (87021145507 / 396316767208603648 : ℚ), (-4041951422583 / 936748722493063168 : ℚ), (60458439305281 / 1080863910568919040 : ℚ), (-597579194209379 / 1224979098644774912 : ℚ), (7838883513169807 / 2738188573441261568 : ℚ), (-33555344064323621 / 3026418949592973312 : ℚ), (180266810822518905 / 6629298651489370112 : ℚ), (-1 / 25 : ℚ), (1 / 27 : ℚ), (-1 / 29 : ℚ), (1 / 31 : ℚ), (-1 / 33 : ℚ), (1 / 35 : ℚ), (-1 / 37 : ℚ), (1 / 39 : ℚ), (-1 / 41 : ℚ), (1 / 43 : ℚ), (-1 / 45 : ℚ), (1 / 47 : ℚ), (-1 / 49 : ℚ), (1 / 51 : ℚ), (-1 / 53 : ℚ)]

def zMax : ℚ := (4375 / 10000) ^ 2

def certAtan (n : ℕ) (B : ℚ) : Bool := (List.range n).all fun k =>
  let t0 := zMax * k / n
  let t1 := zMax * (k + 1) / n
  decide (evalP ((shiftP ((t0 + t1) / 2) qatan).map (fun a => |a|)) ((t1 - t0) / 2) ≤ B)

theorem certAtan_ok : certAtan 30 (1 / 2 ^ 56) = true := by decide +kernel

lemma qatan_le {x : ℚ} (hx : |x| ≤ 4375 / 10000) : |evalP qatan (x ^ 2)| ≤ 1 / 2 ^ 56 := by
  have hcert := certAtan_ok
  unfold certAtan at hcert
  rw [List.all_eq_true] at hcert
  set t := x ^ 2 with htdef
  have ht0 : 0 ≤ t := sq_nonneg x
  have htM : t ≤ zMax := by
    unfold zMax; rw [htdef, ← sq_abs]; exact pow_le_pow_left₀ (abs_nonneg x) hx 2
  have hM : (0 : ℚ) < zMax := by unfold zMax; norm_num
  set k : ℕ := min (⌊t * 30 / zMax⌋₊) 29 with hk
  have hkn : k < 30 := by omega
  have hc := hcert k (List.mem_range.mpr hkn)
  simp only [decide_eq_true_eq] at hc
  have hkle : (k : ℚ) ≤ t * 30 / zMax := by
    have : (k : ℚ) ≤ ⌊t * 30 / zMax⌋₊ := by exact_mod_cast min_le_left _ _
    exact le_trans this (Nat.floor_le (by positivity))
  have hkge : t * 30 / zMax ≤ k + 1 := by
    rcases lt_or_ge (⌊t * 30 / zMax⌋₊) 29 with h | h
    · have : k = ⌊t * 30 / zMax⌋₊ := by omega
      rw [this]; exact (Nat.lt_floor_add_one _).le
    · have : k = 29 := by omega
      rw [this]
      have : t * 30 / zMax ≤ 30 := by rw [div_le_iff₀ hM]; nlinarith
      push_cast; linarith
  have ht0' : zMax * k / 30 ≤ t := by
    rw [le_div_iff₀ hM] at hkle; rw [div_le_iff₀ (by norm_num)]; nlinarith
  have ht1' : t ≤ zMax * (k + 1) / 30 := by
    rw [div_le_iff₀ hM] at hkge; rw [le_div_iff₀ (by norm_num)]; nlinarith
  set t0 := zMax * k / 30
  set t1 := zMax * (k + 1) / 30
  have hh : 0 ≤ (t1 - t0) / 2 := by linarith
  have hu : |t - (t0 + t1) / 2| ≤ (t1 - t0) / 2 := abs_le.mpr ⟨by linarith, by linarith⟩
  have hq := evalP_abs_le hh (shiftP ((t0 + t1) / 2) qatan) hu
  rw [evalP_shiftP, show (t0 + t1) / 2 + (t - (t0 + t1) / 2) = t by ring] at hq
  exact le_trans hq hc

lemma tatan_sum (x : ℚ) : x * evalP tatanL (x ^ 2) =
    ∑ k ∈ Finset.range 27, (-1) ^ k * x ^ (2 * k + 1) / (2 * k + 1) := by
  simp only [evalP, tatanL, Finset.sum_range_succ, Finset.sum_range_zero]
  push_cast; ring

lemma katan_tatan (x : ℚ) : x * evalP katanL (x ^ 2) - x * evalP tatanL (x ^ 2) = x * evalP qatan (x ^ 2) := by
  simp only [evalP, katanL, tatanL, qatan]; ring

/-- **`atan`'s polynomial.** On `|t| ≤ 0.4375`, `|t·katanL(t²) − arctan t| ≤ 2^-57 + 10^-19`. -/
theorem katan_poly {x : ℚ} (hx : |x| ≤ 4375 / 10000) :
    |((x * evalP katanL (x ^ 2) : ℚ) : ℝ) - Real.arctan x| ≤ 1 / 2 ^ 57 + 1 / 10 ^ 19 := by
  have h1 : |x * evalP qatan (x ^ 2)| ≤ 1 / 2 ^ 57 := by
    rw [abs_mul]
    calc |x| * |evalP qatan (x ^ 2)| ≤ 4375 / 10000 * (1 / 2 ^ 56) :=
          mul_le_mul hx (qatan_le hx) (abs_nonneg _) (by norm_num)
      _ ≤ 1 / 2 ^ 57 := by norm_num
  have h1r : |((x * evalP qatan (x ^ 2) : ℚ) : ℝ)| ≤ 1 / 2 ^ 57 := by
    have h := (Rat.cast_le (K := ℝ)).mpr h1; push_cast at h ⊢; exact h
  have h2 := atan_taylor 27 (x : ℝ)
  have hT : ((x * evalP tatanL (x ^ 2) : ℚ) : ℝ) =
      ∑ k ∈ Finset.range 27, (-1) ^ k * (x : ℝ) ^ (2 * k + 1) / (2 * k + 1) := by
    rw [tatan_sum]; push_cast; rfl
  have h3 : |(x : ℝ)| ^ (2 * 27 + 1) ≤ 1 / 10 ^ 19 := by
    have : |(x : ℝ)| ≤ 4375 / 10000 := by
      have h := (Rat.cast_le (K := ℝ)).mpr hx; push_cast at h; exact h
    exact le_trans (pow_le_pow_left₀ (abs_nonneg _) this _) (by norm_num)
  have e := katan_tatan x
  have er : ((x * evalP katanL (x ^ 2) : ℚ) : ℝ) = ((x * evalP tatanL (x ^ 2) : ℚ) : ℝ) +
      ((x * evalP qatan (x ^ 2) : ℚ) : ℝ) := by rw [← e]; push_cast; ring
  rw [er, hT]
  have := abs_le.mp h1r; have := abs_le.mp h2
  rw [abs_le]; constructor <;> linarith

end IEEE754.Dmath
