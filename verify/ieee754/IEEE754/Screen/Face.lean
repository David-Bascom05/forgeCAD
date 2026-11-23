/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Cross
import IEEE754.Screen.CrossScale
import IEEE754.Screen.Apart

/-!
# decide-once A8's `face_fast` screens (f885b18)

Every point carries an f64 triple `f` and a bound `e`, with `|f_k − P_k| ≤ e` on each axis for the
exact point `P`.

- `axis_sound` (`cmp_axis`, and `same`'s apart test): `|f_i,a − f_j,a| > (e_i + e_j)(1 + 1e-12) +
  slack` in f64 gives `P_i,a ≠ P_j,a`, with the f64 difference's sign.
- `midpoint_ok`: `mid_in`'s `q = 0.5(f_a + f_b)` keeps `crosses_f_sound`'s hypothesis.
- `crosses_line_sound` (`crosses_line`): `|l.f·n| > 1e-13·(l.ab·|n|)` gives the exact `d·n ≠ 0`, for a
  line direction `l.f` and magnitude `l.ab` tracked as `line_tr` gives them.
- `along_sound` (`along`'s f64 test): `|v| > b`, `v = Σ (f_i,k − f_j,k)·l.f_k`, `b` the Rust's bound,
  gives the exact `(P_i − P_j)·d ≠ 0` with `v`'s sign.
- `orient_sound` (`orient`'s f64 test): `|bs·ct − bt·cs| > e·Σ|·| + e·e + 1e-14·(|bs·ct| + |bt·cs|)`
  gives the exact 2D orientation `≠ 0` with the f64 sign (`orient_e`: the Rust's `e` is at least
  `2(1 − 6ε)(e_a + max(e_b, e_c))`, twice what is needed).
- `pt_diff_close`, `esum_le`: the point bound `|f − P| ≤ e` gives the screens' per-axis hypotheses.
-/

namespace IEEE754.Screen

open FP

/-- `1.0 + 1e-12` as the Rust folds it. -/
def G12 : F64 := add F1 E12

lemma G12_lo (h : G12.isFinite) : (1 + 99 / 100 * (1 / 10 ^ 12) : ℚ) ≤ G12.val := by
  have fO : F1.isFinite := by decide
  have fE : E12.isFinite := by decide
  have e := add_rel0 fO fE h
  rw [F1_val, E12_val, abs_of_nonneg (by norm_num : (0 : ℚ) ≤ 1 + 4951760157141521 / 2 ^ 92)] at e
  have := (abs_le.mp e).1
  unfold G12; nlinarith

set_option maxHeartbeats 1000000 in
/-- **`cmp_axis` / `same`'s apart test.** -/
theorem axis_sound {fi fj ei ej sl : F64} {Pi Pj : ℚ}
    (hfi : fi.isFinite) (hfj : fj.isFinite) (hei : ei.isFinite) (hej : ej.isFinite) (hsl : sl.isFinite)
    (ei0 : 0 ≤ ei.val) (ej0 : 0 ≤ ej.val) (sl0 : 0 ≤ sl.val)
    (ehi : |fi.val - Pi| ≤ ei.val) (ehj : |fj.val - Pj| ≤ ej.val)
    (hfd : (sub fi fj).isFinite) (hft : (add (mul (add ei ej) G12) sl).isFinite)
    (hz : NZ ((add ei ej).val * G12.val))
    (hdec : (add (mul (add ei ej) G12) sl).val < |(sub fi fj).val|) :
    Pi - Pj ≠ 0 ∧ (0 < Pi - Pj ↔ 0 < (sub fi fj).val) := by
  have fm := (isFinite_of_add hft).1
  have fe := (isFinite_of_mul fm).1
  have fG := (isFinite_of_mul fm).2
  have gl := G12_lo fG
  have ee := add_rel0 hei hej fe
  have em := mul_rel0 fe fG fm hz
  have et := add_rel0 fm hsl hft
  have ed := sub_rel0 hfi hfj hfd
  set E := ei.val + ej.val
  set d := (sub fi fj).val
  set s := (add ei ej).val
  set m := (mul (add ei ej) G12).val
  set t := (add (mul (add ei ej) G12) sl).val
  have hE0 : 0 ≤ E := by positivity
  rw [abs_of_nonneg hE0] at ee
  have hs : (1 - ε) * E ≤ s := by have := (abs_le.mp ee).1; linarith
  have hs0 : 0 ≤ s := le_trans (by nlinarith) hs
  have hG0 : (0 : ℚ) ≤ G12.val := by linarith
  rw [abs_of_nonneg (mul_nonneg hs0 hG0)] at em
  have hm : (1 - ε) * (s * G12.val) ≤ m := by have := (abs_le.mp em).1; linarith
  have hm0 : 0 ≤ m := le_trans (by positivity) hm
  rw [abs_of_nonneg (add_nonneg hm0 sl0)] at et
  have ht : (1 - ε) * (m + sl.val) ≤ t := by have := (abs_le.mp et).1; linarith
  -- `t ≥ (1 + 0.9·10^-12)·E`
  have h1 : (1 - ε) * E * (1 + 99 / 100 * (1 / 10 ^ 12)) ≤ s * G12.val :=
    mul_le_mul hs gl (by norm_num) hs0
  have htE : (1 + 9 / 10 * (1 / 10 ^ 12)) * E ≤ t := by nlinarith
  -- `|d − D| < |d|`
  have hdD : |d - (Pi - Pj)| ≤ E + ε * |fi.val - fj.val| := by
    have := abs_sub_le d (fi.val - fj.val) (Pi - Pj)
    have : |fi.val - fj.val - (Pi - Pj)| ≤ E := by
      rw [show fi.val - fj.val - (Pi - Pj) = (fi.val - Pi) - (fj.val - Pj) by ring]
      exact le_trans (abs_sub _ _) (by linarith)
    linarith
  have hff : |fi.val - fj.val| ≤ |d| + ε * |fi.val - fj.val| := by
    have h1 := abs_sub_abs_le_abs_sub (fi.val - fj.val) d
    have h2 : |(fi.val - fj.val) - d| = |d - (fi.val - fj.val)| := abs_sub_comm _ _
    linarith
  have hff' : |fi.val - fj.val| ≤ (1 + 2 * ε) * |d| := by
    have : (1 - ε) * |fi.val - fj.val| ≤ |d| := by linarith
    nlinarith [abs_nonneg d, abs_nonneg (fi.val - fj.val)]
  have hlt : |d - (Pi - Pj)| < |d| := by
    have hd0 : 0 < |d| := by nlinarith
    nlinarith [abs_nonneg d]
  exact sign_close_d hlt

/-- **`mid_in`'s midpoint.** `q = fl(fl(f_a + f_b)·0.5)` is within `(e_a + e_b)/2 + ε·|f_a + f_b|/2`
of the exact midpoint (`·0.5` is exact); with `e ≤ 1.0000001e-13·(1 + M)` and `|f| ≤ M`, that is
under `2·10^-13·scale` once `(1 − ε)(1 + M) ≤ scale` (`scale_facts`). -/
theorem midpoint_ok {ea eb M S fa fb : ℚ} (hea : ea ≤ 10000001 / 10 ^ 20 * (1 + M))
    (heb : eb ≤ 10000001 / 10 ^ 20 * (1 + M)) (hfa : |fa| ≤ M) (hfb : |fb| ≤ M)
    (hS : (1 - ε) * (1 + M) ≤ S) :
    (ea + eb) / 2 + ε * |fa + fb| / 2 ≤ 2 / 10 ^ 13 * S := by
  have hM : 0 ≤ M := le_trans (abs_nonneg _) hfa
  have : |fa + fb| ≤ 2 * M := le_trans (abs_add_le _ _) (by linarith)
  nlinarith

/-! ## A class line's direction, and `crosses_line` -/

/-- `l.f`, `l.ab` from exact (f64) normals `x`, `y`: `l.f_k` is within `8ε·A_k` of the exact
`d_k = (x × y)_k`, and `l.ab_k` within `8ε·A_k` of `A_k = |x_i y_j| + |x_j y_i|`. -/
lemma line_tr {x1 x2 y1 y2 : F64} (h1 : x1.isFinite) (h2 : x2.isFinite) (g1 : y1.isFinite) (g2 : y2.isFinite)
    (hf : (crs x1 x2 y1 y2).isFinite) (ha : (cab x1 x2 y1 y2).isFinite)
    (z1 : NZ (x1.val * y2.val)) (z2 : NZ (x2.val * y1.val)) :
    Tr (crs x1 x2 y1 y2) (x1.val * y2.val - x2.val * y1.val) (|x1.val| * |y2.val| + |x2.val| * |y1.val|) 8 ∧
      Tr (cab x1 x2 y1 y2) (|x1.val| * |y2.val| + |x2.val| * |y1.val|) (|x1.val| * |y2.val| + |x2.val| * |y1.val|) 8 := by
  have w : ∀ {v : F64}, v.isFinite → Tr v v.val |v.val| 2 :=
    fun hv => (Tr.exact hv).weaken (by norm_num) (abs_nonneg _)
  exact ⟨tr_crs (w h1) (w h2) (w g1) (w g2) hf z1 z2, tr_cab (w h1) (w h2) (w g1) (w g2) ha z1 z2⟩

set_option maxHeartbeats 1000000 in
/-- **`crosses_line`.** If `|l.f·n| > 1e-13·Σ l.ab_k |n_k|` in f64, then `d·n ≠ 0` exactly: class
`h` meets the line in one point. -/
theorem crosses_line_sound {f0 f1 f2 a0 a1 a2 n0 n1 n2 : F64} {d0 d1 d2 A0 A1 A2 : ℚ}
    (tf0 : Tr f0 d0 A0 8) (tf1 : Tr f1 d1 A1 8) (tf2 : Tr f2 d2 A2 8)
    (ta0 : Tr a0 A0 A0 8) (ta1 : Tr a1 A1 A1 8) (ta2 : Tr a2 A2 A2 8)
    (hn0 : n0.isFinite) (hn1 : n1.isFinite) (hn2 : n2.isFinite)
    (hfv : (dot3 f0 f1 f2 n0 n1 n2).isFinite) (hfb : (mul E13 (dot3 a0 a1 a2 n0.abs n1.abs n2.abs)).isFinite)
    (hU : ∀ r ∈ [f0.val * n0.val, f1.val * n1.val, f2.val * n2.val, a0.val * n0.abs.val, a1.val * n1.abs.val,
      a2.val * n2.abs.val, E13.val * (dot3 a0 a1 a2 n0.abs n1.abs n2.abs).val], NZ r)
    (hdec : (mul E13 (dot3 a0 a1 a2 n0.abs n1.abs n2.abs)).val < |(dot3 f0 f1 f2 n0 n1 n2).val|) :
    d0 * n0.val + d1 * n1.val + d2 * n2.val ≠ 0 := by
  have u : ∀ i, (h : i < 7) → NZ ([f0.val * n0.val, f1.val * n1.val, f2.val * n2.val, a0.val * n0.abs.val,
      a1.val * n1.abs.val, a2.val * n2.abs.val, E13.val * (dot3 a0 a1 a2 n0.abs n1.abs n2.abs).val].get ⟨i, by simpa using h⟩) :=
    fun i h => hU _ (List.get_mem _ _)
  have en0 := Tr.exact hn0; have en1 := Tr.exact hn1; have en2 := Tr.exact hn2
  -- `v`
  unfold dot3 at hfv
  have fv2 := (isFinite_of_add hfv).1
  have mv0 := tr_mul tf0 en0 (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_add fv2).1 (u 0 (by norm_num))
  have mv1 := tr_mul tf1 en1 (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_add fv2).2 (u 1 (by norm_num))
  have mv2 := tr_mul tf2 en2 (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_add hfv).2 (u 2 (by norm_num))
  norm_num at mv0 mv1 mv2
  have sv1 := tr_add mv0 mv1 (by norm_num) (by norm_num) fv2
  norm_num at sv1
  have sv := tr_add sv1 (mv2.weaken (show (10 : ℚ) ≤ 12 by norm_num) mv2.M_nonneg) (by norm_num) (by norm_num) hfv
  norm_num at sv
  -- `b`, from below
  have fbs := (isFinite_of_mul hfb).2
  unfold dot3 at fbs hfb
  have fb2 := (isFinite_of_add fbs).1
  have mb0 := tr_mul ta0 en0.abs (by norm_num) (by norm_num) (by norm_num) (by norm_num) (by simpa using (isFinite_of_add fb2).1) (u 3 (by norm_num))
  have mb1 := tr_mul ta1 en1.abs (by norm_num) (by norm_num) (by norm_num) (by norm_num) (by simpa using (isFinite_of_add fb2).2) (u 4 (by norm_num))
  have mb2 := tr_mul ta2 en2.abs (by norm_num) (by norm_num) (by norm_num) (by norm_num) (by simpa using (isFinite_of_add fbs).2) (u 5 (by norm_num))
  norm_num at mb0 mb1 mb2
  have sb1 := tr_add mb0 mb1 (by norm_num) (by norm_num) fb2
  norm_num at sb1
  have sb := tr_add sb1 (mb2.weaken (show (10 : ℚ) ≤ 12 by norm_num) mb2.M_nonneg) (by norm_num) (by norm_num) fbs
  norm_num at sb
  have fE : E13.isFinite := by decide
  have mb := tr_mul (Tr.exact fE) sb (by norm_num) (by norm_num) (by norm_num) (by norm_num) hfb (by simpa [dot3] using u 6 (by norm_num))
  norm_num at mb
  rw [E13_val] at mb
  norm_num at mb
  have hlow := mb.lower
  have hM0 := sb.M_nonneg
  set Mb := A0 * |n0.val| + A1 * |n1.val| + A2 * |n2.val|
  have hve := sv.err
  -- the error is inside the bound
  have hlt : |(add (add (mul f0 n0) (mul f1 n1)) (mul f2 n2)).val - (d0 * n0.val + d1 * n1.val + d2 * n2.val)| <
      |(add (add (mul f0 n0) (mul f1 n1)) (mul f2 n2)).val| := by
    have : 14 * ε * Mb ≤ (1 - 16 * ε) * (3961408125713217 / 2 ^ 95 * Mb) := by nlinarith
    unfold dot3 at hdec
    linarith
  exact (sign_close_d hlt).1

/-! ## `along` -/

/-- One component of `along`'s error, expanded. -/
lemma along_term {D lf ab d A Dx E : ℚ} (hd : |lf - d| ≤ 8 * ε * A) (hA : (1 - 8 * ε) * A ≤ ab)
    (hdA : |d| ≤ A) (hD : |D - Dx| ≤ E + 2 * ε * |D|) (hE : 0 ≤ E) :
    |D * lf - Dx * d| ≤ 9 * ε * (|D| * ab) + E * |lf| + 9 * ε * (E * ab) + 3 * ε * (|D| * |lf|) + 9 * ε * (|D| * ab) ∧
      |lf| ≤ (1 + 18 * ε) * ab := by
  have hA0 : 0 ≤ A := le_trans (abs_nonneg _) hdA
  have hAab : A ≤ (1 + 9 * ε) * ab := by nlinarith
  have hab0 : 0 ≤ ab := by nlinarith
  have hlf : |lf| ≤ (1 + 18 * ε) * ab := by
    have := abs_sub_abs_le_abs_sub lf d
    have : |lf| ≤ A + 8 * ε * A := by linarith
    nlinarith
  refine ⟨?_, hlf⟩
  have hdl : |d| ≤ |lf| + 8 * ε * A := by
    have := abs_sub_abs_le_abs_sub d lf; rw [abs_sub_comm] at this; linarith
  rw [show D * lf - Dx * d = D * (lf - d) + d * (D - Dx) by ring]
  refine le_trans (abs_add_le _ _) ?_
  rw [abs_mul, abs_mul]
  have h1 : |D| * |lf - d| ≤ |D| * (8 * ε * A) := mul_le_mul_of_nonneg_left hd (abs_nonneg _)
  have h2 : |d| * |D - Dx| ≤ (|lf| + 8 * ε * A) * (E + 2 * ε * |D|) :=
    mul_le_mul hdl hD (abs_nonneg _) (by positivity)
  have h3 : |D| * (8 * ε * A) ≤ 9 * ε * (|D| * ab) := by
    have := mul_le_mul_of_nonneg_left hAab (abs_nonneg D); nlinarith [abs_nonneg D]
  have h4 : (|lf| + 8 * ε * A) * (E + 2 * ε * |D|) ≤
      E * |lf| + 9 * ε * (E * ab) + 3 * ε * (|D| * |lf|) + 9 * ε * (|D| * ab) := by
    have hA' : 8 * ε * A ≤ 17 / 2 * ε * ab := by nlinarith
    have hD0 := abs_nonneg D
    have hl0 := abs_nonneg lf
    nlinarith [mul_nonneg hE hab0, mul_nonneg hD0 hab0, mul_nonneg hD0 hl0, mul_nonneg hE hl0,
      mul_le_mul_of_nonneg_left hA' hE, mul_le_mul_of_nonneg_left hA' hD0]
  linarith

/-- A nonnegative f64 input, tracked exactly. -/
lemma Tr.exact_nn {v : F64} (hv : v.isFinite) (h0 : 0 ≤ v.val) (k : ℚ) (hk : 0 ≤ k) : Tr v v.val v.val k :=
  ⟨hv, by simp only [_root_.sub_self, abs_zero]; positivity, by rw [abs_of_nonneg h0]⟩

/-- `along`'s term `(e_i + e_j)·(|lf| + 1e-13·ab) + 1e-13·|dk|·(|lf| + ab)`, tracked. -/
def alongT (E lf ab dk : F64) : F64 :=
  add (mul E (add lf.abs (mul E13 ab))) (mul (mul E13 dk.abs) (add lf.abs ab))

set_option maxHeartbeats 2000000 in
lemma alongT_tr {E lf ab dk : F64} (hE : E.isFinite) (hE0 : 0 ≤ E.val) (hab0 : 0 ≤ ab.val)
    (hf : (alongT E lf ab dk).isFinite)
    (z1 : NZ (E.val * (add lf.abs (mul E13 ab)).val)) (z2 : NZ (E13.val * ab.val))
    (z3 : NZ (E13.val * dk.abs.val)) (z4 : NZ ((mul E13 dk.abs).val * (add lf.abs ab).val)) :
    Tr (alongT E lf ab dk) (E.val * (|lf.val| + 3961408125713217 / 2 ^ 95 * ab.val) +
        3961408125713217 / 2 ^ 95 * |dk.val| * (|lf.val| + ab.val))
      (E.val * (|lf.val| + 3961408125713217 / 2 ^ 95 * ab.val) +
        3961408125713217 / 2 ^ 95 * |dk.val| * (|lf.val| + ab.val)) 10 := by
  unfold alongT at hf ⊢
  have f1 := (isFinite_of_add hf).1
  have f2 := (isFinite_of_add hf).2
  have fs1 := (isFinite_of_mul f1).2
  have fca := (isFinite_of_add fs1).2
  have fla : lf.abs.isFinite := (isFinite_of_add fs1).1
  have fab := (isFinite_of_mul fca).2
  have fcd := (isFinite_of_mul f2).1
  have fs2 := (isFinite_of_mul f2).2
  have fda := (isFinite_of_mul fcd).2
  have fE13 : E13.isFinite := by decide
  have c13 : E13.val = 3961408125713217 / 2 ^ 95 := E13_val
  have c0 : (0 : ℚ) ≤ E13.val := by rw [c13]; norm_num
  have tc := Tr.exact_nn fE13 c0 0 le_rfl
  have tab := Tr.exact_nn fab hab0 0 le_rfl
  have tla := Tr.exact_nn fla (by rw [FP.abs_val]; exact abs_nonneg _) 0 le_rfl
  have tda := Tr.exact_nn fda (by rw [FP.abs_val]; exact abs_nonneg _) 0 le_rfl
  have tE := Tr.exact_nn hE hE0 2 (by norm_num)
  -- `|lf| + 1e-13·ab`
  have mca := tr_mul tc tab (by norm_num) (by norm_num) (by norm_num) (by norm_num) fca z2
  norm_num at mca
  have s1 := tr_add (tla.weaken (show (0 : ℚ) ≤ 2 by norm_num) tla.M_nonneg) mca (by norm_num) (by norm_num) fs1
  norm_num at s1
  have m1 := tr_mul tE s1 (by norm_num) (by norm_num) (by norm_num) (by norm_num) f1 z1
  norm_num at m1
  -- `1e-13·|dk|·(|lf| + ab)`
  have mcd := tr_mul tc tda (by norm_num) (by norm_num) (by norm_num) (by norm_num) fcd z3
  norm_num at mcd
  have s2 := tr_add tla tab (by norm_num) (by norm_num) fs2
  norm_num at s2
  have m2 := tr_mul mcd s2 (by norm_num) (by norm_num) (by norm_num) (by norm_num) f2 z4
  norm_num at m2
  have t := tr_add m1 (m2.weaken (show (6 : ℚ) ≤ 8 by norm_num) m2.M_nonneg) (by norm_num) (by norm_num) hf
  norm_num at t
  rw [FP.abs_val, FP.abs_val, c13] at t
  convert t using 2

/-- One axis of `along`: its error and the tracker's fit inside its share of `b`. -/
lemma along_comp {D lf ab d A Dx Ee Ef : ℚ} (hd : |lf - d| ≤ 8 * ε * A) (hA : (1 - 8 * ε) * A ≤ ab)
    (hdA : |d| ≤ A) (hD : |D - Dx| ≤ Ee + 2 * ε * |D|) (hE0 : 0 ≤ Ee) (hEf : Ee ≤ (1 + 4 * ε) * Ef) :
    |D * lf - Dx * d| + 8 * ε * (|D| * |lf|) ≤
      (1 - 16 * ε) * (Ef * (|lf| + 3961408125713217 / 2 ^ 95 * ab) +
        3961408125713217 / 2 ^ 95 * |D| * (|lf| + ab)) := by
  obtain ⟨h1, h2⟩ := along_term hd hA hdA hD hE0
  have hA0 : 0 ≤ A := le_trans (abs_nonneg _) hdA
  have hab : 0 ≤ ab := le_trans (by nlinarith) hA
  have hEf0 : 0 ≤ Ef := by nlinarith
  have hl := abs_nonneg lf
  have ha := abs_nonneg D
  have p1 : Ee * |lf| ≤ (1 + 4 * ε) * Ef * |lf| := mul_le_mul_of_nonneg_right hEf hl
  have p2 : Ee * ab ≤ (1 + 4 * ε) * Ef * ab := mul_le_mul_of_nonneg_right hEf hab
  have p3 : Ef * |lf| ≤ Ef * ((1 + 18 * ε) * ab) := mul_le_mul_of_nonneg_left h2 hEf0
  have p4 : 0 ≤ Ef * ab := mul_nonneg hEf0 hab
  have p5 : 0 ≤ |D| * |lf| := mul_nonneg ha hl
  have p6 : 0 ≤ |D| * ab := mul_nonneg ha hab
  have p7 : 0 ≤ Ef * |lf| := mul_nonneg hEf0 hl
  linarith

/-- `along`'s `v = Σ (f_i,k − f_j,k)·l.f_k`, from `0.0`. -/
def alongV (D0 D1 D2 l0 l1 l2 : F64) : F64 := add (add (add F0 (mul D0 l0)) (mul D1 l1)) (mul D2 l2)

/-- `along`'s bound `b`, from `0.0`. -/
def alongB (E D0 D1 D2 l0 l1 l2 a0 a1 a2 : F64) : F64 :=
  add (add (add F0 (alongT E l0 a0 D0)) (alongT E l1 a1 D1)) (alongT E l2 a2 D2)

/-- `0.0` (`F0`), where the Rust's sums start. -/
lemma F0_tr (k : ℚ) : Tr F0 0 0 k := ⟨by decide, by simp [F0, FP.val], by simp⟩

set_option maxHeartbeats 4000000 in
/-- **`along`'s screen.** `D_k` is the f64 difference `f_i,k − f_j,k` of two points and `Dx_k` the exact
`P_i,k − P_j,k`, apart by `Ee + 2ε|D_k|` with `Ee ≤ (1 + 4ε)·(e_i + e_j)` in f64 (`E`); `l_k` tracks
the exact line direction `d_k` and `a_k` its magnitude, as `line_tr` gives them. Then `|v| > b` says
`(P_i − P_j)·d ≠ 0`, with `v`'s sign. -/
theorem along_sound {E D0 D1 D2 l0 l1 l2 a0 a1 a2 : F64} {d0 d1 d2 A0 A1 A2 X0 X1 X2 Ee : ℚ}
    (tl0 : Tr l0 d0 A0 8) (tl1 : Tr l1 d1 A1 8) (tl2 : Tr l2 d2 A2 8)
    (ta0 : Tr a0 A0 A0 8) (ta1 : Tr a1 A1 A1 8) (ta2 : Tr a2 A2 A2 8)
    (hD0 : D0.isFinite) (hD1 : D1.isFinite) (hD2 : D2.isFinite)
    (hx0 : |D0.val - X0| ≤ Ee + 2 * ε * |D0.val|) (hx1 : |D1.val - X1| ≤ Ee + 2 * ε * |D1.val|)
    (hx2 : |D2.val - X2| ≤ Ee + 2 * ε * |D2.val|)
    (hE : E.isFinite) (hE0 : 0 ≤ E.val) (hEe0 : 0 ≤ Ee) (hEe : Ee ≤ (1 + 4 * ε) * E.val)
    (hfv : (alongV D0 D1 D2 l0 l1 l2).isFinite) (hfb : (alongB E D0 D1 D2 l0 l1 l2 a0 a1 a2).isFinite)
    (zv : ∀ r ∈ [D0.val * l0.val, D1.val * l1.val, D2.val * l2.val], NZ r)
    (zb : ∀ lf ∈ [(l0, a0, D0), (l1, a1, D1), (l2, a2, D2)],
      NZ (E.val * (add lf.1.abs (mul E13 lf.2.1)).val) ∧ NZ (E13.val * lf.2.1.val) ∧
      NZ (E13.val * lf.2.2.abs.val) ∧ NZ ((mul E13 lf.2.2.abs).val * (add lf.1.abs lf.2.1).val))
    (hdec : (alongB E D0 D1 D2 l0 l1 l2 a0 a1 a2).val < |(alongV D0 D1 D2 l0 l1 l2).val|) :
    X0 * d0 + X1 * d1 + X2 * d2 ≠ 0 ∧
      (0 < X0 * d0 + X1 * d1 + X2 * d2 ↔ 0 < (alongV D0 D1 D2 l0 l1 l2).val) := by
  -- `v`
  unfold alongV at hfv hdec ⊢
  have fv2 := (isFinite_of_add hfv).1
  have fv1 := (isFinite_of_add fv2).1
  have m0 := tr_mul (Tr.exact hD0) (Tr.exact tl0.fin) (by norm_num) (by norm_num) (by norm_num) (by norm_num)
    (isFinite_of_add fv1).2 (zv _ (by simp))
  have m1 := tr_mul (Tr.exact hD1) (Tr.exact tl1.fin) (by norm_num) (by norm_num) (by norm_num) (by norm_num)
    (isFinite_of_add fv2).2 (zv _ (by simp))
  have m2 := tr_mul (Tr.exact hD2) (Tr.exact tl2.fin) (by norm_num) (by norm_num) (by norm_num) (by norm_num)
    (isFinite_of_add hfv).2 (zv _ (by simp))
  norm_num at m0 m1 m2
  have s1 := tr_add (F0_tr 2) m0 (by norm_num) (by norm_num) fv1
  norm_num at s1
  have s2 := tr_add s1 (m1.weaken (show (2 : ℚ) ≤ 4 by norm_num) m1.M_nonneg) (by norm_num) (by norm_num) fv2
  norm_num at s2
  have sv := tr_add s2 (m2.weaken (show (2 : ℚ) ≤ 6 by norm_num) m2.M_nonneg) (by norm_num) (by norm_num) hfv
  norm_num at sv
  -- `b`, from below
  unfold alongB at hfb hdec
  have fb2 := (isFinite_of_add hfb).1
  have fb1 := (isFinite_of_add fb2).1
  have ab0 : 0 ≤ a0.val := le_trans (by nlinarith [ta0.M_nonneg, ta0.lower]) ta0.lower
  have ab1 : 0 ≤ a1.val := le_trans (by nlinarith [ta1.M_nonneg, ta1.lower]) ta1.lower
  have ab2 : 0 ≤ a2.val := le_trans (by nlinarith [ta2.M_nonneg, ta2.lower]) ta2.lower
  obtain ⟨y1, y2, y3, y4⟩ := zb (l0, a0, D0) (by simp)
  have b0 := alongT_tr hE hE0 ab0 (isFinite_of_add fb1).2 y1 y2 y3 y4
  obtain ⟨y1, y2, y3, y4⟩ := zb (l1, a1, D1) (by simp)
  have b1 := alongT_tr hE hE0 ab1 (isFinite_of_add fb2).2 y1 y2 y3 y4
  obtain ⟨y1, y2, y3, y4⟩ := zb (l2, a2, D2) (by simp)
  have b2 := alongT_tr hE hE0 ab2 (isFinite_of_add hfb).2 y1 y2 y3 y4
  have t1 := tr_add (F0_tr 10) b0 (by norm_num) (by norm_num) fb1
  norm_num at t1
  have t2 := tr_add t1 (b1.weaken (show (10 : ℚ) ≤ 12 by norm_num) b1.M_nonneg) (by norm_num) (by norm_num) fb2
  norm_num at t2
  have tb := tr_add t2 (b2.weaken (show (10 : ℚ) ≤ 14 by norm_num) b2.M_nonneg) (by norm_num) (by norm_num) hfb
  norm_num at tb
  have hlow := tb.lower
  -- each axis
  have c0 := along_comp (D := D0.val) tl0.err ta0.lower tl0.mag hx0 hEe0 hEe
  have c1 := along_comp (D := D1.val) tl1.err ta1.lower tl1.mag hx1 hEe0 hEe
  have c2 := along_comp (D := D2.val) tl2.err ta2.lower tl2.mag hx2 hEe0 hEe
  have hve := sv.err
  have hlt : |(add (add (add F0 (mul D0 l0)) (mul D1 l1)) (mul D2 l2)).val - (X0 * d0 + X1 * d1 + X2 * d2)| <
      |(add (add (add F0 (mul D0 l0)) (mul D1 l1)) (mul D2 l2)).val| := by
    have e3 : |(D0.val * l0.val + D1.val * l1.val + D2.val * l2.val) - (X0 * d0 + X1 * d1 + X2 * d2)| ≤
        |D0.val * l0.val - X0 * d0| + |D1.val * l1.val - X1 * d1| + |D2.val * l2.val - X2 * d2| := by
      have := abs_add_three (D0.val * l0.val - X0 * d0) (D1.val * l1.val - X1 * d1) (D2.val * l2.val - X2 * d2)
      convert this using 2; ring
    have e4 := abs_sub_le (add (add (add F0 (mul D0 l0)) (mul D1 l1)) (mul D2 l2)).val
      (D0.val * l0.val + D1.val * l1.val + D2.val * l2.val) (X0 * d0 + X1 * d1 + X2 * d2)
    linarith
  have r := sign_close_d hlt
  exact ⟨r.1, r.2⟩

/-! ## `orient` -/

/-- The f64 literal `1e-14`. -/
def E14 : F64 := .fin false 6338253001141147 (-99) (by decide)
theorem E14_bits : E14.toBits = 0x3d06849b86a12b9b := by decide
theorem E14_val : E14.val = (6338253001141147 / 2 ^ 99 : ℚ) := by simp only [E14, val, sgn]; norm_num

lemma abs_sub_le_add (a b : ℚ) : |a - b| ≤ |a| + |b| := by
  simpa [sub_eq_add_neg, abs_neg] using abs_add_le a (-b)

/-- A product of two differences, each off its exact value by `Ee + 2ε|·|`. -/
lemma prod_err {x y X Y Ee : ℚ} (hx : |x - X| ≤ Ee + 2 * ε * |x|) (hy : |y - Y| ≤ Ee + 2 * ε * |y|)
    (hE : 0 ≤ Ee) :
    |x * y - X * Y| ≤ Ee ^ 2 + (1 + 2 * ε) * Ee * (|x| + |y|) + (4 * ε + 4 * ε ^ 2) * (|x| * |y|) := by
  have e : x * y - X * Y = x * (y - Y) + (x - X) * y - (x - X) * (y - Y) := by ring
  rw [e]
  have t := abs_sub_le_add (x * (y - Y) + (x - X) * y) ((x - X) * (y - Y))
  have t2 := abs_add_le (x * (y - Y)) ((x - X) * y)
  simp only [abs_mul] at t t2
  have hxa := abs_nonneg x; have hya := abs_nonneg y
  have a1 := mul_le_mul_of_nonneg_left hy hxa
  have a2 := mul_le_mul_of_nonneg_right hx hya
  have a3 := mul_le_mul hx hy (abs_nonneg _) (by positivity)
  nlinarith

/-- `orient`'s `e = 2(e_a + max(e_b, e_c))(1 + 1e-12)`, from below. -/
lemma orient_e {ea m : F64} (ha : ea.isFinite) (hm : m.isFinite) (ha0 : 0 ≤ ea.val) (hm0 : 0 ≤ m.val)
    (hf : (mul (mul F2 (add ea m)) G12).isFinite)
    (z1 : NZ (F2.val * (add ea m).val)) (z2 : NZ ((mul F2 (add ea m)).val * G12.val)) :
    2 * (1 - 6 * ε) * (ea.val + m.val) ≤ (mul (mul F2 (add ea m)) G12).val := by
  have f1 := (isFinite_of_mul hf).1
  have fG := (isFinite_of_mul hf).2
  have fs := (isFinite_of_mul f1).2
  have f2 : F2.isFinite := by decide
  have t2 := Tr.exact_nn f2 (by rw [F2_val]; norm_num) 0 le_rfl
  have tG := Tr.exact_nn fG (by have := G12_lo fG; linarith) 0 le_rfl
  have s := tr_add (Tr.exact_nn ha ha0 0 le_rfl) (Tr.exact_nn hm hm0 0 le_rfl) le_rfl (by norm_num) fs
  have m1 := tr_mul t2 s (by norm_num) (by norm_num) (by norm_num) (by norm_num) f1 z1
  norm_num at m1
  have m2 := tr_mul m1 tG (by norm_num) (by norm_num) (by norm_num) (by norm_num) hf z2
  norm_num at m2
  have lo := m2.lower
  have g := G12_lo fG
  rw [F2_val] at lo
  have : 0 ≤ (1 - 6 * ε) * (2 * (ea.val + m.val)) := by norm_num; positivity
  nlinarith

/-- `orient`'s `v = bs·ct − bt·cs`. -/
def orientV (bs bt cs ct : F64) : F64 := sub (mul bs ct) (mul bt cs)

/-- `orient`'s bound `e·(|bs| + |bt| + |cs| + |ct|) + e·e + 1e-14·(|bs·ct| + |bt·cs|)`. -/
def orientB (e bs bt cs ct : F64) : F64 :=
  add (add (mul e (add (add (add bs.abs bt.abs) cs.abs) ct.abs)) (mul e e))
    (mul E14 (add (mul bs ct).abs (mul bt cs).abs))

set_option maxHeartbeats 4000000 in
/-- **`orient`'s screen.** `bs, bt, cs, ct` are the f64 differences `f_b − f_a`, `f_c − f_a` on axes
`s, t`, each off the exact `BS, BT, CS, CT` by `Ee + 2ε|·|` (`Ee = e_a + max(e_b, e_c)`), and `e`
is at least `2(1 − 6ε)·Ee` (`orient_e`). Then `|v| > bound` says the exact `BS·CT − BT·CS ≠ 0`,
with `v`'s sign. -/
theorem orient_sound {e bs bt cs ct : F64} {BS BT CS CT Ee : ℚ}
    (hbs : bs.isFinite) (hbt : bt.isFinite) (hcs : cs.isFinite) (hct : ct.isFinite) (hef : e.isFinite)
    (xbs : |bs.val - BS| ≤ Ee + 2 * ε * |bs.val|) (xbt : |bt.val - BT| ≤ Ee + 2 * ε * |bt.val|)
    (xcs : |cs.val - CS| ≤ Ee + 2 * ε * |cs.val|) (xct : |ct.val - CT| ≤ Ee + 2 * ε * |ct.val|)
    (hE0 : 0 ≤ Ee) (he : 2 * (1 - 6 * ε) * Ee ≤ e.val)
    (hfv : (orientV bs bt cs ct).isFinite) (hfb : (orientB e bs bt cs ct).isFinite)
    (z : ∀ r ∈ [bs.val * ct.val, bt.val * cs.val,
      e.val * (add (add (add bs.abs bt.abs) cs.abs) ct.abs).val, e.val * e.val,
      E14.val * (add (mul bs ct).abs (mul bt cs).abs).val], NZ r)
    (hdec : (orientB e bs bt cs ct).val < |(orientV bs bt cs ct).val|) :
    BS * CT - BT * CS ≠ 0 ∧ (0 < BS * CT - BT * CS ↔ 0 < (orientV bs bt cs ct).val) := by
  unfold orientV at hfv hdec ⊢
  unfold orientB at hfb hdec
  have f1 := (isFinite_of_sub hfv).1
  have f2 := (isFinite_of_sub hfv).2
  -- `v`
  have m1 := tr_mul (Tr.exact hbs) (Tr.exact hct) (by norm_num) (by norm_num) (by norm_num) (by norm_num) f1 (z _ (by simp))
  have m2 := tr_mul (Tr.exact hbt) (Tr.exact hcs) (by norm_num) (by norm_num) (by norm_num) (by norm_num) f2 (z _ (by simp))
  norm_num at m1 m2
  have sv := tr_sub m1 m2 (by norm_num) (by norm_num) hfv
  norm_num at sv
  -- `bound`, from below
  have fb1 := (isFinite_of_add hfb).1
  have fb2 := (isFinite_of_add hfb).2
  have fes := (isFinite_of_add fb1).1
  have fee := (isFinite_of_add fb1).2
  have fS := (isFinite_of_mul fes).2
  have fS2 := (isFinite_of_add fS).1
  have fS1 := (isFinite_of_add fS2).1
  have fP := (isFinite_of_mul fb2).2
  have e0 : 0 ≤ e.val := le_trans (mul_nonneg (by norm_num) hE0) he
  have ab : ∀ x : F64, x.isFinite → ∀ k : ℚ, 0 ≤ k → Tr x.abs |x.val| |x.val| k := fun x hx k hk => by
    have := Tr.exact_nn (v := x.abs) (by simpa using hx) (by rw [FP.abs_val]; exact abs_nonneg _) k hk
    rwa [FP.abs_val] at this
  have te := Tr.exact_nn hef e0 0 le_rfl
  have s1 := tr_add (ab bs hbs 0 le_rfl) (ab bt hbt 0 le_rfl) le_rfl (by norm_num) fS1
  norm_num at s1
  have s2 := tr_add s1 (ab cs hcs 2 (by norm_num)) (by norm_num) (by norm_num) fS2
  norm_num at s2
  have s3 := tr_add s2 (ab ct hct 4 (by norm_num)) (by norm_num) (by norm_num) fS
  norm_num at s3
  have mes := tr_mul te s3 (by norm_num) (by norm_num) (by norm_num) (by norm_num) fes (z _ (by simp))
  have mee := tr_mul te te (by norm_num) (by norm_num) (by norm_num) (by norm_num) fee (z _ (by simp))
  norm_num at mes mee
  have a1 := tr_add mes (mee.weaken (show (2 : ℚ) ≤ 8 by norm_num) mee.M_nonneg) (by norm_num) (by norm_num) fb1
  norm_num at a1
  have p1 := tr_add (ab _ f1 0 le_rfl) (ab _ f2 0 le_rfl) le_rfl (by norm_num) fP
  norm_num at p1
  have f14 : E14.isFinite := by decide
  have c14 : (0 : ℚ) ≤ E14.val := by rw [E14_val]; norm_num
  have mp := tr_mul (Tr.exact_nn f14 c14 0 le_rfl) p1 (by norm_num) (by norm_num) (by norm_num) (by norm_num) fb2
    (z _ (by simp))
  norm_num at mp
  have tb := tr_add a1 (mp.weaken (show (4 : ℚ) ≤ 10 by norm_num) mp.M_nonneg) (by norm_num) (by norm_num) hfb
  norm_num at tb
  have hlow := tb.lower
  rw [E14_val] at hlow
  -- the error
  have q1 := prod_err xbs xct hE0
  have q2 := prod_err xbt xcs hE0
  have hve := sv.err
  have r1 := m1.err
  have r2 := m2.err
  set P := |bs.val| * |ct.val| + |bt.val| * |cs.val| with hP
  set S := |bs.val| + |bt.val| + |cs.val| + |ct.val| with hS
  have hS0 : 0 ≤ S := by positivity
  have hP0 : 0 ≤ P := by positivity
  have l1 : |(mul bs ct).val| + |(mul bt cs).val| ≥ (1 - 2 * ε) * P := by
    have u1 := abs_sub_abs_le_abs_sub (bs.val * ct.val) (mul bs ct).val
    have u2 := abs_sub_abs_le_abs_sub (bt.val * cs.val) (mul bt cs).val
    rw [abs_sub_comm] at u1 u2
    rw [abs_mul] at u1 u2
    linarith
  have l2 : 2 * (1 - 6 * ε) * Ee * S ≤ e.val * S := mul_le_mul_of_nonneg_right he hS0
  have l3 : (2 * (1 - 6 * ε) * Ee) * (2 * (1 - 6 * ε) * Ee) ≤ e.val * e.val :=
    mul_le_mul he he (mul_nonneg (by norm_num) hE0) e0
  have l4 : 0 ≤ Ee * S := mul_nonneg hE0 hS0
  have l5 : 0 ≤ Ee ^ 2 := sq_nonneg _
  have hlt : |(sub (mul bs ct) (mul bt cs)).val - (BS * CT - BT * CS)| < |(sub (mul bs ct) (mul bt cs)).val| := by
    have w := abs_sub_le (sub (mul bs ct) (mul bt cs)).val (bs.val * ct.val - bt.val * cs.val) (BS * CT - BT * CS)
    have w2 : |bs.val * ct.val - bt.val * cs.val - (BS * CT - BT * CS)| ≤
        |bs.val * ct.val - BS * CT| + |bt.val * cs.val - BT * CS| := by
      have := abs_sub_le_add (bs.val * ct.val - BS * CT) (bt.val * cs.val - BT * CS)
      convert this using 2; ring
    have : e.val * (|bs.val| + |bt.val| + |cs.val| + |ct.val|) = e.val * S := by rw [hS]
    nlinarith
  exact sign_close_d hlt

/-! ## The glue: point bounds to the screens' hypotheses -/

/-- The f64 difference of two bounded points' coordinates is off the exact one by `e_i + e_j + 2ε|D|`. -/
lemma pt_diff_close {fi fj : F64} {Pi Pj ei ej : ℚ} (hi : fi.isFinite) (hj : fj.isFinite)
    (hf : (sub fi fj).isFinite) (bi : |fi.val - Pi| ≤ ei) (bj : |fj.val - Pj| ≤ ej) :
    |(sub fi fj).val - (Pi - Pj)| ≤ ei + ej + 2 * ε * |(sub fi fj).val| := by
  have r := sub_rel0 hi hj hf
  have u := abs_sub_abs_le_abs_sub (fi.val - fj.val) (sub fi fj).val
  rw [abs_sub_comm (fi.val - fj.val)] at u
  have w : |(sub fi fj).val - (Pi - Pj)| ≤ |(sub fi fj).val - (fi.val - fj.val)| + |fi.val - Pi| + |fj.val - Pj| := by
    have h1 := abs_sub_le (sub fi fj).val (fi.val - fj.val) (Pi - Pj)
    have h2 : |fi.val - fj.val - (Pi - Pj)| ≤ |fi.val - Pi| + |fj.val - Pj| := by
      have e : fi.val - fj.val - (Pi - Pj) = (fi.val - Pi) - (fj.val - Pj) := by ring
      rw [e]; exact abs_sub_le_add _ _
    linarith
  have := abs_nonneg (fi.val - fj.val)
  nlinarith

/-- `e_i + e_j ≤ (1 + 4ε)·(e_i + e_j)` as f64. -/
lemma esum_le {ei ej : F64} (hi : ei.isFinite) (hj : ej.isFinite) (h0 : 0 ≤ ei.val) (h1 : 0 ≤ ej.val)
    (hf : (add ei ej).isFinite) : ei.val + ej.val ≤ (1 + 4 * ε) * (add ei ej).val := by
  have r := add_rel0 hi hj hf
  rw [abs_of_nonneg (by linarith : (0 : ℚ) ≤ ei.val + ej.val)] at r
  have := (_root_.abs_le.mp r).1
  nlinarith

end IEEE754.Screen
