/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Screen.Meet
import IEEE754.Screen.Gap
import IEEE754.Compare

/-!
# decide-once's `Pts::first_meet_f` (item 5)

`first_meet_f` runs the same nested scan as `vertex_at` over the planes around a vertex (by
position). It skips a position triple with a repeat. It returns the f64 meet of the first triple
`meet_f` accepts, provided that meet's bound is within `1e-13·(1 + max|p_k|)`. When `meet_f`
refuses a triple, it goes on only if the exact determinant is zero; otherwise it returns `None`.

- `searchF_sound`: the scan's answer comes from the first exactly independent triple, which is the
  triple `vertex_at` and `ensure` meet (`meet3` is `Some` exactly when `det ≠ 0`). This needs two
  facts: a repeated triple is dependent (`rep_dep`, two equal rows), and `meet_f` accepting means
  `det ≠ 0` (item 1).
- `meetF_sound`: the Rust `meet_f` returns `Some ((p0, p1, p2), err)` only with `det ≠ 0` and every
  `|p_k − X_k| ≤ err`. This takes item 1's `meet_sound` side conditions (no underflow, every
  intermediate finite), with `|det| > 4·edet` read off the f64 comparison.
- `tolF_hi`: the f64 tolerance `1e-13 * (1.0 + max|p_k|)` is at most `(1 + 3ε)·c13·(1 + max|p_k|)`,
  where `c13` is the f64 `1e-13`.
- `first_meet_f_sound`: these combined.
-/

namespace IEEE754.Screen

open FP

/-! ## The scan -/

/-- The nested `for x, for y, for z` over positions `0..n`, in order. -/
def trip (n : ℕ) : List (ℕ × ℕ × ℕ) :=
  (List.range n).flatMap fun x => (List.range n).flatMap fun y => (List.range n).map fun z => (x, y, z)

/-- `x == y || y == z || x == z`. -/
def rep (t : ℕ × ℕ × ℕ) : Bool := t.1 == t.2.1 || t.2.1 == t.2.2 || t.1 == t.2.2

/-- `first_meet_f`'s loop: `mf` is `meet_f`, `dep` the exact `det == 0`, `acc` the acceptance. -/
def searchF {P E : Type} (mf : ℕ × ℕ × ℕ → Option (P × E)) (dep : ℕ × ℕ × ℕ → Bool)
    (acc : P → E → Bool) : List (ℕ × ℕ × ℕ) → Option P
  | [] => none
  | t :: ts =>
    if rep t then searchF mf dep acc ts else
      match mf t with
      | some (p, e) => if acc p e then some p else none
      | none => if dep t then searchF mf dep acc ts else none

theorem searchF_sound {P E : Type} {mf : ℕ × ℕ × ℕ → Option (P × E)} {dep : ℕ × ℕ × ℕ → Bool}
    {acc : P → E → Bool} (hrep : ∀ t, rep t = true → dep t = true)
    (hmf : ∀ t p e, mf t = some (p, e) → dep t = false) :
    ∀ (ts : List (ℕ × ℕ × ℕ)) (p : P), searchF mf dep acc ts = some p →
      ∃ t e, ts.find? (fun t => !dep t) = some t ∧ mf t = some (p, e) ∧ acc p e = true := by
  intro ts
  induction ts with
  | nil => intro p h; simp [searchF] at h
  | cons t ts ih =>
    intro p h
    simp only [searchF] at h
    by_cases hr : rep t = true
    · simp only [hr, ↓reduceIte] at h
      obtain ⟨u, e, h1, h2, h3⟩ := ih p h
      exact ⟨u, e, by simp [List.find?, hrep t hr, h1], h2, h3⟩
    · simp only [hr, Bool.false_eq_true, ↓reduceIte] at h
      cases hm : mf t with
      | none =>
        rw [hm] at h
        by_cases hd : dep t = true
        · simp only [hd, ↓reduceIte] at h
          obtain ⟨u, e, h1, h2, h3⟩ := ih p h
          exact ⟨u, e, by simp [List.find?, hd, h1], h2, h3⟩
        · simp only [hd, Bool.false_eq_true, ↓reduceIte] at h; simp at h
      | some pe =>
        obtain ⟨q, e⟩ := pe
        rw [hm] at h
        simp only at h
        by_cases ha : acc q e = true
        · simp only [ha, ↓reduceIte] at h
          cases h
          exact ⟨t, e, by simp [List.find?, hmf t p e hm], hm, ha⟩
        · simp only [ha, Bool.false_eq_true, ↓reduceIte] at h; simp at h

