import ForgeSheetCore.RefineExact
import AeneasF64.Err

/-!
# The expansions' arithmetic on the models: exact, finite, on the grid

On the domain where every input is a multiple of `2⁻²⁶⁸` (`Mult x (−268)`: any double of
magnitude at least `2⁻²¹⁶`, or zero) and at most `2¹²⁰` in magnitude, every expansion component
the computation forms is finite, on the `2⁻¹⁰⁷⁴` grid, and the two-sums and two-products are
exact (`twoSum_good`, `twoProduct_good`, then `growL_good`, `addL_good`, `mulL_good`). The
expansions' values are therefore the exact polynomial values (Exact.lean's terms). On the grid,
every rounding is relatively accurate, subnormal results included (`rel_mult`), which is what
Filter16.lean's filters assume.
-/

namespace forge_sheet_core

open IEEE754 hiding F64


/-- Rounding a value on a grid at least the least subnormal's keeps it on that grid. -/
theorem round_mult {zs : Bool} {r : ℚ} {L : ℤ} (hm : Mult r L) (hL : (-1074 : ℤ) ≤ L)
    (hfin : (IEEE754.round IEEE754.binary64 zs r).isFinite) : Mult (IEEE754.round IEEE754.binary64 zs r).val L := by
  by_cases hr : r = 0
  · subst hr; simp [IEEE754.round, FP.val]; exact mult_zero L
  by_cases hb : (2 : ℚ) ^ (L + (53 : ℤ) - 1) ≤ |r|
  · have hq := qOf_ge (f := IEEE754.binary64) hb
    simp only [binary64_prec] at hq
    exact mult_of_le (round_grid (f := IEEE754.binary64) hr hfin).1 (by omega)
  · push Not at hb
    have hrep := rep_of_mult (f := IEEE754.binary64) hm (by simpa using hL) (by
      have : (2 : ℚ) ^ (L + (53 : ℤ) - 1) ≤ 2 ^ (53 : ℤ) * 2 ^ L := by
        rw [← two_zpow_add]; exact two_zpow_le (by omega)
      simp only [binary64_prec]; push_cast; linarith) (abs_lt_of_round_finite hfin)
    rw [(hrep zs).2]; exact hm

/-- **On the `2⁻¹⁰⁷⁴` grid, every finite rounding is relatively accurate**: normal results by
`round_rel_err_q`, subnormal ones exactly (they are representable). -/
theorem rel_mult {zs : Bool} {r : ℚ} (hm : Mult r (-1074))
    (hfin : (IEEE754.round IEEE754.binary64 zs r).isFinite) :
    |(IEEE754.round IEEE754.binary64 zs r).val - r| ≤ 2 ^ (-53 : ℤ) * |r| := by
  by_cases hn : (2 : ℚ) ^ (-1022 : ℤ) ≤ |r|
  · have := round_rel_err_q hfin (by simpa using hn); simpa using this
  · push Not at hn
    have hrep := rep_of_mult (f := IEEE754.binary64) hm (by simp) (by
      have : (2 : ℚ) ^ (-1022 : ℤ) ≤ 2 ^ (53 : ℤ) * 2 ^ (-1074 : ℤ) := by
        rw [← two_zpow_add]; exact two_zpow_le (by norm_num)
      simp only [binary64_prec, binary64_qmin]; push_cast; linarith) (abs_lt_of_round_finite hfin)
    rw [(hrep zs).2, sub_self, abs_zero]; positivity

/-- `u = 2⁻⁵³`, `η = 2⁻¹⁰⁷⁵` over ℚ. -/
irreducible_def uq : ℚ := 2 ^ (-53 : ℤ)
irreducible_def ηq : ℚ := 2 ^ (-1075 : ℤ)

theorem uq_pos : 0 < uq := by rw [uq_def]; positivity
theorem ηq_pos : 0 < ηq := by rw [ηq_def]; positivity
theorem uq_le1 : uq ≤ 1 := by rw [uq_def]; exact zpow_le_one_of_nonpos₀ (by norm_num) (by norm_num)
theorem ηq_le1 : ηq ≤ 1 := by rw [ηq_def]; exact zpow_le_one_of_nonpos₀ (by norm_num) (by norm_num)

theorem fin_of_le {zs : Bool} {r : ℚ} (h : |r| ≤ 2 ^ (1023 : ℤ)) :
    (IEEE754.round IEEE754.binary64 zs r).isFinite :=
  round_finite_of_le_emax (by simpa using h)

/-- The magnitude budget, `2¹⁰²⁰`, kept opaque so arithmetic tactics treat it as an atom. -/
irreducible_def Kmax : ℚ := 2 ^ (1020 : ℤ)

theorem Kmax_ge : 1 ≤ Kmax := by rw [Kmax_def]; exact one_le_zpow₀ (by norm_num) (by norm_num)

theorem fin_of_le8 {zs : Bool} {r : ℚ} (h : |r| ≤ 8 * Kmax) :
    (IEEE754.round IEEE754.binary64 zs r).isFinite := by
  apply fin_of_le
  have : (2 : ℚ) ^ (1023 : ℤ) = 8 * 2 ^ (1020 : ℤ) := by
    rw [show (1023 : ℤ) = 3 + 1020 by norm_num, two_zpow_add]; norm_num
  rw [this, ← Kmax_def]; exact h

theorem err_le {zs : Bool} {r : ℚ} (h : (IEEE754.round IEEE754.binary64 zs r).isFinite) :
    |(IEEE754.round IEEE754.binary64 zs r).val - r| ≤ uq * |r| + ηq := by
  rw [uq_def, ηq_def]; exact F64.Ops.round_err' h

