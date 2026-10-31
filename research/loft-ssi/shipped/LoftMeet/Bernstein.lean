import Mathlib

/-!
# Bernstein enclosure and de Casteljau halving

The certificate clears a box when every Bernstein coefficient of `g` (or of `Psi`) has one strict
sign. These are the facts it relies on, in one variable and as tensor products in three.
-/

namespace LoftSSI

open Finset

/-- The Bernstein basis polynomial `C(n,k) x^k (1-x)^(n-k)` as a real function. -/
noncomputable def bb (n k : ℕ) (x : ℝ) : ℝ := (n.choose k : ℝ) * x ^ k * (1 - x) ^ (n - k)

theorem bb_nonneg (n k : ℕ) {x : ℝ} (h0 : 0 ≤ x) (h1 : x ≤ 1) : 0 ≤ bb n k x := by
  unfold bb
  have : 0 ≤ 1 - x := by linarith
  positivity

/-- Partition of unity. -/
theorem bb_sum (n : ℕ) (x : ℝ) : ∑ k ∈ range (n + 1), bb n k x = 1 := by
  have := (add_pow x (1 - x) n).symm
  simp only [add_sub_cancel, one_pow] at this
  rw [← this]
  refine sum_congr rfl fun k _ => ?_
  unfold bb; ring

/-- A polynomial in Bernstein form on [0,1]. -/
noncomputable def bern (n : ℕ) (b : ℕ → ℝ) (x : ℝ) : ℝ := ∑ k ∈ range (n + 1), b k * bb n k x

/-- Enclosure: coefficients all at least `m` bound the polynomial below by `m` on [0,1]. -/
theorem bern_ge (n : ℕ) (b : ℕ → ℝ) (m : ℝ) (hb : ∀ k ≤ n, m ≤ b k) {x : ℝ} (h0 : 0 ≤ x)
    (h1 : x ≤ 1) : m ≤ bern n b x := by
  calc m = ∑ k ∈ range (n + 1), m * bb n k x := by rw [← mul_sum, bb_sum, mul_one]
    _ ≤ bern n b x := by
      refine sum_le_sum fun k hk => ?_
      exact mul_le_mul_of_nonneg_right (hb k (by simp at hk; omega)) (bb_nonneg n k h0 h1)

theorem bern_le (n : ℕ) (b : ℕ → ℝ) (M : ℝ) (hb : ∀ k ≤ n, b k ≤ M) {x : ℝ} (h0 : 0 ≤ x)
    (h1 : x ≤ 1) : bern n b x ≤ M := by
  calc bern n b x ≤ ∑ k ∈ range (n + 1), M * bb n k x := by
        refine sum_le_sum fun k hk => ?_
        exact mul_le_mul_of_nonneg_right (hb k (by simp at hk; omega)) (bb_nonneg n k h0 h1)
    _ = M := by rw [← mul_sum, bb_sum, mul_one]

/-- Trivariate tensor-product Bernstein form on the unit cube. -/
noncomputable def bern3 (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ) (x y z : ℝ) : ℝ :=
  ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
    b i j k * (bb n1 i x * bb n2 j y * bb n3 k z)

/-- The 3D basis also sums to one. -/
theorem bb3_sum (n1 n2 n3 : ℕ) (x y z : ℝ) :
    ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
      bb n1 i x * bb n2 j y * bb n3 k z = 1 := by
  have h : ∀ i j, ∑ k ∈ range (n3 + 1), bb n1 i x * bb n2 j y * bb n3 k z =
      bb n1 i x * bb n2 j y := by
    intro i j; rw [← mul_sum, bb_sum, mul_one]
  have h2 : ∀ i, ∑ j ∈ range (n2 + 1), bb n1 i x * bb n2 j y = bb n1 i x := by
    intro i; rw [← mul_sum, bb_sum, mul_one]
  simp only [h, h2]
  exact bb_sum n1 x