/-- A triple with a repeated plane has zero determinant (two equal rows). -/
lemma rep_dep (A B C : ℚ × ℚ × ℚ) (h : A = B ∨ B = C ∨ A = C) :
    A.1 * (B.2.1 * C.2.2 - B.2.2 * C.2.1) + A.2.1 * (B.2.2 * C.1 - B.1 * C.2.2)
      + A.2.2 * (B.1 * C.2.1 - B.2.1 * C.1) = 0 := by
  rcases h with rfl | rfl | rfl <;> ring

/-! ## The acceptance -/

/-- The f64 literal `1e-13`. -/
def E13 : F64 := .fin false 7922816251426434 (-96) (by decide)
theorem E13_bits : E13.toBits = 0x3d3c25c268497682 := by decide
theorem E13_val : E13.val = (3961408125713217 / 2 ^ 95 : ℚ) := by simp only [E13, val, sgn]; norm_num

/-- `0.0`, the start of `err`'s fold. -/
def F0 : F64 := .zero false

/-- `1.0 + p0.abs().max(p1.abs()).max(p2.abs())`. -/
def sizeF (p0 p1 p2 : F64) : F64 := add F1 (fmax (fmax p0.abs p1.abs) p2.abs)
/-- `1e-13 * m`. -/
def tolF (p0 p1 p2 : F64) : F64 := mul E13 (sizeF p0 p1 p2)
/-- `err`, folded with `max` from `0.0`. -/
def errMax (e0 e1 e2 : F64) : F64 := fmax (fmax (fmax F0 e0) e1) e2

lemma errMax_ge {e0 e1 e2 : F64} :
    e0.val ≤ (errMax e0 e1 e2).val ∧ e1.val ≤ (errMax e0 e1 e2).val ∧ e2.val ≤ (errMax e0 e1 e2).val := by
  simp only [errMax, fmax_val]
  refine ⟨?_, ?_, ?_⟩
  · exact le_trans (le_max_right _ _) (le_trans (le_max_left _ _) (le_max_left _ _))
  · exact le_trans (le_max_right _ _) (le_max_left _ _)
  · exact le_max_right _ _

lemma errMax_fin {e0 e1 e2 : F64} (h0 : e0.isFinite) (h1 : e1.isFinite) (h2 : e2.isFinite) :
    (errMax e0 e1 e2).isFinite :=
  fmax_fin (fmax_fin (fmax_fin (by decide) h0) h1) h2

