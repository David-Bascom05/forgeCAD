/-
The through-spline's slope system is strictly diagonally dominant row by row, so it has exactly one
solution, and any vector's distance from it is bounded by its residual (`slopes_bound`, the bound
the oracle uses). Over any linear ordered field.
-/
import SplineSpec.Defs
import Mathlib.Algebra.Order.Field.Basic
import Mathlib.LinearAlgebra.FiniteDimensional.Basic
import Mathlib.Tactic

open Finset

namespace SplineSpec

set_option linter.unusedSectionVars false
set_option linter.unusedTactic false
set_option linter.unnecessarySeqFocus false

variable {K : Type} [Field K] [LinearOrder K] [IsStrictOrderedRing K]

/-- Row by row dominance: from `lo z_l + di z_i + up z_r = r_i` with `|lo| + |up| < |di|`, every
`|z_j|` is at most `B` once every `|r_i| ≤ B (|di| - |lo| - |up|)`. (Varah's bound, row-scaled.) -/
theorem dominant_bound (n : ℕ) (z lo di up r : ℕ → K) (L R : ℕ → ℕ) (B : K)
    (hL : ∀ i < n, L i < n) (hR : ∀ i < n, R i < n)
    (hrow : ∀ i < n, lo i * z (L i) + di i * z i + up i * z (R i) = r i)
    (hdom : ∀ i < n, |lo i| + |up i| < |di i|)
    (hr : ∀ i < n, |r i| ≤ B * (|di i| - |lo i| - |up i|)) :
    ∀ j < n, |z j| ≤ B := by
  intro j hj
  have hne : (range n).Nonempty := ⟨j, mem_range.2 hj⟩
  obtain ⟨k, hk, hmax⟩ := exists_max_image (range n) (fun i => |z i|) hne
  rw [mem_range] at hk
  have hmax' : ∀ i < n, |z i| ≤ |z k| := fun i hi => hmax i (mem_range.2 hi)
  have e := hrow k hk
  have h1 : |di k| * |z k| ≤ |r k| + (|lo k| + |up k|) * |z k| := by
    have : di k * z k = r k - lo k * z (L k) - up k * z (R k) := by linarith
    calc |di k| * |z k| = |di k * z k| := (abs_mul _ _).symm
      _ = |r k - lo k * z (L k) - up k * z (R k)| := by rw [this]
      _ ≤ |r k| + |lo k * z (L k)| + |up k * z (R k)| := by
          have := abs_sub (r k - lo k * z (L k)) (up k * z (R k))
          have := abs_sub (r k) (lo k * z (L k))
          linarith
      _ = |r k| + |lo k| * |z (L k)| + |up k| * |z (R k)| := by rw [abs_mul, abs_mul]
      _ ≤ |r k| + |lo k| * |z k| + |up k| * |z k| := by
          have a := mul_le_mul_of_nonneg_left (hmax' _ (hL k hk)) (abs_nonneg (lo k))
          have b := mul_le_mul_of_nonneg_left (hmax' _ (hR k hk)) (abs_nonneg (up k))
          linarith
      _ = |r k| + (|lo k| + |up k|) * |z k| := by ring
  have hd : 0 < |di k| - |lo k| - |up k| := by linarith [hdom k hk]
  have h2 : (|di k| - |lo k| - |up k|) * |z k| ≤ B * (|di k| - |lo k| - |up k|) := by
    linarith [hr k hk]
  have h3 : |z k| ≤ B := by
    have h2' : (|di k| - |lo k| - |up k|) * |z k| ≤ (|di k| - |lo k| - |up k|) * B := by
      linarith [mul_comm B (|di k| - |lo k| - |up k|)]
    exact le_of_mul_le_mul_left h2' hd
  exact le_trans (hmax' j hj) h3

/-- What the slope system is: a valid parameter sequence `0 < h i` and the size of the system. -/
structure SysOk (τ : ℕ → K) (m : ℕ) (e : Ends K) : Prop where
  pos : ∀ i < m, τ i < τ (i + 1)
  size : match e with
    | .open_ _ _ => 1 ≤ m
    | .periodic => 2 ≤ m

theorem unknowns_pos {τ : ℕ → K} {m : ℕ} {e : Ends K} (h : SysOk τ m e) : 0 < unknowns m e := by
  have := h.size
  cases e <;> simp [unknowns] at * <;> omega

theorem leftIx_lt {n i : ℕ} (hn : 0 < n) (hi : i < n) : leftIx n i < n := by
  unfold leftIx; split_ifs <;> omega

theorem rightIx_lt {n i : ℕ} (hn : 0 < n) (hi : i < n) : rightIx n i < n := by
  unfold rightIx; split_ifs <;> omega

/-- The rows' coefficients do not depend on q; each row is dominant. -/
theorem rows_dominant {τ q : ℕ → K} {m : ℕ} {e : Ends K} (h : SysOk τ m e) :
    ∀ i < unknowns m e, |(rows τ q m e i).lo| + |(rows τ q m e i).up| < |(rows τ q m e i).di| := by
  intro i hi
  have hp := h.pos
  have hsz := h.size
  have hpos : ∀ j < m, 0 < τ (j + 1) - τ j := fun j hj => sub_pos.2 (hp j hj)
  cases e with
  | open_ s f =>
    simp only [unknowns] at hi
    simp only [rows]
    by_cases h0 : i = 0
    · subst h0
      cases s <;> simp <;> norm_num
    · by_cases hm : i = m
      · subst hm
        cases f <;> simp [h0] <;> norm_num
      · simp only [h0, hm, ite_false]
        have a := hpos i (by omega)
        have b := hpos (i - 1) (by omega)
        rw [abs_of_pos a, abs_of_pos b, abs_of_pos (by positivity)]
        linarith
  | periodic =>
    simp only [unknowns] at hi
    simp only [rows]
    by_cases h0 : i = 0
    · subst h0
      simp only [ite_true]
      have a := hpos 0 (by omega)
      have b := hpos (m - 1) (by omega)
      rw [abs_of_pos a, abs_of_pos b, abs_of_pos (by positivity)]
      linarith
    · simp only [h0, ite_false]
      have a := hpos i (by omega)
      have b := hpos (i - 1) (by omega)
      rw [abs_of_pos a, abs_of_pos b, abs_of_pos (by positivity)]
      linarith

/-- The residual is the row applied to `x` minus its right side. -/
theorem residual_eq (τ q : ℕ → K) (m : ℕ) (e : Ends K) (x : ℕ → K) (i : ℕ) :
    residual τ q m e x i =
      (rows τ q m e i).lo * x (leftIx (unknowns m e) i) + (rows τ q m e i).di * x i
        + (rows τ q m e i).up * x (rightIx (unknowns m e) i) - (rows τ q m e i).rhs := rfl

/-- **The bound.** Any `x`'s slopes are within `B` of a solution `y`'s wherever every row's
residual at `x` is at most `B` times the row's dominance margin. -/
theorem slopes_bound {τ q : ℕ → K} {m : ℕ} {e : Ends K} (h : SysOk τ m e) (x y : ℕ → K) (B : K)
    (hy : ∀ i < unknowns m e, residual τ q m e y i = 0)
    (hx : ∀ i < unknowns m e, |residual τ q m e x i| ≤
      B * (|(rows τ q m e i).di| - |(rows τ q m e i).lo| - |(rows τ q m e i).up|)) :
    ∀ j < unknowns m e, |x j - y j| ≤ B := by
  have hn := unknowns_pos h
  refine dominant_bound (unknowns m e) (fun j => x j - y j) (fun i => (rows τ q m e i).lo)
    (fun i => (rows τ q m e i).di) (fun i => (rows τ q m e i).up)
    (fun i => residual τ q m e x i) (leftIx (unknowns m e)) (rightIx (unknowns m e)) B
    (fun i hi => leftIx_lt hn hi) (fun i hi => rightIx_lt hn hi) ?_ (rows_dominant h) hx
  intro i hi
  have e1 := residual_eq τ q m e x i
  have e2 := residual_eq τ q m e y i
  have := hy i hi
  linear_combination -e1 + e2 - this

/-- Uniqueness: two solutions agree on every unknown. -/
theorem slopes_unique {τ q : ℕ → K} {m : ℕ} {e : Ends K} (h : SysOk τ m e) (x y : ℕ → K)
    (hx : ∀ i < unknowns m e, residual τ q m e x i = 0)
    (hy : ∀ i < unknowns m e, residual τ q m e y i = 0) :
    ∀ j < unknowns m e, x j = y j := by
  intro j hj
  have := slopes_bound h x y 0 hy (fun i hi => by rw [hx i hi]; simp) j hj
  have : |x j - y j| = 0 := le_antisymm (by simpa using this) (abs_nonneg _)
  exact sub_eq_zero.1 (abs_eq_zero.1 this)

/-- The rows applied to a vector of `Fin n`, as a linear map. -/
def sysMap (τ q : ℕ → K) (m : ℕ) (e : Ends K) :
    (Fin (unknowns m e) → K) →ₗ[K] (Fin (unknowns m e) → K) where
  toFun v := fun i =>
    let ext : ℕ → K := fun j => if hj : j < unknowns m e then v ⟨j, hj⟩ else 0
    residual τ q m e ext i + (rows τ q m e i).rhs
  map_add' v w := by
    funext i
    simp only [residual_eq, Pi.add_apply]
    split_ifs <;> ring
  map_smul' c v := by
    funext i
    simp only [residual_eq, Pi.smul_apply, smul_eq_mul, RingHom.id_apply]
    split_ifs <;> ring

/-- Existence: the system has a solution (an injective linear map of a finite space onto itself). -/
theorem slopes_exist {τ q : ℕ → K} {m : ℕ} {e : Ends K} (h : SysOk τ m e) :
    ∃ x : ℕ → K, ∀ i < unknowns m e, residual τ q m e x i = 0 := by
  have hn := unknowns_pos h
  set n := unknowns m e with hndef
  let ext : (Fin n → K) → ℕ → K := fun v j => if hj : j < n then v ⟨j, hj⟩ else 0
  have hinj : Function.Injective (sysMap τ q m e) := by
    intro v w hvw
    have hres : ∀ i < n, residual τ q m e (ext v) i = residual τ q m e (ext w) i := by
      intro i hi
      have := congrFun hvw ⟨i, hi⟩
      simp only [sysMap, LinearMap.coe_mk, AddHom.coe_mk] at this
      linarith
    -- the difference solves the system with zero residual differences
    have hb : ∀ j < n, |ext v j - ext w j| ≤ 0 := by
      refine dominant_bound n (fun j => ext v j - ext w j) (fun i => (rows τ q m e i).lo)
        (fun i => (rows τ q m e i).di) (fun i => (rows τ q m e i).up) (fun _ => 0)
        (leftIx n) (rightIx n) 0 (fun i hi => leftIx_lt hn hi) (fun i hi => rightIx_lt hn hi)
        ?_ (rows_dominant h) (fun i _ => by simp)
      intro i hi
      have e1 := residual_eq τ q m e (ext v) i
      have e2 := residual_eq τ q m e (ext w) i
      have := hres i hi
      linear_combination -e1 + e2 + this
    funext ⟨j, hj⟩
    have := hb j hj
    have h0 : ext v j - ext w j = 0 := abs_nonpos_iff.1 this
    have h1 := sub_eq_zero.1 h0
    simp only [ext] at h1
    rwa [dif_pos hj, dif_pos hj] at h1
  have hsurj := LinearMap.injective_iff_surjective.1 hinj
  obtain ⟨v, hv⟩ := hsurj (fun i => (rows τ q m e i).rhs)
  refine ⟨ext v, fun i hi => ?_⟩
  have := congrFun hv ⟨i, hi⟩
  simp only [sysMap, LinearMap.coe_mk, AddHom.coe_mk] at this
  linarith

end SplineSpec