/-- **Two-sum on bounded doubles**: exact, finite, its two parts' magnitudes within
`(1 + 2u)(|a| + |b|) + 2η`, and on any grid `a` and `b` share. -/
theorem twoSum_good {a b : _root_.F64} (ha : a.isFinite) (hb : b.isFinite)
    (hm : |a.val| + |b.val| ≤ Kmax) :
    (twoSum a b).1.isFinite ∧ (twoSum a b).2.isFinite ∧
      (twoSum a b).1.val + (twoSum a b).2.val = a.val + b.val ∧
      |(twoSum a b).1.val| + |(twoSum a b).2.val| ≤ (1 + 2 * uq) * (|a.val| + |b.val|) + 2 * ηq ∧
      (∀ L : ℤ, -1074 ≤ L → Mult a.val L → Mult b.val L →
        Mult (twoSum a b).1.val L ∧ Mult (twoSum a b).2.val L) := by
  have hab : |a.val + b.val| ≤ |a.val| + |b.val| := abs_add_le _ _
  have h1 := Kmax_ge
  have hu : uq ≤ 1 := uq_le1
  have hη : ηq ≤ 1 := ηq_le1
  have hu0 : 0 ≤ uq := uq_pos.le
  have hs : (FP.add a b).isFinite := by
    rw [add_eq_round ha hb]; exact fin_of_le8 (by linarith only [hab, hm, h1])
  have es : |(FP.add a b).val - (a.val + b.val)| ≤ uq * |a.val + b.val| + ηq := by
    have := err_le (zs := a.sign && b.sign) (r := a.val + b.val) (by rwa [← add_eq_round ha hb])
    rwa [← add_eq_round ha hb] at this
  have sb : |(FP.add a b).val| ≤ 3 * Kmax := by
    have := abs_sub_abs_le_abs_sub (FP.add a b).val (a.val + b.val)
    have : uq * |a.val + b.val| ≤ |a.val + b.val| := by
      have := abs_nonneg (a.val + b.val); nlinarith
    linarith only [this, ‹|(FP.add a b).val| - |a.val + b.val| ≤ _›, es, hab, hm, h1, hη]
  have hbv : (FP.sub (FP.add a b) a).isFinite := by
    rw [sub_eq_round hs ha]; apply fin_of_le8
    have e1 : |(FP.add a b).val - a.val| ≤ |(FP.add a b).val| + |a.val| := abs_sub _ _
    have e2 := abs_nonneg b.val
    linarith only [e1, e2, sb, hm, h1]
  obtain ⟨e1, f2, esum⟩ := two_sum_exact ha hb hs hbv (by simp)
  refine ⟨by rw [e1]; exact hs, f2, esum, ?_, ?_⟩
  · rw [e1] at esum ⊢
    have e2 : (twoSum a b).2.val = (a.val + b.val) - (FP.add a b).val := by linarith
    rw [e2, abs_sub_comm]
    have := abs_sub_abs_le_abs_sub (FP.add a b).val (a.val + b.val)
    have : uq * |a.val + b.val| ≤ uq * (|a.val| + |b.val|) :=
      mul_le_mul_of_nonneg_left hab (by positivity)
    nlinarith [abs_nonneg (a.val + b.val)]
  · intro L hL hma hmb
    have m1 : Mult (FP.add a b).val L := by
      rw [add_eq_round ha hb] at hs ⊢
      exact round_mult (mult_add hma hmb) (by simpa using hL) hs
    rw [e1]
    refine ⟨m1, ?_⟩
    have e2 : (twoSum a b).2.val = (a.val + b.val) - (FP.add a b).val := by rw [e1] at esum; linarith
    rw [e2]; exact mult_sub (mult_add hma hmb) m1

/-! ## Grow-Expansion: exact, finite, on the grid, its mass bounded -/

def esumL (l : List _root_.F64) : ℚ := (l.map fun x => x.val).sum
def massL (l : List _root_.F64) : ℚ := (l.map fun x => |x.val|).sum

/-- Finite and on the `2^L` grid. -/
def Good (L : ℤ) (x : _root_.F64) : Prop := x.isFinite ∧ Mult x.val L

theorem massL_nonneg (l : List _root_.F64) : 0 ≤ massL l := by
  unfold massL; apply List.sum_nonneg; intro x hx; simp at hx; obtain ⟨y, _, rfl⟩ := hx; positivity

@[simp] theorem esumL_append (a b : List _root_.F64) : esumL (a ++ b) = esumL a + esumL b := by
  simp [esumL]
@[simp] theorem massL_append (a b : List _root_.F64) : massL (a ++ b) = massL a + massL b := by
  simp [massL]
@[simp] theorem esumL_cons (x : _root_.F64) (l : List _root_.F64) : esumL (x :: l) = x.val + esumL l := by
  simp [esumL]
@[simp] theorem massL_cons (x : _root_.F64) (l : List _root_.F64) : massL (x :: l) = |x.val| + massL l := by
  simp [massL]
@[simp] theorem esumL_nil : esumL [] = 0 := rfl
@[simp] theorem massL_nil : massL [] = 0 := rfl

theorem eq_zero_val {x : _root_.F64} (hx : x.isFinite) (h : FP.eq x Z = true) : x.val = 0 := by
  have := (FP.eq_iff hx (by decide : (Z : _root_.F64).isFinite)).mp h
  rw [this]; decide

/-- The numerical step of the mass bound. -/
theorem mass_step {T u η : ℚ} {k : ℕ} (hT : 0 ≤ T) (hu0 : 0 ≤ u) (hη : 0 ≤ η)
    (hk : (k : ℚ) * (8 * u) ≤ 1) :
    ((1 + 2 * u) * T + 2 * η) * (1 + 4 * k * u) + 3 * k * η ≤ T * (1 + 4 * (k + 1) * u) + 3 * (k + 1) * η := by
  have hk0 : (0 : ℚ) ≤ k := Nat.cast_nonneg k
  have a : (1 + 2 * u) * (1 + 4 * k * u) ≤ 1 + 4 * (k + 1) * u := by nlinarith
  have b : 2 * η * (1 + 4 * k * u) ≤ 3 * η := by nlinarith
  nlinarith [mul_le_mul_of_nonneg_left a hT]