theorem tolF_hi {p0 p1 p2 : F64} (h0 : p0.isFinite) (h1 : p1.isFinite) (h2 : p2.isFinite)
    (ht : (tolF p0 p1 p2).isFinite) :
    (tolF p0 p1 p2).val ≤
      (1 + 3 * ε) * (3961408125713217 / 2 ^ 95) * (1 + max (max |p0.val| |p1.val|) |p2.val|) := by
  unfold tolF sizeF at ht ⊢
  have fs := (isFinite_of_mul ht).2
  have fO : F1.isFinite := by decide
  have fM : (fmax (fmax p0.abs p1.abs) p2.abs).isFinite :=
    fmax_fin (fmax_fin (by simpa using h0) (by simpa using h1)) (by simpa using h2)
  have hMv : (fmax (fmax p0.abs p1.abs) p2.abs).val = max (max |p0.val| |p1.val|) |p2.val| := by
    simp only [fmax_val, FP.abs_val]
  set M := max (max |p0.val| |p1.val|) |p2.val| with hM
  have M0 : 0 ≤ M := le_trans (abs_nonneg _) (le_max_right _ _)
  have es := add_rel0 fO fM fs
  rw [F1_val, hMv] at es
  have hs : (add F1 (fmax (fmax p0.abs p1.abs) p2.abs)).val ≤ (1 + ε) * (1 + M) := by
    rw [abs_of_nonneg (by linarith : (0 : ℚ) ≤ 1 + M)] at es
    have := (abs_le.mp es).2; linarith
  have hs0 : 0 ≤ (add F1 (fmax (fmax p0.abs p1.abs) p2.abs)).val := by
    rw [abs_of_nonneg (by linarith : (0 : ℚ) ≤ 1 + M)] at es
    have := (abs_le.mp es).1
    have : ε * (1 + M) ≤ (1 + M) / 2 := by nlinarith
    linarith
  have fE : E13.isFinite := by decide
  have em := mul_rel fE fs ht
  rw [E13_val] at em
  rw [abs_of_nonneg (by positivity : (0 : ℚ) ≤ 3961408125713217 / 2 ^ 95 *
    (add F1 (fmax (fmax p0.abs p1.abs) p2.abs)).val)] at em
  have hη := η_small
  have e1 := (abs_le.mp em).2
  have key : (3961408125713217 / 2 ^ 95 : ℚ) * (add F1 (fmax (fmax p0.abs p1.abs) p2.abs)).val ≤
      (3961408125713217 / 2 ^ 95) * ((1 + ε) * (1 + M)) :=
    mul_le_mul_of_nonneg_left hs (by norm_num)
  have hηM : η ≤ ε * (3961408125713217 / 2 ^ 95) * (1 + M) / 2 := by
    have : (1 : ℚ) ≤ 1 + M := by linarith
    have : (1 / 10 ^ 50 : ℚ) ≤ ε * (3961408125713217 / 2 ^ 95) / 2 := by norm_num
    nlinarith
  nlinarith

/-! ## `meet_f` -/

/-- The literal `4.0`. -/
def F4 : F64 := .fin false 4503599627370496 (-50) (by decide)
theorem F4_val : F4.val = (4 : ℚ) := by simp only [F4, val, sgn]; norm_num

/-- Rust's `meet_f` on planes `a·x = oa`, `b·x = ob`, `c·x = oc`: `None` when
`det.abs() <= 4.0 * edet` or a coordinate is not finite; else the point and `err`. -/
def meetF (a0 a1 a2 oa b0 b1 b2 ob c0 c1 c2 oc : F64) : Option ((F64 × F64 × F64) × F64) :=
  let det := (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))
  let edet := (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)))
  if det.abs.le (mul F4 edet) then none else
  let p0 := div (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)) det
  let p1 := div (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)) det
  let p2 := div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) det
  if p0.isFinite && p1.isFinite && p2.isFinite then
    some ((p0, p1, p2), errMax (errTerm p0 det edet (dot3 oa.abs ob.abs oc.abs (cab b1 b2 c1 c2) (cab c1 c2 a1 a2) (cab a1 a2 b1 b2))) (errTerm p1 det edet (dot3 oa.abs ob.abs oc.abs (cab b2 b0 c2 c0) (cab c2 c0 a2 a0) (cab a2 a0 b2 b0)))
      (errTerm p2 det edet (dot3 oa.abs ob.abs oc.abs (cab b0 b1 c0 c1) (cab c0 c1 a0 a1) (cab a0 a1 b0 b1))))
  else none

