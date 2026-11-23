/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Apart
import IEEE754.Screen.Gap

/-!
# decide-once's `near_f` (item 9, abs-dot form, ffb810a)

At one corner `c` of the f64 extent (each coordinate one rounding off the exact corner `C`), with
both normals f64 exactly and offsets within `2ε`:

- `e_close`: `e = tc·ns2 − sc·nts` in f64 is within `err + η` of the exact
  `E = T(C)|nS|² − S(C)(nT·nS)`, where `err = 1e-12·m` and
  `m = (|nT|·|c| + |tc|)·ns2 + (|nS|·|c| + |sc|)·Σ|s_i||t_i|` (the abs-dot; `|nts|` would not
  cover `sc`'s error when the normals are near perpendicular).
- `r_close`: `r = d·√nt2·ns2` in f64 is within `40ε` of `R = d·|nT|·|nS|²`, relatively.
- `near_f_past`: if `|e| − err > r·(1.0 + 1e-12)` in f64 (`Some(false)`), then `near` fails at `C`.
- `near_f_within`: if `|e| + err < r·(1.0 − 1e-12)` in f64 (the corner keeps `all`), then `near`
  holds at `C`.

So `Some(false)` means `near` fails at that corner, and `Some(true)` means it holds at all eight:
the exact `near` over the same corners. Each product is zero or normal (`NZ`), and the squared
normals and `d` are at least `2^-100`.
-/

namespace IEEE754.Screen

open FP

/-- `|nT|·|c| + |tc|`-type magnitude, then times a weight: the halves of `m`. -/
def magF (n0 n1 n2 c x y z : F64) : F64 := add (dot3 n0.abs n1.abs n2.abs x.abs y.abs z.abs) c.abs

/-- `e = tc*ns2 - sc*nts`. -/
def eF (s0 s1 s2 sc t0 t1 t2 tc x y z : F64) : F64 :=
  sub (mul (sub (dot3 t0 t1 t2 x y z) tc) (dot3 s0 s1 s2 s0 s1 s2))
    (mul (sub (dot3 s0 s1 s2 x y z) sc) (dot3 t0 t1 t2 s0 s1 s2))

/-- `m = (|t.n|·|c| + |t.c|)*ns2 + (|s.n|·|c| + |s.c|)*nts_abs`. -/
def mF (s0 s1 s2 sc t0 t1 t2 tc x y z : F64) : F64 :=
  add (mul (magF t0 t1 t2 tc x y z) (dot3 s0 s1 s2 s0 s1 s2))
    (mul (magF s0 s1 s2 sc x y z) (dot3 s0.abs s1.abs s2.abs t0.abs t1.abs t2.abs))

/-- `r = d * nt2.sqrt() * ns2`. -/
def rF (s0 s1 s2 t0 t1 t2 d : F64) : F64 :=
  mul (mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))) (dot3 s0 s1 s2 s0 s1 s2)

/-- The last step of `e_close`: `38ε·M` is far inside `1e-12·m`. -/
lemma e_fin {e E err m ME MM : ℚ} (h1 : |e - E| ≤ 38 * ε * ME) (hME : ME = MM) (h0 : 0 ≤ MM)
    (h2 : (1 - 38 * ε) * MM ≤ m)
    (h3 : |err - 4951760157141521 / 2 ^ 92 * m| ≤ ε * |4951760157141521 / 2 ^ 92 * m| + η) :
    |e - E| ≤ err + η := by
  subst hME
  have hm : 0 ≤ m := le_trans (by nlinarith) h2
  rw [abs_of_nonneg (by positivity : (0 : ℚ) ≤ 4951760157141521 / 2 ^ 92 * m)] at h3
  have := (abs_le.mp h3).1
  nlinarith

