/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Theorems
import IEEE754.RoundProps
import IEEE754.TwoProduct

/-!
# A conic's size, made by a division, is finite and positive

kernel/forge-geom/src/intersect.rs `plane_cylinder` makes the ellipse where a plane crosses a
cylinder of radius `r` at `cos = |n·a| > ANGULAR` (its branch condition): `major = r / cos`,
`minor = r`. `Ellipse::new` (forge-geom curve.rs) asks of it H.curveOK's conic parameters: finite,
`major > 0`, `minor >= 0`. Over the IEEE 754 model, for any format:

* (`round_finite_of_le_emax`, TwoProduct.lean: a rational at most `2^emax` rounds to a finite value;)
* **`round_ge_two_pow_qmin`**: one at least `2^qmin` (the least subnormal) rounds to at least `2^qmin`, so
  never to zero;
* **`div_pos_finite`**: hence `x / y` is finite and positive when `x` is finite with
  `2^(qmin+1) <= x <= 2^emax * A` and `y` is finite with `A <= y <= 2`, `A > 0`.

For `plane_cylinder` (binary64): `x = r`, `y = cos`, `A = ANGULAR`; `cos <= 2` holds for the f64 dot
product of two unit vectors (at most 1 plus rounding). So the ellipse it makes satisfies H.curveOK's
conic parameters whenever the cylinder's radius is finite with `2^-1073 <= r <= 2^1023 * ANGULAR`:
that is the cylinder's own hypothesis (surface radii finite and in range), which moves H.curveOK's
ellipse clause onto the surfaces the booleans meet.
-/

namespace IEEE754

variable (f : Format)

/-- A rational at least `2^qmin` (and at most `2^emax`) rounds to a finite value at least `2^qmin`:
never to zero. -/
theorem round_ge_two_pow_qmin {zs : Bool} {q : ℚ} (hq : (2 : ℚ) ^ f.qmin ≤ q) (hle : q ≤ 2 ^ f.emax) :
    (round f zs q).isFinite ∧ (2 : ℚ) ^ f.qmin ≤ (round f zs q).val := by
  have hpos : 0 < q := lt_of_lt_of_le (two_zpow_pos _) hq
  have hfin : (round f zs q).isFinite := round_finite_of_le_emax (by rw [abs_of_pos hpos]; exact hle)
  have hu : uround f q < 2 ^ (f.emax + 1) := by
    have := (round_finite_iff f (zs := zs) (ne_of_gt hpos)).mp hfin
    rwa [abs_of_pos hpos] at this
  obtain ⟨_, hval, _⟩ := round_pos f (zs := zs) hpos hu
  refine ⟨hfin, ?_⟩
  have hex : uround f ((1 : ℕ) * (2 : ℚ) ^ f.qmin) = (1 : ℕ) * 2 ^ f.qmin :=
    uround_exact f (by norm_num) le_rfl (Nat.one_le_two_pow)
  have hmono := uround_mono f (two_zpow_pos (K := ℚ) f.qmin) hq
  simp only [Nat.cast_one, one_mul] at hex
  rw [hex] at hmono
  have : ((round f zs q).val : ℚ) = uround f q := hval
  linarith

/-- **`div_pos_finite`**: `x / y` is finite and positive for `x` finite in
`[2^(qmin+1), 2^emax * A]` and `y` finite in `[A, 2]`, `A > 0`. -/
theorem div_pos_finite {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) {A : ℚ} (hA : 0 < A)
    (hyA : A ≤ y.val) (hy2 : y.val ≤ 2) (hxlo : (2 : ℚ) ^ (f.qmin + 1) ≤ x.val)
    (hxhi : x.val ≤ 2 ^ f.emax * A) :
    (FP.div x y).isFinite ∧ 0 < (FP.div x y).val := by
  have hy0 : y.val ≠ 0 := ne_of_gt (lt_of_lt_of_le hA hyA)
  have hypos : 0 < y.val := lt_of_lt_of_le hA hyA
  rw [div_eq_round hx hy hy0]
  have hlo : (2 : ℚ) ^ f.qmin ≤ x.val / y.val := by
    rw [le_div_iff₀ hypos]
    have h2 : (2 : ℚ) ^ (f.qmin + 1) = 2 ^ f.qmin * 2 := by
      rw [zpow_add₀ (by norm_num : (2 : ℚ) ≠ 0), zpow_one]
    have hq := two_zpow_pos (K := ℚ) f.qmin
    nlinarith
  have hhi : x.val / y.val ≤ 2 ^ f.emax := by
    rw [div_le_iff₀ hypos]
    have he := two_zpow_pos (K := ℚ) f.emax
    nlinarith
  obtain ⟨hfin, hge⟩ := round_ge_two_pow_qmin f (zs := x.sign ^^ y.sign) hlo hhi
  exact ⟨hfin, lt_of_lt_of_le (two_zpow_pos _) hge⟩

end IEEE754