/-- `4.0 * edet` is exact for the `edet` of a finite `dabs`. -/
lemma edet_four {d : F64} (hd : d.isFinite) :
    (mul GAMMA d).isFinite ∧ (mul F4 (mul GAMMA d)).isFinite ∧
      (mul F4 (mul GAMMA d)).val = 4 * (mul GAMMA d).val := by
  have fG : GAMMA.isFinite := by decide
  have hGv : GAMMA.val = (1 / 2 ^ 46 : ℚ) := by simp only [GAMMA, val, sgn]; norm_num
  have fe : (mul GAMMA d).isFinite := by
    rw [mul_eq_round fG hd]
    refine round_finite_of_le hd ?_
    rw [hGv, abs_mul]
    have := abs_nonneg d.val
    have : |(1 / 2 ^ 46 : ℚ)| = 1 / 2 ^ 46 := by norm_num
    nlinarith
  have em := mul_rel fG hd fe
  rw [hGv] at em
  have hB := abs_val_lt hd
  simp only [binary64_emax] at hB
  generalize hBB : (2 : ℚ) ^ ((1023 : ℤ) + 1) = B at hB
  have hB1 : (1 : ℚ) ≤ B := by rw [← hBB]; exact one_le_zpow₀ (by norm_num) (by norm_num)
  have hη := η_small
  have hle : |(mul GAMMA d).val| * 4 < B := by
    have e1 := abs_sub_abs_le_abs_sub (mul GAMMA d).val (1 / 2 ^ 46 * d.val)
    rw [abs_mul, show |(1 / 2 ^ 46 : ℚ)| = 1 / 2 ^ 46 by norm_num] at em e1
    have := abs_nonneg d.val
    nlinarith
  obtain ⟨e, he, hm, hlt⟩ := mult_val fe
  have f4 : F4.isFinite := by decide
  have hr : Rep binary64 (F4.val * (mul GAMMA d).val) := by
    rw [F4_val]
    refine rep_of_mult (mult_mul (show Mult (4 : ℚ) 2 from ⟨1, by norm_num⟩) hm) ?_ ?_ ?_
    · simp only [binary64_qmin] at he ⊢; omega
    · rw [abs_mul, two_zpow_add, show |(4 : ℚ)| = 2 ^ (2 : ℤ) by norm_num]
      have : (0 : ℚ) < 2 ^ (2 : ℤ) := by positivity
      nlinarith
    · simp only [binary64_emax]; rw [hBB, abs_mul, show |(4 : ℚ)| = 4 by norm_num]; linarith
  obtain ⟨h1, h2⟩ := mul_of_rep f4 fe hr
  exact ⟨fe, h1, by rw [h2, F4_val]⟩

