import Proofs.Section
import Proofs.GeneralDegree
import Proofs.Bernstein

/-!
# Which hypotheses are needed

Each hypothesis of the main theorems is either dropped by a stronger theorem (`pair_iff_sym`,
`on_section_mp`, `sections_meet_mp`, `meet_imp`, `bern3_pos_corners`, `volute_deg`) or shown
necessary here by a concrete counterexample, checked in Lean.
-/

namespace LoftSSI

open Polynomial V3

/-- The volute-class meet (the left side of `pair_iff`). -/
def vmeet (A1x A1y P1x P1y r1 A2x A2y P2x P2y r2 : ℝ) : Prop :=
  ∃ c1 s1 c2 s2 : ℝ, c1 ^ 2 + s1 ^ 2 = 1 ∧ c2 ^ 2 + s2 ^ 2 = 1 ∧
    P1x + c1 * A1x = P2x + c2 * A2x ∧ P1y + c1 * A1y = P2y + c2 * A2y ∧ s1 * r1 = s2 * r2

/-- `pair_iff` needs `W != 0`: parallel coplanar sections 5 apart (`A1 = A2 = (1, 0)`, `P2 = (5, 0)`,
unit radii) satisfy `F = 0` and both side conditions but do not meet. -/
theorem need_W :
    let W := cross 1 0 1 0
    let a1 := cross (5 - 0) (0 - 0) 1 0
    let a2 := cross (5 - 0) (0 - 0) 1 0
    W = 0 ∧ 1 ^ 2 * (W ^ 2 - a2 ^ 2) - 1 ^ 2 * (W ^ 2 - a1 ^ 2) = 0 ∧ a1 ^ 2 ≤ W ^ 2 ∧
      a2 ^ 2 ≤ W ^ 2 ∧ ¬ vmeet 1 0 0 0 1 1 0 5 0 1 := by
  intro W a1 a2
  refine ⟨by simp [W, cross], by simp [W, a1, a2, cross], by simp [W, a1, cross],
    by simp [W, a2, cross], ?_⟩
  rintro ⟨c1, s1, c2, s2, h1, h2, hx, -, -⟩
  nlinarith [sq_nonneg s1, sq_nonneg s2, sq_nonneg (c1 - 1), sq_nonneg (c2 + 1)]

/-- The side condition is needed: `A1 = (1, 0)`, `A2 = (0, 1)`, `P2 = (2, 2)`, unit radii give
`F = 0` (both brackets are -3) but `cos phi1 = 2`. -/
theorem need_side :
    let W := cross 1 0 0 1
    let a1 := cross (2 - 0) (2 - 0) 1 0
    let a2 := cross (2 - 0) (2 - 0) 0 1
    1 ^ 2 * (W ^ 2 - a2 ^ 2) - 1 ^ 2 * (W ^ 2 - a1 ^ 2) = 0 ∧ W ≠ 0 ∧
      ¬ vmeet 1 0 0 0 1 0 1 2 2 1 := by
  intro W a1 a2
  refine ⟨by simp [W, a1, a2, cross], by simp [W, cross], ?_⟩
  rintro ⟨c1, s1, c2, s2, h1, h2, hx, -, -⟩
  nlinarith [sq_nonneg s1]

/-- With only `a2^2 <= W^2`, `rho2 != 0` is needed: `rho2 = 0`, `P2 = (1, 2)` gives `F = 0`,
`a2^2 = W^2` but `cos phi2 = -2`. (`pair_iff_sym` adds `a1^2 <= W^2` instead.) -/
theorem need_r2 :
    let W := cross 1 0 0 1
    let a1 := cross (1 - 0) (2 - 0) 1 0
    let a2 := cross (1 - 0) (2 - 0) 0 1
    1 ^ 2 * (W ^ 2 - a2 ^ 2) - 0 ^ 2 * (W ^ 2 - a1 ^ 2) = 0 ∧ a2 ^ 2 ≤ W ^ 2 ∧ W ≠ 0 ∧
      ¬ vmeet 1 0 0 0 1 0 1 1 2 0 := by
  intro W a1 a2
  refine ⟨by simp [W, a2, cross], by simp [W, a2, cross], by simp [W, cross], ?_⟩
  rintro ⟨c1, s1, c2, s2, h1, h2, -, hy, -⟩
  nlinarith [sq_nonneg s2]

