/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.FaceB

/-!
# Curved site 1, `ssi::coincident`: the coincidence tests' `false` answers (origin/main ec32818)

`forge_geom::intersect::planes_coincident`, `ssi::cylinders_coincident`, and `ssi::sphere_sphere`'s
Coincident branch answer `false` when a computed f64 measure exceeds `ANGULAR = 1e-11` or
`LINEAR = 1e-5`. Each theorem says that the exact measure, the same formula over ℚ on the same
f64 inputs, is then nonzero. Unit vectors are any f64 triples: only `|a_i| ≤ 2` is assumed, and
only where a product is scaled again.

No underflow hypotheses are needed. `add` and `sub` of floats are exact relative to `ε` (`add_rel0`,
`sub_rel0`), and `mul` is within `ε|xy| + η` with `η = 2^-1075` (`mul_err`, from `round_err`).

- `cross_sound` (the parallel test): computed `|n₁ × n₂| > T ≥ 0` gives exact `n₁ × n₂ ≠ 0`. This is exact:
  an exact zero cross is a float zero, since equal products round alike and `x ⊖ x = 0`.
- `diff_sound` (`|r₁ − r₂| > T`), `centre_sound` (`|t.o − s.o| > T`): the same, exactly.
- `dot_err`, `plane_dist_sound` (`|z·(q.o − p.o)| > LINEAR`): within `10εM + 4η`,
  `M = Σ|z_i||q_i − p_i|`; nonzero when `10εM + 4η < T`.
- `offp_err`, `offp_sound` (`|off − a(a·off)| > LINEAR`, `|a_i| ≤ 2`): each component is within
  `e = 60εS + 10η` of the exact one, `S = Σ|d_i − c_i|`; nonzero when `4e² + 4η < (T − η)²/(1+ε)²`
  (for example `S ≤ 10^8`).
-/

namespace IEEE754.Screen

open FP

/-- `2^-1075`, half the least subnormal. -/
abbrev η' : ℚ := 1 / 2 ^ 1075

/-- `tol::ANGULAR = 1e-11`, `tol::LINEAR = 1e-5`. -/
def ANG : F64 := .fin false 6189700196426901 (-89) (by decide)
theorem ANG_bits : ANG.toBits = 0x3da5fd7fe1796495 := by decide
def LIN : F64 := .fin false 5902958103587057 (-69) (by decide)
theorem LIN_bits : LIN.toBits = 0x3ee4f8b588e368f1 := by decide
theorem LIN_val : LIN.val = (5902958103587057 / 2 ^ 69 : ℚ) := by simp only [LIN, FP.val, FP.sgn]; norm_num

/-! ## Rounding facts -/

lemma round_val_indep {s₁ s₂ : Bool} (r : ℚ) : (round binary64 s₁ r).val = (round binary64 s₂ r).val := by
  unfold round; split_ifs <;> simp [FP.val]

lemma round_zero_val (s : Bool) : (round binary64 s (0 : ℚ)).val = 0 := by
  unfold round; simp [FP.val]