set_option maxHeartbeats 2000000 in
/-- `meet_f`'s `Some` is item 1's case: the exact `det ≠ 0` and each coordinate within `err`. -/
theorem meetF_sound {a0 a1 a2 b0 b1 b2 c0 c1 c2 oa ob oc : F64} {A0 A1 A2 B0 B1 B2 C0 C1 C2 Oa Ob Oc : ℚ}
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
      (div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs.val * GAMMA.val], NZ r)
    {p0 p1 p2 err : F64} (hS : meetF a0 a1 a2 oa b0 b1 b2 ob c0 c1 c2 oc = some ((p0, p1, p2), err)) :
    (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0)) ≠ 0 ∧
      |p0.val - (Oa * (B1 * C2 - B2 * C1) + Ob * (C1 * A2 - C2 * A1) + Oc * (A1 * B2 - A2 * B1)) / (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0))| ≤ err.val ∧
      |p1.val - (Oa * (B2 * C0 - B0 * C2) + Ob * (C2 * A0 - C0 * A2) + Oc * (A2 * B0 - A0 * B2)) / (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0))| ≤ err.val ∧
      |p2.val - (Oa * (B0 * C1 - B1 * C0) + Ob * (C0 * A1 - C1 * A0) + Oc * (A0 * B1 - A1 * B0)) / (A0 * (B1 * C2 - B2 * C1) + A1 * (B2 * C0 - B0 * C2) + A2 * (B0 * C1 - B1 * C0))| ≤ err.val ∧
      err.isFinite ∧ p0.isFinite ∧ p1.isFinite ∧ p2.isFinite := by
  obtain ⟨fe, f4, h4⟩ := edet_four hfa
  unfold meetF at hS
  simp only at hS
  split_ifs at hS with hc hp
  simp only [Option.some.injEq, Prod.mk.injEq] at hS
  obtain ⟨⟨rfl, rfl, rfl⟩, rfl⟩ := hS
  have fda : ((dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).abs.isFinite := by simpa using hfd
  have hsome : 4 * ((mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1)))).val < |((dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))).val| := by
    rw [Bool.not_eq_true, ← Bool.not_eq_true, le_iff fda f4, FP.abs_val, h4] at hc
    exact lt_of_not_ge hc
  obtain ⟨hD, e0, e1, e2⟩ := meet_sound ha0 ha1 ha2 hb0 hb1 hb2 hc0 hc1 hc2 hoa hob hoc hfd hfa hsome
    hfn0 hfn1 hfn2 hfm0 hfm1 hfm2 hfe0 hfe1 hfe2 hUd hU0 hU1 hU2
  obtain ⟨g0, g1, g2⟩ := errMax_ge (e0 := errTerm (div (dot3 oa ob oc (crs b1 b2 c1 c2) (crs c1 c2 a1 a2) (crs a1 a2 b1 b2)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b1 b2 c1 c2) (cab c1 c2 a1 a2) (cab a1 a2 b1 b2)))
    (e1 := errTerm (div (dot3 oa ob oc (crs b2 b0 c2 c0) (crs c2 c0 a2 a0) (crs a2 a0 b2 b0)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b2 b0 c2 c0) (cab c2 c0 a2 a0) (cab a2 a0 b2 b0)))
    (e2 := errTerm (div (dot3 oa ob oc (crs b0 b1 c0 c1) (crs c0 c1 a0 a1) (crs a0 a1 b0 b1)) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1))) (dot3 a0 a1 a2 (crs b1 b2 c1 c2) (crs b2 b0 c2 c0) (crs b0 b1 c0 c1)) (mul GAMMA (dot3 a0.abs a1.abs a2.abs (cab b1 b2 c1 c2) (cab b2 b0 c2 c0) (cab b0 b1 c0 c1))) (dot3 oa.abs ob.abs oc.abs (cab b0 b1 c0 c1) (cab c0 c1 a0 a1) (cab a0 a1 b0 b1)))
  exact ⟨hD, le_trans e0 g0, le_trans e1 g1, le_trans e2 g2, errMax_fin hfe0 hfe1 hfe2, by simp only [Bool.and_eq_true] at hp; exact ⟨hp.1.1, hp.1.2, hp.2⟩⟩

/-! ## `first_meet_f` -/

/-- `a · (b × c)` of three normals. -/
def det3 (A B C : ℚ × ℚ × ℚ) : ℚ :=
  A.1 * (B.2.1 * C.2.2 - B.2.2 * C.2.1) + A.2.1 * (B.2.2 * C.1 - B.1 * C.2.2)
    + A.2.2 * (B.1 * C.2.1 - B.2.1 * C.1)

/-- The normal of an exact plane `(n0, n1, n2, c)`. -/
def nrm (P : ℚ × ℚ × ℚ × ℚ) : ℚ × ℚ × ℚ := (P.1, P.2.1, P.2.2.1)

/-- `meet3`'s point, Cramer's rule: `(c_a (b×c) + c_b (c×a) + c_c (a×b)) / det`. -/
def meetQ (A B C : ℚ × ℚ × ℚ × ℚ) : ℚ × ℚ × ℚ :=
  let D := det3 (nrm A) (nrm B) (nrm C)
  ((A.2.2.2 * (B.2.1 * C.2.2.1 - B.2.2.1 * C.2.1) + B.2.2.2 * (C.2.1 * A.2.2.1 - C.2.2.1 * A.2.1)
      + C.2.2.2 * (A.2.1 * B.2.2.1 - A.2.2.1 * B.2.1)) / D,
   (A.2.2.2 * (B.2.2.1 * C.1 - B.1 * C.2.2.1) + B.2.2.2 * (C.2.2.1 * A.1 - C.1 * A.2.2.1)
      + C.2.2.2 * (A.2.2.1 * B.1 - A.1 * B.2.2.1)) / D,
   (A.2.2.2 * (B.1 * C.2.1 - B.2.1 * C.1) + B.2.2.2 * (C.1 * A.2.1 - C.2.1 * A.1)
      + C.2.2.2 * (A.1 * B.2.1 - A.2.1 * B.1)) / D)

