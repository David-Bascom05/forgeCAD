/-
The kernel's cyclic solve (kernel/forge-geom/src/spline.rs, `cyclic`), the periodic slope system's,
in exact arithmetic: Sherman–Morrison on the ordinary tridiagonal solve (`cyclic_solves`).

The cyclic rows are `lo_i x_(i-1) + di_i x_i + up_i x_(i+1) = rhs_i`, indices cyclic (n ≥ 3). The
kernel takes γ = -di_0, α = up_(n-1), β = lo_0 and the tridiagonal T with lo_0 and up_(n-1) dropped,
di_0 - γ and di_(n-1) - αβ/γ on the diagonal, so that the cyclic matrix is T + u vᵀ with
u = (γ, 0, .., 0, α) and v = (1, 0, .., 0, β/γ). It solves T x = rhs and T z = u by `tridiagonal`
(`thomas_solves`) and returns `x - f z`, f = vᵀx / (1 + vᵀz). That is the solution: A(x - f z) =
rhs + u vᵀx - f u (1 + vᵀz) = rhs; and 1 + vᵀz ≠ 0, else A z = 0 with z ≠ 0, which the cyclic rows'
dominance forbids.
-/
import SplineSpec.Thomas

namespace SplineSpec

set_option linter.unusedSectionVars false

variable {K : Type} [Field K] [LinearOrder K] [IsStrictOrderedRing K]

section
variable (n : ℕ) (lo di up rhs : ℕ → K)

/-- The kernel's γ, α, β. -/
def cγ : K := -di 0
def cα : K := up (n - 1)
def cβ : K := lo 0

/-- T: the cyclic rows with their corners taken out. -/
def tLo (i : ℕ) : K := if i = 0 then 0 else lo i
def tUp (i : ℕ) : K := if i = n - 1 then 0 else up i
def tDi (i : ℕ) : K :=
  if i = 0 then di 0 - cγ di else if i = n - 1 then di (n - 1) - cα n up * cβ lo / cγ di else di i
/-- u = (γ, 0, .., 0, α). -/
def uVec (i : ℕ) : K := if i = 0 then cγ di else if i = n - 1 then cα n up else 0

/-- The kernel's answer. -/
def cycX : ℕ → K := backSub (tLo lo) (tDi n lo di up) (tUp n up) rhs n
def cycZ : ℕ → K := backSub (tLo lo) (tDi n lo di up) (tUp n up) (uVec n di up) n
def cycF : K :=
  (cycX n lo di up rhs 0 + cycX n lo di up rhs (n - 1) * (cβ lo / cγ di)) /
    (1 + cycZ n lo di up 0 + cycZ n lo di up (n - 1) * (cβ lo / cγ di))
def cycY (i : ℕ) : K := cycX n lo di up rhs i - cycF n lo di up rhs * cycZ n lo di up i
end

/-- Row i of the cyclic matrix applied to x. -/
def cycRow (n : ℕ) (lo di up : ℕ → K) (x : ℕ → K) (i : ℕ) : K :=
  lo i * x (leftIx n i) + di i * x i + up i * x (rightIx n i)

/-- Row i of T applied to x. -/
def tRow (n : ℕ) (lo di up : ℕ → K) (x : ℕ → K) (i : ℕ) : K :=
  tLo lo i * x (i - 1) + tDi n lo di up i * x i + tUp n up i * x (i + 1)

/-- **The cyclic matrix is T + u vᵀ**, row by row. -/
theorem cyc_split (n : ℕ) (hn : 3 ≤ n) (lo di up : ℕ → K) (hd0 : di 0 ≠ 0) (x : ℕ → K) (i : ℕ)
    (hi : i < n) :
    cycRow n lo di up x i = tRow n lo di up x i
      + uVec n di up i * (x 0 + x (n - 1) * (cβ lo / cγ di)) := by
  have hγ : cγ di ≠ 0 := by unfold cγ; exact neg_ne_zero.2 hd0
  unfold cycRow tRow tLo tUp tDi uVec leftIx rightIx
  by_cases h0 : i = 0
  · subst h0
    have h1 : (0 : ℕ) ≠ n - 1 := by omega
    have h2 : (0 : ℕ) + 1 ≠ n := by omega
    simp only [ite_true, h1, ite_false, h2]
    unfold cγ cβ at *
    field_simp
    ring
  · by_cases hl : i = n - 1
    · subst hl
      have h3 : n - 1 + 1 = n := by omega
      simp only [h0, ite_false, ite_true, h3]
      unfold cγ cα cβ at *
      field_simp
      ring
    · have h4 : i + 1 ≠ n := by omega
      simp only [h0, hl, h4, ite_false]
      ring

/-- **Sherman–Morrison**, row by row: with T x = rhs and T z = u, `x - f z` solves the cyclic rows
once `1 + vᵀz ≠ 0`. -/
theorem sm_solves (n : ℕ) (hn : 3 ≤ n) (lo di up rhs : ℕ → K) (hd0 : di 0 ≠ 0) (x z : ℕ → K)
    (hx : ∀ i < n, tRow n lo di up x i = rhs i) (hz : ∀ i < n, tRow n lo di up z i = uVec n di up i)
    (hden : 1 + z 0 + z (n - 1) * (cβ lo / cγ di) ≠ 0) :
    ∀ i < n, cycRow n lo di up
      (fun j => x j - (x 0 + x (n - 1) * (cβ lo / cγ di)) / (1 + z 0 + z (n - 1) * (cβ lo / cγ di)) * z j) i
      = rhs i := by
  intro i hi
  rw [cyc_split n hn lo di up hd0 _ i hi]
  set f := (x 0 + x (n - 1) * (cβ lo / cγ di)) / (1 + z 0 + z (n - 1) * (cβ lo / cγ di))
  have lin : tRow n lo di up (fun j => x j - f * z j) i = tRow n lo di up x i - f * tRow n lo di up z i := by
    unfold tRow; ring
  rw [lin, hx i hi, hz i hi]
  have : f * (1 + z 0 + z (n - 1) * (cβ lo / cγ di)) = x 0 + x (n - 1) * (cβ lo / cγ di) := by
    unfold f; exact div_mul_cancel₀ _ hden
  linear_combination (-(uVec n di up i)) * this

