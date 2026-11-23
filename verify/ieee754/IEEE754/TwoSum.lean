/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.Nearest

/-!
# Toward TwoSum: the error of an addition is representable

`add_err_representable`: for finite `a`, `b` with a finite sum `s = a ⊕ b`, the error
`δ = a + b − s` rounds to itself. This is the step that makes TwoSum's (and Fast2Sum's) last
operation exact.
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {f : Format}

/-- A finite rounding of `r ≠ 0` is `k · 2^e` with `e` its quantum exponent, `|k| ≤ 2^prec`. -/
lemma round_repr {zs : Bool} {r : ℚ} (hr : r ≠ 0) (hfin : (round f zs r).isFinite) :
    ∃ k : ℤ, (round f zs r).val = k * 2 ^ (roundPos f |r|).2 ∧
      |(round f zs r).val - r| ≤ 2 ^ ((roundPos f |r|).2 - 1) := by
  have ha : 0 < |r| := abs_pos.mpr hr
  have hv := abs_round_val (K := ℚ) (f := f) hr hfin
  have he := round_val_abs (K := ℚ) f hr hfin
  simp only [Rat.cast_id] at hv he
  refine ⟨if (round f zs r).val < 0 then -((roundPos f |r|).1 : ℤ) else (roundPos f |r|).1, ?_, ?_⟩
  · split_ifs with h
    · rw [abs_of_neg h] at hv; push_cast; simp only [uround] at hv; linarith
    · rw [abs_of_nonneg (not_lt.mp h)] at hv; push_cast; simp only [uround] at hv; linarith
  · rw [he]; exact uround_err f ha

/-- **The error of an addition is representable.** -/
theorem add_err_representable {a b : FP f} (ha : a.isFinite) (hb : b.isFinite)
    (hs : (add a b).isFinite) (zs : Bool) :
    (round f zs (a.val + b.val - (add a b).val)).isFinite ∧
      (round f zs (a.val + b.val - (add a b).val)).val = a.val + b.val - (add a b).val := by
  set s := add a b
  set δ := a.val + b.val - s.val
  obtain ⟨ka, ga, hka, hga, hva, hav⟩ := FP.val_repr ha
  obtain ⟨kb, gb, hkb, hgb, hvb, hbv⟩ := FP.val_repr hb
  by_cases h0 : a.val + b.val = 0
  · have : s.val = 0 := by
      show (add a b).val = 0
      rw [add_eq_round ha hb, h0]; simp [val]
    have : δ = 0 := by simp only [δ]; rw [h0, this]; ring
    rw [this]; simp [isFinite, val]
  -- `s = ks · 2^e`, `|δ| ≤ 2^(e-1)`
  have hsr : s = round f (a.sign && b.sign) (a.val + b.val) := add_eq_round ha hb
  obtain ⟨ks, hks, herr⟩ := round_repr (f := f) (zs := a.sign && b.sign) h0 (by rw [← hsr]; exact hs)
  rw [← hsr] at hks herr
  set e := (roundPos f |a.val + b.val|).2
  have he : f.qmin ≤ e := roundPos_ge_qmin f _
  -- `|δ| ≤ |a|` and `|δ| ≤ |b|`: `s` is at least as close to `a + b` as `a` and `b` are
  have hna : |δ| ≤ |b.val| := by
    have := round_nearest (f := f) (zs := a.sign && b.sign) (r := a.val + b.val) (by rw [← hsr]; exact hs) ha
    rw [← hsr] at this
    rw [show δ = -(s.val - (a.val + b.val)) by simp only [δ]; ring, abs_neg]
    calc |s.val - (a.val + b.val)| ≤ |a.val - (a.val + b.val)| := this
      _ = |b.val| := by rw [show a.val - (a.val + b.val) = -b.val by ring, abs_neg]
  have hnb : |δ| ≤ |a.val| := by
    have := round_nearest (f := f) (zs := a.sign && b.sign) (r := a.val + b.val) (by rw [← hsr]; exact hs) hb
    rw [← hsr] at this
    rw [show δ = -(s.val - (a.val + b.val)) by simp only [δ]; ring, abs_neg]
    calc |s.val - (a.val + b.val)| ≤ |b.val - (a.val + b.val)| := this
      _ = |a.val| := by rw [show b.val - (a.val + b.val) = -a.val by ring, abs_neg]
  have hne : |δ| ≤ 2 ^ (e - 1) := by
    rw [show δ = -(s.val - (a.val + b.val)) by simp only [δ]; ring, abs_neg]; exact herr
  -- `δ` is a multiple of `2^g`, `g = min ga gb e`
  set g := min (min ga gb) e
  obtain ⟨pa, hpa⟩ := FP.two_zpow_dvd (show g ≤ ga by omega)
  obtain ⟨pb, hpb⟩ := FP.two_zpow_dvd (show g ≤ gb by omega)
  obtain ⟨pe, hpe⟩ := FP.two_zpow_dvd (show g ≤ e by omega)
  set k : ℤ := ka * pa + kb * pb - ks * pe
  have hk : δ = (k : ℚ) * 2 ^ g := by
    simp only [δ, k]; rw [hva, hvb, hks, hpa, hpb, hpe]; push_cast; ring
  have hpos : (0 : ℚ) < 2 ^ g := two_zpow_pos g
  have hkd : |(k : ℚ)| * 2 ^ g = |δ| := by rw [hk, abs_mul, abs_of_pos hpos]
  -- `|k| ≤ 2^prec`: the grid is that of `a`, of `b`, or of `s` (then `δ = 0`)
  have hkb' : |k| ≤ 2 ^ f.prec := by
    have bound : |(k : ℚ)| * 2 ^ g ≤ (2 ^ f.prec : ℚ) * 2 ^ g := by
      rw [hkd]
      rcases le_total ga gb with h1 | h1 <;> rcases le_total (min ga gb) e with h2 | h2
      · have hg : g = ga := by simp only [g]; omega
        rw [hg]; refine le_trans hnb ?_; rw [hav]
        exact mul_le_mul_of_nonneg_right (by exact_mod_cast hka.le) (two_zpow_pos _).le
      · have hg : g = e := by simp only [g]; omega
        rw [hg]; refine le_trans hne ?_
        calc (2 : ℚ) ^ (e - 1) ≤ 2 ^ e := two_zpow_le (by omega)
          _ ≤ 2 ^ f.prec * 2 ^ e := le_mul_of_one_le_left (two_zpow_pos _).le
              (one_le_pow₀ (by norm_num))
      · have hg : g = gb := by simp only [g]; omega
        rw [hg]; refine le_trans hna ?_; rw [hbv]
        exact mul_le_mul_of_nonneg_right (by exact_mod_cast hkb.le) (two_zpow_pos _).le
      · have hg : g = e := by simp only [g]; omega
        rw [hg]; refine le_trans hne ?_
        calc (2 : ℚ) ^ (e - 1) ≤ 2 ^ e := two_zpow_le (by omega)
          _ ≤ 2 ^ f.prec * 2 ^ e := le_mul_of_one_le_left (two_zpow_pos _).le
              (one_le_pow₀ (by norm_num))
    have := le_of_mul_le_mul_right bound hpos
    exact_mod_cast this
  have hlt : |(k : ℚ) * 2 ^ g| < 2 ^ (f.emax + 1) := by
    rw [← hk]; exact lt_of_le_of_lt hnb (FP.abs_val_lt ha)
  have hg : f.qmin ≤ g := by simp only [g]; omega
  have := round_exact_q (f := f) zs hkb' hg hlt
  rw [← hk] at this
  exact this