/-- `first_meet_f` over the planes round a vertex, by position: `sp` the f64 planes (`spf`),
`pl` the exact ones. -/
def firstMeetF (n : ℕ) (sp : ℕ → F64 × F64 × F64 × F64) (pl : ℕ → ℚ × ℚ × ℚ × ℚ) :
    Option (F64 × F64 × F64) :=
  searchF
    (fun t => meetF (sp t.1).1 (sp t.1).2.1 (sp t.1).2.2.1 (sp t.1).2.2.2
      (sp t.2.1).1 (sp t.2.1).2.1 (sp t.2.1).2.2.1 (sp t.2.1).2.2.2
      (sp t.2.2).1 (sp t.2.2).2.1 (sp t.2.2).2.2.1 (sp t.2.2).2.2.2)
    (fun t => decide (det3 (nrm (pl t.1)) (nrm (pl t.2.1)) (nrm (pl t.2.2)) = 0))
    (fun p e => e.le (tolF p.1 p.2.1 p.2.2))
    (trip n)

/-- `vertex_at`'s (and `ensure`'s) triple: the first in the same scan whose `meet3` is `Some`,
i.e. whose exact `det ≠ 0`. -/
def firstExact (n : ℕ) (pl : ℕ → ℚ × ℚ × ℚ × ℚ) : Option (ℕ × ℕ × ℕ) :=
  (trip n).find? fun t => !decide (det3 (nrm (pl t.1)) (nrm (pl t.2.1)) (nrm (pl t.2.2)) = 0)

/-- `meetF_sound`'s conclusion, for planes `A`, `B`, `C` by components, is `hok`'s. -/
lemma ok_of_sound {A B C : ℚ × ℚ × ℚ × ℚ} {p0 p1 p2 err : F64}
    (h : (A.1 * (B.2.1 * C.2.2.1 - B.2.2.1 * C.2.1) + A.2.1 * (B.2.2.1 * C.1 - B.1 * C.2.2.1) + A.2.2.1 * (B.1 * C.2.1 - B.2.1 * C.1)) ≠ 0 ∧
      |p0.val - (A.2.2.2 * (B.2.1 * C.2.2.1 - B.2.2.1 * C.2.1) + B.2.2.2 * (C.2.1 * A.2.2.1 - C.2.2.1 * A.2.1) + C.2.2.2 * (A.2.1 * B.2.2.1 - A.2.2.1 * B.2.1)) / (A.1 * (B.2.1 * C.2.2.1 - B.2.2.1 * C.2.1) + A.2.1 * (B.2.2.1 * C.1 - B.1 * C.2.2.1) + A.2.2.1 * (B.1 * C.2.1 - B.2.1 * C.1))| ≤ err.val ∧
      |p1.val - (A.2.2.2 * (B.2.2.1 * C.1 - B.1 * C.2.2.1) + B.2.2.2 * (C.2.2.1 * A.1 - C.1 * A.2.2.1) + C.2.2.2 * (A.2.2.1 * B.1 - A.1 * B.2.2.1)) / (A.1 * (B.2.1 * C.2.2.1 - B.2.2.1 * C.2.1) + A.2.1 * (B.2.2.1 * C.1 - B.1 * C.2.2.1) + A.2.2.1 * (B.1 * C.2.1 - B.2.1 * C.1))| ≤ err.val ∧
      |p2.val - (A.2.2.2 * (B.1 * C.2.1 - B.2.1 * C.1) + B.2.2.2 * (C.1 * A.2.1 - C.2.1 * A.1) + C.2.2.2 * (A.1 * B.2.1 - A.2.1 * B.1)) / (A.1 * (B.2.1 * C.2.2.1 - B.2.2.1 * C.2.1) + A.2.1 * (B.2.2.1 * C.1 - B.1 * C.2.2.1) + A.2.2.1 * (B.1 * C.2.1 - B.2.1 * C.1))| ≤ err.val ∧
      err.isFinite ∧ p0.isFinite ∧ p1.isFinite ∧ p2.isFinite) :
    det3 (nrm A) (nrm B) (nrm C) ≠ 0 ∧
      |p0.val - (meetQ A B C).1| ≤ err.val ∧ |p1.val - (meetQ A B C).2.1| ≤ err.val ∧
      |p2.val - (meetQ A B C).2.2| ≤ err.val ∧ err.isFinite ∧ p0.isFinite ∧ p1.isFinite ∧ p2.isFinite :=
  h