/-- The denominator is nonzero: else z, which T maps to u ≠ 0, would be a nonzero solution of the
cyclic rows with right side 0, which their dominance forbids (`dominant_bound`). -/
theorem sm_den_ne (n : ℕ) (hn : 3 ≤ n) (lo di up : ℕ → K) (hd0 : di 0 ≠ 0)
    (hdom : ∀ i < n, |lo i| + |up i| < |di i|) (z : ℕ → K)
    (hz : ∀ i < n, tRow n lo di up z i = uVec n di up i) :
    1 + z 0 + z (n - 1) * (cβ lo / cγ di) ≠ 0 := by
  intro h
  -- the cyclic rows send z to u (1 + vᵀz) = 0
  have hrow : ∀ i < n, lo i * z (leftIx n i) + di i * z i + up i * z (rightIx n i) = 0 := by
    intro i hi
    have := cyc_split n hn lo di up hd0 z i hi
    unfold cycRow at this
    rw [this, hz i hi]
    have e : z 0 + z (n - 1) * (cβ lo / cγ di) = -1 := by linear_combination h
    rw [e]
    -- u_i (1 + (-1)) ... : u_i + u_i * (-1) = 0
    ring
  have hb := dominant_bound n z lo di up (fun _ => 0) (leftIx n) (rightIx n) 0
    (fun i hi => leftIx_lt (by omega) hi) (fun i hi => rightIx_lt (by omega) hi) hrow hdom
    (fun i _ => by simp)
  have hz0 : ∀ j < n, z j = 0 := fun j hj => abs_nonpos_iff.1 (hb j hj)
  -- but row 0 of T sends z to γ ≠ 0
  have := hz 0 (by omega)
  unfold tRow tLo tDi tUp uVec at this
  have h1 : (0 : ℕ) ≠ n - 1 := by omega
  simp only [ite_true, h1, ite_false, hz0 0 (by omega), hz0 1 (by omega), mul_zero, add_zero] at this
  exact (neg_ne_zero.2 hd0) (by unfold cγ at this; linarith)

/-- **The kernel's cyclic solve solves the periodic system** (n ≥ 3): with the cyclic rows dominant
and T's rows dominant (both hold for the slope system, whose coefficients are positive), the
answer `x - f z` satisfies every cyclic row. -/
theorem cyclic_solves (n : ℕ) (hn : 3 ≤ n) (lo di up rhs : ℕ → K) (hd0 : di 0 ≠ 0)
    (hdom : ∀ i < n, |lo i| + |up i| < |di i|)
    (htdom : ∀ i < n, |tLo lo i| + |tUp n up i| < |tDi n lo di up i|) :
    ∀ i < n, cycRow n lo di up (cycY n lo di up rhs) i = rhs i := by
  have ha0 : tLo lo 0 = (0 : K) := by simp [tLo]
  have hcn : tUp n up (n - 1) = (0 : K) := by simp [tUp]
  have hx := thomas_solves (tLo lo) (tDi n lo di up) (tUp n up) rhs n (by omega) htdom ha0 hcn
  have hz := thomas_solves (tLo lo) (tDi n lo di up) (tUp n up) (uVec n di up) n (by omega) htdom ha0 hcn
  have hden := sm_den_ne n hn lo di up hd0 hdom (cycZ n lo di up) hz
  exact sm_solves n hn lo di up rhs hd0 (cycX n lo di up rhs) (cycZ n lo di up) hx hz hden

/-- The kernel's two-unknown case (each unknown coupled to the other twice): Cramer's rule, its
determinant nonzero by dominance. -/
theorem two_solves (p q r s d0 d1 : K) (hp : |q| < |p|) (hs : |r| < |s|) :
    p * s - q * r ≠ 0 ∧
      p * ((d0 * s - d1 * q) / (p * s - q * r)) + q * ((d1 * p - d0 * r) / (p * s - q * r)) = d0 ∧
      r * ((d0 * s - d1 * q) / (p * s - q * r)) + s * ((d1 * p - d0 * r) / (p * s - q * r)) = d1 := by
  have hdet : p * s - q * r ≠ 0 := by
    intro h
    have : |p| * |s| = |q| * |r| := by rw [← abs_mul, ← abs_mul]; congr 1; linarith
    have h1 : |q| * |r| < |p| * |s| := by
      rcases eq_or_lt_of_le (abs_nonneg r) with hr | hr
      · rw [← hr, mul_zero]; exact mul_pos (lt_of_le_of_lt (abs_nonneg _) hp) (lt_of_le_of_lt (abs_nonneg _) hs)
      · exact mul_lt_mul'' hp hs (abs_nonneg _) (abs_nonneg _) |>.trans_le le_rfl
    linarith
  refine ⟨hdet, ?_, ?_⟩ <;> rw [mul_div_assoc', mul_div_assoc', ← add_div, div_eq_iff hdet] <;> ring

end SplineSpec
