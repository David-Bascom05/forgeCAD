/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.FaceB

/-!
# decide-once A8's cycle-area orientation screen (41afccb, `ccw`)

A cycle of `n = N + 1` points, with shifted f64 coordinates `x_j = f_j,s − f0_s`, `y_j` (`x_0 = y_0 = 0`).
The code sums the `N` terms `x_j·y_{j+1} − x_{j+1}·y_j`, `j < N`, into `a2`. It leaves out the closing
term, which is `0` in f64. Alongside it sums `mag = Σ (|x_j y_{j+1}| + |x_{j+1} y_j|)` and
`ext = Σ_{j ≤ N} (|x_j| + |y_j|)`, and decides when
`|a2| > 4e·ext + 2n·e² + (n + 2)·2·EPSILON·mag`, where `e = 2·emax·(1 + 1e-12)`.

- `fold_err`: a left fold of rounded additions with terms each within `4ε·m_k` of `t_k` (`|t_k| ≤ m_k`)
  is within `(4 + 1.01K)ε·Σ m` of `Σ t`, for `Kε ≤ 1/1000`. There is no cap on `K`.
- `area_sound`: then the exact doubled signed area `Σ_{j ≤ N} (X_j Y_{j+1} − X_{j+1} Y_j)` of the
  closed polygon is `≠ 0`, with `a2`'s sign. The exact points are shifted by the same float `f0`, and the
  closing term counts on the exact side. Each exact shifted coordinate is within
  `emax + 2ε|x_j|` of `x_j` (`pt_diff_close` with `f0` exact).
-/

namespace IEEE754.Screen

open FP Finset

/-! ## Folds of rounded additions -/

/-- A left fold `s_{k+1} = s_k ⊕ d_k` from `0`. -/
def foldF (d : ℕ → F64) : ℕ → F64
  | 0 => F0
  | k + 1 => add (foldF d k) (d k)

theorem fold_err {d : ℕ → F64} {t m : ℕ → ℚ} (K : ℕ) (hK : (K : ℚ) * ε ≤ 1 / 1000)
    (fin : ∀ k ≤ K, (foldF d k).isFinite) (fd : ∀ k < K, (d k).isFinite)
    (hd : ∀ k < K, |(d k).val - t k| ≤ 4 * ε * m k) (ht : ∀ k < K, |t k| ≤ m k) :
    |(foldF d K).val - ∑ k ∈ range K, t k| ≤ (4 + 101 / 100 * K) * ε * ∑ k ∈ range K, m k := by
  induction K with
  | zero => simp [foldF, F0, FP.val]
  | succ K ih =>
    have hK' : (K : ℚ) * ε ≤ 1 / 1000 := le_trans (by push_cast at hK ⊢; nlinarith) hK
    have e := ih hK' (fun k hk => fin k (by omega)) (fun k hk => fd k (by omega))
      (fun k hk => hd k (by omega)) (fun k hk => ht k (by omega))
    rw [sum_range_succ, sum_range_succ]
    set S := (foldF d K).val
    set T := ∑ k ∈ range K, t k
    set M := ∑ k ∈ range K, m k
    have hM : 0 ≤ M := sum_nonneg fun k hk => le_trans (abs_nonneg _) (ht k (by simp at hk; omega))
    have r := add_rel0 (fin K (by omega)) (fd K (by omega)) (fin (K + 1) le_rfl)
    simp only [foldF] at r ⊢
    have dk := hd K (by omega)
    have tk := ht K (by omega)
    have mk : 0 ≤ m K := le_trans (abs_nonneg _) tk
    have hT : |T| ≤ M := le_trans (abs_sum_le_sum_abs _ _) (sum_le_sum fun k hk => ht k (by simp at hk; omega))
    have hK0 : (0 : ℚ) ≤ K := Nat.cast_nonneg K
    have hc : (4 + 101 / 100 * (K : ℚ)) * ε ≤ 1 / 100 := by nlinarith
    have p1 : (4 + 101 / 100 * (K : ℚ)) * ε * M ≤ 1 / 100 * M := mul_le_mul_of_nonneg_right hc hM
    set A := (add (foldF d K) (d K)).val
    set D := (d K).val
    have q : |S + D| ≤ |S - T| + |T| + (|D - t K| + |t K|) := by
      rw [show S + D = (S - T) + T + ((D - t K) + t K) by ring]
      exact le_trans (abs_add_three _ _ _) (by linarith [abs_add_le (D - t K) (t K)])
    have p2 : |S + D| ≤ 101 / 100 * (M + m K) := by nlinarith
    have tri : |A - (T + t K)| ≤ |A - (S + D)| + |S - T| + |D - t K| := by
      have := abs_add_three (A - (S + D)) (S - T) (D - t K)
      rwa [show A - (S + D) + (S - T) + (D - t K) = A - (T + t K) by ring] at this
    have p3 : 0 ≤ (101 / 100 * (K : ℚ)) * m K := by positivity
    push_cast
    nlinarith

