/-
CAM's L1 (research/cam/design.md §5.2): the obligations G1–G4 for one operation, over ℝ³.
G5 is in ForgeCam/Machine.lean, G6 in ForgeCam/Post.lean.

- T: the cutter (the flutes). N: the non-cutting part (neck, shank, holder). Both are sets of
  points in the tool's frame (tip at the origin, axis +z).
- g s: the tool's placement at instant s ∈ [0, 1], a map of space (a translation for 3-axis).
- P: the target part. δ: the stock to leave. ε_in, ε_out: the tolerances.
- S s: the in-process stock at s. F s: the fixture and machine bodies at s.
-/
import ForgeCam.Hypograph

open Metric Set

namespace ForgeCam

/-- The instants of a piece. -/
abbrev I01 : Set ℝ := Icc 0 1

/-- The swept set Sw(X, g) = ⋃_{s ∈ [0,1]} g(s)·X. -/
def swept (X : Set E3) (g : ℝ → E3 → E3) : Set E3 := ⋃ s ∈ I01, g s '' X

/-- **G1, no gouge**, with the inner tolerance: every point the cutter reaches is at least
δ − ε_in from the part. -/
def G1 (T P : Set E3) (g : ℝ → E3 → E3) (δ εin : ℝ) : Prop :=
  ∀ s ∈ I01, ∀ x ∈ g s '' T, ∀ y ∈ P, δ - εin ≤ dist x y

/-- **G2 (corrected, design.md §7b)**: on every piece kind, the whole tool T ∪ N misses the
fixture grown by its clearance c, F(s) ⊕ B_c; and the non-cutting part never meets the stock's
interior. -/
def G2 (T N : Set E3) (S F : ℝ → Set E3) (g : ℝ → E3 → E3) (c : ℝ) : Prop :=
  ∀ s ∈ I01, Disjoint (g s '' (T ∪ N)) (cthickening c (F s)) ∧ Disjoint (g s '' N) (interior (S s))

/-- **G3, no cutting on a non-cutting move** (rapids and links). -/
def G3 (T : Set E3) (S : ℝ → Set E3) (g : ℝ → E3 → E3) : Prop :=
  ∀ s ∈ I01, Disjoint (g s '' T) (interior (S s))

/-- The in-process stock after a piece: S₀ less the interior of what the cutter swept. -/
def stockAfter (S₀ : Set E3) (T : Set E3) (g : ℝ → E3 → E3) : Set E3 :=
  S₀ \ interior (swept T g)

/-- G1 in its swept-set form: the swept cutter misses the open (δ − ε)-thickening of P. -/
theorem G1_iff_swept {T P : Set E3} {g : ℝ → E3 → E3} {δ εin : ℝ} :
    G1 T P g δ εin ↔ ∀ x ∈ swept T g, ∀ y ∈ P, δ - εin ≤ dist x y := by
  simp only [G1, swept, mem_iUnion]
  constructor
  · rintro h x ⟨s, hs, hx⟩ y hy; exact h s hs x hx y hy
  · intro h s hs x hx y hy; exact h x ⟨s, hs, hx⟩ y hy

/-- **G4, the lower half**: what G1 promises about the result. If the open (δ − ε)-thickening of
P was in the stock, it is still there after the piece; so is P. -/
theorem G4_lower {S₀ T P : Set E3} {g : ℝ → E3 → E3} {δ εin : ℝ} (h1 : G1 T P g δ εin)
    (hS : thickening (δ - εin) P ⊆ S₀) : thickening (δ - εin) P ⊆ stockAfter S₀ T g := by
  intro z hz
  refine ⟨hS hz, fun hzi => ?_⟩
  obtain ⟨y, hy, hd⟩ := mem_thickening_iff.mp hz
  have := (G1_iff_swept.mp h1) z (interior_subset hzi) y hy
  linarith

theorem G4_part {S₀ T P : Set E3} {g : ℝ → E3 → E3} {δ εin : ℝ} (h1 : G1 T P g δ εin)
    (hpos : 0 < δ - εin) (hS : P ⊆ S₀) : P ⊆ stockAfter S₀ T g := by
  intro z hz
  refine ⟨hS hz, fun hzi => ?_⟩
  have := (G1_iff_swept.mp h1) z (interior_subset hzi) z hz
  simp at this; linarith

/-- The leftover: what of the result lies beyond P grown by δ + ε_out. -/
noncomputable def leftover (Send P : Set E3) (r : ℝ) : Set E3 := Send \ cthickening r P

