/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import FloatVerification.Basic

/-!
# `orient2d`'s fast path, and where it fails

`orient2d(a, b, c)` (kernel/forge-math/src/predicates.rs) works on the rounded differences
`D₁ = a.x − c.x`, `D₂ = b.y − c.y`, `D₃ = a.y − c.y` and `D₄ = b.x − c.x`. It forms
- `left = D₁·D₂`, `right = D₃·D₄` and `det = left − right`;
- `detsum = |left| + |right|` and `bound = CCW_BOUND · detsum`, every step rounded.

It returns `det` when `det > bound` or `−det > bound`.

* `orient2d_fast`: if the two products `D₁·D₂` and `D₃·D₄` and the product
  `CCW_BOUND · detsum` are each zero or at least `2^-1022` (no underflow), then whenever the
  test passes the returned value is nonzero and has the sign of the exact determinant
  `(a.x − c.x)(b.y − c.y) − (a.y − c.y)(b.x − c.x)` of the input doubles. This is Shewchuk's
  bound `(3 + 16ε)ε`, proven. Overflow needs no hypothesis: the test fails whenever anything
  overflows.
* The no-underflow hypothesis cannot be dropped. `Counterexample.lean` gives finite inputs, with
  coordinates near `1e-160`, for which the test passes and the sign is wrong.
-/

namespace FloatVerification

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP
open fv_predicates

/-! ## Shewchuk's bound, as an inequality between rationals -/

lemma abs_le_add_of_err {a b k : ℚ} (h : |a - b| ≤ k * |a|) : |b| ≤ (1 + k) * |a| := by
  have : |b| ≤ |a| + |a - b| := by
    calc |b| = |a - (a - b)| := by ring_nf
      _ ≤ |a| + |a - b| := abs_sub _ _
  linarith

lemma prod_err {D₁ D₂ d₁ d₂ : ℚ} (h₁ : |D₁ - d₁| ≤ ε * |D₁|) (h₂ : |D₂ - d₂| ≤ ε * |D₂|) :
    |D₁ * D₂ - d₁ * d₂| ≤ (2 * ε + ε ^ 2) * |D₁ * D₂| := by
  have e : D₁ * D₂ - d₁ * d₂ = D₁ * (D₂ - d₂) + (D₁ - d₁) * D₂ - (D₁ - d₁) * (D₂ - d₂) := by ring
  rw [e, abs_mul D₁ D₂]
  have hε := ε_pos
  calc |D₁ * (D₂ - d₂) + (D₁ - d₁) * D₂ - (D₁ - d₁) * (D₂ - d₂)|
      ≤ |D₁ * (D₂ - d₂)| + |(D₁ - d₁) * D₂| + |(D₁ - d₁) * (D₂ - d₂)| := by
        refine le_trans (abs_sub _ _) ?_
        gcongr; exact abs_add_le _ _
    _ = |D₁| * |D₂ - d₂| + |D₁ - d₁| * |D₂| + |D₁ - d₁| * |D₂ - d₂| := by
        simp only [abs_mul]
    _ ≤ |D₁| * (ε * |D₂|) + (ε * |D₁|) * |D₂| + (ε * |D₁|) * (ε * |D₂|) := by
        gcongr
    _ = (2 * ε + ε ^ 2) * (|D₁| * |D₂|) := by ring

/-- The constant checks: `(ε + (2ε + ε²)(1 + ε))(1 + ε)² ≤ (3 + 16ε)ε · (1 − ε)`. -/
lemma shewchuk_constant :
    (ε + (2 * ε + ε ^ 2) * (1 + ε)) * (1 + ε) ^ 2 ≤ (3 + 16 * ε) * ε * (1 - ε) := by
  norm_num [ε]

