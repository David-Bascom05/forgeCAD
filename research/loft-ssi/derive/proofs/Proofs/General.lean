import Mathlib

/-!
# Two arbitrary ellipse sections: the exact meet condition

A loft whose rails are arcs with one common sweep has sections
`S(phi) = c + cos phi * X + sin phi * Y` (`arc_family` below), an ellipse in a plane of any
orientation. Two sections meet iff `alpha X1 + beta Y1 - gamma X2 - delta Y2 + (c1 - c2) = 0`
has a solution with `alpha^2 + beta^2 = 1 = gamma^2 + delta^2`.

Write the five columns `A1 = X1, A2 = Y1, A3 = -X2, A4 = -Y2, A5 = c1 - c2` and `p_ij` for the
3x3 determinant of the three columns other than `i, j` (the Plücker coordinates of the solution
pencil). With `d = (1, 1, l, l, -(1 + l))`, `E(l) = sum_{i<j} d_i d_j p_ij^2 = E0 + E1 l + E2 l^2`
and `F = E1^2 - 4 E0 E2`:

* `meet_imp`: a meet forces `F = 0`, `E0 <= 0`, `E2 <= 0`, with no hypothesis at all;
* `meet_iff`: if `X1, Y1, X2, Y2` span R^3 (`General`), the converse holds;
* `meet_iff_of_ne`: if also `X2 x Y2 != 0`, `E0 <= 0` alone suffices.

`F` equals `Res_t(q1, q2) / (|n1|^4 |n2|^4 |l|^8)` of the line form (checked exactly in SymPy,
`general_identity.py`); it has degree 8 in each section's data, so degree 24 in each spline
parameter for cubic data (`GeneralDegree.lean`).
-/

namespace LoftSSI

/-- A vector in a commutative ring (reals for geometry, polynomials for degree counts). -/
@[ext] structure V3 (R : Type) where
  x : R
  y : R
  z : R

namespace V3

variable {R : Type} [CommRing R]

/-- `det a b c = a . (b x c)`. -/
def det (a b c : V3 R) : R :=
  a.x * (b.y * c.z - b.z * c.y) - a.y * (b.x * c.z - b.z * c.x) + a.z * (b.x * c.y - b.y * c.x)

def cross (a b : V3 R) : V3 R :=
  ⟨a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x⟩

def neg (a : V3 R) : V3 R := ⟨-a.x, -a.y, -a.z⟩

def sub (a b : V3 R) : V3 R := ⟨a.x - b.x, a.y - b.y, a.z - b.z⟩

def map {S : Type} (f : R → S) (a : V3 R) : V3 S := ⟨f a.x, f a.y, f a.z⟩

end V3

open V3

/-- One section ellipse `c + cos phi X + sin phi Y`. -/
structure Sec (R : Type) where
  c : V3 R
  X : V3 R
  Y : V3 R

section Plucker

variable {R : Type} [CommRing R]

/-- The five columns of the meet system. -/
structure Cols (R : Type) where
  A1 : V3 R
  A2 : V3 R
  A3 : V3 R
  A4 : V3 R
  A5 : V3 R

def cols (e1 e2 : Sec R) : Cols R := ⟨e1.X, e1.Y, neg e2.X, neg e2.Y, sub e1.c e2.c⟩

namespace Cols

variable (A : Cols R)

def p12 : R := det A.A3 A.A4 A.A5
def p13 : R := det A.A2 A.A4 A.A5
def p14 : R := det A.A2 A.A3 A.A5
def p15 : R := det A.A2 A.A3 A.A4
def p23 : R := det A.A1 A.A4 A.A5
def p24 : R := det A.A1 A.A3 A.A5
def p25 : R := det A.A1 A.A3 A.A4
def p34 : R := det A.A1 A.A2 A.A5
def p35 : R := det A.A1 A.A2 A.A4
def p45 : R := det A.A1 A.A2 A.A3

def E0 : R := A.p12 ^ 2 - A.p15 ^ 2 - A.p25 ^ 2
def E1 : R := A.p13 ^ 2 + A.p14 ^ 2 + A.p23 ^ 2 + A.p24 ^ 2 - A.p15 ^ 2 - A.p25 ^ 2 -
  A.p35 ^ 2 - A.p45 ^ 2
def E2 : R := A.p34 ^ 2 - A.p35 ^ 2 - A.p45 ^ 2

/-- The exact section-pair polynomial. -/
def F : R := A.E1 ^ 2 - 4 * A.E0 * A.E2

end Cols

end Plucker

/-! Binary quadratic forms `a s^2 + 2 b s t + c t^2`. -/

def bq (a b c s t : ℝ) : ℝ := a * s ^ 2 + 2 * b * s * t + c * t ^ 2

