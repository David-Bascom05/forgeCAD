import LoftMeet.Quadric

/-!
# Subdivision, the lines case, and the sign test on an arc piece

loftmeet.rs never runs de Casteljau: `halves` splits the parameter box at its midpoint and
`Skinned::hull` recomputes each half's control points from the span's power form. Soundness of the
recursion needs only (1) the halves cover the parent and (2) every piece's points are valid for
that piece (`patch_exact`, `bpts_exact`, for any interval). (3) shows the recomputed points describe
the same polynomial as de Casteljau's on the parent's points.
-/

namespace LoftSSI

open Finset

/-- (1) `halves` (and `pieces`, whose ends k/n are shared exactly) cover the parent. -/
theorem halves_cover {lo m hi x : ℝ} (h1 : lo ≤ x) (h2 : x ≤ hi) :
    (lo ≤ x ∧ x ≤ m) ∨ (m ≤ x ∧ x ≤ hi) := by
  rcases le_total x m with h | h
  · exact Or.inl ⟨h1, h⟩
  · exact Or.inr ⟨h, h2⟩

/-- The recursion: a pair of boxes is clear (no common point) when both children of a split box
are clear against the other box. -/
theorem clear_of_halves {α β γ : Type*} (f : α → γ) (g : β → γ) (X X1 X2 : Set α) (Y : Set β)
    (hX : X ⊆ X1 ∪ X2) (h1 : ∀ x ∈ X1, ∀ y ∈ Y, f x ≠ g y) (h2 : ∀ x ∈ X2, ∀ y ∈ Y, f x ≠ g y) :
    ∀ x ∈ X, ∀ y ∈ Y, f x ≠ g y := by
  intro x hx y hy
  rcases hX hx with h | h
  · exact h1 x h y hy
  · exact h2 x h y hy

/-- (3) Recomputing the points over the left part [b0, b0 + r (b1 - b0)] gives the same
polynomial as de Casteljau's left points of the parent's (`bern_split_left`). -/
theorem recompute_eq_casteljau (c0 c1 c2 c3 b0 b1 r σ : ℝ) :
    bern 3 (bpts c0 c1 c2 c3 b0 (b0 + r * (b1 - b0))) σ =
      bern 3 (fun j => ∑ i ∈ range 4, bpts c0 c1 c2 c3 b0 b1 i * bb j i r) σ := by
  rw [← bpts_exact, ← bern_split_left, ← bpts_exact]
  ring_nf

/-- The same for the right part [b0 + r (b1 - b0), b1] (`bern_split_right`). -/
theorem recompute_eq_casteljau_right (c0 c1 c2 c3 b0 b1 r σ : ℝ) :
    bern 3 (bpts c0 c1 c2 c3 (b0 + r * (b1 - b0)) b1) σ =
      bern 3 (fun j => ∑ i ∈ range 4, bpts c0 c1 c2 c3 b0 b1 i * bb (3 - j) (3 - i) (1 - r)) σ := by
  rw [← bpts_exact, ← bern_split_right, ← bpts_exact]
  ring_nf

/-- Lines (`Poly::hull` with no arcs): S = cub(A) + cub(E) t, control points a_r + e_r t0 and
a_r + e_r t1, weights 1: exact, with t = t0 + (t1 - t0) τ. -/
theorem lines_exact (A E : ℕ → ℝ) (b0 b1 t0 t1 σ τ : ℝ) :
    cubv A (b0 + (b1 - b0) * σ) + cubv E (b0 + (b1 - b0) * σ) * (t0 + (t1 - t0) * τ) =
      ∑ r ∈ range 4, ∑ j ∈ range 2, bb 3 r σ * bb 1 j τ *
        (bp A b0 b1 r + bp E b0 b1 r * (if j = 0 then t0 else t1)) := by
  simp only [cubv, bp, bpts_exact]
  simp only [bern, bb, sum_range_succ, sum_range_zero]
  norm_num [Nat.choose]
  ring

/-- The arc piece's points satisfy the rational-patch hypothesis of `quad_subst` with the code's
affine control points (`cpt`) and weights 1, cos h, 1 (needs cos h ≠ 0; the code has |h| ≤ π/4). -/
theorem arc_rational (A B D : Fin 3 → ℕ → ℝ) (b0 b1 xm h τ σ θ : ℝ) (hw : Real.cos h ≠ 0)
    (hex : ∀ (A B D : ℕ → ℝ) (b0 b1 σ : ℝ),
      arcW h (2 * τ - 1) * spanV A B D (b0 + (b1 - b0) * σ) θ = patchN A B D b0 b1 xm h σ τ) :
    ∀ c', wden 3 (aw h) τ * (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) c' =
      ∑ x ∈ idx 3, al 3 σ τ x * aw h x.2 *
        (fun r j c => cpt (A c) (B c) (D c) b0 b1 xm h r j) x.1 x.2 c' := by
  intro c'
  have hW : wden 3 (aw h) τ = arcW h (2 * τ - 1) := by
    rw [← (arc_bernstein h τ).1, wden, bern]
    refine sum_congr rfl fun j _ => ?_; unfold aw; ring
  have awne : ∀ j, aw h j ≠ 0 := fun j => by unfold aw; split_ifs <;> simp [hw]
  simp only [hW, hex, patchN, idx, sum_product, al]
  refine sum_congr rfl fun r _ => sum_congr rfl fun j _ => ?_
  rw [hpt_eq _ _ _ _ _ _ _ _ _ (awne j)]; ring

/-- **`Hull::misses` on an arc piece.** If every coefficient the code computes (exactly) exceeds
some eps ≥ 0, the quadric is positive at every point of the piece (θ within h of xm, b in
[b0, b1]): the piece misses it. Holds for any piece turning less than a full turn with cos h ≠ 0;
the sign of the weight does not matter here. -/
theorem arc_misses (A B D : Fin 3 → ℕ → ℝ) (o bq : Fin 3 → ℝ) (Aq : Matrix (Fin 3) (Fin 3) ℝ)
    (cq b0 b1 xm h θ σ eps : ℝ) (hh : |h| < Real.pi) (hw : Real.cos h ≠ 0) (heps : 0 ≤ eps)
    (hθ : |θ - xm| ≤ |h|) (hσ : 0 ≤ σ ∧ σ ≤ 1)
    (hpos : ∀ p ≤ 6, ∀ q ≤ 2 * (3 - 1), eps <
      coefQ 3 (fun r j c => cpt (A c) (B c) (D c) b0 b1 xm h r j) (aw h) o bq Aq cq p q) :
    0 < quadQ o bq Aq cq (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) := by
  obtain ⟨τ, hτ0, hτ1, hWpos, hex⟩ := patch_exact hh hθ
  have hW : wden 3 (aw h) τ = arcW h (2 * τ - 1) := by
    rw [← (arc_bernstein h τ).1, wden, bern]
    refine sum_congr rfl fun j _ => ?_; unfold aw; ring
  exact misses_sound 3 _ (aw h) o bq Aq cq σ τ (by norm_num) _ eps heps (by rw [hW]; exact hWpos.ne')
    (arc_rational A B D b0 b1 xm h τ σ θ hw hex) hσ ⟨hτ0, hτ1⟩ hpos

end LoftSSI
