/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Side
import IEEE754.Screen.Meet

/-!
# decide-once's `interior_f` (item 12)

`interior_f` projects the f64 point `m` onto the class plane `c` along axis `a`. Off `a`, the
projection keeps `m`'s coordinates exactly. On `a`, the f64 coordinate is
`p_a = (n.c − n_s·m_s − n_t·m_t)/n_a`, computed left to right. The normals are f64 exactly, and
`n.c` is one rounding off the exact offset `C` (`2ε` relative).

- `proj_close` (claim A): `|p_a − P_a| ≤ e`, where `P_a = (C − n_s m_s − n_t m_t)/n_a` exactly and
  `e = γ·(|n.c| + |n_s m_s| + |n_t m_t|)/|n_a| + γ·|p_a|` in f64 (`γ = 2^-46`). The rounding error
  is about `5ε` of the same terms, against `γ = 128ε`.
- `e_nonneg`: so `e ≥ 0`, and the two exact coordinates are within `e` too.

Claim B is then item 2 as stated: `side_err_sound` with `err = e` at all three coordinates.
Products and quotients are zero or normal (`NZ`).
-/

namespace IEEE754.Screen

open FP

/-- `p[a] = (n.c - ps - pt) / n.n[a]`. -/
def paF (c ns nt na ms mt : F64) : F64 := div (sub (sub c (mul ns ms)) (mul nt mt)) na

/-- `e = GAMMA * (n.c.abs() + ps.abs() + pt.abs()) / n.n[a].abs() + GAMMA * p[a].abs()`. -/
def eaF (c ns nt na ms mt : F64) : F64 :=
  add (div (mul GAMMA (add (add c.abs (mul ns ms).abs) (mul nt mt).abs)) na.abs)
    (mul GAMMA (paF c ns nt na ms mt).abs)

