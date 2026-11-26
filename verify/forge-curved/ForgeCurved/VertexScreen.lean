import ForgeCurved.Transversal

/-!
# (S3) A certified vertex is a transversal point: route B's local model there

curved-builder-lean.md S3. The builder isolates a vertex in a box and certifies, with proven
enclosures (or by the per-question exact answer):
* which carriers vanish at the vertex: the named ones by construction (S0), the others by a sign
  screen over the box that excludes 0;
* that the named carriers' gradients are independent: for three carriers in ℝ³, a determinant
  enclosure excluding 0.

Then:
* **`transversalAt_of_screen`**: the vertex is a transversal point of the carriers;
* **`localGood_of_screen`**: route B's `LocalGood'` holds there, which gives `LocalConn` for the
  `VertModel` (FaceModel.lean);
* **`det3_independent`**: a nonzero 3×3 determinant (`det3`) of three functionals on `ℝ × ℝ × ℝ` makes
  them linearly independent. This is the screen's fact in the form the first lemma takes.
-/

namespace ForgeCurved.VertexScreen

open ForgeCurved ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E] [FiniteDimensional ℝ E]

/-- **The screened vertex is transversal**: the carriers vanishing at `x` are exactly those with
`I`, and their gradients are independent. -/
theorem transversalAt_of_screen {cs : List (Carrier E ℝ)} {x : E} (I : Fin cs.length → Prop)
    (hz : ∀ i, cs[i] x = 0 ↔ I i)
    (hli : LinearIndependent ℝ fun i : {i // I i} => fderiv ℝ cs[i.1] x) : TransversalAt cs x := by
  apply transversalAt_of_linearIndependent
  let e : Through cs x ≃ {i // I i} :=
    { toFun := fun t => ⟨t.1, (hz t.1).mp t.2⟩
      invFun := fun t => ⟨t.1, (hz t.1).mpr t.2⟩
      left_inv := fun _ => rfl
      right_inv := fun _ => rfl }
  have h2 := (linearIndependent_equiv e (f := fun i : {i // I i} => fderiv ℝ cs[i.1] x)).mpr hli
  exact h2

/-- **Route B's local model at a screened vertex.** -/
theorem localGood_of_screen {cs : List (Carrier E ℝ)} (hd : ∀ c ∈ cs, ContDiff ℝ 1 c) {x : E}
    (I : Fin cs.length → Prop) (hz : ∀ i, cs[i] x = 0 ↔ I i)
    (hli : LinearIndependent ℝ fun i : {i // I i} => fderiv ℝ cs[i.1] x) : LocalGood' cs x :=
  localGood'_of_transversal hd (transversalAt_of_screen I hz hli)

/-- The 3×3 determinant of three functionals on the standard basis. -/
def det3 (L : Fin 3 → (ℝ × ℝ × ℝ) →L[ℝ] ℝ) : ℝ :=
  L 0 (1, 0, 0) * (L 1 (0, 1, 0) * L 2 (0, 0, 1) - L 1 (0, 0, 1) * L 2 (0, 1, 0)) -
    L 0 (0, 1, 0) * (L 1 (1, 0, 0) * L 2 (0, 0, 1) - L 1 (0, 0, 1) * L 2 (1, 0, 0)) +
    L 0 (0, 0, 1) * (L 1 (1, 0, 0) * L 2 (0, 1, 0) - L 1 (0, 1, 0) * L 2 (1, 0, 0))

/-- **A nonzero determinant makes three functionals independent.** -/
theorem det3_independent (L : Fin 3 → (ℝ × ℝ × ℝ) →L[ℝ] ℝ) (hdet : det3 L ≠ 0) :
    LinearIndependent ℝ L := by
  rw [Fintype.linearIndependent_iff]
  intro g hg
  have ev : ∀ v : ℝ × ℝ × ℝ, g 0 * L 0 v + g 1 * L 1 v + g 2 * L 2 v = 0 := by
    intro v
    have := congrArg (fun φ : (ℝ × ℝ × ℝ) →L[ℝ] ℝ => φ v) hg
    simpa [Fin.sum_univ_three] using this
  have c0 := ev (1, 0, 0)
  have c1 := ev (0, 1, 0)
  have c2 := ev (0, 0, 1)
  unfold det3 at hdet
  have e0 : g 0 * det3 L = 0 := by
    unfold det3
    linear_combination (L 1 (0, 1, 0) * L 2 (0, 0, 1) - L 1 (0, 0, 1) * L 2 (0, 1, 0)) * c0 -
      (L 1 (1, 0, 0) * L 2 (0, 0, 1) - L 1 (0, 0, 1) * L 2 (1, 0, 0)) * c1 +
      (L 1 (1, 0, 0) * L 2 (0, 1, 0) - L 1 (0, 1, 0) * L 2 (1, 0, 0)) * c2
  have e1 : g 1 * det3 L = 0 := by
    unfold det3
    linear_combination -(L 0 (0, 1, 0) * L 2 (0, 0, 1) - L 0 (0, 0, 1) * L 2 (0, 1, 0)) * c0 +
      (L 0 (1, 0, 0) * L 2 (0, 0, 1) - L 0 (0, 0, 1) * L 2 (1, 0, 0)) * c1 -
      (L 0 (1, 0, 0) * L 2 (0, 1, 0) - L 0 (0, 1, 0) * L 2 (1, 0, 0)) * c2
  have e2 : g 2 * det3 L = 0 := by
    unfold det3
    linear_combination (L 0 (0, 1, 0) * L 1 (0, 0, 1) - L 0 (0, 0, 1) * L 1 (0, 1, 0)) * c0 -
      (L 0 (1, 0, 0) * L 1 (0, 0, 1) - L 0 (0, 0, 1) * L 1 (1, 0, 0)) * c1 +
      (L 0 (1, 0, 0) * L 1 (0, 1, 0) - L 0 (0, 1, 0) * L 1 (1, 0, 0)) * c2
  have hd : det3 L ≠ 0 := by unfold det3; exact hdet
  intro i
  fin_cases i
  · exact (mul_eq_zero.mp e0).resolve_right hd
  · exact (mul_eq_zero.mp e1).resolve_right hd
  · exact (mul_eq_zero.mp e2).resolve_right hd

end ForgeCurved.VertexScreen
