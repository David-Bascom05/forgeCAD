import ForgeCurved.QTransverse
import ForgeExact.QRef.Faces

/-!
# The per-kind sense rule beside F6: each face kind's quadric gradient

F6 (F6.lean) reads a face's inside as `cf f < 0` or `cf f > 0` by its orientation `o_f`. The
kernel stores `o_f` exactly as `Face.same_sense`: whether the outward normal is the surface's
natural normal. For the canonical quadric `QFace.quadric` (negative inside the face's solid),
this file gives the gradient per kind, at `d = p − o`:
* **plane**: `∇q = z` (the oriented normal);
* **sphere**: `∇q = 2 d` (radial, outward);
* **cylinder**: `∇q = 2 (|z|² d − (d·z) z)`, which is `2|z|²` times `d`'s component across the axis
  (radial, outward);
* **cone**: `∇q = 2 c² d − 2 (c² + s²)(d·z) z − 2 r c s z`.

So the natural normal is a positive multiple of `∇q` for planes, spheres and cylinders (where
`∇q ≠ 0`). The inside is `q < 0` on the side the normal leaves, and `o_f` follows from
`same_sense` by a fixed rule per kind (`centred_grad` is the common step).
-/

namespace ForgeCurved

open ForgeExact ForgeExact.Ref ForgeExact.Quad ForgeExact.QRef

/-- The gradient of a centred quadric. -/
theorem centred_grad (sxx syy szz sxy sxz syz : ℚ) (L : V3) (k : ℚ) (o : V3) (p : R3) :
    let q := centred sxx syy szz sxy sxz syz L k o
    gx q p = 2 * sxx * (p.1 - o.x) + sxy * (p.2.1 - o.y) + sxz * (p.2.2 - o.z) + L.x ∧
    gy q p = sxy * (p.1 - o.x) + 2 * syy * (p.2.1 - o.y) + syz * (p.2.2 - o.z) + L.y ∧
    gz q p = sxz * (p.1 - o.x) + syz * (p.2.1 - o.y) + 2 * szz * (p.2.2 - o.z) + L.z := by
  simp only [centred, gx, gy, gz]
  push_cast
  refine ⟨by ring, by ring, by ring⟩

variable (f : QFace) (p : R3)


theorem grad_plane (h : f.kind = .plane) :
    gx f.quadric p = f.z.x ∧ gy f.quadric p = f.z.y ∧ gz f.quadric p = f.z.z := by
  obtain ⟨h1, h2, h3⟩ := centred_grad 0 0 0 0 0 0 f.z 0 f.o p
  simp only [QFace.quadric, h]
  push_cast at h1 h2 h3
  refine ⟨by rw [h1]; ring, by rw [h2]; ring, by rw [h3]; ring⟩

theorem grad_sphere (h : f.kind = .sphere) :
    gx f.quadric p = 2 * (p.1 - (f.o.x : ℝ)) ∧ gy f.quadric p = 2 * (p.2.1 - (f.o.y : ℝ)) ∧ gz f.quadric p = 2 * (p.2.2 - (f.o.z : ℝ)) := by
  obtain ⟨h1, h2, h3⟩ := centred_grad 1 1 1 0 0 0 ⟨0, 0, 0⟩ (-(f.r * f.r)) f.o p
  simp only [QFace.quadric, h]
  push_cast at h1 h2 h3
  refine ⟨by rw [h1]; ring, by rw [h2]; ring, by rw [h3]; ring⟩

theorem grad_cyl (h : f.kind = .cyl) :
    let n : ℝ := (f.z.x : ℝ) ^ 2 + (f.z.y : ℝ) ^ 2 + (f.z.z : ℝ) ^ 2
    let DZ : ℝ := (p.1 - (f.o.x : ℝ)) * f.z.x + (p.2.1 - (f.o.y : ℝ)) * f.z.y + (p.2.2 - (f.o.z : ℝ)) * f.z.z
    gx f.quadric p = 2 * (n * (p.1 - (f.o.x : ℝ)) - DZ * f.z.x) ∧ gy f.quadric p = 2 * (n * (p.2.1 - (f.o.y : ℝ)) - DZ * f.z.y) ∧
      gz f.quadric p = 2 * (n * (p.2.2 - (f.o.z : ℝ)) - DZ * f.z.z) := by
  intro n DZ
  have e := centred_grad (f.z.norm2 - f.z.x * f.z.x) (f.z.norm2 - f.z.y * f.z.y)
    (f.z.norm2 - f.z.z * f.z.z) (-2 * f.z.x * f.z.y) (-2 * f.z.x * f.z.z) (-2 * f.z.y * f.z.z)
    ⟨0, 0, 0⟩ (-(f.r * f.r * f.z.norm2)) f.o p
  obtain ⟨h1, h2, h3⟩ := e
  simp only [QFace.quadric, h, V3.norm2, V3.dot]
  simp only [V3.norm2, V3.dot] at h1 h2 h3
  push_cast at h1 h2 h3
  refine ⟨by rw [h1]; simp only [n, DZ]; ring, by rw [h2]; simp only [n, DZ]; ring,
    by rw [h3]; simp only [n, DZ]; ring⟩

theorem grad_cone (h : f.kind = .cone) :
    let c2 : ℝ := (f.c : ℝ) ^ 2
    let k : ℝ := (f.c : ℝ) ^ 2 + (f.s : ℝ) ^ 2
    let a : ℝ := 2 * f.r * f.c * f.s
    let DZ : ℝ := (p.1 - (f.o.x : ℝ)) * f.z.x + (p.2.1 - (f.o.y : ℝ)) * f.z.y + (p.2.2 - (f.o.z : ℝ)) * f.z.z
    gx f.quadric p = 2 * c2 * (p.1 - (f.o.x : ℝ)) - 2 * k * DZ * f.z.x - a * f.z.x ∧
    gy f.quadric p = 2 * c2 * (p.2.1 - (f.o.y : ℝ)) - 2 * k * DZ * f.z.y - a * f.z.y ∧
    gz f.quadric p = 2 * c2 * (p.2.2 - (f.o.z : ℝ)) - 2 * k * DZ * f.z.z - a * f.z.z := by
  intro c2 k a DZ
  obtain ⟨h1, h2, h3⟩ := centred_grad (f.c * f.c - (f.c * f.c + f.s * f.s) * f.z.x * f.z.x)
    (f.c * f.c - (f.c * f.c + f.s * f.s) * f.z.y * f.z.y)
    (f.c * f.c - (f.c * f.c + f.s * f.s) * f.z.z * f.z.z)
    (-2 * (f.c * f.c + f.s * f.s) * f.z.x * f.z.y) (-2 * (f.c * f.c + f.s * f.s) * f.z.x * f.z.z)
    (-2 * (f.c * f.c + f.s * f.s) * f.z.y * f.z.z) ((-2 * f.r * f.c * f.s) * f.z)
    (-(f.r * f.r * (f.c * f.c))) f.o p
  have sx : ∀ (t : ℚ) (v : V3), (t * v).x = t * v.x := fun _ _ => rfl
  have sy : ∀ (t : ℚ) (v : V3), (t * v).y = t * v.y := fun _ _ => rfl
  have sz : ∀ (t : ℚ) (v : V3), (t * v).z = t * v.z := fun _ _ => rfl
  simp only [QFace.quadric, h]
  simp only [sx, sy, sz] at h1 h2 h3
  push_cast at h1 h2 h3
  refine ⟨?_, ?_, ?_⟩
  · rw [h1]; simp only [c2, k, a, DZ]; ring
  · rw [h2]; simp only [c2, k, a, DZ]; ring
  · rw [h3]; simp only [c2, k, a, DZ]; ring

end ForgeCurved
