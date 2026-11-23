/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.RoundProps

/-!
# Theorems about the operations

* Each operation on finite operands is `round` of its exact rational result
  (`add_eq_round`, `sub_eq_round`, `mul_eq_round`, `div_eq_round`, `fma_eq_round`).
* Their error: relative `2^-prec` outside the subnormal range (`add_rel_err` …), absolute
  `2^(qmin-1)` inside it (`add_abs_err` …).
* Sterbenz's lemma (`sterbenz`), exactness of small integers (`ofInt_exact`, `add_int_exact`,
  `mul_int_exact`), identities (`mul_one`, `add_neg_zero`, `sub_zero`, `div_one`, …).
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {f : Format}

/-! ## Values -/

lemma round_val_q (zs : Bool) {s : Bool} {m : ℕ} {e : ℤ} (h : Canon f m e) :
    round f zs (FP.fin s m e h).val = .fin s m e h := by
  simpa using round_val (K := ℚ) f zs h

lemma round_exact_q (zs : Bool) {k e : ℤ} (hk : |k| ≤ 2 ^ f.prec) (he : f.qmin ≤ e)
    (hlt : |(k : ℚ) * 2 ^ e| < 2 ^ (f.emax + 1)) :
    (round f zs ((k : ℚ) * 2 ^ e)).isFinite ∧ (round f zs ((k : ℚ) * 2 ^ e)).val = k * 2 ^ e := by
  simpa using round_exact (K := ℚ) f zs hk he hlt

namespace FP

@[simp] lemma val_zero (s : Bool) : (zero s : FP f).val = 0 := rfl
@[simp] lemma val_inf (s : Bool) : (inf s : FP f).val = 0 := rfl
@[simp] lemma val_nan : (nan : FP f).val = 0 := rfl

@[simp] lemma val_neg (x : FP f) : x.neg.val = -x.val := by
  cases x with
  | fin s m e h => cases s <;> simp [neg, val, sgn]
  | _ => simp [neg]

@[simp] lemma isFinite_neg (x : FP f) : x.neg.isFinite = x.isFinite := by cases x <;> rfl

@[simp] lemma sign_neg {x : FP f} (hx : x.isNaN = false) : x.neg.sign = !x.sign := by
  cases x <;> simp_all [neg, sign, isNaN]

@[simp] lemma neg_neg (x : FP f) : x.neg.neg = x := by
  cases x <;> simp [neg]

