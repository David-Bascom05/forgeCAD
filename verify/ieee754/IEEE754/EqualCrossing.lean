/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Theorems
import IEEE754.RoundProps
import IEEE754.TwoProduct
import IEEE754.Sqrt
import IEEE754.Conic

/-!
# ssi `equal_crossing`'s ellipse: the divisor `|x × a|` is not too small (stage 1: one operation)

kernel/forge-bool/src/ssi.rs `equal_crossing` makes `major = r / |x × a|` with
`x = normalize(a ± b)`, reached only where the f64 `|a × b| > ANGULAR = 1e-11`. This file bounds
the f64 `|x × a|` from below (so `div_pos_finite` gives a finite, positive `major`). Stage 1: the
error of one rounded operation, `|op(x, y) - exact| <= u |exact| + eta`, `u = 2^-prec`,
`eta = 2^(qmin-1)`, for a result at most `2^emax`.
-/

namespace IEEE754

variable (f : Format)

/-- One rounding: finite, and off by at most `u |r| + eta`, for `|r| <= 2^emax`. -/
theorem round_err_le {zs : Bool} {r : ℚ} (hr : |r| ≤ 2 ^ f.emax) :
    (round f zs r).isFinite ∧
      |(round f zs r).val - r| ≤ 2 ^ (-(f.prec : ℤ)) * |r| + 2 ^ (f.qmin - 1) := by
  have hfin : (round f zs r).isFinite := round_finite_of_le_emax hr
  refine ⟨hfin, ?_⟩
  have h := round_err (K := ℚ) f hfin
  have h1 : (0 : ℚ) ≤ 2 ^ (-(f.prec : ℤ)) * |r| := mul_nonneg (two_zpow_pos _).le (abs_nonneg _)
  have h2 : (0 : ℚ) ≤ 2 ^ (f.qmin - 1) := (two_zpow_pos _).le
  exact le_trans h (max_le (by linarith) (by linarith))

theorem add_err_le {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hr : |x.val + y.val| ≤ 2 ^ f.emax) :
    (FP.add x y).isFinite ∧ |(FP.add x y).val - (x.val + y.val)| ≤
      2 ^ (-(f.prec : ℤ)) * |x.val + y.val| + 2 ^ (f.qmin - 1) := by
  rw [add_eq_round hx hy]; exact round_err_le f hr

theorem sub_err_le {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hr : |x.val - y.val| ≤ 2 ^ f.emax) :
    (FP.sub x y).isFinite ∧ |(FP.sub x y).val - (x.val - y.val)| ≤
      2 ^ (-(f.prec : ℤ)) * |x.val - y.val| + 2 ^ (f.qmin - 1) := by
  rw [sub_eq_round hx hy]; exact round_err_le f hr

theorem mul_err_le {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hr : |x.val * y.val| ≤ 2 ^ f.emax) :
    (FP.mul x y).isFinite ∧ |(FP.mul x y).val - x.val * y.val| ≤
      2 ^ (-(f.prec : ℤ)) * |x.val * y.val| + 2 ^ (f.qmin - 1) := by
  rw [mul_eq_round hx hy]; exact round_err_le f hr

theorem div_err_le {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hy0 : y.val ≠ 0)
    (hr : |x.val / y.val| ≤ 2 ^ f.emax) :
    (FP.div x y).isFinite ∧ |(FP.div x y).val - x.val / y.val| ≤
      2 ^ (-(f.prec : ℤ)) * |x.val / y.val| + 2 ^ (f.qmin - 1) := by
  rw [div_eq_round hx hy hy0]; exact round_err_le f hr

/-- A rounding of a non-negative value within `u |r| + eta`, as two one-sided bounds. -/
theorem two_sided {v r u η : ℚ} (hr : 0 ≤ r) (h : |v - r| ≤ u * |r| + η) :
    r - u * r - η ≤ v ∧ v ≤ r + u * r + η := by
  rw [abs_of_nonneg hr] at h
  constructor <;> linarith [abs_le.mp h]