/-- The certificate's clearing rule: all coefficients strictly positive (beyond a margin `e >= 0`
covering their rounding) means the polynomial has no zero on the box. -/
theorem bern3_pos (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ) (m : ℝ) (hm : 0 < m)
    (hb : ∀ i ≤ n1, ∀ j ≤ n2, ∀ k ≤ n3, m ≤ b i j k) {x y z : ℝ}
    (hx : 0 ≤ x ∧ x ≤ 1) (hy : 0 ≤ y ∧ y ≤ 1) (hz : 0 ≤ z ∧ z ≤ 1) :
    0 < bern3 n1 n2 n3 b x y z := by
  have key : m ≤ bern3 n1 n2 n3 b x y z := by
    calc m = m * 1 := (mul_one m).symm
      _ = m * ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
          bb n1 i x * bb n2 j y * bb n3 k z := by rw [bb3_sum]
      _ = ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
          m * (bb n1 i x * bb n2 j y * bb n3 k z) := by simp only [mul_sum]
      _ ≤ _ := by
        unfold bern3
        refine sum_le_sum fun i hi => sum_le_sum fun j hj => sum_le_sum fun k hk => ?_
        simp only [mem_range] at hi hj hk
        refine mul_le_mul_of_nonneg_right (hb i (by omega) j (by omega) k (by omega)) ?_
        have := bb_nonneg n1 i hx.1 hx.2
        have := bb_nonneg n2 j hy.1 hy.2
        have := bb_nonneg n3 k hz.1 hz.2
        positivity
  linarith

/-- Rounding: coefficients computed with error at most `e` and found above `e` are positive. -/
theorem rounded_pos {b bt e : ℝ} (herr : |bt - b| ≤ e) (h : e < bt) : 0 < b := by
  have := (abs_le.mp herr).2
  linarith

/-- de Casteljau halving: the basis on the left half re-expands in the basis on [0,1]
(`B^n_k(t/2) = sum_j B^n_j(t) B^j_k(1/2)`), so halving maps coefficients b to
`b'_j = sum_k b_k C(j,k) 2^-j`: the subdivided coefficients describe the same polynomial. -/
theorem bb_half (n k : ℕ) (t : ℝ) :
    bb n k (t / 2) = ∑ j ∈ range (n + 1), bb n j t * ((j.choose k : ℝ) * (2 : ℝ)⁻¹ ^ j) := by
  rcases lt_or_ge n k with hnk | hkn
  · -- both sides vanish
    have l : bb n k (t / 2) = 0 := by unfold bb; rw [Nat.choose_eq_zero_of_lt hnk]; simp
    rw [l]; symm
    refine sum_eq_zero fun j hj => ?_
    simp only [mem_range] at hj
    rw [Nat.choose_eq_zero_of_lt (by omega)]; simp
  · have split : range (n + 1) = range k ∪ Ico k (n + 1) := by
      rw [range_eq_Ico, range_eq_Ico, Ico_union_Ico_eq_Ico (Nat.zero_le _) (by omega)]
    rw [split, sum_union (by
      rw [range_eq_Ico]; exact Ico_disjoint_Ico_consecutive 0 k (n + 1))]
    rw [sum_eq_zero (fun j hj => by
      simp only [mem_range] at hj; rw [Nat.choose_eq_zero_of_lt hj]; simp), zero_add]
    rw [sum_Ico_eq_sum_range]
    have hsub : n + 1 - k = (n - k) + 1 := by omega
    rw [hsub]
    unfold bb
    simp only [div_eq_mul_inv]
    have e1 : (1 : ℝ) - t * 2⁻¹ = t * 2⁻¹ + (1 - t) := by ring
    rw [e1, add_pow, mul_sum]
    refine sum_congr rfl fun m hm => ?_
    simp only [mem_range] at hm
    have hc : ((n.choose (k + m) : ℕ) : ℝ) * (((k + m).choose k : ℕ) : ℝ) =
        ((n.choose k : ℕ) : ℝ) * (((n - k).choose m : ℕ) : ℝ) := by
      have := Nat.choose_mul (n := n) (k := k + m) (s := k) (by omega)
      rw [show k + m - k = m by omega] at this
      exact_mod_cast this
    have hp : n - (k + m) = n - k - m := by omega
    rw [hp]
    have : ((n.choose (k + m) : ℕ) : ℝ) * t ^ (k + m) * (1 - t) ^ (n - k - m) *
        ((((k + m).choose k : ℕ) : ℝ) * (2 : ℝ)⁻¹ ^ (k + m)) =
        (((n.choose (k + m) : ℕ) : ℝ) * (((k + m).choose k : ℕ) : ℝ)) *
          ((t * 2⁻¹) ^ k * (t * 2⁻¹) ^ m * (1 - t) ^ (n - k - m)) := by
      rw [mul_pow, mul_pow, pow_add, pow_add]; ring
    rw [this, hc]; ring

