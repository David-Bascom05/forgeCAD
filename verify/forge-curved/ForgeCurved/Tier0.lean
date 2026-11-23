import ForgeCurved.Transversal
import ForgeCurved.Faces
import ForgeCurved.Strata
import ForgeCurved.Linked
import ForgeCurved.Homogeneous
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

/-- The same, with R1 by faces (Faces.lean): every realised cell is connected outright or has
linked faces and no closed piece. -/
def Tier0FaceSpec (cs : List (Carrier R3 ℝ)) : Prop :=
  Transversal cs ∧ ∀ Y, RealisedK cs Y → FaceCert cs Y

/-- **R1 by faces suffices**, for `C¹` carriers. -/
theorem tier0Spec_of_faces {cs : List (Carrier R3 ℝ)} (hd : ∀ c ∈ cs, ContDiff ℝ 1 c)
    (h : Tier0FaceSpec cs) : Tier0Spec cs :=
  ⟨h.1, cellsConnected_of_faces (fun c hc => (hd c hc).continuous)
    (fun x => (localGood'_of_transversal hd (h.1 x)).toLocalGood) h.2⟩

/-- **What the enumerator must hand Tier 0**, for the seamed arrangement `cs`: the realised
covectors (its covering theorem), transversality, the cells it certifies connected outright (a
vertex found to be one point, a whole conic with nothing on it such as a sphere's equator), and for
every other lower cell that each component of its zero locus reaches another carrier. -/
structure Tier0Data (cs : List (Carrier R3 ℝ)) where
  /-- the realised covectors -/
  R : List Cov
  covers : ∀ X, RealisedK cs X ↔ X ∈ R
  /-- every point is transversal, or conical (a cone apex with its seams) -/
  pts : ∀ x, TransversalAt cs x ∨ ConicalAt cs x
  /-- the cells certified connected outright -/
  direct : Cov → Bool
  direct_spec : ∀ Y ∈ R, direct Y = true → IsPreconnected (cell cs Y)
  locus : ∀ Y ∈ R, Sgn.zero ∈ Y → direct Y = false →
    ∀ x ∈ zeroLocus cs Y, ∃ y ∈ connectedComponentIn (zeroLocus cs Y) x,
      ∃ i, ∃ (hi : i < cs.length) (hi' : i < Y.length), Y[i] ≠ .zero ∧ cs[i] y = 0
  wall : ∃ x, wallK cs x

/-- **The Tier-0 face check**, on the enumerator's data: every realised cell is certified
connected or has linked faces. -/
def tier0B {cs : List (Carrier R3 ℝ)} (D : Tier0Data cs) : Bool :=
  D.R.all fun Y => D.direct Y || linkedB D.R Y

/-- **The face check is sound**: with the data's facts it gives `Tier0FaceSpec`. -/
theorem cellsConnected_of_check {cs : List (Carrier R3 ℝ)} (hd : ∀ c ∈ cs, ContDiff ℝ 1 c)
    (D : Tier0Data cs) (h : tier0B D = true) : CellsConnected cs := by
  have hc : ∀ c ∈ cs, Continuous c := fun c hc => (hd c hc).continuous
  refine cellsConnected_of_faces hc (fun x => ?_) fun Y hY => ?_
  · rcases D.pts x with h | h
    · exact (localGood'_of_transversal hd h).toLocalGood
    · exact localGood_of_homogeneous hc h.hom h.cone
  have hYR := (D.covers Y).mp hY
  have hlen : Y.length = cs.length := by
    obtain ⟨x, hx⟩ := hY
    have hx' : covOfK cs x = Y := hx
    rw [← hx', length_covOfK]
  by_cases hp : D.direct Y = true
  · exact Or.inl (D.direct_spec Y hYR hp)
  · have hl : linkedB D.R Y = true :=
      (Bool.or_eq_true_iff.mp (List.all_eq_true.mp h Y hYR)).resolve_left hp
    right
    refine ⟨facesLinked_of_linkedB D.covers hl, ?_⟩
    by_cases hz : Sgn.zero ∈ Y
    · exact noClosedPiece_of_locus hc hlen (D.locus Y hYR hz (by simpa using hp))
    · exact noClosedPiece_full hc D.wall hz

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

/-- **Tier 0 for Q1, on the enumerator's data**: a check that, when it passes, comes with
`Tier0Data` for the seamed arrangement on which `tier0B` passes, is sound. Cone apexes are
allowed, as conical points (Homogeneous.lean). -/
theorem qRule_tier0Sound_of_data (check : List QFace → Bool)
    (walls : List QFace → List (Carrier R3 ℝ)) (Dom : List QFace → Prop)
    (h : ∀ fs, Dom fs → check fs = true →
      ∃ D : Tier0Data ((qRule check walls).carriers fs ++ (qRule check walls).seams fs),
        tier0B D = true) :
    Tier0Sound (qRule check walls) Dom := by
  intro fs hfs hc
  obtain ⟨D, hD⟩ := h fs hfs hc
  have hd : ∀ c ∈ (qRule check walls).carriers fs ++ (qRule check walls).seams fs,
      ContDiff ℝ 1 c := by
    intro c hc
    rcases List.mem_append.mp hc with h | h
    · obtain ⟨f, -, rfl⟩ := List.mem_map.mp h; exact qR_contDiff f.quadric
    · obtain ⟨π, -, rfl⟩ := List.mem_map.mp h; exact planeR_contDiff π
  exact faithfulAll_of_mixed hd D.pts (cellsConnected_of_check hd D hD)

/-! The face check on two crossing planes (all nine covectors realised): the quadrant's faces,
two half-planes and their common line, are linked; a list with the line missing is not. -/
example : linkedB [[.pos, .pos], [.zero, .pos], [.pos, .zero], [.zero, .zero]] [.pos, .pos] = true := by
  decide
example : linkedB [[.pos, .pos], [.zero, .pos], [.pos, .zero]] [.pos, .pos] = false := by
  decide

/-- A vertex found to be one point is certified connected. -/
theorem preconnected_of_point {cs : List (Carrier R3 ℝ)} {Y : Cov}
    (h : ∀ x y, x ∈ cell cs Y → y ∈ cell cs Y → x = y) : IsPreconnected (cell cs Y) :=
  Set.Subsingleton.isPreconnected fun x hx y hy => h x y hx hy

end ForgeCurved