/-! ## The screen -/

/-- `f64::EPSILON = 2^-52`. -/
def EPSF : F64 := .fin false 4503599627370496 (-104) (by decide)
theorem EPSF_val : EPSF.val = (1 / 2 ^ 52 : ℚ) := by simp only [EPSF, FP.val, FP.sgn]; norm_num

/-- The term `x_j·y_{j+1} − x_{j+1}·y_j`, its magnitude term, and `ext`'s term. -/
def termF (x y : ℕ → F64) (j : ℕ) : F64 := sub (mul (x j) (y (j + 1))) (mul (x (j + 1)) (y j))
def magT (x y : ℕ → F64) (j : ℕ) : F64 := add (mul (x j) (y (j + 1))).abs (mul (x (j + 1)) (y j)).abs
def extT (x y : ℕ → F64) (j : ℕ) : F64 := add (x j).abs (y j).abs

/-- `e = 2·emax·(1 + 1e-12)`. -/
def areaE (em : F64) : F64 := mul (mul F2 em) G12

/-- The bound `4e·ext + 2n·e·e + (n + 2)·2·EPSILON·mag`. -/
def areaB (nf e ext mag : F64) : F64 :=
  add (add (mul (mul F4 e) ext) (mul (mul (mul F2 nf) e) e)) (mul (mul (mul (add nf F2) F2) EPSF) mag)

lemma areaE_lo {em : F64} (h : em.isFinite) (h0 : 0 ≤ em.val) (hf : (areaE em).isFinite)
    (z1 : NZ (F2.val * em.val)) (z2 : NZ ((mul F2 em).val * G12.val)) :
    2 * (1 - 4 * ε) * em.val ≤ (areaE em).val := by
  unfold areaE at hf ⊢
  have f1 := (isFinite_of_mul hf).1
  have fG := (isFinite_of_mul hf).2
  have f2 : F2.isFinite := by decide
  have m1 := tr_mul (Tr.exact_nn f2 (by rw [F2_val]; norm_num) 0 le_rfl) (Tr.exact_nn h h0 0 le_rfl)
    (by norm_num) (by norm_num) (by norm_num) (by norm_num) f1 z1
  have m2 := tr_mul m1 (Tr.exact_nn fG (by have := G12_lo fG; linarith) 0 le_rfl) (by norm_num) (by norm_num)
    (by norm_num) (by norm_num) hf z2
  norm_num at m2
  have lo := m2.lower
  have g := G12_lo fG
  rw [F2_val] at lo
  have : 0 ≤ (1 - 4 * ε) * (2 * em.val) := by norm_num; positivity
  nlinarith

