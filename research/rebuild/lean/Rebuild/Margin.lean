/-
The certificate: when may a drag step be sure two things have not come to touch, and how big must
the margin be for the check itself to be right in floating point?
-/
import Mathlib.Analysis.SpecialFunctions.Pow.Real
import Mathlib.Data.Real.Basic

namespace Rebuild

/-- Two intervals, one on each side of a gap g, moving at most La and Lb per unit parameter: for a
step dp with (La + Lb) * dp < g nothing can have closed the gap. Needed: nothing but the speeds. -/
theorem gap_holds {a b g La Lb dp a' b' : ℝ}
    (hg : a + g ≤ b) (ha : a' ≤ a + La * dp) (hb : b - Lb * dp ≤ b')
    (hs : (La + Lb) * dp < g) : a' < b' := by
  nlinarith

/-- A one-sided box test, computed with rounding: the computed `a` is within `ea` of the true one,
and so on. If the computed gap exceeds the margin the true gap exceeds the tolerance. -/
theorem box_clear {a b A B ea eb tol m : ℝ}
    (ha : |A - a| ≤ ea) (hb : |B - b| ≤ eb)
    (hm : tol + ea + eb ≤ m) (hc : A + m < B) : a + tol < b := by
  have h1 := (abs_le.mp ha).1
  have h2 := (abs_le.mp hb).2
  have h3 := (abs_le.mp ha).2
  have h4 := (abs_le.mp hb).1
  linarith

/-- The rounding budget: coordinates up to M rounded once each (relative u) and compared after one
add: the error `ea + eb` is under 4 u M (1 + u), so the margin LINEAR + 4 u M (1 + u) is enough.
At M = 1e6 mm (the model box) and u = 2^-53, 4 u M < 4.5e-10 mm: under LINEAR = 1e-5 mm. -/
theorem budget : (4 : ℝ) * (2:ℝ)^(-53:ℤ) * 1e6 * (1 + (2:ℝ)^(-53:ℤ)) < 1e-5 := by
  norm_num [zpow_neg]

/-- A vertex where three planes meet with one moving at unit speed moves at |n1 × n2| / |det N|:
SymPy (`topo_range.py` item 4) shows it, independent of the parameter. As the Lipschitz constant:
for the linear motion v(p) = v0 + p w, the movement over dp is exactly |w| dp. -/
theorem linear_motion {v0 w dp : ℝ} : |(v0 + dp * w) - v0| = |w| * |dp| := by
  rw [add_sub_cancel_left, abs_mul, mul_comm]

end Rebuild