/-- Product of Bernstein polynomials (degree bookkeeping of `Hull::misses`):
`B^m_i B^n_j = C(m,i) C(n,j) / C(m+n,i+j) · B^(m+n)_(i+j)`. -/
theorem bb_mul (m n i j : ℕ) (hi : i ≤ m) (hj : j ≤ n) (x : ℝ) :
    bb m i x * bb n j x =
      ((m.choose i : ℝ) * (n.choose j : ℝ) / ((m + n).choose (i + j) : ℝ)) * bb (m + n) (i + j) x := by
  have hc : ((m + n).choose (i + j) : ℝ) ≠ 0 := by
    have := Nat.choose_pos (show i + j ≤ m + n by omega); positivity
  unfold bb
  have e : m + n - (i + j) = (m - i) + (n - j) := by omega
  rw [e, pow_add, pow_add]
  field_simp

/-- Bivariate tensor-product Bernstein form, and its positivity from its coefficients. -/
noncomputable def bern2 (n1 n2 : ℕ) (b : ℕ → ℕ → ℝ) (x y : ℝ) : ℝ :=
  ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), b i j * (bb n1 i x * bb n2 j y)

theorem bern2_ge (n1 n2 : ℕ) (b : ℕ → ℕ → ℝ) (m : ℝ)
    (hb : ∀ i ≤ n1, ∀ j ≤ n2, m ≤ b i j) {x y : ℝ}
    (hx : 0 ≤ x ∧ x ≤ 1) (hy : 0 ≤ y ∧ y ≤ 1) : m ≤ bern2 n1 n2 b x y := by
  have hs : ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), bb n1 i x * bb n2 j y = 1 := by
    simp_rw [← mul_sum, bb_sum, mul_one]; exact bb_sum n1 x
  calc m = m * ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), bb n1 i x * bb n2 j y := by
        rw [hs, mul_one]
    _ = ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), m * (bb n1 i x * bb n2 j y) := by
        simp only [mul_sum]
    _ ≤ bern2 n1 n2 b x y := by
      unfold bern2
      refine sum_le_sum fun i hi => sum_le_sum fun j hj => ?_
      simp only [mem_range] at hi hj
      exact mul_le_mul_of_nonneg_right (hb i (by omega) j (by omega))
        (mul_nonneg (bb_nonneg _ _ hx.1 hx.2) (bb_nonneg _ _ hy.1 hy.2))

/-! ## de Casteljau at any split, both sides (lean-derive) -/
/-- Symmetry of the basis. -/
theorem bb_symm (n k : ℕ) (hk : k ≤ n) (x : ℝ) : bb n k (1 - x) = bb n (n - k) x := by
  unfold bb
  rw [Nat.choose_symm hk, show n - (n - k) = k by omega, sub_sub_cancel]
  ring

