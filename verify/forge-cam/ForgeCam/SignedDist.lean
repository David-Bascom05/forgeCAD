/-
The signed distance, and the one topological fact every clearance obligation (G1, G2, G3) rests
on: a connected body whose every point is clear of a closed set's frontier, and one of whose
points is outside the set, is clear of the whole set.
-/
import Mathlib

open Metric Set

set_option linter.unusedSectionVars false

namespace ForgeCam

variable {V : Type*} [NormedAddCommGroup V] [NormedSpace ℝ V]

/-- The signed distance of `x` to the body `B`: positive outside (the distance to `B`), and
minus the distance to the complement inside. -/
noncomputable def sd (B : Set V) (x : V) : ℝ :=
  open Classical in if x ∈ B then -infDist x Bᶜ else infDist x B

theorem sd_of_not_mem {B : Set V} {x : V} (h : x ∉ B) : sd B x = infDist x B := by
  simp [sd, h]

theorem sd_of_mem {B : Set V} {x : V} (h : x ∈ B) : sd B x = -infDist x Bᶜ := by
  simp [sd, h]

theorem sd_nonpos_of_mem {B : Set V} {x : V} (h : x ∈ B) : sd B x ≤ 0 := by
  rw [sd_of_mem h]; simp [infDist_nonneg]

/-- A point whose signed distance is positive is outside, at that distance. -/
theorem not_mem_of_sd_pos {B : Set V} {x : V} (h : 0 < sd B x) : x ∉ B := fun hx =>
  absurd (sd_nonpos_of_mem hx) (not_le.mpr h)

/-- The translate of a body. -/
def translate (t : V) (B : Set V) : Set V := (fun x => x + t) '' B

theorem mem_translate {t x : V} {B : Set V} : x ∈ translate t B ↔ x - t ∈ B := by
  constructor
  · rintro ⟨y, hy, rfl⟩; simpa using hy
  · intro h; exact ⟨x - t, h, by simp⟩

/-- The signed distance to a translated body is that to the body, of the point moved back. -/
theorem sd_translate (t : V) (B : Set V) (x : V) : sd (translate t B) x = sd B (x - t) := by
  have hi : Isometry (fun y : V => y + t) := isometry_add_right t
  have hc : (translate t B)ᶜ = (fun y => y + t) '' Bᶜ := by
    ext y; simp only [mem_compl_iff, mem_translate]
    constructor
    · intro h; exact ⟨y - t, h, by simp⟩
    · rintro ⟨z, hz, rfl⟩; simpa using hz
  have e : x = (x - t) + t := by simp
  by_cases h : x - t ∈ B
  · rw [sd_of_mem (mem_translate.mpr h), sd_of_mem h, hc, e, infDist_image hi]; simp
  · rw [sd_of_not_mem (fun h' => h (mem_translate.mp h')), sd_of_not_mem h, translate, e,
      infDist_image hi]; simp

/-- From a point outside a closed set to a point inside, the segment meets the frontier, no
farther from the first point than the second. -/
theorem exists_frontier_le {A : Set V} (hA : IsClosed A) {x y : V} (hx : x ∉ A) (hy : y ∈ A) :
    ∃ f ∈ frontier A, dist x f ≤ dist x y := by
  by_contra hno
  push Not at hno
  have hseg : segment ℝ x y ⊆ interior A ∪ Aᶜ := by
    intro z hz
    by_cases hzA : z ∈ A
    · left
      by_contra hzi
      have hzf : z ∈ frontier A := by
        rw [frontier, hA.closure_eq]; exact ⟨hzA, hzi⟩
      have := hno z hzf
      have h2 := dist_add_dist_of_mem_segment hz
      linarith [dist_nonneg (x := z) (y := y)]
    · right; exact hzA
  have hsub := (convex_segment x y).isPreconnected.subset_right_of_subset_union
    isOpen_interior hA.isOpen_compl
    (disjoint_compl_right.mono_left interior_subset) hseg
    ⟨x, left_mem_segment ℝ x y, hx⟩
  exact hsub (right_mem_segment ℝ x y) hy

/-- The clearance theorem. `X` is connected, `A` closed, `F` holds the frontier of `A`. If every
point of `F` has signed distance at least `c > 0` to `X`, and some point of `X` is outside `A`,
then every point of `X` is at least `c` from `A`. -/
theorem clear_of_frontier {X A F : Set V} {c : ℝ} (hX : IsPreconnected X) (hA : IsClosed A)
    (hF : frontier A ⊆ F) (hc : 0 < c) (hcl : ∀ f ∈ F, c ≤ sd X f) (hout : ∃ x ∈ X, x ∉ A) :
    ∀ x ∈ X, ∀ y ∈ A, c ≤ dist x y := by
  -- no point of X is on the frontier
  have hfr : ∀ f ∈ frontier A, c ≤ infDist f X ∧ f ∉ X := by
    intro f hf
    have h1 := hcl f (hF hf)
    have hpos : 0 < sd X f := lt_of_lt_of_le hc h1
    have hn := not_mem_of_sd_pos hpos
    exact ⟨by rwa [sd_of_not_mem hn] at h1, hn⟩
  -- so X lies outside A
  have hXout : X ⊆ Aᶜ := by
    have hsplit : X ⊆ interior A ∪ Aᶜ := by
      intro z hz
      by_cases hzA : z ∈ A
      · left
        by_contra hzi
        have hzf : z ∈ frontier A := by rw [frontier, hA.closure_eq]; exact ⟨hzA, hzi⟩
        exact (hfr z hzf).2 hz
      · right; exact hzA
    obtain ⟨x, hx, hxA⟩ := hout
    exact hX.subset_right_of_subset_union isOpen_interior hA.isOpen_compl
      (disjoint_compl_right.mono_left interior_subset) hsplit ⟨x, hx, hxA⟩
  intro x hx y hy
  obtain ⟨f, hf, hle⟩ := exists_frontier_le hA (hXout hx) hy
  have := (hfr f hf).1
  have h2 : infDist f X ≤ dist f x := infDist_le_dist_of_mem hx
  rw [dist_comm f x] at h2
  linarith

/-- The same, as a distance to the set: `c ≤ infDist x A` (for nonempty `A`). -/
theorem clear_of_frontier' {X A F : Set V} {c : ℝ} (hX : IsPreconnected X) (hA : IsClosed A)
    (hne : A.Nonempty) (hF : frontier A ⊆ F) (hc : 0 < c) (hcl : ∀ f ∈ F, c ≤ sd X f)
    (hout : ∃ x ∈ X, x ∉ A) : ∀ x ∈ X, c ≤ infDist x A := fun x hx =>
  (le_infDist hne).mpr fun _ hy => clear_of_frontier hX hA hF hc hcl hout x hx _ hy

end ForgeCam
