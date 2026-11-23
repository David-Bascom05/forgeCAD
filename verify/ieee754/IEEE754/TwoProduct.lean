/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.TwoSum

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

end IEEE754
