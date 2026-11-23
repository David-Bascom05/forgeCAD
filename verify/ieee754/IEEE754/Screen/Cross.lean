/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Gap
import IEEE754.Screen.Track

/-!
# decide-once's `crosses_f` (item 4)

`crosses_f` screens the exact `crosses`, the perturbed ray-crossing test (`Ref.crossesYZ` in axes
`(s, t)`), in f64. The f64 inputs are `q_s, q_t, a_s, a_t, b_s, b_t`, each within `δ` of the exact
point's coordinate, with `δ ≤ 2·10^-13·scale`. Every input is at most `scale`, and `scale ≥ 1`.
This holds for `scale = 1 + max` over every coordinate of `q`, `a` and `b`, with `pf`'s
`1e-13·(1 + max|p_k|)` (item 5).

- `above_sound`: if `|x − q_t| > 1e-12·scale` in f64, then the exact `X − Q_t` is nonzero and has
  the f64 sign.
- `crosses_f_sound`: if `crosses_f` answers, the answer is the exact `crosses`. When the two
  `above`s differ, `|b_t − a_t| > 2·10^-12·scale`, so `d0`'s error is far inside `1e-12·mag`. Then
  the exact `d0 ≠ 0`, and the `(ε, ε²)` tie rule is never reached.
-/

namespace IEEE754.Screen

open FP

/-- The sign `Q::sgn` takes. -/
def sgnQ (x : ℚ) : ℤ := if 0 < x then 1 else if x < 0 then -1 else 0

/-- The exact `crosses`, a_s in `decide_once.rs`. -/
def crossesQ (Qs Qt As At Bs Bt : ℚ) : Bool :=
  let aa := decide (0 < At - Qt)
  let ab := decide (0 < Bt - Qt)
  if aa = ab then false else
    let dz := Bt - At
    let d0 := (Bs - As) * (Qt - At) - (Qs - As) * dz
    let sg := if sgnQ d0 ≠ 0 then sgnQ d0 else -sgnQ dz
    decide (0 < sg * sgnQ dz)

/-- `above`: `Some (d > 0)` when `|d| > E·scale`, `d = x − q_t`. -/
def aboveF (x q_t sc : F64) : Option Bool :=
  if (mul E12 sc).val < |(sub x q_t).val| then some (decide (0 < (sub x q_t).val)) else none

/-- `d0 = (b_s − a_s)(q_t − a_t) − (q_s − a_s)(b_t − a_t)`. -/
def d0F (q_s q_t a_s a_t b_s b_t : F64) : F64 :=
  sub (mul (sub b_s a_s) (sub q_t a_t)) (mul (sub q_s a_s) (sub b_t a_t))

/-- `mag = (|b_s − a_s| + |q_s − a_s| + |q_t − a_t| + |b_t − a_t|)·scale`. -/
def magX (q_s q_t a_s a_t b_s b_t sc : F64) : F64 :=
  mul (add (add (add (sub b_s a_s).abs (sub q_s a_s).abs) (sub q_t a_t).abs) (sub b_t a_t).abs) sc

/-- `crosses_f`, with its `scale` given. -/
def crossesF (q_s q_t a_s a_t b_s b_t sc : F64) : Option Bool :=
  match aboveF a_t q_t sc, aboveF b_t q_t sc with
  | some aa, some ab =>
    if aa = ab then some false else
      if (mul E12 (magX q_s q_t a_s a_t b_s b_t sc)).val < |(d0F q_s q_t a_s a_t b_s b_t).val| then
        some (decide (0 < (d0F q_s q_t a_s a_t b_s b_t).val) = ab)
      else none
  | _, _ => none

