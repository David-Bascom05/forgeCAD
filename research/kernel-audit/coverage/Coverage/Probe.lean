/-!
# `interior_point`'s probe on a strip, and the specification an exact probe must meet

`classify_specs` classifies a region at a point inside it, from `interior_point`
(boolean.rs:6150–6175):
- `clear_step` (6179–6207) steps off a half-edge by s₀, s₀/2, s₀/4, … while the step is at least
  2·LINEAR (6188), with s₀ = min(0.01·half chord, 0.05).
- It takes the first step whose probe is at least that step from every other piece of the region.
- If no step is clear, `probe` (6174) steps the default 1% of the half chord, at most 0.05,
  whatever lies there.

**The strip.** Here a region is a strip of width `w` (mm, over ℚ): two long sides, and ends far
from the probe. A probe a step `s` off one long side is `w - s` from the other, so it is clear iff
`s ≤ w - s`. The steps are `s k = 2^(n-1-k) · u` for `k < n`, where `u` is the last step tried
(`u` ≥ 2·LINEAR > u/2).

The geometry of each band, where the probe sits a distance `w` across:
- G3: a sliver `δ` wide;
- G4: a strip `(reach/2)·θ` wide at its probe;
- G5: the stretch beside a corner.

**Theorems.**
- `fits_iff`: some step fits iff `2·u ≤ w`.
- `kernelProbe_inside_iff`: the kernel's probe lands strictly inside the strip iff `2·u ≤ w` or the
  fallback step is under `w`.
- `g3_edge`: at a half chord ≥ 5 (s₀ = 0.05, n = 11, u = 0.05·2⁻¹¹), the probe of a strip
  `w ≤ 0.05` wide is outside it iff `w < 0.05·2⁻¹⁰`. With the sliver existing only past LINEAR,
  this is G3's band (LINEAR, 0.05·2⁻¹⁰], whose edge was measured at 4.8828·LINEAR.
- `ProbeSpec`: what the fix-probe session's exact interior point has to satisfy: strictly inside
  every strip of positive width. `kernel_not_spec`: the kernel's probe does not (a strip
  2·LINEAR wide). `midpoint_spec`: the midpoint does.

Kernel-checked, core Lean.
-/

namespace Coverage.Probe

/-- The k-th step, with `n` steps and the last `u`. -/
def step (u : Rat) (n k : Nat) : Rat := (2 ^ (n - 1 - k) : Nat) * u

/-- The first step that is clear across a strip of width `w`. -/
def firstFit (u : Rat) (n : Nat) (w : Rat) : Option Nat :=
  (List.range n).find? fun k => decide (2 * step u n k ≤ w)

/-- The kernel's probe: its distance from the side it steps off. -/
def kernelProbe (u : Rat) (n : Nat) (sdef w : Rat) : Rat :=
  match firstFit u n w with
  | some k => step u n k
  | none => sdef

theorem one_le_two_pow (m : Nat) : (1 : Rat) ≤ ((2 ^ m : Nat) : Rat) := by
  have : 1 ≤ 2 ^ m := Nat.one_le_two_pow
  exact_mod_cast this

theorem step_last (u : Rat) (n : Nat) (hn : 0 < n) : step u n (n - 1) = u := by
  simp [step, show n - 1 - (n - 1) = 0 by omega]

theorem step_ge (u : Rat) (hu : 0 ≤ u) (n k : Nat) : u ≤ step u n k := by
  unfold step
  have h := one_le_two_pow (n - 1 - k)
  have : (1 : Rat) * u ≤ ((2 ^ (n - 1 - k) : Nat) : Rat) * u := Rat.mul_le_mul_of_nonneg_right h hu
  simpa using this

/-- Some step fits across the strip iff it is at least twice the last step wide. -/
theorem fits_iff (u : Rat) (hu : 0 ≤ u) (n : Nat) (hn : 0 < n) (w : Rat) :
    (∃ k, k < n ∧ 2 * step u n k ≤ w) ↔ 2 * u ≤ w := by
  constructor
  · rintro ⟨k, _, hk⟩
    have := step_ge u hu n k
    grind
  · intro h
    exact ⟨n - 1, by omega, by rw [step_last u n hn]; exact h⟩

theorem firstFit_some (u : Rat) (n : Nat) (w : Rat) (k : Nat) (h : firstFit u n w = some k) :
    k < n ∧ 2 * step u n k ≤ w := by
  unfold firstFit at h
  have hm := List.mem_of_find?_eq_some h
  have hp := List.find?_some h
  simp only [List.mem_range, decide_eq_true_eq] at hm hp
  exact ⟨hm, hp⟩

