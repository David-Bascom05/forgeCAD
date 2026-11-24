/-
The staged stock skipped (interface/forge-cam-lang `post`, `stock_needed`): a body that stays
at least c above a height T is at least c from any set lying at or under T. forge-cam-lang
checks, for every move, that each body the stock is checked against has its lowest point
(the tip's least height plus the body's z0) at least the clearance above the stock's top; the
in-process stock lies under its box's top, so those checks (limit at most the clearance) hold
and the stock is not built for them.
-/
import ForgeCam.Hypograph

namespace ForgeCam

/-- **Separated by height.** -/
theorem apart_of_heights {p q : E3} {T c : ℝ} (hp : T + c ≤ p 2) (hq : q 2 ≤ T) :
    c ≤ dist p q := by
  have h1 : |p 2 - q 2| ≤ dist p q := by
    rw [EuclideanSpace.dist_eq]
    calc |p 2 - q 2| = √((p 2 - q 2) ^ 2) := (Real.sqrt_sq_eq_abs _).symm
      _ ≤ √(∑ i, dist (p i) (q i) ^ 2) := by
        apply Real.sqrt_le_sqrt
        have : (p 2 - q 2) ^ 2 = dist (p 2) (q 2) ^ 2 := by
          rw [Real.dist_eq, sq_abs]
        rw [this]
        exact Finset.single_le_sum (f := fun i => dist (p i) (q i) ^ 2)
          (fun i _ => sq_nonneg _) (Finset.mem_univ 2)
  have : c ≤ p 2 - q 2 := by linarith
  exact this.trans ((le_abs_self _).trans h1)

/-- **So a body above the stock's top by c is c from the stock** (every point of each). -/
theorem setDist_of_heights {B S : Set E3} {T c : ℝ} (hB : ∀ p ∈ B, T + c ≤ p 2)
    (hS : ∀ q ∈ S, q 2 ≤ T) : ∀ p ∈ B, ∀ q ∈ S, c ≤ dist p q :=
  fun p hp q hq => apart_of_heights (hB p hp) (hS q hq)

end ForgeCam
