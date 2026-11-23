/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.KCos
import IEEE754.Dmath.Reduction

/-!
# `dmath::sin_cos`, `sin`, `cos`: the contract

`sinCos_err` (contract C1): for finite `|x| ≤ 2^20`, `dmath::sin` and `dmath::cos` are finite and
within `2^-52` of `sin x` and `cos x` (absolute). The budget: `rem_pio2`'s `2^-56`, the
polynomials' `2^-57`, the evaluations' `1.36e-16` / `1.63e-16`, and the tail `y` against the
addition formulas.

The model compares `|x| < 2^-27` on rationals: for finite operands IEEE comparison is exact.
-/

namespace IEEE754.Dmath

open FP

/-- `dmath::sin_cos`, branch for branch. `n & 3` on an `i32` is `n mod 4`. -/
def sinCos (x : F64) : F64 × F64 :=
  if x.isFinite then
    if |x.val| < 1 / 134217728 then (x, ONE)
    else
      let p := remPio2 x
      let s := ksin p.2.1 p.2.2
      let c := kcos p.2.1 p.2.2
      if p.1 % 4 = 0 then (s, c)
      else if p.1 % 4 = 1 then (c, s.neg)
      else if p.1 % 4 = 2 then (s.neg, c.neg)
      else (c.neg, s)
  else (.nan, .nan)

/-- `dmath::sin`. -/
def sin (x : F64) : F64 := (sinCos x).1
/-- `dmath::cos`. -/
def cos (x : F64) : F64 := (sinCos x).2

lemma pow_abs_le {a : ℝ} {M : ℝ} (h : |a| ≤ M) (k : ℕ) : |a| ^ k ≤ M ^ k :=
  pow_le_pow_left₀ (abs_nonneg a) h k

lemma qcast {q c : ℚ} (h : |q| ≤ c) : |(q : ℝ)| ≤ (c : ℝ) := by
  have := (Rat.cast_le (K := ℝ)).mpr h; rwa [Rat.cast_abs] at this

lemma qcast_lt {q c : ℚ} (h : |q| < c) : |(q : ℝ)| < (c : ℝ) := by
  have := (Rat.cast_lt (K := ℝ)).mpr h; rwa [Rat.cast_abs] at this

lemma mul_abs_le {b X B M : ℝ} (hb : |b| ≤ B) (hX : |X| ≤ M) (hB : 0 ≤ B) : |b * X| ≤ B * M := by
  rw [abs_mul]; exact mul_le_mul hb hX (abs_nonneg _) hB

/-- The tail `y1`: `sin(a+b) − sin a − b·cos a` and `cos(a+b) − cos a + b·sin a` are tiny. -/
lemma tail_terms {a b : ℝ} (hb : |b| ≤ 1 / 10 ^ 16) :
    |Real.sin a * (Real.cos b - 1)| ≤ 1 / 10 ^ 31 ∧ |Real.cos a * (Real.sin b - b)| ≤ 1 / 10 ^ 31 := by
  have hb1 : |b| ≤ 1 := le_trans hb (by norm_num)
  have c1 := Real.cos_bound hb1
  have s1 := Real.sin_bound hb1
  have p2 := pow_abs_le hb 2
  have p3 := pow_abs_le hb 3
  have p4 := pow_abs_le hb 4
  have p5 := pow_abs_le hb 5
  have e2 : b ^ 2 = |b| ^ 2 := (sq_abs b).symm
  have e3 : |b ^ 3| = |b| ^ 3 := abs_pow b 3
  have hcb : |Real.cos b - 1| ≤ 1 / 10 ^ 31 := by
    have := abs_le.mp c1; rw [abs_le]; constructor <;> nlinarith [sq_nonneg b]
  have hsb : |Real.sin b - b| ≤ 1 / 10 ^ 31 := by
    have := abs_le.mp s1; have h3 := abs_le.mp (le_of_eq e3); rw [abs_le]; constructor <;> nlinarith
  constructor
  · rw [abs_mul]; exact le_trans (mul_le_of_le_one_left (abs_nonneg _) (Real.abs_sin_le_one a)) hcb
  · rw [abs_mul]; exact le_trans (mul_le_of_le_one_left (abs_nonneg _) (Real.abs_cos_le_one a)) hsb

