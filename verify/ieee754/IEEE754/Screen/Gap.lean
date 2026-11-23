/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Side
import IEEE754.Compare

/-!
# decide-once's interval screens (items 6, 8, 10)

- `gap_sound`: for `lo` and `hi` each one rounding off exact `X` and `Y`, if
  `|lo − hi| > 1e-12·(1 + |lo| + |hi|)` in f64, then `lo − hi` has the sign of `X − Y`, which is
  nonzero. No underflow hypothesis is needed, because of the `1 +`.
- `snap_gap_sound` (item 10) and `region_sound` (item 8) instantiate it with the f64 `max`/`min`.
  For item 8, rounding is monotone, so `max(alo, blo)` is the rounding of the exact `max`.
- `wall_sound` (item 6): if `|c_f − n·x| > 1e-9·(|c_f| + |n·x|)` in f64, the exact offset is not
  `n·x`.
-/

namespace IEEE754.Screen

open FP

def E12 : F64 := .fin false 4951760157141521 (-92) (by decide)
theorem E12_bits : E12.toBits = 0x3d719799812dea11 := by decide
theorem E12_val : E12.val = (4951760157141521 / 2 ^ 92 : ℚ) := by simp only [E12, val, sgn]; norm_num
def E9 : F64 := .fin false 4835703278458517 (-82) (by decide)
theorem E9_bits : E9.toBits = 0x3e112e0be826d695 := by decide
theorem E9_val : E9.val = (4835703278458517 / 2 ^ 82 : ℚ) := by simp only [E9, val, sgn]; norm_num

/-- Rust's `f64::max`/`min` on finite operands (equal values: either, the value is the same). -/
def fmax (a b : F64) : F64 := if a.val < b.val then b else a
def fmin (a b : F64) : F64 := if b.val < a.val then b else a

lemma fmax_val (a b : F64) : (fmax a b).val = max a.val b.val := by
  unfold fmax; split_ifs with h
  · exact (max_eq_right h.le).symm
  · exact (max_eq_left (not_lt.mp h)).symm
lemma fmin_val (a b : F64) : (fmin a b).val = min a.val b.val := by
  unfold fmin; split_ifs with h
  · exact (min_eq_right h.le).symm
  · exact (min_eq_left (not_lt.mp h)).symm
lemma fmax_fin {a b : F64} (ha : a.isFinite) (hb : b.isFinite) : (fmax a b).isFinite := by
  unfold fmax; split_ifs <;> assumption
lemma fmin_fin {a b : F64} (ha : a.isFinite) (hb : b.isFinite) : (fmin a b).isFinite := by
  unfold fmin; split_ifs <;> assumption

/-- The literal `1.0`. -/
def F1 : F64 := .fin false 4503599627370496 (-52) (by decide)
theorem F1_val : F1.val = (1 : ℚ) := by simp only [F1, val, sgn]; norm_num

lemma η_small : η ≤ 1 / 10 ^ 50 :=
  le_trans (two_zpow_le (show (-1075 : ℤ) ≤ -200 by norm_num)) (by norm_num)

/-- `1e-12 * (1.0 + lo.abs() + hi.abs())`. -/
def gapTol (lo hi : F64) : F64 := mul E12 (add (add F1 lo.abs) hi.abs)