/-- **`sq3_err`**: the f64 `x*x + y*y + z*z` (Rust's `length_sq`, left to right) is within
`4u S + 6 eta` of `S = x² + y² + z²`, for `4 S + 8 <= 2^emax` and `u <= 1/8`. -/
theorem sq3_err {x y z : FP f} (hx : x.isFinite) (hy : y.isFinite) (hz : z.isFinite)
    (hu : (2 : ℚ) ^ (-(f.prec : ℤ)) ≤ 1 / 8) (hη : (2 : ℚ) ^ (f.qmin - 1) ≤ 1)
    (hS : 4 * (x.val * x.val + y.val * y.val + z.val * z.val) + 8 ≤ 2 ^ f.emax) :
    (FP.add (FP.add (FP.mul x x) (FP.mul y y)) (FP.mul z z)).isFinite ∧
      |(FP.add (FP.add (FP.mul x x) (FP.mul y y)) (FP.mul z z)).val -
          (x.val * x.val + y.val * y.val + z.val * z.val)| ≤
        4 * 2 ^ (-(f.prec : ℤ)) * (x.val * x.val + y.val * y.val + z.val * z.val) +
          6 * 2 ^ (f.qmin - 1) := by
  have sx := mul_self_nonneg x.val; have sy := mul_self_nonneg y.val
  have sz := mul_self_nonneg z.val
  have hS0 : 0 ≤ x.val * x.val + y.val * y.val + z.val * z.val := by linarith
  have bnd : ∀ t : ℚ, 0 ≤ t →
      t ≤ 2 * (x.val * x.val + y.val * y.val + z.val * z.val) + 6 → |t| ≤ 2 ^ f.emax :=
    fun t ht h => by rw [abs_of_nonneg ht]; linarith
  obtain ⟨f1, e1⟩ := mul_err_le f hx hx (bnd _ sx (by linarith))
  obtain ⟨f2, e2⟩ := mul_err_le f hy hy (bnd _ sy (by linarith))
  obtain ⟨f3, e3⟩ := mul_err_le f hz hz (bnd _ sz (by linarith))
  have p1n : 0 ≤ (FP.mul x x).val := by rw [mul_eq_round hx hx]; exact round_val_nonneg f sx
  have p2n : 0 ≤ (FP.mul y y).val := by rw [mul_eq_round hy hy]; exact round_val_nonneg f sy
  have p3n : 0 ≤ (FP.mul z z).val := by rw [mul_eq_round hz hz]; exact round_val_nonneg f sz
  have u0 : (0 : ℚ) ≤ 2 ^ (-(f.prec : ℤ)) := (two_zpow_pos _).le
  have η0 : (0 : ℚ) ≤ 2 ^ (f.qmin - 1) := (two_zpow_pos _).le
  generalize hS_def : x.val * x.val + y.val * y.val + z.val * z.val = S at *
  generalize hu_eq : (2 : ℚ) ^ (-(f.prec : ℤ)) = u at *
  generalize hη_eq : (2 : ℚ) ^ (f.qmin - 1) = η at *
  obtain ⟨l1, r1⟩ := two_sided sx e1
  obtain ⟨l2, r2⟩ := two_sided sy e2
  obtain ⟨l3, r3⟩ := two_sided sz e3
  -- A = p1 + p2 + p3 is within u S + 3 eta of S
  have uS_split : u * S = u * (x.val * x.val) + u * (y.val * y.val) + u * (z.val * z.val) := by
    rw [← hS_def]; ring
  have hAhi : (FP.mul x x).val + (FP.mul y y).val + (FP.mul z z).val ≤ S + u * S + 3 * η := by linarith
  have uS : u * S ≤ S / 8 := by
    have := mul_le_mul_of_nonneg_right hu (by rw [← hS_def]; linarith : (0:ℚ) ≤ S); linarith
  have uη : u * η ≤ η / 8 := by
    have := mul_le_mul_of_nonneg_right hu η0; linarith
  have hA : (FP.mul x x).val + (FP.mul y y).val + (FP.mul z z).val ≤ 9 / 8 * S + 3 * η := by linarith
  -- the first sum
  have hp12 : (FP.mul x x).val + (FP.mul y y).val ≤ 2 * S + 6 := by linarith
  obtain ⟨fs, es⟩ := add_err_le f f1 f2 (bnd _ (by linarith) hp12)
  rw [hu_eq, hη_eq] at es
  have sn : 0 ≤ (FP.add (FP.mul x x) (FP.mul y y)).val := by
    rw [add_eq_round f1 f2]; exact round_val_nonneg f (by linarith)
  obtain ⟨ls, rs⟩ := two_sided (by linarith) es
  have u12 : u * ((FP.mul x x).val + (FP.mul y y).val) ≤ u * (9 / 8 * S + 3 * η) :=
    mul_le_mul_of_nonneg_left (by linarith) u0
  have hsp_lin : (FP.add (FP.mul x x) (FP.mul y y)).val + (FP.mul z z).val ≤ 81 / 64 * S + 35 / 8 * η := by
    have := mul_le_mul_of_nonneg_left uS (by norm_num : (0:ℚ) ≤ 9 / 8)
    linarith
  -- the second
  have hsp : (FP.add (FP.mul x x) (FP.mul y y)).val + (FP.mul z z).val ≤ 2 * S + 6 := by linarith
  obtain ⟨fq, eq⟩ := add_err_le f fs f3 (bnd _ (by linarith) hsp)
  rw [hu_eq, hη_eq] at eq
  obtain ⟨lq, rq⟩ := two_sided (by linarith) eq
  have u23 : u * ((FP.add (FP.mul x x) (FP.mul y y)).val + (FP.mul z z).val) ≤ u * (81 / 64 * S + 35 / 8 * η) :=
    mul_le_mul_of_nonneg_left hsp_lin u0
  have hgoal : |(FP.add (FP.add (FP.mul x x) (FP.mul y y)) (FP.mul z z)).val - S| ≤
      4 * u * S + 6 * η := by
    have k1 : u * (9 / 8 * S + 3 * η) = 9 / 8 * (u * S) + 3 * (u * η) := by ring
    have k2 : u * (81 / 64 * S + 35 / 8 * η) = 81 / 64 * (u * S) + 35 / 8 * (u * η) := by ring
    have k3 : 4 * u * S = 4 * (u * S) := by ring
    have uS0 : 0 ≤ u * S := mul_nonneg u0 (by rw [← hS_def]; linarith)
    have uη0 : 0 ≤ u * η := mul_nonneg u0 η0
    rw [k3]
    apply abs_le.mpr
    constructor <;> linarith
  exact ⟨fq, hgoal⟩

/-- **`diff_prod_err`**: a cross product's component, the f64 `p*q - r*s`, is within `5u M + 4 eta` of
the exact `pq - rs` when `|pq|, |rs| <= M`, `4 M + 8 <= 2^emax`, `u <= 1/8`, `eta <= 1`. -/
theorem diff_prod_err {p q r s : FP f} (hp : p.isFinite) (hq : q.isFinite) (hr : r.isFinite)
    (hs : s.isFinite) {M : ℚ} (h1 : |p.val * q.val| ≤ M) (h2 : |r.val * s.val| ≤ M)
    (hM : 4 * M + 8 ≤ 2 ^ f.emax)
    (hu : (2 : ℚ) ^ (-(f.prec : ℤ)) ≤ 1 / 8) (hη : (2 : ℚ) ^ (f.qmin - 1) ≤ 1) :
    (FP.sub (FP.mul p q) (FP.mul r s)).isFinite ∧
      |(FP.sub (FP.mul p q) (FP.mul r s)).val - (p.val * q.val - r.val * s.val)| ≤
        5 * 2 ^ (-(f.prec : ℤ)) * M + 4 * 2 ^ (f.qmin - 1) := by
  have M0 : 0 ≤ M := le_trans (abs_nonneg _) h1
  obtain ⟨f1, e1⟩ := mul_err_le f hp hq (by linarith)
  obtain ⟨f2, e2⟩ := mul_err_le f hr hs (by linarith)
  have u0 : (0 : ℚ) ≤ 2 ^ (-(f.prec : ℤ)) := (two_zpow_pos _).le
  have η0 : (0 : ℚ) ≤ 2 ^ (f.qmin - 1) := (two_zpow_pos _).le
  generalize hu_eq : (2 : ℚ) ^ (-(f.prec : ℤ)) = u at *
  generalize hη_eq : (2 : ℚ) ^ (f.qmin - 1) = η at *
  have a1 : u * |p.val * q.val| ≤ u * M := mul_le_mul_of_nonneg_left h1 u0
  have a2 : u * |r.val * s.val| ≤ u * M := mul_le_mul_of_nonneg_left h2 u0
  have uM : u * M ≤ M / 8 := by have := mul_le_mul_of_nonneg_right hu M0; linarith
  have uη : u * η ≤ η / 8 := by have := mul_le_mul_of_nonneg_right hu η0; linarith
  -- the two products, each within u M + eta, so their difference is at most 2 M + 2 u M + 2 eta
  have b1 := abs_le.mp e1; have b2 := abs_le.mp e2
  have c1 := abs_le.mp h1; have c2 := abs_le.mp h2
  have hd : |(FP.mul p q).val - (FP.mul r s).val| ≤ 2 * M + 2 * (u * M) + 2 * η := by
    rw [abs_le]; constructor <;> linarith
  obtain ⟨fd, ed⟩ := sub_err_le f f1 f2 (by linarith)
  rw [hu_eq, hη_eq] at ed
  have a3 : u * |(FP.mul p q).val - (FP.mul r s).val| ≤ u * (2 * M + 2 * (u * M) + 2 * η) :=
    mul_le_mul_of_nonneg_left hd u0
  have k : u * (2 * M + 2 * (u * M) + 2 * η) = 2 * (u * M) + 2 * (u * (u * M)) + 2 * (u * η) := by
    ring
  have uuM : u * (u * M) ≤ u * (M / 8) := mul_le_mul_of_nonneg_left uM u0
  have k2 : u * (M / 8) = (u * M) / 8 := by ring
  have k3 : 5 * u * M = 5 * (u * M) := by ring
  have uM0 : 0 ≤ u * M := mul_nonneg u0 M0
  have uη0 : 0 ≤ u * η := mul_nonneg u0 η0
  refine ⟨fd, ?_⟩
  rw [k3]
  have b3 := abs_le.mp ed
  rw [abs_le]; constructor <;> linarith

section Sqrt

variable {K : Type*} [Field K] [LinearOrder K] [IsStrictOrderedRing K] [FloorRing K]

/-- `round_finite_of_le_emax` (TwoProduct.lean) over any field: a value at most `2^emax` rounds
to a finite datum. -/
lemma round_finite_of_le_emax_K {zs : Bool} {r : K} (h : |r| ≤ 2 ^ f.emax) :
    (round f zs r).isFinite := by
  by_cases hr : r = 0
  · subst hr; simp [round, FP.isFinite]
  rw [round_finite_iff (K := K) f hr]
  have hpow : (2 : K) ^ f.emax = ((2 ^ (f.prec - 1) : ℕ) : K) * 2 ^ f.qmax := by
    push_cast
    rw [← zpow_natCast, ← two_zpow_add, f.prec_sub_one]; congr 1; unfold Format.qmax; ring
  have hex : uround f ((2 : K) ^ f.emax) = 2 ^ f.emax := by
    rw [hpow, uround_exact f (by positivity) f.qmin_le_qmax
      (Nat.pow_le_pow_right (by norm_num) (by omega))]
  calc uround f |r| ≤ uround f (2 ^ f.emax) := uround_mono f (abs_pos.mpr hr) h
    _ = 2 ^ f.emax := hex
    _ < 2 ^ (f.emax + 1) := two_zpow_lt (by omega)

end Sqrt

lemma one_le_two_pow_int (n : ℕ) : (1 : ℤ) ≤ 2 ^ n := one_le_pow₀ (by norm_num)

/-- `2^k` rounds to itself (`qmin <= k <= emax`), over ℝ. -/
lemma round_two_pow_real (zs : Bool) {k : ℤ} (hk : f.qmin ≤ k) (hk2 : k ≤ f.emax) :
    (round f zs ((2 : ℝ) ^ k)).isFinite ∧ ((round f zs ((2 : ℝ) ^ k)).val : ℝ) = 2 ^ k := by
  have h := round_exact (K := ℝ) f zs (k := 1) (e := k)
    (by simpa using (one_le_two_pow_int f.prec)) hk
    (by simpa using two_zpow_lt (K := ℝ) (by omega : k < f.emax + 1))
  simpa using h

/-- **`sqrt_ge_two_pow`**: the f64 square root of a finite `x >= 2^(2k)` is finite and at least
`2^k` (`qmin <= k`, `2k <= emax`). -/
theorem sqrt_ge_two_pow {x : FP f} (hx : x.isFinite) {k : ℤ} (hk : f.qmin ≤ k)
    (hk2 : 2 * k ≤ f.emax) (hq : (2 : ℚ) ^ (2 * k) ≤ x.val) :
    x.sqrt.isFinite ∧ (2 : ℚ) ^ k ≤ x.sqrt.val := by
  have he1 := f.one_le_emax
  have hpos : (0 : ℚ) < x.val := lt_of_lt_of_le (two_zpow_pos _) hq
  have hvlt := FP.abs_val_lt hx
  cases x with
  | zero s => simp [FP.val] at hpos
  | inf s => simp [FP.isFinite] at hx
  | nan => simp [FP.isFinite] at hx
  | fin s m e h =>
    cases s with
    | true =>
      exfalso
      have hval : (FP.fin true m e h : FP f).val = -((m : ℚ) * 2 ^ e) := by
        simp [FP.val, FP.sgn]
      have : (0 : ℚ) ≤ (m : ℚ) * 2 ^ e := mul_nonneg (Nat.cast_nonneg m) (two_zpow_pos e).le
      rw [hval] at hpos; linarith
    | false =>
      rw [sqrt_eq_round]
      set v : ℝ := ((FP.fin false m e h : FP f).val : ℝ) with hv
      have hv0 : (0 : ℝ) ≤ v := by rw [hv]; exact_mod_cast hpos.le
      have hqR : (2 : ℝ) ^ (2 * k) ≤ v := by
        rw [hv]; have h' := (Rat.cast_le (K := ℝ)).mpr hq; push_cast at h'; exact h'
      have hsq : (2 : ℝ) ^ k ≤ Real.sqrt v := by
        rw [Real.le_sqrt (two_zpow_pos _).le hv0, sq_two_zpow]; exact hqR
      -- finite: sqrt v <= 2^emax, as v < 2^(emax+1) <= 2^(2 emax)
      have hvlt' : v < 2 ^ (f.emax + 1) := by
        rw [hv]; have h' := (Rat.cast_lt (K := ℝ)).mpr (abs_lt.mp hvlt).2; push_cast at h'; exact h'
      have hsqrt_le : Real.sqrt v ≤ 2 ^ f.emax := by
        rw [Real.sqrt_le_left (two_zpow_pos _).le, sq_two_zpow]
        exact le_trans hvlt'.le (two_zpow_le (by omega))
      have hfin : (round f false (Real.sqrt v)).isFinite :=
        round_finite_of_le_emax_K f (by rw [abs_of_nonneg (Real.sqrt_nonneg _)]; exact hsqrt_le)
      refine ⟨hfin, ?_⟩
      obtain ⟨hf2, hv2⟩ := round_two_pow_real f false hk (by omega)
      have hle := round_mono (K := ℝ) f (s₁ := false) (s₂ := false) hsq
      have hle' := (FP.le_iff hf2 hfin).mp hle
      have hv2' : (round f false ((2 : ℝ) ^ k)).val = (2 : ℚ) ^ k := by
        have : ((round f false ((2 : ℝ) ^ k)).val : ℝ) = (((2 : ℚ) ^ k : ℚ) : ℝ) := by
          rw [hv2]; push_cast; rfl
        exact_mod_cast this
      linarith

/-- **`sqrt_le_two_pow`**: the f64 square root of a finite positive `x <= 2^(2k)` is at most `2^k`
(`qmin <= k`, `2k <= emax`): so an f64 length above `2^k` has a squared length above `2^(2k)`. -/
theorem sqrt_le_two_pow {x : FP f} (hx : x.isFinite) (hpos : 0 < x.val) {k : ℤ} (hk : f.qmin ≤ k)
    (hk2 : 2 * k ≤ f.emax) (hq : x.val ≤ (2 : ℚ) ^ (2 * k)) :
    x.sqrt.isFinite ∧ x.sqrt.val ≤ (2 : ℚ) ^ k := by
  have he1 := f.one_le_emax
  have hvlt := FP.abs_val_lt hx
  cases x with
  | zero s => simp [FP.val] at hpos
  | inf s => simp [FP.isFinite] at hx
  | nan => simp [FP.isFinite] at hx
  | fin s m e h =>
    cases s with
    | true =>
      exfalso
      have hval : (FP.fin true m e h : FP f).val = -((m : ℚ) * 2 ^ e) := by
        simp [FP.val, FP.sgn]
      have : (0 : ℚ) ≤ (m : ℚ) * 2 ^ e := mul_nonneg (Nat.cast_nonneg m) (two_zpow_pos e).le
      rw [hval] at hpos; linarith
    | false =>
      rw [sqrt_eq_round]
      set v : ℝ := ((FP.fin false m e h : FP f).val : ℝ) with hv
      have hv0 : (0 : ℝ) ≤ v := by rw [hv]; exact_mod_cast hpos.le
      have hqR : v ≤ (2 : ℝ) ^ (2 * k) := by
        rw [hv]; have h' := (Rat.cast_le (K := ℝ)).mpr hq; push_cast at h'; exact h'
      have hsq : Real.sqrt v ≤ (2 : ℝ) ^ k := by
        rw [Real.sqrt_le_left (two_zpow_pos _).le, sq_two_zpow]; exact hqR
      have hvlt' : v < 2 ^ (f.emax + 1) := by
        rw [hv]; have h' := (Rat.cast_lt (K := ℝ)).mpr (abs_lt.mp hvlt).2; push_cast at h'; exact h'
      have hsqrt_le : Real.sqrt v ≤ 2 ^ f.emax := by
        rw [Real.sqrt_le_left (two_zpow_pos _).le, sq_two_zpow]
        exact le_trans hvlt'.le (two_zpow_le (by omega))
      have hfin : (round f false (Real.sqrt v)).isFinite :=
        round_finite_of_le_emax_K f (by rw [abs_of_nonneg (Real.sqrt_nonneg _)]; exact hsqrt_le)
      refine ⟨hfin, ?_⟩
      obtain ⟨hf2, hv2⟩ := round_two_pow_real f false hk (by omega)
      have hle := round_mono (K := ℝ) f (s₁ := false) (s₂ := false) hsq
      have hle' := (FP.le_iff hfin hf2).mp hle
      have hv2' : (round f false ((2 : ℝ) ^ k)).val = (2 : ℚ) ^ k := by
        have : ((round f false ((2 : ℝ) ^ k)).val : ℝ) = (((2 : ℚ) ^ k : ℚ) : ℝ) := by
          rw [hv2]; push_cast; rfl
        exact_mod_cast this
      linarith

/-- A square, from its rounded value: `c² >= ĉ²/2 - (ĉ - c)²`. -/
lemma sq_ge_half_sub (c d : ℚ) : d ^ 2 / 2 - (d - c) ^ 2 ≤ c ^ 2 := by
  nlinarith [sq_nonneg (d - 2 * c)]

/-- A zero square root: the f64 square root of a finite zero is zero. -/
lemma sqrt_val_of_val_zero {x : FP f} (hx : x.isFinite) (h0 : x.val = 0) : x.sqrt.val = 0 := by
  cases x with
  | zero s => cases s <;> rfl
  | inf s => simp [FP.isFinite] at hx
  | nan => simp [FP.isFinite] at hx
  | fin s m e h =>
    exfalso
    have hm : (0 : ℚ) < m := by exact_mod_cast h.1
    have he := two_zpow_pos (K := ℚ) e
    have hm' : m ≠ 0 := by exact_mod_cast hm.ne'
    cases s <;> simp [FP.val, FP.sgn] at h0 <;> rcases h0 with h0 | h0
    all_goals first | exact hm' h0 | exact absurd h0 he.ne'

/-- **`len_gt_sq_gt`**: an f64 length `sqrt q` above `2^k` comes from a squared length `q` above
`2^(2k)` (for `q` finite and non-negative). -/
theorem len_gt_sq_gt {q : FP f} (hq : q.isFinite) (hq0 : 0 ≤ q.val) {k : ℤ} (hk : f.qmin ≤ k)
    (hk2 : 2 * k ≤ f.emax) (hL : (2 : ℚ) ^ k < q.sqrt.val) : (2 : ℚ) ^ (2 * k) < q.val := by
  by_contra hle
  push_neg at hle
  rcases eq_or_lt_of_le hq0 with h0 | hpos
  · rw [sqrt_val_of_val_zero f hq h0.symm] at hL
    exact absurd hL (not_lt.mpr (two_zpow_pos k).le)
  · have := (sqrt_le_two_pow f hq hpos hk hk2 hle).2
    linarith

/-- The f64 sum of three squares is non-negative (given its intermediate results finite). -/
lemma sq3_nonneg {x y z : FP f} (hx : x.isFinite) (hy : y.isFinite) (hz : z.isFinite)
    (f1 : (FP.mul x x).isFinite) (f2 : (FP.mul y y).isFinite) (f3 : (FP.mul z z).isFinite)
    (fs : (FP.add (FP.mul x x) (FP.mul y y)).isFinite) :
    0 ≤ (FP.add (FP.add (FP.mul x x) (FP.mul y y)) (FP.mul z z)).val := by
  have p1 : 0 ≤ (FP.mul x x).val := by
    rw [mul_eq_round hx hx]; exact round_val_nonneg f (mul_self_nonneg _)
  have p2 : 0 ≤ (FP.mul y y).val := by
    rw [mul_eq_round hy hy]; exact round_val_nonneg f (mul_self_nonneg _)
  have p3 : 0 ≤ (FP.mul z z).val := by
    rw [mul_eq_round hz hz]; exact round_val_nonneg f (mul_self_nonneg _)
  have ps : 0 ≤ (FP.add (FP.mul x x) (FP.mul y y)).val := by
    rw [add_eq_round f1 f2]; exact round_val_nonneg f (by linarith)
  rw [add_eq_round fs f3]; exact round_val_nonneg f (by linarith)

/-! ## binary64: the caller's test gives the exact cross product a size -/

/-- Rust's `Vec3::cross` component: `p*q - r*s`. -/
def cx (p q r s : F64) : F64 := FP.sub (FP.mul p q) (FP.mul r s)

/-- Rust's `Vec3::length_sq`: `x*x + y*y + z*z`, left to right. -/
def lsq (x y z : F64) : F64 := FP.add (FP.add (FP.mul x x) (FP.mul y y)) (FP.mul z z)

lemma u64 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) = 1 / 9007199254740992 := by
  rw [binary64_prec]; norm_num

