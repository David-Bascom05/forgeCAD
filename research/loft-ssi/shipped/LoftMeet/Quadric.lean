import LoftMeet.Counter

/-!
# `Hull::misses`: the Bernstein sign test of a quadric over a rational piece

Q(p) = (p - o).A(p - o) + 2 b.(p - o) + c (`Alg::Quadric`). The piece has 4 x n control points P i j
(n = 3 for arcs, 2 for lines) with weights w j along t; the code forms h i j = (P i j - o) w j and
  coef[i+k][j+l] += v · C3_i C3_k Cm_j Cm_l / (C6_(i+k) C2m_(j+l)),
  v = h k l . (A h i j) + 2 b . h i j w l + c w j w l,           m = n - 1.
Theorem: Q(S) W² is the bivariate Bernstein polynomial of degree (6, 2m) with exactly these
coefficients, so one strict sign of all of them gives Q(S) that sign on the whole piece.
-/

namespace LoftSSI

open Finset Matrix

variable (n : ℕ) (P : ℕ → ℕ → Fin 3 → ℝ) (w : ℕ → ℝ) (o b : Fin 3 → ℝ)
  (A : Matrix (Fin 3) (Fin 3) ℝ) (c σ τ : ℝ)

def quadQ (p : Fin 3 → ℝ) : ℝ := (p - o) ⬝ᵥ (A *ᵥ (p - o)) + 2 * (b ⬝ᵥ (p - o)) + c

/-- Control point indices (i, j): i < 4 along the loft, j < n round the arc / along the line. -/
def idx : Finset (ℕ × ℕ) := range 4 ×ˢ range n

/-- The code's `h(i, j)`. -/
def hv (x : ℕ × ℕ) : Fin 3 → ℝ := w x.2 • (P x.1 x.2 - o)

/-- The code's `v` for x = (i, j), y = (k, l). -/
def vq (x y : ℕ × ℕ) : ℝ :=
  hv P w o y ⬝ᵥ (A *ᵥ hv P w o x) + 2 * (b ⬝ᵥ hv P w o x) * w y.2 + c * w x.2 * w y.2

/-- The code's factor `C3[i] C3[k] cn[j] cn[l] / (C6[i+k] c2n[j+l])`. -/
noncomputable def kq (x y : ℕ × ℕ) : ℝ :=
  (Nat.choose 3 x.1 : ℝ) * Nat.choose 3 y.1 * Nat.choose (n - 1) x.2 * Nat.choose (n - 1) y.2 /
    (Nat.choose 6 (x.1 + y.1) * Nat.choose (2 * (n - 1)) (x.2 + y.2))

/-- The code's 7 x (2m+1) coefficient array, as its four nested loops accumulate it. -/
noncomputable def coefQ (p q : ℕ) : ℝ :=
  ∑ x ∈ idx n, ∑ y ∈ idx n,
    if x.1 + y.1 = p ∧ x.2 + y.2 = q then vq P w o b A c x y * kq n x y else 0

/-- The rational piece's denominator. -/
noncomputable def wden : ℝ := ∑ j ∈ range n, bb (n - 1) j τ * w j

noncomputable def al (x : ℕ × ℕ) : ℝ := bb 3 x.1 σ * bb (n - 1) x.2 τ

/-- One term (x, y) of the products lands on coefficient (i + k, j + l) with the code's factor. -/
theorem one_term (hn : 1 ≤ n) (V : ℝ) (x y : ℕ × ℕ) (hx : x ∈ idx n) (hy : y ∈ idx n) :
    ∑ p ∈ range 7, ∑ q ∈ range (2 * (n - 1) + 1), bb 6 p σ * bb (2 * (n - 1)) q τ *
      (if x.1 + y.1 = p ∧ x.2 + y.2 = q then V * kq n x y else 0) =
      al n σ τ x * al n σ τ y * V := by
  simp only [idx, mem_product, mem_range] at hx hy
  simp only [ite_and, mul_ite, mul_zero]
  rw [sum_comm]
  simp only [sum_ite_eq, mem_range, show x.1 + y.1 < 7 by omega, if_true,
    show x.2 + y.2 < 2 * (n - 1) + 1 by omega]
  have m1 := bb_mul 3 3 x.1 y.1 (by omega) (by omega) σ
  have m2 := bb_mul (n - 1) (n - 1) x.2 y.2 (by omega) (by omega) τ
  rw [al, al, show bb 3 x.1 σ * bb (n - 1) x.2 τ * (bb 3 y.1 σ * bb (n - 1) y.2 τ) =
    (bb 3 x.1 σ * bb 3 y.1 σ) * (bb (n - 1) x.2 τ * bb (n - 1) y.2 τ) by ring, m1, m2,
    show n - 1 + (n - 1) = 2 * (n - 1) by omega, kq]
  norm_num
  ring

