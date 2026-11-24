/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Theorems
import IEEE754.RoundProps
import IEEE754.TwoProduct
import IEEE754.Sqrt

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

end IEEE754
