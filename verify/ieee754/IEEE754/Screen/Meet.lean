/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Track

/-!
# decide-once's `meet_f` (item 1)

The Cramer meet of three planes in f64, against the exact meet of the exact planes whose shadows
they are (each coefficient within `2ε`). `meet_sound`: when `|det| > 4·edet`, the exact `Det ≠ 0`
and each coordinate `p_k` is within the `k`-th error term `e_k` of `Num_k / Det`. Each product and
quotient is assumed zero or normal (`NZ`); additions never underflow.
-/

namespace IEEE754.Screen

open FP

/-- `u1*v2 - u2*v1`. -/
def crs (u1 u2 v1 v2 : F64) : F64 := sub (mul u1 v2) (mul u2 v1)
/-- `(u1*v2).abs() + (u2*v1).abs()`. -/
def cab (u1 u2 v1 v2 : F64) : F64 := add (mul u1 v2).abs (mul u2 v1).abs
/-- `x0*y0 + x1*y1 + x2*y2`. -/
def dot3 (x0 x1 x2 y0 y1 y2 : F64) : F64 := add (add (mul x0 y0) (mul x1 y1)) (mul x2 y2)

lemma tr_crs {u1 u2 v1 v2 : F64} {U1 U2 V1 V2 : ℚ} (h1 : Tr u1 U1 |U1| 2) (h2 : Tr u2 U2 |U2| 2)
    (g1 : Tr v1 V1 |V1| 2) (g2 : Tr v2 V2 |V2| 2) (hf : (crs u1 u2 v1 v2).isFinite)
    (z1 : NZ (u1.val * v2.val)) (z2 : NZ (u2.val * v1.val)) :
    Tr (crs u1 u2 v1 v2) (U1 * V2 - U2 * V1) (|U1| * |V2| + |U2| * |V1|) 8 := by
  unfold crs at hf ⊢
  have m1 := tr_mul h1 g2 (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_sub hf).1 z1
  have m2 := tr_mul h2 g1 (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_sub hf).2 z2
  norm_num at m1 m2
  have := tr_sub m1 m2 (by norm_num) (by norm_num) hf
  norm_num at this; exact this

lemma tr_cab {u1 u2 v1 v2 : F64} {U1 U2 V1 V2 : ℚ} (h1 : Tr u1 U1 |U1| 2) (h2 : Tr u2 U2 |U2| 2)
    (g1 : Tr v1 V1 |V1| 2) (g2 : Tr v2 V2 |V2| 2) (hf : (cab u1 u2 v1 v2).isFinite)
    (z1 : NZ (u1.val * v2.val)) (z2 : NZ (u2.val * v1.val)) :
    Tr (cab u1 u2 v1 v2) (|U1| * |V2| + |U2| * |V1|) (|U1| * |V2| + |U2| * |V1|) 8 := by
  unfold cab at hf ⊢
  have f1 : (mul u1 v2).isFinite := by simpa using (isFinite_of_add hf).1
  have f2 : (mul u2 v1).isFinite := by simpa using (isFinite_of_add hf).2
  have m1 := (tr_mul h1 g2 (by norm_num) (by norm_num) (by norm_num) (by norm_num) f1 z1).abs
  have m2 := (tr_mul h2 g1 (by norm_num) (by norm_num) (by norm_num) (by norm_num) f2 z2).abs
  norm_num at m1 m2
  have := tr_add m1 m2 (by norm_num) (by norm_num) hf
  norm_num at this; exact this

lemma tr_dot3 {x0 x1 x2 y0 y1 y2 : F64} {X0 X1 X2 Y0 Y1 Y2 Mx0 Mx1 Mx2 My0 My1 My2 : ℚ}
    (h0 : Tr x0 X0 Mx0 2) (h1 : Tr x1 X1 Mx1 2) (h2 : Tr x2 X2 Mx2 2)
    (g0 : Tr y0 Y0 My0 8) (g1 : Tr y1 Y1 My1 8) (g2 : Tr y2 Y2 My2 8)
    (hf : (dot3 x0 x1 x2 y0 y1 y2).isFinite)
    (z0 : NZ (x0.val * y0.val)) (z1 : NZ (x1.val * y1.val)) (z2 : NZ (x2.val * y2.val)) :
    Tr (dot3 x0 x1 x2 y0 y1 y2) (X0 * Y0 + X1 * Y1 + X2 * Y2) (Mx0 * My0 + Mx1 * My1 + Mx2 * My2) 16 := by
  unfold dot3 at hf ⊢
  have f2 := (isFinite_of_add hf).1
  have m0 := tr_mul h0 g0 (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_add f2).1 z0
  have m1 := tr_mul h1 g1 (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_add f2).2 z1
  have m2 := tr_mul h2 g2 (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_add hf).2 z2
  norm_num at m0 m1 m2
  have s1 := tr_add m0 m1 (by norm_num) (by norm_num) f2
  have m2' := m2.weaken (show (12 : ℚ) ≤ 14 by norm_num) m2.M_nonneg
  norm_num at s1
  have := tr_add s1 m2' (by norm_num) (by norm_num) hf
  norm_num at this; exact this

