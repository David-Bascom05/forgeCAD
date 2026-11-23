import ForgeMass.Surfaces

/-!
# Frames as rigid motions (kernel/forge-math/src/frame.rs, at 8250ebb)

A `Frame` (an orthonormal, right-handed frame, ForgeMass.Surfaces) is the rigid motion from its
local coordinates to world.
* `world`, `local` (`point_to_world`, `point_to_local`, frame.rs:75–83);
* `vw`, `vl` (`vec_to_world`, `vec_to_local`, 66–73);
* `compose` (89–96) and `inverse` (98–110).

Proven over the reals:
* `vw` keeps dot and cross products: a rotation, orientation kept;
* `compose` is a frame, and its motion is the composite;
* `local` undoes `world`, both ways;
* `inverse` is a frame whose motion is `local`.

So rigid motions keep distances, angles, normals' orientation and, by
`vol_integrand_rigid`, the volume integrand at corresponding points (mass.rs:227).
-/

noncomputable section
open Matrix

namespace ForgeMass
namespace Frame
variable (f : Frame)

def vw (v : V3) : V3 := v 0 • f.X + v 1 • f.Y + v 2 • f.Z
def vl (v : V3) : V3 := ![v ⬝ᵥ f.X, v ⬝ᵥ f.Y, v ⬝ᵥ f.Z]
def world (p : V3) : V3 := f.o + f.vw p
def «local» (q : V3) : V3 := f.vl (q - f.o)

theorem dX : f.X ⬝ᵥ f.Y = 0 := f.xy0
theorem dYX : f.Y ⬝ᵥ f.X = 0 := by rw [dotProduct_comm]; exact f.xy0
theorem dZY : f.Z ⬝ᵥ f.Y = 0 := by rw [dotProduct_comm]; exact f.yz0
theorem dXZ : f.X ⬝ᵥ f.Z = 0 := by rw [dotProduct_comm]; exact f.zx0

