import ForgeExact.Ops.Hole

/-!
# Draft of a cylinder: the cone through the neutral circle

research/ops-spec/design.md §4. The kernel (draft.rs:161–168) drafts a cylinder of radius `ρ`
whose axis is the pull into the cone through its circle on the neutral plane, of half-angle `a`.
In the cylinder's frame (`z` along the pull, `0` on the neutral plane) and with the angle as the
pair `(c, s)`, `c > 0`, the cone's radius at height `z` is `(c ρ − s z) / c`:

* `cone_eq_frustum`: over a face's height range `[z₀, z₁]` the drafted solid is the hole spec's
  `Frustum` with those end radii: `c² ρ²(x) ≤ (c ρ − s z)²` there. One solid-of-revolution form
  serves holes, countersinks, drill points and drafted bosses.
* `neutral_circle`: at `z = 0` it is the cylinder's own circle (the radius is `ρ`).
* `apex_free_iff`: the cone's radius is positive over the whole face iff it is at the face's two
  extreme heights. So the kernel's `ThroughApex` (draft.rs:98, a radius `≤ MIN_LENGTH` at a
  vertex) is exact when decided at the face's extreme heights, not only at its vertices.
-/

namespace ForgeExact.Ops

open ForgeExact Fillet Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

namespace DraftCone

/-- The drafted cone over `[z₀, z₁]`, as a frustum. -/
def frustum (ρ c s z₀ z₁ : K) : Frustum K :=
  ⟨(c * ρ - s * z₀) / c, (c * ρ - s * z₁) / c, z₀, z₁⟩

/-- **The drafted cylinder is a frustum.** -/
theorem cone_eq_frustum {ρ c s z₀ z₁ : K} (hc : 0 < c) (hz : z₀ < z₁) (x : V K) :
    (frustum ρ c s z₀ z₁).mem x ↔
      z₀ ≤ x.z ∧ x.z ≤ z₁ ∧ c * c * rho2 x ≤ (c * ρ - s * x.z) * (c * ρ - s * x.z) := by
  have hc0 : c ≠ 0 := by intro h; rw [h] at hc; exact absurd hc (by grind)
  simp only [Frustum.mem, frustum]
  -- the linear interpolation of the end radii is (z₁ − z₀)(c ρ − s z) / c
  have e : (c * ρ - s * z₀) / c * (z₁ - x.z) + (c * ρ - s * z₁) / c * (x.z - z₀)
      = (z₁ - z₀) * (c * ρ - s * x.z) / c := by
    rw [Field.div_eq_mul_inv, Field.div_eq_mul_inv, Field.div_eq_mul_inv]; grind
  rw [e]
  have hd : 0 < (z₁ - z₀) * (z₁ - z₀) := mul_pos' (by grind) (by grind)
  have hcc : 0 < c * c := mul_pos' hc hc
  -- both sides scale: multiply the frustum's inequality by c², the cone's by (z₁ − z₀)²
  have key : ∀ A B : K, ((z₁ - z₀) * (z₁ - z₀) * A ≤ (z₁ - z₀) * B / c * ((z₁ - z₀) * B / c)
      ↔ c * c * A ≤ B * B) := by
    intro A B
    have e2 : (z₁ - z₀) * B / c * ((z₁ - z₀) * B / c) * (c * c)
        = (z₁ - z₀) * (z₁ - z₀) * (B * B) := by
      rw [Field.div_eq_mul_inv]
      have : c * c⁻¹ = 1 := Field.mul_inv_cancel hc0
      grind
    constructor
    · intro h
      have h1 := mul_le_mul_of_nonneg_left' h (le_of_lt hcc)
      have h2 : (z₁ - z₀) * (z₁ - z₀) * (c * c * A) ≤ (z₁ - z₀) * (z₁ - z₀) * (B * B) := by
        grind
      exact le_of_mul_le_mul_left' h2 hd
    · intro h
      have h1 := mul_le_mul_of_nonneg_left' h (le_of_lt hd)
      have h2 : c * c * ((z₁ - z₀) * (z₁ - z₀) * A)
          ≤ c * c * ((z₁ - z₀) * B / c * ((z₁ - z₀) * B / c)) := by grind
      exact le_of_mul_le_mul_left' h2 hcc
  rw [key]
where le_of_lt {a b : K} (h : a < b) : a ≤ b := by grind

/-- At the neutral plane the cone is the cylinder's circle. -/
theorem neutral_circle {ρ c s : K} (hc : 0 < c) : (c * ρ - s * 0) / c = ρ := by
  have hc0 : c ≠ 0 := by intro h; rw [h] at hc; exact absurd hc (by grind)
  rw [Field.div_eq_mul_inv]
  have : c * c⁻¹ = 1 := Field.mul_inv_cancel hc0
  grind

/-- **No apex on the face** iff none at its two extreme heights: the radius is linear in `z`. -/
theorem apex_free_iff {ρ c s z₀ z₁ : K} (hz : z₀ ≤ z₁) :
    (∀ z, z₀ ≤ z → z ≤ z₁ → 0 < c * ρ - s * z) ↔
      (0 < c * ρ - s * z₀ ∧ 0 < c * ρ - s * z₁) := by
  constructor
  · intro h; exact ⟨h z₀ (by grind) hz, h z₁ hz (by grind)⟩
  · rintro ⟨h₀, h₁⟩ z hz₀ hz₁
    -- s z lies between s z₀ and s z₁
    rcases le_or_lt' 0 s with hs | hs
    · have := mul_le_mul_of_nonneg_left' hz₁ hs; grind
    · have := mul_le_mul_of_nonneg_left' hz₀ (show 0 ≤ -s by grind); grind

end DraftCone

end ForgeExact.Ops
