/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Dmath.Consts

/-!
# `dmath::rem_pio2`

The model of `rem_pio2` (Cody–Waite with a three-part `π/2`) and its accuracy for `|x| ≤ 2048`:
`y0 + y1` is `x − n·π/2` within `2^-56`.
-/

set_option linter.unusedSectionVars false

namespace IEEE754.Dmath

open FP

/-- `dmath`'s `exponent`: the biased exponent field. -/
def expo (x : F64) : ℕ := x.toBits / 2 ^ 52 % 2 ^ 11

/-- `dmath::rem_pio2`, operation for operation. -/
def remPio2 (x : F64) : ℤ × F64 × F64 :=
  let f := sub (add (mul x INVPIO2) TOINT) TOINT
  let n := FP.toIntSat (-2 ^ 31) (2 ^ 31 - 1) f
  let r := sub x (mul f PIO2_1)
  let w := mul f PIO2_1T
  let y0 := sub r w
  if 16 < (expo x : ℤ) - expo y0 then
    let t := r
    let w := mul f PIO2_2
    let r := sub t w
    let w := sub (mul f PIO2_2T) (sub (sub t r) w)
    let y0 := sub r w
    if 49 < (expo x : ℤ) - expo y0 then
      let t := r
      let w := mul f PIO2_3
      let r := sub t w
      let w := sub (mul f PIO2_3T) (sub (sub t r) w)
      let y0 := sub r w
      (n, y0, sub (sub r y0) w)
    else (n, y0, sub (sub r y0) w)
  else (n, y0, sub (sub r y0) w)

/-! ## The exponent field -/

lemma expo_aux (c v : ℕ) : (c * 2 ^ 63 + v) / 2 ^ 52 % 2 ^ 11 = v / 2 ^ 52 % 2 ^ 11 := by
  rw [show c * 2 ^ 63 + v = v + (c * 2 ^ 11) * 2 ^ 52 by ring, Nat.add_mul_div_right _ _ (by positivity)]
  simp [Nat.add_mul_mod_self_right]

lemma expo_fin {s : Bool} {m : ℕ} {e : ℤ} (h : Canon binary64 m e) :
    expo (.fin s m e h) = if m < 2 ^ 52 then 0 else (e + 1075).toNat := by
  have h' := h
  obtain ⟨h0, h1, h2, h3, _⟩ := h'
  simp only [binary64_qmin, binary64_qmax] at h2 h3
  have hm : m < 2 ^ 53 := by simpa using h1
  have hv : (if m < 2 ^ (binary64.prec - 1) then m
      else (e - binary64.qmin + 1).toNat * 2 ^ (binary64.prec - 1) + (m - 2 ^ (binary64.prec - 1))) / 2 ^ 52 % 2 ^ 11
      = if m < 2 ^ 52 then 0 else (e + 1075).toNat := by
    simp only [binary64_prec, show (53 - 1 : ℕ) = 52 from rfl, binary64_qmin]
    split_ifs with hm2
    · rw [Nat.div_eq_of_lt hm2]; rfl
    · have ht : (e - -1074 + 1).toNat = (e + 1075).toNat := by congr 1; ring
      rw [ht, show (e + 1075).toNat * 2 ^ 52 + (m - 2 ^ 52) = (m - 2 ^ 52) + (e + 1075).toNat * 2 ^ 52 by ring,
        Nat.add_mul_div_right _ _ (by positivity), Nat.div_eq_of_lt (by omega), Nat.zero_add]
      exact Nat.mod_eq_of_lt (by omega)
  unfold expo toBits
  have hsb : signBit binary64 = 1 * 2 ^ 63 := rfl
  cases s
  · simp only [Bool.false_eq_true, ite_false, Nat.zero_add]; exact hv
  · simp only [ite_true]; rw [hsb, expo_aux]; exact hv

lemma abs_val_fin {s : Bool} {m : ℕ} {e : ℤ} {h : Canon binary64 m e} :
    |(FP.fin s m e h : F64).val| = m * 2 ^ e := by
  have : (0 : ℚ) ≤ m * 2 ^ e := mul_nonneg (by positivity) (two_zpow_pos e).le
  cases s <;> simp [val, sgn, abs_mul, abs_of_nonneg this]

