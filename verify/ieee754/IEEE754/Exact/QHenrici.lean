/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import Mathlib

/-!
# dol-curved-rust's `Q` (exact/alg.rs, 5aca447): Henrici's normal form, over ℤ

`Q { n, d }` with `Big` fields. Its operations use `Big`'s `add`, `sub`, `mul`, `neg`, `signum`,
`divrem` (truncated) and `gcd` (of the magnitudes, nonnegative). Those are the integer operations
(`Exact.Big`: `add_spec` and so on; `divrem` and `gcd` once proven). So `Q` is modelled here on `ℤ`
fields, with `divrem`'s quotient as `Int.tdiv` and `gcd` as `Int.gcd`.

`Normal q` means `d > 0`, `gcd(n, d) = 1`, and `0 = 0/1`. Under it:
- `ratio_spec`: `Q::ratio` normalises `n/d`, with `x/0 = 0`.
- `add_spec`: Henrici's sum is normal and the exact sum.
- `mul_spec`: Henrici's product is normal and the exact product.
- `neg_spec`.
- `normal_unique`: normal forms of equal value are equal. So `Q`'s derived `Eq`/`Hash` is rational
  equality, through `Big.eq_iff_val`.
-/

namespace IEEE754.Exact.QZ

/-- A `Q` on integer fields. -/
structure QZ where
  n : ℤ
  d : ℤ

/-- Its value. -/
def val (q : QZ) : ℚ := (q.n : ℚ) / q.d

/-- The normal form. -/
def Normal (q : QZ) : Prop := 0 < q.d ∧ Int.gcd q.n q.d = 1

/-- `gcd` as `Big::gcd` gives it: of the magnitudes, nonnegative. -/
def g (a b : ℤ) : ℤ := (Int.gcd a b : ℤ)

/-- `Q::zero`. -/
def zero : QZ := ⟨0, 1⟩

/-- `Q::ratio(n, d)`. -/
def ratio (n d : ℤ) : QZ :=
  if d = 0 ∨ n = 0 then zero else
    let n1 := if d < 0 then -n else n
    let d1 := if d < 0 then -d else d
    if d1 = 1 then ⟨n1, d1⟩ else
      let gg := g n1 d1
      if gg = 1 then ⟨n1, d1⟩ else
        ⟨if n1 < 0 then -(Int.tdiv |n1| gg) else Int.tdiv n1 gg, Int.tdiv d1 gg⟩

/-- `Q::add`, Henrici's sum. -/
def add (x y : QZ) : QZ :=
  if x.n = 0 then y else if y.n = 0 then x else
    let gg := g x.d y.d
    if gg = 1 then ⟨x.n * y.d + y.n * x.d, x.d * y.d⟩ else
      let d1g := Int.tdiv x.d gg
      let d2g := Int.tdiv y.d gg
      let t := x.n * d2g + y.n * d1g
      if t = 0 then zero else
        let h := g t gg
        if h = 1 then ⟨t, d1g * y.d⟩ else ⟨Int.tdiv t h, d1g * Int.tdiv y.d h⟩

/-- `Q::mul`, Henrici's product. -/
def mul (x y : QZ) : QZ :=
  if x.n = 0 ∨ y.n = 0 then zero else
    let g1 := g x.n y.d
    let g2 := g y.n x.d
    let p1 : ℤ × ℤ := if g1 = 1 then (x.n, y.d) else (Int.tdiv x.n g1, Int.tdiv y.d g1)
    let p2 : ℤ × ℤ := if g2 = 1 then (y.n, x.d) else (Int.tdiv y.n g2, Int.tdiv x.d g2)
    ⟨p1.1 * p2.1, p2.2 * p1.2⟩

/-- `Q::neg`. -/
def neg (x : QZ) : QZ := ⟨-x.n, x.d⟩

/-! ## Helpers -/

lemma g_nonneg (a b : ℤ) : 0 ≤ g a b := Int.natCast_nonneg _

lemma g_dvd_left (a b : ℤ) : g a b ∣ a := Int.gcd_dvd_left a b
lemma g_dvd_right (a b : ℤ) : g a b ∣ b := Int.gcd_dvd_right a b

lemma tdiv_exact {a b : ℤ} (h : b ∣ a) : Int.tdiv a b * b = a := Int.tdiv_mul_cancel h

