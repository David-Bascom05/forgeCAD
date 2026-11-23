/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Round

/-!
# decide-once's plane-side screen `side_err` (item 2)

`side_err_sound`: let `(n, c)` be a `PlaneF`, each coefficient within `2ε` (relative) of the exact
plane's `(N, C)`, and `p` an f64 point within `err` of the exact point `P` per coordinate. If
`|e| > bound`, with `e`, `bound` as `side_err` computes them in f64, then `e` has the sign of the
exact `N·P − C`, which is nonzero. There is no underflow: `Σ|n_i p_i| + |c|` and `Σ|n_i|` are at
least `2^-900`.
-/

namespace IEEE754.Screen

open FP

/-- `GAMMA = 64·f64::EPSILON = 2^-46`. -/
def GAMMA : F64 := .fin false 4503599627370496 (-98) (by decide)
theorem GAMMA_bits : GAMMA.toBits = 0x3d10000000000000 := by decide
theorem GAMMA_val : GAMMA.val = (1 / 2 ^ 46 : ℚ) := by simp only [GAMMA, val, sgn]; norm_num

/-- `n·p − c`, left-associated. -/
def sideE (n0 n1 n2 p0 p1 p2 c : F64) : F64 := sub (add (add (mul n0 p0) (mul n1 p1)) (mul n2 p2)) c
/-- `|n0|·|p0| + |n1|·|p1| + |n2|·|p2| + |c|`. -/
def sideMag (n0 n1 n2 p0 p1 p2 c : F64) : F64 :=
  add (add (add (mul n0.abs p0.abs) (mul n1.abs p1.abs)) (mul n2.abs p2.abs)) c.abs
/-- `GAMMA·mag + (|n0| + |n1| + |n2|)·err`. -/
def sideBound (n0 n1 n2 p0 p1 p2 c err : F64) : F64 :=
  add (mul GAMMA (sideMag n0 n1 n2 p0 p1 p2 c)) (mul (add (add n0.abs n1.abs) n2.abs) err)

/-- The decision, from bounds on reals: if `|e − T|` is within `B` whenever `|e| > B`, the signs agree. -/
lemma sign_of_close {e T B : ℚ} (h : |e - T| < |e|) : (0 < e → 0 < T) ∧ (e < 0 → T < 0) := by
  constructor
  · intro he; rw [abs_of_pos he] at h; have := (abs_lt.mp h).2; linarith
  · intro he; rw [abs_of_neg he] at h; have := (abs_lt.mp h).1; linarith

lemma exact_rel {n N : ℚ} (h : |n - N| ≤ 2 * ε * |N|) : |N| ≤ (1 + 4 * ε) * |n| ∧ |n - N| ≤ 3 * ε * |n| := by
  have a := abs_sub_abs_le_abs_sub N n
  rw [abs_sub_comm] at h
  have hN := abs_nonneg N
  have hn := abs_nonneg n
  have h2 : |N| ≤ 2 * |n| := by linarith
  have h3 : |N| ≤ (1 + 4 * ε) * |n| := by linarith
  refine ⟨h3, ?_⟩
  rw [abs_sub_comm]; linarith

lemma sum3_err {a b c : F64} (ha : a.isFinite) (hb : b.isFinite) (hc : c.isFinite)
    (hf : (add (add a b) c).isFinite) :
    |(add (add a b) c).val - (a.val + b.val + c.val)| ≤ 3 * ε * (|a.val| + |b.val| + |c.val|) + 3 * η := by
  have f1 := (isFinite_of_add hf).1
  have s1 := step_add ha hb f1 (show |a.val - a.val| ≤ 0 by simp) (show |b.val - b.val| ≤ 0 by simp)
  have s1' : |(add a b).val - (a.val + b.val)| ≤ ε * (|a.val| + |b.val|) + η := by linarith
  have s2 := step_add f1 hc hf s1' (show |c.val - c.val| ≤ 0 by simp)
  have hη := η_pos
  have := abs_nonneg a.val; have := abs_nonneg b.val; have := abs_nonneg c.val
  have hab : |a.val + b.val| ≤ |a.val| + |b.val| := abs_add_le _ _
  have : ε * (|a.val + b.val| + |c.val| + (ε * (|a.val| + |b.val|) + η) + 0) ≤
      ε * (|a.val| + |b.val| + |c.val| + (ε * (|a.val| + |b.val|) + η)) := by
    apply mul_le_mul_of_nonneg_left _ (by norm_num); linarith
  linarith

