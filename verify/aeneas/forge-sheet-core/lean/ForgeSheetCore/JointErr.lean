import Mathlib.Analysis.SpecialFunctions.Pow.Real
import Mathlib.Analysis.SpecialFunctions.Sqrt
import Mathlib.Tactic

/-!
# The joint frame's error (the model)

build.rs's joint frame (place.rs `joint`): `la = √((p0 − p1)²)`, `k = 1 / la`, axis
`right((p1 − p0) · k)`. With every operation relatively accurate (`|r' − r| ≤ u |r|`, the normal
range, as verify/ieee754's `*_rel_err` give), `joint_err`: the computed length is within
`(1 + u)³ − 1` of `|D|` relatively, and each computed axis component within
`(1 + u)³ / (1 − u)³ − 1` of the exact unit vector's, relatively to `|Dᵢ| / |D|`, for
`D = p1 − p0` the exact difference of the doubles. So the frame is the exact one to about `6 u`.
-/

namespace ForgeSheetCore.Joint

variable {u : ℝ}

/-- A relatively accurate result is the exact one times `1 + δ`, `|δ| ≤ u`. -/
theorem rel_of (hu0 : 0 ≤ u) {x x' : ℝ} (h : |x' - x| ≤ u * |x|) :
    ∃ δ, |δ| ≤ u ∧ x' = x * (1 + δ) := by
  rcases eq_or_ne x 0 with rfl | hx
  · refine ⟨0, by simpa using hu0, ?_⟩
    simp at h; simpa using h
  · refine ⟨x' / x - 1, ?_, by field_simp; ring⟩
    rw [show x' / x - 1 = (x' - x) / x by field_simp, abs_div, div_le_iff₀ (abs_pos.mpr hx)]
    exact h

/-- Products of `k` factors `1 + δᵢ`, `|δᵢ| ≤ u < 1`, lie in `[(1 − u)^k, (1 + u)^k]`. -/
theorem fac_bounds {δ : ℝ} (hd : |δ| ≤ u) : 1 - u ≤ 1 + δ ∧ 1 + δ ≤ 1 + u := by
  constructor <;> linarith [abs_le.mp hd]

theorem prod_bounds {a b : ℝ} {k j : ℕ} (hu0 : 0 ≤ u) (hu1 : u < 1) (ha : (1 - u) ^ k ≤ a ∧ a ≤ (1 + u) ^ k)
    (hb : (1 - u) ^ j ≤ b ∧ b ≤ (1 + u) ^ j) :
    (1 - u) ^ (k + j) ≤ a * b ∧ a * b ≤ (1 + u) ^ (k + j) := by
  have h0 : 0 ≤ 1 - u := by linarith
  have pa : 0 ≤ (1 - u) ^ k := pow_nonneg h0 k
  have pb : 0 ≤ (1 - u) ^ j := pow_nonneg h0 j
  rw [pow_add, pow_add]
  exact ⟨mul_le_mul ha.1 hb.1 pb (le_trans pa ha.1),
    mul_le_mul ha.2 hb.2 (le_trans pb hb.1) (pow_nonneg (by linarith) k)⟩

theorem one_bounds {δ : ℝ} (hd : |δ| ≤ u) : (1 - u) ^ 1 ≤ 1 + δ ∧ 1 + δ ≤ (1 + u) ^ 1 := by
  simp only [pow_one]; exact fac_bounds hd

/-- A relative bound on a ratio: `ρ ∈ [lo, hi]` with `lo ≤ 1 ≤ hi` and `1 − lo ≤ hi − 1` gives
`|ρ x − x| ≤ (hi − 1) |x|`. -/
theorem ratio_err {ρ lo hi x : ℝ} (h1 : lo ≤ ρ) (h2 : ρ ≤ hi) (hlo : 1 - lo ≤ hi - 1) :
    |ρ * x - x| ≤ (hi - 1) * |x| := by
  rw [show ρ * x - x = (ρ - 1) * x by ring, abs_mul]
  apply mul_le_mul_of_nonneg_right _ (abs_nonneg x)
  rw [abs_le]; constructor <;> linarith

