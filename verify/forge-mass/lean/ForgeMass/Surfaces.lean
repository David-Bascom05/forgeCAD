import ForgeMass.Trig

/-!
# The analytic surfaces' normals and area integrands

`SurfaceGeom::deriv` for each surface (kernel/forge-geom/src/surface.rs, at 8250ebb):
* the plane at 485–496;
* the cylinder at 520–532;
* the cone at 597–610, with `radius_at` at 592;
* the sphere at 652–665;
* the torus at 727–743;
* `radial` at 467–470.

Each is written over an orthonormal right-handed frame. `n = du × dv` is the normal the integrand
reads (mass.rs:1981). Here it is in closed form, with the area integrand mass.rs uses: `|n|`, or for
the sphere and a spindle torus the signed forms of mass.rs:1983–1993. They equal `|n|` where the
face lies: v ∈ [−π/2, π/2] on a sphere, ρ ≥ 0 on a torus.
-/

noncomputable section
open Matrix

namespace ForgeMass

abbrev V3 := Fin 3 → ℝ

theorem cross_add (u v w : V3) : u ⨯₃ (v + w) = u ⨯₃ v + u ⨯₃ w := map_add _ _ _
theorem add_cross (u v w : V3) : (u + v) ⨯₃ w = u ⨯₃ w + v ⨯₃ w := LinearMap.map_add₂ _ _ _ _
theorem cross_smul (c : ℝ) (u v : V3) : u ⨯₃ (c • v) = c • (u ⨯₃ v) := map_smul _ _ _
theorem smul_cross (c : ℝ) (u v : V3) : (c • u) ⨯₃ v = c • (u ⨯₃ v) := LinearMap.map_smul₂ _ _ _ _
theorem neg_cross (u v : V3) : (-u) ⨯₃ v = -(u ⨯₃ v) := LinearMap.map_neg₂ _ _ _

/-- An orthonormal, right-handed frame. -/
structure Frame where
  o : V3
  X : V3
  Y : V3
  Z : V3
  xy : X ⨯₃ Y = Z
  yz : Y ⨯₃ Z = X
  zx : Z ⨯₃ X = Y
  xx : X ⬝ᵥ X = 1
  yy : Y ⬝ᵥ Y = 1
  zz : Z ⬝ᵥ Z = 1
  xy0 : X ⬝ᵥ Y = 0
  yz0 : Y ⬝ᵥ Z = 0
  zx0 : Z ⬝ᵥ X = 0

namespace Frame
variable (f : Frame)

/-- surface.rs:467–470 -/
def rad (u : ℝ) : V3 := Real.cos u • f.X + Real.sin u • f.Y
def drad (u : ℝ) : V3 := (-Real.sin u) • f.X + Real.cos u • f.Y

theorem yx : f.Y ⨯₃ f.X = -f.Z := by rw [← cross_anticomm, f.xy]
theorem zy : f.Z ⨯₃ f.Y = -f.X := by rw [← cross_anticomm, f.yz]
theorem xz : f.X ⨯₃ f.Z = -f.Y := by rw [← cross_anticomm, f.zx]

theorem drad_rad (u : ℝ) : f.drad u ⨯₃ f.rad u = -f.Z := by
  simp only [rad, drad, cross_add, add_cross, cross_smul, smul_cross, cross_self, f.xy, f.yx,
    smul_zero, add_zero, zero_add]
  linear_combination (norm := module) (-(Real.sin_sq_add_cos_sq u)) • f.Z

theorem drad_z (u : ℝ) : f.drad u ⨯₃ f.Z = f.rad u := by
  simp only [rad, drad, add_cross, smul_cross, f.xz, f.yz]
  module

theorem rad_z (u : ℝ) : f.rad u ⨯₃ f.Z = -f.drad u := by
  simp only [rad, drad, add_cross, smul_cross, f.xz, f.yz]
  module

theorem dot_rad_rad (u : ℝ) : f.rad u ⬝ᵥ f.rad u = 1 := by
  simp only [rad, add_dotProduct, dotProduct_add, smul_dotProduct, dotProduct_smul, smul_eq_mul,
    f.xx, f.yy, f.xy0, dotProduct_comm f.Y f.X]
  nlinarith [Real.sin_sq_add_cos_sq u]

