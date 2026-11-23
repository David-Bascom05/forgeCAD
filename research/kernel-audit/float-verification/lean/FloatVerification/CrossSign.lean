/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import FloatVerification.Basic

/-!
# `cross_sign`'s fast path is right for every finite input

`cross_sign(a, b, c, d)` (kernel/forge-math/src/predicates.rs) returns `det = l - r`, with
`l = a*b` and `r = c*d` rounded, when `|det| > CROSS_BOUND · (|l| + |r|)`. Otherwise it
computes the sign exactly. `cross_sign_fast` proves that whenever that test passes, the
returned value is nonzero and has the sign of the exact `a·b − c·d`. This holds for all finite
doubles, overflow and underflow included, and needs no error analysis. Rounding is monotone, so
`a·b ≤ c·d` gives `l ≤ r`, and the sign of a rounded difference is that of the exact difference
or zero. So a nonzero `det` can never have the wrong sign, whatever the bound is. The test only
has to exclude `det = 0`, NaN and overflow, and it does.
-/

namespace FloatVerification

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP
open fv_predicates

/-- The test of `cross_sign`'s fast path: `|det| > CROSS_BOUND · (|l| + |r|)`. -/
def crossFast (a b c d : F64) : Bool :=
  FP.lt (mul (FP.ofBits 4372995238176751618) (add (mul a b).abs (mul c d).abs))
    (sub (mul a b) (mul c d)).abs

/-- The translated `cross_sign` takes the fast path exactly when `crossFast` holds, and then
returns `l - r`. -/
lemma cross_sign_of_fast {a b c d : F64} (h : crossFast a b c d = true) :
    cross_sign a b c d = ok (sub (mul a b) (mul c d)) := by
  simp only [crossFast] at h
  simp only [cross_sign, F64.Insts.CoreOpsArithMulF64F64.mul, F64.Insts.CoreOpsArithSubF64F64.sub,
    F64.Insts.CoreOpsArithAddF64F64.add, core.f64.F64.abs, cross_bound, core.f64.F64.from_bits,
    F64.Insts.CoreCmpPartialOrdF64.gt, bind_ok]
  rw [if_pos (by exact h)]

/-- **`cross_sign`'s fast path gives the exact sign.** For finite `a b c d`, if the test
passes, `cross_sign` returns a finite nonzero value with the sign of `a·b − c·d`. -/
theorem cross_sign_fast (a b c d : F64) (ha : a.isFinite) (hb : b.isFinite) (hc : c.isFinite)
    (hd : d.isFinite) (h : crossFast a b c d = true) :
    ∃ v, cross_sign a b c d = ok v ∧ v.isFinite ∧ v.val ≠ 0 ∧
      (0 < v.val ↔ 0 < a.val * b.val - c.val * d.val) ∧
      (v.val < 0 ↔ a.val * b.val - c.val * d.val < 0) := by
  refine ⟨_, cross_sign_of_fast h, ?_⟩
  set l := mul a b
  set r := mul c d
  have hC : (FP.ofBits 4372995238176751618 : F64).isFinite := by decide
  have hC0 : 0 < (FP.ofBits 4372995238176751618 : F64).val := by
    rw [cross_bound_val]; positivity
  have hfast := h
  simp only [crossFast] at hfast
  obtain ⟨hl, hr, hS, hB, hdet⟩ := finite_of_bound hC hC0 hfast
  -- `B < |det|` with `B ≥ 0`, so `det ≠ 0`
  have hlt := lt_finite hB (by simpa using hdet) hfast
  rw [abs_val] at hlt
  have hB0 : 0 ≤ (mul (FP.ofBits 4372995238176751618) (add l.abs r.abs)).val := by
    rw [mul_eq_round hC hS]
    apply round_val_nonneg binary64 (mul_nonneg hC0.le _)
    rw [add_eq_round (by simpa using hl) (by simpa using hr)]
    exact round_val_nonneg binary64 (add_nonneg (val_abs_nonneg l) (val_abs_nonneg r))
  have hne : (sub l r).val ≠ 0 := by
    intro h0; rw [h0, abs_zero] at hlt; linarith
  obtain ⟨hpos, hneg⟩ := sub_sign hl hr
  -- monotonicity of the rounded products
  have hmono₁ := fun hle => mul_mono ha hb hc hd hl hr hle
  have hmono₂ := fun hle => mul_mono hc hd ha hb hr hl hle
  refine ⟨hdet, hne, ⟨fun hv => ?_, fun hv => ?_⟩, ⟨fun hv => ?_, fun hv => ?_⟩⟩
  · by_contra hc'
    have := hmono₁ (by linarith)
    have := hpos hv; linarith
  · rcases lt_or_gt_of_ne hne with h' | h'
    · have := hneg h'
      have := hmono₂ (by linarith)
      linarith
    · exact h'
  · by_contra hc'
    have := hmono₂ (by linarith)
    have := hneg hv; linarith
  · rcases lt_or_gt_of_ne hne with h' | h'
    · exact h'
    · have := hpos h'
      have := hmono₁ (by linarith)
      linarith

end FloatVerification