lemma val_eq_zero_iff {x : FP f} (hx : x.isFinite) : x.val = 0 ↔ x.isZero := by
  cases x with
  | fin s m e h =>
    cases s <;> simp [val, sgn, isZero, Nat.pos_iff_ne_zero.mp h.1, (zpow_pos (by norm_num : (0:ℚ) < 2) e).ne']
  | _ => simp_all [isFinite, isZero]

lemma abs_val (x : FP f) : x.abs.val = |x.val| := by
  cases x with
  | fin s m e h =>
    have : (0 : ℚ) ≤ m * 2 ^ e := (mul_pos (by exact_mod_cast h.1) (zpow_pos (by norm_num) e)).le
    cases s <;> simp [FP.abs, val, sgn, abs_of_nonneg this]
  | _ => simp [FP.abs]

/-- Every finite value is `k · 2^e` with `|k| < 2^prec` and `e ≥ qmin`. -/
lemma val_repr {x : FP f} (hx : x.isFinite) :
    ∃ k e : ℤ, |k| < 2 ^ f.prec ∧ f.qmin ≤ e ∧ x.val = k * 2 ^ e ∧ |x.val| = |k| * 2 ^ e := by
  cases x with
  | zero s => exact ⟨0, f.qmin, by simp, le_rfl, by simp, by simp⟩
  | fin s m e h =>
    obtain ⟨hm0, hmp, he1, _, _⟩ := h
    have hpos : (0 : ℚ) < 2 ^ e := zpow_pos (by norm_num) e
    refine ⟨if s then -(m : ℤ) else m, e, ?_, he1, ?_, ?_⟩
    · cases s <;> simp <;> exact_mod_cast hmp
    · cases s <;> simp [val, sgn]
    · cases s <;> simp [val, sgn, abs_mul, abs_of_pos hpos]
  | _ => simp [isFinite] at hx

/-- A finite value is below the overflow threshold `2^(emax+1)`. -/
lemma abs_val_lt {x : FP f} (hx : x.isFinite) : |x.val| < 2 ^ (f.emax + 1) := by
  cases x with
  | zero s => simp; exact two_zpow_pos _
  | fin s m e h =>
    obtain ⟨_, hmp, _, he2, _⟩ := h
    have hpos : (0 : ℚ) < 2 ^ e := zpow_pos (by norm_num) e
    have : |(fin s m e ⟨‹_›, hmp, ‹_›, he2, ‹_›⟩ : FP f).val| = m * 2 ^ e := by
      cases s <;> simp [val, sgn, abs_mul, abs_of_pos hpos]
    rw [this, ← f.qmax_add_prec, add_comm, two_zpow_add]
    calc (m : ℚ) * 2 ^ e < 2 ^ (f.prec : ℤ) * 2 ^ e := by
          apply mul_lt_mul_of_pos_right _ hpos
          rw [← cast_two_pow]; exact_mod_cast hmp
      _ ≤ 2 ^ (f.prec : ℤ) * 2 ^ f.qmax :=
          mul_le_mul_of_nonneg_left (two_zpow_le he2) (two_zpow_pos _).le
  | _ => simp [isFinite] at hx

/-- `2^e'` is an integer multiple of `2^e` when `e ≤ e'`. -/
lemma two_zpow_dvd {e e' : ℤ} (h : e ≤ e') : ∃ p : ℤ, (2 : ℚ) ^ e' = p * 2 ^ e := by
  refine ⟨2 ^ (e' - e).toNat, ?_⟩
  push_cast; rw [← zpow_natCast, Int.toNat_of_nonneg (by omega), ← two_zpow_add]; congr 1; ring

end FP

/-! ## Each operation is the rounding of its exact result -/

/-- Finite operands: the operation is not NaN, not infinite. -/
private lemma fin_cases {x : FP f} (hx : x.isFinite) : (∃ s, x = .zero s) ∨ (∃ s m e h, x = .fin s m e h) := by
  cases x <;> simp_all [isFinite]

theorem add_eq_round {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) :
    add x y = round f (x.sign && y.sign) (x.val + y.val) := by
  rcases fin_cases hx with ⟨s, rfl⟩ | ⟨s, m, e, h, rfl⟩ <;>
  rcases fin_cases hy with ⟨s', rfl⟩ | ⟨s', m', e', h', rfl⟩ <;> rfl

theorem sub_eq_round {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) :
    sub x y = round f (x.sign && !y.sign) (x.val - y.val) := by
  have hny : y.isNaN = false := by cases y <;> simp_all [isFinite, isNaN]
  unfold sub
  rw [add_eq_round hx (by simpa using hy), sign_neg hny, val_neg, sub_eq_add_neg]

theorem mul_eq_round {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) :
    mul x y = round f (x.sign ^^ y.sign) (x.val * y.val) := by
  rcases fin_cases hx with ⟨s, rfl⟩ | ⟨s, m, e, h, rfl⟩ <;>
  rcases fin_cases hy with ⟨s', rfl⟩ | ⟨s', m', e', h', rfl⟩ <;> rfl

theorem div_eq_round {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hy0 : y.val ≠ 0) :
    div x y = round f (x.sign ^^ y.sign) (x.val / y.val) := by
  rcases fin_cases hx with ⟨s, rfl⟩ | ⟨s, m, e, h, rfl⟩ <;>
  rcases fin_cases hy with ⟨s', rfl⟩ | ⟨s', m', e', h', rfl⟩ <;> simp_all [div, isZero]

theorem fma_eq_round {x y z : FP f} (hx : x.isFinite) (hy : y.isFinite) (hz : z.isFinite) :
    fma x y z = round f ((x.sign ^^ y.sign) && z.sign) (x.val * y.val + z.val) := by
  have a : x.isNaN = false ∧ x.isInf = false := by cases x <;> simp_all [isFinite, isNaN, isInf]
  have b : y.isNaN = false ∧ y.isInf = false := by cases y <;> simp_all [isFinite, isNaN, isInf]
  have c : z.isNaN = false ∧ z.isInf = false := by cases z <;> simp_all [isFinite, isNaN, isInf]
  simp [fma, a, b, c]

/-! ## Error bounds of the operations -/

/-- The rounding of an exact result `q`: relative error outside the subnormal range. -/
theorem round_rel_err_q {zs : Bool} {q : ℚ} (hfin : (round f zs q).isFinite) (hq : 2 ^ f.emin ≤ |q|) :
    |(round f zs q).val - q| ≤ 2 ^ (-(f.prec : ℤ)) * |q| := by
  simpa using round_rel_err (K := ℚ) f hfin hq

theorem round_abs_err_q {zs : Bool} {q : ℚ} (hq : |q| < 2 ^ f.emin) :
    (round f zs q).isFinite ∧ |(round f zs q).val - q| ≤ 2 ^ (f.qmin - 1) := by
  simpa using round_abs_err (K := ℚ) f (zs := zs) hq

theorem add_rel_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (add x y).isFinite)
    (hn : 2 ^ f.emin ≤ |x.val + y.val|) :
    |(add x y).val - (x.val + y.val)| ≤ 2 ^ (-(f.prec : ℤ)) * |x.val + y.val| := by
  rw [add_eq_round hx hy] at hfin ⊢; exact round_rel_err_q hfin hn

theorem sub_rel_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (sub x y).isFinite)
    (hn : 2 ^ f.emin ≤ |x.val - y.val|) :
    |(sub x y).val - (x.val - y.val)| ≤ 2 ^ (-(f.prec : ℤ)) * |x.val - y.val| := by
  rw [sub_eq_round hx hy] at hfin ⊢; exact round_rel_err_q hfin hn

