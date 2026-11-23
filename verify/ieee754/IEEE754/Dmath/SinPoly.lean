/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.Consts
import Mathlib.Analysis.SpecialFunctions.Trigonometric.Bounds

/-!
# `k_sin`'s polynomial against `sin`

`ksin_poly`: on `|x| ≤ 0.7855`, the polynomial `k_sin` evaluates (with `y = 0`, in exact
arithmetic) is within `2^-57 + 10^-27` of `sin x`. The proof: `sin` against its Taylor polynomial
of degree 23 (`Complex.exp_bound`), and the Taylor polynomial against `k_sin`'s, by a certificate
the kernel checks (`decide +kernel`, rationals only): on each of 60 intervals of `t = x²`, the
difference, expanded about the interval's centre, is bounded term by term.
-/

namespace IEEE754.Dmath

/-! ## Polynomials as coefficient lists -/

/-- `Σ c_i t^i`, Horner. -/
def evalP : List ℚ → ℚ → ℚ
  | [], _ => 0
  | a :: c, t => a + t * evalP c t

def addP : List ℚ → List ℚ → List ℚ
  | [], d => d
  | c, [] => c
  | a :: c, b :: d => (a + b) :: addP c d

/-- The coefficients of `p(tc + u)`. -/
def shiftP (tc : ℚ) : List ℚ → List ℚ
  | [] => []
  | a :: c => addP [a] (addP ((shiftP tc c).map (tc * ·)) (0 :: shiftP tc c))

lemma evalP_addP : ∀ (c d : List ℚ) (t : ℚ), evalP (addP c d) t = evalP c t + evalP d t
  | [], d, t => by simp [addP, evalP]
  | a :: c, [], t => by simp [addP, evalP]
  | a :: c, b :: d, t => by simp only [addP, evalP, evalP_addP c d t]; ring

lemma evalP_map_mul (k : ℚ) : ∀ (c : List ℚ) (t : ℚ), evalP (c.map (k * ·)) t = k * evalP c t
  | [], t => by simp [evalP]
  | a :: c, t => by simp only [List.map_cons, evalP, evalP_map_mul k c t]; ring

lemma evalP_shiftP (tc : ℚ) : ∀ (c : List ℚ) (u : ℚ), evalP (shiftP tc c) u = evalP c (tc + u)
  | [], u => by simp [shiftP, evalP]
  | a :: c, u => by
    simp only [shiftP, evalP_addP, evalP_map_mul, evalP, evalP_shiftP tc c u]; ring

/-- On `|u| ≤ h`, `|p(u)| ≤ Σ |c_i| h^i`. -/
lemma evalP_abs_le {h : ℚ} (hh : 0 ≤ h) : ∀ (c : List ℚ) {u : ℚ}, |u| ≤ h →
    |evalP c u| ≤ evalP (c.map (fun a => |a|)) h
  | [], u, _ => by simp [evalP]
  | a :: c, u, hu => by
    simp only [List.map_cons, evalP]
    calc |a + u * evalP c u| ≤ |a| + |u| * |evalP c u| := by rw [← abs_mul]; exact abs_add_le _ _
      _ ≤ |a| + h * evalP (c.map (fun a => |a|)) h :=
          add_le_add_right (mul_le_mul hu (evalP_abs_le hh c hu) (abs_nonneg _) hh) _

/-! ## The certificate -/

/-- `k_sin`'s polynomial, in `t = x²`: `P(x) = x · pL(x²)`. -/
def ksinL : List ℚ := [(1 / 1 : ℚ), (-6004799503160649 / 36028797018963968 : ℚ), (2401919801261139 / 288230376151711744 : ℚ), (-7320136532976085 / 36893488147419103232 : ℚ), (6506786730409597 / 2361183241434822606848 : ℚ), (-7571127717829867 / 302231454903657293676544 : ℚ), (1537454791456095 / 9671406556917033397649408 : ℚ)]

/-- `sin`'s Taylor polynomial of degree 23, in `t = x²`. -/
def tsinL : List ℚ := [(1 / 1 : ℚ), (-1 / 6 : ℚ), (1 / 120 : ℚ), (-1 / 5040 : ℚ), (1 / 362880 : ℚ), (-1 / 39916800 : ℚ), (1 / 6227020800 : ℚ), (-1 / 1307674368000 : ℚ), (1 / 355687428096000 : ℚ), (-1 / 121645100408832000 : ℚ), (1 / 51090942171709440000 : ℚ), (-1 / 25852016738884976640000 : ℚ)]