set_option maxHeartbeats 4000000 in
/-- `e` is within `err + η` of the exact `E`. -/
theorem e_close {s0 s1 s2 sc t0 t1 t2 tc x y z : F64} {Sc Tc X Y Z : ℚ}
    (hs0 : s0.isFinite) (hs1 : s1.isFinite) (hs2 : s2.isFinite)
    (ht0 : t0.isFinite) (ht1 : t1.isFinite) (ht2 : t2.isFinite)
    (hsc : Tr sc Sc |Sc| 2) (htc : Tr tc Tc |Tc| 2)
    (hx : Tr x X |X| 1) (hy : Tr y Y |Y| 1) (hz : Tr z Z |Z| 1)
    (hfe : (eF s0 s1 s2 sc t0 t1 t2 tc x y z).isFinite)
    (hfm : (mF s0 s1 s2 sc t0 t1 t2 tc x y z).isFinite)
    (hferr : (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z)).isFinite)
    (hU : ∀ r ∈ [s0.val * x.val, s1.val * y.val, s2.val * z.val, t0.val * x.val, t1.val * y.val,
      t2.val * z.val, s0.val * s0.val, s1.val * s1.val, s2.val * s2.val, t0.val * s0.val, t1.val * s1.val,
      t2.val * s2.val,
      (sub (dot3 t0 t1 t2 x y z) tc).val * (dot3 s0 s1 s2 s0 s1 s2).val,
      (sub (dot3 s0 s1 s2 x y z) sc).val * (dot3 t0 t1 t2 s0 s1 s2).val,
      |t0.val| * |x.val|, |t1.val| * |y.val|, |t2.val| * |z.val|,
      |s0.val| * |x.val|, |s1.val| * |y.val|, |s2.val| * |z.val|,
      |s0.val| * |t0.val|, |s1.val| * |t1.val|, |s2.val| * |t2.val|,
      (magF t0 t1 t2 tc x y z).val * (dot3 s0 s1 s2 s0 s1 s2).val,
      (magF s0 s1 s2 sc x y z).val * (dot3 s0.abs s1.abs s2.abs t0.abs t1.abs t2.abs).val], NZ r) :
    |(eF s0 s1 s2 sc t0 t1 t2 tc x y z).val -
        ((t0.val * X + t1.val * Y + t2.val * Z - Tc) * (s0.val * s0.val + s1.val * s1.val + s2.val * s2.val) -
          (s0.val * X + s1.val * Y + s2.val * Z - Sc) * (t0.val * s0.val + t1.val * s1.val + t2.val * s2.val))| ≤
      (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z)).val + η := by
  have u : ∀ i, (h : i < 25) → NZ ([s0.val * x.val, s1.val * y.val, s2.val * z.val, t0.val * x.val, t1.val * y.val,
      t2.val * z.val, s0.val * s0.val, s1.val * s1.val, s2.val * s2.val, t0.val * s0.val, t1.val * s1.val,
      t2.val * s2.val,
      (sub (dot3 t0 t1 t2 x y z) tc).val * (dot3 s0 s1 s2 s0 s1 s2).val,
      (sub (dot3 s0 s1 s2 x y z) sc).val * (dot3 t0 t1 t2 s0 s1 s2).val,
      |t0.val| * |x.val|, |t1.val| * |y.val|, |t2.val| * |z.val|,
      |s0.val| * |x.val|, |s1.val| * |y.val|, |s2.val| * |z.val|,
      |s0.val| * |t0.val|, |s1.val| * |t1.val|, |s2.val| * |t2.val|,
      (magF t0 t1 t2 tc x y z).val * (dot3 s0 s1 s2 s0 s1 s2).val,
      (magF s0 s1 s2 sc x y z).val * (dot3 s0.abs s1.abs s2.abs t0.abs t1.abs t2.abs).val].get ⟨i, by simpa using h⟩) :=
    fun i h => hU _ (List.get_mem _ _)
  -- inputs
  have es0 := Tr.exact hs0; have es1 := Tr.exact hs1; have es2 := Tr.exact hs2
  have et0 := Tr.exact ht0; have et1 := Tr.exact ht1; have et2 := Tr.exact ht2
  have w : ∀ {v : F64} {X M k : ℚ}, Tr v X M k → 0 ≤ k → ∀ k', k ≤ k' → Tr v X M k' :=
    fun h hk k' hk' => h.weaken hk' h.M_nonneg
  have hx8 := w hx (by norm_num) 8 (by norm_num); have hy8 := w hy (by norm_num) 8 (by norm_num)
  have hz8 := w hz (by norm_num) 8 (by norm_num)
  -- the f64 terms of `e`
  unfold eF at hfe ⊢
  have fA := (isFinite_of_sub hfe).1; have fB := (isFinite_of_sub hfe).2
  have fTC := (isFinite_of_mul fA).1; have fNS := (isFinite_of_mul fA).2
  have fSC := (isFinite_of_mul fB).1; have fNTS := (isFinite_of_mul fB).2
  have tT := tr_dot3 (w et0 le_rfl 2 (by norm_num)) (w et1 le_rfl 2 (by norm_num)) (w et2 le_rfl 2 (by norm_num))
    hx8 hy8 hz8 (isFinite_of_sub fTC).1 (u 3 (by norm_num)) (u 4 (by norm_num)) (u 5 (by norm_num))
  have tS := tr_dot3 (w es0 le_rfl 2 (by norm_num)) (w es1 le_rfl 2 (by norm_num)) (w es2 le_rfl 2 (by norm_num))
    hx8 hy8 hz8 (isFinite_of_sub fSC).1 (u 0 (by norm_num)) (u 1 (by norm_num)) (u 2 (by norm_num))
  have tTv := tr_sub tT (w htc (by norm_num) 16 (by norm_num)) (by norm_num) (by norm_num) fTC
  have tSv := tr_sub tS (w hsc (by norm_num) 16 (by norm_num)) (by norm_num) (by norm_num) fSC
  have tSS := tr_dot3 (w es0 le_rfl 2 (by norm_num)) (w es1 le_rfl 2 (by norm_num)) (w es2 le_rfl 2 (by norm_num))
    (w es0 le_rfl 8 (by norm_num)) (w es1 le_rfl 8 (by norm_num)) (w es2 le_rfl 8 (by norm_num)) fNS
    (u 6 (by norm_num)) (u 7 (by norm_num)) (u 8 (by norm_num))
  have tTS := tr_dot3 (w et0 le_rfl 2 (by norm_num)) (w et1 le_rfl 2 (by norm_num)) (w et2 le_rfl 2 (by norm_num))
    (w es0 le_rfl 8 (by norm_num)) (w es1 le_rfl 8 (by norm_num)) (w es2 le_rfl 8 (by norm_num)) fNTS
    (u 9 (by norm_num)) (u 10 (by norm_num)) (u 11 (by norm_num))
  norm_num at tTv tSv
  have mA := tr_mul tTv tSS (by norm_num) (by norm_num) (by norm_num) (by norm_num) fA (u 12 (by norm_num))
  have mB := tr_mul tSv tTS (by norm_num) (by norm_num) (by norm_num) (by norm_num) fB (u 13 (by norm_num))
  norm_num at mA mB
  have tE := tr_sub mA mB (by norm_num) (by norm_num) hfe
  norm_num at tE
  -- the f64 `m`, from below
  unfold mF at hfm hferr
  have gA := (isFinite_of_add hfm).1; have gB := (isFinite_of_add hfm).2
  have gMT := (isFinite_of_mul gA).1; have gMS := (isFinite_of_mul gB).1
  have gNA := (isFinite_of_mul gB).2
  unfold magF at gMT gMS
  have aT := tr_dot3 (w et0.abs le_rfl 2 (by norm_num)) (w et1.abs le_rfl 2 (by norm_num))
    (w et2.abs le_rfl 2 (by norm_num)) (w hx.abs (by norm_num) 8 (by norm_num)) (w hy.abs (by norm_num) 8 (by norm_num))
    (w hz.abs (by norm_num) 8 (by norm_num)) (isFinite_of_add gMT).1
    (by rw [FP.abs_val, FP.abs_val]; exact u 14 (by norm_num)) (by rw [FP.abs_val, FP.abs_val]; exact u 15 (by norm_num)) (by rw [FP.abs_val, FP.abs_val]; exact u 16 (by norm_num))
  have aS := tr_dot3 (w es0.abs le_rfl 2 (by norm_num)) (w es1.abs le_rfl 2 (by norm_num))
    (w es2.abs le_rfl 2 (by norm_num)) (w hx.abs (by norm_num) 8 (by norm_num)) (w hy.abs (by norm_num) 8 (by norm_num))
    (w hz.abs (by norm_num) 8 (by norm_num)) (isFinite_of_add gMS).1
    (by rw [FP.abs_val, FP.abs_val]; exact u 17 (by norm_num)) (by rw [FP.abs_val, FP.abs_val]; exact u 18 (by norm_num)) (by rw [FP.abs_val, FP.abs_val]; exact u 19 (by norm_num))
  have aNA := tr_dot3 (w es0.abs le_rfl 2 (by norm_num)) (w es1.abs le_rfl 2 (by norm_num))
    (w es2.abs le_rfl 2 (by norm_num)) (w et0.abs le_rfl 8 (by norm_num)) (w et1.abs le_rfl 8 (by norm_num))
    (w et2.abs le_rfl 8 (by norm_num)) gNA
    (by rw [FP.abs_val, FP.abs_val]; exact u 20 (by norm_num)) (by rw [FP.abs_val, FP.abs_val]; exact u 21 (by norm_num)) (by rw [FP.abs_val, FP.abs_val]; exact u 22 (by norm_num))
  have aMT := tr_add aT (w htc.abs (by norm_num) 16 (by norm_num)) (by norm_num) (by norm_num) gMT
  have aMS := tr_add aS (w hsc.abs (by norm_num) 16 (by norm_num)) (by norm_num) (by norm_num) gMS
  norm_num at aMT aMS
  have bA := tr_mul aMT tSS (by norm_num) (by norm_num) (by norm_num) (by norm_num) gA
    (by simpa [magF] using u 23 (by norm_num))
  have bB := tr_mul aMS aNA (by norm_num) (by norm_num) (by norm_num) (by norm_num) gB
    (by simpa [magF] using u 24 (by norm_num))
  norm_num at bA bB
  have tM := tr_add bA bB (by norm_num) (by norm_num) hfm
  norm_num at tM
  have fE : E12.isFinite := by decide
  have em := mul_rel fE hfm hferr
  rw [E12_val] at em
  unfold mF
  exact e_fin tE.err (by ring) tM.M_nonneg tM.lower em

