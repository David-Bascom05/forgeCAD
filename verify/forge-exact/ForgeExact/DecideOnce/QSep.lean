import ForgeExact.DecideOnce.Classes
import ForgeExact.QRef.Faces

/-!
# Q1 L2: the Tier-0 seams (design §2.3), exact and rational

Each curved face gets fixed separator planes, read off its stored frame exactly:

| kind | seams |
|---|---|
| plane | none |
| cylinder | `axisPlane o z`: through the axis `o + t z`, normal `z × e` with `e` the first coordinate axis not parallel to `z`; and `axisPlane2 o z`, normal `z × (z × e)`, orthogonal to it |
| cone | both axial planes, and the **nappe plane** `s z·(p − o) + r c = 0` (`QFace.nappe`'s zero set), through the apex |
| sphere | the plane `x = o.x` through the centre |
| torus | the axial plane, and the equatorial plane `z·(p − o) = 0` |

Theorems:
* `axisPlane_on`: the axial plane holds the whole axis, exactly;
* `axisPlane_nz`: its normal is nonzero for a nonzero axis;
* `axisPlane2_on`, `axisPlane2_nz`, `axisPlanes_orth`: the second axial plane (cylinder, cone)
  holds the axis, has a nonzero normal and is orthogonal to the first;
* `nappePlane_on`: the nappe plane is the nappe quadric's zero set, so it holds the apex;
* `centrePlane_on`, `equatorPlane_on`: they hold the centre;
* `seamsOf`, and `seams` over all faces: their classes (`classesOf`, Classes.lean) are distinct
  planes (`noScaled_seams`), so coaxial surfaces share one seam class.
-/

namespace ForgeExact.DecideOnce.Curved

open ForgeExact ForgeExact.Ref ForgeExact.Quad ForgeExact.QRef

/-- The first coordinate axis not parallel to `z`. -/
def offAxis (z : V3) : V3 :=
  if (z.cross ⟨1, 0, 0⟩).isZero = false then ⟨1, 0, 0⟩
  else if (z.cross ⟨0, 1, 0⟩).isZero = false then ⟨0, 1, 0⟩ else ⟨0, 0, 1⟩

/-- The plane through the axis `o + t z`. -/
def axisPlane (o z : V3) : Plane :=
  let n := z.cross (offAxis z)
  ⟨n, n.dot o⟩

theorem cross_dot_self (z e : V3) : (z.cross e).dot z = 0 := by
  simp only [V3.cross, V3.dot]; grind

/-- **The axial plane holds the axis.** -/
theorem axisPlane_on (o z : V3) (t : Rat) : (axisPlane o z).eval (o + t * z) = 0 := by
  unfold axisPlane
  simp only [Plane.eval]
  have h := cross_dot_self z (offAxis z)
  generalize z.cross (offAxis z) = n at h
  show V3.dot n (V3.add o (V3.smul t z)) - V3.dot n o = 0
  simp only [V3.dot, V3.add, V3.smul] at h ⊢
  grind

theorem axisPlane_nz {z : V3} (hz : z ≠ ⟨0, 0, 0⟩) (o : V3) : (axisPlane o z).n ≠ ⟨0, 0, 0⟩ := by
  unfold axisPlane offAxis
  simp only
  split
  · rename_i h; intro h0; rw [h0] at h; simp [V3.isZero] at h
  · split
    · rename_i _ h; intro h0; rw [h0] at h; simp [V3.isZero] at h
    · rename_i h1 h2
      simp only [Bool.not_eq_false, isZero_iff, V3.cross] at h1 h2
      intro h0
      simp only [V3.cross, V3.mk.injEq] at h0
      apply hz
      obtain ⟨a, b, c⟩ := z
      simp only [V3.mk.injEq]
      grind

/-- The second axial plane, orthogonal to the first: normal `z × (z × e)`. One axial seam leaves
a holed plate's room annular (tier0's `plate1_not_faithful`); two orthogonal ones cut it into
cells (`plate_faithful`). -/
def axisPlane2 (o z : V3) : Plane :=
  let n := z.cross (z.cross (offAxis z))
  ⟨n, n.dot o⟩

