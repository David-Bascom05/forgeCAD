/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Interval.Sin
import IEEE754.Exact.QF64

/-!
# `forge_math::Interval` add and mul, and curved's `horner_iv` / `sign_filter_iv` (C1)

`Interval::out(lo, hi) = (lo.next_down(), hi.next_up())`. `next_down` and `next_up` are taken
abstractly: all that is used is `nd f < f` and `f < nu f` for finite `f` with a finite result.

- `ivAdd_encl`, `ivMul_encl`: if `u ∈ a` and `v ∈ b`, then `u + v ∈ a + b` and `u·v ∈ a·b`. Each
  rounded corner is ≥ `min4` of the corners, and `nd(min4) <` that, so `below_round` puts it below every
  exact corner. Monotone rounding is not needed.
- `horner_encl`: Horner in these intervals (`acc = c_i + x·acc` from the top, from `point(0)`)
  encloses `Σ p_i t^i` for every real `t ∈ x`, when each `c_i` encloses `p_i`.
- `sign_filter_sound`: so `lo > 0` gives `p > 0` on all of `x`, and `hi < 0` gives `p < 0`.
- `enclose_encl`: `enclose(q) = (lo_f64, hi_f64)` encloses `q` once both ends are finite.

`HOk` (the first form) assumes every inner sum and corner finite. The no-overflow
hypothesis is then discharged from what `horner_iv` actually checks (`HChk`: `x`, each coefficient
enclosure and each `acc` with finite ends). `min`/`max` are taken in the IEEE order with ±∞
(`rmin`, `rmax`), and `next_down(−∞) = −∞`, `next_up(+∞) = +∞` (`DownX`, `UpX`).
- `ivMul_corners`: finite ends of `a·b` give finite corners. A `+∞` corner makes `hi = +∞`; a `−∞`
  corner makes `lo = −∞`.
- `ivAdd_sums`: finite ends of `a + b` give finite inner sums. A `+∞` low sum forces a `+∞`
  high sum (`round_mono`).
- `horner_encl_chk`, `sign_filter_chk`: the enclosure and the sign from the checks alone.
-/

namespace IEEE754.Interval

open IEEE754.FP

/-- An interval of floats. -/
abbrev IV := F64 × F64

/-- `x ∈ I`, with finite ends. -/
def Encl (I : IV) (v : ℝ) : Prop := I.1.isFinite ∧ I.2.isFinite ∧ ((I.1.val : ℚ) : ℝ) ≤ v ∧ v ≤ ((I.2.val : ℚ) : ℝ)

/-- `next_down` and `next_up`: strictly outward, where finite. -/
def Down (nd : F64 → F64) : Prop := ∀ f : F64, f.isFinite → (nd f).isFinite → (nd f).val < f.val
def Up (nu : F64 → F64) : Prop := ∀ f : F64, f.isFinite → (nu f).isFinite → f.val < (nu f).val

/-- `Interval + Interval`. -/
def ivAdd (nd nu : F64 → F64) (a b : IV) : IV := (nd (add a.1 b.1), nu (add a.2 b.2))

/-- `f64::min`, `f64::max` on non-NaN floats, in the IEEE order (infinities included). -/
def rmin (a b : F64) : F64 := if FP.lt b a then b else a
def rmax (a b : F64) : F64 := if FP.lt a b then b else a

/-- The four corners' `min4` and `max4`, as `c[0].min(c[1]).min(c[2]).min(c[3])`. -/
def min4 (a b : IV) : F64 := rmin (rmin (rmin (mul a.1 b.1) (mul a.1 b.2)) (mul a.2 b.1)) (mul a.2 b.2)
def max4 (a b : IV) : F64 := rmax (rmax (rmax (mul a.1 b.1) (mul a.1 b.2)) (mul a.2 b.1)) (mul a.2 b.2)

lemma lt_fin {a b : F64} (ha : a.isFinite) (hb : b.isFinite) : FP.lt a b = true ↔ a.val < b.val := by
  cases a <;> cases b <;> simp_all [FP.lt, FP.cmp, isFinite, compare_lt_iff_lt]

lemma rmin_eq {a b : F64} (ha : a.isFinite) (hb : b.isFinite) : rmin a b = Screen.fmin a b := by
  unfold rmin Screen.fmin
  by_cases h : b.val < a.val
  · rw [if_pos ((lt_fin hb ha).2 h), if_pos h]
  · rw [if_neg (fun h' => h ((lt_fin hb ha).1 h')), if_neg h]