theorem firstFit_none (u : Rat) (n : Nat) (w : Rat) (h : firstFit u n w = none) :
    ∀ k, k < n → ¬ 2 * step u n k ≤ w := by
  unfold firstFit at h
  intro k hk hle
  have := List.find?_eq_none.mp h k (List.mem_range.mpr hk)
  simp [hle] at this

theorem step_pos (u : Rat) (hu : 0 < u) (n k : Nat) : 0 < step u n k := by
  have := step_ge u (Rat.le_of_lt hu) n k; grind

/-- **The kernel's probe is inside the strip** iff a step fits (the strip is at least twice the
last step wide) or the fallback step is under its width. -/
theorem kernelProbe_inside_iff (u sdef w : Rat) (hu : 0 < u) (hs : 0 < sdef) (n : Nat) (hn : 0 < n) :
    (0 < kernelProbe u n sdef w ∧ kernelProbe u n sdef w < w) ↔ (2 * u ≤ w ∨ sdef < w) := by
  unfold kernelProbe
  cases hf : firstFit u n w with
  | some k =>
    obtain ⟨hk, hle⟩ := firstFit_some u n w k hf
    have hp := step_pos u hu n k
    have h2u : 2 * u ≤ w := (fits_iff u (Rat.le_of_lt hu) n hn w).mp ⟨k, hk, hle⟩
    simp only
    constructor
    · intro _; exact Or.inl h2u
    · intro _; exact ⟨hp, by grind⟩
  | none =>
    have hno := firstFit_none u n w hf
    have : ¬ 2 * u ≤ w := fun h => by
      obtain ⟨k, hk, hle⟩ := (fits_iff u (Rat.le_of_lt hu) n hn w).mpr h
      exact hno k hk hle
    simp only
    constructor
    · rintro ⟨_, h⟩; exact Or.inr h
    · rintro (h | h)
      · exact absurd h this
      · exact ⟨hs, h⟩

/-- At a half chord of 5 or more: s₀ = 0.05 and the steps 0.05·2⁻ᵏ for k = 0..11 (the last,
0.05·2⁻¹¹ ≈ 2.44e-5, is ≥ 2·LINEAR = 2e-5 and the next is not: so n = 12, u = 0.05·2⁻¹¹), the
fallback 0.05. -/
def u10 : Rat := 5 / 100 / 2 ^ 11
def LINEAR : Rat := 1 / 100000

theorem u10_floor : 2 * LINEAR ≤ u10 ∧ u10 / 2 < 2 * LINEAR := by
  unfold u10 LINEAR; grind

/-- **G3's band as a theorem**: a strip at most 0.05 wide gets its probe outside it iff it is
narrower than 0.05·2⁻¹⁰ mm (= 4.8828125·LINEAR). -/
theorem g3_edge (w : Rat) (hw : w ≤ 5 / 100) :
    ¬(0 < kernelProbe u10 12 (5 / 100) w ∧ kernelProbe u10 12 (5 / 100) w < w) ↔ w < 5 / 100 / 2 ^ 10 := by
  rw [kernelProbe_inside_iff u10 (5 / 100) w (by unfold u10; grind) (by grind) 12 (by decide)]
  unfold u10
  constructor
  · intro h; apply Classical.byContradiction; intro hn; apply h; left; grind
  · intro h hc; rcases hc with hc | hc <;> grind

theorem g3_edge_linear : (5 / 100 / 2 ^ 10 : Rat) = 48828125 / 10000000 * LINEAR := by
  unfold LINEAR; grind

/-- **What an exact interior point must meet**: inside every strip of positive width. -/
def ProbeSpec (probe : Rat → Rat) : Prop := ∀ w, 0 < w → 0 < probe w ∧ probe w < w

/-- The kernel's probe fails it: a strip twice LINEAR wide (inside G3's band). -/
theorem kernel_not_spec : ¬ ProbeSpec (kernelProbe u10 12 (5 / 100)) := by
  intro h
  have := (g3_edge (2 * LINEAR) (by unfold LINEAR; grind)).mpr (by unfold LINEAR; grind)
  exact this (h (2 * LINEAR) (by unfold LINEAR; grind))

/-- The midpoint meets it. -/
theorem midpoint_spec : ProbeSpec fun w => w / 2 := by
  intro w hw; constructor <;> grind

end Coverage.Probe