/-- `2^-53` in `ℝ`. -/
local notation "εr" => (1 / 9007199254740992 : ℝ)

set_option maxHeartbeats 4000000 in
/-- `r` is within `40ε` of `R = d·|nT|·|nS|²`, relatively. -/
theorem r_close {s0 s1 s2 t0 t1 t2 d : F64}
    (hs0 : s0.isFinite) (hs1 : s1.isFinite) (hs2 : s2.isFinite)
    (ht0 : t0.isFinite) (ht1 : t1.isFinite) (ht2 : t2.isFinite)
    (hSS : 1 / 2 ^ 100 ≤ s0.val * s0.val + s1.val * s1.val + s2.val * s2.val)
    (hTT : 1 / 2 ^ 100 ≤ t0.val * t0.val + t1.val * t1.val + t2.val * t2.val)
    (hd : 1 / 2 ^ 100 ≤ d.val)
    (hfr : (rF s0 s1 s2 t0 t1 t2 d).isFinite) (fNT : (dot3 t0 t1 t2 t0 t1 t2).isFinite)
    (hU : ∀ r ∈ [s0.val * s0.val, s1.val * s1.val, s2.val * s2.val, t0.val * t0.val, t1.val * t1.val,
      t2.val * t2.val, d.val * (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2)).val,
      (mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))).val * (dot3 s0 s1 s2 s0 s1 s2).val], NZ r) :
    |((rF s0 s1 s2 t0 t1 t2 d).val : ℝ) - (d.val : ℝ) * √((t0.val * t0.val + t1.val * t1.val + t2.val * t2.val : ℚ) : ℝ) *
        ((s0.val * s0.val + s1.val * s1.val + s2.val * s2.val : ℚ) : ℝ)| ≤
      40 * εr * ((d.val : ℝ) * √((t0.val * t0.val + t1.val * t1.val + t2.val * t2.val : ℚ) : ℝ) *
        ((s0.val * s0.val + s1.val * s1.val + s2.val * s2.val : ℚ) : ℝ)) := by
  have u : ∀ i, (h : i < 8) → NZ ([s0.val * s0.val, s1.val * s1.val, s2.val * s2.val, t0.val * t0.val, t1.val * t1.val,
      t2.val * t2.val, d.val * (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2)).val,
      (mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))).val * (dot3 s0 s1 s2 s0 s1 s2).val].get ⟨i, by simpa using h⟩) :=
    fun i h => hU _ (List.get_mem _ _)
  have es0 := Tr.exact hs0; have es1 := Tr.exact hs1; have es2 := Tr.exact hs2
  have et0 := Tr.exact ht0; have et1 := Tr.exact ht1; have et2 := Tr.exact ht2
  have w : ∀ {v : F64} {X M k : ℚ}, Tr v X M k → 0 ≤ k → ∀ k', k ≤ k' → Tr v X M k' :=
    fun h hk k' hk' => h.weaken hk' h.M_nonneg
  unfold rF at hfr ⊢
  have fP := (isFinite_of_mul hfr).1; have fNS := (isFinite_of_mul hfr).2
  have fQ := (isFinite_of_mul fP).2
  have tSS := tr_dot3 (w es0 le_rfl 2 (by norm_num)) (w es1 le_rfl 2 (by norm_num)) (w es2 le_rfl 2 (by norm_num))
    (w es0 le_rfl 8 (by norm_num)) (w es1 le_rfl 8 (by norm_num)) (w es2 le_rfl 8 (by norm_num)) fNS
    (u 0 (by norm_num)) (u 1 (by norm_num)) (u 2 (by norm_num))
  have tTT := tr_dot3 (w et0 le_rfl 2 (by norm_num)) (w et1 le_rfl 2 (by norm_num)) (w et2 le_rfl 2 (by norm_num))
    (w et0 le_rfl 8 (by norm_num)) (w et1 le_rfl 8 (by norm_num)) (w et2 le_rfl 8 (by norm_num)) fNT
    (u 3 (by norm_num)) (u 4 (by norm_num)) (u 5 (by norm_num))
  norm_num at tSS tTT
  set Ss := s0.val * s0.val + s1.val * s1.val + s2.val * s2.val with hSsd
  set Tt := t0.val * t0.val + t1.val * t1.val + t2.val * t2.val with hTtd
  have eS := tSS.err; have eT := tTT.err
  set q := (dot3 t0 t1 t2 t0 t1 t2).val
  set ns := (dot3 s0 s1 s2 s0 s1 s2).val
  have hq0 : Tt / 2 ≤ q := by have := (abs_le.mp eT).1; nlinarith
  have hqpos : 0 < q := by linarith
  have hqlo : (2 : ℚ) ^ (-1000 : ℤ) ≤ q := by
    refine le_trans ?_ (le_trans (by linarith : (1 / 2 ^ 101 : ℚ) ≤ Tt / 2) hq0)
    rw [zpow_neg, zpow_ofNat, ← one_div]
    exact one_div_le_one_div_of_le (by positivity) (pow_le_pow_right₀ (by norm_num) (by norm_num))
  have sq := sqrt_close fNT hqpos hqlo fQ
  -- in ℝ
  have eTr : |(q : ℝ) - (Tt : ℝ)| ≤ 16 * εr * Tt := by have h := (Rat.cast_le (K := ℝ)).mpr eT; push_cast at h ⊢; linarith
  have eSr : |(ns : ℝ) - (Ss : ℝ)| ≤ 16 * εr * Ss := by have h := (Rat.cast_le (K := ℝ)).mpr eS; push_cast at h ⊢; linarith
  have hTr : (1 / 2 ^ 100 : ℝ) ≤ (Tt : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hTT; push_cast at h ⊢; linarith
  have hSr : (1 / 2 ^ 100 : ℝ) ≤ (Ss : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hSS; push_cast at h ⊢; linarith
  have hdr : (1 / 2 ^ 100 : ℝ) ≤ (d.val : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hd; push_cast at h ⊢; linarith
  have hqr : (0 : ℝ) < q := by exact_mod_cast hqpos
  set sT := √(Tt : ℝ)
  set sq' := √(q : ℝ)
  have hsT : 0 < sT := Real.sqrt_pos.mpr (by linarith)
  have hsq : 0 < sq' := Real.sqrt_pos.mpr hqr
  have hsT2 : sT * sT = Tt := Real.mul_self_sqrt (by linarith)
  have hsq2 : sq' * sq' = q := Real.mul_self_sqrt hqr.le
  -- `|√q − √Tt| ≤ 16ε√Tt`
  have dq : |sq' - sT| ≤ 16 * εr * sT := by
    have h1 : |sq' - sT| * (sq' + sT) = |(q : ℝ) - Tt| := by
      rw [← hsq2, ← hsT2, show sq' * sq' - sT * sT = (sq' - sT) * (sq' + sT) by ring, abs_mul,
        abs_of_pos (by linarith : 0 < sq' + sT)]
    have h2 : |sq' - sT| * sT ≤ 16 * εr * sT * sT := by
      have := abs_nonneg (sq' - sT)
      nlinarith
    exact le_of_mul_le_mul_right (by linarith) hsT
  set sf := ((FP.sqrt (dot3 t0 t1 t2 t0 t1 t2)).val : ℝ)
  have dsf : |sf - sT| ≤ 18 * εr * sT := by
    have := abs_sub_le sf sq' sT
    have : sq' ≤ (1 + 16 * εr) * sT := by have := (abs_le.mp dq).2; linarith
    have : εr * sq' ≤ εr * ((1 + 16 * εr) * sT) := by nlinarith
    nlinarith
  -- `p1 = d·sf`
  have fD := (isFinite_of_mul fP).1
  have m1 := mul_rel0 fD fQ fP (u 6 (by norm_num))
  have m1r : |((mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))).val : ℝ) - (d.val : ℝ) * sf| ≤ εr * |(d.val : ℝ) * sf| := by
    have h := (Rat.cast_le (K := ℝ)).mpr m1; push_cast at h ⊢; linarith
  set p1 := ((mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))).val : ℝ)
  set dv := (d.val : ℝ)
  have hdv : 0 < dv := by linarith
  have hsf : 0 ≤ sf := by have := (abs_le.mp dsf).1; nlinarith
  rw [abs_of_nonneg (by positivity : 0 ≤ dv * sf)] at m1r
  have dp1 : |p1 - dv * sT| ≤ 20 * εr * (dv * sT) := by
    have h1 := abs_sub_le p1 (dv * sf) (dv * sT)
    have h2 : |dv * sf - dv * sT| ≤ dv * (18 * εr * sT) := by
      rw [← mul_sub, abs_mul, abs_of_pos hdv]; exact mul_le_mul_of_nonneg_left dsf hdv.le
    have h3 : sf ≤ (1 + 18 * εr) * sT := by have := (abs_le.mp dsf).2; linarith
    have h4 : εr * (dv * sf) ≤ εr * (dv * ((1 + 18 * εr) * sT)) := by
      have := mul_le_mul_of_nonneg_left h3 hdv.le; nlinarith
    nlinarith
  -- `r = p1·ns2`
  have m2 := mul_rel0 fP fNS hfr (u 7 (by norm_num))
  have m2r : |((mul (mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))) (dot3 s0 s1 s2 s0 s1 s2)).val : ℝ) - p1 * ns| ≤
      εr * |p1 * ns| := by
    have h := (Rat.cast_le (K := ℝ)).mpr m2; push_cast at h ⊢; linarith
  set rv := ((mul (mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))) (dot3 s0 s1 s2 s0 s1 s2)).val : ℝ)
  set R0 := dv * sT
  have hR0 : 0 < R0 := mul_pos hdv hsT
  have hp1 : 0 ≤ p1 := by have := (abs_le.mp dp1).1; nlinarith
  have hp1u : p1 ≤ (1 + 20 * εr) * R0 := by have := (abs_le.mp dp1).2; linarith
  have hnsu : (ns : ℝ) ≤ (1 + 16 * εr) * Ss := by have := (abs_le.mp eSr).2; linarith
  have hns0 : (0 : ℝ) ≤ ns := by have := (abs_le.mp eSr).1; nlinarith
  rw [abs_of_nonneg (mul_nonneg hp1 hns0)] at m2r
  have hSs0 : (0 : ℝ) < Ss := by linarith
  have d1 : |p1 * ns - R0 * Ss| ≤ (1 + 20 * εr) * R0 * (16 * εr * Ss) + 20 * εr * R0 * Ss := by
    rw [show p1 * ns - R0 * Ss = p1 * (ns - Ss) + (p1 - R0) * Ss by ring]
    refine le_trans (abs_add_le _ _) (add_le_add ?_ ?_)
    · rw [abs_mul, abs_of_nonneg hp1]
      exact mul_le_mul hp1u eSr (abs_nonneg _) (by positivity)
    · rw [abs_mul, abs_of_pos hSs0]
      exact mul_le_mul_of_nonneg_right dp1 hSs0.le
  have d2 : εr * (p1 * ns) ≤ εr * (((1 + 20 * εr) * R0) * ((1 + 16 * εr) * Ss)) := by
    have := mul_le_mul hp1u hnsu hns0 (by positivity); nlinarith
  have := abs_sub_le rv (p1 * ns) (R0 * Ss)
  have hRS : 0 < R0 * Ss := mul_pos hR0 hSs0
  nlinarith [mul_pos hR0 hSs0]

