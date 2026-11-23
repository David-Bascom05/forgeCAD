import ForgeCurved.Local
import Mathlib.Analysis.Calculus.InverseFunctionTheorem.FDeriv
import Mathlib.Analysis.Calculus.ContDiff.RCLike
import Mathlib.Analysis.Calculus.FDeriv.Prod
import Mathlib.Analysis.Normed.Module.Convex
import Mathlib.LinearAlgebra.Basis.VectorSpace
import Mathlib.LinearAlgebra.Projection
import Mathlib.Topology.Algebra.Module.FiniteDimension

/-!
# Transversal points have the flat local picture

A point `x` is **transversal** for the carriers `cs` when the derivatives at `x` of the carriers
vanishing there are jointly onto: `v ↦ (Dcᵢ(x) v)ᵢ` maps onto `ℝ^Z`, `Z` the carriers through `x`
(so at most `dim E` of them, with independent gradients). The flat arrangement is transversal
wherever no more than three of its planes meet with independent normals; a cylinder and its axial
seam are transversal; a cone apex (zero gradient) and a tangent contact line (parallel
gradients) are not.

**`localGood'_of_transversal`**: at a transversal point of `C¹` carriers, the local picture of
Local.lean holds, every cell above `x`'s being near `x`. The proof is the inverse function theorem:
`F y = ((cᵢ y)ᵢ, π (y - x))`, with `π` a projection onto the kernel of `(Dcᵢ x)ᵢ`, is a local
homeomorphism at `x`, and in its coordinates a cell near `x` is an orthant face of a small ball,
which is convex.

**`faithfulAll_of_transversal`**: so for carriers transversal everywhere, `FaithfulAll` is R1.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set

/-! ## `Cov.le` entry by entry -/

theorem le_iff_getElem {X Y : Cov} : Cov.le X Y = true ↔
    X.length = Y.length ∧ ∀ i (h₁ : i < X.length) (h₂ : i < Y.length),
      X[i] = .zero ∨ X[i] = Y[i] := by
  rw [le_iff, List.forall₂_iff_get]
  rfl

