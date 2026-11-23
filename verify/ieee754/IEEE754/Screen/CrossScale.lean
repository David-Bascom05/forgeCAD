/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Cross
import IEEE754.Screen.First
import IEEE754.Nearest
import IEEE754.Identities

/-!
# `crosses_f`'s scale, fix (a) (decide-once 5eb8e9d)

`scale = 1.0 + m3(q).max(m3(a)).max(m3(b))` with `m3(p) = max(|p0|, |p1|, |p2|)`: every coordinate
of the three points, the drop axis included. `scale_facts` gives the three hypotheses
`crosses_f_sound` takes of it:
- `1 ≤ scale`;
- every coordinate is at most `scale`;
- `(1 − ε)(1 + M) ≤ scale`, where `M` is the exact max.

`pf_delta_ok`: so `pf`'s bound from item 5, `(1 + 3ε)·1e-13·(1 + max|p_k|)`, is under
`2·10^-13·scale` for each of the three points. So is an exactly rounded point (`to_f64`, `2ε`
relative).
-/

namespace IEEE754.Screen

open FP

/-- `m3(p)`. -/
def m3 (p0 p1 p2 : F64) : F64 := fmax (fmax p0.abs p1.abs) p2.abs

/-- `scale`, fix (a). -/
def scaleA (q0 q1 q2 a0 a1 a2 b0 b1 b2 : F64) : F64 :=
  add F1 (fmax (fmax (m3 q0 q1 q2) (m3 a0 a1 a2)) (m3 b0 b1 b2))

/-- A float at most `r` is at most `fl(r)` (rounding is to nearest, and a float rounds to itself). -/
lemma float_le_round {zs : Bool} {r : ℚ} {x : F64} (hx : x.isFinite) (hf : (round binary64 zs r).isFinite)
    (h : x.val ≤ r) : x.val ≤ (round binary64 zs r).val := by
  by_contra hn; push Not at hn
  have hn' := round_nearest (f := binary64) hf hx
  rw [abs_of_neg (by linarith : (round binary64 zs r).val - r < 0)] at hn'
  have hr : r = x.val := by
    rcases eq_or_lt_of_le h with h1 | h1
    · exact h1.symm
    · exfalso; rw [abs_of_neg (by linarith : x.val - r < 0)] at hn'; linarith
  subst hr
  by_cases hz : x.isZero = true
  · have : x.val = 0 := by cases x <;> simp_all [isZero, val]
    rw [this] at hn; simp [round] at hn
  · rw [round_val_self hx (by simpa using hz)] at hn; exact lt_irrefl _ hn

lemma m3_val (p0 p1 p2 : F64) : (m3 p0 p1 p2).val = max (max |p0.val| |p1.val|) |p2.val| := by
  simp only [m3, fmax_val, FP.abs_val]

lemma m3_fin {p0 p1 p2 : F64} (h0 : p0.isFinite) (h1 : p1.isFinite) (h2 : p2.isFinite) : (m3 p0 p1 p2).isFinite :=
  fmax_fin (fmax_fin (by simpa using h0) (by simpa using h1)) (by simpa using h2)

/-- The exact max of the nine coordinates' magnitudes. -/
def max9 (q0 q1 q2 a0 a1 a2 b0 b1 b2 : F64) : ℚ :=
  max (max (max (max |q0.val| |q1.val|) |q2.val|) (max (max |a0.val| |a1.val|) |a2.val|))
    (max (max |b0.val| |b1.val|) |b2.val|)

/-- **Fix (a)'s scale.** -/
theorem scale_facts {q0 q1 q2 a0 a1 a2 b0 b1 b2 : F64}
    (hq0 : q0.isFinite) (hq1 : q1.isFinite) (hq2 : q2.isFinite) (ha0 : a0.isFinite) (ha1 : a1.isFinite)
    (ha2 : a2.isFinite) (hb0 : b0.isFinite) (hb1 : b1.isFinite) (hb2 : b2.isFinite)
    (hs : (scaleA q0 q1 q2 a0 a1 a2 b0 b1 b2).isFinite) :
    1 ≤ (scaleA q0 q1 q2 a0 a1 a2 b0 b1 b2).val ∧
      max9 q0 q1 q2 a0 a1 a2 b0 b1 b2 ≤ (scaleA q0 q1 q2 a0 a1 a2 b0 b1 b2).val ∧
      (1 - ε) * (1 + max9 q0 q1 q2 a0 a1 a2 b0 b1 b2) ≤ (scaleA q0 q1 q2 a0 a1 a2 b0 b1 b2).val := by
  set M := max9 q0 q1 q2 a0 a1 a2 b0 b1 b2 with hMd
  have fM := fmax_fin (fmax_fin (m3_fin hq0 hq1 hq2) (m3_fin ha0 ha1 ha2)) (m3_fin hb0 hb1 hb2)
  have hMv : (fmax (fmax (m3 q0 q1 q2) (m3 a0 a1 a2)) (m3 b0 b1 b2)).val = M := by
    simp only [fmax_val, m3_val, hMd, max9]
  have M0 : 0 ≤ M := by
    rw [hMd, max9]
    exact le_max_of_le_left (le_max_of_le_left (le_max_of_le_left (le_max_of_le_left (abs_nonneg q0.val))))
  have fO : F1.isFinite := by decide
  unfold scaleA at hs ⊢
  rw [add_eq_round fO fM] at hs
  rw [F1_val, hMv] at hs
  rw [add_eq_round fO fM, F1_val, hMv]
  refine ⟨?_, ?_, ?_⟩
  · have := float_le_round fO hs (show F1.val ≤ 1 + M by rw [F1_val]; linarith)
    rwa [F1_val] at this
  · have := float_le_round fM hs (show _ ≤ 1 + M by rw [hMv]; linarith)
    rwa [hMv] at this
  · have e2 := add_rel0 fO fM (by rw [add_eq_round fO fM, F1_val, hMv]; exact hs)
    rw [add_eq_round fO fM, F1_val, hMv, abs_of_nonneg (show (0 : ℚ) ≤ 1 + M by linarith)] at e2
    have := (abs_le.mp e2).1; linarith

/-- **`pf`'s bound fits.** Item 5's `(1 + 3ε)·c13·(1 + max|p_k|)`, for any point among `q, a, b`
(`max|p_k| ≤ M`), is under `2·10^-13·scale`. -/
theorem pf_delta_ok {S M Mp : ℚ} (hS : (1 - ε) * (1 + M) ≤ S) (hM : Mp ≤ M) (hMp : 0 ≤ Mp) :
    (1 + 3 * ε) * (3961408125713217 / 2 ^ 95) * (1 + Mp) ≤ 2 / 10 ^ 13 * S := by
  have : (1 + 3 * ε) * (3961408125713217 / 2 ^ 95) * (1 + Mp) ≤ (1 + 3 * ε) * (3961408125713217 / 2 ^ 95) * (1 + M) := by
    apply mul_le_mul_of_nonneg_left (by linarith) (by norm_num)
  have h2 : (1 + 3 * ε) * (3961408125713217 / 2 ^ 95) * (1 + M) ≤ 2 / 10 ^ 13 * ((1 - ε) * (1 + M)) := by
    have : (0 : ℚ) ≤ 1 + M := by linarith
    nlinarith
  have h3 : 2 / 10 ^ 13 * ((1 - ε) * (1 + M)) ≤ 2 / 10 ^ 13 * S := mul_le_mul_of_nonneg_left hS (by norm_num)
  linarith

end IEEE754.Screen
