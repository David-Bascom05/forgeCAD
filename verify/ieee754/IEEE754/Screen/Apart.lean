/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Meet

/-!
# decide-once's `plainly_apart` (item 7, two-sided)

`two_sided`: `near(a,b) ∧ near(b,a)` at a point forces `min(|δa − δb|, |δa + δb|) ≤ 2d`, where
`δ` is the signed distance to each plane. So a corner where the f64 distances are plainly further
apart refutes `tol_eq`.
-/

namespace IEEE754.Screen

open Real

/-- `2^-53` in `ℝ`. -/
local notation "εr" => (1 / 9007199254740992 : ℝ)

/-- `Snap.near S T` at one point, exactly: `(T(c)|nS|² − S(c)(nT·nS))² ≤ d²|nT|²|nS|⁴`, with
`sv = S(c)`, `tv = T(c)`, `ss = |nS|²`, `tt = |nT|²`, `ts = nT·nS`. -/
def nearAt (sv tv ss tt ts d : ℝ) : Prop := (tv * ss - sv * ts) ^ 2 ≤ d ^ 2 * tt * ss ^ 2

/-- **Two-sided `near`.** With `ts² ≤ ss·tt` (Cauchy–Schwarz) and both normals nonzero. -/
theorem two_sided {av bv aa bb ab d : ℝ} (ha : 0 < aa) (hb : 0 < bb) (hd : 0 ≤ d)
    (hcs : ab ^ 2 ≤ aa * bb)
    (h1 : nearAt av bv aa bb ab d) (h2 : nearAt bv av bb aa ab d) :
    min |av / √aa - bv / √bb| |av / √aa + bv / √bb| ≤ 2 * d := by
  set la := √aa; set lb := √bb
  have hla : 0 < la := Real.sqrt_pos.mpr ha
  have hlb : 0 < lb := Real.sqrt_pos.mpr hb
  have haa : aa = la ^ 2 := (Real.sq_sqrt ha.le).symm
  have hbb : bb = lb ^ 2 := (Real.sq_sqrt hb.le).symm
  set κ := ab / (la * lb)
  have hκ : |κ| ≤ 1 := by
    rw [abs_div, abs_of_pos (mul_pos hla hlb), div_le_one (mul_pos hla hlb)]
    rw [haa, hbb] at hcs
    exact (sq_le_sq₀ (abs_nonneg _) (by positivity)).mp (by rw [sq_abs, mul_pow]; exact hcs)
  set δa := av / la; set δb := bv / lb
  -- `near(a, b)`: `|δb − δa·κ| ≤ d`
  have n1 : |δb - δa * κ| ≤ d := by
    unfold nearAt at h1
    have e : bv * aa - av * ab = (δb - δa * κ) * (lb * la ^ 2) := by
      simp only [δa, δb, κ]; rw [haa]; field_simp
    rw [e, hbb, haa] at h1
    have : (δb - δa * κ) ^ 2 * (lb * la ^ 2) ^ 2 ≤ d ^ 2 * (lb * la ^ 2) ^ 2 := by nlinarith
    have hp : 0 < (lb * la ^ 2) ^ 2 := by positivity
    have := le_of_mul_le_mul_right this hp
    exact abs_le_of_sq_le_sq' this hd |> fun h => abs_le.mpr h
  have n2 : |δa - δb * κ| ≤ d := by
    unfold nearAt at h2
    have e : av * bb - bv * ab = (δa - δb * κ) * (la * lb ^ 2) := by
      simp only [δa, δb, κ]; rw [hbb]; field_simp
    rw [e, hbb, haa] at h2
    have : (δa - δb * κ) ^ 2 * (la * lb ^ 2) ^ 2 ≤ d ^ 2 * (la * lb ^ 2) ^ 2 := by nlinarith
    have hp : 0 < (la * lb ^ 2) ^ 2 := by positivity
    have := le_of_mul_le_mul_right this hp
    exact abs_le_of_sq_le_sq' this hd |> fun h => abs_le.mpr h
  have s1 : |(δa - δb) * (1 + κ)| ≤ 2 * d := by
    rw [show (δa - δb) * (1 + κ) = (δa - δb * κ) - (δb - δa * κ) by ring]
    exact le_trans (abs_sub _ _) (by linarith)
  have s2 : |(δa + δb) * (1 - κ)| ≤ 2 * d := by
    rw [show (δa + δb) * (1 - κ) = (δa - δb * κ) + (δb - δa * κ) by ring]
    exact le_trans (abs_add_le _ _) (by linarith)
  rcases le_total 0 κ with hk | hk
  · apply min_le_of_left_le
    rw [abs_mul, abs_of_nonneg (by linarith : (0 : ℝ) ≤ 1 + κ)] at s1
    nlinarith [abs_nonneg (δa - δb)]
  · apply min_le_of_right_le
    rw [abs_mul, abs_of_nonneg (by linarith [(abs_le.mp hκ).2] : (0 : ℝ) ≤ 1 - κ)] at s2
    nlinarith [abs_nonneg (δa + δb)]