/-- de Casteljau, left part of a split at `r`: the basis on `[0, r]` re-expands in the basis on
`[0, 1]`, `B^n_k(r t) = sum_j B^n_j(t) B^j_k(r)`. -/
theorem bb_split_left (n k : ℕ) (r t : ℝ) :
    bb n k (r * t) = ∑ j ∈ range (n + 1), bb n j t * bb j k r := by
  rcases lt_or_ge n k with hnk | hkn
  · have l : bb n k (r * t) = 0 := by unfold bb; rw [Nat.choose_eq_zero_of_lt hnk]; simp
    rw [l]; symm
    refine sum_eq_zero fun j hj => ?_
    simp only [mem_range] at hj
    unfold bb; rw [Nat.choose_eq_zero_of_lt (n := j) (by omega)]; simp
  · have split : range (n + 1) = range k ∪ Ico k (n + 1) := by
      rw [range_eq_Ico, range_eq_Ico, Ico_union_Ico_eq_Ico (Nat.zero_le _) (by omega)]
    rw [split, sum_union (by
      rw [range_eq_Ico]; exact Ico_disjoint_Ico_consecutive 0 k (n + 1))]
    rw [sum_eq_zero (fun j hj => by
      simp only [mem_range] at hj; unfold bb; rw [Nat.choose_eq_zero_of_lt (n := j) hj]; simp),
      zero_add]
    rw [sum_Ico_eq_sum_range]
    have hsub : n + 1 - k = (n - k) + 1 := by omega
    rw [hsub]
    unfold bb
    have e1 : (1 : ℝ) - r * t = t * (1 - r) + (1 - t) := by ring
    rw [e1, add_pow, mul_sum]
    refine sum_congr rfl fun m hm => ?_
    simp only [mem_range] at hm
    have hc : ((n.choose (k + m) : ℕ) : ℝ) * (((k + m).choose k : ℕ) : ℝ) =
        ((n.choose k : ℕ) : ℝ) * (((n - k).choose m : ℕ) : ℝ) := by
      have := Nat.choose_mul (n := n) (k := k + m) (s := k) (by omega)
      rw [show k + m - k = m by omega] at this
      exact_mod_cast this
    rw [show n - (k + m) = n - k - m by omega, show k + m - k = m by omega]
    have : ((n.choose (k + m) : ℕ) : ℝ) * t ^ (k + m) * (1 - t) ^ (n - k - m) *
        ((((k + m).choose k : ℕ) : ℝ) * r ^ k * (1 - r) ^ m) =
        (((n.choose (k + m) : ℕ) : ℝ) * (((k + m).choose k : ℕ) : ℝ)) *
          (r ^ k * t ^ k * (t * (1 - r)) ^ m * (1 - t) ^ (n - k - m)) := by
      rw [mul_pow, pow_add]; ring
    rw [this, hc, mul_pow]; ring

/-- de Casteljau, right part of a split at `r`: the basis on `[r, 1]`,
`B^n_k(r + (1-r) t) = sum_j B^n_j(t) B^(n-j)_(n-k)(1-r)` (`= B^(n-j)_(k-j)(r)` for `j <= k`, else 0). -/
theorem bb_split_right (n k : ℕ) (hk : k ≤ n) (r t : ℝ) :
    bb n k (r + (1 - r) * t) = ∑ j ∈ range (n + 1), bb n j t * bb (n - j) (n - k) (1 - r) := by
  have e : r + (1 - r) * t = 1 - (1 - r) * (1 - t) := by ring
  rw [e, bb_symm n k hk, bb_split_left, ← sum_range_reflect]
  refine sum_congr rfl fun j hj => ?_
  simp only [mem_range] at hj
  rw [show n + 1 - 1 - j = n - j by omega, bb_symm n (n - j) (by omega),
    show n - (n - j) = j by omega]

/-- The right half: `B^n_k((1 + t)/2) = sum_j B^n_j(t) B^(n-j)_(n-k)(1/2)`. -/
theorem bb_half_right (n k : ℕ) (hk : k ≤ n) (t : ℝ) :
    bb n k ((1 + t) / 2) = ∑ j ∈ range (n + 1), bb n j t * bb (n - j) (n - k) (1 / 2) := by
  have := bb_split_right n k hk (1 / 2) t
  rw [show (1 : ℝ) / 2 + (1 - 1 / 2) * t = (1 + t) / 2 by ring,
    show (1 : ℝ) - 1 / 2 = 1 / 2 by norm_num] at this
  exact this

