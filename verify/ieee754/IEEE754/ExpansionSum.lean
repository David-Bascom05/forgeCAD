/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Expansion

/-!
# Toward Fast-Expansion-Sum and Scale-Expansion: the rounding facts (Shewchuk 1997, §2.2–2.3)

The theorems on Fast-Expansion-Sum (Shewchuk's Theorem 13) and Scale-Expansion (Theorem 19,
Corollary 22) need two facts about one rounding that hold for round to nearest, ties to even:

* `round_tie_even`: a rounding exactly half a quantum away from the exact value lands on an
  even multiple of the quantum (the tie went to the even significand);
* `err_large`: a nonzero rounding error `t` of `a ⊕ b` means the quantum `q` is above the least
  one, `|t| ≤ 2^(q−1)`, and `|a + b| ≥ 2^(q+p−1) + |t|` (the exact sum is past the binade's
  first float by at least the error).

From them, for TwoSum's pair `(s, t)` (Shewchuk's Corollaries 8 and 9, in grid form):
* `two_sum_cor8a`: `|t| ≥ 2^i` gives `|a + b| ≥ 2^i (2^p + 1)`;
* `two_sum_cor8b`: `|t| > 2^i` gives `|a + b| > 2^i (2^(p+1) + 1)`;
* `two_sum_cor9`: `|t| ≥ 2^i` gives `s` a multiple of `2^(i+2)` (`s` and `t` nonadjacent).
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {f : Format}

/-! ## Ties go to the even significand -/

lemma rne_tie {y : ℚ} (hy : 0 ≤ y) (h : |(rne y : ℚ) - y| = 1 / 2) : rne y % 2 = 0 := by
  have hfl := Nat.floor_le hy
  have hlt := Nat.lt_floor_add_one y
  unfold rne at h ⊢
  by_cases h1 : y < ⌊y⌋₊ + 1 / 2
  · rw [if_pos h1] at h
    rw [abs_of_nonpos (by linarith)] at h; linarith
  · rw [if_neg h1] at h ⊢
    by_cases h2 : (⌊y⌋₊ : ℚ) + 1 / 2 < y
    · rw [if_pos h2] at h
      push_cast at h
      rw [abs_of_pos (by linarith)] at h; linarith
    · rw [if_neg h2]
      split_ifs with h3 <;> omega

