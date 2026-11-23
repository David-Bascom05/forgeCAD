import ForgeTransform.Rigid

/-!
# Mirrors (kernel/forge-model/src/pattern.rs, at 8250ebb)

`Motion::Mirror(m)` reflects in the xy plane of frame m: a point's local z is negated
(pattern.rs:103–121). Proven:
* `mv`, its linear part, is the reflection v − 2(v·Z)Z: it keeps dot products and is its own
  inverse, and it turns cross products back, `mv a × mv b = −mv (a × b)`;
* `Motion::frame` (pattern.rs:126–138) gives a right-handed frame either way: (Mx, −My, Mz)
  keeping z, (Mx, My, −Mz) turning it back;
* a surface mirrored as `surface` does it (frame keeping z, pattern.rs:320–347) is the
  reflection of the old one with u or v negated, and its natural normal is the reflection of the
  old normal there. So a face keeps its sense, as pattern.rs:318 says;
* a curve mirrored as `curve` does it (frame turning z back, pattern.rs:289–304) is the
  reflection of the old one at the same parameter. So an edge keeps its range and its ends;
* `moved` walks each loop backwards with every sense flipped (pattern.rs:401–403). That keeps
  the face on the left of each coedge (`left_kept`);
* the volume integrand is the same at corresponding points.
-/

noncomputable section
open Matrix

namespace ForgeMass

def flipz (v : V3) : V3 := ![v 0, v 1, -v 2]

theorem cross_neg (u v : V3) : u ⨯₃ (-v) = -(u ⨯₃ v) := map_neg _ _

namespace Frame
variable (m : Frame)

/-- pattern.rs:113–121 -/
def mv (v : V3) : V3 := m.vw (flipz (m.vl v))
/-- pattern.rs:103–111 -/
def mp (p : V3) : V3 := m.world (flipz (m.local p))

theorem mv_eq (v : V3) : m.mv v = v - (2 * (v ⬝ᵥ m.Z)) • m.Z := by
  have hc := m.complete v
  have h1 : m.mv v = (v ⬝ᵥ m.X) • m.X + (v ⬝ᵥ m.Y) • m.Y + (-(v ⬝ᵥ m.Z)) • m.Z := by
    simp [mv, vw, vl, flipz]
  have h2 : v - (2 * (v ⬝ᵥ m.Z)) • m.Z =
      ((v ⬝ᵥ m.X) • m.X + (v ⬝ᵥ m.Y) • m.Y + (v ⬝ᵥ m.Z) • m.Z) - (2 * (v ⬝ᵥ m.Z)) • m.Z := by
    rw [← hc]
  rw [h1, h2]
  module

theorem mv_dot (a b : V3) : m.mv a ⬝ᵥ m.mv b = a ⬝ᵥ b := by
  rw [mv_eq, mv_eq]
  simp only [sub_dotProduct, dotProduct_sub, smul_dotProduct, dotProduct_smul, smul_eq_mul, m.zz,
    dotProduct_comm m.Z a, dotProduct_comm m.Z b]
  ring

theorem mv_mv (v : V3) : m.mv (m.mv v) = v := by
  rw [mv_eq, mv_eq]
  simp only [sub_dotProduct, smul_dotProduct, smul_eq_mul, m.zz]
  module

theorem mv_add (a b : V3) : m.mv (a + b) = m.mv a + m.mv b := by
  simp only [mv_eq, add_dotProduct]; module

theorem mv_smul (c : ℝ) (a : V3) : m.mv (c • a) = c • m.mv a := by
  simp only [mv_eq, smul_dotProduct, smul_eq_mul]; module

theorem mv_neg (a : V3) : m.mv (-a) = -m.mv a := by
  simp only [mv_eq, neg_dotProduct]; module

/-- A reflection turns cross products back. -/
theorem mv_cross (a b : V3) : m.mv a ⨯₃ m.mv b = -m.mv (a ⨯₃ b) := by
  -- in the mirror's own frame, the reflection is flipz
  have e : ∀ v, m.mv v = m.vw (flipz (m.vl v)) := fun v => rfl
  rw [e, e, e, vw_cross, ← vw_cross m, ← m.vw_vl (a ⨯₃ b), ← vl_cross]
  rw [show -m.vw (flipz (m.vl (m.vw (m.vl a ⨯₃ m.vl b)))) = m.vw (-flipz (m.vl a ⨯₃ m.vl b)) by
    rw [vl_vw, ← neg_one_smul ℝ, ← vw_smul, neg_one_smul]]
  rw [vw_cross]
  congr 1
  ext i; fin_cases i <;> simp [flipz, cross_apply] <;> ring