/-- A finite datum is below `2^(expo − 1022)`. -/
lemma abs_lt_expo {y : F64} (hy : y.isFinite) : |y.val| < 2 ^ ((expo y : ℤ) - 1022) := by
  cases y with
  | zero s => simp; exact two_zpow_pos _
  | fin s m e h =>
    rw [expo_fin h, abs_val_fin]
    obtain ⟨h0, h1, h2, h3, hn⟩ := h
    simp only [binary64_qmin, binary64_prec] at h2 h1 hn
    split_ifs with hm
    · have he : e = -1074 := by
        rcases hn with hn | hn
        · exfalso; simp at hn; omega
        · exact hn
      subst he
      calc (m : ℚ) * 2 ^ (-1074 : ℤ) < 2 ^ (52 : ℕ) * 2 ^ (-1074 : ℤ) :=
            mul_lt_mul_of_pos_right (by exact_mod_cast hm) (two_zpow_pos _)
        _ = 2 ^ (((0 : ℕ) : ℤ) - 1022) := by rw [← zpow_natCast, ← two_zpow_add]; norm_num
    · have : ((e + 1075).toNat : ℤ) = e + 1075 := Int.toNat_of_nonneg (by omega)
      rw [this, show e + 1075 - 1022 = (53 : ℕ) + e by push_cast; ring, two_zpow_add, zpow_natCast]
      exact mul_lt_mul_of_pos_right (by exact_mod_cast h1) (two_zpow_pos e)
  | _ => simp [isFinite] at hy

/-- A datum with a nonzero exponent field is at least `2^(expo − 1023)`. -/
lemma expo_le_abs {x : F64} (hx : x.isFinite) (h : 1 ≤ expo x) : 2 ^ ((expo x : ℤ) - 1023) ≤ |x.val| := by
  cases x with
  | zero s =>
    exfalso
    have : expo (FP.zero s : F64) = 0 := by
      cases s
      · rfl
      · show (1 * 2 ^ 63 + 0) / 2 ^ 52 % 2 ^ 11 = 0; rw [expo_aux]; rfl
    omega
  | fin s m e hc =>
    rw [expo_fin hc] at h ⊢
    rw [abs_val_fin]
    obtain ⟨h0, h1, h2, h3, hn⟩ := hc
    simp only [binary64_qmin] at h2
    split_ifs at h ⊢ with hm
    · simp at h
    · have : ((e + 1075).toNat : ℤ) = e + 1075 := Int.toNat_of_nonneg (by omega)
      rw [this, show e + 1075 - 1023 = (52 : ℕ) + e by push_cast; ring, two_zpow_add, zpow_natCast]
      exact mul_le_mul_of_nonneg_right (by exact_mod_cast (not_lt.mp hm)) (two_zpow_pos e).le
  | _ => simp [isFinite] at hx

/-- An exponent gap of more than `k` makes `y` smaller than `2^-k·x`. -/
lemma expo_gap {x y : F64} (hx : x.isFinite) (hy : y.isFinite) {k : ℤ} (hk : 0 ≤ k)
    (h : k < (expo x : ℤ) - expo y) : |y.val| < 2 ^ (-k) * |x.val| := by
  have h1 : 1 ≤ expo x := by omega
  calc |y.val| < 2 ^ ((expo y : ℤ) - 1022) := abs_lt_expo hy
    _ ≤ 2 ^ (-k + ((expo x : ℤ) - 1023)) := two_zpow_le (by omega)
    _ = 2 ^ (-k) * 2 ^ ((expo x : ℤ) - 1023) := two_zpow_add _ _
    _ ≤ 2 ^ (-k) * |x.val| := mul_le_mul_of_nonneg_left (expo_le_abs hx h1) (two_zpow_pos _).le

/-! ## The quadrant `f` -/

lemma fin_consts : INVPIO2.isFinite ∧ TOINT.isFinite ∧ PIO2_1.isFinite ∧ PIO2_1T.isFinite ∧ PIO2_2.isFinite ∧
    PIO2_2T.isFinite ∧ PIO2_3.isFinite ∧ PIO2_3T.isFinite := by decide