/-- **Item 2.** -/
theorem side_err_sound {n0 n1 n2 p0 p1 p2 c err : F64} {N0 N1 N2 P0 P1 P2 C : ℚ}
    (he : (sideE n0 n1 n2 p0 p1 p2 c).isFinite) (hb : (sideBound n0 n1 n2 p0 p1 p2 c err).isFinite)
    (hn0 : |n0.val - N0| ≤ 2 * ε * |N0|) (hn1 : |n1.val - N1| ≤ 2 * ε * |N1|)
    (hn2 : |n2.val - N2| ≤ 2 * ε * |N2|) (hc : |c.val - C| ≤ 2 * ε * |C|)
    (hp0 : |p0.val - P0| ≤ err.val) (hp1 : |p1.val - P1| ≤ err.val) (hp2 : |p2.val - P2| ≤ err.val)
    (hM : (2 : ℚ) ^ (-900 : ℤ) ≤ |n0.val * p0.val| + |n1.val * p1.val| + |n2.val * p2.val| + |c.val|)
    (hS : (2 : ℚ) ^ (-900 : ℤ) ≤ |n0.val| + |n1.val| + |n2.val|)
    (hdec : (sideBound n0 n1 n2 p0 p1 p2 c err).val < |(sideE n0 n1 n2 p0 p1 p2 c).val|) :
    (0 < (sideE n0 n1 n2 p0 p1 p2 c).val → 0 < N0 * P0 + N1 * P1 + N2 * P2 - C) ∧
      ((sideE n0 n1 n2 p0 p1 p2 c).val < 0 → N0 * P0 + N1 * P1 + N2 * P2 - C < 0) := by
  apply sign_of_close (B := 0)
  -- finiteness
  unfold sideBound at hb hdec
  unfold sideE at he hdec ⊢
  have fGm := (isFinite_of_add hb).1
  have fSe := (isFinite_of_add hb).2
  have fmag := (isFinite_of_mul fGm).2
  have fS := (isFinite_of_mul fSe).1
  have ferr := (isFinite_of_mul fSe).2
  have fe3 := (isFinite_of_sub he).1
  have fc := (isFinite_of_sub he).2
  have fe2 := (isFinite_of_add fe3).1
  have fq2 := (isFinite_of_add fe3).2
  have fq0 := (isFinite_of_add fe2).1
  have fq1 := (isFinite_of_add fe2).2
  obtain ⟨fn0, fp0⟩ := isFinite_of_mul fq0
  obtain ⟨fn1, fp1⟩ := isFinite_of_mul fq1
  obtain ⟨fn2, fp2⟩ := isFinite_of_mul fq2
  have fG : GAMMA.isFinite := by decide
  have fa0 : n0.abs.isFinite := by simpa using fn0
  have fa1 : n1.abs.isFinite := by simpa using fn1
  have fa2 : n2.abs.isFinite := by simpa using fn2
  have fb0 : p0.abs.isFinite := by simpa using fp0
  have fb1 : p1.abs.isFinite := by simpa using fp1
  have fb2 : p2.abs.isFinite := by simpa using fp2
  have fca : c.abs.isFinite := by simpa using fc
  -- the quantities
  set e := sub (add (add (mul n0 p0) (mul n1 p1)) (mul n2 p2)) c
  set M := |n0.val * p0.val| + |n1.val * p1.val| + |n2.val * p2.val| + |c.val|
  set S := |n0.val| + |n1.val| + |n2.val|
  set T := N0 * P0 + N1 * P1 + N2 * P2 - C
  have hη := η_pos
  have hκ : (2 : ℚ) ^ (-175 : ℤ) ≤ 1 / 10 ^ 50 := by norm_num
  have hηe : η = 2 ^ (-900 : ℤ) * 2 ^ (-175 : ℤ) := by rw [← two_zpow_add]; norm_num
  have hηM : η ≤ 1 / 10 ^ 50 * M := by
    rw [hηe]; exact le_trans (mul_le_mul_of_nonneg_right hM (two_zpow_pos _).le)
      (by rw [_root_.mul_comm]; exact mul_le_mul_of_nonneg_right hκ (by positivity))
  have hηS : η ≤ 1 / 10 ^ 50 * S := by
    rw [hηe]; exact le_trans (mul_le_mul_of_nonneg_right hS (two_zpow_pos _).le)
      (by rw [_root_.mul_comm]; exact mul_le_mul_of_nonneg_right hκ (by positivity))
  have herr0 : 0 ≤ err.val := le_trans (abs_nonneg _) hp0
  -- (i) e against ẽ, and ẽ against T
  have E1 := lin3_err fn0 fn1 fn2 fp0 fp1 fp2 fc he
  obtain ⟨a0, b0⟩ := exact_rel hn0
  obtain ⟨a1, b1⟩ := exact_rel hn1
  obtain ⟨a2, b2⟩ := exact_rel hn2
  obtain ⟨ac, bc⟩ := exact_rel hc
  have t0 : |n0.val * p0.val - N0 * P0| ≤ 3 * ε * |n0.val * p0.val| + (1 + 4 * ε) * |n0.val| * err.val := by
    rw [show n0.val * p0.val - N0 * P0 = (n0.val - N0) * p0.val + N0 * (p0.val - P0) by ring]
    refine le_trans (abs_add_le _ _) (add_le_add ?_ ?_)
    · rw [abs_mul, abs_mul, show 3 * ε * (|n0.val| * |p0.val|) = (3 * ε * |n0.val|) * |p0.val| by ring]
      exact mul_le_mul_of_nonneg_right b0 (abs_nonneg _)
    · rw [abs_mul]; exact mul_le_mul a0 hp0 (abs_nonneg _) (by positivity)
  have t1 : |n1.val * p1.val - N1 * P1| ≤ 3 * ε * |n1.val * p1.val| + (1 + 4 * ε) * |n1.val| * err.val := by
    rw [show n1.val * p1.val - N1 * P1 = (n1.val - N1) * p1.val + N1 * (p1.val - P1) by ring]
    refine le_trans (abs_add_le _ _) (add_le_add ?_ ?_)
    · rw [abs_mul, abs_mul, show 3 * ε * (|n1.val| * |p1.val|) = (3 * ε * |n1.val|) * |p1.val| by ring]
      exact mul_le_mul_of_nonneg_right b1 (abs_nonneg _)
    · rw [abs_mul]; exact mul_le_mul a1 hp1 (abs_nonneg _) (by positivity)
  have t2 : |n2.val * p2.val - N2 * P2| ≤ 3 * ε * |n2.val * p2.val| + (1 + 4 * ε) * |n2.val| * err.val := by
    rw [show n2.val * p2.val - N2 * P2 = (n2.val - N2) * p2.val + N2 * (p2.val - P2) by ring]
    refine le_trans (abs_add_le _ _) (add_le_add ?_ ?_)
    · rw [abs_mul, abs_mul, show 3 * ε * (|n2.val| * |p2.val|) = (3 * ε * |n2.val|) * |p2.val| by ring]
      exact mul_le_mul_of_nonneg_right b2 (abs_nonneg _)
    · rw [abs_mul]; exact mul_le_mul a2 hp2 (abs_nonneg _) (by positivity)
  set Q := S * err.val
  have hQ : (1 + 4 * ε) * |n0.val| * err.val + (1 + 4 * ε) * |n1.val| * err.val +
      (1 + 4 * ε) * |n2.val| * err.val = (1 + 4 * ε) * Q := by simp only [Q, S]; ring
  have E2 : |(n0.val * p0.val + n1.val * p1.val + n2.val * p2.val - c.val) - T| ≤ 3 * ε * M + (1 + 4 * ε) * Q := by
    have k : (n0.val * p0.val + n1.val * p1.val + n2.val * p2.val - c.val) - T =
        (n0.val * p0.val - N0 * P0) + (n1.val * p1.val - N1 * P1) + (n2.val * p2.val - N2 * P2) - (c.val - C) := by
      simp only [T]; ring
    rw [k]
    have := abs_sub ((n0.val * p0.val - N0 * P0) + (n1.val * p1.val - N1 * P1) + (n2.val * p2.val - N2 * P2)) (c.val - C)
    have := abs_add_le ((n0.val * p0.val - N0 * P0) + (n1.val * p1.val - N1 * P1)) (n2.val * p2.val - N2 * P2)
    have := abs_add_le (n0.val * p0.val - N0 * P0) (n1.val * p1.val - N1 * P1)
    simp only [M]; linarith
  have Ei : |e.val - T| ≤ 17 * ε * M + (1 + 4 * ε) * Q + 9 * η := by
    have := abs_sub_le e.val (n0.val * p0.val + n1.val * p1.val + n2.val * p2.val - c.val) T
    linarith
  -- (ii) |e| ≤ (1 + 14ε)M + 9η
  have Eii : |e.val| ≤ (1 + 14 * ε) * M + 9 * η := by
    have := abs_sub_abs_le_abs_sub e.val (n0.val * p0.val + n1.val * p1.val + n2.val * p2.val - c.val)
    have h3 : |n0.val * p0.val + n1.val * p1.val + n2.val * p2.val - c.val| ≤ M := by
      have := abs_sub (n0.val * p0.val + n1.val * p1.val + n2.val * p2.val) c.val
      have := abs_add_le (n0.val * p0.val + n1.val * p1.val) (n2.val * p2.val)
      have := abs_add_le (n0.val * p0.val) (n1.val * p1.val)
      simp only [M]; linarith
    linarith
  -- (iii) the computed bound
  have hmag : sideMag n0 n1 n2 p0 p1 p2 c =
      sub (add (add (mul n0.abs p0.abs) (mul n1.abs p1.abs)) (mul n2.abs p2.abs)) c.abs.neg := by
    simp [sideMag, FP.sub, FP.neg_neg]
  have fmag' := fmag; rw [hmag] at fmag'
  have Emag := lin3_err fa0 fa1 fa2 fb0 fb1 fb2 (by simpa using fca) fmag'
  rw [← hmag] at Emag
  simp only [FP.abs_val, FP.val_neg, abs_neg, abs_abs, ← abs_mul, sub_neg_eq_add] at Emag
  have hMn : 0 ≤ M := by positivity
  have mag_lo : (1 - 14 * ε) * M - 9 * η ≤ (sideMag n0 n1 n2 p0 p1 p2 c).val := by
    have := (abs_le.mp Emag).1
    linarith
  have mag_hi : (sideMag n0 n1 n2 p0 p1 p2 c).val ≤ (1 + 14 * ε) * M + 9 * η := by
    have := (abs_le.mp Emag).2
    linarith
  -- `GAMMA ⊗ mag`
  have eGm := mul_rel fG fmag fGm
  rw [GAMMA_val] at eGm
  have hmag0 : 0 ≤ (sideMag n0 n1 n2 p0 p1 p2 c).val := by linarith
  rw [abs_of_nonneg (by positivity : (0 : ℚ) ≤ 1 / 2 ^ 46 * (sideMag n0 n1 n2 p0 p1 p2 c).val)] at eGm
  have gm_lo := (abs_le.mp eGm).1
  have gm_hi := (abs_le.mp eGm).2
  -- `S` in f64, and `S ⊗ err`
  have eS := sum3_err fa0 fa1 fa2 fS
  simp only [FP.abs_val, abs_abs] at eS
  set Sf := (add (add n0.abs n1.abs) n2.abs).val
  have hS0 : 0 ≤ S := by positivity
  have Sf_lo : S - 3 * ε * S - 3 * η ≤ Sf := by have := (abs_le.mp eS).1; linarith
  have Sf_hi : Sf ≤ S + 3 * ε * S + 3 * η := by have := (abs_le.mp eS).2; linarith
  have hSf0 : 0 ≤ Sf := by linarith
  have eSe := mul_rel fS ferr fSe
  rw [abs_of_nonneg (mul_nonneg hSf0 herr0)] at eSe
  have hηerr : η * err.val ≤ 1 / 10 ^ 50 * Q := by
    have := mul_le_mul_of_nonneg_right hηS herr0; simp only [Q]; linarith
  have SfE_lo : (S - 3 * ε * S - 3 * η) * err.val ≤ Sf * err.val := mul_le_mul_of_nonneg_right Sf_lo herr0
  have SfE_hi : Sf * err.val ≤ (S + 3 * ε * S + 3 * η) * err.val := mul_le_mul_of_nonneg_right Sf_hi herr0
  have k1 : (S - 3 * ε * S - 3 * η) * err.val = Q - 3 * ε * Q - 3 * (η * err.val) := by simp only [Q]; ring
  have k2 : (S + 3 * ε * S + 3 * η) * err.val = Q + 3 * ε * Q + 3 * (η * err.val) := by simp only [Q]; ring
  have se_lo := (abs_le.mp eSe).1
  have se_hi := (abs_le.mp eSe).2
  have hQ0 : 0 ≤ Q := mul_nonneg hS0 herr0
  have hηe0 : 0 ≤ η * err.val := mul_nonneg hη.le herr0
  -- the final `+`
  have eB := add_rel fGm fSe hb
  set gm := (mul GAMMA (sideMag n0 n1 n2 p0 p1 p2 c)).val
  set se := (mul (add (add n0.abs n1.abs) n2.abs) err).val
  have gm0 : 0 ≤ gm := by
    simp only [gm]; rw [mul_eq_round fG fmag]
    exact round_val_nonneg binary64 (mul_nonneg (by rw [GAMMA_val]; positivity) hmag0)
  have se0 : 0 ≤ se := by
    simp only [se]; rw [mul_eq_round fS ferr]
    exact round_val_nonneg binary64 (mul_nonneg hSf0 herr0)
  rw [abs_of_nonneg (by linarith : (0 : ℚ) ≤ gm + se)] at eB
  have B_lo := (abs_le.mp eB).1
  have hmagG : 1 / 2 ^ 46 * ((1 - 14 * ε) * M - 9 * η) ≤ 1 / 2 ^ 46 * (sideMag n0 n1 n2 p0 p1 p2 c).val :=
    mul_le_mul_of_nonneg_left mag_lo (by positivity)
  have hεgm : 0 ≤ ε * gm := by positivity
  set SE := Sf * err.val
  have := Ei
  have := Eii
  linarith

