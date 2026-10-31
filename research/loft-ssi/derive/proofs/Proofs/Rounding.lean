import Proofs.Bernstein

/-!
# Floating-point rounding for the Bernstein certificate

A standard rounding model, expression trees carrying an operation count `c`, and the bound
`|computed - exact| ≤ γ_c · mag` (`γ_c = c u / (1 - c u)`). From it: the certificate's rule
`coefficient > K u · (computed magnitude)` is sound whenever `K u ≥ c u / (1 - 2 c u)`, and each
de Casteljau halving along an axis of degree `n` raises the count by `n`.
-/

namespace LoftSSI

open Finset

/-- A rounding model: each rounded result `x` comes back as `rnd x` with relative error at most
`u`. Round-to-nearest f64 satisfies this with `u = 2^-53` for every result in the normal range or
exactly zero; add/sub results that underflow are exact under gradual underflow. So the excluded
case is: a product or quotient underflowing below `2^-1022`, or anything overflowing. -/
structure Model where
  u : ℝ
  rnd : ℝ → ℝ
  u_nonneg : 0 ≤ u
  rel : ∀ x, |rnd x - x| ≤ u * |x|

/-- `θ u k = 1 / (1 - k u) = 1 + γ_k`. -/
noncomputable def θ (u : ℝ) (k : ℕ) : ℝ := 1 / (1 - k * u)

theorem cast_mul_lt {u : ℝ} (hu : 0 ≤ u) {j k : ℕ} (h : j ≤ k) (hk : (k : ℝ) * u < 1) :
    (j : ℝ) * u < 1 :=
  lt_of_le_of_lt (mul_le_mul_of_nonneg_right (by exact_mod_cast h) hu) hk

theorem θ_zero (u : ℝ) : θ u 0 = 1 := by simp [θ]

theorem θ_ge_one {u : ℝ} {k : ℕ} (hu : 0 ≤ u) (hk : (k : ℝ) * u < 1) : 1 ≤ θ u k := by
  unfold θ
  rw [le_div_iff₀ (by linarith)]
  have : 0 ≤ (k : ℝ) * u := by positivity
  linarith

/-- `θ` is monotone in the count. -/
theorem θ_mono {u : ℝ} {j k : ℕ} (hu : 0 ≤ u) (hjk : j ≤ k) (hk : (k : ℝ) * u < 1) :
    θ u j ≤ θ u k := by
  unfold θ
  apply one_div_le_one_div_of_le (by linarith)
  have : (j : ℝ) * u ≤ k * u := mul_le_mul_of_nonneg_right (by exact_mod_cast hjk) hu
  linarith

/-- One more rounding: `θ_k (1 + u) ≤ θ_(k+1)`. -/
theorem θ_succ {u : ℝ} {k : ℕ} (hu : 0 ≤ u) (hk : ((k + 1 : ℕ) : ℝ) * u < 1) :
    θ u k * (1 + u) ≤ θ u (k + 1) := by
  unfold θ
  push_cast at hk ⊢
  have hk0 : (0 : ℝ) ≤ k := Nat.cast_nonneg k
  have ha : 0 < 1 - (k : ℝ) * u := by nlinarith
  have hb : 0 < 1 - ((k : ℝ) + 1) * u := by linarith
  rw [one_div_mul_eq_div, div_le_div_iff₀ ha hb]
  nlinarith [mul_nonneg (by linarith : (0 : ℝ) ≤ k + 1) (sq_nonneg u)]

/-- `θ_j θ_k ≤ θ_(j+k)`. -/
theorem θ_mul {u : ℝ} {j k : ℕ} (hu : 0 ≤ u) (h : ((j + k : ℕ) : ℝ) * u < 1) :
    θ u j * θ u k ≤ θ u (j + k) := by
  unfold θ
  push_cast at h ⊢
  have hj : (0 : ℝ) ≤ j * u := by positivity
  have hk : (0 : ℝ) ≤ k * u := by positivity
  rw [one_div_mul_one_div]
  apply one_div_le_one_div_of_le (by linarith)
  nlinarith [mul_nonneg hj hk]

/-- Expressions evaluated in floating point. `lit x`: an exactly representable input.
`approx xh x k`: a constant computed in f64 as `xh`, standing for the real `x`, within `γ_k`.
`divc a c`: division by an exactly representable constant (rounded). `neg` and `half`
(multiplication by `0.5`) are exact; `add`, `sub`, `mul`, `divc` round. -/
inductive Ex where
  | lit (x : ℝ)
  | approx (xh x : ℝ) (k : ℕ)
  | add (a b : Ex)
  | sub (a b : Ex)
  | mul (a b : Ex)
  | divc (a : Ex) (c : ℝ)
  | neg (a : Ex)
  | half (a : Ex)

namespace Ex

/-- Exact real value. -/
noncomputable def val : Ex → ℝ
  | lit x => x
  | approx _ x _ => x
  | add a b => a.val + b.val
  | sub a b => a.val - b.val
  | mul a b => a.val * b.val
  | divc a c => a.val / c
  | neg a => -a.val
  | half a => a.val / 2

/-- Computed value: every add/sub/mul/divc result passes through the rounding. -/
noncomputable def comp (M : Model) : Ex → ℝ
  | lit x => x
  | approx xh _ _ => xh
  | add a b => M.rnd (a.comp M + b.comp M)
  | sub a b => M.rnd (a.comp M - b.comp M)
  | mul a b => M.rnd (a.comp M * b.comp M)
  | divc a c => M.rnd (a.comp M / c)
  | neg a => -a.comp M
  | half a => a.comp M / 2

