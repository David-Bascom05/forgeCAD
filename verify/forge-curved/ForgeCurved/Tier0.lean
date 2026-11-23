import ForgeCurved.Transversal
import Mathlib.Analysis.Calculus.ContDiff.Operations

/-!
# Tier 0: what a passed check must certify

`Tier0Sound` (Separators.lean) asks a passed check to make the seamed arrangement faithful. By
Local.lean and Transversal.lean, R2–R5 are local, and at transversal points they hold. So:

* **`tier0Sound_of_spec`**: for any rule whose carriers and seams are `C¹`, a check that certifies
  transversality of the seamed arrangement and R1 (`Tier0Spec`) is sound;
* **`qRule_tier0Sound`**: for the Q1 rule (rational quadrics, planar seams, all `C^∞`) that is
  all there is to show.

What is left of Tier 0 for Q1 is therefore the two halves of `Tier0Spec`, each a statement about
the check alone:
* transversality, an algebraic fact about the carriers (no point on two carriers with parallel
  gradients, on three with dependent ones, or a singular point on a carrier); and
* R1, the global, Jordan-type part: every realised cell is connected.
A check that also accepts non-transversal points (a cone apex, a tangent contact line) needs a
local model there (`LocalGood`, Local.lean) in place of the chart.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved ForgeExact.QRef

/-- What a passed Tier-0 check certifies, as semantics: the seamed arrangement is transversal
everywhere and its realised cells are connected. -/
def Tier0Spec (cs : List (Carrier R3 ℝ)) : Prop := Transversal cs ∧ CellsConnected cs

variable {ι : Type}

/-- **Tier 0 reduces to `Tier0Spec`**, for `C¹` carriers and seams. -/
theorem tier0Sound_of_spec {r : SepRule ι} {Dom : List ι → Prop}
    (he : ∀ f, ContDiff ℝ 1 (r.eval f)) (hs : ∀ fs, ∀ c ∈ r.seams fs, ContDiff ℝ 1 c)
    (h : ∀ fs, Dom fs → r.check fs = true → Tier0Spec (r.carriers fs ++ r.seams fs)) :
    Tier0Sound r Dom := by
  intro fs hfs hc
  obtain ⟨ht, h1⟩ := h fs hfs hc
  refine faithfulAll_of_transversal (fun c hc => ?_) ht h1
  rcases List.mem_append.mp hc with h | h
  · obtain ⟨f, -, rfl⟩ := List.mem_map.mp h; exact he f
  · exact hs fs c h

theorem qR_contDiff (q : ForgeExact.Quad.Quadric) {n : WithTop ℕ∞} : ContDiff ℝ n (qR q) := by
  unfold qR; fun_prop

theorem planeR_contDiff (π : Ref.Plane) {n : WithTop ℕ∞} : ContDiff ℝ n (planeR π) := by
  unfold planeR; fun_prop

/-- **Tier 0 for Q1 is `Tier0Spec`**: whatever the check, it is sound as soon as it certifies a
transversal seamed arrangement with connected cells. -/
theorem qRule_tier0Sound (check : List QFace → Bool) (walls : List QFace → List (Carrier R3 ℝ))
    (Dom : List QFace → Prop)
    (h : ∀ fs, Dom fs → check fs = true →
      Tier0Spec ((qRule check walls).carriers fs ++ (qRule check walls).seams fs)) :
    Tier0Sound (qRule check walls) Dom :=
  tier0Sound_of_spec (fun f => qR_contDiff f.quadric)
    (fun fs c hc => by
      obtain ⟨π, -, rfl⟩ := List.mem_map.mp hc; exact planeR_contDiff π) h

end ForgeCurved
