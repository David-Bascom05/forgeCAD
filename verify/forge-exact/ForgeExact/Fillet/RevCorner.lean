import ForgeExact.Fillet.Revolve

/-!
# Plane square to a cylinder: the blend is a torus (or the Pivot sphere)

The concave edge where a plane square to a cylinder's axis meets it, in its two kinds, the cylinder
of radius `ρ₀ > 0` about the z axis and the plane `z = 0`:

* **boss** on a plate: the material is `z ≤ 0` or `ρ ≤ ρ₀`. `boss_closing`: the closing is the
  material, or the points with `ρ < ρ₀ + r`, `z < r` outside the torus of major `ρ₀ + r` and minor
  `r` at height `r`. The blend is that torus.
* **hole** floor: the material is `z ≤ 0` or `ρ ≥ ρ₀`. For `r ≤ ρ₀`, `hole_closing`: the closing is
  the material, or the points with `ρ > ρ₀ − r`, `z < r` outside the torus of major `R = ρ₀ − r`
  and minor `r`. The formula covers every `R ≥ 0` at once:
  * `R > r`: a ring torus;
  * `0 < R ≤ r`: a **spindle** torus, of which only `ρ ≥ R` is used, so the blend never reaches the
    self-intersection. It is still the torus, not a procedural pipe;
  * `R = 0` (`r = ρ₀`, the kernel's `Pivot`): the torus is the **sphere** of radius `r` about
    `(0, 0, r)`.
* `hole_too_large`: for `r > ρ₀` no ball fits, and every point is in the closing. The kernel's
  `TooLarge { max = ρ₀ }` is exactly this bound.

Each is the plane–plane theorem (`PlanePair.closing_iff`) in the meridian half-plane, revolved by
`closing_rev`.
-/

namespace ForgeExact.Fillet

open Lean.Grind Sqrt

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K] [Sqrt K]

/-- The meridian of a boss on a plate. -/
def bossT (ρ₀ : K) : K → K → Prop := fun u z => z ≤ 0 ∨ u ≤ ρ₀

/-- The meridian of a hole's floor and wall. -/
def holeT (ρ₀ : K) : K → K → Prop := fun u z => z ≤ 0 ∨ ρ₀ ≤ u

/-- The disc about `(a, b)` misses the boss's meridian iff `b ≥ r` and `a ≥ ρ₀ + r`. -/
theorem free2_boss {ρ₀ r a b : K} (h0 : 0 ≤ ρ₀) (hr : 0 < r) (ha : 0 ≤ a) :
    Free2 (bossT ρ₀) r a b ↔ r ≤ b ∧ r ≤ a - ρ₀ := by
  have rr := mul_pos' hr hr
  constructor
  · intro h
    refine ⟨?_, ?_⟩
    · rcases le_or_lt' r b with h1 | h1
      · exact h1
      · exfalso
        rcases le_or_lt' b 0 with h2 | h2
        · have := h a b ha (Or.inl h2); grind
        · have := h a 0 ha (Or.inl (by grind))
          have := sq_lt_of_abs_lt (a := b) (r := r) (by grind) h1
          grind
    · rcases le_or_lt' r (a - ρ₀) with h1 | h1
      · exact h1
      · exfalso
        rcases le_or_lt' a ρ₀ with h2 | h2
        · have := h a b ha (Or.inr h2); grind
        · have := h ρ₀ b h0 (Or.inr (by grind))
          have := sq_lt_of_abs_lt (a := a - ρ₀) (r := r) (by grind) h1
          grind
  · rintro ⟨h1, h2⟩ u z _ hT
    rcases hT with hz | hu
    · have := mul_self_le_mul_self' (le_of_lt'' hr) (show r ≤ b - z by grind)
      have := sq_nonneg' (u - a); grind
    · have := mul_self_le_mul_self' (le_of_lt'' hr) (show r ≤ a - u by grind)
      have := sq_nonneg' (z - b); grind
where le_of_lt'' {a b : K} (h : a < b) : a ≤ b := by grind

