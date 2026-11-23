import Mathlib.Analysis.SpecialFunctions.Trigonometric.Deriv
import Mathlib.Analysis.SpecialFunctions.Trigonometric.Complex

/-!
# The fold of one bend, over ℝ (research/kernel-audit/audit2.md §2.0)

Coordinates are wall `a`'s joint frame (the kernel's `ja`): `x` across the bend, `y` along it,
`z` the normal of the reference skin. Wall `a`'s mold line is `x = 0`, the wall at `x ≤ 0`; a
point at depth `w ∈ [0, t]` below the reference skin is at `z = -w` (refold.rs `wall_at`).

A bend has inside radius `R`, angle `θ ∈ (0, π)`, gauge `t`, allowance `BA`, and a sense `σ`:
`σ = +1` when it turns toward the reference side (`towardRef`), `-1` otherwise. The kernel's
numbers, and the ones this file proves right:

* `ρ = R` toward the reference side, else `R + t` (build.rs, `towardRef`): the reference skin's
  radius, so the inside skin's radius is `R` either way (`inner_radius`);
* `s = ρ · sin θ / (1 + cos θ)`, the setback on the reference skin (shop.rs);
* `rn = BA / θ`, the neutral radius, and `κ = (rn - R) / t`, the effective K;
* the zone point `(u, y, w)` folds to angle `φ = u / rn` at radius `ρ + σ w` round the axis
  through `(-s, ·, σ ρ)` (refold.rs `Zone::at`);
