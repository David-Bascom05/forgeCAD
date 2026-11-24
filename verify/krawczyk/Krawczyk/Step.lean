import Mathlib

/-!
# The Krawczyk step lemma

The certificate of kernel/forge-geom/src/certify.rs (research/kernel-audit/certified-tracing.md
§3.2) proves each step of a traced intersection curve with the Krawczyk test

  K = y - C F(y) + (I - C 𝐉)(X - y) ⊆ X,   ‖I - C A‖ ≤ q < 1 for every A ∈ 𝐉,

where X is a box, y ∈ X, C a matrix (a float inverse of 𝐉's midpoint) and 𝐉 an enclosure of F's
Jacobians over X: every difference F u - F v with u, v ∈ X is A (u - v) for some A ∈ 𝐉 (the
mean-value form, which holds for 𝐉 the interval hull of the Jacobian over a convex X).

`unique_zero`: then F has exactly one zero in X. The kernel uses it once per parameter s of the
pencil of planes (F = (f_a, f_b, g(·, s))), so every plane of the pencil holds exactly one zero.

`maps_into`: the Krawczyk inclusion gives the hypothesis `unique_zero` needs, that x ↦ x - C F x
maps X into X.

`nearest_between`: the interval side. A result rounded to nearest lies between the float below it
and the float above it, so pushing each computed bound out one float (`next_down`, `next_up`, as
forge_math::Interval does) encloses the exact bound. With verify/ieee754's `round` (nearest, ties to
even, over ℝ) as `f`, this is the soundness of every interval operation the certificate uses.
-/

namespace Krawczyk

variable {n : ℕ}

/-- The vectors: `Fin n → ℝ` with the sup norm. -/
abbrev E (n : ℕ) := Fin n → ℝ

/-- The Krawczyk inclusion makes x ↦ x - C F x map X into X: with A the mean-value matrix of x and
y, x - C F x = y - C F y + (I - C A)(x - y). -/
theorem maps_into
    (X : Set (E n)) (F : E n → E n) (C : E n →L[ℝ] E n) (J : Set (E n →L[ℝ] E n))
    (y : E n) (hy : y ∈ X)
    (hmv : ∀ u ∈ X, ∀ v ∈ X, ∃ A ∈ J, F u - F v = A (u - v))
    (hK : ∀ x ∈ X, ∀ A ∈ J,
      y - C (F y) + (ContinuousLinearMap.id ℝ (E n) - C.comp A) (x - y) ∈ X) :
    ∀ x ∈ X, x - C (F x) ∈ X := by
  intro x hx
  obtain ⟨A, hA, hd⟩ := hmv x hx y hy
  have : x - C (F x) = y - C (F y) + (ContinuousLinearMap.id ℝ (E n) - C.comp A) (x - y) := by
    have hF : F x = F y + A (x - y) := by rw [← hd]; abel
    simp only [sub_apply, ContinuousLinearMap.id_apply,
      ContinuousLinearMap.comp_apply, hF, map_add]
    abel
  rw [this]
  exact hK x hx A hA

/-- ‖I - C A‖ < 1 makes C A invertible, so C is onto, so (the space being finite-dimensional) C
is one to one. -/
theorem injective_of_contracting
    (C A : E n →L[ℝ] E n) (h : ‖ContinuousLinearMap.id ℝ (E n) - C.comp A‖ < 1) :
    Function.Injective C := by
  set t : E n →L[ℝ] E n := ContinuousLinearMap.id ℝ (E n) - C.comp A
  let u := Units.oneSub t h
  have hu : (u : E n →L[ℝ] E n) = C.comp A := by
    show 1 - t = C.comp A
    simp [t, ContinuousLinearMap.one_def]
  have hsurj : Function.Surjective C := by
    intro w
    refine ⟨A ((↑u⁻¹ : E n →L[ℝ] E n) w), ?_⟩
    have : (u : E n →L[ℝ] E n) ((↑u⁻¹ : E n →L[ℝ] E n) w) = w := by
      rw [← mul_apply_eq_comp, Units.mul_inv, one_apply_eq_self]
    rw [hu] at this
    simpa using this
  have hl : Function.Surjective (C : E n →ₗ[ℝ] E n) := hsurj
  exact LinearMap.injective_iff_surjective.mpr hl

