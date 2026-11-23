import ForgeExact.Fillet.Closing

/-!
# Overflow, decided exactly

general-method.md P3: along a crease arc, a generator `g` not active on it blocks the ball where
`d_g(C(s)) < r`. In the spec, overflow is not a refusal. Its first event on the arc is a vertex of the
crease graph (cliff onset, roll-over, a swallowed face). What the kernel must decide exactly is
**whether there is an event on the arc at all**: `Clear`.

For a line crease `C(s) = o + s d`, `s ∈ [s₀, s₁]`, the squared clearance from a plane, from a
cylinder's axis, or from a sphere's centre is a quadratic in `s`. `Clear` is then "a quadratic is
≥ 0 on an interval", and `quadNonneg` decides it exactly:

* `quadNonneg_spec`: for `a ≤ b`, `quadNonneg A B C a b = true` iff `A s² + B s + C ≥ 0` for every
  `s ∈ [a, b]`, in any ordered field (ℚ for the executable model);
* `clear_sphere`: a ball rolled along the crease misses the open ball of radius `ρ` about `p` (a
  sphere generator, or a vertex with `ρ = 0`) iff `quadNonneg` holds for
  `|o + s d − p|² − (ρ + r)²`.

So the kernel's refusal of an overflow, where it has no construction for the event, is specified
exactly: it refuses iff `quadNonneg` is false, and names the generator.
-/

namespace ForgeExact.Fillet

open Lean.Grind

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LE K] [LT K] [Std.IsLinearOrder K] [Std.LawfulOrderLT K]
  [OrderedRing K]

def quad (A B C s : K) : K := A * s * s + B * s + C

open Classical in
/-- Whether `A s² + B s + C ≥ 0` on `[a, b]`: at the ends, and at the vertex when it is a minimum
inside. -/
noncomputable def quadNonneg (A B C a b : K) : Bool :=
  decide (0 ≤ quad A B C a) && decide (0 ≤ quad A B C b) &&
    (decide (A ≤ 0) ||
      decide (-B / (2 * A) ≤ a) || decide (b ≤ -B / (2 * A)) ||
      decide (0 ≤ quad A B C (-B / (2 * A))))

/-- **The decision is exact.** -/
theorem quadNonneg_spec {A B C a b : K} (hab : a ≤ b) :
    quadNonneg A B C a b = true ↔ ∀ s, a ≤ s → s ≤ b → 0 ≤ quad A B C s := by
  constructor
  · intro h s hs₁ hs₂
    simp only [quadNonneg, Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq] at h
    obtain ⟨⟨ha, hb⟩, hv⟩ := h
    rcases le_or_lt' A 0 with hA | hA
    · -- concave: above the chord, which is ≥ 0
      have e : (b - a) * quad A B C s
          = (b - s) * quad A B C a + (s - a) * quad A B C b - A * (b - a) * ((s - a) * (b - s)) := by
        simp only [quad]; grind
      have p₁ := mul_nonneg' (show 0 ≤ b - s by grind) ha
      have p₂ := mul_nonneg' (show 0 ≤ s - a by grind) hb
      have p₃ := mul_nonneg' (mul_nonneg' (show 0 ≤ -A by grind) (show 0 ≤ b - a by grind))
        (mul_nonneg' (show 0 ≤ s - a by grind) (show 0 ≤ b - s by grind))
      rcases le_or_lt' b a with h' | h'
      · have : s = a := by grind
        rw [this]; exact ha
      · have : 0 ≤ (b - a) * quad A B C s := by grind
        rcases le_or_lt' 0 (quad A B C s) with h'' | h''
        · exact h''
        · have := mul_lt_mul_of_pos_left' h'' (show 0 < b - a by grind); grind
    · -- convex: v the vertex, B = −2Av
      let v := -B / (2 * A)
      have hB : B = -2 * A * v := by simp only [v]; grind
      have shift : ∀ t, quad A B C s - quad A B C t = A * ((s - t) * (s + t - 2 * v)) := by
        intro t; simp only [quad]; rw [hB]; grind
      rcases hv with ((h0 | hv) | hv) | hv
      · grind
      · have := shift a
        have := mul_nonneg' (le_of_lt'' hA)
          (mul_nonneg' (show 0 ≤ s - a by grind) (show 0 ≤ s + a - 2 * v by grind))
        grind
      · have := shift b
        have := mul_nonneg' (le_of_lt'' hA)
          (mul_nonneg' (show 0 ≤ b - s by grind) (show 0 ≤ 2 * v - s - b by grind))
        grind
      · have := shift v
        have := mul_nonneg' (le_of_lt'' hA) (sq_nonneg' (s - v))
        grind
  · intro h
    have ha := h a (by grind) hab
    have hb := h b hab (by grind)
    simp only [quadNonneg, Bool.and_eq_true, Bool.or_eq_true, decide_eq_true_eq]
    refine ⟨⟨ha, hb⟩, ?_⟩
    rcases le_or_lt' A 0 with hA | hA
    · exact Or.inl (Or.inl (Or.inl hA))
    · rcases le_or_lt' (-B / (2 * A)) a with h1 | h1
      · exact Or.inl (Or.inl (Or.inr h1))
      · rcases le_or_lt' b (-B / (2 * A)) with h2 | h2
        · exact Or.inl (Or.inr h2)
        · exact Or.inr (h _ (by grind) (by grind))

where le_of_lt'' {x y : K} (h : x < y) : x ≤ y := by grind

/-- **A ball rolled along a line crease misses a sphere generator** (radius `ρ` about `p`; a vertex
for `ρ = 0`) on `[a, b]` iff the clearance quadratic is non-negative there:
`|o + s d − p|² − (ρ + r)² = |d|² s² + 2 d·(o − p) s + |o − p|² − (ρ + r)²`. -/
theorem clear_sphere {o d p : V K} {ρ r a b : K} (hab : a ≤ b) :
    quadNonneg (V.dot d d) (2 * V.dot d (V.sub o p)) (V.dot (V.sub o p) (V.sub o p) - (ρ + r) * (ρ + r))
        a b = true ↔
      ∀ s, a ≤ s → s ≤ b → (ρ + r) * (ρ + r) ≤ V.d2 (V.add o (V.smul s d)) p := by
  rw [quadNonneg_spec hab]
  have e : ∀ s : K, quad (V.dot d d) (2 * V.dot d (V.sub o p))
      (V.dot (V.sub o p) (V.sub o p) - (ρ + r) * (ρ + r)) s
      = V.d2 (V.add o (V.smul s d)) p - (ρ + r) * (ρ + r) := by
    intro s
    simp only [quad, V.d2_def, V.dot_def, V.add_x, V.add_y, V.add_z, V.smul_x, V.smul_y, V.smul_z,
      V.sub_x, V.sub_y, V.sub_z]
    grind
  constructor
  · intro h s h1 h2; have := h s h1 h2; rw [e] at this; grind
  · intro h s h1 h2; rw [e]; have := h s h1 h2; grind

end ForgeExact.Fillet
