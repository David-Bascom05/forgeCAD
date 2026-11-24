import Mathlib.Topology.MetricSpace.Basic
import Mathlib.Analysis.Calculus.LocalExtr.Rolle

/-!
# C.place: `place_cuts` merging cuts within tolerance, and §E's δ-faithfulness

forge-bool's `place_cuts` (boolean.rs) puts two cuts on one edge into one vertex when their points
are within `reach = max(tol)` of each other (a cluster's head and its members). On the corpus
(curved-fallback-rates.md, C.place) 13 such merges have 1e-9 < d ≤ reach:

* K1 (8): one root of one surface along one edge, computed twice (from the edge's two faces): the
  merge is of one exact point, given that root is unique within the reach (open: `RootUnique`);
* K2 (4, fuzz11): two different surfaces meeting the edge at exact points 1.57e-7 apart;
* K3 (1, fuzz27): a touch and a crossing of one curve, 1.92e-7 apart.

§E's faithfulness (F1, H.fin) asks every vertex to lie within δ of its exact counterpart. For K2
and K3 the merged vertex stands for two exact points; `merge_within` shows it is within δ of both
whenever the code's distances and the vertex's placement are, so the merge keeps F1 (validity is
the separate check the result passes: forge_topo::check, clean on fuzz11 and fuzz27).
-/

namespace ForgeCurved.PlaceMerge

variable {α : Type*} [PseudoMetricSpace α]

/-- **The merged vertex stays faithful.** Exact points `p`, `q` with computed points `p'`, `q'`
within `η` of them, the code merging when `dist p' q' ≤ reach`, and the vertex `v` placed within
`ρ` of `p'`: then `v` is within `ρ + η` of `p` and within `ρ + reach + η` of `q`. With
`ρ + reach + η ≤ δ`, both exact points are within δ of the one vertex. -/
theorem merge_within {p q p' q' v : α} {η reach ρ δ : ℝ}
    (hp : dist p' p ≤ η) (hq : dist q' q ≤ η) (hm : dist p' q' ≤ reach) (hv : dist v p' ≤ ρ)
    (hδ : ρ + reach + η ≤ δ) (hr : 0 ≤ reach) :
    dist v p ≤ δ ∧ dist v q ≤ δ := by
  constructor
  · calc dist v p ≤ dist v p' + dist p' p := dist_triangle _ _ _
      _ ≤ ρ + η := add_le_add hv hp
      _ ≤ δ := by linarith
  · calc dist v q ≤ dist v p' + dist p' q' + dist q' q := dist_triangle4 _ _ _ _
      _ ≤ ρ + reach + η := by linarith
      _ ≤ δ := hδ

/-- K1's hypothesis: the root of the surface along the edge is unique within the cluster's reach,
so two computed hits within it are one exact point. -/
def RootUnique (roots : Set α) (reach : ℝ) : Prop :=
  ∀ x ∈ roots, ∀ y ∈ roots, dist x y ≤ reach → x = y

open Set in
/-- **At most one root where the derivative does not vanish** (Rolle): g, the other surface's
implicit along the edge's parameter, differentiable on `[lo, hi]` with g' ≠ 0 there, has at most
one zero in it. `krawczyk1`'s test (forge-geom isolate.rs; the oracle's rim_cert) encloses g' over
the interval away from 0, which is this hypothesis. -/
theorem zero_unique_of_deriv_ne {g g' : ℝ → ℝ} {lo hi : ℝ}
    (hg : ∀ x ∈ Icc lo hi, HasDerivAt g (g' x) x) (hne : ∀ x ∈ Icc lo hi, g' x ≠ 0)
    {s t : ℝ} (hs : s ∈ Icc lo hi) (ht : t ∈ Icc lo hi) (h0s : g s = 0) (h0t : g t = 0) :
    s = t := by
  by_contra hst
  rcases lt_or_gt_of_ne hst with h | h
  · obtain ⟨c, hc, hc0⟩ := exists_hasDerivAt_eq_zero (f := g) (f' := g') h
      (fun x hx => (hg x ⟨hs.1.trans hx.1, hx.2.trans ht.2⟩).continuousAt.continuousWithinAt)
      (by rw [h0s, h0t])
      (fun x hx => hg x ⟨hs.1.trans hx.1.le, hx.2.le.trans ht.2⟩)
    exact hne c ⟨hs.1.trans hc.1.le, hc.2.le.trans ht.2⟩ hc0
  · obtain ⟨c, hc, hc0⟩ := exists_hasDerivAt_eq_zero (f := g) (f' := g') h
      (fun x hx => (hg x ⟨ht.1.trans hx.1, hx.2.trans hs.2⟩).continuousAt.continuousWithinAt)
      (by rw [h0t, h0s])
      (fun x hx => hg x ⟨ht.1.trans hx.1.le, hx.2.le.trans hs.2⟩)
    exact hne c ⟨ht.1.trans hc.1.le, hc.2.le.trans hs.2⟩ hc0

open Set in
/-- **RootUnique from the parameter.** An edge curve `γ`, the roots its points of the surface,
`g = F ∘ γ` with g' ≠ 0 on the parameter interval `[lo, hi]` that holds every root within the
reach (the oracle's interval: the reach over the curve's speed either side of the hit): then the
roots within the reach are one point. -/
theorem rootUnique_of_param {γ : ℝ → α} {g g' : ℝ → ℝ} {lo hi reach : ℝ}
    (hg : ∀ x ∈ Icc lo hi, HasDerivAt g (g' x) x) (hne : ∀ x ∈ Icc lo hi, g' x ≠ 0)
    (roots : Set α) (hroot : ∀ x ∈ roots, ∃ t ∈ Icc lo hi, γ t = x ∧ g t = 0) :
    RootUnique roots reach := by
  intro x hx y hy _
  obtain ⟨s, hs, rfl, h0s⟩ := hroot x hx
  obtain ⟨t, ht, rfl, h0t⟩ := hroot y hy
  rw [zero_unique_of_deriv_ne hg hne hs ht h0s h0t]

end ForgeCurved.PlaceMerge
