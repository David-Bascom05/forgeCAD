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

end IEEE754
