import ForgeExact.Fillet.PlanePlane

/-!
# Draft of a planar face: the turn about the neutral line, with no trigonometry

research/ops-spec/design.md §4. The kernel (draft.rs:170–189) drafts a plane with outward normal
`n` against the pull `d` (the neutral plane's `z`) to the plane through the **neutral line**
`L = face ∩ neutral`, with unit normal `ĥ cos a + d̂ sin a`, where `h` is `n` with its pull component
removed. The drafted face makes the angle `a` with the pull, whatever its old inclination.

Here, with no square root and no unit vectors: `h := (d·d) n − (n·d) d`, the angle is the pair
`(c, s)`, and the drafted normal is any `n' = α h + β d` with `α > 0`, `β s ≥ 0` and
`c² β² (d·d) = s² α² (h·h)` — the kernel's `ĥ c + d̂ s` up to a positive scale.

* D1 `on_neutral_line`: the drafted plane holds the neutral line.
* D2 `drafted_angle`: its inclination to the pull is the angle's:
  `(n'·d)² (c² + s²) = s² |n'|² |d|²`, i.e. `sin² φ = s² / (c² + s²)`.
* D3 `h_sq`: `h·h = (d·d) |n × d|²`. So `h = 0` exactly when `n ∥ d`, the face parallel to the
  neutral plane: the kernel's `AlongPull` (draft.rs:178) and `NoNeutralLine` (draft.rs:184, for
  planes) are **one condition**.
-/

namespace ForgeExact.Ops

open ForgeExact Fillet Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

namespace Draft

/-- The normal with its pull component removed (scaled by `d·d`). -/
def h (n d : V K) : V K := V.sub (V.smul (V.dot d d) n) (V.smul (V.dot n d) d)

theorem dot_comm' (p q : V K) : V.dot p q = V.dot q p := by simp only [V.dot_def]; grind

theorem h_perp (n d : V K) : V.dot (h n d) d = 0 := by
  simp only [h, V.dot_def, V.sub_x, V.sub_y, V.sub_z, V.smul_x, V.smul_y, V.smul_z]; grind

/-- **D3.** `h·h = (d·d) |n × d|²`. -/
theorem h_sq (n d : V K) : V.dot (h n d) (h n d) = V.dot d d * V.dot (V.cross n d) (V.cross n d) := by
  have lag := PlanePair.lagrange n d
  simp only [h, V.dot_def, V.sub_x, V.sub_y, V.sub_z, V.smul_x, V.smul_y, V.smul_z,
    V.cross_x, V.cross_y, V.cross_z] at *
  grind

/-- So with a nonzero pull, `h` vanishes exactly when the face's normal is parallel to the pull. -/
theorem h_zero_iff {n d : V K} (hd : 0 < V.dot d d) :
    V.dot (h n d) (h n d) = 0 ↔ V.dot (V.cross n d) (V.cross n d) = 0 := by
  rw [h_sq]
  constructor
  · intro h0
    rcases le_or_lt' (V.dot (V.cross n d) (V.cross n d)) 0 with h1 | h1
    · have := PlanePair.dot_self_nonneg (K := K) (V.cross n d); grind
    · have := mul_pos' hd h1; grind
  · intro h0; rw [h0]; grind

/-- A drafted normal for the angle pair `(c, s)`. -/
structure Drafted (n d : V K) (c s : K) where
  α : K
  β : K
  α_pos : 0 < α
  sign : 0 ≤ β * s
  angle : c * c * (β * β) * V.dot d d = s * s * (α * α) * V.dot (h n d) (h n d)

namespace Drafted

variable {n d : V K} {c s : K} (D : Drafted n d c s)

/-- The drafted normal. -/
def normal : V K := V.add (V.smul D.α (h n d)) (V.smul D.β d)

/-- **D1.** The drafted plane through a point `p₀` of the neutral line holds the whole line: every
`x` on the face's old plane and on the neutral plane. -/
theorem on_neutral_line (p₀ x : V K) (hn : V.dot n x = V.dot n p₀) (hd : V.dot d x = V.dot d p₀) :
    V.dot D.normal x = V.dot D.normal p₀ := by
  simp only [normal, h, V.dot_def, V.add_x, V.add_y, V.add_z, V.sub_x, V.sub_y, V.sub_z,
    V.smul_x, V.smul_y, V.smul_z] at *
  grind

/-- **D2.** The drafted face's inclination to the pull is the angle's: with `φ` the angle between
the face and the pull, `sin² φ = (n'·d̂)² / |n'|² = s² / (c² + s²)`. -/
theorem drafted_angle :
    V.dot D.normal d * V.dot D.normal d * (c * c + s * s)
      = s * s * (V.dot D.normal D.normal * V.dot d d) := by
  have hp := h_perp n d
  have hp' : V.dot d (h n d) = 0 := by rw [dot_comm']; exact hp
  have e1 : V.dot D.normal d = D.β * V.dot d d := by
    simp only [normal, V.dot_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z] at *
    grind
  have e2 : V.dot D.normal D.normal
      = D.α * D.α * V.dot (h n d) (h n d) + D.β * D.β * V.dot d d := by
    simp only [normal, V.dot_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z] at *
    grind
  rw [e1, e2]
  have := D.angle
  grind

end Drafted

/-- The kernel's refusal `BadAngle` (draft.rs:51) is `c ≤ 0 ∨ s = 0` on the pair: a draft of 90°
or more, which turns the face through vertical, or of nothing. Under it the drafted normal is
well defined: for `c > 0` and `h ≠ 0`, `β = 0` forces `s = 0`. -/
theorem beta_ne_zero {n d : V K} {c s : K} (D : Drafted n d c s) (hs : s ≠ 0)
    (hh : 0 < V.dot (h n d) (h n d)) : D.β ≠ 0 := by
  intro hb
  have := D.angle
  rw [hb] at this
  have h1 : 0 < s * s := by
    rcases le_or_lt' s 0 with h | h
    · have : s < 0 := by
        rcases le_or_lt' 0 s with h' | h'
        · exact absurd (by grind : s = 0) hs
        · exact h'
      have := mul_pos' (a := -s) (b := -s) (by grind) (by grind); grind
    · exact mul_pos' h h
  have h2 := mul_pos' (mul_pos' h1 (mul_pos' D.α_pos D.α_pos)) hh
  grind

end Draft

end ForgeExact.Ops
