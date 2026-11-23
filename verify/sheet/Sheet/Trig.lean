import Mathlib.Analysis.SpecialFunctions.Trigonometric.Basic
import Mathlib.Analysis.Complex.Exponential
import Mathlib.Analysis.Complex.Trigonometric
import Mathlib.Tactic

/-!
# Rational intervals, and proven enclosures of sin and cos at a rational point

The refold checker's arithmetic. Everything is exact over ℚ (a double is a dyadic rational); the
only transcendental steps are `sin` and `cos` of a rational, enclosed here:

* `QI`: a pair of rationals read as the real interval `[lo, hi]`; `add`, `sub`, `mul`, `neg`,
  `coarsen` (outward to a dyadic grid, to keep the numbers small), each with its containment lemma;
* `sinCos x k`: for `|x| ≤ 2^k`, halve `k` times, Taylor to degree `N` (the remainder by Mathlib's
  `Complex.exp_bound`), then double back with `sin 2a = 2 sin a cos a`, `cos 2a = 1 − 2 sin² a`;
  `sinCos_mem`: `sin x` and `cos x` lie in the two intervals.
-/

namespace Sheet.Trig

/-- A real interval with rational ends. -/
abbrev QI := ℚ × ℚ

def QI.mem (I : QI) (x : ℝ) : Prop := (I.1 : ℝ) ≤ x ∧ x ≤ I.2

def QI.pt (q : ℚ) : QI := (q, q)

def QI.add (I J : QI) : QI := (I.1 + J.1, I.2 + J.2)

def QI.neg (I : QI) : QI := (-I.2, -I.1)

def QI.sub (I J : QI) : QI := I.add J.neg

def min4 (a b c d : ℚ) : ℚ := min (min a b) (min c d)
def max4 (a b c d : ℚ) : ℚ := max (max a b) (max c d)

def QI.mul (I J : QI) : QI :=
  (min4 (I.1 * J.1) (I.1 * J.2) (I.2 * J.1) (I.2 * J.2),
    max4 (I.1 * J.1) (I.1 * J.2) (I.2 * J.1) (I.2 * J.2))

/-- Widen outward to multiples of `2^-p`. -/
def QI.coarsen (p : ℕ) (I : QI) : QI :=
  ((⌊I.1 * 2 ^ p⌋ : ℚ) / 2 ^ p, (⌈I.2 * 2 ^ p⌉ : ℚ) / 2 ^ p)

theorem mem_pt (q : ℚ) : (QI.pt q).mem q := ⟨le_rfl, le_rfl⟩

theorem mem_add {I J : QI} {x y : ℝ} (hx : I.mem x) (hy : J.mem y) : (I.add J).mem (x + y) := by
  simp only [QI.mem, QI.add, Rat.cast_add] at *; constructor <;> linarith [hx.1, hx.2, hy.1, hy.2]

theorem mem_neg {I : QI} {x : ℝ} (hx : I.mem x) : I.neg.mem (-x) := by
  simp only [QI.mem, QI.neg, Rat.cast_neg] at *; constructor <;> linarith [hx.1, hx.2]

theorem mem_sub {I J : QI} {x y : ℝ} (hx : I.mem x) (hy : J.mem y) : (I.sub J).mem (x - y) := by
  rw [sub_eq_add_neg]; exact mem_add hx (mem_neg hy)

theorem mul_between {a b x y : ℝ} (h1 : a ≤ x) (h2 : x ≤ b) :
    min (a * y) (b * y) ≤ x * y ∧ x * y ≤ max (a * y) (b * y) := by
  rcases le_total 0 y with hy | hy
  · exact ⟨le_trans (min_le_left _ _) (mul_le_mul_of_nonneg_right h1 hy),
      le_trans (mul_le_mul_of_nonneg_right h2 hy) (le_max_right _ _)⟩
  · exact ⟨le_trans (min_le_right _ _) (mul_le_mul_of_nonpos_right h2 hy),
      le_trans (mul_le_mul_of_nonpos_right h1 hy) (le_max_left _ _)⟩