lemma gapTol_lo {lo hi : F64} (hl : lo.isFinite) (hh : hi.isFinite) (ht : (gapTol lo hi).isFinite) :
    4951760157141521 / 2 ^ 92 * (1 - 4 * ε) * (1 + |lo.val| + |hi.val|) - 3 * η ≤ (gapTol lo hi).val := by
  unfold gapTol at ht ⊢
  have fs := (isFinite_of_mul ht).2
  have fO : F1.isFinite := by decide
  have fla : lo.abs.isFinite := by simpa using hl
  have fha : hi.abs.isFinite := by simpa using hh
  have e := sum3_err fO fla fha fs
  simp only [FP.abs_val, abs_abs, F1_val, abs_one] at e
  have fE : E12.isFinite := by decide
  have em := mul_rel fE fs ht
  rw [E12_val] at em
  have hη := η_pos
  have hs : (1 - 3 * ε) * (1 + |lo.val| + |hi.val|) - 3 * η ≤ (add (add F1 lo.abs) hi.abs).val := by
    have := (abs_le.mp e).1; linarith
  have hs0 : 0 ≤ (add (add F1 lo.abs) hi.abs).val := by
    have := abs_nonneg lo.val; have := abs_nonneg hi.val; have := η_small; linarith
  rw [abs_of_nonneg (by positivity : (0 : ℚ) ≤ 4951760157141521 / 2 ^ 92 * (add (add F1 lo.abs) hi.abs).val)] at em
  have := (abs_le.mp em).1
  have := mul_le_mul_of_nonneg_left hs (by positivity : (0 : ℚ) ≤ 4951760157141521 / 2 ^ 92)
  have := abs_nonneg lo.val; have := abs_nonneg hi.val
  linarith

/-- **The gap screen.** -/
theorem gap_sound {lo hi : F64} {X Y : ℚ} (hl : lo.isFinite) (hh : hi.isFinite)
    (elo : |lo.val - X| ≤ ε * |X| + η) (ehi : |hi.val - Y| ≤ ε * |Y| + η)
    (hd : (sub lo hi).isFinite) (ht : (gapTol lo hi).isFinite)
    (hdec : (gapTol lo hi).val < |(sub lo hi).val|) :
    (0 < (sub lo hi).val → Y < X) ∧ ((sub lo hi).val < 0 → X < Y) := by
  have T := gapTol_lo hl hh ht
  have ed := sub_rel hl hh hd
  have hη := η_pos
  have hηs := η_small
  have hX : |X| ≤ 2 * |lo.val| + 2 * η := by
    have := abs_sub_abs_le_abs_sub X lo.val; rw [abs_sub_comm] at this
    have := abs_nonneg X; linarith
  have hY : |Y| ≤ 2 * |hi.val| + 2 * η := by
    have := abs_sub_abs_le_abs_sub Y hi.val; rw [abs_sub_comm] at this
    have := abs_nonneg Y; linarith
  have hlh : |lo.val - hi.val| ≤ |lo.val| + |hi.val| := abs_sub _ _
  have close : |(sub lo hi).val - (X - Y)| < |(sub lo hi).val| := by
    have k : (sub lo hi).val - (X - Y) = ((sub lo hi).val - (lo.val - hi.val)) + (lo.val - X) - (hi.val - Y) := by ring
    rw [k]
    have := abs_sub (((sub lo hi).val - (lo.val - hi.val)) + (lo.val - X)) (hi.val - Y)
    have := abs_add_le ((sub lo hi).val - (lo.val - hi.val)) (lo.val - X)
    have := abs_nonneg lo.val; have := abs_nonneg hi.val
    linarith
  obtain ⟨a, b⟩ := sign_of_close (B := 0) close
  exact ⟨fun h => by linarith [a h], fun h => by linarith [b h]⟩