/-- The disc about `(a, b)`, `a ≥ 0`, misses the hole's meridian iff `b ≥ r` and `a ≤ ρ₀ − r`. -/
theorem free2_hole {ρ₀ r a b : K} (hr : 0 < r) (ha : 0 ≤ a) :
    Free2 (holeT ρ₀) r a b ↔ r ≤ b ∧ r ≤ ρ₀ - a := by
  have rr := mul_pos' hr hr
  constructor
  · intro h
    refine ⟨?_, ?_⟩
    · rcases le_or_lt' r b with h1 | h1
      · exact h1
      · exfalso
        rcases le_or_lt' b 0 with h2 | h2
        · have := h a b ha (Or.inl h2); grind
        · have := h a 0 ha (Or.inl (by grind))
          have := sq_lt_of_abs_lt (a := b) (r := r) (by grind) h1
          grind
    · rcases le_or_lt' r (ρ₀ - a) with h1 | h1
      · exact h1
      · exfalso
        rcases le_or_lt' ρ₀ a with h2 | h2
        · have := h a b ha (Or.inr h2); grind
        · have := h ρ₀ b (by grind) (Or.inr (by grind))
          have := sq_lt_of_abs_lt (a := ρ₀ - a) (r := r) (by grind) h1
          grind
  · rintro ⟨h1, h2⟩ u z _ hT
    rcases hT with hz | hu
    · have := mul_self_le_mul_self' (le_of_lt'' hr) (show r ≤ b - z by grind)
      have := sq_nonneg' (u - a); grind
    · have := mul_self_le_mul_self' (le_of_lt'' hr) (show r ≤ u - a by grind)
      have := sq_nonneg' (z - b); grind
where le_of_lt'' {a b : K} (h : a < b) : a ≤ b := by grind

/-- The boss's corner in the meridian plane: planes `z = 0` and `x = ρ₀`. -/
def bossPP (ρ₀ : K) : PlanePair K where
  n₁ := ⟨0, 0, 1⟩
  n₂ := ⟨1, 0, 0⟩
  d₁ := 0
  d₂ := ρ₀
  unit₁ := by simp only [V.dot_def]; grind
  unit₂ := by simp only [V.dot_def]; grind
  skew := by simp only [V.dot_def]; have : (0 : K) < 1 := OrderedRing.zero_lt_one; grind

/-- The hole's corner in the meridian plane: planes `z = 0` and `x = ρ₀`, the material beyond. -/
def holePP (ρ₀ : K) : PlanePair K where
  n₁ := ⟨0, 0, 1⟩
  n₂ := ⟨-1, 0, 0⟩
  d₁ := 0
  d₂ := -ρ₀
  unit₁ := by simp only [V.dot_def]; grind
  unit₂ := by simp only [V.dot_def]; grind
  skew := by simp only [V.dot_def]; have : (0 : K) < 1 := OrderedRing.zero_lt_one; grind

theorem d2_meridian (a b : K) (c : V K) :
    (a - c.x) * (a - c.x) + (b - c.z) * (b - c.z) ≤ V.d2 (⟨a, 0, b⟩ : V K) c := by
  have := sq_nonneg' c.y; simp only [V.d2_def]; grind

/-- **The fillet of a boss on a plate is the torus of major `ρ₀ + r`, minor `r`.** -/
theorem boss_closing {ρ₀ r : K} (h0 : 0 ≤ ρ₀) (hr : 0 < r) (x : V K) :
    closing (Rev (bossT ρ₀)) r x ↔
      bossT ρ₀ (rho x) x.z ∨
        (rho x - ρ₀ < r ∧ x.z < r ∧
          r * r ≤ (rho x - (ρ₀ + r)) * (rho x - (ρ₀ + r)) + (x.z - r) * (x.z - r)) := by
  let P := bossPP ρ₀
  rw [closing_rev]
  have na := rho_nonneg x
  -- the half-plane closing is the corner's closing at the meridian point
  have red : closing2 (bossT ρ₀) r (rho x) x.z ↔ closing P.S r ⟨rho x, 0, x.z⟩ := by
    constructor
    · intro h ctr hf
      obtain ⟨f₁, f₂⟩ := (P.free_iff hr ctr).mp hf
      simp only [P, bossPP, PlanePair.a₁, PlanePair.a₂, V.dot_def] at f₁ f₂
      have := h ctr.x ctr.z (by grind) ((free2_boss h0 hr (by grind)).mpr ⟨by grind, by grind⟩)
      have := d2_meridian (rho x) x.z ctr
      grind
    · intro h a' b' ha hf
      obtain ⟨f₁, f₂⟩ := (free2_boss h0 hr ha).mp hf
      have := h ⟨a', 0, b'⟩ ((P.free_iff hr _).mpr ⟨by
        simp only [P, bossPP, PlanePair.a₁, V.dot_def]; grind, by
        simp only [P, bossPP, PlanePair.a₂, V.dot_def]; grind⟩)
      simp only [V.d2_def] at this; grind
  rw [red, P.closing_iff hr]
  simp only [P, bossPP, PlanePair.S, PlanePair.Cone, PlanePair.q, PlanePair.c, PlanePair.a₁,
    PlanePair.a₂, V.dot_def, bossT]
  constructor
  · rintro (h | ⟨⟨h1, h2⟩, h3⟩)
    · left; grind
    · right; refine ⟨by grind, by grind, by grind⟩
  · rintro (h | ⟨h1, h2, h3⟩)
    · left; grind
    · right; refine ⟨⟨by grind, by grind⟩, by grind⟩

