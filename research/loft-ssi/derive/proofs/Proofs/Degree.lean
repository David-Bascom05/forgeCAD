import Mathlib

/-!
# Degree bounds

Per span, the section data `P, A, rho` of each wall are cubics in that wall's spline parameter.
In wall 1's parameter `s1` (wall 2's data constant): `g` has degree <= 6, `Psi` <= 18, `F` <= 12.
In wall 2's parameter `s2` (wall 1's data constant): `g` <= 3, `Psi` <= 6, `F` <= 12.
-/

namespace LoftSSI

open Polynomial

variable {p q : ℝ[X]} {a b : ℕ}

theorem dmul (hp : p.natDegree ≤ a) (hq : q.natDegree ≤ b) : (p * q).natDegree ≤ a + b :=
  natDegree_mul_le.trans (add_le_add hp hq)

theorem dadd (hp : p.natDegree ≤ a) (hq : q.natDegree ≤ a) : (p + q).natDegree ≤ a :=
  (natDegree_add_le p q).trans (max_le hp hq)

theorem dsub (hp : p.natDegree ≤ a) (hq : q.natDegree ≤ a) : (p - q).natDegree ≤ a :=
  (natDegree_sub_le p q).trans (max_le hp hq)

theorem dsq (hp : p.natDegree ≤ a) : (p ^ 2).natDegree ≤ 2 * a :=
  natDegree_pow_le.trans (Nat.mul_le_mul_left 2 hp)

theorem dmono (hp : p.natDegree ≤ a) (h : a ≤ b) : p.natDegree ≤ b := hp.trans h

theorem dC (x : ℝ) (n : ℕ) : (C x).natDegree ≤ n := by rw [natDegree_C]; exact Nat.zero_le n