/-- **The squared length**: `n = (ex ⊗ ex) ⊕ (ey ⊗ ey)` for `ex ≈ −Dx`, `ey ≈ −Dy` lies within
`(1 ∓ u)⁴` of `Dx² + Dy²`. -/
theorem norm_err (hu0 : 0 ≤ u) (hu1 : u < 1) {Dx Dy ex ey sx sy n : ℝ}
    (hex : |ex - -Dx| ≤ u * |-Dx|) (hey : |ey - -Dy| ≤ u * |-Dy|)
    (hsx : |sx - ex * ex| ≤ u * |ex * ex|) (hsy : |sy - ey * ey| ≤ u * |ey * ey|)
    (hn : |n - (sx + sy)| ≤ u * |sx + sy|) :
    (1 - u) ^ 4 * (Dx ^ 2 + Dy ^ 2) ≤ n ∧ n ≤ (1 + u) ^ 4 * (Dx ^ 2 + Dy ^ 2) := by
  have R := @rel_of u hu0
  have PB := @prod_bounds u
  obtain ⟨d1, hd1, rfl⟩ := R hex
  obtain ⟨d2, hd2, rfl⟩ := R hey
  obtain ⟨d3, hd3, rfl⟩ := R hsx
  obtain ⟨d4, hd4, rfl⟩ := R hsy
  obtain ⟨d5, hd5, en⟩ := R hn
  have h0 : 0 ≤ 1 - u := by linarith
  have f1 := PB hu0 hu1 (PB hu0 hu1 (one_bounds hd1) (one_bounds hd1)) (one_bounds hd3)
  have f2 := PB hu0 hu1 (PB hu0 hu1 (one_bounds hd2) (one_bounds hd2)) (one_bounds hd4)
  simp only [Nat.reduceAdd] at f1 f2
  have eN : n = (Dx ^ 2 * ((1 + d1) * (1 + d1) * (1 + d3)) + Dy ^ 2 * ((1 + d2) * (1 + d2) * (1 + d4))) *
      (1 + d5) := by rw [en]; ring
  have hx2 := sq_nonneg Dx
  have hy2 := sq_nonneg Dy
  have hN : 0 ≤ Dx ^ 2 + Dy ^ 2 := by positivity
  have s1 : (1 - u) ^ 3 * (Dx ^ 2 + Dy ^ 2) ≤
      Dx ^ 2 * ((1 + d1) * (1 + d1) * (1 + d3)) + Dy ^ 2 * ((1 + d2) * (1 + d2) * (1 + d4)) := by
    have := add_le_add (mul_le_mul_of_nonneg_left f1.1 hx2) (mul_le_mul_of_nonneg_left f2.1 hy2)
    linarith
  have s2 : Dx ^ 2 * ((1 + d1) * (1 + d1) * (1 + d3)) + Dy ^ 2 * ((1 + d2) * (1 + d2) * (1 + d4)) ≤
      (1 + u) ^ 3 * (Dx ^ 2 + Dy ^ 2) := by
    have := add_le_add (mul_le_mul_of_nonneg_left f1.2 hx2) (mul_le_mul_of_nonneg_left f2.2 hy2)
    linarith
  have g5 := fac_bounds hd5
  constructor
  · rw [eN]
    have := mul_le_mul s1 g5.1 h0 (le_trans (mul_nonneg (pow_nonneg h0 3) hN) s1)
    rw [show (1 - u) ^ 4 * (Dx ^ 2 + Dy ^ 2) = (1 - u) ^ 3 * (Dx ^ 2 + Dy ^ 2) * (1 - u) by ring]
    exact this
  · rw [eN]
    have := mul_le_mul s2 g5.2 (by linarith) (by positivity)
    rw [show (1 + u) ^ 4 * (Dx ^ 2 + Dy ^ 2) = (1 + u) ^ 3 * (Dx ^ 2 + Dy ^ 2) * (1 + u) by ring]
    exact this