/-- A rounded difference of inputs `δ` off is within `2δ + 2ε·S` of the exact difference. -/
lemma diff_close {x y : F64} {X Y δ S : ℚ} (hx : x.isFinite) (hy : y.isFinite) (hf : (sub x y).isFinite)
    (ex : |x.val - X| ≤ δ) (ey : |y.val - Y| ≤ δ) (bx : |x.val| ≤ S) (byy : |y.val| ≤ S) :
    |(sub x y).val - (X - Y)| ≤ 2 * δ + 2 * ε * S ∧ |(sub x y).val - (x.val - y.val)| ≤ ε * |x.val - y.val| ∧
      |(sub x y).val| ≤ 3 * S := by
  have e := sub_rel0 hx hy hf
  have h1 : |x.val - y.val| ≤ 2 * S := le_trans (abs_sub _ _) (by linarith)
  have h2 : |(x.val - y.val) - (X - Y)| ≤ 2 * δ := by
    rw [show (x.val - y.val) - (X - Y) = (x.val - X) - (y.val - Y) by ring]
    exact le_trans (abs_sub _ _) (by linarith)
  refine ⟨?_, e, ?_⟩
  · have := abs_sub_le (sub x y).val (x.val - y.val) (X - Y)
    have : ε * |x.val - y.val| ≤ ε * (2 * S) := mul_le_mul_of_nonneg_left h1 (by norm_num)
    linarith
  · have := abs_sub_abs_le_abs_sub (sub x y).val (x.val - y.val)
    have : ε * |x.val - y.val| ≤ ε * (2 * S) := mul_le_mul_of_nonneg_left h1 (by norm_num)
    have : 0 ≤ S := le_trans (abs_nonneg _) bx
    linarith

/-- The `E·scale` tolerance, from below. -/
lemma tolE_lo {sc : F64} (hs : sc.isFinite) (hS : 1 ≤ sc.val) (hf : (mul E12 sc).isFinite) :
    (99 / 100) * (1 / 10 ^ 12) * sc.val ≤ (mul E12 sc).val := by
  have fE : E12.isFinite := by decide
  have em := mul_rel fE hs hf
  rw [E12_val, abs_of_nonneg (by positivity : (0 : ℚ) ≤ 4951760157141521 / 2 ^ 92 * sc.val)] at em
  have := (abs_le.mp em).1
  have hη := η_small
  have : (99 / 100) * (1 / 10 ^ 12) * sc.val + η ≤ (1 - ε) * (4951760157141521 / 2 ^ 92 * sc.val) := by
    nlinarith
  linarith

/-- **`above`.** -/
theorem above_sound {x q_t sc : F64} {X Qt δ : ℚ} (hx : x.isFinite) (hq : q_t.isFinite) (hs : sc.isFinite)
    (hS : 1 ≤ sc.val) (bx : |x.val| ≤ sc.val) (bq : |q_t.val| ≤ sc.val)
    (ex : |x.val - X| ≤ δ) (eq : |q_t.val - Qt| ≤ δ) (hδ : δ ≤ 2 / 10 ^ 13 * sc.val)
    (hfd : (sub x q_t).isFinite) (hft : (mul E12 sc).isFinite)
    (hdec : (mul E12 sc).val < |(sub x q_t).val|) :
    X - Qt ≠ 0 ∧ (0 < X - Qt ↔ 0 < (sub x q_t).val) := by
  obtain ⟨e1, -, -⟩ := diff_close hx hq hfd ex eq bx bq
  have t := tolE_lo hs hS hft
  have hd : |(sub x q_t).val - (X - Qt)| < |(sub x q_t).val| := by
    have : 2 * δ + 2 * ε * sc.val < (99 / 100) * (1 / 10 ^ 12) * sc.val := by nlinarith
    linarith
  constructor
  · intro h0; rw [h0, _root_.sub_zero] at hd; exact lt_irrefl _ hd
  · constructor
    · intro h; by_contra hn; push_neg at hn
      rw [abs_of_nonpos hn] at hd
      have := (abs_lt.mp hd).1
      nlinarith [neg_abs_le (sub x q_t).val, le_abs_self (sub x q_t).val]
    · intro h; by_contra hn; push_neg at hn
      rw [abs_of_pos h] at hd
      have := (abs_lt.mp hd).2
      linarith