open FP in
/-- `sqrt` of a positive, not-tiny f64: one rounding of the real square root. -/
lemma sqrt_close {p : F64} (hp : p.isFinite) (hpos : 0 < p.val) (hlo : (2 : ℚ) ^ (-1000 : ℤ) ≤ p.val)
    (hs : (FP.sqrt p).isFinite) :
    |((FP.sqrt p).val : ℝ) - √(p.val : ℝ)| ≤ ε * √(p.val : ℝ) := by
  cases p with
  | fin s m e hc =>
    cases s
    · have hn : (2 : ℝ) ^ binary64.emin ≤ √(((FP.fin false m e hc : F64).val : ℚ) : ℝ) := by
        simp only [binary64_emin]
        rw [Real.le_sqrt (by positivity) (by exact_mod_cast hpos.le)]
        have h2 : ((2 : ℚ) ^ (-1000 : ℤ) : ℝ) ≤ ((FP.fin false m e hc : F64).val : ℝ) := by exact_mod_cast hlo
        push_cast at h2
        calc ((2 : ℝ) ^ (-1022 : ℤ)) ^ 2 = 2 ^ (-2044 : ℤ) := by rw [← zpow_natCast, ← zpow_mul]; norm_num
          _ ≤ 2 ^ (-1000 : ℤ) := zpow_le_zpow_right₀ (by norm_num) (by norm_num)
          _ ≤ _ := h2
      have e := sqrt_rel_err (f := binary64) hc hs hn
      simp only [binary64_prec] at e
      rw [show (2 : ℝ) ^ (-((53 : ℕ) : ℤ)) = 1 / 9007199254740992 by norm_num] at e
      push_cast; exact e
    · exfalso; simp [FP.val, sgn] at hpos; nlinarith [two_zpow_pos (K := ℚ) e, show (0 : ℚ) ≤ m from Nat.cast_nonneg m]
  | zero s => simp [FP.val] at hpos
  | _ => simp_all [FP.isFinite]

