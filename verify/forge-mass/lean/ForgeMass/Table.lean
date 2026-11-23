import ForgeMass.Degrees

/-!
# The rule table of mass.rs:2416–2436, row by row

`F u v k` is moment k of the integrand at S(u, v), about a point `c` (mass.rs:1971–1982). For each
surface, the lemmas below give F's class along u with v fixed and along v with u fixed, as the
rule each axis gets needs (Degrees.lean's table), and the area integrand's.
-/

noncomputable section
open Matrix

namespace ForgeMass

theorem TrigR.vec {k : ℕ} {w : ℝ → V3} {f g : ℝ → ℝ} (P Q R : V3)
    (hw : ∀ a, w a = f a • P + g a • Q + R) (hf : TrigR k f) (hg : TrigR k g) :
    ∀ i, TrigR k (fun a => w a i) := by
  intro i
  have := TrigR.comb hf hg P Q R i
  convert this using 1; funext a; rw [hw a]

theorem PolyR.vec {d : ℕ} {w : ℝ → V3} {f g : ℝ → ℝ} (P Q R : V3)
    (hw : ∀ a, w a = f a • P + g a • Q + R) (hf : PolyR d f) (hg : PolyR d g) :
    ∀ i, PolyR d (fun a => w a i) := by
  intro i
  have := PolyR.comb hf hg P Q R i
  convert this using 1; funext a; rw [hw a]

theorem trig_cos (A : ℝ) : TrigR 1 (fun x => A * Real.cos x) := TrigR.smul A TrigR.cos
theorem trig_sin (A : ℝ) : TrigR 1 (fun x => A * Real.sin x) := TrigR.smul A TrigR.sin
theorem trig_cos_cos (A : ℝ) : TrigR 2 (fun x => A * (Real.cos x * Real.cos x)) :=
  TrigR.smul A (TrigR.mono (by norm_num) (TrigR.cos.mul TrigR.cos))
theorem trig_cos_sin (A : ℝ) : TrigR 2 (fun x => A * (Real.cos x * Real.sin x)) :=
  TrigR.smul A (TrigR.mono (by norm_num) (TrigR.cos.mul TrigR.sin))
theorem trig_c (k : ℕ) : TrigR k (fun _ => (0 : ℝ)) := TrigR.const k 0
theorem poly_x (A : ℝ) : PolyR 1 (fun x => A * x) := PolyR.smul A PolyR.id
theorem poly_c (d : ℕ) : PolyR d (fun _ => (0 : ℝ)) := PolyR.const d 0

/-- The moments at a surface point. -/
def F (d : Deriv) (c : V3) (k : Fin 10) : ℝ := mom (d.p - c) d.n k

section
variable (f : Frame) (c : V3)

/-! ## Plane: GL2 either way -/

theorem plane_u (v : ℝ) (k : Fin 10) : PolyR 3 (fun u => F (plane f u v) c k) := by
  have hx := PolyR.vec (w := fun u => (plane f u v).p - c) f.X f.Y (f.o + v • f.Y - c)
    (fun u => by simp only [plane]; module) (poly_x 1) (poly_c 1)
  have hn := PolyR.vec (w := fun u => (plane f u v).n) f.X f.Y f.Z
    (fun u => by rw [plane_n]; simp) (poly_c 0) (poly_c 0)
  simpa [F] using mom_poly hx hn k

theorem plane_v (u : ℝ) (k : Fin 10) : PolyR 3 (fun v => F (plane f u v) c k) := by
  have hx := PolyR.vec (w := fun v => (plane f u v).p - c) f.Y f.X (f.o + u • f.X - c)
    (fun v => by simp only [plane]; module) (poly_x 1) (poly_c 1)
  have hn := PolyR.vec (w := fun v => (plane f u v).n) f.X f.Y f.Z
    (fun v => by rw [plane_n]; simp) (poly_c 0) (poly_c 0)
  simpa [F] using mom_poly hx hn k

/-! ## Cylinder: GL2 along v; round u a fit of degree 4, polynomial of degree 3 along v -/

theorem cylinder_v (r u : ℝ) (k : Fin 10) : PolyR 3 (fun v => F (cylinder f r u v) c k) := by
  have hx := PolyR.vec (w := fun v => (cylinder f r u v).p - c) f.Z f.X (f.o + r • f.rad u - c)
    (fun v => by simp only [cylinder]; module) (poly_x 1) (poly_c 1)
  have hn := PolyR.vec (w := fun v => (cylinder f r u v).n) f.X f.Y (r • f.rad u)
    (fun v => by rw [cylinder_n]; simp) (poly_c 0) (poly_c 0)
  simpa [F] using mom_poly hx hn k

theorem cylinder_u (r v : ℝ) (k : Fin 10) : TrigR 4 (fun u => F (cylinder f r u v) c k) := by
  have hx := TrigR.vec (w := fun u => (cylinder f r u v).p - c) f.X f.Y (f.o + v • f.Z - c)
    (fun u => by simp only [cylinder, Frame.rad]; module) (trig_cos r) (trig_sin r)
  have hn := TrigR.vec (w := fun u => (cylinder f r u v).n) f.X f.Y 0
    (fun u => by rw [cylinder_n]; simp only [Frame.rad]; module) (trig_cos r) (trig_sin r)
  simpa [F] using mom_trig hx hn k

/-! ## Cone: GL3 along v; round u a fit of degree 4, polynomial of degree 4 along v -/

theorem cone_v (r k' u : ℝ) (k : Fin 10) : PolyR 4 (fun v => F (cone f r k' u v) c k) := by
  have hx := PolyR.vec (w := fun v => (cone f r k' u v).p - c) (k' • f.rad u + f.Z) f.X
    (f.o + r • f.rad u - c) (fun v => by simp only [cone]; module) (poly_x 1) (poly_c 1)
  have hn := PolyR.vec (w := fun v => (cone f r k' u v).n) (k' • (f.rad u + (-k') • f.Z)) f.X
    (r • (f.rad u + (-k') • f.Z)) (fun v => by rw [cone_n]; module) (poly_x 1) (poly_c 1)
  simpa [F] using mom_poly hx hn k

theorem cone_u (r k' v : ℝ) (k : Fin 10) : TrigR 4 (fun u => F (cone f r k' u v) c k) := by
  have hx := TrigR.vec (w := fun u => (cone f r k' u v).p - c) f.X f.Y (f.o + v • f.Z - c)
    (fun u => by simp only [cone, Frame.rad]; module) (trig_cos (r + v * k')) (trig_sin (r + v * k'))
  have hn := TrigR.vec (w := fun u => (cone f r k' u v).n) f.X f.Y ((-(r + v * k') * k') • f.Z)
    (fun u => by rw [cone_n]; simp only [Frame.rad]; module) (trig_cos (r + v * k'))
    (trig_sin (r + v * k'))
  simpa [F] using mom_trig hx hn k

/-! ## Sphere: round u a fit of degree 4, along v of degree 5 -/

theorem sphere_u (r v : ℝ) (k : Fin 10) : TrigR 4 (fun u => F (sphere f r u v) c k) := by
  have hx := TrigR.vec (w := fun u => (sphere f r u v).p - c) f.X f.Y
    (f.o + (r * Real.sin v) • f.Z - c) (fun u => by simp only [sphere, Frame.rad]; module)
    (trig_cos (r * Real.cos v)) (trig_sin (r * Real.cos v))
  have hn := TrigR.vec (w := fun u => (sphere f r u v).n) f.X f.Y
    ((r ^ 2 * Real.cos v * Real.sin v) • f.Z) (fun u => by rw [sphere_n]; simp only [Frame.rad]; module)
    (trig_cos (r ^ 2 * Real.cos v * Real.cos v)) (trig_sin (r ^ 2 * Real.cos v * Real.cos v))
  simpa [F] using mom_trig hx hn k

theorem sphere_v (r u : ℝ) (k : Fin 10) : TrigR 5 (fun v => F (sphere f r u v) c k) := by
  have hx := TrigR.vec (w := fun v => (sphere f r u v).p - c) (f.rad u) f.Z (f.o - c)
    (fun v => by simp only [sphere]; module) (trig_cos r) (trig_sin r)
  have hn := TrigR.vec (w := fun v => (sphere f r u v).n) (f.rad u) f.Z 0
    (fun v => by rw [sphere_n]; module) (trig_cos_cos (r ^ 2)) (trig_cos_sin (r ^ 2))
  simpa [F] using mom_trig hx hn k

/-! ## Torus: as the sphere -/

theorem torus_u (R r v : ℝ) (k : Fin 10) : TrigR 4 (fun u => F (torus f R r u v) c k) := by
  have hx := TrigR.vec (w := fun u => (torus f R r u v).p - c) f.X f.Y
    (f.o + (r * Real.sin v) • f.Z - c) (fun u => by simp only [torus, Frame.rad]; module)
    (trig_cos (R + r * Real.cos v)) (trig_sin (R + r * Real.cos v))
  have hn := TrigR.vec (w := fun u => (torus f R r u v).n) f.X f.Y
    ((r * (R + r * Real.cos v) * Real.sin v) • f.Z)
    (fun u => by rw [torus_n]; simp only [Frame.rad]; module)
    (trig_cos (r * (R + r * Real.cos v) * Real.cos v)) (trig_sin (r * (R + r * Real.cos v) * Real.cos v))
  simpa [F] using mom_trig hx hn k

theorem torus_v (R r u : ℝ) (k : Fin 10) : TrigR 5 (fun v => F (torus f R r u v) c k) := by
  have hx := TrigR.vec (w := fun v => (torus f R r u v).p - c) (f.rad u) f.Z (f.o + R • f.rad u - c)
    (fun v => by simp only [torus]; module) (trig_cos r) (trig_sin r)
  have hcc : TrigR 2 (fun v => r * R * Real.cos v + r ^ 2 * (Real.cos v * Real.cos v)) :=
    (TrigR.mono (by norm_num) (trig_cos (r * R))).add (trig_cos_cos (r ^ 2))
  have hcs : TrigR 2 (fun v => r * R * Real.sin v + r ^ 2 * (Real.cos v * Real.sin v)) :=
    (TrigR.mono (by norm_num) (trig_sin (r * R))).add (trig_cos_sin (r ^ 2))
  have hn := TrigR.vec (w := fun v => (torus f R r u v).n) (f.rad u) f.Z 0
    (fun v => by rw [torus_n]; module) hcc hcs
  simpa [F] using mom_trig hx hn k

/-! ## The area integrand -/

theorem area_plane (u v : ℝ) : len (plane f u v).n = 1 := plane_area f u v

theorem area_cylinder (r u v : ℝ) (hr : 0 ≤ r) : len (cylinder f r u v).n = r := cylinder_area f r u v hr

/-- Constant round u; along v, a polynomial of degree 1 where ρ ≥ 0. -/
theorem area_cone (r k' u : ℝ) :
    PolyR 1 (fun v => (r + v * k') * Real.sqrt (1 + k' ^ 2)) ∧
      ∀ v, 0 ≤ r + v * k' → len (cone f r k' u v).n = (r + v * k') * Real.sqrt (1 + k' ^ 2) := by
  refine ⟨?_, fun v h => cone_area f r k' u v h⟩
  have := (PolyR.const 1 r).add (poly_x k')
  convert this.mul_const (Real.sqrt (1 + k' ^ 2)) using 1; funext v; ring

/-- The sphere's signed area integrand, r² cos v: degree 0 round u, 1 along v. -/
theorem area_sphere (r u : ℝ) (hr : r ≠ 0) :
    TrigR 1 (fun v => (sphere f r u v).n ⬝ᵥ ((sphere f r u v).p - f.o) / r) := by
  have := trig_cos (r ^ 2)
  convert this using 1; funext v; exact sphere_signed f r u v hr

/-- The torus's area integrand r(R + r cos v), signed or not: degree 0 round u, 1 along v. -/
theorem area_torus (R r : ℝ) : TrigR 1 (fun v => r * (R + r * Real.cos v)) := by
  have := (TrigR.const 1 (r * R)).add (trig_cos (r * r))
  convert this using 1; funext v; ring

end

end ForgeMass
end
