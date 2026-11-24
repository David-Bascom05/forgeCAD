/-
real.rs's square-root-free sign (`Ctx::rational_sign`, kernel/forge-cam-geom, CAM a5a40675): a
closure of +, −, ×, ÷ over f64 literals is evaluated as one exact quotient num / den of dyadics
(sums, differences and products of dyadics are exact; a quotient keeps its numerator and
denominator), and its sign is read as sign(num) · sign(den), zero included, with no refinement.
The model here: exact rationals (a dyadic is one), the evaluation's identities, and the sign of the
quotient. That the Rust Nat/dyadic operations are exact is the code = model step (owed; sampled
in tests/real_rational.rs against refinement to the separation bound).
-/
import Mathlib

namespace ForgeCam.Rational

/-- **The sign of a quotient** is the product of the signs: what `rational_sign` returns for the
value num / den (a zero denominator never reaches it: the path falls back to refinement). -/
theorem sign_div (n d : ℚ) : SignType.sign (n / d) = SignType.sign n * SignType.sign d := by
  rw [div_eq_mul_inv, sign_mul]
  congr 1
  rcases lt_trichotomy d 0 with h | h | h
  · rw [sign_neg h, sign_neg (inv_lt_zero.mpr h)]
  · simp [h]
  · rw [sign_pos h, sign_pos (inv_pos.mpr h)]

/-- The evaluation's closure as quotients: (a/b) + (c/d) = (ad + cb)/(bd), and so on for −, ×,
÷, each exact over ℚ (nonzero denominators). -/
theorem add_quot (a b c d : ℚ) (hb : b ≠ 0) (hd : d ≠ 0) :
    a / b + c / d = (a * d + c * b) / (b * d) := by
  field_simp

theorem sub_quot (a b c d : ℚ) (hb : b ≠ 0) (hd : d ≠ 0) :
    a / b - c / d = (a * d - c * b) / (b * d) := by
  field_simp

theorem mul_quot (a b c d : ℚ) : a / b * (c / d) = (a * c) / (b * d) := by
  rw [div_mul_div_comm]

theorem div_quot (a b c d : ℚ) : a / b / (c / d) = (a * d) / (b * c) := by
  rw [div_div_div_eq]

end ForgeCam.Rational
