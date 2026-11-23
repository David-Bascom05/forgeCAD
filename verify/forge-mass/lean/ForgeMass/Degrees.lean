import ForgeMass.Surfaces
import ForgeMass.Divergence

/-!
# The integrand's degree along each parameter (mass.rs:2416–2436)

`face_integrals` integrates `F(a, b) = integrand(S(a, b) − c, Su × Sv)` along the inner parameter
`a` by a rule chosen by surface and axis. Each rule is exact only on a class of functions of `a`:
* GL2 for polynomials of degree ≤ 3, GL3 for degree ≤ 5;
* the (2k+1)-point fit (`Inner::Fourier`) for trigonometric polynomials of degree ≤ k.

The fit's coefficients are then interpolated along `b`:
* by a polynomial on d+1 points (`Along::Poly`);
* or by a trigonometric polynomial on 2m+1 points (`Along::Trig`, m = 9 − k).

Here, for each row of the table, every component of F is shown to lie in the class its rule needs,
along `a` with `b` fixed and along `b` with `a` fixed:

| surface, inner axis | rule along a | F along a | along b |
|---|---|---|---|
| plane, either | GL2 | poly 3 | (none needed) |
| cylinder, v | GL2 | poly 3 | |
| cone, v | GL3 | poly 4 | |
| cylinder, u | Fourier k = 4 | trig 4 | poly 3 |
| cone, u | Fourier k = 4 | trig 4 | poly 4 |
| sphere or torus, u | Fourier k = 4 | trig 4 | trig 5 |
| sphere or torus, v | Fourier k = 5 | trig 5 | trig 4 |

The area component (10) is `|n|`, or the signed forms of the sphere and spindle torus. It is in
the class where the face lies: on a cone where ρ ≥ 0, on a torus where ρ ≥ 0 (Surfaces.lean).
-/

noncomputable section
open Matrix

namespace ForgeMass

/-- mass.rs:224–239, the ten moments (component 10, the area, is `len n`). -/
def mom (x n : V3) : Fin 10 → ℝ
  | 0 => (x ⬝ᵥ n) / 3
  | 1 => x 0 * x 0 * n 0 / 2
  | 2 => x 1 * x 1 * n 1 / 2
  | 3 => x 2 * x 2 * n 2 / 2
  | 4 => x 0 * x 0 * x 0 * n 0 / 3
  | 5 => x 1 * x 1 * x 1 * n 1 / 3
  | 6 => x 2 * x 2 * x 2 * n 2 / 3
  | 7 => x 0 * x 0 * x 1 * n 0 / 2
  | 8 => x 1 * x 1 * x 2 * n 1 / 2
  | 9 => x 2 * x 2 * x 0 * n 2 / 2

/-- Each moment is the flux of Divergence.lean's field `phi k` through n. -/
theorem mom_flux (x n : V3) (k : Fin 10) :
    mom x n k = (phi k).f1 (x 0) (x 1) (x 2) * n 0 + (phi k).f2 (x 0) (x 1) (x 2) * n 1 +
      (phi k).f3 (x 0) (x 1) (x 2) * n 2 := by
  fin_cases k <;> simp [mom, phi, dotProduct, Fin.sum_univ_three] <;> ring

/-! ## Closure helpers -/

theorem TrigR.mul_const {k : ℕ} {f : ℝ → ℝ} (hf : TrigR k f) (c : ℝ) : TrigR k (fun x => f x * c) := by
  have := TrigR.smul c hf
  convert this using 1; funext x; ring

theorem PolyR.mul_const {d : ℕ} {f : ℝ → ℝ} (hf : PolyR d f) (c : ℝ) : PolyR d (fun x => f x * c) := by
  have := PolyR.smul c hf
  convert this using 1; funext x; ring

/-- A vector `f a • P + g a • Q + R`: each component is in the class of f and g. -/
theorem TrigR.comb {k : ℕ} {f g : ℝ → ℝ} (hf : TrigR k f) (hg : TrigR k g) (P Q R : V3) (i : Fin 3) :
    TrigR k (fun a => (f a • P + g a • Q + R) i) := by
  simp only [Pi.add_apply, Pi.smul_apply, smul_eq_mul]
  exact (hf.mul_const _).add (hg.mul_const _) |>.add (TrigR.const k _)