/-- **Item 10**, snap's y/z gap: `lo = max(a.lo, b.lo) − l`, `hi = min(a.hi, b.hi) + l`. If
`lo − hi > 1e-12·(1 + |lo| + |hi|)`, then the exact grown boxes are disjoint on the axis. -/
theorem snap_gap_sound {alo blo ahi bhi l : F64} (h1 : alo.isFinite) (h2 : blo.isFinite) (h3 : ahi.isFinite)
    (h4 : bhi.isFinite) (hlf : l.isFinite)
    (hd : (sub (sub (fmax alo blo) l) (add (fmin ahi bhi) l)).isFinite)
    (ht : (gapTol (sub (fmax alo blo) l) (add (fmin ahi bhi) l)).isFinite)
    (hdec : (gapTol (sub (fmax alo blo) l) (add (fmin ahi bhi) l)).val <
      (sub (sub (fmax alo blo) l) (add (fmin ahi bhi) l)).val) :
    min ahi.val bhi.val + l.val < max alo.val blo.val - l.val := by
  have flo := (isFinite_of_sub hd).1
  have fhi := (isFinite_of_sub hd).2
  have elo := sub_rel (fmax_fin h1 h2) hlf flo
  have ehi := add_rel (fmin_fin h3 h4) hlf fhi
  rw [fmax_val] at elo; rw [fmin_val] at ehi
  have g := gap_sound flo fhi elo ehi hd ht (lt_of_lt_of_le hdec (le_abs_self _))
  have t0 : 0 ≤ (gapTol (sub (fmax alo blo) l) (add (fmin ahi bhi) l)).val := by
    have := gapTol_lo flo fhi ht
    have := η_small
    have := abs_nonneg (sub (fmax alo blo) l).val; have := abs_nonneg (add (fmin ahi bhi) l).val
    linarith
  exact g.1 (lt_of_le_of_lt t0 hdec)

lemma sub_mono {a b l : F64} (ha : a.isFinite) (hb : b.isFinite) (hl : l.isFinite)
    (fa : (sub a l).isFinite) (fb : (sub b l).isFinite) (h : a.val ≤ b.val) :
    (sub a l).val ≤ (sub b l).val := by
  have := round_mono (K := ℚ) binary64 (s₁ := a.sign && !l.sign) (s₂ := b.sign && !l.sign)
    (show a.val - l.val ≤ b.val - l.val by linarith)
  rw [← sub_eq_round ha hl, ← sub_eq_round hb hl] at this
  exact (le_iff fa fb).mp this

lemma add_mono {a b l : F64} (ha : a.isFinite) (hb : b.isFinite) (hl : l.isFinite)
    (fa : (add a l).isFinite) (fb : (add b l).isFinite) (h : a.val ≤ b.val) :
    (add a l).val ≤ (add b l).val := by
  have := round_mono (K := ℚ) binary64 (s₁ := a.sign && l.sign) (s₂ := b.sign && l.sign)
    (show a.val + l.val ≤ b.val + l.val by linarith)
  rw [← add_eq_round ha hl, ← add_eq_round hb hl] at this
  exact (le_iff fa fb).mp this