lemma rmax_eq {a b : F64} (ha : a.isFinite) (hb : b.isFinite) : rmax a b = Screen.fmax a b := by
  unfold rmax Screen.fmax
  by_cases h : a.val < b.val
  · rw [if_pos ((lt_fin ha hb).2 h), if_pos h]
  · rw [if_neg (fun h' => h ((lt_fin ha hb).1 h')), if_neg h]

/-- `Interval * Interval`. -/
def ivMul (nd nu : F64 → F64) (a b : IV) : IV := (nd (min4 a b), nu (max4 a b))

/-- A float strictly below a rounded `r` (finite) is at most `r`, in `ℝ`. -/
lemma below_r {zs : Bool} {r : ℚ} {x : F64} (hx : x.isFinite) (hf : (round binary64 zs r).isFinite)
    (h : x.val ≤ (round binary64 zs r).val) (hlt : ∃ y : F64, y.isFinite ∧ x.val < y.val ∧ y.val ≤ (round binary64 zs r).val) :
    ((x.val : ℚ) : ℝ) ≤ (r : ℝ) := by
  obtain ⟨y, -, hy, hy'⟩ := hlt
  exact_mod_cast below_round hx hf (lt_of_lt_of_le hy hy')

theorem ivAdd_encl {nd nu : F64 → F64} (hd : Down nd) (hu : Up nu) {a b : IV} {u v : ℝ}
    (ha : Encl a u) (hb : Encl b v) (f1 : (add a.1 b.1).isFinite) (f2 : (add a.2 b.2).isFinite)
    (g1 : (ivAdd nd nu a b).1.isFinite) (g2 : (ivAdd nd nu a b).2.isFinite) : Encl (ivAdd nd nu a b) (u + v) := by
  obtain ⟨a1, a2, a3, a4⟩ := ha
  obtain ⟨b1, b2, b3, b4⟩ := hb
  refine ⟨g1, g2, ?_, ?_⟩
  · have hlt := hd _ f1 g1
    have hlt' : (ivAdd nd nu a b).1.val < (round binary64 (a.1.sign && b.1.sign) (a.1.val + b.1.val)).val := by
      rw [← add_eq_round a1 b1]; exact hlt
    have f1' : (round binary64 (a.1.sign && b.1.sign) (a.1.val + b.1.val)).isFinite := by
      rw [← add_eq_round a1 b1]; exact f1
    have := below_round g1 f1' hlt'
    have : (((ivAdd nd nu a b).1.val : ℚ) : ℝ) ≤ ((a.1.val + b.1.val : ℚ) : ℝ) := by exact_mod_cast this
    push_cast at this; linarith
  · have hlt := hu _ f2 g2
    have hlt' : (round binary64 (a.2.sign && b.2.sign) (a.2.val + b.2.val)).val < (ivAdd nd nu a b).2.val := by
      rw [← add_eq_round a2 b2]; exact hlt
    have f2' : (round binary64 (a.2.sign && b.2.sign) (a.2.val + b.2.val)).isFinite := by
      rw [← add_eq_round a2 b2]; exact f2
    have := above_round g2 f2' hlt'
    have : ((a.2.val + b.2.val : ℚ) : ℝ) ≤ (((ivAdd nd nu a b).2.val : ℚ) : ℝ) := by exact_mod_cast this
    push_cast at this; linarith

/-- `u·v` for `u ∈ [a, b]`, `v ∈ [c, d]` is between the corners' min and max. -/
lemma mul_corners {a b c d u v : ℝ} (hu : a ≤ u ∧ u ≤ b) (hv : c ≤ v ∧ v ≤ d) :
    min (min (min (a * c) (a * d)) (b * c)) (b * d) ≤ u * v ∧
      u * v ≤ max (max (max (a * c) (a * d)) (b * c)) (b * d) := by
  have l1 : min (a * v) (b * v) ≤ u * v := by
    rcases le_total 0 v with h | h
    · exact le_trans (min_le_left _ _) (mul_le_mul_of_nonneg_right hu.1 h)
    · exact le_trans (min_le_right _ _) (mul_le_mul_of_nonpos_right hu.2 h)
  have l2 : u * v ≤ max (a * v) (b * v) := by
    rcases le_total 0 v with h | h
    · exact le_trans (mul_le_mul_of_nonneg_right hu.2 h) (le_max_right _ _)
    · exact le_trans (mul_le_mul_of_nonpos_right hu.1 h) (le_max_left _ _)
  have ka : min (a * c) (a * d) ≤ a * v ∧ a * v ≤ max (a * c) (a * d) := by
    rcases le_total 0 a with h | h
    · exact ⟨le_trans (min_le_left _ _) (mul_le_mul_of_nonneg_left hv.1 h),
        le_trans (mul_le_mul_of_nonneg_left hv.2 h) (le_max_right _ _)⟩
    · exact ⟨le_trans (min_le_right _ _) (mul_le_mul_of_nonpos_left hv.2 h),
        le_trans (mul_le_mul_of_nonpos_left hv.1 h) (le_max_left _ _)⟩
  have kb : min (b * c) (b * d) ≤ b * v ∧ b * v ≤ max (b * c) (b * d) := by
    rcases le_total 0 b with h | h
    · exact ⟨le_trans (min_le_left _ _) (mul_le_mul_of_nonneg_left hv.1 h),
        le_trans (mul_le_mul_of_nonneg_left hv.2 h) (le_max_right _ _)⟩
    · exact ⟨le_trans (min_le_right _ _) (mul_le_mul_of_nonpos_left hv.2 h),
        le_trans (mul_le_mul_of_nonpos_left hv.1 h) (le_max_left _ _)⟩
  constructor
  · rw [min_assoc]
    exact le_trans (le_trans (min_le_min ka.1 kb.1) le_rfl) l1
  · rw [max_assoc]
    exact le_trans l2 (max_le_max ka.2 kb.2)

/-- Each rounded corner, against the exact one: a float below every rounded corner is below
every exact corner. -/
lemma corner_lo {x y z : F64} (hx : x.isFinite) (hy : y.isFinite) (hm : (mul x y).isFinite) (hz : z.isFinite)
    (h : z.val < (mul x y).val) : ((z.val : ℚ) : ℝ) ≤ ((x.val : ℚ) : ℝ) * ((y.val : ℚ) : ℝ) := by
  rw [mul_eq_round hx hy] at hm h
  have := below_round hz hm h
  exact_mod_cast this

lemma corner_hi {x y z : F64} (hx : x.isFinite) (hy : y.isFinite) (hm : (mul x y).isFinite) (hz : z.isFinite)
    (h : (mul x y).val < z.val) : ((x.val : ℚ) : ℝ) * ((y.val : ℚ) : ℝ) ≤ ((z.val : ℚ) : ℝ) := by
  rw [mul_eq_round hx hy] at hm h
  have := above_round hz hm h
  exact_mod_cast this

theorem ivMul_encl {nd nu : F64 → F64} (hd : Down nd) (hu : Up nu) {a b : IV} {u v : ℝ}
    (ha : Encl a u) (hb : Encl b v)
    (f1 : (mul a.1 b.1).isFinite) (f2 : (mul a.1 b.2).isFinite) (f3 : (mul a.2 b.1).isFinite) (f4 : (mul a.2 b.2).isFinite)
    (g1 : (ivMul nd nu a b).1.isFinite) (g2 : (ivMul nd nu a b).2.isFinite) : Encl (ivMul nd nu a b) (u * v) := by
  obtain ⟨a1, a2, a3, a4⟩ := ha
  obtain ⟨b1, b2, b3, b4⟩ := hb
  have mc := mul_corners ⟨a3, a4⟩ ⟨b3, b4⟩
  have em : min4 a b = Screen.fmin (Screen.fmin (Screen.fmin (mul a.1 b.1) (mul a.1 b.2)) (mul a.2 b.1)) (mul a.2 b.2) := by
    unfold min4
    rw [rmin_eq f1 f2, rmin_eq (Screen.fmin_fin f1 f2) f3, rmin_eq (Screen.fmin_fin (Screen.fmin_fin f1 f2) f3) f4]
  have ex : max4 a b = Screen.fmax (Screen.fmax (Screen.fmax (mul a.1 b.1) (mul a.1 b.2)) (mul a.2 b.1)) (mul a.2 b.2) := by
    unfold max4
    rw [rmax_eq f1 f2, rmax_eq (Screen.fmax_fin f1 f2) f3, rmax_eq (Screen.fmax_fin (Screen.fmax_fin f1 f2) f3) f4]
  have fmn : (min4 a b).isFinite := by rw [em]; exact Screen.fmin_fin (Screen.fmin_fin (Screen.fmin_fin f1 f2) f3) f4
  have fmx : (max4 a b).isFinite := by rw [ex]; exact Screen.fmax_fin (Screen.fmax_fin (Screen.fmax_fin f1 f2) f3) f4
  have vmn : (min4 a b).val = min (min (min (mul a.1 b.1).val (mul a.1 b.2).val) (mul a.2 b.1).val) (mul a.2 b.2).val := by
    rw [em]; simp only [Screen.fmin_val]
  have vmx : (max4 a b).val = max (max (max (mul a.1 b.1).val (mul a.1 b.2).val) (mul a.2 b.1).val) (mul a.2 b.2).val := by
    rw [ex]; simp only [Screen.fmax_val]
  refine ⟨g1, g2, ?_, ?_⟩
  · have hl := hd _ fmn g1
    rw [vmn] at hl
    simp only [ivMul] at hl g1 ⊢
    have c1 := corner_lo a1 b1 f1 g1 (lt_of_lt_of_le hl (by simp only [min_le_iff, le_refl, true_or]))
    have c2 := corner_lo a1 b2 f2 g1 (lt_of_lt_of_le hl (by simp only [min_le_iff, le_refl, true_or, or_true]))
    have c3 := corner_lo a2 b1 f3 g1 (lt_of_lt_of_le hl (by simp only [min_le_iff, le_refl, true_or, or_true]))
    have c4 := corner_lo a2 b2 f4 g1 (lt_of_lt_of_le hl (by simp only [min_le_iff, le_refl, or_true]))
    exact le_trans (le_min (le_min (le_min c1 c2) c3) c4) mc.1
  · have hl := hu _ fmx g2
    rw [vmx] at hl
    simp only [ivMul] at hl g2 ⊢
    have c1 := corner_hi a1 b1 f1 g2 (lt_of_le_of_lt (by simp only [le_max_iff, le_refl, true_or]) hl)
    have c2 := corner_hi a1 b2 f2 g2 (lt_of_le_of_lt (by simp only [le_max_iff, le_refl, true_or, or_true]) hl)
    have c3 := corner_hi a2 b1 f3 g2 (lt_of_le_of_lt (by simp only [le_max_iff, le_refl, true_or, or_true]) hl)
    have c4 := corner_hi a2 b2 f4 g2 (lt_of_le_of_lt (by simp only [le_max_iff, le_refl, or_true]) hl)
    exact le_trans mc.2 (max_le (max_le (max_le c1 c2) c3) c4)

/-! ## Horner -/

/-- `Interval::point(0.0)`. -/
def zeroIV : IV := (.zero false, .zero false)

/-- `horner_iv`'s loop, coefficients lowest first: `acc = c_i + x·acc` from the top. -/
def hornerIV (nd nu : F64 → F64) (x : IV) : List IV → IV
  | [] => zeroIV
  | c :: cs => ivAdd nd nu c (ivMul nd nu x (hornerIV nd nu x cs))

/-- The polynomial, coefficients lowest first. -/
def polyR (t : ℝ) : List ℝ → ℝ
  | [] => 0
  | p :: ps => p + t * polyR t ps

/-- Every inner sum and corner is finite, and every result's ends (checked by `horner_iv`). -/
def HOk (nd nu : F64 → F64) (x : IV) : List IV → Prop
  | [] => True
  | c :: cs =>
    let a := hornerIV nd nu x cs
    let m := ivMul nd nu x a
    HOk nd nu x cs ∧ (mul x.1 a.1).isFinite ∧ (mul x.1 a.2).isFinite ∧ (mul x.2 a.1).isFinite ∧
      (mul x.2 a.2).isFinite ∧ m.1.isFinite ∧ m.2.isFinite ∧ (add c.1 m.1).isFinite ∧ (add c.2 m.2).isFinite ∧
      (ivAdd nd nu c m).1.isFinite ∧ (ivAdd nd nu c m).2.isFinite

/-- **`horner_iv` encloses.** Coefficient intervals enclosing `p_i`, `t ∈ x`: the result encloses
`p(t)`. -/
theorem horner_encl {nd nu : F64 → F64} (hd : Down nd) (hu : Up nu) {x : IV} {t : ℝ} (ht : Encl x t) :
    ∀ (cs : List IV) (ps : List ℝ) (hl : cs.length = ps.length), (∀ i (h : i < cs.length), Encl (cs.get ⟨i, h⟩) (ps.get ⟨i, by omega⟩)) →
      HOk nd nu x cs → Encl (hornerIV nd nu x cs) (polyR t ps)
  | [], [], _, _, _ => ⟨rfl, rfl, by simp [hornerIV, zeroIV, polyR, FP.val], by simp [hornerIV, zeroIV, polyR, FP.val]⟩
  | [], _ :: _, h, _, _ => by simp at h
  | _ :: _, [], h, _, _ => by simp at h
  | c :: cs, p :: ps, hl, hc, hok => by
    obtain ⟨h0, f1, f2, f3, f4, g1, g2, k1, k2, r1, r2⟩ := hok
    have ih := horner_encl hd hu ht cs ps (by simpa using hl)
      (fun i h => by simpa using hc (i + 1) (by simp; omega)) h0
    have hm := ivMul_encl hd hu ht ih f1 f2 f3 f4 g1 g2
    have hc0 : Encl c p := by simpa using hc 0 (by simp)
    exact ivAdd_encl hd hu hc0 hm k1 k2 r1 r2

/-- **`sign_filter_iv`.** -/
theorem sign_filter_sound {nd nu : F64 → F64} (hd : Down nd) (hu : Up nu) {x : IV} {cs : List IV} {ps : List ℝ}
    (hl : cs.length = ps.length) (hc : ∀ i (h : i < cs.length), Encl (cs.get ⟨i, h⟩) (ps.get ⟨i, by omega⟩))
    (hok : HOk nd nu x cs) {t : ℝ} (ht : Encl x t) :
    (0 < (hornerIV nd nu x cs).1.val → 0 < polyR t ps) ∧ ((hornerIV nd nu x cs).2.val < 0 → polyR t ps < 0) := by
  obtain ⟨-, -, l, h⟩ := horner_encl hd hu ht cs ps hl hc hok
  constructor
  · intro p; have : (0 : ℝ) < ((hornerIV nd nu x cs).1.val : ℝ) := by exact_mod_cast p
    linarith
  · intro p; have : (((hornerIV nd nu x cs).2.val : ℚ) : ℝ) < 0 := by exact_mod_cast p
    linarith

/-- **`enclose(q) = (q.lo_f64(), q.hi_f64())` encloses `q`** once both ends are finite (as `horner_iv`
checks): `lo_sound`, `hi_sound`. -/
theorem enclose_encl {x : Exact.QB.QB} (hx : Exact.QB.Can x) (hd : 0 < Exact.Big.val x.d)
    {dec : F64 → Exact.Big × ℤ} (hdec : Exact.QB.DecOk dec) (nd nu : F64 → F64) (f0 : F64)
    (h1 : (Exact.QB.loF (Exact.QB.cmpOf x dec) nd f0).isFinite)
    (h2 : (Exact.QB.hiF (Exact.QB.cmpOf x dec) nu f0).isFinite) :
    Encl (Exact.QB.loF (Exact.QB.cmpOf x dec) nd f0, Exact.QB.hiF (Exact.QB.cmpOf x dec) nu f0)
      ((Exact.QZ.val (Exact.QB.toZ x) : ℚ) : ℝ) := by
  refine ⟨h1, h2, ?_, ?_⟩
  · rcases Exact.QB.lo_sound hx hd hdec nd f0 with e | ⟨-, l⟩
    · rw [e] at h1; exact absurd h1 (by decide)
    · exact_mod_cast l
  · rcases Exact.QB.hi_sound hx hd hdec nu f0 with e | ⟨-, l⟩
    · rw [e] at h2; exact absurd h2 (by decide)
    · exact_mod_cast l

/-! ## The no-overflow hypothesis, from what `horner_iv` checks -/

/-- `next_down(−∞) = −∞`; `next_down` keeps non-NaN and never gives `+∞`. `next_up` likewise. -/
def DownX (nd : F64 → F64) : Prop :=
  nd (.inf true) = .inf true ∧ ∀ f : F64, f.isNaN = false → (nd f).isNaN = false ∧ nd f ≠ .inf false
def UpX (nu : F64 → F64) : Prop :=
  nu (.inf false) = .inf false ∧ ∀ f : F64, f.isNaN = false → (nu f).isNaN = false ∧ nu f ≠ .inf true

lemma notnan_cases {z : F64} (h : z.isNaN = false) : z.isFinite ∨ z = .inf false ∨ z = .inf true := by
  cases z with
  | inf s => cases s <;> simp
  | nan => simp [isNaN] at h
  | _ => simp [isFinite]

lemma fin_notnan {z : F64} (h : z.isFinite) : z.isNaN = false := by cases z <;> simp_all [isFinite, isNaN]

lemma mul_notnan {x y : F64} (hx : x.isFinite) (hy : y.isFinite) : (mul x y).isNaN = false := by
  rw [mul_eq_round hx hy]; exact round_not_nan _ _ _

lemma rmax_notnan {a b : F64} (ha : a.isNaN = false) (hb : b.isNaN = false) : (rmax a b).isNaN = false := by
  unfold rmax; split_ifs <;> assumption
lemma rmin_notnan {a b : F64} (ha : a.isNaN = false) (hb : b.isNaN = false) : (rmin a b).isNaN = false := by
  unfold rmin; split_ifs <;> assumption

lemma rmax_pinf {a b : F64} (ha : a.isNaN = false) (hb : b.isNaN = false) (h : a = .inf false ∨ b = .inf false) :
    rmax a b = .inf false := by
  rcases h with rfl | rfl
  · cases b with
    | inf s => cases s <;> simp [rmax, FP.lt, FP.cmp]
    | nan => simp [isNaN] at hb
    | _ => simp [rmax, FP.lt, FP.cmp]
  · cases a with
    | inf s => cases s <;> simp [rmax, FP.lt, FP.cmp]
    | nan => simp [isNaN] at ha
    | _ => simp [rmax, FP.lt, FP.cmp]

lemma rmin_ninf {a b : F64} (ha : a.isNaN = false) (hb : b.isNaN = false) (h : a = .inf true ∨ b = .inf true) :
    rmin a b = .inf true := by
  rcases h with rfl | rfl
  · cases b with
    | inf s => cases s <;> simp [rmin, FP.lt, FP.cmp]
    | nan => simp [isNaN] at hb
    | _ => simp [rmin, FP.lt, FP.cmp]
  · cases a with
    | inf s => cases s <;> simp [rmin, FP.lt, FP.cmp]
    | nan => simp [isNaN] at ha
    | _ => simp [rmin, FP.lt, FP.cmp]

/-- **Corners from the ends.** If `a·b`'s ends are finite (as checked), every corner is finite: a
corner `+∞` makes `max4 = +∞` and `hi = next_up(+∞) = +∞`; a corner `−∞` makes `lo = −∞`. -/
theorem ivMul_corners {nd nu : F64 → F64} (hdx : DownX nd) (hux : UpX nu) {a b : IV}
    (a1 : a.1.isFinite) (a2 : a.2.isFinite) (b1 : b.1.isFinite) (b2 : b.2.isFinite)
    (g1 : (ivMul nd nu a b).1.isFinite) (g2 : (ivMul nd nu a b).2.isFinite) :
    (mul a.1 b.1).isFinite ∧ (mul a.1 b.2).isFinite ∧ (mul a.2 b.1).isFinite ∧ (mul a.2 b.2).isFinite := by
  have n1 := mul_notnan a1 b1; have n2 := mul_notnan a1 b2
  have n3 := mul_notnan a2 b1; have n4 := mul_notnan a2 b2
  have hmax : (mul a.1 b.1 = .inf false ∨ mul a.1 b.2 = .inf false ∨ mul a.2 b.1 = .inf false ∨
      mul a.2 b.2 = .inf false) → False := by
    intro h
    have e : max4 a b = .inf false := by
      unfold max4
      apply rmax_pinf (rmax_notnan (rmax_notnan n1 n2) n3) n4
      rcases h with h | h | h | h
      · exact Or.inl (rmax_pinf (rmax_notnan n1 n2) n3 (Or.inl (rmax_pinf n1 n2 (Or.inl h))))
      · exact Or.inl (rmax_pinf (rmax_notnan n1 n2) n3 (Or.inl (rmax_pinf n1 n2 (Or.inr h))))
      · exact Or.inl (rmax_pinf (rmax_notnan n1 n2) n3 (Or.inr h))
      · exact Or.inr h
    have : (ivMul nd nu a b).2 = .inf false := by simp only [ivMul]; rw [e, hux.1]
    rw [this] at g2; exact absurd g2 (by decide)
  have hmin : (mul a.1 b.1 = .inf true ∨ mul a.1 b.2 = .inf true ∨ mul a.2 b.1 = .inf true ∨
      mul a.2 b.2 = .inf true) → False := by
    intro h
    have e : min4 a b = .inf true := by
      unfold min4
      apply rmin_ninf (rmin_notnan (rmin_notnan n1 n2) n3) n4
      rcases h with h | h | h | h
      · exact Or.inl (rmin_ninf (rmin_notnan n1 n2) n3 (Or.inl (rmin_ninf n1 n2 (Or.inl h))))
      · exact Or.inl (rmin_ninf (rmin_notnan n1 n2) n3 (Or.inl (rmin_ninf n1 n2 (Or.inr h))))
      · exact Or.inl (rmin_ninf (rmin_notnan n1 n2) n3 (Or.inr h))
      · exact Or.inr h
    have : (ivMul nd nu a b).1 = .inf true := by simp only [ivMul]; rw [e, hdx.1]
    rw [this] at g1; exact absurd g1 (by decide)
  refine ⟨?_, ?_, ?_, ?_⟩ <;>
  [rcases notnan_cases n1 with h | h | h; rcases notnan_cases n2 with h | h | h;
   rcases notnan_cases n3 with h | h | h; rcases notnan_cases n4 with h | h | h] <;>
  first | exact h | exact (hmax (by tauto)).elim | exact (hmin (by tauto)).elim

lemma pinf_le {y : F64} (hy : y.isNaN = false) (h : (FP.inf false : F64).le y = true) : y = .inf false := by
  cases y with
  | inf s => cases s <;> simp_all [FP.le, FP.cmp]
  | nan => simp [isNaN] at hy
  | _ => simp_all [FP.le, FP.cmp]

lemma le_ninf {x : F64} (hx : x.isNaN = false) (h : x.le (FP.inf true : F64) = true) : x = .inf true := by
  cases x with
  | inf s => cases s <;> simp_all [FP.le, FP.cmp]
  | nan => simp [isNaN] at hx
  | _ => simp_all [FP.le, FP.cmp]

/-- **Sums from the ends.** If `a + b`'s ends are finite, both inner sums are: `−∞` gives
`lo = −∞`; `+∞` in the low sum forces `+∞` in the high one (rounding is monotone), so `hi = +∞`. -/
theorem ivAdd_sums {nd nu : F64 → F64} (hdx : DownX nd) (hux : UpX nu) {a b : IV}
    (a1 : a.1.isFinite) (a2 : a.2.isFinite) (b1 : b.1.isFinite) (b2 : b.2.isFinite)
    (oa : a.1.val ≤ a.2.val) (ob : b.1.val ≤ b.2.val)
    (g1 : (ivAdd nd nu a b).1.isFinite) (g2 : (ivAdd nd nu a b).2.isFinite) :
    (add a.1 b.1).isFinite ∧ (add a.2 b.2).isFinite := by
  have mono : (add a.1 b.1).le (add a.2 b.2) = true := by
    rw [add_eq_round a1 b1, add_eq_round a2 b2]; exact round_mono _ (by linarith)
  have n1 : (add a.1 b.1).isNaN = false := by rw [add_eq_round a1 b1]; exact round_not_nan _ _ _
  have n2 : (add a.2 b.2).isNaN = false := by rw [add_eq_round a2 b2]; exact round_not_nan _ _ _
  have lo_ninf : add a.1 b.1 ≠ .inf true := fun h => by
    have : (ivAdd nd nu a b).1 = .inf true := by simp only [ivAdd]; rw [h, hdx.1]
    rw [this] at g1; exact absurd g1 (by decide)
  have hi_pinf : add a.2 b.2 ≠ .inf false := fun h => by
    have : (ivAdd nd nu a b).2 = .inf false := by simp only [ivAdd]; rw [h, hux.1]
    rw [this] at g2; exact absurd g2 (by decide)
  constructor
  · rcases notnan_cases n1 with h | h | h
    · exact h
    · rw [h] at mono; exact absurd (pinf_le n2 mono) hi_pinf
    · exact absurd h lo_ninf
  · rcases notnan_cases n2 with h | h | h
    · exact h
    · exact absurd h hi_pinf
    · rw [h] at mono; exact absurd (le_ninf n1 mono) lo_ninf

/-- What `horner_iv` checks: each `acc` has finite ends. -/
def HChk (nd nu : F64 → F64) (x : IV) : List IV → Prop
  | [] => True
  | c :: cs => HChk nd nu x cs ∧ (hornerIV nd nu x (c :: cs)).1.isFinite ∧ (hornerIV nd nu x (c :: cs)).2.isFinite

lemma add_ninf {c : F64} (hc : c.isFinite) : add c (.inf true) = .inf true := by
  cases c <;> simp_all [isFinite, add]
lemma add_pinf {c : F64} (hc : c.isFinite) : add c (.inf false) = .inf false := by
  cases c <;> simp_all [isFinite, add]

/-- **`horner_iv` encloses, from its own checks.** `x` and the coefficients enclose (finite ends),
and each `acc` has finite ends: the result encloses `p(t)` for every `t ∈ x`. -/
theorem horner_encl_chk {nd nu : F64 → F64} (hd : Down nd) (hu : Up nu) (hdx : DownX nd) (hux : UpX nu)
    {x : IV} {t : ℝ} (ht : Encl x t) :
    ∀ (cs : List IV) (ps : List ℝ) (hl : cs.length = ps.length),
      (∀ i (h : i < cs.length), Encl (cs.get ⟨i, h⟩) (ps.get ⟨i, by omega⟩)) →
      HChk nd nu x cs → Encl (hornerIV nd nu x cs) (polyR t ps)
  | [], [], _, _, _ => ⟨rfl, rfl, by simp [hornerIV, zeroIV, polyR, FP.val], by simp [hornerIV, zeroIV, polyR, FP.val]⟩
  | [], _ :: _, h, _, _ => by simp at h
  | _ :: _, [], h, _, _ => by simp at h
  | c :: cs, p :: ps, hl, hc, hchk => by
    obtain ⟨h0, r1, r2⟩ := hchk
    have ih := horner_encl_chk hd hu hdx hux ht cs ps (by simpa using hl)
      (fun i h => by simpa using hc (i + 1) (by simp; omega)) h0
    have hc0 : Encl c p := by simpa using hc 0 (by simp)
    have x1 := ht.1; have x2 := ht.2.1
    have q1 := ih.1; have q2 := ih.2.1
    have c1 := hc0.1; have c2 := hc0.2.1
    simp only [hornerIV] at r1 r2
    set m := ivMul nd nu x (hornerIV nd nu x cs)
    -- the product's ends are finite
    have m1 : m.1.isFinite := by
      have nn : (min4 x (hornerIV nd nu x cs)).isNaN = false :=
        rmin_notnan (rmin_notnan (rmin_notnan (mul_notnan x1 q1) (mul_notnan x1 q2)) (mul_notnan x2 q1)) (mul_notnan x2 q2)
      obtain ⟨k1, k2⟩ := hdx.2 _ nn
      rcases notnan_cases k1 with h | h | h
      · exact h
      · exact absurd h k2
      · have : (ivAdd nd nu c m).1 = .inf true := by simp only [ivAdd, m, ivMul]; rw [h, add_ninf c1, hdx.1]
        rw [this] at r1; exact absurd r1 (by decide)
    have m2 : m.2.isFinite := by
      have nn : (max4 x (hornerIV nd nu x cs)).isNaN = false :=
        rmax_notnan (rmax_notnan (rmax_notnan (mul_notnan x1 q1) (mul_notnan x1 q2)) (mul_notnan x2 q1)) (mul_notnan x2 q2)
      obtain ⟨k1, k2⟩ := hux.2 _ nn
      rcases notnan_cases k1 with h | h | h
      · exact h
      · have : (ivAdd nd nu c m).2 = .inf false := by simp only [ivAdd, m, ivMul]; rw [h, add_pinf c2, hux.1]
        rw [this] at r2; exact absurd r2 (by decide)
      · exact absurd h k2
    obtain ⟨f1, f2, f3, f4⟩ := ivMul_corners hdx hux x1 x2 q1 q2 m1 m2
    have em := ivMul_encl hd hu ht ih f1 f2 f3 f4 m1 m2
    have oc : c.1.val ≤ c.2.val := by
      have := le_trans hc0.2.2.1 hc0.2.2.2; exact_mod_cast this
    have om : m.1.val ≤ m.2.val := by
      have := le_trans em.2.2.1 em.2.2.2; exact_mod_cast this
    obtain ⟨s1, s2⟩ := ivAdd_sums hdx hux c1 c2 m1 m2 oc om r1 r2
    exact ivAdd_encl hd hu hc0 em s1 s2 r1 r2

/-- **`sign_filter_iv`, from `horner_iv`'s own checks** (no extra hypothesis). -/
theorem sign_filter_chk {nd nu : F64 → F64} (hd : Down nd) (hu : Up nu) (hdx : DownX nd) (hux : UpX nu)
    {x : IV} {cs : List IV} {ps : List ℝ} (hl : cs.length = ps.length)
    (hc : ∀ i (h : i < cs.length), Encl (cs.get ⟨i, h⟩) (ps.get ⟨i, by omega⟩))
    (hchk : HChk nd nu x cs) {t : ℝ} (ht : Encl x t) :
    (0 < (hornerIV nd nu x cs).1.val → 0 < polyR t ps) ∧ ((hornerIV nd nu x cs).2.val < 0 → polyR t ps < 0) := by
  obtain ⟨-, -, l, h⟩ := horner_encl_chk hd hu hdx hux ht cs ps hl hc hchk
  constructor
  · intro p; have : (0 : ℝ) < ((hornerIV nd nu x cs).1.val : ℝ) := by exact_mod_cast p
    linarith
  · intro p; have : (((hornerIV nd nu x cs).2.val : ℚ) : ℝ) < 0 := by exact_mod_cast p
    linarith

end IEEE754.Interval
