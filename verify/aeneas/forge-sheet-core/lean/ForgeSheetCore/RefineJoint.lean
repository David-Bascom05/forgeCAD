import ForgeSheetCore.Funs
import ForgeSheetCore.JointErr
import AeneasF64.Lemmas
import IEEE754.Computed
import IEEE754.Sqrt
import IEEE754.Binary64
open Aeneas Aeneas.Std Result

/-!
# `place::joint`: the joint frame, through the f64 bridge

The translated `joint` (`ex = p0x ⊖ p1x`, `la = √(ex² ⊕ ey²)`, `k = 1 ⊘ la`, axis
`right((p1 ⊖ p0) ⊗ k)`) is each step verify/ieee754's binary64 operation, square root included
(`std.f64.F64.sqrt` is `FP.sqrt`, correctly rounded: `IEEE754.sqrt_eq_round`). On the domain
`Dom`: each coordinate difference `D = p1 − p0` of the doubles is `0` or has
`2⁻⁵⁰⁰ ≤ |D| ≤ 2⁵⁰⁰`, and not both are `0`, every step's exact result is zero or normal, so each
rounds within `u = 2⁻⁵³` relatively, and `joint_err` applies (`joint_spec`): when the outputs
are finite, the length is within `(1 + u)³ − 1` of `|D|` and the frame's axis `(xx, xy)` within
`(1 + u)³ / (1 − u)³ − 1` of `(D.y, −D.x) / |D|`, componentwise and relatively.
-/

set_option exponentiation.threshold 1100

namespace forge_sheet_core

open IEEE754 ForgeSheetCore.Joint

/-- The unit roundoff as a real. -/
noncomputable def U : ℝ := 2 ^ (-53 : ℤ)

theorem U_nonneg : 0 ≤ U := by unfold U; positivity
theorem U_small : U ≤ 1 / 8 := by unfold U; norm_num

/-- Zero or normal. -/
def NZ (r : ℝ) : Prop := r = 0 ∨ (2 : ℝ) ^ (-1022 : ℤ) ≤ |r|