/-- `(P − T)(x) = x³ · qsin(x²)`. -/
def qsin : List ℚ := [(37 / 108086391056891904 : ℚ), (-46883 / 4323455642275676160 : ℚ), (1326227177 / 11621448766437017518080 : ℚ), (-3692998344121 / 6693954489467722090414080 : ℚ), (63531314789291449 / 47125439605852763516515123200 : ℚ), (-95355069785417525267 / 58812548628104248868610873753600 : ℚ), (1 / 1307674368000 : ℚ), (-1 / 355687428096000 : ℚ), (1 / 121645100408832000 : ℚ), (-1 / 51090942171709440000 : ℚ), (1 / 25852016738884976640000 : ℚ)]

def tMax : ℚ := (7855 / 10000) ^ 2

/-- On each of `n` intervals of `[0, tMax]`, the centred expansion of `qsin`, times the right
end (a bound on `|x|³ ≤ x² = t`), is at most `B`. -/
def certSin (n : ℕ) (B : ℚ) : Bool := (List.range n).all fun k =>
  let t0 := tMax * k / n
  let t1 := tMax * (k + 1) / n
  decide (evalP ((shiftP ((t0 + t1) / 2) qsin).map (fun a => |a|)) ((t1 - t0) / 2) * t1 ≤ B)

theorem certSin_ok : certSin 60 (1 / 2 ^ 57) = true := by decide +kernel

/-- The certificate, at a point. -/
lemma qsin_le {x : ℚ} (hx : |x| ≤ 7855 / 10000) : |x ^ 3 * evalP qsin (x ^ 2)| ≤ 1 / 2 ^ 57 := by
  have hcert := certSin_ok
  unfold certSin at hcert
  rw [List.all_eq_true] at hcert
  set t := x ^ 2 with htdef
  have ht0 : 0 ≤ t := sq_nonneg x
  have htM : t ≤ tMax := by
    unfold tMax; rw [htdef, ← sq_abs]; exact pow_le_pow_left₀ (abs_nonneg x) hx 2
  have hM : (0 : ℚ) < tMax := by unfold tMax; norm_num
  -- the interval of `t`
  set k : ℕ := min (⌊t * 60 / tMax⌋₊) 59 with hk
  have hk59 : k < 60 := by omega
  have hc := hcert k (List.mem_range.mpr hk59)
  simp only [decide_eq_true_eq] at hc
  have hkle : (k : ℚ) ≤ t * 60 / tMax := by
    have : (k : ℚ) ≤ ⌊t * 60 / tMax⌋₊ := by exact_mod_cast min_le_left _ _
    exact le_trans this (Nat.floor_le (by positivity))
  have hkge : t * 60 / tMax ≤ k + 1 := by
    rcases lt_or_ge (⌊t * 60 / tMax⌋₊) 59 with h | h
    · have : k = ⌊t * 60 / tMax⌋₊ := by omega
      rw [this]; exact (Nat.lt_floor_add_one _).le
    · have : k = 59 := by omega
      rw [this]
      have : t * 60 / tMax ≤ 60 := by rw [div_le_iff₀ hM]; nlinarith
      push_cast; linarith
  have ht0' : tMax * k / 60 ≤ t := by
    rw [le_div_iff₀ hM] at hkle; rw [div_le_iff₀ (by norm_num)]; nlinarith
  have ht1' : t ≤ tMax * (k + 1) / 60 := by
    rw [div_le_iff₀ hM] at hkge; rw [le_div_iff₀ (by norm_num)]; nlinarith
  set t0 := tMax * k / 60
  set t1 := tMax * (k + 1) / 60
  have hh : 0 ≤ (t1 - t0) / 2 := by linarith
  have hu : |t - (t0 + t1) / 2| ≤ (t1 - t0) / 2 := abs_le.mpr ⟨by linarith, by linarith⟩
  have hq := evalP_abs_le hh (shiftP ((t0 + t1) / 2) qsin) hu
  rw [evalP_shiftP, show (t0 + t1) / 2 + (t - (t0 + t1) / 2) = t by ring] at hq
  -- `|x|³ ≤ x² = t ≤ t1`
  have hx1 : |x| ≤ 1 := le_trans hx (by norm_num)
  have hx3 : |x ^ 3| ≤ t1 := by
    rw [abs_pow]
    calc |x| ^ 3 ≤ |x| ^ 2 := pow_le_pow_of_le_one (abs_nonneg x) hx1 (by norm_num)
      _ = t := by rw [htdef, sq_abs]
      _ ≤ t1 := ht1'
  rw [abs_mul]
  calc |x ^ 3| * |evalP qsin t| ≤ t1 * evalP ((shiftP ((t0 + t1) / 2) qsin).map (fun a => |a|)) ((t1 - t0) / 2) :=
        mul_le_mul hx3 hq (abs_nonneg _) (by linarith [abs_nonneg x])
    _ = evalP ((shiftP ((t0 + t1) / 2) qsin).map (fun a => |a|)) ((t1 - t0) / 2) * t1 := mul_comm _ _
    _ ≤ 1 / 2 ^ 57 := hc

