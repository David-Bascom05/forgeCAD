import ForgeTransform.Rigid

/-!
# Circular patterns (pattern.rs:247–273; bodies.rs:184–200 `Step::Rotate` alike)

Copy k of a circular pattern about the axis of frame `at` is
`at.rotated_about_z(step·k).compose(&at.inverse())` (frame.rs:113–126 for `rotated_about_z`).
Over the reals its axes are (rad a, drad a, z): `normalize` changes nothing there.
* `turn_turn`: these turns compose by adding angles, so copy j of copy k is copy j + k;
* `turn_axis`: they fix the axis;
* each is a rigid motion (a `Frame`, Rigid.lean), so it keeps distances, orientation and the
  volume integrand.
-/

noncomputable section
open Matrix

namespace ForgeMass
namespace Frame
variable (f : Frame)

theorem dot_drad_drad (u : ℝ) : f.drad u ⬝ᵥ f.drad u = 1 := by
  simp only [drad, add_dotProduct, dotProduct_add, smul_dotProduct, dotProduct_smul, smul_eq_mul,
    f.xx, f.yy, f.xy0, dotProduct_comm f.Y f.X]
  nlinarith [Real.sin_sq_add_cos_sq u]

theorem dot_rad_drad (u : ℝ) : f.rad u ⬝ᵥ f.drad u = 0 := by
  simp only [rad, drad, add_dotProduct, dotProduct_add, smul_dotProduct, dotProduct_smul,
    smul_eq_mul, f.xx, f.yy, f.xy0, dotProduct_comm f.Y f.X]
  ring

theorem dot_drad_z (u : ℝ) : f.drad u ⬝ᵥ f.Z = 0 := by
  simp only [drad, add_dotProduct, smul_dotProduct, smul_eq_mul, f.yz0, dotProduct_comm f.X f.Z,
    f.zx0]
  ring

/-- frame.rs:113–126: same origin and z, x turned by `a` about z. -/
def rot (a : ℝ) : Frame where
  o := f.o
  X := f.rad a
  Y := f.drad a
  Z := f.Z
  xy := by rw [← cross_anticomm, f.drad_rad, neg_neg]
  yz := f.drad_z a
  zx := by rw [← cross_anticomm, f.rad_z, neg_neg]
  xx := f.dot_rad_rad a
  yy := f.dot_drad_drad a
  zz := f.zz
  xy0 := f.dot_rad_drad a
  yz0 := f.dot_drad_z a
  zx0 := by rw [dotProduct_comm]; exact f.dot_rad_z a

/-- The kernel's y is z × x (frame.rs:119): the same vector. -/
theorem rot_y (a : ℝ) : f.Z ⨯₃ f.rad a = f.drad a := by
  rw [← cross_anticomm, f.rad_z, neg_neg]

/-- pattern.rs:268: copy k turned by `a` about the frame's z axis. -/
def turn (a : ℝ) : Frame := (f.rot a).compose f.inverse

def Rz (b : ℝ) (w : V3) : V3 :=
  ![Real.cos b * w 0 - Real.sin b * w 1, Real.sin b * w 0 + Real.cos b * w 1, w 2]

theorem turn_world (a : ℝ) (p : V3) : (f.turn a).world p = (f.rot a).world (f.local p) := by
  rw [turn, compose_world, inverse_world]

theorem local_rot_world (b : ℝ) (w : V3) : f.local ((f.rot b).world w) = Rz b w := by
  simp only [«local», world, rot, vw, vl, rad, drad, add_sub_cancel_left]
  ext i
  fin_cases i <;>
  · simp only [Rz, add_dotProduct, smul_dotProduct, smul_eq_mul, f.xx, f.yy, f.zz, f.xy0, f.yz0,
      f.dYX, f.dZY, f.dXZ, dotProduct_comm f.Z f.X, f.zx0]
    try simp
    try ring

theorem rot_world_Rz (a b : ℝ) (w : V3) : (f.rot a).world (Rz b w) = (f.rot (a + b)).world w := by
  simp only [world, rot, vw, rad, drad, Rz, Real.cos_add, Real.sin_add]
  simp
  module

/-- Copy j of copy k is copy j + k. -/
theorem turn_turn (a b : ℝ) (p : V3) :
    (f.turn a).world ((f.turn b).world p) = (f.turn (a + b)).world p := by
  rw [turn_world, turn_world, local_rot_world, rot_world_Rz, turn_world]

/-- A turn keeps its axis. -/
theorem turn_axis (a s : ℝ) : (f.turn a).world (f.o + s • f.Z) = f.o + s • f.Z := by
  rw [turn_world, show f.local (f.o + s • f.Z) = ![0, 0, s] by
    ext i; fin_cases i <;> simp [«local», vl, smul_dotProduct, f.zz, f.zx0, f.dZY]]
  simp [world, rot, vw]

/-- n copies at step 2π/n close up: copy n is the original. -/
theorem turn_full (n : ℕ) (hn : n ≠ 0) (p : V3) :
    (f.turn (n * (2 * Real.pi / n))).world p = p := by
  have : (n : ℝ) * (2 * Real.pi / n) = 2 * Real.pi := by field_simp
  rw [this, turn_world]
  have hr : f.rot (2 * Real.pi) = f.rot 0 := by
    simp only [rot, rad, drad, Real.cos_two_pi, Real.sin_two_pi, Real.cos_zero, Real.sin_zero]
  rw [hr]
  conv_rhs => rw [← f.world_local p]
  simp [world, rot, vw, rad, drad]

end Frame
end ForgeMass
end