lemma mul_err {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hf : (mul x y).isFinite) :
    |(mul x y).val - x.val * y.val| ≤ ε * |x.val * y.val| + η' := by
  rw [mul_eq_round hx hy] at hf ⊢
  have := round_err (K := ℚ) binary64 hf
  simp only [binary64_qmin] at this
  have h1 : (2 : ℚ) ^ (-((binary64.prec : ℕ) : ℤ)) = ε := by norm_num [binary64]
  have h2 : (2 : ℚ) ^ ((-1074 : ℤ) - 1) = η' := by norm_num
  rw [h1, h2] at this
  exact le_trans this (max_le (by linarith [show (0 : ℚ) ≤ η' by positivity]) (by
    linarith [mul_nonneg (show (0 : ℚ) ≤ ε by norm_num) (abs_nonneg (x.val * y.val))]))

lemma sub_same {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (h : a.val = b.val) : (sub a b).val = 0 := by
  rw [sub_eq_round ha hb, h, _root_.sub_self]; exact round_zero_val _

lemma mul_same {x y u v : F64} (hx : x.isFinite) (hy : y.isFinite) (hu : u.isFinite) (hv : v.isFinite)
    (h : x.val * y.val = u.val * v.val) : (mul x y).val = (mul u v).val := by
  rw [mul_eq_round hx hy, mul_eq_round hu hv, h]; exact round_val_indep _

lemma mul_zero_left {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (h : x.val = 0) : (mul x y).val = 0 := by
  rw [mul_eq_round hx hy, h, zero_mul]; exact round_zero_val _

lemma add_zeros {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (h1 : x.val = 0) (h2 : y.val = 0) :
    (add x y).val = 0 := by
  rw [add_eq_round hx hy, h1, h2, _root_.add_zero]; exact round_zero_val _

lemma sqrt_fin {x : F64} (h : x.sqrt.isFinite) : x.isFinite := by
  cases x with
  | zero s => rfl
  | inf s => cases s <;> simp_all [FP.sqrt, isFinite]
  | nan => simp_all [FP.sqrt, isFinite]
  | fin s m e c => rfl

lemma sqrt_zero_val {x : F64} (hx : x.isFinite) (h : x.val = 0) : x.sqrt.val = 0 := by
  cases x with
  | zero s => rfl
  | fin s m e c =>
    exfalso
    have hm : 0 < m := c.1
    simp only [FP.val, FP.sgn] at h
    have : (0 : ℚ) < (m : ℚ) * 2 ^ e := by positivity
    split_ifs at h <;> linarith
  | _ => simp [isFinite] at hx

lemma eta_cast : ((η' : ℚ) : ℝ) = (2 : ℝ) ^ ((-1074 : ℤ) - 1) := by
  rw [show ((-1074 : ℤ) - 1) = -((1075 : ℕ) : ℤ) by norm_num, zpow_neg, zpow_natCast]
  simp only [η', one_div, Rat.cast_inv, Rat.cast_pow, Rat.cast_ofNat]

/-! ## Vectors -/

/-- `Vec3::dot`: `x·u + y·v + z·w`, left to right. -/
def dotF (x y z u v w : F64) : F64 := add (add (mul x u) (mul y v)) (mul z w)

/-- `Vec3::length`: `sqrt(dot(self, self))`. -/
def lenF (x y z : F64) : F64 := (dotF x y z x y z).sqrt

lemma lenF_fin {x y z : F64} (h : (lenF x y z).isFinite) :
    x.isFinite ∧ y.isFinite ∧ z.isFinite ∧ (dotF x y z x y z).isFinite := by
  have hd := sqrt_fin h
  unfold dotF at hd
  have a1 := (isFinite_of_add hd).1
  exact ⟨(isFinite_of_mul (isFinite_of_add a1).1).1, (isFinite_of_mul (isFinite_of_add a1).2).1,
    (isFinite_of_mul (isFinite_of_add hd).2).1, hd⟩

/-- The length of a float zero vector is `0`. -/
lemma lenF_zero {x y z : F64} (h : (lenF x y z).isFinite) (h1 : x.val = 0) (h2 : y.val = 0) (h3 : z.val = 0) :
    (lenF x y z).val = 0 := by
  obtain ⟨fx, fy, fz, fd⟩ := lenF_fin h
  apply sqrt_zero_val fd
  unfold dotF at fd ⊢
  have a1 := (isFinite_of_add fd).1
  exact add_zeros a1 (isFinite_of_add fd).2
    (add_zeros (isFinite_of_add a1).1 (isFinite_of_add a1).2 (mul_zero_left fx fx h1) (mul_zero_left fy fy h2))
    (mul_zero_left fz fz h3)

/-- `Vec3::cross`'s length. -/
def crossLen (x y z u v w : F64) : F64 :=
  lenF (sub (mul y w) (mul z v)) (sub (mul z u) (mul x w)) (sub (mul x v) (mul y u))

/-- **The parallel test** (`n₁.cross(n₂).length() > ANGULAR`): the exact cross product is not `0`. -/
theorem cross_sound {x y z u v w T : F64} (hT : 0 ≤ T.val) (hf : (crossLen x y z u v w).isFinite)
    (h : T.val < (crossLen x y z u v w).val) :
    ¬ (y.val * w.val - z.val * v.val = 0 ∧ z.val * u.val - x.val * w.val = 0 ∧ x.val * v.val - y.val * u.val = 0) := by
  rintro ⟨e1, e2, e3⟩
  obtain ⟨f1, f2, f3, -⟩ := lenF_fin hf
  have g1 := isFinite_of_sub f1; have g2 := isFinite_of_sub f2; have g3 := isFinite_of_sub f3
  have k1 := isFinite_of_mul g1.1; have k2 := isFinite_of_mul g1.2
  have k3 := isFinite_of_mul g2.1; have k4 := isFinite_of_mul g2.2
  have k5 := isFinite_of_mul g3.1; have k6 := isFinite_of_mul g3.2
  have z1 := sub_same g1.1 g1.2 (mul_same k1.1 k1.2 k2.1 k2.2 (by linarith))
  have z2 := sub_same g2.1 g2.2 (mul_same k3.1 k3.2 k4.1 k4.2 (by linarith))
  have z3 := sub_same g3.1 g3.2 (mul_same k5.1 k5.2 k6.1 k6.2 (by linarith))
  have := lenF_zero hf z1 z2 z3
  unfold crossLen at h; rw [this] at h; linarith

/-- **A difference test** (`(r₁ − r₂).abs() > LINEAR`): the exact difference is not `0`. -/
theorem diff_sound {a b T : F64} (ha : a.isFinite) (hb : b.isFinite) (hT : 0 ≤ T.val)
    (h : T.val < (sub a b).abs.val) : a.val - b.val ≠ 0 := by
  intro e
  rw [FP.abs_val, sub_same ha hb (by linarith), abs_zero] at h; linarith

/-- **The distance test** (`(t.o − s.o).length() > LINEAR`): the exact centres differ. -/
theorem centre_sound {tx ty tz sx sy sz T : F64} (hT : 0 ≤ T.val)
    (hf : (lenF (sub tx sx) (sub ty sy) (sub tz sz)).isFinite)
    (h : T.val < (lenF (sub tx sx) (sub ty sy) (sub tz sz)).val) :
    ¬ (tx.val - sx.val = 0 ∧ ty.val - sy.val = 0 ∧ tz.val - sz.val = 0) := by
  rintro ⟨e1, e2, e3⟩
  obtain ⟨f1, f2, f3, -⟩ := lenF_fin hf
  have g1 := isFinite_of_sub f1; have g2 := isFinite_of_sub f2; have g3 := isFinite_of_sub f3
  have := lenF_zero hf (sub_same g1.1 g1.2 (by linarith)) (sub_same g2.1 g2.2 (by linarith))
    (sub_same g3.1 g3.2 (by linarith))
  rw [this] at h; linarith

/-! ## The plane offset: a dot product of a vector and a difference -/

/-- One term `z ⊗ (q ⊖ p)`: within `3ε|z||Δ| + η` of `zΔ`, and at most `2|z||Δ| + η`. -/
lemma term_err {z q p : F64} (hz : z.isFinite) (hq : q.isFinite) (hp : p.isFinite)
    (hf : (mul z (sub q p)).isFinite) :
    |(mul z (sub q p)).val - z.val * (q.val - p.val)| ≤ 3 * ε * (|z.val| * |q.val - p.val|) + η' ∧
      |(mul z (sub q p)).val| ≤ 2 * (|z.val| * |q.val - p.val|) + η' := by
  have fs := (isFinite_of_mul hf).2
  have e1 := sub_rel0 hq hp fs
  have e2 := mul_err hz fs hf
  set D := q.val - p.val
  set d := (sub q p).val
  have hd : |d| ≤ (1 + ε) * |D| := by have := abs_sub_abs_le_abs_sub d D; linarith
  have za := abs_nonneg z.val
  have p1 : |z.val * d - z.val * D| ≤ |z.val| * (ε * |D|) := by
    rw [← mul_sub, abs_mul]; exact mul_le_mul_of_nonneg_left e1 za
  have p2 : |z.val * d| ≤ |z.val| * ((1 + ε) * |D|) := by rw [abs_mul]; exact mul_le_mul_of_nonneg_left hd za
  have hzD : 0 ≤ |z.val| * |D| := by positivity
  constructor
  · have := abs_sub_le (mul z (sub q p)).val (z.val * d) (z.val * D)
    nlinarith
  · have := abs_sub_abs_le_abs_sub (mul z (sub q p)).val (z.val * d)
    nlinarith

/-- **The dot `z·(q ⊖ p)`** is within `10εM + 4η` of `z·(q − p)`, `M = Σ|z_i||q_i − p_i|`. -/
theorem dot_err {z0 z1 z2 q0 q1 q2 p0 p1 p2 : F64}
    (hz0 : z0.isFinite) (hz1 : z1.isFinite) (hz2 : z2.isFinite) (hq0 : q0.isFinite) (hq1 : q1.isFinite)
    (hq2 : q2.isFinite) (hp0 : p0.isFinite) (hp1 : p1.isFinite) (hp2 : p2.isFinite)
    (hf : (dotF z0 z1 z2 (sub q0 p0) (sub q1 p1) (sub q2 p2)).isFinite) :
    |(dotF z0 z1 z2 (sub q0 p0) (sub q1 p1) (sub q2 p2)).val -
        (z0.val * (q0.val - p0.val) + z1.val * (q1.val - p1.val) + z2.val * (q2.val - p2.val))| ≤
      10 * ε * (|z0.val| * |q0.val - p0.val| + |z1.val| * |q1.val - p1.val| + |z2.val| * |q2.val - p2.val|) +
        4 * η' := by
  unfold dotF at hf ⊢
  have f1 := (isFinite_of_add hf).1
  obtain ⟨a0, b0⟩ := term_err hz0 hq0 hp0 (isFinite_of_add f1).1
  obtain ⟨a1, b1⟩ := term_err hz1 hq1 hp1 (isFinite_of_add f1).2
  obtain ⟨a2, b2⟩ := term_err hz2 hq2 hp2 (isFinite_of_add hf).2
  have s1 := add_rel0 (isFinite_of_add f1).1 (isFinite_of_add f1).2 f1
  have s2 := add_rel0 f1 (isFinite_of_add hf).2 hf
  set π0 := (mul z0 (sub q0 p0)).val
  set π1 := (mul z1 (sub q1 p1)).val
  set π2 := (mul z2 (sub q2 p2)).val
  set S1 := (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))).val
  set m0 := |z0.val| * |q0.val - p0.val|
  set m1 := |z1.val| * |q1.val - p1.val|
  set m2 := |z2.val| * |q2.val - p2.val|
  have hm0 : 0 ≤ m0 := by positivity
  have hm1 : 0 ≤ m1 := by positivity
  have hm2 : 0 ≤ m2 := by positivity
  have hη : (0 : ℚ) ≤ η' := by positivity
  have t1 : |π0 + π1| ≤ |π0| + |π1| := abs_add_le _ _
  have hS1 : |S1| ≤ |π0 + π1| + ε * |π0 + π1| := by
    have := abs_sub_abs_le_abs_sub S1 (π0 + π1); linarith
  have t2 : |S1 + π2| ≤ |S1| + |π2| := abs_add_le _ _
  have big : |(add (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))) (mul z2 (sub q2 p2))).val -
      (z0.val * (q0.val - p0.val) + z1.val * (q1.val - p1.val) + z2.val * (q2.val - p2.val))| ≤
      |(add (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))) (mul z2 (sub q2 p2))).val - (S1 + π2)| +
        |S1 - (π0 + π1)| + |π0 - z0.val * (q0.val - p0.val)| + |π1 - z1.val * (q1.val - p1.val)| +
        |π2 - z2.val * (q2.val - p2.val)| := by
    have e : (add (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))) (mul z2 (sub q2 p2))).val -
        (z0.val * (q0.val - p0.val) + z1.val * (q1.val - p1.val) + z2.val * (q2.val - p2.val)) =
        ((add (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))) (mul z2 (sub q2 p2))).val - (S1 + π2)) +
        (S1 - (π0 + π1)) + (π0 - z0.val * (q0.val - p0.val)) + (π1 - z1.val * (q1.val - p1.val)) +
        (π2 - z2.val * (q2.val - p2.val)) := by ring
    rw [e]
    have h1 := abs_add_le ((add (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))) (mul z2 (sub q2 p2))).val - (S1 + π2) +
        (S1 - (π0 + π1)) + (π0 - z0.val * (q0.val - p0.val)) + (π1 - z1.val * (q1.val - p1.val)))
      (π2 - z2.val * (q2.val - p2.val))
    have h2 := abs_add_le ((add (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))) (mul z2 (sub q2 p2))).val - (S1 + π2) +
        (S1 - (π0 + π1)) + (π0 - z0.val * (q0.val - p0.val))) (π1 - z1.val * (q1.val - p1.val))
    have h3 := abs_add_le ((add (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))) (mul z2 (sub q2 p2))).val - (S1 + π2) +
        (S1 - (π0 + π1))) (π0 - z0.val * (q0.val - p0.val))
    have h4 := abs_add_le ((add (add (mul z0 (sub q0 p0)) (mul z1 (sub q1 p1))) (mul z2 (sub q2 p2))).val - (S1 + π2))
        (S1 - (π0 + π1))
    linarith
  linarith

