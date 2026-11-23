import Mathlib.Analysis.InnerProductSpace.Basic
import Mathlib.Analysis.InnerProductSpace.Calculus
import Mathlib.Analysis.Calculus.Deriv.Mul
import Mathlib.Analysis.Calculus.Deriv.Add

/-!
# M.ct: the angles every torus × cylinder loop holds (forge-geom `critical_us`)

forge-geom's `intersect` finds a torus on a cylinder (`Form::Poly4`) between *events*, the
cylinder angles u where the root count of F(u, ·) changes. Its discriminant search can miss a
loop narrower than a sample; `critical_us` lists angles that every loop must hold, and the count
is asked there. The argument, for a loop Γ that does not wind round the cylinder: Γ bounds a disc
D on the cylinder with the same span of u; on Γ the distance to the torus's core circle C is the
minor radius r; so the distance has an extremum inside D, at

1. a point of C (the distance is 0 there, and not smooth): where C pierces the cylinder;
2. a point of the torus's axis (every point of C is as far: not smooth);
3. a smooth critical point p of the distance restricted to the cylinder.

This file proves the step that makes (3) computable: at such a p, with nearest core point
c = C(θ), the core point's θ is a critical point of g(θ) = dist²(C(θ), cylinder axis), so p lies
on the radial line through one of g's (at most four) critical points.

* `foot_eq_of_radial`, `sub_foot_of_radial`: a point c on the radial line through p (p − c a
  multiple of p's radial vector) has p's foot on the axis, and its radial vector is a multiple of
  p's;
* `critical_of_radial`: if moreover p − c ⟂ C'(θ) (c is p's nearest point on C) and p ≠ c, then
  ⟪C(θ) − foot C(θ), C'(θ)⟫ = 0;
* `hasDerivAt_distSq_axis`: g′(θ) = 2 ⟪C(θ) − foot C(θ), C′(θ)⟫, so that inner product's zeros
  are g's critical points.

Open (ledger M.ct): the topological steps (a loop bounds a disc with its own span of u; the
extremum in it), that the count's nearest change from such an angle on each side is that loop's
event (two events at one u can cancel in the count), `count`'s own thresholds, and the float
error of the computed angles.
-/

set_option linter.unusedSectionVars false

namespace ForgeCurved.MctSample

open RealInnerProductSpace

variable {V : Type*} [NormedAddCommGroup V] [InnerProductSpace ℝ V]

/-- The foot of `x` on the axis through `a` along the unit vector `d`. -/
noncomputable def foot (a d x : V) : V := a + ⟪x - a, d⟫ • d

/-- A radial vector is square to the axis. -/
theorem radial_perp (a d x : V) (hd : ⟪d, d⟫ = 1) : ⟪x - foot a d x, d⟫ = 0 := by
  simp only [foot, sub_add_eq_sub_sub, inner_sub_left, inner_smul_left, hd]
  simp

/-- A point on p's radial line has p's foot. -/
theorem foot_eq_of_radial (a d p c : V) (lam : ℝ) (hd : ⟪d, d⟫ = 1)
    (hpc : p - c = lam • (p - foot a d p)) : foot a d c = foot a d p := by
  have hc : c = p - lam • (p - foot a d p) := by rw [← hpc]; abel
  have h0 := radial_perp a d p hd
  have : ⟪c - a, d⟫ = ⟪p - a, d⟫ := by
    rw [hc, show p - lam • (p - foot a d p) - a = (p - a) - lam • (p - foot a d p) by abel,
      inner_sub_left, inner_smul_left, h0]
    simp
  simp only [foot, this]

/-- Its radial vector is a multiple of p's. -/
theorem sub_foot_of_radial (a d p c : V) (lam : ℝ) (hd : ⟪d, d⟫ = 1)
    (hpc : p - c = lam • (p - foot a d p)) :
    c - foot a d c = (1 - lam) • (p - foot a d p) := by
  rw [foot_eq_of_radial a d p c lam hd hpc]
  have hc : c = p - lam • (p - foot a d p) := by rw [← hpc]; abel
  rw [hc, sub_smul, one_smul]
  abel

