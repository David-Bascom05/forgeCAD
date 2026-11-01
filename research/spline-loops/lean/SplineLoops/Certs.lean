/-
What `kernel/forge-geom/src/spline.rs` relies on when it reads a Bernstein polynomial's coefficients.
`crossings` takes one sign change among the coefficients to mean one root; `bern_mul` multiplies in
Bernstein form; `bern_roots` and `CurveHull::misses` take coefficients of one sign to mean no root.
-/
import Mathlib.Data.Nat.Choose.Sum
import Mathlib.Algebra.Order.Field.Basic
import Mathlib.Tactic

open Finset

namespace SplineLoops

/-- A Bernstein basis polynomial. -/
noncomputable def bb (n k : ℕ) (x : ℝ) : ℝ := (n.choose k : ℝ) * x ^ k * (1 - x) ^ (n - k)

/-- A polynomial in Bernstein form. -/
noncomputable def bern (n : ℕ) (c : ℕ → ℝ) (x : ℝ) : ℝ := ∑ k ∈ range (n + 1), c k * bb n k x

theorem bb_nonneg (n k : ℕ) {x : ℝ} (h0 : 0 ≤ x) (h1 : x ≤ 1) : 0 ≤ bb n k x := by
  unfold bb
  have : 0 ≤ 1 - x := by linarith
  positivity

theorem bb_sum (n : ℕ) (x : ℝ) : ∑ k ∈ range (n + 1), bb n k x = 1 := by
  have h := add_pow x (1 - x) n
  simp only [add_sub_cancel, one_pow] at h
  rw [h]
  refine sum_congr rfl fun k _ => ?_
  unfold bb
  ring

/-- Coefficients all above m keep the polynomial above m on [0, 1] (`bern_roots`' first test, and each
`CurveHull::misses`). -/
theorem bern_ge (n : ℕ) (c : ℕ → ℝ) (m : ℝ) (hc : ∀ k ≤ n, m ≤ c k) {x : ℝ} (h0 : 0 ≤ x) (h1 : x ≤ 1) :
    m ≤ bern n c x := by
  have : m = ∑ k ∈ range (n + 1), m * bb n k x := by
    rw [← mul_sum, bb_sum, mul_one]
  rw [this]
  refine sum_le_sum fun k hk => ?_
  exact mul_le_mul_of_nonneg_right (hc k (by simpa [Nat.lt_succ_iff] using hk)) (bb_nonneg n k h0 h1)

theorem bern_pos (n : ℕ) (c : ℕ → ℝ) (m : ℝ) (hm : 0 < m) (hc : ∀ k ≤ n, m ≤ c k) {x : ℝ} (h0 : 0 ≤ x)
    (h1 : x ≤ 1) : 0 < bern n c x :=
  lt_of_lt_of_le hm (bern_ge n c m hc h0 h1)

/-- The same for a box: the bivariate form `CurveHull::misses` tests. -/
theorem bern2_pos (m n : ℕ) (c : ℕ → ℕ → ℝ) (e : ℝ) (he : 0 < e) (hc : ∀ i ≤ m, ∀ j ≤ n, e ≤ c i j) {x y : ℝ}
    (hx0 : 0 ≤ x) (hx1 : x ≤ 1) (hy0 : 0 ≤ y) (hy1 : y ≤ 1) :
    0 < ∑ i ∈ range (m + 1), ∑ j ∈ range (n + 1), c i j * (bb m i x * bb n j y) := by
  have inner : ∀ i ∈ range (m + 1), e * bb m i x ≤ ∑ j ∈ range (n + 1), c i j * (bb m i x * bb n j y) := by
    intro i hi
    have hi' : i ≤ m := by simpa [Nat.lt_succ_iff] using hi
    have : e * bb m i x = ∑ j ∈ range (n + 1), e * (bb m i x * bb n j y) := by
      rw [← mul_sum, ← mul_sum, bb_sum, mul_one]
    rw [this]
    refine sum_le_sum fun j hj => ?_
    have hj' : j ≤ n := by simpa [Nat.lt_succ_iff] using hj
    exact mul_le_mul_of_nonneg_right (hc i hi' j hj')
      (mul_nonneg (bb_nonneg m i hx0 hx1) (bb_nonneg n j hy0 hy1))
  calc
    0 < e := he
    _ = ∑ i ∈ range (m + 1), e * bb m i x := by rw [← mul_sum, bb_sum, mul_one]
    _ ≤ _ := sum_le_sum inner

/-- A product of two basis polynomials is a basis polynomial of the sum of the degrees, the factor
`bern_mul` divides by. -/
theorem bb_mul (m n i j : ℕ) (hi : i ≤ m) (hj : j ≤ n) (x : ℝ) :
    bb m i x * bb n j x * ((m + n).choose (i + j) : ℝ)
      = (m.choose i : ℝ) * (n.choose j : ℝ) * bb (m + n) (i + j) x := by
  unfold bb
  have : m + n - (i + j) = (m - i) + (n - j) := by omega
  rw [this, pow_add, pow_add]
  ring