/-- Any coefficient family, left part: `sum_i f_i B^n_i(r t) = sum_j (sum_i f_i B^j_i(r)) B^n_j(t)`. -/
theorem sum_split_left (n : ℕ) (f : ℕ → ℝ) (r t : ℝ) :
    ∑ i ∈ range (n + 1), f i * bb n i (r * t) =
      ∑ j ∈ range (n + 1), (∑ i ∈ range (n + 1), f i * bb j i r) * bb n j t := by
  simp only [bb_split_left, mul_sum, sum_mul]
  rw [sum_comm]
  refine sum_congr rfl fun j _ => sum_congr rfl fun i _ => by ring

/-- Any coefficient family, right part. -/
theorem sum_split_right (n : ℕ) (f : ℕ → ℝ) (r t : ℝ) :
    ∑ i ∈ range (n + 1), f i * bb n i (r + (1 - r) * t) =
      ∑ j ∈ range (n + 1), (∑ i ∈ range (n + 1), f i * bb (n - j) (n - i) (1 - r)) * bb n j t := by
  rw [sum_congr rfl fun i hi => by
    rw [bb_split_right n i (by simp only [mem_range] at hi; omega)]]
  simp only [mul_sum, sum_mul]
  rw [sum_comm]
  refine sum_congr rfl fun j _ => sum_congr rfl fun i _ => by ring

/-- Coefficient level, left: subdividing the coefficients gives the same polynomial on `[0, r]`. -/
theorem bern_split_left (n : ℕ) (b : ℕ → ℝ) (r t : ℝ) :
    bern n b (r * t) = bern n (fun j => ∑ i ∈ range (n + 1), b i * bb j i r) t := by
  unfold bern; rw [sum_split_left]

/-- Coefficient level, right: the same on `[r, 1]`. -/
theorem bern_split_right (n : ℕ) (b : ℕ → ℝ) (r t : ℝ) :
    bern n b (r + (1 - r) * t) =
      bern n (fun j => ∑ i ∈ range (n + 1), b i * bb (n - j) (n - i) (1 - r)) t := by
  unfold bern; rw [sum_split_right]

/-- The 3D tensor form splits along each axis; first axis, both sides. -/
theorem bern3_split_x (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ) (r x y z : ℝ) :
    bern3 n1 n2 n3 b (r * x) y z =
      bern3 n1 n2 n3 (fun i j k => ∑ a ∈ range (n1 + 1), b a j k * bb i a r) x y z ∧
    bern3 n1 n2 n3 b (r + (1 - r) * x) y z =
      bern3 n1 n2 n3
        (fun i j k => ∑ a ∈ range (n1 + 1), b a j k * bb (n1 - i) (n1 - a) (1 - r)) x y z := by
  have g : ∀ (c : ℕ → ℕ → ℕ → ℝ) (x : ℝ), bern3 n1 n2 n3 c x y z =
      ∑ i ∈ range (n1 + 1), (∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
        c i j k * (bb n2 j y * bb n3 k z)) * bb n1 i x := by
    intro c x; unfold bern3
    refine sum_congr rfl fun i _ => ?_
    rw [sum_mul]; refine sum_congr rfl fun j _ => ?_
    rw [sum_mul]; refine sum_congr rfl fun k _ => by ring
  constructor
  · rw [g, g, sum_split_left]
    refine sum_congr rfl fun i _ => ?_
    congr 1
    simp only [sum_mul]; rw [sum_comm]
    refine sum_congr rfl fun j _ => ?_; rw [sum_comm]
    refine sum_congr rfl fun k _ => sum_congr rfl fun a _ => by ring
  · rw [g, g, sum_split_right]
    refine sum_congr rfl fun i _ => ?_
    congr 1
    simp only [sum_mul]; rw [sum_comm]
    refine sum_congr rfl fun j _ => ?_; rw [sum_comm]
    refine sum_congr rfl fun k _ => sum_congr rfl fun a _ => by ring