theorem getElem_covOfK {E : Type} (cs : List (Carrier E ℝ)) (y : E) (i : Nat)
    (h : i < (covOfK cs y).length) :
    (covOfK cs y)[i] = sgnK ((cs[i]'(by simpa [covOfK] using h)) y) := by
  simp [covOfK]

/-! ## The value of a sign -/

/-- A real number of the sign `s`. -/
def sgnVal : Sgn → ℝ
  | .pos => 1
  | .neg => -1
  | .zero => 0

theorem sgnK_mul_sgnVal (s : Sgn) {δ : ℝ} (hδ : 0 < δ) : sgnK (δ * sgnVal s) = s := by
  cases s
  · simp only [sgnVal, mul_neg, mul_one]; exact sgnK_neg (by linarith)
  · simp [sgnVal, sgnK]
  · simp only [sgnVal, mul_one]; exact sgnK_pos hδ

theorem abs_sgnVal_le (s : Sgn) : |sgnVal s| ≤ 1 := by
  cases s <;> simp [sgnVal]

theorem convex_sgnK (s : Sgn) : Convex ℝ {r : ℝ | sgnK r = s} := by
  cases s
  · have : {r : ℝ | sgnK r = .neg} = Iio 0 := by
      ext r; simp only [mem_ofPred_eq, mem_Iio]
      constructor
      · intro h
        rcases lt_trichotomy r 0 with h' | h' | h'
        · exact h'
        · simp [sgnK, h'] at h
        · rw [sgnK_pos h'] at h; cases h
      · exact sgnK_neg
    rw [this]; exact convex_Iio 0
  · have : {r : ℝ | sgnK r = .zero} = {0} := by ext r; simp [sgnK_zero_iff]
    rw [this]; exact convex_singleton 0
  · have : {r : ℝ | sgnK r = .pos} = Ioi 0 := by
      ext r; simp only [mem_ofPred_eq, mem_Ioi]
      constructor
      · intro h
        rcases lt_trichotomy r 0 with h' | h' | h'
        · rw [sgnK_neg h'] at h; cases h
        · simp [sgnK, h'] at h
        · exact h'
      · exact sgnK_pos
    rw [this]; exact convex_Ioi 0

/-! ## Transversality -/

variable {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E] [FiniteDimensional ℝ E]

/-- The carriers through `x`, by index. -/
def Through (cs : List (Carrier E ℝ)) (x : E) : Type := {i : Fin cs.length // cs[i] x = 0}

noncomputable instance (cs : List (Carrier E ℝ)) (x : E) : Fintype (Through cs x) := by
  classical
  unfold Through; infer_instance

/-- The derivatives at `x` of the carriers through `x`, as one linear map to `ℝ^Z`. -/
noncomputable def throughDeriv (cs : List (Carrier E ℝ)) (x : E) :
    E →L[ℝ] (Through cs x → ℝ) :=
  ContinuousLinearMap.pi fun i => fderiv ℝ cs[i.1] x

/-- **Transversal at `x`**: the derivatives of the carriers through `x` are jointly onto. -/
def TransversalAt (cs : List (Carrier E ℝ)) (x : E) : Prop :=
  Function.Surjective (throughDeriv cs x)

/-- Transversal everywhere. -/
def Transversal (cs : List (Carrier E ℝ)) : Prop := ∀ x, TransversalAt cs x

/-- **Independent gradients give transversality**: the certificate form (a nonzero gradient, a
nonzero cross product, a nonzero determinant, for one, two or three carriers through `x`). -/
theorem transversalAt_of_linearIndependent {cs : List (Carrier E ℝ)} {x : E}
    (h : LinearIndependent ℝ fun i : Through cs x => fderiv ℝ cs[i.1] x) : TransversalAt cs x := by
  classical
  unfold TransversalAt
  have key := (LinearMap.dualMap_injective_iff
    (f := ((throughDeriv cs x : E →L[ℝ] (Through cs x → ℝ)) : E →ₗ[ℝ] (Through cs x → ℝ)))).mp
  refine key ?_
  rw [← LinearMap.ker_eq_bot, LinearMap.ker_eq_bot']
  intro φ hφ
  have hsum : ∑ i : Through cs x, φ (fun j => if i = j then 1 else 0) • fderiv ℝ cs[i.1] x = 0 := by
    ext v
    have := LinearMap.congr_fun hφ v
    simp only [LinearMap.dualMap_apply, LinearMap.zero_apply] at this
    rw [LinearMap.pi_apply_eq_sum_univ] at this
    simpa [throughDeriv, mul_comm] using this
  have hz := Fintype.linearIndependent_iff.mp h _ hsum
  ext w
  simp only [LinearMap.comp_apply, LinearMap.zero_apply]
  convert hz w using 2
  ext j
  simp [Pi.single_apply, eq_comm]

/-- **The chart at a transversal point**: small neighbourhoods `U` of `x` meet every cell above
`x`'s in a nonempty preconnected set. -/
theorem transversal_chart {cs : List (Carrier E ℝ)} (hd : ∀ c ∈ cs, ContDiff ℝ 1 c) {x : E}
    (ht : TransversalAt cs x) (W : Set E) (hW : IsOpen W) (hxW : x ∈ W) :
    ∃ U : Set E, IsOpen U ∧ U ⊆ W ∧ x ∈ U ∧ ∀ Y, Cov.le (covOfK cs x) Y = true →
      IsPreconnected (cell cs Y ∩ U) ∧ (cell cs Y ∩ U).Nonempty := by
  classical
  have : CompleteSpace E := FiniteDimensional.completeSpace ℝ E
  set Z := Through cs x
  set T := throughDeriv cs x
  -- the kernel, a complement, and the projection onto the kernel
  set K := LinearMap.ker (T : E →ₗ[ℝ] (Z → ℝ))
  obtain ⟨C, hC⟩ := K.exists_isCompl
  set π : E →L[ℝ] K := LinearMap.toContinuousLinearMap (K.projectionOnto C hC)
  set D : E →L[ℝ] (Z → ℝ) × K := T.prod π
  have hDinj : Function.Injective D := by
    have key : ∀ v, D v = 0 → v = 0 := by
      intro v hv
      have h1 : T v = 0 := congrArg Prod.fst hv
      have h2 : π v = 0 := congrArg Prod.snd hv
      have hvK : v ∈ K := h1
      have : (π v : E) = v := by
        simp [π, Submodule.projectionOnto_apply_of_mem_left hC hvK]
      rw [← this, h2]; rfl
    intro v w hvw
    have := key (v - w) (by rw [map_sub, hvw, sub_self])
    exact sub_eq_zero.mp this
  have hrank : Module.finrank ℝ E = Module.finrank ℝ ((Z → ℝ) × K) := by
    rw [Module.finrank_prod]
    have := LinearMap.finrank_range_add_finrank_ker (T : E →ₗ[ℝ] (Z → ℝ))
    rw [LinearMap.range_eq_top.mpr ht, finrank_top] at this
    change Module.finrank ℝ (Z → ℝ) + Module.finrank ℝ K = _ at this
    omega
  have hDsurj : Function.Surjective D :=
    (LinearMap.injective_iff_surjective_of_finrank_eq_finrank
      (f := (D : E →ₗ[ℝ] (Z → ℝ) × K)) hrank).mp hDinj
  set D' : E ≃L[ℝ] (Z → ℝ) × K :=
    (LinearEquiv.ofBijective (D : E →ₗ[ℝ] (Z → ℝ) × K) ⟨hDinj, hDsurj⟩).toContinuousLinearEquiv
  -- the chart
  set F : E → (Z → ℝ) × K := fun y => (fun i => cs[i.1] y, π (y - x))
  have hF : HasStrictFDerivAt F (D' : E →L[ℝ] (Z → ℝ) × K) x := by
    have hD' : (D' : E →L[ℝ] (Z → ℝ) × K) = D := by ext v <;> rfl
    rw [hD']
    refine HasStrictFDerivAt.prodMk ?_ ?_
    · refine hasStrictFDerivAt_pi.mpr fun i => ?_
      have hc := hd _ (List.getElem_mem i.1.2)
      exact hc.contDiffAt.hasStrictFDerivAt
        (hc.differentiable one_ne_zero x).hasFDerivAt one_ne_zero
    · have := π.hasStrictFDerivAt.comp x ((hasStrictFDerivAt_id x).sub_const x)
      simpa using this
  set Φ := hF.toOpenPartialHomeomorph F
  have hΦ : ⇑Φ = F := hF.toOpenPartialHomeomorph_coe
  have hxs : x ∈ Φ.source := hF.mem_toOpenPartialHomeomorph_source
  have hFx : F x = 0 := by
    ext i
    · exact i.2
    · simp [F]
  -- the neighbourhood where the signs off `Z` are kept
  obtain ⟨N, hNo, hxN, hN⟩ := exists_open_le (fun c hc => (hd c hc).continuous) x
  set S := Φ.source ∩ W ∩ N
  have hSo : IsOpen S := (Φ.open_source.inter hW).inter hNo
  have hO := Φ.isOpen_inter_preimage_symm hSo
  have hFxO : F x ∈ Φ.target ∩ Φ.symm ⁻¹' S := by
    refine ⟨hΦ ▸ Φ.map_source hxs, ?_⟩
    rw [← hΦ, mem_preimage, Φ.left_inv hxs]
    exact ⟨⟨hxs, hxW⟩, hxN⟩
  obtain ⟨ε, hε, hball⟩ := Metric.isOpen_iff.mp hO (F x) hFxO
  set B := Metric.ball (F x) ε
  refine ⟨Φ.source ∩ F ⁻¹' B, ?_, ?_, ⟨hxs, show F x ∈ B from Metric.mem_ball_self hε⟩, ?_⟩
  · rw [← hΦ]; exact Φ.isOpen_inter_preimage Metric.isOpen_ball
  · intro y ⟨hys, hyB⟩
    have := (hball hyB).2
    rw [mem_preimage, ← hΦ, Φ.left_inv hys] at this
    exact this.1.2
  · intro Y hY
    obtain ⟨hlen, hent⟩ := le_iff_getElem.mp hY
    rw [length_covOfK] at hlen
    -- the cell, in chart coordinates
    set Q : Set ((Z → ℝ) × K) :=
      {z | ∀ i : Z, sgnK (z.1 i) = Y[i.1.1]'(hlen ▸ i.1.2)}
    have hQ : ∀ y ∈ Φ.source ∩ F ⁻¹' B, (y ∈ cell cs Y ↔ F y ∈ Q) := by
      intro y ⟨hys, hyB⟩
      have hyN : y ∈ N := by
        have := (hball hyB).2
        rw [mem_preimage, ← hΦ, Φ.left_inv hys] at this
        exact this.2
      obtain ⟨-, hxy⟩ := le_iff_getElem.mp (hN y hyN)
      rw [mem_cell]
      constructor
      · intro hyY i
        have : (covOfK cs y)[i.1.1]'(by rw [length_covOfK]; exact i.1.2) = Y[i.1.1]'(hlen ▸ i.1.2) :=
          by simp only [hyY]
        rw [getElem_covOfK] at this
        exact this
      · intro hq
        apply List.ext_getElem (by rw [length_covOfK, hlen])
        intro j hj hjY
        have hjn : j < cs.length := by rwa [length_covOfK] at hj
        rw [getElem_covOfK]
        by_cases hz : cs[j] x = 0
        · exact hq ⟨⟨j, hjn⟩, hz⟩
        · have hxj : (covOfK cs x)[j]'(by rw [length_covOfK]; exact hjn) ≠ .zero := by
            rw [getElem_covOfK]; exact fun h => hz (sgnK_zero_iff.mp h)
          have e1 := (hxy j (by rw [length_covOfK]; exact hjn) hj).resolve_left hxj
          have e2 := (hent j (by rw [length_covOfK]; exact hjn) hjY).resolve_left hxj
          rw [← e2, e1, getElem_covOfK]
    have himg : cell cs Y ∩ (Φ.source ∩ F ⁻¹' B) = Φ.symm '' (B ∩ Q) := by
      ext y
      constructor
      · intro ⟨hyY, hyU⟩
        refine ⟨F y, ⟨hyU.2, (hQ y hyU).mp hyY⟩, ?_⟩
        rw [← hΦ]; exact Φ.left_inv hyU.1
      · rintro ⟨z, ⟨hzB, hzQ⟩, rfl⟩
        have hzt : z ∈ Φ.target := (hball hzB).1
        have hFz : F (Φ.symm z) = z := by rw [← hΦ]; exact Φ.right_inv hzt
        have hU : Φ.symm z ∈ Φ.source ∩ F ⁻¹' B :=
          ⟨Φ.map_target hzt, by rw [mem_preimage, hFz]; exact hzB⟩
        exact ⟨(hQ _ hU).mpr (by rw [hFz]; exact hzQ), hU⟩
    have hQc : Convex ℝ Q := by
      have : Q = ⋂ i : Z, (fun z : (Z → ℝ) × K => z.1 i) ⁻¹'
          {r | sgnK r = Y[i.1.1]'(hlen ▸ i.1.2)} := by
        ext z; simp [Q]
      rw [this]
      refine convex_iInter fun i => ?_
      exact (convex_sgnK _).linear_preimage
        ((LinearMap.proj i).comp (LinearMap.fst ℝ (Z → ℝ) K))
    rw [himg]
    refine ⟨((convex_ball _ _).inter hQc).isPreconnected.image _ ?_, ?_⟩
    · exact Φ.continuousOn_symm.mono fun z hz => (hball hz.1).1
    · -- a point of the cell: a small multiple of the signs
      refine ⟨Φ.symm ((fun i : Z => ε / 2 * sgnVal (Y[i.1.1]'(hlen ▸ i.1.2))), 0),
        _, ⟨?_, ?_⟩, rfl⟩
      · rw [Metric.mem_ball, hFx, dist_zero_right]
        refine lt_of_le_of_lt ?_ (half_lt_self hε)
        refine (norm_prod_le_iff.mpr ⟨?_, by simp [(half_pos hε).le]⟩)
        refine (pi_norm_le_iff_of_nonneg (by positivity)).mpr fun i => ?_
        rw [Real.norm_eq_abs, abs_mul, abs_of_pos (half_pos hε)]
        exact mul_le_of_le_one_right (half_pos hε).le (abs_sgnVal_le _)
      · intro i
        exact sgnK_mul_sgnVal _ (half_pos hε)

/-- **The flat local picture at a transversal point.** -/
theorem localGood'_of_transversal {cs : List (Carrier E ℝ)} (hd : ∀ c ∈ cs, ContDiff ℝ 1 c)
    {x : E} (ht : TransversalAt cs x) : LocalGood' cs x where
  conn W hW hxW := by
    obtain ⟨U, hUo, hUW, hxU, h⟩ := transversal_chart hd ht W hW hxW
    exact ⟨U, hUo, hUW, hxU, fun Y hY => (h Y hY).1⟩
  near Y hY := by
    rw [mem_closure_iff]
    intro o ho hxo
    obtain ⟨U, -, hUo, hxU, h⟩ := transversal_chart hd ht o ho hxo
    obtain ⟨y, hyY, hyU⟩ := (h Y hY).2
    exact ⟨y, hUo hyU, hyY⟩

/-- **For transversal carriers, `FaithfulAll` is R1**: R2–R5 hold at every point. -/
theorem faithfulAll_of_transversal {cs : List (Carrier E ℝ)} (hd : ∀ c ∈ cs, ContDiff ℝ 1 c)
    (ht : Transversal cs) (h1 : CellsConnected cs) : FaithfulAll cs :=
  faithfulAll_of_local' (fun c hc => (hd c hc).continuous) h1
    fun x => localGood'_of_transversal hd (ht x)

end ForgeCurved
