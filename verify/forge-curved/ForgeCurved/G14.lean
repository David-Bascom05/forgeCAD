import Mathlib.Order.Interval.Set.UnorderedInterval
import Mathlib.Basic.Real.Basic

/-!
# G14: trim_in's box-only accept adds only harmless cuts

old-path-statements.md §G, G14. `trim_in` (boolean.rs) cuts the pair curve `γ` of faces `f`
(its own) and `f'` (the other's) where `γ` crosses an edge of `f`. It accepts such a crossing when
the point lies in `f'`'s box (`other.contains(h.point) && !CAREFUL`), without asking whether the
point lies in `f'` itself. The curve is then split at its cuts, and a stretch between two
consecutive cuts is kept when its middle lies inside both faces (`at(k, M)`, B:3387).

A curve parameter is in `K` when `γ(t)` lies in the relative interior of both faces. A crossing of
`∂f` is never in `K`. So whatever the f'-test says, an accepted cut `x` satisfies `x ∉ K`.

The statement, over the parameter line, with no finiteness or order of the cuts assumed:
* `cell C t`: the parameters `s` joined to `t` by a stretch that meets no cut of `C`;
* `Sorted C0 K`: every cell of the true cuts `C0` lies in `K` or misses it (D4's contract: the
  true cuts are every crossing of `∂f` and `∂f'`, so `K` is constant on each cell);
* **`kept_iff`**: for any cut set `C ⊇ C0`, a cell of `C` lies in `K` iff the cell of `C0` round the
  same point does. The kept stretches are those of the true cuts, cut again only at extra points;
* **`extra_not_end`**: an extra cut `x ∉ K` never ends a kept stretch: a stretch from a point `t` to
  `x` with no cut of `C0` between them leaves `K` (it is part of one cell of `C0`, which holds
  `x ∉ K`, so that cell misses `K`).

Together: the cuts the box accepts beyond the true ones only fall inside stretches that are
dropped, so the kept segments and their ends are those of the true cuts. G14 is not a soundness
gap; the f'-test is an optimisation (it saves splitting `f`'s edge where no segment ends).

Measured (FLAT-SITES, 2026-11-23, gate parts seq + nary1, fuzz 1–40 seq): the box accepted 20,489
hits that point_in_face places Outside `f'`. Replacing the box accept by point_in_face (strict)
changed no face, edge or vertex count and no exact-fuzz verdict (agree 463 both ways, wrong 0);
bits moved only in the last ulps of vertex points (the extra cuts join cut clusters).
-/

namespace ForgeCurved.G14

open Set

/-- The parameters joined to `t` by a closed stretch that meets no cut. -/
def cell (C : Set ℝ) (t : ℝ) : Set ℝ := {s | Disjoint (uIcc t s) C}

/-- Each cell of the cut set `C0` lies in `K` or misses it. -/
def Sorted (C0 K : Set ℝ) : Prop := ∀ t, t ∉ C0 → cell C0 t ⊆ K ∨ Disjoint (cell C0 t) K

theorem mem_cell {C : Set ℝ} {t : ℝ} (ht : t ∉ C) : t ∈ cell C t := by
  simp only [cell, mem_ofPred_eq, uIcc_self, disjoint_singleton_left]
  exact ht

theorem cell_mono {C0 C : Set ℝ} (h : C0 ⊆ C) (t : ℝ) : cell C t ⊆ cell C0 t :=
  fun _ hs => Disjoint.mono_right h hs

/-- **The kept stretches are those of the true cuts.** -/
theorem kept_iff {C0 C K : Set ℝ} (hs : Sorted C0 K) (hsub : C0 ⊆ C) {t : ℝ} (ht : t ∉ C) :
    cell C t ⊆ K ↔ cell C0 t ⊆ K := by
  have ht0 : t ∉ C0 := fun h => ht (hsub h)
  constructor
  · intro h
    rcases hs t ht0 with h0 | h0
    · exact h0
    · exact absurd (h (mem_cell ht)) (Set.disjoint_left.mp h0 (mem_cell ht0))
  · exact fun h => (cell_mono hsub t).trans h

/-- **An extra cut outside `K` ends no kept stretch.** If no true cut lies between `t` and the
extra cut `x ∉ K`, the true cell round `t` holds `x`, so it misses `K`, and the stretch at `t` is
dropped. -/
theorem extra_not_end {C0 K : Set ℝ} (hs : Sorted C0 K) {t x : ℝ} (ht : t ∉ C0) (hxK : x ∉ K)
    (hfree : Disjoint (uIcc t x) C0) : Disjoint (cell C0 t) K := by
  rcases hs t ht with h0 | h0
  · exact absurd (h0 hfree) hxK
  · exact h0

/-- Every crossing of `∂f` is outside `K`, whatever the other face: so every cut `trim_in` puts on
an edge of `f`, accepted by the box or by `point_in_face`, satisfies `extra_not_end`'s `x ∉ K`. -/
theorem edge_cut_not_kept {X : Type} (γ : ℝ → X) (relint_f relint_f' bdry_f : Set X)
    (hdisj : Disjoint relint_f bdry_f) {x : ℝ} (hx : γ x ∈ bdry_f) :
    x ∉ {t | γ t ∈ relint_f ∧ γ t ∈ relint_f'} :=
  fun h => Set.disjoint_left.mp hdisj h.1 hx

end ForgeCurved.G14