/-- `k_sin(y0, y1)` against `sin(y0 + y1)`. -/
theorem ksin_err {y0 y1 : F64} (h0 : y0.isFinite) (h1 : y1.isFinite) (hb0 : |y0.val| ≤ 78542 / 100000)
    (hb1 : |y1.val| ≤ 1 / 10 ^ 16) :
    (ksin y0 y1).isFinite ∧
      |((ksin y0 y1).val : ℝ) - Real.sin ((y0.val : ℝ) + y1.val)| ≤ 145 / 10 ^ 18 := by
  obtain ⟨f, e⟩ := ksin_eval h0 h1 hb0 hb1
  refine ⟨f, ?_⟩
  have hp := ksin_poly (x := y0.val) (le_trans hb0 (by norm_num))
  set A : ℝ := ((y0.val * evalP ksinL (y0.val ^ 2) : ℚ) : ℝ)
  set a : ℝ := (y0.val : ℝ)
  set b : ℝ := (y1.val : ℝ)
  have er : |((ksin y0 y1).val : ℝ) - (A + b * (1 - a ^ 2 / 2))| ≤ 17 / 125000000000000000 := by
    simp only [A, a, b]; have h := qcast e; push_cast at h ⊢; exact h
  have ha : |a| ≤ 78542 / 100000 := by simp only [a]; have h := qcast hb0; push_cast at h ⊢; exact h
  have hb : |b| ≤ 1 / 10 ^ 16 := by simp only [b]; have h := qcast hb1; push_cast at h ⊢; exact h
  have c0 := Real.cos_bound (le_trans ha (by norm_num))
  have p4 := pow_abs_le ha 4
  have t1 : |b * ((1 - a ^ 2 / 2) - Real.cos a)| ≤ 1 / 10 ^ 16 * ((78542 / 100000) ^ 4 * (5 / 96)) := by
    refine mul_abs_le hb ?_ (by norm_num)
    rw [abs_sub_comm]; exact le_trans c0 (mul_le_mul_of_nonneg_right p4 (by norm_num))
  obtain ⟨r1, r2⟩ := tail_terms (a := a) hb
  rw [Real.sin_add]
  have := abs_le.mp er; have := abs_le.mp hp; have := abs_le.mp t1; have := abs_le.mp r1
  have := abs_le.mp r2
  rw [abs_le]; constructor <;> nlinarith

/-- `k_cos(y0, y1)` against `cos(y0 + y1)`. -/
theorem kcos_err {y0 y1 : F64} (h0 : y0.isFinite) (h1 : y1.isFinite) (hb0 : |y0.val| ≤ 78542 / 100000)
    (hb1 : |y1.val| ≤ 1 / 10 ^ 16) :
    (kcos y0 y1).isFinite ∧
      |((kcos y0 y1).val : ℝ) - Real.cos ((y0.val : ℝ) + y1.val)| ≤ 179 / 10 ^ 18 := by
  obtain ⟨f, e⟩ := kcos_eval h0 h1 hb0 hb1
  refine ⟨f, ?_⟩
  have hp := kcos_poly (x := y0.val) (le_trans hb0 (by norm_num))
  set A : ℝ := ((evalP kcosL (y0.val ^ 2) : ℚ) : ℝ)
  set a : ℝ := (y0.val : ℝ)
  set b : ℝ := (y1.val : ℝ)
  have er : |((kcos y0 y1).val : ℝ) - (A - a * b)| ≤ 163 / 1000000000000000000 := by
    simp only [A, a, b]; have h := qcast e; push_cast at h ⊢; exact h
  have ha : |a| ≤ 78542 / 100000 := by simp only [a]; have h := qcast hb0; push_cast at h ⊢; exact h
  have hb : |b| ≤ 1 / 10 ^ 16 := by simp only [b]; have h := qcast hb1; push_cast at h ⊢; exact h
  have s0 := Real.sin_bound (le_trans ha (by norm_num))
  have p3 := pow_abs_le ha 3
  have p5 := pow_abs_le ha 5
  have e3 : |a ^ 3| = |a| ^ 3 := abs_pow a 3
  have hsa : |Real.sin a - a| ≤ (78542 / 100000) ^ 5 / 100 + (78542 / 100000) ^ 3 / 6 := by
    have h := abs_sub_le (Real.sin a) (a - a ^ 3 / 6) a
    rw [show a - a ^ 3 / 6 - a = -(a ^ 3 / 6) by ring, abs_neg, abs_div, e3, abs_of_pos (by norm_num : (0:ℝ) < 6)] at h
    linarith
  have t1 : |b * (Real.sin a - a)| ≤ 1 / 10 ^ 16 * ((78542 / 100000) ^ 5 / 100 + (78542 / 100000) ^ 3 / 6) :=
    mul_abs_le hb hsa (by norm_num)
  obtain ⟨r1, r2⟩ := tail_terms (a := a) hb
  have r1' : |Real.cos a * (Real.cos b - 1)| ≤ 1 / 10 ^ 31 := by
    have := tail_terms (a := a + Real.pi / 2) hb
    rw [Real.sin_add_pi_div_two] at this; exact this.1
  have r2' : |Real.sin a * (Real.sin b - b)| ≤ 1 / 10 ^ 31 := by
    have := tail_terms (a := a - Real.pi / 2) hb
    rw [Real.cos_sub_pi_div_two] at this; exact this.2
  rw [Real.cos_add]
  have := abs_le.mp er; have := abs_le.mp hp; have := abs_le.mp t1; have := abs_le.mp r1'
  have := abs_le.mp r2'
  rw [abs_le]; constructor <;> nlinarith