/-! ## `sin` against its Taylor polynomial -/

open Complex in
lemma xI_pow_odd (x : ℝ) (N : ℕ) : ((x : ℂ) * I) ^ (2 * N + 1) = (((-1) ^ N * x ^ (2 * N + 1) : ℝ) : ℂ) * I := by
  have h2 : ((x : ℂ) * I) ^ 2 = -((x : ℂ) ^ 2) := by rw [mul_pow, I_sq]; ring
  rw [pow_succ, pow_mul, h2]; push_cast; ring

open Complex in
lemma xI_pow_even (x : ℝ) (N : ℕ) : ((x : ℂ) * I) ^ (2 * N + 2) = (((-1) ^ (N + 1) * x ^ (2 * N + 2) : ℝ) : ℂ) := by
  have h2 : ((x : ℂ) * I) ^ 2 = -((x : ℂ) ^ 2) := by rw [mul_pow, I_sq]; ring
  rw [show 2 * N + 2 = 2 * (N + 1) by ring, pow_mul, h2]; push_cast; ring

open Complex in
lemma im_partial (x : ℝ) : ∀ N : ℕ, (∑ m ∈ Finset.range (2 * N + 1), ((x : ℂ) * I) ^ m / (m.factorial : ℂ)).im =
    ∑ i ∈ Finset.range N, (-1) ^ i * x ^ (2 * i + 1) / ((2 * i + 1).factorial : ℝ)
  | 0 => by simp
  | N + 1 => by
    rw [show 2 * (N + 1) + 1 = (2 * N + 1) + 1 + 1 by ring]
    conv_lhs => rw [Finset.sum_range_succ, Finset.sum_range_succ]
    conv_rhs => rw [Finset.sum_range_succ]
    rw [Complex.add_im, Complex.add_im, im_partial x N, xI_pow_odd,
      show 2 * N + 1 + 1 = 2 * N + 2 by ring, xI_pow_even]
    have e1 : ((((-1) ^ N * x ^ (2 * N + 1) : ℝ) : ℂ) * I / ((2 * N + 1).factorial : ℂ)).im =
        (-1) ^ N * x ^ (2 * N + 1) / ((2 * N + 1).factorial : ℝ) := by
      rw [show ((2 * N + 1).factorial : ℂ) = (((2 * N + 1).factorial : ℝ) : ℂ) by push_cast; rfl,
        Complex.div_ofReal_im, Complex.mul_I_im, Complex.ofReal_re]
    have e2 : ((((-1) ^ (N + 1) * x ^ (2 * N + 2) : ℝ) : ℂ) / ((2 * N + 2).factorial : ℂ)).im = 0 := by
      rw [show ((2 * N + 2).factorial : ℂ) = (((2 * N + 2).factorial : ℝ) : ℂ) by push_cast; rfl,
        Complex.div_ofReal_im, Complex.ofReal_im, zero_div]
    rw [e1, e2]; ring