* wall `b`, in its own joint frame (the kernel's `jb`: mold line `x' = 0`, wall at `x' ≥ 0`),
  folds by `J`, the turn by `θ` about the mold line, and develops by `shift (BA - 2 s)`.
-/

namespace Sheet

open Real

/-- The sense of a bend: toward the reference side (`pos`) or away from it. -/
inductive Sense
  | pos
  | neg
  deriving DecidableEq, Repr

/-- `σ`, as a number. -/
def Sense.val : Sense → ℝ
  | .pos => 1
  | .neg => -1

@[simp] theorem Sense.val_sq (σ : Sense) : σ.val * σ.val = 1 := by cases σ <;> simp [Sense.val]

/-- The data of one bend, with the hypotheses the kernel checks before it builds. -/
structure Bend where
  R : ℝ
  θ : ℝ
  t : ℝ
  BA : ℝ
  sense : Sense
  hR : 0 ≤ R
  hθ0 : 0 < θ
  hθπ : θ < π
  ht : 0 < t
  hBA : 0 < BA

namespace Bend

variable (b : Bend)

/-- The reference skin's radius: `R` toward the reference side, `R + t` away from it. -/
def ρ : ℝ := match b.sense with
  | .pos => b.R
  | .neg => b.R + b.t

/-- The setback on the reference skin. -/
noncomputable def s : ℝ := b.ρ * sin b.θ / (1 + cos b.θ)

/-- The neutral radius. -/
noncomputable def rn : ℝ := b.BA / b.θ

/-- The effective K. -/
noncomputable def κ : ℝ := (b.rn - b.R) / b.t

/-- `σ`. -/
def σ : ℝ := b.sense.val

/-- The neutral layer's depth below the reference skin. -/
noncomputable def wn : ℝ := match b.sense with
  | .pos => b.rn - b.R
  | .neg => b.R + b.t - b.rn

theorem one_add_cos_pos : 0 < 1 + cos b.θ := by
  have h1 : -1 < cos b.θ := by
    have := Real.cos_lt_cos_of_nonneg_of_le_pi b.hθ0.le le_rfl b.hθπ
    simpa [Real.cos_pi] using this
  linarith

theorem sin_pos : 0 < sin b.θ := Real.sin_pos_of_pos_of_lt_pi b.hθ0 b.hθπ

theorem rn_pos : 0 < b.rn := div_pos b.hBA b.hθ0

/-- The setback is `ρ · tan (θ / 2)`. -/
theorem s_eq_tan : b.s = b.ρ * tan (b.θ / 2) := by
  have h2 : cos (b.θ / 2) ≠ 0 := by
    apply ne_of_gt; apply Real.cos_pos_of_mem_Ioo; constructor <;> linarith [b.hθ0, b.hθπ]
  have hs : sin b.θ = 2 * sin (b.θ / 2) * cos (b.θ / 2) := by
    rw [← Real.sin_two_mul]; ring_nf
  have hc : 1 + cos b.θ = 2 * cos (b.θ / 2) ^ 2 := by
    rw [show b.θ = 2 * (b.θ / 2) by ring, Real.cos_two_mul]; ring_nf
  rw [s, hs, Real.tan_eq_sin_div_cos, hc]
  field_simp

/-- **`towardRef`**: the inside skin's radius is `R` in both senses; every layer of the material
is at radius at least `R`, and the inside skin is at exactly `R`. -/
theorem inner_radius : (∀ w, 0 ≤ w → w ≤ b.t → b.R ≤ b.ρ + b.σ * w) ∧
    ∃ w, 0 ≤ w ∧ w ≤ b.t ∧ b.ρ + b.σ * w = b.R := by
  have ht := b.ht
  cases h : b.sense <;> simp only [ρ, σ, h, Sense.val]
  · exact ⟨fun w h0 _ => by linarith, 0, le_rfl, ht.le, by ring⟩
  · exact ⟨fun w _ h1 => by linarith, b.t, ht.le, le_rfl, by ring⟩

/-- **The neutral layer.** In both senses the layer at depth `wn` is at radius `rn`. -/
theorem neutral_radius : b.ρ + b.σ * b.wn = b.rn := by
  cases h : b.sense <;> simp only [ρ, σ, wn, h, Sense.val] <;> ring

/-- **Physical K** (§2.0 item 7): the neutral layer lies inside the material iff `κ ∈ [0, 1]`,
iff `R ≤ rn ≤ R + t`. -/
theorem wn_mem_iff : (0 ≤ b.wn ∧ b.wn ≤ b.t) ↔ (0 ≤ b.κ ∧ b.κ ≤ 1) := by
  have ht := b.ht
  have k0 : 0 ≤ b.κ ↔ b.R ≤ b.rn := by
    rw [κ, div_nonneg_iff]; constructor
    · rintro (⟨h, _⟩ | ⟨_, h⟩) <;> linarith
    · intro h; left; exact ⟨by linarith, ht.le⟩
  have k1 : b.κ ≤ 1 ↔ b.rn ≤ b.R + b.t := by
    rw [κ, div_le_one ht]; constructor <;> intro h <;> linarith
  rw [k0, k1]
  cases h : b.sense <;> simp only [wn, h] <;> constructor <;> rintro ⟨h1, h2⟩ <;>
    constructor <;> linarith

theorem rn_mem_iff : (b.R ≤ b.rn ∧ b.rn ≤ b.R + b.t) ↔ (0 ≤ b.κ ∧ b.κ ≤ 1) := by
  have ht := b.ht
  rw [κ, div_nonneg_iff, div_le_one ht]
  constructor
  · rintro ⟨h1, h2⟩; exact ⟨Or.inl ⟨by linarith, ht.le⟩, by linarith⟩
  · rintro ⟨(⟨h1, _⟩ | ⟨_, h⟩), h2⟩
    · exact ⟨by linarith, by linarith⟩
    · linarith

/-! ## The fold -/

/-- A point of ℝ³, as `(x, y, z)`. -/
abbrev V3 := ℝ × ℝ × ℝ

/-- The fold angle of zone coordinate `u`. -/
noncomputable def φ (u : ℝ) : ℝ := u / b.rn

/-- The zone point `(u, y, w)`, folded, in wall `a`'s joint frame (refold.rs `Zone::at`). -/
noncomputable def zone (u y w : ℝ) : V3 :=
  (-b.s + (b.ρ + b.σ * w) * sin (b.φ u), y,
    b.σ * b.ρ - b.σ * (b.ρ + b.σ * w) * cos (b.φ u))

/-- The turn by `θ` about the mold line: wall `b`'s joint frame in wall `a`'s. -/
noncomputable def J (p : V3) : V3 :=
  (p.1 * cos b.θ - b.σ * p.2.2 * sin b.θ, p.2.1, b.σ * p.1 * sin b.θ + p.2.2 * cos b.θ)

/-- **The far tangent line folds to angle `θ`** (§2.0 item 3). -/
theorem φ_BA : b.φ b.BA = b.θ := by
  simp only [φ, rn]; field_simp [b.hθ0.ne', b.hBA.ne']

/-- **Continuity at `u = 0`**: the zone starts on wall `a`'s tangent line `x = -s`. -/
theorem zone_zero (y w : ℝ) : b.zone 0 y w = (-b.s, y, -w) := by
  have := b.sense.val_sq
  simp only [zone, φ, zero_div, Real.sin_zero, Real.cos_zero, σ]
  exact Prod.ext (by ring) (Prod.ext rfl (by linear_combination (-w) * this))

/-- **Continuity at `u = BA`**: the zone ends on wall `b`'s tangent line `x' = s`, folded. This is
where `s = ρ · sin θ / (1 + cos θ)` and `θ < π` are needed. -/
theorem zone_BA (y w : ℝ) : b.zone b.BA y w = b.J (b.s, y, -w) := by
  have hc := b.one_add_cos_pos
  have hsc := Real.sin_sq_add_cos_sq b.θ
  have hσ := b.sense.val_sq
  have hs : b.s * (1 + cos b.θ) = b.ρ * sin b.θ := by
    simp only [s]; field_simp
  simp only [zone, J, φ_BA, σ]
  refine Prod.ext ?_ (Prod.ext rfl ?_)
  · simp only; nlinarith [hs]
  · simp only
    have : b.ρ * (1 - cos b.θ) = b.s * sin b.θ := by
      have h2 : b.ρ * (1 - cos b.θ) * (1 + cos b.θ) = b.s * sin b.θ * (1 + cos b.θ) := by
        linear_combination (-b.ρ) * hsc + (-sin b.θ) * hs
      exact mul_right_cancel₀ hc.ne' h2
    linear_combination (b.sense.val) * this + (-(w) * cos b.θ) * hσ

/-! ## Derivatives: continuity of direction and the isometry -/

/-- `∂F/∂u`, componentwise. -/
theorem hasDerivAt_zone_u (y w u : ℝ) :
    HasDerivAt (fun u => (b.zone u y w).1) ((b.ρ + b.σ * w) / b.rn * cos (b.φ u)) u ∧
    HasDerivAt (fun u => (b.zone u y w).2.1) 0 u ∧
    HasDerivAt (fun u => (b.zone u y w).2.2)
      (b.σ * ((b.ρ + b.σ * w) / b.rn) * sin (b.φ u)) u := by
  have hφ : HasDerivAt b.φ (1 / b.rn) u := by
    have := (hasDerivAt_id u).div_const b.rn
    exact this
  refine ⟨?_, hasDerivAt_const _ _, ?_⟩
  · have := ((hφ.sin).const_mul (b.ρ + b.σ * w)).const_add (-b.s)
    have e : (fun u => (b.zone u y w).1) = fun x => -b.s + (b.ρ + b.σ * w) * sin (b.φ x) := by
      funext x; simp [zone]
    rw [e]; convert this using 1; field_simp
  · have := ((hφ.cos).const_mul (b.σ * (b.ρ + b.σ * w))).const_sub (b.σ * b.ρ)
    have e : (fun u => (b.zone u y w).2.2) =
        fun x => b.σ * b.ρ - b.σ * (b.ρ + b.σ * w) * cos (b.φ x) := by
      funext x; simp [zone]
    rw [e]; convert this using 1; field_simp

/-- `∂F/∂y = (0, 1, 0)`. -/
theorem hasDerivAt_zone_y (u w y : ℝ) :
    HasDerivAt (fun y => (b.zone u y w).1) 0 y ∧ HasDerivAt (fun y => (b.zone u y w).2.1) 1 y ∧
    HasDerivAt (fun y => (b.zone u y w).2.2) 0 y :=
  ⟨hasDerivAt_const _ _, hasDerivAt_id y, hasDerivAt_const _ _⟩

/-- The first fundamental form of the fold at `(u, y, w)`: `E = |∂F/∂u|²`, `F = ∂F/∂u · ∂F/∂y`,
`G = |∂F/∂y|²`, from the derivatives above. -/
noncomputable def metricE (u w : ℝ) : ℝ :=
  ((b.ρ + b.σ * w) / b.rn * cos (b.φ u)) ^ 2 + 0 ^ 2 +
    (b.σ * ((b.ρ + b.σ * w) / b.rn) * sin (b.φ u)) ^ 2

theorem metricE_eq (u w : ℝ) : b.metricE u w = ((b.ρ + b.σ * w) / b.rn) ^ 2 := by
  have := Real.sin_sq_add_cos_sq (b.φ u)
  have hσ := b.sense.val_sq
  simp only [metricE, σ]
  linear_combination ((b.ρ + b.sense.val * w) / b.rn) ^ 2 * this +
    ((b.ρ + b.sense.val * w) / b.rn) ^ 2 * sin (b.φ u) ^ 2 * hσ

/-- **`iso`** (§2.0 item 2): at the neutral depth the fold is an isometry: `E = 1`, and (from the
derivatives) `F = 0`, `G = 1`. -/
theorem iso (u : ℝ) : b.metricE u b.wn = 1 ∧
    ((b.ρ + b.σ * b.wn) / b.rn * cos (b.φ u)) * 0 + 0 * 1 +
      (b.σ * ((b.ρ + b.σ * b.wn) / b.rn) * sin (b.φ u)) * 0 = 0 ∧
    (0 : ℝ) ^ 2 + 1 ^ 2 + 0 ^ 2 = 1 := by
  refine ⟨?_, by ring, by norm_num⟩
  rw [metricE_eq, neutral_radius, div_self b.rn_pos.ne', one_pow]

/-- The neutral layer's folded length is the allowance: `rn · θ = BA`. -/
theorem neutral_length : b.rn * b.θ = b.BA := by
  simp only [rn]; field_simp [b.hθ0.ne']

/-- **Direction at `u = 0`**: the zone leaves along wall `a`'s `+x`. -/
theorem dir_zero :
    (cos (b.φ 0), sin (b.φ 0)) = (1, 0) := by simp [φ]

/-- **Direction at `u = BA`**: the zone arrives along wall `b`'s `+x'`, folded: `J (1, 0, 0)`
minus `J 0`, which is `(cos θ, 0, σ sin θ)`. -/
theorem dir_BA : (cos (b.φ b.BA), (0 : ℝ), b.σ * sin (b.φ b.BA)) =
    ((b.J (1, 0, 0)).1 - (b.J 0).1, (b.J (1, 0, 0)).2.1 - (b.J 0).2.1,
      (b.J (1, 0, 0)).2.2 - (b.J 0).2.2) := by
  simp [J, φ_BA]

/-! ## Development and deduction -/

/-- Wall `b`'s joint coordinate `x'` on the flat, in wall `a`'s: `shift (BA - 2 s)`
(build.rs: `dev[a] · ja · shift(BA - 2s) · jb⁻¹`). -/
noncomputable def develop (x' : ℝ) : ℝ := x' + (b.BA - 2 * b.s)

/-- The zone coordinate of a flat point at `x` in wall `a`'s joint frame. -/
noncomputable def u (x : ℝ) : ℝ := x + b.s

/-- **`deduction`** (§2.0 item 4): the two mold lines are `BA - 2 s` apart on the flat, so for
flanges `La` and `Lb` measured to the mold line the flat length is `La + Lb - BD` with the bend
deduction `BD = 2 s - BA`; and the two tangent lines are at `u = 0` and `u = BA`. -/
theorem deduction (La Lb : ℝ) :
    b.develop 0 - 0 = b.BA - 2 * b.s ∧
    b.develop Lb - (-La) = La + Lb - (2 * b.s - b.BA) ∧
    b.u (-b.s) = 0 ∧ b.u (b.develop b.s) = b.BA := by
  simp only [develop, u]; refine ⟨by ring, by ring, by ring, by ring⟩

/-- The flat-to-folded map of one bend, on wall `a`'s joint frame: wall `a` rigidly, the zone
round its axis, wall `b` rigidly through `J`, by where the flat point lies. -/
noncomputable def F (x y w : ℝ) : V3 :=
  if x ≤ -b.s then (x, y, -w)
  else if x ≤ b.BA - b.s then b.zone (b.u x) y w
  else b.J (x - (b.BA - 2 * b.s), y, -w)

/-- **The fold is well defined at both tangent lines**: the rules meeting there agree. -/
theorem F_glue (y w : ℝ) :
    b.zone (b.u (-b.s)) y w = (-b.s, y, -w) ∧
    b.zone (b.u (b.BA - b.s)) y w = b.J ((b.BA - b.s) - (b.BA - 2 * b.s), y, -w) := by
  constructor
  · simp only [u, neg_add_cancel]; exact b.zone_zero y w
  · simp only [u, sub_add_cancel]
    rw [b.zone_BA]; congr 1; ext <;> simp; ring

/-- **Refold of one bend**: a point of wall `b` at its own joint coordinate `x' ≥ s` (past its
tangent line), developed and then folded, is where the folded part has it: `J (x', y, -w)`. -/
theorem refold_b (x' y w : ℝ) (h : b.s ≤ x') :
    b.F (b.develop x') y w = b.J (x', y, -w) := by
  have hBA := b.hBA
  have h1 : ¬ b.develop x' ≤ -b.s := by simp only [develop]; intro h'; linarith
  unfold F
  rw [if_neg h1]
  by_cases h2 : b.develop x' ≤ b.BA - b.s
  · rw [if_pos h2]
    have hx : x' = b.s := by simp only [develop] at h2; linarith
    subst hx
    have e : b.develop b.s = b.BA - b.s := by simp only [develop]; ring
    rw [e, (b.F_glue y w).2]
    congr 1; ext <;> simp; ring
  · rw [if_neg h2]; congr 1; ext <;> simp [develop]

/-- **Refold of wall `a`**: a point of wall `a` up to its tangent line folds to itself. -/
theorem refold_a (x y w : ℝ) (h : x ≤ -b.s) : b.F x y w = (x, y, -w) := by
  simp [F, h]

end Bend

end Sheet