/-- `sin` and `cos` of `θ + n·π/2` depend on `n mod 4` only. -/
lemma rot (θ : ℝ) (n : ℤ) :
    Real.sin (θ + n * (Real.pi / 2)) = Real.sin (θ + ((n % 4 : ℤ) : ℝ) * (Real.pi / 2)) ∧
      Real.cos (θ + n * (Real.pi / 2)) = Real.cos (θ + ((n % 4 : ℤ) : ℝ) * (Real.pi / 2)) := by
  have h : (n : ℝ) * (Real.pi / 2) = ((n % 4 : ℤ) : ℝ) * (Real.pi / 2) + ((n / 4 : ℤ) : ℝ) * (2 * Real.pi) := by
    have h4 : n = n % 4 + 4 * (n / 4) := by omega
    have : (n : ℝ) = ((n % 4 : ℤ) : ℝ) + 4 * ((n / 4 : ℤ) : ℝ) := by exact_mod_cast h4
    rw [this]; ring
  rw [h, ← add_assoc, Real.sin_add_int_mul_two_pi, Real.cos_add_int_mul_two_pi]; exact ⟨rfl, rfl⟩

lemma neg_err {s : F64} {t E : ℝ} (h : |(s.val : ℝ) - t| ≤ E) : |((s.neg).val : ℝ) - -t| ≤ E := by
  rw [FP.val_neg]; push_cast; rw [show -(s.val : ℝ) - -t = -((s.val : ℝ) - t) by ring, abs_neg]; exact h

