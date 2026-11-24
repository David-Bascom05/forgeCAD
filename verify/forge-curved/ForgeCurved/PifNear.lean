/-!
# C.pif: `point_in_face`'s nearest-edge `On` at its curved callers, where it decides nothing

forge-bool's `trim_in` (boolean.rs) asks `point_in_face` about points of a face pair's curve at
two curved sites. At a sample within `LINEAR` of the face's nearest edge it answers `On` (the
tolerance band), where the exact side of that edge could be `Inside` or `Outside`. The corpus has
nine such near-misses (curved-fallback-rates.md, re-measured on merge-train/1): fuzz40 ×4 at the
closed curve's midpoint, fuzz20 ×3, shaft ×1 and fuzz30 ×1 in `touch_inside`.

This file models the two decisions as written and proves, for each near-miss's measured
neighbourhood, that the band answer and every exact answer give the same decision:

* `whole_of_outside`: the closed curve's midpoint (`whole`): once the second face answers
  `Outside`, the result is `some false` whatever the first face answers (fuzz40 ×4);
* `touch_inside_on_eq_outside`: `touch_inside`'s step search: when the first step's far sample is
  `Inside`, the faces are not tangent along the edge (`smooth = false`) and the second step's near
  sample is `Outside`, an `On` and an `Outside` at the first near sample give the same `some false`
  (fuzz20 ×3); only `Inside` there would differ (`touch_inside_inside_differs`).

What is measured, not proven (ledger C.pif): that each near-miss's other samples are as stated
(the decision log), and that the exact side of each near sample is not `Inside` (its signed f64
distance is −1.9e-6 … −9.0e-6, far past its rounding, which is not bounded here: a hypothesis).
-/

namespace ForgeCurved.PifNear

/-- classify.rs `Where`. -/
inductive Where
  | inside
  | on
  | outside
  deriving DecidableEq, Repr

open Where

/-- boolean.rs `trim_in`, the closed curve with at most one cut: `w0` is the first face's answer
at the midpoint, `w1'` the second face's (asked only when `w0 ≠ Outside`), and `third` the
golden-section probe's decision, used only in the last arm. -/
def whole (w0 w1' : Where) (third : Option Bool) : Option Bool :=
  let w1 := if w0 = outside then outside else w1'
  if w0 = inside ∧ w1 = inside then some true
  else if w0 = outside ∨ w1 = outside then some false
  else third

/-- **The midpoint, fuzz40 ×4.** With the second face `Outside` at the midpoint, the first face's
answer (the near-miss: `On` today, the exact side otherwise) decides nothing. -/
theorem whole_of_outside (w0 : Where) (third : Option Bool) :
    whole w0 outside third = some false := by
  cases w0 <;> simp [whole]

/-- One step of `touch_inside`'s search: `a` the sample before the touch, `b` the one after,
`smooth` whether the faces are tangent along the edge. -/
def step (smooth : Bool) (a b : Where) : Option Bool :=
  if a = outside ∧ smooth = false then some false
  else
    match a, b with
    | outside, inside => some smooth
    | inside, outside => some smooth
    | outside, _ => some false
    | _, outside => some false
    | inside, inside => some true
    | _, _ => none

/-- `find_map` over the steps `[1e-3, 1e-2, 5e-2, 0.2, 0.5]`, each its two samples. -/
def search (smooth : Bool) : List (Where × Where) → Option Bool
  | [] => none
  | (a, b) :: rest =>
    match step smooth a b with
    | some r => some r
    | none => search smooth rest

/-- **`touch_inside`, fuzz20 ×3.** The first step's far sample is `Inside`, the faces are not
tangent along the edge, the second step's near sample is `Outside`: the first near sample's
`On` (today) and `Outside` (its exact side) give the same answer, `some false`. -/
theorem touch_inside_on_eq_outside (b2 : Where) (rest : List (Where × Where)) :
    search false ((on, inside) :: (outside, b2) :: rest) = some false ∧
      search false ((outside, inside) :: (outside, b2) :: rest) = some false := by
  constructor <;> simp [search, step]

/-- Only `Inside` at the first near sample would change it. -/
theorem touch_inside_inside_differs (b2 : Where) (rest : List (Where × Where)) :
    search false ((inside, inside) :: (outside, b2) :: rest) = some true := by
  simp [search, step]

/-- **`touch_inside`, shaft and fuzz30.** There the second step's samples are both `Inside`: `On`
gives `some true`, the exact `Outside` gives `some false`. The decisions differ; the bodies built
from them do not (C.pif (c): same V/E/F, validity and volume, a vertex within one ulp, a closed
edge's parameter shifted by its period), which is measured, not proven here. -/
theorem touch_inside_on_ne_outside (rest : List (Where × Where)) :
    search false ((on, inside) :: (inside, inside) :: rest) = some true ∧
      search false ((outside, inside) :: (inside, inside) :: rest) = some false := by
  constructor <;> simp [search, step]

end ForgeCurved.PifNear