lemma η64_le : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 / 2 ^ 1000 := by
  rw [binary64_qmin]; norm_num
  exact inv_anti₀ (by positivity) (pow_le_pow_right₀ (by norm_num) (by norm_num))

lemma emax64 : (2 : ℚ) ^ binary64.emax = 2 ^ 1023 := by rw [binary64_emax]; norm_num

/-- The caller's arithmetic over ℚ: f64 components `d` within `5u + 4 eta` of exact `c`, an f64
squared length `Q` within `4u S + 6 eta` of `S = Σ d²` and above `2^-74`: then `Σ c² >= 2^-76`
(`u = 2^-53`, `eta <= 2^-1000`). -/
lemma caller_core (c1 c2 c3 d1 d2 d3 Q η : ℚ) (hη0 : 0 ≤ η) (hη : η ≤ 1 / 2 ^ 1000)
    (e1 : |d1 - c1| ≤ 5 * (1 / 9007199254740992) * 1 + 4 * η)
    (e2 : |d2 - c2| ≤ 5 * (1 / 9007199254740992) * 1 + 4 * η)
    (e3 : |d3 - c3| ≤ 5 * (1 / 9007199254740992) * 1 + 4 * η)
    (hq : Q - (d1 * d1 + d2 * d2 + d3 * d3) ≤
      4 * (1 / 9007199254740992) * (d1 * d1 + d2 * d2 + d3 * d3) + 6 * η)
    (hgt : 1 / 2 ^ 74 < Q) : 1 / 2 ^ 76 ≤ c1 * c1 + c2 * c2 + c3 * c3 := by
  have hη' : η ≤ 1 / 2 ^ 100 :=
    le_trans hη (one_div_le_one_div_of_le (by positivity) (pow_le_pow_right₀ (by norm_num) (by norm_num)))
  have hδ : 5 * (1 / 9007199254740992) * 1 + 4 * η ≤ (6 : ℚ) / 9007199254740992 := by
    have : (4 : ℚ) / 2 ^ 100 ≤ 1 / 9007199254740992 := by norm_num
    linarith
  have sq_of_abs : ∀ d c : ℚ, |d - c| ≤ 6 / 9007199254740992 →
      (d - c) * (d - c) ≤ (6 / 9007199254740992) * (6 / 9007199254740992) := fun d c h => by
    have := abs_le.mp h; nlinarith
  have s1 := sq_of_abs d1 c1 (le_trans e1 hδ)
  have s2 := sq_of_abs d2 c2 (le_trans e2 hδ)
  have s3 := sq_of_abs d3 c3 (le_trans e3 hδ)
  have h1 := sq_ge_half_sub c1 d1; have h2 := sq_ge_half_sub c2 d2; have h3 := sq_ge_half_sub c3 d3
  simp only [sq] at h1 h2 h3
  -- S = Σ d² is at least (2^-74 - 6 eta) / (1 + 4u)
  have hS : (1 / 2 ^ 74 - 6 / 2 ^ 100 : ℚ) ≤ (d1 * d1 + d2 * d2 + d3 * d3) * (1 + 4 * (1 / 9007199254740992)) := by
    linarith
  have hnum : (1 : ℚ) / 2 ^ 76 + 3 * ((6 / 9007199254740992) * (6 / 9007199254740992)) ≤
      (1 / 2 ^ 74 - 6 / 2 ^ 100) / (2 * (1 + 4 * (1 / 9007199254740992))) := by norm_num
  have hd : (1 / 2 ^ 74 - 6 / 2 ^ 100) / (2 * (1 + 4 * (1 / 9007199254740992))) ≤
      (d1 * d1 + d2 * d2 + d3 * d3) / 2 := by
    rw [div_le_div_iff₀ (by norm_num) (by norm_num)]; linarith
  linarith

/-- **`caller_bound`**: where `cylinder_cylinder` goes on to `equal_crossing`, the f64
`|a × b|` exceeds `ANGULAR = 1e-11 > 2^-37`; then the exact `|a × b|²` is at least `2^-76`
(for `a`, `b` with components finite and at most 1, as unit vectors' are). -/
theorem caller_bound {a1 a2 a3 b1 b2 b3 : F64} (ha1 : a1.isFinite) (ha2 : a2.isFinite)
    (ha3 : a3.isFinite) (hb1 : b1.isFinite) (hb2 : b2.isFinite) (hb3 : b3.isFinite)
    (ba1 : |a1.val| ≤ 1) (ba2 : |a2.val| ≤ 1) (ba3 : |a3.val| ≤ 1)
    (bb1 : |b1.val| ≤ 1) (bb2 : |b2.val| ≤ 1) (bb3 : |b3.val| ≤ 1)
    (hL : (2 : ℚ) ^ (-37 : ℤ) < (lsq (cx a2 b3 a3 b2) (cx a3 b1 a1 b3) (cx a1 b2 a2 b1)).sqrt.val) :
    (2 : ℚ) ^ (-76 : ℤ) ≤ (a2.val * b3.val - a3.val * b2.val) * (a2.val * b3.val - a3.val * b2.val) +
      (a3.val * b1.val - a1.val * b3.val) * (a3.val * b1.val - a1.val * b3.val) +
      (a1.val * b2.val - a2.val * b1.val) * (a1.val * b2.val - a2.val * b1.val) := by
  have hu8 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) ≤ 1 / 8 := by rw [u64]; norm_num
  have hη1 : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 :=
    le_trans η64_le (by rw [one_div]; exact inv_le_one_of_one_le₀ (one_le_pow₀ (by norm_num)))
  have hM : 4 * (1 : ℚ) + 8 ≤ 2 ^ binary64.emax := by
    rw [emax64]; norm_num
    exact le_trans (by norm_num : (12 : ℚ) ≤ 2 ^ 4) (pow_le_pow_right₀ (by norm_num) (by norm_num))
  have pb : ∀ {p q : F64}, |p.val| ≤ 1 → |q.val| ≤ 1 → |p.val * q.val| ≤ 1 := fun hp hq => by
    rw [abs_mul]; exact mul_le_one₀ hp (abs_nonneg _) hq
  obtain ⟨fc1, ec1⟩ := diff_prod_err binary64 ha2 hb3 ha3 hb2 (pb ba2 bb3) (pb ba3 bb2) hM hu8 hη1
  obtain ⟨fc2, ec2⟩ := diff_prod_err binary64 ha3 hb1 ha1 hb3 (pb ba3 bb1) (pb ba1 bb3) hM hu8 hη1
  obtain ⟨fc3, ec3⟩ := diff_prod_err binary64 ha1 hb2 ha2 hb1 (pb ba1 bb2) (pb ba2 bb1) hM hu8 hη1
  rw [u64] at ec1 ec2 ec3
  have hη := η64_le
  have η0 : (0 : ℚ) ≤ 2 ^ (binary64.qmin - 1) := (two_zpow_pos _).le
  -- each f64 component at most 3 (the exact one at most 2, the error at most 1)
  have hδ1 : 5 * (1 / 9007199254740992 : ℚ) * 1 + 4 * 2 ^ (binary64.qmin - 1) ≤ 1 := by
    have : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 / 8 :=
      le_trans hη (one_div_le_one_div_of_le (by norm_num)
        (le_trans (by norm_num : (8 : ℚ) ≤ 2 ^ 3) (pow_le_pow_right₀ (by norm_num) (by norm_num))))
    linarith
  have small : ∀ (p q r t : F64) (c : FP binary64), |p.val * q.val| ≤ 1 → |r.val * t.val| ≤ 1 →
      |c.val - (p.val * q.val - r.val * t.val)| ≤ 5 * (1 / 9007199254740992) * 1 + 4 * 2 ^ (binary64.qmin - 1) →
      c.val * c.val ≤ 9 := fun p q r t c h1 h2 hc => by
    have := abs_le.mp h1; have := abs_le.mp h2; have := abs_le.mp hc
    have hc3 : |c.val| ≤ 3 := abs_le.mpr ⟨by linarith, by linarith⟩
    have := abs_le.mp hc3; nlinarith
  have q1 := small a2 b3 a3 b2 _ (pb ba2 bb3) (pb ba3 bb2) ec1
  have q2 := small a3 b1 a1 b3 _ (pb ba3 bb1) (pb ba1 bb3) ec2
  have q3 := small a1 b2 a2 b1 _ (pb ba1 bb2) (pb ba2 bb1) ec3
  have e10 : (10 : ℚ) ≤ 2 ^ binary64.emax := by
    rw [emax64]; exact le_trans (by norm_num : (10 : ℚ) ≤ 2 ^ 4) (pow_le_pow_right₀ (by norm_num) (by norm_num))
  have e116 : (116 : ℚ) ≤ 2 ^ binary64.emax := by
    rw [emax64]; exact le_trans (by norm_num : (116 : ℚ) ≤ 2 ^ 7) (pow_le_pow_right₀ (by norm_num) (by norm_num))
  have hS : 4 * ((FP.sub (FP.mul a2 b3) (FP.mul a3 b2)).val * (FP.sub (FP.mul a2 b3) (FP.mul a3 b2)).val +
      (FP.sub (FP.mul a3 b1) (FP.mul a1 b3)).val * (FP.sub (FP.mul a3 b1) (FP.mul a1 b3)).val +
      (FP.sub (FP.mul a1 b2) (FP.mul a2 b1)).val * (FP.sub (FP.mul a1 b2) (FP.mul a2 b1)).val) + 8 ≤
      2 ^ binary64.emax := by linarith
  obtain ⟨fq, eq⟩ := sq3_err binary64 fc1 fc2 fc3 hu8 hη1 hS
  rw [u64] at eq
  -- the squares and their first sum are finite, so the f64 squared length is non-negative
  have m1 := (mul_err_le binary64 fc1 fc1 (by rw [abs_of_nonneg (mul_self_nonneg _)]; linarith)).1
  have m2 := (mul_err_le binary64 fc2 fc2 (by rw [abs_of_nonneg (mul_self_nonneg _)]; linarith)).1
  have m3 := (mul_err_le binary64 fc3 fc3 (by rw [abs_of_nonneg (mul_self_nonneg _)]; linarith)).1
  have r1 : (FP.mul (FP.sub (FP.mul a2 b3) (FP.mul a3 b2)) (FP.sub (FP.mul a2 b3) (FP.mul a3 b2))).val ≤ 10 := by
    have := (mul_err_le binary64 fc1 fc1 (by rw [abs_of_nonneg (mul_self_nonneg _)]; linarith)).2
    rw [u64, abs_of_nonneg (mul_self_nonneg (FP.sub (FP.mul a2 b3) (FP.mul a3 b2)).val)] at this
    have := (abs_le.mp this).2
    have : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 := hη1
    nlinarith
  have r2 : (FP.mul (FP.sub (FP.mul a3 b1) (FP.mul a1 b3)) (FP.sub (FP.mul a3 b1) (FP.mul a1 b3))).val ≤ 10 := by
    have := (mul_err_le binary64 fc2 fc2 (by rw [abs_of_nonneg (mul_self_nonneg _)]; linarith)).2
    rw [u64, abs_of_nonneg (mul_self_nonneg (FP.sub (FP.mul a3 b1) (FP.mul a1 b3)).val)] at this
    have := (abs_le.mp this).2
    have : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 := hη1
    nlinarith
  have n1 : 0 ≤ (FP.mul (FP.sub (FP.mul a2 b3) (FP.mul a3 b2)) (FP.sub (FP.mul a2 b3) (FP.mul a3 b2))).val := by
    rw [mul_eq_round fc1 fc1]; exact round_val_nonneg _ (mul_self_nonneg _)
  have n2 : 0 ≤ (FP.mul (FP.sub (FP.mul a3 b1) (FP.mul a1 b3)) (FP.sub (FP.mul a3 b1) (FP.mul a1 b3))).val := by
    rw [mul_eq_round fc2 fc2]; exact round_val_nonneg _ (mul_self_nonneg _)
  have ms := (add_err_le binary64 m1 m2 (by rw [abs_of_nonneg (by linarith)]; linarith)).1
  have q0 := sq3_nonneg binary64 fc1 fc2 fc3 m1 m2 m3 ms
  have hgt := len_gt_sq_gt binary64 (q := lsq (cx a2 b3 a3 b2) (cx a3 b1 a1 b3) (cx a1 b2 a2 b1))
    fq q0 (k := -37) (by rw [binary64_qmin]; norm_num) (by rw [binary64_emax]; norm_num) hL
  have h74 : (2 : ℚ) ^ (2 * (-37 : ℤ)) = 1 / 2 ^ 74 := by norm_num
  have h76 : (2 : ℚ) ^ (-76 : ℤ) = 1 / 2 ^ 76 := by norm_num
  rw [h74] at hgt; rw [h76]
  exact caller_core _ _ _ _ _ _ _ _ η0 hη ec1 ec2 ec3 (abs_le.mp eq).2 hgt

