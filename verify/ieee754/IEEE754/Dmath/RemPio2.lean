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

/-- The first remainder: `f ⊗ PIO2_1` and `x ⊖ f·PIO2_1` are exact, and `|r| ≤ 0.78540`. -/
lemma rem1 {x f : F64} (hx : x.isFinite) (hb : |x.val| ≤ 2048) (hf : f.isFinite) {k : ℤ} (hk : f.val = k)
    (hk2 : |k| ≤ 1305) (hkx : |f.val - x.val * INVPIO2.val| ≤ 1 / 2 + 2 ^ (-43 : ℤ)) :
    (mul f PIO2_1).isFinite ∧ (mul f PIO2_1).val = k * PIO2_1.val ∧
      (sub x (mul f PIO2_1)).isFinite ∧ (sub x (mul f PIO2_1)).val = x.val - k * PIO2_1.val ∧
      |x.val - k * PIO2_1.val| ≤ 78540 / 100000 := by
  obtain ⟨hI, -, hP1, -⟩ := fin_consts
  have hIv : INVPIO2.val = 5734161139222659 / 2 ^ 53 := INVPIO2_val
  have hPv : PIO2_1.val = 1686629713 / 2 ^ 30 := PIO2_1_val
  -- `k·P1` is a datum
  have hm : Rep binary64 (k * PIO2_1.val) := by
    rw [hPv]
    have e : (k : ℚ) * (1686629713 / 2 ^ 30) = ((k * 1686629713 : ℤ) : ℚ) * 2 ^ (-30 : ℤ) := by
      push_cast; ring
    rw [e]
    refine rep_grid ?_ (by simp) ?_
    · rw [abs_mul]; simp only [binary64_prec]
      rw [abs_of_pos (by norm_num : (0 : ℤ) < 1686629713)]
      calc |k| * 1686629713 ≤ 1305 * 1686629713 := mul_le_mul_of_nonneg_right hk2 (by norm_num)
        _ ≤ 2 ^ 53 := by norm_num
    · rw [← e, abs_mul, abs_of_pos (by norm_num : (0 : ℚ) < 1686629713 / 2 ^ 30)]
      have : |(k : ℚ)| ≤ 1305 := by exact_mod_cast hk2
      calc |(k : ℚ)| * (1686629713 / 2 ^ 30) ≤ 1305 * (1686629713 / 2 ^ 30) :=
            mul_le_mul_of_nonneg_right this (by norm_num)
        _ < 2 ^ (binary64.emax + 1) := lt_of_lt_of_le (by norm_num : (1305 * (1686629713 / 2 ^ 30) : ℚ) < 2 ^ (12 : ℤ))
            (two_zpow_le (by simp))
  obtain ⟨hmf, hmv⟩ := mul_of_rep hf hP1 (by rw [hk]; exact hm)
  rw [hk] at hmv
  -- the bound on `r`
  have hrb : |x.val - k * PIO2_1.val| ≤ 78540 / 100000 := by
    have key : x.val - k * PIO2_1.val =
        PIO2_1.val * ((x.val * INVPIO2.val - k) + x.val * (1 / PIO2_1.val - INVPIO2.val)) := by
      rw [hPv]; field_simp; ring
    rw [key, abs_mul, hPv, abs_of_pos (by norm_num : (0:ℚ) < 1686629713 / 2 ^ 30)]
    have h1 : |x.val * INVPIO2.val - k| ≤ 1 / 2 + 2 ^ (-43 : ℤ) := by rw [abs_sub_comm, ← hk]; exact hkx
    have h2 : |x.val * (1 / (1686629713 / 2 ^ 30) - 5734161139222659 / 2 ^ 53)| ≤
        2048 * (1 / (1686629713 / 2 ^ 30) - 5734161139222659 / 2 ^ 53) := by
      rw [abs_mul, abs_of_pos (by norm_num : (0:ℚ) < 1 / (1686629713 / 2 ^ 30) - 5734161139222659 / 2 ^ 53)]
      exact mul_le_mul_of_nonneg_right hb (by norm_num)
    rw [hIv] at h1 ⊢
    calc 1686629713 / 2 ^ 30 * |x.val * (5734161139222659 / 2 ^ 53) - k +
          x.val * (1 / (1686629713 / 2 ^ 30) - 5734161139222659 / 2 ^ 53)|
        ≤ 1686629713 / 2 ^ 30 * ((1 / 2 + 2 ^ (-43 : ℤ)) +
          2048 * (1 / (1686629713 / 2 ^ 30) - 5734161139222659 / 2 ^ 53)) :=
          mul_le_mul_of_nonneg_left (le_trans (abs_add_le _ _) (add_le_add h1 h2)) (by norm_num)
      _ ≤ 78540 / 100000 := by norm_num
  -- `x ⊖ k·P1` is exact
  have hr : Rep binary64 (x.val - k * PIO2_1.val) := by
    by_cases hk0 : k = 0
    · rw [hk0]; simp only [Int.cast_zero, zero_mul, _root_.sub_zero]; exact rep_val hx
    · obtain ⟨e, he, hMe, hlt⟩ := mult_val hx
      -- `|x| ≥ 1/2`, so its grid is at least `2^-53`
      have hx2 : (1 : ℚ) / 2 ≤ |x.val| := by
        have hk1 : (1 : ℚ) ≤ |(k : ℚ)| := by exact_mod_cast Int.one_le_abs hk0
        have : |(k : ℚ)| ≤ |x.val * INVPIO2.val| + (1 / 2 + 2 ^ (-43 : ℤ)) := by
          have hkx' : |(k : ℚ) - x.val * INVPIO2.val| ≤ 1 / 2 + 2 ^ (-43 : ℤ) := by rw [← hk]; exact hkx
          have := abs_sub_abs_le_abs_sub (k : ℚ) (x.val * INVPIO2.val)
          linarith
        rw [abs_mul, hIv, abs_of_pos (by norm_num : (0:ℚ) < 5734161139222659 / 2 ^ 53)] at this
        have h3 : (2 : ℚ) ^ (-43 : ℤ) ≤ 1 / 1000 := by norm_num
        by_contra hc; rw [not_le] at hc
        have := mul_lt_mul_of_pos_right hc (by norm_num : (0:ℚ) < 5734161139222659 / 2 ^ 53)
        norm_num at this ⊢; linarith
      have he53 : -53 ≤ e := by
        by_contra hc; rw [not_le] at hc
        have : |x.val| < 2 ^ ((53 : ℤ) + e) := by
          rw [two_zpow_add, ← zpow_natCast] at *; simpa [binary64_prec] using hlt
        have : (2 : ℚ) ^ ((53 : ℤ) + e) ≤ 2 ^ (-1 : ℤ) := two_zpow_le (by omega)
        norm_num at this; linarith
      have hM1 : Mult (k * PIO2_1.val) (-30) := by
        rw [hPv]; exact ⟨k * 1686629713, by push_cast; ring⟩
      have hMr : Mult (x.val - k * PIO2_1.val) (min e (-30)) :=
        mult_sub (mult_of_le hMe (min_le_left _ _)) (mult_of_le hM1 (min_le_right _ _))
      refine rep_of_mult hMr (by simp; omega) ?_ (lt_of_le_of_lt hrb (lt_of_lt_of_le (by norm_num : (78540 / 100000 : ℚ) < 2 ^ (11:ℤ)) (two_zpow_le (by simp))))
      calc |x.val - k * PIO2_1.val| ≤ 1 := le_trans hrb (by norm_num)
        _ = 2 ^ (53 : ℕ) * 2 ^ (-53 : ℤ) := by rw [← zpow_natCast, ← two_zpow_add]; norm_num
        _ ≤ 2 ^ binary64.prec * 2 ^ (min e (-30)) := by
          simp only [binary64_prec]
          exact mul_le_mul_of_nonneg_left (two_zpow_le (by omega)) (by positivity)
  obtain ⟨hrf, hrv⟩ := sub_of_rep hx hmf (by rw [hmv]; exact hr)
  rw [hmv] at hrv
  exact ⟨hmf, hmv, hrf, hrv, hrb⟩

