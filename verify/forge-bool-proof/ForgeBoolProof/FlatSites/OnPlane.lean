import Mathlib.Data.Matrix.Mul
import Mathlib.Tactic.Ring

/-!
# §1a fact 1, as classify.rs's `on_plane` decides it

`classify`'s `On` branch now also requires `on_carrier`: for a plane face, `on_plane(pl, p)`, which
decides `n·(p − o) = 0` with no rounding:
* each `d_i = p_i ⊖ o_i` with TwoSum's error `e_i`. When every `e_i = 0`, `d_i = p_i − o_i` exactly
  (verify/ieee754 TwoSum.lean `two_sum_exact`: `s + e = a + b`), and `dot3(n, d)` has the exact
  sign of `n·d` (forge-math's proven predicate, L5), so `dot3 = 0 ↔ n·d = 0`;
* otherwise `on_plane_exact` evaluates `n·p − n·o` in exact dyadics (`DyPlane::side`).

`on_plane_iff` is the algebra that joins the first branch to the carrier: with exact differences,
`n·d = 0` iff `p` is on the plane `n·x = n·o`. With it, `classify_on_sound`'s hypothesis
`f.carrier p = 0` is what the code checks, for every plane face.
-/

namespace ForgeBoolProof.FlatSites.OnPlane

open Matrix

theorem on_plane_iff {n p o d : Fin 3 → ℚ} (hd : ∀ i, d i = p i - o i) :
    n ⬝ᵥ d = 0 ↔ n ⬝ᵥ p = n ⬝ᵥ o := by
  have : d = p - o := funext fun i => by simp [hd i]
  rw [this, dotProduct_sub, sub_eq_zero]

/-- TwoSum's error-free form, read on the rationals: with no error the difference is exact. -/
theorem exact_of_no_error {s e a b : ℚ} (h : s + e = a + b) (he : e = 0) : s = a + b := by
  rw [he, add_zero] at h; exact h

end ForgeBoolProof.FlatSites.OnPlane