/-- **G4, the upper half**: S_end ⊆ (P ⊕ B_r) ∪ L, and the reported L is exact. It covers
every excess point, it holds no point within r of P, and it is the least set that covers. -/
theorem G4_upper (Send P : Set E3) (r : ℝ) :
    Send ⊆ cthickening r P ∪ leftover Send P r ∧
    Disjoint (leftover Send P r) (cthickening r P) ∧
    ∀ L, Send ⊆ cthickening r P ∪ L → leftover Send P r ⊆ L := by
  refine ⟨fun x hx => ?_, disjoint_sdiff_left, fun L hL x hx => ?_⟩
  · by_cases h : x ∈ cthickening r P
    · exact Or.inl h
    · exact Or.inr ⟨hx, h⟩
  · rcases hL hx.1 with h | h
    · exact absurd h hx.2
    · exact h

/-- G4 by z bands (how the 2.5D stock reports it): the leftover's slice at each height is the
result's slice less the grown part's slice, so a per-band exact computation is the exact L. -/
theorem leftover_slice (Send P : Set E3) (r z : ℝ) :
    {p ∈ leftover Send P r | p 2 = z} = {p ∈ Send | p 2 = z} \ cthickening r P := by
  ext p; simp only [leftover, mem_ofPred_eq, mem_sdiff]; tauto

/-! ### Declared removals and the fixture (design.md §7b) -/

/-- The part less its declared removals Δ (an edge break, a cosmetic thread): G1 and G4 are
stated for P' = P ∖ Δ, never with a blanket tolerance. -/
def target (P Δ : Set E3) : Set E3 := P \ Δ

/-- G1 for a smaller part is weaker. So G1 for P implies it for P', and removing Δ relaxes G1
exactly on Δ: every point of P outside Δ keeps its full clearance. -/
theorem G1_mono_part {T P P' : Set E3} {g : ℝ → E3 → E3} {δ εin : ℝ} (h : P' ⊆ P)
    (h1 : G1 T P g δ εin) : G1 T P' g δ εin :=
  fun s hs x hx y hy => h1 s hs x hx y (h hy)

theorem G1_target_iff {T P Δ : Set E3} {g : ℝ → E3 → E3} {δ εin : ℝ} :
    G1 T (target P Δ) g δ εin ↔
      ∀ s ∈ I01, ∀ x ∈ g s '' T, ∀ y ∈ P, y ∉ Δ → δ - εin ≤ dist x y := by
  simp only [G1, target, Set.mem_sdiff]
  constructor
  · intro h s hs x hx y hy hyΔ; exact h s hs x hx y ⟨hy, hyΔ⟩
  · intro h s hs x hx y ⟨hy, hyΔ⟩; exact h s hs x hx y hy hyΔ

/-- The leftover split by its reason: the region K held in the fixture ("held by fixture"),
and the rest ("uncut"). -/
noncomputable def leftoverHeld (Send P : Set E3) (r : ℝ) (K : Set E3) : Set E3 := leftover Send P r ∩ K
noncomputable def leftoverUncut (Send P : Set E3) (r : ℝ) (K : Set E3) : Set E3 := leftover Send P r \ K

/-- **G4 with the clamped region.** The two lists together are the exact leftover, they do not
overlap, and the held list is exactly the excess inside the clamped region. -/
theorem G4_held (Send P : Set E3) (r : ℝ) (K : Set E3) :
    Send ⊆ cthickening r P ∪ (leftoverHeld Send P r K ∪ leftoverUncut Send P r K) ∧
    leftoverHeld Send P r K ∪ leftoverUncut Send P r K = leftover Send P r ∧
    Disjoint (leftoverHeld Send P r K) (leftoverUncut Send P r K) ∧
    leftoverHeld Send P r K = (Send ∩ K) \ cthickening r P := by
  have hu : leftoverHeld Send P r K ∪ leftoverUncut Send P r K = leftover Send P r :=
    Set.inter_union_sdiff _ _
  refine ⟨?_, hu, ?_, ?_⟩
  · rw [hu]; exact (G4_upper Send P r).1
  · exact Set.disjoint_left.mpr fun x h1 h2 => h2.2 h1.2
  · ext x; simp only [leftoverHeld, leftover, Set.mem_inter_iff, Set.mem_sdiff]; tauto

end ForgeCam
