import LoftMeet.Hull

/-!
# The span piece is the rational Bezier patch; hull, separation, and why each assumption is there

A coordinate of an arc span piece is cub(A) + cub(B) cos θ + cub(D) sin θ (`Poly::point`); the code's
control points (`Poly::hull`) are, in homogeneous form with the weights 1, cos h, 1 folded in,
  H r j = w_j a_r + b_r hc_j + d_r hs_j,   (hc, hs) = (cos, sin) of xm - h, xm, xm + h,
a_r, b_r, d_r the cubics' Bernstein points over [b0, b1] (`bpts`).
-/

namespace LoftSSI

open Finset

noncomputable def cubv (c : ℕ → ℝ) (b : ℝ) : ℝ := cub (c 0) (c 1) (c 2) (c 3) b
noncomputable def bp (c : ℕ → ℝ) (b0 b1 : ℝ) (r : ℕ) : ℝ := bpts (c 0) (c 1) (c 2) (c 3) b0 b1 r

/-- The arc weights 1, cos h, 1. -/
noncomputable def aw (h : ℝ) (j : ℕ) : ℝ := if j = 1 then Real.cos h else 1
/-- Weighted (homogeneous) cos / sin parts of the arc's control points. -/
noncomputable def hc (xm h : ℝ) (j : ℕ) : ℝ :=
  if j = 0 then Real.cos (xm - h) else if j = 1 then Real.cos xm else Real.cos (xm + h)
noncomputable def hs (xm h : ℝ) (j : ℕ) : ℝ :=
  if j = 0 then Real.sin (xm - h) else if j = 1 then Real.sin xm else Real.sin (xm + h)

/-- Homogeneous control point (one coordinate). -/
noncomputable def hpt (A B D : ℕ → ℝ) (b0 b1 xm h : ℝ) (r j : ℕ) : ℝ :=
  aw h j * bp A b0 b1 r + bp B b0 b1 r * hc xm h j + bp D b0 b1 r * hs xm h j

/-- The code's affine control point `a[r] + bb[r] * cx + dd[r] * sy`, cx, sy = cos, sin of x0,
then cos xm / cos h, sin xm / cos h, then of x1. -/
noncomputable def cpt (A B D : ℕ → ℝ) (b0 b1 xm h : ℝ) (r j : ℕ) : ℝ :=
  bp A b0 b1 r + bp B b0 b1 r * (hc xm h j / aw h j) + bp D b0 b1 r * (hs xm h j / aw h j)

theorem hpt_eq (A B D : ℕ → ℝ) (b0 b1 xm h : ℝ) (r j : ℕ) (hw : aw h j ≠ 0) :
    hpt A B D b0 b1 xm h r j = aw h j * cpt A B D b0 b1 xm h r j := by
  unfold hpt cpt; field_simp

/-- The patch (one coordinate): Σ_r Σ_j B³_r(σ) B²_j(τ) H r j. -/
noncomputable def patchN (A B D : ℕ → ℝ) (b0 b1 xm h σ τ : ℝ) : ℝ :=
  ∑ r ∈ range 4, ∑ j ∈ range 3, bb 3 r σ * bb 2 j τ * hpt A B D b0 b1 xm h r j

/-- The span point (one coordinate), `Poly::point`'s value. -/
noncomputable def spanV (A B D : ℕ → ℝ) (b θ : ℝ) : ℝ :=
  cubv A b + cubv B b * Real.cos θ + cubv D b * Real.sin θ