theorem dot_rad_z (u : ℝ) : f.rad u ⬝ᵥ f.Z = 0 := by
  simp only [rad, add_dotProduct, smul_dotProduct, smul_eq_mul, f.yz0, dotProduct_comm f.X f.Z,
    f.zx0]
  ring

theorem dot_z_rad (u : ℝ) : f.Z ⬝ᵥ f.rad u = 0 := by rw [dotProduct_comm]; exact f.dot_rad_z u

/-- |a • rad + b • Z|² = a² + b². -/
theorem dot_comb (u a b : ℝ) : (a • f.rad u + b • f.Z) ⬝ᵥ (a • f.rad u + b • f.Z) = a ^ 2 + b ^ 2 := by
  simp only [add_dotProduct, dotProduct_add, smul_dotProduct, dotProduct_smul, smul_eq_mul,
    f.dot_rad_rad, f.dot_rad_z, f.dot_z_rad, f.zz]
  ring

end Frame

/-- A surface's point and first derivatives at (u, v) (`SurfDeriv`'s p, du, dv). -/
structure Deriv where
  p : V3
  du : V3
  dv : V3

def Deriv.n (d : Deriv) : V3 := d.du ⨯₃ d.dv

/-- surface.rs:485–496 -/
def plane (f : Frame) (u v : ℝ) : Deriv := ⟨f.o + u • f.X + v • f.Y, f.X, f.Y⟩
/-- surface.rs:520–532 -/
def cylinder (f : Frame) (r u v : ℝ) : Deriv := ⟨f.o + r • f.rad u + v • f.Z, r • f.drad u, f.Z⟩
/-- surface.rs:597–610; `radius_at v = r + v·k` (592), k = tan of the half angle. -/
def cone (f : Frame) (r k u v : ℝ) : Deriv :=
  ⟨f.o + (r + v * k) • f.rad u + v • f.Z, (r + v * k) • f.drad u, k • f.rad u + f.Z⟩
/-- surface.rs:652–665 -/
def sphere (f : Frame) (r u v : ℝ) : Deriv :=
  ⟨f.o + (r * Real.cos v) • f.rad u + (r * Real.sin v) • f.Z, (r * Real.cos v) • f.drad u,
    (-r * Real.sin v) • f.rad u + (r * Real.cos v) • f.Z⟩
/-- surface.rs:727–743 (major R, minor r). -/
def torus (f : Frame) (R r u v : ℝ) : Deriv :=
  ⟨f.o + (R + r * Real.cos v) • f.rad u + (r * Real.sin v) • f.Z, (R + r * Real.cos v) • f.drad u,
    (-r * Real.sin v) • f.rad u + (r * Real.cos v) • f.Z⟩
/-- The torus's `dvv` (surface.rs:741), which the spindle's area reads. -/
def torusDvv (f : Frame) (r u v : ℝ) : V3 := (-r * Real.cos v) • f.rad u + (-r * Real.sin v) • f.Z

/-! ## The normals -/

theorem plane_n (f : Frame) (u v : ℝ) : (plane f u v).n = f.Z := f.xy

theorem cylinder_n (f : Frame) (r u v : ℝ) : (cylinder f r u v).n = r • f.rad u := by
  simp [Deriv.n, cylinder, smul_cross, f.drad_z]

theorem cone_n (f : Frame) (r k u v : ℝ) :
    (cone f r k u v).n = (r + v * k) • (f.rad u + (-k) • f.Z) := by
  simp only [Deriv.n, cone, smul_cross, cross_add, cross_smul, f.drad_rad, f.drad_z]
  module

theorem sphere_n (f : Frame) (r u v : ℝ) :
    (sphere f r u v).n = (r ^ 2 * Real.cos v) • (Real.cos v • f.rad u + Real.sin v • f.Z) := by
  simp only [Deriv.n, sphere, smul_cross, cross_add, cross_smul, f.drad_rad, f.drad_z]
  module

theorem torus_n (f : Frame) (R r u v : ℝ) :
    (torus f R r u v).n = (r * (R + r * Real.cos v)) • (Real.cos v • f.rad u + Real.sin v • f.Z) := by
  simp only [Deriv.n, torus, smul_cross, cross_add, cross_smul, f.drad_rad, f.drad_z]
  module

/-! ## The area integrands (mass.rs:238, 1983–1993) -/

