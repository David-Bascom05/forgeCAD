import ForgeCurved.BranchOrder
import Mathlib.Topology.Compactness.Compact
import Mathlib.Topology.Order.IntermediateValue
import Mathlib.Topology.Algebra.Order.Field
import Mathlib.Order.Filter.Basic

/-!
# (L2) A box round an event meets the curve in a star

Curved Rust's stage (b) matches branches at an event `x = e` with 3D boxes
`B = [x_l, x_r] × strip_y × strip_z`:
* over `e`, `B` holds at most one curve point `p` (QArc boxCrossings on its faces);
* no separator plane of the strips meets the curve over `[x_l, x_r]` (QArc face crossings);
* off `e`, the curve's points in `B` lie on branches over the slabs `(x_l, e)` and `(e, x_r)`
  (between events, by (L1) `branch_order` and the slab samples).

Then the curve in `B` is a star from `p`, with one arm per branch:
* **`strip_const`**: a branch never meets a separator, so it stays in the strip where it starts;
* **`arm_tendsto_left`**, **`arm_tendsto_right`**: a branch in `B` tends to `p` at the event,
  because `C ∩ B` is compact and `p` is its only point over `e`. Continuity of the branch is
  not used;
* **`star`**: `C ∩ B` is `p` with the arms, and `p` is in the closure of every arm.

The space is any topological space with a continuous "x" coordinate `π`.
-/

namespace ForgeCurved.Star

open Set Filter Topology

/-- **A branch stays in its strip.** -/
theorem strip_const {f : ℝ → ℝ} {u v σ : ℝ} (huv : u ≤ v) (hf : ContinuousOn f (Icc u v))
    (hσ : ∀ t ∈ Icc u v, f t ≠ σ) : (f u < σ ↔ f v < σ) := by
  constructor
  · intro hu
    by_contra hv
    obtain ⟨t, ht, hft⟩ := intermediate_value_Icc huv hf ⟨hu.le, not_lt.mp hv⟩
    exact hσ t ht hft
  · intro hv
    by_contra hu
    obtain ⟨t, ht, hft⟩ := intermediate_value_Icc' huv hf ⟨hv.le, not_lt.mp hu⟩
    exact hσ t ht hft

variable {X : Type} [TopologicalSpace X] {π : X → ℝ}

/-- A cluster point at `e` of a path with `π ∘ γ = id` lies over `e`. -/
theorem cluster_over (hπ : Continuous π) {γ : ℝ → X} {l : Filter ℝ} {e : ℝ}
    (hl : l ≤ 𝓝 e) (hγπ : ∀ᶠ t in l, π (γ t) = t) {q : X} (hq : MapClusterPt q l γ) :
    π q = e := by
  by_contra hne
  set d := |π q - e|
  have hd : 0 < d := abs_pos.mpr (sub_ne_zero.mpr hne)
  have hN : π ⁻¹' Metric.ball (π q) (d / 2) ∈ 𝓝 q :=
    hπ.continuousAt.preimage_mem_nhds (Metric.ball_mem_nhds _ (by linarith))
  have hfr := mapClusterPt_iff_frequently.mp hq _ hN
  have he : ∀ᶠ t in l, |t - e| < d / 2 :=
    hl (Filter.mem_of_superset (Metric.ball_mem_nhds e (ε := d / 2) (by linarith)) fun t ht => by
      simpa [Real.dist_eq] using ht)
  obtain ⟨t, ⟨h1, h2⟩, h3⟩ := ((hfr.and_eventually hγπ).and_eventually he).exists
  simp only [mem_preimage, Metric.mem_ball, Real.dist_eq, h2] at h1
  have : |π q - e| ≤ |t - π q| + |t - e| := by
    calc |π q - e| = |(t - e) - (t - π q)| := by ring_nf
      _ ≤ |t - e| + |t - π q| := abs_sub _ _
      _ = |t - π q| + |t - e| := add_comm _ _
  linarith

/-- **An arm tends to the event's point**, along any filter at `e` where it stays in `s`. -/
theorem arm_tendsto (hπ : Continuous π) {s : Set X} (hs : IsCompact s) {e : ℝ} {p : X}
    (hfib : ∀ q ∈ s, π q = e → q = p) {γ : ℝ → X} {l : Filter ℝ} (hl : l ≤ 𝓝 e)
    (hγs : ∀ᶠ t in l, γ t ∈ s) (hγπ : ∀ᶠ t in l, π (γ t) = t) : Tendsto γ l (𝓝 p) :=
  hs.tendsto_nhds_of_unique_mapClusterPt hγs fun q hq hcl =>
    hfib q hq (cluster_over hπ hl hγπ hcl)

