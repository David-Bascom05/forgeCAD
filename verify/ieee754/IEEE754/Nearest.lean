/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Computed

/-!
# Rounding gives a nearest representable value

`round_nearest`: a finite rounding of `r` is at least as close to `r` as any finite datum.
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {f : Format}

/-- `rne y` is a nearest integer to `y ≥ 0`. -/
lemma rne_nearest {y : ℚ} (hy : 0 ≤ y) (j : ℤ) : |(rne y : ℚ) - y| ≤ |(j : ℚ) - y| := by
  have h1 := Nat.floor_le hy
  have h2 := Nat.lt_floor_add_one y
  have he := rne_err hy
  set k := ⌊y⌋₊
  -- `j ≤ k` or `j ≥ k + 1`; the distance to `j` is at least that to `k` or to `k + 1`
  have hj : (j : ℚ) ≤ k ∨ (k : ℚ) + 1 ≤ j := by
    rcases le_or_gt j k with h | h
    · left; exact_mod_cast h
    · right; have : (k : ℤ) + 1 ≤ j := h; exact_mod_cast this
  have hk : (k : ℚ) - y ≤ 0 := by linarith
  have hk1 : 0 < (k : ℚ) + 1 - y := by linarith
  rcases rne_cases y with hr | hr
  · rw [hr] at he ⊢
    rw [abs_of_nonpos hk] at he ⊢
    rcases hj with hj | hj
    · rw [abs_of_nonpos (by linarith)]; linarith
    · rw [abs_of_nonneg (by linarith)]; linarith
  · rw [hr] at he ⊢
    push_cast at he ⊢
    rw [abs_of_pos hk1] at he ⊢
    rcases hj with hj | hj
    · rw [abs_of_nonpos (by linarith)]; linarith
    · rw [abs_of_nonneg (by linarith)]; linarith

