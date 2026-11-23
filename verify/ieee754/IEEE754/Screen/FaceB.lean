/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Face

/-!
# decide-once A8's `face_fast` screens kept by the rebuilt builder (41afccb)

Rust's `f64::EPSILON` is `2^-52 = 2ε` here (`ε = 2^-53`, the unit roundoff).

- `angle_sound` (the half-edge order, fixed in efd1d6f): `fx, fy` are each the exact direction
  component rounded once (`|f − X| ≤ ε|X|`). If `|fx_a·fy_b − fy_a·fx_b|` is greater than
  `8·EPSILON·(|fx_a·fy_b| + |fy_a·fx_b|)`, the exact cross product is `≠ 0` with that sign.
- `wall_close`, `wall_lo`, `wall_hi` (`whole_plane`, `axis_plane`): a wall `x = c/n` from a plane
  rounded once per field is within `4ε|X|` of the exact `X = C/N`. A box face past `x ± tol(x)`,
  with `tol(x) = 8·EPSILON·(1 + |x|)`, is strictly past the exact wall.
- `touches_sound` (41afccb: the box is the hull of the vertices' `f ± e`): the plane's `n, c` are
  each the exact `N, C` rounded once. If `touches` answers "misses" (`|v| > r + 1e-12·m`), every
  point of the box is on one nonzero exact side, `v`'s. The margin needed is about `40ε·m`.
-/

namespace IEEE754.Screen

open FP

/-- `8.0 * f64::EPSILON = 2^-49`. -/
def K16 : F64 := .fin false 4503599627370496 (-101) (by decide)
theorem K16_val : K16.val = (1 / 2 ^ 49 : ℚ) := by simp only [K16, val, sgn]; norm_num

/-- A float rounded once from an exact value. -/
lemma Tr.round1 {f : F64} {X : ℚ} (hf : f.isFinite) (h : |f.val - X| ≤ ε * |X|) : Tr f X |X| 1 :=
  ⟨hf, by simpa using h, le_rfl⟩

/-! ## The angle order -/

set_option maxHeartbeats 1000000 in
theorem angle_sound {xa ya xb yb : F64} {Xa Ya Xb Yb : ℚ}
    (fa : xa.isFinite) (ga : ya.isFinite) (fb : xb.isFinite) (gb : yb.isFinite)
    (ea : |xa.val - Xa| ≤ ε * |Xa|) (eya : |ya.val - Ya| ≤ ε * |Ya|)
    (eb : |xb.val - Xb| ≤ ε * |Xb|) (eyb : |yb.val - Yb| ≤ ε * |Yb|)
    (hfv : (sub (mul xa yb) (mul ya xb)).isFinite)
    (hfb : (mul K16 (add (mul xa yb).abs (mul ya xb).abs)).isFinite)
    (z1 : NZ (xa.val * yb.val)) (z2 : NZ (ya.val * xb.val))
    (z3 : NZ (K16.val * (add (mul xa yb).abs (mul ya xb).abs).val))
    (hdec : (mul K16 (add (mul xa yb).abs (mul ya xb).abs)).val < |(sub (mul xa yb) (mul ya xb)).val|) :
    Xa * Yb - Ya * Xb ≠ 0 ∧ (0 < Xa * Yb - Ya * Xb ↔ 0 < (sub (mul xa yb) (mul ya xb)).val) := by
  have f1 := (isFinite_of_sub hfv).1
  have f2 := (isFinite_of_sub hfv).2
  have m1 := tr_mul (Tr.round1 fa ea) (Tr.round1 gb eyb) (by norm_num) (by norm_num) (by norm_num) (by norm_num) f1 z1
  have m2 := tr_mul (Tr.round1 ga eya) (Tr.round1 fb eb) (by norm_num) (by norm_num) (by norm_num) (by norm_num) f2 z2
  norm_num at m1 m2
  have sv := tr_sub m1 m2 (by norm_num) (by norm_num) hfv
  norm_num at sv
  -- the bound, from below
  have fs := (isFinite_of_mul hfb).2
  have a1 := m1.abs
  have a2 := m2.abs
  rw [abs_mul] at a1 a2
  have s := tr_add a1 a2 (by norm_num) (by norm_num) fs
  norm_num at s
  have fK : K16.isFinite := by decide
  have tK := Tr.exact fK
  have mb := tr_mul tK s (by norm_num) (by norm_num) (by norm_num) (by norm_num) hfb z3
  norm_num at mb
  rw [K16_val] at mb
  norm_num at mb
  have hlow := mb.lower
  have hM := s.M_nonneg
  have hve := sv.err
  have hlt : |(sub (mul xa yb) (mul ya xb)).val - (Xa * Yb - Ya * Xb)| < |(sub (mul xa yb) (mul ya xb)).val| := by
    nlinarith
  exact sign_close_d hlt

/-! ## The walls -/

/-- A wall `x = c/n`, `c` and `n` each rounded once from the exact `C`, `N ≠ 0`: `|x − C/N| ≤ 4ε|C/N|`. -/
theorem wall_close {c n : F64} {C N : ℚ} (hc : c.isFinite) (hn : n.isFinite) (hN : N ≠ 0)
    (ec : |c.val - C| ≤ ε * |C|) (en : |n.val - N| ≤ ε * |N|) (hf : (div c n).isFinite)
    (z : NZ (c.val / n.val)) : |(div c n).val - C / N| ≤ 4 * ε * |C / N| := by
  have hNa : 0 < |N| := abs_pos.2 hN
  have hn1 : (1 - ε) * |N| ≤ |n.val| := by
    have := abs_sub_abs_le_abs_sub N n.val; rw [abs_sub_comm] at this; nlinarith
  have hn0 : n.val ≠ 0 := by intro h; rw [h, abs_zero] at hn1; norm_num at hn1; nlinarith
  have hna : 0 < |n.val| := abs_pos.2 hn0
  have r := div_rel0 hc hn hn0 hf z
  -- `|c/n − C/N| ≤ 2ε|C|/|n| ≤ (2ε/(1−ε))|C/N|`
  have e1 : c.val / n.val - C / N = ((c.val - C) * N - C * (n.val - N)) / (n.val * N) := by
    field_simp; ring
  have e2 : |c.val / n.val - C / N| ≤ 5 / 2 * ε * |C / N| := by
    have num : |(c.val - C) * N - C * (n.val - N)| ≤ 2 * ε * (|C| * |N|) := by
      have t := abs_sub_le_add ((c.val - C) * N) (C * (n.val - N))
      rw [abs_mul, abs_mul] at t
      have u1 := mul_le_mul_of_nonneg_right ec (abs_nonneg N)
      have u2 := mul_le_mul_of_nonneg_left en (abs_nonneg C)
      nlinarith
    have key : |C / N| * |N| = |C| := by rw [abs_div, div_mul_cancel₀ _ (ne_of_gt hNa)]
    rw [e1, abs_div, abs_mul, div_le_iff₀ (mul_pos hna hNa)]
    have h3 := mul_le_mul_of_nonneg_left hn1 (abs_nonneg C)
    have : 5 / 2 * ε * |C / N| * (|n.val| * |N|) = 5 / 2 * ε * |C| * |n.val| := by
      rw [show 5 / 2 * ε * |C / N| * (|n.val| * |N|) = 5 / 2 * ε * (|C / N| * |N|) * |n.val| by ring, key]
    rw [this]
    nlinarith [abs_nonneg C, abs_nonneg N]
  have e3 : |c.val / n.val| ≤ (1 + 5 / 2 * ε) * |C / N| := by
    have := abs_sub_abs_le_abs_sub (c.val / n.val) (C / N); linarith
  have := abs_sub_le (div c n).val (c.val / n.val) (C / N)
  nlinarith [abs_nonneg (C / N)]

/-- `tol(x) = 8·EPSILON·(1 + |x|)`. -/
def wallTol (x : F64) : F64 := mul K16 (add F1 x.abs)

lemma wallTol_lo {x : F64} (hf : (wallTol x).isFinite) (z : NZ (K16.val * (add F1 x.abs).val)) :
    (1 - 4 * ε) * (1 / 2 ^ 49) * (1 + |x.val|) ≤ (wallTol x).val := by
  unfold wallTol at hf ⊢
  have fs := (isFinite_of_mul hf).2
  have fx := (isFinite_of_add fs).2
  have f1 : F1.isFinite := by decide
  have t1 := Tr.exact_nn f1 (by rw [F1_val]; norm_num) 0 le_rfl
  have tx := Tr.exact_nn fx (by rw [FP.abs_val]; exact abs_nonneg _) 0 le_rfl
  have s := tr_add t1 tx le_rfl (by norm_num) fs
  have fK : K16.isFinite := by decide
  have m := tr_mul (Tr.exact_nn fK (by rw [K16_val]; norm_num) 0 le_rfl) s (by norm_num) (by norm_num)
    (by norm_num) (by norm_num) hf z
  have lo := m.lower
  rw [K16_val, F1_val, FP.abs_val] at lo
  norm_num at lo ⊢
  nlinarith [abs_nonneg x.val]

/-- **The low wall.** A box low end above `x + tol(x)` (in f64) is above the exact wall `W`. -/
theorem wall_lo {x lo : F64} {W : ℚ} (hx : x.isFinite) (hW : |x.val - W| ≤ 4 * ε * |W|)
    (ht : (wallTol x).isFinite) (z : NZ (K16.val * (add F1 x.abs).val)) (hf : (add x (wallTol x)).isFinite)
    (h : (add x (wallTol x)).val < lo.val) : W < lo.val := by
  have t := wallTol_lo ht z
  have r := add_rel0 hx ht hf
  have hWa : |W| ≤ |x.val| + 4 * ε * |W| := by
    have := abs_sub_abs_le_abs_sub W x.val; rw [abs_sub_comm] at this; linarith
  have tpos : 0 ≤ (wallTol x).val := le_trans (by positivity) t
  have := abs_add_le x.val (wallTol x).val
  rw [abs_of_nonneg tpos] at this
  rw [abs_le] at r hW
  norm_num at t
  nlinarith [abs_nonneg x.val, abs_nonneg W, neg_abs_le x.val, le_abs_self x.val]

/-- **The high wall.** A box high end below `x − tol(x)` is below the exact wall `W`. -/
theorem wall_hi {x hi : F64} {W : ℚ} (hx : x.isFinite) (hW : |x.val - W| ≤ 4 * ε * |W|)
    (ht : (wallTol x).isFinite) (z : NZ (K16.val * (add F1 x.abs).val)) (hf : (sub x (wallTol x)).isFinite)
    (h : hi.val < (sub x (wallTol x)).val) : hi.val < W := by
  have t := wallTol_lo ht z
  have r := sub_rel0 hx ht hf
  have hWa : |W| ≤ |x.val| + 4 * ε * |W| := by
    have := abs_sub_abs_le_abs_sub W x.val; rw [abs_sub_comm] at this; linarith
  have tpos : 0 ≤ (wallTol x).val := le_trans (by positivity) t
  have := abs_sub_le_add x.val (wallTol x).val
  rw [abs_of_nonneg tpos] at this
  rw [abs_le] at r hW
  norm_num at t
  nlinarith [abs_nonneg x.val, abs_nonneg W, neg_abs_le x.val, le_abs_self x.val]

/-! ## `touches`: a box against a plane -/

/-- `0.5`. -/
def H05 : F64 := .fin false 4503599627370496 (-53) (by decide)
theorem H05_val : H05.val = (1 / 2 : ℚ) := by simp only [H05, val, sgn]; norm_num

/-- `mid = 0.5·(lo + hi)`, `half = 0.5·(hi − lo)`. -/
def midF (lo hi : F64) : F64 := mul H05 (add lo hi)
def halfF (lo hi : F64) : F64 := mul H05 (sub hi lo)

/-- One axis of `touches`: `n·mid` and `|n|·half`, tracked against `N·μ` and `|N|·h`
(`μ = (lo+hi)/2`, `h = (hi−lo)/2`), both of magnitude `|N|·(|lo|+|hi|)/2`. -/
lemma touch_axis {n lo hi : F64} {N : ℚ} (hn : n.isFinite) (en : |n.val - N| ≤ ε * |N|)
    (hlo : lo.isFinite) (hhi : hi.isFinite)
    (f1 : (mul n (midF lo hi)).isFinite) (f2 : (mul n.abs (halfF lo hi)).isFinite)
    (z1 : NZ (H05.val * (add lo hi).val)) (z2 : NZ (H05.val * (sub hi lo).val))
    (z3 : NZ (n.val * (midF lo hi).val)) (z4 : NZ (n.abs.val * (halfF lo hi).val)) :
    Tr (mul n (midF lo hi)) (N * ((lo.val + hi.val) / 2)) (|N| * ((|lo.val| + |hi.val|) / 2)) 7 ∧
      Tr (mul n.abs (halfF lo hi)) (|N| * ((hi.val - lo.val) / 2)) (|N| * ((|lo.val| + |hi.val|) / 2)) 7 := by
  have fm := (isFinite_of_mul f1).2
  have fh := (isFinite_of_mul f2).2
  unfold midF at fm z3 ⊢
  unfold halfF at fh z4 ⊢
  have fH : H05.isFinite := by decide
  have tH := Tr.exact_nn fH (by rw [H05_val]; norm_num) 0 le_rfl
  have sa := tr_add (Tr.exact hlo) (Tr.exact hhi) le_rfl (by norm_num) (isFinite_of_mul fm).2
  have sb := tr_sub (Tr.exact hhi) (Tr.exact hlo) le_rfl (by norm_num) (isFinite_of_mul fh).2
  have ma := tr_mul tH sa (by norm_num) (by norm_num) (by norm_num) (by norm_num) fm z1
  have mb := tr_mul tH sb (by norm_num) (by norm_num) (by norm_num) (by norm_num) fh z2
  norm_num at ma mb
  rw [H05_val] at ma mb
  have tn := Tr.round1 hn en
  have p1 := tr_mul tn ma (by norm_num) (by norm_num) (by norm_num) (by norm_num) f1 z3
  have p2 := tr_mul tn.abs mb (by norm_num) (by norm_num) (by norm_num) (by norm_num) f2 z4
  norm_num at p1 p2
  constructor
  · convert p1 using 2 <;> ring
  · convert p2 using 2 <;> ring

/-- `touches`' `v = −c + Σ n_k·mid_k`. -/
def touchV (c n0 n1 n2 l0 l1 l2 h0 h1 h2 : F64) : F64 :=
  add (add (add c.neg (mul n0 (midF l0 h0))) (mul n1 (midF l1 h1))) (mul n2 (midF l2 h2))

/-- `touches`' `r = Σ |n_k|·half_k`, from `0.0`. -/
def touchR (n0 n1 n2 l0 l1 l2 h0 h1 h2 : F64) : F64 :=
  add (add (add F0 (mul n0.abs (halfF l0 h0))) (mul n1.abs (halfF l1 h1))) (mul n2.abs (halfF l2 h2))

/-- `touches`' `m = |c| + Σ (|n_k·mid_k| + |n_k|·half_k)`. -/
def touchM (c n0 n1 n2 l0 l1 l2 h0 h1 h2 : F64) : F64 :=
  add (add (add c.abs (add (mul n0 (midF l0 h0)).abs (mul n0.abs (halfF l0 h0))))
    (add (mul n1 (midF l1 h1)).abs (mul n1.abs (halfF l1 h1))))
    (add (mul n2 (midF l2 h2)).abs (mul n2.abs (halfF l2 h2)))

/-- `|μ| + h ≥ (|lo| + |hi|)/2` for `lo ≤ hi`. -/
lemma mid_half {lo hi : ℚ} (h : lo ≤ hi) : (|lo| + |hi|) / 2 ≤ |(lo + hi) / 2| + (hi - lo) / 2 := by
  rcases abs_cases lo with ⟨a, _⟩ | ⟨a, _⟩ <;> rcases abs_cases hi with ⟨b, _⟩ | ⟨b, _⟩ <;>
    rcases abs_cases ((lo + hi) / 2) with ⟨c, _⟩ | ⟨c, _⟩ <;> rw [a, b, c] <;> linarith

/-- A point of the box `[lo, hi]` is within `h` of `μ`. -/
lemma in_box {x lo hi : ℚ} (h1 : lo ≤ x) (h2 : x ≤ hi) : |x - (lo + hi) / 2| ≤ (hi - lo) / 2 := by
  rw [abs_le]; constructor <;> linarith

set_option maxHeartbeats 8000000 in
/-- **`touches`.** The plane's fields `n_k, c` are each the exact `N_k, C` rounded once; the box
is `[lo_k, hi_k]`. If `touches` answers "misses" (`|v| > r + 1e-12·m` in f64), then every point
`x` of the box has `N·x − C ≠ 0`, with `v`'s sign: one nonzero side over the box. -/
theorem touches_sound {c n0 n1 n2 l0 l1 l2 h0 h1 h2 : F64} {C N0 N1 N2 : ℚ}
    (hc : c.isFinite) (ec : |c.val - C| ≤ ε * |C|)
    (hn0 : n0.isFinite) (hn1 : n1.isFinite) (hn2 : n2.isFinite)
    (en0 : |n0.val - N0| ≤ ε * |N0|) (en1 : |n1.val - N1| ≤ ε * |N1|) (en2 : |n2.val - N2| ≤ ε * |N2|)
    (fl0 : l0.isFinite) (fl1 : l1.isFinite) (fl2 : l2.isFinite)
    (fh0 : h0.isFinite) (fh1 : h1.isFinite) (fh2 : h2.isFinite)
    (o0 : l0.val ≤ h0.val) (o1 : l1.val ≤ h1.val) (o2 : l2.val ≤ h2.val)
    (fv : (touchV c n0 n1 n2 l0 l1 l2 h0 h1 h2).isFinite)
    (fb : (add (touchR n0 n1 n2 l0 l1 l2 h0 h1 h2) (mul E12 (touchM c n0 n1 n2 l0 l1 l2 h0 h1 h2))).isFinite)
    (zh : ∀ p ∈ [(l0, h0), (l1, h1), (l2, h2)],
      NZ (H05.val * (add p.1 p.2).val) ∧ NZ (H05.val * (sub p.2 p.1).val))
    (zp : ∀ p ∈ [(n0, l0, h0), (n1, l1, h1), (n2, l2, h2)],
      NZ (p.1.val * (midF p.2.1 p.2.2).val) ∧ NZ (p.1.abs.val * (halfF p.2.1 p.2.2).val))
    (zm : NZ (E12.val * (touchM c n0 n1 n2 l0 l1 l2 h0 h1 h2).val))
    (hdec : (add (touchR n0 n1 n2 l0 l1 l2 h0 h1 h2) (mul E12 (touchM c n0 n1 n2 l0 l1 l2 h0 h1 h2))).val <
      |(touchV c n0 n1 n2 l0 l1 l2 h0 h1 h2).val|)
    {x0 x1 x2 : ℚ} (b0 : l0.val ≤ x0 ∧ x0 ≤ h0.val) (b1 : l1.val ≤ x1 ∧ x1 ≤ h1.val)
    (b2 : l2.val ≤ x2 ∧ x2 ≤ h2.val) :
    N0 * x0 + N1 * x1 + N2 * x2 - C ≠ 0 ∧
      (0 < N0 * x0 + N1 * x1 + N2 * x2 - C ↔ 0 < (touchV c n0 n1 n2 l0 l1 l2 h0 h1 h2).val) := by
  -- finiteness, piece by piece
  have fr := (isFinite_of_add fb).1
  have fq := (isFinite_of_add fb).2
  have fm := (isFinite_of_mul fq).2
  unfold touchV at fv hdec ⊢
  unfold touchR at fr hdec
  unfold touchM at fm zm hdec
  have fv2 := (isFinite_of_add fv).1
  have fv1 := (isFinite_of_add fv2).1
  have fr2 := (isFinite_of_add fr).1
  have fr1 := (isFinite_of_add fr2).1
  have fm2 := (isFinite_of_add fm).1
  have fm1 := (isFinite_of_add fm2).1
  -- the axes
  obtain ⟨y1, y2⟩ := zh (l0, h0) (by simp)
  obtain ⟨w1, w2⟩ := zp (n0, l0, h0) (by simp)
  obtain ⟨A0, B0⟩ := touch_axis hn0 en0 fl0 fh0 (isFinite_of_add fv1).2 (isFinite_of_add fr1).2 y1 y2 w1 w2
  obtain ⟨y1, y2⟩ := zh (l1, h1) (by simp)
  obtain ⟨w1, w2⟩ := zp (n1, l1, h1) (by simp)
  obtain ⟨A1, B1⟩ := touch_axis hn1 en1 fl1 fh1 (isFinite_of_add fv2).2 (isFinite_of_add fr2).2 y1 y2 w1 w2
  obtain ⟨y1, y2⟩ := zh (l2, h2) (by simp)
  obtain ⟨w1, w2⟩ := zp (n2, l2, h2) (by simp)
  obtain ⟨A2, B2⟩ := touch_axis hn2 en2 fl2 fh2 (isFinite_of_add fv).2 (isFinite_of_add fr).2 y1 y2 w1 w2
  -- `v`
  have tc : Tr c.neg (-C) |C| 7 :=
    ⟨by simpa using hc, by rw [FP.val_neg]; rw [show -c.val - -C = -(c.val - C) by ring, abs_neg]; nlinarith [abs_nonneg C],
      by rw [abs_neg]⟩
  have v1 := tr_add tc A0 (by norm_num) (by norm_num) fv1
  norm_num at v1
  have v2 := tr_add v1 (A1.weaken (show (7 : ℚ) ≤ 9 by norm_num) A1.M_nonneg) (by norm_num) (by norm_num) fv2
  norm_num at v2
  have vv := tr_add v2 (A2.weaken (show (7 : ℚ) ≤ 11 by norm_num) A2.M_nonneg) (by norm_num) (by norm_num) fv
  norm_num at vv
  -- `r`
  have r1 := tr_add (F0_tr 7) B0 (by norm_num) (by norm_num) fr1
  norm_num at r1
  have r2 := tr_add r1 (B1.weaken (show (7 : ℚ) ≤ 9 by norm_num) B1.M_nonneg) (by norm_num) (by norm_num) fr2
  norm_num at r2
  have rr := tr_add r2 (B2.weaken (show (7 : ℚ) ≤ 11 by norm_num) B2.M_nonneg) (by norm_num) (by norm_num) fr
  norm_num at rr
  -- `m`
  have q0 := tr_add A0.abs B0 (by norm_num) (by norm_num) (isFinite_of_add fm1).2
  have q1 := tr_add A1.abs B1 (by norm_num) (by norm_num) (isFinite_of_add fm2).2
  have q2 := tr_add A2.abs B2 (by norm_num) (by norm_num) (isFinite_of_add fm).2
  norm_num at q0 q1 q2
  have tca : Tr c.abs |C| |C| 9 := (Tr.round1 hc ec).abs.weaken (by norm_num) (abs_nonneg _)
  have m1 := tr_add tca q0 (by norm_num) (by norm_num) fm1
  norm_num at m1
  have m2 := tr_add m1 (q1.weaken (show (9 : ℚ) ≤ 11 by norm_num) q1.M_nonneg) (by norm_num) (by norm_num) fm2
  norm_num at m2
  have mm := tr_add m2 (q2.weaken (show (9 : ℚ) ≤ 13 by norm_num) q2.M_nonneg) (by norm_num) (by norm_num) fm
  norm_num at mm
  have fE : E12.isFinite := by decide
  have tq := tr_mul (Tr.exact_nn fE (by rw [E12_val]; norm_num) 0 le_rfl) mm (by norm_num) (by norm_num)
    (by norm_num) (by norm_num) fq zm
  norm_num at tq
  have tb := tr_add (rr.weaken (show (13 : ℚ) ≤ 17 by norm_num) rr.M_nonneg) tq (by norm_num) (by norm_num) fb
  norm_num at tb
  rw [E12_val] at tb
  -- the box point
  have k0 := in_box b0.1 b0.2
  have k1 := in_box b1.1 b1.2
  have k2 := in_box b2.1 b2.2
  have d0 := mid_half o0
  have d1 := mid_half o1
  have d2 := mid_half o2
  have e0 := mul_le_mul_of_nonneg_left k0 (abs_nonneg N0)
  have e1 := mul_le_mul_of_nonneg_left k1 (abs_nonneg N1)
  have e2 := mul_le_mul_of_nonneg_left k2 (abs_nonneg N2)
  have g0 := mul_le_mul_of_nonneg_left d0 (abs_nonneg N0)
  have g1 := mul_le_mul_of_nonneg_left d1 (abs_nonneg N1)
  have g2 := mul_le_mul_of_nonneg_left d2 (abs_nonneg N2)
  have hS : |(N0 * x0 + N1 * x1 + N2 * x2 - C) - (-C + N0 * ((l0.val + h0.val) / 2) +
      N1 * ((l1.val + h1.val) / 2) + N2 * ((l2.val + h2.val) / 2))| ≤
      |N0| * ((h0.val - l0.val) / 2) + |N1| * ((h1.val - l1.val) / 2) + |N2| * ((h2.val - l2.val) / 2) := by
    have : (N0 * x0 + N1 * x1 + N2 * x2 - C) - (-C + N0 * ((l0.val + h0.val) / 2) +
        N1 * ((l1.val + h1.val) / 2) + N2 * ((l2.val + h2.val) / 2)) =
        N0 * (x0 - (l0.val + h0.val) / 2) + N1 * (x1 - (l1.val + h1.val) / 2) + N2 * (x2 - (l2.val + h2.val) / 2) := by
      ring
    rw [this]
    refine le_trans (abs_add_three _ _ _) ?_
    rw [abs_mul, abs_mul, abs_mul]
    linarith
  have hve := vv.err
  have hlo := tb.err
  rw [abs_le] at hlo
  have nm0 := abs_nonneg N0; have nm1 := abs_nonneg N1; have nm2 := abs_nonneg N2
  have hC := abs_nonneg C
  have a0 : 0 ≤ |N0| * ((|l0.val| + |h0.val|) / 2) := by positivity
  have a1 : 0 ≤ |N1| * ((|l1.val| + |h1.val|) / 2) := by positivity
  have a2 : 0 ≤ |N2| * ((|l2.val| + |h2.val|) / 2) := by positivity
  have hm0 : |N0 * ((l0.val + h0.val) / 2)| = |N0| * |(l0.val + h0.val) / 2| := abs_mul _ _
  have hm1 : |N1 * ((l1.val + h1.val) / 2)| = |N1| * |(l1.val + h1.val) / 2| := abs_mul _ _
  have hm2 : |N2 * ((l2.val + h2.val) / 2)| = |N2| * |(l2.val + h2.val) / 2| := abs_mul _ _
  have hk0 : |N0| * ((h0.val - l0.val) / 2) ≤ |N0| * ((|l0.val| + |h0.val|) / 2) :=
    mul_le_mul_of_nonneg_left (by rcases abs_cases l0.val with ⟨u, _⟩ | ⟨u, _⟩ <;> rcases abs_cases h0.val with ⟨w, _⟩ | ⟨w, _⟩ <;> rw [u, w] <;> linarith) nm0
  have hk1 : |N1| * ((h1.val - l1.val) / 2) ≤ |N1| * ((|l1.val| + |h1.val|) / 2) :=
    mul_le_mul_of_nonneg_left (by rcases abs_cases l1.val with ⟨u, _⟩ | ⟨u, _⟩ <;> rcases abs_cases h1.val with ⟨w, _⟩ | ⟨w, _⟩ <;> rw [u, w] <;> linarith) nm1
  have hk2 : |N2| * ((h2.val - l2.val) / 2) ≤ |N2| * ((|l2.val| + |h2.val|) / 2) :=
    mul_le_mul_of_nonneg_left (by rcases abs_cases l2.val with ⟨u, _⟩ | ⟨u, _⟩ <;> rcases abs_cases h2.val with ⟨w, _⟩ | ⟨w, _⟩ <;> rw [u, w] <;> linarith) nm2
  have hlt : |(add (add (add c.neg (mul n0 (midF l0 h0))) (mul n1 (midF l1 h1))) (mul n2 (midF l2 h2))).val -
      (N0 * x0 + N1 * x1 + N2 * x2 - C)| <
      |(add (add (add c.neg (mul n0 (midF l0 h0))) (mul n1 (midF l1 h1))) (mul n2 (midF l2 h2))).val| := by
    have w := abs_sub_le (add (add (add c.neg (mul n0 (midF l0 h0))) (mul n1 (midF l1 h1))) (mul n2 (midF l2 h2))).val
      (-C + N0 * ((l0.val + h0.val) / 2) + N1 * ((l1.val + h1.val) / 2) + N2 * ((l2.val + h2.val) / 2))
      (N0 * x0 + N1 * x1 + N2 * x2 - C)
    rw [abs_sub_comm (N0 * x0 + N1 * x1 + N2 * x2 - C)] at hS
    linarith
  exact sign_close_d hlt

end IEEE754.Screen