/-! ## The two-part tail `(y0, y1)` -/

lemma fin_of_le_one {zs : Bool} {r : ℚ} (h : |r| ≤ 16) : (round binary64 zs r).isFinite :=
  round_finite_of_le_emax (le_trans h (le_trans (by norm_num : (16 : ℚ) ≤ 2 ^ (4 : ℤ)) (two_zpow_le (by simp))))

lemma sub_fin_le {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (h : |a.val - b.val| ≤ 16) :
    (sub a b).isFinite := by rw [sub_eq_round ha hb]; exact fin_of_le_one h

lemma sub_err {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (h : |a.val - b.val| ≤ 16) :
    |(sub a b).val - (a.val - b.val)| ≤ 2 ^ (-53 : ℤ) * |(sub a b).val| := by
  simpa using sub_err_res ha hb (sub_fin_le ha hb h)

/-- `y0 = r ⊖ w`, `y1 = (r ⊖ y0) ⊖ w`: `y0 + y1` is `r − w`, exactly when `|w| ≤ |r|`, and within
`2^-49·|w|` otherwise; `|y1| ≤ 2^-53·|y0| + 8·|w|`. -/
lemma tail {r w : F64} (hr : r.isFinite) (hw : w.isFinite) (hrb : |r.val| ≤ 1) (hwb : |w.val| ≤ 1) :
    (sub r w).isFinite ∧ (sub (sub r (sub r w)) w).isFinite ∧
      |(sub r w).val + (sub (sub r (sub r w)) w).val - (r.val - w.val)| ≤ 2 ^ (-49 : ℤ) * |w.val| ∧
      |(sub (sub r (sub r w)) w).val| ≤ 2 ^ (-53 : ℤ) * |(sub r w).val| + 8 * |w.val| := by
  set y0 := sub r w
  have hrw : |r.val - w.val| ≤ 2 := le_trans (abs_sub _ _) (by linarith)
  have hy0 : y0.isFinite := sub_fin_le hr hw (by linarith)
  have e0 : |y0.val - (r.val - w.val)| ≤ 2 ^ (-53 : ℤ) * |y0.val| := sub_err hr hw (by linarith)
  have hp : (2 : ℚ) ^ (-53 : ℤ) ≤ 1 / 8 := by norm_num
  have hy0b : |y0.val| ≤ 3 := by
    have a1 := abs_sub_abs_le_abs_sub y0.val (r.val - w.val)
    have a2 : 2 ^ (-53 : ℤ) * |y0.val| ≤ |y0.val| / 8 := by nlinarith [abs_nonneg y0.val]
    have a3 := le_trans e0 a2
    linarith
  have hz' : |r.val - y0.val| ≤ 16 := le_trans (abs_sub _ _) (by linarith)
  set z := sub r y0
  have hz : z.isFinite := sub_fin_le hr hy0 hz'
  rcases le_or_gt |w.val| |r.val| with hle | hlt
  · -- Fast2Sum: `r ⊖ y0` is exact, and so is `y1`
    have hwn : w.neg.isFinite := by simpa using hw
    have hy0' : add r w.neg = y0 := rfl
    have hf := fast2sum hr hwn (by rw [hy0']; exact hy0) (by rw [FP.val_neg, abs_neg]; exact hle)
    rw [hy0'] at hf
    have hrep : Rep binary64 (r.val - y0.val) := by
      have h1 : Rep binary64 (y0.val - r.val) := by
        have h2 := hf; rw [sub_eq_round hy0 hr] at h2; exact rep_of_round h2
      have h3 := rep_neg h1; rwa [show -(y0.val - r.val) = r.val - y0.val by ring] at h3
    obtain ⟨-, hzv⟩ := sub_of_rep hr hy0 hrep
    have herr : Rep binary64 (z.val - w.val) := by
      rw [hzv]
      have h4 := add_err_representable hr hwn (by rw [hy0']; exact hy0)
      rw [hy0', FP.val_neg] at h4
      have h2 : Rep binary64 (r.val + -w.val - y0.val) := fun zs => h4 zs
      rwa [show r.val + -w.val - y0.val = r.val - y0.val - w.val by ring] at h2
    obtain ⟨hy1, hy1v⟩ := sub_of_rep hz hw herr
    refine ⟨hy0, hy1, ?_, ?_⟩
    · rw [hy1v, hzv, show y0.val + (r.val - y0.val - w.val) - (r.val - w.val) = 0 by ring, abs_zero]
      positivity
    · rw [hy1v, hzv, show r.val - y0.val - w.val = -(y0.val - (r.val - w.val)) by ring, abs_neg]
      linarith [abs_nonneg w.val]
  · -- `|r| < |w|`: everything is within a few `|w|`
    have hy0w : |y0.val| ≤ 3 * |w.val| := by
      have a1 := abs_sub_abs_le_abs_sub y0.val (r.val - w.val)
      have h1 : |r.val - w.val| ≤ 2 * |w.val| := le_trans (abs_sub _ _) (by linarith)
      have a2 : 2 ^ (-53 : ℤ) * |y0.val| ≤ |y0.val| / 8 := by nlinarith [abs_nonneg y0.val]
      have a3 := le_trans e0 a2
      linarith
    have ez : |z.val - (r.val - y0.val)| ≤ 2 ^ (-53 : ℤ) * |z.val| := sub_err hr hy0 hz'
    have hzw : |z.val| ≤ 5 * |w.val| := by
      have a1 := abs_sub_abs_le_abs_sub z.val (r.val - y0.val)
      have h1 : |r.val - y0.val| ≤ 4 * |w.val| := le_trans (abs_sub _ _) (by linarith)
      have a2 : 2 ^ (-53 : ℤ) * |z.val| ≤ |z.val| / 8 := by nlinarith [abs_nonneg z.val]
      have a3 := le_trans ez a2
      linarith
    have hzw' : |z.val - w.val| ≤ 16 := le_trans (abs_sub _ _) (by linarith)
    have hy1 := sub_fin_le hz hw hzw'
    have e1 := sub_err hz hw hzw'
    have hy1w : |(sub z w).val| ≤ 7 * |w.val| := by
      have a1 := abs_sub_abs_le_abs_sub (sub z w).val (z.val - w.val)
      have h1 : |z.val - w.val| ≤ 6 * |w.val| := le_trans (abs_sub _ _) (by linarith)
      have a2 : 2 ^ (-53 : ℤ) * |(sub z w).val| ≤ |(sub z w).val| / 8 := by nlinarith [abs_nonneg (sub z w).val]
      have a3 := le_trans e1 a2
      linarith
    refine ⟨hy0, hy1, ?_, ?_⟩
    · have key : y0.val + (sub z w).val - (r.val - w.val) =
          ((sub z w).val - (z.val - w.val)) + (z.val - (r.val - y0.val)) := by ring
      rw [key]
      calc _ ≤ |(sub z w).val - (z.val - w.val)| + |z.val - (r.val - y0.val)| := abs_add_le _ _
        _ ≤ 2 ^ (-53 : ℤ) * (7 * |w.val|) + 2 ^ (-53 : ℤ) * (5 * |w.val|) :=
            add_le_add (le_trans e1 (mul_le_mul_of_nonneg_left hy1w (by positivity)))
              (le_trans ez (mul_le_mul_of_nonneg_left hzw (by positivity)))
        _ ≤ 2 ^ (-49 : ℤ) * |w.val| := by
            have : (2 : ℚ) ^ (-53 : ℤ) * 12 ≤ 2 ^ (-49 : ℤ) := by norm_num
            nlinarith [abs_nonneg w.val]
    · nlinarith [abs_nonneg y0.val, abs_nonneg w.val, two_zpow_pos (K := ℚ) (-53)]

/-! ## One refinement step: `r' = t ⊖ k·P`, `w' = k·PT ⊖ ((t ⊖ r') ⊖ k·P)` -/

lemma mul_fin_small {a b : F64} (ha : a.isFinite) (hb : b.isFinite) (h : |a.val * b.val| ≤ 16) :
    (mul a b).isFinite := by rw [mul_eq_round ha hb]; exact fin_of_le_one h

/-- The relative error `2^-53` as a numeral, for `linarith`. -/
lemma u53 : (2 : ℚ) ^ (-53 : ℤ) = 1 / 9007199254740992 := by norm_num

/-- From `|v − x| ≤ 2^-53·|v|` and `|x| ≤ B`: `|v| ≤ 8/7·B`. -/
lemma rel_bound {v x B : ℚ} (h : |v - x| ≤ 2 ^ (-53 : ℤ) * |v|) (hx : |x| ≤ B) : |v| ≤ 8 / 7 * B := by
  rw [u53] at h
  have := abs_sub_abs_le_abs_sub v x
  have : 0 ≤ |v| := abs_nonneg v
  linarith

lemma refine {t f P PT : F64} (ht : t.isFinite) (hf : f.isFinite) (hP : P.isFinite) (hPT : PT.isFinite)
    {k : ℤ} (hk : f.val = k) (htb : |t.val| ≤ 1) (hmP : Rep binary64 (k * P.val))
    (hkP : |k * P.val| ≤ 1 / 524288) (hkPT : |k * PT.val| ≤ 1 / 576460752303423488) :
    (sub t (mul f P)).isFinite ∧
      (sub (mul f PT) (sub (sub t (sub t (mul f P))) (mul f P))).isFinite ∧
      |(sub t (mul f P)).val - (t.val - k * P.val)| ≤ 2 ^ (-53 : ℤ) * |(sub t (mul f P)).val| ∧
      |(sub t (mul f P)).val| ≤ |t.val| + 1 / 262144 ∧
      |(sub (mul f PT) (sub (sub t (sub t (mul f P))) (mul f P))).val| ≤ 1 / 32768 ∧
      |((sub t (mul f P)).val - (sub (mul f PT) (sub (sub t (sub t (mul f P))) (mul f P))).val) -
        (t.val - k * P.val - k * PT.val)| ≤ 1 / 4611686018427387904 := by
  obtain ⟨hmf, hmv⟩ := mul_of_rep hf hP (by rw [hk]; exact hmP)
  rw [hk] at hmv
  set mP := mul f P
  have hkP' : |mP.val| ≤ 1 / 524288 := by rw [hmv]; exact hkP
  -- `r' = t ⊖ k·P`
  have h1 : |t.val - mP.val| ≤ 16 := by have := abs_sub t.val mP.val; linarith
  set r' := sub t mP
  have hr' : r'.isFinite := sub_fin_le ht hmf h1
  have e1 : |r'.val - (t.val - mP.val)| ≤ 2 ^ (-53 : ℤ) * |r'.val| := sub_err ht hmf h1
  have e1' := e1; rw [u53] at e1'
  have hr'2 : |r'.val| ≤ 2 := le_trans (rel_bound e1 (le_trans (abs_sub _ _) (by linarith : |t.val| + |mP.val| ≤ 7 / 4)))
    (by norm_num)
  have hr'b : |r'.val| ≤ |t.val| + 1 / 262144 := by
    have := abs_sub_abs_le_abs_sub r'.val (t.val - mP.val)
    have := abs_sub t.val mP.val
    linarith
  -- `z = t ⊖ r'`, `e = z ⊖ k·P`
  have hz' : |t.val - r'.val| ≤ 16 := by have := abs_sub t.val r'.val; linarith
  set z := sub t r'
  have hz : z.isFinite := sub_fin_le ht hr' hz'
  have ez : |z.val - (t.val - r'.val)| ≤ 2 ^ (-53 : ℤ) * |z.val| := sub_err ht hr' hz'
  have htr : |t.val - r'.val| ≤ 1 / 524288 + 1 / 4503599627370496 := by
    have := abs_le.mp e1'; have := abs_le.mp hkP'
    exact abs_le.mpr ⟨by linarith, by linarith⟩
  have hzb : |z.val| ≤ 1 / 262144 := le_trans (rel_bound ez htr) (by norm_num)
  have ez' := ez; rw [u53] at ez'
  have he' : |z.val - mP.val| ≤ 16 := by have := abs_sub z.val mP.val; linarith
  set e := sub z mP
  have he : e.isFinite := sub_fin_le hz hmf he'
  have ee : |e.val - (z.val - mP.val)| ≤ 2 ^ (-53 : ℤ) * |e.val| := sub_err hz hmf he'
  have heb : |e.val| ≤ 1 / 131072 :=
    le_trans (rel_bound ee (le_trans (abs_sub _ _) (by linarith : |z.val| + |mP.val| ≤ 3 / 524288))) (by norm_num)
  have ee' := ee; rw [u53] at ee'
  -- the errors of `z` and `e`: zero when `|k·P| ≤ |t|`, else below `2^-68`
  have hsmall : |z.val - (t.val - r'.val)| + |e.val - (z.val - mP.val)| ≤ 1 / 295147905179352825856 := by
    rcases le_or_gt |mP.val| |t.val| with hle | hlt
    · have hmn : mP.neg.isFinite := by simpa using hmf
      have hr'' : add t mP.neg = r' := rfl
      have hf2' := fast2sum ht hmn (by rw [hr'']; exact hr') (by rw [FP.val_neg, abs_neg]; exact hle)
      have hf2 : (sub r' t).isFinite ∧ (sub r' t).val = r'.val - t.val := hf2'
      have hrep : Rep binary64 (t.val - r'.val) := by
        have h1 : Rep binary64 (r'.val - t.val) := by
          have h2 := hf2; rw [sub_eq_round hr' ht] at h2; exact rep_of_round h2
        have h3 := rep_neg h1; rwa [show -(r'.val - t.val) = t.val - r'.val by ring] at h3
      obtain ⟨-, hzv⟩ := sub_of_rep ht hr' hrep
      have herr : Rep binary64 (z.val - mP.val) := by
        rw [hzv]
        have h4 := add_err_representable ht hmn (by rw [hr'']; exact hr')
        rw [hr'', FP.val_neg] at h4
        have h2 : Rep binary64 (t.val + -mP.val - r'.val) := fun zs => h4 zs
        rwa [show t.val + -mP.val - r'.val = t.val - r'.val - mP.val by ring] at h2
      obtain ⟨-, hev⟩ := sub_of_rep hz hmf herr
      rw [hev, hzv, _root_.sub_self, _root_.sub_self, abs_zero]; norm_num
    · have htk : |t.val| ≤ 1 / 524288 := by linarith
      have hr2 : |r'.val| ≤ 1 / 131072 := by
        have := abs_sub_abs_le_abs_sub r'.val (t.val - mP.val)
        have := abs_sub t.val mP.val
        linarith
      have hz2 : |z.val| ≤ 1 / 32768 := by
        have := abs_sub_abs_le_abs_sub z.val (t.val - r'.val)
        have := abs_sub t.val r'.val
        linarith
      have he2 : |e.val| ≤ 1 / 16384 := by
        have := abs_sub_abs_le_abs_sub e.val (z.val - mP.val)
        have := abs_sub z.val mP.val
        linarith
      linarith
  -- `w' = k·PT ⊖ e`
  have hmPT : |f.val * PT.val| ≤ 16 := by rw [hk]; linarith
  have hmTf := mul_fin_small hf hPT hmPT
  set mPT := mul f PT
  have e5 := mul_err_abs hf hPT hmTf
  simp only [binary64_prec, binary64_qmin, hk] at e5
  have e5' : |mPT.val - k * PT.val| ≤ 1 / 9007199254740992 * |mPT.val| + 1 / 2 ^ 100 := by
    have h2 : (2 : ℚ) ^ (-1074 - 1 : ℤ) ≤ 1 / 2 ^ 100 :=
      le_trans (two_zpow_le (by norm_num : (-1074 - 1 : ℤ) ≤ -100)) (by norm_num)
    have h3 : (2 : ℚ) ^ (-((53 : ℕ) : ℤ)) = 1 / 9007199254740992 := by norm_num
    rw [h3] at e5; linarith
  have hmPTb : |mPT.val| ≤ 1 / 288230376151711744 := by
    have := abs_sub_abs_le_abs_sub mPT.val (k * PT.val)
    linarith
  have hw'' : |mPT.val - e.val| ≤ 16 := by have := abs_sub mPT.val e.val; linarith
  have hw' := sub_fin_le hmTf he hw''
  have e4 : |(sub mPT e).val - (mPT.val - e.val)| ≤ 2 ^ (-53 : ℤ) * |(sub mPT e).val| := sub_err hmTf he hw''
  have hw'b : |(sub mPT e).val| ≤ 1 / 32768 :=
    le_trans (rel_bound e4 (le_trans (abs_sub _ _) (by linarith : |mPT.val| + |e.val| ≤ 1 / 65536))) (by norm_num)
  have e4' := e4; rw [u53] at e4'
  refine ⟨hr', hw', by rw [← hmv]; exact e1, hr'b, hw'b, ?_⟩
  have key : (r'.val - (sub mPT e).val) - (t.val - k * P.val - k * PT.val) =
      (z.val - (t.val - r'.val)) + (e.val - (z.val - mP.val)) - ((sub mPT e).val - (mPT.val - e.val)) -
        (mPT.val - k * PT.val) := by rw [hmv]; ring
  rw [key]
  have a1 := abs_add_le (z.val - (t.val - r'.val)) (e.val - (z.val - mP.val))
  have a2 := abs_sub ((z.val - (t.val - r'.val)) + (e.val - (z.val - mP.val)))
    ((sub mPT e).val - (mPT.val - e.val))
  have a3 := abs_sub ((z.val - (t.val - r'.val)) + (e.val - (z.val - mP.val)) -
    ((sub mPT e).val - (mPT.val - e.val))) (mPT.val - k * PT.val)
  linarith

end IEEE754.Dmath
