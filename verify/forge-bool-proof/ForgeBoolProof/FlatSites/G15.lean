import Mathlib.Analysis.InnerProductSpace.Basic

/-!
# G15: a straight stretch whose middle is exactly on a face's edge line

old-path-statements.md §G, G15. In `trim_in`, a stretch of a flat pair's line between two
consecutive cuts is classified by its middle `M` (`at(k, M)`, B:3387). When `M` is exactly on an
edge of `f_k`, today's code calls the stretch `On`: it lies along that edge (both ends cut on it and
the middle on it: `cut_along`), so it is boundary and not a new edge, or another sample decides.

On flat faces this is the only case. The edge's carrier, in the face's plane, is the zero set of an
affine function `x ↦ ⟪n, x⟫ - c`. Along the line `γ t = a + t • v` that function is affine in `t`:
* **`along_or_crossing`**: either it is zero all along (`⟪n, v⟫ = 0`: the line lies on the edge's
  carrier) or it vanishes at `M`'s parameter only, with opposite signs either side (`crossing`).
* A transversal zero inside the stretch is a crossing of the edge's carrier there. If `M` is on the
  edge itself, it is a crossing of `∂f_k`, which D4's cut completeness lists as a cut, so it is not
  inside a stretch. Hence (**`on_middle_along`**) a stretch with no cut inside, whose middle is
  exactly on the edge's carrier, lies on that carrier entirely: today's "along the edge" reading is
  the exact one.

Ledger: G15 (flat) proven modulo D4's cut completeness; curved G15 (a conic touching an edge at a
point, a touch rather than a crossing) is out of this file's scope.
-/

namespace ForgeBoolProof.FlatSites.G15

variable {V : Type} [NormedAddCommGroup V] [InnerProductSpace ℝ V]

open RealInnerProductSpace

/-- The edge carrier's function along the line `a + t • v`. -/
theorem along_affine (n a v : V) (c t : ℝ) :
    ⟪n, a + t • v⟫ - c = (⟪n, a⟫ - c) + t * ⟪n, v⟫ := by
  rw [inner_add_right, inner_smul_right]; ring

/-- Either the line lies on the carrier, or the carrier's function changes sign strictly at the
line's zero `t0`. -/
theorem along_or_crossing (n a v : V) (c t0 : ℝ) (h0 : ⟪n, a + t0 • v⟫ - c = 0) :
    (∀ t : ℝ, ⟪n, a + t • v⟫ - c = 0) ∨
      (∀ t : ℝ, t ≠ t0 → ⟪n, a + t • v⟫ - c ≠ 0) ∧
      (∀ s t : ℝ, s < t0 → t0 < t → (⟪n, a + s • v⟫ - c) * (⟪n, a + t • v⟫ - c) < 0) := by
  rw [along_affine] at h0
  by_cases hv : ⟪n, v⟫ = 0
  · left; intro t; rw [along_affine, hv]; rw [hv] at h0; linarith
  · right
    have key : ∀ t, ⟪n, a + t • v⟫ - c = (t - t0) * ⟪n, v⟫ := by
      intro t; rw [along_affine]; linarith
    refine ⟨fun t ht => ?_, fun s t hs ht => ?_⟩
    · rw [key]; exact mul_ne_zero (sub_ne_zero.mpr ht) hv
    · rw [key, key]
      have h1 : s - t0 < 0 := by linarith
      have h2 : 0 < t - t0 := by linarith
      have : (s - t0) * (t - t0) < 0 := mul_neg_of_neg_of_pos h1 h2
      nlinarith [sq_pos_of_ne_zero hv]

/-- **A stretch with no crossing inside, whose middle is exactly on the edge's carrier, lies on the
carrier.** `nocross`: the carrier's function does not change sign strictly inside the stretch
(D4: every crossing of `∂f_k` is a cut, and a cut ends a stretch). -/
theorem on_middle_along (n a v : V) (c t1 t2 : ℝ) (h12 : t1 < t2)
    (hmid : ⟪n, a + ((t1 + t2) / 2) • v⟫ - c = 0)
    (nocross : ∀ s t : ℝ, t1 < s → s < t → t < t2 →
      0 ≤ (⟪n, a + s • v⟫ - c) * (⟪n, a + t • v⟫ - c)) :
    ∀ t : ℝ, ⟪n, a + t • v⟫ - c = 0 := by
  rcases along_or_crossing n a v c _ hmid with h | ⟨-, hsign⟩
  · exact h
  · exfalso
    set m := (t1 + t2) / 2
    have hs : t1 < (t1 + m) / 2 := by simp only [m]; linarith
    have hm1 : (t1 + m) / 2 < m := by simp only [m]; linarith
    have hm2 : m < (m + t2) / 2 := by simp only [m]; linarith
    have ht : (m + t2) / 2 < t2 := by simp only [m]; linarith
    have := hsign ((t1 + m) / 2) ((m + t2) / 2) hm1 hm2
    have := nocross ((t1 + m) / 2) ((m + t2) / 2) hs (by linarith) ht
    linarith

end ForgeBoolProof.FlatSites.G15