/-- **A tie rounds to an even multiple of the quantum.** -/
lemma round_tie_even {zs : Bool} {r : ℚ} (hr : r ≠ 0) (hfin : (round f zs r).isFinite)
    (h : |(round f zs r).val - r| = 2 ^ (qOf f r - 1)) : Mult (round f zs r).val (qOf f r + 1) := by
  have ha : 0 < |r| := abs_pos.mpr hr
  have hv := abs_round_val (K := ℚ) (f := f) hr hfin
  have he := round_val_abs (K := ℚ) f hr hfin
  simp only [Rat.cast_id] at hv he
  set q := (roundPos f |r|).2 with hq
  have hqq : qOf f r = q := rfl
  rw [hqq] at h ⊢
  set y := |r| / 2 ^ q with hy
  have hu : uround f |r| = (rne y : ℚ) * 2 ^ q := by simp only [uround, roundPos_fst, ← hq, hy]
  have hpos : (0 : ℚ) < 2 ^ q := two_zpow_pos q
  have hr' : |r| = y * 2 ^ q := by rw [hy]; field_simp
  have htie : |(rne y : ℚ) - y| = 1 / 2 := by
    rw [he, hu, hr', ← sub_mul, abs_mul, abs_of_pos hpos] at h
    have : (2 : ℚ) ^ (q - 1) = 1 / 2 * 2 ^ q := by
      rw [show q - 1 = q + (-1) by ring, two_zpow_add]; norm_num; ring
    rw [this] at h
    exact mul_right_cancel₀ hpos.ne' h
  obtain ⟨m, hm⟩ := Nat.dvd_of_mod_eq_zero (rne_tie (by positivity) htie)
  have habs : Mult |(round f zs r).val| (q + 1) :=
    ⟨m, by rw [hv, hu, hm, two_zpow_add]; push_cast; ring⟩
  rcases abs_choice (round f zs r).val with e | e
  · rw [e] at habs; exact habs
  · obtain ⟨n, hn⟩ := habs; exact ⟨-n, by rw [← neg_neg (round f zs r).val, ← e, hn]; push_cast; ring⟩

/-! ## A large rounding error means a large exact sum -/

/-- The exact value's distance past the first float of its binade bounds the error. -/
theorem err_large {zs : Bool} {r : ℚ} (hm : Mult r f.qmin) (hfin : (round f zs r).isFinite)
    (ht : (round f zs r).val ≠ r) :
    f.qmin < qOf f r ∧ |(round f zs r).val - r| ≤ 2 ^ (qOf f r - 1) ∧
      Mult (round f zs r).val (qOf f r) ∧
      2 ^ (qOf f r + f.prec - 1) + |(round f zs r).val - r| ≤ |r| := by
  have hr : r ≠ 0 := by intro h; subst h; apply ht; simp [round, val]
  obtain ⟨g1, g2⟩ := round_grid hr hfin
  set q := qOf f r
  have hq : f.qmin < q := by
    by_contra hc; push Not at hc
    have hq' : q = f.qmin := le_antisymm hc (qOf_ge_qmin r)
    -- on the least grid, `r` is representable: no error
    have hb : |r| ≤ 2 ^ f.prec * 2 ^ f.qmin := by
      have : |r| < 2 ^ (q + f.prec) := qOf_lt (f := f) hr
      rw [hq', _root_.add_comm] at this
      rw [← zpow_natCast, ← two_zpow_add]; linarith
    have hrep := rep_of_mult (f := f) hm le_rfl hb (abs_lt_of_round_finite hfin)
    exact ht (hrep zs).2
  refine ⟨hq, g2, g1, ?_⟩
  -- `|r| ≥ 2^(q+p−1)`: the quantum is above the least, so `r` is normal
  have hlog : q = Int.log 2 |r| - f.prec + 1 := by
    have : q = max (Int.log 2 |r| - f.prec + 1) f.qmin := roundPos_snd f |r|
    have hq2 : f.qmin < q := hq
    rw [max_def] at this; split_ifs at this <;> omega
  have ha : 0 < |r| := abs_pos.mpr hr
  have hlow : (2 : ℚ) ^ (q + f.prec - 1) ≤ |r| := by
    rw [show q + (f.prec : ℤ) - 1 = Int.log 2 |r| by omega]; exact (log_spec ha).1
  -- the float `± 2^(q+p−1)` is no closer than the rounding
  set F : ℚ := if 0 < r then 2 ^ (q + f.prec - 1) else -2 ^ (q + f.prec - 1) with hF
  have hFrep : Rep f F := by
    have hp2 := f.two_le_prec
    have hM : Mult ((2 : ℚ) ^ (q + f.prec - 1)) q := mult_pow (by omega)
    have hb : |(2 : ℚ) ^ (q + f.prec - 1)| ≤ 2 ^ f.prec * 2 ^ q := by
      rw [abs_of_pos (two_zpow_pos _), ← zpow_natCast, ← two_zpow_add]
      exact two_zpow_le (by omega)
    have ho : |(2 : ℚ) ^ (q + f.prec - 1)| < 2 ^ (f.emax + 1) := by
      rw [abs_of_pos (two_zpow_pos _)]; exact lt_of_le_of_lt hlow (abs_lt_of_round_finite hfin)
    split_ifs at hF
    · rw [hF]; exact rep_of_mult hM hq.le hb ho
    · rw [hF]; exact rep_neg (rep_of_mult hM hq.le hb ho)
  have hnear := round_nearest hfin (hFrep false).1
  rw [(hFrep false).2] at hnear
  have hdist : |F - r| = |r| - 2 ^ (q + f.prec - 1) := by
    rw [hF]; split_ifs with hpos
    · rw [abs_of_pos hpos, abs_sub_comm, abs_of_nonneg (by rw [abs_of_pos hpos] at hlow; linarith)]
    · push Not at hpos
      have hneg : r < 0 := lt_of_le_of_ne hpos hr
      rw [abs_of_neg hneg] at hlow ⊢
      rw [abs_of_nonneg (by linarith)]; ring
  linarith

/-! ## Corollaries 8 and 9, for TwoSum -/

/-- Every float is on the least grid. -/
lemma mult_qmin {x : FP f} (hx : x.isFinite) : Mult x.val f.qmin := by
  obtain ⟨e, he, hm, _⟩ := mult_val hx; exact mult_of_le hm he

section TwoSumCor

variable {a b : FP f} (ha : a.isFinite) (hb : b.isFinite) (hs : (add a b).isFinite)
  (hsum : (twoSum a b).1.val + (twoSum a b).2.val = a.val + b.val)

include ha hb hs hsum

/-- TwoSum's error, when nonzero, in terms of the rounding of `a + b`. -/
lemma two_sum_err_large (h0 : (twoSum a b).2.val ≠ 0) :
    f.qmin < qOf f (a.val + b.val) ∧ |(twoSum a b).2.val| ≤ 2 ^ (qOf f (a.val + b.val) - 1) ∧
      Mult (twoSum a b).1.val (qOf f (a.val + b.val)) ∧
      2 ^ (qOf f (a.val + b.val) + f.prec - 1) + |(twoSum a b).2.val| ≤ |a.val + b.val| := by
  have h1 : (twoSum a b).1 = add a b := rfl
  have hr : add a b = round f (a.sign && b.sign) (a.val + b.val) := add_eq_round ha hb
  have ht : (twoSum a b).2.val = (a.val + b.val) - (add a b).val := by rw [← h1]; linarith
  have hm := mult_add (mult_qmin ha) (mult_qmin hb)
  have hfin : (round f (a.sign && b.sign) (a.val + b.val)).isFinite := by rw [← hr]; exact hs
  have hne : (round f (a.sign && b.sign) (a.val + b.val)).val ≠ a.val + b.val := by
    intro e; apply h0; rw [ht, hr, e]; ring
  obtain ⟨q1, q2, q3, q4⟩ := err_large hm hfin hne
  have et : |(twoSum a b).2.val| = |(round f (a.sign && b.sign) (a.val + b.val)).val -
      (a.val + b.val)| := by rw [ht, hr, abs_sub_comm]
  refine ⟨q1, by rw [et]; exact q2, by rw [h1, hr]; exact q3, by rw [et]; exact q4⟩

/-- **Corollary 8(a)**: an error of at least `2^i` means `|a + b| ≥ 2^i (2^p + 1)`. -/
theorem two_sum_cor8a {i : ℤ} (h : 2 ^ i ≤ |(twoSum a b).2.val|) :
    2 ^ i * (2 ^ f.prec + 1) ≤ |a.val + b.val| := by
  have h0 : (twoSum a b).2.val ≠ 0 := by
    intro e; rw [e, abs_zero] at h; linarith [two_zpow_pos (K := ℚ) i]
  obtain ⟨-, q2, -, q4⟩ := two_sum_err_large ha hb hs hsum h0
  have hi : i ≤ qOf f (a.val + b.val) - 1 := two_zpow_le_iff.mp (le_trans h q2)
  have : (2 : ℚ) ^ i * 2 ^ f.prec ≤ 2 ^ (qOf f (a.val + b.val) + f.prec - 1) := by
    rw [← zpow_natCast, ← two_zpow_add]; exact two_zpow_le (by omega)
  nlinarith

/-- **Corollary 8(b)**: an error above `2^i` means `|a + b| > 2^i (2^(p+1) + 1)`. -/
theorem two_sum_cor8b {i : ℤ} (h : 2 ^ i < |(twoSum a b).2.val|) :
    2 ^ i * (2 ^ (f.prec + 1) + 1) < |a.val + b.val| := by
  have h0 : (twoSum a b).2.val ≠ 0 := by
    intro e; rw [e, abs_zero] at h; linarith [two_zpow_pos (K := ℚ) i]
  obtain ⟨-, q2, -, q4⟩ := two_sum_err_large ha hb hs hsum h0
  have hi : i < qOf f (a.val + b.val) - 1 := two_zpow_lt_iff.mp (lt_of_lt_of_le h q2)
  have : (2 : ℚ) ^ i * 2 ^ (f.prec + 1) ≤ 2 ^ (qOf f (a.val + b.val) + f.prec - 1) := by
    rw [← zpow_natCast, ← two_zpow_add]; exact two_zpow_le (by push_cast; omega)
  nlinarith

/-- **Corollary 9** (round to even): an error of at least `2^i` leaves the sum a multiple of
`2^(i+2)`: sum and error are nonadjacent. -/
theorem two_sum_cor9 {i : ℤ} (h : 2 ^ i ≤ |(twoSum a b).2.val|) :
    Mult (twoSum a b).1.val (i + 2) := by
  have h0 : (twoSum a b).2.val ≠ 0 := by
    intro e; rw [e, abs_zero] at h; linarith [two_zpow_pos (K := ℚ) i]
  obtain ⟨-, q2, q3, -⟩ := two_sum_err_large ha hb hs hsum h0
  set q := qOf f (a.val + b.val)
  have hi : i ≤ q - 1 := two_zpow_le_iff.mp (le_trans h q2)
  by_cases hq : i + 2 ≤ q
  · exact mult_of_le q3 hq
  · have hq' : q = i + 1 := by omega
    have h1 : (twoSum a b).1 = add a b := rfl
    have hr : add a b = round f (a.sign && b.sign) (a.val + b.val) := add_eq_round ha hb
    have ht : (twoSum a b).2.val = (a.val + b.val) - (add a b).val := by rw [← h1]; linarith
    have hfin : (round f (a.sign && b.sign) (a.val + b.val)).isFinite := by rw [← hr]; exact hs
    have hr0 : a.val + b.val ≠ 0 := by
      intro e; apply h0; rw [ht, hr, e]; simp [round, val]
    have et : |(twoSum a b).2.val| = |(round f (a.sign && b.sign) (a.val + b.val)).val -
        (a.val + b.val)| := by rw [ht, hr, abs_sub_comm]
    have htie : |(round f (a.sign && b.sign) (a.val + b.val)).val - (a.val + b.val)| =
        2 ^ (q - 1) := by
      rw [← et]; apply le_antisymm q2; rw [hq']; simpa using h
    have := round_tie_even hr0 hfin htie
    have e : qOf f (a.val + b.val) = i + 1 := hq'
    rw [e, show i + 1 + 1 = i + 2 by ring] at this
    rw [h1, hr]; exact this

end TwoSumCor

end IEEE754