/-- `f = (x·INVPIO2 + TOINT) − TOINT` is an integer within `1/2 + 2^-43` of `x·INVPIO2`. -/
lemma quadrant {x : F64} (hx : x.isFinite) (hb : |x.val| ≤ 2048) :
    let f := sub (add (mul x INVPIO2) TOINT) TOINT
    f.isFinite ∧ (∃ k : ℤ, f.val = k ∧ |k| ≤ 1305) ∧
      |f.val - x.val * INVPIO2.val| ≤ 1 / 2 + 2 ^ (-43 : ℤ) := by
  intro f
  obtain ⟨hI, hT, -⟩ := fin_consts
  have hIv : INVPIO2.val = 5734161139222659 / 2 ^ 53 := INVPIO2_val
  have hTv : TOINT.val = 6755399441055744 := TOINT_val
  -- `t = x ⊗ INVPIO2`, within `2^-43`
  set t := mul x INVPIO2 with ht
  have htr : t = round binary64 (x.sign ^^ INVPIO2.sign) (x.val * INVPIO2.val) := mul_eq_round hx hI
  have hXI : |x.val * INVPIO2.val| ≤ 1304 := by
    rw [abs_mul, hIv]; rw [abs_of_pos (by norm_num : (0:ℚ) < 5734161139222659 / 2 ^ 53)]
    calc |x.val| * (5734161139222659 / 2 ^ 53) ≤ 2048 * (5734161139222659 / 2 ^ 53) :=
          mul_le_mul_of_nonneg_right hb (by norm_num)
      _ ≤ 1304 := by norm_num
  have htf : t.isFinite := by
    rw [htr]; exact round_finite_of_le_emax (le_trans hXI (le_trans (by norm_num : (1304:ℚ) ≤ 2 ^ (11:ℤ)) (two_zpow_le (by simp))))
  have hte : |t.val - x.val * INVPIO2.val| ≤ 2 ^ (-43 : ℤ) := by
    by_cases h0 : x.val * INVPIO2.val = 0
    · rw [htr, h0]; simp [round, val]
    · have := (round_grid (f := binary64) (zs := x.sign ^^ INVPIO2.sign) h0 (by rw [← htr]; exact htf)).2
      rw [← htr] at this
      have hq := qOf_le (f := binary64) h0 (k := 11) (lt_of_le_of_lt hXI (by norm_num))
      simp only [binary64_prec, binary64_qmin] at hq
      exact le_trans this (two_zpow_le (by omega))
  have htb : |t.val| ≤ 1304 + 1 / 1000 := by
    have h2 : (2 : ℚ) ^ (-43 : ℤ) ≤ 1 / 1000 := by norm_num
    calc |t.val| = |(t.val - x.val * INVPIO2.val) + x.val * INVPIO2.val| := by ring_nf
      _ ≤ |t.val - x.val * INVPIO2.val| + |x.val * INVPIO2.val| := abs_add_le _ _
      _ ≤ 2 ^ (-43 : ℤ) + 1304 := add_le_add hte hXI
      _ ≤ 1304 + 1 / 1000 := by linarith
  -- `u = t ⊕ TOINT` is an integer within `1/2` of `t + TOINT`
  set u := add t TOINT with hu
  have hur : u = round binary64 (t.sign && TOINT.sign) (t.val + TOINT.val) := add_eq_round htf hT
  have hlo : (2 : ℚ) ^ (52 : ℤ) ≤ |t.val + TOINT.val| := by
    rw [hTv]; have := (abs_le.mp htb).1
    rw [abs_of_pos (by linarith)]; norm_num; linarith
  have hhi : |t.val + TOINT.val| < 2 ^ (53 : ℤ) := by
    rw [hTv]; have := abs_le.mp htb
    rw [abs_of_pos (by linarith)]; norm_num; linarith
  have h0' : t.val + TOINT.val ≠ 0 := by intro h; rw [h, abs_zero] at hlo; norm_num at hlo
  have huf : u.isFinite := by
    rw [hur]; exact round_finite_of_le_emax (le_trans hhi.le (two_zpow_le (by simp)))
  have hq0 : qOf binary64 (t.val + TOINT.val) = 0 := by
    have h1 := qOf_ge (f := binary64) hlo
    have h2 := qOf_le (f := binary64) h0' hhi
    simp only [binary64_prec, binary64_qmin] at h1 h2
    omega
  obtain ⟨huM, huE⟩ := round_grid (f := binary64) (zs := t.sign && TOINT.sign) h0' (by rw [← hur]; exact huf)
  rw [← hur, hq0] at huM huE
  obtain ⟨ku, hku⟩ := huM
  simp only [zpow_zero, _root_.mul_one] at hku
  -- `f = u ⊖ TOINT`, exact by Sterbenz
  have hub := abs_le.mp huE
  have htb' := abs_le.mp htb
  rw [hTv] at hub
  norm_num at hub
  obtain ⟨hub1, hub2⟩ := hub
  obtain ⟨htb1, htb2⟩ := htb'
  have hf := sterbenz huf hT (by rw [hTv]; linarith) (by rw [hTv]; linarith)
  change (sub u TOINT).isFinite ∧ (∃ k : ℤ, (sub u TOINT).val = k ∧ |k| ≤ 1305) ∧
    |(sub u TOINT).val - x.val * INVPIO2.val| ≤ 1 / 2 + 2 ^ (-43 : ℤ)
  refine ⟨hf.1, ⟨ku - 6755399441055744, by rw [hf.2, hku, hTv]; push_cast; ring, ?_⟩, ?_⟩
  · have : |(ku : ℚ) - 6755399441055744| ≤ 1305 := by
      rw [← hku]; exact abs_le.mpr ⟨by linarith, by linarith⟩
    exact_mod_cast this
  · rw [hf.2]
    obtain ⟨e1, e2⟩ := abs_le.mp hte
    rw [hTv]
    have key : u.val - 6755399441055744 - x.val * INVPIO2.val =
        (u.val - (t.val + 6755399441055744)) + (t.val - x.val * INVPIO2.val) := by ring
    rw [key]
    refine le_trans (abs_add_le _ _) (add_le_add ?_ hte)
    exact abs_le.mpr ⟨by linarith, by linarith⟩

end IEEE754.Dmath
