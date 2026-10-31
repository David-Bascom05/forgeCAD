import Proofs.General
import Proofs.Degree

/-!
# Degrees of the general section-pair polynomial, tightness, and the volute case

If one section's data `c, X, Y` are polynomials of degree `<= d` in its spline parameter (`d = 3`
for a cubic spline span, natural or clamped) and the other section is fixed, then `F` is a
polynomial of degree `<= 8 d` in that parameter (`deg_s1`, `deg_s2`): bidegree `(24, 24)` for cubic
data. `witness_s1`, `witness_s2` reach 24. In the volute class the general `F` is the square of the
volute `F` (`volute_F`).
-/

namespace LoftSSI

open Polynomial V3

section Map

variable {R S : Type} [CommRing R] [CommRing S] (φ : R →+* S)

def Sec.map (e : Sec R) : Sec S := ⟨e.c.map φ, e.X.map φ, e.Y.map φ⟩

/-- `F` commutes with ring maps (evaluation at `s`, constants into polynomials). -/
theorem F_map (e1 e2 : Sec R) : (cols (e1.map φ) (e2.map φ)).F = φ (cols e1 e2).F := by
  simp only [Cols.F, Cols.E0, Cols.E1, Cols.E2, Cols.p12, Cols.p13, Cols.p14, Cols.p15, Cols.p23,
    Cols.p24, Cols.p25, Cols.p34, Cols.p35, Cols.p45, cols, Sec.map, V3.map, det, neg, sub,
    map_sub, map_add, map_mul, map_pow, map_neg, map_ofNat]

end Map

/-- Every component has degree `<= d`. -/
def V3.deg (v : V3 ℝ[X]) (d : ℕ) : Prop :=
  v.x.natDegree ≤ d ∧ v.y.natDegree ≤ d ∧ v.z.natDegree ≤ d

theorem V3.deg_mono {v : V3 ℝ[X]} {d e : ℕ} (h : v.deg d) (hde : d ≤ e) : v.deg e :=
  ⟨h.1.trans hde, h.2.1.trans hde, h.2.2.trans hde⟩

theorem V3.deg_neg {v : V3 ℝ[X]} {d : ℕ} (h : v.deg d) : (neg v).deg d := by
  simp only [V3.deg, neg, natDegree_neg]; exact h

