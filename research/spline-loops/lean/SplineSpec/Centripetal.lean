/-
The through-spline's parameters: centripetal (Lee 1989), `τ 0 = 0`,
`τ (i+1) = τ i + s · |q (i+1) - q i|^(1/2)`, scaled by `s = L / Σ |Δq|^(1/2)` so that they run as
long as the polygon (L its length), as `Spline::through` takes them. They increase strictly exactly
when consecutive points differ, so then the through-spline exists and is unique (`spline_exists_unique`).
-/
import SplineSpec.Interp
import Mathlib.Analysis.SpecialFunctions.Pow.Real

open Finset

namespace SplineSpec

variable {d : ℕ}

/-- `|q (i+1) - q i|`, the Euclidean length of a chord. -/
noncomputable def chord (q : ℕ → Fin d → ℝ) (i : ℕ) : ℝ :=
  Real.sqrt (∑ c, (q (i + 1) c - q i c) ^ 2)

/-- The centripetal parameters of `q 0 .. q m`, scaled to the polygon's length. -/
noncomputable def centripetal (q : ℕ → Fin d → ℝ) (m : ℕ) (i : ℕ) : ℝ :=
  (∑ k ∈ range m, chord q k) / (∑ k ∈ range m, Real.sqrt (chord q k)) *
    ∑ k ∈ range i, Real.sqrt (chord q k)

theorem chord_pos {q : ℕ → Fin d → ℝ} {i : ℕ} (h : q (i + 1) ≠ q i) : 0 < chord q i := by
  unfold chord
  apply Real.sqrt_pos.2
  obtain ⟨c, hc⟩ : ∃ c, q (i + 1) c ≠ q i c := by
    by_contra hne; push_neg at hne; exact h (funext hne)
  have : 0 < (q (i + 1) c - q i c) ^ 2 := by
    have := sub_ne_zero.2 hc; positivity
  exact lt_of_lt_of_le this (single_le_sum (f := fun c => (q (i + 1) c - q i c) ^ 2)
    (fun _ _ => sq_nonneg _) (mem_univ c))

/-- **The parameters increase strictly** when consecutive points differ. -/
theorem centripetal_strictMono {q : ℕ → Fin d → ℝ} {m : ℕ} (h : ∀ i < m, q (i + 1) ≠ q i) :
    ∀ i < m, centripetal q m i < centripetal q m (i + 1) := by
  intro i hi
  unfold centripetal
  rw [sum_range_succ]
  have hs : ∀ k < m, 0 < Real.sqrt (chord q k) := fun k hk => Real.sqrt_pos.2 (chord_pos (h k hk))
  have hL : 0 < ∑ k ∈ range m, chord q k :=
    sum_pos (fun k hk => chord_pos (h k (mem_range.1 hk))) ⟨i, mem_range.2 hi⟩
  have hS : 0 < ∑ k ∈ range m, Real.sqrt (chord q k) :=
    sum_pos (fun k hk => hs k (mem_range.1 hk)) ⟨i, mem_range.2 hi⟩
  have := hs i hi
  have hk : 0 < (∑ k ∈ range m, chord q k) / (∑ k ∈ range m, Real.sqrt (chord q k)) := div_pos hL hS
  nlinarith

/-- **The through-spline of `spline(..)`**: at the centripetal parameters, for consecutive points
distinct (and the last the first when periodic), each coordinate has exactly one C2 cubic spline
through the points with the end conditions. -/
theorem spline_exists_unique {q : ℕ → Fin d → ℝ} {m : ℕ} (e : Fin d → Ends ℝ)
    (hq : ∀ i < m, q (i + 1) ≠ q i)
    (hsize : ∀ c, match e c with | .open_ _ _ => 1 ≤ m | .periodic => 2 ≤ m)
    (hclose : ∀ c, e c = .periodic → q m = q 0) (c : Fin d) :
    (∃ B, IsInterp (centripetal q m) (fun i => q i c) m (e c) B) ∧
    ∀ B B', IsInterp (centripetal q m) (fun i => q i c) m (e c) B →
      IsInterp (centripetal q m) (fun i => q i c) m (e c) B' → ∀ i < m, ∀ k ≤ 3, B i k = B' i k :=
  interp_exists_unique
    { pos := centripetal_strictMono hq
      size := by have h := hsize c; revert h; cases e c <;> exact id
      closes := fun he => by simp [hclose c he] }

end SplineSpec
