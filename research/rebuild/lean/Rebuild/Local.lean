/-
Why touching matters: a boolean's boundary is local. Two parts that agree near the tool's boundary
get the same result near it, and what moved elsewhere comes through unchanged.
-/
import Mathlib.Topology.Neighborhoods
import Mathlib.Topology.ClusterPt
import Mathlib.Order.Filter.Basic
import Mathlib.Tactic.Tauto
import Rebuild.Sets

namespace Rebuild
variable {α : Type*} [TopologicalSpace α]
open Set Topology

/-- Two sets agree near a point. -/
def AgreeNear (A B : Set α) (x : α) : Prop := ∀ᶠ y in 𝓝 x, y ∈ A ↔ y ∈ B

/-- The frontier at a point depends only on the set near that point. -/
theorem frontier_local {A B : Set α} {x : α} (h : AgreeNear A B x) :
    x ∈ frontier A ↔ x ∈ frontier B := by
  have hc : (∃ᶠ y in 𝓝 x, y ∈ A) ↔ (∃ᶠ y in 𝓝 x, y ∈ B) := Filter.frequently_congr h
  have hi : (∀ᶠ y in 𝓝 x, y ∈ A) ↔ (∀ᶠ y in 𝓝 x, y ∈ B) := Filter.eventually_congr h
  have hi' : (A ∈ 𝓝 x) ↔ (B ∈ 𝓝 x) := hi
  rw [frontier, frontier, mem_diff, mem_diff, mem_closure_iff_frequently, mem_closure_iff_frequently,
    mem_interior_iff_mem_nhds, mem_interior_iff_mem_nhds, hc, hi']

/-- Parts agreeing near `x`, after the same tool, still agree near `x`. No condition on the tool. -/
theorem cut_near {A B : Set α} (T : Set α) {x : α} (h : AgreeNear A B x) :
    AgreeNear (cut A T) (cut B T) x := by
  filter_upwards [h] with y hy
  simp [cut, hy]
theorem add_near {A B : Set α} (T : Set α) {x : α} (h : AgreeNear A B x) :
    AgreeNear (add A T) (add B T) x := by
  filter_upwards [h] with y hy
  simp [add, hy]
theorem keep_near {A B : Set α} (T : Set α) {x : α} (h : AgreeNear A B x) :
    AgreeNear (keep A T) (keep B T) x := by
  filter_upwards [h] with y hy
  simp [keep, hy]

/-- A point away from the tool's closure: cutting or adding leaves the part as it was near it. -/
theorem away_of_closure {T : Set α} {x : α} (h : x ∉ closure T) : ∀ᶠ y in 𝓝 x, y ∉ T := by
  rw [mem_closure_iff_frequently] at h
  simpa using h
theorem cut_away (B : Set α) {T : Set α} {x : α} (h : x ∉ closure T) : AgreeNear (cut B T) B x := by
  filter_upwards [away_of_closure h] with y hy
  simp [cut, hy]
theorem add_away (B : Set α) {T : Set α} {x : α} (h : x ∉ closure T) : AgreeNear (add B T) B x := by
  filter_upwards [away_of_closure h] with y hy
  simp [add, hy]

/-- Inside the tool, or anywhere it covers a neighbourhood, a cut has no part left and an add has
all of it, whatever the part was: a change entirely under a tool is masked. -/
theorem cut_masked {A B T : Set α} {x : α} (h : ∀ᶠ y in 𝓝 x, y ∈ T) : AgreeNear (cut A T) (cut B T) x := by
  filter_upwards [h] with y hy
  simp [cut, hy]
theorem add_masked {A B T : Set α} {x : α} (h : ∀ᶠ y in 𝓝 x, y ∈ T) : AgreeNear (add A T) (add B T) x := by
  filter_upwards [h] with y hy
  simp [add, hy]

/-- Inside the tool, a cut's result is empty and an add's is full near that point: no frontier. -/
theorem masked_no_frontier {A T : Set α} {x : α} (h : ∀ᶠ y in 𝓝 x, y ∈ T) :
    x ∉ frontier (cut A T) ∧ x ∉ frontier (add A T) := by
  constructor
  · intro hx
    have hx' := hx.1
    rw [mem_closure_iff_frequently] at hx'
    exact (hx'.and_eventually h).exists.elim fun y hy => hy.1.2 hy.2
  · intro hx
    apply hx.2
    rw [mem_interior_iff_mem_nhds]
    filter_upwards [h] with y hy
    exact Or.inr hy

/-- The theorem a skip rests on, for a boolean with an unmoved tool: if two parts agree near every
point of the tool's frontier, the results' frontiers are the same set wherever the tool's frontier
is, and away from it each result's frontier is its own part's or none. So all a feature adds to the
boundary (its cut curves, its tool faces) lies where the parts agree. Only near the tool's FRONTIER
do they need to agree: a change wholly inside the tool or wholly away from it needs nothing. -/
theorem skip_cut {A B T : Set α} (h : ∀ x ∈ frontier T, AgreeNear A B x) :
    ∀ x ∈ frontier T, (x ∈ frontier (cut A T) ↔ x ∈ frontier (cut B T)) := fun x hx =>
  frontier_local (cut_near T (h x hx))
theorem skip_add {A B T : Set α} (h : ∀ x ∈ frontier T, AgreeNear A B x) :
    ∀ x ∈ frontier T, (x ∈ frontier (add A T) ↔ x ∈ frontier (add B T)) := fun x hx =>
  frontier_local (add_near T (h x hx))

theorem skip_keep {A B T : Set α} (h : ∀ x ∈ frontier T, AgreeNear A B x) :
    ∀ x ∈ frontier T, (x ∈ frontier (keep A T) ↔ x ∈ frontier (keep B T)) := fun x hx =>
  frontier_local (keep_near T (h x hx))

/-- Away from the tool's frontier there are only two cases: under the tool (masked, no frontier) or
clear of its closure (the part's own frontier). Needs no agreement of the parts at all. -/
theorem cut_off_tool_frontier (A : Set α) {T : Set α} {x : α} (h : x ∉ frontier T) :
    (x ∈ frontier (cut A T) ↔ x ∈ frontier A) ∧ x ∉ closure T ∨ x ∉ frontier (cut A T) ∧ x ∈ interior T := by
  by_cases hc : x ∈ closure T
  · have hi : x ∈ interior T := by
      by_contra hn
      exact h ⟨hc, hn⟩
    right
    exact ⟨(masked_no_frontier (mem_interior_iff_mem_nhds.mp hi)).1, hi⟩
  · left
    exact ⟨frontier_local (cut_away A hc), hc⟩

/-- What the skip test checks gives what `skip_cut` asks. The test: every changed thing lies clear of
the tool's frontier. So the change's closure misses the tool's frontier, and the parts agree near it.
-/
theorem agree_of_clear {A B T : Set α}
    (h : ∀ x ∈ frontier T, x ∉ closure (diff A B)) : ∀ x ∈ frontier T, AgreeNear A B x := by
  intro x hx
  filter_upwards [away_of_closure (h x hx)] with y hy
  by_cases ha : y ∈ A <;> by_cases hb : y ∈ B <;> simp_all [diff]

end Rebuild