/-- The same computation on absolute values, exactly. -/
noncomputable def mag : Ex → ℝ
  | lit x => |x|
  | approx _ x _ => |x|
  | add a b => a.mag + b.mag
  | sub a b => a.mag + b.mag
  | mul a b => a.mag * b.mag
  | divc a c => a.mag / |c|
  | neg a => a.mag
  | half a => a.mag / 2

/-- Operation count: the depth of rounding, with products adding. -/
def cnt : Ex → ℕ
  | lit _ => 0
  | approx _ _ k => k
  | add a b => max a.cnt b.cnt + 1
  | sub a b => max a.cnt b.cnt + 1
  | mul a b => a.cnt + b.cnt + 1
  | divc a _ => a.cnt + 1
  | neg a => a.cnt
  | half a => a.cnt

/-- Every approximate constant is within its bound and every divisor is nonzero. -/
def ok (u : ℝ) : Ex → Prop
  | lit _ => True
  | approx xh x k => |xh - x| ≤ (θ u k - 1) * |x|
  | add a b => a.ok u ∧ b.ok u
  | sub a b => a.ok u ∧ b.ok u
  | mul a b => a.ok u ∧ b.ok u
  | divc a c => a.ok u ∧ c ≠ 0
  | neg a => a.ok u
  | half a => a.ok u

/-- The magnitude bounds the exact value. -/
theorem abs_val_le_mag (e : Ex) : |e.val| ≤ e.mag := by
  induction e with
  | lit x => simp [val, mag]
  | approx xh x k => simp [val, mag]
  | add a b ha hb => exact (abs_add_le _ _).trans (add_le_add ha hb)
  | sub a b ha hb => exact (abs_sub _ _).trans (add_le_add ha hb)
  | mul a b ha hb =>
    simp only [val, mag]
    rw [abs_mul]
    exact mul_le_mul ha hb (abs_nonneg _) ((abs_nonneg _).trans ha)
  | divc a c ha =>
    simp only [val, mag]
    rw [abs_div]
    exact div_le_div_of_nonneg_right ha (abs_nonneg _)
  | neg a ha => simpa [val, mag] using ha
  | half a ha =>
    simp only [val, mag]
    rw [abs_div, abs_two]
    linarith

theorem mag_nonneg (e : Ex) : 0 ≤ e.mag := (abs_nonneg _).trans e.abs_val_le_mag

end Ex

/-- One rounded operation on a result already within `γ_c`: within `γ_(c+1)`. -/
theorem Model.step (M : Model) {s v m : ℝ} {c : ℕ} (hc : ((c + 1 : ℕ) : ℝ) * M.u < 1)
    (hs : |s - v| ≤ (θ M.u c - 1) * m) (hv : |v| ≤ m) :
    |M.rnd s - v| ≤ (θ M.u (c + 1) - 1) * m := by
  have hm : 0 ≤ m := (abs_nonneg _).trans hv
  have h1 : |s| ≤ θ M.u c * m := by
    have := abs_sub_abs_le_abs_sub s v
    linarith
  have h3 := θ_succ M.u_nonneg hc
  calc |M.rnd s - v| = |(M.rnd s - s) + (s - v)| := by ring_nf
    _ ≤ |M.rnd s - s| + |s - v| := abs_add_le _ _
    _ ≤ M.u * (θ M.u c * m) + (θ M.u c - 1) * m :=
        add_le_add ((M.rel s).trans (mul_le_mul_of_nonneg_left h1 M.u_nonneg)) hs
    _ = (θ M.u c * (1 + M.u) - 1) * m := by ring
    _ ≤ (θ M.u (c + 1) - 1) * m := mul_le_mul_of_nonneg_right (by linarith) hm

/-- Raise the count in an error bound. -/
theorem err_mono {u : ℝ} (hu : 0 ≤ u) {j k : ℕ} (hjk : j ≤ k) (hk : (k : ℝ) * u < 1) {d m : ℝ}
    (hm : 0 ≤ m) (h : d ≤ (θ u j - 1) * m) : d ≤ (θ u k - 1) * m :=
  h.trans (mul_le_mul_of_nonneg_right (by linarith [θ_mono hu hjk hk]) hm)

namespace Ex

