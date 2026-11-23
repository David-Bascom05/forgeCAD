import Mathlib.Analysis.SpecialFunctions.Trigonometric.Basic
import Mathlib.Tactic

/-!
# The setback's error (the model)

shop.rs `setback`: `ρ ⊗ s ⊘ (1 ⊕ c)` with `(s, c)` dmath's sine and cosine of `θ`, each within
`ε` of the true one (verify/ieee754 `Dmath.sinCos_err`: `ε = 2⁻⁵²`), and each operation within
`u |r| + η` of its exact result `r`. With `C = 1 + cos θ` the exact setback is `ρ sin θ / C`
(`= ρ tan(θ/2)`, verify/sheet `Fold.s_eq_tan`), and (`setback_err`), when `ε ≤ C/4`, `η ≤ C/8`:

  `|computed − ρ sin θ / C| ≤ |ρ| (4ε + 4η) / C² + 10 u |ρ| / C + 3η / C + η`.

The `ε / C²` term is the cosine's error seen through `1 / C`: as `θ → π` the setback is
ill-conditioned (relative error about `ε / C`), so a fold within `ε`'s reach of a hairpin can't
be computed to `LINEAR` from double sines however it is evaluated.
-/

namespace ForgeSheetCore.Setback

/-- The computed denominator `1 ⊕ c` is at least `C / 2`. -/
theorem den_pos {u η ε θ c d : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 8) (hC : 0 < 1 + Real.cos θ)
    (hε : ε ≤ (1 + Real.cos θ) / 4) (hηC : η ≤ (1 + Real.cos θ) / 8)
    (hc : |c - Real.cos θ| ≤ ε) (hd : |d - (1 + c)| ≤ u * |1 + c| + η) :
    (1 + Real.cos θ) / 2 ≤ d := by
  have hcl := (abs_le.mp hc).1
  have hcu := (abs_le.mp hc).2
  have hε0 : 0 ≤ ε := le_trans (abs_nonneg _) hc
  have hc1p : 0 < 1 + c := by linarith
  rw [abs_of_pos hc1p] at hd
  have hdl := (abs_le.mp hd).1
  have : u * (1 + c) ≤ (1 + c) / 8 := by nlinarith
  nlinarith