/-- **Shewchuk's orientation bound**, over the rationals. The hypotheses are the rounding errors
of each step, relative to the computed value, and the test `bound < |det|`. -/
theorem shewchuk {d₁ d₂ d₃ d₄ D₁ D₂ D₃ D₄ L R det S B : ℚ}
    (h₁ : |D₁ - d₁| ≤ ε * |D₁|) (h₂ : |D₂ - d₂| ≤ ε * |D₂|)
    (h₃ : |D₃ - d₃| ≤ ε * |D₃|) (h₄ : |D₄ - d₄| ≤ ε * |D₄|)
    (hL : |L - D₁ * D₂| ≤ ε * |L|) (hR : |R - D₃ * D₄| ≤ ε * |R|)
    (hdet : |det - (L - R)| ≤ ε * |det|)
    (hS0 : 0 ≤ S) (hS : |S - (|L| + |R|)| ≤ ε * |S|)
    (hB0 : 0 ≤ B) (hB : |B - (3 + 16 * ε) * ε * S| ≤ ε * |B|)
    (hsign : (0 < det → 0 < L - R) ∧ (det < 0 → L - R < 0))
    (htest : B < |det|) :
    (0 < det → 0 < d₁ * d₂ - d₃ * d₄) ∧ (det < 0 → d₁ * d₂ - d₃ * d₄ < 0) := by
  have hε := ε_pos
  have hε1 := ε_le
  set C := (3 + 16 * ε) * ε with hCdef
  have hC : 0 < C := by positivity
  set K := ε + (2 * ε + ε ^ 2) * (1 + ε) with hKdef
  have hK : 0 < K := by positivity
  set x := L - R
  set t := d₁ * d₂ - d₃ * d₄
  -- each product, against the exact one
  have hLt : |L - d₁ * d₂| ≤ K * |L| := by
    have p := prod_err h₁ h₂
    have q := abs_le_add_of_err (a := L) (b := D₁ * D₂) hL
    calc |L - d₁ * d₂| = |(L - D₁ * D₂) + (D₁ * D₂ - d₁ * d₂)| := by ring_nf
      _ ≤ |L - D₁ * D₂| + |D₁ * D₂ - d₁ * d₂| := abs_add_le _ _
      _ ≤ ε * |L| + (2 * ε + ε ^ 2) * ((1 + ε) * |L|) := by
          gcongr; exact le_trans p (by gcongr)
      _ = K * |L| := by ring
  have hRt : |R - d₃ * d₄| ≤ K * |R| := by
    have p := prod_err h₃ h₄
    have q := abs_le_add_of_err (a := R) (b := D₃ * D₄) hR
    calc |R - d₃ * d₄| = |(R - D₃ * D₄) + (D₃ * D₄ - d₃ * d₄)| := by ring_nf
      _ ≤ |R - D₃ * D₄| + |D₃ * D₄ - d₃ * d₄| := abs_add_le _ _
      _ ≤ ε * |R| + (2 * ε + ε ^ 2) * ((1 + ε) * |R|) := by
          gcongr; exact le_trans p (by gcongr)
      _ = K * |R| := by ring
  have hxt : |x - t| ≤ K * (|L| + |R|) := by
    calc |x - t| = |(L - d₁ * d₂) - (R - d₃ * d₄)| := by simp only [x, t]; ring_nf
      _ ≤ |L - d₁ * d₂| + |R - d₃ * d₄| := abs_sub _ _
      _ ≤ K * |L| + K * |R| := add_le_add hLt hRt
      _ = K * (|L| + |R|) := by ring
  -- the computed sum, bound and determinant, against what they round
  have hS' : |L| + |R| ≤ (1 + ε) * S := by
    have := abs_le_add_of_err (a := S) (b := |L| + |R|) hS
    rwa [abs_of_nonneg (by positivity : (0 : ℚ) ≤ |L| + |R|), abs_of_nonneg hS0] at this
  have hB' : C * S ≤ (1 + ε) * B := by
    have := abs_le_add_of_err (a := B) (b := C * S) hB
    rwa [abs_of_nonneg (by positivity : (0 : ℚ) ≤ C * S), abs_of_nonneg hB0] at this
  have hdet' : (1 - ε) * |det| ≤ |x| := by
    have h2 : |det| ≤ |x| + |det - x| := by
      calc |det| = |x + (det - x)| := by ring_nf
        _ ≤ |x| + |det - x| := abs_add_le _ _
    linarith
  -- chain: `(1 - ε) C |x - t| < (1 - ε) K (1+ε)² |det| ≤ K (1+ε)² |x| ≤ C (1 - ε) |x|`
  have hM := shewchuk_constant
  have c1 : C * |x - t| ≤ K * (1 + ε) * (C * S) := by
    calc C * |x - t| ≤ C * (K * ((1 + ε) * S)) := by
          gcongr; exact le_trans hxt (by gcongr)
      _ = K * (1 + ε) * (C * S) := by ring
  have c2 : K * (1 + ε) * (C * S) ≤ K * (1 + ε) ^ 2 * B := by
    calc K * (1 + ε) * (C * S) ≤ K * (1 + ε) * ((1 + ε) * B) := by gcongr
      _ = K * (1 + ε) ^ 2 * B := by ring
  have c3 : K * (1 + ε) ^ 2 * B < K * (1 + ε) ^ 2 * |det| := by
    apply mul_lt_mul_of_pos_left htest; positivity
  have c4 : (1 - ε) * (K * (1 + ε) ^ 2 * |det|) ≤ K * (1 + ε) ^ 2 * |x| := by
    calc (1 - ε) * (K * (1 + ε) ^ 2 * |det|) = K * (1 + ε) ^ 2 * ((1 - ε) * |det|) := by ring
      _ ≤ K * (1 + ε) ^ 2 * |x| := by gcongr
  have c5 : K * (1 + ε) ^ 2 * |x| ≤ C * (1 - ε) * |x| := by
    apply mul_le_mul_of_nonneg_right _ (abs_nonneg _)
    simpa [hKdef, hCdef] using hM
  have h1e : 0 < 1 - ε := sub_pos.mpr (lt_of_le_of_lt hε1 (by norm_num))
  have key : C * (1 - ε) * |x - t| < C * (1 - ε) * |x| := by
    have := mul_lt_mul_of_pos_left (lt_of_le_of_lt (le_trans c1 c2) c3) h1e
    calc C * (1 - ε) * |x - t| = (1 - ε) * (C * |x - t|) := by ring
      _ < (1 - ε) * (K * (1 + ε) ^ 2 * |det|) := this
      _ ≤ K * (1 + ε) ^ 2 * |x| := c4
      _ ≤ C * (1 - ε) * |x| := c5
  have hlt : |x - t| < |x| := lt_of_mul_lt_mul_left key (by positivity)
  obtain ⟨hp, hn⟩ := hsign
  rw [abs_sub_lt_iff] at hlt
  constructor
  · intro h; have := hp h; rw [abs_of_pos this] at hlt; linarith
  · intro h; have := hn h; rw [abs_of_neg this] at hlt; linarith