theorem mul_rel_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hfin : (mul x y).isFinite)
    (hn : 2 ^ f.emin ≤ |x.val * y.val|) :
    |(mul x y).val - x.val * y.val| ≤ 2 ^ (-(f.prec : ℤ)) * |x.val * y.val| := by
  rw [mul_eq_round hx hy] at hfin ⊢; exact round_rel_err_q hfin hn

theorem div_rel_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hy0 : y.val ≠ 0)
    (hfin : (div x y).isFinite) (hn : 2 ^ f.emin ≤ |x.val / y.val|) :
    |(div x y).val - x.val / y.val| ≤ 2 ^ (-(f.prec : ℤ)) * |x.val / y.val| := by
  rw [div_eq_round hx hy hy0] at hfin ⊢; exact round_rel_err_q hfin hn

theorem fma_rel_err {x y z : FP f} (hx : x.isFinite) (hy : y.isFinite) (hz : z.isFinite)
    (hfin : (fma x y z).isFinite) (hn : 2 ^ f.emin ≤ |x.val * y.val + z.val|) :
    |(fma x y z).val - (x.val * y.val + z.val)| ≤ 2 ^ (-(f.prec : ℤ)) * |x.val * y.val + z.val| := by
  rw [fma_eq_round hx hy hz] at hfin ⊢; exact round_rel_err_q hfin hn

theorem add_abs_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hn : |x.val + y.val| < 2 ^ f.emin) :
    (add x y).isFinite ∧ |(add x y).val - (x.val + y.val)| ≤ 2 ^ (f.qmin - 1) := by
  rw [add_eq_round hx hy]; exact round_abs_err_q hn

theorem sub_abs_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hn : |x.val - y.val| < 2 ^ f.emin) :
    (sub x y).isFinite ∧ |(sub x y).val - (x.val - y.val)| ≤ 2 ^ (f.qmin - 1) := by
  rw [sub_eq_round hx hy]; exact round_abs_err_q hn

theorem mul_abs_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hn : |x.val * y.val| < 2 ^ f.emin) :
    (mul x y).isFinite ∧ |(mul x y).val - x.val * y.val| ≤ 2 ^ (f.qmin - 1) := by
  rw [mul_eq_round hx hy]; exact round_abs_err_q hn