def res (a1 b1 c1 a2 b2 c2 : ℝ) : ℝ :=
  (a1 * c2 - a2 * c1) ^ 2 - 4 * (a1 * b2 - a2 * b1) * (b1 * c2 - b2 * c1)

theorem res_comm (a1 b1 c1 a2 b2 c2 : ℝ) : res a1 b1 c1 a2 b2 c2 = res a2 b2 c2 a1 b1 c1 := by
  unfold res; ring

/-- A nonzero real zero forces a nonnegative discriminant. -/
theorem disc_of_zero {a b c s t : ℝ} (h : bq a b c s t = 0) (hst : s ≠ 0 ∨ t ≠ 0) :
    0 ≤ b ^ 2 - a * c := by
  unfold bq at h
  rcases eq_or_ne a 0 with ha | ha
  · subst ha; nlinarith [sq_nonneg b]
  rcases eq_or_ne t 0 with ht | ht
  · subst ht
    have hs : s ≠ 0 := hst.resolve_right (by simp)
    have : a * s ^ 2 = 0 := by linarith
    exact absurd ((mul_eq_zero.mp this).resolve_right (pow_ne_zero 2 hs)) ha
  · -- a f = (a s + b t)^2 - (b^2 - a c) t^2
    have e : (b ^ 2 - a * c) * t ^ 2 = (a * s + b * t) ^ 2 := by linear_combination (-a) * h
    have ht2 : 0 < t ^ 2 := by positivity
    by_contra hneg
    push Not at hneg
    nlinarith [sq_nonneg (a * s + b * t)]

/-- A common nonzero zero forces the resultant to vanish. -/
theorem res_of_common {a1 b1 c1 a2 b2 c2 s t : ℝ} (h1 : bq a1 b1 c1 s t = 0)
    (h2 : bq a2 b2 c2 s t = 0) (hst : s ≠ 0 ∨ t ≠ 0) : res a1 b1 c1 a2 b2 c2 = 0 := by
  unfold bq at h1 h2
  -- N = (2(b1c2 - b2c1), a2c1 - a1c2, 2(a1b2 - a2b1)) is parallel to (s^2, st, t^2)
  set N1 := 2 * (b1 * c2 - b2 * c1)
  set N2 := a2 * c1 - a1 * c2
  set N3 := 2 * (a1 * b2 - a2 * b1)
  have e12 : N1 * (s * t) = N2 * s ^ 2 := by
    simp only [N1, N2]; linear_combination (-c1) * h2 + c2 * h1
  have e23 : N2 * t ^ 2 = N3 * (s * t) := by
    simp only [N2, N3]; linear_combination (-a1) * h2 + a2 * h1
  have e13 : N1 * t ^ 2 = N3 * s ^ 2 := by
    simp only [N1, N3]; linear_combination (-2 * b2) * h1 + 2 * b1 * h2
  have hr : res a1 b1 c1 a2 b2 c2 = N2 ^ 2 - N1 * N3 := by
    unfold res; simp only [N1, N2, N3]; ring
  rw [hr]
  have k1 : s ^ 4 * (N2 ^ 2 - N1 * N3) = 0 := by
    linear_combination (-(N2 * s ^ 2 + N1 * s * t)) * e12 + (N1 * s ^ 2) * e13
  have k2 : t ^ 4 * (N2 ^ 2 - N1 * N3) = 0 := by
    linear_combination (N2 * t ^ 2 + N3 * s * t) * e23 - (N3 * t ^ 2) * e13
  rcases hst with hs | ht
  · exact (mul_eq_zero.mp k1).resolve_left (pow_ne_zero 4 hs)
  · exact (mul_eq_zero.mp k2).resolve_left (pow_ne_zero 4 ht)