/-! ## The translated `orient2d` -/

/-- The values `orient2d` computes before its test. -/
def oLeft (a b c : Point2) : F64 := mul (sub a.x c.x) (sub b.y c.y)
def oRight (a b c : Point2) : F64 := mul (sub a.y c.y) (sub b.x c.x)
def oDet (a b c : Point2) : F64 := sub (oLeft a b c) (oRight a b c)
def oSum (a b c : Point2) : F64 := add (oLeft a b c).abs (oRight a b c).abs
/-- `CCW_BOUND` -/
def ccw : F64 := FP.ofBits 4375247037990436868
def oBound (a b c : Point2) : F64 := mul ccw (oSum a b c)

/-- The test of `orient2d`'s fast path: `det > bound || -det > bound`. -/
def orientFast (a b c : Point2) : Bool :=
  FP.lt (oBound a b c) (oDet a b c) || FP.lt (oBound a b c) (oDet a b c).neg

/-- The exact determinant of the input doubles. -/
def orientExact (a b c : Point2) : ℚ :=
  (a.x.val - c.x.val) * (b.y.val - c.y.val) - (a.y.val - c.y.val) * (b.x.val - c.x.val)

lemma orient2d_of_fast {a b c : Point2} (h : orientFast a b c = true) :
    orient2d a b c = ok (oDet a b c) := by
  simp only [orient2d, F64.Insts.CoreOpsArithMulF64F64.mul, F64.Insts.CoreOpsArithSubF64F64.sub,
    F64.Insts.CoreOpsArithAddF64F64.add, F64.Insts.CoreOpsArithNegF64.neg, core.f64.F64.abs,
    ccw_bound, core.f64.F64.from_bits, F64.Insts.CoreCmpPartialOrdF64.gt, bind_ok]
  simp only [orientFast, oDet, oBound, oSum, oLeft, oRight, ccw, Bool.or_eq_true] at h
  rcases h with h | h
  · rw [if_pos (by exact h)]; rfl
  · by_cases h' : FP.lt (mul (FP.ofBits 4375247037990436868) (add (mul (sub a.x c.x) (sub b.y c.y)).abs
        (mul (sub a.y c.y) (sub b.x c.x)).abs)) (sub (mul (sub a.x c.x) (sub b.y c.y))
        (mul (sub a.y c.y) (sub b.x c.x))) = true
    · rw [if_pos (by exact h')]; rfl
    · rw [if_neg (by exact h'), if_pos (by exact h)]; rfl

lemma prec_eps : (2 : ℚ) ^ (-(binary64.prec : ℤ)) = ε := rfl

/-- **`orient2d`'s fast path gives the exact sign when nothing underflows.** For finite
coordinates, if the products `(a.x − c.x)·(b.y − c.y)` and `(a.y − c.y)·(b.x − c.x)` of the
rounded differences, and `CCW_BOUND · detsum`, are each zero or at least `2^-1022` in magnitude,
then whenever the test passes `orient2d` returns a finite nonzero value with the sign of the
exact determinant. -/
theorem orient2d_fast (a b c : Point2)
    (hP : (sub a.x c.x).val * (sub b.y c.y).val = 0 ∨
      2 ^ (-1022 : ℤ) ≤ |(sub a.x c.x).val * (sub b.y c.y).val|)
    (hQ : (sub a.y c.y).val * (sub b.x c.x).val = 0 ∨
      2 ^ (-1022 : ℤ) ≤ |(sub a.y c.y).val * (sub b.x c.x).val|)
    (hBn : ccw.val * (oSum a b c).val = 0 ∨ 2 ^ (-1022 : ℤ) ≤ |ccw.val * (oSum a b c).val|)
    (hax : a.x.isFinite) (hay : a.y.isFinite) (hbx : b.x.isFinite) (hby : b.y.isFinite)
    (hcx : c.x.isFinite) (hcy : c.y.isFinite)
    (h : orientFast a b c = true) :
    ∃ v, orient2d a b c = ok v ∧ v.isFinite ∧ v.val ≠ 0 ∧
      (0 < v.val ↔ 0 < orientExact a b c) ∧ (v.val < 0 ↔ orientExact a b c < 0) := by
  refine ⟨_, orient2d_of_fast h, ?_⟩
  have hC : ccw.isFinite := by decide
  have hC0 : 0 < ccw.val := by rw [ccw, ccw_bound_val]; positivity
  have hfin : ∀ X, FP.lt (oBound a b c) X = true →
      (oLeft a b c).isFinite ∧ (oRight a b c).isFinite ∧ (oSum a b c).isFinite ∧
        (oBound a b c).isFinite ∧ (oDet a b c).isFinite :=
    fun X hX => finite_of_bound hC hC0 hX
  simp only [orientFast, Bool.or_eq_true] at h
  obtain ⟨hL, hR, hS, hB, hdet⟩ : (oLeft a b c).isFinite ∧ (oRight a b c).isFinite ∧
      (oSum a b c).isFinite ∧ (oBound a b c).isFinite ∧ (oDet a b c).isFinite := by
    rcases h with h | h
    · exact hfin _ h
    · exact hfin _ h
  -- `bound < |det|`
  have htest : (oBound a b c).val < |(oDet a b c).val| := by
    rcases h with h | h
    · exact lt_of_lt_of_le (lt_finite hB hdet h) (le_abs_self _)
    · have := lt_finite hB (by simpa using hdet) h
      rw [val_neg] at this
      exact lt_of_lt_of_le this (neg_le_abs _)
  obtain ⟨h1, h2⟩ := isFinite_of_mul hL
  obtain ⟨h3, h4⟩ := isFinite_of_mul hR
  have hLa : (oLeft a b c).abs.isFinite := by simpa using hL
  have hRa : (oRight a b c).abs.isFinite := by simpa using hR
  -- the rounding error of each step, relative to the computed value
  have e₁ := sub_err_res hax hcx h1
  have e₂ := sub_err_res hby hcy h2
  have e₃ := sub_err_res hay hcy h3
  have e₄ := sub_err_res hbx hcx h4
  have eL := mul_err_res h1 h2 hL (by simpa using hP)
  have eR := mul_err_res h3 h4 hR (by simpa using hQ)
  have edet := sub_err_res hL hR hdet
  have eS := add_err_res hLa hRa hS
  have eB := mul_err_res hC hS hB (by simpa using hBn)
  rw [prec_eps] at e₁ e₂ e₃ e₄ eL eR edet eS eB
  rw [abs_val, abs_val] at eS
  have hS0 : 0 ≤ (oSum a b c).val := by
    simp only [oSum]; rw [add_eq_round hLa hRa]
    exact round_val_nonneg binary64 (add_nonneg (val_abs_nonneg _) (val_abs_nonneg _))
  have hB0 : 0 ≤ (oBound a b c).val := by
    simp only [oBound]; rw [mul_eq_round hC hS]
    exact round_val_nonneg binary64 (mul_nonneg hC0.le hS0)
  rw [show ccw.val = (3 + 16 * ε) * ε from ccw_bound_val] at eB
  have key := shewchuk e₁ e₂ e₃ e₄ eL eR edet hS0 eS hB0 eB (sub_sign hL hR) htest
  have hne : (oDet a b c).val ≠ 0 := by
    intro h0; rw [h0, abs_zero] at htest; linarith
  simp only [orientExact]
  obtain ⟨kp, kn⟩ := key
  refine ⟨hdet, hne, ⟨kp, fun ht => ?_⟩, ⟨kn, fun ht => ?_⟩⟩
  · rcases lt_or_gt_of_ne hne with h' | h'
    · have := kn h'; linarith
    · exact h'
  · rcases lt_or_gt_of_ne hne with h' | h'
    · exact h'
    · have := kp h'; linarith

end FloatVerification