/-- **Contract C1.** For finite `|x| ≤ 2^20`, `dmath::sin_cos(x)` is finite and each part is within
`2^-52` of `sin x`, `cos x`. -/
theorem sinCos_err {x : F64} (hx : x.isFinite) (hb : |x.val| ≤ 1048576) :
    (sinCos x).1.isFinite ∧ (sinCos x).2.isFinite ∧
      |((sinCos x).1.val : ℝ) - Real.sin x.val| ≤ 1 / 2 ^ 52 ∧
      |((sinCos x).2.val : ℝ) - Real.cos x.val| ≤ 1 / 2 ^ 52 := by
  unfold sinCos
  rw [if_pos hx]
  by_cases hs : |x.val| < 1 / 134217728
  · rw [if_pos hs]
    refine ⟨hx, (by decide : ONE.isFinite = true), ?_, ?_⟩
    · have hr : |(x.val : ℝ)| < 1 / 134217728 := by have h := qcast_lt hs; push_cast at h ⊢; exact h
      have s1 := Real.sin_bound (le_trans hr.le (by norm_num))
      have p3 := pow_abs_le hr.le 3
      have p5 := pow_abs_le hr.le 5
      have e3 : |(x.val : ℝ) ^ 3| = |(x.val : ℝ)| ^ 3 := abs_pow _ 3
      have h := abs_sub_le (x.val : ℝ) ((x.val : ℝ) - (x.val : ℝ) ^ 3 / 6) (Real.sin x.val)
      rw [show (x.val : ℝ) - ((x.val : ℝ) - (x.val : ℝ) ^ 3 / 6) = (x.val : ℝ) ^ 3 / 6 by ring, abs_div, e3,
        abs_of_pos (by norm_num : (0:ℝ) < 6), abs_sub_comm ((x.val : ℝ) - _)] at h
      linarith
    · have hr : |(x.val : ℝ)| < 1 / 134217728 := by have h := qcast_lt hs; push_cast at h ⊢; exact h
      have c1 := Real.cos_bound (le_trans hr.le (by norm_num))
      have p2 := pow_abs_le hr.le 2
      have p4 := pow_abs_le hr.le 4
      rw [ONE_val]; push_cast
      have e2 : (x.val : ℝ) ^ 2 = |(x.val : ℝ)| ^ 2 := (sq_abs _).symm
      have := abs_le.mp c1; rw [abs_le]; constructor <;> nlinarith
  rw [if_neg hs]
  obtain ⟨f0, f1, hn, hθ, hy1, hu⟩ := remPio2_spec hx hb
  generalize remPio2 x = p at *
  obtain ⟨n, y0, y1⟩ := p
  simp only at f0 f1 hn hθ hy1 hu ⊢
  rw [show (2 : ℚ) ^ (-53 : ℤ) = 1 / 9007199254740992 by norm_num] at hy1
  have hy0 : |y0.val| ≤ 78542 / 100000 := by
    have := abs_sub_abs_le_abs_sub y0.val (y0.val + y1.val)
    rw [show y0.val - (y0.val + y1.val) = -y1.val by ring, abs_neg] at this
    linarith
  have hy1' : |y1.val| ≤ 1 / 10 ^ 16 := by linarith
  obtain ⟨fs, es⟩ := ksin_err f0 f1 hy0 hy1'
  obtain ⟨fc, ec⟩ := kcos_err f0 f1 hy0 hy1'
  set θ : ℝ := (x.val : ℝ) - n * (Real.pi / 2)
  have ls := Real.abs_sin_sub_sin_le ((y0.val : ℝ) + y1.val) θ
  have lc := Real.abs_cos_sub_cos_le ((y0.val : ℝ) + y1.val) θ
  have hS : |((ksin y0 y1).val : ℝ) - Real.sin θ| ≤ 1 / 2 ^ 52 := by
    have := abs_sub_le ((ksin y0 y1).val : ℝ) (Real.sin ((y0.val : ℝ) + y1.val)) (Real.sin θ)
    linarith
  have hC : |((kcos y0 y1).val : ℝ) - Real.cos θ| ≤ 1 / 2 ^ 52 := by
    have := abs_sub_le ((kcos y0 y1).val : ℝ) (Real.cos ((y0.val : ℝ) + y1.val)) (Real.cos θ)
    linarith
  have hxθ : (x.val : ℝ) = θ + n * (Real.pi / 2) := by simp only [θ]; ring
  obtain ⟨rs, rc⟩ := rot θ n
  rw [hxθ, rs, rc]
  have fsn : (ksin y0 y1).neg.isFinite := by simpa using fs
  have fcn : (kcos y0 y1).neg.isFinite := by simpa using fc
  have h4 : 0 ≤ n % 4 ∧ n % 4 < 4 := ⟨Int.emod_nonneg _ (by norm_num), Int.emod_lt_of_pos _ (by norm_num)⟩
  split_ifs with k0 k1 k2
  · rw [k0]; push_cast; rw [zero_mul, _root_.add_zero]; exact ⟨fs, fc, hS, hC⟩
  · rw [k1]; push_cast; rw [_root_.one_mul, Real.sin_add_pi_div_two, Real.cos_add_pi_div_two]
    exact ⟨fc, fsn, hC, neg_err hS⟩
  · rw [k2]; push_cast; rw [show θ + 2 * (Real.pi / 2) = θ + Real.pi by ring, Real.sin_add_pi, Real.cos_add_pi]
    exact ⟨fsn, fcn, neg_err hS, neg_err hC⟩
  · have k3 : n % 4 = 3 := by omega
    rw [k3]; push_cast
    rw [show θ + 3 * (Real.pi / 2) = (θ + Real.pi) + Real.pi / 2 by ring, Real.sin_add_pi_div_two,
      Real.cos_add_pi_div_two, Real.cos_add_pi, Real.sin_add_pi, _root_.neg_neg]
    exact ⟨fcn, fs, neg_err hC, hS⟩

/-- `dmath::sin`, contract C1. -/
theorem sin_err {x : F64} (hx : x.isFinite) (hb : |x.val| ≤ 1048576) :
    (sin x).isFinite ∧ |((sin x).val : ℝ) - Real.sin x.val| ≤ 1 / 2 ^ 52 :=
  ⟨(sinCos_err hx hb).1, (sinCos_err hx hb).2.2.1⟩

/-- `dmath::cos`, contract C1. -/
theorem cos_err {x : F64} (hx : x.isFinite) (hb : |x.val| ≤ 1048576) :
    (cos x).isFinite ∧ |((cos x).val : ℝ) - Real.cos x.val| ≤ 1 / 2 ^ 52 :=
  ⟨(sinCos_err hx hb).2.1, (sinCos_err hx hb).2.2.2⟩

end IEEE754.Dmath