theorem round_nz {zs : Bool} {q : ℚ} (hfin : (IEEE754.round binary64 zs q).isFinite) (h : NZ (q : ℝ)) :
    |((IEEE754.round binary64 zs q).val : ℝ) - q| ≤ U * |(q : ℝ)| := by
  rcases h with h | h
  · have : q = 0 := Rat.cast_eq_zero.mp h
    subst this; simp [IEEE754.round, FP.val]
  · have h' : (2 : ℚ) ^ binary64.emin ≤ |q| := by
      rw [binary64_emin]; exact (Rat.cast_le (K := ℝ)).mp (by push_cast; exact h)
    have := Rat.cast_le (K := ℝ) |>.mpr (round_rel_err_q hfin h')
    simp only [binary64_prec] at this
    push_cast at this
    unfold U; exact this

theorem sub_nz {a b : IEEE754.F64} (h : (FP.sub a b).isFinite) (hn : NZ ((a.val : ℝ) - b.val)) :
    |((FP.sub a b).val : ℝ) - ((a.val : ℝ) - b.val)| ≤ U * |(a.val : ℝ) - b.val| := by
  obtain ⟨ha, hb⟩ := isFinite_of_sub h
  rw [sub_eq_round ha hb] at h ⊢
  have := round_nz h (by push_cast; exact hn); push_cast at this; exact this

theorem add_nz {a b : IEEE754.F64} (h : (FP.add a b).isFinite) (hn : NZ ((a.val : ℝ) + b.val)) :
    |((FP.add a b).val : ℝ) - ((a.val : ℝ) + b.val)| ≤ U * |(a.val : ℝ) + b.val| := by
  obtain ⟨ha, hb⟩ := isFinite_of_add h
  rw [add_eq_round ha hb] at h ⊢
  have := round_nz h (by push_cast; exact hn); push_cast at this; exact this

theorem mul_nz {a b : IEEE754.F64} (h : (FP.mul a b).isFinite) (hn : NZ ((a.val : ℝ) * b.val)) :
    |((FP.mul a b).val : ℝ) - (a.val : ℝ) * b.val| ≤ U * |(a.val : ℝ) * b.val| := by
  obtain ⟨ha, hb⟩ := isFinite_of_mul h
  rw [mul_eq_round ha hb] at h ⊢
  have := round_nz h (by push_cast; exact hn); push_cast at this; exact this

theorem div_nz {a b : IEEE754.F64} (ha : a.isFinite) (hb : b.isFinite) (hb0 : (b.val : ℝ) ≠ 0)
    (h : (FP.div a b).isFinite) (hn : (2 : ℝ) ^ (-1022 : ℤ) ≤ |(a.val : ℝ) / b.val|) :
    |((FP.div a b).val : ℝ) - (a.val : ℝ) / b.val| ≤ U * |(a.val : ℝ) / b.val| := by
  have := Rat.cast_le (K := ℝ) |>.mpr
    (F64.div_rel_err ha hb (Rat.cast_ne_zero.mp hb0) h
      ((Rat.cast_le (K := ℝ)).mp (by push_cast; exact hn)))
  push_cast at this
  unfold U; exact this

theorem isFinite_of_sqrt {x : IEEE754.F64} (h : x.sqrt.isFinite) : x.isFinite := by
  cases x with
  | inf s => cases s <;> simp [FP.sqrt, FP.isFinite] at h
  | nan => simp [FP.sqrt, FP.isFinite] at h
  | _ => rfl

theorem sqrt_nz {x : IEEE754.F64} (hx : x.isFinite) (hpos : 0 < (x.val : ℝ)) (hfin : x.sqrt.isFinite)
    (hn : (2 : ℝ) ^ (-1022 : ℤ) ≤ Real.sqrt (x.val : ℝ)) :
    |(x.sqrt.val : ℝ) - Real.sqrt (x.val : ℝ)| ≤ U * Real.sqrt (x.val : ℝ) := by
  cases x with
  | zero s => simp [FP.val] at hpos
  | fin s m e h =>
    cases s
    · have := sqrt_rel_err (f := binary64) h hfin (by simpa using hn)
      simpa [U] using this
    · exfalso
      have : (FP.fin (f := binary64) true m e h).val ≤ 0 := by
        simp only [FP.val, FP.sgn]; norm_num; positivity
      have : ((FP.fin (f := binary64) true m e h).val : ℝ) ≤ 0 := by exact_mod_cast this
      linarith
  | inf s => simp [FP.isFinite] at hx
  | nan => simp [FP.isFinite] at hx

theorem one_val : (F64.Ops.ofBits 4607182418800017408).val = 1 := by
  rw [show F64.Ops.ofBits 4607182418800017408 = FP.fin false (2 ^ 52) (-52) (by decide) by decide]
  simp [FP.val, FP.sgn]; norm_num

theorem one_fin : (F64.Ops.ofBits 4607182418800017408).isFinite := by decide

/-! ## The domain's scale: `t = 2⁻⁵⁰⁰`, the least normal `c t²`, `c = 2⁻²²` -/

noncomputable irreducible_def tt : ℝ := 2 ^ (-500 : ℤ)
noncomputable irreducible_def cc : ℝ := 2 ^ (-22 : ℤ)

theorem tt_pos : 0 < tt := by rw [tt_def]; positivity
theorem tt_le : tt ≤ 1 := by rw [tt_def]; exact zpow_le_one_of_nonpos₀ (by norm_num) (by norm_num)
theorem cc_pos : 0 < cc := by rw [cc_def]; positivity
theorem cc_le : cc ≤ 1 / 16 := by rw [cc_def]; norm_num
theorem least_normal : (2 : ℝ) ^ (-1022 : ℤ) = cc * tt ^ 2 := by
  rw [cc_def, tt_def, ← zpow_natCast, ← zpow_mul, ← zpow_add₀ (by norm_num)]; norm_num
theorem tt_inv : (2 : ℝ) ^ (500 : ℤ) = 1 / tt := by
  rw [tt_def, zpow_neg, one_div, inv_inv]

/-- The domain of a coordinate difference: `0`, or `2⁻⁵⁰⁰ ≤ |r| ≤ 2⁵⁰⁰` (`tt = 2⁻⁵⁰⁰`). -/
def Dom (r : ℝ) : Prop := r = 0 ∨ (tt ≤ |r| ∧ |r| ≤ 1 / tt)

theorem nz_of {r : ℝ} (h : cc * tt ^ 2 ≤ |r|) : NZ r := Or.inr (by rw [least_normal]; exact h)

/-- A difference on the domain is zero or normal, and its rounding is `0` or at least `7 t / 8`. -/
theorem diff_step {D e : ℝ} (hD : Dom D) (he : NZ D → |e - D| ≤ U * |D|) :
    NZ D ∧ |e - D| ≤ U * |D| ∧ (D = 0 → e = 0) ∧ (D ≠ 0 → 7 / 8 * tt ≤ |e|) := by
  have hU := U_small
  have hU0 := U_nonneg
  have nz : NZ D := by
    rcases hD with h | ⟨h1, _⟩
    · exact Or.inl h
    · apply nz_of
      have := tt_pos; have := tt_le; have := cc_le; have := cc_pos
      have : cc * tt ^ 2 ≤ tt := by nlinarith
      linarith
  have b := he nz
  refine ⟨nz, b, fun h0 => ?_, fun h0 => ?_⟩
  · subst h0; simpa using b
  · rcases hD with h | ⟨h1, _⟩
    · exact absurd h h0
    · have : |D| - |e| ≤ |e - D| := by
        have := abs_sub_abs_le_abs_sub D e; rwa [abs_sub_comm] at this
      have := mul_le_mul_of_nonneg_right hU (abs_nonneg D)
      linarith

theorem sq_step {D e s : ℝ} (h0 : D = 0 → e = 0) (h1 : D ≠ 0 → 7 / 8 * tt ≤ |e|)
    (hs : NZ (e * e) → |s - e * e| ≤ U * |e * e|) :
    NZ (e * e) ∧ |s - e * e| ≤ U * |e * e| ∧ 0 ≤ s ∧ (D ≠ 0 → 7 / 8 * (7 / 8 * tt) ^ 2 ≤ s) := by
  have hU := U_small
  have hU0 := U_nonneg
  have ht := tt_pos
  have lo : D ≠ 0 → (7 / 8 * tt) ^ 2 ≤ |e * e| := fun h => by
    rw [abs_mul]; have := h1 h; nlinarith
  have nz : NZ (e * e) := by
    by_cases hD : D = 0
    · left; rw [h0 hD]; ring
    · apply nz_of
      have := lo hD; have := tt_le; have := cc_le
      have : cc * tt ^ 2 ≤ (7 / 8 * tt) ^ 2 := by nlinarith
      linarith
  have b := hs nz
  have ee : |e * e| = e * e := abs_of_nonneg (mul_self_nonneg e)
  rw [ee] at b
  have sl : (1 - U) * (e * e) ≤ s := by have := (abs_le.mp b).1; nlinarith
  refine ⟨nz, by rwa [ee], le_trans (mul_nonneg (by linarith) (mul_self_nonneg e)) sl, fun h => ?_⟩
  have := lo h; rw [ee] at this
  nlinarith

theorem sqrt_ge_min {n : ℝ} (h : 0 ≤ n) : min n 1 ≤ Real.sqrt n := by
  rcases le_total n 1 with h1 | h1
  · rw [min_eq_left h1]
    have := Real.sqrt_le_one.mpr h1
    calc n = Real.sqrt n * Real.sqrt n := (Real.mul_self_sqrt h).symm
      _ ≤ Real.sqrt n * 1 := mul_le_mul_of_nonneg_left (Real.sqrt_le_one.mpr h1) (Real.sqrt_nonneg _)
      _ = Real.sqrt n := mul_one _
  · rw [min_eq_right h1]; exact Real.one_le_sqrt.mpr h1

/-- **The joint's arithmetic on the domain**: each step given as "relatively accurate when its
exact result is zero or normal" (what the f64 operations give), the steps' exact results are all
zero or normal, `la > 0`, and `joint_err`'s bounds hold. -/
theorem joint_real {Dx Dy ex ey sx sy n la k dx dy vx vy : ℝ}
    (hDx : Dom Dx) (hDy : Dom Dy) (hD : Dx ≠ 0 ∨ Dy ≠ 0)
    (hex : NZ (-Dx) → |ex - -Dx| ≤ U * |-Dx|) (hey : NZ (-Dy) → |ey - -Dy| ≤ U * |-Dy|)
    (hsx : NZ (ex * ex) → |sx - ex * ex| ≤ U * |ex * ex|)
    (hsy : NZ (ey * ey) → |sy - ey * ey| ≤ U * |ey * ey|)
    (hn : NZ (sx + sy) → |n - (sx + sy)| ≤ U * |sx + sy|)
    (hla : 0 < n → (2 : ℝ) ^ (-1022 : ℤ) ≤ Real.sqrt n → |la - Real.sqrt n| ≤ U * Real.sqrt n)
    (hk : la ≠ 0 → (2 : ℝ) ^ (-1022 : ℤ) ≤ |1 / la| → |k - 1 / la| ≤ U * |1 / la|)
    (hdx : NZ Dx → |dx - Dx| ≤ U * |Dx|) (hdy : NZ Dy → |dy - Dy| ≤ U * |Dy|)
    (hvx : NZ (dx * k) → |vx - dx * k| ≤ U * |dx * k|)
    (hvy : NZ (dy * k) → |vy - dy * k| ≤ U * |dy * k|) :
    0 < n ∧ 0 < la ∧
    let L := Real.sqrt (Dx ^ 2 + Dy ^ 2)
    |la - L| ≤ ((1 + U) ^ 3 - 1) * L ∧
    |vx - Dx / L| ≤ ((1 + U) ^ 3 / (1 - U) ^ 3 - 1) * |Dx / L| ∧
    |vy - Dy / L| ≤ ((1 + U) ^ 3 / (1 - U) ^ 3 - 1) * |Dy / L| := by
  have hU := U_small
  have hU0 := U_nonneg
  have hU1 : U < 1 := by linarith
  have ht := tt_pos
  have ht1 := tt_le
  have hc := cc_le
  have hc0 := cc_pos
  have domneg : ∀ {D : ℝ}, Dom D → Dom (-D) := fun {D} h => by
    rcases h with h | h
    · left; rw [h, neg_zero]
    · right; rwa [abs_neg]
  obtain ⟨_, ex1, ex0, exl⟩ := diff_step (domneg hDx) hex
  obtain ⟨_, ey1, ey0, eyl⟩ := diff_step (domneg hDy) hey
  obtain ⟨_, sx1, sx0, sxl⟩ := sq_step ex0 exl hsx
  obtain ⟨_, sy1, sy0, syl⟩ := sq_step ey0 eyl hsy
  -- the sum: both nonnegative, one at least `7/8 (7/8 t)²`
  have sl : 7 / 8 * (7 / 8 * tt) ^ 2 ≤ sx + sy := by
    rcases hD with h | h
    · have := sxl (neg_ne_zero.mpr h); linarith
    · have := syl (neg_ne_zero.mpr h); linarith
  have lo1 : cc * tt ^ 2 ≤ 7 / 8 * (7 / 8 * tt) ^ 2 := by nlinarith
  have nzs : NZ (sx + sy) := nz_of (by rw [abs_of_nonneg (by linarith)]; linarith)
  have n1 := hn nzs
  rw [abs_of_nonneg (show 0 ≤ sx + sy by linarith)] at n1
  have nl : (1 - U) * (sx + sy) ≤ n := by have := (abs_le.mp n1).1; nlinarith
  have hn0 : 0 < n := by nlinarith
  have hsq : (2 : ℝ) ^ (-1022 : ℤ) ≤ Real.sqrt n := by
    rw [least_normal]
    refine le_trans ?_ (sqrt_ge_min hn0.le)
    refine le_min ?_ ?_
    · nlinarith
    · nlinarith
  have hla' := hla hn0 hsq
  have NB := norm_err hU0 hU1 ex1 ey1 sx1 sy1
    (by rw [abs_of_nonneg (show 0 ≤ sx + sy by linarith)]; exact n1)
  have hN : 0 < Dx ^ 2 + Dy ^ 2 := by
    rcases hD with h | h
    · have := pow_pos (abs_pos.mpr h) 2; rw [sq_abs] at this; positivity
    · have := pow_pos (abs_pos.mpr h) 2; rw [sq_abs] at this; positivity
  have hL : 0 < Real.sqrt (Dx ^ 2 + Dy ^ 2) := Real.sqrt_pos.mpr hN
  have B := len_err hU0 hU1 hN.le NB hla'
  have hla0 : 0 < la := lt_of_lt_of_le (mul_pos (pow_pos (by linarith) 3) hL) B.1
  -- `1 / la ≥ t / 4`: `L ≤ 2 / t`, `la ≤ (1 + U)³ L ≤ 2 L`
  have hLt : Real.sqrt (Dx ^ 2 + Dy ^ 2) ≤ 2 / tt := by
    have bx : Dx ^ 2 ≤ (1 / tt) ^ 2 := by
      rcases hDx with h | ⟨_, h⟩
      · rw [h, zero_pow two_ne_zero]; positivity
      · rw [← sq_abs]; exact pow_le_pow_left₀ (abs_nonneg _) h 2
    have by' : Dy ^ 2 ≤ (1 / tt) ^ 2 := by
      rcases hDy with h | ⟨_, h⟩
      · rw [h, zero_pow two_ne_zero]; positivity
      · rw [← sq_abs]; exact pow_le_pow_left₀ (abs_nonneg _) h 2
    have : Dx ^ 2 + Dy ^ 2 ≤ (2 / tt) ^ 2 := by
      have : (2 / tt) ^ 2 = 4 * (1 / tt) ^ 2 := by ring
      rw [this]; nlinarith [sq_nonneg (1 / tt)]
    calc _ ≤ Real.sqrt ((2 / tt) ^ 2) := Real.sqrt_le_sqrt this
      _ = 2 / tt := Real.sqrt_sq (by positivity)
  have u3 : (1 + U) ^ 3 ≤ 2 := by
    have := pow_le_pow_left₀ (by linarith : 0 ≤ 1 + U) (show 1 + U ≤ 9 / 8 by linarith) 3
    linarith [show ((9 : ℝ) / 8) ^ 3 ≤ 2 by norm_num]
  have hla4 : la ≤ 4 / tt := by
    have := B.2
    have : (1 + U) ^ 3 * Real.sqrt (Dx ^ 2 + Dy ^ 2) ≤ 2 * (2 / tt) :=
      mul_le_mul u3 hLt hL.le (by norm_num)
    have e : 2 * (2 / tt) = 4 / tt := by ring
    linarith
  have kl : tt / 4 ≤ 1 / la := by
    rw [div_le_div_iff₀ (by norm_num) hla0]
    have := mul_le_mul_of_nonneg_left hla4 ht.le
    rw [mul_div_cancel₀ _ ht.ne'] at this; linarith
  have hk' := hk hla0.ne' (by
    rw [least_normal, abs_of_pos (by positivity)]
    refine le_trans ?_ kl; nlinarith)
  -- `k ≥ (1 − U) / la ≥ 7 t / 32`
  have kk : 7 / 32 * tt ≤ k := by
    rw [abs_of_pos (show 0 < 1 / la by positivity)] at hk'
    have := (abs_le.mp hk').1
    have : U * (1 / la) ≤ 1 / 8 * (1 / la) := mul_le_mul_of_nonneg_right hU (by positivity)
    linarith [kl]
  obtain ⟨_, dx1, dx0, dxl⟩ := diff_step hDx hdx
  obtain ⟨_, dy1, dy0, dyl⟩ := diff_step hDy hdy
  have prod_nz : ∀ {D d : ℝ}, (D = 0 → d = 0) → (D ≠ 0 → 7 / 8 * tt ≤ |d|) → NZ (d * k) :=
    fun {D d} h0 h1 => by
      by_cases hD0 : D = 0
      · left; rw [h0 hD0, zero_mul]
      · apply nz_of
        rw [abs_mul, abs_of_pos (show 0 < k from lt_of_lt_of_le (by positivity) kk)]
        have := h1 hD0
        have : 7 / 8 * tt * (7 / 32 * tt) ≤ |d| * k := mul_le_mul this kk (by positivity) (abs_nonneg _)
        have : cc * tt ^ 2 ≤ 49 / 256 * tt ^ 2 := mul_le_mul_of_nonneg_right (by linarith) (by positivity)
        linarith
  refine ⟨hn0, hla0, ?_⟩
  exact joint_err hU0 hU hN.ne' ex1 ey1 sx1 sy1
    (by rw [abs_of_nonneg (show 0 ≤ sx + sy by linarith)]; exact n1) hla' hk' dx1 dy1
    (hvx (prod_nz dx0 dxl)) (hvy (prod_nz dy0 dyl))

/-- **`joint` refines the exact joint frame**: on the domain (each coordinate difference
`D = p1 − p0` of the doubles `0` or in `[2⁻⁵⁰⁰, 2⁵⁰⁰]` in magnitude, not both `0`), the frame's
origin is `p0` exactly and, when the length and the axis are finite, the length is within
`(1 + u)³ − 1` of `|D|` relatively and the axis `(xx, xy)` within `(1 + u)³ / (1 − u)³ − 1` of
`(D.y, −D.x) / |D|`, each component relatively. -/
theorem joint_spec (p0x p0y p1x p1y : IEEE754.F64)
    (hDx : Dom ((p1x.val : ℝ) - p0x.val)) (hDy : Dom ((p1y.val : ℝ) - p0y.val))
    (hD : (p1x.val : ℝ) - p0x.val ≠ 0 ∨ (p1y.val : ℝ) - p0y.val ≠ 0) :
    place.joint p0x p0y p1x p1y ⦃ r =>
      r.1.ox = p0x ∧ r.1.oy = p0y ∧
      (r.2.isFinite → r.1.xx.isFinite → r.1.xy.isFinite →
        let Dx : ℝ := p1x.val - p0x.val
        let Dy : ℝ := p1y.val - p0y.val
        let L := Real.sqrt (Dx ^ 2 + Dy ^ 2)
        |(r.2.val : ℝ) - L| ≤ ((1 + U) ^ 3 - 1) * L ∧
        |(r.1.xx.val : ℝ) - Dy / L| ≤ ((1 + U) ^ 3 / (1 - U) ^ 3 - 1) * |Dy / L| ∧
        |(r.1.xy.val : ℝ) - -(Dx / L)| ≤ ((1 + U) ^ 3 / (1 - U) ^ 3 - 1) * |Dx / L|) ⦄ := by
  unfold place.joint
  simp only [lift, Std.bind_ok, std.f64.F64.sqrt]
  simp only [WP.spec_ok, F64.Ops.add_def, F64.Ops.sub_def, F64.Ops.mul_def, F64.Ops.div_def,
    F64.Ops.neg_def]
  refine ⟨trivial, trivial, ?_⟩
  intro hla hxx hxy
  simp only [FP.isFinite_neg, FP.val_neg, Rat.cast_neg] at hxy ⊢
  -- finiteness, back along the computation
  obtain ⟨fdy, fk⟩ := isFinite_of_mul hxx
  obtain ⟨fdx, _⟩ := isFinite_of_mul hxy
  have fn := isFinite_of_sqrt hla
  obtain ⟨fsx, fsy⟩ := isFinite_of_add fn
  have fla := hla
  have one_f := one_fin
  set ex := FP.sub p0x p1x
  set ey := FP.sub p0y p1y
  set sx := FP.mul ex ex
  set sy := FP.mul ey ey
  set n := FP.add sx sy
  set la := FP.sqrt n
  set k := FP.div (F64.Ops.ofBits 4607182418800017408) la
  have ex' : (p0x.val : ℝ) - p1x.val = -((p1x.val : ℝ) - p0x.val) := by ring
  have ey' : (p0y.val : ℝ) - p1y.val = -((p1y.val : ℝ) - p0y.val) := by ring
  have R := joint_real (ex := (ex.val : ℝ)) (ey := (ey.val : ℝ)) (sx := (sx.val : ℝ))
    (sy := (sy.val : ℝ)) (n := (n.val : ℝ)) (la := (la.val : ℝ)) (k := (k.val : ℝ))
    hDx hDy hD
    (fun h => by
      have := sub_nz (isFinite_of_mul fsx).1 (by rw [ex']; exact h)
      rwa [ex'] at this)
    (fun h => by
      have := sub_nz (isFinite_of_mul fsy).1 (by rw [ey']; exact h)
      rwa [ey'] at this)
    (fun h => mul_nz fsx h) (fun h => mul_nz fsy h) (fun h => add_nz fn h)
    (fun h0 h1 => sqrt_nz fn h0 hla h1)
    (fun h0 h1 => by
      have := div_nz one_f fla h0 fk (by rwa [one_val, Rat.cast_one])
      rwa [one_val, Rat.cast_one] at this)
    (fun h => sub_nz fdx h) (fun h => sub_nz fdy h)
    (fun h => mul_nz hxy h) (fun h => mul_nz hxx h)
  obtain ⟨_, _, h1, h2, h3⟩ := R
  refine ⟨h1, h3, ?_⟩
  rw [show ∀ a b : ℝ, -a - -b = -(a - b) by intros; ring, abs_neg]
  exact h2

end forge_sheet_core