/-- **The plane offset test** (`p.signed_distance(q.o).abs() > LINEAR`): the exact offset along
`p`'s normal is not `0`, when `10εM + 4η < T`. -/
theorem plane_dist_sound {z0 z1 z2 q0 q1 q2 p0 p1 p2 T : F64}
    (hz0 : z0.isFinite) (hz1 : z1.isFinite) (hz2 : z2.isFinite) (hq0 : q0.isFinite) (hq1 : q1.isFinite)
    (hq2 : q2.isFinite) (hp0 : p0.isFinite) (hp1 : p1.isFinite) (hp2 : p2.isFinite)
    (hf : (dotF z0 z1 z2 (sub q0 p0) (sub q1 p1) (sub q2 p2)).isFinite)
    (hM : 10 * ε * (|z0.val| * |q0.val - p0.val| + |z1.val| * |q1.val - p1.val| + |z2.val| * |q2.val - p2.val|) +
        4 * η' < T.val)
    (h : T.val < (dotF z0 z1 z2 (sub q0 p0) (sub q1 p1) (sub q2 p2)).abs.val) :
    z0.val * (q0.val - p0.val) + z1.val * (q1.val - p1.val) + z2.val * (q2.val - p2.val) ≠ 0 := by
  intro e
  have := dot_err hz0 hz1 hz2 hq0 hq1 hq2 hp0 hp1 hp2 hf
  rw [e, _root_.sub_zero] at this
  rw [FP.abs_val] at h
  linarith

/-! ## The cylinders' axis offset `off − a·(a·off)` -/

/-- One component `off_i ⊖ (a_i ⊗ t)`, with `t` the dot `a·off` within `10εN + 4η` of the
exact `T` (`dot_err`), `N ≤ 2S`, `|T| ≤ N`, `|a_i| ≤ 2`, `|Δ_i| ≤ S`: within `60εS + 10η` of
`Δ_i − a_i T`. -/
lemma offp_comp {a d c t : F64} {T N S : ℚ} (ha : a.isFinite) (hd : d.isFinite) (hc : c.isFinite) (htf : t.isFinite)
    (ha2 : |a.val| ≤ 2) (hΔ : |d.val - c.val| ≤ S) (hN : N ≤ 2 * S) (hT : |T| ≤ N)
    (ht : |t.val - T| ≤ 10 * ε * N + 4 * η')
    (hf : (sub (sub d c) (mul a t)).isFinite) :
    |(sub (sub d c) (mul a t)).val - ((d.val - c.val) - a.val * T)| ≤ 60 * ε * S + 10 * η' := by
  have fo := (isFinite_of_sub hf).1
  have fu := (isFinite_of_sub hf).2
  have e1 := sub_rel0 hd hc fo
  have e2 := mul_err ha htf fu
  have e3 := sub_rel0 fo fu hf
  set δ := (sub d c).val
  set u := (mul a t).val
  set Δ := d.val - c.val
  have hη : (0 : ℚ) ≤ η' := by positivity
  have hS : 0 ≤ S := le_trans (abs_nonneg _) hΔ
  have hta : |t.val| ≤ 2 * S + 20 * ε * S + 4 * η' := by
    have := abs_sub_abs_le_abs_sub t.val T; linarith
  have hat : |a.val * t.val| ≤ 2 * (2 * S + 20 * ε * S + 4 * η') := by
    rw [abs_mul]; exact mul_le_mul ha2 hta (abs_nonneg _) (by norm_num)
  have haT : |a.val * t.val - a.val * T| ≤ 2 * (10 * ε * N + 4 * η') := by
    rw [← mul_sub, abs_mul]; exact mul_le_mul ha2 ht (abs_nonneg _) (by norm_num)
  have hδ : |δ| ≤ (1 + ε) * S := by
    have := abs_sub_abs_le_abs_sub δ Δ; nlinarith
  have hu : |u| ≤ |a.val * t.val| + ε * |a.val * t.val| + η' := by
    have := abs_sub_abs_le_abs_sub u (a.val * t.val); linarith
  have hdu : |δ - u| ≤ |δ| + |u| := abs_sub_le_add δ u
  have tri : |(sub (sub d c) (mul a t)).val - (Δ - a.val * T)| ≤
      |(sub (sub d c) (mul a t)).val - (δ - u)| + |δ - Δ| + |u - a.val * t.val| + |a.val * t.val - a.val * T| := by
    have e : (sub (sub d c) (mul a t)).val - (Δ - a.val * T) = ((sub (sub d c) (mul a t)).val - (δ - u)) +
        (δ - Δ) - (u - a.val * t.val) - (a.val * t.val - a.val * T) := by ring
    rw [e]
    have h1 := abs_sub_le_add (((sub (sub d c) (mul a t)).val - (δ - u)) + (δ - Δ) - (u - a.val * t.val))
      (a.val * t.val - a.val * T)
    have h2 := abs_sub_le_add (((sub (sub d c) (mul a t)).val - (δ - u)) + (δ - Δ)) (u - a.val * t.val)
    have h3 := abs_add_le ((sub (sub d c) (mul a t)).val - (δ - u)) (δ - Δ)
    linarith
  have : ε * |Δ| ≤ ε * S := mul_le_mul_of_nonneg_left hΔ (by norm_num)
  nlinarith

/-- `off'`'s three components, as the Rust computes them. -/
def offpC (a d c t : F64) : F64 := sub (sub d c) (mul a t)

/-- The f64 sum of squares of three floats each at most `e`: at most `4e² + 4η`. -/
lemma sumsq_le {x y z : F64} {e : ℚ} (hd : (dotF x y z x y z).isFinite)
    (hx : |x.val| ≤ e) (hy : |y.val| ≤ e) (hz : |z.val| ≤ e) :
    (dotF x y z x y z).val ≤ 4 * e ^ 2 + 4 * η' := by
  unfold dotF at hd ⊢
  have f1 := (isFinite_of_add hd).1
  have fx := (isFinite_of_mul (isFinite_of_add f1).1).1
  have fy := (isFinite_of_mul (isFinite_of_add f1).2).1
  have fz := (isFinite_of_mul (isFinite_of_add hd).2).1
  have q0 := mul_err fx fx (isFinite_of_add f1).1
  have q1 := mul_err fy fy (isFinite_of_add f1).2
  have q2 := mul_err fz fz (isFinite_of_add hd).2
  have s1 := add_rel0 (isFinite_of_add f1).1 (isFinite_of_add f1).2 f1
  have s2 := add_rel0 f1 (isFinite_of_add hd).2 hd
  have e0 : 0 ≤ e := le_trans (abs_nonneg _) hx
  have sx : |x.val * x.val| ≤ e ^ 2 := by rw [abs_mul]; nlinarith [abs_nonneg x.val]
  have sy : |y.val * y.val| ≤ e ^ 2 := by rw [abs_mul]; nlinarith [abs_nonneg y.val]
  have sz : |z.val * z.val| ≤ e ^ 2 := by rw [abs_mul]; nlinarith [abs_nonneg z.val]
  set Q0 := (mul x x).val
  set Q1 := (mul y y).val
  set Q2 := (mul z z).val
  set S1 := (add (mul x x) (mul y y)).val
  have a0 : |Q0| ≤ e ^ 2 + ε * e ^ 2 + η' := by
    have := abs_sub_abs_le_abs_sub Q0 (x.val * x.val); nlinarith
  have a1 : |Q1| ≤ e ^ 2 + ε * e ^ 2 + η' := by
    have := abs_sub_abs_le_abs_sub Q1 (y.val * y.val); nlinarith
  have a2 : |Q2| ≤ e ^ 2 + ε * e ^ 2 + η' := by
    have := abs_sub_abs_le_abs_sub Q2 (z.val * z.val); nlinarith
  have t1 := abs_add_le Q0 Q1
  have b1 : |S1| ≤ |Q0 + Q1| + ε * |Q0 + Q1| := by have := abs_sub_abs_le_abs_sub S1 (Q0 + Q1); linarith
  have t2 := abs_add_le S1 Q2
  have := le_abs_self (add (add (mul x x) (mul y y)) (mul z z)).val
  have hη : (0 : ℚ) ≤ η' := by positivity
  have he2 : 0 ≤ e ^ 2 := sq_nonneg e
  have u1 := le_abs_self (S1 + Q2)
  rw [abs_le] at s2
  have : ε * |S1 + Q2| ≤ ε * (|S1| + |Q2|) := mul_le_mul_of_nonneg_left t2 (by norm_num)
  have : ε * |Q0 + Q1| ≤ ε * (|Q0| + |Q1|) := mul_le_mul_of_nonneg_left t1 (by norm_num)
  linarith

/-- **The cylinders' offset test** (`off'.length() > LINEAR`, `|a_i| ≤ 2`): the exact `off'`
is not `0`, when `(1+ε)²(4e² + 4η) < (T − η)²`, `e = 60εS + 10η`, `S = Σ|d_i − c_i|`. -/
theorem offp_sound {a0 a1 a2 d0 d1 d2 c0 c1 c2 T : F64}
    (fa0 : a0.isFinite) (fa1 : a1.isFinite) (fa2 : a2.isFinite) (fd0 : d0.isFinite) (fd1 : d1.isFinite)
    (fd2 : d2.isFinite) (fc0 : c0.isFinite) (fc1 : c1.isFinite) (fc2 : c2.isFinite)
    (ha0 : |a0.val| ≤ 2) (ha1 : |a1.val| ≤ 2) (ha2 : |a2.val| ≤ 2)
    (hT : η' < T.val)
    (hS : (1 + ε) ^ 2 * (4 * (60 * ε * (|d0.val - c0.val| + |d1.val - c1.val| + |d2.val - c2.val|) + 10 * η') ^ 2 +
      4 * η') < (T.val - η') ^ 2)
    (hf : (lenF (offpC a0 d0 c0 (dotF a0 a1 a2 (sub d0 c0) (sub d1 c1) (sub d2 c2)))
      (offpC a1 d1 c1 (dotF a0 a1 a2 (sub d0 c0) (sub d1 c1) (sub d2 c2)))
      (offpC a2 d2 c2 (dotF a0 a1 a2 (sub d0 c0) (sub d1 c1) (sub d2 c2)))).isFinite)
    (h : T.val < (lenF (offpC a0 d0 c0 (dotF a0 a1 a2 (sub d0 c0) (sub d1 c1) (sub d2 c2)))
      (offpC a1 d1 c1 (dotF a0 a1 a2 (sub d0 c0) (sub d1 c1) (sub d2 c2)))
      (offpC a2 d2 c2 (dotF a0 a1 a2 (sub d0 c0) (sub d1 c1) (sub d2 c2)))).val) :
    let Tx := a0.val * (d0.val - c0.val) + a1.val * (d1.val - c1.val) + a2.val * (d2.val - c2.val)
    ¬ ((d0.val - c0.val) - a0.val * Tx = 0 ∧ (d1.val - c1.val) - a1.val * Tx = 0 ∧ (d2.val - c2.val) - a2.val * Tx = 0) := by
  intro Tx
  rintro ⟨z0, z1, z2⟩
  set t := dotF a0 a1 a2 (sub d0 c0) (sub d1 c1) (sub d2 c2)
  set S := |d0.val - c0.val| + |d1.val - c1.val| + |d2.val - c2.val| with hSd
  set N := |a0.val| * |d0.val - c0.val| + |a1.val| * |d1.val - c1.val| + |a2.val| * |d2.val - c2.val| with hNd
  obtain ⟨f0, f1, f2, fdot⟩ := lenF_fin hf
  have ft : t.isFinite := (isFinite_of_mul (isFinite_of_sub f0).2).2
  have hte := dot_err fa0 fa1 fa2 fd0 fd1 fd2 fc0 fc1 fc2 ft
  have hNS : N ≤ 2 * S := by
    have := mul_le_mul_of_nonneg_right ha0 (abs_nonneg (d0.val - c0.val))
    have := mul_le_mul_of_nonneg_right ha1 (abs_nonneg (d1.val - c1.val))
    have := mul_le_mul_of_nonneg_right ha2 (abs_nonneg (d2.val - c2.val))
    rw [hNd, hSd]; linarith
  have hTN : |Tx| ≤ N := by
    have := abs_add_three (a0.val * (d0.val - c0.val)) (a1.val * (d1.val - c1.val)) (a2.val * (d2.val - c2.val))
    simp only [abs_mul] at this; exact this
  have n0 := abs_nonneg (d0.val - c0.val)
  have n1 := abs_nonneg (d1.val - c1.val)
  have n2 := abs_nonneg (d2.val - c2.val)
  have s0 : |d0.val - c0.val| ≤ S := by rw [hSd]; linarith
  have s1 : |d1.val - c1.val| ≤ S := by rw [hSd]; linarith
  have s2 : |d2.val - c2.val| ≤ S := by rw [hSd]; linarith
  have k0 := offp_comp fa0 fd0 fc0 ft ha0 s0 hNS hTN hte f0
  have k1 := offp_comp fa1 fd1 fc1 ft ha1 s1 hNS hTN hte f1
  have k2 := offp_comp fa2 fd2 fc2 ft ha2 s2 hNS hTN hte f2
  rw [z0, _root_.sub_zero] at k0
  rw [z1, _root_.sub_zero] at k1
  rw [z2, _root_.sub_zero] at k2
  have hs := sumsq_le fdot k0 k1 k2
  -- the square root, from below: `T < sqrt(s)` gives `(T − η)² < (1+ε)² s`
  set sF := dotF (offpC a0 d0 c0 t) (offpC a1 d1 c1 t) (offpC a2 d2 c2 t)
    (offpC a0 d0 c0 t) (offpC a1 d1 c1 t) (offpC a2 d2 c2 t)
  have hη : (0 : ℚ) ≤ η' := by positivity
  unfold lenF at h hf
  change T.val < sF.sqrt.val at h
  change sF.sqrt.isFinite at hf
  cases hsF : sF with
  | zero s =>
    rw [hsF] at h
    have h0 : (FP.sqrt (FP.zero s : F64)).val = 0 := rfl
    rw [h0] at h; linarith [hT, hη]
  | inf s => rw [hsF] at fdot; simp [isFinite] at fdot
  | nan => rw [hsF] at fdot; simp [isFinite] at fdot
  | fin sg m e cn =>
    cases sg with
    | true => rw [hsF] at hf; simp [FP.sqrt, isFinite] at hf
    | false =>
      rw [hsF] at h hf hs
      rw [sqrt_eq_round] at h hf
      generalize hx : η' = x at hs hS hT hη
      have er := round_err (K := ℝ) binary64 hf
      simp only [binary64_qmin] at er
      have hv0 : (0 : ℝ) ≤ (((FP.fin false m e cn : F64).val : ℚ) : ℝ) := by
        have : (0 : ℚ) ≤ (FP.fin false m e cn : F64).val := by
          simp only [FP.val, FP.sgn]; norm_num; positivity
        exact_mod_cast this
      set r := Real.sqrt ((((FP.fin false m e cn : F64).val : ℚ) : ℝ))
      have hr0 : 0 ≤ r := Real.sqrt_nonneg _
      have hrr : r * r = (((FP.fin false m e cn : F64).val : ℚ) : ℝ) := Real.mul_self_sqrt hv0
      rw [abs_of_nonneg hr0] at er
      have e1 : (2 : ℝ) ^ (-((binary64.prec : ℕ) : ℤ)) = (ε : ℝ) := by norm_num [binary64]
      rw [e1, ← eta_cast, hx] at er
      have hε0 : (0 : ℝ) ≤ ((ε : ℚ) : ℝ) := by norm_num
      have hη0 : (0 : ℝ) ≤ ((x : ℚ) : ℝ) := by exact_mod_cast hη
      have mx : max (((ε : ℚ) : ℝ) * r) ((x : ℚ) : ℝ) ≤ ((ε : ℚ) : ℝ) * r + ((x : ℚ) : ℝ) :=
        max_le (by nlinarith) (by nlinarith)
      have up : (((round binary64 false r).val : ℚ) : ℝ) ≤ r + ((ε : ℚ) : ℝ) * r + ((x : ℚ) : ℝ) := by
        have := (abs_le.mp (le_trans er mx)).2
        linarith
      have hTr : ((T.val : ℚ) : ℝ) < r + ((ε : ℚ) : ℝ) * r + ((x : ℚ) : ℝ) := lt_of_lt_of_le (by exact_mod_cast h) up
      have hT' : ((x : ℚ) : ℝ) < ((T.val : ℚ) : ℝ) := by exact_mod_cast hT
      have sq : (((T.val : ℚ) : ℝ) - ((x : ℚ) : ℝ)) ^ 2 < (1 + ((ε : ℚ) : ℝ)) ^ 2 * (r * r) := by
        have l1 : ((T.val : ℚ) : ℝ) - ((x : ℚ) : ℝ) < (1 + ((ε : ℚ) : ℝ)) * r := by linarith
        have l0 : (0 : ℝ) < ((T.val : ℚ) : ℝ) - ((x : ℚ) : ℝ) := by linarith
        nlinarith
      rw [hrr] at sq
      have hs' : (((FP.fin false m e cn : F64).val : ℚ) : ℝ) ≤ ((4 * (60 * ε * S + 10 * x) ^ 2 + 4 * x : ℚ) : ℝ) := by
        exact_mod_cast hs
      have hS' : ((((1 + ε) ^ 2 * (4 * (60 * ε * S + 10 * x) ^ 2 + 4 * x)) : ℚ) : ℝ) < (((T.val - x) ^ 2 : ℚ) : ℝ) := by
        exact_mod_cast hS
      push_cast at hs' hS' sq
      nlinarith

/-- With `T = LINEAR` and offsets of at most `10^8` (100 km in mm) in all, the margin holds. -/
lemma offp_margin {S : ℚ} (h0 : 0 ≤ S) (h : S ≤ 10 ^ 8) :
    (1 + ε) ^ 2 * (4 * (60 * ε * S + 10 * η') ^ 2 + 4 * η') < (LIN.val - η') ^ 2 := by
  rw [LIN_val]
  have hx : η' ≤ 1 / 2 ^ 100 := by
    show (1 : ℚ) / 2 ^ 1075 ≤ 1 / 2 ^ 100
    exact one_div_le_one_div_of_le (by positivity) (pow_le_pow_right₀ (by norm_num) (by norm_num))
  have hx0 : (0 : ℚ) ≤ η' := by positivity
  generalize η' = x at hx hx0 ⊢
  have a1 : 60 * ε * S + 10 * x ≤ 60 * ε * 10 ^ 8 + 10 / 2 ^ 100 := by nlinarith
  have a2 : (60 * ε * S + 10 * x) ^ 2 ≤ (60 * ε * 10 ^ 8 + 10 / 2 ^ 100) ^ 2 :=
    pow_le_pow_left₀ (by positivity) a1 2
  have a3 : 5902958103587057 / 2 ^ 69 - 1 / 2 ^ 100 ≤ (5902958103587057 / 2 ^ 69 : ℚ) - x := by linarith
  have a4 : (5902958103587057 / 2 ^ 69 - 1 / 2 ^ 100 : ℚ) ^ 2 ≤ (5902958103587057 / 2 ^ 69 - x) ^ 2 :=
    pow_le_pow_left₀ (by norm_num) a3 2
  have a5 : (1 + ε) ^ 2 * (4 * (60 * ε * 10 ^ 8 + 10 / 2 ^ 100) ^ 2 + 4 / 2 ^ 100) <
      (5902958103587057 / 2 ^ 69 - 1 / 2 ^ 100 : ℚ) ^ 2 := by norm_num
  have a6 : (1 + ε) ^ 2 * (4 * (60 * ε * S + 10 * x) ^ 2 + 4 * x) ≤
      (1 + ε) ^ 2 * (4 * (60 * ε * 10 ^ 8 + 10 / 2 ^ 100) ^ 2 + 4 / 2 ^ 100) := by
    apply mul_le_mul_of_nonneg_left _ (by positivity); nlinarith
  linarith

end IEEE754.Screen