theorem setback_err {u η ε ρ θ s c m d r : ℝ} (hu0 : 0 ≤ u) (hu : u ≤ 1 / 8) (hη : 0 ≤ η)
    (hC : 0 < 1 + Real.cos θ) (hε : ε ≤ (1 + Real.cos θ) / 4) (hηC : η ≤ (1 + Real.cos θ) / 8)
    (hs : |s - Real.sin θ| ≤ ε) (hc : |c - Real.cos θ| ≤ ε)
    (hm : |m - ρ * s| ≤ u * |ρ * s| + η) (hd : |d - (1 + c)| ≤ u * |1 + c| + η)
    (hr : |r - m / d| ≤ u * |m / d| + η) :
    let C := 1 + Real.cos θ
    |r - ρ * Real.sin θ / C| ≤ |ρ| * (4 * ε + 4 * η) / C ^ 2 + 10 * u * |ρ| / C + 3 * η / C + η := by
  intro C
  have hC2 : C ≤ 2 := by have := Real.cos_le_one θ; simp only [C]; linarith
  have hε0 : 0 ≤ ε := le_trans (abs_nonneg _) hs
  have hsin : |Real.sin θ| ≤ 1 := Real.abs_sin_le_one θ
  have hsb : |s| ≤ 3 / 2 := by
    have := abs_sub_abs_le_abs_sub s (Real.sin θ); linarith
  have hcl := (abs_le.mp hc).1
  have hcu := (abs_le.mp hc).2
  have hc1 : 3 * C / 4 ≤ 1 + c := by simp only [C]; linarith
  have hc1p : 0 < 1 + c := by linarith
  rw [abs_of_pos hc1p] at hd
  have hdl := (abs_le.mp hd).1
  have hdu := (abs_le.mp hd).2
  have hu1c : u * (1 + c) ≤ (1 + c) / 8 := by nlinarith
  have hd1 : C / 2 ≤ d := by
    have : 1 + c ≤ C + ε := by simp only [C]; linarith
    nlinarith
  have hdp : 0 < d := by linarith
  -- reciprocals
  set iC := C⁻¹ with hiC
  set ia := d⁻¹ with hia
  set ib := (1 + c)⁻¹ with hib
  have hiC0 : 0 < iC := inv_pos.mpr hC
  have hia0 : 0 < ia := inv_pos.mpr hdp
  have hib0 : 0 < ib := inv_pos.mpr hc1p
  have hCne : C ≠ 0 := hC.ne'
  have ha : ia ≤ 2 * iC := by
    calc ia = 1 / d := (one_div d).symm
      _ ≤ 1 / (C / 2) := one_div_le_one_div_of_le (by positivity) hd1
      _ = 2 * iC := by rw [hiC]; field_simp
  have hb : ib ≤ 4 / 3 * iC := by
    calc ib = 1 / (1 + c) := (one_div _).symm
      _ ≤ 1 / (3 * C / 4) := one_div_le_one_div_of_le (by positivity) hc1
      _ = 4 / 3 * iC := by rw [hiC]; field_simp
  have hcb : (1 + c) * ib = 1 := mul_inv_cancel₀ hc1p.ne'
  have hda : d * ia = 1 := mul_inv_cancel₀ hdp.ne'
  have hA := abs_nonneg ρ
  have hρs : |ρ * s| ≤ 3 / 2 * |ρ| := by rw [abs_mul]; nlinarith [abs_nonneg s]
  -- the four pieces
  have e4 : |ρ * s / (1 + c) - ρ * Real.sin θ / C| ≤ 4 * |ρ| * ε * iC ^ 2 := by
    have : ρ * s / (1 + c) - ρ * Real.sin θ / C =
        ρ * ((s - Real.sin θ) * C - Real.sin θ * (c - Real.cos θ)) * ib * iC := by
      rw [hib, hiC]; field_simp; simp only [C]; ring_nf
    rw [this, abs_mul, abs_mul, abs_mul, abs_of_pos hib0, abs_of_pos hiC0]
    have num : |(s - Real.sin θ) * C - Real.sin θ * (c - Real.cos θ)| ≤ 3 * ε := by
      calc _ ≤ |(s - Real.sin θ) * C| + |Real.sin θ * (c - Real.cos θ)| := abs_sub _ _
        _ = |s - Real.sin θ| * C + |Real.sin θ| * |c - Real.cos θ| := by
            rw [abs_mul, abs_mul, abs_of_pos hC]
        _ ≤ ε * 2 + 1 * ε := by gcongr
        _ = 3 * ε := by ring
    calc |ρ| * |(s - Real.sin θ) * C - Real.sin θ * (c - Real.cos θ)| * ib * iC
        ≤ |ρ| * (3 * ε) * (4 / 3 * iC) * iC := by gcongr
      _ = 4 * |ρ| * ε * iC ^ 2 := by ring
  have e3 : |ρ * s / d - ρ * s / (1 + c)| ≤ 3 * u * |ρ| * iC + 4 * |ρ| * η * iC ^ 2 := by
    have : ρ * s / d - ρ * s / (1 + c) = ρ * s * ((1 + c) - d) * ia * ib := by
      rw [hia, hib]; field_simp
    rw [this, abs_mul, abs_mul, abs_mul, abs_of_pos hia0, abs_of_pos hib0]
    have k : |1 + c - d| ≤ u * (1 + c) + η := by rw [abs_sub_comm]; exact abs_le.mpr ⟨by linarith, by linarith⟩
    calc |ρ * s| * |1 + c - d| * ia * ib ≤ (3 / 2 * |ρ|) * (u * (1 + c) + η) * ia * ib := by gcongr
      _ = 3 / 2 * |ρ| * (u * ((1 + c) * ib) * ia + η * ia * ib) := by ring
      _ = 3 / 2 * |ρ| * (u * ia + η * ia * ib) := by rw [hcb]; ring
      _ ≤ 3 / 2 * |ρ| * (u * (2 * iC) + η * (2 * iC) * (4 / 3 * iC)) := by gcongr
      _ = 3 * u * |ρ| * iC + 4 * |ρ| * η * iC ^ 2 := by ring
  have e2 : |m / d - ρ * s / d| ≤ 3 * u * |ρ| * iC + 2 * η * iC := by
    rw [← sub_div, div_eq_mul_inv, ← hia, abs_mul, abs_of_pos hia0]
    calc |m - ρ * s| * ia ≤ (u * (3 / 2 * |ρ|) + η) * (2 * iC) := by
          gcongr
          exact le_trans hm (by gcongr)
      _ = 3 * u * |ρ| * iC + 2 * η * iC := by ring
  have e1 : |r - m / d| ≤ 4 * u * |ρ| * iC + η * iC + η := by
    have hmb : |m| ≤ 27 / 16 * |ρ| + η := by
      have := abs_sub_abs_le_abs_sub m (ρ * s)
      have : u * |ρ * s| ≤ 1 / 8 * (3 / 2 * |ρ|) := mul_le_mul hu hρs (abs_nonneg _) (by norm_num)
      linarith
    have : u * |m / d| ≤ 4 * u * |ρ| * iC + η * iC := by
      rw [div_eq_mul_inv, ← hia, abs_mul, abs_of_pos hia0]
      calc u * (|m| * ia) ≤ u * ((27 / 16 * |ρ| + η) * (2 * iC)) := by gcongr
        _ = 27 / 8 * u * |ρ| * iC + (2 * u) * η * iC := by ring
        _ ≤ 4 * u * |ρ| * iC + 1 * η * iC := by
            have : 2 * u ≤ 1 := by linarith
            gcongr; norm_num
        _ = _ := by ring
    linarith
  have tot : |r - ρ * Real.sin θ / C| ≤ |r - m / d| + |m / d - ρ * s / d| +
      |ρ * s / d - ρ * s / (1 + c)| + |ρ * s / (1 + c) - ρ * Real.sin θ / C| := by
    calc _ = |(r - m / d) + (m / d - ρ * s / d) + (ρ * s / d - ρ * s / (1 + c)) +
          (ρ * s / (1 + c) - ρ * Real.sin θ / C)| := by ring_nf
      _ ≤ |(r - m / d) + (m / d - ρ * s / d) + (ρ * s / d - ρ * s / (1 + c))| +
          |ρ * s / (1 + c) - ρ * Real.sin θ / C| := abs_add_le _ _
      _ ≤ _ := by gcongr; exact abs_add_three _ _ _
  have g : |ρ| * (4 * ε + 4 * η) / C ^ 2 + 10 * u * |ρ| / C + 3 * η / C + η =
      4 * |ρ| * ε * iC ^ 2 + 4 * |ρ| * η * iC ^ 2 + 10 * u * |ρ| * iC + 3 * η * iC + η := by
    rw [hiC]; field_simp
  rw [g]
  have : η * iC + 2 * η * iC ≤ 3 * η * iC := by linarith
  linarith

end ForgeSheetCore.Setback