lemma coprime_iff {a b : ℤ} : IsCoprime a b ↔ Int.gcd a b = 1 := Int.isCoprime_iff_gcd_eq_one

lemma normal_zero : Normal zero := ⟨by norm_num [zero], by simp [zero]⟩

lemma val_zero : val zero = 0 := by simp [val, zero]

/-- An exact quotient. -/
lemma tdiv_of_dvd {a k : ℤ} (h : k ∣ a) (hk : k ≠ 0) : ∃ c, a = c * k ∧ Int.tdiv a k = c := by
  obtain ⟨c, rfl⟩ := h
  exact ⟨c, by ring, by rw [mul_comm, Int.mul_tdiv_cancel _ hk]⟩

/-- Dividing both by their gcd leaves them coprime. -/
lemma coprime_div_g {a b : ℤ} (h : 0 < Int.gcd a b) :
    IsCoprime (Int.tdiv a (g a b)) (Int.tdiv b (g a b)) := by
  rw [Int.tdiv_eq_ediv_of_dvd (g_dvd_left a b), Int.tdiv_eq_ediv_of_dvd (g_dvd_right a b)]
  exact coprime_iff.mpr (Int.gcd_div_gcd_div_gcd h)

lemma g_pos_of_right {a b : ℤ} (hb : b ≠ 0) : 0 < g a b := by
  unfold g; exact_mod_cast Int.gcd_pos_of_ne_zero_right a hb

lemma tdiv_one' (a : ℤ) : Int.tdiv a 1 = a := Int.tdiv_one a

/-- **`Q::ratio`.** -/
theorem ratio_spec (n d : ℤ) : Normal (ratio n d) ∧ val (ratio n d) = (n : ℚ) / d := by
  unfold ratio
  by_cases hz : d = 0 ∨ n = 0
  · rw [if_pos hz]; refine ⟨normal_zero, ?_⟩
    rcases hz with h | h <;> simp [val_zero, h]
  · rw [if_neg hz]
    push Not at hz
    obtain ⟨hd, hn⟩ := hz
    simp only
    set n1 := if d < 0 then -n else n with hn1
    set d1 := if d < 0 then -d else d with hd1
    have hd1p : 0 < d1 := by simp only [hd1]; split_ifs <;> omega
    have hval : (n1 : ℚ) / d1 = (n : ℚ) / d := by
      simp only [hn1, hd1]; split_ifs
      · push_cast; rw [neg_div_neg_eq]
      · rfl
    have hn10 : n1 ≠ 0 := by simp only [hn1]; split_ifs <;> omega
    by_cases h1 : d1 = 1
    · rw [if_pos h1]; refine ⟨⟨hd1p, by rw [h1]; simp⟩, ?_⟩; simp only [val]; exact hval
    · rw [if_neg h1]
      have hgp : 0 < Int.gcd n1 d1 := Int.gcd_pos_of_ne_zero_right n1 hd1p.ne'
      by_cases h2 : g n1 d1 = 1
      · rw [if_pos h2]; refine ⟨⟨hd1p, by unfold g at h2; exact_mod_cast h2⟩, hval⟩
      · rw [if_neg h2]
        have hg0 : g n1 d1 ≠ 0 := by unfold g; exact_mod_cast hgp.ne'
        have cop := coprime_div_g hgp
        have hgpos : 0 < g n1 d1 := by unfold g; exact_mod_cast hgp
        generalize hG : g n1 d1 = G at hg0 cop hgpos ⊢
        obtain ⟨cn, ecn, tcn⟩ := tdiv_of_dvd (hG ▸ g_dvd_left n1 d1) hg0
        obtain ⟨cd, ecd, tcd⟩ := tdiv_of_dvd (hG ▸ g_dvd_right n1 d1) hg0
        have hnum : (if n1 < 0 then -(Int.tdiv |n1| G) else Int.tdiv n1 G) = cn := by
          split_ifs with hneg
          · rw [abs_of_neg hneg, Int.neg_tdiv, tcn, neg_neg]
          · exact tcn
        rw [hnum, tcd]
        rw [tcn, tcd] at cop
        refine ⟨⟨?_, coprime_iff.mp cop⟩, ?_⟩
        · by_contra hc; push Not at hc; nlinarith
        · simp only [val]; rw [← hval, ecn, ecd]
          have : (G : ℚ) ≠ 0 := by exact_mod_cast hg0
          push_cast; field_simp

