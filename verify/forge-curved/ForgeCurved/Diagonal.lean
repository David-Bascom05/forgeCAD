import ForgeCurved.ConeModel

/-!
# Coaxial arrangements: R1 outright, by squaring the coordinates

Take coordinates `u = L y` (an axis frame), and suppose the three coordinate planes `uₖ = 0` are
carriers (the two axial seams and a plane across the axis). Then every cell fixes the sign of
each `uₖ`, and on such an orthant face `pₖ = uₖ²` is a homeomorphism onto an orthant face of
`p`-space. A carrier is **diagonal** in the frame when, on each orthant face, its sign is that of
an affine function of `p`: a cylinder or sphere about the axis frame (`a u₁² + b u₂² + c u₃² + d`), a
plane across the axis (`u₃ - h`: on `u₃ > 0` its sign is that of `p₃ - h²` when `h ≥ 0`), the
coordinate planes themselves.

**`cellsConnected_of_diagonal`**: an arrangement of diagonal carriers containing the three
coordinate planes has every realised cell connected (each one is the image of a convex set under
`p ↦ (sₖ √pₖ)`). This is R1 for coaxial cylinders, spheres centred on the axis at the frame's
origin, and planes across the axis, with the two axial seams. No enumerator data is needed.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set

/-- The coordinates of a point of `R3`, by index. -/
def crd (u : R3) : Fin 3 → ℝ := ![u.1, u.2.1, u.2.2]

/-- The squares of the coordinates. -/
def sqs (u : R3) : R3 := (u.1 ^ 2, u.2.1 ^ 2, u.2.2 ^ 2)

/-- An affine function on `p`-space. -/
structure Aff where
  w₁ : ℝ
  w₂ : ℝ
  w₃ : ℝ
  d : ℝ

def Aff.eval (α : Aff) (p : R3) : ℝ := α.w₁ * p.1 + α.w₂ * p.2.1 + α.w₃ * p.2.2 + α.d

theorem convex_aff (α : Aff) (σ : Sgn) : Convex ℝ {p : R3 | sgnK (α.eval p) = σ} := by
  intro p hp q hq a b ha hb hab
  have e : α.eval (a • p + b • q) = a • α.eval p + b • α.eval q := by
    simp only [Aff.eval, Prod.fst_add, Prod.snd_add, Prod.smul_fst, Prod.smul_snd, smul_eq_mul]
    linear_combination (-α.d) * hab
  show sgnK (α.eval (a • p + b • q)) = σ
  rw [e]
  exact convex_sgnK σ hp hq ha hb hab

/-- The signs of the coordinates. -/
noncomputable def sgns (u : R3) : Fin 3 → Sgn := fun k => sgnK (crd u k)

/-- **Diagonal in the frame `L`**: on each orthant face, the carrier's sign is that of an affine
function of the squared coordinates. -/
def DiagonalIn (L : R3 ≃ₜ R3) (c : Carrier R3 ℝ) : Prop :=
  ∀ s : Fin 3 → Sgn, ∃ α : Aff, ∀ y, sgns (L y) = s → sgnK (c y) = sgnK (α.eval (sqs (L y)))

/-- A coordinate, as an affine function. -/
def coordAff : Fin 3 → Aff
  | 0 => ⟨1, 0, 0, 0⟩
  | 1 => ⟨0, 1, 0, 0⟩
  | 2 => ⟨0, 0, 1, 0⟩

theorem coordAff_eval (k : Fin 3) (p : R3) : (coordAff k).eval p = crd p k := by
  fin_cases k <;> simp [coordAff, Aff.eval, crd]

/-- Back from the squares, on the orthant face `s`. -/
noncomputable def unsq3 (s : Fin 3 → Sgn) (p : R3) : R3 :=
  (unsq (s 0) p.1, unsq (s 1) p.2.1, unsq (s 2) p.2.2)

theorem continuous_unsq3 (s : Fin 3 → Sgn) : Continuous (unsq3 s) := by
  unfold unsq3 unsq; fun_prop

theorem crd_unsq3 (s : Fin 3 → Sgn) (p : R3) (k : Fin 3) :
    crd (unsq3 s p) k = unsq (s k) (crd p k) := by
  fin_cases k <;> rfl

theorem crd_sqs (u : R3) (k : Fin 3) : crd (sqs u) k = crd u k ^ 2 := by
  fin_cases k <;> rfl

theorem ext_crd {u v : R3} (h : ∀ k, crd u k = crd v k) : u = v := by
  have h0 := h 0; have h1 := h 1; have h2 := h 2
  simp only [crd, Matrix.cons_val_zero, Matrix.cons_val_one, Matrix.cons_val_two,
    Matrix.head_cons, Matrix.tail_cons] at h0 h1 h2
  exact Prod.ext h0 (Prod.ext h1 h2)