/-- **The critical angle.** p on the cylinder is a smooth critical point of the distance to C:
p − c is radial at p (the cylinder's normal) and square to C's tangent `t` at c (c nearest), with
p ≠ c. Then c's radial vector is square to `t`: g′ vanishes at c's angle. -/
theorem critical_of_radial (a d p c t : V) (lam : ℝ) (hd : ⟪d, d⟫ = 1) (hlam : lam ≠ 0)
    (hpc : p - c = lam • (p - foot a d p)) (ht : ⟪p - c, t⟫ = 0) :
    ⟪c - foot a d c, t⟫ = 0 := by
  rw [sub_foot_of_radial a d p c lam hd hpc, inner_smul_left]
  have : ⟪p - foot a d p, t⟫ = 0 := by
    rw [hpc, inner_smul_left] at ht
    simpa [hlam] using ht
  simp [this]

/-- The squared distance to the axis, as the code's g: ‖x − a‖² − ⟪x − a, d⟫². -/
theorem distSq_axis (a d x : V) (hd : ⟪d, d⟫ = 1) :
    ‖x - foot a d x‖ ^ 2 = ‖x - a‖ ^ 2 - ⟪x - a, d⟫ ^ 2 := by
  have h1 : x - foot a d x = (x - a) - ⟪x - a, d⟫ • d := by simp only [foot]; abel
  rw [h1, @norm_sub_sq_real, norm_smul, inner_smul_right, Real.norm_eq_abs, mul_pow, sq_abs,
    show ‖d‖ ^ 2 = 1 by rw [← real_inner_self_eq_norm_sq, hd]]
  ring

/-- **g′.** Along a curve γ with derivative γ′ at θ, g = ‖γ − foot γ‖² has derivative
2 ⟪γ(θ) − foot γ(θ), γ′⟫: g's critical points are the zeros of that inner product. -/
theorem hasDerivAt_distSq_axis (a d : V) (hd : ⟪d, d⟫ = 1) (γ : ℝ → V) (γ' : V) (θ : ℝ)
    (hγ : HasDerivAt γ γ' θ) :
    HasDerivAt (fun s => ‖γ s - foot a d (γ s)‖ ^ 2)
      (2 * ⟪γ θ - foot a d (γ θ), γ'⟫) θ := by
  have e : (fun s => ‖γ s - foot a d (γ s)‖ ^ 2) =
      fun s => ‖γ s - a‖ ^ 2 - ⟪γ s - a, d⟫ ^ 2 := by
    funext s; exact distSq_axis a d (γ s) hd
  rw [e]
  have h1 : HasDerivAt (fun s => γ s - a) γ' θ := hγ.sub_const a
  have hn : HasDerivAt (fun s => ‖γ s - a‖ ^ 2) (2 * ⟪γ θ - a, γ'⟫) θ := by
    simpa using h1.norm_sq
  have hi : HasDerivAt (fun s => ⟪γ s - a, d⟫) ⟪γ', d⟫ θ := by
    simpa using HasDerivAt.inner ℝ h1 (hasDerivAt_const θ d)
  have hq : HasDerivAt (fun s => ⟪γ s - a, d⟫ ^ 2) (2 * ⟪γ θ - a, d⟫ * ⟪γ', d⟫) θ := by
    have h := hi.mul hi
    simp only [← sq] at h
    exact HasDerivAt.congr_deriv h (by ring)
  refine HasDerivAt.congr_deriv (HasDerivAt.sub hn hq) ?_
  rw [show γ θ - foot a d (γ θ) = (γ θ - a) - ⟪γ θ - a, d⟫ • d by simp only [foot]; abel,
    inner_sub_left (γ θ - a) (⟪γ θ - a, d⟫ • d) γ', inner_smul_left, real_inner_comm d γ']
  simp
  ring

end ForgeCurved.MctSample