set_option maxHeartbeats 2000000 in
open FP in
/-- **A plane's f64 signed distance at a corner.** Normals exact (frame f64s), offset `2ε` off the
exact `Ca`, corner coordinates one rounding off the exact `C`; the plane within `2(Σ|n_i|)·S`
of the origin, `S = 1 + Σ|C_i|`. Then `(n·c − ca)/√(n·n)` in f64 is within `200ε·S` of the exact
signed distance `A(C)/|n|`. -/
theorem dist_sound {n0 n1 n2 ca x y z : F64} {Ca X Y Z : ℚ}
    (hn0 : n0.isFinite) (hn1 : n1.isFinite) (hn2 : n2.isFinite)
    (hca : Tr ca Ca |Ca| 2) (hx : Tr x X |X| 1) (hy : Tr y Y |Y| 1) (hz : Tr z Z |Z| 1)
    (hNN : (2 : ℚ) ^ (-900 : ℤ) ≤ n0.val * n0.val + n1.val * n1.val + n2.val * n2.val)
    (hoff : |Ca| ≤ 2 * (|n0.val| + |n1.val| + |n2.val|) * (1 + |X| + |Y| + |Z|))
    (hf : (div (sub (dot3 n0 n1 n2 x y z) ca) (FP.sqrt (dot3 n0 n1 n2 n0 n1 n2))).isFinite)
    (hfn : (sub (dot3 n0 n1 n2 x y z) ca).isFinite) (hfs : (FP.sqrt (dot3 n0 n1 n2 n0 n1 n2)).isFinite)
    (hfq : (dot3 n0 n1 n2 n0 n1 n2).isFinite)
    (hU : ∀ r ∈ [n0.val * x.val, n1.val * y.val, n2.val * z.val, n0.val * n0.val, n1.val * n1.val,
      n2.val * n2.val, (sub (dot3 n0 n1 n2 x y z) ca).val / (FP.sqrt (dot3 n0 n1 n2 n0 n1 n2)).val], NZ r) :
    |((div (sub (dot3 n0 n1 n2 x y z) ca) (FP.sqrt (dot3 n0 n1 n2 n0 n1 n2))).val : ℝ) -
        ((n0.val * X + n1.val * Y + n2.val * Z - Ca : ℚ) : ℝ) /
          √((n0.val * n0.val + n1.val * n1.val + n2.val * n2.val : ℚ) : ℝ)| ≤
      200 * εr * (1 + |X| + |Y| + |Z|) := by
  have u : ∀ i, (h : i < 7) → NZ ([n0.val * x.val, n1.val * y.val, n2.val * z.val, n0.val * n0.val, n1.val * n1.val,
      n2.val * n2.val, (sub (dot3 n0 n1 n2 x y z) ca).val / (FP.sqrt (dot3 n0 n1 n2 n0 n1 n2)).val].get ⟨i, by simpa using h⟩) :=
    fun i h => hU _ (List.get_mem _ _)
  have e0 := (Tr.exact hn0); have e1 := (Tr.exact hn1); have e2 := (Tr.exact hn2)
  have w0 := e0.weaken (show (0 : ℚ) ≤ 2 by norm_num) (abs_nonneg _)
  have w1 := e1.weaken (show (0 : ℚ) ≤ 2 by norm_num) (abs_nonneg _)
  have w2 := e2.weaken (show (0 : ℚ) ≤ 2 by norm_num) (abs_nonneg _)
  have v0 := e0.weaken (show (0 : ℚ) ≤ 8 by norm_num) (abs_nonneg _)
  have v1 := e1.weaken (show (0 : ℚ) ≤ 8 by norm_num) (abs_nonneg _)
  have v2 := e2.weaken (show (0 : ℚ) ≤ 8 by norm_num) (abs_nonneg _)
  have hx8 := hx.weaken (show (1 : ℚ) ≤ 8 by norm_num) (abs_nonneg _)
  have hy8 := hy.weaken (show (1 : ℚ) ≤ 8 by norm_num) (abs_nonneg _)
  have hz8 := hz.weaken (show (1 : ℚ) ≤ 8 by norm_num) (abs_nonneg _)
  have td := tr_dot3 w0 w1 w2 hx8 hy8 hz8 (isFinite_of_sub hfn).1 (u 0 (by norm_num)) (u 1 (by norm_num)) (u 2 (by norm_num))
  have tn := tr_sub td (hca.weaken (show (2 : ℚ) ≤ 16 by norm_num) (abs_nonneg _)) (by norm_num) (by norm_num) hfn
  have tq := tr_dot3 w0 w1 w2 v0 v1 v2 hfq (u 3 (by norm_num)) (u 4 (by norm_num)) (u 5 (by norm_num))
  -- in ℚ
  set NN := n0.val * n0.val + n1.val * n1.val + n2.val * n2.val
  have hNNq : |n0.val| * |n0.val| + |n1.val| * |n1.val| + |n2.val| * |n2.val| = NN := by
    simp only [NN]; rw [abs_mul_abs_self, abs_mul_abs_self, abs_mul_abs_self]
  rw [hNNq] at tq
  have hNN0 : 0 < NN := lt_of_lt_of_le (two_zpow_pos _) hNN
  set nn := (dot3 n0 n1 n2 n0 n1 n2).val
  have eq := tq.err
  have nn_lo : (1 - 16 * ε) * NN ≤ nn := by have := (abs_le.mp eq).1; linarith
  have nn_hi : nn ≤ (1 + 16 * ε) * NN := by have := (abs_le.mp eq).2; linarith
  have hnn0 : 0 < nn := lt_of_lt_of_le (by positivity) nn_lo
  have hnnlo : (2 : ℚ) ^ (-1000 : ℤ) ≤ nn := by
    have : (2 : ℚ) ^ (-1000 : ℤ) ≤ (1 - 16 * ε) * 2 ^ (-900 : ℤ) := by
      have h1 : (2 : ℚ) ^ (-1000 : ℤ) ≤ 2 ^ (-901 : ℤ) := two_zpow_le (by norm_num)
      have h2 : (2 : ℚ) ^ (-901 : ℤ) = 2 ^ (-900 : ℤ) / 2 := by
        rw [show (-901 : ℤ) = -900 - 1 by norm_num, zpow_sub₀ two_ne_zero, zpow_one]
      have h3 := two_zpow_pos (K := ℚ) (-900)
      rw [h2] at h1; linarith
    have := mul_le_mul_of_nonneg_left hNN (show (0 : ℚ) ≤ 1 - 16 * ε by norm_num)
    linarith
  set num := (sub (dot3 n0 n1 n2 x y z) ca).val
  set A := n0.val * X + n1.val * Y + n2.val * Z - Ca
  set Ma := |n0.val| * |X| + |n1.val| * |Y| + |n2.val| * |Z| + |Ca|
  have enA : |num - A| ≤ 18 * ε * Ma := by have := tn.err; norm_num at this ⊢; linarith
  have hAM : |A| ≤ Ma := tn.mag
  -- in ℝ
  have sq := sqrt_close hfq hnn0 hnnlo hfs
  set la := ((FP.sqrt (dot3 n0 n1 n2 n0 n1 n2)).val : ℝ)
  have hNNr : (0 : ℝ) < (NN : ℝ) := by exact_mod_cast hNN0
  set R := √(NN : ℝ)
  have hR : 0 < R := Real.sqrt_pos.mpr hNNr
  have hnnr0 : (0 : ℝ) ≤ (nn : ℝ) := by exact_mod_cast hnn0.le
  have hRR : R * R = (NN : ℝ) := Real.mul_self_sqrt hNNr.le
  -- `√nn` against `R`
  have hsq : |√(nn : ℝ) - R| ≤ 16 * εr * R := by
    have h1 : |√(nn : ℝ) - R| * (√(nn : ℝ) + R) = |(nn : ℝ) - NN| := by
      rw [← abs_of_pos (show 0 < √(nn : ℝ) + R by positivity), ← abs_mul,
        show (√(nn : ℝ) - R) * (√(nn : ℝ) + R) = √(nn : ℝ) ^ 2 - R ^ 2 by ring, Real.sq_sqrt hnnr0,
        Real.sq_sqrt hNNr.le]
    have hq : |nn - NN| ≤ 16 * ε * NN := by rw [abs_le]; constructor <;> linarith
    have h2 : |(nn : ℝ) - NN| ≤ 16 * εr * NN := by
      have := (Rat.cast_le (K := ℝ)).mpr hq
      rw [Rat.cast_abs] at this; push_cast at this; exact this
    have h3 : |√(nn : ℝ) - R| * R ≤ 16 * εr * (R * R) := by
      rw [hRR]; have := mul_le_mul_of_nonneg_left (show R ≤ √(nn : ℝ) + R by linarith [Real.sqrt_nonneg (nn : ℝ)])
        (abs_nonneg (√(nn : ℝ) - R))
      linarith
    nlinarith
  have hla : |la - R| ≤ 171 / 10 * εr * R := by
    have h1 : √(nn : ℝ) ≤ (1 + 16 * εr) * R := by have := (abs_le.mp hsq).2; linarith
    have := abs_sub_le la (√(nn : ℝ)) R
    have : εr * √(nn : ℝ) ≤ εr * ((1 + 16 * εr) * R) := mul_le_mul_of_nonneg_left h1 (by norm_num)
    norm_num at *; nlinarith
  have hla0 : (1 - 171 / 10 * εr) * R ≤ la := by have := (abs_le.mp hla).1; linarith
  have hlap : 0 < la := lt_of_lt_of_le (by positivity) hla0
  -- the bound on `Ma` by `R·S`
  set S := 1 + |X| + |Y| + |Z|
  have hn_i : ∀ v : ℚ, v * v ≤ NN → |(v : ℝ)| ≤ R := by
    intro v hv
    apply Real.abs_le_sqrt
    have : ((v * v : ℚ) : ℝ) ≤ (NN : ℝ) := by exact_mod_cast hv
    push_cast at this; nlinarith
  have q0 := hn_i n0.val (by simp only [NN]; nlinarith [mul_self_nonneg n1.val, mul_self_nonneg n2.val])
  have q1 := hn_i n1.val (by simp only [NN]; nlinarith [mul_self_nonneg n0.val, mul_self_nonneg n2.val])
  have q2 := hn_i n2.val (by simp only [NN]; nlinarith [mul_self_nonneg n0.val, mul_self_nonneg n1.val])
  have hsum : |(n0.val : ℝ)| + |(n1.val : ℝ)| + |(n2.val : ℝ)| ≤ 2 * R := by
    have h1 : (|(n0.val : ℝ)| + |(n1.val : ℝ)| + |(n2.val : ℝ)|) ^ 2 ≤ (2 * R) ^ 2 := by
      have : (|(n0.val : ℝ)| + |(n1.val : ℝ)| + |(n2.val : ℝ)|) ^ 2 ≤ 3 * (NN : ℝ) := by
        have hN : (NN : ℝ) = (n0.val : ℝ) * n0.val + n1.val * n1.val + n2.val * n2.val := by simp only [NN]; push_cast; ring
        rw [hN]; nlinarith [sq_abs (n0.val : ℝ), sq_abs (n1.val : ℝ), sq_abs (n2.val : ℝ),
          sq_nonneg (|(n0.val : ℝ)| - |(n1.val : ℝ)|), sq_nonneg (|(n1.val : ℝ)| - |(n2.val : ℝ)|),
          sq_nonneg (|(n0.val : ℝ)| - |(n2.val : ℝ)|)]
      nlinarith
    exact abs_le_of_sq_le_sq' h1 (by positivity) |>.2 |> fun h => by
      have := abs_nonneg (n0.val : ℝ); have := abs_nonneg (n1.val : ℝ); have := abs_nonneg (n2.val : ℝ)
      nlinarith [sq_nonneg (|(n0.val : ℝ)| + |(n1.val : ℝ)| + |(n2.val : ℝ)| - 2 * R)]
  have hMa : (Ma : ℝ) ≤ 5 * R * S := by
    have hoffr : |(Ca : ℝ)| ≤ 2 * (|(n0.val : ℝ)| + |(n1.val : ℝ)| + |(n2.val : ℝ)|) * (S : ℝ) := by
      have := (Rat.cast_le (K := ℝ)).mpr hoff; push_cast at this; simp only [S]; push_cast; exact this
    have hS1 : (1 : ℝ) ≤ S := by simp only [S]; push_cast; linarith [abs_nonneg (X : ℝ), abs_nonneg (Y : ℝ), abs_nonneg (Z : ℝ)]
    simp only [Ma, S]; push_cast
    have a0 := mul_le_mul_of_nonneg_right q0 (abs_nonneg (X : ℝ))
    have a1 := mul_le_mul_of_nonneg_right q1 (abs_nonneg (Y : ℝ))
    have a2 := mul_le_mul_of_nonneg_right q2 (abs_nonneg (Z : ℝ))
    have b := mul_le_mul_of_nonneg_right hsum (show (0 : ℝ) ≤ S by linarith)
    simp only [S] at b hoffr; push_cast at b hoffr
    linarith [abs_nonneg (X : ℝ), abs_nonneg (Y : ℝ), abs_nonneg (Z : ℝ)]
  -- the quotient
  have hlaq0 : (FP.sqrt (dot3 n0 n1 n2 n0 n1 n2)).val ≠ 0 := by
    intro h; have : la = 0 := by simp only [la]; rw [h]; simp
    linarith
  have ed := div_rel0 hfn hfs hlaq0 hf (u 6 (by norm_num))
  set da := (div (sub (dot3 n0 n1 n2 x y z) ca) (FP.sqrt (dot3 n0 n1 n2 n0 n1 n2))).val
  have edr : |(da : ℝ) - (num : ℝ) / la| ≤ εr * |(num : ℝ) / la| := by
    have := (Rat.cast_le (K := ℝ)).mpr ed; rw [Rat.cast_abs, Rat.cast_mul, Rat.cast_abs] at this
    push_cast at this; exact this
  have hnAr : |(num : ℝ) - A| ≤ 18 * εr * Ma := by
    have := (Rat.cast_le (K := ℝ)).mpr enA; rw [Rat.cast_abs] at this; push_cast at this; exact this
  have hAr : |(A : ℝ)| ≤ Ma := by have := (Rat.cast_le (K := ℝ)).mpr hAM; rw [Rat.cast_abs] at this; exact this
  have hMa0 : (0 : ℝ) ≤ Ma := le_trans (abs_nonneg _) hAr
  have hS0 : (0 : ℝ) ≤ S := by simp only [S]; push_cast; positivity
  have key : (num : ℝ) / la - A / R = (((num : ℝ) - A) * R + A * (R - la)) / (la * R) := by
    field_simp; ring
  have hq1 : |(num : ℝ) / la - A / R| ≤ 176 * εr * S := by
    rw [key, abs_div, abs_of_pos (mul_pos hlap hR), div_le_iff₀ (mul_pos hlap hR)]
    have b1 : |((num : ℝ) - A) * R + A * (R - la)| ≤ 18 * εr * Ma * R + Ma * (171 / 10 * εr * R) := by
      refine le_trans (abs_add_le _ _) (add_le_add ?_ ?_)
      · rw [abs_mul, abs_of_pos hR]; exact mul_le_mul_of_nonneg_right hnAr hR.le
      · rw [abs_mul, abs_sub_comm]; exact mul_le_mul hAr hla (abs_nonneg _) hMa0
    have b2 : 176 * εr * S * ((1 - 171 / 10 * εr) * R) ≤ 176 * εr * S * la :=
      mul_le_mul_of_nonneg_left hla0 (by positivity)
    have b3 : (351 / 10 * εr) * Ma ≤ (351 / 10 * εr) * (5 * R * S) := mul_le_mul_of_nonneg_left hMa (by norm_num)
    have b4 : 351 / 10 * εr * Ma * R ≤ 351 / 10 * εr * (5 * R * S) * R := mul_le_mul_of_nonneg_right b3 hR.le
    have b5 : 176 * εr * S * la * R ≥ 176 * εr * S * ((1 - 171 / 10 * εr) * R) * R :=
      mul_le_mul_of_nonneg_right b2 hR.le
    linarith [mul_nonneg (mul_nonneg hR.le hS0) hR.le]
  have hAR : |(A : ℝ) / R| ≤ 5 * S := by
    rw [abs_div, abs_of_pos hR, div_le_iff₀ hR]; linarith
  have hq2 : |(num : ℝ) / la| ≤ 5 * S + 176 * εr * S := by
    have := abs_sub_abs_le_abs_sub ((num : ℝ) / la) (A / R); linarith
  have := abs_sub_le (da : ℝ) ((num : ℝ) / la) (A / R)
  have : εr * |(num : ℝ) / la| ≤ εr * (5 * S + 176 * εr * S) := mul_le_mul_of_nonneg_left hq2 (by norm_num)
  have hgoal : (200 : ℝ) * εr * (1 + |(X : ℝ)| + |(Y : ℝ)| + |(Z : ℝ)|) = 200 * εr * S := by simp only [S]; push_cast; ring
  have hA' : ((n0.val * X + n1.val * Y + n2.val * Z - Ca : ℚ) : ℝ) / √((n0.val * n0.val + n1.val * n1.val + n2.val * n2.val : ℚ) : ℝ) = A / R := by
    simp only [A, R, NN]
  rw [hA']
  push_cast at hgoal ⊢
  rw [hgoal]
  linarith

open FP in
/-- The literal `2.0`. -/
def F2 : F64 := .fin false 4503599627370496 (-51) (by decide)
theorem F2_val : F2.val = (2 : ℚ) := by simp only [F2, FP.val, FP.sgn]; norm_num

/-- `(n·c − ca)/√(n·n)` as `plainly_apart` computes it. -/
def distF (n0 n1 n2 ca x y z : F64) : F64 :=
  FP.div (FP.sub (dot3 n0 n1 n2 x y z) ca) (FP.sqrt (dot3 n0 n1 n2 n0 n1 n2))

set_option maxHeartbeats 2000000 in
open FP in
/-- **Item 7, `plainly_apart`** (two-sided). At a corner of the f64 extent (each coordinate one
rounding off the exact corner `C`), with both planes' normals exact, offsets `2ε` off, and each
plane within `2(Σ|n_i|)·S` of the origin: if `min(|da − db|, |da + db|) > 2kl + 1e-9·(1 + |x| + |y|
+ |z|)` in f64, then `near(a, b) ∧ near(b, a)` fails at `C` for any tolerance `d ≤ 2^20` with
`2d ≤ 2.0*k*l` (as f64). -/
theorem apart_sound {a0 a1 a2 ca b0 b1 b2 cb x y z k l : F64} {Ca Cb X Y Z : ℚ} {d : ℝ}
    (ha0 : a0.isFinite) (ha1 : a1.isFinite) (ha2 : a2.isFinite)
    (hb0 : b0.isFinite) (hb1 : b1.isFinite) (hb2 : b2.isFinite)
    (hca : Tr ca Ca |Ca| 2) (hcb : Tr cb Cb |Cb| 2)
    (hx : Tr x X |X| 1) (hy : Tr y Y |Y| 1) (hz : Tr z Z |Z| 1)
    (hNA : (2 : ℚ) ^ (-900 : ℤ) ≤ a0.val * a0.val + a1.val * a1.val + a2.val * a2.val)
    (hNB : (2 : ℚ) ^ (-900 : ℤ) ≤ b0.val * b0.val + b1.val * b1.val + b2.val * b2.val)
    (hoa : |Ca| ≤ 2 * (|a0.val| + |a1.val| + |a2.val|) * (1 + |X| + |Y| + |Z|))
    (hob : |Cb| ≤ 2 * (|b0.val| + |b1.val| + |b2.val|) * (1 + |X| + |Y| + |Z|))
    (hfa : (distF a0 a1 a2 ca x y z).isFinite) (hfb : (distF b0 b1 b2 cb x y z).isFinite)
    (hfna : (sub (dot3 a0 a1 a2 x y z) ca).isFinite) (hfsa : (FP.sqrt (dot3 a0 a1 a2 a0 a1 a2)).isFinite)
    (hfqa : (dot3 a0 a1 a2 a0 a1 a2).isFinite)
    (hfnb : (sub (dot3 b0 b1 b2 x y z) cb).isFinite) (hfsb : (FP.sqrt (dot3 b0 b1 b2 b0 b1 b2)).isFinite)
    (hfqb : (dot3 b0 b1 b2 b0 b1 b2).isFinite)
    (hUa : ∀ r ∈ [a0.val * x.val, a1.val * y.val, a2.val * z.val, a0.val * a0.val, a1.val * a1.val,
      a2.val * a2.val, (sub (dot3 a0 a1 a2 x y z) ca).val / (FP.sqrt (dot3 a0 a1 a2 a0 a1 a2)).val], NZ r)
    (hUb : ∀ r ∈ [b0.val * x.val, b1.val * y.val, b2.val * z.val, b0.val * b0.val, b1.val * b1.val,
      b2.val * b2.val, (sub (dot3 b0 b1 b2 x y z) cb).val / (FP.sqrt (dot3 b0 b1 b2 b0 b1 b2)).val], NZ r)
    (hd0 : 0 ≤ d) (hdB : d ≤ 2 ^ 20) (hd2 : 2 * d ≤ ((mul (mul F2 k) l).val : ℝ))
    (hfd : (sub (distF a0 a1 a2 ca x y z) (distF b0 b1 b2 cb x y z)).isFinite)
    (hfs : (add (distF a0 a1 a2 ca x y z) (distF b0 b1 b2 cb x y z)).isFinite)
    (hft : (add (mul (mul F2 k) l) (mul E9 (add (add (add F1 x.abs) y.abs) z.abs))).isFinite)
    (hdec : (add (mul (mul F2 k) l) (mul E9 (add (add (add F1 x.abs) y.abs) z.abs))).val <
      (fmin (sub (distF a0 a1 a2 ca x y z) (distF b0 b1 b2 cb x y z)).abs
        (add (distF a0 a1 a2 ca x y z) (distF b0 b1 b2 cb x y z)).abs).val) :
    ¬ (nearAt ((a0.val * X + a1.val * Y + a2.val * Z - Ca : ℚ) : ℝ) ((b0.val * X + b1.val * Y + b2.val * Z - Cb : ℚ) : ℝ)
          ((a0.val * a0.val + a1.val * a1.val + a2.val * a2.val : ℚ) : ℝ)
          ((b0.val * b0.val + b1.val * b1.val + b2.val * b2.val : ℚ) : ℝ)
          ((a0.val * b0.val + a1.val * b1.val + a2.val * b2.val : ℚ) : ℝ) d ∧
        nearAt ((b0.val * X + b1.val * Y + b2.val * Z - Cb : ℚ) : ℝ) ((a0.val * X + a1.val * Y + a2.val * Z - Ca : ℚ) : ℝ)
          ((b0.val * b0.val + b1.val * b1.val + b2.val * b2.val : ℚ) : ℝ)
          ((a0.val * a0.val + a1.val * a1.val + a2.val * a2.val : ℚ) : ℝ)
          ((a0.val * b0.val + a1.val * b1.val + a2.val * b2.val : ℚ) : ℝ) d) := by
  rintro ⟨h1, h2⟩
  -- the exact two-sided bound
  have hNA0 : (0 : ℝ) < ((a0.val * a0.val + a1.val * a1.val + a2.val * a2.val : ℚ) : ℝ) := by
    exact_mod_cast lt_of_lt_of_le (two_zpow_pos _) hNA
  have hNB0 : (0 : ℝ) < ((b0.val * b0.val + b1.val * b1.val + b2.val * b2.val : ℚ) : ℝ) := by
    exact_mod_cast lt_of_lt_of_le (two_zpow_pos _) hNB
  have hcs : ((a0.val * b0.val + a1.val * b1.val + a2.val * b2.val : ℚ) : ℝ) ^ 2 ≤
      ((a0.val * a0.val + a1.val * a1.val + a2.val * a2.val : ℚ) : ℝ) *
        ((b0.val * b0.val + b1.val * b1.val + b2.val * b2.val : ℚ) : ℝ) := by
    push_cast
    nlinarith [sq_nonneg ((a0.val : ℝ) * b1.val - a1.val * b0.val), sq_nonneg ((a0.val : ℝ) * b2.val - a2.val * b0.val),
      sq_nonneg ((a1.val : ℝ) * b2.val - a2.val * b1.val)]
  have T := two_sided hNA0 hNB0 hd0 hcs h1 h2
  -- the f64 distances
  have Da := dist_sound ha0 ha1 ha2 hca hx hy hz hNA hoa hfa hfna hfsa hfqa hUa
  have Db := dist_sound hb0 hb1 hb2 hcb hx hy hz hNB hob hfb hfnb hfsb hfqb hUb
  unfold distF at hfd hfs hdec
  set da := (FP.div (FP.sub (dot3 a0 a1 a2 x y z) ca) (FP.sqrt (dot3 a0 a1 a2 a0 a1 a2)))
  set db := (FP.div (FP.sub (dot3 b0 b1 b2 x y z) cb) (FP.sqrt (dot3 b0 b1 b2 b0 b1 b2)))
  set δa := ((a0.val * X + a1.val * Y + a2.val * Z - Ca : ℚ) : ℝ) / √((a0.val * a0.val + a1.val * a1.val + a2.val * a2.val : ℚ) : ℝ)
  set δb := ((b0.val * X + b1.val * Y + b2.val * Z - Cb : ℚ) : ℝ) / √((b0.val * b0.val + b1.val * b1.val + b2.val * b2.val : ℚ) : ℝ)
  set S : ℝ := 1 + |(X : ℝ)| + |(Y : ℝ)| + |(Z : ℝ)|
  have hS1 : 1 ≤ S := by simp only [S]; linarith [abs_nonneg (X : ℝ), abs_nonneg (Y : ℝ), abs_nonneg (Z : ℝ)]
  -- `apart` in f64 against the exact one
  have es := sub_rel0 hfa hfb hfd
  have ea := add_rel0 hfa hfb hfs
  have esr : |((sub da db).val : ℝ) - ((da.val : ℝ) - db.val)| ≤ εr * |(da.val : ℝ) - db.val| := by
    have := (Rat.cast_le (K := ℝ)).mpr es; rw [Rat.cast_abs, Rat.cast_mul, Rat.cast_abs] at this
    push_cast at this; exact this
  have ear : |((add da db).val : ℝ) - ((da.val : ℝ) + db.val)| ≤ εr * |(da.val : ℝ) + db.val| := by
    have := (Rat.cast_le (K := ℝ)).mpr ea; rw [Rat.cast_abs, Rat.cast_mul, Rat.cast_abs] at this
    push_cast at this; exact this
  have hdif : |(da.val : ℝ) - db.val| ≤ |δa - δb| + 400 * εr * S := by
    have := abs_sub_abs_le_abs_sub ((da.val : ℝ) - db.val) (δa - δb)
    have := abs_sub ((da.val : ℝ) - δa) (db.val - δb)
    rw [show (da.val : ℝ) - db.val - (δa - δb) = ((da.val : ℝ) - δa) - (db.val - δb) by ring] at *
    push_cast at Da Db ⊢; simp only [S] at *; linarith
  have hsum : |(da.val : ℝ) + db.val| ≤ |δa + δb| + 400 * εr * S := by
    have := abs_sub_abs_le_abs_sub ((da.val : ℝ) + db.val) (δa + δb)
    have := abs_add_le ((da.val : ℝ) - δa) (db.val - δb)
    rw [show (da.val : ℝ) + db.val - (δa + δb) = ((da.val : ℝ) - δa) + (db.val - δb) by ring] at *
    push_cast at Da Db ⊢; simp only [S] at *; linarith
  have hap : ((fmin (sub da db).abs (add da db).abs).val : ℝ) ≤ (1 + εr) * (2 * d + 400 * εr * S) := by
    rw [fmin_val, FP.abs_val, FP.abs_val]; push_cast
    have h1' : |((sub da db).val : ℝ)| ≤ (1 + εr) * |(da.val : ℝ) - db.val| := by
      have := abs_sub_abs_le_abs_sub ((sub da db).val : ℝ) ((da.val : ℝ) - db.val); linarith
    have h2' : |((add da db).val : ℝ)| ≤ (1 + εr) * |(da.val : ℝ) + db.val| := by
      have := abs_sub_abs_le_abs_sub ((add da db).val : ℝ) ((da.val : ℝ) + db.val); linarith
    rcases min_choice |δa - δb| |δa + δb| with hm | hm
    · rw [hm] at T
      have : |((sub da db).val : ℝ)| ≤ (1 + εr) * (2 * d + 400 * εr * S) := by
        refine le_trans h1' (mul_le_mul_of_nonneg_left (by linarith) (by norm_num))
      exact le_trans (min_le_left _ _) this
    · rw [hm] at T
      have : |((add da db).val : ℝ)| ≤ (1 + εr) * (2 * d + 400 * εr * S) := by
        refine le_trans h2' (mul_le_mul_of_nonneg_left (by linarith) (by norm_num))
      exact le_trans (min_le_right _ _) this
  -- the threshold
  have fO : F1.isFinite := by decide
  have fE : E9.isFinite := by decide
  have fsum := (isFinite_of_mul (isFinite_of_add hft).2).2
  have t0 := tr_add ((Tr.exact fO).weaken (show (0 : ℚ) ≤ 1 by norm_num) (abs_nonneg _)) hx.abs
    (by norm_num) (by norm_num) (isFinite_of_add (isFinite_of_add fsum).1).1
  rw [show (1 : ℚ) + 2 = 3 by norm_num] at t0
  have ts := tr_add t0 (hy.abs.weaken (show (1 : ℚ) ≤ 3 by norm_num) (abs_nonneg _)) (by norm_num) (by norm_num)
    (isFinite_of_add fsum).1
  rw [show (3 : ℚ) + 2 = 5 by norm_num] at ts
  have ts' := tr_add ts (hz.abs.weaken (show (1 : ℚ) ≤ 5 by norm_num) (abs_nonneg _)) (by norm_num) (by norm_num) fsum
  rw [show (5 : ℚ) + 2 = 7 by norm_num, F1_val, abs_one] at ts'
  have sum_lo := ts'.lower
  set sm := (add (add (add F1 x.abs) y.abs) z.abs).val
  have hsm0 : 0 < sm := lt_of_lt_of_le (by positivity) sum_lo
  have et2 := mul_rel0 fE fsum (isFinite_of_add hft).2 (Or.inr (by
    rw [E9_val, abs_of_pos (by positivity)]
    have : (2 : ℚ) ^ (-1022 : ℤ) ≤ 4835703278458517 / 2 ^ 82 * (1 - 7 * ε) := by
      exact le_trans (two_zpow_le (show (-1022 : ℤ) ≤ -100 by norm_num)) (by norm_num)
    have h1 : (1 : ℚ) ≤ 1 + |X| + |Y| + |Z| := by linarith [abs_nonneg X, abs_nonneg Y, abs_nonneg Z]
    nlinarith))
  rw [E9_val, abs_of_pos (by positivity : (0 : ℚ) < 4835703278458517 / 2 ^ 82 * sm)] at et2
  have t2_lo := (abs_le.mp et2).1
  have ft1 := (isFinite_of_add hft).1
  have eth := add_rel0 ft1 (isFinite_of_add hft).2 hft
  have hd2q : (0 : ℝ) ≤ ((mul (mul F2 k) l).val : ℝ) := by linarith
  have ht1 : (0 : ℚ) ≤ (mul (mul F2 k) l).val := by exact_mod_cast hd2q
  have t2_0 : 0 ≤ (mul E9 (add (add (add F1 x.abs) y.abs) z.abs)).val := by nlinarith
  rw [abs_of_nonneg (add_nonneg ht1 t2_0)] at eth
  have th_lo := (abs_le.mp eth).1
  -- combine in ℝ
  have th_r : (((add (mul (mul F2 k) l) (mul E9 (add (add (add F1 x.abs) y.abs) z.abs))).val : ℚ) : ℝ) ≥
      (1 - εr) * (((mul (mul F2 k) l).val : ℝ) + (1 - εr) * (4835703278458517 / 2 ^ 82) * ((1 - 7 * εr) * S)) := by
    have hS' : ((1 + |X| + |Y| + |Z| : ℚ) : ℝ) = S := by simp only [S]; push_cast; ring
    have sum_lo' : ((1 - 7 * ε) * (1 + |X| + |Y| + |Z|) : ℚ) ≤ sm := sum_lo
    have := (Rat.cast_le (K := ℝ)).mpr sum_lo'; push_cast at this
    have t2r := (Rat.cast_le (K := ℝ)).mpr t2_lo; push_cast at t2r
    have thr := (Rat.cast_le (K := ℝ)).mpr th_lo; push_cast at thr
    simp only [S]; nlinarith
  have hdec' := (Rat.cast_lt (K := ℝ)).mpr hdec
  have : (1 - εr) * (2 * d + (1 - εr) * (4835703278458517 / 2 ^ 82) * ((1 - 7 * εr) * S)) ≤
      (1 - εr) * (((mul (mul F2 k) l).val : ℝ) + (1 - εr) * (4835703278458517 / 2 ^ 82) * ((1 - 7 * εr) * S)) := by
    apply mul_le_mul_of_nonneg_left _ (by norm_num); linarith
  nlinarith

end IEEE754.Screen