theorem mul_between' {a b x y : ℝ} (h1 : a ≤ y) (h2 : y ≤ b) :
    min (x * a) (x * b) ≤ x * y ∧ x * y ≤ max (x * a) (x * b) := by
  have := mul_between (y := x) h1 h2
  simpa only [mul_comm _ x] using this

theorem mem_mul {I J : QI} {x y : ℝ} (hx : I.mem x) (hy : J.mem y) : (I.mul J).mem (x * y) := by
  obtain ⟨h1, h2⟩ := mul_between (y := y) hx.1 hx.2
  obtain ⟨a1, a2⟩ := mul_between' (x := (I.1 : ℝ)) hy.1 hy.2
  obtain ⟨b1, b2⟩ := mul_between' (x := (I.2 : ℝ)) hy.1 hy.2
  simp only [QI.mem, QI.mul, min4, max4, Rat.cast_min, Rat.cast_max, Rat.cast_mul]
  set A := (I.1 : ℝ) * J.1; set B := (I.1 : ℝ) * J.2
  set C := (I.2 : ℝ) * J.1; set D := (I.2 : ℝ) * J.2
  constructor
  · have l1 : min (min A B) (min C D) ≤ (I.1 : ℝ) * y := le_trans (min_le_left _ _) a1
    have l2 : min (min A B) (min C D) ≤ (I.2 : ℝ) * y := le_trans (min_le_right _ _) b1
    exact le_trans (le_min l1 l2) h1
  · have l1 : (I.1 : ℝ) * y ≤ max (max A B) (max C D) := le_trans a2 (le_max_left _ _)
    have l2 : (I.2 : ℝ) * y ≤ max (max A B) (max C D) := le_trans b2 (le_max_right _ _)
    exact le_trans h2 (max_le l1 l2)

theorem mem_coarsen (p : ℕ) {I : QI} {x : ℝ} (hx : I.mem x) : (I.coarsen p).mem x := by
  have h2 : (0 : ℚ) < 2 ^ p := by positivity
  have a : (I.coarsen p).1 ≤ I.1 := by
    simp only [QI.coarsen]; rw [div_le_iff₀ h2]; exact Int.floor_le _
  have b : I.2 ≤ (I.coarsen p).2 := by
    simp only [QI.coarsen]; rw [le_div_iff₀ h2]; exact Int.le_ceil _
  exact ⟨le_trans (by exact_mod_cast a) hx.1, le_trans hx.2 (by exact_mod_cast b)⟩

/-! ## Taylor at a small rational -/

/-- `Re (I^m)` and `Im (I^m)`. -/
def iRe (m : ℕ) : ℚ := match m % 4 with | 0 => 1 | 2 => -1 | _ => 0
def iIm (m : ℕ) : ℚ := match m % 4 with | 1 => 1 | 3 => -1 | _ => 0

theorem I_pow_eq (m : ℕ) : Complex.I ^ m = ((iRe m : ℚ) : ℂ) + ((iIm m : ℚ) : ℂ) * Complex.I := by
  have h : Complex.I ^ m = Complex.I ^ (m % 4) := by
    conv_lhs => rw [← Nat.div_add_mod m 4, pow_add, pow_mul, Complex.I_pow_four, one_pow, one_mul]
  rw [h]
  have hm : m % 4 < 4 := Nat.mod_lt _ (by norm_num)
  simp only [iRe, iIm]
  interval_cases hr : m % 4 <;> simp [pow_succ]

/-- The partial sums of `cos` and `sin`. -/
def cosPoly (N : ℕ) (x : ℚ) : ℚ := ∑ m ∈ Finset.range N, iRe m * x ^ m / m.factorial
def sinPoly (N : ℕ) (x : ℚ) : ℚ := ∑ m ∈ Finset.range N, iIm m * x ^ m / m.factorial

