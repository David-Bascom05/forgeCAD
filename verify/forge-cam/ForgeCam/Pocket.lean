/-
Pocket coverage (the owner's rule, design.md §8: prove what holds for every toolpath an op
generates, and drop the runtime check). An offset pocket's passes run the tool's centre along
the offsets of the region's original boundary at r, r + s, r + 2s, …, with the stepover
0 < s ≤ r. Then the tool's disks cover the region's opening R ∘ B_r, the points of R that a
disk of radius r inside R can reach. The in-process stock (forge-cam stock.rs) relies on this
for the void each pocket clears.

The plane is EuclideanSpace ℝ (Fin 2). "Inside R" is measured by depth x = infDist x Rᶜ, the
distance to the outside. The pass k is the level set depth = r + k·s. The op emits every
nonempty level; an empty one contributes nothing.
-/
import Mathlib

open Metric Set

namespace ForgeCam.Pocket

abbrev E2 := EuclideanSpace ℝ (Fin 2)

variable (R : Set E2) (r s : ℝ)

/-- The distance to the outside of R. -/
noncomputable def depth (x : E2) : ℝ := infDist x Rᶜ

/-- The centres the k-th pass runs along: the offset at r + k·s. -/
def pass (k : ℕ) : Set E2 := {x | depth R x = r + k * s}

/-- The opening R ∘ B_r: the points of disks of radius r whose centres are at least r deep. -/
def opening : Set E2 := {q | ∃ p, r ≤ depth R p ∧ dist q p ≤ r}

theorem continuous_depth : Continuous (depth R) := continuous_infDist_pt _

/-- Along the segment from q towards b. -/
theorem dist_lineMap_left (q b : E2) (t : ℝ) (ht : t ∈ Icc (0 : ℝ) 1) :
    dist q (q + t • (b - q)) = t * dist q b := by
  rw [dist_eq_norm, sub_add_cancel_left, norm_neg, norm_smul, Real.norm_of_nonneg ht.1,
    dist_comm, dist_eq_norm]

theorem dist_lineMap_right (q b : E2) (t : ℝ) (ht : t ∈ Icc (0 : ℝ) 1) :
    dist (q + t • (b - q)) b = (1 - t) * dist q b := by
  have : b - (q + t • (b - q)) = (1 - t) • (b - q) := by
    rw [sub_smul, one_smul]; abel
  rw [dist_comm, dist_eq_norm, this, norm_smul, Real.norm_of_nonneg (by linarith [ht.2]),
    dist_comm, dist_eq_norm]

/-- A level is reached on the segment from x to y wherever the depth crosses it. -/
theorem exists_level (x y : E2) (v : ℝ) (h : v ∈ uIcc (depth R x) (depth R (x + (1:ℝ) • (y - x)))) :
    ∃ t ∈ Icc (0 : ℝ) 1, depth R (x + t • (y - x)) = v := by
  have hc : ContinuousOn (fun t : ℝ => depth R (x + t • (y - x))) (Icc 0 1) :=
    ((continuous_depth R).comp (continuous_const.add (continuous_id.smul continuous_const)))
      |>.continuousOn
  have hc' : ContinuousOn (fun t : ℝ => depth R (x + t • (y - x))) (uIcc 0 1) := by
    rwa [uIcc_of_le zero_le_one]
  have := intermediate_value_uIcc hc'
  simp only [zero_smul, add_zero] at this
  obtain ⟨t, ht, hv⟩ := this h
  rw [uIcc_of_le zero_le_one] at ht
  exact ⟨t, ht, hv⟩

/-- **The passes cover the opening.** With R's outside nonempty (a bounded pocket) and
0 < s ≤ r, every point of R ∘ B_r is within r of a point of some pass. -/
theorem passes_cover (hR : (Rᶜ).Nonempty) {r s : ℝ} (hs : 0 < s) (hsr : s ≤ r) :
    ∀ q ∈ opening R r, ∃ k : ℕ, ∃ z ∈ pass R r s k, dist q z ≤ r := by
  intro q ⟨p, hp, hqp⟩
  have hr : 0 < r := lt_of_lt_of_le hs hsr
  by_cases hq : r ≤ depth R q
  · -- q is itself deep: walk towards its nearest outside point to the level just above
    obtain ⟨b, hb, hdb⟩ := exists_mem_closure_infDist_eq_dist hR q
    set d := depth R q with hd
    have hdq : d = dist q b := hdb
    set k := ⌊(d - r) / s⌋₊ with hk
    have hk1 : (k : ℝ) * s ≤ d - r := by
      have := Nat.floor_le (a := (d - r) / s) (div_nonneg (by linarith) hs.le)
      rwa [le_div_iff₀ hs] at this
    have hk2 : d - r < (k + 1) * s := by
      have := Nat.lt_floor_add_one ((d - r) / s)
      rwa [div_lt_iff₀ hs] at this
    have hb0 : depth R b = 0 := by
      unfold depth; exact infDist_zero_of_mem_closure hb
    have hlev : r + k * s ∈ uIcc (depth R q) (depth R (q + (1:ℝ) • (b - q))) := by
      rw [one_smul, add_sub_cancel, hb0]
      exact Set.mem_uIcc.mpr (Or.inr ⟨by positivity, by linarith⟩)
    obtain ⟨t, ht, hz⟩ := exists_level R q b _ hlev
    refine ⟨k, q + t • (b - q), hz, ?_⟩
    rw [dist_lineMap_left q b t ht]
    -- depth z ≤ dist z b = (1 − t) d, so t d ≤ d − (r + k s) < s ≤ r
    have hzb : depth R (q + t • (b - q)) ≤ dist (q + t • (b - q)) b := by
      unfold depth
      rw [← infDist_closure]; exact infDist_le_dist_of_mem hb
    rw [hz, dist_lineMap_right q b t ht, ← hdq] at hzb
    nlinarith
  · -- q is shallow: walk towards the deep centre p to the first level
    push Not at hq
    have hlev : r + (0 : ℕ) * s ∈ uIcc (depth R q) (depth R (q + (1:ℝ) • (p - q))) := by
      rw [one_smul, add_sub_cancel]
      simp only [Nat.cast_zero, zero_mul, add_zero]
      exact Set.mem_uIcc.mpr (Or.inl ⟨hq.le, hp⟩)
    obtain ⟨t, ht, hz⟩ := exists_level R q p _ hlev
    refine ⟨0, q + t • (p - q), hz, ?_⟩
    rw [dist_lineMap_left q p t ht]
    nlinarith [ht.1, ht.2, dist_nonneg (x := q) (y := p)]

end ForgeCam.Pocket