/-- Main error bound: the computed value is within `γ_cnt · mag` of the exact one. -/
theorem err_le (M : Model) (e : Ex) (hok : e.ok M.u) (hc : (e.cnt : ℝ) * M.u < 1) :
    |e.comp M - e.val| ≤ (θ M.u e.cnt - 1) * e.mag := by
  have hu := M.u_nonneg
  induction e with
  | lit x => simp [comp, val, cnt, θ_zero]
  | approx xh x k => exact hok
  | add a b ha hb =>
    obtain ⟨oa, ob⟩ := hok
    simp only [cnt] at hc ⊢
    have hm : ((max a.cnt b.cnt : ℕ) : ℝ) * M.u < 1 := cast_mul_lt hu (by omega) hc
    have ea := err_mono hu (le_max_left _ _) hm (mag_nonneg a)
      (ha oa (cast_mul_lt hu (by omega) hc))
    have eb := err_mono hu (le_max_right _ _) hm (mag_nonneg b)
      (hb ob (cast_mul_lt hu (by omega) hc))
    have hv := abs_val_le_mag (add a b)
    simp only [val, mag] at hv
    refine M.step hc ?_ hv
    calc |a.comp M + b.comp M - (a.val + b.val)|
        = |(a.comp M - a.val) + (b.comp M - b.val)| := by ring_nf
      _ ≤ |a.comp M - a.val| + |b.comp M - b.val| := abs_add_le _ _
      _ ≤ _ := by simp only [mag]; linarith
  | sub a b ha hb =>
    obtain ⟨oa, ob⟩ := hok
    simp only [cnt] at hc ⊢
    have hm : ((max a.cnt b.cnt : ℕ) : ℝ) * M.u < 1 := cast_mul_lt hu (by omega) hc
    have ea := err_mono hu (le_max_left _ _) hm (mag_nonneg a)
      (ha oa (cast_mul_lt hu (by omega) hc))
    have eb := err_mono hu (le_max_right _ _) hm (mag_nonneg b)
      (hb ob (cast_mul_lt hu (by omega) hc))
    have hv := abs_val_le_mag (sub a b)
    simp only [val, mag] at hv
    refine M.step hc ?_ hv
    calc |a.comp M - b.comp M - (a.val - b.val)|
        = |(a.comp M - a.val) - (b.comp M - b.val)| := by ring_nf
      _ ≤ |a.comp M - a.val| + |b.comp M - b.val| := abs_sub _ _
      _ ≤ _ := by simp only [mag]; linarith
  | mul a b ha hb =>
    obtain ⟨oa, ob⟩ := hok
    simp only [cnt] at hc ⊢
    have hab : ((a.cnt + b.cnt : ℕ) : ℝ) * M.u < 1 := cast_mul_lt hu (by omega) hc
    have ea := ha oa (cast_mul_lt hu (by omega) hc)
    have eb := hb ob (cast_mul_lt hu (by omega) hc)
    have va := abs_val_le_mag a
    have vb := abs_val_le_mag b
    have ta := θ_ge_one hu (cast_mul_lt hu (by omega) hc : (a.cnt : ℝ) * M.u < 1)
    have hma := mag_nonneg a
    have hmb := mag_nonneg b
    have hA : |a.comp M| ≤ θ M.u a.cnt * a.mag := by
      have := abs_sub_abs_le_abs_sub (a.comp M) a.val
      linarith
    have hv := abs_val_le_mag (mul a b)
    simp only [val, mag] at hv
    refine M.step hc ?_ hv
    have e1 : |a.comp M * b.comp M - a.val * b.val| ≤
        θ M.u a.cnt * a.mag * ((θ M.u b.cnt - 1) * b.mag) +
          (θ M.u a.cnt - 1) * a.mag * b.mag := by
      rw [show a.comp M * b.comp M - a.val * b.val =
        a.comp M * (b.comp M - b.val) + (a.comp M - a.val) * b.val by ring]
      refine (abs_add_le _ _).trans (add_le_add ?_ ?_) <;> rw [abs_mul]
      · exact mul_le_mul hA eb (abs_nonneg _) (by positivity)
      · exact mul_le_mul ea vb (abs_nonneg _) (mul_nonneg (by linarith) hma)
    have e2 := θ_mul hu hab
    calc _ ≤ _ := e1
      _ = (θ M.u a.cnt * θ M.u b.cnt - 1) * (a.mag * b.mag) := by ring
      _ ≤ _ := mul_le_mul_of_nonneg_right (by linarith) (mul_nonneg hma hmb)
  | divc a c ha =>
    obtain ⟨oa, _⟩ := hok
    simp only [cnt] at hc ⊢
    have ea := ha oa (cast_mul_lt hu (by omega) hc)
    have hv := abs_val_le_mag (divc a c)
    simp only [val, mag] at hv
    refine M.step hc ?_ hv
    simp only [val, mag]
    rw [← sub_div, abs_div, mul_div_assoc']
    exact div_le_div_of_nonneg_right ea (abs_nonneg _)
  | neg a ha =>
    simp only [comp, val, mag, cnt] at hok hc ⊢
    rw [neg_sub_neg, abs_sub_comm]
    exact ha hok hc
  | half a ha =>
    simp only [comp, val, mag, cnt] at hok hc ⊢
    rw [← sub_div, abs_div, abs_two, mul_div_assoc']
    exact div_le_div_of_nonneg_right (ha hok hc) (by norm_num)

/-- The bound with any count `c ≥ cnt`, so one constant serves a whole tensor. -/
theorem err_le_of_le (M : Model) (e : Ex) {c : ℕ} (hok : e.ok M.u) (hle : e.cnt ≤ c)
    (hc : (c : ℝ) * M.u < 1) : |e.comp M - e.val| ≤ (θ M.u c - 1) * e.mag :=
  err_mono M.u_nonneg hle hc (mag_nonneg e)
    (err_le M e hok (cast_mul_lt M.u_nonneg hle hc))

/-- The magnitude computation as an expression: absolute inputs, `sub` becomes `add`, `neg`
disappears. This is how the certificate computes `mag`. -/
def absEx : Ex → Ex
  | lit x => lit |x|
  | approx xh x k => approx |xh| |x| k
  | add a b => add a.absEx b.absEx
  | sub a b => add a.absEx b.absEx
  | mul a b => mul a.absEx b.absEx
  | divc a c => divc a.absEx |c|
  | neg a => a.absEx
  | half a => half a.absEx

theorem val_absEx (e : Ex) : e.absEx.val = e.mag := by
  induction e <;> simp_all [absEx, val, mag]

theorem mag_absEx (e : Ex) : e.absEx.mag = e.mag := by
  induction e <;> simp_all [absEx, mag]

theorem cnt_absEx (e : Ex) : e.absEx.cnt = e.cnt := by
  induction e <;> simp_all [absEx, cnt]

theorem ok_absEx {u : ℝ} {e : Ex} (h : e.ok u) : e.absEx.ok u := by
  induction e with
  | lit x => trivial
  | approx xh x k =>
    simp only [absEx, ok, abs_abs] at h ⊢
    exact (abs_abs_sub_abs_le_abs_sub _ _).trans h
  | add a b ha hb => exact ⟨ha h.1, hb h.2⟩
  | sub a b ha hb => exact ⟨ha h.1, hb h.2⟩
  | mul a b ha hb => exact ⟨ha h.1, hb h.2⟩
  | divc a c ha => exact ⟨ha h.1, abs_ne_zero.mpr h.2⟩
  | neg a ha => exact ha h
  | half a ha => exact ha h

/-- The computed magnitude is at least `(2 - θ_c)` times the exact one. -/
theorem mag_comp_ge (M : Model) (e : Ex) {c : ℕ} (hok : e.ok M.u) (hle : e.cnt ≤ c)
    (hc : (c : ℝ) * M.u < 1) : (2 - θ M.u c) * e.mag ≤ e.absEx.comp M := by
  have h := err_le_of_le M e.absEx (ok_absEx hok) (by rwa [cnt_absEx]) hc
  rw [val_absEx, mag_absEx] at h
  have := (abs_le.mp h).1
  linarith

/-- The margin: `K u ≥ c u / (1 - 2 c u)` gives `γ_c ≤ K u (2 - θ_c)`. -/
theorem margin {u K : ℝ} {c : ℕ} (hu : 0 ≤ u) (hc : 2 * c * u < 1)
    (hK : c * u / (1 - 2 * c * u) ≤ K * u) : θ u c - 1 ≤ K * u * (2 - θ u c) := by
  have t0 : (0 : ℝ) ≤ c * u := by positivity
  have h2 : 0 < 1 - 2 * c * u := by linarith
  have h1 : 0 < 1 - c * u := by linarith
  rw [div_le_iff₀ h2] at hK
  have hT : θ u c * (1 - c * u) = 1 := by unfold θ; field_simp
  have key : (θ u c - 1) * (1 - c * u) ≤ K * u * (2 - θ u c) * (1 - c * u) := by
    have e1 : (θ u c - 1) * (1 - c * u) = c * u := by linarith
    have e2 : K * u * (2 - θ u c) * (1 - c * u) = K * u * (1 - 2 * c * u) := by
      linear_combination (-(K * u)) * hT
    rw [e1, e2]; linarith
  exact le_of_mul_le_mul_right key h1

/-- Clearing, uniform count: a coefficient computed above `K u M̂`, with `M̂` at least its
computed magnitude, is positive. -/
theorem clear_pos_of_le (M : Model) (e : Ex) {c : ℕ} {K Mh : ℝ} (hok : e.ok M.u)
    (hle : e.cnt ≤ c) (hc : 2 * c * M.u < 1) (hK : c * M.u / (1 - 2 * c * M.u) ≤ K * M.u)
    (hMh : e.absEx.comp M ≤ Mh) (hgt : K * M.u * Mh < e.comp M) : 0 < e.val := by
  have hu := M.u_nonneg
  have t0 : (0 : ℝ) ≤ c * M.u := by positivity
  have hcu : (c : ℝ) * M.u < 1 := by linarith
  have err := (abs_le.mp (err_le_of_le M e hok hle hcu)).2
  have low := mag_comp_ge M e hok hle hcu
  have key := margin hu hc hK
  have hKu : 0 ≤ K * M.u :=
    le_trans (div_nonneg t0 (by linarith)) hK
  have hm := mag_nonneg e
  have s1 : K * M.u * ((2 - θ M.u c) * e.mag) ≤ K * M.u * Mh :=
    mul_le_mul_of_nonneg_left (low.trans hMh) hKu
  have s2 : (θ M.u c - 1) * e.mag ≤ K * M.u * (2 - θ M.u c) * e.mag :=
    mul_le_mul_of_nonneg_right key hm
  nlinarith

/-- Clearing with the expression's own count. -/
theorem clear_pos (M : Model) (e : Ex) {K Mh : ℝ} (hok : e.ok M.u)
    (hc : 2 * e.cnt * M.u < 1) (hK : e.cnt * M.u / (1 - 2 * e.cnt * M.u) ≤ K * M.u)
    (hMh : e.absEx.comp M ≤ Mh) (hgt : K * M.u * Mh < e.comp M) : 0 < e.val :=
  clear_pos_of_le M e hok le_rfl hc hK hMh hgt

/-- Clearing, negative side. -/
theorem clear_neg_of_le (M : Model) (e : Ex) {c : ℕ} {K Mh : ℝ} (hok : e.ok M.u)
    (hle : e.cnt ≤ c) (hc : 2 * c * M.u < 1) (hK : c * M.u / (1 - 2 * c * M.u) ≤ K * M.u)
    (hMh : e.absEx.comp M ≤ Mh) (hlt : e.comp M < -(K * M.u * Mh)) : e.val < 0 := by
  have := clear_pos_of_le M (neg e) hok hle hc hK hMh (by simp only [comp]; linarith)
  simp only [val] at this
  linarith

end Ex

open Ex

theorem bern3_neg (n1 n2 n3 : ℕ) (b : ℕ → ℕ → ℕ → ℝ) (x y z : ℝ) :
    bern3 n1 n2 n3 (fun i j k => -b i j k) x y z = -bern3 n1 n2 n3 b x y z := by
  simp [bern3, neg_mul, sum_neg_distrib]

/-- The certificate's rule in 3D: a tensor of expressions, all `ok`, all with count at most `c`,
all computed above `K u M̂` where `M̂` bounds every computed magnitude, describes a polynomial that
is positive on the unit cube. -/
theorem bern3_clear_pos (M : Model) (n1 n2 n3 c : ℕ) (e : ℕ → ℕ → ℕ → Ex) {K Mh : ℝ}
    (hc : 2 * c * M.u < 1) (hK : c * M.u / (1 - 2 * c * M.u) ≤ K * M.u)
    (he : ∀ i ≤ n1, ∀ j ≤ n2, ∀ k ≤ n3, (e i j k).ok M.u ∧ (e i j k).cnt ≤ c ∧
      (e i j k).absEx.comp M ≤ Mh ∧ K * M.u * Mh < (e i j k).comp M)
    {x y z : ℝ} (hx : 0 ≤ x ∧ x ≤ 1) (hy : 0 ≤ y ∧ y ≤ 1) (hz : 0 ≤ z ∧ z ≤ 1) :
    0 < bern3 n1 n2 n3 (fun i j k => (e i j k).val) x y z := by
  have pos : ∀ i ≤ n1, ∀ j ≤ n2, ∀ k ≤ n3, 0 < (e i j k).val := by
    intro i hi j hj k hk
    obtain ⟨h1, h2, h3, h4⟩ := he i hi j hj k hk
    exact clear_pos_of_le M _ h1 h2 hc hK h3 h4
  obtain ⟨p, hp, hmin⟩ := exists_min_image
    (range (n1 + 1) ×ˢ range (n2 + 1) ×ˢ range (n3 + 1))
    (fun p : ℕ × ℕ × ℕ => (e p.1 p.2.1 p.2.2).val) ⟨(0, 0, 0), by simp⟩
  simp only [mem_product, mem_range] at hp
  refine bern3_pos n1 n2 n3 _ _ (pos p.1 (by omega) p.2.1 (by omega) p.2.2 (by omega)) ?_ hx hy hz
  intro i hi j hj k hk
  exact hmin (i, j, k) (by simp only [mem_product, mem_range]; omega)

/-- Negative side of `bern3_clear_pos`. -/
theorem bern3_clear_neg (M : Model) (n1 n2 n3 c : ℕ) (e : ℕ → ℕ → ℕ → Ex) {K Mh : ℝ}
    (hc : 2 * c * M.u < 1) (hK : c * M.u / (1 - 2 * c * M.u) ≤ K * M.u)
    (he : ∀ i ≤ n1, ∀ j ≤ n2, ∀ k ≤ n3, (e i j k).ok M.u ∧ (e i j k).cnt ≤ c ∧
      (e i j k).absEx.comp M ≤ Mh ∧ (e i j k).comp M < -(K * M.u * Mh))
    {x y z : ℝ} (hx : 0 ≤ x ∧ x ≤ 1) (hy : 0 ≤ y ∧ y ≤ 1) (hz : 0 ≤ z ∧ z ≤ 1) :
    bern3 n1 n2 n3 (fun i j k => (e i j k).val) x y z < 0 := by
  have := bern3_clear_pos M n1 n2 n3 c (fun i j k => neg (e i j k)) hc hK (by
    intro i hi j hj k hk
    obtain ⟨h1, h2, h3, h4⟩ := he i hi j hj k hk
    exact ⟨h1, h2, h3, by simp only [comp]; linarith⟩) hx hy hz
  simp only [Ex.val] at this
  rw [bern3_neg] at this
  linarith

/-- Sum of a list, left to right from its first element (`lit 0` when empty). -/
def sumL : List Ex → Ex
  | [] => lit 0
  | e :: l => l.foldl add e

/-- Largest count in a list. -/
def maxCnt : List Ex → ℕ
  | [] => 0
  | e :: l => max e.cnt (maxCnt l)

theorem maxCnt_le {l : List Ex} {m : ℕ} (h : ∀ e ∈ l, e.cnt ≤ m) : maxCnt l ≤ m := by
  induction l with
  | nil => simp [maxCnt]
  | cons e l ih =>
    simp only [List.mem_cons, forall_eq_or_imp] at h
    simp only [maxCnt]
    exact max_le h.1 (ih h.2)

theorem cnt_foldl_add (a : Ex) (l : List Ex) :
    (l.foldl add a).cnt ≤ max a.cnt (maxCnt l) + l.length := by
  induction l generalizing a with
  | nil => simp
  | cons e l ih =>
    have := ih (add a e)
    simp only [List.foldl_cons, maxCnt, List.length_cons, cnt] at this ⊢
    omega

theorem val_foldl_add (a : Ex) (l : List Ex) :
    (l.foldl add a).val = a.val + (l.map Ex.val).sum := by
  induction l generalizing a with
  | nil => simp
  | cons e l ih => simp only [List.foldl_cons, ih, Ex.val, List.map_cons, List.sum_cons]; ring

theorem mag_foldl_add (a : Ex) (l : List Ex) :
    (l.foldl add a).mag = a.mag + (l.map mag).sum := by
  induction l generalizing a with
  | nil => simp
  | cons e l ih => simp only [List.foldl_cons, ih, mag, List.map_cons, List.sum_cons]; ring

theorem ok_foldl_add {u : ℝ} {a : Ex} {l : List Ex} (ha : a.ok u) (hl : ∀ e ∈ l, e.ok u) :
    (l.foldl add a).ok u := by
  induction l generalizing a with
  | nil => exact ha
  | cons e l ih =>
    simp only [List.mem_cons, forall_eq_or_imp] at hl
    exact ih ⟨ha, hl.1⟩ hl.2

/-- Count of a left-to-right sum: the largest term count plus one per addition. -/
theorem cnt_sumL (l : List Ex) : (sumL l).cnt ≤ maxCnt l + l.length - 1 := by
  cases l with
  | nil => simp [sumL, cnt]
  | cons e l =>
    have := cnt_foldl_add e l
    simp only [sumL, maxCnt, List.length_cons]
    omega

theorem cnt_sumL_le {l : List Ex} {m : ℕ} (h : ∀ e ∈ l, e.cnt ≤ m) :
    (sumL l).cnt ≤ m + l.length - 1 := by
  have := cnt_sumL l
  have := maxCnt_le h
  omega

/-- Sum accumulated from `lit 0` (as `out = zeros; out += term`): one addition per term. -/
theorem cnt_sum0_le {l : List Ex} {m : ℕ} (h : ∀ e ∈ l, e.cnt ≤ m) :
    (l.foldl add (lit 0)).cnt ≤ m + l.length := by
  have := cnt_foldl_add (lit 0) l
  have := maxCnt_le h
  simp only [cnt] at *
  omega

theorem val_sumL (l : List Ex) : (sumL l).val = (l.map Ex.val).sum := by
  cases l with
  | nil => simp [sumL, Ex.val]
  | cons e l => simp [sumL, val_foldl_add]

theorem mag_sumL (l : List Ex) : (sumL l).mag = (l.map mag).sum := by
  cases l with
  | nil => simp [sumL, mag]
  | cons e l => simp [sumL, mag_foldl_add]

theorem ok_sumL {u : ℝ} {l : List Ex} (h : ∀ e ∈ l, e.ok u) : (sumL l).ok u := by
  cases l with
  | nil => trivial
  | cons e l =>
    simp only [List.mem_cons, forall_eq_or_imp] at h
    exact ok_foldl_add h.1 h.2

/-- Product of exact inputs, multiplied left to right (`lit 1` when empty). -/
def prodL : List ℝ → Ex
  | [] => lit 1
  | x :: xs => xs.foldl (fun acc y => mul acc (lit y)) (lit x)

theorem foldl_mulLit (a : Ex) (xs : List ℝ) :
    (xs.foldl (fun acc y => mul acc (lit y)) a).cnt = a.cnt + xs.length ∧
    (xs.foldl (fun acc y => mul acc (lit y)) a).val = a.val * xs.prod ∧
    (xs.foldl (fun acc y => mul acc (lit y)) a).mag = a.mag * |xs.prod| ∧
    ∀ u, a.ok u → (xs.foldl (fun acc y => mul acc (lit y)) a).ok u := by
  induction xs generalizing a with
  | nil => simp
  | cons y ys ih =>
    obtain ⟨h1, h2, h3, h4⟩ := ih (mul a (lit y))
    simp only [List.foldl_cons, List.length_cons, List.prod_cons, cnt, Ex.val, mag] at *
    refine ⟨by omega, by rw [h2]; ring, by rw [h3, abs_mul]; ring, fun u hu => h4 u ⟨hu, trivial⟩⟩

theorem prodL_spec (xs : List ℝ) :
    (prodL xs).cnt = xs.length - 1 ∧ (prodL xs).val = xs.prod ∧
    (prodL xs).mag = |xs.prod| ∧ ∀ u, (prodL xs).ok u := by
  cases xs with
  | nil => simp [prodL, cnt, Ex.val, mag, ok]
  | cons x xs =>
    obtain ⟨h1, h2, h3, h4⟩ := foldl_mulLit (lit x) xs
    simp only [prodL, cnt, Ex.val, mag, List.length_cons, List.prod_cons] at *
    exact ⟨by omega, by rw [h2], by rw [h3, abs_mul], fun u => h4 u trivial⟩

/-- A left-to-right sum of `n` products, each of `m` exact inputs multiplied left to right, is
within `γ_(m-1+n-1)` of the exact sum, relative to `Σ |term|`. -/
theorem sum_prod_err (M : Model) (rows : List (List ℝ)) (m : ℕ)
    (hm : ∀ r ∈ rows, r.length = m) (hu : ((m - 1 + (rows.length - 1) : ℕ) : ℝ) * M.u < 1) :
    |(sumL (rows.map prodL)).comp M - (rows.map List.prod).sum| ≤
      (θ M.u (m - 1 + (rows.length - 1)) - 1) * (rows.map fun r => |r.prod|).sum := by
  have hcnt : (sumL (rows.map prodL)).cnt ≤ m - 1 + (rows.length - 1) := by
    have := cnt_sumL_le (l := rows.map prodL) (m := m - 1) (by
      intro e he
      obtain ⟨r, hr, rfl⟩ := List.mem_map.mp he
      rw [(prodL_spec r).1, hm r hr])
    rw [List.length_map] at this
    omega
  have hok : (sumL (rows.map prodL)).ok M.u := ok_sumL (by
    intro e he
    obtain ⟨r, _, rfl⟩ := List.mem_map.mp he
    exact (prodL_spec r).2.2.2 _)
  have h := err_le_of_le M _ hok hcnt hu
  rwa [val_sumL, mag_sumL, List.map_map, List.map_map,
    show Ex.val ∘ prodL = List.prod from funext fun r => (prodL_spec r).2.1,
    show mag ∘ prodL = (fun r => |r.prod|) from funext fun r => (prodL_spec r).2.2.1] at h

/-- de Casteljau at `1/2` as computed: `dc b r i` is entry `i` after `r` averaging steps,
each step `0.5 * (x + y)`, one rounded add. A halving along an axis of degree `n` outputs
`dc b j 0` (left) and `dc b (n-j) j` (right) for `j ≤ n`, so it raises the count by up to `n`:
the error coefficient grows linearly with subdivision depth. -/
def dc (b : ℕ → Ex) : ℕ → ℕ → Ex
  | 0, i => b i
  | r + 1, i => half (add (dc b r i) (dc b r (i + 1)))

/-- Repeated averaging in closed form. -/
theorem avg_closed (F : ℕ → ℕ → ℝ) (hF : ∀ r i, F (r + 1) i = (F r i + F r (i + 1)) / 2)
    (r i : ℕ) : F r i = ∑ k ∈ range (r + 1), (r.choose k : ℝ) / 2 ^ r * F 0 (i + k) := by
  induction r generalizing i with
  | zero => simp
  | succ r ih =>
    rw [hF, ih, ih]
    have pascal : ∑ k ∈ range (r + 1 + 1), ((r + 1).choose k : ℝ) * F 0 (i + k) =
        ∑ k ∈ range (r + 1), (r.choose k : ℝ) * F 0 (i + k) +
          ∑ k ∈ range (r + 1), (r.choose k : ℝ) * F 0 (i + 1 + k) := by
      rw [sum_range_succ', sum_range_succ' (fun k => (r.choose k : ℝ) * F 0 (i + k))]
      simp only [Nat.choose_succ_succ, Nat.cast_add, add_mul, sum_add_distrib]
      rw [sum_range_succ (fun k => (r.choose (k + 1) : ℝ) * F 0 (i + (k + 1))),
        Nat.choose_succ_self]
      simp only [Nat.choose_zero_right, Nat.cast_zero, zero_mul, add_zero]
      have : ∀ k, i + 1 + k = i + (k + 1) := fun k => by ring
      simp only [this]
      ring
    have e : ∀ (s : ℕ) (f : ℕ → ℝ), ∑ k ∈ range (s + 1), (s.choose k : ℝ) / 2 ^ s * f k =
        (∑ k ∈ range (s + 1), (s.choose k : ℝ) * f k) / 2 ^ s := by
      intro s f; rw [sum_div]; exact sum_congr rfl fun k _ => by ring
    rw [e, e, e (r + 1) (fun k => F 0 (i + k)), pascal, pow_succ]
    field_simp

theorem val_dc (b : ℕ → Ex) (r i : ℕ) :
    (dc b r i).val = ∑ k ∈ range (r + 1), (r.choose k : ℝ) / 2 ^ r * (b (i + k)).val :=
  avg_closed (fun r i => (dc b r i).val) (fun _ _ => rfl) r i

theorem mag_dc (b : ℕ → Ex) (r i : ℕ) :
    (dc b r i).mag = ∑ k ∈ range (r + 1), (r.choose k : ℝ) / 2 ^ r * (b (i + k)).mag :=
  avg_closed (fun r i => (dc b r i).mag) (fun _ _ => rfl) r i

/-- Each averaging step adds one to the count. -/
theorem cnt_dc (b : ℕ → Ex) {c : ℕ} (r i : ℕ) (h : ∀ k ≤ r, (b (i + k)).cnt ≤ c) :
    (dc b r i).cnt ≤ c + r := by
  induction r generalizing i with
  | zero => simpa [dc] using h 0 le_rfl
  | succ r ih =>
    have h1 := ih i fun k hk => h k (by omega)
    have h2 := ih (i + 1) fun k hk => by
      rw [show i + 1 + k = i + (k + 1) by ring]; exact h _ (by omega)
    simp only [dc, cnt]
    omega

theorem ok_dc (b : ℕ → Ex) {u : ℝ} (r i : ℕ) (h : ∀ k ≤ r, (b (i + k)).ok u) :
    (dc b r i).ok u := by
  induction r generalizing i with
  | zero => simpa [dc] using h 0 le_rfl
  | succ r ih =>
    exact ⟨ih i fun k hk => h k (by omega), ih (i + 1) fun k hk => by
      rw [show i + 1 + k = i + (k + 1) by ring]; exact h _ (by omega)⟩

/-- Halving the computed magnitudes is the magnitude computation of the halved tensor. -/
theorem absEx_dc (b : ℕ → Ex) (r i : ℕ) : (dc b r i).absEx = dc (fun k => (b k).absEx) r i := by
  induction r generalizing i with
  | zero => rfl
  | succ r ih => simp only [dc, absEx, ih]

theorem bb_half_eq {m k : ℕ} (hk : k ≤ m) : bb m k (1 / 2) = (m.choose k : ℝ) / 2 ^ m := by
  unfold bb
  rw [show (1 : ℝ) - 1 / 2 = 1 / 2 by norm_num, mul_assoc, ← pow_add, Nat.add_sub_cancel' hk,
    one_div_pow, mul_one_div]

theorem bb_half_zero {m k : ℕ} (hk : m < k) : bb m k (1 / 2) = 0 := by
  unfold bb; rw [Nat.choose_eq_zero_of_lt hk]; simp

/-- Left half, output `j`: the coefficient of `bern_split_left` at `r = 1/2`. -/
theorem val_dc_left (b : ℕ → Ex) {n j : ℕ} (hj : j ≤ n) :
    (dc b j 0).val = ∑ k ∈ range (n + 1), (b k).val * bb j k (1 / 2) := by
  rw [val_dc, ← sum_range_add_sum_Ico _ (show j + 1 ≤ n + 1 by omega)]
  rw [sum_eq_zero (s := Ico _ _) fun k hk => by
    rw [mem_Ico] at hk; rw [bb_half_zero (by omega), mul_zero], add_zero]
  refine sum_congr rfl fun k hk => ?_
  rw [mem_range] at hk
  rw [bb_half_eq (by omega), zero_add]; ring

/-- Right half, output `j`: the coefficient of `bern_split_right` at `r = 1/2`. -/
theorem val_dc_right (b : ℕ → Ex) {n j : ℕ} (hj : j ≤ n) :
    (dc b (n - j) j).val = ∑ k ∈ range (n + 1), (b k).val * bb (n - j) (n - k) (1 / 2) := by
  rw [val_dc, ← sum_range_add_sum_Ico _ (show j ≤ n + 1 by omega)]
  rw [sum_eq_zero (s := range j) fun k hk => by
    rw [mem_range] at hk; rw [bb_half_zero (by omega), mul_zero], zero_add,
    sum_Ico_eq_sum_range, show n + 1 - j = n - j + 1 by omega]
  refine sum_congr rfl fun k hk => ?_
  rw [mem_range] at hk
  rw [bb_half_eq (by omega), show n - (j + k) = n - j - k by omega,
    Nat.choose_symm (by omega)]; ring

/-- The rounded halving describes, in exact arithmetic, the polynomial on the left half. -/
theorem bern_dc_left (n : ℕ) (b : ℕ → Ex) (t : ℝ) :
    bern n (fun k => (b k).val) (t / 2) = bern n (fun j => (dc b j 0).val) t := by
  rw [show t / 2 = 1 / 2 * t by ring, bern_split_left]
  unfold bern
  exact sum_congr rfl fun j hj => by dsimp only; rw [val_dc_left b (by simp at hj; omega)]

/-- ... and on the right half. -/
theorem bern_dc_right (n : ℕ) (b : ℕ → Ex) (t : ℝ) :
    bern n (fun k => (b k).val) ((1 + t) / 2) = bern n (fun j => (dc b (n - j) j).val) t := by
  rw [show (1 + t) / 2 = 1 / 2 + (1 - 1 / 2) * t by ring, bern_split_right,
    show (1 : ℝ) - 1 / 2 = 1 / 2 by norm_num]
  unfold bern
  exact sum_congr rfl fun j hj => by dsimp only; rw [val_dc_right b (by simp at hj; omega)]

/-- Degree elevation, the exact identity behind the certificate's `elevate` (its weights
`1 - i/(n+1)` and `i/(n+1)` are the `approx` leaves above):
`B^n_i = (n+1-i)/(n+1) B^(n+1)_i + (i+1)/(n+1) B^(n+1)_(i+1)`. -/
theorem bb_elevate (n i : ℕ) (hi : i ≤ n) (x : ℝ) :
    bb n i x = ((n + 1 - i : ℕ) : ℝ) / (n + 1) * bb (n + 1) i x +
      ((i + 1 : ℕ) : ℝ) / (n + 1) * bb (n + 1) (i + 1) x := by
  have c1 : ((n.choose i : ℕ) : ℝ) * (n + 1) =
      (((n + 1).choose i : ℕ) : ℝ) * ((n + 1 - i : ℕ) : ℝ) := by
    exact_mod_cast Nat.choose_mul_succ_eq n i
  have c2 : ((n + 1 : ℕ) : ℝ) * ((n.choose i : ℕ) : ℝ) =
      (((n + 1).choose (i + 1) : ℕ) : ℝ) * ((i + 1 : ℕ) : ℝ) := by
    exact_mod_cast Nat.add_one_mul_choose_eq n i
  have hn : (n : ℝ) + 1 ≠ 0 := by positivity
  unfold bb
  rw [show n + 1 - i = (n - i) + 1 by omega, show n + 1 - (i + 1) = n - i by omega]
  rw [show n + 1 - i = (n - i) + 1 by omega] at c1
  push_cast at c1 c2 ⊢
  field_simp
  rw [pow_succ, pow_succ]
  linear_combination (x ^ i * (1 - x) ^ (n - i) * (1 - x)) * c1 +
    (x ^ i * x * (1 - x) ^ (n - i)) * c2

end LoftSSI