/-- `|sin x − T₂₃(x)| ≤ |x|^25 · 26 / (25!·25)` on `|x| ≤ 1`. -/
lemma sin_taylor {x : ℝ} (hx : |x| ≤ 1) :
    |Real.sin x - ∑ i ∈ Finset.range 12, (-1) ^ i * x ^ (2 * i + 1) / ((2 * i + 1).factorial : ℝ)| ≤
      |x| ^ 25 * (26 * ((25).factorial * 25 : ℝ)⁻¹) := by
  have hn : ‖(x : ℂ) * Complex.I‖ = |x| := by simp
  have hb := Complex.exp_bound (x := (x : ℂ) * Complex.I) (by rw [hn]; exact hx) (n := 25) (by norm_num)
  rw [hn] at hb
  have him : (Complex.exp ((x : ℂ) * Complex.I) -
      ∑ m ∈ Finset.range 25, ((x : ℂ) * Complex.I) ^ m / (m.factorial : ℂ)).im =
      Real.sin x - ∑ i ∈ Finset.range 12, (-1) ^ i * x ^ (2 * i + 1) / ((2 * i + 1).factorial : ℝ) := by
    rw [Complex.sub_im, Complex.exp_ofReal_mul_I_im, show (25 : ℕ) = 2 * 12 + 1 from rfl, im_partial]
  rw [← him]
  refine le_trans (Complex.abs_im_le_norm _) ?_
  simpa using hb

lemma tsin_sum (x : ℚ) : x * evalP tsinL (x ^ 2) =
    ∑ i ∈ Finset.range 12, (-1) ^ i * x ^ (2 * i + 1) / ((2 * i + 1).factorial : ℚ) := by
  simp only [evalP, tsinL, Finset.sum_range_succ, Finset.sum_range_zero, Nat.factorial]
  push_cast; ring

lemma ksin_tsin (x : ℚ) : x * evalP ksinL (x ^ 2) - x * evalP tsinL (x ^ 2) = x ^ 3 * evalP qsin (x ^ 2) := by
  simp only [evalP, ksinL, tsinL, qsin]; ring

/-- **`k_sin`'s polynomial.** On `|x| ≤ 0.7855`, `|x·ksinL(x²) − sin x| ≤ 2^-57 + 10^-27`. -/
theorem ksin_poly {x : ℚ} (hx : |x| ≤ 7855 / 10000) :
    |((x * evalP ksinL (x ^ 2) : ℚ) : ℝ) - Real.sin x| ≤ 1 / 2 ^ 57 + 1 / 10 ^ 27 := by
  have h1 := qsin_le hx
  have h1r : |((x ^ 3 * evalP qsin (x ^ 2) : ℚ) : ℝ)| ≤ 1 / 2 ^ 57 := by
    have h := (Rat.cast_le (K := ℝ)).mpr h1; push_cast at h ⊢; exact h
  have hxr : |(x : ℝ)| ≤ 1 := by
    have h := (Rat.cast_le (K := ℝ)).mpr (le_trans hx (by norm_num : (7855 / 10000 : ℚ) ≤ 1)); push_cast at h; exact h
  have h2 := sin_taylor hxr
  have hT : ((x * evalP tsinL (x ^ 2) : ℚ) : ℝ) =
      ∑ i ∈ Finset.range 12, (-1) ^ i * (x : ℝ) ^ (2 * i + 1) / ((2 * i + 1).factorial : ℝ) := by
    rw [tsin_sum]; push_cast; rfl
  have h3 : |(x : ℝ)| ^ 25 * (26 * ((25).factorial * 25 : ℝ)⁻¹) ≤ 1 / 10 ^ 27 := by
    have : |(x : ℝ)| ≤ 7855 / 10000 := by
      have h := (Rat.cast_le (K := ℝ)).mpr hx; push_cast at h; exact h
    have hp : |(x : ℝ)| ^ 25 ≤ (7855 / 10000) ^ 25 := pow_le_pow_left₀ (abs_nonneg _) this 25
    calc |(x : ℝ)| ^ 25 * (26 * ((25).factorial * 25 : ℝ)⁻¹) ≤ (7855 / 10000) ^ 25 * (26 * ((25).factorial * 25 : ℝ)⁻¹) :=
          mul_le_mul_of_nonneg_right hp (by positivity)
      _ ≤ 1 / 10 ^ 27 := by norm_num [Nat.factorial]
  have key : ((x * evalP ksinL (x ^ 2) : ℚ) : ℝ) - Real.sin x =
      ((x ^ 3 * evalP qsin (x ^ 2) : ℚ) : ℝ) - (Real.sin x - ((x * evalP tsinL (x ^ 2) : ℚ) : ℝ)) := by
    rw [← ksin_tsin]; push_cast; ring
  rw [key]
  have := abs_sub ((x ^ 3 * evalP qsin (x ^ 2) : ℚ) : ℝ) (Real.sin x - ((x * evalP tsinL (x ^ 2) : ℚ) : ℝ))
  rw [hT] at this ⊢
  linarith

end IEEE754.Dmath