/-! ## Item 3: `side_at` -/

/-- The f64 literal `1e-11`. -/
def E11 : F64 := .fin false 6189700196426901 (-89) (by decide)
theorem E11_bits : E11.toBits = 0x3da5fd7fe1796495 := by decide
theorem E11_val : E11.val = (6189700196426901 / 2 ^ 89 : ℚ) := by simp only [E11, val, sgn]; norm_num

lemma ab_close {n N p P : ℚ} (hn : |n - N| ≤ 2 * ε * |N|) (hp : |p - P| ≤ 4 * ε * |P|) :
    |n * p - N * P| ≤ 8 * ε * |n * p| := by
  obtain ⟨a, b⟩ := exact_rel hn
  have hP : |P| ≤ (1 + 9 * ε) * |p| := by
    have := abs_sub_abs_le_abs_sub P p; rw [abs_sub_comm] at hp
    have := abs_nonneg P; have := abs_nonneg p
    have : |P| ≤ 2 * |p| := by linarith
    linarith
  have hP' : |p - P| ≤ 4 * ε * ((1 + 9 * ε) * |p|) := le_trans hp (mul_le_mul_of_nonneg_left hP (by norm_num))
  rw [show n * p - N * P = (n - N) * p + N * (p - P) by ring]
  refine le_trans (abs_add_le _ _) ?_
  rw [abs_mul, abs_mul, abs_mul]
  have h1 : |n - N| * |p| ≤ 3 * ε * |n| * |p| := mul_le_mul_of_nonneg_right b (abs_nonneg _)
  have h2 : |N| * |p - P| ≤ (1 + 4 * ε) * |n| * (4 * ε * ((1 + 9 * ε) * |p|)) :=
    mul_le_mul a hP' (abs_nonneg _) (by positivity)
  have : |n| * |p| ≥ 0 := by positivity
  nlinarith

