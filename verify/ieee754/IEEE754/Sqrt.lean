/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Theorems
import Mathlib.Analysis.SpecialFunctions.Pow.Real

/-!
# Square root

`sqrt` is computed with an integer square root (`sqrtPos`). `sqrt_eq_round`: on a positive finite
operand it is `round` of the real square root, so the bounds about `round` hold for it
(`sqrt_rel_err`, `sqrt_mono`).
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable (f : Format)

lemma natFloor_ratCast {q : ℚ} (hq : 0 ≤ q) : ⌊(q : ℝ)⌋₊ = ⌊q⌋₊ := by
  rw [Nat.floor_eq_iff (by exact_mod_cast hq)]
  constructor
  · exact_mod_cast Nat.floor_le hq
  · exact_mod_cast Nat.lt_floor_add_one q

lemma natFloor_sqrt {N : ℝ} (hN : 0 ≤ N) : ⌊Real.sqrt N⌋₊ = Nat.sqrt ⌊N⌋₊ := by
  rw [Nat.floor_eq_iff (Real.sqrt_nonneg _)]
  have h1 := Nat.sqrt_le' ⌊N⌋₊
  have h2 := Nat.lt_succ_sqrt' ⌊N⌋₊
  have f1 := Nat.floor_le hN
  have f2 := Nat.lt_floor_add_one N
  constructor
  · rw [Real.le_sqrt (Nat.cast_nonneg _) hN]
    exact le_trans (by exact_mod_cast h1) f1
  · rw [Real.sqrt_lt' (by positivity)]
    have : (⌊N⌋₊ : ℝ) + 1 ≤ ((Nat.sqrt ⌊N⌋₊ : ℝ) + 1) ^ 2 := by
      have : ⌊N⌋₊ + 1 ≤ (Nat.sqrt ⌊N⌋₊ + 1) ^ 2 := h2
      exact_mod_cast this
    linarith

lemma sq_two_zpow (E : ℤ) : ((2 : ℝ) ^ E) ^ 2 = 2 ^ (2 * E) := by
  rw [← zpow_natCast, ← zpow_mul]; congr 1; push_cast; ring

/-- The integer square root algorithm computes the rounding of the real square root. -/
theorem sqrtPos_eq {q : ℚ} (hq : 0 < q) : sqrtPos f q = roundPos f (Real.sqrt (q : ℝ)) := by
  have hq' : (0 : ℝ) < q := by exact_mod_cast hq
  obtain ⟨hL1, hL2⟩ := log_spec hq
  have hL1' : (2 : ℝ) ^ Int.log 2 q ≤ q := by
    have := (Rat.cast_le (K := ℝ)).mpr hL1; push_cast at this; exact this
  have hL2' : (q : ℝ) < 2 ^ (Int.log 2 q + 1) := by
    have := (Rat.cast_lt (K := ℝ)).mpr hL2; push_cast at this; exact this
  have hE : Int.log 2 (Real.sqrt q) = Int.log 2 q / 2 := by
    apply log_eq
    · rw [Real.le_sqrt (two_zpow_pos _).le hq'.le, sq_two_zpow]
      exact le_trans (two_zpow_le (by omega)) hL1'
    · rw [Real.sqrt_lt' (two_zpow_pos _), sq_two_zpow]
      exact lt_of_lt_of_le hL2' (two_zpow_le (by omega))
  have hsnd : (roundPos f (Real.sqrt q)).2 = f.qexp (Int.log 2 q / 2) := by
    simp only [roundPos, hE]
  have hfst : (roundPos f (Real.sqrt q)).1 = rne (Real.sqrt q / 2 ^ f.qexp (Int.log 2 q / 2)) := by
    rw [roundPos_fst, hsnd]
  refine Prod.ext ?_ hsnd.symm
  rw [hfst]
  -- the scaled root is `√N`
  set e := f.qexp (Int.log 2 q / 2)
  set N : ℚ := q / 2 ^ (2 * e) with hNdef
  have hN0 : 0 ≤ N := div_nonneg hq.le (two_zpow_pos _).le
  have hN0' : (0 : ℝ) ≤ N := by exact_mod_cast hN0
  have hy : Real.sqrt q / 2 ^ e = Real.sqrt (N : ℝ) := by
    have : ((N : ℚ) : ℝ) = q / ((2 : ℝ) ^ e) ^ 2 := by
      rw [sq_two_zpow, hNdef]; push_cast; rfl
    rw [this, Real.sqrt_div' _ (sq_nonneg _), Real.sqrt_sq (two_zpow_pos _).le]
  rw [hy]
  have hfl : ⌊Real.sqrt (N : ℝ)⌋₊ = Nat.sqrt ⌊N⌋₊ := by
    rw [natFloor_sqrt hN0', natFloor_ratCast hN0]
  set k := Nat.sqrt ⌊N⌋₊
  have hk : (0 : ℝ) < k + 1 / 2 := by positivity
  have c1 : Real.sqrt (N : ℝ) < k + 1 / 2 ↔ N < ((k : ℚ) + 1 / 2) ^ 2 := by
    rw [Real.sqrt_lt' hk, ← Rat.cast_lt (K := ℝ)]; push_cast; rfl
  have c2 : (k : ℝ) + 1 / 2 < Real.sqrt (N : ℝ) ↔ ((k : ℚ) + 1 / 2) ^ 2 < N := by
    rw [Real.lt_sqrt hk.le, ← Rat.cast_lt (K := ℝ)]; push_cast; rfl
  unfold rne
  rw [hfl]
  show (if N < ((k : ℚ) + 1 / 2) ^ 2 then k else if ((k : ℚ) + 1 / 2) ^ 2 < N then k + 1
    else if k % 2 = 0 then k else k + 1) = _
  simp only [c1, c2]

/-- **Square root is correctly rounded**: on a positive finite operand it is `round` of the real
square root. -/
theorem sqrt_eq_round {m : ℕ} {e : ℤ} (h : Canon f m e) :
    (FP.fin false m e h).sqrt = round f false (Real.sqrt ((FP.fin false m e h).val : ℝ)) := by
  have hv : (FP.fin false m e h).val = (m : ℚ) * 2 ^ e := by simp [val, sgn]
  have hq : (0 : ℚ) < m * 2 ^ e := mul_pos (by exact_mod_cast h.1) (two_zpow_pos e)
  have hs : (0 : ℝ) < Real.sqrt ((m : ℚ) * 2 ^ e : ℚ) := Real.sqrt_pos.mpr (by exact_mod_cast hq)
  rw [hv]
  unfold round
  rw [if_neg hs.ne', if_pos hs, ← sqrtPos_eq f hq]
  rfl

/-- The relative error of square root (no underflow or overflow is possible for it, but the
bound is stated as for the other operations). -/
theorem sqrt_rel_err {m : ℕ} {e : ℤ} (h : Canon f m e)
    (hfin : (FP.fin false m e h).sqrt.isFinite)
    (hn : 2 ^ f.emin ≤ Real.sqrt ((FP.fin false m e h).val : ℝ)) :
    |((FP.fin false m e h).sqrt.val : ℝ) - Real.sqrt ((FP.fin false m e h).val : ℝ)| ≤
      2 ^ (-(f.prec : ℤ)) * Real.sqrt ((FP.fin false m e h).val : ℝ) := by
  rw [sqrt_eq_round] at hfin ⊢
  have := round_rel_err (K := ℝ) f hfin (by rwa [abs_of_nonneg (Real.sqrt_nonneg _)])
  rwa [abs_of_nonneg (Real.sqrt_nonneg _)] at this

/-- Square root is monotone on positive finite operands. -/
theorem sqrt_mono {m m' : ℕ} {e e' : ℤ} (h : Canon f m e) (h' : Canon f m' e')
    (hle : (FP.fin false m e h).val ≤ (FP.fin false m' e' h').val) :
    (FP.fin false m e h).sqrt.le (FP.fin false m' e' h').sqrt = true := by
  rw [sqrt_eq_round, sqrt_eq_round]
  exact round_mono (K := ℝ) f (Real.sqrt_le_sqrt (by exact_mod_cast hle))

end IEEE754