lemma div_num_fin {x y : F64} (h : (div x y).isFinite) (hy : y.isFinite) : x.isFinite := by
  cases x <;> cases y <;> simp_all [FP.div, FP.isFinite] <;> split_ifs at h <;> simp_all [FP.isFinite]

/-- `(GAMMA*mnum + |p|*edet)/(|det| - edet) + |p|*GAMMA`. -/
def errTerm (p det edet mnum : F64) : F64 :=
  add (div (add (mul GAMMA mnum) (mul p.abs edet)) (sub det.abs edet)) (mul p.abs GAMMA)

lemma lo_rel {x : F64} {r : ℚ} (h : |x.val - r| ≤ ε * |r|) (hr : 0 ≤ r) : (1 - ε) * r ≤ x.val := by
  rw [abs_of_nonneg hr] at h; have := (abs_le.mp h).1; linarith

lemma hi_rel {x : F64} {r : ℚ} (h : |x.val - r| ≤ ε * |r|) (hr : 0 ≤ r) : x.val ≤ (1 + ε) * r := by
  rw [abs_of_nonneg hr] at h; have := (abs_le.mp h).2; linarith

/-- **The quotient step of `meet_f`.** -/
theorem quot_sound {det dabs num mnum : F64} {Det Num Perm Mn : ℚ}
    (hd : Tr det Det Perm 16) (hda : Tr dabs Perm Perm 16) (hn : Tr num Num Mn 16) (hm : Tr mnum Mn Mn 16)
    (hfe : (errTerm (div num det) det (mul GAMMA dabs) mnum).isFinite)
    (hsome : 4 * (mul GAMMA dabs).val < |det.val|)
    (z1 : NZ (GAMMA.val * dabs.val)) (z2 : NZ (num.val / det.val)) (z3 : NZ (GAMMA.val * mnum.val))
    (z4 : NZ ((div num det).abs.val * (mul GAMMA dabs).val))
    (z5 : NZ ((add (mul GAMMA mnum) (mul (div num det).abs (mul GAMMA dabs))).val /
      (sub det.abs (mul GAMMA dabs)).val))
    (z6 : NZ ((div num det).abs.val * GAMMA.val)) :
    Det ≠ 0 ∧ |(div num det).val - Num / Det| ≤ (errTerm (div num det) det (mul GAMMA dabs) mnum).val := by
  unfold errTerm at hfe ⊢
  set edet := mul GAMMA dabs
  set p := div num det
  have hG : GAMMA.val = 1 / 2 ^ 46 := GAMMA_val
  have fG : GAMMA.isFinite := by decide
  have fdet := hd.fin
  have fda : det.abs.isFinite := by simpa using fdet
  -- `edet` against `Perm`
  have hPerm := hda.M_nonneg
  have dlo := hda.lower
  have hdab0 : 0 ≤ dabs.val := le_trans (mul_nonneg (by norm_num) hPerm) dlo
  have fedet : edet.isFinite := by
    simp only [edet]; rw [mul_eq_round fG hda.fin]
    refine round_finite_of_le hda.fin ?_
    rw [hG, abs_mul, abs_of_pos (by norm_num : (0 : ℚ) < 1 / 2 ^ 46)]
    have := abs_nonneg dabs.val; linarith
  have ee := mul_rel0 fG hda.fin fedet z1
  rw [hG] at ee z1
  have ed_lo := lo_rel ee (mul_nonneg (by norm_num) hdab0)
  have hedet0 : 0 ≤ edet.val := le_trans (mul_nonneg (by norm_num) (mul_nonneg (by norm_num) hdab0)) ed_lo
  -- `det` against `Det`, `|Det|` bounded below
  have dD := hd.err
  have k1 : 16 * ε * Perm ≤ edet.val / 7 := by linarith
  have hdet_pos : 0 < |det.val| := lt_of_le_of_lt (by linarith) hsome
  have hDet : |det.val| - edet.val ≤ |Det| := by
    have := abs_sub_abs_le_abs_sub det.val Det; linarith
  have hD0 : 0 < |det.val| - edet.val := by linarith
  have hDet0 : Det ≠ 0 := by intro h; rw [h, abs_zero] at hDet; linarith
  refine ⟨hDet0, ?_⟩
  have hdet0 : det.val ≠ 0 := by intro h; rw [h, abs_zero] at hdet_pos; linarith
  have ft4 : (sub det.abs edet).isFinite := by
    rw [sub_eq_round fda fedet]
    refine round_finite_of_le fdet ?_
    rw [FP.abs_val, abs_of_nonneg (by linarith)]; linarith
  -- finiteness of the rest
  have ft5 := (isFinite_of_add hfe).1
  have ft6 := (isFinite_of_add hfe).2
  have fpa := (isFinite_of_mul ft6).1
  have fp : p.isFinite := by simpa using fpa
  have ft3 := div_num_fin ft5 ft4
  have ft1 := (isFinite_of_add ft3).1
  have ft2 := (isFinite_of_add ft3).2
  -- `p` against `q = num/det`
  set q := num.val / det.val
  have ep := div_rel0 hn.fin fdet hdet0 (by simpa using fp) z2
  have hq : |q| ≤ (1 + 2 * ε) * |p.val| := by
    have := abs_sub_abs_le_abs_sub q p.val; rw [abs_sub_comm] at this; have := abs_nonneg q; linarith
  -- `q` against `P = Num/Det`
  have mlo := hm.lower
  have hMn := hm.M_nonneg
  have hmn0 : 0 ≤ mnum.val := le_trans (mul_nonneg (by norm_num) hMn) mlo
  have k2 : 16 * ε * Mn ≤ (1 / 2 ^ 46 * mnum.val) / 7 := by linarith
  have key : q - Num / Det = ((num.val - Num) - q * (det.val - Det)) / Det := by
    rw [eq_div_iff hDet0, sub_mul, div_mul_cancel₀ _ hDet0]
    simp only [q]; field_simp; ring
  have hqP : |q - Num / Det| ≤ (16 * ε * Mn + |q| * (16 * ε * Perm)) / (|det.val| - edet.val) := by
    rw [key, abs_div]
    apply div_le_div₀ (by positivity) _ hD0 hDet
    refine le_trans (abs_sub _ _) (add_le_add hn.err ?_)
    rw [abs_mul]; exact mul_le_mul_of_nonneg_left hd.err (abs_nonneg _)
  -- the computed terms
  have hpa : p.abs.val = |p.val| := FP.abs_val _
  have e1 := mul_rel0 fG hm.fin ft1 z3
  rw [hG] at e1
  have t1_lo := lo_rel e1 (mul_nonneg (by norm_num) hmn0)
  have e2 := mul_rel0 fpa fedet ft2 z4
  rw [hpa] at e2
  have t2_lo := lo_rel e2 (mul_nonneg (abs_nonneg _) hedet0)
  have e3 := add_rel0 ft1 ft2 ft3
  have t1_0 : 0 ≤ (mul GAMMA mnum).val := le_trans (mul_nonneg (by norm_num) (mul_nonneg (by norm_num) hmn0)) t1_lo
  have t2_0 : 0 ≤ (mul p.abs edet).val :=
    le_trans (mul_nonneg (by norm_num) (mul_nonneg (abs_nonneg _) hedet0)) t2_lo
  have t3_lo := lo_rel e3 (add_nonneg t1_0 t2_0)
  have hda' : det.abs.val = |det.val| := FP.abs_val _
  have e4 := sub_rel0 fda fedet ft4
  rw [hda'] at e4
  have t4_hi := hi_rel e4 hD0.le
  have t4_lo := lo_rel e4 hD0.le
  have t4_pos : 0 < (sub det.abs edet).val := lt_of_lt_of_le (mul_pos (by norm_num) hD0) t4_lo
  have t3_0 : 0 ≤ (add (mul GAMMA mnum) (mul p.abs edet)).val :=
    le_trans (mul_nonneg (by norm_num) (add_nonneg t1_0 t2_0)) t3_lo
  have e5 := div_rel0 ft3 ft4 t4_pos.ne' ft5 z5
  have t5_lo := lo_rel e5 (div_nonneg t3_0 t4_pos.le)
  have e6 := mul_rel0 fpa fG ft6 z6
  rw [hpa, hG] at e6
  have t6_lo := lo_rel e6 (mul_nonneg (abs_nonneg _) (by norm_num))
  have e7 := add_rel0 ft5 ft6 hfe
  have t5_0 : 0 ≤ (div (add (mul GAMMA mnum) (mul p.abs edet)) (sub det.abs edet)).val :=
    le_trans (mul_nonneg (by norm_num) (div_nonneg t3_0 t4_pos.le)) t5_lo
  have t6_0 : 0 ≤ (mul p.abs GAMMA).val := le_trans (mul_nonneg (by norm_num) (mul_nonneg (abs_nonneg _) (by norm_num))) t6_lo
  have e_lo := lo_rel e7 (add_nonneg t5_0 t6_0)
  -- `t3/t4 ≥ (1−ε)²/(1+ε) · R`
  set N := 1 / 2 ^ 46 * mnum.val + |p.val| * edet.val
  set D := |det.val| - edet.val
  have hN0 : 0 ≤ N := add_nonneg (mul_nonneg (by norm_num) hmn0) (mul_nonneg (abs_nonneg _) hedet0)
  have h1 : (1 - ε) ^ 2 * N ≤ (add (mul GAMMA mnum) (mul p.abs edet)).val := by
    simp only [N]; nlinarith
  have t34 : (1 - ε) ^ 2 / (1 + ε) * (N / D) ≤
      (add (mul GAMMA mnum) (mul p.abs edet)).val / (sub det.abs edet).val := by
    rw [div_mul_div_comm, div_le_div_iff₀ (by positivity) t4_pos]
    calc (1 - ε) ^ 2 * N * (sub det.abs edet).val
        ≤ (add (mul GAMMA mnum) (mul p.abs edet)).val * ((1 + ε) * D) :=
          mul_le_mul h1 t4_hi t4_pos.le t3_0
      _ = _ := by ring
  -- the error against `N/D` and `|p|·G`
  have hpq : |p.val - q| ≤ ε * ((1 + 2 * ε) * |p.val|) := le_trans ep (mul_le_mul_of_nonneg_left hq (by norm_num))
  have hq16 : |q| * (16 * ε * Perm) ≤ ((1 + 2 * ε) * |p.val|) * (edet.val / 7) :=
    mul_le_mul hq k1 (by positivity) (by positivity)
  have hnum : 16 * ε * Mn + |q| * (16 * ε * Perm) ≤ N / 6 := by
    simp only [N]; have := mul_nonneg (abs_nonneg p.val) hedet0; linarith
  have hqP' : |q - Num / Det| ≤ N / D / 6 := by
    refine le_trans hqP ?_
    rw [div_div, mul_comm D 6, ← div_div]
    exact div_le_div_of_nonneg_right hnum hD0.le
  have hND0 : 0 ≤ N / D := div_nonneg hN0 hD0.le
  have := abs_sub_le p.val q (Num / Det)
  have hp0 := abs_nonneg p.val
  have c1 : (1 - ε) ^ 2 / (1 + ε) = (9007199254740991 / 9007199254740992) ^ 2 / (9007199254740993 / 9007199254740992) := by
    norm_num
  rw [c1] at t34
  linarith

/-! ## The whole meet -/

/-- One coordinate: from the determinant's trackers and the `k`-th components of `b×c`, `c×a`,
`a×b`, the `k`-th numerator and its error term. -/
theorem coord_sound {det dabs : F64} {Det Perm : ℚ} (hd : Tr det Det Perm 16) (hda : Tr dabs Perm Perm 16)
    {oa ob oc x1 x2 x3 x4 y1 y2 y3 y4 w1 w2 w3 w4 : F64}
    {Oa Ob Oc X1 X2 X3 X4 Y1 Y2 Y3 Y4 W1 W2 W3 W4 : ℚ}
    (hoa : Tr oa Oa |Oa| 2) (hob : Tr ob Ob |Ob| 2) (hoc : Tr oc Oc |Oc| 2)
    (hx1 : Tr x1 X1 |X1| 2) (hx2 : Tr x2 X2 |X2| 2) (hx3 : Tr x3 X3 |X3| 2) (hx4 : Tr x4 X4 |X4| 2)
    (hy1 : Tr y1 Y1 |Y1| 2) (hy2 : Tr y2 Y2 |Y2| 2) (hy3 : Tr y3 Y3 |Y3| 2) (hy4 : Tr y4 Y4 |Y4| 2)
    (hw1 : Tr w1 W1 |W1| 2) (hw2 : Tr w2 W2 |W2| 2) (hw3 : Tr w3 W3 |W3| 2) (hw4 : Tr w4 W4 |W4| 2)
    (hsome : 4 * (mul GAMMA dabs).val < |det.val|)
    (hfe : (errTerm (div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det) det
      (mul GAMMA dabs) (dot3 oa.abs ob.abs oc.abs (cab x1 x2 x3 x4) (cab y1 y2 y3 y4) (cab w1 w2 w3 w4))).isFinite)
    (hfn : (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)).isFinite)
    (hfm : (dot3 oa.abs ob.abs oc.abs (cab x1 x2 x3 x4) (cab y1 y2 y3 y4) (cab w1 w2 w3 w4)).isFinite)
    (hU : ∀ r ∈ [x1.val * x4.val, x2.val * x3.val, y1.val * y4.val, y2.val * y3.val, w1.val * w4.val,
      w2.val * w3.val,
      oa.val * (crs x1 x2 x3 x4).val, ob.val * (crs y1 y2 y3 y4).val, oc.val * (crs w1 w2 w3 w4).val,
      oa.abs.val * (cab x1 x2 x3 x4).val, ob.abs.val * (cab y1 y2 y3 y4).val, oc.abs.val * (cab w1 w2 w3 w4).val,
      GAMMA.val * dabs.val,
      (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)).val / det.val,
      GAMMA.val * (dot3 oa.abs ob.abs oc.abs (cab x1 x2 x3 x4) (cab y1 y2 y3 y4) (cab w1 w2 w3 w4)).val,
      (div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det).abs.val * (mul GAMMA dabs).val,
      (add (mul GAMMA (dot3 oa.abs ob.abs oc.abs (cab x1 x2 x3 x4) (cab y1 y2 y3 y4) (cab w1 w2 w3 w4)))
        (mul (div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det).abs (mul GAMMA dabs))).val /
        (sub det.abs (mul GAMMA dabs)).val,
      (div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det).abs.val * GAMMA.val], NZ r) :
    Det ≠ 0 ∧
      |(div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det).val -
        (Oa * (X1 * X4 - X2 * X3) + Ob * (Y1 * Y4 - Y2 * Y3) + Oc * (W1 * W4 - W2 * W3)) / Det| ≤
      (errTerm (div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det) det
        (mul GAMMA dabs) (dot3 oa.abs ob.abs oc.abs (cab x1 x2 x3 x4) (cab y1 y2 y3 y4) (cab w1 w2 w3 w4))).val := by
  have u : ∀ i, (h : i < 18) → NZ ([x1.val * x4.val, x2.val * x3.val, y1.val * y4.val, y2.val * y3.val, w1.val * w4.val,
      w2.val * w3.val,
      oa.val * (crs x1 x2 x3 x4).val, ob.val * (crs y1 y2 y3 y4).val, oc.val * (crs w1 w2 w3 w4).val,
      oa.abs.val * (cab x1 x2 x3 x4).val, ob.abs.val * (cab y1 y2 y3 y4).val, oc.abs.val * (cab w1 w2 w3 w4).val,
      GAMMA.val * dabs.val,
      (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)).val / det.val,
      GAMMA.val * (dot3 oa.abs ob.abs oc.abs (cab x1 x2 x3 x4) (cab y1 y2 y3 y4) (cab w1 w2 w3 w4)).val,
      (div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det).abs.val * (mul GAMMA dabs).val,
      (add (mul GAMMA (dot3 oa.abs ob.abs oc.abs (cab x1 x2 x3 x4) (cab y1 y2 y3 y4) (cab w1 w2 w3 w4)))
        (mul (div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det).abs (mul GAMMA dabs))).val /
        (sub det.abs (mul GAMMA dabs)).val,
      (div (dot3 oa ob oc (crs x1 x2 x3 x4) (crs y1 y2 y3 y4) (crs w1 w2 w3 w4)) det).abs.val * GAMMA.val].get ⟨i, by simpa using h⟩) :=
    fun i h => hU _ (List.get_mem _ _)
  have fcx := (isFinite_of_mul (isFinite_of_add (isFinite_of_add hfn).1).1).2
  have fcy := (isFinite_of_mul (isFinite_of_add (isFinite_of_add hfn).1).2).2
  have fcw := (isFinite_of_mul (isFinite_of_add hfn).2).2
  have fax := (isFinite_of_mul (isFinite_of_add (isFinite_of_add hfm).1).1).2
  have fay := (isFinite_of_mul (isFinite_of_add (isFinite_of_add hfm).1).2).2
  have faw := (isFinite_of_mul (isFinite_of_add hfm).2).2
  have tx := tr_crs hx1 hx2 hx3 hx4 fcx (u 0 (by norm_num)) (u 1 (by norm_num))
  have ty := tr_crs hy1 hy2 hy3 hy4 fcy (u 2 (by norm_num)) (u 3 (by norm_num))
  have tw := tr_crs hw1 hw2 hw3 hw4 fcw (u 4 (by norm_num)) (u 5 (by norm_num))
  have ax := tr_cab hx1 hx2 hx3 hx4 fax (u 0 (by norm_num)) (u 1 (by norm_num))
  have ay := tr_cab hy1 hy2 hy3 hy4 fay (u 2 (by norm_num)) (u 3 (by norm_num))
  have aw := tr_cab hw1 hw2 hw3 hw4 faw (u 4 (by norm_num)) (u 5 (by norm_num))
  have tn := tr_dot3 hoa hob hoc tx ty tw hfn (u 6 (by norm_num)) (u 7 (by norm_num)) (u 8 (by norm_num))
  have tm := tr_dot3 hoa.abs hob.abs hoc.abs ax ay aw hfm (u 9 (by norm_num)) (u 10 (by norm_num)) (u 11 (by norm_num))
  rw [show |Oa| * (|X1| * |X4| + |X2| * |X3|) + |Ob| * (|Y1| * |Y4| + |Y2| * |Y3|) + |Oc| * (|W1| * |W4| + |W2| * |W3|) =
    |Oa| * (|X1| * |X4| + |X2| * |X3|) + |Ob| * (|Y1| * |Y4| + |Y2| * |Y3|) + |Oc| * (|W1| * |W4| + |W2| * |W3|) from rfl] at tn
  exact quot_sound hd hda tn tm hfe hsome (u 12 (by norm_num)) (u 13 (by norm_num)) (u 14 (by norm_num))
    (u 15 (by norm_num)) (u 16 (by norm_num)) (u 17 (by norm_num))