/-- **Exact conversion, surface ⊆ patch.** For a piece turning less than a full turn, every point
of the span piece (b in [b0, b1], θ within h of xm) is the rational patch at some (σ, τ) in the
unit square, with denominator W > 0; τ depends only on θ, so it is the same for all coordinates. -/
theorem patch_exact {h xm θ : ℝ} (hh : |h| < Real.pi) (hθ : |θ - xm| ≤ |h|) :
    ∃ τ, 0 ≤ τ ∧ τ ≤ 1 ∧ 0 < arcW h (2 * τ - 1) ∧
      ∀ (A B D : ℕ → ℝ) (b0 b1 σ : ℝ),
        arcW h (2 * τ - 1) * spanV A B D (b0 + (b1 - b0) * σ) θ = patchN A B D b0 b1 xm h σ τ := by
  obtain ⟨z, hz, hW, hx, hy⟩ := arc_hits hh hθ
  have hz1 : -1 ≤ z ∧ z ≤ 1 := by constructor <;> nlinarith [sq_nonneg (z + 1), sq_nonneg (z - 1)]
  refine ⟨(z + 1) / 2, by linarith, by linarith, by rwa [show 2 * ((z + 1) / 2) - 1 = z by ring], ?_⟩
  intro A B D b0 b1 σ
  rw [show 2 * ((z + 1) / 2) - 1 = z by ring]
  have key : patchN A B D b0 b1 xm h σ ((z + 1) / 2) =
      arcW h z * cubv A (b0 + (b1 - b0) * σ) +
      cubv B (b0 + (b1 - b0) * σ) * (Real.cos xm * arcNx h z - Real.sin xm * arcNy h z) +
      cubv D (b0 + (b1 - b0) * σ) * (Real.sin xm * arcNx h z + Real.cos xm * arcNy h z) := by
    simp only [cubv, bpts_exact]
    simp only [patchN, hpt, bp, hc, hs, aw, bern, bb, arcW, arcNx, arcNy, Real.cos_add,
      Real.sin_add, Real.cos_sub, Real.sin_sub, sum_range_succ, sum_range_zero]
    norm_num [Nat.choose]
    ring
  rw [key, spanV, show θ = xm + (θ - xm) by ring, Real.cos_add, Real.sin_add]
  linear_combination
    (cubv B (b0 + (b1 - b0) * σ) * Real.cos xm + cubv D (b0 + (b1 - b0) * σ) * Real.sin xm) * hx +
    (cubv D (b0 + (b1 - b0) * σ) * Real.cos xm - cubv B (b0 + (b1 - b0) * σ) * Real.sin xm) * hy

/-! ## Convex hull (positive weights) and the separating-axis test -/

/-- A linear functional on points (`p.dot(u)` in `Hull::extent`; any u, unit or not). -/
def ell (u p : Fin 3 → ℝ) : ℝ := ∑ c, u c * p c

