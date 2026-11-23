import ForgeExact.Fillet.PlanePlane

/-!
# Conic sections: the same construction with a different section

A conic fillet is not a ball closing. The spec keeps everything of the ball's construction except the
section (lean-spec-design.md §2):

* the same reach Γ_E, crease and spine `c(s)`, and the **ball's** springs `p₁(s)`, `p₂(s)`;
* at each `s`, the section is the rational quadratic arc with ends `p₁`, `p₂`, apex `a` (where the two
  faces' tangent planes meet in the plane of `p₁`, `c`, `p₂`), and middle weight `w` (`ρ = w/(1 + w)`,
  `w = ρ/(1 − ρ)`):

  `B(t) = ((1 − t)² p₁ + 2w t(1 − t) a + t² p₂) / ((1 − t)² + 2w t(1 − t) + t²)`;
* the same vertex table: run-out, and mitre as the ⋂ of the conic fills carried on. The all-picked
  row has no conic corner.

In the section's plane, with `u = p₁ − c`, `s = p₂ − c`, `|u| = |s| = r` and `u·s = r² k`
(`k = cos α`, `α` the arc's angle at the centre), the apex is `c + (u + s)/(1 + k)`.

* `conic_circle`: the arc lies on the ball's circle, `|B(t) − c| = r`, at any `t` with
  `t (1 − t) ≠ 0`, **iff** `2w² = 1 + k`. That is `w = cos(α/2)`, written without trig. So a constant
  `ρ` is the ball only at the one dihedral angle where `2w² = 1 + k` holds. Where `k` varies along
  the edge, the circle's `w` varies with it.
* `conic_tangent`: the arc leaves `p₁` along `a − p₁`, `B'(0) = 2w (a − p₁)`, for every `w`. Since
  `a − p₁` lies in face 1's tangent plane at the spring, every conic fillet is G1 to its faces.
-/

namespace ForgeExact.Fillet.Conic

open Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

/-- The numerator of the arc, about the centre: `(1 − t)² u + 2w t(1 − t) v + t² s`. -/
def num (u v s : V K) (w t : K) : V K :=
  V.add (V.add (V.smul ((1 - t) * (1 - t)) u) (V.smul (2 * w * t * (1 - t)) v)) (V.smul (t * t) s)

/-- Its denominator. -/
def den (w t : K) : K := (1 - t) * (1 - t) + 2 * w * t * (1 - t) + t * t

/-- The apex about the centre: `(u + s)/(1 + k)`. -/
def apex (u s : V K) (k : K) : V K := V.smul (1 / (1 + k)) (V.add u s)

/-- **The conic is the ball's circle iff `2w² = 1 + k`.** -/
theorem conic_circle {u s : V K} {r k w t : K} (hu : V.dot u u = r * r) (hs : V.dot s s = r * r)
    (hk : V.dot u s = r * r * k) (hr : r ≠ 0) (k1 : k ≠ 1) (k2 : 1 + k ≠ 0)
    (ht : t * (1 - t) ≠ 0) :
    V.dot (num u (apex u s k) s w t) (num u (apex u s k) s w t) = r * r * (den w t * den w t)
      ↔ 2 * (w * w) = 1 + k := by
  let l := 1 / (1 + k)
  let A := (1 - t) * (1 - t) + 2 * w * t * (1 - t) * l
  let C := t * t + 2 * w * t * (1 - t) * l
  have hl : l * (1 + k) = 1 := by simp only [l]; grind
  have lin : num u (apex u s k) s w t = V.add (V.smul A u) (V.smul C s) := by
    apply V.ext' <;> simp only [num, apex, A, C, l, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y,
      V.smul_z] <;> grind
  have nrm : ∀ a c : K, V.dot (V.add (V.smul a u) (V.smul c s)) (V.add (V.smul a u) (V.smul c s))
      = a * a * V.dot u u + 2 * a * c * V.dot u s + c * c * V.dot s s := by
    intro a c
    simp only [V.dot_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z]; grind
  -- |N|² − r² D² = −2 r² t²(1 − t)² (1 − k)/(1 + k) · (1 + k − 2w²)
  have key : V.dot (num u (apex u s k) s w t) (num u (apex u s k) s w t) - r * r * (den w t * den w t)
      = r * r * (t * (1 - t)) * (t * (1 - t)) * (-2 * (1 - k) * l) * (1 + k - 2 * (w * w)) := by
    rw [lin, nrm, hu, hs, hk]
    generalize hm : 2 * w * t * (1 - t) = m
    have eA : A = (1 - t) * (1 - t) + m * l := by simp only [A]; grind
    have eC : C = t * t + m * l := by simp only [C]; grind
    have eD : den w t = (1 - t) * (1 - t) + t * t + m := by simp only [den]; grind
    rw [eA, eC, eD]
    have hm2 : m * m = 4 * (w * w) * ((t * (1 - t)) * (t * (1 - t))) := by rw [← hm]; grind
    generalize l = L at hl ⊢
    have e1 : 2 * (m * L) * ((1 - t) * (1 - t) + t * t) * (1 + k)
        = 2 * m * ((1 - t) * (1 - t) + t * t) := by grind
    have e2 : 2 * (m * L) * (m * L) * (1 + k) = 2 * (m * m) * L := by grind
    have e3 : 2 * L - 1 = (1 - k) * L := by grind
    grind
  have hr2 : r * r ≠ 0 := by
    intro h; rcases le_or_lt' 0 r with h' | h'
    · have := mul_pos' (show 0 < r by grind) (show 0 < r by grind); grind
    · have := mul_pos' (show 0 < -r by grind) (show 0 < -r by grind); grind
  have c : r * r * (t * (1 - t)) * (t * (1 - t)) * (-2 * (1 - k) * l) ≠ 0 := by
    have : -2 * (1 - k) * l ≠ 0 := by
      intro h
      have h' : -2 * (1 - k) * l * (1 + k) = 0 := by rw [h]; grind
      have : 2 * (1 - k) = 0 := by grind
      grind
    intro h
    have h1 : r * r * (t * (1 - t)) * (t * (1 - t)) ≠ 0 := by
      intro h0
      have : r * r * ((t * (1 - t)) * (t * (1 - t))) = 0 := by grind
      rcases mul_eq_zero' this with h2 | h2
      · exact hr2 h2
      · rcases mul_eq_zero' h2 with h3 | h3 <;> exact ht h3
    rcases mul_eq_zero' h with h2 | h2
    · exact h1 h2
    · exact this h2
  constructor
  · intro h
    have : r * r * (t * (1 - t)) * (t * (1 - t)) * (-2 * (1 - k) * l) * (1 + k - 2 * (w * w)) = 0 := by
      rw [← key]; grind
    rcases mul_eq_zero' this with h2 | h2
    · exact absurd h2 c
    · grind
  · intro h
    have : 1 + k - 2 * (w * w) = 0 := by grind
    rw [this] at key
    grind
where
  mul_eq_zero' {a b : K} (h : a * b = 0) : a = 0 ∨ b = 0 := by
    rcases Classical.em (a = 0) with ha | ha
    · exact Or.inl ha
    · right
      have : a⁻¹ * (a * b) = b := by grind
      grind

/-- **The arc leaves the spring along the apex**, `B'(0) = 2w (a − p₁)` (the quotient rule at `t = 0`,
where the denominator is 1): every conic fillet is tangent to its faces. -/
theorem conic_tangent (u v : V K) (w : K) :
    -- N'(0) D(0) − N(0) D'(0), with N'(0) = −2u + 2w v, D(0) = 1, N(0) = u, D'(0) = 2w − 2
    V.sub (V.add (V.smul (-2) u) (V.smul (2 * w) v)) (V.smul (2 * w - 2) u)
      = V.smul (2 * w) (V.sub v u) := by
  apply V.ext' <;> simp only [V.sub_x, V.sub_y, V.sub_z, V.add_x, V.add_y, V.add_z, V.smul_x,
    V.smul_y, V.smul_z] <;> grind

end ForgeExact.Fillet.Conic