/-- The terms' trackers. -/
lemma term_tr {x y : ℕ → F64} {j : ℕ} (fx : (x j).isFinite) (fy : (y (j + 1)).isFinite)
    (gx : (x (j + 1)).isFinite) (gy : (y j).isFinite)
    (z1 : NZ ((x j).val * (y (j + 1)).val)) (z2 : NZ ((x (j + 1)).val * (y j).val))
    (ft : (termF x y j).isFinite) (fm : (magT x y j).isFinite) :
    |(termF x y j).val - ((x j).val * (y (j + 1)).val - (x (j + 1)).val * (y j).val)| ≤
        4 * ε * (|(x j).val| * |(y (j + 1)).val| + |(x (j + 1)).val| * |(y j).val|) ∧
      |(magT x y j).val - (|(x j).val| * |(y (j + 1)).val| + |(x (j + 1)).val| * |(y j).val|)| ≤
        4 * ε * (|(x j).val| * |(y (j + 1)).val| + |(x (j + 1)).val| * |(y j).val|) := by
  unfold termF at ft ⊢
  unfold magT at fm ⊢
  have m1 := tr_mul (Tr.exact fx) (Tr.exact fy) (by norm_num) (by norm_num) (by norm_num) (by norm_num)
    (isFinite_of_sub ft).1 z1
  have m2 := tr_mul (Tr.exact gx) (Tr.exact gy) (by norm_num) (by norm_num) (by norm_num) (by norm_num)
    (isFinite_of_sub ft).2 z2
  norm_num at m1 m2
  have sv := tr_sub m1 m2 (by norm_num) (by norm_num) ft
  have a1 := m1.abs
  have a2 := m2.abs
  rw [abs_mul] at a1 a2
  have sm := tr_add a1 a2 (by norm_num) (by norm_num) fm
  norm_num at sv sm
  exact ⟨sv.err, sm.err⟩

lemma ext_tr {x y : ℕ → F64} {j : ℕ} (fx : (x j).isFinite) (fy : (y j).isFinite) (fe : (extT x y j).isFinite) :
    |(extT x y j).val - (|(x j).val| + |(y j).val|)| ≤ 4 * ε * (|(x j).val| + |(y j).val|) := by
  unfold extT at fe ⊢
  have a1 := (Tr.exact fx).abs
  have a2 := (Tr.exact fy).abs
  have s := tr_add a1 a2 le_rfl (by norm_num) fe
  exact le_trans s.err (by have := add_nonneg (abs_nonneg (x j).val) (abs_nonneg (y j).val); nlinarith)

/-- `Σ_{j ≤ N} f(j+1) = Σ_{j ≤ N} f j` when `f(N+1) = f 0`. -/
lemma sum_shift (f : ℕ → ℚ) (N : ℕ) (h : f (N + 1) = f 0) :
    ∑ j ∈ range (N + 1), f (j + 1) = ∑ j ∈ range (N + 1), f j := by
  have := sum_range_succ' f (N + 1)
  rw [sum_range_succ, h] at this
  linarith

