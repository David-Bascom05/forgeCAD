import ForgeCurved.Diagonal
import Mathlib.Analysis.Calculus.FDeriv.Pow
import Mathlib.Analysis.Calculus.ContDiff.Operations

/-!
# A worked instance: a plate with a through hole

The carriers of a plate `0 ≤ z ≤ t` with a hole of radius `R` on the `z` axis: the two caps, the
cylinder, and the axial seams.

* **`plate_faithful`**: with **both** axial seams `x = 0` and `y = 0`, the arrangement is
  `FaithfulAll`, with no hypothesis left. It is transversal everywhere (`plate_transversal`, from
  the gradients), and its cells are connected because it is coaxial (`cellsConnected_of_diagonal`).
* **`plate1_not_faithful`**: with only the seam `y = 0` (design §2.3's single axial seam), it is
  not. The edge cell `0 < z < t`, on the cylinder and the seam, is the two segments `x = ±R`, with
  one covector.

So Tier 0 is not vacuous on curved inputs, and it needs the second axial seam.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved

open ForgeExact ForgeExact.DecideOnce ForgeExact.DecideOnce.Curved Topology Set

/-! ## Transversality from a sum over all carriers -/

/-- Transversality at `x`, checked on all carriers with weights vanishing off `x`'s. -/
theorem transversalAt_of_fin {E : Type} [NormedAddCommGroup E] [NormedSpace ℝ E]
    [FiniteDimensional ℝ E] {cs : List (Carrier E ℝ)} {x : E}
    (h : ∀ g : Fin cs.length → ℝ, (∀ i, cs[i] x ≠ 0 → g i = 0) →
      (∀ v, ∑ i, g i * fderiv ℝ cs[i] x v = 0) → ∀ i, g i = 0) : TransversalAt cs x := by
  classical
  apply transversalAt_of_linearIndependent
  rw [Fintype.linearIndependent_iff]
  intro g hg j
  set g' : Fin cs.length → ℝ := fun i => if hi : cs[i] x = 0 then g ⟨i, hi⟩ else 0
  have hsupp : ∀ i, cs[i] x ≠ 0 → g' i = 0 := fun i hi => by simp only [g', dif_neg hi]
  have hsum : ∀ v, ∑ i, g' i * fderiv ℝ cs[i] x v = 0 := by
    intro v
    have hgv := congrArg (fun f : E →L[ℝ] ℝ => f v) hg
    simp only [ContinuousLinearMap.coe_sum', Finset.sum_apply, ContinuousLinearMap.smul_apply,
      smul_eq_mul, ContinuousLinearMap.zero_apply] at hgv
    rw [← Fintype.sum_subtype_add_sum_subtype (fun i : Fin cs.length => cs[i] x = 0)]
    have h2 : ∑ i : {i : Fin cs.length // ¬cs[i] x = 0}, g' i * fderiv ℝ cs[i.1] x v = 0 :=
      Finset.sum_eq_zero fun i _ => by simp only [g', dif_neg i.2, zero_mul]
    rw [h2, add_zero]
    convert hgv using 1
    apply Finset.sum_congr rfl
    intro i _
    simp only [g', dif_pos i.2]
  have := h g' hsupp hsum j.1
  rwa [show g' j.1 = g j from dif_pos j.2] at this

/-! ## The carriers -/

def πx : R3 →L[ℝ] ℝ := ContinuousLinearMap.fst ℝ ℝ (ℝ × ℝ)
def πy : R3 →L[ℝ] ℝ := (ContinuousLinearMap.fst ℝ ℝ ℝ).comp (ContinuousLinearMap.snd ℝ ℝ (ℝ × ℝ))
def πz : R3 →L[ℝ] ℝ := (ContinuousLinearMap.snd ℝ ℝ ℝ).comp (ContinuousLinearMap.snd ℝ ℝ (ℝ × ℝ))

/-- Caps `z = 0` and `z = t`, the cylinder `x² + y² = R²`, the seams `x = 0` and `y = 0`. -/
def plate (t R : ℝ) : List (Carrier R3 ℝ) :=
  [fun p => p.2.2, fun p => p.2.2 - t, fun p => p.1 ^ 2 + p.2.1 ^ 2 - R ^ 2, fun p => p.1,
    fun p => p.2.1]

theorem fderiv_cyl (R : ℝ) (x v : R3) :
    fderiv ℝ (fun p : R3 => p.1 ^ 2 + p.2.1 ^ 2 - R ^ 2) x v = 2 * x.1 * v.1 + 2 * x.2.1 * v.2.1 := by
  have h : HasFDerivAt (fun p : R3 => πx p ^ 2 + πy p ^ 2 - R ^ 2) _ x :=
    ((HasFDerivAt.pow πx.hasFDerivAt 2).add (HasFDerivAt.pow πy.hasFDerivAt 2)).sub_const _
  rw [show (fun p : R3 => p.1 ^ 2 + p.2.1 ^ 2 - R ^ 2) =
    (fun p : R3 => πx p ^ 2 + πy p ^ 2 - R ^ 2) from rfl, h.fderiv]
  simp [πx, πy]

theorem fderiv_z (t : ℝ) (x v : R3) : fderiv ℝ (fun p : R3 => p.2.2 - t) x v = v.2.2 := by
  rw [show (fun p : R3 => p.2.2 - t) = (fun p : R3 => πz p - t) from rfl,
    (πz.hasFDerivAt.sub_const t).fderiv]
  rfl

theorem fderiv_z0 (x v : R3) : fderiv ℝ (fun p : R3 => p.2.2) x v = v.2.2 := by
  rw [show (fun p : R3 => p.2.2) = (fun p : R3 => πz p) from rfl, πz.hasFDerivAt.fderiv]
  rfl

theorem fderiv_x (x v : R3) : fderiv ℝ (fun p : R3 => p.1) x v = v.1 := by
  rw [show (fun p : R3 => p.1) = (fun p : R3 => πx p) from rfl, πx.hasFDerivAt.fderiv]
  rfl

theorem fderiv_y (x v : R3) : fderiv ℝ (fun p : R3 => p.2.1) x v = v.2.1 := by
  rw [show (fun p : R3 => p.2.1) = (fun p : R3 => πy p) from rfl, πy.hasFDerivAt.fderiv]
  rfl

theorem plate_contDiff (t R : ℝ) : ∀ c ∈ plate t R, ContDiff ℝ 1 c := by
  intro c hc
  simp only [plate, List.mem_cons, List.not_mem_nil, or_false] at hc
  rcases hc with rfl | rfl | rfl | rfl | rfl <;> fun_prop

/-- **The seamed plate is transversal everywhere.** -/
theorem plate_transversal {t R : ℝ} (ht : t ≠ 0) (hR : R ≠ 0) : Transversal (plate t R) := by
  intro x
  apply transversalAt_of_fin
  intro g' hg hs i'
  let g : Fin 5 → ℝ := fun k => g' k
  have hs5 : ∀ v : R3, ∑ k : Fin 5, g k * fderiv ℝ ((plate t R)[k.1]'(by simp [plate])) x v = 0 := hs
  have hsum : ∀ v : R3, g 0 * v.2.2 + g 1 * v.2.2 + g 2 * (2 * x.1 * v.1 + 2 * x.2.1 * v.2.1) +
      g 3 * v.1 + g 4 * v.2.1 = 0 := by
    intro v
    have := hs5 v
    simp only [Fin.sum_univ_five] at this
    simpa [plate, fderiv_cyl, fderiv_z, fderiv_z0, fderiv_x, fderiv_y] using this
  have hg' : ∀ k : Fin 5, ((plate t R)[k.1]'(by simp [plate])) x ≠ 0 → g k = 0 := fun k => hg k
  show g i' = 0
  have e1 := hsum (1, 0, 0)
  have e2 := hsum (0, 1, 0)
  have e3 := hsum (0, 0, 1)
  simp only [mul_zero, mul_one, add_zero, zero_add] at e1 e2 e3
  have h0 : x.2.2 ≠ 0 → g 0 = 0 := fun h => hg' 0 h
  have h1 : x.2.2 - t ≠ 0 → g 1 = 0 := fun h => hg' 1 h
  have h2 : x.1 ^ 2 + x.2.1 ^ 2 - R ^ 2 ≠ 0 → g 2 = 0 := fun h => hg' 2 h
  have h3 : x.1 ≠ 0 → g 3 = 0 := fun h => hg' 3 h
  have h4 : x.2.1 ≠ 0 → g 4 = 0 := fun h => hg' 4 h
  -- the caps
  have g01 : g 0 = 0 ∧ g 1 = 0 := by
    by_cases hz : x.2.2 = 0
    · have := h1 (by rw [hz]; simpa using ht); exact ⟨by linarith, this⟩
    · have := h0 hz; exact ⟨this, by linarith⟩
  -- the cylinder and the seams
  have g234 : g 2 = 0 ∧ g 3 = 0 ∧ g 4 = 0 := by
    by_cases hc : x.1 ^ 2 + x.2.1 ^ 2 - R ^ 2 = 0
    · by_cases hx : x.1 = 0
      · have hy : x.2.1 ≠ 0 := by
          intro hy; rw [hx, hy] at hc; apply hR; nlinarith [sq_nonneg R]
        have g4 := h4 hy
        rw [g4, add_zero] at e2
        have g2 : g 2 = 0 := by
          rcases mul_eq_zero.mp e2 with h | h
          · exact h
          · exact absurd (by linarith : x.2.1 = 0) hy
        refine ⟨g2, ?_, g4⟩
        rw [g2] at e1; linarith
      · have g3 := h3 hx
        rw [g3, add_zero] at e1
        have g2 : g 2 = 0 := by
          rcases mul_eq_zero.mp e1 with h | h
          · exact h
          · exact absurd (by linarith : x.1 = 0) hx
        refine ⟨g2, g3, ?_⟩
        rw [g2] at e2; linarith
    · have g2 := h2 hc
      rw [g2] at e1 e2
      exact ⟨g2, by linarith, by linarith⟩
  have hall : ∀ k : Fin 5, g k = 0 := by
    intro k
    fin_cases k
    · exact g01.1
    · exact g01.2
    · exact g234.1
    · exact g234.2.1
    · exact g234.2.2
  exact hall i'

/-! ## Its cells -/

theorem sgnK_sgnVal (s : Sgn) : sgnK (sgnVal s) = s := by
  simpa using sgnK_mul_sgnVal s one_pos

/-- A constant affine function. -/
def constAff (d : ℝ) : Aff := ⟨0, 0, 0, d⟩

theorem constAff_eval (d : ℝ) (p : R3) : (constAff d).eval p = d := by
  simp [constAff, Aff.eval]

theorem sgns_apply (y : R3) (k : Fin 3) : sgns y k = sgnK (crd y k) := rfl

theorem plate_diagonal {t R : ℝ} (ht : 0 < t) :
    ∀ c ∈ plate t R, DiagonalIn (Homeomorph.refl R3) c := by
  intro c hc
  simp only [plate, List.mem_cons, List.not_mem_nil, or_false] at hc
  rcases hc with rfl | rfl | rfl | rfl | rfl
  · intro s; refine ⟨constAff (sgnVal (s 2)), fun y hy => ?_⟩
    rw [constAff_eval, sgnK_sgnVal, ← hy]; rfl
  · intro s
    rcases hs : s 2 with _ | _ | _
    · refine ⟨constAff (-1), fun y hy => ?_⟩
      have : sgnK y.2.2 = .neg := by rw [← hs, ← hy]; rfl
      have hy0 : y.2.2 < 0 := by
        rcases lt_trichotomy y.2.2 0 with h | h | h
        · exact h
        · rw [h] at this; simp [sgnK] at this
        · rw [sgnK_pos h] at this; cases this
      rw [constAff_eval, sgnK_neg (by norm_num : (-1 : ℝ) < 0)]
      exact sgnK_neg (by simp only [Homeomorph.refl_apply, id]; linarith)
    · refine ⟨constAff (-1), fun y hy => ?_⟩
      have : sgnK y.2.2 = .zero := by rw [← hs, ← hy]; rfl
      have hy0 := sgnK_zero_iff.mp this
      rw [constAff_eval, sgnK_neg (by norm_num : (-1 : ℝ) < 0)]
      exact sgnK_neg (by simp only [Homeomorph.refl_apply, id]; linarith)
    · refine ⟨⟨0, 0, 1, -t ^ 2⟩, fun y hy => ?_⟩
      have : sgnK y.2.2 = .pos := by rw [← hs, ← hy]; rfl
      have hy0 : 0 < y.2.2 := by
        rcases lt_trichotomy y.2.2 0 with h | h | h
        · rw [sgnK_neg h] at this; cases this
        · rw [h] at this; simp [sgnK] at this
        · exact h
      have e : Aff.eval ⟨0, 0, 1, -t ^ 2⟩ (sqs (Homeomorph.refl R3 y)) =
          (y.2.2 + t) * (y.2.2 - t) := by
        simp only [Aff.eval, sqs, Homeomorph.refl_apply, id]; ring
      rw [e, sgnK_mul_pos (by linarith)]
  · intro s; refine ⟨⟨1, 1, 0, -R ^ 2⟩, fun y _ => ?_⟩
    congr 1
    simp only [Aff.eval, sqs, Homeomorph.refl_apply, id]; ring
  · intro s; refine ⟨constAff (sgnVal (s 0)), fun y hy => ?_⟩
    rw [constAff_eval, sgnK_sgnVal, ← hy]; rfl
  · intro s; refine ⟨constAff (sgnVal (s 1)), fun y hy => ?_⟩
    rw [constAff_eval, sgnK_sgnVal, ← hy]; rfl

theorem plate_cells {t R : ℝ} (ht : 0 < t) : CellsConnected (plate t R) := by
  refine cellsConnected_of_diagonal (Homeomorph.refl R3) (fun k => ?_) (plate_diagonal ht)
  fin_cases k
  · exact ⟨⟨3, by simp [plate]⟩, fun y => rfl⟩
  · exact ⟨⟨4, by simp [plate]⟩, fun y => rfl⟩
  · exact ⟨⟨0, by simp [plate]⟩, fun y => rfl⟩

/-- **The plate with a hole and both axial seams is faithful**, every cell of it. -/
theorem plate_faithful {t R : ℝ} (ht : 0 < t) (hR : R ≠ 0) : FaithfulAll (plate t R) :=
  faithfulAll_of_transversal (plate_contDiff t R) (plate_transversal ht.ne' hR) (plate_cells ht)

/-! ## With one axial seam it is not -/

/-- The same plate with the single axial seam `y = 0`. -/
def plate1 (t R : ℝ) : List (Carrier R3 ℝ) :=
  [fun p => p.2.2, fun p => p.2.2 - t, fun p => p.1 ^ 2 + p.2.1 ^ 2 - R ^ 2, fun p => p.2.1]

/-- **One axial seam is not enough**: the seam edge `x = ±R`, `y = 0`, `0 < z < t` is one
covector and two segments. -/
theorem plate1_not_faithful {t R : ℝ} (ht : 0 < t) (hR : 0 < R) : ¬FaithfulAll (plate1 t R) := by
  intro hf
  set a : R3 := (R, 0, t / 2)
  set b : R3 := (-R, 0, t / 2)
  have hab : covOfK (plate1 t R) b = covOfK (plate1 t R) a := by
    simp [covOfK, plate1, a, b]
  have hc := (connected_cell_iff _ _).mp (hf.connected (covOfK (plate1 t R) a) trivial ⟨a, rfl⟩)
  obtain ⟨z, hz, hz1, hz2⟩ := hc {p | 0 < p.1} {p | p.1 < 0}
    (isOpen_lt continuous_const continuous_fst) (isOpen_lt continuous_fst continuous_const)
    (fun p hp => by
      have hp' : covOfK (plate1 t R) p = covOfK (plate1 t R) a := hp
      have h3 : sgnK (p.1 ^ 2 + p.2.1 ^ 2 - R ^ 2) = sgnK (R ^ 2 + 0 ^ 2 - R ^ 2) := by
        have := congrArg (fun l => l[2]?) hp'
        simpa [covOfK, plate1, a] using this
      have h4 : sgnK p.2.1 = sgnK (0 : ℝ) := by
        have := congrArg (fun l => l[3]?) hp'
        simpa [covOfK, plate1, a] using this
      have hy : p.2.1 = 0 := sgnK_zero_iff.mp (by simpa [sgnK] using h4)
      have hc0 : p.1 ^ 2 + p.2.1 ^ 2 - R ^ 2 = 0 := sgnK_zero_iff.mp (by simpa [sgnK] using h3)
      rw [hy] at hc0
      rcases lt_trichotomy p.1 0 with h | h | h
      · exact Or.inr h
      · rw [h] at hc0; nlinarith
      · exact Or.inl h)
    ⟨a, rfl, show (0 : ℝ) < R from hR⟩ ⟨b, hab, show -R < (0 : ℝ) by linarith⟩
  have h1 : (0 : ℝ) < z.1 := hz1
  have h2 : z.1 < 0 := hz2
  linarith

end ForgeCurved