/-- **The left arm**: a branch over `(x_l, e)` in `s` tends to `p` as `x → e⁻`. -/
theorem arm_tendsto_left (hπ : Continuous π) {s : Set X} (hs : IsCompact s) {e xl : ℝ} {p : X}
    (hfib : ∀ q ∈ s, π q = e → q = p) {γ : ℝ → X} (hxl : xl < e)
    (hγs : ∀ t ∈ Ioo xl e, γ t ∈ s) (hγπ : ∀ t ∈ Ioo xl e, π (γ t) = t) :
    Tendsto γ (𝓝[<] e) (𝓝 p) := by
  have hI : Ioo xl e ∈ 𝓝[<] e := Ioo_mem_nhdsLT hxl
  exact arm_tendsto hπ hs hfib nhdsWithin_le_nhds (Filter.mem_of_superset hI fun t ht => hγs t ht)
    (Filter.mem_of_superset hI fun t ht => hγπ t ht)

/-- **The right arm**: a branch over `(e, x_r)` in `s` tends to `p` as `x → e⁺`. -/
theorem arm_tendsto_right (hπ : Continuous π) {s : Set X} (hs : IsCompact s) {e xr : ℝ} {p : X}
    (hfib : ∀ q ∈ s, π q = e → q = p) {γ : ℝ → X} (hxr : e < xr)
    (hγs : ∀ t ∈ Ioo e xr, γ t ∈ s) (hγπ : ∀ t ∈ Ioo e xr, π (γ t) = t) :
    Tendsto γ (𝓝[>] e) (𝓝 p) := by
  have hI : Ioo e xr ∈ 𝓝[>] e := Ioo_mem_nhdsGT hxr
  exact arm_tendsto hπ hs hfib nhdsWithin_le_nhds (Filter.mem_of_superset hI fun t ht => hγs t ht)
    (Filter.mem_of_superset hI fun t ht => hγπ t ht)

/-- **(L2) The star.** `s = C ∩ B` is compact, `p` is its one point over `e`, and off `e` its
points lie on the arms: branches `γ k` over `(x_l, e)` (`left k`) or `(e, x_r)`. Then `s` is `p`
with the arms' points, and every arm tends to `p` at the event. -/
theorem star [T2Space X] (hπ : Continuous π) {s : Set X} (hs : IsCompact s) {e xl xr : ℝ} {p : X}
    (hxl : xl < e) (hxr : e < xr) (hp : p ∈ s) (hfib : ∀ q ∈ s, π q = e → q = p)
    {ι : Type} (γ : ι → ℝ → X) (left : ι → Bool)
    (harm : ∀ k, ∀ t ∈ (if left k then Ioo xl e else Ioo e xr), γ k t ∈ s ∧ π (γ k t) = t)
    (hcov : ∀ q ∈ s, π q ≠ e → ∃ k, ∃ t ∈ (if left k then Ioo xl e else Ioo e xr), γ k t = q) :
    s = {p} ∪ ⋃ k, γ k '' (if left k then Ioo xl e else Ioo e xr) ∧
      ∀ k, Tendsto (γ k) (if left k then 𝓝[<] e else 𝓝[>] e) (𝓝 p) := by
  refine ⟨Set.Subset.antisymm (fun q hq => ?_) ?_, fun k => ?_⟩
  · by_cases hqe : π q = e
    · exact Or.inl (hfib q hq hqe)
    · obtain ⟨k, t, ht, rfl⟩ := hcov q hq hqe
      exact Or.inr (mem_iUnion.mpr ⟨k, t, ht, rfl⟩)
  · rintro q (rfl | hq)
    · exact hp
    · obtain ⟨k, t, ht, rfl⟩ := mem_iUnion.mp hq
      exact (harm k t ht).1
  · cases hk : left k
    · simp only [Bool.false_eq_true, ↓reduceIte]
      exact arm_tendsto_right hπ hs hfib hxr (fun t ht => (harm k t (by simp [hk, ht])).1)
        (fun t ht => (harm k t (by simp [hk, ht])).2)
    · simp only [↓reduceIte]
      exact arm_tendsto_left hπ hs hfib hxl (fun t ht => (harm k t (by simp [hk, ht])).1)
        (fun t ht => (harm k t (by simp [hk, ht])).2)

end ForgeCurved.Star
