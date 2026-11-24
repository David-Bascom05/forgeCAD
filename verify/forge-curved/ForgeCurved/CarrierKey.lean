import Mathlib.Basic.Real.Basic
import Mathlib.Data.Matrix.Mul
import Mathlib.Tactic.Abel
import Mathlib.Tactic.Ring
import Mathlib.Tactic.LinearCombination

/-!
# One surface by exact keys: `ssi::coincident`'s `true`, proven where a key holds

forge-bool decides "two faces lie on one surface" with tolerances (`ssi::coincident`,
`unify::same_surface`, `planes_coincident`, `cylinders_coincident`). Measured on the gate parts
and fuzz 1–40 (curved-sites), every `true` has measure exactly 0, and all but 36 one-ulp pattern
copies in patterns seq satisfy an exact key on the stored data (`ssi::same_carrier`, and its
general plane form here). This file proves each key: the two stored surfaces are the same point
set, read exactly as the stored f64 data (exactOf's reading, BodyModel).

* `plane_key`: planes `{x | n ⬝ (x − o) = 0}` with `n' = c • n` (c ≠ 0) and `n ⬝ (o' − o) = 0`
  are one set. The Rust key tests exactly these two facts with error-free arithmetic (the cross
  product's components and the dot product's sign by TwoProduct/TwoSum expansions); the
  axis-aligned case (`n = ±e_k`, `o_k = o'_k`) is an instance.
* `cyl_key`: two cylinders along one coordinate axis with equal radii and equal origins in the
  other two coordinates are one set (axis-aligned, as `same_carrier`).
* `sphere_key`: equal centres and radii (immediate).

The 36 unkeyed `true`s are not covered: exactly, their carriers are distinct planes 1e-15 to 1e-13
apart, so the merged answer is right only under an input hypothesis (FaithfulInput with keyed
carriers, or provenance keys). They stay UNKNOWN in the ledger (C.coinc.unkeyed).
-/

namespace ForgeCurved.CarrierKey

open Matrix

abbrev V3 := Fin 3 → ℝ

/-- A plane through `o` with normal `n` (not necessarily unit: the stored f64 normal, exactly). -/
def planeSet (n o : V3) : Set V3 := {x | n ⬝ᵥ (x - o) = 0}

/-- **The plane key.** A nonzero multiple of the normal, and the second origin on the first
plane: one set. -/
theorem plane_key {n o n' o' : V3} {c : ℝ} (hc : c ≠ 0) (hn : n' = c • n)
    (ho : n ⬝ᵥ (o' - o) = 0) : planeSet n' o' = planeSet n o := by
  ext x
  simp only [planeSet, Set.mem_ofPred_eq, hn, smul_dotProduct, smul_eq_mul]
  have hx : n ⬝ᵥ (x - o') = n ⬝ᵥ (x - o) - n ⬝ᵥ (o' - o) := by
    rw [← dotProduct_sub]
    congr 1
    abel
  rw [hx, ho, sub_zero]
  constructor
  · intro h
    exact (mul_eq_zero.mp h).resolve_left hc
  · intro h
    rw [h, mul_zero]

/-- The axis-aligned instance (`same_carrier`'s plane case): normals `±e_k`, equal `k`-th
coordinates of the origins. -/
theorem plane_key_axis {n o n' o' : V3} {k : Fin 3} {s : ℝ}
    (hn : n = Pi.single k 1) (hn' : n' = s • Pi.single k 1) (hs : s ≠ 0) (hk : o' k = o k) :
    planeSet n' o' = planeSet n o := by
  refine plane_key hs (by rw [hn', hn]) ?_
  rw [hn, single_dotProduct, one_mul, Pi.sub_apply, hk, sub_self]

/-- A cylinder along coordinate axis `k` (its other two coordinates `u`, `v`): the points at
radius `r` from the axis line through `o`. -/
def cylSet (u v : Fin 3) (o : V3) (r : ℝ) : Set V3 :=
  {x | (x u - o u) ^ 2 + (x v - o v) ^ 2 = r ^ 2}

/-- **The cylinder key.** Equal radii, origins equal in the two coordinates across the axis. -/
theorem cyl_key {u v : Fin 3} {o o' : V3} {r r' : ℝ} (hr : r' = r) (hu : o' u = o u)
    (hv : o' v = o v) : cylSet u v o' r' = cylSet u v o r := by
  ext x
  simp only [cylSet, Set.mem_ofPred_eq, hr, hu, hv]

/-- A sphere. -/
def sphereSet (o : V3) (r : ℝ) : Set V3 :=
  {x | (x 0 - o 0) ^ 2 + (x 1 - o 1) ^ 2 + (x 2 - o 2) ^ 2 = r ^ 2}

/-- **The sphere key.** -/
theorem sphere_key {o o' : V3} {r r' : ℝ} (ho : o' = o) (hr : r' = r) :
    sphereSet o' r' = sphereSet o r := by
  rw [ho, hr]

end ForgeCurved.CarrierKey
