import Mathlib.Analysis.SpecialFunctions.Sqrt
import Mathlib.Analysis.SpecialFunctions.Trigonometric.Basic

/-!
# C.node: one branch out of a tangent node (forge-bool `node.rs`)

Two rounds, one of each operand, are tubes of radii `rA`, `rB` whose spines lie parallel to a
common plane S at their radius from it, on one side: each touches S along its spring. Where the
springs cross (the node) both share S's normal. Across the spine at angle φ from S's normal, a
tube's point lies `r |sin φ|` from the spring in S's direction and `r (1 − cos φ)` from S
(`cap_section`): so over S, within its radius of the spring, the tube's cap is the graph of the
depth `h r ρ = r − √(r² − ρ²)` at plane distance ρ from the spring.

Where both faces lie past their springs, a point on both caps has `h rA ρA = h rB ρB`. `h r` is
strictly increasing on [0, r] (`h_strictMonoOn`), so:

* `one_branch`: for each ρA there is at most one ρB on both caps: one branch, not the X's two;
* `branch_increasing`: along it ρB grows with ρA: it leaves the node, never returns;
* `branch_meets_springs_only_at_node`: on the branch ρA = 0 iff ρB = 0, both at the node, which
  is on both caps (`h_zero`).

The code's side (node.rs): the tubes recognised exactly (cylinder, torus, a pipe about a line or a
circle), the spines checked parallel to S at the radius on one side, the node the springs' crossing
in closed form, the mend kept within a quarter of `min (r, a circular spine's radius)` of it,
where the cap is the one graph above, and clear of any other crossing of the springs.

Open (ledger C.node): that a face's tube is the whole of its cap within that reach (the face's own
trimming is the boolean's, as everywhere), and the trace's points (`towards_touch`'s Newton steps)
are on both surfaces only to its tolerance.
-/

namespace ForgeCurved.BlendNode

open Real

/-- A tube's depth under S at plane distance `ρ` from its spring. -/
noncomputable def h (r ρ : ℝ) : ℝ := r - Real.sqrt (r ^ 2 - ρ ^ 2)

/-- The spring is on S. -/
theorem h_zero {r : ℝ} (hr : 0 ≤ r) : h r 0 = 0 := by
  simp [h, Real.sqrt_sq hr]

/-- The cross-section: at angle φ from S's normal (cos φ ≥ 0, the cap), the tube's point is
`r |sin φ|` from the spring along S and `r (1 − cos φ)` under it, and that depth is `h`. -/
theorem cap_section {r φ : ℝ} (hr : 0 ≤ r) (hc : 0 ≤ Real.cos φ) :
    r * (1 - Real.cos φ) = h r (r * |Real.sin φ|) := by
  have e : r ^ 2 - (r * |Real.sin φ|) ^ 2 = (r * Real.cos φ) ^ 2 := by
    have := Real.sin_sq_add_cos_sq φ
    rw [mul_pow, sq_abs, mul_pow]
    nlinarith
  rw [h, e, Real.sqrt_sq (mul_nonneg hr hc)]
  ring

/-- Deeper the farther from the spring, up to the radius. -/
theorem h_strictMonoOn {r : ℝ} (hr : 0 < r) : StrictMonoOn (h r) (Set.Icc 0 r) := by
  intro a ha b hb hab
  have ha0 : 0 ≤ a := ha.1
  have hbr : b ≤ r := hb.2
  have sq_lt : a ^ 2 < b ^ 2 := by nlinarith
  have nn : 0 ≤ r ^ 2 - b ^ 2 := by nlinarith
  have lt : r ^ 2 - b ^ 2 < r ^ 2 - a ^ 2 := by linarith
  have := Real.sqrt_lt_sqrt nn lt
  simp only [h]
  linarith

/-- For depths strictly increasing on [0, δ]: at most one partner per point, so one branch. -/
theorem one_branch {f g : ℝ → ℝ} {δ u v₁ v₂ : ℝ} (hg : StrictMonoOn g (Set.Icc 0 δ))
    (h₁ : v₁ ∈ Set.Icc 0 δ) (h₂ : v₂ ∈ Set.Icc 0 δ) (e₁ : f u = g v₁) (e₂ : f u = g v₂) :
    v₁ = v₂ :=
  hg.injOn h₁ h₂ (e₁.symm.trans e₂)

/-- Along the branch the partner grows with the point: it leaves the node and does not turn back. -/
theorem branch_increasing {f g : ℝ → ℝ} {δ u₁ u₂ v₁ v₂ : ℝ} (hf : StrictMonoOn f (Set.Icc 0 δ))
    (hg : StrictMonoOn g (Set.Icc 0 δ)) (hu₁ : u₁ ∈ Set.Icc 0 δ) (hu₂ : u₂ ∈ Set.Icc 0 δ)
    (hv₁ : v₁ ∈ Set.Icc 0 δ) (hv₂ : v₂ ∈ Set.Icc 0 δ) (lt : u₁ < u₂) (e₁ : f u₁ = g v₁)
    (e₂ : f u₂ = g v₂) : v₁ < v₂ := by
  have : g v₁ < g v₂ := by rw [← e₁, ← e₂]; exact hf hu₁ hu₂ lt
  exact (hg.lt_iff_lt hv₁ hv₂).mp this

/-- The branch meets either spring only at the node. -/
theorem branch_meets_springs_only_at_node {f g : ℝ → ℝ} {δ u v : ℝ} (hf : StrictMonoOn f (Set.Icc 0 δ))
    (hg : StrictMonoOn g (Set.Icc 0 δ)) (hδ : 0 ≤ δ) (f0 : f 0 = 0) (g0 : g 0 = 0)
    (hu : u ∈ Set.Icc 0 δ) (hv : v ∈ Set.Icc 0 δ) (e : f u = g v) : u = 0 ↔ v = 0 := by
  have z : (0 : ℝ) ∈ Set.Icc 0 δ := ⟨le_refl 0, hδ⟩
  constructor
  · intro hu0
    subst hu0
    exact hg.injOn hv z (by rw [← e, f0, g0])
  · intro hv0
    subst hv0
    exact hf.injOn hu z (by rw [e, f0, g0])

/-- The tubes' case: B's depth `h rB` over [0, δ], δ at most its radius (A's depth any value). -/
theorem tubes_one_branch {rA rB δ ρA ρ₁ ρ₂ : ℝ} (hB : 0 < rB) (hδB : δ ≤ rB)
    (h₁ : ρ₁ ∈ Set.Icc 0 δ) (h₂ : ρ₂ ∈ Set.Icc 0 δ) (e₁ : h rA ρA = h rB ρ₁)
    (e₂ : h rA ρA = h rB ρ₂) : ρ₁ = ρ₂ := by
  have hg : StrictMonoOn (h rB) (Set.Icc 0 δ) :=
    (h_strictMonoOn hB).mono (Set.Icc_subset_Icc_right hδB)
  exact one_branch hg h₁ h₂ e₁ e₂

end ForgeCurved.BlendNode
