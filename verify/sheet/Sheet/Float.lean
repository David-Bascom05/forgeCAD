import IEEE754.Binary64
import IEEE754.Computed
import Sheet.Region

/-!
# L5: the float filter of the vertex-class predicate, proven against verify/ieee754

region.rs decides whether two split points are one vertex by `|p − q| ≤ L` in `f64`. The spec
(Region.lean `near`) is the exact `(px − qx)² + (py − qy)² ≤ L²` over the doubles' rationals. The
shipped form this file proves is a filter with an exact fallback:

```
dx = px ⊖ qx; dy = py ⊖ qy; c = (dx ⊗ dx) ⊕ (dy ⊗ dy)
c ≤ lo  → near        (lo ≤ L² (1 − u)⁴)
c > hi  → not near    (hi ≥ L² (1 + u)⁴)
else    → the exact rule (expansions)
```

with `u = 2⁻⁵³`. **Preconditions** (checked by debug builds): each difference `px − qx`, `py − qy`
is `0` or at least `2⁻⁵¹⁰` in magnitude, and every operation's result is finite. Under them every
operation is in the normal range, so each is `exact · (1 + δ)`, `|δ| ≤ u` (`F64.*_rel_err`), and
`(1 − u)⁴ D ≤ c ≤ (1 + u)⁴ D` for the exact `D` (`sq_sum_bounds`): `filter_sound`.
-/

namespace Sheet.Float

open IEEE754

/-- `u`, binary64's unit roundoff. -/
noncomputable def u : ℚ := 2 ^ (-53 : ℤ)

theorem u_pos : 0 < u := by unfold u; positivity
theorem u_lt : u < 1 := by
  unfold u; rw [zpow_neg]; apply inv_lt_one_of_one_lt₀; exact one_lt_zpow₀ (by norm_num) (by norm_num)

/-- A computed difference: exact when the inputs are equal, else within `u` relatively. -/
theorem sub_bound {x y : F64} (hx : x.isFinite) (hy : y.isFinite) (hf : (FP.sub x y).isFinite)
    (hpre : x.val - y.val = 0 ∨ 2 ^ (-510 : ℤ) ≤ |x.val - y.val|) :
    |(FP.sub x y).val - (x.val - y.val)| ≤ u * |x.val - y.val| := by
  rcases hpre with h | h
  · -- equal values: x ⊖ y is a zero
    have hn : 2 ^ (-1022 : ℤ) ≤ |x.val - y.val| ∨ x.val - y.val = 0 := Or.inr h
    rw [h, abs_zero, mul_zero]
    have := sub_err_res hx hy hf
    rw [h, sub_zero] at this
    have h2 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) < 1 := by
      simp only [binary64_prec]; rw [zpow_neg]
      exact inv_lt_one_of_one_lt₀ (one_lt_zpow₀ (by norm_num) (by norm_num))
    have : |(FP.sub x y).val| = 0 := by
      by_contra hne
      have hp : 0 < |(FP.sub x y).val| := lt_of_le_of_ne (abs_nonneg _) (Ne.symm hne)
      nlinarith
    rw [abs_eq_zero] at this
    rw [this]; simp
  · have h' : 2 ^ (-1022 : ℤ) ≤ |x.val - y.val| :=
      le_trans (zpow_le_zpow_right₀ (by norm_num) (by norm_num)) h
    exact F64.sub_rel_err hx hy hf h'

/-- The relative-error form: `r' = r (1 + δ)`, `|δ| ≤ u`. -/
theorem rel_form {r r' : ℚ} (h : |r' - r| ≤ u * |r|) :
    (1 - u) * |r| ≤ |r'| ∧ |r'| ≤ (1 + u) * |r| := by
  have := abs_sub_abs_le_abs_sub r' r
  have := abs_sub_abs_le_abs_sub r r'
  rw [abs_sub_comm] at this
  constructor <;> nlinarith [abs_nonneg r]

