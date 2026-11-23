import ForgeSheetCore.Funs
import ForgeSheetCore.ZoneErr
import ForgeMath
import IEEE754.Dmath.SinCos
open Aeneas Aeneas.Std Result

set_option exponentiation.threshold 1100

namespace forge_sheet_core

open IEEE754 ForgeSheetCore.Zone

/-- `u = 2⁻⁵³` and `η = 2⁻¹⁰⁷⁵` as reals. -/
noncomputable abbrev uZ : ℝ := 2 ^ (-53 : ℤ)
noncomputable abbrev hZ : ℝ := 2 ^ (-1075 : ℤ)

theorem toR {x r : ℚ} (h : |x - r| ≤ F64.Ops.u * |r| + F64.Ops.η) :
    |(x : ℝ) - r| ≤ uZ * |(r : ℝ)| + hZ := by
  have := (Rat.cast_le (K := ℝ)).mpr h
  push_cast [F64.Ops.u, F64.Ops.η] at this; exact this

theorem addR {a b : IEEE754.F64} (h : (FP.add a b).isFinite) :
    |((FP.add a b).val : ℝ) - ((a.val : ℝ) + b.val)| ≤ uZ * |(a.val : ℝ) + b.val| + hZ := by
  obtain ⟨ha, hb⟩ := isFinite_of_add h
  have := toR (F64.Ops.add_err ha hb (by simpa [F64.Ops.add_def] using h))
  simp only [F64.Ops.add_def] at this; push_cast at this; exact this

theorem mulR {a b : IEEE754.F64} (h : (FP.mul a b).isFinite) :
    |((FP.mul a b).val : ℝ) - ((a.val : ℝ) * b.val)| ≤ uZ * |(a.val : ℝ) * b.val| + hZ := by
  obtain ⟨ha, hb⟩ := isFinite_of_mul h
  have := toR (F64.Ops.mul_err ha hb (by simpa [F64.Ops.mul_def] using h))
  simp only [F64.Ops.mul_def] at this; push_cast at this; exact this

theorem uZ0 : (0 : ℝ) ≤ uZ := by positivity
theorem hZ0 : (0 : ℝ) ≤ hZ := by positivity

/-- One world coordinate's error bound: `o ⊕ ((X⊗px ⊕ Y⊗py) ⊕ Z⊗pz)`, the point within
`ex, ey, ez` of `(Px, Py, Pz)`. -/
noncomputable def ptwE (o X Y Z Px ex Py ey Pz ez : ℝ) : ℝ :=
  addE uZ hZ o 0 (X * Px + Y * Py + Z * Pz)
    (addE uZ hZ (X * Px + Y * Py)
      (addE uZ hZ (X * Px) (mulE uZ hZ X 0 Px ex) (Y * Py) (mulE uZ hZ Y 0 Py ey))
      (Z * Pz) (mulE uZ hZ Z 0 Pz ez))

theorem coord_err {o X Y Z px py pz : IEEE754.F64} {Px Py Pz ex ey ez : ℝ}
    (hx : |(px.val : ℝ) - Px| ≤ ex) (hy : |(py.val : ℝ) - Py| ≤ ey) (hz : |(pz.val : ℝ) - Pz| ≤ ez)
    (h : (FP.add o (FP.add (FP.add (FP.mul X px) (FP.mul Y py)) (FP.mul Z pz))).isFinite) :
    |((FP.add o (FP.add (FP.add (FP.mul X px) (FP.mul Y py)) (FP.mul Z pz))).val : ℝ) -
      ((o.val : ℝ) + ((X.val : ℝ) * Px + (Y.val : ℝ) * Py + (Z.val : ℝ) * Pz))| ≤
      ptwE o.val X.val Y.val Z.val Px ex Py ey Pz ez := by
  obtain ⟨_, h1⟩ := isFinite_of_add h
  obtain ⟨h2, h3⟩ := isFinite_of_add h1
  obtain ⟨h4, h5⟩ := isFinite_of_add h2
  have z0 : ∀ v : IEEE754.F64, |(v.val : ℝ) - v.val| ≤ 0 := fun v => by simp
  have m1 := mul_approx uZ0 (z0 X) hx (mulR h4)
  have m2 := mul_approx uZ0 (z0 Y) hy (mulR h5)
  have m3 := mul_approx uZ0 (z0 Z) hz (mulR h3)
  have a1 := add_approx uZ0 m1 m2 (addR h2)
  have a2 := add_approx uZ0 a1 m3 (addR h1)
  exact add_approx uZ0 (z0 o) a2 (addR h)