/-- What one sign change does. In the variable r = x / (1 - x), a polynomial with coefficients at most
0 below index m and at least 0 from it on, over r^m, cannot fall: once at or above zero it stays there,
and it is strictly above at any later point if a coefficient off index m is not zero. -/
theorem one_change_mono (n m : ℕ) (a : ℕ → ℝ) (hlo : ∀ k ≤ n, k < m → a k ≤ 0) (hhi : ∀ k ≤ n, m ≤ k → 0 ≤ a k)
    {r s : ℝ} (hr : 0 < r) (hrs : r < s) :
    (s / r) ^ m * ∑ k ∈ range (n + 1), a k * r ^ k ≤ ∑ k ∈ range (n + 1), a k * s ^ k := by
  have hs : 0 < s := hr.trans hrs
  have hq : 1 < s / r := (one_lt_div hr).mpr hrs
  rw [mul_sum]
  refine sum_le_sum fun k hk => ?_
  have hk' : k ≤ n := by simpa [Nat.lt_succ_iff] using hk
  have split : s ^ k = (s / r) ^ k * r ^ k := by
    rw [← mul_pow, div_mul_cancel₀ s hr.ne']
  rw [split]
  have rk : 0 < r ^ k := pow_pos hr k
  rcases lt_or_ge k m with h | h
  · have : (s / r) ^ k ≤ (s / r) ^ m := pow_le_pow_right₀ hq.le h.le
    have ha := hlo k hk' h
    nlinarith [mul_nonneg (mul_nonneg (neg_nonneg.mpr ha) rk.le) (sub_nonneg.mpr this)]
  · have : (s / r) ^ m ≤ (s / r) ^ k := pow_le_pow_right₀ hq.le h
    have ha := hhi k hk' h
    nlinarith [mul_nonneg (mul_nonneg ha rk.le) (sub_nonneg.mpr this)]

/-- A Bernstein polynomial in r: on (0, 1), its value is (1 - x)^n times the power form. -/
theorem bern_eq_power (n : ℕ) (c : ℕ → ℝ) {x : ℝ} (h1 : x < 1) :
    bern n c x = (1 - x) ^ n * ∑ k ∈ range (n + 1), (c k * (n.choose k : ℝ)) * (x / (1 - x)) ^ k := by
  have hne : (1 - x) ≠ 0 := by
    have : 0 < 1 - x := by linarith
    exact this.ne'
  unfold bern
  rw [mul_sum]
  refine sum_congr rfl fun k hk => ?_
  have hk' : k ≤ n := by simpa [Nat.lt_succ_iff] using hk
  unfold bb
  have : (1 - x) ^ n = (1 - x) ^ k * (1 - x) ^ (n - k) := by
    rw [← pow_add]; congr 1; omega
  rw [this, div_pow]
  field_simp

/-- At most one sign change among the coefficients: the polynomial, once at or above zero, is at or
above zero at every later point of (0, 1). So two points where it changes sign cannot both be there:
`crossings` may take such a polynomial, opposite at its ends, to have its one root by bisection. -/
theorem one_change_stays (n m : ℕ) (c : ℕ → ℝ) (hlo : ∀ k ≤ n, k < m → c k ≤ 0) (hhi : ∀ k ≤ n, m ≤ k → 0 ≤ c k)
    {x y : ℝ} (hx : 0 < x) (hxy : x < y) (hy : y < 1) (hb : 0 ≤ bern n c x) : 0 ≤ bern n c y := by
  have hx1 : x < 1 := hxy.trans hy
  have d1 : 0 < 1 - x := by linarith
  have d2 : 0 < 1 - y := by linarith
  set r := x / (1 - x) with hr
  set s := y / (1 - y) with hs
  have r0 : 0 < r := div_pos hx d1
  have rs : r < s := by
    rw [hr, hs, div_lt_div_iff₀ d1 d2]
    nlinarith
  rw [bern_eq_power n c hx1] at hb
  rw [bern_eq_power n c hy]
  have hb' : 0 ≤ ∑ k ∈ range (n + 1), (c k * (n.choose k : ℝ)) * r ^ k :=
    nonneg_of_mul_nonneg_right hb (pow_pos d1 n)
  have mono := one_change_mono n m (fun k => c k * (n.choose k : ℝ))
    (fun k hk h => mul_nonpos_of_nonpos_of_nonneg (hlo k hk h) (by positivity))
    (fun k hk h => mul_nonneg (hhi k hk h) (by positivity)) r0 rs
  have q : 0 ≤ (s / r) ^ m := pow_nonneg (div_nonneg (r0.trans rs).le r0.le) m
  exact mul_nonneg (pow_pos d2 n).le (le_trans (mul_nonneg q hb') mono)

end SplineLoops