/-- Second axis, both sides. -/
theorem bern3_split_y (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ) (r x y z : ℝ) :
    bern3 n1 n2 n3 b x (r * y) z =
      bern3 n1 n2 n3 (fun i j k => ∑ a ∈ range (n2 + 1), b i a k * bb j a r) x y z ∧
    bern3 n1 n2 n3 b x (r + (1 - r) * y) z =
      bern3 n1 n2 n3
        (fun i j k => ∑ a ∈ range (n2 + 1), b i a k * bb (n2 - j) (n2 - a) (1 - r)) x y z := by
  have g : ∀ (c : ℕ → ℕ → ℕ → ℝ) (y : ℝ), bern3 n1 n2 n3 c x y z =
      ∑ j ∈ range (n2 + 1), (∑ i ∈ range (n1 + 1), ∑ k ∈ range (n3 + 1),
        c i j k * (bb n1 i x * bb n3 k z)) * bb n2 j y := by
    intro c y; unfold bern3; rw [sum_comm]
    refine sum_congr rfl fun j _ => ?_
    rw [sum_mul]; refine sum_congr rfl fun i _ => ?_
    rw [sum_mul]; refine sum_congr rfl fun k _ => by ring
  constructor
  · rw [g, g, sum_split_left]
    refine sum_congr rfl fun j _ => ?_
    congr 1
    simp only [sum_mul]; rw [sum_comm]
    refine sum_congr rfl fun i _ => ?_; rw [sum_comm]
    refine sum_congr rfl fun k _ => sum_congr rfl fun a _ => by ring
  · rw [g, g, sum_split_right]
    refine sum_congr rfl fun j _ => ?_
    congr 1
    simp only [sum_mul]; rw [sum_comm]
    refine sum_congr rfl fun i _ => ?_; rw [sum_comm]
    refine sum_congr rfl fun k _ => sum_congr rfl fun a _ => by ring

/-- Third axis, both sides. -/
theorem bern3_split_z (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ) (r x y z : ℝ) :
    bern3 n1 n2 n3 b x y (r * z) =
      bern3 n1 n2 n3 (fun i j k => ∑ a ∈ range (n3 + 1), b i j a * bb k a r) x y z ∧
    bern3 n1 n2 n3 b x y (r + (1 - r) * z) =
      bern3 n1 n2 n3
        (fun i j k => ∑ a ∈ range (n3 + 1), b i j a * bb (n3 - k) (n3 - a) (1 - r)) x y z := by
  have g : ∀ (c : ℕ → ℕ → ℕ → ℝ) (z : ℝ), bern3 n1 n2 n3 c x y z =
      ∑ i ∈ range (n1 + 1), ∑ j ∈ range (n2 + 1), ∑ k ∈ range (n3 + 1),
        c i j k * (bb n1 i x * bb n2 j y) * bb n3 k z := by
    intro c z; unfold bern3
    refine sum_congr rfl fun i _ => sum_congr rfl fun j _ => sum_congr rfl fun k _ => by ring
  have h1 : ∀ (c : ℕ → ℝ) (w t : ℝ), ∑ k ∈ range (n3 + 1), c k * w * bb n3 k t =
      w * ∑ k ∈ range (n3 + 1), c k * bb n3 k t := by
    intro c w t; rw [mul_sum]; exact sum_congr rfl fun k _ => by ring
  constructor
  · rw [g, g]
    exact sum_congr rfl fun i _ => sum_congr rfl fun j _ =>
      (h1 (fun k => b i j k) _ _).trans
        ((congrArg _ (sum_split_left n3 (fun k => b i j k) r z)).trans (h1 _ _ _).symm)
  · rw [g, g]
    exact sum_congr rfl fun i _ => sum_congr rfl fun j _ =>
      (h1 (fun k => b i j k) _ _).trans
        ((congrArg _ (sum_split_right n3 (fun k => b i j k) r z)).trans (h1 _ _ _).symm)

end LoftSSI
