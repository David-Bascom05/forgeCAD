/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.TwoSum
import IEEE754.Binary64

/-!
# Veltkamp's split and Dekker's product

`forge-math`'s `split` (Veltkamp, with `SPLITTER = 2^σ + 1`) and `two_product` (Dekker):
`veltkamp` gives `hi + lo = a` with `hi` on a grid `2^σ` coarser than `a` and `|lo|` at most half
that grid; `two_product_exact` gives `x + y = a · b` exactly when the precision is `2σ − 1`
(binary64: `σ = 27`), the product does not underflow inexactly (the kernel's `product_exact`) and
nothing overflows.
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {f : Format}

/-! ## Grids -/

/-- `x` is an integer multiple of `2^k`. -/
def Mult (x : ℚ) (k : ℤ) : Prop := ∃ n : ℤ, x = n * 2 ^ k

lemma mult_zero (k : ℤ) : Mult 0 k := ⟨0, by simp⟩

lemma mult_add {x y : ℚ} {k : ℤ} (hx : Mult x k) (hy : Mult y k) : Mult (x + y) k := by
  obtain ⟨n, rfl⟩ := hx; obtain ⟨m, rfl⟩ := hy; exact ⟨n + m, by push_cast; ring⟩

lemma mult_sub {x y : ℚ} {k : ℤ} (hx : Mult x k) (hy : Mult y k) : Mult (x - y) k := by
  obtain ⟨n, rfl⟩ := hx; obtain ⟨m, rfl⟩ := hy; exact ⟨n - m, by push_cast; ring⟩

lemma mult_neg {x : ℚ} {k : ℤ} (hx : Mult x k) : Mult (-x) k := by
  obtain ⟨n, rfl⟩ := hx; exact ⟨-n, by push_cast; ring⟩

lemma mult_of_le {x : ℚ} {k j : ℤ} (hx : Mult x k) (h : j ≤ k) : Mult x j := by
  obtain ⟨n, rfl⟩ := hx
  obtain ⟨p, hp⟩ := FP.two_zpow_dvd h
  exact ⟨n * p, by rw [hp]; push_cast; ring⟩

lemma mult_mul {x y : ℚ} {k j : ℤ} (hx : Mult x k) (hy : Mult y j) : Mult (x * y) (k + j) := by
  obtain ⟨n, rfl⟩ := hx; obtain ⟨m, rfl⟩ := hy
  exact ⟨n * m, by rw [two_zpow_add]; push_cast; ring⟩

lemma mult_max {x : ℚ} {k j : ℤ} (hk : Mult x k) (hj : Mult x j) : Mult x (max k j) := by
  rcases le_total k j with h | h
  · rw [max_eq_right h]; exact hj
  · rw [max_eq_left h]; exact hk

lemma mult_pow {k j : ℤ} (h : j ≤ k) : Mult (2 ^ k) j := mult_of_le ⟨1, by simp⟩ h

/-- A multiple of `2^k` smaller than `2^k` is zero. -/
lemma mult_small {x : ℚ} {k : ℤ} (hx : Mult x k) (h : |x| < 2 ^ k) : x = 0 := by
  obtain ⟨n, rfl⟩ := hx
  have hpos : (0 : ℚ) < 2 ^ k := two_zpow_pos k
  rw [abs_mul, abs_of_pos hpos] at h
  have : |(n : ℚ)| < 1 := by
    by_contra hc; rw [not_lt] at hc; nlinarith
  have : n = 0 := by
    have : |n| < 1 := by exact_mod_cast this
    exact Int.abs_lt_one_iff.mp this
  simp [this]

/-- A multiple of `2^k` below `M + 2^k`, `M` a multiple too, is at most `M`. -/
lemma mult_le_of_lt {x M : ℚ} {k : ℤ} (hx : Mult x k) (hM : Mult M k) (h : x < M + 2 ^ k) : x ≤ M := by
  obtain ⟨n, rfl⟩ := hx; obtain ⟨m, rfl⟩ := hM
  have hpos : (0 : ℚ) < 2 ^ k := two_zpow_pos k
  have : (n : ℚ) < m + 1 := by
    by_contra hc; rw [not_lt] at hc; nlinarith
  have : n ≤ m := by exact_mod_cast Int.lt_add_one_iff.mp (by exact_mod_cast this)
  exact mul_le_mul_of_nonneg_right (by exact_mod_cast this) hpos.le

lemma rep_of_mult {x : ℚ} {k : ℤ} (hx : Mult x k) (hk : f.qmin ≤ k) (hb : |x| ≤ 2 ^ f.prec * 2 ^ k)
    (ho : |x| < 2 ^ (f.emax + 1)) : Rep f x := by
  obtain ⟨n, rfl⟩ := hx
  have hpos : (0 : ℚ) < 2 ^ k := two_zpow_pos k
  refine rep_grid ?_ hk ho
  rw [abs_mul, abs_of_pos hpos] at hb
  exact_mod_cast le_of_mul_le_mul_right hb hpos

lemma mult_val {x : FP f} (hx : x.isFinite) : ∃ e, f.qmin ≤ e ∧ Mult x.val e ∧ |x.val| < 2 ^ f.prec * 2 ^ e := by
  obtain ⟨k, e, hk, he, hv, hav⟩ := FP.val_repr hx
  refine ⟨e, he, ⟨k, hv⟩, ?_⟩
  rw [hav]
  exact mul_lt_mul_of_pos_right (by exact_mod_cast hk) (two_zpow_pos e)

/-! ## The quantum of a rounding -/

/-- The quantum exponent of rounding `r`. -/
def qOf (f : Format) (r : ℚ) : ℤ := (roundPos f |r|).2

lemma qOf_ge_qmin (r : ℚ) : f.qmin ≤ qOf f r := roundPos_ge_qmin f _

lemma qOf_ge {r : ℚ} {k : ℤ} (h : 2 ^ k ≤ |r|) : k - f.prec + 1 ≤ qOf f r := by
  have hr : 0 < |r| := lt_of_lt_of_le (two_zpow_pos k) h
  have : k ≤ Int.log 2 |r| := (Int.zpow_le_iff_le_log (b := 2) (by norm_num) hr).mp (by exact_mod_cast h)
  unfold qOf; rw [roundPos_snd]; omega

lemma qOf_le {r : ℚ} {k : ℤ} (hr : r ≠ 0) (h : |r| < 2 ^ k) : qOf f r ≤ max (k - f.prec) f.qmin := by
  have hr' : 0 < |r| := abs_pos.mpr hr
  have : Int.log 2 |r| < k := (Int.lt_zpow_iff_log_lt (b := 2) (by norm_num) hr').mp (by exact_mod_cast h)
  unfold qOf; rw [roundPos_snd]; omega

/-- A finite rounding of `r ≠ 0` is on the grid of its quantum and within half of it. -/
lemma round_grid {zs : Bool} {r : ℚ} (hr : r ≠ 0) (hfin : (round f zs r).isFinite) :
    Mult (round f zs r).val (qOf f r) ∧ |(round f zs r).val - r| ≤ 2 ^ (qOf f r - 1) := by
  obtain ⟨k, hk, he⟩ := round_repr (f := f) (zs := zs) hr hfin
  exact ⟨⟨k, hk⟩, he⟩

/-- A rounding of something no bigger than a finite datum is finite. -/
lemma round_finite_of_le {zs : Bool} {r : ℚ} {x : FP f} (hx : x.isFinite) (h : |r| ≤ |x.val|) :
    (round f zs r).isFinite := by
  by_cases hr : r = 0
  · subst hr; simp [round, isFinite]
  have hx0 : x.val ≠ 0 := by intro e; rw [e, abs_zero] at h; exact hr (abs_nonpos_iff.mp h)
  rw [round_finite_iff (K := ℚ) f hr]
  have hself : round f false x.val = x := by
    cases x with
    | fin s m e hc => exact round_val_q false hc
    | zero s => simp [val] at hx0
    | _ => simp [isFinite] at hx
  have hu : uround f |x.val| = |x.val| := by
    have := abs_round_val (K := ℚ) (f := f) (zs := false) hx0 (by rw [hself]; exact hx)
    simp only [Rat.cast_id] at this
    rw [hself] at this; exact this.symm
  calc uround f |r| ≤ uround f |x.val| := uround_mono f (abs_pos.mpr hr) h
    _ = |x.val| := hu
    _ < _ := FP.abs_val_lt hx

/-- A datum nearest to a representable `r` is `r`. -/
lemma near_eq {S r : ℚ} (hN : ∀ x : FP f, x.isFinite → |S - r| ≤ |x.val - r|) (hr : Rep f r) : S = r := by
  have := hN _ (hr false).1
  rw [(hr false).2, _root_.sub_self, abs_zero] at this
  linarith [abs_nonneg (S - r), abs_nonpos_iff.mp this]

/-! ## Veltkamp's split -/

/-- What Dekker's product needs of a split `a = hi + lo`: `|a|` is in the binade
`[2^(ε+p−1), 2^(ε+p))`; `hi` is on the grid `2^(ε+σ)` and at most `2^(ε+p)`; `lo` is on the grid
`2^ε` and at most `2^(ε+σ−1)`; and both are on every grid `a` is on. -/
structure Split (f : Format) (σ : ℤ) (a hi lo : ℚ) (ε lam : ℤ) : Prop where
  sum : hi + lo = a
  a_ge : 2 ^ (ε + f.prec - 1) ≤ |a|
  a_lt : |a| < 2 ^ (ε + f.prec)
  hi_grid : Mult hi (ε + σ)
  hi_le : |hi| ≤ 2 ^ (ε + f.prec)
  lo_grid : Mult lo ε
  lo_le : |lo| ≤ 2 ^ (ε + σ - 1)
  hi_lam : Mult hi lam
  lo_lam : Mult lo lam

lemma split_neg {σ : ℤ} {a hi lo : ℚ} {ε lam : ℤ} (h : Split f σ a hi lo ε lam) : Split f σ (-a) (-hi) (-lo) ε lam :=
  ⟨by rw [← h.sum]; ring, by rw [abs_neg]; exact h.a_ge, by rw [abs_neg]; exact h.a_lt, mult_neg h.hi_grid,
    by rw [abs_neg]; exact h.hi_le, mult_neg h.lo_grid, by rw [abs_neg]; exact h.lo_le, mult_neg h.hi_lam,
    mult_neg h.lo_lam⟩

lemma mult_int_mul {x : ℚ} {k : ℤ} (n : ℤ) (hx : Mult x k) : Mult (n * x) k := by
  obtain ⟨m, rfl⟩ := hx; exact ⟨n * m, by push_cast; ring⟩

/-- Veltkamp on values, `a > 0`: `g` the rounding of `(2^σ + 1)·a`, `dd` that of `g − a`. -/
lemma veltkamp_core {σ : ℕ} (hσ : 2 ≤ σ) (hσp : σ + 1 ≤ f.prec) {a g dd : FP f} {lam : ℤ}
    (ha : a.isFinite) (hg : g.isFinite) (hd : dd.isFinite) (ha0 : 0 < a.val) (hlam : Mult a.val lam)
    (hCA : |(2 ^ σ + 1) * a.val| < 2 ^ (f.emax + 1))
    (hgN : ∀ x : FP f, x.isFinite → |g.val - (2 ^ σ + 1) * a.val| ≤ |x.val - (2 ^ σ + 1) * a.val|)
    (hgQ : Mult g.val (qOf f ((2 ^ σ + 1) * a.val)) ∧
      |g.val - (2 ^ σ + 1) * a.val| ≤ 2 ^ (qOf f ((2 ^ σ + 1) * a.val) - 1))
    (hdN : ∀ x : FP f, x.isFinite → |dd.val - (g.val - a.val)| ≤ |x.val - (g.val - a.val)|)
    (hdQ : g.val - a.val ≠ 0 → Mult dd.val (qOf f (g.val - a.val)) ∧
      |dd.val - (g.val - a.val)| ≤ 2 ^ (qOf f (g.val - a.val) - 1))
    (hdrel : |dd.val - (g.val - a.val)| ≤ 2 ^ (-(f.prec : ℤ)) * |dd.val|) :
    Rep f (g.val - dd.val) ∧ ∃ ε, Split f σ a.val (g.val - dd.val) (a.val - (g.val - dd.val)) ε lam := by
  obtain ⟨k, e, hk, he, hvk, hcan⟩ := val_repr_canon ha
  set A := a.val
  set G := g.val
  set D := dd.val
  set CA := (2 ^ σ + 1 : ℚ) * A with hCAdef
  have hp3 : 3 ≤ f.prec := by omega
  have hpow8 : (2 : ℚ) ^ (-(f.prec : ℤ)) ≤ 1 / 8 := by
    calc (2 : ℚ) ^ (-(f.prec : ℤ)) ≤ 2 ^ (-3 : ℤ) := two_zpow_le (by omega)
      _ = 1 / 8 := by norm_num
  have h2σ : (1 : ℚ) ≤ 2 ^ σ := one_le_pow₀ (by norm_num)
  -- the binade of `a`, and its canonical grid
  set ε : ℤ := Int.log 2 A - f.prec + 1 with hεdef
  obtain ⟨hl1, hl2⟩ := log_spec (K := ℚ) ha0
  have hA1 : (2 : ℚ) ^ (ε + f.prec - 1) ≤ A := by rw [show ε + f.prec - 1 = Int.log 2 A by omega]; exact hl1
  have hA2 : A < 2 ^ (ε + f.prec) := by rw [show ε + f.prec = Int.log 2 A + 1 by omega]; exact hl2
  have hMe : Mult A e := ⟨k, hvk⟩
  have hk0 : 0 < k := by
    have : (0 : ℚ) < k * 2 ^ e := by rw [← hvk]; exact ha0
    have := (pos_iff_pos_of_mul_pos this).mpr (two_zpow_pos e)
    exact_mod_cast this
  have hεe : ε ≤ e := by
    have : A < 2 ^ (e + f.prec) := by
      have hk' : (k : ℚ) < 2 ^ (f.prec : ℤ) := by
        rw [← cast_two_pow]; rw [abs_of_pos hk0] at hk; exact_mod_cast hk
      rw [hvk, two_zpow_add]
      calc (k : ℚ) * 2 ^ e < 2 ^ (f.prec : ℤ) * 2 ^ e := mul_lt_mul_of_pos_right hk' (two_zpow_pos e)
        _ = 2 ^ e * 2 ^ (f.prec : ℤ) := by ring
    have := (Int.lt_zpow_iff_log_lt (b := 2) (by norm_num) ha0).mp (by exact_mod_cast this)
    omega
  have hele : e ≤ ε + f.prec - 1 := by
    have : (2 : ℚ) ^ e ≤ A := by
      rw [hvk]; exact le_mul_of_one_le_left (two_zpow_pos e).le (by exact_mod_cast hk0)
    have := (Int.zpow_le_iff_le_log (b := 2) (by norm_num) ha0).mp (by exact_mod_cast this)
    omega
  have hMε : Mult A ε := mult_of_le hMe hεe
  have hAle : A ≤ 2 ^ (ε + f.prec) - 2 ^ ε := by
    refine mult_le_of_lt hMε (mult_sub (mult_pow (by omega)) (mult_pow le_rfl)) (by linarith)
  -- `g` and its grid
  have hCA1 : (2 : ℚ) ^ (ε + σ + f.prec - 1) ≤ CA := by
    rw [show ε + σ + f.prec - 1 = (σ : ℤ) + (ε + f.prec - 1) by ring, two_zpow_add, zpow_natCast]
    nlinarith
  have hCA2 : CA < 2 ^ (ε + σ + f.prec + 1) := by
    rw [show ε + σ + f.prec + 1 = ((σ : ℤ) + 1) + (ε + f.prec) by ring, two_zpow_add, zpow_add_one₀ two_ne_zero,
      zpow_natCast]
    nlinarith
  have hCApos : 0 < CA := by positivity
  have hqg1 : ε + σ ≤ qOf f CA := by
    have := qOf_ge (f := f) (r := CA) (k := ε + σ + f.prec - 1) (by rw [abs_of_pos hCApos]; exact hCA1); omega
  have hqg2 : qOf f CA ≤ max (ε + σ + 1) f.qmin := by
    have := qOf_le (f := f) hCApos.ne' (k := ε + σ + f.prec + 1) (by rw [abs_of_pos hCApos]; exact hCA2)
    rw [show ε + σ + f.prec + 1 - f.prec = ε + σ + 1 by ring] at this; exact this
  obtain ⟨hgM, hgE⟩ := hgQ
  -- `g ≥ 4a`
  have h4σ : (4 : ℚ) ≤ 2 ^ σ := by
    calc (4 : ℚ) = 2 ^ 2 := by norm_num
      _ ≤ 2 ^ σ := pow_le_pow_right₀ (by norm_num) hσ
  have h4CA : 4 * A ≤ CA := by rw [hCAdef]; nlinarith
  have hG4 : 4 * A ≤ G := by
    have h4 : 4 * A = (k : ℚ) * 2 ^ (e + 2) := by rw [hvk, zpow_add₀ two_ne_zero]; norm_num; ring
    have hr : Rep f (4 * A) := by
      rw [h4]; refine rep_grid hk.le (by omega) ?_
      rw [← h4, abs_of_pos (by linarith)]
      rw [abs_of_pos hCApos] at hCA; linarith
    have := near_ge hgN (hr false).1 (by rw [(hr false).2]; exact h4CA)
    rwa [(hr false).2] at this
  have hG0 : 0 < G := by linarith
  -- `d`: between `g/2` and `g`
  have hD0 : 0 ≤ D := by
    have := near_ge hdN (x := FP.zero false) (by simp [isFinite]) (by rw [FP.val_zero]; linarith)
    rwa [FP.val_zero] at this
  have hDG : D ≤ G := near_le hdN hg (by linarith)
  have hDG2 : G ≤ 2 * D := by
    rw [abs_of_nonneg hD0] at hdrel
    have h1 := (abs_le.mp hdrel).1
    have h2 : 2 ^ (-(f.prec : ℤ)) * D ≤ D / 8 := by nlinarith
    nlinarith
  have hrep : Rep f (G - D) := rep_sub hg hd (by linarith) hDG2
  refine ⟨hrep, ε, ?_⟩
  have hlo : A - (G - D) = D - (G - A) := by ring
  have hGA : G - A ≠ 0 := by intro h; linarith
  obtain ⟨hdM, hdE⟩ := hdQ hGA
  have hpowp : (2 : ℚ) ^ σ + 1 ≤ 2 ^ f.prec := by
    have : (2 : ℚ) ^ (σ + 1) ≤ 2 ^ f.prec := pow_le_pow_right₀ (by norm_num) hσp
    rw [pow_succ] at this; linarith
  have hMC : ∀ {x : ℚ} {j : ℤ}, Mult x j → Mult ((2 ^ σ + 1) * x) j := fun hx => by
    have := mult_int_mul (2 ^ σ + 1) hx; push_cast at this; exact this
  rcases lt_or_ge (ε + σ) f.qmin with hT | hN
  · -- everything below the subnormal threshold: exact
    have he' : e = f.qmin := by
      rcases hcan with h | h
      · rcases (show e = ε ∨ ε < e by omega) with h' | h'
        · omega
        · exfalso
          have : (2 : ℚ) ^ (e + f.prec - 1) ≤ A := by
            rw [hvk, show e + f.prec - 1 = (f.prec - 1 : ℕ) + e by rw [f.prec_sub_one]; ring, two_zpow_add,
              zpow_natCast]
            rw [abs_of_pos hk0] at h
            exact mul_le_mul_of_nonneg_right (by exact_mod_cast h) (two_zpow_pos e).le
          have := (Int.zpow_le_iff_le_log (b := 2) (by norm_num) ha0).mp (by exact_mod_cast this)
          omega
      · exact h
    have hMq : Mult A f.qmin := he' ▸ hMe
    have hbig : (2 : ℚ) ^ (ε + σ + f.prec + 1) ≤ 2 ^ f.prec * 2 ^ f.qmin := by
      rw [← zpow_natCast, ← two_zpow_add]; exact two_zpow_le (by omega)
    have hGe : G = CA := near_eq hgN (rep_of_mult (hMC hMq) le_rfl (by rw [abs_of_pos hCApos]; linarith) hCA)
    have hGA' : G - A = 2 ^ σ * A := by rw [hGe]; ring
    have hrGA : Rep f (G - A) := by
      rw [hGA']
      have : Mult (2 ^ σ * A) f.qmin := by
        have := mult_int_mul (2 ^ σ) hMq; push_cast at this; exact this
      refine rep_of_mult this le_rfl ?_ ?_
      · rw [abs_of_pos (by positivity)]; nlinarith
      · rw [abs_of_pos (by positivity)]; rw [abs_of_pos hCApos] at hCA; nlinarith
    have hDe : D = G - A := near_eq hdN hrGA
    have hhi : G - D = A := by rw [hDe]; ring
    rw [hhi, _root_.sub_self]
    exact ⟨by ring, by rwa [abs_of_pos ha0], by rwa [abs_of_pos ha0], mult_of_le hMq hT.le,
      by rw [abs_of_pos ha0]; linarith, mult_zero _, by simp; exact (two_zpow_pos _).le, hlam, mult_zero _⟩
  · -- `qmin ≤ ε + σ`
    have hqg3 : qOf f CA ≤ ε + σ + 1 := by have := hqg2; omega
    have hgE' : |G - CA| ≤ 2 ^ (ε + σ) := le_trans hgE (two_zpow_le (by omega))
    -- `g − a` reaches the binade `2^(ε+σ+p−1)`
    have hGA1 : (2 : ℚ) ^ (ε + σ + f.prec - 1) ≤ G - A := by
      by_contra hc
      rw [not_le] at hc
      have hG1 := (abs_le.mp hgE').1
      have hσε : (2 : ℚ) ^ (ε + σ) = 2 ^ σ * 2 ^ ε := by rw [two_zpow_add, zpow_natCast, _root_.mul_comm]
      have hσε2 : (2 : ℚ) ^ (ε + σ + f.prec - 1) = 2 ^ σ * 2 ^ (ε + f.prec - 1) := by
        rw [show ε + σ + f.prec - 1 = (σ : ℤ) + (ε + f.prec - 1) by ring, two_zpow_add, zpow_natCast]
      have hlt : A - 2 ^ (ε + f.prec - 1) < 2 ^ e := by
        have e1 : CA - A = 2 ^ σ * A := by rw [hCAdef]; ring
        have key : 2 ^ σ * A < 2 ^ σ * (2 ^ (ε + f.prec - 1) + 2 ^ ε) := by
          rw [mul_add, ← hσε, ← hσε2]; linarith
        have hlt0 : A < 2 ^ (ε + f.prec - 1) + 2 ^ ε := lt_of_mul_lt_mul_left key (by positivity)
        linarith [two_zpow_le (K := ℚ) hεe]
      have hpow : A = 2 ^ (ε + f.prec - 1) := by
        have := mult_small (mult_sub hMe (mult_pow (k := ε + f.prec - 1) hele))
          (by rw [abs_of_nonneg (by linarith)]; exact hlt)
        linarith
      have hr : Rep f CA := by
        rw [hCAdef, hpow]
        refine rep_of_mult (hMC (mult_pow le_rfl)) (by omega) ?_ ?_
        · rw [abs_of_pos (mul_pos (by positivity) (two_zpow_pos _))]
          exact mul_le_mul_of_nonneg_right hpowp (two_zpow_pos _).le
        · rw [← hpow]; exact hCA
      have := near_eq hgN hr
      rw [this, hCAdef, hpow] at hc
      rw [hσε2] at hc
      linarith
    have hqd1 : ε + σ ≤ qOf f (G - A) := by
      have := qOf_ge (f := f) (r := G - A) (k := ε + σ + f.prec - 1) (by rw [abs_of_pos (by linarith)]; exact hGA1)
      omega
    have hhiM : Mult (G - D) (ε + σ) := mult_sub (mult_of_le hgM hqg1) (mult_of_le hdM hqd1)
    -- `|lo| ≤ 2^(ε+σ−1)`
    have hGA2 : G - A ≤ 2 ^ (ε + σ + f.prec) := by
      have h1 := (abs_le.mp hgE').2
      have : (2 : ℚ) ^ (ε + σ + f.prec) = 2 ^ σ * 2 ^ (ε + f.prec) := by
        rw [show ε + σ + f.prec = (σ : ℤ) + (ε + f.prec) by ring, two_zpow_add, zpow_natCast]
      have h3 : (2 : ℚ) ^ (ε + σ) = 2 ^ σ * 2 ^ ε := by rw [two_zpow_add, zpow_natCast, _root_.mul_comm]
      have e1 : CA - A = 2 ^ σ * A := by rw [hCAdef]; ring
      have h4 : 2 ^ σ * A ≤ 2 ^ σ * (2 ^ (ε + f.prec) - 2 ^ ε) := mul_le_mul_of_nonneg_left hAle (by positivity)
      rw [this]; rw [h3] at h1
      have h5 : 2 ^ σ * (2 ^ (ε + f.prec) - 2 ^ ε) + 2 ^ σ * 2 ^ ε = (2 : ℚ) ^ σ * 2 ^ (ε + f.prec) := by ring
      linarith
    have hloB : |D - (G - A)| ≤ 2 ^ (ε + σ - 1) := by
      rcases lt_or_eq_of_le hGA2 with hlt | heq
      · have := qOf_le (f := f) hGA (k := ε + σ + f.prec) (by rw [abs_of_pos (by linarith)]; exact hlt)
        rw [show ε + σ + f.prec - f.prec = ε + σ by ring, max_eq_left hN] at this
        exact le_trans hdE (two_zpow_le (by omega))
      · have hr : Rep f (G - A) := by
          rw [heq]
          refine rep_of_mult (mult_pow le_rfl) (by omega) ?_ ?_
          · rw [abs_of_pos (two_zpow_pos _)]; exact le_mul_of_one_le_left (two_zpow_pos _).le (one_le_pow₀ (by norm_num))
          · rw [← heq, abs_of_pos (by linarith)]
            have := FP.abs_val_lt hg; rw [abs_of_pos hG0] at this; linarith
        rw [near_eq hdN hr, _root_.sub_self, abs_zero]; exact (two_zpow_pos _).le
    have hloB' : |A - (G - D)| ≤ 2 ^ (ε + σ - 1) := by rw [hlo]; exact hloB
    have hhi0 : 0 ≤ G - D := by linarith
    have hhiB : |G - D| ≤ 2 ^ (ε + f.prec) := by
      rw [abs_of_nonneg hhi0]
      have h1 := (abs_le.mp hloB').1
      refine mult_le_of_lt hhiM (mult_pow (by omega)) ?_
      have : (2 : ℚ) ^ (ε + σ - 1) < 2 ^ (ε + σ) := two_zpow_lt (by omega)
      have : (0 : ℚ) < 2 ^ ε := two_zpow_pos _
      linarith
    have hloM : Mult (A - (G - D)) ε := mult_sub hMε (mult_of_le hhiM (by omega))
    have hlamc : Mult (G - D) lam ∧ Mult (A - (G - D)) lam := by
      rcases le_or_gt lam (ε + σ) with hl | hl
      · have h1 := mult_of_le hhiM hl
        exact ⟨h1, mult_sub hlam h1⟩
      · have hz : A - (G - D) = 0 := mult_small (mult_sub (mult_of_le hlam hl.le) hhiM)
          (lt_of_le_of_lt hloB' (two_zpow_lt (by omega)))
        have : G - D = A := by linarith
        rw [hz, this]; exact ⟨hlam, mult_zero _⟩
    exact ⟨by ring, by rwa [abs_of_pos ha0], by rwa [abs_of_pos ha0], hhiM, hhiB, hloM, hloB', hlamc.1, hlamc.2⟩

/-- `forge-math`'s `split`: `c = SPLITTER = 2^σ + 1`. -/
def split (c a : FP f) : FP f × FP f :=
  let g := mul c a
  let hi := sub g (sub g a)
  (hi, sub a hi)

/-- Veltkamp's split, for `a > 0`, on the datums. -/
lemma veltkamp_pos {σ : ℕ} (hσ : 2 ≤ σ) (hσp : σ + 1 ≤ f.prec) {c a : FP f} {lam : ℤ}
    (hc : c.isFinite) (hcv : c.val = 2 ^ σ + 1) (ha : a.isFinite) (hca : (mul c a).isFinite)
    (ha0 : 0 < a.val) (hlam : Mult a.val lam) :
    (sub (mul c a) a).isFinite ∧ Rep f ((mul c a).val - (sub (mul c a) a).val) ∧
      ∃ ε, Split f σ a.val ((mul c a).val - (sub (mul c a) a).val)
        (a.val - ((mul c a).val - (sub (mul c a) a).val)) ε lam := by
  have hgr : mul c a = round f (c.sign ^^ a.sign) ((2 ^ σ + 1) * a.val) := by
    rw [mul_eq_round hc ha, hcv]
  set g := mul c a
  have hCA0 : (2 ^ σ + 1) * a.val ≠ 0 := by positivity
  have hgN : ∀ x : FP f, x.isFinite → |g.val - (2 ^ σ + 1) * a.val| ≤ |x.val - (2 ^ σ + 1) * a.val| := by
    intro x hx; rw [hgr]; exact round_nearest (by rw [← hgr]; exact hca) hx
  have hAG : a.val ≤ g.val := near_ge hgN ha (by nlinarith [one_le_pow₀ (M₀ := ℚ) (n := σ) (by norm_num : (1:ℚ) ≤ 2)])
  set d := sub g a
  have hdr : d = round f (g.sign && !a.sign) (g.val - a.val) := sub_eq_round hca ha
  have hd : d.isFinite := by
    rw [hdr]; exact round_finite_of_le hca (by rw [abs_of_nonneg (by linarith), abs_of_pos (by linarith)]; linarith)
  have hcore := veltkamp_core hσ hσp ha hca hd ha0 hlam
    (abs_lt_of_round_finite (by rw [← hgr]; exact hca)) hgN
    (by rw [hgr]; exact round_grid hCA0 (by rw [← hgr]; exact hca))
    (fun x hx => by rw [hdr]; exact round_nearest (by rw [← hdr]; exact hd) hx)
    (fun h => by rw [hdr]; exact round_grid h (by rw [← hdr]; exact hd))
    (sub_err_res hca ha hd)
  exact ⟨hd, hcore⟩

lemma qOf_neg (r : ℚ) : qOf f (-r) = qOf f r := by unfold qOf; rw [abs_neg]

/-- **Veltkamp's split.** For finite `a ≠ 0` with `SPLITTER · a` finite: `hi` and `lo` are finite
and `hi + lo = a`, `hi` on the grid `2^σ` above `a`'s binade, `|lo|` at most half that grid,
both on every grid `a` is on (`Split`). -/
theorem veltkamp {σ : ℕ} (hσ : 2 ≤ σ) (hσp : σ + 1 ≤ f.prec) {c a : FP f} {lam : ℤ}
    (hc : c.isFinite) (hcv : c.val = 2 ^ σ + 1) (ha : a.isFinite) (hca : (mul c a).isFinite)
    (ha0 : a.val ≠ 0) (hlam : Mult a.val lam) :
    (split c a).1.isFinite ∧ (split c a).2.isFinite ∧
      ∃ ε, Split f σ a.val (split c a).1.val (split c a).2.val ε lam := by
  have hgr0 : mul c a = round f (c.sign ^^ a.sign) ((2 ^ σ + 1) * a.val) := by rw [mul_eq_round hc ha, hcv]
  set g := mul c a
  set d := sub g a
  obtain ⟨hd, hrep, ε, hsp⟩ : d.isFinite ∧ Rep f (g.val - d.val) ∧
      ∃ ε, Split f σ a.val (g.val - d.val) (a.val - (g.val - d.val)) ε lam := by
    rcases lt_or_gt_of_ne ha0 with hn | hp
    · -- `a < 0`: the split of `−a`, negated
      have hgr := hgr0
      have hCA0 : (2 ^ σ + 1) * a.val ≠ 0 := mul_ne_zero (by positivity) ha0
      have hgN : ∀ x : FP f, x.isFinite → |g.val - (2 ^ σ + 1) * a.val| ≤ |x.val - (2 ^ σ + 1) * a.val| := by
        intro x hx; rw [hgr]; exact round_nearest (by rw [← hgr]; exact hca) hx
      have hGA : g.val ≤ a.val := near_le hgN ha (by nlinarith [one_le_pow₀ (M₀ := ℚ) (n := σ) (by norm_num : (1:ℚ) ≤ 2)])
      have hdr : d = round f (g.sign && !a.sign) (g.val - a.val) := sub_eq_round hca ha
      have hd : d.isFinite := by
        rw [hdr]; exact round_finite_of_le hca (by rw [abs_of_nonpos (by linarith), abs_of_neg (by linarith)]; linarith)
      have hgQ := round_grid hCA0 (by rw [← hgr]; exact hca)
      rw [← hgr] at hgQ
      have hdN : ∀ x : FP f, x.isFinite → |d.val - (g.val - a.val)| ≤ |x.val - (g.val - a.val)| := by
        intro x hx; rw [hdr]; exact round_nearest (by rw [← hdr]; exact hd) hx
      have hcore := veltkamp_core (a := a.neg) (g := g.neg) (dd := d.neg) hσ hσp (by simpa using ha)
        (by simpa using hca) (by simpa using hd) (by simp; linarith) (by simpa using mult_neg hlam)
        (by simp only [FP.val_neg]; rw [show (2 ^ σ + 1) * -a.val = -((2 ^ σ + 1) * a.val) by ring, abs_neg];
            exact abs_lt_of_round_finite (by rw [← hgr]; exact hca))
        (by
          have := near_neg hgN; simp only [FP.val_neg]
          rwa [show (2 ^ σ + 1) * -a.val = -((2 ^ σ + 1) * a.val) by ring])
        (by
          simp only [FP.val_neg]
          rw [show (2 ^ σ + 1) * -a.val = -((2 ^ σ + 1) * a.val) by ring, qOf_neg,
            show -g.val - -((2 ^ σ + 1) * a.val) = -(g.val - (2 ^ σ + 1) * a.val) by ring, abs_neg]
          exact ⟨mult_neg hgQ.1, hgQ.2⟩)
        (by
          have := near_neg hdN; simp only [FP.val_neg]
          rwa [show -g.val - -a.val = -(g.val - a.val) by ring])
        (by
          intro h
          simp only [FP.val_neg] at h ⊢
          have h' : g.val - a.val ≠ 0 := by intro e; apply h; linarith
          obtain ⟨m1, m2⟩ := round_grid h' (by rw [← hdr]; exact hd)
          rw [← hdr] at m1 m2
          rw [show -g.val - -a.val = -(g.val - a.val) by ring, qOf_neg,
            show -d.val - -(g.val - a.val) = -(d.val - (g.val - a.val)) by ring, abs_neg]
          exact ⟨mult_neg m1, m2⟩)
        (by
          simp only [FP.val_neg]
          rw [show -d.val - (-g.val - -a.val) = -(d.val - (g.val - a.val)) by ring, abs_neg, abs_neg]
          exact sub_err_res hca ha hd)
      simp only [FP.val_neg] at hcore
      obtain ⟨h1, ε, h2⟩ := hcore
      refine ⟨hd, ?_, ε, ?_⟩
      · have := rep_neg h1; rwa [show -(-g.val - -d.val) = g.val - d.val by ring] at this
      · have := split_neg h2
        rwa [show - -a.val = a.val by ring, show -(-g.val - -d.val) = g.val - d.val by ring,
          show -(-a.val - (-g.val - -d.val)) = a.val - (g.val - d.val) by ring] at this
    · exact veltkamp_pos hσ hσp hc hcv ha hca hp hlam
  -- `hi = g ⊖ d` and `lo = a ⊖ hi` are exact
  obtain ⟨hhi, hhi'⟩ := sub_of_rep hca hd hrep
  have hlo : Rep f (a.val - (sub g d).val) := by
    have := add_err_representable hca (by simpa using ha : a.neg.isFinite) (by have : d = add g a.neg := rfl; rw [← this]; exact hd)
    have hr : Rep f (g.val + a.neg.val - (add g a.neg).val) := fun zs => this zs
    have := rep_neg hr
    rw [hhi']
    rw [show (add g a.neg) = d by rfl, FP.val_neg] at this
    rwa [show -(g.val + -a.val - d.val) = a.val - (g.val - d.val) by ring] at this
  obtain ⟨hlo1, hlo2⟩ := sub_of_rep ha hhi hlo
  refine ⟨hhi, hlo1, ε, ?_⟩
  show Split f σ a.val (sub g d).val (sub a (sub g d)).val ε lam
  rw [hlo2, hhi']
  exact hsp

/-! ## Dekker's product -/

lemma qOf_lt {r : ℚ} (hr : r ≠ 0) : |r| < 2 ^ (qOf f r + f.prec) := by
  have hr' : 0 < |r| := abs_pos.mpr hr
  have h2 := (log_spec (K := ℚ) hr').2
  have : Int.log 2 |r| + 1 ≤ qOf f r + f.prec := by unfold qOf; rw [roundPos_snd]; omega
  exact lt_of_lt_of_le h2 (two_zpow_le this)

/-- Dekker on values: with the splits of `a` and `b`, every step is exact. -/
lemma dekker_core {σ : ℕ} (hp : (f.prec : ℤ) = 2 * σ - 1) (hσ : 3 ≤ σ) {A B ah al bh bl x : ℚ}
    {εa εb la lb : ℤ} (Sa : Split f σ A ah al εa la) (Sb : Split f σ B bh bl εb lb)
    (hΛ : f.qmin ≤ la + lb) (hP : |A * B| < 2 ^ (f.emax - 1))
    (hxN : ∀ z : FP f, z.isFinite → |x - A * B| ≤ |z.val - A * B|)
    (hxQ : Mult x (qOf f (A * B)) ∧ |x - A * B| ≤ 2 ^ (qOf f (A * B) - 1)) :
    Rep f (ah * bh) ∧ Rep f (al * bh) ∧ Rep f (ah * bl) ∧ Rep f (al * bl) ∧
      Rep f (x - ah * bh) ∧ Rep f (x - ah * bh - al * bh) ∧ Rep f (x - ah * bh - al * bh - ah * bl) ∧
      Rep f (al * bl - (x - ah * bh - al * bh - ah * bl)) ∧
      al * bl - (x - ah * bh - al * bh - ah * bl) = A * B - x := by
  set E := εa + εb
  set Λ := la + lb
  set P := A * B
  have hpz : (f.prec : ℤ) = 2 * σ - 1 := hp
  -- the binade of the product
  have hPa : (2 : ℚ) ^ (E + 2 * f.prec - 2) ≤ |P| := by
    rw [abs_mul, show E + 2 * (f.prec : ℤ) - 2 = (εa + f.prec - 1) + (εb + f.prec - 1) by ring, two_zpow_add]
    exact mul_le_mul Sa.a_ge Sb.a_ge (two_zpow_pos _).le (abs_nonneg _)
  have hPb : |P| < 2 ^ (E + 2 * f.prec) := by
    rw [abs_mul, show E + 2 * (f.prec : ℤ) = (εa + f.prec) + (εb + f.prec) by ring, two_zpow_add]
    exact mul_lt_mul'' Sa.a_lt Sb.a_lt (abs_nonneg _) (abs_nonneg _)
  have hP0 : P ≠ 0 := by intro h; rw [h, abs_zero] at hPa; linarith [two_zpow_pos (K := ℚ) (E + 2 * f.prec - 2)]
  have hovz : E + 2 * (f.prec : ℤ) < f.emax + 1 := by
    have : E + 2 * (f.prec : ℤ) - 2 < f.emax - 1 :=
      (two_zpow_lt_iff (K := ℚ)).mp (lt_of_le_of_lt hPa hP)
    omega
  -- the four products
  have hAε : Mult A εa := by rw [← Sa.sum]; exact mult_add (mult_of_le Sa.hi_grid (by omega)) Sa.lo_grid
  have hBε : Mult B εb := by rw [← Sb.sum]; exact mult_add (mult_of_le Sb.hi_grid (by omega)) Sb.lo_grid
  have hhhM : Mult (ah * bh) (max (E + 2 * σ) Λ) := mult_max
    (by have := mult_mul Sa.hi_grid Sb.hi_grid; rwa [show εa + σ + (εb + σ) = E + 2 * σ by ring] at this)
    (mult_mul Sa.hi_lam Sb.hi_lam)
  have hlhM : Mult (al * bh) (max (E + σ) Λ) := mult_max
    (by have := mult_mul Sa.lo_grid Sb.hi_grid; rwa [show εa + (εb + σ) = E + σ by ring] at this)
    (mult_mul Sa.lo_lam Sb.hi_lam)
  have hhlM : Mult (ah * bl) (max (E + σ) Λ) := mult_max
    (by have := mult_mul Sa.hi_grid Sb.lo_grid; rwa [show εa + σ + εb = E + σ by ring] at this)
    (mult_mul Sa.hi_lam Sb.lo_lam)
  have hllM : Mult (al * bl) (max E Λ) := mult_max (mult_mul Sa.lo_grid Sb.lo_grid) (mult_mul Sa.lo_lam Sb.lo_lam)
  have hhhB : |ah * bh| ≤ 2 ^ (E + 2 * f.prec) := by
    rw [abs_mul, show E + 2 * (f.prec : ℤ) = (εa + f.prec) + (εb + f.prec) by ring, two_zpow_add]
    exact mul_le_mul Sa.hi_le Sb.hi_le (abs_nonneg _) (two_zpow_pos _).le
  have hlhB : |al * bh| ≤ 2 ^ (E + f.prec + σ - 1) := by
    rw [abs_mul, show E + (f.prec : ℤ) + σ - 1 = (εa + σ - 1) + (εb + f.prec) by ring, two_zpow_add]
    exact mul_le_mul Sa.lo_le Sb.hi_le (abs_nonneg _) (two_zpow_pos _).le
  have hhlB : |ah * bl| ≤ 2 ^ (E + f.prec + σ - 1) := by
    rw [abs_mul, show E + (f.prec : ℤ) + σ - 1 = (εa + f.prec) + (εb + σ - 1) by ring, two_zpow_add]
    exact mul_le_mul Sa.hi_le Sb.lo_le (abs_nonneg _) (two_zpow_pos _).le
  have hllB : |al * bl| ≤ 2 ^ (E + f.prec - 1) := by
    rw [abs_mul, show E + (f.prec : ℤ) - 1 = (εa + σ - 1) + (εb + σ - 1) by omega, two_zpow_add]
    exact mul_le_mul Sa.lo_le Sb.lo_le (abs_nonneg _) (two_zpow_pos _).le
  have hsum : P = ah * bh + al * bh + ah * bl + al * bl := by
    rw [show P = A * B from rfl, ← Sa.sum, ← Sb.sum]; ring
  -- a bound in units of a grid
  have rep : ∀ {z : ℚ} {g n : ℤ}, Mult z (max g Λ) → |z| ≤ 2 ^ n → n ≤ f.prec + g → n ≤ E + 2 * f.prec → Rep f z :=
    fun {z g n} hz hb hn hn2 => rep_of_mult hz (le_trans hΛ (le_max_right _ _))
      (le_trans hb (by rw [← zpow_natCast, ← two_zpow_add]; exact two_zpow_le (by have := le_max_left g Λ; omega)))
      (lt_of_le_of_lt hb (two_zpow_lt (by omega)))
  have hA_l : Mult A la := by rw [← Sa.sum]; exact mult_add Sa.hi_lam Sa.lo_lam
  have hB_l : Mult B lb := by rw [← Sb.sum]; exact mult_add Sb.hi_lam Sb.lo_lam
  have hPΛ : Mult P Λ := mult_mul hA_l hB_l
  obtain ⟨hxM, hxE⟩ := hxQ
  have hq1 : E + f.prec - 1 ≤ qOf f P := by
    have := qOf_ge (f := f) (r := P) hPa; omega
  have hq2 : qOf f P ≤ max (E + f.prec) f.qmin := by
    have := qOf_le (f := f) hP0 hPb; rwa [show E + 2 * (f.prec : ℤ) - f.prec = E + f.prec by ring] at this
  -- `x`: exact, or on the grid `Λ` with error at most `2^(E+p−1)`
  have hx : |x - P| ≤ 2 ^ (E + f.prec - 1) ∧ Mult x Λ := by
    have hexact : x = P → |x - P| ≤ 2 ^ (E + f.prec - 1) ∧ Mult x Λ := fun h => by
      rw [h, _root_.sub_self, abs_zero]; exact ⟨(two_zpow_pos _).le, hPΛ⟩
    by_cases h1 : Λ ≤ qOf f P
    · by_cases h2 : qOf f P ≤ E + f.prec
      · exact ⟨le_trans hxE (two_zpow_le (by omega)), mult_of_le hxM h1⟩
      · apply hexact
        have hq : qOf f P = f.qmin := by have := qOf_ge_qmin (f := f) P; omega
        have := mult_small (mult_sub hxM (mult_of_le hPΛ (by omega)))
          (lt_of_le_of_lt hxE (two_zpow_lt (by omega)))
        linarith
    · apply hexact
      rw [not_le] at h1
      refine near_eq hxN (rep_of_mult (mult_of_le hPΛ h1.le) (qOf_ge_qmin _) ?_ ?_)
      · rw [← zpow_natCast, ← two_zpow_add, _root_.add_comm]; exact (qOf_lt hP0).le
      · exact lt_trans hP (two_zpow_lt (by omega))
  obtain ⟨hB0, hxΛ⟩ := hx
  have hxE1 : Mult x (E + f.prec - 1) := mult_of_le hxM hq1
  have b0 := abs_le.mp hB0
  have b1 := abs_le.mp hlhB
  have b2 := abs_le.mp hhlB
  have b3 := abs_le.mp hllB
  have pw : ∀ k : ℤ, (2 : ℚ) ^ k + 2 ^ k = 2 ^ (k + 1) := fun k => by rw [zpow_add_one₀ two_ne_zero]; ring
  -- the chain
  have e1 : x - ah * bh = (x - P) + al * bh + ah * bl + al * bl := by rw [hsum]; ring
  have r1 : Rep f (x - ah * bh) := by
    refine rep (g := E + f.prec - 1) (n := E + 2 * f.prec - 1) (mult_sub (mult_max hxE1 hxΛ)
      (mult_of_le hhhM (max_le_max (by omega) le_rfl))) ?_ (by omega) (by omega)
    rw [e1]
    have h1 : (2 : ℚ) ^ (E + f.prec - 1) ≤ 2 ^ (E + 2 * f.prec - 3) := two_zpow_le (by omega)
    have h2 : (2 : ℚ) ^ (E + f.prec + σ - 1) ≤ 2 ^ (E + 2 * f.prec - 3) := two_zpow_le (by omega)
    have h3 := pw (E + 2 * f.prec - 3)
    have h4 := pw (E + 2 * f.prec - 2)
    rw [show E + 2 * (f.prec : ℤ) - 3 + 1 = E + 2 * f.prec - 2 by ring] at h3
    rw [show E + 2 * (f.prec : ℤ) - 2 + 1 = E + 2 * f.prec - 1 by ring] at h4
    exact abs_le.mpr ⟨by linarith, by linarith⟩
  have e2 : x - ah * bh - al * bh = (x - P) + ah * bl + al * bl := by rw [hsum]; ring
  have r2 : Rep f (x - ah * bh - al * bh) := by
    refine rep (g := E + σ) (n := E + f.prec + σ) (mult_sub
      (mult_of_le (mult_sub (mult_max hxE1 hxΛ) (mult_of_le hhhM (max_le_max (by omega) le_rfl)))
        (max_le_max (by omega) le_rfl)) hlhM) ?_ (by omega) (by omega)
    rw [e2]
    have h1 : (2 : ℚ) ^ (E + f.prec - 1) ≤ 2 ^ (E + f.prec + σ - 2) := two_zpow_le (by omega)
    have h3 := pw (E + f.prec + σ - 2)
    have h4 := pw (E + f.prec + σ - 1)
    rw [show E + (f.prec : ℤ) + σ - 2 + 1 = E + f.prec + σ - 1 by ring] at h3
    rw [show E + (f.prec : ℤ) + σ - 1 + 1 = E + f.prec + σ by ring] at h4
    exact abs_le.mpr ⟨by linarith, by linarith⟩
  have hMt2 : Mult (x - ah * bh - al * bh) (max E Λ) := mult_of_le (mult_sub
      (mult_of_le (mult_sub (mult_max hxE1 hxΛ) (mult_of_le hhhM (max_le_max (by omega) le_rfl)))
        (max_le_max (by omega) le_rfl)) hlhM) (max_le_max (by omega) le_rfl)
  have e3 : x - ah * bh - al * bh - ah * bl = (x - P) + al * bl := by rw [hsum]; ring
  have hMt3 : Mult (x - ah * bh - al * bh - ah * bl) (max E Λ) :=
    mult_sub hMt2 (mult_of_le hhlM (max_le_max (by omega) le_rfl))
  have r3 : Rep f (x - ah * bh - al * bh - ah * bl) := by
    refine rep (g := E) (n := E + f.prec) hMt3 ?_ (by omega) (by omega)
    rw [e3]
    have h4 := pw (E + f.prec - 1)
    rw [show E + (f.prec : ℤ) - 1 + 1 = E + f.prec by ring] at h4
    exact abs_le.mpr ⟨by linarith, by linarith⟩
  have e4 : al * bl - (x - ah * bh - al * bh - ah * bl) = P - x := by rw [e3]; ring
  have r4 : Rep f (al * bl - (x - ah * bh - al * bh - ah * bl)) := by
    refine rep (g := E) (n := E + f.prec - 1) (mult_sub hllM hMt3) ?_ (by omega) (by omega)
    rw [e4, abs_sub_comm]; exact hB0
  have rr : ∀ {z : ℚ} {g n : ℤ}, Mult z (max g Λ) → |z| ≤ 2 ^ n → n ≤ f.prec + g → n ≤ E + 2 * f.prec → Rep f z :=
    fun hz hb hn hn2 => rep hz hb hn hn2
  exact ⟨rr hhhM hhhB (by omega) le_rfl, rr hlhM hlhB (by omega) (by omega), rr hhlM hhlB (by omega) (by omega),
    rr hllM hllB (by omega) (by omega), r1, r2, r3, r4, e4⟩

/-- `forge-math`'s `two_product`: `x = a ⊗ b` and `y = alo ⊗ blo ⊖ err`. -/
def twoProduct (c a b : FP f) : FP f × FP f :=
  let x := mul a b
  let ah := (split c a).1
  let al := (split c a).2
  let bh := (split c b).1
  let bl := (split c b).2
  let err := sub (sub (sub x (mul ah bh)) (mul al bh)) (mul ah bl)
  (x, sub (mul al bl) err)

lemma rep_zero : Rep f 0 := fun zs => by simp [round, isFinite, val]

lemma mul_of_rep {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (h : Rep f (x.val * y.val)) :
    (mul x y).isFinite ∧ (mul x y).val = x.val * y.val := by
  rw [mul_eq_round hx hy]; exact h _

/-- A sum at most `2^emax` in magnitude rounds to a finite datum. -/
lemma round_finite_of_le_emax {zs : Bool} {r : ℚ} (h : |r| ≤ 2 ^ f.emax) : (round f zs r).isFinite := by
  by_cases hr : r = 0
  · subst hr; simp [round, isFinite]
  rw [round_finite_iff (K := ℚ) f hr]
  have hpow : (2 : ℚ) ^ f.emax = ((2 ^ (f.prec - 1) : ℕ) : ℚ) * 2 ^ f.qmax := by
    push_cast
    rw [← zpow_natCast, ← two_zpow_add, f.prec_sub_one]; congr 1; unfold Format.qmax; ring
  have hex : uround f ((2 : ℚ) ^ f.emax) = 2 ^ f.emax := by
    rw [hpow, uround_exact f (by positivity) f.qmin_le_qmax (Nat.pow_le_pow_right (by norm_num) (by omega))]
  calc uround f |r| ≤ uround f (2 ^ f.emax) := uround_mono f (abs_pos.mpr hr) h
    _ = 2 ^ f.emax := hex
    _ < 2 ^ (f.emax + 1) := two_zpow_lt (by omega)

/-- The split of a finite datum is finite, and of zero is zero. -/
lemma split_zero {c a : FP f} (hc : c.isFinite) (ha : a.isFinite) (h0 : a.val = 0) :
    (split c a).1.isFinite ∧ (split c a).1.val = 0 ∧ (split c a).2.isFinite ∧ (split c a).2.val = 0 := by
  obtain ⟨hg, hg'⟩ := mul_of_rep hc ha (by rw [h0, mul_zero]; exact rep_zero)
  rw [h0, mul_zero] at hg'
  obtain ⟨hd, hd'⟩ := sub_of_rep hg ha (by rw [hg', h0, _root_.sub_self]; exact rep_zero)
  rw [hg', h0, _root_.sub_self] at hd'
  obtain ⟨hh, hh'⟩ := sub_of_rep hg hd (by rw [hg', hd', _root_.sub_self]; exact rep_zero)
  rw [hg', hd', _root_.sub_self] at hh'
  obtain ⟨hl, hl'⟩ := sub_of_rep ha hh (by rw [h0, hh', _root_.sub_self]; exact rep_zero)
  rw [h0, hh', _root_.sub_self] at hl'
  exact ⟨hh, hh', hl, hl'⟩

/-- **Dekker's product is error-free.** With the precision `2σ − 1` (binary64: `σ = 27`),
`c = SPLITTER = 2^σ + 1`, `SPLITTER · a` and `SPLITTER · b` finite, `a` and `b` on grids `2^la`,
`2^lb` with `la + lb ≥ qmin` (the kernel's `product_exact`: the product does not underflow
inexactly) and `|a · b| < 2^(emax−1)`: `y` is finite and `x + y = a · b`. -/
theorem two_product_exact {σ : ℕ} (hp : (f.prec : ℤ) = 2 * σ - 1) (hσ : 3 ≤ σ) {c a b : FP f}
    (hc : c.isFinite) (hcv : c.val = 2 ^ σ + 1) (ha : a.isFinite) (hb : b.isFinite)
    (hca : (mul c a).isFinite) (hcb : (mul c b).isFinite) {la lb : ℤ} (hla : Mult a.val la)
    (hlb : Mult b.val lb) (hΛ : f.qmin ≤ la + lb) (hP : |a.val * b.val| < 2 ^ (f.emax - 1)) :
    (twoProduct c a b).1 = mul a b ∧ (twoProduct c a b).2.isFinite ∧
      (twoProduct c a b).1.val + (twoProduct c a b).2.val = a.val * b.val := by
  have hσp : σ + 1 ≤ f.prec := by omega
  have hσ2 : 2 ≤ σ := by omega
  have hxr : mul a b = round f (a.sign ^^ b.sign) (a.val * b.val) := mul_eq_round ha hb
  have hx : (mul a b).isFinite := by
    rw [hxr]; exact round_finite_of_le_emax (le_trans hP.le (two_zpow_le (by omega)))
  set x := mul a b
  set ah := (split c a).1
  set al := (split c a).2
  set bh := (split c b).1
  set bl := (split c b).2
  refine ⟨rfl, ?_⟩
  show (sub (mul al bl) (sub (sub (sub x (mul ah bh)) (mul al bh)) (mul ah bl))).isFinite ∧
    x.val + (sub (mul al bl) (sub (sub (sub x (mul ah bh)) (mul al bh)) (mul ah bl))).val = a.val * b.val
  by_cases hz : a.val = 0 ∨ b.val = 0
  · -- a zero factor: every value is zero
    have hsa : ah.isFinite ∧ al.isFinite ∧ (a.val = 0 → ah.val = 0 ∧ al.val = 0) := by
      by_cases h0 : a.val = 0
      · obtain ⟨h1, h2, h3, h4⟩ := split_zero hc ha h0; exact ⟨h1, h3, fun _ => ⟨h2, h4⟩⟩
      · obtain ⟨h1, h2, _⟩ := veltkamp hσ2 hσp hc hcv ha hca h0 hla; exact ⟨h1, h2, fun h => absurd h h0⟩
    have hsb : bh.isFinite ∧ bl.isFinite ∧ (b.val = 0 → bh.val = 0 ∧ bl.val = 0) := by
      by_cases h0 : b.val = 0
      · obtain ⟨h1, h2, h3, h4⟩ := split_zero hc hb h0; exact ⟨h1, h3, fun _ => ⟨h2, h4⟩⟩
      · obtain ⟨h1, h2, _⟩ := veltkamp hσ2 hσp hc hcv hb hcb h0 hlb; exact ⟨h1, h2, fun h => absurd h h0⟩
    have hprod : ah.val * bh.val = 0 ∧ al.val * bh.val = 0 ∧ ah.val * bl.val = 0 ∧ al.val * bl.val = 0 := by
      rcases hz with h | h
      · obtain ⟨e1, e2⟩ := hsa.2.2 h; simp [e1, e2]
      · obtain ⟨e1, e2⟩ := hsb.2.2 h; simp [e1, e2]
    have hP0 : a.val * b.val = 0 := by rcases hz with h | h <;> simp [h]
    obtain ⟨hx1, hx2⟩ : x.isFinite ∧ x.val = 0 := by
      have := mul_of_rep ha hb (by rw [hP0]; exact rep_zero); rw [hP0] at this; exact this
    obtain ⟨p1, p1'⟩ := mul_of_rep hsa.1 hsb.1 (by rw [hprod.1]; exact rep_zero)
    obtain ⟨p2, p2'⟩ := mul_of_rep hsa.2.1 hsb.1 (by rw [hprod.2.1]; exact rep_zero)
    obtain ⟨p3, p3'⟩ := mul_of_rep hsa.1 hsb.2.1 (by rw [hprod.2.2.1]; exact rep_zero)
    obtain ⟨p4, p4'⟩ := mul_of_rep hsa.2.1 hsb.2.1 (by rw [hprod.2.2.2]; exact rep_zero)
    rw [hprod.1] at p1'; rw [hprod.2.1] at p2'; rw [hprod.2.2.1] at p3'; rw [hprod.2.2.2] at p4'
    obtain ⟨t1, t1'⟩ := sub_of_rep hx1 p1 (by rw [hx2, p1', _root_.sub_self]; exact rep_zero)
    rw [hx2, p1', _root_.sub_self] at t1'
    obtain ⟨t2, t2'⟩ := sub_of_rep t1 p2 (by rw [t1', p2', _root_.sub_self]; exact rep_zero)
    rw [t1', p2', _root_.sub_self] at t2'
    obtain ⟨t3, t3'⟩ := sub_of_rep t2 p3 (by rw [t2', p3', _root_.sub_self]; exact rep_zero)
    rw [t2', p3', _root_.sub_self] at t3'
    obtain ⟨y, y'⟩ := sub_of_rep p4 t3 (by rw [p4', t3', _root_.sub_self]; exact rep_zero)
    rw [p4', t3', _root_.sub_self] at y'
    exact ⟨y, by rw [hx2, y', hP0]; ring⟩
  rw [not_or] at hz
  obtain ⟨ha1, ha2, εa, Sa⟩ := veltkamp hσ2 hσp hc hcv ha hca hz.1 hla
  obtain ⟨hb1, hb2, εb, Sb⟩ := veltkamp hσ2 hσp hc hcv hb hcb hz.2 hlb
  have hP0 : a.val * b.val ≠ 0 := mul_ne_zero hz.1 hz.2
  obtain ⟨r1, r2, r3, r4, r5, r6, r7, r8, e⟩ := dekker_core (x := x.val) hp hσ Sa Sb hΛ hP
    (fun z hz => by rw [hxr]; exact round_nearest (by rw [← hxr]; exact hx) hz)
    (by rw [hxr]; exact round_grid hP0 (by rw [← hxr]; exact hx))
  obtain ⟨p1, p1'⟩ := mul_of_rep ha1 hb1 r1
  obtain ⟨p2, p2'⟩ := mul_of_rep ha2 hb1 r2
  obtain ⟨p3, p3'⟩ := mul_of_rep ha1 hb2 r3
  obtain ⟨p4, p4'⟩ := mul_of_rep ha2 hb2 r4
  obtain ⟨t1, t1'⟩ := sub_of_rep hx p1 (by rw [p1']; exact r5)
  obtain ⟨t2, t2'⟩ := sub_of_rep t1 p2 (by rw [t1', p1', p2']; exact r6)
  obtain ⟨t3, t3'⟩ := sub_of_rep t2 p3 (by rw [t2', t1', p1', p2', p3']; exact r7)
  obtain ⟨y, y'⟩ := sub_of_rep p4 t3 (by rw [t3', t2', t1', p1', p2', p3', p4']; exact r8)
  refine ⟨y, ?_⟩
  rw [y', t3', t2', t1', p1', p2', p3', p4', e]; ring

namespace F64

/-- `forge-math`'s `SPLITTER`, `2^27 + 1`. -/
def splitter : F64 := round binary64 false ((2 : ℚ) ^ 27 + 1)

lemma splitter_spec : splitter.isFinite ∧ splitter.val = 2 ^ 27 + 1 := by
  have := rep_grid (f := binary64) (k := 2 ^ 27 + 1) (g := 0) (by simp) (by simp) (by
    have : (134217729 : ℚ) < 2 ^ (1024 : ℤ) := by
      rw [show (1024 : ℤ) = ((1024 : ℕ) : ℤ) from rfl, zpow_natCast]
      exact lt_of_lt_of_le (by norm_num : (134217729 : ℚ) < 2 ^ 28) (pow_le_pow_right₀ (by norm_num) (by norm_num))
    simpa using this)
  have e : (((2 ^ 27 + 1 : ℤ)) : ℚ) * 2 ^ (0 : ℤ) = (2 : ℚ) ^ 27 + 1 := by norm_num
  rw [e] at this
  exact this false

/-- **`two_product` on `f64`**: `x + y = a · b` exactly, for finite `a`, `b` with
`SPLITTER · a` and `SPLITTER · b` finite, `|a · b| < 2^1022`, and the kernel's `product_exact`
(`a`, `b` on grids `2^la`, `2^lb` with `la + lb ≥ −1074`). -/
theorem two_product_exact {a b : F64} (ha : a.isFinite) (hb : b.isFinite)
    (hca : (mul splitter a).isFinite) (hcb : (mul splitter b).isFinite) {la lb : ℤ}
    (hla : Mult a.val la) (hlb : Mult b.val lb) (hΛ : -1074 ≤ la + lb) (hP : |a.val * b.val| < 2 ^ 1022) :
    (twoProduct splitter a b).1 = mul a b ∧ (twoProduct splitter a b).2.isFinite ∧
      (twoProduct splitter a b).1.val + (twoProduct splitter a b).2.val = a.val * b.val :=
  IEEE754.two_product_exact (σ := 27) (by simp) (by norm_num) splitter_spec.1 (by simp [splitter_spec.2])
    ha hb hca hcb hla hlb (by simpa using hΛ) (by simpa using hP)

end F64

end IEEE754
