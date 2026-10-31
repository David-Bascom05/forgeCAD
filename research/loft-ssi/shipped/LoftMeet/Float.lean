import LoftMeet.Subdiv

/-!
# What the f64 tests need of their rounding

Error bounds E are hypotheses here; lean-derive's Rounding.lean (derive/proofs) proves the
γ_k-style bounds |computed - exact| ≤ (θ_k - 1) Σ|terms| that supply them. fp/fp_check.py measures
them for the code's exact operation order.
-/

namespace LoftSSI

open Finset

/-- `apart` with rounded extents: if each computed extent is within E of the exact projection of
the exact control points, the code's test `ix_hi + gap < iy_lo` still proves the exact hulls apart
as long as the two errors fit in the gap. tol::LINEAR = 1e-5 against ~12.5 u L measured. -/
theorem apart_rounded {ixhi iylo gap ex ey : ℝ} {X Y : Set ℝ}
    (hx : ∀ p ∈ X, p ≤ ixhi + ex) (hy : ∀ q ∈ Y, iylo - ey ≤ q)
    (hsep : ixhi + gap < iylo) (hfit : ex + ey ≤ gap) : ∀ p ∈ X, ∀ q ∈ Y, p < q := by
  intro p hp q hq; linarith [hx p hp, hy q hq]

/-- `misses` with rounded coefficients: computed coefficients ĉ all above eps ≥ 0, each within E of
the exact one. Then the exact Bernstein polynomial, W² Q(S), is above eps - E on the piece. The
code's eps = 1e-12 max|ĉ| is relative and is NOT a bound on E (cancellation; measured E up to
4.7e-5 max|ĉ|), so the theorem the code gets is this one: a cleared piece can still reach
Q(S) W² > -E, i.e. dip into the quadric by about E / (|∇Q| W²), never more. -/
theorem misses_rounded (n : ℕ) (P : ℕ → ℕ → Fin 3 → ℝ) (w : ℕ → ℝ) (o b : Fin 3 → ℝ)
    (A : Matrix (Fin 3) (Fin 3) ℝ) (c σ τ eps E : ℝ) (hn : 1 ≤ n) (S : Fin 3 → ℝ)
    (chat : ℕ → ℕ → ℝ)
    (hS : ∀ c', wden n w τ * S c' = ∑ x ∈ idx n, al n σ τ x * w x.2 * P x.1 x.2 c')
    (hσ : 0 ≤ σ ∧ σ ≤ 1) (hτ : 0 ≤ τ ∧ τ ≤ 1)
    (herr : ∀ p ≤ 6, ∀ q ≤ 2 * (n - 1), |chat p q - coefQ n P w o b A c p q| ≤ E)
    (hclear : ∀ p ≤ 6, ∀ q ≤ 2 * (n - 1), eps < chat p q) :
    eps - E ≤ wden n w τ ^ 2 * quadQ o b A c S := by
  rw [quad_subst n P w o b A c σ τ hn S hS]
  have g := bern2_ge 6 (2 * (n - 1)) (coefQ n P w o b A c) (eps - E) (fun p hp q hq => by
    have := (abs_le.mp (herr p hp q hq)); linarith [hclear p hp q hq]) hσ hτ
  rw [bern2] at g
  calc eps - E ≤ _ := g
    _ = _ := sum_congr rfl fun p _ => sum_congr rfl fun q _ => by ring

/-- When the rounding does fit in the eps (E ≤ eps), the cleared piece truly misses the quadric. -/
theorem misses_rounded_sound (n : ℕ) (P : ℕ → ℕ → Fin 3 → ℝ) (w : ℕ → ℝ) (o b : Fin 3 → ℝ)
    (A : Matrix (Fin 3) (Fin 3) ℝ) (c σ τ eps E : ℝ) (hn : 1 ≤ n) (S : Fin 3 → ℝ)
    (chat : ℕ → ℕ → ℝ) (hW : wden n w τ ≠ 0)
    (hS : ∀ c', wden n w τ * S c' = ∑ x ∈ idx n, al n σ τ x * w x.2 * P x.1 x.2 c')
    (hσ : 0 ≤ σ ∧ σ ≤ 1) (hτ : 0 ≤ τ ∧ τ ≤ 1)
    (herr : ∀ p ≤ 6, ∀ q ≤ 2 * (n - 1), |chat p q - coefQ n P w o b A c p q| ≤ E)
    (hclear : ∀ p ≤ 6, ∀ q ≤ 2 * (n - 1), eps < chat p q) (hfit : E ≤ eps) :
    0 < quadQ o b A c S := by
  have hE : 0 ≤ E := le_trans (abs_nonneg _) (herr 0 (by omega) 0 (by omega))
  exact misses_sound n P w o b A c σ τ hn S 0 le_rfl hW hS hσ hτ (fun p hp q hq => by
    have := (abs_le.mp (herr p hp q hq)); linarith [hclear p hp q hq])

end LoftSSI
