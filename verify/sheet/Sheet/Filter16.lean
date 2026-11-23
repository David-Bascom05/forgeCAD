import Mathlib.Tactic

/-!
# L5: fix-sheet's 16ε filters (kernel/forge-sheet/src/exact.rs `disc`, `line_gap`)

exact.rs decides `sign(|p − c|² − r²)` and `sign(orient(a, b, c)² − l²|b − a|²)` by a float
filter, `|v| > 16ε · (magnitudes)` with `ε = u = 2⁻⁵³`, then the exact expansion. These
theorems prove the filter: whenever it fires, `v`'s sign is the exact value's. The model is the
code's operation order, each operation relatively accurate (`|r' − r| ≤ u |r|`: the normal
range, verify/ieee754's `F64.*_rel_err`, the same debug-checked preconditions as Float.lean's).
-/

namespace Sheet.Filter16

/-- Close to an exact value within less than the value's own magnitude: same sign. -/
theorem sign_of_close {v V : ℝ} (h : |v - V| < |v|) : (0 < v ↔ 0 < V) ∧ (v < 0 ↔ V < 0) := by
  have := abs_lt.mp (lt_of_le_of_lt (le_refl _) h)
  rcases lt_trichotomy v 0 with hv | hv | hv
  · rw [abs_of_neg hv] at h; have := (abs_lt.mp h); constructor <;> constructor <;> intro <;> linarith
  · rw [hv, abs_zero] at h; exact absurd h (not_lt.mpr (abs_nonneg _))
  · rw [abs_of_pos hv] at h; have := (abs_lt.mp h); constructor <;> constructor <;> intro <;> linarith

theorem tri {x y z : ℝ} : |x - z| ≤ |x - y| + |y - z| := abs_sub_le x y z

/-- One rounding on top of an approximation: `|r − X| ≤ u (|X| + e) + e`. -/
theorem rnd_close {u r x X e : ℝ} (hu0 : 0 ≤ u) (hr : |r - x| ≤ u * |x|) (hx : |x - X| ≤ e) :
    |r - X| ≤ u * (|X| + e) + e := by
  have : |x| ≤ |X| + e := by have := abs_sub_abs_le_abs_sub x X; linarith
  have := mul_le_mul_of_nonneg_left this hu0
  linarith [tri (x := r) (y := x) (z := X)]

theorem add_close {x y X Y ex ey : ℝ} (hx : |x - X| ≤ ex) (hy : |y - Y| ≤ ey) :
    |(x + y) - (X + Y)| ≤ ex + ey := by
  rw [show (x + y) - (X + Y) = (x - X) + (y - Y) by ring]
  exact le_trans (abs_add_le _ _) (add_le_add hx hy)

theorem sub_close {x y X Y ex ey : ℝ} (hx : |x - X| ≤ ex) (hy : |y - Y| ≤ ey) :
    |(x - y) - (X - Y)| ≤ ex + ey := by
  rw [show (x - y) - (X - Y) = (x - X) - (y - Y) by ring]
  exact le_trans (abs_sub _ _) (add_le_add hx hy)

/-- A rounded square of a rounded value: within `3.01 u` of the exact square. -/
theorem sq_close {u D d a : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 1024)
    (hd : |d - D| ≤ u * |D|) (ha : |a - d * d| ≤ u * |d * d|) :
    |a - D * D| ≤ 301 / 100 * u * (D * D) := by
  have hD := mul_self_nonneg D
  have hdd : |d * d - D * D| ≤ (2 * u + u * u) * (D * D) := by
    rw [show d * d - D * D = (d - D) * (d - D) + 2 * D * (d - D) by ring]
    calc _ ≤ |(d - D) * (d - D)| + |2 * D * (d - D)| := abs_add_le _ _
      _ = |d - D| * |d - D| + 2 * |D| * |d - D| := by rw [abs_mul, abs_mul, abs_mul]; norm_num
      _ ≤ (u * |D|) * (u * |D|) + 2 * |D| * (u * |D|) := by gcongr
      _ = (2 * u + u * u) * (D * D) := by rw [← abs_mul_abs_self D]; ring
  have h := rnd_close hu0 ha hdd
  rw [abs_of_nonneg hD] at h
  have : u * (D * D + (2 * u + u * u) * (D * D)) + (2 * u + u * u) * (D * D) ≤
      301 / 100 * u * (D * D) := by
    have a1 : u * u ≤ u / 1024 := by nlinarith
    have a2 : u * (u * u) ≤ u / 1024 := by nlinarith
    have : u * (D * D + (2 * u + u * u) * (D * D)) + (2 * u + u * u) * (D * D) =
        (3 * u + 3 * (u * u) + u * (u * u)) * (D * D) := by ring
    rw [this]; apply mul_le_mul_of_nonneg_right _ hD; linarith
  linarith

/-- The disc's error budget, numerically (`W = S + Q`). -/
theorem budget {u S Q : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 1024) (hS : 0 ≤ S) (hQ : 0 ≤ Q) :
    let es := u * (S + 301 / 100 * u * S) + 301 / 100 * u * S
    let ed := es + u * Q
    u * ((S + Q) + ed) + ed ≤ 7 * u * (S + Q) ∧
      7 * u * (S + Q) ≤ 16 * u * (1 - u) * ((S + Q) - (u * ((S + Q) + ed) + ed)) := by
  intro es ed
  have hW := add_nonneg hS hQ
  have a1 : u * u ≤ u / 1024 := by nlinarith
  have e1 : es ≤ 403 / 100 * u * S := by
    have : u * (301 / 100 * u * S) ≤ 301 / 100 * (u / 1024) * S := by
      have := mul_le_mul_of_nonneg_right a1 hS; nlinarith
    simp only [es]; nlinarith
  have e2 : ed ≤ 403 / 100 * u * (S + Q) := by simp only [ed]; nlinarith
  have e3 : u * ed ≤ 403 / 100 * (u / 1024) * (S + Q) := by
    have := mul_le_mul_of_nonneg_left e2 hu0
    have : u * (403 / 100 * u * (S + Q)) = 403 / 100 * (u * u) * (S + Q) := by ring
    have := mul_le_mul_of_nonneg_right a1 hW
    nlinarith
  have E : u * ((S + Q) + ed) + ed ≤ (u + 403 / 100 * (u / 1024) + 403 / 100 * u) * (S + Q) := by
    nlinarith
  have E7 : (u + 403 / 100 * (u / 1024) + 403 / 100 * u) * (S + Q) ≤ 7 * u * (S + Q) := by
    apply mul_le_mul_of_nonneg_right _ hW; linarith
  refine ⟨le_trans E E7, ?_⟩
  -- 16 u (1 − u)(W − 7uW) ≥ 7 u W
  have h7 : (S + Q) - 7 * u * (S + Q) ≤ (S + Q) - (u * ((S + Q) + ed) + ed) := by linarith
  have g : 7 * u * (S + Q) ≤ 16 * u * (1 - u) * ((S + Q) - 7 * u * (S + Q)) := by
    have : 16 * u * (1 - u) * ((S + Q) - 7 * u * (S + Q)) = u * (16 * (1 - u) * (1 - 7 * u)) * (S + Q) := by
      ring
    rw [this]
    apply mul_le_mul_of_nonneg_right _ hW
    have : 7 ≤ 16 * (1 - u) * (1 - 7 * u) := by nlinarith
    nlinarith
  have : 0 ≤ 16 * u * (1 - u) := by have : 0 ≤ 1 - u := by linarith
                                    positivity
  nlinarith [mul_le_mul_of_nonneg_left h7 this]

/-- **exact.rs `disc`'s filter**: `dx = px ⊖ cx`, `dy = py ⊖ cy`, `v = (dx⊗dx ⊕ dy⊗dy) ⊖ r⊗r`,
`T = 2⁻⁴⁹ ⊗ ((dx⊗dx ⊕ dy⊗dy) ⊕ r⊗r)` (`2⁻⁴⁹ = 16 u`): `|v| > T` gives `v`'s sign to
`|p − c|² − r²`. -/
theorem disc_filter_sound {u DX DY r dx dy a b s q v t T : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 1024)
    (hdx : |dx - DX| ≤ u * |DX|) (hdy : |dy - DY| ≤ u * |DY|)
    (ha : |a - dx * dx| ≤ u * |dx * dx|) (hb : |b - dy * dy| ≤ u * |dy * dy|)
    (hs : |s - (a + b)| ≤ u * |a + b|) (hq : |q - r * r| ≤ u * |r * r|)
    (hv : |v - (s - q)| ≤ u * |s - q|) (ht : |t - (s + q)| ≤ u * |s + q|)
    (hT : |T - 16 * u * t| ≤ u * |16 * u * t|) (hgt : T < |v|) :
    (0 < v ↔ 0 < DX * DX + DY * DY - r * r) ∧ (v < 0 ↔ DX * DX + DY * DY - r * r < 0) := by
  apply sign_of_close
  have hX := mul_self_nonneg DX
  have hY := mul_self_nonneg DY
  have hR := mul_self_nonneg r
  have ea := sq_close hu0 hu hdx ha
  have eb := sq_close hu0 hu hdy hb
  have eab := add_close ea eb
  rw [show 301 / 100 * u * (DX * DX) + 301 / 100 * u * (DY * DY) =
    301 / 100 * u * (DX * DX + DY * DY) by ring] at eab
  have es := rnd_close hu0 hs eab
  rw [abs_of_nonneg (add_nonneg hX hY)] at es
  rw [abs_of_nonneg hR] at hq
  have ed := sub_close es hq
  have ev := rnd_close hu0 hv ed
  have et := rnd_close hu0 ht (add_close es hq)
  obtain ⟨B1, B2⟩ := budget hu0 hu (add_nonneg hX hY) hR
  have hV : |DX * DX + DY * DY - r * r| ≤ DX * DX + DY * DY + r * r := by
    rw [abs_le]; constructor <;> linarith
  rw [abs_of_nonneg (by positivity : 0 ≤ DX * DX + DY * DY + r * r)] at et
  -- |v − V| ≤ E ≤ 7 u W
  have evE : |v - (DX * DX + DY * DY - r * r)| ≤ 7 * u * (DX * DX + DY * DY + r * r) := by
    refine le_trans ev (le_trans ?_ B1)
    have := mul_le_mul_of_nonneg_left (add_le_add_right hV
      (u * (DX * DX + DY * DY + 301 / 100 * u * (DX * DX + DY * DY)) +
        301 / 100 * u * (DX * DX + DY * DY) + u * (r * r))) hu0
    linarith
  -- T ≥ 16 u (1 − u) (W − E) ≥ 7 u W
  have tl := (abs_le.mp et).1
  have tW : (DX * DX + DY * DY + r * r) - (u * ((DX * DX + DY * DY + r * r) +
      (u * (DX * DX + DY * DY + 301 / 100 * u * (DX * DX + DY * DY)) +
        301 / 100 * u * (DX * DX + DY * DY) + u * (r * r))) +
      (u * (DX * DX + DY * DY + 301 / 100 * u * (DX * DX + DY * DY)) +
        301 / 100 * u * (DX * DX + DY * DY) + u * (r * r))) ≤ t := by linarith
  have t0 : 0 ≤ t := by
    have := B2; have : 0 ≤ 7 * u * (DX * DX + DY * DY + r * r) := by positivity
    nlinarith
  rw [abs_of_nonneg (by positivity : 0 ≤ 16 * u * t)] at hT
  have T1 : 16 * u * (1 - u) * t ≤ T := by have := (abs_le.mp hT).1; linarith
  have : 16 * u * (1 - u) * ((DX * DX + DY * DY + r * r) - (u * ((DX * DX + DY * DY + r * r) +
      (u * (DX * DX + DY * DY + 301 / 100 * u * (DX * DX + DY * DY)) +
        301 / 100 * u * (DX * DX + DY * DY) + u * (r * r))) +
      (u * (DX * DX + DY * DY + 301 / 100 * u * (DX * DX + DY * DY)) +
        301 / 100 * u * (DX * DX + DY * DY) + u * (r * r)))) ≤ 16 * u * (1 - u) * t :=
    mul_le_mul_of_nonneg_left tW (by have : 0 ≤ 1 - u := by linarith
                                     positivity)
  linarith

/-! ## `line_gap` -/

/-- A rounded product of two approximations, relatively: `k₁ + k₂ + 1.1`. -/
theorem rel_mul {u X Y x y p k1 k2 : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 1024)
    (hk1 : 0 ≤ k1) (hk2 : 0 ≤ k2) (hk1' : k1 ≤ 8) (hk2' : k2 ≤ 8)
    (h1 : |x - X| ≤ k1 * u * |X|) (h2 : |y - Y| ≤ k2 * u * |Y|) (hp : |p - x * y| ≤ u * |x * y|) :
    |p - X * Y| ≤ (k1 + k2 + 11 / 10) * u * |X * Y| := by
  have hX := abs_nonneg X
  have hY := abs_nonneg Y
  have hy : |y| ≤ (1 + k2 * u) * |Y| := by
    have := abs_sub_abs_le_abs_sub y Y; nlinarith
  have exy : |x * y - X * Y| ≤ (k1 * (1 + k2 * u) + k2) * u * |X * Y| := by
    rw [show x * y - X * Y = (x - X) * y + X * (y - Y) by ring]
    calc _ ≤ |(x - X) * y| + |X * (y - Y)| := abs_add_le _ _
      _ = |x - X| * |y| + |X| * |y - Y| := by rw [abs_mul, abs_mul]
      _ ≤ (k1 * u * |X|) * ((1 + k2 * u) * |Y|) + |X| * (k2 * u * |Y|) := by gcongr
      _ = _ := by rw [abs_mul]; ring
  have h := rnd_close hu0 hp exy
  have hXY := abs_nonneg (X * Y)
  have ku : k1 * k2 * u ≤ 64 / 1024 := by
    have : k1 * k2 ≤ 64 := by nlinarith
    nlinarith
  have ku2 : (k1 * (1 + k2 * u) + k2) * u ≤ (17 : ℝ) / 1024 := by nlinarith
  have : u * (|X * Y| + (k1 * (1 + k2 * u) + k2) * u * |X * Y|) + (k1 * (1 + k2 * u) + k2) * u * |X * Y|
      ≤ (k1 + k2 + 11 / 10) * u * |X * Y| := by
    have e : u * (|X * Y| + (k1 * (1 + k2 * u) + k2) * u * |X * Y|) + (k1 * (1 + k2 * u) + k2) * u * |X * Y|
        = u * |X * Y| * (1 + (k1 * (1 + k2 * u) + k2) * u + k1 + k1 * k2 * u + k2) := by ring
    rw [e, show (k1 + k2 + 11 / 10) * u * |X * Y| = u * |X * Y| * (k1 + k2 + 11 / 10) by ring]
    apply mul_le_mul_of_nonneg_left _ (by positivity)
    linarith
  linarith

/-- A rounded sum of two nonnegative approximations: `k + 1.01`. -/
theorem rel_add {u X Y x y s k : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 1024) (hk : 0 ≤ k) (hk' : k ≤ 8)
    (hX : 0 ≤ X) (hY : 0 ≤ Y) (h1 : |x - X| ≤ k * u * X) (h2 : |y - Y| ≤ k * u * Y)
    (hs : |s - (x + y)| ≤ u * |x + y|) : |s - (X + Y)| ≤ (k + 101 / 100) * u * (X + Y) := by
  have e := add_close h1 h2
  have h := rnd_close hu0 hs e
  rw [abs_of_nonneg (add_nonneg hX hY)] at h
  have : k * u ≤ 8 / 1024 := by nlinarith
  have hW := add_nonneg hX hY
  have : u * ((X + Y) + (k * u * X + k * u * Y)) + (k * u * X + k * u * Y) ≤
      (k + 101 / 100) * u * (X + Y) := by
    have e2 : u * ((X + Y) + (k * u * X + k * u * Y)) + (k * u * X + k * u * Y) =
        u * (X + Y) * (1 + k * u + k) := by ring
    rw [e2, show (k + 101 / 100) * u * (X + Y) = u * (X + Y) * (k + 101 / 100) by ring]
    apply mul_le_mul_of_nonneg_left _ (by positivity); linarith
  linarith

theorem poly1 {u : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 1024) :
    1 - 10 * u ≤ (1 - u) ^ 3 * (1 - 31 / 10 * u) ^ 2 := by
  nlinarith [sq_nonneg u, mul_nonneg hu0 (sq_nonneg u), mul_nonneg (sq_nonneg u) (sq_nonneg u),
    mul_nonneg hu0 (mul_nonneg (sq_nonneg u) (sq_nonneg u))]

theorem poly2 {u : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 1024) :
    107 / 10 ≤ 16 * (1 - u) ^ 2 * (1 - 10 * u) := by
  nlinarith [sq_nonneg u, mul_nonneg hu0 (sq_nonneg u)]

set_option maxHeartbeats 2000000 in
/-- **exact.rs `line_gap`'s filter**: `ux = bx ⊖ ax`, `uy`, `vx = cx ⊖ ax`, `vy`;
`p₁ = ux⊗vy`, `p₂ = uy⊗vx`, `o = p₁ ⊖ p₂`, `n = ux⊗ux ⊕ uy⊗uy`, `ll = l⊗l`, `w = ll⊗n`,
`v = o⊗o ⊖ w`, `m = |p₁| ⊕ |p₂|`, `T = 2⁻⁴⁹ ⊗ (m⊗m ⊕ w)`: `|v| > T` gives `v`'s sign to
`orient(a, b, c)² − l² |b − a|²` (`O = UX VY − UY VX`, `N = UX² + UY²`). -/
theorem line_gap_filter_sound {u UX UY VX VY l ux uy vx vy p1 p2 o a b n ll w oo v m mm t T : ℝ}
    (hu0 : 0 ≤ u) (hu : u ≤ 1 / 1024)
    (hux : |ux - UX| ≤ u * |UX|) (huy : |uy - UY| ≤ u * |UY|)
    (hvx : |vx - VX| ≤ u * |VX|) (hvy : |vy - VY| ≤ u * |VY|)
    (hp1 : |p1 - ux * vy| ≤ u * |ux * vy|) (hp2 : |p2 - uy * vx| ≤ u * |uy * vx|)
    (ho : |o - (p1 - p2)| ≤ u * |p1 - p2|)
    (ha : |a - ux * ux| ≤ u * |ux * ux|) (hb : |b - uy * uy| ≤ u * |uy * uy|)
    (hn : |n - (a + b)| ≤ u * |a + b|) (hll : |ll - l * l| ≤ u * |l * l|)
    (hw : |w - ll * n| ≤ u * |ll * n|) (hoo : |oo - o * o| ≤ u * |o * o|)
    (hv : |v - (oo - w)| ≤ u * |oo - w|) (hm : |m - (|p1| + |p2|)| ≤ u * |(|p1| + |p2|)|)
    (hmm : |mm - m * m| ≤ u * |m * m|) (ht : |t - (mm + w)| ≤ u * |mm + w|)
    (hT : |T - 16 * u * t| ≤ u * |16 * u * t|) (hgt : T < |v|) :
    let V := (UX * VY - UY * VX) ^ 2 - l * l * (UX * UX + UY * UY)
    (0 < v ↔ 0 < V) ∧ (v < 0 ↔ V < 0) := by
  intro V
  apply sign_of_close
  have one : |ux - UX| ≤ 1 * u * |UX| := by linarith
  have one' : |uy - UY| ≤ 1 * u * |UY| := by linarith
  have e1 := rel_mul hu0 hu (by norm_num) (by norm_num) (by norm_num) (by norm_num) one
    (by linarith : |vy - VY| ≤ 1 * u * |VY|) hp1
  have e2 := rel_mul hu0 hu (by norm_num) (by norm_num) (by norm_num) (by norm_num) one'
    (by linarith : |vx - VX| ≤ 1 * u * |VX|) hp2
  rw [show (1 : ℝ) + 1 + 11 / 10 = 31 / 10 by norm_num] at e1 e2
  have ro := rnd_close hu0 ho (sub_close e1 e2)
  set M := |UX * VY| + |UY * VX| with hM
  set O := UX * VY - UY * VX
  set N := UX * UX + UY * UY
  have hM0 : 0 ≤ M := by positivity
  have hOM : |O| ≤ M := abs_sub _ _
  -- o within 4.2 u M
  have eo : |o - O| ≤ 42 / 10 * u * M := by
    have := ro
    have k : u * (|O| + (31 / 10 * u * |UX * VY| + 31 / 10 * u * |UY * VX|)) +
        (31 / 10 * u * |UX * VY| + 31 / 10 * u * |UY * VX|) ≤ 42 / 10 * u * M := by
      have : 31 / 10 * u * |UX * VY| + 31 / 10 * u * |UY * VX| = 31 / 10 * u * M := by ring
      rw [this]
      have := mul_le_mul_of_nonneg_left hOM hu0
      have : u * (31 / 10 * u * M) ≤ 31 / 10 * (1 / 1024) * u * M := by
        have := mul_le_mul_of_nonneg_right hu (by positivity : (0 : ℝ) ≤ 31 / 10 * u * M); linarith
      have := mul_nonneg hu0 hM0
      linarith
    linarith
  -- o² within 9.6 u M²
  have eoo : |oo - O ^ 2| ≤ 96 / 10 * u * M ^ 2 := by
    have esq : |o * o - O ^ 2| ≤ 42 / 10 * u * M * (2 * M + 42 / 10 * u * M) := by
      rw [show o * o - O ^ 2 = (o - O) * (o + O) by ring, abs_mul]
      have : |o + O| ≤ 2 * M + 42 / 10 * u * M := by
        rw [show o + O = (o - O) + 2 * O by ring]
        calc _ ≤ |o - O| + |2 * O| := abs_add_le _ _
          _ = |o - O| + 2 * |O| := by rw [abs_mul]; norm_num
          _ ≤ _ := by linarith
      exact mul_le_mul eo this (abs_nonneg _) (by positivity)
    have := rnd_close hu0 hoo esq
    have hO2 : |O ^ 2| ≤ M ^ 2 := by
      rw [abs_pow]; exact pow_le_pow_left₀ (abs_nonneg _) hOM 2
    have k : u * (|O ^ 2| + 42 / 10 * u * M * (2 * M + 42 / 10 * u * M)) +
        42 / 10 * u * M * (2 * M + 42 / 10 * u * M) ≤ 96 / 10 * u * M ^ 2 := by
      have hu2 : u * u ≤ u / 1024 := by
        have := mul_le_mul_of_nonneg_left hu hu0; linarith
      have hu3 : u * (u * u) ≤ u / 1024 := by
        have := mul_le_mul_of_nonneg_left hu2 hu0
        have : u * (u / 1024) ≤ u / 1024 := by
          have := mul_le_mul_of_nonneg_right (le_trans hu (by norm_num : (1 : ℝ) / 1024 ≤ 1))
            (by positivity : (0 : ℝ) ≤ u / 1024)
          linarith
        linarith
      have := mul_le_mul_of_nonneg_left hO2 hu0
      have hM2 := sq_nonneg M
      have e : u * (M ^ 2 + 42 / 10 * u * M * (2 * M + 42 / 10 * u * M)) +
          42 / 10 * u * M * (2 * M + 42 / 10 * u * M) =
          M ^ 2 * (u + 84 / 10 * (u * u) + (42 / 10) ^ 2 * (u * (u * u)) + 84 / 10 * u +
            (42 / 10) ^ 2 * (u * u)) := by ring
      have : u * (M ^ 2 + 42 / 10 * u * M * (2 * M + 42 / 10 * u * M)) +
          42 / 10 * u * M * (2 * M + 42 / 10 * u * M) ≤ 96 / 10 * u * M ^ 2 := by
        rw [e, show 96 / 10 * u * M ^ 2 = M ^ 2 * (96 / 10 * u) by ring]
        apply mul_le_mul_of_nonneg_left _ hM2; linarith
      linarith
    linarith
  -- n within 4.2 u N, ll within u l², w within 6.3 u l² N
  have hUX := mul_self_nonneg UX
  have hUY := mul_self_nonneg UY
  have ea := rel_mul hu0 hu (by norm_num) (by norm_num) (by norm_num) (by norm_num) one one ha
  have eb := rel_mul hu0 hu (by norm_num) (by norm_num) (by norm_num) (by norm_num) one' one' hb
  rw [show (1 : ℝ) + 1 + 11 / 10 = 31 / 10 by norm_num, ← abs_mul_abs_self UX] at ea
  rw [show (1 : ℝ) + 1 + 11 / 10 = 31 / 10 by norm_num, ← abs_mul_abs_self UY] at eb
  rw [abs_mul_abs_self] at ea eb
  rw [abs_of_nonneg hUX] at ea; rw [abs_of_nonneg hUY] at eb
  have en := rel_add hu0 hu (by norm_num) (by norm_num) hUX hUY ea eb hn
  rw [show (31 : ℝ) / 10 + 101 / 100 = 411 / 100 by norm_num] at en
  have hN0 : 0 ≤ N := by positivity
  have en' : |n - N| ≤ 42 / 10 * u * |N| := by
    rw [abs_of_nonneg hN0]; have := mul_nonneg hu0 hN0; linarith
  have ell : |ll - l * l| ≤ 1 * u * |l * l| := by linarith
  have ew := rel_mul hu0 hu (by norm_num) (by norm_num) (by norm_num) (by norm_num) ell en' hw
  rw [show (1 : ℝ) + 42 / 10 + 11 / 10 = 63 / 10 by norm_num] at ew
  have hL := mul_self_nonneg l
  have hLN : 0 ≤ l * l * N := by positivity
  rw [abs_of_nonneg hLN] at ew
  -- v within 10.7 u (M² + l² N)
  set W := M ^ 2 + l * l * N
  have hW0 : 0 ≤ W := by positivity
  have ev : |v - V| ≤ 107 / 10 * u * W := by
    have d := sub_close eoo ew
    have := rnd_close hu0 hv d
    have hV : |V| ≤ W := by
      have hO2 : O ^ 2 ≤ M ^ 2 := by
        rw [← sq_abs]; exact pow_le_pow_left₀ (abs_nonneg _) hOM 2
      rw [abs_le]; constructor <;> linarith [sq_nonneg O]
    have eW : 96 / 10 * u * M ^ 2 + 63 / 10 * u * (l * l * N) ≤ 96 / 10 * u * W := by
      have : 0 ≤ u * (l * l * N) := by positivity
      simp only [W]; linarith
    have a1 := mul_le_mul_of_nonneg_left hV hu0
    have a2 := mul_le_mul_of_nonneg_left eW hu0
    have a3 : u * (96 / 10 * u * W) ≤ 96 / 10 * (1 / 1024) * u * W := by
      have := mul_le_mul_of_nonneg_right hu (by positivity : (0 : ℝ) ≤ 96 / 10 * u * W); linarith
    have a4 := mul_nonneg hu0 hW0
    have e : u * (|V| + (96 / 10 * u * M ^ 2 + 63 / 10 * u * (l * l * N))) +
        (96 / 10 * u * M ^ 2 + 63 / 10 * u * (l * l * N)) ≤ 107 / 10 * u * W := by
      have : u * (|V| + (96 / 10 * u * M ^ 2 + 63 / 10 * u * (l * l * N))) =
          u * |V| + u * (96 / 10 * u * M ^ 2 + 63 / 10 * u * (l * l * N)) := by ring
      linarith
    linarith
  -- T ≥ 15.8 u W
  have p1l : (1 - 31 / 10 * u) * |UX * VY| ≤ |p1| := by
    have := abs_sub_abs_le_abs_sub (UX * VY) p1
    have h' : |UX * VY - p1| = |p1 - UX * VY| := abs_sub_comm _ _
    linarith
  have p2l : (1 - 31 / 10 * u) * |UY * VX| ≤ |p2| := by
    have := abs_sub_abs_le_abs_sub (UY * VX) p2
    have h' : |UY * VX - p2| = |p2 - UY * VX| := abs_sub_comm _ _
    linarith
  have m0 : 0 ≤ |p1| + |p2| := by positivity
  rw [abs_of_nonneg m0] at hm
  have ml : (1 - u) * ((1 - 31 / 10 * u) * M) ≤ m := by
    have := (abs_le.mp hm).1
    have h2 : (1 - 31 / 10 * u) * M ≤ |p1| + |p2| := by simp only [M]; linarith
    have := mul_le_mul_of_nonneg_left h2 (by linarith : (0 : ℝ) ≤ 1 - u)
    linarith
  have hm0 : 0 ≤ (1 - u) * ((1 - 31 / 10 * u) * M) := by
    apply mul_nonneg (by linarith); apply mul_nonneg (by linarith) hM0
  have mml : (1 - u) * ((1 - u) * ((1 - 31 / 10 * u) * M)) ^ 2 ≤ mm := by
    rw [abs_of_nonneg (mul_self_nonneg m)] at hmm
    have := (abs_le.mp hmm).1
    have : ((1 - u) * ((1 - 31 / 10 * u) * M)) ^ 2 ≤ m * m := by
      rw [sq]; exact mul_le_mul ml ml hm0 (le_trans hm0 ml)
    have := mul_le_mul_of_nonneg_left this (by linarith : (0 : ℝ) ≤ 1 - u)
    linarith
  have wl : (1 - 63 / 10 * u) * (l * l * N) ≤ w := by have := (abs_le.mp ew).1; linarith
  have c1 : (1 - 10 * u) * M ^ 2 ≤ (1 - u) * ((1 - u) * ((1 - 31 / 10 * u) * M)) ^ 2 := by
    have e : (1 - u) * ((1 - u) * ((1 - 31 / 10 * u) * M)) ^ 2 =
        ((1 - u) ^ 3 * (1 - 31 / 10 * u) ^ 2) * M ^ 2 := by ring
    rw [e]; exact mul_le_mul_of_nonneg_right (poly1 hu0 hu) (sq_nonneg M)
  have sW : (1 - 10 * u) * W ≤ mm + w := by
    have : (1 - 10 * u) * (l * l * N) ≤ (1 - 63 / 10 * u) * (l * l * N) :=
      mul_le_mul_of_nonneg_right (by linarith) hLN
    simp only [W]; linarith
  have sw0 : 0 ≤ mm + w := le_trans (mul_nonneg (by linarith) hW0) sW
  rw [abs_of_nonneg sw0] at ht
  have tl : (1 - u) * ((1 - 10 * u) * W) ≤ t := by
    have := (abs_le.mp ht).1
    have := mul_le_mul_of_nonneg_left sW (by linarith : (0 : ℝ) ≤ 1 - u)
    linarith
  have t0 : 0 ≤ t := le_trans (mul_nonneg (by linarith) (mul_nonneg (by linarith) hW0)) tl
  rw [abs_of_nonneg (by positivity : 0 ≤ 16 * u * t)] at hT
  have T1 : 16 * u * (1 - u) * t ≤ T := by have := (abs_le.mp hT).1; linarith
  have key : 107 / 10 * u * W ≤ 16 * u * (1 - u) * ((1 - u) * ((1 - 10 * u) * W)) := by
    have e : 16 * u * (1 - u) * ((1 - u) * ((1 - 10 * u) * W)) =
        u * W * (16 * (1 - u) ^ 2 * (1 - 10 * u)) := by ring
    rw [e, show 107 / 10 * u * W = u * W * (107 / 10) by ring]
    exact mul_le_mul_of_nonneg_left (poly2 hu0 hu) (mul_nonneg hu0 hW0)
  have : 16 * u * (1 - u) * ((1 - u) * ((1 - 10 * u) * W)) ≤ 16 * u * (1 - u) * t :=
    mul_le_mul_of_nonneg_left tl (mul_nonneg (by linarith) (by linarith))
  linarith

end Sheet.Filter16