open forge_math in
/-- **The translated `Frame::point_to_world` refines the exact affine map**, coordinate by
coordinate, for a point within `(ex, ey, ez)` of `(Px, Py, Pz)`. -/
theorem ptw_spec (F : frame.Frame) (p : vec.Point3) {Px Py Pz ex ey ez : ℝ}
    (hx : |(p.x.val : ℝ) - Px| ≤ ex) (hy : |(p.y.val : ℝ) - Py| ≤ ey) (hz : |(p.z.val : ℝ) - Pz| ≤ ez) :
    frame.Frame.point_to_world F p ⦃ q =>
      (q.x.isFinite → |(q.x.val : ℝ) - ((F.origin.x.val : ℝ) + ((F.x : vec.Vec3).x.val * Px +
        (F.y : vec.Vec3).x.val * Py + (F.z : vec.Vec3).x.val * Pz))| ≤
        ptwE F.origin.x.val (F.x : vec.Vec3).x.val (F.y : vec.Vec3).x.val (F.z : vec.Vec3).x.val
          Px ex Py ey Pz ez) ∧
      (q.y.isFinite → |(q.y.val : ℝ) - ((F.origin.y.val : ℝ) + ((F.x : vec.Vec3).y.val * Px +
        (F.y : vec.Vec3).y.val * Py + (F.z : vec.Vec3).y.val * Pz))| ≤
        ptwE F.origin.y.val (F.x : vec.Vec3).y.val (F.y : vec.Vec3).y.val (F.z : vec.Vec3).y.val
          Px ex Py ey Pz ez) ∧
      (q.z.isFinite → |(q.z.val : ℝ) - ((F.origin.z.val : ℝ) + ((F.x : vec.Vec3).z.val * Px +
        (F.y : vec.Vec3).z.val * Py + (F.z : vec.Vec3).z.val * Pz))| ≤
        ptwE F.origin.z.val (F.x : vec.Vec3).z.val (F.y : vec.Vec3).z.val (F.z : vec.Vec3).z.val
          Px ex Py ey Pz ez) ⦄ := by
  unfold frame.Frame.point_to_world vec.Point3.to_vec frame.Frame.vec_to_world
    vec.UnitVec3.Insts.CoreOpsArithMulF64Vec3.mul vec.Vec3.Insts.CoreOpsArithMulF64Vec3.mul
    vec.Vec3.Insts.CoreOpsArithAddVec3Vec3.add vec.Point3.Insts.CoreOpsArithAddVec3Point3.add
    vec.Vec3.new
  simp only [lift, Std.bind_ok, WP.spec_ok, F64.Ops.add_def, F64.Ops.mul_def]
  exact ⟨fun h => coord_err hx hy hz h, fun h => coord_err hx hy hz h, fun h => coord_err hx hy hz h⟩

theorem divR {a b : IEEE754.F64} (ha : a.isFinite) (hb : b.isFinite) (hb0 : b.val ≠ 0)
    (h : (FP.div a b).isFinite) :
    |((FP.div a b).val : ℝ) - ((a.val : ℝ) / b.val)| ≤ uZ * |(a.val : ℝ) / b.val| + hZ := by
  have := toR (F64.Ops.div_err ha hb hb0 (by simpa [F64.Ops.div_def] using h))
  simp only [F64.Ops.div_def] at this; push_cast at this; exact this

/-- The angle's exact value and error: `θ` exactly at the far tangent line, else `u / rn`
rounded once. -/
noncomputable def phiX (u rn θ ba : IEEE754.F64) : ℝ :=
  if (u.val : ℝ) = ba.val then θ.val else (u.val : ℝ) / rn.val
noncomputable def phiE (u rn ba : IEEE754.F64) : ℝ :=
  if (u.val : ℝ) = ba.val then 0 else uZ * |(u.val : ℝ) / rn.val| + hZ