/-- **The length**: `la = √n` rounded lies within `(1 ∓ u)³` of `√N`. -/
theorem len_err (hu0 : 0 ≤ u) (hu1 : u < 1) {N n la : ℝ} (hN : 0 ≤ N)
    (hn : (1 - u) ^ 4 * N ≤ n ∧ n ≤ (1 + u) ^ 4 * N) (hla : |la - Real.sqrt n| ≤ u * Real.sqrt n) :
    (1 - u) ^ 3 * Real.sqrt N ≤ la ∧ la ≤ (1 + u) ^ 3 * Real.sqrt N := by
  have h0 : 0 ≤ 1 - u := by linarith
  have h1 : 0 ≤ 1 + u := by linarith
  have sq1 : (1 - u) ^ 2 * Real.sqrt N ≤ Real.sqrt n := by
    have := Real.sqrt_le_sqrt hn.1
    rwa [Real.sqrt_mul (pow_nonneg h0 4), show (1 - u) ^ 4 = ((1 - u) ^ 2) ^ 2 by ring,
      Real.sqrt_sq (pow_nonneg h0 2)] at this
  have sq2 : Real.sqrt n ≤ (1 + u) ^ 2 * Real.sqrt N := by
    have := Real.sqrt_le_sqrt hn.2
    rwa [Real.sqrt_mul (pow_nonneg h1 4), show (1 + u) ^ 4 = ((1 + u) ^ 2) ^ 2 by ring,
      Real.sqrt_sq (pow_nonneg h1 2)] at this
  obtain ⟨d6, hd6, ela⟩ := rel_of hu0 (show |la - Real.sqrt n| ≤ u * |Real.sqrt n| by
    rwa [abs_of_nonneg (Real.sqrt_nonneg _)])
  have g6 := fac_bounds hd6
  constructor
  · have := mul_le_mul sq1 g6.1 h0 (Real.sqrt_nonneg _)
    rw [ela, show (1 - u) ^ 3 * Real.sqrt N = (1 - u) ^ 2 * Real.sqrt N * (1 - u) by ring]
    exact this
  · have := mul_le_mul sq2 g6.2 (by linarith) (by positivity)
    rw [ela, show (1 + u) ^ 3 * Real.sqrt N = (1 + u) ^ 2 * Real.sqrt N * (1 + u) by ring]
    exact this

/-- **An axis component**: `v = (d ⊗ k)` for `d ≈ D`, `k ≈ 1 / la`, `la` within `(1 ∓ u)³` of
`L`, is within `(1 + u)³ / (1 − u)³ − 1` of `D / L` relatively. -/
theorem axis_err (hu0 : 0 ≤ u) (hu1 : u < 1) {L la D d k v : ℝ} (hL : 0 < L)
    (hla : (1 - u) ^ 3 * L ≤ la ∧ la ≤ (1 + u) ^ 3 * L)
    (hk : |k - 1 / la| ≤ u * |1 / la|) (hd : |d - D| ≤ u * |D|) (hv : |v - d * k| ≤ u * |d * k|) :
    |v - D / L| ≤ ((1 + u) ^ 3 / (1 - u) ^ 3 - 1) * |D / L| := by
  have h0 : 0 ≤ 1 - u := by linarith
  have a3 : 0 < (1 - u) ^ 3 := pow_pos (by linarith) 3
  have b3 : 0 < (1 + u) ^ 3 := pow_pos (by linarith) 3
  have hla0 : 0 < la := lt_of_lt_of_le (mul_pos a3 hL) hla.1
  obtain ⟨d7, hd7, ek⟩ := rel_of hu0 hk
  obtain ⟨d8, hd8, ed⟩ := rel_of hu0 hd
  obtain ⟨d10, hd10, ev⟩ := rel_of hu0 hv
  have hlo : 1 - (1 - u) ^ 3 / (1 + u) ^ 3 ≤ (1 + u) ^ 3 / (1 - u) ^ 3 - 1 := by
    have ab : (1 - u) ^ 3 ≤ (1 + u) ^ 3 := pow_le_pow_left₀ h0 (by linarith) 3
    rw [div_sub_one a3.ne', one_sub_div b3.ne', div_le_div_iff₀ b3 a3]
    exact mul_le_mul_of_nonneg_left ab (by linarith)
  have r1 : 1 / (1 + u) ^ 3 ≤ L / la := by
    rw [div_le_div_iff₀ b3 hla0]; linarith [hla.2]
  have r2 : L / la ≤ 1 / (1 - u) ^ 3 := by
    rw [div_le_div_iff₀ hla0 a3]; linarith [hla.1]
  have P := prod_bounds hu0 hu1 (prod_bounds hu0 hu1 (one_bounds hd8) (one_bounds hd7)) (one_bounds hd10)
  simp only [Nat.reduceAdd] at P
  have hP0 : 0 ≤ (1 + d8) * (1 + d7) * (1 + d10) := le_trans a3.le P.1
  have lo : (1 - u) ^ 3 / (1 + u) ^ 3 ≤ (1 + d8) * (1 + d7) * (1 + d10) * (L / la) := by
    have := mul_le_mul P.1 r1 (by positivity) hP0
    rwa [mul_one_div] at this
  have hi : (1 + d8) * (1 + d7) * (1 + d10) * (L / la) ≤ (1 + u) ^ 3 / (1 - u) ^ 3 := by
    have := mul_le_mul P.2 r2 (div_nonneg hL.le hla0.le) b3.le
    rwa [mul_one_div] at this
  have hLne := hL.ne'
  have hlane := hla0.ne'
  have e : v = ((1 + d8) * (1 + d7) * (1 + d10) * (L / la)) * (D / L) := by
    rw [ev, ed, ek]; field_simp
  rw [e]
  exact ratio_err lo hi hlo