/-- **Item 3.** `side_at`: the point is `HPoint::to_f64` of the exact one (each coordinate within
`2` ulps, `4ε` relative), and the screen is `|e| > 1e-11·mag`. -/
theorem side_at_sound {n0 n1 n2 p0 p1 p2 c : F64} {N0 N1 N2 P0 P1 P2 C : ℚ}
    (he : (sideE n0 n1 n2 p0 p1 p2 c).isFinite)
    (hm : (mul E11 (sideMag n0 n1 n2 p0 p1 p2 c)).isFinite)
    (hn0 : |n0.val - N0| ≤ 2 * ε * |N0|) (hn1 : |n1.val - N1| ≤ 2 * ε * |N1|)
    (hn2 : |n2.val - N2| ≤ 2 * ε * |N2|) (hc : |c.val - C| ≤ 2 * ε * |C|)
    (hp0 : |p0.val - P0| ≤ 4 * ε * |P0|) (hp1 : |p1.val - P1| ≤ 4 * ε * |P1|) (hp2 : |p2.val - P2| ≤ 4 * ε * |P2|)
    (hM : (2 : ℚ) ^ (-900 : ℤ) ≤ |n0.val * p0.val| + |n1.val * p1.val| + |n2.val * p2.val| + |c.val|)
    (hdec : (mul E11 (sideMag n0 n1 n2 p0 p1 p2 c)).val < |(sideE n0 n1 n2 p0 p1 p2 c).val|) :
    (0 < (sideE n0 n1 n2 p0 p1 p2 c).val → 0 < N0 * P0 + N1 * P1 + N2 * P2 - C) ∧
      ((sideE n0 n1 n2 p0 p1 p2 c).val < 0 → N0 * P0 + N1 * P1 + N2 * P2 - C < 0) := by
  apply sign_of_close (B := 0)
  unfold sideE at he hdec ⊢
  have fmag := (isFinite_of_mul hm).2
  have fe3 := (isFinite_of_sub he).1
  have fc := (isFinite_of_sub he).2
  have fe2 := (isFinite_of_add fe3).1
  have fq2 := (isFinite_of_add fe3).2
  have fq0 := (isFinite_of_add fe2).1
  have fq1 := (isFinite_of_add fe2).2
  obtain ⟨fn0, fp0⟩ := isFinite_of_mul fq0
  obtain ⟨fn1, fp1⟩ := isFinite_of_mul fq1
  obtain ⟨fn2, fp2⟩ := isFinite_of_mul fq2
  have fE : E11.isFinite := by decide
  have fa0 : n0.abs.isFinite := by simpa using fn0
  have fa1 : n1.abs.isFinite := by simpa using fn1
  have fa2 : n2.abs.isFinite := by simpa using fn2
  have fb0 : p0.abs.isFinite := by simpa using fp0
  have fb1 : p1.abs.isFinite := by simpa using fp1
  have fb2 : p2.abs.isFinite := by simpa using fp2
  have fca : c.abs.isFinite := by simpa using fc
  set e := sub (add (add (mul n0 p0) (mul n1 p1)) (mul n2 p2)) c
  set M := |n0.val * p0.val| + |n1.val * p1.val| + |n2.val * p2.val| + |c.val|
  set T := N0 * P0 + N1 * P1 + N2 * P2 - C
  have hη := η_pos
  have hκ : (2 : ℚ) ^ (-175 : ℤ) ≤ 1 / 10 ^ 50 := by norm_num
  have hηe : η = 2 ^ (-900 : ℤ) * 2 ^ (-175 : ℤ) := by rw [← two_zpow_add]; norm_num
  have hηM : η ≤ 1 / 10 ^ 50 * M := by
    rw [hηe]; exact le_trans (mul_le_mul_of_nonneg_right hM (two_zpow_pos _).le)
      (by rw [_root_.mul_comm]; exact mul_le_mul_of_nonneg_right hκ (by positivity))
  have E1 := lin3_err fn0 fn1 fn2 fp0 fp1 fp2 fc he
  have t0 := ab_close hn0 hp0
  have t1 := ab_close hn1 hp1
  have t2 := ab_close hn2 hp2
  obtain ⟨-, bc⟩ := exact_rel hc
  have E2 : |(n0.val * p0.val + n1.val * p1.val + n2.val * p2.val - c.val) - T| ≤ 8 * ε * M := by
    have k : (n0.val * p0.val + n1.val * p1.val + n2.val * p2.val - c.val) - T =
        (n0.val * p0.val - N0 * P0) + (n1.val * p1.val - N1 * P1) + (n2.val * p2.val - N2 * P2) - (c.val - C) := by
      simp only [T]; ring
    rw [k]
    have := abs_sub ((n0.val * p0.val - N0 * P0) + (n1.val * p1.val - N1 * P1) + (n2.val * p2.val - N2 * P2)) (c.val - C)
    have := abs_add_le ((n0.val * p0.val - N0 * P0) + (n1.val * p1.val - N1 * P1)) (n2.val * p2.val - N2 * P2)
    have := abs_add_le (n0.val * p0.val - N0 * P0) (n1.val * p1.val - N1 * P1)
    have := abs_nonneg c.val
    simp only [M]; linarith
  have Ei : |e.val - T| ≤ 22 * ε * M + 9 * η := by
    have := abs_sub_le e.val (n0.val * p0.val + n1.val * p1.val + n2.val * p2.val - c.val) T
    linarith
  have hmag : sideMag n0 n1 n2 p0 p1 p2 c =
      sub (add (add (mul n0.abs p0.abs) (mul n1.abs p1.abs)) (mul n2.abs p2.abs)) c.abs.neg := by
    simp [sideMag, FP.sub, FP.neg_neg]
  have fmag' := fmag; rw [hmag] at fmag'
  have Emag := lin3_err fa0 fa1 fa2 fb0 fb1 fb2 (by simpa using fca) fmag'
  rw [← hmag] at Emag
  simp only [FP.abs_val, FP.val_neg, abs_neg, abs_abs, ← abs_mul, sub_neg_eq_add] at Emag
  have mag_lo : (1 - 14 * ε) * M - 9 * η ≤ (sideMag n0 n1 n2 p0 p1 p2 c).val := by
    have := (abs_le.mp Emag).1; linarith
  have hMn : 0 ≤ M := by positivity
  have hmag0 : 0 ≤ (sideMag n0 n1 n2 p0 p1 p2 c).val := by linarith
  have eG := mul_rel fE fmag hm
  rw [E11_val, abs_of_nonneg (by positivity : (0 : ℚ) ≤ 6189700196426901 / 2 ^ 89 * (sideMag n0 n1 n2 p0 p1 p2 c).val)] at eG
  have g_lo := (abs_le.mp eG).1
  have hmagG : 6189700196426901 / 2 ^ 89 * ((1 - 14 * ε) * M - 9 * η) ≤
      6189700196426901 / 2 ^ 89 * (sideMag n0 n1 n2 p0 p1 p2 c).val :=
    mul_le_mul_of_nonneg_left mag_lo (by positivity)
  linarith

end IEEE754.Screen