theorem expSum_eq (N : ℕ) (x : ℚ) :
    ∑ m ∈ Finset.range N, ((x : ℂ) * Complex.I) ^ m / (m.factorial : ℂ) =
      ((cosPoly N x : ℚ) : ℂ) + ((sinPoly N x : ℚ) : ℂ) * Complex.I := by
  simp only [cosPoly, sinPoly, Rat.cast_sum, Rat.cast_div, Rat.cast_mul, Rat.cast_pow,
    Rat.cast_natCast, Finset.sum_mul, ← Finset.sum_add_distrib]
  apply Finset.sum_congr rfl
  intro m _
  rw [mul_pow, I_pow_eq]
  field_simp

/-- The Taylor remainder bound, `|x|^N (N + 1) / (N! N)`. -/
def rem (N : ℕ) (a : ℚ) : ℚ := a ^ N * ((N + 1 : ℚ) / (N.factorial * N))

theorem taylor (N : ℕ) (hN : 0 < N) (x : ℚ) (hx : |x| ≤ 1) :
    |Real.cos x - cosPoly N x| ≤ rem N |x| ∧ |Real.sin x - sinPoly N x| ≤ rem N |x| := by
  have hn : ‖((x : ℝ) : ℂ) * Complex.I‖ ≤ 1 := by
    rw [norm_mul, Complex.norm_I, mul_one, Complex.norm_real, Real.norm_eq_abs]
    exact_mod_cast hx
  have hb := Complex.exp_bound hn hN
  rw [show (((x : ℝ) : ℂ) * Complex.I) = ((x : ℂ) * Complex.I) by push_cast; rfl, expSum_eq,
    show ((x : ℂ) * Complex.I) = (((x : ℝ) : ℂ) * Complex.I) by push_cast; rfl,
    ← Complex.cos_add_sin_I] at hb
  have hnorm : ‖((x : ℝ) : ℂ) * Complex.I‖ = |(x : ℝ)| := by
    rw [norm_mul, Complex.norm_I, mul_one, Complex.norm_real, Real.norm_eq_abs]
  rw [hnorm] at hb
  have hr : |(x : ℝ)| ^ N * (((N.succ : ℕ) : ℝ) * ((N.factorial : ℝ) * N)⁻¹) =
      ((rem N |x| : ℚ) : ℝ) := by
    simp only [rem]; push_cast; ring
  rw [hr, ← Complex.ofReal_cos, ← Complex.ofReal_sin] at hb
  set z := ((Real.cos x : ℝ) : ℂ) + ((Real.sin x : ℝ) : ℂ) * Complex.I -
    (((cosPoly N x : ℚ) : ℂ) + ((sinPoly N x : ℚ) : ℂ) * Complex.I) with hz
  have hre : z.re = Real.cos x - cosPoly N x := by
    rw [hz]; simp only [Complex.sub_re, Complex.add_re, Complex.mul_re, Complex.ofReal_re,
      Complex.ofReal_im, Complex.I_re, Complex.I_im, Complex.sub_im, Complex.add_im,
      Complex.mul_im, Complex.ratCast_re, Complex.ratCast_im]; ring
  have him : z.im = Real.sin x - sinPoly N x := by
    rw [hz]; simp only [Complex.sub_re, Complex.add_re, Complex.mul_re, Complex.ofReal_re,
      Complex.ofReal_im, Complex.I_re, Complex.I_im, Complex.sub_im, Complex.add_im,
      Complex.mul_im, Complex.ratCast_re, Complex.ratCast_im]; ring
  exact ⟨by rw [← hre]; exact le_trans (Complex.abs_re_le_norm z) hb,
    by rw [← him]; exact le_trans (Complex.abs_im_le_norm z) hb⟩

/-- The Taylor enclosure at a small rational. -/
def taylorI (N : ℕ) (x : ℚ) : QI × QI :=
  let r := rem N |x|
  ((sinPoly N x - r, sinPoly N x + r), (cosPoly N x - r, cosPoly N x + r))

