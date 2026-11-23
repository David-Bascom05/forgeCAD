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
abbrev uq : ℚ := 2 ^ (-53 : ℤ)
abbrev ηq : ℚ := 2 ^ (-1075 : ℤ)

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
    |(IEEE754.round IEEE754.binary64 zs r).val - r| ≤ uq * |r| + ηq := F64.Ops.round_err' h

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
  have hu : uq ≤ 1 := zpow_le_one_of_nonpos₀ (by norm_num) (by norm_num)
  have hη : ηq ≤ 1 := zpow_le_one_of_nonpos₀ (by norm_num) (by norm_num)
  have hu0 : 0 ≤ uq := by positivity
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
      have hu0 : (0 : ℚ) ≤ uq := by positivity
      have hη0 : (0 : ℚ) ≤ ηq := by positivity
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

theorem growMany_good (L : ℤ) (hL : -1074 ≤ L) (N : ℕ) :
    ∀ (xs out : List _root_.F64), (∀ x ∈ out, Good L x) → (∀ x ∈ xs, Good L x) →
      out.length + xs.length ≤ N → (N : ℚ) * xs.length * (8 * uq) ≤ 1 →
      (massL out + massL xs) * (1 + 8 * N * xs.length * uq) + 6 * N * xs.length * ηq ≤ Kmax →
      (∀ x ∈ growMany out xs, Good L x) ∧ esumL (growMany out xs) = esumL out + esumL xs ∧
        massL (growMany out xs) ≤
          (massL out + massL xs) * (1 + 8 * N * xs.length * uq) + 6 * N * xs.length * ηq
  | [], out, ho, _, _, _, _ => by simp [growMany]; exact ho
  | x :: rest, out, ho, hx, hl, hk, hB => by
      simp only [List.length_cons, Nat.cast_add, Nat.cast_one, massL_cons, esumL_cons] at hl hk hB ⊢
      have hxg := hx x (by simp)
      have hrest : ∀ y ∈ rest, Good L y := fun y h => hx y (by simp [h])
      have hu0 : (0 : ℚ) ≤ uq := by positivity
      have hη0 : (0 : ℚ) ≤ ηq := by positivity
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
      obtain ⟨h1, h2, h3⟩ := growMany_good L hL N rest (growL out x) g1 hrest hlen hk' hB'
      simp only [growMany, List.foldl_cons] at h1 h2 h3 ⊢
      refine ⟨h1, by rw [h2, g2]; ring, ?_⟩
      have e : 6 * (N : ℚ) * (rest.length + 1) * ηq = 6 * N * rest.length * ηq + 6 * N * ηq := by ring
      linarith

end forge_sheet_core