/-! ## binary64: `equal_crossing`'s own `|x × a|`, `x = normalize(a ± b)` -/

/-- A cross product's squared length is at most `|v|² |a|²` (Lagrange's identity). -/
lemma cross_sq_le (v1 v2 v3 a1 a2 a3 : ℚ) :
    (v2 * a3 - v3 * a2) * (v2 * a3 - v3 * a2) + (v3 * a1 - v1 * a3) * (v3 * a1 - v1 * a3) +
      (v1 * a2 - v2 * a1) * (v1 * a2 - v2 * a1) ≤
    (v1 * v1 + v2 * v2 + v3 * v3) * (a1 * a1 + a2 * a2 + a3 * a3) := by
  nlinarith [sq_nonneg (v1 * a1 + v2 * a2 + v3 * a3)]

/-- `D` within `2e` of `E`: `D² >= E²/2 - 4e²`. -/
lemma sq_near_half (ε D E : ℚ) (h1 : -(2 * ε) ≤ D - E) (h2 : D - E ≤ 2 * ε) :
    E * E / 2 - 4 * ε * ε ≤ D * D := by
  nlinarith [sq_nonneg (D + (D - E)), mul_nonneg (by linarith : (0:ℚ) ≤ 2 * ε - (D - E))
    (by linarith : (0:ℚ) ≤ 2 * ε + (D - E))]