theorem V3.deg_sub {v w : V3 ℝ[X]} {d : ℕ} (h : v.deg d) (h' : w.deg d) : (sub v w).deg d :=
  ⟨dsub h.1 h'.1, dsub h.2.1 h'.2.1, dsub h.2.2 h'.2.2⟩

theorem V3.deg_det {a b c : V3 ℝ[X]} {da db dc : ℕ} (ha : a.deg da) (hb : b.deg db)
    (hc : c.deg dc) : (det a b c).natDegree ≤ da + db + dc := by
  obtain ⟨a1, a2, a3⟩ := ha
  obtain ⟨b1, b2, b3⟩ := hb
  obtain ⟨c1, c2, c3⟩ := hc
  have t : ∀ {p q r s u : ℝ[X]}, p.natDegree ≤ da → q.natDegree ≤ db → r.natDegree ≤ dc →
      s.natDegree ≤ db → u.natDegree ≤ dc →
      (p * (q * r - s * u)).natDegree ≤ da + db + dc := by
    intro p q r s u hp hq hr hs hu
    rw [add_assoc]; exact dmul hp (dsub (dmul hq hr) (dmul hs hu))
  exact dadd (dsub (t a1 b2 c3 b3 c2) (t a2 b1 c3 b3 c1)) (t a3 b1 c2 b2 c1)

/-- Constants as polynomials. -/
noncomputable def Sec.const (e : Sec ℝ) : Sec ℝ[X] := e.map C

theorem Sec.const_deg (e : Sec ℝ) :
    e.const.c.deg 0 ∧ e.const.X.deg 0 ∧ e.const.Y.deg 0 := by
  simp only [Sec.const, Sec.map, V3.map, V3.deg, natDegree_C, le_refl, and_self]

theorem Sec.eval_const (e : Sec ℝ) (s : ℝ) : e.const.map (evalRingHom s) = e := by
  simp only [Sec.const, Sec.map, V3.map, coe_evalRingHom, eval_C]

/-- Degree in wall 1's parameter: wall 1's section data of degree `<= d`, wall 2's section fixed.
Then `F` (as a function of `s`) is a polynomial of degree `<= 8 d`. -/
theorem deg_s1_poly (e1 : Sec ℝ[X]) (e2 : Sec ℝ) (d : ℕ) (hc : e1.c.deg d) (hX : e1.X.deg d)
    (hY : e1.Y.deg d) : (cols e1 e2.const).F.natDegree ≤ 8 * d := by
  · obtain ⟨c2, X2, Y2⟩ := e2.const_deg
    have A1 : (cols e1 e2.const).A1.deg d := hX
    have A2 : (cols e1 e2.const).A2.deg d := hY
    have A3 : (cols e1 e2.const).A3.deg 0 := V3.deg_neg X2
    have A4 : (cols e1 e2.const).A4.deg 0 := V3.deg_neg Y2
    have A5 : (cols e1 e2.const).A5.deg d := V3.deg_sub hc (V3.deg_mono c2 (Nat.zero_le d))
    set A := cols e1 e2.const
    have p12 : A.p12.natDegree ≤ d := by
      have := V3.deg_det A3 A4 A5; simp only [Cols.p12]; omega
    have p13 : A.p13.natDegree ≤ 2 * d := by
      have := V3.deg_det A2 A4 A5; simp only [Cols.p13]; omega
    have p14 : A.p14.natDegree ≤ 2 * d := by
      have := V3.deg_det A2 A3 A5; simp only [Cols.p14]; omega
    have p15 : A.p15.natDegree ≤ d := by
      have := V3.deg_det A2 A3 A4; simp only [Cols.p15]; omega
    have p23 : A.p23.natDegree ≤ 2 * d := by
      have := V3.deg_det A1 A4 A5; simp only [Cols.p23]; omega
    have p24 : A.p24.natDegree ≤ 2 * d := by
      have := V3.deg_det A1 A3 A5; simp only [Cols.p24]; omega
    have p25 : A.p25.natDegree ≤ d := by
      have := V3.deg_det A1 A3 A4; simp only [Cols.p25]; omega
    have p34 : A.p34.natDegree ≤ 3 * d := by
      have := V3.deg_det A1 A2 A5; simp only [Cols.p34]; omega
    have p35 : A.p35.natDegree ≤ 2 * d := by
      have := V3.deg_det A1 A2 A4; simp only [Cols.p35]; omega
    have p45 : A.p45.natDegree ≤ 2 * d := by
      have := V3.deg_det A1 A2 A3; simp only [Cols.p45]; omega
    have sq : ∀ {p : ℝ[X]} {a b : ℕ}, p.natDegree ≤ a → 2 * a ≤ b → (p ^ 2).natDegree ≤ b :=
      fun h hab => dmono (dsq h) hab
    have E0 : A.E0.natDegree ≤ 2 * d :=
      dsub (dsub (sq p12 le_rfl) (sq p15 le_rfl)) (sq p25 le_rfl)
    have E1 : A.E1.natDegree ≤ 4 * d := by
      unfold Cols.E1
      refine dsub (dsub (dsub (dsub (dadd (dadd (dadd (sq p13 ?_) (sq p14 ?_)) (sq p23 ?_))
        (sq p24 ?_)) (sq p15 ?_)) (sq p25 ?_)) (sq p35 ?_)) (sq p45 ?_) <;> omega
    have E2 : A.E2.natDegree ≤ 6 * d := by
      unfold Cols.E2
      refine dsub (dsub (sq p34 ?_) (sq p35 ?_)) (sq p45 ?_) <;> omega
    unfold Cols.F
    refine dsub (sq E1 (by omega)) ?_
    exact dmono (dmul (dmul (dC 4 0) E0) E2) (by omega)

theorem deg_s1 (e1 : Sec ℝ[X]) (e2 : Sec ℝ) (d : ℕ) (hc : e1.c.deg d) (hX : e1.X.deg d)
    (hY : e1.Y.deg d) :
    ∃ P : ℝ[X], P.natDegree ≤ 8 * d ∧
      ∀ s : ℝ, (cols (e1.map (evalRingHom s)) e2).F = P.eval s := by
  refine ⟨(cols e1 e2.const).F, deg_s1_poly e1 e2 d hc hX hY, fun s => ?_⟩
  have h := F_map (evalRingHom s) e1 e2.const
  rw [Sec.eval_const] at h; simpa using h

/-- Degree in wall 2's parameter (wall 1's section fixed): also `<= 8 d`. -/
theorem deg_s2 (e1 : Sec ℝ) (e2 : Sec ℝ[X]) (d : ℕ) (hc : e2.c.deg d) (hX : e2.X.deg d)
    (hY : e2.Y.deg d) :
    ∃ P : ℝ[X], P.natDegree ≤ 8 * d ∧
      ∀ s : ℝ, (cols e1 (e2.map (evalRingHom s))).F = P.eval s := by
  refine ⟨(cols e1.const e2).F, ?_, fun s => ?_⟩
  · obtain ⟨c1, X1, Y1⟩ := e1.const_deg
    have A1 : (cols e1.const e2).A1.deg 0 := X1
    have A2 : (cols e1.const e2).A2.deg 0 := Y1
    have A3 : (cols e1.const e2).A3.deg d := V3.deg_neg hX
    have A4 : (cols e1.const e2).A4.deg d := V3.deg_neg hY
    have A5 : (cols e1.const e2).A5.deg d := V3.deg_sub (V3.deg_mono c1 (Nat.zero_le d)) hc
    set A := cols e1.const e2
    have p12 : A.p12.natDegree ≤ 3 * d := by
      have := V3.deg_det A3 A4 A5; simp only [Cols.p12]; omega
    have p13 : A.p13.natDegree ≤ 2 * d := by
      have := V3.deg_det A2 A4 A5; simp only [Cols.p13]; omega
    have p14 : A.p14.natDegree ≤ 2 * d := by
      have := V3.deg_det A2 A3 A5; simp only [Cols.p14]; omega
    have p15 : A.p15.natDegree ≤ 2 * d := by
      have := V3.deg_det A2 A3 A4; simp only [Cols.p15]; omega
    have p23 : A.p23.natDegree ≤ 2 * d := by
      have := V3.deg_det A1 A4 A5; simp only [Cols.p23]; omega
    have p24 : A.p24.natDegree ≤ 2 * d := by
      have := V3.deg_det A1 A3 A5; simp only [Cols.p24]; omega
    have p25 : A.p25.natDegree ≤ 2 * d := by
      have := V3.deg_det A1 A3 A4; simp only [Cols.p25]; omega
    have p34 : A.p34.natDegree ≤ d := by
      have := V3.deg_det A1 A2 A5; simp only [Cols.p34]; omega
    have p35 : A.p35.natDegree ≤ d := by
      have := V3.deg_det A1 A2 A4; simp only [Cols.p35]; omega
    have p45 : A.p45.natDegree ≤ d := by
      have := V3.deg_det A1 A2 A3; simp only [Cols.p45]; omega
    have sq : ∀ {p : ℝ[X]} {a b : ℕ}, p.natDegree ≤ a → 2 * a ≤ b → (p ^ 2).natDegree ≤ b :=
      fun h hab => dmono (dsq h) hab
    have E0 : A.E0.natDegree ≤ 6 * d := by
      unfold Cols.E0
      refine dsub (dsub (sq p12 ?_) (sq p15 ?_)) (sq p25 ?_) <;> omega
    have E1 : A.E1.natDegree ≤ 4 * d := by
      unfold Cols.E1
      refine dsub (dsub (dsub (dsub (dadd (dadd (dadd (sq p13 ?_) (sq p14 ?_)) (sq p23 ?_))
        (sq p24 ?_)) (sq p15 ?_)) (sq p25 ?_)) (sq p35 ?_)) (sq p45 ?_) <;> omega
    have E2 : A.E2.natDegree ≤ 2 * d := by
      unfold Cols.E2
      refine dsub (dsub (sq p34 ?_) (sq p35 ?_)) (sq p45 ?_) <;> omega
    unfold Cols.F
    refine dsub (sq E1 (by omega)) ?_
    exact dmono (dmul (dmul (dC 4 0) E0) E2) (by omega)
  · have h := F_map (evalRingHom s) e1.const e2
    rw [Sec.eval_const] at h; simpa using h