/-- `η` is far below any `R ≥ 2^-250`. -/
lemma η_le {R : ℝ} (hR : (1 / 2 ^ 250 : ℝ) ≤ R) : (η : ℝ) ≤ R / 10 ^ 20 := by
  have h1 : (η : ℚ) ≤ 2 ^ (-400 : ℤ) := two_zpow_le (by norm_num)
  have h2 : (2 : ℚ) ^ (-400 : ℤ) = 1 / 2 ^ 400 := by
    rw [show (-400 : ℤ) = -((400 : ℕ) : ℤ) from rfl, zpow_neg, zpow_natCast, one_div]
  have h3 : (2 : ℚ) ^ 250 * 10 ^ 20 ≤ 2 ^ 400 := by
    calc (2 : ℚ) ^ 250 * 10 ^ 20 ≤ 2 ^ 250 * 2 ^ 70 := by gcongr; norm_num
      _ = 2 ^ 320 := by rw [← pow_add]
      _ ≤ 2 ^ 400 := pow_le_pow_right₀ (by norm_num) (by norm_num)
  have h4 : (η : ℚ) ≤ 1 / (2 ^ 250 * 10 ^ 20) := by
    rw [h2] at h1; exact le_trans h1 (one_div_le_one_div_of_le (by positivity) h3)
  have h5 : ((η : ℚ) : ℝ) ≤ ((1 / (2 ^ 250 * 10 ^ 20) : ℚ) : ℝ) := Rat.cast_le.mpr h4
  have h6 : ((1 / (2 ^ 250 * 10 ^ 20) : ℚ) : ℝ) = 1 / 2 ^ 250 / 10 ^ 20 := by
    rw [div_div]; push_cast; ring
  rw [h6] at h5
  have : (1 / 2 ^ 250 / 10 ^ 20 : ℝ) ≤ R / 10 ^ 20 := by gcongr
  linarith