theorem taylorI_mem (N : ℕ) (hN : 0 < N) (x : ℚ) (hx : |x| ≤ 1) :
    (taylorI N x).1.mem (Real.sin x) ∧ (taylorI N x).2.mem (Real.cos x) := by
  obtain ⟨hc, hs⟩ := taylor N hN x hx
  simp only [taylorI, QI.mem, Rat.cast_sub, Rat.cast_add]
  rw [abs_le] at hc hs
  exact ⟨⟨by linarith [hs.1], by linarith [hs.2]⟩, ⟨by linarith [hc.1], by linarith [hc.2]⟩⟩

/-! ## Halving and doubling -/

/-- From `(sin a, cos a)` to `(sin 2a, cos 2a)`, coarsened to `2^-p`. -/
def double (p : ℕ) (sc : QI × QI) : QI × QI :=
  (((QI.pt 2).mul (sc.1.mul sc.2)).coarsen p,
    ((QI.pt 1).sub ((QI.pt 2).mul (sc.1.mul sc.1))).coarsen p)

theorem double_mem (p : ℕ) {sc : QI × QI} {a : ℝ} (h : sc.1.mem (Real.sin a) ∧ sc.2.mem (Real.cos a)) :
    (double p sc).1.mem (Real.sin (2 * a)) ∧ (double p sc).2.mem (Real.cos (2 * a)) := by
  constructor
  · rw [Real.sin_two_mul, mul_assoc]
    have := mem_mul (mem_pt 2) (mem_mul h.1 h.2)
    push_cast at this
    exact mem_coarsen p this
  · rw [Real.cos_two_mul', show Real.cos a ^ 2 - Real.sin a ^ 2 = 1 - 2 * (Real.sin a * Real.sin a) by
      nlinarith [Real.sin_sq_add_cos_sq a]]
    have := mem_sub (mem_pt 1) (mem_mul (mem_pt 2) (mem_mul h.1 h.1))
    push_cast at this
    exact mem_coarsen p this

/-- `sin x` and `cos x` for `|x| ≤ 2^k`: halve `k` times, Taylor of degree `N`, double back,
keeping `p` bits. -/
def sinCos (N p : ℕ) : ℕ → ℚ → QI × QI
  | 0, x => taylorI N x
  | k + 1, x => double p (sinCos N p k (x / 2))

theorem sinCos_mem (N p : ℕ) (hN : 0 < N) :
    ∀ (k : ℕ) (x : ℚ), |x| ≤ 2 ^ k →
      (sinCos N p k x).1.mem (Real.sin x) ∧ (sinCos N p k x).2.mem (Real.cos x)
  | 0, x, hx => by simp only [sinCos]; exact taylorI_mem N hN x (by simpa using hx)
  | k + 1, x, hx => by
      have h := sinCos_mem N p hN k (x / 2) (by
        rw [abs_div, abs_two, div_le_iff₀ (by norm_num : (0 : ℚ) < 2)]
        rw [pow_succ] at hx; exact hx)
      have := double_mem p h
      simp only [sinCos]
      have e : (2 : ℝ) * ((x / 2 : ℚ) : ℝ) = x := by push_cast; ring
      rw [e] at this
      exact this

/-- The checker's choice: degree 24, 96 bits, `k` the least with `|x| ≤ 2^k` (up to 2^12). -/
def halvings (x : ℚ) : ℕ := (List.range 13).find? (fun k => |x| ≤ 2 ^ k) |>.getD 13

def sc (x : ℚ) : Option (QI × QI) :=
  if |x| ≤ 2 ^ halvings x then some (sinCos 24 96 (halvings x) x) else none

theorem sc_mem {x : ℚ} {r : QI × QI} (h : sc x = some r) :
    r.1.mem (Real.sin x) ∧ r.2.mem (Real.cos x) := by
  unfold sc at h
  split_ifs at h with hx
  cases h
  exact sinCos_mem 24 96 (by norm_num) _ x hx

end Sheet.Trig