/-- Item 5. If `first_meet_f` returns `p`, then `p` is within `(1 + 3ε)·1e-13·(1 + max|p_k|)` per
coordinate of the exact meet of `vertex_at`'s triple, which is the point `ensure` stores.

Two hypotheses. `hok`: wherever `meet_f` answers, it is sound; `meetF_sound` gives this under
item 1's side conditions. `hsz`: the vertices are below `2^1022`, so the tolerance is finite. -/
theorem first_meet_f_sound {n : ℕ} {sp : ℕ → F64 × F64 × F64 × F64} {pl : ℕ → ℚ × ℚ × ℚ × ℚ}
    (hok : ∀ (t : ℕ × ℕ × ℕ) (p0 p1 p2 err : F64),
      meetF (sp t.1).1 (sp t.1).2.1 (sp t.1).2.2.1 (sp t.1).2.2.2
        (sp t.2.1).1 (sp t.2.1).2.1 (sp t.2.1).2.2.1 (sp t.2.1).2.2.2
        (sp t.2.2).1 (sp t.2.2).2.1 (sp t.2.2).2.2.1 (sp t.2.2).2.2.2 = some ((p0, p1, p2), err) →
      det3 (nrm (pl t.1)) (nrm (pl t.2.1)) (nrm (pl t.2.2)) ≠ 0 ∧
        |p0.val - (meetQ (pl t.1) (pl t.2.1) (pl t.2.2)).1| ≤ err.val ∧
        |p1.val - (meetQ (pl t.1) (pl t.2.1) (pl t.2.2)).2.1| ≤ err.val ∧
        |p2.val - (meetQ (pl t.1) (pl t.2.1) (pl t.2.2)).2.2| ≤ err.val ∧ err.isFinite ∧
        p0.isFinite ∧ p1.isFinite ∧ p2.isFinite)
    (hsz : ∀ (t : ℕ × ℕ × ℕ) (p0 p1 p2 err : F64),
      meetF (sp t.1).1 (sp t.1).2.1 (sp t.1).2.2.1 (sp t.1).2.2.2
        (sp t.2.1).1 (sp t.2.1).2.1 (sp t.2.1).2.2.1 (sp t.2.1).2.2.2
        (sp t.2.2).1 (sp t.2.2).2.1 (sp t.2.2).2.2.1 (sp t.2.2).2.2.2 = some ((p0, p1, p2), err) →
      max (max |p0.val| |p1.val|) |p2.val| ≤ 2 ^ 1022)
    {p0 p1 p2 : F64} (h : firstMeetF n sp pl = some (p0, p1, p2)) :
    ∃ t, firstExact n pl = some t ∧
      det3 (nrm (pl t.1)) (nrm (pl t.2.1)) (nrm (pl t.2.2)) ≠ 0 ∧
      let M := max (max |p0.val| |p1.val|) |p2.val|
      let tol := (1 + 3 * ε) * (3961408125713217 / 2 ^ 95) * (1 + M)
      |p0.val - (meetQ (pl t.1) (pl t.2.1) (pl t.2.2)).1| ≤ tol ∧
      |p1.val - (meetQ (pl t.1) (pl t.2.1) (pl t.2.2)).2.1| ≤ tol ∧
      |p2.val - (meetQ (pl t.1) (pl t.2.1) (pl t.2.2)).2.2| ≤ tol := by
  have hrep : ∀ t : ℕ × ℕ × ℕ, rep t = true →
      decide (det3 (nrm (pl t.1)) (nrm (pl t.2.1)) (nrm (pl t.2.2)) = 0) = true := by
    rintro ⟨x, y, z⟩ hr
    simp only [rep, Bool.or_eq_true, beq_iff_eq] at hr
    rw [decide_eq_true_eq]
    unfold det3
    rcases hr with (rfl | rfl) | rfl
    · exact rep_dep _ _ _ (Or.inl rfl)
    · exact rep_dep _ _ _ (Or.inr (Or.inl rfl))
    · exact rep_dep _ _ _ (Or.inr (Or.inr rfl))
  have hmf : ∀ (t : ℕ × ℕ × ℕ) (pe : F64 × F64 × F64) (e : F64),
      meetF (sp t.1).1 (sp t.1).2.1 (sp t.1).2.2.1 (sp t.1).2.2.2
        (sp t.2.1).1 (sp t.2.1).2.1 (sp t.2.1).2.2.1 (sp t.2.1).2.2.2
        (sp t.2.2).1 (sp t.2.2).2.1 (sp t.2.2).2.2.1 (sp t.2.2).2.2.2 = some (pe, e) →
      decide (det3 (nrm (pl t.1)) (nrm (pl t.2.1)) (nrm (pl t.2.2)) = 0) = false := fun t pe e hm => by
    obtain ⟨q0, q1, q2⟩ := pe
    have := (hok t q0 q1 q2 e hm).1
    simpa using this
  obtain ⟨t, e, hf, hm, ha⟩ := searchF_sound hrep hmf (trip n) (p0, p1, p2) h
  obtain ⟨hD, e0, e1, e2, fe, f0, f1, f2⟩ := hok t p0 p1 p2 e hm
  have hM := hsz t p0 p1 p2 e hm
  -- the tolerance is finite
  have fM : (fmax (fmax p0.abs p1.abs) p2.abs).isFinite :=
    fmax_fin (fmax_fin (by simpa using f0) (by simpa using f1)) (by simpa using f2)
  have hMv : (fmax (fmax p0.abs p1.abs) p2.abs).val = max (max |p0.val| |p1.val|) |p2.val| := by
    simp only [fmax_val, FP.abs_val]
  have fO : F1.isFinite := by decide
  have fs : (sizeF p0 p1 p2).isFinite := by
    unfold sizeF; rw [add_eq_round fO fM]
    refine round_finite_of_le_emax ?_
    rw [F1_val, hMv, binary64_emax]
    have : (0 : ℚ) ≤ max (max |p0.val| |p1.val|) |p2.val| :=
      le_trans (abs_nonneg _) (le_max_right _ _)
    rw [abs_of_nonneg (by linarith)]
    rw [show (1023 : ℤ) = ((1023 : ℕ) : ℤ) from rfl, zpow_natCast, pow_succ]
    have h1 : (1 : ℚ) ≤ 2 ^ 1022 := one_le_pow₀ (by norm_num)
    generalize (2 : ℚ) ^ 1022 = K at h1 hM ⊢
    linarith
  have fE : E13.isFinite := by decide
  have ft : (tolF p0 p1 p2).isFinite := by
    unfold tolF; rw [mul_eq_round fE fs]
    refine round_finite_of_le fs ?_
    rw [E13_val, abs_mul]
    have := abs_nonneg (sizeF p0 p1 p2).val
    have : |(3961408125713217 / 2 ^ 95 : ℚ)| ≤ 1 := by norm_num
    nlinarith
  have hle : e.val ≤ (tolF p0 p1 p2).val := (le_iff fe ft).mp ha
  have ht := tolF_hi f0 f1 f2 ft
  refine ⟨t, ?_, ?_, ?_, ?_, ?_⟩
  · simpa [firstExact] using hf
  · simpa using hD
  · exact le_trans e0 (le_trans hle ht)
  · exact le_trans e1 (le_trans hle ht)
  · exact le_trans e2 (le_trans hle ht)

end IEEE754.Screen