theorem mp_sub (p q : V3) : m.mp p - m.mp q = m.mv (p - q) := by
  simp only [mp, mv, world, «local», vl, vw]
  ext i
  simp only [flipz, Pi.add_apply, Pi.sub_apply, Pi.smul_apply, smul_eq_mul, sub_dotProduct]
  simp [Fin.isValue]
  ring

theorem mp_add (p v : V3) : m.mp (p + v) = m.mp p + m.mv v := by
  have := m.mp_sub (p + v) p
  rw [add_sub_cancel_left] at this
  rw [← this]; abel

theorem mp_mp (p : V3) : m.mp (m.mp p) = p := by
  have h := m.mp_add (m.mp m.o) (p - m.o)
  have h0 : m.mp m.o = m.o := by
    simp [mp, world, «local», vl, vw, flipz]
  rw [show m.mp p = m.mp (m.o + (p - m.o)) by rw [add_sub_cancel], m.mp_add, h0, m.mp_add, h0,
    mv_mv]
  abel

/-- pattern.rs:126–138 with `keep_z`: the image frame (Mx, −My, Mz) (from_zx of Mz and Mx, which
are already unit and perpendicular). -/
def keepZ (f : Frame) : Frame where
  o := m.mp f.o
  X := m.mv f.X
  Y := -m.mv f.Y
  Z := m.mv f.Z
  xy := by rw [cross_neg, mv_cross, f.xy, neg_neg]
  yz := by rw [neg_cross, mv_cross, f.yz, neg_neg]
  zx := by rw [mv_cross, f.zx]
  xx := by rw [mv_dot, f.xx]
  yy := by rw [neg_dotProduct, dotProduct_neg, neg_neg, mv_dot, f.yy]
  zz := by rw [mv_dot, f.zz]
  xy0 := by rw [dotProduct_neg, mv_dot, f.xy0, neg_zero]
  yz0 := by rw [neg_dotProduct, mv_dot, f.yz0, neg_zero]
  zx0 := by rw [mv_dot, f.zx0]

/-- pattern.rs:126–138 without `keep_z`: (Mx, My, −Mz). -/
def turnZ (f : Frame) : Frame where
  o := m.mp f.o
  X := m.mv f.X
  Y := m.mv f.Y
  Z := -m.mv f.Z
  xy := by rw [mv_cross, f.xy]
  yz := by rw [cross_neg, mv_cross, f.yz, neg_neg]
  zx := by rw [neg_cross, mv_cross, f.zx, neg_neg]
  xx := by rw [mv_dot, f.xx]
  yy := by rw [mv_dot, f.yy]
  zz := by rw [neg_dotProduct, dotProduct_neg, neg_neg, mv_dot, f.zz]
  xy0 := by rw [mv_dot, f.xy0]
  yz0 := by rw [dotProduct_neg, mv_dot, f.yz0, neg_zero]
  zx0 := by rw [neg_dotProduct, mv_dot, f.zx0, neg_zero]

/-- `keep_z` frames: the image of rad at −u. -/
theorem keepZ_rad (f : Frame) (u : ℝ) : (m.keepZ f).rad u = m.mv (f.rad (-u)) := by
  simp only [Frame.rad, keepZ, mv_add, mv_smul, mv_neg, Real.cos_neg, Real.sin_neg, smul_neg, neg_smul]

/-- `keep_z` frames on a plane: the image of the point at (u, −v). -/
theorem keepZ_plane (f : Frame) (u v : ℝ) :
    (m.keepZ f).o + u • (m.keepZ f).X + v • (m.keepZ f).Y = m.mp (f.o + u • f.X + (-v) • f.Y) := by
  simp only [keepZ, mp_add, mv_smul, smul_neg, neg_smul, neg_neg, mv_neg]