/-- `on_section_iff` (backward) needs `|A| != 0`: with `A = 0` every point has `g = Psi = 0`. -/
theorem need_A : g 0 0 0 0 5 0 = 0 ∧ psi 0 0 0 0 1 5 0 0 = 0 ∧
    ¬ ∃ c s : ℝ, c ^ 2 + s ^ 2 = 1 ∧ (5 : ℝ) = 0 + c * 0 ∧ (0 : ℝ) = 0 + c * 0 ∧
      (0 : ℝ) = -(s * 1) := by
  refine ⟨by simp [g, cross], by simp [psi], ?_⟩
  rintro ⟨c, s, -, h, -, -⟩; norm_num at h

/-- `on_section_iff` (backward) needs `rho != 0`: with `rho = 0` the point `(5, 0)` of the
section's line passes `g = Psi = 0` but lies off the segment `|c| <= 1`. -/
theorem need_rho : g 1 0 0 0 5 0 = 0 ∧ psi 1 0 0 0 0 5 0 0 = 0 ∧
    ¬ ∃ c s : ℝ, c ^ 2 + s ^ 2 = 1 ∧ (5 : ℝ) = 0 + c * 1 ∧ (0 : ℝ) = 0 + c * 0 ∧
      (0 : ℝ) = -(s * 0) := by
  refine ⟨by simp [g, cross], by simp [psi], ?_⟩
  rintro ⟨c, s, h1, h, -, -⟩
  nlinarith [sq_nonneg s]

/-- The volute formula needs the centres on one plane square to `e`: lift wall 2's centre by 5
and `F` (which never sees it) still vanishes with both side conditions, yet the circles miss.
(The general theorem `meet_iff` has no such hypothesis.) -/
theorem need_centres :
    let W := cross 1 0 0 1
    let a1 := cross (0 - 0) (0 - 0) 1 0
    let a2 := cross (0 - 0) (0 - 0) 0 1
    1 ^ 2 * (W ^ 2 - a2 ^ 2) - 1 ^ 2 * (W ^ 2 - a1 ^ 2) = 0 ∧ a1 ^ 2 ≤ W ^ 2 ∧ a2 ^ 2 ≤ W ^ 2 ∧
      ¬ Sec.Meet (volSec 1 0 0 0 1) ⟨⟨0, 0, 5⟩, ⟨0, 1, 0⟩, ⟨0, 0, -1⟩⟩ := by
  intro W a1 a2
  refine ⟨by simp [W, a1, a2, cross], by simp [W, a1, cross], by simp [W, a2, cross], ?_⟩
  rintro ⟨α, β, γ, δ, h1, h2, -, -, hz⟩
  simp [volSec] at hz
  nlinarith [sq_nonneg α, sq_nonneg γ, sq_nonneg (β + 1), sq_nonneg (δ - 1)]

/-- `meet_iff` needs general position: two unit circles in one plane, 10 apart, have every
Plücker coordinate zero (so `F = E0 = E2 = 0`) and do not meet. -/
theorem need_general :
    let e1 : Sec ℝ := ⟨⟨0, 0, 0⟩, ⟨1, 0, 0⟩, ⟨0, 1, 0⟩⟩
    let e2 : Sec ℝ := ⟨⟨10, 0, 0⟩, ⟨1, 0, 0⟩, ⟨0, 1, 0⟩⟩
    ¬ (cols e1 e2).General ∧ (cols e1 e2).F = 0 ∧ (cols e1 e2).E0 ≤ 0 ∧ (cols e1 e2).E2 ≤ 0 ∧
      ¬ e1.Meet e2 := by
  intro e1 e2
  simp only [Cols.General, Cols.k1, Cols.k2, Cols.k3, Cols.k4, Cols.F, Cols.E0, Cols.E1, Cols.E2,
    Cols.p12, Cols.p13, Cols.p14, Cols.p15, Cols.p23, Cols.p24, Cols.p25, Cols.p34, Cols.p35,
    Cols.p45, cols, e1, e2, det, neg, sub]
  refine ⟨by norm_num, by norm_num, by norm_num, by norm_num, ?_⟩
  rintro ⟨α, β, γ, δ, h1, h2, hx, -, -⟩
  simp at hx
  nlinarith [sq_nonneg β, sq_nonneg δ, sq_nonneg (α - 1), sq_nonneg (γ + 1)]