/-- The converse: a vanishing resultant and a form with real zeros (not identically zero)
give a common nonzero real zero. -/
theorem common_of_res {a1 b1 c1 a2 b2 c2 : ℝ} (hr : res a1 b1 c1 a2 b2 c2 = 0)
    (hd : 0 ≤ b1 ^ 2 - a1 * c1) (h0 : a1 ≠ 0 ∨ b1 ≠ 0 ∨ c1 ≠ 0) :
    ∃ s t : ℝ, (s ≠ 0 ∨ t ≠ 0) ∧ bq a1 b1 c1 s t = 0 ∧ bq a2 b2 c2 s t = 0 := by
  rcases eq_or_ne a1 0 with ha | ha
  · -- f1 = t (2 b1 s + c1 t): zeros (1, 0) and (c1, -2 b1)
    subst ha
    have hbc : b1 ≠ 0 ∨ c1 ≠ 0 := h0.resolve_left (by simp)
    have hp : res 0 b1 c1 a2 b2 c2 = bq a2 b2 c2 1 0 * bq a2 b2 c2 c1 (-2 * b1) := by
      unfold res bq; ring
    rw [hp] at hr
    rcases mul_eq_zero.mp hr with h | h
    · exact ⟨1, 0, Or.inl one_ne_zero, by unfold bq; ring, h⟩
    · refine ⟨c1, -2 * b1, ?_, by unfold bq; ring, h⟩
      rcases hbc with hb | hc
      · exact Or.inr (by intro h'; apply hb; linarith)
      · exact Or.inl hc
  · set σ := Real.sqrt (b1 ^ 2 - a1 * c1)
    have hσ : σ ^ 2 = b1 ^ 2 - a1 * c1 := Real.sq_sqrt hd
    have hp : a1 ^ 2 * res a1 b1 c1 a2 b2 c2 =
        bq a2 b2 c2 (-b1 + σ) a1 * bq a2 b2 c2 (-b1 - σ) a1 := by
      unfold res bq
      linear_combination (-2 * a1 ^ 2 * a2 * c2 + 4 * a1 ^ 2 * b2 ^ 2 + a1 * a2 ^ 2 * c1 -
        4 * a1 * a2 * b1 * b2 + a2 ^ 2 * b1 ^ 2 - a2 ^ 2 * σ ^ 2) * hσ
    rw [hr, mul_zero] at hp
    have z1 : ∀ e : ℝ, e ^ 2 = σ ^ 2 → bq a1 b1 c1 (-b1 + e) a1 = 0 := by
      intro e he; unfold bq
      linear_combination a1 * he + a1 * hσ
    rcases mul_eq_zero.mp hp.symm with h | h
    · exact ⟨-b1 + σ, a1, Or.inr ha, z1 σ rfl, h⟩
    · refine ⟨-b1 - σ, a1, Or.inr ha, ?_, h⟩
      have := z1 (-σ) (by ring)
      rwa [show -b1 + -σ = -b1 - σ by ring] at this

/-! The pencil algebra: basis `(w, k)` of the solution plane, `k = (k1, k2, k3, k4, 0)`. -/

/-- With `pi_ij(w, k) = ±w5 p_ij` the binary forms `Q1 = z1^2 + z2^2 - z5^2`,
`Q2 = z3^2 + z4^2 - z5^2` on the basis `(w, k)` have discriminants `-w5^2 E0`, `-w5^2 E2` and
resultant `w5^4 F`. Pure algebra. -/
theorem pencil_alg (w1 w2 w3 w4 w5 p12 p13 p14 p15 p23 p24 p25 p34 p35 p45 : ℝ)
    (h12 : w1 * -p25 - w2 * p15 = w5 * p12) (h13 : w1 * p35 - w3 * p15 = -(w5 * p13))
    (h14 : w1 * -p45 - w4 * p15 = w5 * p14) (h23 : w2 * p35 - w3 * -p25 = w5 * p23)
    (h24 : w2 * -p45 - w4 * -p25 = -(w5 * p24)) (h34 : w3 * -p45 - w4 * p35 = w5 * p34) :
    let a1 := w1 ^ 2 + w2 ^ 2 - w5 ^ 2
    let b1 := w1 * p15 + w2 * -p25
    let c1 := p15 ^ 2 + (-p25) ^ 2
    let a2 := w3 ^ 2 + w4 ^ 2 - w5 ^ 2
    let b2 := w3 * p35 + w4 * -p45
    let c2 := p35 ^ 2 + (-p45) ^ 2
    let E0 := p12 ^ 2 - p15 ^ 2 - p25 ^ 2
    let E1 := p13 ^ 2 + p14 ^ 2 + p23 ^ 2 + p24 ^ 2 - p15 ^ 2 - p25 ^ 2 - p35 ^ 2 - p45 ^ 2
    let E2 := p34 ^ 2 - p35 ^ 2 - p45 ^ 2
    b1 ^ 2 - a1 * c1 = -(w5 ^ 2 * E0) ∧ b2 ^ 2 - a2 * c2 = -(w5 ^ 2 * E2) ∧
      res a1 b1 c1 a2 b2 c2 = w5 ^ 4 * (E1 ^ 2 - 4 * E0 * E2) := by
  intro a1 b1 c1 a2 b2 c2 E0 E1 E2
  have d0 : b1 ^ 2 - a1 * c1 = -(w5 ^ 2 * E0) := by
    simp only [a1, b1, c1, E0]
    linear_combination (-(w1 * -p25 - w2 * p15 + w5 * p12)) * h12
  have d2 : b2 ^ 2 - a2 * c2 = -(w5 ^ 2 * E2) := by
    simp only [a2, b2, c2, E2]
    linear_combination (-(w3 * -p45 - w4 * p35 + w5 * p34)) * h34
  have d1 : 2 * b1 * b2 - a1 * c2 - a2 * c1 = -(w5 ^ 2 * E1) := by
    simp only [a1, b1, c1, a2, b2, c2, E1]
    linear_combination (-(w1 * p35 - w3 * p15 - w5 * p13)) * h13 +
      (-(w1 * -p45 - w4 * p15 + w5 * p14)) * h14 + (-(w2 * p35 - w3 * -p25 + w5 * p23)) * h23 +
      (-(w2 * -p45 - w4 * -p25 - w5 * p24)) * h24
  refine ⟨d0, d2, ?_⟩
  have hr : res a1 b1 c1 a2 b2 c2 =
      (2 * b1 * b2 - a1 * c2 - a2 * c1) ^ 2 - 4 * (b1 ^ 2 - a1 * c1) * (b2 ^ 2 - a2 * c2) := by
    unfold res; ring
  rw [hr, d0, d1, d2]; ring

/-! Geometry: the kernel vector `k` of `[A1 A2 A3 A4]` and the Plücker relations. -/

section Kernel

variable (A : Cols ℝ)

/-- `k`, the null vector of the first four columns (signed 3x3 minors). -/
def Cols.k1 : ℝ := A.p15
def Cols.k2 : ℝ := -A.p25
def Cols.k3 : ℝ := A.p35
def Cols.k4 : ℝ := -A.p45

/-- `w` solves the homogeneous system `sum w_i A_i = 0`. -/
def Cols.Ker (w1 w2 w3 w4 w5 : ℝ) : Prop :=
  w1 * A.A1.x + w2 * A.A2.x + w3 * A.A3.x + w4 * A.A4.x + w5 * A.A5.x = 0 ∧
  w1 * A.A1.y + w2 * A.A2.y + w3 * A.A3.y + w4 * A.A4.y + w5 * A.A5.y = 0 ∧
  w1 * A.A1.z + w2 * A.A2.z + w3 * A.A3.z + w4 * A.A4.z + w5 * A.A5.z = 0

theorem Cols.ker_k : A.Ker A.k1 A.k2 A.k3 A.k4 0 := by
  simp only [Cols.Ker, Cols.k1, Cols.k2, Cols.k3, Cols.k4, Cols.p15, Cols.p25, Cols.p35,
    Cols.p45, det]
  refine ⟨by ring, by ring, by ring⟩

/-- For `w` in the kernel, `w ∧ k = w5 * p` (up to fixed signs). -/
theorem Cols.plucker {w1 w2 w3 w4 w5 : ℝ} (h : A.Ker w1 w2 w3 w4 w5) :
    w1 * A.k2 - w2 * A.k1 = w5 * A.p12 ∧ w1 * A.k3 - w3 * A.k1 = -(w5 * A.p13) ∧
    w1 * A.k4 - w4 * A.k1 = w5 * A.p14 ∧ w2 * A.k3 - w3 * A.k2 = w5 * A.p23 ∧
    w2 * A.k4 - w4 * A.k2 = -(w5 * A.p24) ∧ w3 * A.k4 - w4 * A.k3 = w5 * A.p34 := by
  obtain ⟨hx, hy, hz⟩ := h
  obtain ⟨⟨a1x, a1y, a1z⟩, ⟨a2x, a2y, a2z⟩, ⟨a3x, a3y, a3z⟩, ⟨a4x, a4y, a4z⟩,
    ⟨a5x, a5y, a5z⟩⟩ := A
  simp only [Cols.k1, Cols.k2, Cols.k3, Cols.k4, Cols.p12, Cols.p13, Cols.p14, Cols.p15,
    Cols.p23, Cols.p24, Cols.p25, Cols.p34, Cols.p35, Cols.p45, det] at *
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · linear_combination (-(a3y * a4z - a3z * a4y)) * hx + (-(a3z * a4x - a3x * a4z)) * hy +
      (-(a3x * a4y - a3y * a4x)) * hz
  · linear_combination (a2y * a4z - a2z * a4y) * hx + (a2z * a4x - a2x * a4z) * hy +
      (a2x * a4y - a2y * a4x) * hz
  · linear_combination (-(a2y * a3z - a2z * a3y)) * hx + (-(a2z * a3x - a2x * a3z)) * hy +
      (-(a2x * a3y - a2y * a3x)) * hz
  · linear_combination (-(a1y * a4z - a1z * a4y)) * hx + (-(a1z * a4x - a1x * a4z)) * hy +
      (-(a1x * a4y - a1y * a4x)) * hz
  · linear_combination (a1y * a3z - a1z * a3y) * hx + (a1z * a3x - a1x * a3z) * hy +
      (a1x * a3y - a1y * a3x) * hz
  · linear_combination (-(a1y * a2z - a1z * a2y)) * hx + (-(a1z * a2x - a1x * a2z)) * hy +
      (-(a1x * a2y - a1y * a2x)) * hz

/-- The binary forms on the basis `(w, k)`. -/
theorem Cols.pencil {w1 w2 w3 w4 w5 : ℝ} (h : A.Ker w1 w2 w3 w4 w5) :
    let a1 := w1 ^ 2 + w2 ^ 2 - w5 ^ 2
    let b1 := w1 * A.k1 + w2 * A.k2
    let c1 := A.k1 ^ 2 + A.k2 ^ 2
    let a2 := w3 ^ 2 + w4 ^ 2 - w5 ^ 2
    let b2 := w3 * A.k3 + w4 * A.k4
    let c2 := A.k3 ^ 2 + A.k4 ^ 2
    b1 ^ 2 - a1 * c1 = -(w5 ^ 2 * A.E0) ∧ b2 ^ 2 - a2 * c2 = -(w5 ^ 2 * A.E2) ∧
      res a1 b1 c1 a2 b2 c2 = w5 ^ 4 * A.F := by
  obtain ⟨h12, h13, h14, h23, h24, h34⟩ := A.plucker h
  exact pencil_alg w1 w2 w3 w4 w5 A.p12 A.p13 A.p14 A.p15 A.p23 A.p24
    A.p25 A.p34 A.p35 A.p45 h12 h13 h14 h23 h24 h34

/-- Cramer's rule in 3D. -/
theorem cramer (a b c T : V3 ℝ) (h : det a b c ≠ 0) :
    ∃ u v t : ℝ, u * a.x + v * b.x + t * c.x = T.x ∧ u * a.y + v * b.y + t * c.y = T.y ∧
      u * a.z + v * b.z + t * c.z = T.z := by
  refine ⟨det T b c / det a b c, det a T c / det a b c, det a b T / det a b c, ?_, ?_, ?_⟩ <;>
  · field_simp; simp only [det]; ring

/-- General position: the four direction columns span R^3. -/
def Cols.General : Prop := A.k1 ≠ 0 ∨ A.k2 ≠ 0 ∨ A.k3 ≠ 0 ∨ A.k4 ≠ 0

/-- In general position the inhomogeneous system has a solution (`w5 = 1`). -/
theorem Cols.particular (hg : A.General) : ∃ w1 w2 w3 w4 : ℝ, A.Ker w1 w2 w3 w4 1 := by
  obtain ⟨⟨a1x, a1y, a1z⟩, ⟨a2x, a2y, a2z⟩, ⟨a3x, a3y, a3z⟩, ⟨a4x, a4y, a4z⟩,
    ⟨a5x, a5y, a5z⟩⟩ := A
  simp only [Cols.General, Cols.k1, Cols.k2, Cols.k3, Cols.k4, Cols.p15, Cols.p25, Cols.p35,
    Cols.p45, neg_ne_zero] at hg
  simp only [Cols.Ker]
  rcases hg with h | h | h | h
  · obtain ⟨u, v, t, hx, hy, hz⟩ := cramer ⟨a2x, a2y, a2z⟩ ⟨a3x, a3y, a3z⟩ ⟨a4x, a4y, a4z⟩
      ⟨-a5x, -a5y, -a5z⟩ h
    exact ⟨0, u, v, t, by simp at *; linarith, by simp at *; linarith, by simp at *; linarith⟩
  · obtain ⟨u, v, t, hx, hy, hz⟩ := cramer ⟨a1x, a1y, a1z⟩ ⟨a3x, a3y, a3z⟩ ⟨a4x, a4y, a4z⟩
      ⟨-a5x, -a5y, -a5z⟩ h
    exact ⟨u, 0, v, t, by simp at *; linarith, by simp at *; linarith, by simp at *; linarith⟩
  · obtain ⟨u, v, t, hx, hy, hz⟩ := cramer ⟨a1x, a1y, a1z⟩ ⟨a2x, a2y, a2z⟩ ⟨a4x, a4y, a4z⟩
      ⟨-a5x, -a5y, -a5z⟩ h
    exact ⟨u, v, 0, t, by simp at *; linarith, by simp at *; linarith, by simp at *; linarith⟩
  · obtain ⟨u, v, t, hx, hy, hz⟩ := cramer ⟨a1x, a1y, a1z⟩ ⟨a2x, a2y, a2z⟩ ⟨a3x, a3y, a3z⟩
      ⟨-a5x, -a5y, -a5z⟩ h
    exact ⟨u, v, t, 0, by simp at *; linarith, by simp at *; linarith, by simp at *; linarith⟩

/-- The column-level meet: a kernel point with `w5 = 1` on both unit circles. -/
def Cols.Meet : Prop :=
  ∃ w1 w2 w3 w4 : ℝ, A.Ker w1 w2 w3 w4 1 ∧ w1 ^ 2 + w2 ^ 2 = 1 ∧ w3 ^ 2 + w4 ^ 2 = 1

theorem Cols.meet_imp (hm : A.Meet) : A.F = 0 ∧ A.E0 ≤ 0 ∧ A.E2 ≤ 0 := by
  obtain ⟨w1, w2, w3, w4, hk, h1, h2⟩ := hm
  obtain ⟨d0, d2, hr⟩ := A.pencil hk
  have z1 : bq (w1 ^ 2 + w2 ^ 2 - 1 ^ 2) (w1 * A.k1 + w2 * A.k2) (A.k1 ^ 2 + A.k2 ^ 2) 1 0 = 0 := by
    unfold bq; rw [h1]; ring
  have z2 : bq (w3 ^ 2 + w4 ^ 2 - 1 ^ 2) (w3 * A.k3 + w4 * A.k4) (A.k3 ^ 2 + A.k4 ^ 2) 1 0 = 0 := by
    unfold bq; rw [h2]; ring
  have nz : (1 : ℝ) ≠ 0 ∨ (0 : ℝ) ≠ 0 := Or.inl one_ne_zero
  refine ⟨?_, ?_, ?_⟩
  · have := res_of_common z1 z2 nz
    rw [hr] at this; simpa using this
  · have := disc_of_zero z1 nz; rw [d0] at this; nlinarith
  · have := disc_of_zero z2 nz; rw [d2] at this; nlinarith

/-- From a common zero `(s, t)` with `s != 0` of the two forms on `(w, k)`, a meet. -/
theorem Cols.meet_of_zero {w1 w2 w3 w4 s t : ℝ} (hk : A.Ker w1 w2 w3 w4 1) (hs : s ≠ 0)
    (z1 : bq (w1 ^ 2 + w2 ^ 2 - 1 ^ 2) (w1 * A.k1 + w2 * A.k2) (A.k1 ^ 2 + A.k2 ^ 2) s t = 0)
    (z2 : bq (w3 ^ 2 + w4 ^ 2 - 1 ^ 2) (w3 * A.k3 + w4 * A.k4) (A.k3 ^ 2 + A.k4 ^ 2) s t = 0) :
    A.Meet := by
  set m := t / s
  refine ⟨w1 + m * A.k1, w2 + m * A.k2, w3 + m * A.k3, w4 + m * A.k4, ?_, ?_, ?_⟩
  · obtain ⟨kx, ky, kz⟩ := A.ker_k
    obtain ⟨hx, hy, hz⟩ := hk
    refine ⟨by linear_combination hx + m * kx, by linear_combination hy + m * ky,
      by linear_combination hz + m * kz⟩
  · unfold bq at z1
    have hs2 : s ^ 2 ≠ 0 := pow_ne_zero 2 hs
    have : (w1 + m * A.k1) ^ 2 + (w2 + m * A.k2) ^ 2 - 1 = (bq (w1 ^ 2 + w2 ^ 2 - 1 ^ 2)
        (w1 * A.k1 + w2 * A.k2) (A.k1 ^ 2 + A.k2 ^ 2) s t) / s ^ 2 := by
      unfold bq; simp only [m]; field_simp; ring
    unfold bq at this; rw [z1, zero_div] at this; linarith
  · unfold bq at z2
    have hs2 : s ^ 2 ≠ 0 := pow_ne_zero 2 hs
    have : (w3 + m * A.k3) ^ 2 + (w4 + m * A.k4) ^ 2 - 1 = (bq (w3 ^ 2 + w4 ^ 2 - 1 ^ 2)
        (w3 * A.k3 + w4 * A.k4) (A.k3 ^ 2 + A.k4 ^ 2) s t) / s ^ 2 := by
      unfold bq; simp only [m]; field_simp; ring
    unfold bq at this; rw [z2, zero_div] at this; linarith

/-- The converse, in general position, from either circle's discriminant condition, provided
that circle's form is not identically zero (`c != 0`). -/
theorem Cols.meet_of (hg : A.General) (hF : A.F = 0)
    (hc : (A.k1 ^ 2 + A.k2 ^ 2 ≠ 0 ∧ A.E0 ≤ 0) ∨ (A.k3 ^ 2 + A.k4 ^ 2 ≠ 0 ∧ A.E2 ≤ 0)) :
    A.Meet := by
  obtain ⟨w1, w2, w3, w4, hk⟩ := A.particular hg
  obtain ⟨d0, d2, hr⟩ := A.pencil hk
  rw [hF, mul_zero] at hr
  have k0 : ¬ (A.k1 ^ 2 + A.k2 ^ 2 = 0 ∧ A.k3 ^ 2 + A.k4 ^ 2 = 0) := by
    rintro ⟨h1, h2⟩
    have : A.k1 = 0 ∧ A.k2 = 0 ∧ A.k3 = 0 ∧ A.k4 = 0 := by
      refine ⟨?_, ?_, ?_, ?_⟩ <;> nlinarith [sq_nonneg A.k1, sq_nonneg A.k2, sq_nonneg A.k3,
        sq_nonneg A.k4]
    rcases hg with h | h | h | h <;> tauto
  -- a common zero with s = 0 would need both c's zero
  have fin : ∀ s t : ℝ, (s ≠ 0 ∨ t ≠ 0) →
      bq (w1 ^ 2 + w2 ^ 2 - 1 ^ 2) (w1 * A.k1 + w2 * A.k2) (A.k1 ^ 2 + A.k2 ^ 2) s t = 0 →
      bq (w3 ^ 2 + w4 ^ 2 - 1 ^ 2) (w3 * A.k3 + w4 * A.k4) (A.k3 ^ 2 + A.k4 ^ 2) s t = 0 →
      A.Meet := by
    intro s t hst z1 z2
    rcases eq_or_ne s 0 with hs | hs
    · subst hs
      have ht : t ≠ 0 := hst.resolve_left (by simp)
      unfold bq at z1 z2
      have ht2 : t ^ 2 ≠ 0 := pow_ne_zero 2 ht
      exact absurd ⟨by
        have : (A.k1 ^ 2 + A.k2 ^ 2) * t ^ 2 = 0 := by linarith
        exact (mul_eq_zero.mp this).resolve_right ht2, by
        have : (A.k3 ^ 2 + A.k4 ^ 2) * t ^ 2 = 0 := by linarith
        exact (mul_eq_zero.mp this).resolve_right ht2⟩ k0
    · exact A.meet_of_zero hk hs z1 z2
  rcases hc with ⟨hc, he⟩ | ⟨hc, he⟩
  · obtain ⟨s, t, hst, z1, z2⟩ := common_of_res hr (by rw [d0]; nlinarith) (Or.inr (Or.inr hc))
    exact fin s t hst z1 z2
  · rw [res_comm] at hr
    obtain ⟨s, t, hst, z2, z1⟩ := common_of_res hr (by rw [d2]; nlinarith) (Or.inr (Or.inr hc))
    exact fin s t hst z1 z2

end Kernel

/-! Sections. -/

/-- Two section ellipses share a point. -/
def Sec.Meet (e1 e2 : Sec ℝ) : Prop :=
  ∃ α β γ δ : ℝ, α ^ 2 + β ^ 2 = 1 ∧ γ ^ 2 + δ ^ 2 = 1 ∧
    e1.c.x + α * e1.X.x + β * e1.Y.x = e2.c.x + γ * e2.X.x + δ * e2.Y.x ∧
    e1.c.y + α * e1.X.y + β * e1.Y.y = e2.c.y + γ * e2.X.y + δ * e2.Y.y ∧
    e1.c.z + α * e1.X.z + β * e1.Y.z = e2.c.z + γ * e2.X.z + δ * e2.Y.z

theorem sec_meet_cols (e1 e2 : Sec ℝ) : e1.Meet e2 ↔ (cols e1 e2).Meet := by
  simp only [Sec.Meet, Cols.Meet, Cols.Ker, cols, neg, sub]
  constructor
  · rintro ⟨a, b, c, d, h1, h2, hx, hy, hz⟩
    exact ⟨a, b, c, d, ⟨by linarith, by linarith, by linarith⟩, h1, h2⟩
  · rintro ⟨a, b, c, d, ⟨hx, hy, hz⟩, h1, h2⟩
    exact ⟨a, b, c, d, h1, h2, by linarith, by linarith, by linarith⟩

/-- Every meet satisfies `F = 0, E0 <= 0, E2 <= 0`; no hypothesis. -/
theorem meet_imp (e1 e2 : Sec ℝ) (h : e1.Meet e2) :
    (cols e1 e2).F = 0 ∧ (cols e1 e2).E0 ≤ 0 ∧ (cols e1 e2).E2 ≤ 0 :=
  (cols e1 e2).meet_imp ((sec_meet_cols e1 e2).mp h)

/-- The exact meet condition for two sections in general position. -/
theorem meet_iff (e1 e2 : Sec ℝ) (hg : (cols e1 e2).General) :
    e1.Meet e2 ↔ (cols e1 e2).F = 0 ∧ (cols e1 e2).E0 ≤ 0 ∧ (cols e1 e2).E2 ≤ 0 := by
  refine ⟨meet_imp e1 e2, fun ⟨hF, h0, h2⟩ => (sec_meet_cols e1 e2).mpr ?_⟩
  set A := cols e1 e2
  by_cases hc : A.k1 ^ 2 + A.k2 ^ 2 = 0
  · refine A.meet_of hg hF (Or.inr ⟨?_, h2⟩)
    intro hc'
    have : A.k1 = 0 ∧ A.k2 = 0 ∧ A.k3 = 0 ∧ A.k4 = 0 := by
      refine ⟨?_, ?_, ?_, ?_⟩ <;> nlinarith [sq_nonneg A.k1, sq_nonneg A.k2, sq_nonneg A.k3,
        sq_nonneg A.k4]
    rcases hg with h | h | h | h <;> tauto
  · exact A.meet_of hg hF (Or.inl ⟨hc, h0⟩)

/-- If ellipse 2 is not degenerate (`X2 x Y2 != 0`), one side condition suffices. -/
theorem meet_iff_of_ne (e1 e2 : Sec ℝ) (hg : (cols e1 e2).General)
    (hn : cross e2.X e2.Y ≠ ⟨0, 0, 0⟩) :
    e1.Meet e2 ↔ (cols e1 e2).F = 0 ∧ (cols e1 e2).E0 ≤ 0 := by
  refine ⟨fun h => ⟨(meet_imp e1 e2 h).1, (meet_imp e1 e2 h).2.1⟩,
    fun ⟨hF, h0⟩ => (sec_meet_cols e1 e2).mpr ?_⟩
  set A := cols e1 e2 with hA
  refine A.meet_of hg hF (Or.inl ⟨?_, h0⟩)
  intro hc
  have k1 : A.k1 = 0 := by nlinarith [sq_nonneg A.k1, sq_nonneg A.k2]
  have k2 : A.k2 = 0 := by nlinarith [sq_nonneg A.k1, sq_nonneg A.k2]
  -- then k3 (-X2) + k4 (-Y2) = 0, so k3 X2 x Y2 = 0 = k4 X2 x Y2
  obtain ⟨kx, ky, kz⟩ := A.ker_k
  rw [k1, k2, show A.A3 = neg e2.X from rfl, show A.A4 = neg e2.Y from rfl] at kx ky kz
  simp only [neg] at kx ky kz
  have n3 : A.k3 * (cross e2.X e2.Y).x = 0 ∧ A.k3 * (cross e2.X e2.Y).y = 0 ∧
      A.k3 * (cross e2.X e2.Y).z = 0 := by
    simp only [cross]
    refine ⟨?_, ?_, ?_⟩
    · linear_combination (-e2.Y.z) * ky + e2.Y.y * kz
    · linear_combination e2.Y.z * kx - e2.Y.x * kz
    · linear_combination (-e2.Y.y) * kx + e2.Y.x * ky
  have n4 : A.k4 * (cross e2.X e2.Y).x = 0 ∧ A.k4 * (cross e2.X e2.Y).y = 0 ∧
      A.k4 * (cross e2.X e2.Y).z = 0 := by
    simp only [cross]
    refine ⟨?_, ?_, ?_⟩
    · linear_combination e2.X.z * ky - e2.X.y * kz
    · linear_combination (-e2.X.z) * kx + e2.X.x * kz
    · linear_combination e2.X.y * kx - e2.X.x * ky
  have hne : (cross e2.X e2.Y).x ≠ 0 ∨ (cross e2.X e2.Y).y ≠ 0 ∨ (cross e2.X e2.Y).z ≠ 0 := by
    by_contra hh; push Not at hh; exact hn (V3.ext hh.1 hh.2.1 hh.2.2)
  have k3 : A.k3 = 0 := by
    rcases hne with h | h | h
    · exact (mul_eq_zero.mp n3.1).resolve_right h
    · exact (mul_eq_zero.mp n3.2.1).resolve_right h
    · exact (mul_eq_zero.mp n3.2.2).resolve_right h
  have k4 : A.k4 = 0 := by
    rcases hne with h | h | h
    · exact (mul_eq_zero.mp n4.1).resolve_right h
    · exact (mul_eq_zero.mp n4.2.1).resolve_right h
    · exact (mul_eq_zero.mp n4.2.2).resolve_right h
  rcases hg with h | h | h | h <;> contradiction

/-- Arcs with one common sweep blend into one ellipse family (each coordinate): `sum_k w_k
(o_k + R_k (x_k cos(a_k + phi) + y_k sin(a_k + phi))) = c + cos phi X + sin phi Y`. -/
theorem arc_family {ι : Type} (s : Finset ι) (w o R x y a : ι → ℝ) (φ : ℝ) :
    ∑ k ∈ s, w k * (o k + R k * (x k * Real.cos (a k + φ) + y k * Real.sin (a k + φ))) =
      ∑ k ∈ s, w k * o k +
        Real.cos φ * ∑ k ∈ s, w k * R k * (x k * Real.cos (a k) + y k * Real.sin (a k)) +
        Real.sin φ * ∑ k ∈ s, w k * R k * (y k * Real.cos (a k) - x k * Real.sin (a k)) := by
  rw [Finset.mul_sum, Finset.mul_sum, ← Finset.sum_add_distrib, ← Finset.sum_add_distrib]
  refine Finset.sum_congr rfl fun k _ => ?_
  rw [Real.cos_add, Real.sin_add]; ring

end LoftSSI