/-- The frame's axes span: every vector is the sum of its components along them. -/
theorem complete (v : V3) : v = (v ⬝ᵥ f.X) • f.X + (v ⬝ᵥ f.Y) • f.Y + (v ⬝ᵥ f.Z) • f.Z := by
  set w := v - ((v ⬝ᵥ f.X) • f.X + (v ⬝ᵥ f.Y) • f.Y + (v ⬝ᵥ f.Z) • f.Z)
  have wx : w ⬝ᵥ f.X = 0 := by
    simp only [w, sub_dotProduct, add_dotProduct, smul_dotProduct, smul_eq_mul, f.xx, f.dYX,
      dotProduct_comm f.Z f.X, f.dXZ]; ring
  have wy : w ⬝ᵥ f.Y = 0 := by
    simp only [w, sub_dotProduct, add_dotProduct, smul_dotProduct, smul_eq_mul, f.yy, f.xy0,
      f.dZY]; ring
  have wz : w ⬝ᵥ f.Z = 0 := by
    simp only [w, sub_dotProduct, add_dotProduct, smul_dotProduct, smul_eq_mul, f.zz, f.yz0,
      f.dXZ]; ring
  -- (X × Y) × w = (X·w) Y − (Y·w) X = 0, so Z × w = 0, and Z × (Z × w) = (Z·w) Z − w
  have h1 : f.Z ⨯₃ w = 0 := by
    rw [← f.xy, cross_cross_eq_smul_sub_smul, dotProduct_comm f.X, dotProduct_comm f.Y, wx, wy]
    simp
  have h2 : f.Z ⨯₃ (f.Z ⨯₃ w) = -w := by
    rw [cross_cross_eq_smul_sub_smul', dotProduct_comm f.Z w, wz, f.zz]; simp
  have : w = 0 := by rw [h1, map_zero] at h2; exact (neg_eq_zero.1 h2.symm)
  exact (sub_eq_zero.1 this)

theorem vw_dot (a b : V3) : f.vw a ⬝ᵥ f.vw b = a ⬝ᵥ b := by
  simp only [vw, add_dotProduct, dotProduct_add, smul_dotProduct, dotProduct_smul, smul_eq_mul,
    f.xx, f.yy, f.zz, f.xy0, f.yz0, f.dYX, f.dZY, f.dXZ, dotProduct_comm f.Z f.X, f.zx0]
  simp [dotProduct, Fin.sum_univ_three]; ring

theorem vw_cross (a b : V3) : f.vw a ⨯₃ f.vw b = f.vw (a ⨯₃ b) := by
  simp only [vw, cross_add, add_cross, cross_smul, smul_cross, cross_self, f.xy, f.yz, f.zx,
    f.yx, f.zy, f.xz, smul_zero, add_zero, zero_add]
  simp [cross_apply]
  module

theorem vw_add (a b : V3) : f.vw (a + b) = f.vw a + f.vw b := by
  simp only [vw, Pi.add_apply, add_smul]; abel

theorem vw_smul (c : ℝ) (a : V3) : f.vw (c • a) = c • f.vw a := by
  simp only [vw, Pi.smul_apply, smul_eq_mul, smul_add, mul_smul]

theorem vw_sub (a b : V3) : f.vw (a - b) = f.vw a - f.vw b := by
  simp only [vw, Pi.sub_apply, sub_smul]; abel

theorem vl_vw (v : V3) : f.vl (f.vw v) = v := by
  ext i
  fin_cases i <;> simp only [vl, vw, add_dotProduct, smul_dotProduct, smul_eq_mul, f.xx, f.yy,
    f.zz, f.xy0, f.yz0, f.dYX, f.dZY, f.dXZ, dotProduct_comm f.Z f.X, f.zx0] <;> simp

theorem vw_vl (v : V3) : f.vw (f.vl v) = v := by
  conv_rhs => rw [f.complete v]
  simp [vw, vl]

theorem local_world (p : V3) : f.local (f.world p) = p := by
  simp only [«local», world, add_sub_cancel_left, vl_vw]

theorem world_local (q : V3) : f.world (f.local q) = q := by
  simp only [«local», world, vw_vl, add_sub_cancel]

/-- frame.rs:89–96: `self` applied after `inner`. -/
def compose (g : Frame) : Frame where
  o := f.world g.o
  X := f.vw g.X
  Y := f.vw g.Y
  Z := f.vw g.Z
  xy := by rw [vw_cross, g.xy]
  yz := by rw [vw_cross, g.yz]
  zx := by rw [vw_cross, g.zx]
  xx := by rw [vw_dot, g.xx]
  yy := by rw [vw_dot, g.yy]
  zz := by rw [vw_dot, g.zz]
  xy0 := by rw [vw_dot, g.xy0]
  yz0 := by rw [vw_dot, g.yz0]
  zx0 := by rw [vw_dot, g.zx0]

theorem compose_vw (g : Frame) (v : V3) : (f.compose g).vw v = f.vw (g.vw v) := by
  simp only [vw, compose, Pi.add_apply, Pi.smul_apply, smul_eq_mul]
  module

theorem compose_world (g : Frame) (p : V3) : (f.compose g).world p = f.world (g.world p) := by
  simp only [world, compose_vw, vw_add]; simp only [compose, world]; abel

/-- `vl` keeps dot and cross too: it is `vw`'s inverse. -/
theorem vl_dot (a b : V3) : f.vl a ⬝ᵥ f.vl b = a ⬝ᵥ b := by
  conv_rhs => rw [← f.vw_vl a, ← f.vw_vl b]
  rw [vw_dot]

theorem vl_cross (a b : V3) : f.vl a ⨯₃ f.vl b = f.vl (a ⨯₃ b) := by
  conv_rhs => rw [← f.vw_vl a, ← f.vw_vl b, vw_cross, vl_vw]

/-- frame.rs:98–110: the transpose's columns, and the origin taken back. -/
def inverse : Frame where
  o := f.vl (-f.o)
  X := f.vl ![1, 0, 0]
  Y := f.vl ![0, 1, 0]
  Z := f.vl ![0, 0, 1]
  xy := by rw [vl_cross]; congr 1; ext i; fin_cases i <;> simp [cross_apply]
  yz := by rw [vl_cross]; congr 1; ext i; fin_cases i <;> simp [cross_apply]
  zx := by rw [vl_cross]; congr 1; ext i; fin_cases i <;> simp [cross_apply]
  xx := by rw [vl_dot]; simp [dotProduct, Fin.sum_univ_three]
  yy := by rw [vl_dot]; simp [dotProduct, Fin.sum_univ_three]
  zz := by rw [vl_dot]; simp [dotProduct, Fin.sum_univ_three]
  xy0 := by rw [vl_dot]; simp [dotProduct, Fin.sum_univ_three]
  yz0 := by rw [vl_dot]; simp [dotProduct, Fin.sum_univ_three]
  zx0 := by rw [vl_dot]; simp [dotProduct, Fin.sum_univ_three]

/-- frame.rs:99–101 build the inverse's axes from the frame's components:
`ex = (x.x, y.x, z.x)`, which is `vl` of the first unit vector. -/
theorem inverse_X : f.inverse.X = ![f.X 0, f.Y 0, f.Z 0] := by
  ext i; fin_cases i <;> simp [inverse, vl, dotProduct, Fin.sum_univ_three]

/-- The inverse's motion is `local`. -/
theorem inverse_world (p : V3) : f.inverse.world p = f.local p := by
  have e : f.inverse.vw p = f.vl p := by
    have : p = p 0 • ![1, 0, 0] + p 1 • ![0, 1, 0] + p 2 • ![(0 : ℝ), 0, 1] := by
      ext i; fin_cases i <;> simp
    conv_rhs => rw [this]
    simp only [vw, inverse, vl, dotProduct_add, add_dotProduct, smul_dotProduct]
    ext i; fin_cases i <;> simp [smul_eq_mul]
  simp only [world, e, «local»]
  ext i; fin_cases i <;> simp [vl, inverse, sub_dotProduct, neg_dotProduct] <;> ring

theorem inverse_world_world (p : V3) : f.inverse.world (f.world p) = p := by
  rw [inverse_world, local_world]

theorem world_inverse_world (q : V3) : f.world (f.inverse.world q) = q := by
  rw [inverse_world, world_local]

/-- The volume integrand (p − c)·n at corresponding points is the same after a rigid motion: the
moments' centre `c` (the vertex centroid, mass.rs:113–117) moves with the body, and the normal
du × dv turns with it. -/
theorem vol_integrand_rigid (p c du dv : V3) :
    (f.world p - f.world c) ⬝ᵥ (f.vw du ⨯₃ f.vw dv) = (p - c) ⬝ᵥ (du ⨯₃ dv) := by
  have : f.world p - f.world c = f.vw (p - c) := by simp only [world, vw_sub]; abel
  rw [this, vw_cross, vw_dot]

end Frame
end ForgeMass
end
