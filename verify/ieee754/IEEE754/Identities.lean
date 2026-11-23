/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Theorems

/-!
# Identities and exact integers

`x * 1 = x`, `1 * x = x`, `x / 1 = x`, `x + (-0) = x`, `x - (+0) = x`, `x + (+0) = x` unless
`x = -0`, `fma x y (-0) = x * y`, commutativity of `+` and `*`, `x - x = +0`: for every datum,
NaN and infinities included. Integers up to `2^prec` in magnitude convert exactly and back, and
their sums and products of that size are exact.
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {f : Format}

namespace FP

/-- The datum `1`. -/
def one : FP f :=
  fin false (2 ^ (f.prec - 1)) (1 - f.prec)
    ⟨by positivity, Nat.pow_lt_pow_right (by norm_num) (by have := f.two_le_prec; omega),
      by have := f.one_le_emax; unfold Format.qmin Format.emin; omega,
      by have := f.one_le_emax; unfold Format.qmax; omega, Or.inl le_rfl⟩

@[simp] lemma val_one : (one : FP f).val = 1 := by
  simp only [one, val, sgn]
  rw [cast_two_pow, ← two_zpow_add, f.prec_sub_one]; simp

@[simp] lemma round_val_self {x : FP f} (hx : x.isFinite) (h0 : x.isZero = false) (zs : Bool) :
    round f zs x.val = x := by
  cases x with
  | fin s m e h => exact round_val_q zs h
  | _ => simp_all [isFinite, isZero]

theorem mul_one (x : FP f) : mul x one = x := by
  cases x with
  | zero s => simp [mul, one, val, sign]
  | inf s => simp [mul, one, isZero, sign]
  | nan => rfl
  | fin s m e h =>
    show round f (s ^^ false) ((fin s m e h).val * one.val) = _
    simp only [val_one, _root_.mul_one, Bool.xor_false]
    exact round_val_q s h

theorem one_mul (x : FP f) : mul one x = x := by
  cases x with
  | zero s => simp [mul, one, val, sign]
  | inf s => simp [mul, one, isZero, sign]
  | nan => rfl
  | fin s m e h =>
    show round f (false ^^ s) (one.val * (fin s m e h).val) = _
    simp only [val_one, _root_.one_mul, Bool.false_xor]
    exact round_val_q s h

theorem div_one (x : FP f) : div x one = x := by
  cases x with
  | zero s => simp [div, one, val, isZero, sign]
  | inf s => simp [div, one, sign]
  | nan => rfl
  | fin s m e h =>
    have : div (fin s m e h) (one : FP f) = round f (s ^^ false) ((fin s m e h).val / one.val) := rfl
    rw [this, val_one, _root_.div_one, Bool.xor_false]
    exact round_val_q s h

theorem add_neg_zero (x : FP f) : add x (zero true) = x := by
  cases x with
  | zero s => simp [add, val, sign]
  | inf s => rfl
  | nan => rfl
  | fin s m e h =>
    show round f (s && true) ((fin s m e h).val + 0) = _
    simp only [add_zero, Bool.and_true]
    exact round_val_q s h

theorem add_zero {x : FP f} (hx : x ≠ zero true) : add x (zero false) = x := by
  cases x with
  | zero s => cases s <;> simp_all [add, val, sign]
  | inf s => rfl
  | nan => rfl
  | fin s m e h =>
    show round f (s && false) ((fin s m e h).val + 0) = _
    simp only [_root_.add_zero, Bool.and_false]
    exact round_val_q _ h

theorem neg_zero_add (x : FP f) : add (zero true) x = x := by
  cases x with
  | zero s => simp [add, val, sign]
  | inf s => rfl
  | nan => rfl
  | fin s m e h =>
    show round f (true && s) (0 + (fin s m e h).val) = _
    simp only [_root_.zero_add, Bool.true_and]
    exact round_val_q s h

theorem sub_zero (x : FP f) : sub x (zero false) = x := add_neg_zero x

theorem add_comm (x y : FP f) : add x y = add y x := by
  cases x <;> cases y <;> simp [add, Bool.and_comm, _root_.add_comm] <;> grind