/-- **R1 for coaxial arrangements**: diagonal carriers, with the three coordinate planes among
them, have every realised cell connected. -/
theorem cellsConnected_of_diagonal {cs : List (Carrier R3 ℝ)} (L : R3 ≃ₜ R3)
    (hcoord : ∀ k : Fin 3, ∃ i : Fin cs.length, ∀ y, sgnK (cs[i] y) = sgnK (crd (L y) k))
    (hdiag : ∀ c ∈ cs, DiagonalIn L c) : CellsConnected cs := by
  intro Y ⟨y0, hy0⟩
  have hy0' : covOfK cs y0 = Y := hy0
  have hlen : Y.length = cs.length := by rw [← hy0', length_covOfK]
  set s := sgns (L y0)
  choose ik hik using hcoord
  -- the coordinate signs are constant on the cell
  have hs : ∀ y ∈ cell cs Y, sgns (L y) = s := by
    intro y hy
    have hy' : covOfK cs y = Y := hy
    funext k
    show sgnK (crd (L y) k) = sgnK (crd (L y0) k)
    rw [← hik k y, ← hik k y0]
    have e : (covOfK cs y)[(ik k).1]'(by rw [length_covOfK]; exact (ik k).2) =
        (covOfK cs y0)[(ik k).1]'(by rw [length_covOfK]; exact (ik k).2) := by
      simp only [hy', hy0']
    rw [getElem_covOfK, getElem_covOfK] at e
    exact e
  choose α hα using fun i : Fin cs.length => hdiag cs[i] (List.getElem_mem i.2) s
  set C : Set R3 := (⋂ k : Fin 3, {p | sgnK ((coordAff k).eval p) = sq (s k)}) ∩
    ⋂ i : Fin cs.length, {p | sgnK ((α i).eval p) = Y[i.1]'(by rw [hlen]; exact i.2)}
  have hC : Convex ℝ C :=
    (convex_iInter fun k => convex_aff _ _).inter (convex_iInter fun i => convex_aff _ _)
  have he : cell cs Y = L.symm '' (unsq3 s '' C) := by
    ext y
    constructor
    · intro hy
      have hy' : covOfK cs y = Y := hy
      have hsy := hs y hy
      refine ⟨L y, ⟨sqs (L y), ⟨?_, ?_⟩, ?_⟩, L.symm_apply_apply y⟩
      · refine mem_iInter.mpr fun k => ?_
        show sgnK ((coordAff k).eval (sqs (L y))) = sq (s k)
        rw [coordAff_eval, crd_sqs]
        exact sgnK_sq (congrFun hsy k)
      · refine mem_iInter.mpr fun i => ?_
        show sgnK ((α i).eval (sqs (L y))) = _
        rw [← hα i y hsy]
        have : (covOfK cs y)[i.1]'(by rw [length_covOfK]; exact i.2) = Y[i.1]'(by rw [hlen]; exact i.2) := by
          simp only [hy']
        rw [getElem_covOfK] at this
        exact this
      · refine ext_crd fun k => ?_
        rw [crd_unsq3, crd_sqs]
        exact unsq_sq (congrFun hsy k)
    · rintro ⟨u, ⟨p, ⟨hp1, hp2⟩, rfl⟩, rfl⟩
      have hp1' : ∀ k, sgnK (crd p k) = sq (s k) := fun k => by
        have := mem_iInter.mp hp1 k
        simp only [mem_ofPred_eq, coordAff_eval] at this
        exact this
      have hu : L (L.symm (unsq3 s p)) = unsq3 s p := L.apply_symm_apply _
      have hsu : sgns (L (L.symm (unsq3 s p))) = s := by
        rw [hu]; funext k
        show sgnK (crd (unsq3 s p) k) = s k
        rw [crd_unsq3]; exact sgnK_unsq (hp1' k)
      have hsq : sqs (L (L.symm (unsq3 s p))) = p := by
        rw [hu]
        refine ext_crd fun k => ?_
        rw [crd_sqs, crd_unsq3]; exact sq_unsq (hp1' k)
      rw [mem_cell]
      apply List.ext_getElem (by rw [length_covOfK, hlen])
      intro i hi hi'
      have hic : i < cs.length := by rwa [length_covOfK] at hi
      rw [getElem_covOfK]
      have := hα ⟨i, hic⟩ _ hsu
      simp only [Fin.getElem_fin] at this
      rw [this, hsq]
      exact mem_iInter.mp hp2 ⟨i, hic⟩
  rw [he]
  exact (hC.isPreconnected.image _ (continuous_unsq3 s).continuousOn).image _
    L.symm.continuous.continuousOn

end ForgeCurved
