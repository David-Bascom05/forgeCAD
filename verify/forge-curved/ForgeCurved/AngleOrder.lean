import Mathlib.Analysis.SpecialFunctions.Complex.Arg

/-!
# (S4) The angle order at a vertex: the screened comparator is the true angular order

curved-builder-lean.md S4. Today's `split_face` orders the ways out of a vertex by
`forge_math::predicates::angle_cmp` (curved Rust, 2026-11-23). Each way is a direction in the
face's `(u, v)` chart. The comparator:
* first ranks the half-plane **class**: `y < 0`, then the `+x` axis, then `y > 0`, then the `−x`
  axis;
* within a class, decides by the sign of the cross product `cross2(u, v)`.

**`before_iff_arg`**: for nonzero directions, the comparator holds iff `arg u < arg v`, with
`Complex.arg ∈ (−π, π]`. So the comparator is the true angular order, and sorting by it gives the
cyclic order round the vertex.

The screens decide the class (the sign of `y`, and of `x` on the axis) and the cross product's sign
with proven enclosures. A direction exactly on the axis is a keyed fact (S0). A cross product whose
enclosure holds 0 is a tangent tie: that question's exact answer (curvature order) or a typed
refusal. The statement needs the `(u, v)` chart to keep orientation at the vertex (a regular
parametrisation; not at a sphere's pole, which `uv_dir` special-cases).
-/

namespace ForgeCurved.AngleOrder

open Complex Real

/-- The half-plane class: `y < 0`, the `+x` axis, `y > 0`, the `−x` axis. -/
noncomputable def hclass (z : ℂ) : ℕ :=
  if z.im < 0 then 0 else if z.im = 0 ∧ 0 < z.re then 1 else if 0 < z.im then 2 else 3

/-- `cross2(u, v)`. -/
def cross (u v : ℂ) : ℝ := u.re * v.im - u.im * v.re

/-- **The comparator**: a lower class, or the same class and a positive cross product. -/
def Before (u v : ℂ) : Prop := hclass u < hclass v ∨ (hclass u = hclass v ∧ 0 < cross u v)

theorem cross_eq {u v : ℂ} (hu : u ≠ 0) (hv : v ≠ 0) :
    cross u v = ‖u‖ * ‖v‖ * Real.sin (arg v - arg u) := by
  have nu : ‖u‖ ≠ 0 := norm_ne_zero_iff.mpr hu
  have nv : ‖v‖ ≠ 0 := norm_ne_zero_iff.mpr hv
  have ru : u.re = ‖u‖ * Real.cos (arg u) := by rw [cos_arg hu]; field_simp
  have iu : u.im = ‖u‖ * Real.sin (arg u) := by rw [sin_arg]; field_simp
  have rv : v.re = ‖v‖ * Real.cos (arg v) := by rw [cos_arg hv]; field_simp
  have iv : v.im = ‖v‖ * Real.sin (arg v) := by rw [sin_arg]; field_simp
  rw [cross, ru, iu, rv, iv, Real.sin_sub]; ring

/-- Each class's arguments. -/
theorem arg_of_class {z : ℂ} (hz : z ≠ 0) :
    (hclass z = 0 ∧ arg z < 0) ∨ (hclass z = 1 ∧ arg z = 0 ∧ z.im = 0) ∨
      (hclass z = 2 ∧ 0 < arg z ∧ arg z < π) ∨ (hclass z = 3 ∧ arg z = π ∧ z.im = 0) := by
  unfold hclass
  split_ifs with h1 h2 h3
  · exact Or.inl ⟨rfl, arg_neg_iff.mpr h1⟩
  · exact Or.inr (Or.inl ⟨rfl, arg_eq_zero_iff.mpr ⟨h2.2.le, h2.1⟩, h2.1⟩)
  · refine Or.inr (Or.inr (Or.inl ⟨rfl, ?_, ?_⟩))
    · rcases (arg_nonneg_iff.mpr h3.le).lt_or_eq with h | h
      · exact h
      · exact absurd (arg_eq_zero_iff.mp h.symm).2 (ne_of_gt h3)
    · rcases (arg_le_pi z).lt_or_eq with h | h
      · exact h
      · exact absurd (arg_eq_pi_iff.mp h).2 (ne_of_gt h3)
  · have him : z.im = 0 := by
      push Not at h1 h3; exact le_antisymm h3 h1
    have hre : z.re < 0 := by
      rcases lt_trichotomy z.re 0 with h | h | h
      · exact h
      · exact absurd (Complex.ext h him) hz
      · exact absurd ⟨him, h⟩ h2
    exact Or.inr (Or.inr (Or.inr ⟨rfl, arg_eq_pi_iff.mpr ⟨hre, him⟩, him⟩))

/-- Within an open half-plane, the cross product's sign is the order of the arguments. -/
theorem cross_pos_iff {u v : ℂ} (hu : u ≠ 0) (hv : v ≠ 0) (hd : |arg v - arg u| < π) :
    0 < cross u v ↔ arg u < arg v := by
  rw [cross_eq hu hv]
  have hn : 0 < ‖u‖ * ‖v‖ := mul_pos (norm_pos_iff.mpr hu) (norm_pos_iff.mpr hv)
  obtain ⟨h1, h2⟩ := abs_lt.mp hd
  constructor
  · intro h
    by_contra hle
    push Not at hle
    rcases hle.lt_or_eq with hlt | heq
    · have := Real.sin_neg_of_neg_of_neg_pi_lt (x := arg v - arg u) (by linarith) h1
      nlinarith
    · rw [heq, sub_self, Real.sin_zero, mul_zero] at h; exact lt_irrefl _ h
  · intro h
    exact mul_pos hn (Real.sin_pos_of_pos_of_lt_pi (by linarith) h2)

/-- **The comparator is the angular order.** -/
theorem before_iff_arg {u v : ℂ} (hu : u ≠ 0) (hv : v ≠ 0) : Before u v ↔ arg u < arg v := by
  have pp := Real.pi_pos
  have lu := neg_pi_lt_arg u
  have lv := neg_pi_lt_arg v
  unfold Before
  rcases arg_of_class hu with ⟨cu, au⟩ | ⟨cu, au, iu⟩ | ⟨cu, au1, au2⟩ | ⟨cu, au, iu⟩ <;>
    rcases arg_of_class hv with ⟨cv, av⟩ | ⟨cv, av, iv⟩ | ⟨cv, av1, av2⟩ | ⟨cv, av, iv⟩ <;>
    rw [cu, cv]
  -- same open class: the cross product
  · simp only [lt_irrefl, true_and, false_or]
    exact cross_pos_iff hu hv (abs_lt.mpr ⟨by linarith, by linarith⟩)
  all_goals first
    | (simp only [lt_irrefl, true_and, false_or]
       exact cross_pos_iff hu hv (abs_lt.mpr ⟨by linarith, by linarith⟩))
    | (simp only [lt_irrefl, true_and, false_or, cross, iu, iv, mul_zero, sub_zero, lt_irrefl]
       constructor <;> intro h <;> linarith)
    | (constructor
       · intro _; linarith
       · intro _; norm_num)
    | (constructor
       · rintro (h | ⟨h, -⟩) <;> norm_num at h
       · intro h; linarith)

end ForgeCurved.AngleOrder