/-- The unbounded rounding of `r > 0` is at least as close to `r` as any finite datum. -/
lemma uround_nearest {r : ℚ} (hr : 0 < r) {x : FP f} (hx : x.isFinite) :
    |uround f r - r| ≤ |x.val - r| := by
  set e := (roundPos f r).2
  set n := (roundPos f r).1
  have hpos : (0 : ℚ) < 2 ^ e := two_zpow_pos e
  set y := r / 2 ^ e
  have hy : 0 ≤ y := div_nonneg hr.le hpos.le
  have hn : n = rne y := roundPos_fst f r
  have hU : uround f r - r = ((n : ℚ) - y) * 2 ^ e := by
    simp only [uround, y]; rw [sub_mul, div_mul_cancel₀ _ hpos.ne']
  have hUa : |uround f r - r| = |(n : ℚ) - y| * 2 ^ e := by
    rw [hU, abs_mul, abs_of_pos hpos]
  -- against a multiple `j · 2^e`
  have mult : ∀ j : ℤ, |uround f r - r| ≤ |(j : ℚ) * 2 ^ e - r| := by
    intro j
    have : (j : ℚ) * 2 ^ e - r = ((j : ℚ) - y) * 2 ^ e := by
      simp only [y]; rw [sub_mul, div_mul_cancel₀ _ hpos.ne']
    rw [hUa, this, abs_mul, abs_of_pos hpos, hn]
    exact mul_le_mul_of_nonneg_right (rne_nearest hy j) hpos.le
  rcases le_or_gt x.val 0 with hxv | hxv
  · -- `x ≤ 0`: no closer than `0 = 0 · 2^e`
    have := mult 0
    simp only [Int.cast_zero, zero_mul, zero_sub, abs_neg, abs_of_pos hr] at this
    rw [abs_of_nonpos (show x.val - r ≤ 0 by linarith)]; linarith
  · cases x with
    | fin s m ex h =>
      have hs : s = false := by
        cases s
        · rfl
        · exfalso; simp [val, sgn] at hxv
          linarith [mul_nonneg (Nat.cast_nonneg (α := ℚ) m) (two_zpow_pos (K := ℚ) ex).le]
      subst hs
      have hxe : (fin false m ex h : FP f).val = m * 2 ^ ex := by simp [val, sgn]
      rw [hxe] at hxv ⊢
      obtain ⟨hm0, hmp, hex1, _, _⟩ := h
      rcases le_or_gt e ex with hle | hlt
      · have := mult ((m : ℤ) * 2 ^ (ex - e).toNat)
        have hj : ((((m : ℤ) * 2 ^ (ex - e).toNat : ℤ)) : ℚ) * 2 ^ e = m * 2 ^ ex := by
          push_cast
          rw [← zpow_natCast, Int.toNat_of_nonneg (by omega), mul_assoc, ← two_zpow_add]
          congr 2; ring
        rwa [hj] at this
      · -- a finer datum lies below the binade of `r`
        have hq : f.qmin < e := lt_of_le_of_lt hex1 hlt
        obtain ⟨hnn, he⟩ := roundPos_normal f hr hq
        have hE := (log_spec hr).1
        have hp1 := f.prec_sub_one
        -- `x ≤ 2^E - 2^(e-1)`, `U ≥ 2^E`, `|U - r| ≤ 2^(e-1)`
        have hx1 : (m : ℚ) * 2 ^ ex ≤ 2 ^ Int.log 2 r - 2 ^ (e - 1) := by
          have hm' : (m : ℚ) ≤ 2 ^ (f.prec : ℤ) - 1 := by
            have : m + 1 ≤ 2 ^ f.prec := hmp
            have : ((m + 1 : ℕ) : ℚ) ≤ ((2 ^ f.prec : ℕ) : ℚ) := by exact_mod_cast this
            push_cast at this; rw [zpow_natCast]; linarith
          calc (m : ℚ) * 2 ^ ex ≤ (2 ^ (f.prec : ℤ) - 1) * 2 ^ (e - 1) := by
                apply mul_le_mul hm' (two_zpow_le (by omega)) (two_zpow_pos _).le
                have : (1 : ℚ) ≤ 2 ^ (f.prec : ℤ) := one_le_zpow₀ (by norm_num) (by omega)
                linarith
            _ = 2 ^ Int.log 2 r - 2 ^ (e - 1) := by
                rw [sub_mul, _root_.one_mul, ← two_zpow_add]; congr 2; omega
        have hU1 : (2 : ℚ) ^ Int.log 2 r ≤ uround f r := by
          simp only [uround]
          rw [show Int.log 2 r = ((f.prec : ℤ) - 1) + e by omega, two_zpow_add]
          apply mul_le_mul_of_nonneg_right _ hpos.le
          rw [← hp1, ← cast_two_pow]; exact_mod_cast hnn
        have herr : |uround f r - r| ≤ 2 ^ (e - 1) := uround_err f hr
        have he1 := two_zpow_pos (K := ℚ) (e - 1)
        rw [abs_le] at herr
        rw [abs_of_neg (by linarith : (m : ℚ) * 2 ^ ex - r < 0)]
        rcases le_or_gt r (uround f r) with h1 | h1
        · rw [abs_of_nonneg (by linarith)]; linarith
        · rw [abs_of_neg (by linarith)]; linarith
    | _ => simp_all [isFinite, val]

/-- **Rounding gives a nearest finite datum.** -/
theorem round_nearest {zs : Bool} {r : ℚ} (hfin : (round f zs r).isFinite) {x : FP f}
    (hx : x.isFinite) : |(round f zs r).val - r| ≤ |x.val - r| := by
  by_cases h0 : r = 0
  · subst h0; simp [val]
  have := round_val_abs (K := ℚ) f h0 hfin
  simp only [Rat.cast_id] at this
  rw [this]
  rcases lt_or_gt_of_ne h0 with h | h
  · rw [abs_of_neg h]
    have := uround_nearest (f := f) (neg_pos.mpr h) (x := x.neg) (by simpa using hx)
    rw [val_neg] at this
    calc _ ≤ |-x.val - -r| := this
      _ = |x.val - r| := by rw [← abs_neg]; ring_nf
  · rw [abs_of_pos h]; exact uround_nearest h hx

end IEEE754