/-- Wall 1: a circle of radius `s^3` in the plane `z = s^3`. -/
noncomputable def wit1 : Sec ℝ[X] := ⟨⟨0, 0, X ^ 3⟩, ⟨X ^ 3, 0, 0⟩, ⟨0, X ^ 3, 0⟩⟩

/-- Wall 2 (fixed): the unit circle in the plane `x = 1`. -/
def wit2 : Sec ℝ := ⟨⟨1, 0, 0⟩, ⟨0, 1, 0⟩, ⟨0, 0, 1⟩⟩

/-- The bound 24 in `s1` is reached: `F = 4 s^24 - 8 s^18 + 4 s^12`. -/
theorem witness_s1 : (cols wit1 wit2.const).F = 4 * X ^ 24 - 8 * X ^ 18 + 4 * X ^ 12 ∧
    (cols wit1 wit2.const).F.natDegree = 24 := by
  have e : (cols wit1 wit2.const).F = 4 * X ^ 24 - 8 * X ^ 18 + 4 * X ^ 12 := by
    simp only [Cols.F, Cols.E0, Cols.E1, Cols.E2, Cols.p12, Cols.p13, Cols.p14, Cols.p15,
      Cols.p23, Cols.p24, Cols.p25, Cols.p34, Cols.p35, Cols.p45, cols, wit1, wit2, Sec.const,
      Sec.map, V3.map, det, neg, sub, map_one, map_zero]
    ring
  refine ⟨e, ?_⟩
  rw [e]; compute_degree!