/-- **The second axial plane holds the axis.** -/
theorem axisPlane2_on (o z : V3) (t : Rat) : (axisPlane2 o z).eval (o + t * z) = 0 := by
  unfold axisPlane2
  simp only [Plane.eval]
  have h := cross_dot_self z (z.cross (offAxis z))
  generalize z.cross (z.cross (offAxis z)) = n at h
  show V3.dot n (V3.add o (V3.smul t z)) - V3.dot n o = 0
  simp only [V3.dot, V3.add, V3.smul] at h ⊢
  grind

/-- **The two axial planes are orthogonal.** -/
theorem axisPlanes_orth (o z : V3) : (axisPlane o z).n.dot (axisPlane2 o z).n = 0 := by
  simp only [axisPlane, axisPlane2]
  generalize offAxis z = e
  simp only [V3.cross, V3.dot]; grind

theorem sq_nonneg_rat (a : Rat) : 0 ≤ a * a := by
  rcases (Rat.le_total (a := 0) (b := a)) with h | h
  · exact Rat.mul_nonneg h h
  · have h' : 0 ≤ -a := by grind
    have := Rat.mul_nonneg h' h'
    grind

theorem sq_sum_zero {a b c : Rat} (h : a * a + b * b + c * c = 0) : a = 0 ∧ b = 0 ∧ c = 0 := by
  have ha := sq_nonneg_rat a
  have hb := sq_nonneg_rat b
  have hc := sq_nonneg_rat c
  have a0 : a * a = 0 := by grind
  have b0 : b * b = 0 := by grind
  have c0 : c * c = 0 := by grind
  exact ⟨(Rat.mul_eq_zero.mp a0).elim id id, (Rat.mul_eq_zero.mp b0).elim id id,
    (Rat.mul_eq_zero.mp c0).elim id id⟩

theorem axisPlane2_nz {z : V3} (hz : z ≠ ⟨0, 0, 0⟩) (o : V3) : (axisPlane2 o z).n ≠ ⟨0, 0, 0⟩ := by
  have hw := axisPlane_nz hz o
  simp only [axisPlane] at hw
  simp only [axisPlane2]
  generalize hwd : z.cross (offAxis z) = w at hw
  have hdot : z.dot w = 0 := by rw [← hwd]; simp only [V3.cross, V3.dot]; grind
  intro h0
  obtain ⟨a, b, c⟩ := z
  obtain ⟨p, q, r⟩ := w
  simp only [V3.cross, V3.dot, V3.mk.injEq] at h0 hdot
  -- Lagrange: |z × w|² = |z|²|w|² − (z·w)²
  have lag : (a * a + b * b + c * c) * (p * p + q * q + r * r) = 0 := by
    have e : (a * a + b * b + c * c) * (p * p + q * q + r * r) =
        (b * r - c * q) * (b * r - c * q) + (c * p - a * r) * (c * p - a * r) +
          (a * q - b * p) * (a * q - b * p) + (a * p + b * q + c * r) * (a * p + b * q + c * r) := by
      grind
    rw [e, h0.1, h0.2.1, h0.2.2, hdot]; grind
  rcases Rat.mul_eq_zero.mp lag with h | h
  · obtain ⟨h1, h2, h3⟩ := sq_sum_zero h; subst h1 h2 h3; exact hz rfl
  · obtain ⟨h1, h2, h3⟩ := sq_sum_zero h; subst h1 h2 h3; exact hw rfl

/-- The cone's nappe plane: `s z·(p − o) + r c = 0`. -/
def nappePlane (f : QFace) : Plane :=
  ⟨f.s * f.z, (f.s * f.z).dot f.o - f.r * f.c⟩