/-- **Hull property.** With positive weights (|h| < π/2, i.e. the piece turns under half a turn),
every point of the piece projects into [min, max] of its 12 control points' projections, along any
axis u. Nothing else needs to be convex. -/
theorem patch_hull {h xm θ b0 b1 σ M : ℝ} (hh : |h| < Real.pi / 2) (hθ : |θ - xm| ≤ |h|)
    (hσ0 : 0 ≤ σ) (hσ1 : σ ≤ 1) (A B D : Fin 3 → ℕ → ℝ) (u : Fin 3 → ℝ)
    (hM : ∀ r < 4, ∀ j < 3, ell u (fun c => cpt (A c) (B c) (D c) b0 b1 xm h r j) ≤ M) :
    ell u (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) ≤ M := by
  have hpi := Real.pi_pos
  obtain ⟨τ, hτ0, hτ1, hW, hex⟩ := patch_exact (h := h) (xm := xm) (θ := θ) (by linarith) hθ
  have hc : 0 < Real.cos h := Real.cos_pos_of_mem_Ioo ⟨by linarith [neg_abs_le h], by
    linarith [le_abs_self h]⟩
  have awpos : ∀ j, 0 < aw h j := fun j => by unfold aw; split_ifs <;> simp [hc]
  set W := arcW h (2 * τ - 1)
  have e2 : ∑ j ∈ range 3, bb 2 j τ * aw h j = W := by
    show _ = arcW h (2 * τ - 1)
    rw [← (arc_bernstein h τ).1, bern]; refine sum_congr rfl fun j _ => ?_; unfold aw; ring
  have hsumW : ∑ r ∈ range 4, ∑ j ∈ range 3, bb 3 r σ * bb 2 j τ * aw h j = W := by
    simp_rw [mul_assoc, ← mul_sum, e2, ← sum_mul, bb_sum, one_mul]
  have coord : ∀ c, W * spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ =
      ∑ r ∈ range 4, ∑ j ∈ range 3, bb 3 r σ * bb 2 j τ * aw h j *
        cpt (A c) (B c) (D c) b0 b1 xm h r j := by
    intro c
    rw [hex, patchN]
    refine sum_congr rfl fun r _ => sum_congr rfl fun j _ => ?_
    rw [hpt_eq _ _ _ _ _ _ _ _ _ (awpos j).ne']; ring
  have key : W * ell u (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) =
      ∑ r ∈ range 4, ∑ j ∈ range 3, bb 3 r σ * bb 2 j τ * aw h j *
        ell u (fun c => cpt (A c) (B c) (D c) b0 b1 xm h r j) := by
    have : W * ell u (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) =
        ∑ c, u c * (W * spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) := by
      unfold ell; rw [mul_sum]; refine sum_congr rfl fun c _ => ?_; ring
    rw [this]
    simp only [coord, ell, mul_sum]
    rw [sum_comm]; refine sum_congr rfl fun r _ => ?_
    rw [sum_comm]; refine sum_congr rfl fun j _ => ?_
    refine sum_congr rfl fun c _ => ?_; ring
  have le : W * ell u (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) ≤ W * M := by
    rw [key, ← hsumW, sum_mul]
    refine sum_le_sum fun r hr => ?_
    rw [sum_mul]
    refine sum_le_sum fun j hj => ?_
    simp only [mem_range] at hr hj
    have : 0 ≤ bb 3 r σ * bb 2 j τ * aw h j :=
      mul_nonneg (mul_nonneg (bb_nonneg _ _ hσ0 hσ1) (bb_nonneg _ _ hτ0 hτ1)) (awpos j).le
    exact mul_le_mul_of_nonneg_left (hM r hr j hj) this
  exact le_of_mul_le_mul_left le hW

theorem ell_neg (u p : Fin 3 → ℝ) : ell (-u) p = -ell u p := by
  simp [ell, sum_neg_distrib]

theorem patch_hull_ge {h xm θ b0 b1 σ m : ℝ} (hh : |h| < Real.pi / 2) (hθ : |θ - xm| ≤ |h|)
    (hσ0 : 0 ≤ σ) (hσ1 : σ ≤ 1) (A B D : Fin 3 → ℕ → ℝ) (u : Fin 3 → ℝ)
    (hm : ∀ r < 4, ∀ j < 3, m ≤ ell u (fun c => cpt (A c) (B c) (D c) b0 b1 xm h r j)) :
    m ≤ ell u (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) := by
  have := patch_hull (M := -m) hh hθ hσ0 hσ1 A B D (-u) (fun r hr j hj => by
    rw [ell_neg]; linarith [hm r hr j hj])
  rw [ell_neg] at this; linarith

/-- A point of the code's piece: θ = start + sweep·t for t in [t0, t1] lies within |h| of the
middle angle, h = (x1 - x0)/2, x0 = start + sweep t0, x1 = start + sweep t1 (any sweep sign). -/
theorem piece_angle {st sw t0 t1 t : ℝ} (h0 : t0 ≤ t) (h1 : t ≤ t1) :
    |(st + sw * t) - ((st + sw * t0) + ((st + sw * t1) - (st + sw * t0)) / 2)| ≤
      |((st + sw * t1) - (st + sw * t0)) / 2| := by
  have e1 : (st + sw * t) - ((st + sw * t0) + ((st + sw * t1) - (st + sw * t0)) / 2) =
      sw * (t - (t0 + t1) / 2) := by ring
  have e2 : ((st + sw * t1) - (st + sw * t0)) / 2 = sw * ((t1 - t0) / 2) := by ring
  rw [e1, e2, abs_mul, abs_mul]
  apply mul_le_mul_of_nonneg_left _ (abs_nonneg _)
  rw [abs_le, abs_of_nonneg (by linarith)]; constructor <;> linarith

/-- **Separating axis (`apart`).** If, along an axis u, X's control points project to at most
`ix_hi` and Y's to at least `iy_lo` with `ix_hi + gap < iy_lo` (gap ≥ 0, the code's tol::LINEAR),
then no point of piece X is a point of piece Y; along u they are more than `gap` apart. -/
theorem apart_sound {hX xmX hY xmY b0 b1 c0 c1 ixhi iylo gap : ℝ}
    (hX' : |hX| < Real.pi / 2) (hY' : |hY| < Real.pi / 2) (hgap : 0 ≤ gap)
    (A B D A' B' D' : Fin 3 → ℕ → ℝ) (u : Fin 3 → ℝ)
    (hx : ∀ r < 4, ∀ j < 3, ell u (fun c => cpt (A c) (B c) (D c) b0 b1 xmX hX r j) ≤ ixhi)
    (hy : ∀ r < 4, ∀ j < 3, iylo ≤ ell u (fun c => cpt (A' c) (B' c) (D' c) c0 c1 xmY hY r j))
    (hsep : ixhi + gap < iylo)
    {θ θ' σ σ' : ℝ} (hθ : |θ - xmX| ≤ |hX|) (hθ' : |θ' - xmY| ≤ |hY|)
    (hσ : 0 ≤ σ ∧ σ ≤ 1) (hσ' : 0 ≤ σ' ∧ σ' ≤ 1) :
    gap < ell u (fun c => spanV (A' c) (B' c) (D' c) (c0 + (c1 - c0) * σ') θ') -
      ell u (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) ∧
    (fun c => spanV (A c) (B c) (D c) (b0 + (b1 - b0) * σ) θ) ≠
      (fun c => spanV (A' c) (B' c) (D' c) (c0 + (c1 - c0) * σ') θ') := by
  have a := patch_hull hX' hθ hσ.1 hσ.2 A B D u hx
  have b := patch_hull_ge hY' hθ' hσ'.1 hσ'.2 A' B' D' u hy
  refine ⟨by linarith, fun e => ?_⟩
  rw [e] at a; linarith

end LoftSSI