/-- **The fillet of a hole's floor is the torus of major `ρ₀ − r`, minor `r`** (for `r ≤ ρ₀`):
a ring torus, a spindle torus used only outside its major radius, or at `r = ρ₀` the sphere. -/
theorem hole_closing {ρ₀ r : K} (hr : 0 < r) (hle : r ≤ ρ₀) (x : V K) :
    closing (Rev (holeT ρ₀)) r x ↔
      holeT ρ₀ (rho x) x.z ∨
        (ρ₀ - r < rho x ∧ x.z < r ∧
          r * r ≤ (rho x - (ρ₀ - r)) * (rho x - (ρ₀ - r)) + (x.z - r) * (x.z - r)) := by
  let P := holePP ρ₀
  rw [closing_rev]
  have na := rho_nonneg x
  have red : closing2 (holeT ρ₀) r (rho x) x.z ↔ closing P.S r ⟨rho x, 0, x.z⟩ := by
    constructor
    · intro h ctr hf
      obtain ⟨f₁, f₂⟩ := (P.free_iff hr ctr).mp hf
      simp only [P, holePP, PlanePair.a₁, PlanePair.a₂, V.dot_def] at f₁ f₂
      have := d2_meridian (rho x) x.z ctr
      -- a centre across the axis: its foot on the axis is free and nearer
      rcases le_or_lt' 0 ctr.x with hx | hx
      · have := h ctr.x ctr.z hx ((free2_hole hr hx).mpr ⟨by grind, by grind⟩)
        grind
      · have := h 0 ctr.z (le_refl' 0) ((free2_hole hr (le_refl' 0)).mpr ⟨by grind, by grind⟩)
        have := mul_nonneg' na (show 0 ≤ -ctr.x by grind)
        have := sq_nonneg' ctr.x
        grind
    · intro h a' b' ha hf
      obtain ⟨f₁, f₂⟩ := (free2_hole hr ha).mp hf
      have := h ⟨a', 0, b'⟩ ((P.free_iff hr _).mpr ⟨by
        simp only [P, holePP, PlanePair.a₁, V.dot_def]; grind, by
        simp only [P, holePP, PlanePair.a₂, V.dot_def]; grind⟩)
      simp only [V.d2_def] at this; grind
  rw [red, P.closing_iff hr]
  simp only [P, holePP, PlanePair.S, PlanePair.Cone, PlanePair.q, PlanePair.c, PlanePair.a₁,
    PlanePair.a₂, V.dot_def, holeT]
  constructor
  · rintro (h | ⟨⟨h1, h2⟩, h3⟩)
    · left; grind
    · right; refine ⟨by grind, by grind, by grind⟩
  · rintro (h | ⟨h1, h2, h3⟩)
    · left; grind
    · right; refine ⟨⟨by grind, by grind⟩, by grind⟩
where le_refl' (a : K) : a ≤ a := by grind

/-- **The Pivot.** At `r = ρ₀` the hole's blend is the sphere of radius `r` about `(0, 0, r)`. -/
theorem pivot_sphere {r : K} (hr : 0 < r) (x : V K) :
    closing (Rev (holeT r)) r x ↔
      holeT r (rho x) x.z ∨
        (0 < rho x ∧ x.z < r ∧ r * r ≤ V.d2 x ⟨0, 0, r⟩) := by
  rw [hole_closing hr (by grind) x]
  have := rho_sq x
  simp only [V.d2_def]
  constructor
  · rintro (h | ⟨h1, h2, h3⟩)
    · exact Or.inl h
    · right; exact ⟨by grind, h2, by grind⟩
  · rintro (h | ⟨h1, h2, h3⟩)
    · exact Or.inl h
    · right; exact ⟨by grind, h2, by grind⟩

/-- **Too large.** For `r > ρ₀` no ball fits in the hole, and the closing is everything. -/
theorem hole_too_large {ρ₀ r : K} (hr : 0 < r) (hgt : ρ₀ < r) (x : V K) :
    closing (Rev (holeT ρ₀)) r x := by
  intro c hc
  rw [free_rev, free2_hole hr (rho_nonneg c)] at hc
  have := rho_nonneg c
  grind

end ForgeExact.Fillet