set_option maxHeartbeats 4000000 in
/-- **Item 9, `Some(false)`.** If `|e| − err > r·(1.0 + 1e-12)` in f64 at a corner, then the exact
`near` fails at the exact corner `C`. -/
theorem near_f_past {s0 s1 s2 sc t0 t1 t2 tc x y z d : F64} {Sc Tc X Y Z : ℚ}
    (hs0 : s0.isFinite) (hs1 : s1.isFinite) (hs2 : s2.isFinite)
    (ht0 : t0.isFinite) (ht1 : t1.isFinite) (ht2 : t2.isFinite)
    (hsc : Tr sc Sc |Sc| 2) (htc : Tr tc Tc |Tc| 2)
    (hx : Tr x X |X| 1) (hy : Tr y Y |Y| 1) (hz : Tr z Z |Z| 1)
    (hfe : (eF s0 s1 s2 sc t0 t1 t2 tc x y z).isFinite)
    (hfm : (mF s0 s1 s2 sc t0 t1 t2 tc x y z).isFinite)
    (hferr : (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z)).isFinite)
    (hU : ∀ r ∈ [s0.val * x.val, s1.val * y.val, s2.val * z.val, t0.val * x.val, t1.val * y.val,
      t2.val * z.val, s0.val * s0.val, s1.val * s1.val, s2.val * s2.val, t0.val * s0.val, t1.val * s1.val,
      t2.val * s2.val,
      (sub (dot3 t0 t1 t2 x y z) tc).val * (dot3 s0 s1 s2 s0 s1 s2).val,
      (sub (dot3 s0 s1 s2 x y z) sc).val * (dot3 t0 t1 t2 s0 s1 s2).val,
      |t0.val| * |x.val|, |t1.val| * |y.val|, |t2.val| * |z.val|,
      |s0.val| * |x.val|, |s1.val| * |y.val|, |s2.val| * |z.val|,
      |s0.val| * |t0.val|, |s1.val| * |t1.val|, |s2.val| * |t2.val|,
      (magF t0 t1 t2 tc x y z).val * (dot3 s0 s1 s2 s0 s1 s2).val,
      (magF s0 s1 s2 sc x y z).val * (dot3 s0.abs s1.abs s2.abs t0.abs t1.abs t2.abs).val], NZ r)
    (hSS : 1 / 2 ^ 100 ≤ s0.val * s0.val + s1.val * s1.val + s2.val * s2.val)
    (hTT : 1 / 2 ^ 100 ≤ t0.val * t0.val + t1.val * t1.val + t2.val * t2.val)
    (hd : 1 / 2 ^ 100 ≤ d.val)
    (hfr : (rF s0 s1 s2 t0 t1 t2 d).isFinite) (fNT : (dot3 t0 t1 t2 t0 t1 t2).isFinite)
    (hV : ∀ r ∈ [t0.val * t0.val, t1.val * t1.val, t2.val * t2.val, d.val * (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2)).val,
      (mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))).val * (dot3 s0 s1 s2 s0 s1 s2).val], NZ r)
    (hfl : (sub (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z))).isFinite)
    (hfg : (add F1 E12).isFinite) (hfp : (mul (rF s0 s1 s2 t0 t1 t2 d) (add F1 E12)).isFinite)
    (hpast : (mul (rF s0 s1 s2 t0 t1 t2 d) (add F1 E12)).val <
      (sub (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z))).val) :
    ¬ nearAt ((s0.val * X + s1.val * Y + s2.val * Z - Sc : ℚ) : ℝ) ((t0.val * X + t1.val * Y + t2.val * Z - Tc : ℚ) : ℝ)
      ((s0.val * s0.val + s1.val * s1.val + s2.val * s2.val : ℚ) : ℝ)
      ((t0.val * t0.val + t1.val * t1.val + t2.val * t2.val : ℚ) : ℝ)
      ((t0.val * s0.val + t1.val * s1.val + t2.val * s2.val : ℚ) : ℝ) (d.val : ℝ) := by
  have ec := e_close hs0 hs1 hs2 ht0 ht1 ht2 hsc htc hx hy hz hfe hfm hferr hU
  have rc := r_close hs0 hs1 hs2 ht0 ht1 ht2 hSS hTT hd hfr fNT (by
    intro r hr; simp only [List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with h | h | h | h | h | h | h | h <;> subst h
    · exact hU _ (by simp)
    · exact hU _ (by simp)
    · exact hU _ (by simp)
    all_goals exact hV _ (by simp))
  unfold nearAt
  -- names
  set Ss := s0.val * s0.val + s1.val * s1.val + s2.val * s2.val with hSsd
  set Tt := t0.val * t0.val + t1.val * t1.val + t2.val * t2.val with hTtd
  set E := (t0.val * X + t1.val * Y + t2.val * Z - Tc) * Ss -
    (s0.val * X + s1.val * Y + s2.val * Z - Sc) * (t0.val * s0.val + t1.val * s1.val + t2.val * s2.val) with hEd
  have hE : ((t0.val * X + t1.val * Y + t2.val * Z - Tc : ℚ) : ℝ) * (Ss : ℝ) -
      ((s0.val * X + s1.val * Y + s2.val * Z - Sc : ℚ) : ℝ) * ((t0.val * s0.val + t1.val * s1.val + t2.val * s2.val : ℚ) : ℝ) =
      (E : ℝ) := by simp only [hEd]; push_cast; ring
  rw [hE]
  set e := (eF s0 s1 s2 sc t0 t1 t2 tc x y z).val
  set err := (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z)).val
  have ecr : |(e : ℝ) - E| ≤ err + (η : ℝ) := by
    have h := (Rat.cast_le (K := ℝ)).mpr ec; push_cast at h ⊢; linarith
  set r := ((rF s0 s1 s2 t0 t1 t2 d).val : ℝ)
  set sT := √(Tt : ℝ)
  set R := (d.val : ℝ) * sT * (Ss : ℝ)
  have hTr : (1 / 2 ^ 100 : ℝ) ≤ (Tt : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hTT; push_cast at h ⊢; linarith
  have hSr : (1 / 2 ^ 100 : ℝ) ≤ (Ss : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hSS; push_cast at h ⊢; linarith
  have hdr : (1 / 2 ^ 100 : ℝ) ≤ (d.val : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hd; push_cast at h ⊢; linarith
  have hsT : (1 / 2 ^ 50 : ℝ) ≤ sT := Real.le_sqrt_of_sq_le (by norm_num; linarith)
  have hR : (1 / 2 ^ 250 : ℝ) ≤ R := by
    have h1 : (1 / 2 ^ 100 : ℝ) * (1 / 2 ^ 50) ≤ (d.val : ℝ) * sT := mul_le_mul hdr hsT (by norm_num) (by linarith)
    have h2 := mul_le_mul h1 hSr (by norm_num) (by positivity)
    calc (1 / 2 ^ 250 : ℝ) = 1 / 2 ^ 100 * (1 / 2 ^ 50) * (1 / 2 ^ 100) := by norm_num
      _ ≤ R := h2
  have hR2 : (d.val : ℝ) ^ 2 * (Tt : ℝ) * (Ss : ℝ) ^ 2 = R ^ 2 := by
    simp only [R]; rw [show ((d.val : ℝ) * sT * Ss) ^ 2 = (d.val : ℝ) ^ 2 * (sT * sT) * Ss ^ 2 by ring,
      Real.mul_self_sqrt (by linarith)]
  rw [hR2]
  have hηR : (η : ℝ) ≤ R / 10 ^ 20 := η_le hR
  have hη0 : (0 : ℝ) ≤ η := by exact_mod_cast η_pos.le
  have fO : F1.isFinite := by decide
  have fE : E12.isFinite := by decide
  -- the f64 comparison
  have gl := add_rel0 fO fE hfg
  rw [F1_val, E12_val] at gl
  set gp := (add F1 E12).val
  have gpl : (1 + 4951760157141521 / 2 ^ 92) * (1 - ε) ≤ gp := by
    rw [abs_of_nonneg (show (0 : ℚ) ≤ 1 + 4951760157141521 / 2 ^ 92 by norm_num)] at gl
    have := (abs_le.mp gl).1; linarith
  have gpr : ((1 + 4951760157141521 / 2 ^ 92) * (1 - εr) : ℝ) ≤ (gp : ℝ) := by
    have h := (Rat.cast_le (K := ℝ)).mpr gpl; push_cast at h ⊢; linarith
  have fA : (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs.isFinite := by simpa using hfe
  have ll := sub_rel0 fA hferr hfl
  rw [FP.abs_val] at ll
  have llr : |((sub (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z))).val : ℝ) -
      (|(e : ℝ)| - err)| ≤ εr * |(|(e : ℝ)| - err)| := by
    have h := (Rat.cast_le (K := ℝ)).mpr ll; push_cast at h ⊢; linarith
  have pp := mul_rel (isFinite_of_mul hfp).1 hfg hfp
  have ppr : |((mul (rF s0 s1 s2 t0 t1 t2 d) (add F1 E12)).val : ℝ) - r * gp| ≤ εr * |r * gp| + η := by
    have h := (Rat.cast_le (K := ℝ)).mpr pp; push_cast at h ⊢; linarith
  have hp := (Rat.cast_lt (K := ℝ)).mpr hpast
  set L := ((sub (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z))).val : ℝ)
  set P := ((mul (rF s0 s1 s2 t0 t1 t2 d) (add F1 E12)).val : ℝ)
  have hrl : (1 - 40 * εr) * R ≤ r := by have := (abs_le.mp rc).1; linarith
  have hR0 : 0 < R := by linarith
  have hgp0 : (0 : ℝ) ≤ gp := le_trans (by norm_num) gpr
  have hr0 : 0 ≤ r := le_trans (mul_nonneg (by norm_num) hR0.le) hrl
  have hrgp : (1 - 40 * εr) * R * ((1 + 4951760157141521 / 2 ^ 92) * (1 - εr)) ≤ r * gp :=
    mul_le_mul hrl gpr (by norm_num) hr0
  have hrg0 : 0 ≤ r * gp := mul_nonneg hr0 hgp0
  rw [abs_of_nonneg hrg0] at ppr
  have hP : r * gp * (1 - εr) - η ≤ P := by have := (abs_le.mp ppr).1; linarith
  set u := |(e : ℝ)| - err
  have hu : 0 ≤ u := by
    by_contra hneg
    push_neg at hneg
    rw [abs_of_neg hneg] at llr
    have := (abs_le.mp llr).2
    nlinarith
  rw [abs_of_nonneg hu] at llr
  have hL : L ≤ u * (1 + εr) := by have := (abs_le.mp llr).2; linarith
  have hEl : u - η ≤ |(E : ℝ)| := by
    have := abs_sub_abs_le_abs_sub (e : ℝ) E
    have := abs_sub_comm (e : ℝ) E
    simp only [u]; linarith
  have h1 : (1 - 40 * εr) * R * ((1 + 4951760157141521 / 2 ^ 92) * (1 - εr)) * (1 - εr) ≤ r * gp * (1 - εr) :=
    mul_le_mul_of_nonneg_right hrgp (by norm_num)
  have key : R < |(E : ℝ)| := by
    have h2 : (1 - 40 * εr) * R * ((1 + 4951760157141521 / 2 ^ 92) * (1 - εr)) * (1 - εr) =
        ((1 - 40 * εr) * ((1 + 4951760157141521 / 2 ^ 92) * (1 - εr)) * (1 - εr)) * R := by ring
    have h3 : (1 + 2 / 10 ^ 13 : ℝ) * (1 + εr) ≤ (1 - 40 * εr) * ((1 + 4951760157141521 / 2 ^ 92) * (1 - εr)) * (1 - εr) := by
      norm_num
    have h4 : (1 + 2 / 10 ^ 13 : ℝ) * (1 + εr) * R ≤ ((1 - 40 * εr) * ((1 + 4951760157141521 / 2 ^ 92) * (1 - εr)) * (1 - εr)) * R :=
      mul_le_mul_of_nonneg_right h3 hR0.le
    linarith
  intro hle
  have h0 : 0 ≤ R := by linarith
  have := (sq_le_sq₀ (abs_nonneg _) h0).mp (by simpa [sq_abs] using hle)
  linarith

set_option maxHeartbeats 4000000 in
/-- **Item 9, `Some(true)`.** If `|e| + err < r·(1.0 − 1e-12)` in f64 at a corner (the corner keeps
`all`), then the exact `near` holds at the exact corner `C`. -/
theorem near_f_within {s0 s1 s2 sc t0 t1 t2 tc x y z d : F64} {Sc Tc X Y Z : ℚ}
    (hs0 : s0.isFinite) (hs1 : s1.isFinite) (hs2 : s2.isFinite)
    (ht0 : t0.isFinite) (ht1 : t1.isFinite) (ht2 : t2.isFinite)
    (hsc : Tr sc Sc |Sc| 2) (htc : Tr tc Tc |Tc| 2)
    (hx : Tr x X |X| 1) (hy : Tr y Y |Y| 1) (hz : Tr z Z |Z| 1)
    (hfe : (eF s0 s1 s2 sc t0 t1 t2 tc x y z).isFinite)
    (hfm : (mF s0 s1 s2 sc t0 t1 t2 tc x y z).isFinite)
    (hferr : (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z)).isFinite)
    (hU : ∀ r ∈ [s0.val * x.val, s1.val * y.val, s2.val * z.val, t0.val * x.val, t1.val * y.val,
      t2.val * z.val, s0.val * s0.val, s1.val * s1.val, s2.val * s2.val, t0.val * s0.val, t1.val * s1.val,
      t2.val * s2.val,
      (sub (dot3 t0 t1 t2 x y z) tc).val * (dot3 s0 s1 s2 s0 s1 s2).val,
      (sub (dot3 s0 s1 s2 x y z) sc).val * (dot3 t0 t1 t2 s0 s1 s2).val,
      |t0.val| * |x.val|, |t1.val| * |y.val|, |t2.val| * |z.val|,
      |s0.val| * |x.val|, |s1.val| * |y.val|, |s2.val| * |z.val|,
      |s0.val| * |t0.val|, |s1.val| * |t1.val|, |s2.val| * |t2.val|,
      (magF t0 t1 t2 tc x y z).val * (dot3 s0 s1 s2 s0 s1 s2).val,
      (magF s0 s1 s2 sc x y z).val * (dot3 s0.abs s1.abs s2.abs t0.abs t1.abs t2.abs).val], NZ r)
    (hSS : 1 / 2 ^ 100 ≤ s0.val * s0.val + s1.val * s1.val + s2.val * s2.val)
    (hTT : 1 / 2 ^ 100 ≤ t0.val * t0.val + t1.val * t1.val + t2.val * t2.val)
    (hd : 1 / 2 ^ 100 ≤ d.val)
    (hfr : (rF s0 s1 s2 t0 t1 t2 d).isFinite) (fNT : (dot3 t0 t1 t2 t0 t1 t2).isFinite)
    (hV : ∀ r ∈ [t0.val * t0.val, t1.val * t1.val, t2.val * t2.val, d.val * (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2)).val,
      (mul d (FP.sqrt (dot3 t0 t1 t2 t0 t1 t2))).val * (dot3 s0 s1 s2 s0 s1 s2).val], NZ r)
    (hfl : (add (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z))).isFinite)
    (hfg : (sub F1 E12).isFinite) (hfp : (mul (rF s0 s1 s2 t0 t1 t2 d) (sub F1 E12)).isFinite)
    (hin : (add (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z))).val <
      (mul (rF s0 s1 s2 t0 t1 t2 d) (sub F1 E12)).val) :
    nearAt ((s0.val * X + s1.val * Y + s2.val * Z - Sc : ℚ) : ℝ) ((t0.val * X + t1.val * Y + t2.val * Z - Tc : ℚ) : ℝ)
      ((s0.val * s0.val + s1.val * s1.val + s2.val * s2.val : ℚ) : ℝ)
      ((t0.val * t0.val + t1.val * t1.val + t2.val * t2.val : ℚ) : ℝ)
      ((t0.val * s0.val + t1.val * s1.val + t2.val * s2.val : ℚ) : ℝ) (d.val : ℝ) := by
  have ec := e_close hs0 hs1 hs2 ht0 ht1 ht2 hsc htc hx hy hz hfe hfm hferr hU
  have rc := r_close hs0 hs1 hs2 ht0 ht1 ht2 hSS hTT hd hfr fNT (by
    intro r hr; simp only [List.mem_cons, List.not_mem_nil, or_false] at hr
    rcases hr with h | h | h | h | h | h | h | h <;> subst h
    · exact hU _ (by simp)
    · exact hU _ (by simp)
    · exact hU _ (by simp)
    all_goals exact hV _ (by simp))
  unfold nearAt
  -- names
  set Ss := s0.val * s0.val + s1.val * s1.val + s2.val * s2.val with hSsd
  set Tt := t0.val * t0.val + t1.val * t1.val + t2.val * t2.val with hTtd
  set E := (t0.val * X + t1.val * Y + t2.val * Z - Tc) * Ss -
    (s0.val * X + s1.val * Y + s2.val * Z - Sc) * (t0.val * s0.val + t1.val * s1.val + t2.val * s2.val) with hEd
  have hE : ((t0.val * X + t1.val * Y + t2.val * Z - Tc : ℚ) : ℝ) * (Ss : ℝ) -
      ((s0.val * X + s1.val * Y + s2.val * Z - Sc : ℚ) : ℝ) * ((t0.val * s0.val + t1.val * s1.val + t2.val * s2.val : ℚ) : ℝ) =
      (E : ℝ) := by simp only [hEd]; push_cast; ring
  rw [hE]
  set e := (eF s0 s1 s2 sc t0 t1 t2 tc x y z).val
  set err := (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z)).val
  have ecr : |(e : ℝ) - E| ≤ err + (η : ℝ) := by
    have h := (Rat.cast_le (K := ℝ)).mpr ec; push_cast at h ⊢; linarith
  set r := ((rF s0 s1 s2 t0 t1 t2 d).val : ℝ)
  set sT := √(Tt : ℝ)
  set R := (d.val : ℝ) * sT * (Ss : ℝ)
  have hTr : (1 / 2 ^ 100 : ℝ) ≤ (Tt : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hTT; push_cast at h ⊢; linarith
  have hSr : (1 / 2 ^ 100 : ℝ) ≤ (Ss : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hSS; push_cast at h ⊢; linarith
  have hdr : (1 / 2 ^ 100 : ℝ) ≤ (d.val : ℝ) := by have h := (Rat.cast_le (K := ℝ)).mpr hd; push_cast at h ⊢; linarith
  have hsT : (1 / 2 ^ 50 : ℝ) ≤ sT := Real.le_sqrt_of_sq_le (by norm_num; linarith)
  have hR : (1 / 2 ^ 250 : ℝ) ≤ R := by
    have h1 : (1 / 2 ^ 100 : ℝ) * (1 / 2 ^ 50) ≤ (d.val : ℝ) * sT := mul_le_mul hdr hsT (by norm_num) (by linarith)
    have h2 := mul_le_mul h1 hSr (by norm_num) (by positivity)
    calc (1 / 2 ^ 250 : ℝ) = 1 / 2 ^ 100 * (1 / 2 ^ 50) * (1 / 2 ^ 100) := by norm_num
      _ ≤ R := h2
  have hR2 : (d.val : ℝ) ^ 2 * (Tt : ℝ) * (Ss : ℝ) ^ 2 = R ^ 2 := by
    simp only [R]; rw [show ((d.val : ℝ) * sT * Ss) ^ 2 = (d.val : ℝ) ^ 2 * (sT * sT) * Ss ^ 2 by ring,
      Real.mul_self_sqrt (by linarith)]
  rw [hR2]
  have hηR : (η : ℝ) ≤ R / 10 ^ 20 := η_le hR
  have hη0 : (0 : ℝ) ≤ η := by exact_mod_cast η_pos.le
  have fO : F1.isFinite := by decide
  have fE : E12.isFinite := by decide
  have gl := sub_rel0 fO fE hfg
  rw [F1_val, E12_val] at gl
  set gm := (sub F1 E12).val
  have gmu : gm ≤ (1 - 4951760157141521 / 2 ^ 92) * (1 + ε) := by
    rw [abs_of_nonneg (show (0 : ℚ) ≤ 1 - 4951760157141521 / 2 ^ 92 by norm_num)] at gl
    have := (abs_le.mp gl).2; linarith
  have gml : 0 ≤ gm := by
    rw [abs_of_nonneg (show (0 : ℚ) ≤ 1 - 4951760157141521 / 2 ^ 92 by norm_num)] at gl
    have := (abs_le.mp gl).1; nlinarith
  have gmr : (gm : ℝ) ≤ ((1 - 4951760157141521 / 2 ^ 92) * (1 + εr) : ℝ) := by
    have h := (Rat.cast_le (K := ℝ)).mpr gmu; push_cast at h ⊢; linarith
  have gm0 : (0 : ℝ) ≤ gm := by exact_mod_cast gml
  have fA : (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs.isFinite := by simpa using hfe
  have ll := add_rel0 fA hferr hfl
  rw [FP.abs_val] at ll
  have llr : |((add (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z))).val : ℝ) -
      (|(e : ℝ)| + err)| ≤ εr * |(|(e : ℝ)| + err)| := by
    have h := (Rat.cast_le (K := ℝ)).mpr ll; push_cast at h ⊢; linarith
  have pp := mul_rel (isFinite_of_mul hfp).1 hfg hfp
  have ppr : |((mul (rF s0 s1 s2 t0 t1 t2 d) (sub F1 E12)).val : ℝ) - r * gm| ≤ εr * |r * gm| + η := by
    have h := (Rat.cast_le (K := ℝ)).mpr pp; push_cast at h ⊢; linarith
  have hp := (Rat.cast_lt (K := ℝ)).mpr hin
  set U := ((add (eF s0 s1 s2 sc t0 t1 t2 tc x y z).abs (mul E12 (mF s0 s1 s2 sc t0 t1 t2 tc x y z))).val : ℝ)
  set P := ((mul (rF s0 s1 s2 t0 t1 t2 d) (sub F1 E12)).val : ℝ)
  have hru : r ≤ (1 + 40 * εr) * R := by have := (abs_le.mp rc).2; linarith
  have hR0 : 0 < R := by linarith
  have hr0 : 0 ≤ r := by have := (abs_le.mp rc).1; linarith
  have hrgm : r * gm ≤ (1 + 40 * εr) * R * ((1 - 4951760157141521 / 2 ^ 92) * (1 + εr)) :=
    mul_le_mul hru gmr gm0 (mul_nonneg (by norm_num) hR0.le)
  rw [abs_of_nonneg (mul_nonneg hr0 gm0)] at ppr
  have hP : P ≤ r * gm * (1 + εr) + η := by have := (abs_le.mp ppr).2; linarith
  set u := |(e : ℝ)| + err
  have hEu : |(E : ℝ)| ≤ u + η := by
    have := abs_sub_abs_le_abs_sub (E : ℝ) e
    have := abs_sub_comm (e : ℝ) E
    simp only [u]; linarith
  have key : |(E : ℝ)| ≤ R := by
    rcases le_or_gt 0 u with hu | hu
    · rw [abs_of_nonneg hu] at llr
      have hU : u * (1 - εr) ≤ U := by have := (abs_le.mp llr).1; linarith
      have h1 : r * gm * (1 + εr) ≤ (1 + 40 * εr) * R * ((1 - 4951760157141521 / 2 ^ 92) * (1 + εr)) * (1 + εr) :=
        mul_le_mul_of_nonneg_right hrgm (by norm_num)
      have h2 : (1 + 40 * εr) * R * ((1 - 4951760157141521 / 2 ^ 92) * (1 + εr)) * (1 + εr) =
          ((1 + 40 * εr) * ((1 - 4951760157141521 / 2 ^ 92) * (1 + εr)) * (1 + εr)) * R := by ring
      have h3 : (1 + 40 * εr) * ((1 - 4951760157141521 / 2 ^ 92) * (1 + εr)) * (1 + εr) ≤ (1 - 2 / 10 ^ 13 : ℝ) * (1 - εr) := by
        norm_num
      have h4 : ((1 + 40 * εr) * ((1 - 4951760157141521 / 2 ^ 92) * (1 + εr)) * (1 + εr)) * R ≤ (1 - 2 / 10 ^ 13 : ℝ) * (1 - εr) * R :=
        mul_le_mul_of_nonneg_right h3 (by linarith)
      linarith
    · linarith
  have h0 : 0 ≤ R := by linarith
  rw [← sq_abs]
  exact pow_le_pow_left₀ (abs_nonneg _) key 2

end IEEE754.Screen