theorem PolyR.comb {d : ℕ} {f g : ℝ → ℝ} (hf : PolyR d f) (hg : PolyR d g) (P Q R : V3) (i : Fin 3) :
    PolyR d (fun a => (f a • P + g a • Q + R) i) := by
  simp only [Pi.add_apply, Pi.smul_apply, smul_eq_mul]
  exact (hf.mul_const _).add (hg.mul_const _) |>.add (PolyR.const d _)

theorem mom_trig {α β : ℕ} {x n : ℝ → V3} (hx : ∀ i, TrigR α (fun a => x a i))
    (hn : ∀ i, TrigR β (fun a => n a i)) (k : Fin 10) :
    TrigR (3 * α + β) (fun a => mom (x a) (n a) k) := by
  have m3 : ∀ i j l m, TrigR (3 * α + β) (fun a => x a i * x a j * x a l * n a m) := fun i j l m =>
    TrigR.mono (le_of_eq (by ring)) ((((hx i).mul (hx j)).mul (hx l)).mul (hn m))
  have m2 : ∀ i j, TrigR (3 * α + β) (fun a => x a i * x a i * n a j) := fun i j =>
    TrigR.mono (by omega) (((hx i).mul (hx i)).mul (hn j))
  fin_cases k
  · have : TrigR (3 * α + β) (fun a => ∑ i : Fin 3, x a i * n a i) :=
      TrigR.sum _ fun i _ => TrigR.mono (by omega) ((hx i).mul (hn i))
    convert this.smul (1 / 3) using 1; funext a; simp [mom, dotProduct]; ring
  · convert (m2 0 0).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m2 1 1).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m2 2 2).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m3 0 0 0 0).smul (1 / 3) using 1; funext a; simp [mom]; ring
  · convert (m3 1 1 1 1).smul (1 / 3) using 1; funext a; simp [mom]; ring
  · convert (m3 2 2 2 2).smul (1 / 3) using 1; funext a; simp [mom]; ring
  · convert (m3 0 0 1 0).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m3 1 1 2 1).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m3 2 2 0 2).smul (1 / 2) using 1; funext a; simp [mom]; ring

theorem mom_poly {α β : ℕ} {x n : ℝ → V3} (hx : ∀ i, PolyR α (fun a => x a i))
    (hn : ∀ i, PolyR β (fun a => n a i)) (k : Fin 10) :
    PolyR (3 * α + β) (fun a => mom (x a) (n a) k) := by
  have m3 : ∀ i j l m, PolyR (3 * α + β) (fun a => x a i * x a j * x a l * n a m) := fun i j l m =>
    PolyR.mono (le_of_eq (by ring)) ((((hx i).mul (hx j)).mul (hx l)).mul (hn m))
  have m2 : ∀ i j, PolyR (3 * α + β) (fun a => x a i * x a i * n a j) := fun i j =>
    PolyR.mono (by omega) (((hx i).mul (hx i)).mul (hn j))
  fin_cases k
  · have : PolyR (3 * α + β) (fun a => ∑ i : Fin 3, x a i * n a i) :=
      PolyR.sum _ fun i _ => PolyR.mono (by omega) ((hx i).mul (hn i))
    convert this.smul (1 / 3) using 1; funext a; simp [mom, dotProduct]; ring
  · convert (m2 0 0).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m2 1 1).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m2 2 2).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m3 0 0 0 0).smul (1 / 3) using 1; funext a; simp [mom]; ring
  · convert (m3 1 1 1 1).smul (1 / 3) using 1; funext a; simp [mom]; ring
  · convert (m3 2 2 2 2).smul (1 / 3) using 1; funext a; simp [mom]; ring
  · convert (m3 0 0 1 0).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m3 1 1 2 1).smul (1 / 2) using 1; funext a; simp [mom]; ring
  · convert (m3 2 2 0 2).smul (1 / 2) using 1; funext a; simp [mom]; ring

end ForgeMass
end