/-- **`Q::mul`**, Henrici's product. -/
theorem mul_spec {x y : QZ} (hx : Normal x) (hy : Normal y) :
    Normal (mul x y) ∧ val (mul x y) = val x * val y := by
  obtain ⟨d1p, c1⟩ := hx
  obtain ⟨d2p, c2⟩ := hy
  unfold mul
  by_cases hz : x.n = 0 ∨ y.n = 0
  · rw [if_pos hz]; refine ⟨normal_zero, ?_⟩
    rw [val_zero]; rcases hz with h | h <;> simp [val, h]
  · rw [if_neg hz]
    push Not at hz
    simp only
    have g1p := g_pos_of_right (a := x.n) d2p.ne'
    have g2p := g_pos_of_right (a := y.n) d1p.ne'
    -- both branches are the exact quotients (by 1 when the gcd is 1)
    have e1 : (if g x.n y.d = 1 then (x.n, y.d) else (Int.tdiv x.n (g x.n y.d), Int.tdiv y.d (g x.n y.d))) =
        (Int.tdiv x.n (g x.n y.d), Int.tdiv y.d (g x.n y.d)) := by
      split_ifs with h
      · simp [h]
      · rfl
    have e2 : (if g y.n x.d = 1 then (y.n, x.d) else (Int.tdiv y.n (g y.n x.d), Int.tdiv x.d (g y.n x.d))) =
        (Int.tdiv y.n (g y.n x.d), Int.tdiv x.d (g y.n x.d)) := by
      split_ifs with h
      · simp [h]
      · rfl
    rw [e1, e2]
    simp only
    have cAB := coprime_div_g (a := x.n) (b := y.d) (by unfold g at g1p; exact_mod_cast g1p)
    have cCD := coprime_div_g (a := y.n) (b := x.d) (by unfold g at g2p; exact_mod_cast g2p)
    have dA := g_dvd_left x.n y.d; have dB := g_dvd_right x.n y.d
    have dC := g_dvd_left y.n x.d; have dD := g_dvd_right y.n x.d
    generalize g x.n y.d = G1 at g1p cAB dA dB ⊢
    generalize g y.n x.d = G2 at g2p cCD dC dD ⊢
    obtain ⟨A, eA, tA⟩ := tdiv_of_dvd dA g1p.ne'
    obtain ⟨Bb, eB, tB⟩ := tdiv_of_dvd dB g1p.ne'
    obtain ⟨C, eC, tC⟩ := tdiv_of_dvd dC g2p.ne'
    obtain ⟨D, eD, tD⟩ := tdiv_of_dvd dD g2p.ne'
    rw [tA, tB] at cAB; rw [tC, tD] at cCD
    rw [tA, tB, tC, tD]
    have cx : IsCoprime x.n x.d := coprime_iff.mpr c1
    have cy : IsCoprime y.n y.d := coprime_iff.mpr c2
    have cAD : IsCoprime A D :=
      (cx.of_isCoprime_of_dvd_left ⟨G1, by rw [eA]⟩).of_isCoprime_of_dvd_right ⟨G2, by rw [eD]⟩
    have cCB : IsCoprime C Bb :=
      (cy.of_isCoprime_of_dvd_left ⟨G2, by rw [eC]⟩).of_isCoprime_of_dvd_right ⟨G1, by rw [eB]⟩
    have Dp : 0 < D := by by_contra hc; push Not at hc; nlinarith
    have Bp : 0 < Bb := by by_contra hc; push Not at hc; nlinarith
    refine ⟨⟨mul_pos Dp Bp, coprime_iff.mp ?_⟩, ?_⟩
    · exact IsCoprime.mul_left (IsCoprime.mul_right cAD cAB) (IsCoprime.mul_right cCD cCB)
    · simp only [val]
      rw [eA, eB, eC, eD]
      have : ((G1 : ℤ) : ℚ) ≠ 0 := by exact_mod_cast g1p.ne'
      have : ((G2 : ℤ) : ℚ) ≠ 0 := by exact_mod_cast g2p.ne'
      have : (D : ℚ) ≠ 0 := by exact_mod_cast Dp.ne'
      have : (Bb : ℚ) ≠ 0 := by exact_mod_cast Bp.ne'
      push_cast; field_simp