/-! ## Fast2Sum: `(a ⊕ b) ⊖ a` is exact when `|b| ≤ |a|` -/

/-- `v` is representable: every rounding of it is finite and equals it. -/
def Rep (f : Format) (v : ℚ) : Prop := ∀ zs, (round f zs v).isFinite ∧ (round f zs v).val = v

lemma rep_of_round {zs : Bool} {v : ℚ} (h : (round f zs v).isFinite ∧ (round f zs v).val = v) :
    Rep f v := by
  intro zs'
  by_cases h0 : v = 0
  · subst h0; simp [round, isFinite, val]
  · have : round f zs' v = round f zs v := by simp [round, h0]
    rw [this]; exact h

lemma rep_grid {k g : ℤ} (hk : |k| ≤ 2 ^ f.prec) (hg : f.qmin ≤ g)
    (hlt : |(k : ℚ) * 2 ^ g| < 2 ^ (f.emax + 1)) : Rep f ((k : ℚ) * 2 ^ g) :=
  fun zs => round_exact_q zs hk hg hlt

lemma rep_val {x : FP f} (hx : x.isFinite) : Rep f x.val := by
  obtain ⟨k, g, hk, hg, hv, _⟩ := FP.val_repr hx
  rw [hv]; exact rep_grid hk.le hg (by rw [← hv]; exact FP.abs_val_lt hx)

lemma finalize_true (n : ℕ) (e : ℤ) : finalize f true n e = (finalize f false n e).neg := by
  unfold finalize
  by_cases h0 : n = 0
  · simp [h0, FP.neg]
  simp only [h0, ite_false]
  generalize (if n = 2 ^ f.prec then (2 ^ (f.prec - 1), e + 1) else (n, e) : ℕ × ℤ) = p
  by_cases h1 : f.qmax < p.2
  · simp [h1, FP.neg]
  by_cases h2 : Canon f p.1 p.2 <;> simp [h1, h2, FP.neg]

lemma round_neg_eq {zs zs' : Bool} {r : ℚ} (hr : r ≠ 0) : round f zs (-r) = (round f zs' r).neg := by
  rcases lt_or_gt_of_ne hr with h | h
  · have h1 : ¬ (0 : ℚ) < r := not_lt.mpr h.le
    have h2 : (0 : ℚ) < -r := by linarith
    simp only [round, neg_eq_zero, hr, h1, h2, ite_false, ite_true, finalize_true, FP.neg_neg]
  · have h1 : ¬ (0 : ℚ) < -r := by linarith
    simp only [round, neg_eq_zero, hr, h, h1, ite_false, ite_true, _root_.neg_neg, finalize_true]