/-- **Item 8**, `region_f` on one axis. With `rlo = max(a.lo − l, b.lo − l)` and
`rhi = min(a.hi + l, b.hi + l)` in f64, suppose `|rlo − rhi| > 1e-12·(1 + |rlo| + |rhi|)`. Then the
f64 comparison `rlo > rhi` decides the exact one. -/
theorem region_sound {ax bx ay by' l : F64} (h1 : ax.isFinite) (h2 : bx.isFinite) (h3 : ay.isFinite)
    (h4 : by'.isFinite) (hlf : l.isFinite)
    (f1 : (sub ax l).isFinite) (f2 : (sub bx l).isFinite) (f3 : (add ay l).isFinite) (f4 : (add by' l).isFinite)
    (hd : (sub (fmax (sub ax l) (sub bx l)) (fmin (add ay l) (add by' l))).isFinite)
    (ht : (gapTol (fmax (sub ax l) (sub bx l)) (fmin (add ay l) (add by' l))).isFinite)
    (hdec : (gapTol (fmax (sub ax l) (sub bx l)) (fmin (add ay l) (add by' l))).val <
      |(sub (fmax (sub ax l) (sub bx l)) (fmin (add ay l) (add by' l))).val|) :
    ((fmin (add ay l) (add by' l)).val < (fmax (sub ax l) (sub bx l)).val →
        min (ay.val + l.val) (by'.val + l.val) < max (ax.val - l.val) (bx.val - l.val)) ∧
      ((fmax (sub ax l) (sub bx l)).val ≤ (fmin (add ay l) (add by' l)).val →
        max (ax.val - l.val) (bx.val - l.val) < min (ay.val + l.val) (by'.val + l.val)) := by
  set rlo := fmax (sub ax l) (sub bx l)
  set rhi := fmin (add ay l) (add by' l)
  have frlo : rlo.isFinite := fmax_fin f1 f2
  have frhi : rhi.isFinite := fmin_fin f3 f4
  -- `rlo` is one rounding of the exact max, `rhi` of the exact min (rounding is monotone)
  have elo : |rlo.val - max (ax.val - l.val) (bx.val - l.val)| ≤ ε * |max (ax.val - l.val) (bx.val - l.val)| + η := by
    simp only [rlo]; rw [fmax_val]
    rcases le_total ax.val bx.val with h | h
    · have := sub_mono h1 h2 hlf f1 f2 h
      rw [max_eq_right this, max_eq_right (by linarith)]; exact sub_rel h2 hlf f2
    · have := sub_mono h2 h1 hlf f2 f1 h
      rw [max_eq_left this, max_eq_left (by linarith)]; exact sub_rel h1 hlf f1
  have ehi : |rhi.val - min (ay.val + l.val) (by'.val + l.val)| ≤ ε * |min (ay.val + l.val) (by'.val + l.val)| + η := by
    simp only [rhi]; rw [fmin_val]
    rcases le_total ay.val by'.val with h | h
    · have := add_mono h3 h4 hlf f3 f4 h
      rw [min_eq_left this, min_eq_left (by linarith)]; exact add_rel h3 hlf f3
    · have := add_mono h4 h3 hlf f4 f3 h
      rw [min_eq_right this, min_eq_right (by linarith)]; exact add_rel h4 hlf f4
  have g := gap_sound frlo frhi elo ehi hd ht hdec
  have t0 : 0 ≤ (gapTol rlo rhi).val := by
    have := gapTol_lo frlo frhi ht; have := η_small
    have := abs_nonneg rlo.val; have := abs_nonneg rhi.val
    linarith
  have hd0 : (sub rlo rhi).val ≠ 0 := by
    intro h0; rw [h0, abs_zero] at hdec; linarith
  constructor
  · intro h
    have : 0 ≤ (sub rlo rhi).val := by
      rw [sub_eq_round frlo frhi]; exact round_val_nonneg binary64 (by linarith)
    exact g.1 (lt_of_le_of_ne this (Ne.symm hd0))
  · intro h
    have : (sub rlo rhi).val ≤ 0 := by
      rw [sub_eq_round frlo frhi]; exact round_val_nonpos binary64 (by linarith)
    exact g.2 (lt_of_le_of_ne this hd0)

/-- **Item 6**, the region wall clash: an axis-aligned face plane `n_a·x_a = c` (`c_f` one rounding
of the exact `C`) against the wall `x_a = x`. If `|c_f − n_a·x| > 1e-9·(|c_f| + |n_a·x|)` in f64,
then `C ≠ n_a·x`. No underflow: `n_a·x` is `0` or at least `2^-900`. -/
theorem wall_sound {cf na x : F64} {C : ℚ} (hcf : cf.isFinite) (hna : na.isFinite) (hx : x.isFinite)
    (hc : |cf.val - C| ≤ 2 * ε * |C|)
    (hu : na.val * x.val = 0 ∨ (2 : ℚ) ^ (-900 : ℤ) ≤ |na.val * x.val|)
    (fq : (mul na x).isFinite) (fo : (sub cf (mul na x)).isFinite)
    (fb : (mul E9 (add cf.abs (mul na x).abs)).isFinite)
    (hdec : (mul E9 (add cf.abs (mul na x).abs)).val < (sub cf (mul na x)).abs.val) :
    C ≠ na.val * x.val := by
  intro hC
  rw [FP.abs_val] at hdec
  have fE : E9.isFinite := by decide
  have fs := (isFinite_of_mul fb).2
  have fca : cf.abs.isFinite := by simpa using hcf
  have fqa : (mul na x).abs.isFinite := by simpa using fq
  have b0 : 0 ≤ (mul E9 (add cf.abs (mul na x).abs)).val := by
    rw [mul_eq_round fE fs]
    refine round_val_nonneg binary64 (mul_nonneg (by rw [E9_val]; positivity) ?_)
    rw [add_eq_round fca fqa]
    exact round_val_nonneg binary64 (by rw [FP.abs_val, FP.abs_val]; positivity)
  set P := na.val * x.val
  rcases hu with h0 | hb
  · -- everything is an exact zero
    rw [hC, h0, abs_zero, mul_zero] at hc
    have hcf0 : cf.val = 0 := by have := abs_nonneg (cf.val - 0); simp at hc; linarith [abs_nonneg cf.val]
    have hq0 : (mul na x).val = 0 := by
      rw [mul_eq_round hna hx, show na.val * x.val = 0 from h0, round_zero]; rfl
    have : (sub cf (mul na x)).val = 0 := by
      rw [sub_eq_round hcf fq, hcf0, hq0, _root_.sub_self, round_zero]; rfl
    rw [this, abs_zero] at hdec; linarith
  · have hη := η_pos
    have hκ : (2 : ℚ) ^ (-175 : ℤ) ≤ 1 / 10 ^ 50 := by norm_num
    have hηe : η = 2 ^ (-900 : ℤ) * 2 ^ (-175 : ℤ) := by rw [← two_zpow_add]; norm_num
    have hηP : η ≤ 1 / 10 ^ 50 * |P| := by
      rw [hηe]; exact le_trans (mul_le_mul_of_nonneg_right hb (two_zpow_pos _).le)
        (by rw [_root_.mul_comm]; exact mul_le_mul_of_nonneg_right hκ (by positivity))
    rw [hC] at hc
    obtain ⟨a, b⟩ := exact_rel hc
    have eq := mul_rel hna hx fq
    have eo := sub_rel hcf fq fo
    have hqP := abs_le.mp eq
    -- the offset is small
    have off : |(sub cf (mul na x)).val| ≤ 4 * ε * |P| + 3 * η := by
      have h1 : |cf.val - (mul na x).val| ≤ 3 * ε * |P| + η := by
        have k : cf.val - (mul na x).val = (cf.val - P) - ((mul na x).val - P) := by ring
        rw [k]; have := abs_sub (cf.val - P) ((mul na x).val - P)
        have : |cf.val - P| ≤ 2 * ε * |P| := hc
        linarith
      have := abs_sub_abs_le_abs_sub (sub cf (mul na x)).val (cf.val - (mul na x).val)
      have : 1 / 9007199254740992 * |cf.val - (mul na x).val| ≤ 1 / 9007199254740992 * (3 * ε * |P| + η) :=
        mul_le_mul_of_nonneg_left h1 (by norm_num)
      linarith
    -- the bound is large
    have es := add_rel fca fqa fs
    rw [FP.abs_val, FP.abs_val] at es
    have hqa : |(mul na x).val| ≥ |P| - (ε * |P| + η) := by
      have := abs_sub_abs_le_abs_sub P (mul na x).val; rw [abs_sub_comm] at this; linarith
    have hca : |cf.val| ≥ |P| - 2 * ε * |P| := by
      have := abs_sub_abs_le_abs_sub P cf.val; rw [abs_sub_comm] at this
      have : |cf.val - P| ≤ 2 * ε * |P| := hc
      linarith
    have hsum0 : 0 ≤ |cf.val| + |(mul na x).val| := by positivity
    rw [abs_of_nonneg hsum0] at es
    have s_lo := (abs_le.mp es).1
    have eb := mul_rel fE fs fb
    rw [E9_val] at eb
    have hs0 : 0 ≤ (add cf.abs (mul na x).abs).val := by linarith
    rw [abs_of_nonneg (by positivity : (0 : ℚ) ≤ 4835703278458517 / 2 ^ 82 * (add cf.abs (mul na x).abs).val)] at eb
    have b_lo := (abs_le.mp eb).1
    have := le_abs_self (sub cf (mul na x)).val
    have := abs_nonneg P
    linarith

end IEEE754.Screen