/-- **Normal forms are unique.** -/
theorem normal_unique {x y : QZ} (hx : Normal x) (hy : Normal y) (h : val x = val y) : x = y := by
  obtain ⟨d1p, c1⟩ := hx
  obtain ⟨d2p, c2⟩ := hy
  have cross : x.n * y.d = y.n * x.d := by
    simp only [val] at h
    have : (x.d : ℚ) ≠ 0 := by exact_mod_cast d1p.ne'
    have : (y.d : ℚ) ≠ 0 := by exact_mod_cast d2p.ne'
    rw [div_eq_div_iff (by assumption) (by assumption)] at h
    exact_mod_cast h
  have cx : IsCoprime x.n x.d := coprime_iff.mpr c1
  have cy : IsCoprime y.n y.d := coprime_iff.mpr c2
  have h1 : x.d ∣ y.d := cx.symm.dvd_of_dvd_mul_left ⟨y.n, by rw [mul_comm]; linarith [cross]⟩
  have h2 : y.d ∣ x.d := cy.symm.dvd_of_dvd_mul_left ⟨x.n, by linarith [cross]⟩
  have hd : x.d = y.d := Int.dvd_antisymm d1p.le d2p.le h1 h2
  have hn : x.n = y.n := by
    rw [hd] at cross; exact mul_right_cancel₀ d2p.ne' cross
  cases x; cases y; simp_all

/-- **`Q::neg`.** -/
theorem neg_spec {x : QZ} (hx : Normal x) : Normal (neg x) ∧ val (neg x) = -val x :=
  ⟨⟨hx.1, by simp only [neg]; rw [Int.neg_gcd]; exact hx.2⟩, by simp [neg, val, neg_div]⟩

/-- The coprimality at the heart of Henrici's sum: with `x = n₁/(a G)`, `y = n₂/(b G)` reduced and
`a`, `b` coprime, `t = n₁ b + n₂ a` is coprime to `a` and to `b`. -/
lemma henrici_cop {n1 n2 a b G : ℤ} (c1 : IsCoprime n1 (a * G)) (c2 : IsCoprime n2 (b * G))
    (cab : IsCoprime a b) : IsCoprime (n1 * b + n2 * a) a ∧ IsCoprime (n1 * b + n2 * a) b := by
  have c1a : IsCoprime n1 a := c1.of_isCoprime_of_dvd_right ⟨G, rfl⟩
  have c2b : IsCoprime n2 b := c2.of_isCoprime_of_dvd_right ⟨G, rfl⟩
  constructor
  · have : IsCoprime (n1 * b) a := IsCoprime.mul_left c1a cab.symm
    have := this.add_mul_left_left n2
    rwa [mul_comm a n2] at this
  · have : IsCoprime (n2 * a) b := IsCoprime.mul_left c2b cab
    have := this.add_mul_left_left n1
    rwa [mul_comm b n1, add_comm] at this