/-- **The joint frame's error**: the length within `(1 + u)³ − 1` of `L = |D|` relatively, and
each axis component within `(1 + u)³ / (1 − u)³ − 1` of the exact unit vector's. -/
theorem joint_err (hu0 : 0 ≤ u) (hu : u ≤ 1 / 8) {Dx Dy ex ey sx sy n la k dx dy vx vy : ℝ}
    (hD : Dx ^ 2 + Dy ^ 2 ≠ 0)
    -- `ex = p0x ⊖ p1x` (exact `−Dx`), `sx = ex ⊗ ex`, `n = sx ⊕ sy`
    (hex : |ex - -Dx| ≤ u * |-Dx|) (hey : |ey - -Dy| ≤ u * |-Dy|)
    (hsx : |sx - ex * ex| ≤ u * |ex * ex|) (hsy : |sy - ey * ey| ≤ u * |ey * ey|)
    (hn : |n - (sx + sy)| ≤ u * |sx + sy|)
    -- `la = √n` rounded, `k = 1 / la` rounded
    (hla : |la - Real.sqrt n| ≤ u * Real.sqrt n) (hk : |k - 1 / la| ≤ u * |1 / la|)
    -- `dx = p1x ⊖ p0x` (exact `Dx`), `vx = dx ⊗ k`
    (hdx : |dx - Dx| ≤ u * |Dx|) (hdy : |dy - Dy| ≤ u * |Dy|)
    (hvx : |vx - dx * k| ≤ u * |dx * k|) (hvy : |vy - dy * k| ≤ u * |dy * k|) :
    let L := Real.sqrt (Dx ^ 2 + Dy ^ 2)
    |la - L| ≤ ((1 + u) ^ 3 - 1) * L ∧
    |vx - Dx / L| ≤ ((1 + u) ^ 3 / (1 - u) ^ 3 - 1) * |Dx / L| ∧
    |vy - Dy / L| ≤ ((1 + u) ^ 3 / (1 - u) ^ 3 - 1) * |Dy / L| := by
  intro L
  have hu1 : u < 1 := by linarith
  have hN : 0 < Dx ^ 2 + Dy ^ 2 := lt_of_le_of_ne (by positivity) (Ne.symm hD)
  have hL : 0 < L := Real.sqrt_pos.mpr hN
  have B := len_err hu0 hu1 hN.le (norm_err hu0 hu1 hex hey hsx hsy hn) hla
  refine ⟨?_, axis_err hu0 hu1 hL B hk hdx hvx, axis_err hu0 hu1 hL B hk hdy hvy⟩
  rw [abs_le]; constructor
  · have : 1 - (1 - u) ^ 3 ≤ (1 + u) ^ 3 - 1 := by
      have e6 : (1 + u) ^ 3 + (1 - u) ^ 3 - 2 = 6 * u ^ 2 := by ring
      have := sq_nonneg u; linarith
    have := mul_le_mul_of_nonneg_right this hL.le
    linarith [B.1]
  · linarith [B.2]

end ForgeSheetCore.Joint