end Frame

/-! ## Surfaces and curves under a mirror -/

section
variable (m f : Frame)

theorem mirror_plane (u v : ℝ) :
    (plane (m.keepZ f) u v).p = m.mp (plane f u (-v)).p ∧
      (plane (m.keepZ f) u v).n = m.mv (plane f u (-v)).n := by
  refine ⟨by simp only [plane]; exact m.keepZ_plane f u v, ?_⟩
  rw [plane_n, plane_n]; rfl

theorem mirror_cylinder (r u v : ℝ) :
    (cylinder (m.keepZ f) r u v).p = m.mp (cylinder f r (-u) v).p ∧
      (cylinder (m.keepZ f) r u v).n = m.mv (cylinder f r (-u) v).n := by
  constructor
  · simp only [cylinder, Frame.keepZ_rad, m.mp_add, Frame.mv_smul]; rfl
  · rw [cylinder_n, cylinder_n, Frame.keepZ_rad, Frame.mv_smul]

theorem mirror_cone (r k u v : ℝ) :
    (cone (m.keepZ f) r k u v).p = m.mp (cone f r k (-u) v).p ∧
      (cone (m.keepZ f) r k u v).n = m.mv (cone f r k (-u) v).n := by
  constructor
  · simp only [cone, Frame.keepZ_rad, m.mp_add, Frame.mv_smul]; rfl
  · rw [cone_n, cone_n, Frame.keepZ_rad, Frame.mv_smul, Frame.mv_add, Frame.mv_smul]; rfl

theorem mirror_sphere (r u v : ℝ) :
    (sphere (m.keepZ f) r u v).p = m.mp (sphere f r (-u) v).p ∧
      (sphere (m.keepZ f) r u v).n = m.mv (sphere f r (-u) v).n := by
  constructor
  · simp only [sphere, Frame.keepZ_rad, m.mp_add, Frame.mv_smul]; rfl
  · rw [sphere_n, sphere_n, Frame.keepZ_rad, Frame.mv_smul, Frame.mv_add, Frame.mv_smul,
      Frame.mv_smul]; rfl

theorem mirror_torus (R r u v : ℝ) :
    (torus (m.keepZ f) R r u v).p = m.mp (torus f R r (-u) v).p ∧
      (torus (m.keepZ f) R r u v).n = m.mv (torus f R r (-u) v).n := by
  constructor
  · simp only [torus, Frame.keepZ_rad, m.mp_add, Frame.mv_smul]; rfl
  · rw [torus_n, torus_n, Frame.keepZ_rad, Frame.mv_smul, Frame.mv_add, Frame.mv_smul,
      Frame.mv_smul]; rfl

/-- A circle `o + r(cos t x + sin t y)` mirrored with its frame's z turned back: the image of
the old circle at the same t. -/
theorem mirror_circle (r t : ℝ) :
    (m.turnZ f).o + (r * Real.cos t) • (m.turnZ f).X + (r * Real.sin t) • (m.turnZ f).Y =
      m.mp (f.o + (r * Real.cos t) • f.X + (r * Real.sin t) • f.Y) := by
  simp only [Frame.turnZ, m.mp_add, Frame.mv_smul]

/-- A line `origin + t dir` mirrored (pattern.rs:285–288): the image at the same t. -/
theorem mirror_line (o d : V3) (t : ℝ) : m.mp o + t • m.mv d = m.mp (o + t • d) := by
  rw [m.mp_add, Frame.mv_smul]

/-- Walking the loop backwards: a coedge's tangent t becomes −M t, the face's normal n becomes
M n, and the side the face lies on, n × t, goes to its image. So the face stays on the left. -/
theorem left_kept (n t : V3) : m.mv n ⨯₃ (-m.mv t) = m.mv (n ⨯₃ t) := by
  rw [cross_neg, Frame.mv_cross, neg_neg]

/-- The volume integrand (p − c)·n is the same at corresponding points of the mirrored body. -/
theorem vol_integrand_mirror (p c n : V3) : (m.mp p - m.mp c) ⬝ᵥ m.mv n = (p - c) ⬝ᵥ n := by
  rw [Frame.mp_sub, Frame.mv_dot]

end

end ForgeMass
end
