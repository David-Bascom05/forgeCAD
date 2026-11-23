import ForgeCurved.Separators
import ForgeCurved.SweepBound
import ForgeExact.DecideOnce.QBound
import Mathlib.Topology.Order.IntermediateValue
import Mathlib.Topology.Algebra.Order.Field

/-!
# ℝ is a model of forge-exact's reals

forge-exact's exact algebra is proven over any `Model K`: an ordered field with `ℚ` embedded in
order and the intermediate value property for rational polynomials. `realModel` is ℝ as one.
Every sampler theorem of forge-exact (`samplesOf_realised`, `eventsOf_complete`, …) therefore holds
over ℝ, where LemmaD's sweep lemmas live.

* `realModel`, `realModel_q`: the model; its rationals are the casts;
* `evK_real`: a quadric's model value is its real carrier `qR` (Separators.lean).
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Alg ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved

/-- The cast `ℚ → ℝ`, as a ring hom of forge-exact's kind. -/
def ratHom : Hom ℚ ℝ where
  f := fun q => (q : ℝ)
  map_add := fun a b => by push_cast; ring
  map_mul := fun a b => by push_cast; ring
  map_neg := fun a => by push_cast; ring
  map_zero := by simp
  map_one := by simp

theorem evalWith_continuous (p : Poly) : Continuous fun x : ℝ => evalWith ratHom p x := by
  induction p with
  | nil => simp [evalWith]; exact continuous_const
  | cons a p ih =>
    simp only [evalWith]
    exact continuous_const.add (continuous_id.mul ih)

/-- **ℝ, as a model of the reals.** -/
noncomputable def realModel : Model ℝ where
  ι := ratHom
  lt := fun a b h => by simp [ratHom]; exact_mod_cast h
  ivt := fun p a b hab ha hb => by
    obtain ⟨x, hx, h0⟩ := intermediate_value_Icc hab (evalWith_continuous p).continuousOn
      ⟨ha, hb⟩
    exact ⟨x, hx.1, hx.2, h0⟩

@[simp] theorem realModel_q (a : ℚ) : realModel.q a = (a : ℝ) := rfl

/-- **A quadric's model value is its real carrier.** -/
theorem evK_real (q : Quad.Quadric) (p : R3) : ForgeExact.DecideOnce.Curved.Quadric.evK realModel q p = qR q p := by
  simp [ForgeExact.DecideOnce.Curved.Quadric.evK, qR]

theorem carriersOf_real (qs : List Quad.Quadric) :
    carriersOf realModel qs = qs.map qR := by
  simp only [carriersOf]
  exact List.map_congr_left fun q _ => funext fun p => evK_real q p

end ForgeCurved