theorem div_abs_err {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (hy0 : y.val ≠ 0)
    (hn : |x.val / y.val| < 2 ^ f.emin) :
    (div x y).isFinite ∧ |(div x y).val - x.val / y.val| ≤ 2 ^ (f.qmin - 1) := by
  rw [div_eq_round hx hy hy0]; exact round_abs_err_q hn

theorem fma_abs_err {x y z : FP f} (hx : x.isFinite) (hy : y.isFinite) (hz : z.isFinite)
    (hn : |x.val * y.val + z.val| < 2 ^ f.emin) :
    (fma x y z).isFinite ∧ |(fma x y z).val - (x.val * y.val + z.val)| ≤ 2 ^ (f.qmin - 1) := by
  rw [fma_eq_round hx hy hz]; exact round_abs_err_q hn

/-- In the subnormal range addition is exact: the sum of two multiples of `2^qmin` that is below
`2^emin` is representable. -/
theorem add_exact_of_small {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (hn : |x.val + y.val| < 2 ^ f.emin) : (add x y).val = x.val + y.val := by
  obtain ⟨kx, ex, hkx, hex, hvx, _⟩ := FP.val_repr hx
  obtain ⟨ky, ey, hky, hey, hvy, _⟩ := FP.val_repr hy
  obtain ⟨px, hpx⟩ := FP.two_zpow_dvd hex
  obtain ⟨py, hpy⟩ := FP.two_zpow_dvd hey
  rw [add_eq_round hx hy]
  set k : ℤ := kx * px + ky * py
  have hk : x.val + y.val = (k : ℚ) * 2 ^ f.qmin := by
    rw [hvx, hvy, hpx, hpy]; simp only [k]; push_cast; ring
  rw [hk] at hn ⊢
  have hpos := two_zpow_pos (K := ℚ) f.qmin
  rw [abs_mul, abs_of_pos hpos] at hn
  have hkb : |k| ≤ 2 ^ f.prec := by
    have h1 : |(k : ℚ)| * 2 ^ f.qmin < 2 ^ (f.prec : ℤ) * 2 ^ f.qmin := by
      refine lt_of_lt_of_le hn ?_
      rw [← two_zpow_add]; apply two_zpow_le; have := f.qmin_add_prec; omega
    have h2 := lt_of_mul_lt_mul_right h1 hpos.le
    rw [← cast_two_pow] at h2
    have : |k| < 2 ^ f.prec := by exact_mod_cast h2
    exact this.le
  have hlt : |(k : ℚ) * 2 ^ f.qmin| < 2 ^ (f.emax + 1) := by
    rw [abs_mul, abs_of_pos hpos]
    exact lt_trans hn (two_zpow_lt (by have := f.one_le_emax; unfold Format.emin; omega))
  exact (round_exact_q _ hkb le_rfl hlt).2

/-! ## Sterbenz's lemma -/

/-- **Sterbenz's lemma**: if `y/2 ≤ x ≤ 2y`, then `x - y` is computed exactly. -/
theorem sterbenz {x y : FP f} (hx : x.isFinite) (hy : y.isFinite)
    (h1 : y.val / 2 ≤ x.val) (h2 : x.val ≤ 2 * y.val) :
    (sub x y).isFinite ∧ (sub x y).val = x.val - y.val := by
  have hy0 : 0 ≤ y.val := by linarith
  have hx0 : 0 ≤ x.val := by linarith
  obtain ⟨kx, ex, hkx, hex, hvx, hax⟩ := FP.val_repr hx
  obtain ⟨ky, ey, hky, hey, hvy, hay⟩ := FP.val_repr hy
  rw [sub_eq_round hx hy]
  -- `|x - y| ≤ min x y`
  have hxy1 : |x.val - y.val| ≤ |x.val| := by
    rw [abs_of_nonneg hx0, abs_le]; constructor <;> linarith
  have hxy2 : |x.val - y.val| ≤ |y.val| := by
    rw [abs_of_nonneg hy0, abs_le]; constructor <;> linarith
  -- `x - y = k · 2^e` with `e = min ex ey`
  set e := min ex ey
  obtain ⟨px, hpx⟩ := FP.two_zpow_dvd (min_le_left ex ey)
  obtain ⟨py, hpy⟩ := FP.two_zpow_dvd (min_le_right ex ey)
  set k : ℤ := kx * px - ky * py
  have hk : x.val - y.val = (k : ℚ) * 2 ^ e := by
    rw [hvx, hvy, hpx, hpy]; simp only [k]; push_cast; ring
  have hpe : (0 : ℚ) < 2 ^ e := two_zpow_pos e
  -- `|k| ≤ 2^prec`: compare with whichever of `x`, `y` has the exponent `e`
  have hkb : |k| ≤ 2 ^ f.prec := by
    have : |(k : ℚ)| * 2 ^ e ≤ ((2 ^ f.prec : ℤ) : ℚ) * 2 ^ e := by
      rw [← abs_of_pos hpe, ← abs_mul, ← hk, abs_of_pos hpe]
      rcases min_choice ex ey with he | he
      · refine le_trans hxy1 ?_
        rw [hax, show e = ex from he]
        exact mul_le_mul_of_nonneg_right (by exact_mod_cast hkx.le) (two_zpow_pos _).le
      · refine le_trans hxy2 ?_
        rw [hay, show e = ey from he]
        exact mul_le_mul_of_nonneg_right (by exact_mod_cast hky.le) (two_zpow_pos _).le
    exact_mod_cast le_of_mul_le_mul_right this hpe
  have hlt : |(k : ℚ) * 2 ^ e| < 2 ^ (f.emax + 1) := by
    rw [← hk]; exact lt_of_le_of_lt hxy1 (FP.abs_val_lt hx)
  rw [hk]
  exact round_exact_q _ hkb (le_min hex hey) hlt

end IEEE754