theorem phi_spec (u rn θ ba : IEEE754.F64) (hu : u.isFinite) (hrn : rn.isFinite)
    (hba : ba.isFinite) (hrn0 : rn.val ≠ 0) :
    fold.phi u rn θ ba ⦃ a => a.isFinite → |(a.val : ℝ) - phiX u rn θ ba| ≤ phiE u rn ba ⦄ := by
  unfold fold.phi
  have heq : F64.Ops.eq u ba = true ↔ u.val = ba.val := by
    rw [F64.Ops.eq_def']; exact FP.eq_iff hu hba
  by_cases h : F64.Ops.eq u ba = true
  · rw [if_pos h]
    have hv : (u.val : ℝ) = ba.val := by exact_mod_cast heq.mp h
    simp only [WP.spec_ok, phiX, phiE, hv, if_true]; intro; simp
  · rw [if_neg h]
    have hv : (u.val : ℝ) ≠ ba.val := fun e => h (heq.mpr (by exact_mod_cast e))
    simp only [WP.spec_ok, phiX, phiE, hv, if_false]
    intro hf; rw [F64.Ops.div_def] at hf ⊢; exact divR hu hrn hrn0 hf

/-- The local point's errors: `r = ρ0 ⊕ σ⊗w`, then `r⊗c`, `r⊗s`, `(−σ)⊗y`, with `c`, `s`
within `ec`, `es` of `C`, `S`. -/
noncomputable def rE (ρ0 σ w : ℝ) : ℝ := addE uZ hZ ρ0 0 (σ * w) (mulE uZ hZ σ 0 w 0)

theorem round_local_spec (ρ0 σ s c y w : IEEE754.F64) {C S ec es : ℝ}
    (hc : |(c.val : ℝ) - C| ≤ ec) (hs : |(s.val : ℝ) - S| ≤ es) :
    fold.round_local ρ0 σ s c y w ⦃ q =>
      let R := (ρ0.val : ℝ) + σ.val * w.val
      (q.1.isFinite → |(q.1.val : ℝ) - R * C| ≤ mulE uZ hZ R (rE ρ0.val σ.val w.val) C ec) ∧
      (q.2.1.isFinite → |(q.2.1.val : ℝ) - R * S| ≤ mulE uZ hZ R (rE ρ0.val σ.val w.val) S es) ∧
      (q.2.2.isFinite → |(q.2.2.val : ℝ) - (-(σ.val : ℝ)) * y.val| ≤
        mulE uZ hZ (-(σ.val : ℝ)) 0 y.val 0) ⦄ := by
  unfold fold.round_local
  simp only [lift, Std.bind_ok, WP.spec_ok, F64.Ops.add_def, F64.Ops.mul_def, F64.Ops.neg_def]
  have z0 : ∀ v : IEEE754.F64, |(v.val : ℝ) - v.val| ≤ 0 := fun v => by simp
  have rr : ∀ h : (FP.add ρ0 (FP.mul σ w)).isFinite,
      |((FP.add ρ0 (FP.mul σ w)).val : ℝ) - ((ρ0.val : ℝ) + σ.val * w.val)| ≤ rE ρ0.val σ.val w.val :=
    fun h => add_approx uZ0 (z0 ρ0) (mul_approx uZ0 (z0 σ) (z0 w) (mulR (isFinite_of_add h).2)) (addR h)
  refine ⟨fun h => ?_, fun h => ?_, fun h => ?_⟩
  · exact mul_approx uZ0 (rr (isFinite_of_mul h).1) hc (mulR h)
  · exact mul_approx uZ0 (rr (isFinite_of_mul h).1) hs (mulR h)
  · have := mul_approx uZ0 (z0 (FP.neg σ)) (z0 y) (mulR h)
    simpa [FP.val_neg] using this

open forge_math in
/-- build.rs `Geo::Round`'s point, as it is composed there: the angle `phi`, dmath's sine and
cosine (`Dmath.sinCos`, the shipped `sin_cos`), the local point, the axis frame. -/
def zonePoint (F : frame.Frame) (u rn θ ba ρ0 σ y w : IEEE754.F64) : Result vec.Point3 := do
  let a ← fold.phi u rn θ ba
  let sc := Dmath.sinCos a
  let q ← fold.round_local ρ0 σ sc.1 sc.2 y w
  frame.Frame.point_to_world F ⟨q.1, q.2.1, q.2.2⟩

theorem mul_nan_fin {a : IEEE754.F64} : (FP.mul a FP.nan).isFinite = false := by
  cases a <;> rfl

/-- The local point's exact value and errors. -/
noncomputable def locX (u rn θ ba ρ0 σ y w : IEEE754.F64) : ℝ × ℝ × ℝ :=
  let R := (ρ0.val : ℝ) + σ.val * w.val
  (R * Real.cos (phiX u rn θ ba), R * Real.sin (phiX u rn θ ba), -(σ.val : ℝ) * y.val)
noncomputable def locE (u rn θ ba ρ0 σ y w : IEEE754.F64) : ℝ × ℝ × ℝ :=
  let R := (ρ0.val : ℝ) + σ.val * w.val
  let et := 1 / 2 ^ 52 + phiE u rn ba
  (mulE uZ hZ R (rE ρ0.val σ.val w.val) (Real.cos (phiX u rn θ ba)) et,
   mulE uZ hZ R (rE ρ0.val σ.val w.val) (Real.sin (phiX u rn θ ba)) et,
   mulE uZ hZ (-(σ.val : ℝ)) 0 y.val 0)

open forge_math in
/-- **The zone fold refines the exact zone**: for finite inputs, `rn ≠ 0` and the exact angle
within `2¹⁹`, each world coordinate of build.rs's `Geo::Round` point, when all three are finite,
is within `ptwE` (the frame's rounding, with the local point's errors `locE` carried through
its axes) of the exact `o + A · (R cos φ, R sin φ, −σ y)`, `R = ρ0 + σ w`,
`φ = θ` at the far tangent line and `u / rn` elsewhere. -/
theorem zone_spec (F : frame.Frame) (u rn θ ba ρ0 σ y w : IEEE754.F64)
    (hu : u.isFinite) (hrn : rn.isFinite) (hba : ba.isFinite) (hθ : θ.isFinite)
    (hrn0 : rn.val ≠ 0) (hφ : |phiX u rn θ ba| ≤ 2 ^ 19) :
    zonePoint F u rn θ ba ρ0 σ y w ⦃ P =>
      P.x.isFinite → P.y.isFinite → P.z.isFinite →
      let L := locX u rn θ ba ρ0 σ y w
      let E := locE u rn θ ba ρ0 σ y w
      |(P.x.val : ℝ) - ((F.origin.x.val : ℝ) + ((F.x : vec.Vec3).x.val * L.1 +
        (F.y : vec.Vec3).x.val * L.2.1 + (F.z : vec.Vec3).x.val * L.2.2))| ≤
        ptwE F.origin.x.val (F.x : vec.Vec3).x.val (F.y : vec.Vec3).x.val (F.z : vec.Vec3).x.val
          L.1 E.1 L.2.1 E.2.1 L.2.2 E.2.2 ∧
      |(P.y.val : ℝ) - ((F.origin.y.val : ℝ) + ((F.x : vec.Vec3).y.val * L.1 +
        (F.y : vec.Vec3).y.val * L.2.1 + (F.z : vec.Vec3).y.val * L.2.2))| ≤
        ptwE F.origin.y.val (F.x : vec.Vec3).y.val (F.y : vec.Vec3).y.val (F.z : vec.Vec3).y.val
          L.1 E.1 L.2.1 E.2.1 L.2.2 E.2.2 ∧
      |(P.z.val : ℝ) - ((F.origin.z.val : ℝ) + ((F.x : vec.Vec3).z.val * L.1 +
        (F.y : vec.Vec3).z.val * L.2.1 + (F.z : vec.Vec3).z.val * L.2.2))| ≤
        ptwE F.origin.z.val (F.x : vec.Vec3).z.val (F.y : vec.Vec3).z.val (F.z : vec.Vec3).z.val
          L.1 E.1 L.2.1 E.2.1 L.2.2 E.2.2 ⦄ := by
  unfold zonePoint
  apply WP.spec_bind (phi_spec u rn θ ba hu hrn hba hrn0)
  intro a ha
  unfold fold.round_local frame.Frame.point_to_world vec.Point3.to_vec frame.Frame.vec_to_world
    vec.UnitVec3.Insts.CoreOpsArithMulF64Vec3.mul vec.Vec3.Insts.CoreOpsArithMulF64Vec3.mul
    vec.Vec3.Insts.CoreOpsArithAddVec3Vec3.add vec.Point3.Insts.CoreOpsArithAddVec3Point3.add
    vec.Vec3.new
  simp only [lift, Std.bind_ok, WP.spec_ok, F64.Ops.add_def, F64.Ops.mul_def, F64.Ops.neg_def]
  intro hPx hPy hPz
  -- finiteness back from the world x coordinate
  obtain ⟨_, x1⟩ := isFinite_of_add hPx
  obtain ⟨x2, _⟩ := isFinite_of_add x1
  obtain ⟨x4, _⟩ := isFinite_of_add x2
  obtain ⟨_, x6⟩ := isFinite_of_mul x4
  have hcf : (Dmath.sinCos a).2.isFinite := (isFinite_of_mul x6).2
  have haf : a.isFinite := by
    by_contra h
    have e : Dmath.sinCos a = (FP.nan, FP.nan) := by unfold Dmath.sinCos; rw [if_neg h]
    rw [e] at hcf; exact absurd hcf (by decide)
  have hae := ha haf
  have hpe : phiE u rn ba ≤ 1 := by
    by_cases h : (u.val : ℝ) = ba.val
    · simp [phiE, h]
    · have h19 : |(u.val : ℝ) / rn.val| ≤ 2 ^ 19 := by simpa [phiX, h] using hφ
      have : uZ * |(u.val : ℝ) / rn.val| ≤ 2 ^ (-53 : ℤ) * 2 ^ 19 :=
        mul_le_mul_of_nonneg_left h19 uZ0
      have : (2 : ℝ) ^ (-53 : ℤ) * 2 ^ 19 + 2 ^ (-1075 : ℤ) ≤ 1 := by norm_num
      simp only [phiE, h, if_false]; linarith
  have hb : |a.val| ≤ 1048576 := by
    have : |(a.val : ℝ)| ≤ 1048576 := by
      have := abs_sub_abs_le_abs_sub (a.val : ℝ) (phiX u rn θ ba)
      have : (2 : ℝ) ^ 19 + 1 ≤ 1048576 := by norm_num
      linarith
    exact_mod_cast this
  obtain ⟨fs, fc, es, ec⟩ := Dmath.sinCos_err haf hb
  obtain ⟨es', ec'⟩ := trig_approx hae es ec
  -- the local point's finiteness and errors
  obtain ⟨y2, _⟩ := isFinite_of_add x1
  obtain ⟨_, y4⟩ := isFinite_of_add y2
  have y6 := (isFinite_of_mul y4).2
  obtain ⟨_, z1⟩ := isFinite_of_add x1
  have z6 := (isFinite_of_mul z1).2
  have z0 : ∀ v : IEEE754.F64, |(v.val : ℝ) - v.val| ≤ 0 := fun v => by simp
  have rr := add_approx uZ0 (z0 ρ0) (mul_approx uZ0 (z0 σ) (z0 w)
    (mulR (isFinite_of_add (isFinite_of_mul x6).1).2)) (addR (isFinite_of_mul x6).1)
  have lx := mul_approx uZ0 rr ec' (mulR x6)
  have ly := mul_approx uZ0 rr es' (mulR y6)
  have lz : |((FP.mul (FP.neg σ) y).val : ℝ) - (-(σ.val : ℝ)) * y.val| ≤
      mulE uZ hZ (-(σ.val : ℝ)) 0 y.val 0 := by
    have := mul_approx uZ0 (z0 (FP.neg σ)) (z0 y) (mulR z6)
    simpa [FP.val_neg] using this
  simp only [locX, locE]
  exact ⟨coord_err lx ly lz hPx, coord_err lx ly lz hPy, coord_err lx ly lz hPz⟩

end forge_sheet_core
