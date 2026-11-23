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

end forge_sheet_core