/-- **The nappe plane is the nappe quadric's zero set** (up to sign): it holds the apex. -/
theorem nappePlane_on (f : QFace) (hk : f.kind = .cone) (p : V3) :
    ∃ q, f.nappe = some q ∧ q.eval p = -((nappePlane f).eval p) := by
  refine ⟨centred 0 0 0 0 0 0 ((-f.s) * f.z) (-(f.r * f.c)) f.o, by simp [QFace.nappe, hk], ?_⟩
  simp only [centred, Quadric.eval, nappePlane, Plane.eval, V3.dot, Flat.smul_x, Flat.smul_y,
    Flat.smul_z]
  grind

/-- The plane `x = o.x` through a sphere's centre. -/
def centrePlane (o : V3) : Plane := ⟨⟨1, 0, 0⟩, o.x⟩

theorem centrePlane_on (o : V3) : (centrePlane o).eval o = 0 := by
  simp only [centrePlane, Plane.eval, V3.dot]; grind

/-- The torus's equatorial plane `z·(p − o) = 0`. -/
def equatorPlane (o z : V3) : Plane := ⟨z, z.dot o⟩

theorem equatorPlane_on (o z : V3) : (equatorPlane o z).eval o = 0 := by
  simp only [equatorPlane, Plane.eval]; grind

/-- A face's Tier-0 seams. -/
def seamsOf (f : QFace) : List Plane :=
  match f.kind with
  | .plane => []
  | .cyl => [axisPlane f.o f.z, axisPlane2 f.o f.z]
  | .cone => [axisPlane f.o f.z, axisPlane2 f.o f.z, nappePlane f]
  | .sphere => [centrePlane f.o]
  | .torus => [axisPlane f.o f.z, equatorPlane f.o f.z]

/-- The seams of all faces, one per point set. -/
def seams (fs : List QFace) : List Plane := classesOf (fs.flatMap seamsOf)

/-- A face's data is usable: nonzero axis for the axial kinds, and a cone's growth `s ≠ 0`. -/
def _root_.ForgeExact.QRef.QFace.WF (f : QFace) : Prop :=
  (f.kind ≠ .plane ∧ f.kind ≠ .sphere → f.z ≠ ⟨0, 0, 0⟩) ∧ (f.kind = .cone → f.s ≠ 0)

theorem seamsOf_nz {f : QFace} (hf : f.WF) : ∀ p ∈ seamsOf f, p.n ≠ ⟨0, 0, 0⟩ := by
  obtain ⟨hz, hs⟩ := hf
  unfold seamsOf
  cases hk : f.kind <;> simp only [List.mem_cons, false_imp_iff, forall_const,
    forall_eq_or_imp, forall_eq, List.not_mem_nil, or_false]
  · exact ⟨axisPlane_nz (hz (by simp [hk])) _, axisPlane2_nz (hz (by simp [hk])) _⟩
  · refine ⟨axisPlane_nz (hz (by simp [hk])) _, axisPlane2_nz (hz (by simp [hk])) _, ?_⟩
    have hz' := hz (by simp [hk])
    have hs' := hs hk
    simp only [nappePlane]
    intro h0
    change V3.smul f.s f.z = _ at h0
    simp only [V3.smul, V3.mk.injEq] at h0
    have hx := (Rat.mul_eq_zero.mp h0.1).resolve_left hs'
    have hy := (Rat.mul_eq_zero.mp h0.2.1).resolve_left hs'
    have hzz := (Rat.mul_eq_zero.mp h0.2.2).resolve_left hs'
    apply hz'
    revert hx hy hzz
    cases f.z
    simp only [V3.mk.injEq]
    exact fun a b c => ⟨a, b, c⟩
  · simp [centrePlane]
  · refine ⟨axisPlane_nz (hz (by simp [hk])) _, ?_⟩
    exact hz (by simp [hk])

/-- **The seam classes are distinct planes.** -/
theorem noScaled_seams {fs : List QFace} (hf : ∀ f ∈ fs, f.WF) : NoScaled (seams fs) := by
  apply noScaled_classesOf
  intro p hp
  obtain ⟨f, hfm, hp⟩ := List.mem_flatMap.mp hp
  exact seamsOf_nz (hf f hfm) p hp

end ForgeExact.DecideOnce.Curved