theorem dot_smul_comb (f : Frame) (s u a b : ℝ) :
    (s • (a • f.rad u + b • f.Z)) ⬝ᵥ (s • (a • f.rad u + b • f.Z)) = s ^ 2 * (a ^ 2 + b ^ 2) := by
  rw [smul_dotProduct, dotProduct_smul, f.dot_comb, smul_eq_mul, smul_eq_mul]; ring

/-- `|n|` of a vector: what `n.length()` gives. -/
def len (n : V3) : ℝ := Real.sqrt (n ⬝ᵥ n)

theorem plane_area (f : Frame) (u v : ℝ) : len (plane f u v).n = 1 := by
  rw [len, plane_n, f.zz, Real.sqrt_one]

theorem cylinder_area (f : Frame) (r u v : ℝ) (hr : 0 ≤ r) : len (cylinder f r u v).n = r := by
  rw [len, cylinder_n, smul_dotProduct, dotProduct_smul, f.dot_rad_rad, smul_eq_mul, smul_eq_mul,
    mul_one, Real.sqrt_mul_self hr]

/-- On a cone the area integrand is ρ·√(1+k²), linear in v where ρ = r + v·k ≥ 0. -/
theorem cone_area (f : Frame) (r k u v : ℝ) (hρ : 0 ≤ r + v * k) :
    len (cone f r k u v).n = (r + v * k) * Real.sqrt (1 + k ^ 2) := by
  have e : f.rad u + (-k) • f.Z = (1 : ℝ) • f.rad u + (-k) • f.Z := by rw [one_smul]
  rw [len, cone_n, e, dot_smul_comb, Real.sqrt_mul (sq_nonneg _), Real.sqrt_sq hρ]
  congr 2; ring

/-- The sphere's signed area integrand `n · (p − o) / r` (mass.rs:1990–1993) is r² cos v… -/
theorem sphere_signed (f : Frame) (r u v : ℝ) (hr : r ≠ 0) :
    (sphere f r u v).n ⬝ᵥ ((sphere f r u v).p - f.o) / r = r ^ 2 * Real.cos v := by
  rw [sphere_n]
  have e : (sphere f r u v).p - f.o = r • (Real.cos v • f.rad u + Real.sin v • f.Z) := by
    simp only [sphere]; module
  rw [e, smul_dotProduct, dotProduct_smul, f.dot_comb, smul_eq_mul, smul_eq_mul,
    Real.cos_sq_add_sin_sq]
  field_simp

/-- …which is |n| between the poles. -/
theorem sphere_area (f : Frame) (r u v : ℝ) (hr : 0 ≤ r) (hv : v ∈ Set.Icc (-(Real.pi / 2)) (Real.pi / 2)) :
    len (sphere f r u v).n = r ^ 2 * Real.cos v := by
  have hc : 0 ≤ Real.cos v := Real.cos_nonneg_of_mem_Icc hv
  rw [len, sphere_n, dot_smul_comb, Real.cos_sq_add_sin_sq, mul_one, Real.sqrt_sq]
  positivity

/-- A ring torus's area integrand is r·ρ, ρ = R + r cos v > 0. -/
theorem torus_area (f : Frame) (R r u v : ℝ) (hr : 0 ≤ r) (hρ : 0 ≤ R + r * Real.cos v) :
    len (torus f R r u v).n = r * (R + r * Real.cos v) := by
  rw [len, torus_n, dot_smul_comb, Real.cos_sq_add_sin_sq, mul_one, Real.sqrt_sq]
  positivity

/-- A spindle torus's signed area integrand `−n · dvv / r` (mass.rs:1983–1989) is r·ρ, which is
|n| on the part of the tube the face uses (ρ ≥ 0). -/
theorem torus_signed (f : Frame) (R r u v : ℝ) (hr : r ≠ 0) :
    -((torus f R r u v).n ⬝ᵥ torusDvv f r u v) / r = r * (R + r * Real.cos v) := by
  rw [torus_n, torusDvv]
  have e : (-r * Real.cos v) • f.rad u + (-r * Real.sin v) • f.Z =
      (-r) • (Real.cos v • f.rad u + Real.sin v • f.Z) := by module
  rw [e, smul_dotProduct, dotProduct_smul, f.dot_comb, smul_eq_mul, smul_eq_mul,
    Real.cos_sq_add_sin_sq]
  field_simp

end ForgeMass
end