theorem fold_grow_good (L : ℤ) (hL : -1074 ≤ L) :
    ∀ (rest : List _root_.F64) (q : _root_.F64) (out : List _root_.F64),
      Good L q → (∀ x ∈ out, Good L x) → (∀ x ∈ rest, Good L x) →
      (rest.length : ℚ) * (8 * uq) ≤ 1 →
      (|q.val| + massL out + massL rest) * (1 + 4 * rest.length * uq) + 3 * rest.length * ηq ≤
        Kmax →
      let r := rest.foldl growStep (q, out)
      Good L r.1 ∧ (∀ x ∈ r.2, Good L x) ∧
        r.1.val + esumL r.2 = q.val + esumL out + esumL rest ∧
        |r.1.val| + massL r.2 ≤
          (|q.val| + massL out + massL rest) * (1 + 4 * rest.length * uq) + 3 * rest.length * ηq
  | [], q, out, hq, ho, _, _, _ => by simp; exact ⟨hq, ho⟩
  | x :: xs, q, out, hq, ho, hr, hk, hB => by
      have hx := hr x (by simp)
      have hxs : ∀ y ∈ xs, Good L y := fun y h => hr y (by simp [h])
      simp only [List.length_cons, Nat.cast_add, Nat.cast_one, massL_cons, esumL_cons] at hk hB ⊢
      have hu0 : (0 : ℚ) ≤ uq := uq_pos.le
      have hη0 : (0 : ℚ) ≤ ηq := ηq_pos.le
      have hm0 := massL_nonneg out
      have hm1 := massL_nonneg xs
      have hT0 : 0 ≤ |q.val| + massL out + (|x.val| + massL xs) := by positivity
      have hk' : (xs.length : ℚ) * (8 * uq) ≤ 1 := by nlinarith
      have hge : |q.val| + massL out + (|x.val| + massL xs) ≤
          (|q.val| + massL out + (|x.val| + massL xs)) * (1 + 4 * (xs.length + 1) * uq) +
            3 * (xs.length + 1) * ηq := by
        have : (0 : ℚ) ≤ 4 * (xs.length + 1) * uq := by positivity
        nlinarith
      have hqx : |q.val| + |x.val| ≤ Kmax := by linarith
      obtain ⟨f1, f2, es, ms, mm⟩ := twoSum_good hq.1 hx.1 hqx
      obtain ⟨g1, g2⟩ := mm L hL hq.2 hx.2
      set p := twoSum q x with hp
      simp only [List.foldl_cons]
      -- the step's state, then the rest by induction
      have hout' : ∀ y ∈ (growStep (q, out) x).2, Good L y := by
        intro y hy
        simp only [growStep, ← hp] at hy
        split at hy
        · rcases List.mem_append.mp hy with h | h
          · exact ho y h
          · simp at h; rw [h]; exact ⟨f2, g2⟩
        · exact ho y hy
      have esq : (growStep (q, out) x).1.val + esumL (growStep (q, out) x).2 =
          q.val + esumL out + x.val := by
        simp only [growStep, ← hp]
        split
        · simp only [esumL_append, esumL_cons, esumL_nil, massL_append, massL_cons, massL_nil]; linarith
        · rename_i hz
          have : p.2.val = 0 := eq_zero_val f2 (by simpa [F64.Ops.ne_def'] using hz)
          linarith
      have msq : |(growStep (q, out) x).1.val| + massL (growStep (q, out) x).2 ≤
          (1 + 2 * uq) * (|q.val| + |x.val|) + 2 * ηq + massL out := by
        simp only [growStep, ← hp]
        split
        · simp only [esumL_append, esumL_cons, esumL_nil, massL_append, massL_cons, massL_nil]; linarith
        · have := abs_nonneg p.2.val; linarith
      have hT' : |(growStep (q, out) x).1.val| + massL (growStep (q, out) x).2 + massL xs ≤
          (1 + 2 * uq) * (|q.val| + massL out + (|x.val| + massL xs)) + 2 * ηq := by
        have : (1 + 2 * uq) * (|q.val| + |x.val|) + massL out + massL xs ≤
            (1 + 2 * uq) * (|q.val| + massL out + (|x.val| + massL xs)) := by nlinarith
        linarith
      have key := mass_step hT0 hu0 hη0 hk'
      have hB' : (|(growStep (q, out) x).1.val| + massL (growStep (q, out) x).2 + massL xs) *
          (1 + 4 * xs.length * uq) + 3 * xs.length * ηq ≤ Kmax := by
        have := mul_le_mul_of_nonneg_right hT' (by positivity : (0 : ℚ) ≤ 1 + 4 * xs.length * uq)
        linarith
      obtain ⟨r1, r2, r3, r4⟩ := fold_grow_good L hL xs (growStep (q, out) x).1 (growStep (q, out) x).2
        (by simp only [growStep, ← hp]; exact ⟨f1, g1⟩) hout' hxs hk' hB'
      refine ⟨r1, r2, by rw [r3, esq]; ring, ?_⟩
      have := mul_le_mul_of_nonneg_right hT' (by positivity : (0 : ℚ) ≤ 1 + 4 * xs.length * uq)
      linarith

/-! ## Two-product on bounded, gridded doubles -/

/-- The operands' bound, `2⁵⁰⁰`, opaque like `Kmax`. -/
irreducible_def Amax : ℚ := 2 ^ (500 : ℤ)

theorem Amax_pos : 0 < Amax := by rw [Amax_def]; positivity
theorem Amax_sq : Amax * Amax < 2 ^ (1022 : ℤ) := by
  rw [Amax_def, ← two_zpow_add]; exact two_zpow_lt (by norm_num)
theorem Amax_sq_K : Amax * Amax ≤ Kmax := by
  rw [Amax_def, Kmax_def, ← two_zpow_add]; exact two_zpow_le (by norm_num)
theorem split_bound : (2 ^ 27 + 1 : ℚ) * Amax ≤ 8 * Kmax := by
  rw [Amax_def, Kmax_def]
  have : (2 ^ 27 + 1 : ℚ) ≤ 2 ^ (28 : ℤ) := by norm_num
  have h2 : (2 : ℚ) ^ (28 : ℤ) * 2 ^ (500 : ℤ) ≤ 8 * 2 ^ (1020 : ℤ) := by
    rw [← two_zpow_add]
    have : (8 : ℚ) = 2 ^ (3 : ℤ) := by norm_num
    rw [this, ← two_zpow_add]; exact two_zpow_le (by norm_num)
  calc (2 ^ 27 + 1 : ℚ) * 2 ^ (500 : ℤ) ≤ 2 ^ (28 : ℤ) * 2 ^ (500 : ℤ) :=
        mul_le_mul_of_nonneg_right this (by positivity)
    _ ≤ _ := h2

/-- **Two-product on bounded, gridded doubles** (Dekker, exact: verify/ieee754's
`two_product_exact` with the code's splitter): finite parts summing to `a b`, their magnitudes
within `(1 + 2u)|a b| + 2η`, on the grid `2^(la + lb)`. -/
theorem twoProduct_good {a b : _root_.F64} (ha : a.isFinite) (hb : b.isFinite)
    (haA : |a.val| ≤ Amax) (hbA : |b.val| ≤ Amax) {la lb : ℤ} (hla : Mult a.val la)
    (hlb : Mult b.val lb) (hΛ : -1074 ≤ la + lb) :
    let p := twoProduct ForgeMath.splitter a b
    p.1.isFinite ∧ p.2.isFinite ∧ p.1.val + p.2.val = a.val * b.val ∧
      |p.1.val| + |p.2.val| ≤ (1 + 2 * uq) * |a.val * b.val| + 2 * ηq ∧
      Mult p.1.val (la + lb) ∧ Mult p.2.val (la + lb) := by
  intro p
  obtain ⟨sf, sv⟩ := ForgeMath.splitter_spec
  have habs : |a.val * b.val| ≤ Amax * Amax := by
    rw [abs_mul]; exact mul_le_mul haA hbA (abs_nonneg _) Amax_pos.le
  have hca : (FP.mul ForgeMath.splitter a).isFinite := by
    rw [mul_eq_round sf ha]; apply fin_of_le8
    rw [abs_mul, sv, abs_of_pos (by positivity)]
    exact le_trans (mul_le_mul_of_nonneg_left haA (by positivity)) split_bound
  have hcb : (FP.mul ForgeMath.splitter b).isFinite := by
    rw [mul_eq_round sf hb]; apply fin_of_le8
    rw [abs_mul, sv, abs_of_pos (by positivity)]
    exact le_trans (mul_le_mul_of_nonneg_left hbA (by positivity)) split_bound
  obtain ⟨h1, h2, h3⟩ := IEEE754.two_product_exact (f := IEEE754.binary64) (σ := 27) (by decide)
    (by norm_num) sf (by rw [sv]) ha hb hca hcb hla hlb (by simpa using hΛ)
    (by simpa using lt_of_le_of_lt habs Amax_sq)
  have hmf : (FP.mul a b).isFinite := by
    rw [mul_eq_round ha hb]; apply fin_of_le8
    have := Amax_sq_K; have := Kmax_ge; linarith only [habs, this, ‹Amax * Amax ≤ Kmax›]
  have em : |(FP.mul a b).val - a.val * b.val| ≤ uq * |a.val * b.val| + ηq := by
    have := err_le (zs := a.sign ^^ b.sign) (r := a.val * b.val) (by rwa [← mul_eq_round ha hb])
    rwa [← mul_eq_round ha hb] at this
  have hp1 : p.1 = FP.mul a b := h1
  have e2 : p.2.val = a.val * b.val - (FP.mul a b).val := by rw [← hp1]; linarith
  refine ⟨by rw [hp1]; exact hmf, h2, h3, ?_, ?_, ?_⟩
  · rw [hp1, e2, abs_sub_comm]
    have := abs_sub_abs_le_abs_sub (FP.mul a b).val (a.val * b.val)
    linarith
  · rw [hp1]; rw [mul_eq_round ha hb] at hmf ⊢
    exact round_mult (mult_mul hla hlb) (by simpa using hΛ) hmf
  · rw [e2]
    have : Mult (FP.mul a b).val (la + lb) := by
      rw [mul_eq_round ha hb] at hmf ⊢
      exact round_mult (mult_mul hla hlb) (by simpa using hΛ) hmf
    exact mult_sub (mult_mul hla hlb) this

/-! ## Grow-Expansion and sequences of it -/

/-- **Grow-Expansion**: every component finite and on the grid, the exact sum, the mass. -/
theorem growL_good (L : ℤ) (hL : -1074 ≤ L) (e : List _root_.F64) (b : _root_.F64)
    (he : ∀ x ∈ e, Good L x) (hb : Good L b) (hn : (e.length : ℚ) * (8 * uq) ≤ 1)
    (hB : (|b.val| + massL e) * (1 + 4 * e.length * uq) + 3 * e.length * ηq ≤ Kmax) :
    (∀ x ∈ growL e b, Good L x) ∧ esumL (growL e b) = b.val + esumL e ∧
      massL (growL e b) ≤ (|b.val| + massL e) * (1 + 4 * e.length * uq) + 3 * e.length * ηq := by
  have := fold_grow_good L hL e b [] hb (by simp) he hn (by simpa using hB)
  simp only [massL_nil, add_zero, esumL_nil] at this
  obtain ⟨r1, r2, r3, r4⟩ := this
  unfold growL
  dsimp only
  split
  · refine ⟨?_, ?_, ?_⟩
    · intro x hx; rcases List.mem_append.mp hx with h | h
      · exact r2 x h
      · simp at h; rw [h]; exact r1
    · simp only [esumL_append, esumL_cons, esumL_nil]; linarith
    · simp only [massL_append, massL_cons, massL_nil]; linarith
  · rename_i hz
    have h0 := eq_zero_val r1.1 (by simpa [F64.Ops.ne_def'] using hz)
    refine ⟨r2, by linarith, ?_⟩
    have := abs_nonneg (e.foldl growStep (b, [])).1.val
    linarith

theorem step_c1 {N r u : ℚ} (hN : 0 ≤ N) (hu : 0 ≤ u) (hk : N * r * (8 * u) ≤ 1) :
    (1 + 4 * N * u) * (1 + 8 * N * r * u) ≤ 1 + 8 * N * (r + 1) * u := by
  have e : (1 + 4 * N * u) * (1 + 8 * N * r * u) - (1 + 8 * N * (r + 1) * u) =
      (4 * N * u) * (N * r * (8 * u) - 1) := by ring
  have : (4 * N * u) * (N * r * (8 * u) - 1) ≤ 0 :=
    mul_nonpos_of_nonneg_of_nonpos (by positivity) (by linarith)
  linarith

theorem step_c2 {N r u η : ℚ} (hN : 0 ≤ N) (hη : 0 ≤ η) (hk : N * r * (8 * u) ≤ 1) :
    3 * N * η * (1 + 8 * N * r * u) ≤ 6 * N * η := by
  have e : 3 * N * η * (1 + 8 * N * r * u) - 6 * N * η = (3 * N * η) * (N * r * (8 * u) - 1) := by ring
  have : (3 * N * η) * (N * r * (8 * u) - 1) ≤ 0 :=
    mul_nonpos_of_nonneg_of_nonpos (by positivity) (by linarith)
  linarith

theorem le_succ_mul {t r : ℚ} (ht : 0 ≤ t) (hr : 0 ≤ r) : t ≤ t * (r + 1) := by
  have := mul_le_mul_of_nonneg_left (show (1 : ℚ) ≤ r + 1 by linarith) ht; linarith

/-- A sequence of grows: each of `xs` grown into `out` in turn. -/
def growMany (out xs : List _root_.F64) : List _root_.F64 := xs.foldl growL out

/-- A property of expansions preserved by every grow made under `growL_good`'s premises (the
expansion theory's invariant; `True` when only the sums matter). -/
def GrowInv (L : ℤ) (P : List _root_.F64 → Prop) : Prop :=
  ∀ e b, P e → (∀ x ∈ e, Good L x) → Good L b → (e.length : ℚ) * (8 * uq) ≤ 1 →
    (|b.val| + massL e) * (1 + 4 * e.length * uq) + 3 * e.length * ηq ≤ Kmax → P (growL e b)

theorem growMany_good (L : ℤ) (hL : -1074 ≤ L) (N : ℕ) (P : List _root_.F64 → Prop)
    (hP : GrowInv L P) :
    ∀ (xs out : List _root_.F64), P out → (∀ x ∈ out, Good L x) → (∀ x ∈ xs, Good L x) →
      out.length + xs.length ≤ N → (N : ℚ) * xs.length * (8 * uq) ≤ 1 →
      (massL out + massL xs) * (1 + 8 * N * xs.length * uq) + 6 * N * xs.length * ηq ≤ Kmax →
      P (growMany out xs) ∧
      (∀ x ∈ growMany out xs, Good L x) ∧ esumL (growMany out xs) = esumL out + esumL xs ∧
        massL (growMany out xs) ≤
          (massL out + massL xs) * (1 + 8 * N * xs.length * uq) + 6 * N * xs.length * ηq
  | [], out, hPo, ho, _, _, _, _ => by simp [growMany]; exact ⟨hPo, ho⟩
  | x :: rest, out, hPo, ho, hx, hl, hk, hB => by
      simp only [List.length_cons, Nat.cast_add, Nat.cast_one, massL_cons, esumL_cons] at hl hk hB ⊢
      have hxg := hx x (by simp)
      have hrest : ∀ y ∈ rest, Good L y := fun y h => hx y (by simp [h])
      have hu0 : (0 : ℚ) ≤ uq := uq_pos.le
      have hη0 : (0 : ℚ) ≤ ηq := ηq_pos.le
      have hN0 : (0 : ℚ) ≤ N := Nat.cast_nonneg N
      have hr0 : (0 : ℚ) ≤ rest.length := Nat.cast_nonneg _
      have hM0 := massL_nonneg out
      have hM1 := massL_nonneg rest
      have hxa := abs_nonneg x.val
      have hol : (out.length : ℚ) ≤ N := by exact_mod_cast (by omega : out.length ≤ N)
      have hNu : (N : ℚ) * (8 * uq) ≤ 1 := by
        have := le_succ_mul (by positivity : (0 : ℚ) ≤ N * (8 * uq)) hr0
        have e : (N : ℚ) * (8 * uq) * (rest.length + 1) = N * (rest.length + 1) * (8 * uq) := by ring
        linarith
      have hnu : (out.length : ℚ) * (8 * uq) ≤ 1 :=
        le_trans (mul_le_mul_of_nonneg_right hol (by positivity)) hNu
      set A := massL out + (|x.val| + massL rest)
      have hA0 : 0 ≤ A := by positivity
      have hNr : (N : ℚ) ≤ N * (rest.length + 1) := le_succ_mul hN0 hr0
      -- the budget for this grow
      have hB1 : (|x.val| + massL out) * (1 + 4 * out.length * uq) + 3 * out.length * ηq ≤ Kmax := by
        have a1 : (|x.val| + massL out) ≤ A := by simp only [A]; linarith
        have a2 : (1 + 4 * (out.length : ℚ) * uq) ≤ 1 + 8 * N * (rest.length + 1) * uq := by
          have : 4 * (out.length : ℚ) * uq ≤ 4 * N * uq := by
            have := mul_le_mul_of_nonneg_right hol (by positivity : (0 : ℚ) ≤ 4 * uq); linarith
          have : 4 * (N : ℚ) * uq ≤ 8 * N * (rest.length + 1) * uq := by
            have := mul_le_mul_of_nonneg_right hNr (by positivity : (0 : ℚ) ≤ 8 * uq)
            have : 4 * (N : ℚ) * uq ≤ 8 * N * uq := by
              have : (0 : ℚ) ≤ 4 * N * uq := by positivity
              linarith
            linarith
          linarith
        have a3 : 3 * (out.length : ℚ) * ηq ≤ 6 * N * (rest.length + 1) * ηq := by
          have := mul_le_mul_of_nonneg_right hol (by positivity : (0 : ℚ) ≤ 3 * ηq)
          have := mul_le_mul_of_nonneg_right hNr (by positivity : (0 : ℚ) ≤ 6 * ηq)
          have : (0 : ℚ) ≤ 3 * N * ηq := by positivity
          linarith
        have := mul_le_mul a1 a2 (by positivity) hA0
        linarith
      obtain ⟨g1, g2, g3⟩ := growL_good L hL out x ho hxg hnu hB1
      -- the grown expansion's mass, within the N-uniform step
      have hm' : massL (growL out x) ≤ (|x.val| + massL out) * (1 + 4 * N * uq) + 3 * N * ηq := by
        have : (|x.val| + massL out) * (1 + 4 * out.length * uq) ≤ (|x.val| + massL out) * (1 + 4 * N * uq) :=
          mul_le_mul_of_nonneg_left (by
            have := mul_le_mul_of_nonneg_right hol (by positivity : (0 : ℚ) ≤ 4 * uq); linarith)
            (by positivity)
        have : 3 * (out.length : ℚ) * ηq ≤ 3 * N * ηq := by
          have := mul_le_mul_of_nonneg_right hol (by positivity : (0 : ℚ) ≤ 3 * ηq); linarith
        linarith
      have hlen : (growL out x).length + rest.length ≤ N := by have := growL_len out x; omega
      have hk' : (N : ℚ) * rest.length * (8 * uq) ≤ 1 := by
        have : (N : ℚ) * rest.length * (8 * uq) ≤ N * (rest.length + 1) * (8 * uq) := by
          have : (0 : ℚ) ≤ N * (8 * uq) := by positivity
          nlinarith
        linarith
      -- the numerical step
      have c1 := step_c1 hN0 hu0 hk'
      have c2 := step_c2 (η := ηq) hN0 hη0 hk'
      have s1 : massL (growL out x) + massL rest ≤ A * (1 + 4 * N * uq) + 3 * N * ηq := by
        have : (|x.val| + massL out) * (1 + 4 * N * uq) + massL rest ≤ A * (1 + 4 * N * uq) := by
          have : massL rest ≤ massL rest * (1 + 4 * N * uq) :=
            le_trans (le_refl _) (by
              have := le_succ_mul hM1 (by positivity : (0 : ℚ) ≤ 4 * N * uq); linarith)
          have e : A * (1 + 4 * N * uq) = (|x.val| + massL out) * (1 + 4 * N * uq) +
              massL rest * (1 + 4 * N * uq) := by simp only [A]; ring
          linarith
        linarith
      have s2 : (A * (1 + 4 * N * uq) + 3 * N * ηq) * (1 + 8 * N * rest.length * uq) ≤
          A * (1 + 8 * N * (rest.length + 1) * uq) + 6 * N * ηq := by
        have e : (A * (1 + 4 * N * uq) + 3 * N * ηq) * (1 + 8 * N * rest.length * uq) =
            A * ((1 + 4 * N * uq) * (1 + 8 * N * rest.length * uq)) +
              3 * N * ηq * (1 + 8 * N * rest.length * uq) := by ring
        have := mul_le_mul_of_nonneg_left c1 hA0
        linarith
      have s3 := mul_le_mul_of_nonneg_right s1 (by positivity : (0 : ℚ) ≤ 1 + 8 * N * rest.length * uq)
      have hB' : (massL (growL out x) + massL rest) * (1 + 8 * N * rest.length * uq) +
          6 * N * rest.length * ηq ≤ Kmax := by
        have e : 6 * (N : ℚ) * (rest.length + 1) * ηq = 6 * N * rest.length * ηq + 6 * N * ηq := by ring
        linarith
      obtain ⟨h0, h1, h2, h3⟩ := growMany_good L hL N P hP rest (growL out x)
        (hP out x hPo ho hxg hnu hB1) g1 hrest hlen hk' hB'
      simp only [growMany, List.foldl_cons] at h0 h1 h2 h3 ⊢
      refine ⟨h0, h1, by rw [h2, g2]; ring, ?_⟩
      have e : 6 * (N : ℚ) * (rest.length + 1) * ηq = 6 * N * rest.length * ηq + 6 * N * ηq := by ring
      linarith

/-! ## The product's terms -/

abbrev tpr (a b : _root_.F64) := twoProduct ForgeMath.splitter a b

/-- The two-product terms of `e o`, row by row, error before product (`exp_mul`'s order). -/
def prodTerms (e o : List _root_.F64) : List _root_.F64 :=
  e.flatMap fun a => o.flatMap fun b => [(tpr a b).2, (tpr a b).1]

theorem mulRow_eq (a : _root_.F64) (o out : List _root_.F64) :
    mulRow a o out = growMany out (o.flatMap fun b => [(tpr a b).2, (tpr a b).1]) := by
  induction o generalizing out with
  | nil => rfl
  | cons b bs ih =>
      simp only [mulRow, List.foldl_cons, List.flatMap_cons, growMany, List.foldl_append] at ih ⊢
      rw [ih]; rfl

theorem mulL_eq (e o : List _root_.F64) : mulL e o = growMany [] (prodTerms e o) := by
  unfold mulL prodTerms
  suffices h : ∀ out, e.foldl (fun out a => mulRow a o out) out =
      growMany out (e.flatMap fun a => o.flatMap fun b => [(tpr a b).2, (tpr a b).1]) from h []
  induction e with
  | nil => intro out; rfl
  | cons a rest ih =>
      intro out
      simp only [List.foldl_cons, List.flatMap_cons]
      rw [ih, mulRow_eq]; simp only [growMany, List.foldl_append]

theorem prodTerms_len (e o : List _root_.F64) : (prodTerms e o).length = 2 * e.length * o.length := by
  unfold prodTerms
  induction e with
  | nil => simp
  | cons a rest ih =>
      simp only [List.flatMap_cons, List.length_append, ih, List.length_cons]
      have : (o.flatMap fun b => [(tpr a b).2, (tpr a b).1]).length = 2 * o.length := by
        induction o with
        | nil => simp
        | cons b bs ihb => simp only [List.flatMap_cons, List.length_append, ihb]; simp; ring
      rw [this]; ring

/-- **The product's terms**: each good on the grid `2^(Le + Lo)`, their sum `Σe · Σo` exactly,
their mass within `(1 + 2u) Me Mo + 2 |e||o| η`. -/
theorem prodTerms_good {Le Lo : ℤ} (hΛ : -1074 ≤ Le + Lo) (e o : List _root_.F64)
    (he : ∀ x ∈ e, Good Le x ∧ |x.val| ≤ Amax) (ho : ∀ x ∈ o, Good Lo x ∧ |x.val| ≤ Amax) :
    (∀ x ∈ prodTerms e o, Good (Le + Lo) x) ∧ esumL (prodTerms e o) = esumL e * esumL o ∧
      massL (prodTerms e o) ≤ (1 + 2 * uq) * (massL e * massL o) + 2 * e.length * o.length * ηq := by
  have row : ∀ a, Good Le a ∧ |a.val| ≤ Amax →
      (∀ x ∈ o.flatMap (fun b => [(tpr a b).2, (tpr a b).1]), Good (Le + Lo) x) ∧
      esumL (o.flatMap (fun b => [(tpr a b).2, (tpr a b).1])) = a.val * esumL o ∧
      massL (o.flatMap (fun b => [(tpr a b).2, (tpr a b).1])) ≤
        (1 + 2 * uq) * (|a.val| * massL o) + 2 * o.length * ηq := by
    intro a ha
    induction o with
    | nil => simp
    | cons b bs ih =>
        have hb := ho b (by simp)
        obtain ⟨i1, i2, i3⟩ := ih (fun x h => ho x (by simp [h]))
        obtain ⟨p1, p2, p3, p4, p5, p6⟩ :=
          twoProduct_good ha.1.1 hb.1.1 ha.2 hb.2 ha.1.2 hb.1.2 hΛ
        simp only [List.flatMap_cons, List.mem_append, esumL_append, massL_append, esumL_cons,
          massL_cons, esumL_nil, massL_nil, List.length_cons, Nat.cast_add, Nat.cast_one]
        refine ⟨?_, ?_, ?_⟩
        · intro x hx
          rcases hx with hx | hx
          · simp at hx; rcases hx with rfl | rfl
            · exact ⟨p2, p6⟩
            · exact ⟨p1, p5⟩
          · exact i1 x hx
        · rw [i2]; linarith
        · rw [abs_mul] at p4
          have : (1 + 2 * uq) * (|a.val| * (|b.val| + massL bs)) =
              (1 + 2 * uq) * (|a.val| * |b.val|) + (1 + 2 * uq) * (|a.val| * massL bs) := by ring
          linarith
  induction e with
  | nil => simp [prodTerms]
  | cons a rest ih =>
      obtain ⟨i1, i2, i3⟩ := ih (fun x h => he x (by simp [h]))
      obtain ⟨r1, r2, r3⟩ := row a (he a (by simp))
      unfold prodTerms at i1 i2 i3 ⊢
      simp only [List.flatMap_cons, List.mem_append, esumL_append, massL_append, esumL_cons,
        massL_cons, List.length_cons, Nat.cast_add, Nat.cast_one]
      refine ⟨fun x hx => hx.elim (r1 x) (i1 x), by rw [r2, i2]; ring, ?_⟩
      have : (1 + 2 * uq) * ((|a.val| + massL rest) * massL o) =
          (1 + 2 * uq) * (|a.val| * massL o) + (1 + 2 * uq) * (massL rest * massL o) := by ring
      have : 2 * ((rest.length : ℚ) + 1) * o.length * ηq = 2 * o.length * ηq + 2 * rest.length * o.length * ηq := by ring
      linarith

/-! ## The expansion theory, as one named hypothesis -/

/-- The sign of an exact value, as `exp_sign` writes it. -/
def sgnQ (q : ℚ) : _root_.F64 :=
  if 0 < q then F64.Ops.ofBits 4607182418800017408
  else if q < 0 then F64.Ops.ofBits 13830554455654793216 else Z

/-- **The expansion theory** (Shewchuk 1997), the one lemma this proof assumes, as a hypothesis
(never an axiom), for verify/ieee754 to discharge: an invariant `P` of expansions (Shewchuk's:
nonoverlapping, increasing magnitude), held by the empty expansion, kept by every
Grow-Expansion with zero elimination made under `growL_good`'s premises, under which the sign
of the last nonzero component is the sign of the exact sum. -/
structure ExpansionTheory where
  P : List _root_.F64 → Prop
  nil : P []
  grow : ∀ L, -1074 ≤ L → GrowInv L P
  sign : ∀ e, P e → (∀ x ∈ e, x.isFinite) → signL e = sgnQ (esumL e)

/-! ## The pieces' facts -/

theorem comp_le_mass {l : List _root_.F64} {x : _root_.F64} (h : x ∈ l) : |x.val| ≤ massL l := by
  induction l with
  | nil => simp at h
  | cons y ys ih =>
      simp only [massL_cons]
      rcases List.mem_cons.mp h with rfl | h
      · have := massL_nonneg ys; linarith
      · have := ih h; have := abs_nonneg y.val; linarith

theorem ofL_eq (x : _root_.F64) : ofL x = growL [] x := by
  unfold ofL growL; simp

theorem ofL_good {L : ℤ} {x : _root_.F64} (hx : Good L x) :
    (∀ y ∈ ofL x, Good L y) ∧ esumL (ofL x) = x.val ∧ massL (ofL x) = |x.val| ∧ (ofL x).length ≤ 1 := by
  unfold ofL
  split
  · simp; exact hx
  · rename_i h
    have := eq_zero_val hx.1 (by simpa [F64.Ops.ne_def'] using h)
    simp [this]

theorem neg_good {L : ℤ} {x : _root_.F64} (hx : Good L x) : Good L (FP.neg x) :=
  ⟨by simpa using hx.1, by rw [FP.val_neg]; exact mult_neg hx.2⟩

theorem esumL_negL (e : List _root_.F64) : esumL (negL e) = -esumL e := by
  induction e with
  | nil => simp [negL]
  | cons x xs ih => simp only [negL, List.map_cons, esumL_cons, FP.val_neg] at ih ⊢; rw [ih]; ring

theorem massL_negL (e : List _root_.F64) : massL (negL e) = massL e := by
  induction e with
  | nil => simp [negL]
  | cons x xs ih => simp only [negL, List.map_cons, massL_cons, FP.val_neg, abs_neg] at ih ⊢; rw [ih]

theorem negL_good {L : ℤ} {e : List _root_.F64} (he : ∀ x ∈ e, Good L x) :
    (∀ y ∈ negL e, Good L y) ∧ esumL (negL e) = -esumL e ∧ massL (negL e) = massL e ∧
      (negL e).length = e.length := by
  unfold negL
  refine ⟨?_, esumL_negL e, massL_negL e, by simp⟩
  intro y hy; simp at hy; obtain ⟨x, hx, rfl⟩ := hy; exact neg_good (he x hx)

/-- `C0 = 2¹⁰⁰`, the inputs' bound, opaque. -/
irreducible_def C0 : ℚ := 2 ^ (100 : ℤ)

theorem C0_ge : 1 ≤ C0 := by rw [C0_def]; exact one_le_zpow₀ (by norm_num) (by norm_num)
theorem C0_A : 4 * C0 ≤ Amax := by
  rw [C0_def, Amax_def]
  have : (4 : ℚ) = 2 ^ (2 : ℤ) := by norm_num
  rw [this, ← two_zpow_add]; exact two_zpow_le (by norm_num)
theorem C0_K : 1024 * (C0 * C0) ≤ Kmax := by
  rw [C0_def, Kmax_def, ← two_zpow_add]
  have : (1024 : ℚ) = 2 ^ (10 : ℤ) := by norm_num
  rw [this, ← two_zpow_add]; exact two_zpow_le (by norm_num)
theorem uq_small : uq ≤ 1 / 2 ^ 20 := by rw [uq_def]; norm_num
theorem ηq_le_uq : ηq ≤ uq := by rw [ηq_def, uq_def]; exact zpow_le_zpow_right₀ (by norm_num) (by norm_num)
theorem ηq_small : ηq ≤ 1 := ηq_le1

/-- A crude grow budget: `M (1 + 8 N k u) + 6 N k η ≤ 2 M + 1` for `N, k ≤ 32`. -/
theorem budget2 {M : ℚ} {N k : ℕ} (hM : 0 ≤ M) (hN : N ≤ 32) (hk : k ≤ 32) :
    M * (1 + 8 * N * k * uq) + 6 * N * k * ηq ≤ 2 * M + 1 ∧ (N : ℚ) * k * (8 * uq) ≤ 1 := by
  have hN' : (N : ℚ) ≤ 32 := by exact_mod_cast hN
  have hk' : (k : ℚ) ≤ 32 := by exact_mod_cast hk
  have h0 : (0 : ℚ) ≤ N := Nat.cast_nonneg _
  have h1 : (0 : ℚ) ≤ k := Nat.cast_nonneg _
  have hu := uq_small
  have hu0 : (0 : ℚ) ≤ uq := uq_pos.le
  have hη := ηq_small
  have hη0 : (0 : ℚ) ≤ ηq := ηq_pos.le
  have nk : (N : ℚ) * k ≤ 1024 := by nlinarith
  have nk0 : (0 : ℚ) ≤ N * k := by positivity
  have a : 8 * (N : ℚ) * k * uq ≤ 1 := by
    have h2 : (N : ℚ) * k * uq ≤ 1024 * (1 / 2 ^ 20) := mul_le_mul nk hu hu0 (by norm_num)
    have e : 8 * (N : ℚ) * k * uq = 8 * (N * k * uq) := by ring
    have : (8 : ℚ) * (1024 * (1 / 2 ^ 20)) ≤ 1 := by norm_num
    rw [e]; exact le_trans (mul_le_mul_of_nonneg_left h2 (by norm_num)) this
  have b : 6 * (N : ℚ) * k * ηq ≤ 1 := by
    have h3 : (N : ℚ) * k * ηq ≤ N * k * uq := mul_le_mul_of_nonneg_left ηq_le_uq nk0
    have e : 6 * (N : ℚ) * k * ηq = 6 * (N * k * ηq) := by ring
    have e2 : 8 * (N : ℚ) * k * uq = 8 * (N * k * uq) := by ring
    rw [e2] at a; rw [e]
    have : 6 * (N * k * ηq) ≤ 8 * (N * k * uq) := by
      have := mul_le_mul_of_nonneg_left h3 (by norm_num : (0 : ℚ) ≤ 6)
      have : (0 : ℚ) ≤ N * k * uq := by positivity
      linarith
    exact le_trans this a
  refine ⟨?_, by have e : (N : ℚ) * k * (8 * uq) = 8 * N * k * uq := by ring
                 rw [e]; exact a⟩
  have := mul_le_mul_of_nonneg_left a hM
  have e : M * (1 + 8 * N * k * uq) = M + M * (8 * N * k * uq) := by ring
  rw [e]; linarith

/-! ## `disc`'s exact branch on the models -/

theorem growMany_len (out xs : List _root_.F64) : (growMany out xs).length ≤ out.length + xs.length := by
  induction xs generalizing out with
  | nil => simp [growMany]
  | cons x xs ih =>
      simp only [growMany, List.foldl_cons, List.length_cons] at ih ⊢
      have := ih (growL out x); have := growL_len out x; omega

theorem C0_sq : C0 ≤ C0 * C0 := by have := C0_ge; nlinarith
theorem C0_16 : 16 ≤ C0 := by
  rw [C0_def]; have : (16 : ℚ) = 2 ^ (4 : ℤ) := by norm_num
  rw [this]; exact two_zpow_le (by norm_num)

/-- A difference of two inputs as an expansion. -/
theorem diff_facts {a b : _root_.F64} (ha : Good (-200) a ∧ |a.val| ≤ C0)
    (hb : Good (-200) b ∧ |b.val| ≤ C0) :
    let D := diffL a b
    (∀ x ∈ D, Good (-200) x) ∧ esumL D = a.val - b.val ∧ massL D ≤ 3 * C0 ∧ D.length ≤ 2 := by
  intro D
  obtain ⟨o1, o2, o3, o4⟩ := ofL_good ha.1
  have hC := C0_ge
  have hK := C0_K
  have hs := C0_sq
  have hu := uq_small
  have hu0 : (0 : ℚ) ≤ uq := uq_pos.le
  have hη := ηq_small
  have hη0 : (0 : ℚ) ≤ ηq := ηq_pos.le
  have hl : ((ofL a).length : ℚ) ≤ 1 := by exact_mod_cast o4
  have hl0 : (0 : ℚ) ≤ (ofL a).length := Nat.cast_nonneg _
  have hm : |(FP.neg b).val| + massL (ofL a) ≤ 2 * C0 := by
    rw [FP.val_neg, abs_neg, o3]; linarith [ha.2, hb.2]
  have hm0 : 0 ≤ |(FP.neg b).val| + massL (ofL a) := add_nonneg (abs_nonneg _) (massL_nonneg _)
  have h16 := C0_16
  have f1 : (1 + 4 * ((ofL a).length : ℚ) * uq) ≤ 1 + 4 * (1 / 2 ^ 20) := by nlinarith
  have f2 : 3 * ((ofL a).length : ℚ) * ηq ≤ 3 := by nlinarith
  have bnd : (|(FP.neg b).val| + massL (ofL a)) * (1 + 4 * (ofL a).length * uq) +
      3 * (ofL a).length * ηq ≤ 3 * C0 := by
    have := mul_le_mul hm f1 (by positivity) (by positivity)
    have : 2 * C0 * (1 + 4 * (1 / 2 ^ 20)) + 3 ≤ 3 * C0 := by
      have e : 2 * C0 * (1 + 4 * (1 / 2 ^ 20)) = 2 * C0 + C0 * (8 / 2 ^ 20) := by ring
      have : C0 * (8 / 2 ^ 20) ≤ C0 * (1 / 4) := mul_le_mul_of_nonneg_left (by norm_num) (by linarith)
      linarith
    linarith
  obtain ⟨g1, g2, g3⟩ := growL_good (-200) (by norm_num) (ofL a) (FP.neg b) o1 (neg_good hb.1)
    (by nlinarith) (by linarith)
  simp only [D, diffL]
  refine ⟨g1, ?_, by linarith, ?_⟩
  · rw [g2, o2, FP.val_neg]; ring
  · have := growL_len (ofL a) (FP.neg b); omega

/-- A product of two such expansions. -/
theorem mul_facts (Tth : ExpansionTheory) {D E : List _root_.F64} (hD : ∀ x ∈ D, Good (-200) x)
    (hE : ∀ x ∈ E, Good (-200) x) (hmD : massL D ≤ 3 * C0) (hmE : massL E ≤ 3 * C0)
    (hlD : D.length ≤ 2) (hlE : E.length ≤ 2) :
    let M := mulL D E
    Tth.P M ∧ (∀ x ∈ M, Good (-400) x) ∧ esumL M = esumL D * esumL E ∧
      massL M ≤ 21 * (C0 * C0) + 1 ∧ M.length ≤ 8 := by
  intro M
  have hC := C0_ge
  have hA := C0_A
  have hu := uq_small
  have hu0 : (0 : ℚ) ≤ uq := uq_pos.le
  have hη := ηq_small
  have hη0 : (0 : ℚ) ≤ ηq := ηq_pos.le
  have cD : ∀ x ∈ D, Good (-200) x ∧ |x.val| ≤ Amax := fun x h =>
    ⟨hD x h, by have := comp_le_mass h; linarith⟩
  have cE : ∀ x ∈ E, Good (-200) x ∧ |x.val| ≤ Amax := fun x h =>
    ⟨hE x h, by have := comp_le_mass h; linarith⟩
  obtain ⟨t1, t2, t3⟩ := prodTerms_good (Le := -200) (Lo := -200) (by norm_num) D E cD cE
  have hlen : (prodTerms D E).length ≤ 8 := by
    rw [prodTerms_len]; have := Nat.mul_le_mul (Nat.mul_le_mul_left 2 hlD) hlE; omega
  have hmD0 := massL_nonneg D
  have hmE0 := massL_nonneg E
  have hmT : massL (prodTerms D E) ≤ 10 * (C0 * C0) := by
    have a : massL D * massL E ≤ 3 * C0 * (3 * C0) := mul_le_mul hmD hmE hmE0 (by positivity)
    have b : (D.length : ℚ) * E.length ≤ 4 := by
      have : (D.length : ℚ) ≤ 2 := by exact_mod_cast hlD
      have : (E.length : ℚ) ≤ 2 := by exact_mod_cast hlE
      have : (0 : ℚ) ≤ E.length := Nat.cast_nonneg _
      nlinarith
    have hs := C0_sq
    have : (1 + 2 * uq) * (massL D * massL E) ≤ (1 + 2 * (1 / 2 ^ 20)) * (3 * C0 * (3 * C0)) :=
      mul_le_mul (by linarith) a (by positivity) (by positivity)
    have : 2 * (D.length : ℚ) * E.length * ηq ≤ 8 := by
      have : (0 : ℚ) ≤ D.length * E.length := by positivity
      have := mul_le_mul b hη hη0 (by norm_num)
      nlinarith
    have h16 := C0_16
    have e : (1 + 2 * (1 / 2 ^ 20 : ℚ)) * (3 * C0 * (3 * C0)) = 9 * (C0 * C0) + (C0 * C0) * (18 / 2 ^ 20) := by ring
    have : (C0 * C0) * (18 / 2 ^ 20) + 8 ≤ C0 * C0 := by
      have : (256 : ℚ) ≤ C0 * C0 := by nlinarith
      have : (C0 * C0) * (18 / 2 ^ 20) ≤ (C0 * C0) * (1 / 2) := mul_le_mul_of_nonneg_left (by norm_num) (by positivity)
      linarith
    linarith
  obtain ⟨bA, bB⟩ := budget2 (M := 0 + massL (prodTerms D E)) (N := 8) (k := (prodTerms D E).length)
    (by have := massL_nonneg (prodTerms D E); linarith) (by norm_num) (by omega)
  have hK := C0_K
  obtain ⟨m0, m1, m2, m3⟩ := growMany_good (-400) (by norm_num) 8 Tth.P (Tth.grow (-400) (by norm_num))
    (prodTerms D E) [] Tth.nil (by simp) (by simpa using t1) (by simp; omega) (by simpa using bB)
    (by simp only [massL_nil] at bA ⊢; nlinarith)
  simp only [M]; rw [mulL_eq]
  refine ⟨m0, m1, by rw [m2, t2]; simp, ?_, ?_⟩
  · simp only [massL_nil, zero_add] at m3 bA; have := mul_self_nonneg C0; linarith
  · have := growMany_len [] (prodTerms D E); simp at this; omega

/-- A sum of two such expansions (the first carrying the invariant). -/
theorem add_facts (Tth : ExpansionTheory) {A B : List _root_.F64} (hPA : Tth.P A)
    (hA : ∀ x ∈ A, Good (-400) x) (hB : ∀ x ∈ B, Good (-400) x) {X : ℚ}
    (hm : massL A + massL B ≤ X) (hX : 2 * X + 1 ≤ Kmax) (hl : A.length + B.length ≤ 32) :
    Tth.P (addL A B) ∧ (∀ x ∈ addL A B, Good (-400) x) ∧ esumL (addL A B) = esumL A + esumL B ∧
      massL (addL A B) ≤ 2 * X + 1 ∧ (addL A B).length ≤ A.length + B.length := by
  have h0 : 0 ≤ massL A + massL B := by have := massL_nonneg A; have := massL_nonneg B; linarith
  obtain ⟨bA, bB⟩ := budget2 (M := massL A + massL B) (N := A.length + B.length) (k := B.length) h0
    hl (by omega)
  obtain ⟨m0, m1, m2, m3⟩ := growMany_good (-400) (by norm_num) (A.length + B.length) Tth.P
    (Tth.grow (-400) (by norm_num)) B A hPA hA hB le_rfl (by exact_mod_cast bB) (by
      have : (↑(A.length + B.length) : ℚ) = (A.length : ℚ) + B.length := by push_cast; ring
      linarith)
  have e : addL A B = growMany A B := rfl
  rw [e]
  refine ⟨m0, m1, m2, by linarith, growMany_len A B⟩

/-- **`disc`'s exact branch**: on the domain (inputs on the `2⁻²⁰⁰` grid, within `2¹⁰⁰`), with
the expansion theory, the expansion `disc` forms has the sign of `|p − c|² − r²`. -/
theorem disc_model (Tth : ExpansionTheory) (px py cx cy r : _root_.F64)
    (hpx : Good (-200) px ∧ |px.val| ≤ C0) (hpy : Good (-200) py ∧ |py.val| ≤ C0)
    (hcx : Good (-200) cx ∧ |cx.val| ≤ C0) (hcy : Good (-200) cy ∧ |cy.val| ≤ C0)
    (hr : Good (-200) r ∧ |r.val| ≤ C0) :
    signL (addL (addL (mulL (diffL px cx) (diffL px cx)) (mulL (diffL py cy) (diffL py cy)))
      (negL (mulL (ofL r) (ofL r)))) =
      sgnQ ((px.val - cx.val) ^ 2 + (py.val - cy.val) ^ 2 - r.val ^ 2) := by
  obtain ⟨x1, x2, x3, x4⟩ := diff_facts hpx hcx
  obtain ⟨y1, y2, y3, y4⟩ := diff_facts hpy hcy
  obtain ⟨r1, r2, r3, r4⟩ := ofL_good hr.1
  have hC := C0_ge
  have hK := C0_K
  obtain ⟨a0, a1, a2, a3, a4⟩ := mul_facts Tth x1 x1 x3 x3 x4 x4
  obtain ⟨b0, b1, b2, b3, b4⟩ := mul_facts Tth y1 y1 y3 y3 y4 y4
  obtain ⟨c0, c1, c2, c3, c4⟩ := mul_facts Tth r1 r1 (by rw [r3]; linarith [hr.2])
    (by rw [r3]; linarith [hr.2]) (by omega) (by omega)
  obtain ⟨n1, n2, n3, n4⟩ := negL_good c1
  obtain ⟨s0, s1, s2, s3, s4⟩ := add_facts Tth a0 a1 b1 (X := 42 * (C0 * C0) + 2) (by linarith)
    (by nlinarith) (by omega)
  obtain ⟨t0, t1, t2, t3, t4⟩ := add_facts Tth s0 s1 n1 (X := 2 * (42 * (C0 * C0) + 2) + 1 + (21 * (C0 * C0) + 1))
    (by rw [n3]; linarith) (by nlinarith) (by omega)
  rw [Tth.sign _ t0 (fun x h => (t1 x h).1), t2, s2, n2, a2, b2, c2, x2, y2, r2]
  congr 1; ring

end forge_sheet_core