set_option maxHeartbeats 1000000 in
/-- **`Q::add`**, Henrici's sum. -/
theorem add_spec {x y : QZ} (hx : Normal x) (hy : Normal y) :
    Normal (add x y) ∧ val (add x y) = val x + val y := by
  obtain ⟨d1p, c1⟩ := hx
  obtain ⟨d2p, c2⟩ := hy
  have cx : IsCoprime x.n x.d := coprime_iff.mpr c1
  have cy : IsCoprime y.n y.d := coprime_iff.mpr c2
  unfold add
  by_cases hx0 : x.n = 0
  · rw [if_pos hx0]; exact ⟨⟨d2p, c2⟩, by simp [val, hx0]⟩
  rw [if_neg hx0]
  by_cases hy0 : y.n = 0
  · rw [if_pos hy0]; exact ⟨⟨d1p, c1⟩, by simp [val, hy0]⟩
  rw [if_neg hy0]
  simp only
  have ggp := g_pos_of_right (a := x.d) d2p.ne'
  have cab0 := coprime_div_g (a := x.d) (b := y.d) (by unfold g at ggp; exact_mod_cast ggp)
  have dA := g_dvd_left x.d y.d; have dB := g_dvd_right x.d y.d
  generalize hG : g x.d y.d = G at ggp cab0 dA dB ⊢
  obtain ⟨a, ea, ta⟩ := tdiv_of_dvd dA ggp.ne'
  obtain ⟨b, eb, tb⟩ := tdiv_of_dvd dB ggp.ne'
  rw [ta, tb] at cab0 ⊢
  have ap : 0 < a := by by_contra hc; push Not at hc; nlinarith
  have bp : 0 < b := by by_contra hc; push Not at hc; nlinarith
  have hGq : (G : ℚ) ≠ 0 := by exact_mod_cast ggp.ne'
  have haq : (a : ℚ) ≠ 0 := by exact_mod_cast ap.ne'
  have hbq : (b : ℚ) ≠ 0 := by exact_mod_cast bp.ne'
  have c1' : IsCoprime x.n (a * G) := by rw [← ea]; exact cx
  have c2' : IsCoprime y.n (b * G) := by rw [← eb]; exact cy
  obtain ⟨cta, ctb⟩ := henrici_cop c1' c2' cab0
  -- the sum's value is `t / (a b G)`, `t = n₁ b + n₂ a`
  have hsum : val x + val y = ((x.n * b + y.n * a : ℤ) : ℚ) / ((a * b * G : ℤ) : ℚ) := by
    simp only [val]; rw [ea, eb]; push_cast; field_simp
  by_cases hg1 : G = 1
  · rw [if_pos hg1]
    subst hg1
    have ex : x.d = a := by rw [ea, mul_one]
    have ey : y.d = b := by rw [eb, mul_one]
    refine ⟨⟨mul_pos d1p d2p, coprime_iff.mp ?_⟩, ?_⟩
    · rw [ex, ey, show x.n * b + y.n * a = x.n * b + y.n * a from rfl]
      exact IsCoprime.mul_right cta ctb
    · rw [hsum]; simp only [val]; rw [ex, ey]; push_cast; ring
  · rw [if_neg hg1]
    by_cases ht : x.n * b + y.n * a = 0
    · rw [if_pos ht]; refine ⟨normal_zero, ?_⟩
      rw [val_zero, hsum, ht]; simp
    · rw [if_neg ht]
      set t := x.n * b + y.n * a with htd
      have hhp := g_pos_of_right (a := t) ggp.ne'
      have cTG := coprime_div_g (a := t) (b := G) (by unfold g at hhp; exact_mod_cast hhp)
      have dT := g_dvd_left t G; have dH := g_dvd_right t G
      generalize hH : g t G = h at hhp cTG dT dH ⊢
      obtain ⟨T, eT, tT⟩ := tdiv_of_dvd dT hhp.ne'
      obtain ⟨G', eG', tG'⟩ := tdiv_of_dvd dH hhp.ne'
      rw [tT, tG'] at cTG
      have Gp : 0 < G' := by by_contra hc; push Not at hc; nlinarith
      -- both branches: `(t/h) / (a (d₂/h))`, and `d₂/h = b G'`
      have hd2 : Int.tdiv y.d h = b * G' := by
        rw [eb, eG', show b * (G' * h) = (b * G') * h by ring, Int.mul_tdiv_cancel _ hhp.ne']
      have e3 : (if h = 1 then (⟨t, a * y.d⟩ : QZ) else ⟨Int.tdiv t h, a * Int.tdiv y.d h⟩) =
          ⟨T, a * (b * G')⟩ := by
        split_ifs with h1
        · subst h1; rw [← hd2, tdiv_one', ← tT, tdiv_one']
        · rw [tT, hd2]
      rw [e3]
      have cTa : IsCoprime T a := cta.of_isCoprime_of_dvd_left ⟨h, by rw [eT]⟩
      have cTb : IsCoprime T b := ctb.of_isCoprime_of_dvd_left ⟨h, by rw [eT]⟩
      refine ⟨⟨by positivity, coprime_iff.mp ?_⟩, ?_⟩
      · exact IsCoprime.mul_right cTa (IsCoprime.mul_right cTb cTG)
      · rw [hsum]; simp only [val]
        have hhq : (h : ℚ) ≠ 0 := by exact_mod_cast hhp.ne'
        have : (t : ℚ) = T * h := by exact_mod_cast eT
        have : (G : ℚ) = G' * h := by exact_mod_cast eG'
        push_cast
        rw [‹(t : ℚ) = T * h›, ‹(G : ℚ) = G' * h›]
        have : (G' : ℚ) ≠ 0 := by exact_mod_cast Gp.ne'
        field_simp

end IEEE754.Exact.QZ