/-- Cramer: with `Det ≠ 0`, `P_k = Num_k/Det` lies on all three exact planes. -/
lemma cramer {A0 A1 A2 B0 B1 B2 C0 C1 C2 Oa Ob Oc : ℚ}
    (hD : A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0) ≠ 0) :
    let D := A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0)
    let P0 := (Oa * (B1 * C2 - B2 * C1) + Ob * (C1 * A2 - C2 * A1) + Oc * (A1 * B2 - A2 * B1)) / D
    let P1 := (Oa * (B2 * C0 - B0 * C2) + Ob * (C2 * A0 - C0 * A2) + Oc * (A2 * B0 - A0 * B2)) / D
    let P2 := (Oa * (B0 * C1 - B1 * C0) + Ob * (C0 * A1 - C1 * A0) + Oc * (A0 * B1 - A1 * B0)) / D
    A0 * P0 + A1 * P1 + A2 * P2 = Oa ∧ B0 * P0 + B1 * P1 + B2 * P2 = Ob ∧ C0 * P0 + C1 * P1 + C2 * P2 = Oc := by
  intro D P0 P1 P2
  have hD' : D ≠ 0 := hD
  have fin : ∀ (u0 u1 u2 n0 n1 n2 o : ℚ), u0 * n0 + u1 * n1 + u2 * n2 = o * D →
      u0 * (n0 / D) + u1 * (n1 / D) + u2 * (n2 / D) = o := by
    intro u0 u1 u2 n0 n1 n2 o h
    rw [← mul_div_assoc, ← mul_div_assoc, ← mul_div_assoc, ← add_div, ← add_div, h,
      mul_div_assoc, div_self hD', _root_.mul_one]
  refine ⟨fin _ _ _ _ _ _ _ ?_, fin _ _ _ _ _ _ _ ?_, fin _ _ _ _ _ _ _ ?_⟩ <;> (simp only [D]; ring)

lemma det_track {a0 a1 a2 b0 b1 b2 c0 c1 c2 : F64} {A0 A1 A2 B0 B1 B2 C0 C1 C2 : ℚ}
    (ha0 : Tr a0 A0 |A0| 2)
    (ha1 : Tr a1 A1 |A1| 2)
    (ha2 : Tr a2 A2 |A2| 2)
    (hb0 : Tr b0 B0 |B0| 2)
    (hb1 : Tr b1 B1 |B1| 2)
    (hb2 : Tr b2 B2 |B2| 2)
    (hc0 : Tr c0 C0 |C0| 2)
    (hc1 : Tr c1 C1 |C1| 2)
    (hc2 : Tr c2 C2 |C2| 2)
    (hfd : (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).isFinite) (hfa : (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)).isFinite)
    (hU : ∀ r ∈ [b1.val * c2.val, b2.val * c1.val, b2.val * c0.val, b0.val * c2.val, b0.val * c1.val, b1.val * c0.val, a0.val * (crs b1 b2 c1 c2).val, a1.val * (crs b2 b0 c2 c0).val, a2.val * (crs b0 b1 c0 c1).val, a0.abs.val * (cab b1 b2 c1 c2).val, a1.abs.val * (cab b2 b0 c2 c0).val, a2.abs.val * (cab b0 b1 c0 c1).val], NZ r) :
    Tr (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0)) (|A0| * (|B1| * |C2| + |B2| * |C1|) + |A1| * (|B2| * |C0| + |B0| * |C2|) + |A2| * (|B0| * |C1| + |B1| * |C0|)) 16 ∧ Tr (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)) (|A0| * (|B1| * |C2| + |B2| * |C1|) + |A1| * (|B2| * |C0| + |B0| * |C2|) + |A2| * (|B0| * |C1| + |B1| * |C0|)) (|A0| * (|B1| * |C2| + |B2| * |C1|) + |A1| * (|B2| * |C0| + |B0| * |C2|) + |A2| * (|B0| * |C1| + |B1| * |C0|)) 16 := by
  have u : ∀ i, (h : i < 12) → NZ ([b1.val * c2.val, b2.val * c1.val, b2.val * c0.val, b0.val * c2.val, b0.val * c1.val, b1.val * c0.val, a0.val * (crs b1 b2 c1 c2).val, a1.val * (crs b2 b0 c2 c0).val, a2.val * (crs b0 b1 c0 c1).val, a0.abs.val * (cab b1 b2 c1 c2).val, a1.abs.val * (cab b2 b0 c2 c0).val, a2.abs.val * (cab b0 b1 c0 c1).val].get ⟨i, by simpa using h⟩) := fun i h => hU _ (List.get_mem _ _)
  have f0 := (isFinite_of_mul (isFinite_of_add (isFinite_of_add hfd).1).1).2
  have f1 := (isFinite_of_mul (isFinite_of_add (isFinite_of_add hfd).1).2).2
  have f2 := (isFinite_of_mul (isFinite_of_add hfd).2).2
  have g0 := (isFinite_of_mul (isFinite_of_add (isFinite_of_add hfa).1).1).2
  have g1 := (isFinite_of_mul (isFinite_of_add (isFinite_of_add hfa).1).2).2
  have g2 := (isFinite_of_mul (isFinite_of_add hfa).2).2
  have t0 := tr_crs hb1 hb2 hc1 hc2 f0 (u 0 (by norm_num)) (u 1 (by norm_num))
  have t1 := tr_crs hb2 hb0 hc2 hc0 f1 (u 2 (by norm_num)) (u 3 (by norm_num))
  have t2 := tr_crs hb0 hb1 hc0 hc1 f2 (u 4 (by norm_num)) (u 5 (by norm_num))
  have s0 := tr_cab hb1 hb2 hc1 hc2 g0 (u 0 (by norm_num)) (u 1 (by norm_num))
  have s1 := tr_cab hb2 hb0 hc2 hc0 g1 (u 2 (by norm_num)) (u 3 (by norm_num))
  have s2 := tr_cab hb0 hb1 hc0 hc1 g2 (u 4 (by norm_num)) (u 5 (by norm_num))
  refine ⟨tr_dot3 ha0 ha1 ha2 t0 t1 t2 hfd (u 6 (by norm_num)) (u 7 (by norm_num)) (u 8 (by norm_num)), ?_⟩
  exact tr_dot3 ha0.abs ha1.abs ha2.abs s0 s1 s2 hfa (u 9 (by norm_num)) (u 10 (by norm_num)) (u 11 (by norm_num))