/-- With only `E0 <= 0`, `meet_iff_of_ne` needs `X2 x Y2 != 0`: `need_r2`'s sections embedded in
R^3 are in general position, have `F = 0`, `E0 = 0`, but `E2 = 3 > 0` and do not meet. -/
theorem need_E2 :
    let A := cols (volSec 1 0 0 0 1) (volSec 0 1 1 2 0)
    A.General ∧ A.F = 0 ∧ A.E0 ≤ 0 ∧ 0 < A.E2 ∧ ¬ (volSec 1 0 0 0 1).Meet (volSec 0 1 1 2 0) := by
  intro A
  simp only [A, Cols.General, Cols.k1, Cols.k2, Cols.k3, Cols.k4, Cols.F, Cols.E0, Cols.E1,
    Cols.E2, Cols.p12, Cols.p13, Cols.p14, Cols.p15, Cols.p23, Cols.p24, Cols.p25, Cols.p34,
    Cols.p35, Cols.p45, cols, volSec, det, neg, sub]
  refine ⟨by norm_num, by norm_num, by norm_num, by norm_num, ?_⟩
  rintro ⟨α, β, γ, δ, h1, h2, -, hy, -⟩
  simp at hy
  nlinarith [sq_nonneg δ]

/-- Weakening of `bern3_pos`: nonnegative coefficients with the eight corner coefficients
positive already give a positive polynomial on the closed cube. -/
theorem bern3_pos_corners (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ)
    (h0 : ∀ i ≤ n1, ∀ j ≤ n2, ∀ k ≤ n3, 0 ≤ b i j k)
    (hc : ∀ i ∈ ({0, n1} : Finset ℕ), ∀ j ∈ ({0, n2} : Finset ℕ), ∀ k ∈ ({0, n3} : Finset ℕ),
      0 < b i j k) {x y z : ℝ}
    (hx : 0 ≤ x ∧ x ≤ 1) (hy : 0 ≤ y ∧ y ≤ 1) (hz : 0 ≤ z ∧ z ≤ 1) :
    0 < bern3 n1 n2 n3 b x y z := by
  -- a corner index whose basis function is positive at t
  have corner : ∀ (n : ℕ) (t : ℝ), 0 ≤ t → t ≤ 1 →
      ∃ i ∈ ({0, n} : Finset ℕ), i ≤ n ∧ 0 < bb n i t := by
    intro n t h0 h1
    rcases lt_or_eq_of_le h1 with h | h
    · refine ⟨0, by simp, Nat.zero_le _, ?_⟩
      unfold bb; simp only [Nat.choose_zero_right, Nat.cast_one, pow_zero, one_mul, Nat.sub_zero]
      exact pow_pos (by linarith) n
    · refine ⟨n, by simp, le_rfl, ?_⟩
      subst h; unfold bb; simp
  obtain ⟨i, hi, hin, hbi⟩ := corner n1 x hx.1 hx.2
  obtain ⟨j, hj, hjn, hbj⟩ := corner n2 y hy.1 hy.2
  obtain ⟨k, hk, hkn, hbk⟩ := corner n3 z hz.1 hz.2
  have term : ∀ a ∈ Finset.range (n1 + 1), ∀ c ∈ Finset.range (n2 + 1),
      ∀ d ∈ Finset.range (n3 + 1), 0 ≤ b a c d * (bb n1 a x * bb n2 c y * bb n3 d z) := by
    intro a ha c hc' d hd
    simp only [Finset.mem_range] at ha hc' hd
    have := bb_nonneg n1 a hx.1 hx.2
    have := bb_nonneg n2 c hy.1 hy.2
    have := bb_nonneg n3 d hz.1 hz.2
    have := h0 a (by omega) c (by omega) d (by omega)
    positivity
  have pos : 0 < b i j k * (bb n1 i x * bb n2 j y * bb n3 k z) :=
    mul_pos (hc i hi j hj k hk) (by positivity)
  unfold bern3
  calc 0 < b i j k * (bb n1 i x * bb n2 j y * bb n3 k z) := pos
    _ ≤ ∑ d ∈ Finset.range (n3 + 1), b i j d * (bb n1 i x * bb n2 j y * bb n3 d z) :=
        Finset.single_le_sum (f := fun d => b i j d * (bb n1 i x * bb n2 j y * bb n3 d z))
          (fun d hd => term i (by simp; omega) j (by simp; omega) d hd) (by simp; omega)
    _ ≤ ∑ c ∈ Finset.range (n2 + 1), ∑ d ∈ Finset.range (n3 + 1),
          b i c d * (bb n1 i x * bb n2 c y * bb n3 d z) :=
        Finset.single_le_sum (f := fun c => ∑ d ∈ Finset.range (n3 + 1),
          b i c d * (bb n1 i x * bb n2 c y * bb n3 d z))
          (fun c hc' => Finset.sum_nonneg fun d hd => term i (by simp; omega) c hc' d hd)
          (by simp; omega)
    _ ≤ _ :=
        Finset.single_le_sum (f := fun a => ∑ c ∈ Finset.range (n2 + 1),
          ∑ d ∈ Finset.range (n3 + 1), b a c d * (bb n1 a x * bb n2 c y * bb n3 d z))
          (fun a ha => Finset.sum_nonneg fun c hc' => Finset.sum_nonneg fun d hd =>
            term a ha c hc' d hd) (by simp; omega)

/-- Volute degree for data of any degree `d`: `F <= 4 d` in `s1` (`12` for cubic spans, natural or
clamped), from the general bound and `volute_F` (`F_general = F_volute^2`). -/
theorem volute_deg (A1x A1y P1x P1y r1 : ℝ[X]) (A2x A2y P2x P2y r2 : ℝ) (d : ℕ)
    (h1 : A1x.natDegree ≤ d) (h2 : A1y.natDegree ≤ d) (h3 : P1x.natDegree ≤ d)
    (h4 : P1y.natDegree ≤ d) (h5 : r1.natDegree ≤ d) :
    let W := A1x * C A2y - A1y * C A2x
    let a1 := (C P2x - P1x) * A1y - (C P2y - P1y) * A1x
    let a2 := (C P2x - P1x) * C A2y - (C P2y - P1y) * C A2x
    (r1 ^ 2 * (W ^ 2 - a2 ^ 2) - C r2 ^ 2 * (W ^ 2 - a1 ^ 2)).natDegree ≤ 4 * d := by
  intro W a1 a2
  have hg := deg_s1_poly (volSec A1x A1y P1x P1y r1) (volSec A2x A2y P2x P2y r2) d
    (by simp only [V3.deg, volSec]; exact ⟨h3, h4, by simp⟩)
    (by simp only [V3.deg, volSec]; exact ⟨h1, h2, by simp⟩)
    (by simp only [V3.deg, volSec]; exact ⟨by simp, by simp, by simpa using h5⟩)
  have hc : (volSec A2x A2y P2x P2y r2).const = volSec (C A2x) (C A2y) (C P2x) (C P2y) (C r2) := by
    simp [Sec.const, Sec.map, V3.map, volSec]
  rw [hc, (volute_F A1x A1y P1x P1y r1 (C A2x) (C A2y) (C P2x) (C P2y) (C r2)).1,
    natDegree_pow] at hg
  simp only [W, a1, a2]; omega

/-- The volute bound 12 is reached: `A1 = (s^3, 0)`, `P1 = (0, s^3)`, `rho1 = s^3` against the
fixed `A2 = (0, 1)`, `P2 = 0`, `rho2 = 1` give `F = 2 s^12 - s^6`. -/
theorem volute_witness :
    let A1x : ℝ[X] := X ^ 3
    let A1y : ℝ[X] := 0
    let P1x : ℝ[X] := 0
    let P1y : ℝ[X] := X ^ 3
    let r1 : ℝ[X] := X ^ 3
    let W := A1x * C 1 - A1y * C 0
    let a1 := (C 0 - P1x) * A1y - (C 0 - P1y) * A1x
    let a2 := (C 0 - P1x) * C 1 - (C 0 - P1y) * C 0
    (r1 ^ 2 * (W ^ 2 - a2 ^ 2) - C 1 ^ 2 * (W ^ 2 - a1 ^ 2)).natDegree = 12 := by
  intro A1x A1y P1x P1y r1 W a1 a2
  have e : r1 ^ 2 * (W ^ 2 - a2 ^ 2) - C 1 ^ 2 * (W ^ 2 - a1 ^ 2) = 2 * X ^ 12 - X ^ 6 := by
    simp only [A1x, A1y, P1x, P1y, r1, W, a1, a2, map_zero, map_one]; ring
  rw [e]; compute_degree!

end LoftSSI