/-- The bound 24 in `s2` is reached (the roles swapped). -/
theorem witness_s2 : (cols wit2.const wit1).F = 4 * X ^ 24 - 8 * X ^ 18 + 4 * X ^ 12 ∧
    (cols wit2.const wit1).F.natDegree = 24 := by
  have e : (cols wit2.const wit1).F = 4 * X ^ 24 - 8 * X ^ 18 + 4 * X ^ 12 := by
    simp only [Cols.F, Cols.E0, Cols.E1, Cols.E2, Cols.p12, Cols.p13, Cols.p14, Cols.p15,
      Cols.p23, Cols.p24, Cols.p25, Cols.p34, Cols.p35, Cols.p45, cols, wit1, wit2, Sec.const,
      Sec.map, V3.map, det, neg, sub, map_one, map_zero]
    ring
  refine ⟨e, ?_⟩
  rw [e]; compute_degree!

/-- A volute-class section: `P + cos phi A - sin phi rho e_z` (A, P horizontal). -/
def volSec {R : Type} [CommRing R] (Ax Ay Px Py r : R) : Sec R :=
  ⟨⟨Px, Py, 0⟩, ⟨Ax, Ay, 0⟩, ⟨0, 0, -r⟩⟩

/-- In the volute class the general polynomial is the square of the volute one, and the general
side conditions are the volute ones: `E0 = -rho2^2 (W^2 - a2^2)`, `E2 = -rho1^2 (W^2 - a1^2)`. -/
theorem volute_F {R : Type} [CommRing R] (A1x A1y P1x P1y r1 A2x A2y P2x P2y r2 : R) :
    let W := A1x * A2y - A1y * A2x
    let a1 := (P2x - P1x) * A1y - (P2y - P1y) * A1x
    let a2 := (P2x - P1x) * A2y - (P2y - P1y) * A2x
    let A := cols (volSec A1x A1y P1x P1y r1) (volSec A2x A2y P2x P2y r2)
    A.F = (r1 ^ 2 * (W ^ 2 - a2 ^ 2) - r2 ^ 2 * (W ^ 2 - a1 ^ 2)) ^ 2 ∧
      A.E0 = -(r2 ^ 2 * (W ^ 2 - a2 ^ 2)) ∧ A.E2 = -(r1 ^ 2 * (W ^ 2 - a1 ^ 2)) := by
  intro W a1 a2 A
  simp only [A, W, a1, a2, Cols.F, Cols.E0, Cols.E1, Cols.E2, Cols.p12, Cols.p13, Cols.p14,
    Cols.p15, Cols.p23, Cols.p24, Cols.p25, Cols.p34, Cols.p35, Cols.p45, cols, volSec, det, neg,
    sub]
  refine ⟨by ring, by ring, by ring⟩

end LoftSSI