set_option maxHeartbeats 4000000 in
/-- **Claim A.** -/
theorem proj_close {c ns nt na ms mt : F64} {C : ℚ}
    (hc : Tr c C |C| 2) (hns : ns.isFinite) (hnt : nt.isFinite) (hna : na.isFinite) (hms : ms.isFinite)
    (hmt : mt.isFinite) (hna0 : na.val ≠ 0)
    (hfp : (paF c ns nt na ms mt).isFinite) (hfe : (eaF c ns nt na ms mt).isFinite)
    (hU : ∀ r ∈ [ns.val * ms.val, nt.val * mt.val,
      (sub (sub c (mul ns ms)) (mul nt mt)).val / na.val,
      GAMMA.val * (add (add c.abs (mul ns ms).abs) (mul nt mt).abs).val,
      (mul GAMMA (add (add c.abs (mul ns ms).abs) (mul nt mt).abs)).val / na.abs.val,
      GAMMA.val * (paF c ns nt na ms mt).abs.val], NZ r) :
    |(paF c ns nt na ms mt).val - (C - ns.val * ms.val - nt.val * mt.val) / na.val| ≤
      (eaF c ns nt na ms mt).val := by
  have u : ∀ i, (h : i < 6) → NZ ([ns.val * ms.val, nt.val * mt.val,
      (sub (sub c (mul ns ms)) (mul nt mt)).val / na.val,
      GAMMA.val * (add (add c.abs (mul ns ms).abs) (mul nt mt).abs).val,
      (mul GAMMA (add (add c.abs (mul ns ms).abs) (mul nt mt).abs)).val / na.abs.val,
      GAMMA.val * (paF c ns nt na ms mt).abs.val].get ⟨i, by simpa using h⟩) :=
    fun i h => hU _ (List.get_mem _ _)
  have hcf := hc.fin
  have ecC := hc.err
  -- the numerator
  unfold paF at hfp
  have fnum := div_num_fin hfp hna
  have f1 := (isFinite_of_sub fnum).1
  have fpt := (isFinite_of_sub fnum).2
  have fps := (isFinite_of_sub f1).2
  have eps := mul_rel0 hns hms fps (u 0 (by norm_num))
  have ept := mul_rel0 hnt hmt fpt (u 1 (by norm_num))
  have e1 := sub_rel0 hcf fps f1
  have e2 := sub_rel0 f1 fpt fnum
  have eq := div_rel0 fnum hna hna0 hfp (u 2 (by norm_num))
  set ps := (mul ns ms).val; set pt := (mul nt mt).val
  set n1 := (sub c (mul ns ms)).val
  set num := (sub (sub c (mul ns ms)) (mul nt mt)).val
  set pa := (div (sub (sub c (mul ns ms)) (mul nt mt)) na).val
  set cv := c.val
  set A := |cv| + |ps| + |pt| with hAd
  have hA0 : 0 ≤ A := by positivity
  -- the exact inputs, against the f64 ones
  have bps : |ps - ns.val * ms.val| ≤ 2 * ε * |ps| := by
    have := abs_sub_abs_le_abs_sub (ns.val * ms.val) ps
    rw [abs_sub_comm] at this
    have : ε * |ns.val * ms.val| ≤ ε * (|ps| + ε * |ns.val * ms.val|) := by
      apply mul_le_mul_of_nonneg_left _ (by norm_num); linarith
    nlinarith [abs_nonneg (ns.val * ms.val)]
  have bpt : |pt - nt.val * mt.val| ≤ 2 * ε * |pt| := by
    have := abs_sub_abs_le_abs_sub (nt.val * mt.val) pt
    rw [abs_sub_comm] at this
    have : ε * |nt.val * mt.val| ≤ ε * (|pt| + ε * |nt.val * mt.val|) := by
      apply mul_le_mul_of_nonneg_left _ (by norm_num); linarith
    nlinarith [abs_nonneg (nt.val * mt.val)]
  have bc : |cv - C| ≤ 3 * ε * |cv| := (exact_rel ecC).2
  have bn1 : |n1 - (cv - ps)| ≤ ε * A := by
    refine le_trans e1 (mul_le_mul_of_nonneg_left ?_ (by norm_num))
    exact le_trans (abs_sub _ _) (by simp only [hAd]; linarith [abs_nonneg pt])
  have hn1A : |n1 - pt| ≤ 2 * A := by
    have := abs_sub n1 pt
    have := abs_sub_abs_le_abs_sub n1 (cv - ps)
    have := abs_sub cv ps
    have : ε * A ≤ A := by nlinarith
    simp only [hAd] at *; linarith
  have bnum : |num - (C - ns.val * ms.val - nt.val * mt.val)| ≤ 8 * ε * A := by
    have : |num - (n1 - pt)| ≤ ε * (2 * A) := le_trans e2 (mul_le_mul_of_nonneg_left hn1A (by norm_num))
    rw [show num - (C - ns.val * ms.val - nt.val * mt.val) =
      (num - (n1 - pt)) + (n1 - (cv - ps)) + (cv - C) - (ps - ns.val * ms.val) - (pt - nt.val * mt.val) by ring]
    have h1 := abs_add_le ((num - (n1 - pt)) + (n1 - (cv - ps))) (cv - C)
    have h2 := abs_add_le (num - (n1 - pt)) (n1 - (cv - ps))
    have h3 := abs_sub (((num - (n1 - pt)) + (n1 - (cv - ps))) + (cv - C) - (ps - ns.val * ms.val)) (pt - nt.val * mt.val)
    have h4 := abs_sub (((num - (n1 - pt)) + (n1 - (cv - ps))) + (cv - C)) (ps - ns.val * ms.val)
    simp only [hAd] at *
    have := abs_nonneg cv; have := abs_nonneg ps; have := abs_nonneg pt
    linarith
  -- the quotient
  have hna' : 0 < |na.val| := abs_pos.mpr hna0
  set w := 1 / |na.val| with hwd
  have hw : 0 < w := by positivity
  have hq : |num / na.val - (C - ns.val * ms.val - nt.val * mt.val) / na.val| ≤ 8 * ε * A * w := by
    rw [← sub_div, abs_div, hwd, mul_one_div]
    exact div_le_div_of_nonneg_right bnum hna'.le
  have hqa : |num / na.val| ≤ (1 + 2 * ε) * |pa| := by
    have := abs_sub_abs_le_abs_sub (num / na.val) pa
    rw [abs_sub_comm] at this
    have : ε * |num / na.val| ≤ ε * (|pa| + ε * |num / na.val|) := by
      apply mul_le_mul_of_nonneg_left _ (by norm_num); linarith
    nlinarith [abs_nonneg (num / na.val), abs_nonneg pa]
  have err : |pa - (C - ns.val * ms.val - nt.val * mt.val) / na.val| ≤ 8 * ε * (A * w) + 2 * ε * |pa| := by
    have := abs_sub_le pa (num / na.val) ((C - ns.val * ms.val - nt.val * mt.val) / na.val)
    have : ε * |num / na.val| ≤ ε * ((1 + 2 * ε) * |pa|) := mul_le_mul_of_nonneg_left hqa (by norm_num)
    have := abs_nonneg pa
    nlinarith
  -- `e`, from below
  simp only [eaF, paF] at hfe ⊢
  have fe1 := (isFinite_of_add hfe).1
  have fe2 := (isFinite_of_add hfe).2
  have fg := div_num_fin fe1 (by simpa using hna)
  have fsum := (isFinite_of_mul fg).2
  have fs1 := (isFinite_of_add fsum).1
  have fG : GAMMA.isFinite := by decide
  have fca : c.abs.isFinite := by simpa using hcf
  have fpsa : (mul ns ms).abs.isFinite := by simpa using fps
  have fpta : (mul nt mt).abs.isFinite := by simpa using fpt
  have s1 := add_rel0 fca fpsa fs1
  have s2 := add_rel0 fs1 fpta fsum
  simp only [FP.abs_val] at s1 s2
  set sm1 := (add c.abs (mul ns ms).abs).val
  set sm := (add (add c.abs (mul ns ms).abs) (mul nt mt).abs).val
  rw [abs_of_nonneg (by positivity : (0 : ℚ) ≤ |cv| + |ps|)] at s1
  have hsm1 : (1 - ε) * (|cv| + |ps|) ≤ sm1 := by have := (abs_le.mp s1).1; linarith
  have hsm10 : 0 ≤ sm1 := le_trans (by positivity) hsm1
  rw [abs_of_nonneg (by positivity : (0 : ℚ) ≤ sm1 + |pt|)] at s2
  have hsm : (1 - 2 * ε) * A ≤ sm := by
    have := (abs_le.mp s2).1
    have : (1 - ε) * ((1 - ε) * (|cv| + |ps|) + |pt|) ≤ (1 - ε) * (sm1 + |pt|) :=
      mul_le_mul_of_nonneg_left (by linarith) (by norm_num)
    simp only [hAd]; nlinarith [abs_nonneg pt, abs_nonneg cv, abs_nonneg ps]
  have hsm0 : 0 ≤ sm := le_trans (by nlinarith) hsm
  have eg := mul_rel0 fG fsum fg (u 3 (by norm_num))
  rw [GAMMA_val, abs_of_nonneg (by positivity : (0 : ℚ) ≤ 1 / 2 ^ 46 * sm)] at eg
  set g := (mul GAMMA (add (add c.abs (mul ns ms).abs) (mul nt mt).abs)).val
  have hg : (1 - ε) * (1 / 2 ^ 46 * sm) ≤ g := by have := (abs_le.mp eg).1; linarith
  have fnaa : na.abs.isFinite := by simpa using hna
  have hnaa0 : na.abs.val ≠ 0 := by rw [FP.abs_val]; exact hna'.ne'
  have ee1 := div_rel0 fg fnaa hnaa0 fe1 (u 4 (by norm_num))
  rw [FP.abs_val] at ee1
  have hg0 : 0 ≤ g := le_trans (by positivity) hg
  rw [abs_of_nonneg (div_nonneg hg0 hna'.le)] at ee1
  set e1 := (div (mul GAMMA (add (add c.abs (mul ns ms).abs) (mul nt mt).abs)) na.abs).val
  have he1 : (1 - ε) * (g * w) ≤ e1 := by
    have := (abs_le.mp ee1).1; rw [hwd, mul_one_div]; linarith
  have fpa : (div (sub (sub c (mul ns ms)) (mul nt mt)) na).abs.isFinite := by simpa using hfp
  have ee2 := mul_rel0 fG fpa fe2 (by simpa [paF] using u 5 (by norm_num))
  rw [GAMMA_val, FP.abs_val, abs_of_nonneg (by positivity : (0 : ℚ) ≤ 1 / 2 ^ 46 * |pa|)] at ee2
  try simp only [paF] at ee2
  set e2 := (mul GAMMA (div (sub (sub c (mul ns ms)) (mul nt mt)) na).abs).val
  have he2 : (1 - ε) * (1 / 2 ^ 46 * |pa|) ≤ e2 := by have := (abs_le.mp ee2).1; linarith
  have he10 : 0 ≤ e1 := le_trans (by positivity) he1
  have he20 : 0 ≤ e2 := le_trans (by positivity) he2
  have ee := add_rel0 fe1 fe2 hfe
  rw [abs_of_nonneg (by linarith : (0 : ℚ) ≤ e1 + e2)] at ee
  have he : (1 - ε) * (e1 + e2) ≤ (add (div (mul GAMMA (add (add c.abs (mul ns ms).abs) (mul nt mt).abs)) na.abs)
      (mul GAMMA (div (sub (sub c (mul ns ms)) (mul nt mt)) na).abs)).val := by
    have := (abs_le.mp ee).1; linarith
  -- `g·w` against `A·w`
  have hgw : (1 - ε) * (1 / 2 ^ 46) * (1 - 2 * ε) * (A * w) ≤ g * w := by
    have h1 : (1 - ε) * (1 / 2 ^ 46) * (1 - 2 * ε) * A ≤ g := by nlinarith
    have := mul_le_mul_of_nonneg_right h1 hw.le; linarith
  have hAw : 0 ≤ A * w := by positivity
  have hpa := abs_nonneg pa
  nlinarith

/-- `e ≥ 0` (it bounds an absolute value), so the two coordinates `m` keeps exactly are within `e`
too, and `side_err_sound` applies with `err = e` at all three. -/
theorem e_nonneg {c ns nt na ms mt : F64} {Pa : ℚ}
    (h : |(paF c ns nt na ms mt).val - Pa| ≤ (eaF c ns nt na ms mt).val) (x : F64) :
    0 ≤ (eaF c ns nt na ms mt).val ∧ |x.val - x.val| ≤ (eaF c ns nt na ms mt).val :=
  ⟨le_trans (abs_nonneg _) h, by rw [_root_.sub_self, abs_zero]; exact le_trans (abs_nonneg _) h⟩

end IEEE754.Screen