/-- Wall 1's parameter: `A1, P1, r1` cubic, wall 2's point `(hx, hy)` and `z2sq = rho2^2 (1-c2^2)`
constant. -/
theorem degrees_s1 (A1x A1y P1x P1y r1 : ℝ[X]) (hx hy z2sq A2x A2y P2x P2y r2 : ℝ)
    (h1 : A1x.natDegree ≤ 3) (h2 : A1y.natDegree ≤ 3) (h3 : P1x.natDegree ≤ 3)
    (h4 : P1y.natDegree ≤ 3) (h5 : r1.natDegree ≤ 3) :
    (A1x * (C hy - P1y) - A1y * (C hx - P1x)).natDegree ≤ 6 ∧
    (r1 ^ 2 * ((C hx - P1x) * A1x + (C hy - P1y) * A1y) ^ 2 +
        (A1x ^ 2 + A1y ^ 2) ^ 2 * (C z2sq - r1 ^ 2)).natDegree ≤ 18 ∧
    (let W := A1x * C A2y - A1y * C A2x
     let a1 := (C P2x - P1x) * A1y - (C P2y - P1y) * A1x
     let a2 := (C P2x - P1x) * C A2y - (C P2y - P1y) * C A2x
     r1 ^ 2 * (W ^ 2 - a2 ^ 2) - C (r2 ^ 2) * (W ^ 2 - a1 ^ 2)).natDegree ≤ 12 := by
  have dx : (C hx - P1x).natDegree ≤ 3 := dsub (dC _ _) h3
  have dy : (C hy - P1y).natDegree ≤ 3 := dsub (dC _ _) h4
  refine ⟨?_, ?_, ?_⟩
  · exact dsub (dmul h1 dy) (dmul h2 dx)
  · have e1 : ((C hx - P1x) * A1x + (C hy - P1y) * A1y).natDegree ≤ 6 :=
      dadd (dmul dx h1) (dmul dy h2)
    have e2 : (A1x ^ 2 + A1y ^ 2).natDegree ≤ 6 := dadd (dsq h1) (dsq h2)
    exact dadd (dmono (dmul (dsq h5) (dsq e1)) (by norm_num))
      (dmono (dmul (dsq e2) (dsub (dmono (dC _ 6) le_rfl) (dsq h5))) (by norm_num))
  · dsimp only
    have hW : (A1x * C A2y - A1y * C A2x).natDegree ≤ 3 :=
      dsub (dmono (dmul h1 (dC _ 0)) le_rfl) (dmul h2 (dC _ 0))
    have hd : ∀ v : ℝ, (C v - P1x).natDegree ≤ 3 := fun v => dsub (dC _ _) h3
    have hd' : ∀ v : ℝ, (C v - P1y).natDegree ≤ 3 := fun v => dsub (dC _ _) h4
    have ha1 : ((C P2x - P1x) * A1y - (C P2y - P1y) * A1x).natDegree ≤ 6 :=
      dsub (dmul (hd _) h2) (dmul (hd' _) h1)
    have ha2 : ((C P2x - P1x) * C A2y - (C P2y - P1y) * C A2x).natDegree ≤ 3 :=
      dsub (dmul (hd _) (dC _ 0)) (dmul (hd' _) (dC _ 0))
    have hW2 : ((A1x * C A2y - A1y * C A2x) ^ 2).natDegree ≤ 12 := dmono (dsq hW) (by norm_num)
    have t1 : (r1 ^ 2 * ((A1x * C A2y - A1y * C A2x) ^ 2 -
        ((C P2x - P1x) * C A2y - (C P2y - P1y) * C A2x) ^ 2)).natDegree ≤ 12 :=
      dmono (dmul (dsq h5) (dsub (dsq hW) (dsq ha2))) (by norm_num)
    have t2 : (C (r2 ^ 2) * ((A1x * C A2y - A1y * C A2x) ^ 2 -
        ((C P2x - P1x) * A1y - (C P2y - P1y) * A1x) ^ 2)).natDegree ≤ 12 :=
      dmono (dmul (dC _ 0) (dsub hW2 (dmono (dsq ha1) (by norm_num)))) (by norm_num)
    exact dsub t1 t2

/-- Wall 2's parameter: `A2, P2, r2` cubic, wall 1's data and `c2` constant. -/
theorem degrees_s2 (A2x A2y P2x P2y r2 : ℝ[X]) (A1x A1y P1x P1y r1 c2 : ℝ)
    (h1 : A2x.natDegree ≤ 3) (h2 : A2y.natDegree ≤ 3) (h3 : P2x.natDegree ≤ 3)
    (h4 : P2y.natDegree ≤ 3) (h5 : r2.natDegree ≤ 3) :
    let hx := P2x + C c2 * A2x
    let hy := P2y + C c2 * A2y
    (C A1x * (hy - C P1y) - C A1y * (hx - C P1x)).natDegree ≤ 3 ∧
    (C (r1 ^ 2) * ((hx - C P1x) * C A1x + (hy - C P1y) * C A1y) ^ 2 +
        C ((A1x ^ 2 + A1y ^ 2) ^ 2) * (r2 ^ 2 * C (1 - c2 ^ 2) - C (r1 ^ 2))).natDegree ≤ 6 ∧
    (let W := C A1x * A2y - C A1y * A2x
     let a1 := (P2x - C P1x) * C A1y - (P2y - C P1y) * C A1x
     let a2 := (P2x - C P1x) * A2y - (P2y - C P1y) * A2x
     C (r1 ^ 2) * (W ^ 2 - a2 ^ 2) - r2 ^ 2 * (W ^ 2 - a1 ^ 2)).natDegree ≤ 12 := by
  intro hx hy
  have dhx : hx.natDegree ≤ 3 := dadd h3 (dmono (dmul (dC _ 0) h1) (by norm_num))
  have dhy : hy.natDegree ≤ 3 := dadd h4 (dmono (dmul (dC _ 0) h2) (by norm_num))
  have dx : (hx - C P1x).natDegree ≤ 3 := dsub dhx (dC _ _)
  have dy : (hy - C P1y).natDegree ≤ 3 := dsub dhy (dC _ _)
  refine ⟨?_, ?_, ?_⟩
  · exact dsub (dmono (dmul (dC _ 0) dy) (by norm_num)) (dmono (dmul (dC _ 0) dx) (by norm_num))
  · have e1 : ((hx - C P1x) * C A1x + (hy - C P1y) * C A1y).natDegree ≤ 3 :=
      dadd (dmono (dmul dx (dC _ 0)) (by norm_num)) (dmono (dmul dy (dC _ 0)) (by norm_num))
    exact dadd (dmono (dmul (dC _ 0) (dsq e1)) (by norm_num))
      (dmono (dmul (dC _ 0) (dsub (dmono (dmul (dsq h5) (dC _ 0)) (by norm_num)) (dC _ 6)))
        (by norm_num))
  · dsimp only
    have hW : (C A1x * A2y - C A1y * A2x).natDegree ≤ 3 :=
      dsub (dmono (dmul (dC _ 0) h2) (by norm_num)) (dmono (dmul (dC _ 0) h1) (by norm_num))
    have hd : (P2x - C P1x).natDegree ≤ 3 := dsub h3 (dC _ _)
    have hd' : (P2y - C P1y).natDegree ≤ 3 := dsub h4 (dC _ _)
    have ha1 : ((P2x - C P1x) * C A1y - (P2y - C P1y) * C A1x).natDegree ≤ 3 :=
      dsub (dmono (dmul hd (dC _ 0)) (by norm_num)) (dmono (dmul hd' (dC _ 0)) (by norm_num))
    have ha2 : ((P2x - C P1x) * A2y - (P2y - C P1y) * A2x).natDegree ≤ 6 :=
      dsub (dmul hd h2) (dmul hd' h1)
    have hW2 : ((C A1x * A2y - C A1y * A2x) ^ 2).natDegree ≤ 12 := dmono (dsq hW) (by norm_num)
    have t1 : (C (r1 ^ 2) * ((C A1x * A2y - C A1y * A2x) ^ 2 -
        ((P2x - C P1x) * A2y - (P2y - C P1y) * A2x) ^ 2)).natDegree ≤ 12 :=
      dmono (dmul (dC _ 0) (dsub hW2 (dmono (dsq ha2) (by norm_num)))) (by norm_num)
    have t2 : (r2 ^ 2 * ((C A1x * A2y - C A1y * A2x) ^ 2 -
        ((P2x - C P1x) * C A1y - (P2y - C P1y) * C A1x) ^ 2)).natDegree ≤ 12 :=
      dmono (dmul (dsq h5) (dsub (dsq hW) (dsq ha1))) (by norm_num)
    exact dsub t1 t2

end LoftSSI