/-- **Item 1, `meet_f`.** The planes `(a, oa)`, `(b, ob)`, `(c, oc)` are f64 shadows, each
coefficient within `2ε` of the exact plane's. Suppose `|det| > 4·edet` (the `Some` branch) and
every product and quotient formed is zero or normal. Then the exact triple product `Det ≠ 0`, and
each computed `p_k` is within its error term `e_k` of `P_k = Num_k/Det`, the exact meet: it lies
on all three exact planes (`cramer`). `err = max_k e_k` bounds them all. -/
theorem meet_sound {a0 a1 a2 b0 b1 b2 c0 c1 c2 oa ob oc : F64} {A0 A1 A2 B0 B1 B2 C0 C1 C2 Oa Ob Oc : ℚ}
    (ha0 : Tr a0 A0 |A0| 2)
    (ha1 : Tr a1 A1 |A1| 2)
    (ha2 : Tr a2 A2 |A2| 2)
    (hb0 : Tr b0 B0 |B0| 2)
    (hb1 : Tr b1 B1 |B1| 2)
    (hb2 : Tr b2 B2 |B2| 2)
    (hc0 : Tr c0 C0 |C0| 2)
    (hc1 : Tr c1 C1 |C1| 2)
    (hc2 : Tr c2 C2 |C2| 2)
    (hoa : Tr oa Oa |Oa| 2) (hob : Tr ob Ob |Ob| 2) (hoc : Tr oc Oc |Oc| 2)
    (hfd : (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).isFinite) (hfa : (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)).isFinite)
    (hsome : 4 * (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))).val < |(dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).val|)
    (hfn0 : (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)).isFinite) (hfn1 : (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)).isFinite) (hfn2 : (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)).isFinite)
    (hfm0 : (dot3 oa.abs ob.abs oc.abs (cab b1 b2 c1 c2) (cab c1 c2 a1 a2) (cab a1 a2 b1 b2)).isFinite) (hfm1 : (dot3 oa.abs ob.abs oc.abs (cab b2 b0 c2 c0) (cab c2 c0 a2 a0) (cab a2 a0 b2 b0)).isFinite) (hfm2 : (dot3 oa.abs ob.abs oc.abs (cab b0 b1 c0 c1) (cab c0 c1 a0 a1) (cab a0 a1 b0 b1)).isFinite)
    (hfe0 : (errTerm (div (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b1 b2 c1 c2) (cab c1 c2 a1 a2) (cab a1 a2 b1 b2))).isFinite) (hfe1 : (errTerm (div (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b2 b0 c2 c0) (cab c2 c0 a2 a0) (cab a2 a0 b2 b0))).isFinite) (hfe2 : (errTerm (div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b0 b1 c0 c1) (cab c0 c1 a0 a1) (cab a0 a1 b0 b1))).isFinite)
    (hUd : ∀ r ∈ [b1.val * c2.val, b2.val * c1.val, b2.val * c0.val, b0.val * c2.val, b0.val * c1.val, b1.val * c0.val, a0.val * (crs b1 b2 c1 c2).val, a1.val * (crs b2 b0 c2 c0).val, a2.val * (crs b0 b1 c0 c1).val, a0.abs.val * (cab b1 b2 c1 c2).val, a1.abs.val * (cab b2 b0 c2 c0).val, a2.abs.val * (cab b0 b1 c0 c1).val], NZ r)
    (hU0 : ∀ r ∈ [b1.val * c2.val,
      b2.val * c1.val,
      c1.val * a2.val,
      c2.val * a1.val,
      a1.val * b2.val,
      a2.val * b1.val,
      oa.val * (crs b1 b2 c1 c2).val,
      ob.val * (crs c1 c2 a1 a2).val,
      oc.val * (crs a1 a2 b1 b2).val,
      oa.abs.val * (cab b1 b2 c1 c2).val,
      ob.abs.val * (cab c1 c2 a1 a2).val,
      oc.abs.val * (cab a1 a2 b1 b2).val,
      GAMMA.val * (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)).val,
      (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)).val / (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).val,
      GAMMA.val * (dot3 oa.abs ob.abs oc.abs (cab b1 b2 c1 c2) (cab c1 c2 a1 a2) (cab a1 a2 b1 b2)).val,
      (div (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs.val * (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))).val,
      (add (mul GAMMA (dot3 oa.abs ob.abs oc.abs (cab b1 b2 c1 c2) (cab c1 c2 a1 a2) (cab a1 a2 b1 b2))) (mul (div (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))))).val / (sub (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).abs (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)))).val,
      (div (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs.val * GAMMA.val], NZ r)
    (hU1 : ∀ r ∈ [b2.val * c0.val,
      b0.val * c2.val,
      c2.val * a0.val,
      c0.val * a2.val,
      a2.val * b0.val,
      a0.val * b2.val,
      oa.val * (crs b2 b0 c2 c0).val,
      ob.val * (crs c2 c0 a2 a0).val,
      oc.val * (crs a2 a0 b2 b0).val,
      oa.abs.val * (cab b2 b0 c2 c0).val,
      ob.abs.val * (cab c2 c0 a2 a0).val,
      oc.abs.val * (cab a2 a0 b2 b0).val,
      GAMMA.val * (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)).val,
      (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)).val / (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).val,
      GAMMA.val * (dot3 oa.abs ob.abs oc.abs (cab b2 b0 c2 c0) (cab c2 c0 a2 a0) (cab a2 a0 b2 b0)).val,
      (div (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs.val * (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))).val,
      (add (mul GAMMA (dot3 oa.abs ob.abs oc.abs (cab b2 b0 c2 c0) (cab c2 c0 a2 a0) (cab a2 a0 b2 b0))) (mul (div (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))))).val / (sub (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).abs (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)))).val,
      (div (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs.val * GAMMA.val], NZ r)
    (hU2 : ∀ r ∈ [b0.val * c1.val,
      b1.val * c0.val,
      c0.val * a1.val,
      c1.val * a0.val,
      a0.val * b1.val,
      a1.val * b0.val,
      oa.val * (crs b0 b1 c0 c1).val,
      ob.val * (crs c0 c1 a0 a1).val,
      oc.val * (crs a0 a1 b0 b1).val,
      oa.abs.val * (cab b0 b1 c0 c1).val,
      ob.abs.val * (cab c0 c1 a0 a1).val,
      oc.abs.val * (cab a0 a1 b0 b1).val,
      GAMMA.val * (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)).val,
      (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)).val / (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).val,
      GAMMA.val * (dot3 oa.abs ob.abs oc.abs (cab b0 b1 c0 c1) (cab c0 c1 a0 a1) (cab a0 a1 b0 b1)).val,
      (div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs.val * (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))).val,
      (add (mul GAMMA (dot3 oa.abs ob.abs oc.abs (cab b0 b1 c0 c1) (cab c0 c1 a0 a1) (cab a0 a1 b0 b1))) (mul (div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))))).val / (sub (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)).abs (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)))).val,
      (div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs.val * GAMMA.val], NZ r) :
    (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0)) ≠ 0 ∧
      |(div (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).val - (Oa * (B1 * C2 - B2 * C1) + Ob * (C1 * A2 - C2 * A1) + Oc * (A1 * B2 - A2 * B1)) / (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0))| ≤ (errTerm (div (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b1 b2 c1 c2) (cab c1 c2 a1 a2) (cab a1 a2 b1 b2))).val ∧
      |(div (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).val - (Oa * (B2 * C0 - B0 * C2) + Ob * (C2 * A0 - C0 * A2) + Oc * (A2 * B0 - A0 * B2)) / (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0))| ≤ (errTerm (div (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b2 b0 c2 c0) (cab c2 c0 a2 a0) (cab a2 a0 b2 b0))).val ∧
      |(div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).val - (Oa * (B0 * C1 - B1 * C0) + Ob * (C0 * A1 - C1 * A0) + Oc * (A0 * B1 - A1 * B0)) / (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0))| ≤ (errTerm (div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b0 b1 c0 c1) (cab c0 c1 a0 a1) (cab a0 a1 b0 b1))).val := by
  obtain ⟨hd, hda⟩ := det_track ha0 ha1 ha2 hb0 hb1 hb2 hc0 hc1 hc2 hfd hfa hUd
  obtain ⟨hD, e0⟩ := coord_sound hd hda hoa hob hoc hb1 hb2 hc1 hc2 hc1 hc2 ha1 ha2 ha1 ha2 hb1 hb2 hsome hfe0 hfn0 hfm0 hU0
  obtain ⟨-, e1⟩ := coord_sound hd hda hoa hob hoc hb2 hb0 hc2 hc0 hc2 hc0 ha2 ha0 ha2 ha0 hb2 hb0 hsome hfe1 hfn1 hfm1 hU1
  obtain ⟨-, e2⟩ := coord_sound hd hda hoa hob hoc hb0 hb1 hc0 hc1 hc0 hc1 ha0 ha1 ha0 ha1 hb0 hb1 hsome hfe2 hfn2 hfm2 hU2
  exact ⟨hD, e0, e1, e2⟩

end IEEE754.Screen
