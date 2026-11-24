/-
The exact field route of kernel/forge-cam-geom/src/real.rs (`Ctx::exact_sign`): an expression
whose closure has at most one distinct square root is evaluated exactly in Q(√c), each value a
pair (a, b) for a + b√c, and its sign read from the pair. What the code rests on:
- the product and the quotient stay pairs (`quad_mul`, `quad_div`), with √c² = c;
- the sign of a + b√c (√c ≥ 0) from the signs of a, b, c and of a² − b²c (`quad_sign_*`).
The rationals are exact (pairs of dyadics, real.rs `Q`), so these identities are the whole
argument; the route answers nothing (None) where a divisor's norm a₂² − b₂²c is 0 or the radicand
is negative, and the precision loop decides as before.
-/
import Mathlib

namespace ForgeCam.Quad

/-- The product of two pairs is a pair. -/
theorem quad_mul {r c : ℝ} (hr : r ^ 2 = c) (a1 b1 a2 b2 : ℝ) :
    (a1 + b1 * r) * (a2 + b2 * r) = (a1 * a2 + b1 * b2 * c) + (a1 * b2 + a2 * b1) * r := by
  rw [← hr]; ring

/-- The quotient by a pair whose norm a₂² − b₂²c is not 0: the product with the conjugate over
the norm. -/
theorem quad_div {r c : ℝ} (hr : r ^ 2 = c) (a1 b1 a2 b2 : ℝ) (hn : a2 ^ 2 - b2 ^ 2 * c ≠ 0) :
    (a1 + b1 * r) / (a2 + b2 * r) =
      ((a1 * a2 - b1 * b2 * c) / (a2 ^ 2 - b2 ^ 2 * c)) +
        ((b1 * a2 - a1 * b2) / (a2 ^ 2 - b2 ^ 2 * c)) * r := by
  subst hr
  have hn' : (a2 + b2 * r) * (a2 - b2 * r) = a2 ^ 2 - b2 ^ 2 * r ^ 2 := by ring
  have hc : a2 - b2 * r ≠ 0 := by
    intro h; apply hn; rw [← hn', h, mul_zero]
  have e1 : (a1 * a2 - b1 * b2 * r ^ 2) / (a2 ^ 2 - b2 ^ 2 * r ^ 2) +
      (b1 * a2 - a1 * b2) / (a2 ^ 2 - b2 ^ 2 * r ^ 2) * r =
      ((a1 + b1 * r) * (a2 - b2 * r)) / ((a2 + b2 * r) * (a2 - b2 * r)) := by
    rw [hn', div_mul_eq_mul_div, ← add_div]
    congr 1
    ring
  rw [e1, mul_div_mul_right _ _ hc]

/-- No root part, or a zero radicand: the sign is a's. -/
theorem quad_sign_rat (a b c : ℝ) (h : b = 0 ∨ c = 0) : a + b * √c = a := by
  rcases h with h | h <;> simp [h]

/-- a = 0, c > 0: the sign is b's. -/
theorem quad_sign_root {b c : ℝ} (hc : 0 < c) :
    (0 < b → 0 < 0 + b * √c) ∧ (b < 0 → 0 + b * √c < 0) := by
  have hs : 0 < √c := Real.sqrt_pos.mpr hc
  constructor <;> intro h <;> nlinarith

/-- a and b of one sign (c > 0): that sign. -/
theorem quad_sign_same {a b c : ℝ} (hc : 0 < c) :
    (0 < a → 0 < b → 0 < a + b * √c) ∧ (a < 0 → b < 0 → a + b * √c < 0) := by
  have hs : 0 < √c := Real.sqrt_pos.mpr hc
  constructor
  · intro h1 h2; nlinarith [mul_pos h2 hs]
  · intro h1 h2; nlinarith [mul_neg_of_neg_of_pos h2 hs]

/-- a < 0 < b (c > 0): positive, zero or negative as a² is below, at or above b²c. -/
theorem quad_sign_opp {a b c : ℝ} (hc : 0 < c) (ha : a < 0) (hb : 0 < b) :
    (0 < a + b * √c ↔ a ^ 2 < b ^ 2 * c) ∧ (a + b * √c = 0 ↔ a ^ 2 = b ^ 2 * c) := by
  have hs : 0 < √c := Real.sqrt_pos.mpr hc
  have hss : √c ^ 2 = c := Real.sq_sqrt hc.le
  have hbs : 0 < b * √c := mul_pos hb hs
  have e : b ^ 2 * c = (b * √c) ^ 2 := by rw [mul_pow, hss]
  rw [e]
  constructor
  · constructor
    · intro h; nlinarith
    · intro h; nlinarith
  · constructor
    · intro h; have : b * √c = -a := by linarith
      rw [this]; ring
    · intro h
      have h' : (b * √c - (-a)) * (b * √c + (-a)) = 0 := by nlinarith
      rcases mul_eq_zero.mp h' with h1 | h1
      · linarith
      · nlinarith

/-- a > 0 > b (c > 0): the mirror of `quad_sign_opp`. -/
theorem quad_sign_opp' {a b c : ℝ} (hc : 0 < c) (ha : 0 < a) (hb : b < 0) :
    (a + b * √c < 0 ↔ a ^ 2 < b ^ 2 * c) ∧ (a + b * √c = 0 ↔ a ^ 2 = b ^ 2 * c) := by
  have h := quad_sign_opp (a := -a) (b := -b) hc (by linarith) (by linarith)
  have e1 : (-a) ^ 2 = a ^ 2 := by ring
  have e2 : (-b) ^ 2 = b ^ 2 := by ring
  rw [e1, e2] at h
  constructor
  · rw [← h.1]; constructor <;> intro h' <;> linarith
  · rw [← h.2]; constructor <;> intro h' <;> linarith

end ForgeCam.Quad