set_option maxHeartbeats 8000000 in
/-- **The cycle-area screen.** -/
theorem area_sound {N : ℕ} {x y : ℕ → F64} {X Y : ℕ → ℚ} {em nf : F64}
    (hN : ((N : ℚ) + 1) * ε ≤ 1 / 1000)
    (fx : ∀ j ≤ N + 1, (x j).isFinite ∧ (y j).isFinite)
    (z0 : (x 0).val = 0 ∧ (y 0).val = 0) (zN : (x (N + 1)).val = 0 ∧ (y (N + 1)).val = 0)
    (hX : X (N + 1) = X 0) (hY : Y (N + 1) = Y 0)
    (fem : em.isFinite) (em0 : 0 ≤ em.val)
    (cx : ∀ j ≤ N + 1, |(x j).val - X j| ≤ em.val + 2 * ε * |(x j).val| ∧
      |(y j).val - Y j| ≤ em.val + 2 * ε * |(y j).val|)
    (fnf : nf.isFinite) (vnf : nf.val = N + 1)
    (fa : ∀ k ≤ N, (foldF (termF x y) k).isFinite) (fm : ∀ k ≤ N, (foldF (magT x y) k).isFinite)
    (fe : ∀ k ≤ N + 1, (foldF (extT x y) k).isFinite)
    (fb : (areaB nf (areaE em) (foldF (extT x y) (N + 1)) (foldF (magT x y) N)).isFinite)
    (zt : ∀ j < N, NZ ((x j).val * (y (j + 1)).val) ∧ NZ ((x (j + 1)).val * (y j).val))
    (ze : NZ (F2.val * em.val) ∧ NZ ((mul F2 em).val * G12.val))
    (zb : ∀ r ∈ [F4.val * (areaE em).val, (mul F4 (areaE em)).val * (foldF (extT x y) (N + 1)).val,
      F2.val * nf.val, (mul F2 nf).val * (areaE em).val, (mul (mul F2 nf) (areaE em)).val * (areaE em).val,
      (add nf F2).val * F2.val, (mul (add nf F2) F2).val * EPSF.val,
      (mul (mul (add nf F2) F2) EPSF).val * (foldF (magT x y) N).val], NZ r)
    (hdec : (areaB nf (areaE em) (foldF (extT x y) (N + 1)) (foldF (magT x y) N)).val <
      |(foldF (termF x y) N).val|) :
    (∑ j ∈ range (N + 1), (X j * Y (j + 1) - X (j + 1) * Y j)) ≠ 0 ∧
      (0 < ∑ j ∈ range (N + 1), (X j * Y (j + 1) - X (j + 1) * Y j) ↔ 0 < (foldF (termF x y) N).val) := by
  -- the float-side terms
  set xv : ℕ → ℚ := fun j => (x j).val
  set yv : ℕ → ℚ := fun j => (y j).val
  set t : ℕ → ℚ := fun j => xv j * yv (j + 1) - xv (j + 1) * yv j
  set m : ℕ → ℚ := fun j => |xv j| * |yv (j + 1)| + |xv (j + 1)| * |yv j|
  set w : ℕ → ℚ := fun j => |xv j| + |yv j|
  have tm : ∀ j < N, |(termF x y j).val - t j| ≤ 4 * ε * m j ∧ |(magT x y j).val - m j| ≤ 4 * ε * m j :=
    fun j hj => term_tr (fx j (by omega)).1 (fx (j + 1) (by omega)).2 (fx (j + 1) (by omega)).1
      (fx j (by omega)).2 (zt j hj).1 (zt j hj).2
      (by have := fa (j + 1) (by omega); simp only [foldF] at this; exact (isFinite_of_add this).2)
      (by have := fm (j + 1) (by omega); simp only [foldF] at this; exact (isFinite_of_add this).2)
  have tle : ∀ j, |t j| ≤ m j := fun j => by
    simp only [t, m]
    have := abs_sub_le_add (xv j * yv (j + 1)) (xv (j + 1) * yv j); rwa [abs_mul, abs_mul] at this
  have hNK : (N : ℚ) * ε ≤ 1 / 1000 := le_trans (by nlinarith) hN
  have A2 := fold_err (t := t) (m := m) N hNK fa
    (fun k hk => by have := fa (k + 1) (by omega); simp only [foldF] at this; exact (isFinite_of_add this).2)
    (fun k hk => (tm k hk).1) (fun k _ => tle k)
  have MG := fold_err (t := m) (m := m) N hNK fm
    (fun k hk => by have := fm (k + 1) (by omega); simp only [foldF] at this; exact (isFinite_of_add this).2)
    (fun k hk => (tm k hk).2) (fun k _ => by rw [abs_of_nonneg (by simp only [m]; positivity)])
  have EX := fold_err (t := w) (m := w) (N + 1) (by push_cast; exact hN) fe
    (fun k hk => by have := fe (k + 1) (by omega); simp only [foldF] at this; exact (isFinite_of_add this).2)
    (fun k hk => ext_tr (fx k (by omega)).1 (fx k (by omega)).2
      (by have := fe (k + 1) (by omega); simp only [foldF] at this; exact (isFinite_of_add this).2))
    (fun k _ => by rw [abs_of_nonneg (by simp only [w]; positivity)])
  set T := ∑ k ∈ range N, t k
  set MAG := ∑ k ∈ range N, m k
  set EXT := ∑ k ∈ range (N + 1), w k
  have hMAG : 0 ≤ MAG := sum_nonneg fun k _ => by simp only [m]; positivity
  have hEXT : 0 ≤ EXT := sum_nonneg fun k _ => by simp only [w]; positivity
  -- the exact side against the float side
  set E := em.val
  have perr : ∀ j ≤ N, |(X j * Y (j + 1) - X (j + 1) * Y j) - t j| ≤
      2 * E ^ 2 + (1 + 2 * ε) * E * (w j + w (j + 1)) + (4 * ε + 4 * ε ^ 2) * m j := by
    intro j hj
    have q1 := prod_err (cx j (by omega)).1 (cx (j + 1) (by omega)).2 em0
    have q2 := prod_err (cx (j + 1) (by omega)).1 (cx j (by omega)).2 em0
    have := abs_sub_le_add (xv j * yv (j + 1) - X j * Y (j + 1)) (xv (j + 1) * yv j - X (j + 1) * Y j)
    rw [abs_sub_comm]
    simp only [t, m, w, xv, yv] at this ⊢
    rw [show (x j).val * (y (j + 1)).val - (x (j + 1)).val * (y j).val - (X j * Y (j + 1) - X (j + 1) * Y j) =
      (x j).val * (y (j + 1)).val - X j * Y (j + 1) - ((x (j + 1)).val * (y j).val - X (j + 1) * Y j) by ring]
    nlinarith
  have tN : t N = 0 := by simp only [t, xv, yv, zN.1, zN.2]; ring
  have mN : m N = 0 := by simp only [m, xv, yv, zN.1, zN.2]; simp
  have wsh : ∑ j ∈ range (N + 1), w (j + 1) = EXT := sum_shift w N (by simp only [w, xv, yv, zN.1, zN.2, z0.1, z0.2])
  have hS : |(∑ j ∈ range (N + 1), (X j * Y (j + 1) - X (j + 1) * Y j)) - T| ≤
      2 * (N + 1) * E ^ 2 + 2 * (1 + 2 * ε) * E * EXT + (4 * ε + 4 * ε ^ 2) * MAG := by
    have e1 : T = ∑ j ∈ range (N + 1), t j := by rw [sum_range_succ, tN, _root_.add_zero]
    have e2 : MAG = ∑ j ∈ range (N + 1), m j := by rw [sum_range_succ, mN, _root_.add_zero]
    rw [e1, ← sum_sub_distrib]
    refine le_trans (abs_sum_le_sum_abs _ _) (le_trans (sum_le_sum fun j hj => perr j (by simp at hj; omega)) ?_)
    apply le_of_eq
    rw [e2]
    simp only [sum_add_distrib, ← mul_sum, wsh, sum_const, card_range, nsmul_eq_mul]
    push_cast; ring
  -- the bound, from below
  have fbu := fb
  unfold areaB at fbu
  have he := areaE_lo fem em0 (isFinite_of_mul (isFinite_of_mul (isFinite_of_add (isFinite_of_add fbu).1).1).1).2
    ze.1 ze.2
  set ef := (areaE em).val
  set extf := (foldF (extT x y) (N + 1)).val
  set magf := (foldF (magT x y) N).val
  have he0 : 0 ≤ ef := le_trans (mul_nonneg (by norm_num) em0) he
  have hext : 99 / 100 * EXT ≤ extf := by
    rw [abs_le] at EX; push_cast at EX hN
    have : (4 + 101 / 100 * ((N : ℚ) + 1)) * ε * EXT ≤ 1 / 100 * EXT := mul_le_mul_of_nonneg_right (by nlinarith) hEXT
    linarith [EX.1]
  have hmag : 99 / 100 * MAG ≤ magf := by
    rw [abs_le] at MG
    have : (4 + 101 / 100 * (N : ℚ)) * ε * MAG ≤ 1 / 100 * MAG := mul_le_mul_of_nonneg_right (by nlinarith) hMAG
    linarith [MG.1]
  have hext0 : 0 ≤ extf := le_trans (by positivity) hext
  have hmag0 : 0 ≤ magf := le_trans (by positivity) hmag
  unfold areaB at fb hdec
  have u : ∀ i, (h : i < 8) → NZ ([F4.val * ef, (mul F4 (areaE em)).val * extf, F2.val * nf.val,
      (mul F2 nf).val * ef, (mul (mul F2 nf) (areaE em)).val * ef, (add nf F2).val * F2.val,
      (mul (add nf F2) F2).val * EPSF.val, (mul (mul (add nf F2) F2) EPSF).val * magf].get ⟨i, by simpa using h⟩) :=
    fun i h => zb _ (List.get_mem _ _)
  have fAB := (isFinite_of_add fb).1
  have fC := (isFinite_of_add fb).2
  have fA := (isFinite_of_add fAB).1
  have fB := (isFinite_of_add fAB).2
  have f4 : F4.isFinite := by decide
  have f2 : F2.isFinite := by decide
  have fE : EPSF.isFinite := by decide
  have tF4 := Tr.exact_nn f4 (by rw [F4_val]; norm_num) 0 le_rfl
  have tF2 := Tr.exact_nn f2 (by rw [F2_val]; norm_num) 0 le_rfl
  have tE := Tr.exact_nn fE (by rw [EPSF_val]; norm_num) 0 le_rfl
  have te := Tr.exact_nn (isFinite_of_mul (isFinite_of_mul fA).1).2 he0 0 le_rfl
  have tx := Tr.exact_nn (isFinite_of_mul fA).2 hext0 0 le_rfl
  have tg := Tr.exact_nn (isFinite_of_mul fC).2 hmag0 0 le_rfl
  have tn := Tr.exact_nn fnf (by rw [vnf]; positivity) 0 le_rfl
  have a1 := tr_mul tF4 te (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_mul fA).1 (u 0 (by norm_num))
  have a2 := tr_mul a1 tx (by norm_num) (by norm_num) (by norm_num) (by norm_num) fA (u 1 (by norm_num))
  have b1 := tr_mul tF2 tn (by norm_num) (by norm_num) (by norm_num) (by norm_num)
    (isFinite_of_mul (isFinite_of_mul fB).1).1 (u 2 (by norm_num))
  have b2 := tr_mul b1 te (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_mul fB).1 (u 3 (by norm_num))
  have b3 := tr_mul b2 te (by norm_num) (by norm_num) (by norm_num) (by norm_num) fB (u 4 (by norm_num))
  have c1 := tr_add tn tF2 le_rfl (by norm_num) (isFinite_of_mul (isFinite_of_mul (isFinite_of_mul fC).1).1).1
  have c2 := tr_mul c1 tF2 (by norm_num) (by norm_num) (by norm_num) (by norm_num)
    (isFinite_of_mul (isFinite_of_mul fC).1).1 (u 5 (by norm_num))
  have c3 := tr_mul c2 tE (by norm_num) (by norm_num) (by norm_num) (by norm_num) (isFinite_of_mul fC).1 (u 6 (by norm_num))
  have c4 := tr_mul c3 tg (by norm_num) (by norm_num) (by norm_num) (by norm_num) fC (u 7 (by norm_num))
  norm_num at a2 b3 c4
  have ab := tr_add (a2.weaken (show (4 : ℚ) ≤ 6 by norm_num) a2.M_nonneg) b3 (by norm_num) (by norm_num) fAB
  norm_num at ab
  have tb := tr_add ab c4 (by norm_num) (by norm_num) fb
  norm_num at tb
  have hlow := tb.lower
  rw [F4_val, F2_val, EPSF_val, vnf] at hlow
  -- the products
  have k0 : 0 ≤ 2 * (1 - 4 * ε) * E := mul_nonneg (by norm_num) em0
  have p1 : 2 * (1 - 4 * ε) * E * (99 / 100 * EXT) ≤ ef * extf := mul_le_mul he hext (by positivity) he0
  have p2 : (2 * (1 - 4 * ε) * E) * (2 * (1 - 4 * ε) * E) ≤ ef * ef := mul_le_mul he he k0 he0
  have hN0 : (0 : ℚ) ≤ N := Nat.cast_nonneg N
  have p3 := mul_le_mul_of_nonneg_left p2 hN0
  have p4 := mul_le_mul_of_nonneg_left hmag hN0
  have p5 : 0 ≤ (N : ℚ) * MAG := mul_nonneg hN0 hMAG
  have p6 : 0 ≤ (N : ℚ) * E ^ 2 := by positivity
  have p7 : 0 ≤ E * EXT := mul_nonneg em0 hEXT
  have hve := A2
  have hlt : |(foldF (termF x y) N).val - ∑ j ∈ range (N + 1), (X j * Y (j + 1) - X (j + 1) * Y j)| <
      |(foldF (termF x y) N).val| := by
    have w := abs_sub_le (foldF (termF x y) N).val T (∑ j ∈ range (N + 1), (X j * Y (j + 1) - X (j + 1) * Y j))
    rw [abs_sub_comm T] at w
    nlinarith
  exact sign_close_d hlt

end IEEE754.Screen