theorem mul_comm (x y : FP f) : mul x y = mul y x := by
  cases x <;> cases y <;> simp [mul, isZero, sign, Bool.xor_comm, _root_.mul_comm]

/-- `x - x = +0` for finite `x`. -/
theorem sub_self {x : FP f} (hx : x.isFinite) : sub x x = zero false := by
  rw [sub_eq_round hx hx, _root_.sub_self, round_zero]; simp

theorem fma_neg_zero (x y : FP f) : fma x y (zero true) = mul x y := by
  cases x <;> cases y <;> simp [fma, mul, isNaN, isInf, isZero, sign, val]

/-! ## Integers -/

/-- **Small integers are exact**: `|n| ≤ 2^prec` converts exactly (when `prec ≤ emax`, true of
every IEEE binary format). -/
theorem ofInt_exact {n : ℤ} (hn : |n| ≤ 2 ^ f.prec) (hf : (f.prec : ℤ) ≤ f.emax) :
    (ofInt (f := f) n).isFinite ∧ (ofInt (f := f) n).val = n := by
  have := round_exact_q (f := f) false (e := 0) hn f.qmin_le_zero (by
    rw [zpow_zero, _root_.mul_one]
    calc |(n : ℚ)| ≤ 2 ^ (f.prec : ℤ) := by rw [zpow_natCast]; exact_mod_cast hn
      _ < 2 ^ (f.emax + 1) := two_zpow_lt (by omega))
  simpa [ofInt] using this

theorem toInt_ofInt {n : ℤ} (hn : |n| ≤ 2 ^ f.prec) (hf : (f.prec : ℤ) ≤ f.emax) :
    (ofInt (f := f) n).toInt? = some n := by
  obtain ⟨h1, h2⟩ := ofInt_exact hn hf
  simp [toInt?, h1, h2, truncQ]

/-- A sum of two exact integers is exact while it stays within `2^prec`. -/
theorem add_int_exact {a b : ℤ} (ha : |a| ≤ 2 ^ f.prec) (hb : |b| ≤ 2 ^ f.prec)
    (hab : |a + b| ≤ 2 ^ f.prec) (hf : (f.prec : ℤ) ≤ f.emax) :
    (add (ofInt (f := f) a) (ofInt b)).val = a + b := by
  obtain ⟨ha1, ha2⟩ := ofInt_exact (f := f) ha hf
  obtain ⟨hb1, hb2⟩ := ofInt_exact (f := f) hb hf
  rw [add_eq_round ha1 hb1, ha2, hb2]
  have := round_exact_q (f := f) ((ofInt (f := f) a).sign && (ofInt (f := f) b).sign) (e := 0) hab
    f.qmin_le_zero (by
      rw [zpow_zero, _root_.mul_one]
      calc |((a + b : ℤ) : ℚ)| ≤ 2 ^ (f.prec : ℤ) := by rw [zpow_natCast]; exact_mod_cast hab
        _ < 2 ^ (f.emax + 1) := two_zpow_lt (by omega))
  simpa using this.2

/-- A product of two exact integers is exact while it stays within `2^prec`. -/
theorem mul_int_exact {a b : ℤ} (ha : |a| ≤ 2 ^ f.prec) (hb : |b| ≤ 2 ^ f.prec)
    (hab : |a * b| ≤ 2 ^ f.prec) (hf : (f.prec : ℤ) ≤ f.emax) :
    (mul (ofInt (f := f) a) (ofInt b)).val = a * b := by
  obtain ⟨ha1, ha2⟩ := ofInt_exact (f := f) ha hf
  obtain ⟨hb1, hb2⟩ := ofInt_exact (f := f) hb hf
  rw [mul_eq_round ha1 hb1, ha2, hb2]
  have := round_exact_q (f := f) ((ofInt (f := f) a).sign ^^ (ofInt (f := f) b).sign) (e := 0) hab
    f.qmin_le_zero (by
      rw [zpow_zero, _root_.mul_one]
      calc |((a * b : ℤ) : ℚ)| ≤ 2 ^ (f.prec : ℤ) := by rw [zpow_natCast]; exact_mod_cast hab
        _ < 2 ^ (f.emax + 1) := two_zpow_lt (by omega))
  simpa using this.2

end FP

end IEEE754