theorem sq_bounds {a a' : ℚ} (h : |a' - a| ≤ u * |a|) :
    (1 - u) ^ 2 * a ^ 2 ≤ a' ^ 2 ∧ a' ^ 2 ≤ (1 + u) ^ 2 * a ^ 2 := by
  obtain ⟨h1, h2⟩ := rel_form h
  have hu := u_lt
  have hu0 := u_pos
  have e1 : a' ^ 2 = |a'| ^ 2 := (sq_abs a').symm
  have e2 : a ^ 2 = |a| ^ 2 := (sq_abs a).symm
  rw [e1, e2]
  constructor
  · have : 0 ≤ (1 - u) * |a| := mul_nonneg (by linarith) (abs_nonneg _)
    nlinarith
  · nlinarith [abs_nonneg a', abs_nonneg a]

/-- **The computed sum of squares is within `(1 ± u)⁴` of the exact one**, given the squares and
the sum each round relatively (`hs1`, `hs2`, `hc`) and the differences do (`ha`, `hb`). -/
theorem sq_sum_bounds {a b a' b' s1 s2 c : ℚ}
    (ha : |a' - a| ≤ u * |a|) (hb : |b' - b| ≤ u * |b|)
    (hs1 : |s1 - a' * a'| ≤ u * |a' * a'|) (hs2 : |s2 - b' * b'| ≤ u * |b' * b'|)
    (hc : |c - (s1 + s2)| ≤ u * |s1 + s2|) :
    (1 - u) ^ 4 * (a ^ 2 + b ^ 2) ≤ c ∧ c ≤ (1 + u) ^ 4 * (a ^ 2 + b ^ 2) := by
  have hu := u_lt
  have hu0 := u_pos
  obtain ⟨qa1, qa2⟩ := sq_bounds ha
  obtain ⟨qb1, qb2⟩ := sq_bounds hb
  have na : 0 ≤ a' * a' := mul_self_nonneg _
  have nb : 0 ≤ b' * b' := mul_self_nonneg _
  rw [abs_of_nonneg na] at hs1
  rw [abs_of_nonneg nb] at hs2
  obtain ⟨s1l, s1h⟩ := abs_le.mp hs1
  obtain ⟨s2l, s2h⟩ := abs_le.mp hs2
  have hs1n : 0 ≤ s1 := by nlinarith
  have hs2n : 0 ≤ s2 := by nlinarith
  rw [abs_of_nonneg (add_nonneg hs1n hs2n)] at hc
  obtain ⟨cl, ch⟩ := abs_le.mp hc
  have e1 : a' * a' = a' ^ 2 := by ring
  have e2 : b' * b' = b' ^ 2 := by ring
  rw [e1] at s1l s1h; rw [e2] at s2l s2h
  have hl : (1 - u) ^ 2 * (a ^ 2 + b ^ 2) ≤ a' ^ 2 + b' ^ 2 := by nlinarith
  have hh : a' ^ 2 + b' ^ 2 ≤ (1 + u) ^ 2 * (a ^ 2 + b ^ 2) := by nlinarith
  have hD : 0 ≤ a ^ 2 + b ^ 2 := by positivity
  have hs_l : (1 - u) * (a' ^ 2 + b' ^ 2) ≤ s1 + s2 := by nlinarith
  have hs_h : s1 + s2 ≤ (1 + u) * (a' ^ 2 + b' ^ 2) := by nlinarith
  have p1 : 0 ≤ 1 - u := by linarith
  constructor
  · have : (1 - u) ^ 4 * (a ^ 2 + b ^ 2) = (1 - u) * ((1 - u) * ((1 - u) ^ 2 * (a ^ 2 + b ^ 2))) := by
      ring
    rw [this]
    have t1 : (1 - u) * ((1 - u) ^ 2 * (a ^ 2 + b ^ 2)) ≤ (1 - u) * (a' ^ 2 + b' ^ 2) :=
      mul_le_mul_of_nonneg_left hl p1
    have t2 : (1 - u) * ((1 - u) * ((1 - u) ^ 2 * (a ^ 2 + b ^ 2))) ≤ (1 - u) * (s1 + s2) :=
      mul_le_mul_of_nonneg_left (le_trans t1 hs_l) p1
    nlinarith
  · have : (1 + u) ^ 4 * (a ^ 2 + b ^ 2) = (1 + u) * ((1 + u) * ((1 + u) ^ 2 * (a ^ 2 + b ^ 2))) := by
      ring
    rw [this]
    have p2 : 0 ≤ 1 + u := by linarith
    have t1 : (1 + u) * (a' ^ 2 + b' ^ 2) ≤ (1 + u) * ((1 + u) ^ 2 * (a ^ 2 + b ^ 2)) :=
      mul_le_mul_of_nonneg_left hh p2
    have t2 : (1 + u) * (s1 + s2) ≤ (1 + u) * ((1 + u) * ((1 + u) ^ 2 * (a ^ 2 + b ^ 2))) :=
      mul_le_mul_of_nonneg_left (le_trans hs_h t1) p2
    nlinarith

/-- The filter's verdict: `some true` near, `some false` not, `none` for the exact rule. -/
def filter (c lo hi : ℚ) : Option Bool :=
  if c ≤ lo then some true else if hi < c then some false else none

/-- **L5, the filter is the exact rule**: whatever it decides is `Region.near`'s answer on the
exact values, given the thresholds bracket `L²`'s error band. -/
theorem filter_sound {a b a' b' s1 s2 c L lo hi : ℚ}
    (ha : |a' - a| ≤ u * |a|) (hb : |b' - b| ≤ u * |b|)
    (hs1 : |s1 - a' * a'| ≤ u * |a' * a'|) (hs2 : |s2 - b' * b'| ≤ u * |b' * b'|)
    (hc : |c - (s1 + s2)| ≤ u * |s1 + s2|)
    (hlo : lo ≤ L * L * (1 - u) ^ 4) (hhi : L * L * (1 + u) ^ 4 ≤ hi) (v : Bool)
    (hv : filter c lo hi = some v) : v = decide (a * a + b * b ≤ L * L) := by
  obtain ⟨c1, c2⟩ := sq_sum_bounds ha hb hs1 hs2 hc
  have hu := u_lt
  have hu0 := u_pos
  have q4 : 0 < (1 - u) ^ 4 := by have : 0 < 1 - u := by linarith
                                  positivity
  unfold filter at hv
  split_ifs at hv with h1 h2
  · cases hv
    symm; apply decide_eq_true
    have : (1 - u) ^ 4 * (a ^ 2 + b ^ 2) ≤ (1 - u) ^ 4 * (L * L) := by nlinarith
    have := le_of_mul_le_mul_left this q4
    nlinarith
  · cases hv
    symm; apply decide_eq_false
    intro hle
    have : (1 + u) ^ 4 * (a ^ 2 + b ^ 2) ≤ (1 + u) ^ 4 * (L * L) :=
      mul_le_mul_of_nonneg_left (by nlinarith) (by positivity)
    nlinarith

/-- The same, from the doubles: `dx = px ⊖ qx`, `dy = py ⊖ qy`, `c = dx ⊗ dx ⊕ dy ⊗ dy`, under
the preconditions; the exact `a, b` are the differences of the doubles' values. -/
theorem near_filter_sound {px py qx qy : F64}
    (hfin : px.isFinite ∧ py.isFinite ∧ qx.isFinite ∧ qy.isFinite)
    (hdx : (FP.sub px qx).isFinite) (hdy : (FP.sub py qy).isFinite)
    (hpx : px.val - qx.val = 0 ∨ 2 ^ (-510 : ℤ) ≤ |px.val - qx.val|)
    (hpy : py.val - qy.val = 0 ∨ 2 ^ (-510 : ℤ) ≤ |py.val - qy.val|)
    (hm1 : (FP.mul (FP.sub px qx) (FP.sub px qx)).isFinite)
    (hm2 : (FP.mul (FP.sub py qy) (FP.sub py qy)).isFinite)
    (hs : (FP.add (FP.mul (FP.sub px qx) (FP.sub px qx))
      (FP.mul (FP.sub py qy) (FP.sub py qy))).isFinite)
    (hn1 : (FP.sub px qx).val = 0 ∨ 2 ^ (-1022 : ℤ) ≤ |(FP.sub px qx).val * (FP.sub px qx).val|)
    (hn2 : (FP.sub py qy).val = 0 ∨ 2 ^ (-1022 : ℤ) ≤ |(FP.sub py qy).val * (FP.sub py qy).val|)
    (hn3 : (FP.mul (FP.sub px qx) (FP.sub px qx)).val + (FP.mul (FP.sub py qy) (FP.sub py qy)).val
      = 0 ∨ 2 ^ (-1022 : ℤ) ≤ |(FP.mul (FP.sub px qx) (FP.sub px qx)).val +
        (FP.mul (FP.sub py qy) (FP.sub py qy)).val|)
    (L lo hi : ℚ) (hlo : lo ≤ L * L * (1 - u) ^ 4) (hhi : L * L * (1 + u) ^ 4 ≤ hi) (v : Bool)
    (hv : filter (FP.add (FP.mul (FP.sub px qx) (FP.sub px qx))
      (FP.mul (FP.sub py qy) (FP.sub py qy))).val lo hi = some v) :
    v = Region.near L (px.val, py.val) (qx.val, qy.val) := by
  obtain ⟨fpx, fpy, fqx, fqy⟩ := hfin
  set dx := FP.sub px qx
  set dy := FP.sub py qy
  have ha := sub_bound fpx fqx hdx hpx
  have hb := sub_bound fpy fqy hdy hpy
  -- a rounding whose exact value is 0 or normal is within `u`
  have relz : ∀ {r r' : ℚ}, (r = 0 → r' = r) → (2 ^ (-1022 : ℤ) ≤ |r| → |r' - r| ≤ u * |r|) →
      (r = 0 ∨ 2 ^ (-1022 : ℤ) ≤ |r|) → |r' - r| ≤ u * |r| := by
    intro r r' h0 h1 h
    rcases h with h | h
    · rw [h0 h, h]; simp
    · exact h1 h
  have hs1 : |(FP.mul dx dx).val - dx.val * dx.val| ≤ u * |dx.val * dx.val| := by
    rcases hn1 with h | h
    · have hz := mul_err_res (f := binary64) hdx hdx hm1 (Or.inl (by rw [h]; ring))
      rw [h, mul_zero, sub_zero, abs_zero, mul_zero] at *
      have h2 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) < 1 := by
        simp only [binary64_prec]; rw [zpow_neg]
        exact inv_lt_one_of_one_lt₀ (one_lt_zpow₀ (by norm_num) (by norm_num))
      by_contra hne; push Not at hne
      nlinarith [abs_nonneg (FP.mul dx dx).val]
    · exact F64.mul_rel_err hdx hdx hm1 h
  have hs2 : |(FP.mul dy dy).val - dy.val * dy.val| ≤ u * |dy.val * dy.val| := by
    rcases hn2 with h | h
    · have hz := mul_err_res (f := binary64) hdy hdy hm2 (Or.inl (by rw [h]; ring))
      rw [h, mul_zero, sub_zero, abs_zero, mul_zero] at *
      have h2 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) < 1 := by
        simp only [binary64_prec]; rw [zpow_neg]
        exact inv_lt_one_of_one_lt₀ (one_lt_zpow₀ (by norm_num) (by norm_num))
      by_contra hne; push Not at hne
      nlinarith [abs_nonneg (FP.mul dy dy).val]
    · exact F64.mul_rel_err hdy hdy hm2 h
  have hc : |(FP.add (FP.mul dx dx) (FP.mul dy dy)).val - ((FP.mul dx dx).val + (FP.mul dy dy).val)|
      ≤ u * |(FP.mul dx dx).val + (FP.mul dy dy).val| := by
    rcases hn3 with h | h
    · have hz := add_err_res (f := binary64) (IEEE754.isFinite_of_add hs).1 (IEEE754.isFinite_of_add hs).2 hs
      rw [h, sub_zero, abs_zero, mul_zero] at *
      have h2 : (2 : ℚ) ^ (-(binary64.prec : ℤ)) < 1 := by
        simp only [binary64_prec]; rw [zpow_neg]
        exact inv_lt_one_of_one_lt₀ (one_lt_zpow₀ (by norm_num) (by norm_num))
      by_contra hne; push Not at hne
      nlinarith [abs_nonneg (FP.add (FP.mul dx dx) (FP.mul dy dy)).val]
    · exact F64.add_rel_err hm1 hm2 hs h
  have := filter_sound ha hb hs1 hs2 hc hlo hhi v hv
  rw [this]
  simp only [Region.near, Region.norm2, Region.sub]
  congr

end Sheet.Float