/-- **The step lemma.** If x ↦ x - C F x maps the closed nonempty set X into itself, and every
difference of F over X is A (u - v) with ‖I - C A‖ ≤ q < 1, then F has exactly one zero in X. -/
theorem unique_zero
    (X : Set (E n)) (hX : IsClosed X) (hne : X.Nonempty)
    (F : E n → E n) (C : E n →L[ℝ] E n) (J : Set (E n →L[ℝ] E n)) (q : NNReal) (hq : q < 1)
    (hmv : ∀ u ∈ X, ∀ v ∈ X, ∃ A ∈ J, F u - F v = A (u - v))
    (hnorm : ∀ A ∈ J, ‖ContinuousLinearMap.id ℝ (E n) - C.comp A‖ ≤ q)
    (hmap : ∀ x ∈ X, x - C (F x) ∈ X) :
    ∃! x, x ∈ X ∧ F x = 0 := by
  obtain ⟨x0, hx0⟩ := hne
  obtain ⟨A0, hA0, -⟩ := hmv x0 hx0 x0 hx0
  have hinj : Function.Injective C :=
    injective_of_contracting C A0 (lt_of_le_of_lt (hnorm A0 hA0) (by exact_mod_cast hq))
  -- the step map is a q-contraction on X
  have hstep : ∀ u ∈ X, ∀ v ∈ X,
      ‖(u - C (F u)) - (v - C (F v))‖ ≤ q * ‖u - v‖ := by
    intro u hu v hv
    obtain ⟨A, hA, hd⟩ := hmv u hu v hv
    have : (u - C (F u)) - (v - C (F v)) =
        (ContinuousLinearMap.id ℝ (E n) - C.comp A) (u - v) := by
      rw [show (u - C (F u)) - (v - C (F v)) = (u - v) - C (F u - F v) by rw [map_sub]; abel, hd]
      rfl
    rw [this]
    calc ‖(ContinuousLinearMap.id ℝ (E n) - C.comp A) (u - v)‖
        ≤ ‖ContinuousLinearMap.id ℝ (E n) - C.comp A‖ * ‖u - v‖ := by
          exact ContinuousLinearMap.le_opNorm _ _
      _ ≤ q * ‖u - v‖ := by
          exact mul_le_mul_of_nonneg_right (hnorm A hA) (norm_nonneg _)
  -- existence: Banach's fixed point theorem on X
  have : Nonempty X := ⟨⟨x0, hx0⟩⟩
  have : CompleteSpace X := hX.completeSpace_coe
  let φ : X → X := fun x => ⟨x - C (F x), hmap x x.2⟩
  have hφ : ContractingWith q φ := by
    refine ⟨hq, LipschitzWith.of_dist_le_mul fun u v => ?_⟩
    simp only [Subtype.dist_eq, dist_eq_norm, φ]
    exact hstep u u.2 v v.2
  let x := ContractingWith.fixedPoint φ hφ
  have hfix : φ x = x := ContractingWith.fixedPoint_isFixedPt hφ
  have hzero : F x = 0 := by
    have h1 : (x : E n) - C (F x) = x := congrArg Subtype.val hfix
    have h2 : C (F x) = C 0 := by
      rw [map_zero]
      have := congrArg (fun w => (x : E n) - w) h1
      simpa using this
    exact hinj h2
  refine ⟨x, ⟨x.2, hzero⟩, ?_⟩
  -- uniqueness: two zeros u, v give ‖u - v‖ ≤ q ‖u - v‖
  rintro w ⟨hw, hFw⟩
  have h := hstep w hw x x.2
  rw [hFw, hzero, map_zero, sub_zero, sub_zero] at h
  have hq' : (q : ℝ) < 1 := by exact_mod_cast hq
  have : ‖w - (x : E n)‖ = 0 := by
    by_contra hne0
    have hpos : 0 < ‖w - (x : E n)‖ := lt_of_le_of_ne (norm_nonneg _) (Ne.symm hne0)
    nlinarith
  exact sub_eq_zero.mp (norm_eq_zero.mp this)

/-- **The interval side.** If `f` is a nearest element of a set S of reals (the floats) to the real
`r`, and `p < f < s` are elements of S, then `p ≤ r ≤ s`: widening a rounded-to-nearest bound out
to the next float on either side encloses the exact value. -/
theorem nearest_between (S : Set ℝ) (r f p s : ℝ)
    (hnear : ∀ g ∈ S, |r - f| ≤ |r - g|) (hp : p ∈ S) (hs : s ∈ S) (hpf : p < f) (hfs : f < s) :
    p ≤ r ∧ r ≤ s := by
  constructor
  · by_contra h
    push Not at h
    have := hnear p hp
    rw [abs_of_neg (by linarith : r - f < 0), abs_of_neg (by linarith : r - p < 0)] at this
    linarith
  · by_contra h
    push Not at h
    have := hnear s hs
    rw [abs_of_pos (by linarith : r - f > 0), abs_of_pos (by linarith : r - s > 0)] at this
    linarith

/-- An interval sum, its bounds rounded to nearest and widened a float each way, holds the exact
sum of any two points of the operands' intervals (forge_math::Interval's `+`; `-`, `*` and `/` by
an interval clear of 0 go the same way, through the exact interval operation's monotonicity). -/
theorem add_encloses (S : Set ℝ) (a b c d x z lo hi plo shi : ℝ)
    (hx : a ≤ x ∧ x ≤ b) (hz : c ≤ z ∧ z ≤ d)
    (hlo : ∀ g ∈ S, |(a + c) - lo| ≤ |(a + c) - g|) (hhi : ∀ g ∈ S, |(b + d) - hi| ≤ |(b + d) - g|)
    (hp : plo ∈ S) (hs : shi ∈ S) (hplo : plo < lo) (hshi : hi < shi)
    (hlo' : ∃ s ∈ S, lo < s) (hhi' : ∃ p ∈ S, p < hi) :
    plo ≤ x + z ∧ x + z ≤ shi := by
  obtain ⟨s1, hs1, hls1⟩ := hlo'
  obtain ⟨p1, hp1, hph1⟩ := hhi'
  have h1 := (nearest_between S (a + c) lo plo s1 hlo hp hs1 hplo hls1).1
  have h2 := (nearest_between S (b + d) hi p1 shi hhi hp1 hs hph1 hshi).2
  constructor <;> linarith [hx.1, hx.2, hz.1, hz.2]

end Krawczyk