/-- **Substitution identity.** If W·S = Σ B³_i B^m_j w_j P_ij (the rational patch, as
`patch_exact` gives for arcs), then W² Q(S) = Σ_p Σ_q B⁶_p(σ) B^(2m)_q(τ) coef p q. -/
theorem quad_subst (hn : 1 ≤ n) (S : Fin 3 → ℝ)
    (hS : ∀ c', wden n w τ * S c' = ∑ x ∈ idx n, al n σ τ x * w x.2 * P x.1 x.2 c') :
    wden n w τ ^ 2 * quadQ o b A c S =
      ∑ p ∈ range 7, ∑ q ∈ range (2 * (n - 1) + 1),
        bb 6 p σ * bb (2 * (n - 1)) q τ * coefQ n P w o b A c p q := by
  set W := wden n w τ
  have sw : ∑ x ∈ idx n, al n σ τ x * w x.2 = W := by
    simp only [idx, al, W, wden, sum_product]
    simp_rw [mul_assoc, ← mul_sum, ← sum_mul]
    rw [show (4 : ℕ) = 3 + 1 by rfl, bb_sum, one_mul]
  set Y : Fin 3 → ℝ := ∑ x ∈ idx n, al n σ τ x • hv P w o x
  have hY : W • (S - o) = Y := by
    funext c'
    simp only [Y, Pi.smul_apply, Pi.sub_apply, smul_eq_mul, Finset.sum_apply, hv]
    rw [mul_sub, hS, ← sw, sum_mul, ← sum_sub_distrib]
    refine sum_congr rfl fun x _ => ?_
    ring
  have step1 : W ^ 2 * quadQ o b A c S = Y ⬝ᵥ (A *ᵥ Y) + 2 * (b ⬝ᵥ Y) * W + c * W * W := by
    rw [← hY, quadQ, mulVec_smul, dotProduct_smul, smul_dotProduct, dotProduct_smul]
    simp only [smul_eq_mul]; ring
  have step2 : Y ⬝ᵥ (A *ᵥ Y) + 2 * (b ⬝ᵥ Y) * W + c * W * W =
      ∑ x ∈ idx n, ∑ y ∈ idx n, al n σ τ x * al n σ τ y * vq P w o b A c x y := by
    have e1 : Y ⬝ᵥ (A *ᵥ Y) = ∑ x ∈ idx n, ∑ y ∈ idx n,
        al n σ τ x * al n σ τ y * (hv P w o y ⬝ᵥ (A *ᵥ hv P w o x)) := by
      simp only [Y, mulVec_sum, mulVec_smul, sum_dotProduct, dotProduct_sum, smul_dotProduct,
        dotProduct_smul, smul_eq_mul, mul_sum]
      first
        | (refine sum_congr rfl fun x _ => sum_congr rfl fun y _ => ?_; ring; done)
        | (rw [sum_comm]; refine sum_congr rfl fun x _ => sum_congr rfl fun y _ => ?_; ring)
    have e2 : 2 * (b ⬝ᵥ Y) * W = ∑ x ∈ idx n, ∑ y ∈ idx n,
        al n σ τ x * al n σ τ y * (2 * (b ⬝ᵥ hv P w o x) * w y.2) := by
      rw [← sw]
      simp only [Y, dotProduct_sum, dotProduct_smul, smul_eq_mul, mul_sum, sum_mul]
      first
        | (refine sum_congr rfl fun x _ => sum_congr rfl fun y _ => ?_; ring; done)
        | (rw [sum_comm]; refine sum_congr rfl fun x _ => sum_congr rfl fun y _ => ?_; ring)
    have e3 : c * W * W = ∑ x ∈ idx n, ∑ y ∈ idx n,
        al n σ τ x * al n σ τ y * (c * w x.2 * w y.2) := by
      rw [← sw]
      simp only [mul_sum, sum_mul]
      refine sum_congr rfl fun x _ => sum_congr rfl fun y _ => ?_
      ring
    rw [e1, e2, e3, ← sum_add_distrib, ← sum_add_distrib]
    refine sum_congr rfl fun x _ => ?_
    rw [← sum_add_distrib, ← sum_add_distrib]
    refine sum_congr rfl fun y _ => ?_
    simp only [vq]; ring
  have step3 : ∑ x ∈ idx n, ∑ y ∈ idx n, al n σ τ x * al n σ τ y * vq P w o b A c x y =
      ∑ p ∈ range 7, ∑ q ∈ range (2 * (n - 1) + 1),
        bb 6 p σ * bb (2 * (n - 1)) q τ * coefQ n P w o b A c p q := by
    simp only [coefQ, mul_sum]
    let F : ℕ → ℕ → ℕ × ℕ → ℕ × ℕ → ℝ := fun p q x y => bb 6 p σ * bb (2 * (n - 1)) q τ *
      (if x.1 + y.1 = p ∧ x.2 + y.2 = q then vq P w o b A c x y * kq n x y else 0)
    symm
    calc ∑ p ∈ range 7, ∑ q ∈ range (2 * (n - 1) + 1), ∑ x ∈ idx n, ∑ y ∈ idx n, F p q x y
        = ∑ p ∈ range 7, ∑ x ∈ idx n, ∑ q ∈ range (2 * (n - 1) + 1), ∑ y ∈ idx n, F p q x y :=
          sum_congr rfl fun _ _ => sum_comm
      _ = ∑ p ∈ range 7, ∑ x ∈ idx n, ∑ y ∈ idx n, ∑ q ∈ range (2 * (n - 1) + 1), F p q x y :=
          sum_congr rfl fun _ _ => sum_congr rfl fun _ _ => sum_comm
      _ = ∑ x ∈ idx n, ∑ p ∈ range 7, ∑ y ∈ idx n, ∑ q ∈ range (2 * (n - 1) + 1), F p q x y :=
          sum_comm
      _ = ∑ x ∈ idx n, ∑ y ∈ idx n, ∑ p ∈ range 7, ∑ q ∈ range (2 * (n - 1) + 1), F p q x y :=
          sum_congr rfl fun _ _ => sum_comm
      _ = _ := sum_congr rfl fun x hx => sum_congr rfl fun y hy =>
          one_term n σ τ hn _ x y hx hy

  rw [step1, step2, step3]

/-- **Sign test (`Hull::misses`) is sound, with any eps ≥ 0.** If every coefficient exceeds eps
(≥ 0), Q is positive at every point of the piece (W ≠ 0 is all that's needed of the weights). -/
theorem misses_sound (hn : 1 ≤ n) (S : Fin 3 → ℝ) (eps : ℝ) (heps : 0 ≤ eps)
    (hW : wden n w τ ≠ 0)
    (hS : ∀ c', wden n w τ * S c' = ∑ x ∈ idx n, al n σ τ x * w x.2 * P x.1 x.2 c')
    (hσ : 0 ≤ σ ∧ σ ≤ 1) (hτ : 0 ≤ τ ∧ τ ≤ 1)
    (hpos : ∀ p ≤ 6, ∀ q ≤ 2 * (n - 1), eps < coefQ n P w o b A c p q) :
    0 < quadQ o b A c S := by
  have e := quad_subst n P w o b A c σ τ hn S hS
  have g : eps ≤ bern2 6 (2 * (n - 1)) (coefQ n P w o b A c) σ τ :=
    bern2_ge _ _ _ _ (fun p hp q hq => (hpos p hp q hq).le) hσ hτ
  -- strictly: every coefficient > eps, so the minimum coefficient m > eps bounds it below
  obtain ⟨m, hm, hmc⟩ : ∃ m, eps < m ∧ ∀ p ≤ 6, ∀ q ≤ 2 * (n - 1), m ≤ coefQ n P w o b A c p q := by
    set s := (range 7 ×ˢ range (2 * (n - 1) + 1)).image (fun x => coefQ n P w o b A c x.1 x.2)
    have ne : s.Nonempty := by simp [s]
    refine ⟨s.min' ne, ?_, fun p hp q hq => s.min'_le _ (mem_image.mpr ⟨(p, q), by
      simp [mem_product]; omega, rfl⟩)⟩
    obtain ⟨x, hx, hx'⟩ := mem_image.mp (s.min'_mem ne)
    rw [← hx']; simp [mem_product] at hx; exact hpos _ (by omega) _ (by omega)
  have g2 : m ≤ bern2 6 (2 * (n - 1)) (coefQ n P w o b A c) σ τ := bern2_ge _ _ _ _ hmc hσ hτ
  have : bern2 6 (2 * (n - 1)) (coefQ n P w o b A c) σ τ = wden n w τ ^ 2 * quadQ o b A c S := by
    rw [e, bern2]; refine sum_congr rfl fun p _ => sum_congr rfl fun q _ => ?_; ring
  have hW2 : 0 < wden n w τ ^ 2 := by positivity
  have : 0 < wden n w τ ^ 2 * quadQ o b A c S := by linarith
  exact pos_of_mul_pos_right this hW2.le

end LoftSSI