lemma rep_neg {v : ℚ} (h : Rep f v) : Rep f (-v) := by
  by_cases h0 : v = 0
  · subst h0; simpa using h
  intro zs
  rw [round_neg_eq (zs' := zs) h0, FP.isFinite_neg, FP.val_neg, (h zs).2]
  exact ⟨(h zs).1, rfl⟩

lemma rep_sub {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (h1 : y.val / 2 ≤ x.val)
    (h2 : x.val ≤ 2 * y.val) : Rep f (x.val - y.val) := by
  have := sterbenz hx hy h1 h2
  rw [sub_eq_round hx hy] at this
  exact rep_of_round this

/-- Fast2Sum's core, for `a ≥ 0`: `s` is any finite datum nearest to `a + b`. -/
lemma fast2sum_core {a b s : FP f} (ha : a.isFinite) (hb : b.isFinite) (hs : s.isFinite)
    (hN : ∀ x : FP f, x.isFinite → |s.val - (a.val + b.val)| ≤ |x.val - (a.val + b.val)|)
    (ha0 : 0 ≤ a.val) (hab : |b.val| ≤ a.val) : Rep f (s.val - a.val) := by
  obtain ⟨ka, ga, hka, hga, hva, hav⟩ := FP.val_repr ha
  set A := a.val
  set B := b.val
  set S := s.val
  have hNa := hN a ha
  have hN0 := hN (FP.zero false) (by simp [isFinite])
  simp only [FP.val_zero, zero_sub, abs_neg] at hN0
  rw [show A - (A + B) = -B by ring, abs_neg] at hNa
  -- a representable `r` is `s`
  have exact_r : Rep f (A + B) → S = A + B := by
    intro hr
    have := hN _ (hr false).1
    rw [(hr false).2, _root_.sub_self, abs_zero] at this
    linarith [abs_nonneg (S - (A + B)), abs_nonpos_iff.mp this]
  rcases le_or_gt 0 B with hB | hB
  · -- same signs: `a ≤ s ≤ 2a`
    rw [abs_of_nonneg hB] at hNa hab
    have hS1 : A ≤ S := by linarith [neg_abs_le (S - (A + B))]
    have hS2 : S ≤ 2 * A := by
      by_cases hbig : (2 : ℚ) ^ f.emax ≤ A
      · have := FP.abs_val_lt hs
        rw [zpow_add_one₀ two_ne_zero] at this
        linarith [le_abs_self S]
      · rw [not_le] at hbig
        have h2 : (2 : ℚ) * A = (ka : ℚ) * 2 ^ (ga + 1) := by
          rw [hva, zpow_add_one₀ two_ne_zero]; ring
        have hr : Rep f (2 * A) := by
          rw [h2]; refine rep_grid hka.le (by omega) ?_
          rw [← h2, abs_of_nonneg (by linarith), zpow_add_one₀ two_ne_zero]; linarith
        have := hN _ (hr false).1
        rw [(hr false).2, abs_of_nonneg (by linarith : (0 : ℚ) ≤ 2 * A - (A + B))] at this
        linarith [le_abs_self (S - (A + B))]
    exact rep_sub hs ha (by linarith) hS2
  · -- opposite signs: `0 ≤ s ≤ a`
    rw [abs_of_neg hB] at hNa hab
    have hr0 : 0 ≤ A + B := by linarith
    rw [abs_of_nonneg hr0] at hN0
    have hS2 : S ≤ A := by linarith [le_abs_self (S - (A + B))]
    rcases le_or_gt (A / 2) S with hS | hS
    · exact rep_sub hs ha hS (by linarith)
    · -- `r` is representable, so `s = r` and `s - a = b`
      have hr : Rep f (A + B) := by
        rcases le_or_gt (A / 2) (-B) with hB2 | hB2
        · have := rep_sub ha (by simpa using hb : b.neg.isFinite)
            (by rw [FP.val_neg]; linarith) (by rw [FP.val_neg]; linarith)
          rwa [FP.val_neg, sub_neg_eq_add] at this
        · rcases lt_or_eq_of_le hga with hq | hq
          · -- `a / 2` is representable and closer to `r` than `s` would be
            exfalso
            have h2 : A / 2 = (ka : ℚ) * 2 ^ (ga - 1) := by
              rw [hva, zpow_sub_one₀ two_ne_zero]; ring
            have hh : Rep f (A / 2) := by
              rw [h2]; refine rep_grid hka.le (by omega) ?_
              rw [← h2, abs_of_nonneg (by linarith)]
              have := FP.abs_val_lt ha
              rw [abs_of_nonneg ha0] at this; linarith
            have := hN _ (hh false).1
            rw [(hh false).2, abs_of_neg (by linarith : A / 2 - (A + B) < 0),
              abs_of_neg (by linarith : S - (A + B) < 0)] at this
            linarith
          · -- `a` is on the finest grid, and so is `r`, with `|r| ≤ |a|`
            obtain ⟨kb, gb, _, hgb, hvb, _⟩ := FP.val_repr hb
            obtain ⟨pb, hpb⟩ := FP.two_zpow_dvd (show ga ≤ gb by omega)
            have hk : A + B = ((ka + kb * pb : ℤ) : ℚ) * 2 ^ ga := by
              rw [hva, show B = b.val from rfl, hvb, hpb]; push_cast; ring
            have hpos : (0 : ℚ) < 2 ^ ga := two_zpow_pos ga
            rw [hk]; refine rep_grid ?_ hga ?_
            · have : |((ka + kb * pb : ℤ) : ℚ)| * 2 ^ ga ≤ (2 ^ f.prec : ℚ) * 2 ^ ga := by
                rw [← abs_of_pos hpos, ← abs_mul, abs_of_pos hpos, ← hk, abs_of_nonneg hr0]
                calc A + B ≤ A := by linarith
                  _ = |A| := (abs_of_nonneg ha0).symm
                  _ = |(ka : ℚ)| * 2 ^ ga := by rw [hav]; push_cast; rfl
                  _ ≤ _ := mul_le_mul_of_nonneg_right (by exact_mod_cast hka.le) hpos.le
              exact_mod_cast le_of_mul_le_mul_right this hpos
            · rw [← hk, abs_of_nonneg hr0]
              have := FP.abs_val_lt ha
              rw [abs_of_nonneg ha0] at this; linarith
      rw [exact_r hr, show A + B - A = B by ring]
      exact rep_val hb

/-- **Fast2Sum.** If `|b| ≤ |a|`, the error-free transform's second step `(a ⊕ b) ⊖ a` is exact. -/
theorem fast2sum {a b : FP f} (ha : a.isFinite) (hb : b.isFinite) (hs : (add a b).isFinite)
    (hab : |b.val| ≤ |a.val|) :
    (sub (add a b) a).isFinite ∧ (sub (add a b) a).val = (add a b).val - a.val := by
  rw [sub_eq_round hs ha]
  refine (?_ : Rep f ((add a b).val - a.val)) _
  have hN : ∀ x : FP f, x.isFinite →
      |(add a b).val - (a.val + b.val)| ≤ |x.val - (a.val + b.val)| := by
    intro x hx
    have := round_nearest (r := a.val + b.val) (by rw [← add_eq_round ha hb]; exact hs) hx
    rwa [← add_eq_round ha hb] at this
  rcases le_or_gt 0 a.val with ha0 | ha0
  · exact fast2sum_core ha hb hs hN ha0 (by rwa [abs_of_nonneg ha0] at hab)
  · have := fast2sum_core (a := a.neg) (b := b.neg) (s := (add a b).neg)
      (by simpa using ha) (by simpa using hb) (by simpa using hs)
      (by
        intro x hx
        have := hN x.neg (by simpa using hx)
        simp only [FP.val_neg] at this ⊢
        rw [show -(add a b).val - (-a.val + -b.val) = -((add a b).val - (a.val + b.val)) by ring,
          abs_neg, show x.val - (-a.val + -b.val) = -(-x.val - (a.val + b.val)) by ring, abs_neg]
        exact this)
      (by simp; linarith) (by simp only [FP.val_neg, abs_neg]; rwa [abs_of_neg ha0] at hab)
    simp only [FP.val_neg] at this
    have := rep_neg this
    rwa [show -(-(add a b).val - -a.val) = (add a b).val - a.val by ring] at this

/-! ## The error-free transforms -/

/-- Fast2Sum (Dekker): `(s, e)` with `s = a ⊕ b` and `e = b ⊖ (s ⊖ a)`. -/
def fastTwoSum (a b : FP f) : FP f × FP f :=
  let s := add a b
  (s, sub b (sub s a))

/-- TwoSum (Knuth, Møller), as in `forge-math`'s `two_sum`. -/
def twoSum (a b : FP f) : FP f × FP f :=
  let s := add a b
  let bv := sub s a
  let av := sub s bv
  let br := sub b bv
  let ar := sub a av
  (s, add ar br)

lemma sub_of_rep {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (h : Rep f (x.val - y.val)) :
    (sub x y).isFinite ∧ (sub x y).val = x.val - y.val := by
  rw [sub_eq_round hx hy]; exact h _

lemma add_of_rep {x y : FP f} (hx : x.isFinite) (hy : y.isFinite) (h : Rep f (x.val + y.val)) :
    (add x y).isFinite ∧ (add x y).val = x.val + y.val := by
  rw [add_eq_round hx hy]; exact h _

/-- With `|b| ≤ |a|`, the second step and the error of TwoSum's `b` branch are exact. -/
lemma two_sum_steps {a b : FP f} (ha : a.isFinite) (hb : b.isFinite) (hs : (add a b).isFinite)
    (hab : |b.val| ≤ |a.val|) :
    (sub b (sub (add a b) a)).isFinite ∧
      (sub b (sub (add a b) a)).val = a.val + b.val - (add a b).val := by
  obtain ⟨h1, h2⟩ := fast2sum ha hb hs hab
  have hd : b.val - (sub (add a b) a).val = a.val + b.val - (add a b).val := by rw [h2]; ring
  have := sub_of_rep hb h1 (by rw [hd]; exact fun zs => add_err_representable ha hb hs zs)
  rwa [hd] at this

/-- **Fast2Sum is error-free.** If `|b| ≤ |a|` and `s = a ⊕ b` is finite, `s + e = a + b`. -/
theorem fast_two_sum_exact {a b : FP f} (ha : a.isFinite) (hb : b.isFinite)
    (hs : (add a b).isFinite) (hab : |b.val| ≤ |a.val|) :
    (fastTwoSum a b).1 = add a b ∧ (fastTwoSum a b).2.isFinite ∧
      (fastTwoSum a b).1.val + (fastTwoSum a b).2.val = a.val + b.val := by
  obtain ⟨h1, h2⟩ := two_sum_steps ha hb hs hab
  refine ⟨rfl, h1, ?_⟩
  show (add a b).val + (sub b (sub (add a b) a)).val = _
  rw [h2]; ring

/-- **TwoSum is error-free when `|b| ≤ |a|`.** -/
theorem two_sum_exact_of_le {a b : FP f} (ha : a.isFinite) (hb : b.isFinite)
    (hs : (add a b).isFinite) (hab : |b.val| ≤ |a.val|) :
    (twoSum a b).1 = add a b ∧ (twoSum a b).2.isFinite ∧
      (twoSum a b).1.val + (twoSum a b).2.val = a.val + b.val := by
  set s := add a b
  obtain ⟨hbv, hbv'⟩ := fast2sum ha hb hs hab
  set bv := sub s a
  -- `av = s ⊖ (s − a) = a`
  obtain ⟨hav, hav'⟩ := sub_of_rep hs hbv (by rw [hbv', show s.val - (s.val - a.val) = a.val by ring]; exact rep_val ha)
  rw [show s.val - bv.val = a.val by rw [hbv']; ring] at hav'
  set av := sub s bv
  -- `br = b ⊖ (s − a) = δ`
  obtain ⟨hbr, hbr'⟩ := two_sum_steps ha hb hs hab
  -- `ar = a ⊖ a = 0`
  obtain ⟨har, har'⟩ := sub_of_rep ha hav (by rw [hav', _root_.sub_self]; exact rep_of_round (zs := false) (by simp [round, isFinite, val]))
  rw [hav', _root_.sub_self] at har'
  set br := sub b bv
  set ar := sub a av
  obtain ⟨he, he'⟩ := add_of_rep har hbr (by rw [har', zero_add, hbr']; exact fun zs => add_err_representable ha hb hs zs)
  refine ⟨rfl, he, ?_⟩
  show s.val + (add ar br).val = _
  rw [he', har', hbr']; ring

/-! ## TwoSum when `|a| < |b|` (Knuth) -/

/-- Every finite value in canonical form: normal, or on the finest grid. -/
lemma val_repr_canon {x : FP f} (hx : x.isFinite) :
    ∃ k e : ℤ, |k| < 2 ^ f.prec ∧ f.qmin ≤ e ∧ x.val = k * 2 ^ e ∧ (2 ^ (f.prec - 1) ≤ |k| ∨ e = f.qmin) := by
  cases x with
  | zero s => exact ⟨0, f.qmin, by simp, le_rfl, by simp, Or.inr rfl⟩
  | fin s m e h =>
    obtain ⟨_, hmp, he1, _, hn⟩ := h
    refine ⟨if s then -(m : ℤ) else m, e, ?_, he1, ?_, ?_⟩
    · cases s <;> simp <;> exact_mod_cast hmp
    · cases s <;> simp [val, sgn]
    · rcases hn with hn | hn
      · left; cases s <;> simp <;> exact_mod_cast hn
      · exact Or.inr hn
  | _ => simp [isFinite] at hx

/-- A datum nearest to `r` is at least any finite value below `r`, at most any above. -/
lemma near_ge {S r : ℚ} (hN : ∀ x : FP f, x.isFinite → |S - r| ≤ |x.val - r|) {x : FP f} (hx : x.isFinite)
    (h : x.val ≤ r) : x.val ≤ S := by
  have := hN x hx
  rw [abs_of_nonpos (by linarith : x.val - r ≤ 0)] at this
  linarith [neg_abs_le (S - r)]

lemma near_le {S r : ℚ} (hN : ∀ x : FP f, x.isFinite → |S - r| ≤ |x.val - r|) {x : FP f} (hx : x.isFinite)
    (h : r ≤ x.val) : S ≤ x.val := by
  have := hN x hx
  rw [abs_of_nonneg (by linarith : 0 ≤ x.val - r)] at this
  linarith [le_abs_self (S - r)]

/-- Twice a finite `x ≥ 0` is finite, or at least the overflow threshold. -/
lemma two_x {x : FP f} (hx : x.isFinite) (h0 : 0 ≤ x.val) :
    (∃ y : FP f, y.isFinite ∧ y.val = 2 * x.val) ∨ 2 ^ (f.emax + 1) ≤ 2 * x.val := by
  rcases lt_or_ge (2 * x.val) (2 ^ (f.emax + 1)) with h | h
  · left
    obtain ⟨k, e, hk, he, hv, _⟩ := FP.val_repr hx
    have h2 : 2 * x.val = (k : ℚ) * 2 ^ (e + 1) := by rw [hv, zpow_add_one₀ two_ne_zero]; ring
    have hr := rep_grid (f := f) hk.le (show f.qmin ≤ e + 1 by omega) (by rw [← h2, abs_of_nonneg (by linarith)]; exact h)
    exact ⟨_, (hr false).1, by rw [(hr false).2, h2]⟩
  · exact Or.inr h

/-- A sum that rounds to a finite value is below the overflow threshold. -/
lemma abs_lt_of_round_finite {zs : Bool} {r : ℚ} (h : (round f zs r).isFinite) : |r| < 2 ^ (f.emax + 1) := by
  by_contra hc
  rw [not_lt] at hc
  have hr : r ≠ 0 := by intro e; rw [e, abs_zero] at hc; linarith [two_zpow_pos (K := ℚ) (f.emax + 1)]
  have hu := (round_finite_iff (K := ℚ) f hr).mp h
  have hpow : (2 : ℚ) ^ (f.emax + 1) = ((2 ^ (f.prec - 1) : ℕ) : ℚ) * 2 ^ (f.qmax + 1) := by
    push_cast
    rw [← zpow_natCast, ← two_zpow_add, f.prec_sub_one]; congr 1; unfold Format.qmax; ring
  have hex : uround f ((2 : ℚ) ^ (f.emax + 1)) = 2 ^ (f.emax + 1) := by
    rw [hpow, uround_exact f (by positivity) (by have := f.qmin_le_qmax; omega)
      (Nat.pow_le_pow_right (by norm_num) (by omega))]
  have := uround_mono f (two_zpow_pos _) hc
  rw [hex] at this
  linarith

/-- The core of TwoSum when `|a| < b`: `v` is nearest to `s − a`, `s` nearest to `a + b`. -/
lemma two_sum_core {a b s v : FP f} (ha : a.isFinite) (hb : b.isFinite) (hs : s.isFinite) (hv : v.isFinite)
    (hp : 3 ≤ f.prec)
    (hNs : ∀ x : FP f, x.isFinite → |s.val - (a.val + b.val)| ≤ |x.val - (a.val + b.val)|)
    (hNv : ∀ x : FP f, x.isFinite → |v.val - (s.val - a.val)| ≤ |x.val - (s.val - a.val)|)
    (hrel : |s.val - (a.val + b.val)| ≤ 2 ^ (-(f.prec : ℤ)) * |s.val|)
    (hov : |a.val + b.val| < 2 ^ (f.emax + 1))
    (hb0 : 0 < b.val) (hab : |a.val| < b.val) :
    Rep f (s.val - v.val) ∧ Rep f (b.val - v.val) := by
  obtain ⟨ka, ga, hka, hga, hva, _⟩ := FP.val_repr ha
  obtain ⟨kb, gb, _, hgb, hvb, _⟩ := FP.val_repr hb
  obtain ⟨ks, es, hks, hes, hvs, hcan⟩ := val_repr_canon hs
  set A := a.val
  set B := b.val
  set S := s.val
  set V := v.val
  have hA := abs_lt.mp hab
  -- `0 ≤ s ≤ 2b`, and the error is at most `b / 4`
  have hS0 : 0 ≤ S := near_ge hNs (x := FP.zero false) (by simp [isFinite]) (by simp; linarith)
  have hS2 : S ≤ 2 * B := by
    rcases two_x hb hb0.le with ⟨y, hy, hyv⟩ | h
    · rw [← hyv]; exact near_le hNs hy (by rw [hyv]; linarith)
    · have := FP.abs_val_lt hs; rw [abs_of_nonneg hS0] at this; linarith
  have hpow : (2 : ℚ) ^ (-(f.prec : ℤ)) ≤ 1 / 8 := by
    calc (2 : ℚ) ^ (-(f.prec : ℤ)) ≤ 2 ^ (-3 : ℤ) := two_zpow_le (by omega)
      _ = 1 / 8 := by norm_num
  have hd : |S - (A + B)| ≤ B / 4 := by
    rw [abs_of_nonneg hS0] at hrel
    calc _ ≤ 2 ^ (-(f.prec : ℤ)) * S := hrel
      _ ≤ 1 / 8 * (2 * B) := mul_le_mul hpow hS2 hS0 (by norm_num)
      _ = B / 4 := by ring
  have hdv := abs_le.mp hd
  -- `b/2 ≤ v ≤ 2b`, so `b − v` is exact
  have hvb := hNv b hb
  rw [show B - (S - A) = A + B - S by ring, show A + B - S = -(S - (A + B)) by ring, abs_neg] at hvb
  have hvb' := abs_le.mp (le_trans hvb hd)
  have hV0 : B / 2 ≤ V := by linarith
  have hV2 : V ≤ 2 * B := by linarith
  have P2 : Rep f (B - V) := rep_sub hb hv (by linarith) (by linarith)
  refine ⟨?_, P2⟩
  -- `s − v`
  have exact_v : Rep f (S - A) → Rep f (S - V) := by
    intro hr
    have := hNv _ (hr false).1
    rw [(hr false).2, _root_.sub_self, abs_zero] at this
    have hv' : V = S - A := by linarith [abs_nonneg (V - (S - A)), abs_nonpos_iff.mp this]
    rw [hv', show S - (S - A) = A by ring]
    exact rep_val ha
  rcases le_or_gt 0 A with hA0 | hA0
  · -- same signs: `b ≤ s`, `2a ≤ s`, so `s/2 ≤ s − a ≤ s`
    have hSB : B ≤ S := near_ge hNs hb (by linarith)
    have hS2a : 2 * A ≤ S := by
      rcases two_x ha hA0 with ⟨y, hy, hyv⟩ | h
      · rw [← hyv]; exact near_ge hNs hy (by rw [hyv]; linarith)
      · rw [abs_of_nonneg (by linarith)] at hov; linarith
    have hVS : V ≤ S := near_le hNv hs (by linarith)
    rcases hcan with hn | hq
    · -- `s / 2` is a datum
      have hes' : f.qmin < es ∨ f.qmin = es := lt_or_eq_of_le hes
      rcases hes' with hlt | heq
      · have h2 : S / 2 = (ks : ℚ) * 2 ^ (es - 1) := by rw [hvs, zpow_sub_one₀ two_ne_zero]; ring
        have hr : Rep f (S / 2) := by
          rw [h2]; refine rep_grid hks.le (by omega) ?_
          rw [← h2, abs_of_nonneg (by linarith)]
          have := FP.abs_val_lt hs; rw [abs_of_nonneg hS0] at this; linarith
        have : S / 2 ≤ V := by
          have := near_ge hNv (hr false).1 (by rw [(hr false).2]; linarith)
          rwa [(hr false).2] at this
        exact rep_sub hs hv (by linarith) (by linarith)
      · subst heq
        -- the finest grid: `s − a` is a datum
        apply exact_v
        obtain ⟨pa, hpa⟩ := FP.two_zpow_dvd hga
        have hk : S - A = ((ks - ka * pa : ℤ) : ℚ) * 2 ^ f.qmin := by rw [hvs, hva, hpa]; push_cast; ring
        have hpos : (0 : ℚ) < 2 ^ f.qmin := two_zpow_pos _
        rw [hk]; refine rep_grid ?_ le_rfl ?_
        · have : |((ks - ka * pa : ℤ) : ℚ)| * 2 ^ f.qmin ≤ (2 ^ f.prec : ℚ) * 2 ^ f.qmin := by
            rw [← abs_of_pos hpos, ← abs_mul, abs_of_pos hpos, ← hk, abs_of_nonneg (by linarith)]
            calc S - A ≤ S := by linarith
              _ = |(ks : ℚ)| * 2 ^ f.qmin := by rw [← abs_of_nonneg hS0, hvs, abs_mul, abs_of_pos hpos]
              _ ≤ _ := mul_le_mul_of_nonneg_right (by exact_mod_cast hks.le) hpos.le
          exact_mod_cast le_of_mul_le_mul_right this hpos
        · rw [← hk, abs_of_nonneg (by linarith)]
          have := FP.abs_val_lt hs; rw [abs_of_nonneg hS0] at this; linarith
    · -- `es = qmin`
      apply exact_v
      obtain ⟨pa, hpa⟩ := FP.two_zpow_dvd hga
      have hk : S - A = ((ks - ka * pa : ℤ) : ℚ) * 2 ^ f.qmin := by rw [hvs, hq, hva, hpa]; push_cast; ring
      have hpos : (0 : ℚ) < 2 ^ f.qmin := two_zpow_pos _
      rw [hk]; refine rep_grid ?_ le_rfl ?_
      · have hSk : S = |(ks : ℚ)| * 2 ^ f.qmin := by
          rw [← abs_of_nonneg hS0, hvs, hq, abs_mul, abs_of_pos hpos]
        have : |((ks - ka * pa : ℤ) : ℚ)| * 2 ^ f.qmin ≤ (2 ^ f.prec : ℚ) * 2 ^ f.qmin := by
          rw [← abs_of_pos hpos, ← abs_mul, abs_of_pos hpos, ← hk, abs_of_nonneg (by linarith)]
          calc S - A ≤ S := by linarith
            _ = |(ks : ℚ)| * 2 ^ f.qmin := hSk
            _ ≤ _ := mul_le_mul_of_nonneg_right (by exact_mod_cast hks.le) hpos.le
        exact_mod_cast le_of_mul_le_mul_right this hpos
      · rw [← hk, abs_of_nonneg (by linarith)]
        have := FP.abs_val_lt hs; rw [abs_of_nonneg hS0] at this; linarith
  · -- opposite signs
    rcases le_or_gt (B / 2) (-A) with hB2 | hB2
    · -- Sterbenz: `a + b` is exact, so `s = a + b` and `s − a = b`
      have hr : Rep f (A + B) := by
        have := rep_sub hb (by simpa using ha : a.neg.isFinite) (by rw [FP.val_neg]; linarith)
          (by rw [FP.val_neg]; linarith)
        rwa [FP.val_neg, sub_neg_eq_add, _root_.add_comm] at this
      have hsr : S = A + B := by
        have := hNs _ (hr false).1
        rw [(hr false).2, _root_.sub_self, abs_zero] at this
        linarith [abs_nonneg (S - (A + B)), abs_nonpos_iff.mp this]
      apply exact_v
      rw [hsr, show A + B - A = B by ring]
      exact rep_val hb
    · -- `−a ≤ s`, so `s ≤ s − a ≤ 2s`
      have hSa : -A ≤ S := by
        have := near_ge hNs (by simpa using ha : a.neg.isFinite) (by rw [FP.val_neg]; linarith)
        rwa [FP.val_neg] at this
      have hSV : S ≤ V := near_ge hNv hs (by linarith)
      have hV2S : V ≤ 2 * S := by
        rcases two_x hs hS0 with ⟨y, hy, hyv⟩ | h
        · rw [← hyv]; exact near_le hNv hy (by rw [hyv]; linarith)
        · have := FP.abs_val_lt hv; linarith [le_abs_self V]
      exact rep_sub hs hv (by linarith) (by linarith)

lemma near_neg {S r : ℚ} (hN : ∀ x : FP f, x.isFinite → |S - r| ≤ |x.val - r|) :
    ∀ x : FP f, x.isFinite → |-S - -r| ≤ |x.val - -r| := by
  intro x hx
  have := hN x.neg (by simpa using hx)
  rw [FP.val_neg] at this
  rw [show -S - -r = -(S - r) by ring, abs_neg, show x.val - -r = -(-x.val - r) by ring, abs_neg]
  exact this

/-- **TwoSum is error-free.** For any finite `a`, `b` whose sum and `s ⊖ a` do not overflow
(precision at least 3; binary64 has 53), `s + e = a + b`. -/
theorem two_sum_exact {a b : FP f} (ha : a.isFinite) (hb : b.isFinite) (hs : (add a b).isFinite)
    (hbv : (sub (add a b) a).isFinite) (hp : 3 ≤ f.prec) :
    (twoSum a b).1 = add a b ∧ (twoSum a b).2.isFinite ∧
      (twoSum a b).1.val + (twoSum a b).2.val = a.val + b.val := by
  rcases le_total |b.val| |a.val| with h | h
  · exact two_sum_exact_of_le ha hb hs h
  rcases eq_or_lt_of_le h with heq | hlt
  · exact two_sum_exact_of_le ha hb hs heq.symm.le
  set s := add a b with hsdef
  set v := sub s a with hvdef
  have hsr : s = round f (a.sign && b.sign) (a.val + b.val) := add_eq_round ha hb
  have hvr : v = round f (s.sign && !a.sign) (s.val - a.val) := sub_eq_round hs ha
  have hNs : ∀ x : FP f, x.isFinite → |s.val - (a.val + b.val)| ≤ |x.val - (a.val + b.val)| := by
    intro x hx; rw [hsr]; exact round_nearest (by rw [← hsr]; exact hs) hx
  have hNv : ∀ x : FP f, x.isFinite → |v.val - (s.val - a.val)| ≤ |x.val - (s.val - a.val)| := by
    intro x hx; rw [hvr]; exact round_nearest (by rw [← hvr]; exact hbv) hx
  have hrel := add_err_res ha hb hs
  have hov : |a.val + b.val| < 2 ^ (f.emax + 1) := abs_lt_of_round_finite (by rw [← hsr]; exact hs)
  obtain ⟨P1, P2⟩ : Rep f (s.val - v.val) ∧ Rep f (b.val - v.val) := by
    rcases lt_or_gt_of_ne (show b.val ≠ 0 by intro e; rw [e, abs_zero] at hlt; linarith [abs_nonneg a.val]) with hb0 | hb0
    · have := two_sum_core (a := a.neg) (b := b.neg) (s := s.neg) (v := v.neg) (by simpa using ha) (by simpa using hb)
        (by simpa using hs) (by simpa using hbv) hp
        (by have := near_neg hNs; simp only [FP.val_neg]; rwa [show -a.val + -b.val = -(a.val + b.val) by ring])
        (by have := near_neg hNv; simp only [FP.val_neg]; rwa [show -s.val - -a.val = -(s.val - a.val) by ring])
        (by simp only [FP.val_neg]; rw [show -s.val - (-a.val + -b.val) = -(s.val - (a.val + b.val)) by ring,
              abs_neg, abs_neg]; exact hrel)
        (by simp only [FP.val_neg]; rw [show -a.val + -b.val = -(a.val + b.val) by ring, abs_neg]; exact hov)
        (by simp only [FP.val_neg]; linarith)
        (by simp only [FP.val_neg, abs_neg]; rwa [abs_of_neg hb0] at hlt)
      simp only [FP.val_neg] at this
      obtain ⟨h1, h2⟩ := this
      have e1 := rep_neg h1; have e2 := rep_neg h2
      rw [show -(-s.val - -v.val) = s.val - v.val by ring] at e1
      rw [show -(-b.val - -v.val) = b.val - v.val by ring] at e2
      exact ⟨e1, e2⟩
    · exact two_sum_core ha hb hs hbv hp hNs hNv hrel hov hb0 (by rwa [abs_of_pos hb0] at hlt)
  -- the remaining steps, each exact
  obtain ⟨hav, hav'⟩ := sub_of_rep hs hbv P1
  obtain ⟨hbr, hbr'⟩ := sub_of_rep hb hbv P2
  have hη : Rep f (a.val - (sub s v).val) := by
    have := add_err_representable hs (by simpa using ha : a.neg.isFinite) (by have : v = add s a.neg := rfl; rw [← this]; exact hbv)
    have hr : Rep f (s.val + a.neg.val - (add s a.neg).val) := fun zs => this zs
    have := rep_neg hr
    rw [hav']
    rw [show (add s a.neg) = v by rfl, FP.val_neg] at this
    rwa [show -(s.val + -a.val - v.val) = a.val - (s.val - v.val) by ring] at this
  obtain ⟨har, har'⟩ := sub_of_rep ha hav hη
  obtain ⟨he, he'⟩ := add_of_rep har hbr (by
    rw [har', hbr', hav', show a.val - (s.val - v.val) + (b.val - v.val) = a.val + b.val - s.val by ring]
    exact fun zs => add_err_representable ha hb hs zs)
  refine ⟨rfl, he, ?_⟩
  show s.val + (add (sub a (sub s v)) (sub b v)).val = _
  rw [he', har', hbr', hav']; ring

end IEEE754