/-- **`normalize_core`**: the f64 `v = a ± b` (within `eps` of it, componentwise), its f64 squared
length `q` and length `l` (with `l² >= (1-u)² q`): then `v × a` keeps most of `b × a`, and each
`|v|² <= 1.01² l²` (so `x = v / l` has length at most about 1). Over ℚ, `u = 2^-53`,
`eta <= 2^-100`, `|a_i|, |b_i| <= 1`, and `Σ (a × b)² >= 2^-76` (`caller_bound`). -/
lemma normalize_core (a1 a2 a3 b1 b2 b3 v1 v2 v3 q l η : ℚ) (hη0 : 0 ≤ η) (hη : η ≤ 1 / 2 ^ 100)
    (ba1 : |a1| ≤ 1) (ba2 : |a2| ≤ 1) (ba3 : |a3| ≤ 1)
    (bb1 : |b1| ≤ 1) (bb2 : |b2| ≤ 1) (bb3 : |b3| ≤ 1)
    (hab : 1 / 2 ^ 76 ≤ (a2 * b3 - a3 * b2) * (a2 * b3 - a3 * b2) +
      (a3 * b1 - a1 * b3) * (a3 * b1 - a1 * b3) + (a1 * b2 - a2 * b1) * (a1 * b2 - a2 * b1))
    (hv1 : |v1 - (a1 + b1)| ≤ 3 / 9007199254740992)
    (hv2 : |v2 - (a2 + b2)| ≤ 3 / 9007199254740992)
    (hv3 : |v3 - (a3 + b3)| ≤ 3 / 9007199254740992)
    (hq : |q - (v1 * v1 + v2 * v2 + v3 * v3)| ≤
      4 * (1 / 9007199254740992) * (v1 * v1 + v2 * v2 + v3 * v3) + 6 * η)
    (hl0 : 0 < l) (hl : (1 - 1 / 9007199254740992) ^ 2 * q ≤ l * l) :
    1 / 2 ^ 78 ≤ (v2 * a3 - v3 * a2) * (v2 * a3 - v3 * a2) + (v3 * a1 - v1 * a3) * (v3 * a1 - v1 * a3) +
      (v1 * a2 - v2 * a1) * (v1 * a2 - v2 * a1) ∧
    v1 * v1 + v2 * v2 + v3 * v3 ≤ (101 / 100) ^ 2 * (l * l) := by
  obtain ⟨ε, hε⟩ : ∃ ε : ℚ, ε = 3 / 9007199254740992 := ⟨_, rfl⟩
  rw [← hε] at hv1 hv2 hv3
  -- v × a differs from (a + b) × a = b × a by at most 2 eps per component
  have pm : ∀ r c : ℚ, |r| ≤ ε → |c| ≤ 1 → -ε ≤ r * c ∧ r * c ≤ ε := fun r c hr hc => by
    have : |r * c| ≤ ε := by rw [abs_mul]; nlinarith [abs_nonneg r, abs_nonneg c]
    exact abs_le.mp this
  have r1 := pm _ _ hv1 ba1; have r2 := pm _ _ hv2 ba2; have r3 := pm _ _ hv3 ba3
  have r12 := pm _ _ hv1 ba2; have r13 := pm _ _ hv1 ba3; have r21 := pm _ _ hv2 ba1
  have r23 := pm _ _ hv2 ba3; have r31 := pm _ _ hv3 ba1; have r32 := pm _ _ hv3 ba2
  have dsq := sq_near_half ε
  have k1 := dsq (v2 * a3 - v3 * a2) ((a2 + b2) * a3 - (a3 + b3) * a2)
    (by linarith) (by linarith)
  have k2 := dsq (v3 * a1 - v1 * a3) ((a3 + b3) * a1 - (a1 + b1) * a3)
    (by linarith) (by linarith)
  have k3 := dsq (v1 * a2 - v2 * a1) ((a1 + b1) * a2 - (a2 + b2) * a1)
    (by linarith) (by linarith)
  have hE : (a2 * b3 - a3 * b2) * (a2 * b3 - a3 * b2) + (a3 * b1 - a1 * b3) * (a3 * b1 - a1 * b3) +
      (a1 * b2 - a2 * b1) * (a1 * b2 - a2 * b1) =
      ((a2 + b2) * a3 - (a3 + b3) * a2) * ((a2 + b2) * a3 - (a3 + b3) * a2) +
      ((a3 + b3) * a1 - (a1 + b1) * a3) * ((a3 + b3) * a1 - (a1 + b1) * a3) +
      ((a1 + b1) * a2 - (a2 + b2) * a1) * ((a1 + b1) * a2 - (a2 + b2) * a1) := by ring
  have hD : 1 / 2 ^ 78 ≤ (v2 * a3 - v3 * a2) * (v2 * a3 - v3 * a2) +
      (v3 * a1 - v1 * a3) * (v3 * a1 - v1 * a3) + (v1 * a2 - v2 * a1) * (v1 * a2 - v2 * a1) := by
    have : (1 : ℚ) / 2 ^ 78 + 12 * ε * ε ≤ 1 / 2 ^ 76 / 2 := by rw [hε]; norm_num
    linarith only [k1, k2, k3, hE, hab, this]
  refine ⟨hD, ?_⟩
  -- |v|² is at least |v × a|² / 3, far above eta, so l² >= |v|² / 1.01²
  have ha : a1 * a1 + a2 * a2 + a3 * a3 ≤ 3 := by
    have := abs_le.mp ba1; have := abs_le.mp ba2; have := abs_le.mp ba3
    have h1 : a1 * a1 ≤ 1 := by nlinarith only [abs_le.mp ba1]
    have h2 : a2 * a2 ≤ 1 := by nlinarith only [abs_le.mp ba2]
    have h3 : a3 * a3 ≤ 1 := by nlinarith only [abs_le.mp ba3]
    linarith only [h1, h2, h3]
  have lag := cross_sq_le v1 v2 v3 a1 a2 a3
  have hV0 : 0 ≤ v1 * v1 + v2 * v2 + v3 * v3 :=
    add_nonneg (add_nonneg (mul_self_nonneg _) (mul_self_nonneg _)) (mul_self_nonneg _)
  have hV : 1 / 2 ^ 78 / 3 ≤ v1 * v1 + v2 * v2 + v3 * v3 := by
    have := mul_le_mul_of_nonneg_left ha hV0
    rw [div_le_iff₀ (by norm_num)]; linarith only [this, lag, hD]
  have hql := (abs_le.mp hq).1
  have hL : (101 / 100) ^ (2 : ℕ) * (l * l) ≥ v1 * v1 + v2 * v2 + v3 * v3 := by
    have c1 : (0 : ℚ) < (1 - 1 / 9007199254740992) ^ 2 := by norm_num
    have : (1 - 1 / 9007199254740992 : ℚ) ^ 2 *
        ((v1 * v1 + v2 * v2 + v3 * v3) - 4 * (1 / 9007199254740992) * (v1 * v1 + v2 * v2 + v3 * v3) - 6 * η)
        ≤ l * l := le_trans (mul_le_mul_of_nonneg_left (by linarith) c1.le) hl
    have hη' : 6 * η ≤ 6 / 2 ^ 100 := by linarith only [hη]
    linarith only [this, hη', hV]
  exact hL

/-- `|p - q| <= e`: `p² <= 2q² + 2e²`. -/
lemma sq_near_upper (e p q : ℚ) (h : |p - q| ≤ e) : p * p ≤ 2 * (q * q) + 2 * (e * e) := by
  have := abs_le.mp h
  nlinarith [sq_nonneg (p - 2 * q), mul_nonneg (by linarith : (0:ℚ) ≤ e - (p - q))
    (by linarith : (0:ℚ) ≤ e + (p - q))]

/-- A component of `x × a` against `t × a`, for `x` within `e` of `t` and `|a_i| <= 1`. -/
lemma cross_comp_near (e x1 x2 t1 t2 a1 a2 : ℚ) (h1 : |x1 - t1| ≤ e) (h2 : |x2 - t2| ≤ e)
    (ba1 : |a1| ≤ 1) (ba2 : |a2| ≤ 1) :
    -(2 * e) ≤ (x1 * a2 - x2 * a1) - (t1 * a2 - t2 * a1) ∧
      (x1 * a2 - x2 * a1) - (t1 * a2 - t2 * a1) ≤ 2 * e := by
  have k1 : |(x1 - t1) * a2| ≤ e := by
    rw [abs_mul]; nlinarith [abs_nonneg (x1 - t1), abs_nonneg a2]
  have k2 : |(x2 - t2) * a1| ≤ e := by
    rw [abs_mul]; nlinarith [abs_nonneg (x2 - t2), abs_nonneg a1]
  have := abs_le.mp k1; have := abs_le.mp k2
  constructor <;> nlinarith

/-- `unit_core`'s lower chain, over atoms. -/
lemma chain_lower (u T1 T2 T3 X1 X2 X3 c1 c2 c3 C : ℚ)
    (lx1 : T1 * T1 / 2 - 4 * (2 * u) * (2 * u) ≤ X1 * X1)
    (lx2 : T2 * T2 / 2 - 4 * (2 * u) * (2 * u) ≤ X2 * X2)
    (lx3 : T3 * T3 / 2 - 4 * (2 * u) * (2 * u) ≤ X3 * X3)
    (lc1 : X1 * X1 / 2 - 4 * (6 * u) * (6 * u) ≤ c1 * c1)
    (lc2 : X2 * X2 / 2 - 4 * (6 * u) * (6 * u) ≤ c2 * c2)
    (lc3 : X3 * X3 / 2 - 4 * (6 * u) * (6 * u) ≤ c3 * c3)
    (hTa : 1 / 2 ^ 82 ≤ T1 * T1 + T2 * T2 + T3 * T3)
    (hC : c1 * c1 + c2 * c2 + c3 * c3 = C) : 1 / 2 ^ 84 - 456 * (u * u) ≤ C := by
  linarith only [lc1, lc2, lc3, lx1, lx2, lx3, hTa, hC]

/-- `unit_core`'s upper chain, over atoms. -/
lemma chain_upper (u T1 T2 T3 X1 X2 X3 c1 c2 c3 C P : ℚ) (uu : u * u ≤ 1 / 2 ^ 104)
    (ux1 : X1 * X1 ≤ 2 * (T1 * T1) + 2 * (4 * u * (4 * u)))
    (ux2 : X2 * X2 ≤ 2 * (T2 * T2) + 2 * (4 * u * (4 * u)))
    (ux3 : X3 * X3 ≤ 2 * (T3 * T3) + 2 * (4 * u * (4 * u)))
    (uc1 : c1 * c1 ≤ 2 * (X1 * X1) + 2 * (12 * u * (12 * u)))
    (uc2 : c2 * c2 ≤ 2 * (X2 * X2) + 2 * (12 * u * (12 * u)))
    (uc3 : c3 * c3 ≤ 2 * (X3 * X3) + 2 * (12 * u * (12 * u)))
    (lag : T1 * T1 + T2 * T2 + T3 * T3 ≤ P) (hP : P ≤ (101 / 100) ^ 2 * 3)
    (hC : c1 * c1 + c2 * c2 + c3 * c3 = C) : C ≤ 13 := by
  have : (4 : ℚ) * ((101 / 100) ^ 2 * 3) + 4 * (16 + 144) * (1 / 2 ^ 104) ≤ 13 := by norm_num
  linarith only [this, uu, ux1, ux2, ux3, uc1, uc2, uc3, lag, hP, hC]

/-- `unit_core`'s last step, over atoms. -/
lemma chain_q (u η C Q : ℚ) (hη : η ≤ 1 / 2 ^ 100) (uu : u * u ≤ 1 / 2 ^ 104)
    (hClo : 1 / 2 ^ 84 - 456 * (u * u) ≤ C) (hCup : C ≤ 13) (hC0 : 0 ≤ C)
    (uC : u * C ≤ 1 / 8 * C) (hu : u ≤ 1 / 2 ^ 52)
    (hq : -(4 * u * C + 6 * η) ≤ Q - C ∧ Q - C ≤ 4 * u * C + 6 * η) : 1 / 2 ^ 86 ≤ Q ∧ Q ≤ 16 := by
  have uC' : u * C ≤ 13 / 2 ^ 52 := by nlinarith
  have : (1 : ℚ) / 2 ^ 86 + 6 * (1 / 2 ^ 100) ≤ (1 / 2 ^ 84 - 456 * (1 / 2 ^ 104)) / 2 := by norm_num
  have h2 : (13 : ℚ) + 4 * (13 / 2 ^ 52) + 6 * (1 / 2 ^ 100) ≤ 16 := by norm_num
  constructor
  · linarith only [this, hClo, uu, hq.1, uC, hη]
  · linarith only [h2, hCup, hq.2, uC', hη]

/-- **`unit_core`**: `t = v / l` (`t_i l = v_i`) with `|v × a|² >= 2^-78`, `|v|² <= 1.01² l²` and
`0 < l <= 4`; `x` its f64 rounding (`|x_i - t_i| <= u |t_i| + eta`), `c` the f64 `x × a` (each
within `10u + 4 eta`), `Q` its f64 squared length: `2^-86 <= Q <= 16`. Over ℚ, `u <= 2^-52`,
`eta <= 2^-100`, `|a_i| <= 1`. -/
lemma unit_core (u η a1 a2 a3 v1 v2 v3 t1 t2 t3 x1 x2 x3 c1 c2 c3 Q l : ℚ)
    (hul : 1 / 2 ^ 53 ≤ u) (hu : u ≤ 1 / 2 ^ 52) (hη0 : 0 ≤ η) (hη : η ≤ 1 / 2 ^ 100)
    (ba1 : |a1| ≤ 1) (ba2 : |a2| ≤ 1) (ba3 : |a3| ≤ 1)
    (hD : 1 / 2 ^ 78 ≤ (v2 * a3 - v3 * a2) * (v2 * a3 - v3 * a2) +
      (v3 * a1 - v1 * a3) * (v3 * a1 - v1 * a3) + (v1 * a2 - v2 * a1) * (v1 * a2 - v2 * a1))
    (hV : v1 * v1 + v2 * v2 + v3 * v3 ≤ (101 / 100) ^ 2 * (l * l))
    (hl0 : 0 < l) (hl4 : l ≤ 4)
    (ht1 : t1 * l = v1) (ht2 : t2 * l = v2) (ht3 : t3 * l = v3)
    (hx1 : |x1 - t1| ≤ u * |t1| + η) (hx2 : |x2 - t2| ≤ u * |t2| + η)
    (hx3 : |x3 - t3| ≤ u * |t3| + η)
    (hc1 : |c1 - (x2 * a3 - x3 * a2)| ≤ 5 * u * 2 + 4 * η)
    (hc2 : |c2 - (x3 * a1 - x1 * a3)| ≤ 5 * u * 2 + 4 * η)
    (hc3 : |c3 - (x1 * a2 - x2 * a1)| ≤ 5 * u * 2 + 4 * η)
    (hQ : |Q - (c1 * c1 + c2 * c2 + c3 * c3)| ≤ 4 * u * (c1 * c1 + c2 * c2 + c3 * c3) + 6 * η) :
    1 / 2 ^ 86 ≤ Q ∧ Q ≤ 16 := by
  have hu0 : 0 ≤ u := le_trans (by norm_num) hul
  subst ht1 ht2 ht3
  have hl2 : 0 < l * l := mul_pos hl0 hl0
  -- |t|² <= 1.01², and |t × a|² >= 2^-82
  have hT : t1 * t1 + t2 * t2 + t3 * t3 ≤ (101 / 100) ^ 2 := by
    have : (t1 * t1 + t2 * t2 + t3 * t3) * (l * l) ≤ (101 / 100) ^ 2 * (l * l) := by
      linarith only [hV, show (t1 * l) * (t1 * l) + (t2 * l) * (t2 * l) + (t3 * l) * (t3 * l) =
        (t1 * t1 + t2 * t2 + t3 * t3) * (l * l) by ring]
    exact le_of_mul_le_mul_right this hl2
  have hTa : 1 / 2 ^ 82 ≤ (t2 * a3 - t3 * a2) * (t2 * a3 - t3 * a2) +
      (t3 * a1 - t1 * a3) * (t3 * a1 - t1 * a3) + (t1 * a2 - t2 * a1) * (t1 * a2 - t2 * a1) := by
    generalize hS : (t2 * a3 - t3 * a2) * (t2 * a3 - t3 * a2) +
      (t3 * a1 - t1 * a3) * (t3 * a1 - t1 * a3) + (t1 * a2 - t2 * a1) * (t1 * a2 - t2 * a1) = S
    have hS0 : 0 ≤ S := by
      rw [← hS]; exact add_nonneg (add_nonneg (mul_self_nonneg _) (mul_self_nonneg _)) (mul_self_nonneg _)
    have e : (t1 * l * a2 - t2 * l * a1) * (t1 * l * a2 - t2 * l * a1) +
        ((t2 * l * a3 - t3 * l * a2) * (t2 * l * a3 - t3 * l * a2) +
        (t3 * l * a1 - t1 * l * a3) * (t3 * l * a1 - t1 * l * a3)) = (l * l) * S := by
      rw [← hS]; ring
    have h16 : l * l ≤ 16 := by nlinarith
    have : 1 / 2 ^ 78 ≤ (l * l) * S := by linarith only [hD, e]
    nlinarith
  -- each |t_i| <= 1.01, so x is within 2u of t
  have tb : ∀ t : ℚ, t * t ≤ (101 / 100) ^ 2 → |t| ≤ 101 / 100 := fun t h =>
    abs_le_of_sq_le_sq' (by nlinarith) (by norm_num) |>.elim (fun h1 h2 => abs_le.mpr ⟨h1, h2⟩)
  have s1 := mul_self_nonneg t1; have s2 := mul_self_nonneg t2; have s3 := mul_self_nonneg t3
  have tb1 := tb t1 (by linarith only [hT, s2, s3]); have tb2 := tb t2 (by linarith only [hT, s1, s3])
  have tb3 := tb t3 (by linarith only [hT, s1, s2])
  have e2u : ∀ x t : ℚ, |t| ≤ 101 / 100 → |x - t| ≤ u * |t| + η → |x - t| ≤ 2 * u := fun x t h1 h2 => by
    have : u * |t| ≤ u * (101 / 100) := mul_le_mul_of_nonneg_left h1 hu0
    have : η ≤ u * (1 / 2) := by
      have : (1 : ℚ) / 2 ^ 100 ≤ 1 / 2 ^ 53 * (1 / 2) := by norm_num
      linarith only [this, hul, hη]
    linarith only [this, ‹u * |t| ≤ u * (101 / 100)›, h2, hu0]
  have ex1 := e2u x1 t1 tb1 hx1; have ex2 := e2u x2 t2 tb2 hx2; have ex3 := e2u x3 t3 tb3 hx3
  obtain ⟨d1l, d1h⟩ := cross_comp_near (2 * u) x2 x3 t2 t3 a2 a3 ex2 ex3 ba2 ba3
  obtain ⟨d2l, d2h⟩ := cross_comp_near (2 * u) x3 x1 t3 t1 a3 a1 ex3 ex1 ba3 ba1
  obtain ⟨d3l, d3h⟩ := cross_comp_near (2 * u) x1 x2 t1 t2 a1 a2 ex1 ex2 ba1 ba2
  have h12 : 5 * u * 2 + 4 * η ≤ 2 * (6 * u) := by
    have : (4 : ℚ) / 2 ^ 100 ≤ 2 * (1 / 2 ^ 53) := by norm_num
    linarith only [this, hul, hη]
  have c1' := abs_le.mp (le_trans hc1 h12); have c2' := abs_le.mp (le_trans hc2 h12)
  have c3' := abs_le.mp (le_trans hc3 h12)
  -- lower: |x × a|² >= |t × a|² / 2 - 48u², Σ c² >= that / 2 - 432u²
  have lx1 := sq_near_half (2 * u) _ _ d1l d1h; have lx2 := sq_near_half (2 * u) _ _ d2l d2h
  have lx3 := sq_near_half (2 * u) _ _ d3l d3h
  have lc1 := sq_near_half (6 * u) _ _ c1'.1 c1'.2; have lc2 := sq_near_half (6 * u) _ _ c2'.1 c2'.2
  have lc3 := sq_near_half (6 * u) _ _ c3'.1 c3'.2
  have uu : u * u ≤ 1 / 2 ^ 104 := by
    have h := mul_le_mul hu hu hu0 (by norm_num)
    have e : (1 / 2 ^ 52 : ℚ) * (1 / 2 ^ 52) = 1 / 2 ^ 104 := by norm_num
    linarith only [h, e]
  generalize hC : c1 * c1 + c2 * c2 + c3 * c3 = C at hQ
  have hC0 : 0 ≤ C := by
    rw [← hC]; exact add_nonneg (add_nonneg (mul_self_nonneg _) (mul_self_nonneg _)) (mul_self_nonneg _)
  have hClo := chain_lower u _ _ _ _ _ _ c1 c2 c3 C lx1 lx2 lx3 lc1 lc2 lc3 hTa hC
  have hq := abs_le.mp hQ
  have uC : u * C ≤ (1 / 8) * C := mul_le_mul_of_nonneg_right (le_trans hu (by norm_num : (1 : ℚ) / 2 ^ 52 ≤ 1 / 8)) hC0
  -- upper: |t × a|² <= |t|² |a|² <= 1.01² · 3
  have ha : a1 * a1 + a2 * a2 + a3 * a3 ≤ 3 := by
    have h1 : a1 * a1 ≤ 1 := by nlinarith only [abs_le.mp ba1]
    have h2 : a2 * a2 ≤ 1 := by nlinarith only [abs_le.mp ba2]
    have h3 : a3 * a3 ≤ 1 := by nlinarith only [abs_le.mp ba3]
    linarith only [h1, h2, h3]
  have lag := cross_sq_le t1 t2 t3 a1 a2 a3
  have hT3 : (t1 * t1 + t2 * t2 + t3 * t3) * (a1 * a1 + a2 * a2 + a3 * a3) ≤ (101 / 100) ^ 2 * 3 :=
    mul_le_mul hT ha (add_nonneg (add_nonneg (mul_self_nonneg _) (mul_self_nonneg _)) (mul_self_nonneg _))
      (by norm_num)
  have ux1 := sq_near_upper (4 * u) _ _ (abs_le.mpr ⟨by linarith only [d1l], by linarith only [d1h]⟩ :
    |(x2 * a3 - x3 * a2) - (t2 * a3 - t3 * a2)| ≤ 4 * u)
  have ux2 := sq_near_upper (4 * u) _ _ (abs_le.mpr ⟨by linarith only [d2l], by linarith only [d2h]⟩ :
    |(x3 * a1 - x1 * a3) - (t3 * a1 - t1 * a3)| ≤ 4 * u)
  have ux3 := sq_near_upper (4 * u) _ _ (abs_le.mpr ⟨by linarith only [d3l], by linarith only [d3h]⟩ :
    |(x1 * a2 - x2 * a1) - (t1 * a2 - t2 * a1)| ≤ 4 * u)
  have uc1 := sq_near_upper (12 * u) _ _ (abs_le.mpr ⟨by linarith only [c1'.1], by linarith only [c1'.2]⟩ :
    |c1 - (x2 * a3 - x3 * a2)| ≤ 12 * u)
  have uc2 := sq_near_upper (12 * u) _ _ (abs_le.mpr ⟨by linarith only [c2'.1], by linarith only [c2'.2]⟩ :
    |c2 - (x3 * a1 - x1 * a3)| ≤ 12 * u)
  have uc3 := sq_near_upper (12 * u) _ _ (abs_le.mpr ⟨by linarith only [c3'.1], by linarith only [c3'.2]⟩ :
    |c3 - (x1 * a2 - x2 * a1)| ≤ 12 * u)
  have hCup := chain_upper u _ _ _ _ _ _ c1 c2 c3 C _ uu ux1 ux2 ux3 uc1 uc2 uc3 lag hT3 hC
  exact chain_q u η C Q hη uu hClo hCup hC0 uC hu hq

/-- **`sqrt_sq_lower`**: an f64 square root that is finite and positive came from a positive `x`,
and squares back to at least `(1-u)² x` (any format with `2 emin <= qmin`, as binary64's
`-2044 <= -1074`: the root of every positive value is normal). -/
theorem sqrt_sq_lower (he : 2 * f.emin ≤ f.qmin) {x : FP f} (hfin : x.sqrt.isFinite)
    (hpos : 0 < x.sqrt.val) :
    0 < x.val ∧ (1 - 2 ^ (-(f.prec : ℤ))) ^ 2 * x.val ≤ x.sqrt.val * x.sqrt.val := by
  cases x with
  | zero s => simp [FP.sqrt, FP.val] at hpos
  | inf s => cases s <;> simp [FP.sqrt, FP.isFinite] at hfin
  | nan => simp [FP.sqrt, FP.isFinite] at hfin
  | fin s m e h =>
    cases s
    · have hv : (FP.fin false m e h : FP f).val = (m : ℚ) * 2 ^ e := by simp [FP.val, FP.sgn]
      have hm : (1 : ℚ) ≤ m := by exact_mod_cast h.1
      have hX : (2 : ℚ) ^ (2 * f.emin) ≤ (m : ℚ) * 2 ^ e := by
        have h1 : (2 : ℚ) ^ (2 * f.emin) ≤ 2 ^ e :=
          zpow_le_zpow_right₀ (by norm_num) (le_trans he h.2.2.1)
        nlinarith [two_zpow_pos (K := ℚ) e]
      have hX0 : (0 : ℚ) < (m : ℚ) * 2 ^ e := lt_of_lt_of_le (two_zpow_pos _) hX
      refine ⟨by rw [hv]; exact hX0, ?_⟩
      have hn : (2 : ℝ) ^ f.emin ≤ Real.sqrt ((FP.fin false m e h : FP f).val : ℝ) := by
        rw [hv]
        have hsq : (2 : ℝ) ^ f.emin = Real.sqrt (((2 : ℝ) ^ f.emin) ^ 2) :=
          (Real.sqrt_sq (two_zpow_pos _).le).symm
        rw [hsq]
        apply Real.sqrt_le_sqrt
        have : (((2 : ℚ) ^ (2 * f.emin) : ℚ) : ℝ) ≤ (((m : ℚ) * 2 ^ e : ℚ) : ℝ) := by exact_mod_cast hX
        rw [← zpow_natCast, ← zpow_mul, mul_comm]; push_cast at this ⊢; simpa using this
      have hr := sqrt_rel_err f h hfin hn
      generalize hL : (FP.fin false m e h : FP f).sqrt.val = L at hr ⊢
      generalize hXv : (FP.fin false m e h : FP f).val = X at hr hn ⊢
      have hX0' : (0 : ℝ) ≤ (X : ℝ) := by rw [← hXv, hv]; exact_mod_cast hX0.le
      have hS := Real.sq_sqrt hX0'
      have hu : (2 : ℝ) ^ (-(f.prec : ℤ)) ≤ 1 := zpow_le_one_of_nonpos₀ (by norm_num) (by omega)
      have hu0 : (0 : ℝ) ≤ 2 ^ (-(f.prec : ℤ)) := (two_zpow_pos _).le
      have hS0 := Real.sqrt_nonneg (X : ℝ)
      have hlo := (abs_le.mp hr).1
      have hLr : (1 - 2 ^ (-(f.prec : ℤ))) * Real.sqrt (X : ℝ) ≤ (L : ℝ) := by linarith
      have hc0 : (0 : ℝ) ≤ (1 - 2 ^ (-(f.prec : ℤ))) * Real.sqrt (X : ℝ) :=
        mul_nonneg (by linarith) hS0
      have : ((1 - 2 ^ (-(f.prec : ℤ))) ^ 2 * X : ℚ) ≤ (L * L : ℚ) := by
        have h2 := mul_le_mul hLr hLr hc0 (le_trans hc0 hLr)
        have : ((1 - 2 ^ (-(f.prec : ℤ))) ^ 2 * X : ℚ) = (((1 - 2 ^ (-(f.prec : ℤ))) * Real.sqrt (X : ℝ)) *
            ((1 - 2 ^ (-(f.prec : ℤ))) * Real.sqrt (X : ℝ)) : ℝ) := by
          push_cast; rw [show ∀ a b : ℝ, a * b * (a * b) = a ^ 2 * b ^ 2 from fun a b => by ring, hS]
        exact_mod_cast (this ▸ h2 : _)
      exact this
    · simp [FP.sqrt, FP.isFinite] at hfin

/-- `t = v / l` (`t_i l = v_i`) with `|v|² <= 1.01² l²`: each `|t_i| <= 1.01`. -/
lemma t_bound (v1 v2 v3 t1 t2 t3 l : ℚ) (hl0 : 0 < l)
    (hV : v1 * v1 + v2 * v2 + v3 * v3 ≤ (101 / 100) ^ 2 * (l * l))
    (ht1 : t1 * l = v1) (ht2 : t2 * l = v2) (ht3 : t3 * l = v3) :
    |t1| ≤ 101 / 100 ∧ |t2| ≤ 101 / 100 ∧ |t3| ≤ 101 / 100 := by
  subst ht1 ht2 ht3
  have hl2 : 0 < l * l := mul_pos hl0 hl0
  have hT : t1 * t1 + t2 * t2 + t3 * t3 ≤ (101 / 100) ^ 2 := by
    have : (t1 * t1 + t2 * t2 + t3 * t3) * (l * l) ≤ (101 / 100) ^ 2 * (l * l) := by
      linarith only [hV, show (t1 * l) * (t1 * l) + (t2 * l) * (t2 * l) + (t3 * l) * (t3 * l) =
        (t1 * t1 + t2 * t2 + t3 * t3) * (l * l) by ring]
    exact le_of_mul_le_mul_right this hl2
  have s1 := mul_self_nonneg t1; have s2 := mul_self_nonneg t2; have s3 := mul_self_nonneg t3
  have tb : ∀ t : ℚ, t * t ≤ (101 / 100) ^ 2 → |t| ≤ 101 / 100 := fun t h =>
    abs_le_of_sq_le_sq' (by nlinarith) (by norm_num) |>.elim (fun h1 h2 => abs_le.mpr ⟨h1, h2⟩)
  exact ⟨tb t1 (by linarith only [hT, s2, s3]), tb t2 (by linarith only [hT, s1, s3]),
    tb t3 (by linarith only [hT, s1, s2])⟩

/-- `x` within `u |t| + eta` of `t`, `|t| <= 1.01`, `|a| <= 1`: `|x| <= 2` and `|x a| <= 2`. -/
lemma x_bound (u η x t a : ℚ) (hu0 : 0 ≤ u) (hu : u ≤ 1 / 8) (hη : η ≤ 1 / 8)
    (ht : |t| ≤ 101 / 100) (hx : |x - t| ≤ u * |t| + η) (ha : |a| ≤ 1) :
    |x| ≤ 2 ∧ |x * a| ≤ 2 := by
  have : u * |t| ≤ 1 / 8 * (101 / 100) := mul_le_mul hu ht (abs_nonneg _) (by norm_num)
  have h1 := abs_le.mp hx; have h2 := abs_le.mp ht
  have hx2 : |x| ≤ 2 := abs_le.mpr ⟨by linarith, by linarith⟩
  refine ⟨hx2, ?_⟩
  rw [abs_mul]; nlinarith [abs_nonneg x, abs_nonneg a]

/-- `|p| <= 2 + 3 eps` style: `|v - (a + b)| <= e`, `|a|, |b| <= 1`: `|v| <= 2 + e`. -/
lemma v_bound (v a b e : ℚ) (h : |v - (a + b)| ≤ e) (ha : |a| ≤ 1) (hb : |b| ≤ 1) : |v| ≤ 2 + e := by
  have := abs_le.mp h; have := abs_le.mp ha; have := abs_le.mp hb
  exact abs_le.mpr ⟨by linarith, by linarith⟩

lemma sq_le_of_abs_le {v B : ℚ} (h : |v| ≤ B) : v * v ≤ B * B := by
  have := abs_le.mp h; nlinarith [abs_nonneg v]

/-- **`unit_cross_len`** (binary64): `v` the f64 `a ± b` (`b` here the exact `±b`, each component
within `3u` of `a_i + b_i`), `l` its f64 length (finite and positive: `normalize` returns `None`
otherwise), `x_i = v_i / l` in f64, `Q` the f64 `|x × a|²`: then `sqrt Q` is finite and in
`[2^-43, 4]`. For `a`, `b` with components at most 1 and the exact `|a × b|² >= 2^-76`. -/
theorem unit_cross_len {a1 a2 a3 v1 v2 v3 l x1 x2 x3 : F64} {b1 b2 b3 : ℚ}
    (fa1 : a1.isFinite) (fa2 : a2.isFinite) (fa3 : a3.isFinite)
    (fv1 : v1.isFinite) (fv2 : v2.isFinite) (fv3 : v3.isFinite)
    (ba1 : |a1.val| ≤ 1) (ba2 : |a2.val| ≤ 1) (ba3 : |a3.val| ≤ 1)
    (bb1 : |b1| ≤ 1) (bb2 : |b2| ≤ 1) (bb3 : |b3| ≤ 1)
    (hab : 1 / 2 ^ 76 ≤ (a2.val * b3 - a3.val * b2) * (a2.val * b3 - a3.val * b2) +
      (a3.val * b1 - a1.val * b3) * (a3.val * b1 - a1.val * b3) +
      (a1.val * b2 - a2.val * b1) * (a1.val * b2 - a2.val * b1))
    (hv1 : |v1.val - (a1.val + b1)| ≤ 3 / 9007199254740992)
    (hv2 : |v2.val - (a2.val + b2)| ≤ 3 / 9007199254740992)
    (hv3 : |v3.val - (a3.val + b3)| ≤ 3 / 9007199254740992)
    (hl : l = (lsq v1 v2 v3).sqrt) (fl : l.isFinite) (hl0 : 0 < l.val)
    (hx1 : x1 = FP.div v1 l) (hx2 : x2 = FP.div v2 l) (hx3 : x3 = FP.div v3 l) :
    (lsq (cx x2 a3 x3 a2) (cx x3 a1 x1 a3) (cx x1 a2 x2 a1)).sqrt.isFinite ∧
      (2 : ℚ) ^ (-43 : ℤ) ≤ (lsq (cx x2 a3 x3 a2) (cx x3 a1 x1 a3) (cx x1 a2 x2 a1)).sqrt.val ∧
      (lsq (cx x2 a3 x3 a2) (cx x3 a1 x1 a3) (cx x1 a2 x2 a1)).sqrt.val ≤ 4 := by
  have hu8 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) ≤ 1 / 8 := by rw [u64]; norm_num
  have hη1 : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 :=
    le_trans η64_le (by rw [one_div]; exact inv_le_one_of_one_le₀ (one_le_pow₀ (by norm_num)))
  have hη := η64_le
  have η0 : (0 : ℚ) ≤ 2 ^ (binary64.qmin - 1) := (two_zpow_pos _).le
  have hη100 : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 / 2 ^ 100 :=
    le_trans hη (one_div_le_one_div_of_le (by positivity) (pow_le_pow_right₀ (by norm_num) (by norm_num)))
  have hη8 : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 / 8 := le_trans hη100 (by norm_num)
  have e1023 : ∀ c : ℚ, c ≤ 2 ^ 10 → c ≤ 2 ^ binary64.emax := fun c hc => by
    rw [emax64]; exact le_trans hc (pow_le_pow_right₀ (by norm_num) (by norm_num))
  -- the f64 squared length of v, and v's length l
  have vb1 := sq_le_of_abs_le (v_bound _ _ _ _ hv1 ba1 bb1)
  have vb2 := sq_le_of_abs_le (v_bound _ _ _ _ hv2 ba2 bb2)
  have vb3 := sq_le_of_abs_le (v_bound _ _ _ _ hv3 ba3 bb3)
  have hv9 : (2 + 3 / 9007199254740992 : ℚ) * (2 + 3 / 9007199254740992) ≤ 5 := by norm_num
  obtain ⟨fq, eq⟩ := sq3_err binary64 fv1 fv2 fv3 hu8 hη1 (e1023 _ (by linarith))
  rw [u64] at eq
  have hlo := sqrt_sq_lower binary64 (by rw [binary64_emin, binary64_qmin]; norm_num)
    (hl ▸ fl) (hl ▸ hl0)
  rw [← hl, u64] at hlo
  obtain ⟨hD, hV⟩ := normalize_core _ _ _ _ _ _ _ _ _ _ _ _ η0 hη100 ba1 ba2 ba3 bb1 bb2 bb3 hab
    hv1 hv2 hv3 eq hl0 hlo.2
  -- l <= 4: its square q is at most 16
  have hq16 : (lsq v1 v2 v3).val ≤ (2 : ℚ) ^ (2 * (2 : ℤ)) := by
    have := (abs_le.mp eq).2
    have : (16 : ℚ) = 2 ^ (2 * (2 : ℤ)) := by norm_num
    rw [← this]
    show (FP.add (FP.add (FP.mul v1 v1) (FP.mul v2 v2)) (FP.mul v3 v3)).val ≤ 16
    linarith
  have hl4 := (sqrt_le_two_pow binary64 fq hlo.1 (k := 2) (by rw [binary64_qmin]; norm_num)
    (by rw [binary64_emax]; norm_num) hq16).2
  have hl4' : l.val ≤ 4 := by
    have e4 : (2 : ℚ) ^ (2 : ℤ) = 4 := by norm_num
    rw [e4] at hl4; rw [hl]; exact hl4
  -- x_i = v_i / l
  have hl0' : l.val ≠ 0 := ne_of_gt hl0
  have ht : ∀ v : F64, v.val / l.val * l.val = v.val := fun v => div_mul_cancel₀ _ hl0'
  obtain ⟨tb1, tb2, tb3⟩ := t_bound _ _ _ _ _ _ _ hl0 hV (ht v1) (ht v2) (ht v3)
  have dv : ∀ {v : F64}, v.isFinite → |v.val / l.val| ≤ 101 / 100 →
      (FP.div v l).isFinite ∧ |(FP.div v l).val - v.val / l.val| ≤
        1 / 9007199254740992 * |v.val / l.val| + 2 ^ (binary64.qmin - 1) := fun fv h => by
    have := div_err_le binary64 fv fl hl0' (e1023 _ (le_trans h (by norm_num)))
    rwa [u64] at this
  obtain ⟨fx1, ex1⟩ := dv fv1 tb1; obtain ⟨fx2, ex2⟩ := dv fv2 tb2; obtain ⟨fx3, ex3⟩ := dv fv3 tb3
  rw [← hx1] at fx1 ex1; rw [← hx2] at fx2 ex2; rw [← hx3] at fx3 ex3
  have u0 : (0 : ℚ) ≤ 1 / 9007199254740992 := by norm_num
  have u8 : (1 : ℚ) / 9007199254740992 ≤ 1 / 8 := by norm_num
  have xa : ∀ {x : F64} {t : ℚ}, |t| ≤ 101 / 100 →
      |x.val - t| ≤ 1 / 9007199254740992 * |t| + 2 ^ (binary64.qmin - 1) → ∀ a : F64, |a.val| ≤ 1 →
      |x.val * a.val| ≤ 2 := fun ht hx a ha => (x_bound _ _ _ _ _ u0 u8 hη8 ht hx ha).2
  -- the f64 x × a
  have hM : 4 * (2 : ℚ) + 8 ≤ 2 ^ binary64.emax := e1023 _ (by norm_num)
  obtain ⟨fc1, ec1⟩ := diff_prod_err binary64 fx2 fa3 fx3 fa2 (xa tb2 ex2 a3 ba3) (xa tb3 ex3 a2 ba2)
    hM hu8 hη1
  obtain ⟨fc2, ec2⟩ := diff_prod_err binary64 fx3 fa1 fx1 fa3 (xa tb3 ex3 a1 ba1) (xa tb1 ex1 a3 ba3)
    hM hu8 hη1
  obtain ⟨fc3, ec3⟩ := diff_prod_err binary64 fx1 fa2 fx2 fa1 (xa tb1 ex1 a2 ba2) (xa tb2 ex2 a1 ba1)
    hM hu8 hη1
  rw [u64] at ec1 ec2 ec3
  -- each f64 component of x × a at most 5, so the f64 squared length cannot overflow
  have cq : ∀ (c : F64) (p r : ℚ), |p| ≤ 2 → |r| ≤ 2 →
      |c.val - (p - r)| ≤ 5 * (1 / 9007199254740992) * 2 + 4 * 2 ^ (binary64.qmin - 1) →
      c.val * c.val ≤ 25 := fun c p r hp hr hc => by
    have := abs_le.mp hp; have := abs_le.mp hr; have := abs_le.mp hc
    have : 5 * (1 / 9007199254740992 : ℚ) * 2 + 4 * 2 ^ (binary64.qmin - 1) ≤ 1 := by linarith
    exact sq_le_of_abs_le (abs_le.mpr ⟨by linarith, by linarith⟩ : |c.val| ≤ 5) |>.trans (by norm_num)
  have q1 := cq _ _ _ (xa tb2 ex2 a3 ba3) (xa tb3 ex3 a2 ba2) ec1
  have q2 := cq _ _ _ (xa tb3 ex3 a1 ba1) (xa tb1 ex1 a3 ba3) ec2
  have q3 := cq _ _ _ (xa tb1 ex1 a2 ba2) (xa tb2 ex2 a1 ba1) ec3
  obtain ⟨fQ, eQ⟩ := sq3_err binary64 fc1 fc2 fc3 hu8 hη1 (e1023 _ (by linarith))
  rw [u64] at eQ
  obtain ⟨hQlo, hQhi⟩ := unit_core (1 / 9007199254740992) _ a1.val a2.val a3.val v1.val v2.val v3.val
    _ _ _ x1.val x2.val x3.val _ _ _ _ l.val (by norm_num) (by norm_num) η0 hη100 ba1 ba2 ba3 hD hV
    hl0 hl4' (ht v1) (ht v2) (ht v3) ex1 ex2 ex3 ec1 ec2 ec3 eQ
  have hQ0 : 0 < (lsq (cx x2 a3 x3 a2) (cx x3 a1 x1 a3) (cx x1 a2 x2 a1)).val :=
    lt_of_lt_of_le (by norm_num) hQlo
  obtain ⟨fs, hs⟩ := sqrt_ge_two_pow binary64 fQ (k := -43) (by rw [binary64_qmin]; norm_num)
    (by rw [binary64_emax]; norm_num) (by
      have e : (2 : ℚ) ^ (2 * (-43 : ℤ)) = 1 / 2 ^ 86 := by norm_num
      rw [e]; exact hQlo)
  have hs4 := (sqrt_le_two_pow binary64 fQ hQ0 (k := 2) (by rw [binary64_qmin]; norm_num)
    (by rw [binary64_emax]; norm_num) (by
      have e : (2 : ℚ) ^ (2 * (2 : ℤ)) = 16 := by norm_num
      rw [e]; exact hQhi)).2
  have e4 : (2 : ℚ) ^ (2 : ℤ) = 4 := by norm_num
  rw [e4] at hs4
  exact ⟨fs, hs, hs4⟩

/-- `div_pos_finite` for a divisor up to 4: `x / y` is finite and positive for `x` finite in
`[2^(qmin+2), 2^emax * A]` and `y` finite in `[A, 4]`, `A > 0`. -/
theorem div_pos_finite4 {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) {A : ℚ} (hA : 0 < A)
    (hyA : A ≤ y.val) (hy4 : y.val ≤ 4) (hxlo : (2 : ℚ) ^ (f.qmin + 2) ≤ x.val)
    (hxhi : x.val ≤ 2 ^ f.emax * A) :
    (FP.div x y).isFinite ∧ 0 < (FP.div x y).val := by
  have hypos : 0 < y.val := lt_of_lt_of_le hA hyA
  rw [div_eq_round hx hy (ne_of_gt hypos)]
  have hlo : (2 : ℚ) ^ f.qmin ≤ x.val / y.val := by
    rw [le_div_iff₀ hypos]
    have h2 : (2 : ℚ) ^ (f.qmin + 2) = 2 ^ f.qmin * 4 := by
      rw [zpow_add₀ (by norm_num : (2 : ℚ) ≠ 0)]; norm_num
    have hq := two_zpow_pos (K := ℚ) f.qmin
    nlinarith
  have hhi : x.val / y.val ≤ 2 ^ f.emax := by
    rw [div_le_iff₀ hypos]
    have he := two_zpow_pos (K := ℚ) f.emax
    nlinarith
  obtain ⟨hfin, hge⟩ := round_ge_two_pow_qmin f (zs := x.sign ^^ y.sign) hlo hhi
  exact ⟨hfin, lt_of_lt_of_le (two_zpow_pos _) hge⟩

/-- **`equal_crossing_major`** (binary64; kernel/forge-bool/src/ssi.rs `equal_crossing`): for
`a`, `b` with finite components at most 1 where the caller's f64 `|a × b|` exceeds `2^-37`
(`ANGULAR = 1e-11`), `v` the f64 `a + b` or `a - b` (`op`), `normalize` succeeding (`l` finite and
positive; it returns `None` otherwise), `x = v / l` componentwise, and a finite radius `r` in
`[2^(qmin+2), 2^(emax-43)]`: `major = r / |x × a|` is finite and positive. -/
theorem equal_crossing_major {a1 a2 a3 b1 b2 b3 v1 v2 v3 l x1 x2 x3 r : F64}
    {op : F64 → F64 → F64} (hop : op = FP.add ∨ op = FP.sub)
    (fa1 : a1.isFinite) (fa2 : a2.isFinite) (fa3 : a3.isFinite)
    (fb1 : b1.isFinite) (fb2 : b2.isFinite) (fb3 : b3.isFinite)
    (ba1 : |a1.val| ≤ 1) (ba2 : |a2.val| ≤ 1) (ba3 : |a3.val| ≤ 1)
    (bb1 : |b1.val| ≤ 1) (bb2 : |b2.val| ≤ 1) (bb3 : |b3.val| ≤ 1)
    (hL : (2 : ℚ) ^ (-37 : ℤ) < (lsq (cx a2 b3 a3 b2) (cx a3 b1 a1 b3) (cx a1 b2 a2 b1)).sqrt.val)
    (hv1 : v1 = op a1 b1) (hv2 : v2 = op a2 b2) (hv3 : v3 = op a3 b3)
    (hl : l = (lsq v1 v2 v3).sqrt) (fl : l.isFinite) (hl0 : 0 < l.val)
    (hx1 : x1 = FP.div v1 l) (hx2 : x2 = FP.div v2 l) (hx3 : x3 = FP.div v3 l)
    (fr : r.isFinite) (hrlo : (2 : ℚ) ^ (binary64.qmin + 2) ≤ r.val)
    (hrhi : r.val ≤ 2 ^ (binary64.emax - 43)) :
    (FP.div r (lsq (cx x2 a3 x3 a2) (cx x3 a1 x1 a3) (cx x1 a2 x2 a1)).sqrt).isFinite ∧
      0 < (FP.div r (lsq (cx x2 a3 x3 a2) (cx x3 a1 x1 a3) (cx x1 a2 x2 a1)).sqrt).val := by
  have hab := caller_bound fa1 fa2 fa3 fb1 fb2 fb3 ba1 ba2 ba3 bb1 bb2 bb3 hL
  have h76 : (2 : ℚ) ^ (-76 : ℤ) = 1 / 2 ^ 76 := by norm_num
  rw [h76] at hab
  have hu : (2 : ℚ) ^ (-(binary64.prec : ℤ)) = 1 / 9007199254740992 := u64
  have hη : (2 : ℚ) ^ (binary64.qmin - 1) ≤ 1 / 9007199254740992 :=
    le_trans η64_le (le_trans (one_div_le_one_div_of_le (by positivity)
      (pow_le_pow_right₀ (by norm_num) (by norm_num : 100 ≤ 1000))) (by norm_num))
  have e1023 : ∀ c : ℚ, c ≤ 2 ^ 10 → c ≤ 2 ^ binary64.emax := fun c hc => by
    rw [emax64]; exact le_trans hc (pow_le_pow_right₀ (by norm_num) (by norm_num))
  -- each f64 v_i within 3u of a_i ± b_i
  have near : ∀ {p q : ℚ} {w : F64}, |p| ≤ 1 → |q| ≤ 1 →
      |w.val - (p + q)| ≤ 2 ^ (-(binary64.prec : ℤ)) * |p + q| + 2 ^ (binary64.qmin - 1) →
      |w.val - (p + q)| ≤ 3 / 9007199254740992 := fun {p q w} hp hq hw => by
    rw [hu] at hw
    have : |p + q| ≤ 2 := le_trans (abs_add_le _ _) (by linarith)
    have : 1 / 9007199254740992 * |p + q| ≤ 1 / 9007199254740992 * 2 :=
      mul_le_mul_of_nonneg_left this (by norm_num)
    linarith
  have sum2 : ∀ {p q : ℚ}, |p| ≤ 1 → |q| ≤ 1 → |p + q| ≤ 2 ^ binary64.emax := fun {p q} hp hq =>
    e1023 _ (le_trans (abs_add_le _ _) (by linarith))
  have core : ∀ {c1 c2 c3 : ℚ}, |c1| ≤ 1 → |c2| ≤ 1 → |c3| ≤ 1 →
      1 / 2 ^ 76 ≤ (a2.val * c3 - a3.val * c2) * (a2.val * c3 - a3.val * c2) +
        (a3.val * c1 - a1.val * c3) * (a3.val * c1 - a1.val * c3) +
        (a1.val * c2 - a2.val * c1) * (a1.val * c2 - a2.val * c1) →
      (v1.isFinite ∧ |v1.val - (a1.val + c1)| ≤ 3 / 9007199254740992) →
      (v2.isFinite ∧ |v2.val - (a2.val + c2)| ≤ 3 / 9007199254740992) →
      (v3.isFinite ∧ |v3.val - (a3.val + c3)| ≤ 3 / 9007199254740992) →
      (FP.div r (lsq (cx x2 a3 x3 a2) (cx x3 a1 x1 a3) (cx x1 a2 x2 a1)).sqrt).isFinite ∧
        0 < (FP.div r (lsq (cx x2 a3 x3 a2) (cx x3 a1 x1 a3) (cx x1 a2 x2 a1)).sqrt).val :=
    fun {c1 c2 c3} h1 h2 h3 hc e1 e2 e3 => by
      obtain ⟨fs, slo, shi⟩ := unit_cross_len fa1 fa2 fa3 e1.1 e2.1 e3.1 ba1 ba2 ba3 h1 h2 h3 hc
        e1.2 e2.2 e3.2 hl fl hl0 hx1 hx2 hx3
      have hA : (0 : ℚ) < 2 ^ (-43 : ℤ) := two_zpow_pos _
      refine div_pos_finite4 binary64 fr fs hA slo shi hrlo ?_
      rw [← zpow_add₀ (by norm_num : (2 : ℚ) ≠ 0)]; exact hrhi
  rcases hop with rfl | rfl
  · subst hv1 hv2 hv3
    refine core bb1 bb2 bb3 hab ?_ ?_ ?_
    · obtain ⟨f, e⟩ := add_err_le binary64 fa1 fb1 (sum2 ba1 bb1); exact ⟨f, near ba1 bb1 e⟩
    · obtain ⟨f, e⟩ := add_err_le binary64 fa2 fb2 (sum2 ba2 bb2); exact ⟨f, near ba2 bb2 e⟩
    · obtain ⟨f, e⟩ := add_err_le binary64 fa3 fb3 (sum2 ba3 bb3); exact ⟨f, near ba3 bb3 e⟩
  · subst hv1 hv2 hv3
    have nb : ∀ {p : ℚ}, |p| ≤ 1 → |-p| ≤ 1 := fun {p} h => by rwa [abs_neg]
    refine core (nb bb1) (nb bb2) (nb bb3) (by linarith only [hab, show
        (a2.val * -b3.val - a3.val * -b2.val) * (a2.val * -b3.val - a3.val * -b2.val) +
        (a3.val * -b1.val - a1.val * -b3.val) * (a3.val * -b1.val - a1.val * -b3.val) +
        (a1.val * -b2.val - a2.val * -b1.val) * (a1.val * -b2.val - a2.val * -b1.val) =
        (a2.val * b3.val - a3.val * b2.val) * (a2.val * b3.val - a3.val * b2.val) +
        (a3.val * b1.val - a1.val * b3.val) * (a3.val * b1.val - a1.val * b3.val) +
        (a1.val * b2.val - a2.val * b1.val) * (a1.val * b2.val - a2.val * b1.val) by ring]) ?_ ?_ ?_
    · have sn : a1.val - b1.val = a1.val + -b1.val := sub_eq_add_neg _ _
      obtain ⟨f, e⟩ := sub_err_le binary64 fa1 fb1 (by rw [sn]; exact sum2 ba1 (nb bb1))
      rw [sn] at e; exact ⟨f, near ba1 (nb bb1) e⟩
    · have sn : a2.val - b2.val = a2.val + -b2.val := sub_eq_add_neg _ _
      obtain ⟨f, e⟩ := sub_err_le binary64 fa2 fb2 (by rw [sn]; exact sum2 ba2 (nb bb2))
      rw [sn] at e; exact ⟨f, near ba2 (nb bb2) e⟩
    · have sn : a3.val - b3.val = a3.val + -b3.val := sub_eq_add_neg _ _
      obtain ⟨f, e⟩ := sub_err_le binary64 fa3 fb3 (by rw [sn]; exact sum2 ba3 (nb bb3))
      rw [sn] at e; exact ⟨f, near ba3 (nb bb3) e⟩

end IEEE754