/-- The bilinear error: `|uv − UV| ≤ (|u| + |v|)τ + τ²`. -/
lemma bil {u v U V τ : ℚ} (hu : |u - U| ≤ τ) (hv : |v - V| ≤ τ) :
    |u * v - U * V| ≤ |u| * τ + |v| * τ + τ * τ := by
  have hτ : 0 ≤ τ := le_trans (abs_nonneg _) hu
  rw [show u * v - U * V = u * (v - V) + v * (u - U) - (u - U) * (v - V) by ring]
  refine le_trans (abs_sub _ _) (add_le_add (le_trans (abs_add_le _ _) (add_le_add ?_ ?_)) ?_)
  · rw [abs_mul]; exact mul_le_mul_of_nonneg_left hv (abs_nonneg _)
  · rw [abs_mul]; exact mul_le_mul_of_nonneg_left hu (abs_nonneg _)
  · rw [abs_mul]; exact mul_le_mul hu hv (abs_nonneg _) hτ

/-- A rounded sum of nonnegatives is at least `(1 − ε)` of the sum. -/
lemma add_lo {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (hf : (add a b).isFinite)
    (h0 : 0 ≤ a.val + b.val) : (1 - ε) * (a.val + b.val) ≤ (add a b).val := by
  have e := add_rel0 ha hb hf
  rw [abs_of_nonneg h0] at e
  have := (abs_le.mp e).1; linarith

set_option maxHeartbeats 4000000 in
/-- **`d0`.** When the two f64 `above`s are plain and differ, `|d0 − D0| ≤ E·mag`. -/
theorem d0_close {q_s q_t a_s a_t b_s b_t sc : F64} {Qs Qt As At Bs Bt δ : ℚ}
    (hqs : q_s.isFinite) (hqt : q_t.isFinite) (has : a_s.isFinite) (hat : a_t.isFinite)
    (hbs : b_s.isFinite) (hbt : b_t.isFinite) (hs : sc.isFinite) (hS : 1 ≤ sc.val)
    (bqs : |q_s.val| ≤ sc.val) (bqt : |q_t.val| ≤ sc.val) (bas : |a_s.val| ≤ sc.val) (bat : |a_t.val| ≤ sc.val)
    (bbs : |b_s.val| ≤ sc.val) (bbt : |b_t.val| ≤ sc.val)
    (eqs : |q_s.val - Qs| ≤ δ) (eqt : |q_t.val - Qt| ≤ δ) (eas : |a_s.val - As| ≤ δ) (eat : |a_t.val - At| ≤ δ)
    (ebs : |b_s.val - Bs| ≤ δ) (ebt : |b_t.val - Bt| ≤ δ) (hδ : δ ≤ 2 / 10 ^ 13 * sc.val)
    (hfa : (sub a_t q_t).isFinite) (hfb : (sub b_t q_t).isFinite) (hft : (mul E12 sc).isFinite)
    (hda : (mul E12 sc).val < |(sub a_t q_t).val|) (hdb : (mul E12 sc).val < |(sub b_t q_t).val|)
    (hne : (0 < (sub a_t q_t).val) ↔ ¬ (0 < (sub b_t q_t).val))
    (hfd : (d0F q_s q_t a_s a_t b_s b_t).isFinite) (hfm : (mul E12 (magX q_s q_t a_s a_t b_s b_t sc)).isFinite) :
    |(d0F q_s q_t a_s a_t b_s b_t).val - ((Bs - As) * (Qt - At) - (Qs - As) * (Bt - At))| ≤
      (mul E12 (magX q_s q_t a_s a_t b_s b_t sc)).val := by
  set S := sc.val with hSd
  have hδ0 : 0 ≤ δ := le_trans (abs_nonneg _) eqs
  -- the four differences
  unfold d0F at hfd ⊢
  have fP1 := (isFinite_of_sub hfd).1; have fP2 := (isFinite_of_sub hfd).2
  have fu1 := (isFinite_of_mul fP1).1; have fv1 := (isFinite_of_mul fP1).2
  have fu2 := (isFinite_of_mul fP2).1; have fv2 := (isFinite_of_mul fP2).2
  obtain ⟨cu1, -, bu1⟩ := diff_close hbs has fu1 ebs eas bbs bas
  obtain ⟨cv1, -, bv1⟩ := diff_close hqt hat fv1 eqt eat bqt bat
  obtain ⟨cu2, -, bu2⟩ := diff_close hqs has fu2 eqs eas bqs bas
  obtain ⟨cv2, rv2, bv2⟩ := diff_close hbt hat fv2 ebt eat bbt bat
  set u1 := (sub b_s a_s).val; set v1 := (sub q_t a_t).val
  set u2 := (sub q_s a_s).val; set v2 := (sub b_t a_t).val
  set τ := 2 * δ + 2 * ε * S with hτd
  have hτ : τ ≤ 401 / 10 ^ 15 * S := by simp only [hτd]; nlinarith
  have hτ0 : 0 ≤ τ := by positivity
  -- `|v2|` is large: the `above`s differ
  have t := tolE_lo hs hS hft
  obtain ⟨-, ra, -⟩ := diff_close hat hqt hfa eat eqt bat bqt
  obtain ⟨-, rb, -⟩ := diff_close hbt hqt hfb ebt eqt bbt bqt
  set fa := (sub a_t q_t).val; set fb := (sub b_t q_t).val
  have hbig : (195 / 100) * (1 / 10 ^ 12) * S ≤ |v2| := by
    have la : |fa| ≤ (1 + ε) * |a_t.val - q_t.val| := by
      have := abs_sub_abs_le_abs_sub fa (a_t.val - q_t.val); nlinarith [abs_nonneg (a_t.val - q_t.val)]
    have lb : |fb| ≤ (1 + ε) * |b_t.val - q_t.val| := by
      have := abs_sub_abs_le_abs_sub fb (b_t.val - q_t.val); nlinarith [abs_nonneg (b_t.val - q_t.val)]
    -- signs of the unrounded differences follow the rounded ones
    have sa : (0 < fa ↔ 0 < a_t.val - q_t.val) := by
      constructor
      · intro h; by_contra hn; push_neg at hn
        rw [abs_of_nonpos hn] at ra; have := (abs_le.mp ra).2; nlinarith
      · intro h; rw [abs_of_pos h] at ra; have := (abs_le.mp ra).1; nlinarith
    have sb : (0 < fb ↔ 0 < b_t.val - q_t.val) := by
      constructor
      · intro h; by_contra hn; push_neg at hn
        rw [abs_of_nonpos hn] at rb; have := (abs_le.mp rb).2; nlinarith
      · intro h; rw [abs_of_pos h] at rb; have := (abs_le.mp rb).1; nlinarith
    have hsum : |b_t.val - a_t.val| = |b_t.val - q_t.val| + |a_t.val - q_t.val| := by
      have ha0 : a_t.val - q_t.val ≠ 0 := by
        intro h0; rw [h0, abs_zero] at la; linarith [abs_nonneg fa]
      have hb0 : b_t.val - q_t.val ≠ 0 := by
        intro h0; rw [h0, abs_zero] at lb; linarith [abs_nonneg fb]
      rw [show b_t.val - a_t.val = (b_t.val - q_t.val) - (a_t.val - q_t.val) by ring]
      by_cases h : 0 < fa
      · have h1 := sa.mp h
        have h2 : b_t.val - q_t.val < 0 := lt_of_le_of_ne (not_lt.mp (mt sb.mpr (hne.mp h))) hb0
        rw [abs_of_neg h2, abs_of_pos h1, abs_of_neg (by linarith)]; ring
      · have h1 : a_t.val - q_t.val < 0 := lt_of_le_of_ne (not_lt.mp (mt sa.mpr h)) ha0
        have h2 := sb.mp (by by_contra hb; exact h (hne.mpr hb))
        rw [abs_of_pos h2, abs_of_neg h1, abs_of_pos (by linarith)]; ring
    have lv : (1 - ε) * |b_t.val - a_t.val| ≤ |v2| := by
      have := abs_sub_abs_le_abs_sub (b_t.val - a_t.val) v2
      rw [abs_sub_comm] at this; nlinarith
    nlinarith
  -- the products
  have bl1 := bil cu1 cv1
  have bl2 := bil cu2 cv2
  have fE : E12.isFinite := by decide
  have m1 := mul_rel fu1 fv1 fP1
  have m2 := mul_rel fu2 fv2 fP2
  have sd := sub_rel0 fP1 fP2 hfd
  set p1 := (mul (sub b_s a_s) (sub q_t a_t)).val
  set p2 := (mul (sub q_s a_s) (sub b_t a_t)).val
  have hS0 : 0 ≤ S := by linarith
  have au1 := abs_nonneg u1; have av1 := abs_nonneg v1; have au2 := abs_nonneg u2; have av2 := abs_nonneg v2
  have k1 : |u1 * v1| ≤ 3 * (|u1| * S) := by rw [abs_mul]; nlinarith
  have k2 : |u2 * v2| ≤ 3 * (|u2| * S) := by rw [abs_mul]; nlinarith
  have j1 : |u1| * τ ≤ 401 / 10 ^ 15 * (|u1| * S) := by nlinarith
  have j2 : |v1| * τ ≤ 401 / 10 ^ 15 * (|v1| * S) := by nlinarith
  have j3 : |u2| * τ ≤ 401 / 10 ^ 15 * (|u2| * S) := by nlinarith
  have j4 : |v2| * τ ≤ 401 / 10 ^ 15 * (|v2| * S) := by nlinarith
  have jτ : τ * τ ≤ (401 / 10 ^ 15) ^ 2 * (S * S) := by nlinarith
  have jS : (195 / 100) * (1 / 10 ^ 12) * (S * S) ≤ |v2| * S := by nlinarith
  have hp12 : |p1 - p2| ≤ 4 * (|u1| * S) + 4 * (|u2| * S) + 2 * η := by
    have := abs_sub p1 p2
    have := abs_sub_abs_le_abs_sub p1 (u1 * v1); have := abs_sub_abs_le_abs_sub p2 (u2 * v2)
    have : ε * |u1 * v1| ≤ ε * (3 * (|u1| * S)) := mul_le_mul_of_nonneg_left k1 (by norm_num)
    have : ε * |u2 * v2| ≤ ε * (3 * (|u2| * S)) := mul_le_mul_of_nonneg_left k2 (by norm_num)
    have : 0 ≤ |u1| * S := by positivity
    have : 0 ≤ |u2| * S := by positivity
    linarith
  have herr : |(sub (mul (sub b_s a_s) (sub q_t a_t)) (mul (sub q_s a_s) (sub b_t a_t))).val -
      ((Bs - As) * (Qt - At) - (Qs - As) * (Bt - At))| ≤
      58 / 10 ^ 14 * (|u1| * S + |u2| * S + |v1| * S + |v2| * S) + 3 * η := by
    have e1 := abs_sub_le (sub (mul (sub b_s a_s) (sub q_t a_t)) (mul (sub q_s a_s) (sub b_t a_t))).val (p1 - p2)
      ((Bs - As) * (Qt - At) - (Qs - As) * (Bt - At))
    have e2 : |p1 - p2 - ((Bs - As) * (Qt - At) - (Qs - As) * (Bt - At))| ≤
        |p1 - u1 * v1| + |p2 - u2 * v2| + |u1 * v1 - (Bs - As) * (Qt - At)| + |u2 * v2 - (Qs - As) * (Bt - At)| := by
      rw [show p1 - p2 - ((Bs - As) * (Qt - At) - (Qs - As) * (Bt - At)) =
        (p1 - u1 * v1) - (p2 - u2 * v2) + (u1 * v1 - (Bs - As) * (Qt - At)) - (u2 * v2 - (Qs - As) * (Bt - At)) by ring]
      refine le_trans (abs_sub _ _) (add_le_add (le_trans (abs_add_le _ _) (add_le_add (abs_sub _ _) le_rfl)) le_rfl)
    have : ε * |u1 * v1| ≤ ε * (3 * (|u1| * S)) := mul_le_mul_of_nonneg_left k1 (by norm_num)
    have : ε * |u2 * v2| ≤ ε * (3 * (|u2| * S)) := mul_le_mul_of_nonneg_left k2 (by norm_num)
    have : ε * |p1 - p2| ≤ ε * (4 * (|u1| * S) + 4 * (|u2| * S) + 2 * η) := mul_le_mul_of_nonneg_left hp12 (by norm_num)
    have hη := η_pos
    have : 0 ≤ |u1| * S := by positivity
    have : 0 ≤ |u2| * S := by positivity
    have : 0 ≤ |v1| * S := by positivity
    have : 0 ≤ |v2| * S := by positivity
    have : 0 ≤ S * S := by positivity
    linarith
  -- `E·mag` from below
  unfold magX at hfm ⊢
  have fmag := (isFinite_of_mul hfm).2
  have fsum := (isFinite_of_mul fmag).1
  have fs2 := (isFinite_of_add fsum).1
  have fs1 := (isFinite_of_add fs2).1
  have g1 := add_lo (by simpa using fu1) (by simpa using fu2) fs1 (by simp only [FP.abs_val]; positivity)
  have g2 := add_lo fs1 (by simpa using fv1) fs2 (by simp only [FP.abs_val]; nlinarith)
  have g3 := add_lo fs2 (by simpa using fv2) fsum (by simp only [FP.abs_val]; nlinarith)
  simp only [FP.abs_val] at g1 g2 g3
  set sm := (add (add (add (sub b_s a_s).abs (sub q_s a_s).abs) (sub q_t a_t).abs) (sub b_t a_t).abs).val
  have hsm : (1 - 3 * ε) * (|u1| + |u2| + |v1| + |v2|) ≤ sm := by nlinarith
  have mm := mul_rel fsum hs fmag
  have hsm0 : 0 ≤ sm := le_trans (by nlinarith) hsm
  rw [abs_of_nonneg (mul_nonneg hsm0 hS0)] at mm
  set mg := (mul (add (add (add (sub b_s a_s).abs (sub q_s a_s).abs) (sub q_t a_t).abs) (sub b_t a_t).abs) sc).val
  have hmg : (1 - ε) * (sm * S) - η ≤ mg := by have := (abs_le.mp mm).1; linarith
  have em := mul_rel fE fmag hfm
  rw [E12_val] at em
  have hη := η_pos; have hηs := η_small
  have hmg0 : 0 ≤ mg := by
    have : 0 ≤ sm * S := mul_nonneg hsm0 hS0
    by_contra hn; push_neg at hn
    -- a negative rounding of a nonnegative product is impossible beyond `η`
    have := (abs_le.mp mm).1
    nlinarith
  rw [abs_of_nonneg (by positivity : (0 : ℚ) ≤ 4951760157141521 / 2 ^ 92 * mg)] at em
  have hE : (1 - ε) * (4951760157141521 / 2 ^ 92 * mg) - η ≤ (mul E12 (mul (add (add (add (sub b_s a_s).abs
      (sub q_s a_s).abs) (sub q_t a_t).abs) (sub b_t a_t).abs) sc)).val := by
    have := (abs_le.mp em).1; linarith
  have hsmS : (1 - 3 * ε) * (|u1| * S + |u2| * S + |v1| * S + |v2| * S) ≤ sm * S := by
    have := mul_le_mul_of_nonneg_right hsm hS0; linarith
  have hA : (195 / 100) * (1 / 10 ^ 12) ≤ |v2| * S := by nlinarith
  have hη2 : η ≤ 1 / 10 ^ 40 * (|v2| * S) := by nlinarith
  have : 0 ≤ |u1| * S := by positivity
  have : 0 ≤ |u2| * S := by positivity
  have : 0 ≤ |v1| * S := by positivity
  nlinarith

end IEEE754.Screen
