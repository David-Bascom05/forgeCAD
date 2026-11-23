import ForgeSheetCore.RefineDisc
import IEEE754.Expansion

/-!
# The expansion theory, discharged: `disc` is exact with no assumption

verify/ieee754's IEEE754/Expansion.lean proves, for every format, Shewchuk's invariant
(`Nonoverlapping`), that Grow-Expansion keeps it (`grow_nonoverlapping`, his Theorem 10), and
that an expansion's sign is its topmost nonzero component's (`NO_sign`). Here those give the
`ExpansionTheory` that `disc_spec` assumed:
* `P e`: `e`'s components are finite and their values nonoverlapping;
* `growL` (Grow-Expansion with zero elimination) is the chain with its zeros dropped
  (`growL_eq`), a sublist, so it keeps `P` whenever every step is finite and exact, which the
  domain's budget gives (`chain_ok`);
* `signL` (the scan for the first nonzero component from the top) is the sign of the sum.

`disc_exact`: the shipped `exact::disc` decides `sign(|p − c|² − r²)` exactly on the domain.
-/

namespace forge_sheet_core

open Aeneas Aeneas.Std Result
open IEEE754 hiding F64
open IEEE754.FP

/-- Shewchuk's invariant on the models. -/
def ExpP (e : List _root_.F64) : Prop := (∀ x ∈ e, x.isFinite) ∧ Nonoverlapping (e.map FP.val)

/-! ## Grow-Expansion with zero elimination is the chain, zeros dropped -/

theorem fold_growStep_eq : ∀ (e : List _root_.F64) (q : _root_.F64) (out : List _root_.F64),
    e.foldl growStep (q, out) =
      ((growChain q e).2, out ++ (growChain q e).1.filter (fun t => F64.Ops.ne t Z))
  | [], q, out => by simp [growChain]
  | y :: ys, q, out => by
    rw [List.foldl_cons]
    have : growStep (q, out) y = ((twoSum q y).1,
        out ++ [(twoSum q y).2].filter (fun t => F64.Ops.ne t Z)) := by
      simp only [growStep, List.filter_cons, List.filter_nil]; split <;> simp_all
    rw [this, fold_growStep_eq ys]
    simp only [growChain, List.filter_cons, List.append_assoc]
    split <;> simp

theorem growL_eq (e : List _root_.F64) (b : _root_.F64) :
    growL e b = ((growChain b e).1 ++ [(growChain b e).2]).filter (fun t => F64.Ops.ne t Z) := by
  unfold growL
  rw [fold_growStep_eq]
  simp only [List.nil_append, List.filter_append, List.filter_cons, List.filter_nil]
  split <;> simp_all

/-! ## On the domain's budget, every step of the chain is finite and exact -/

theorem chain_ok (L : ℤ) :
    ∀ (rest : List _root_.F64) (q : _root_.F64) (out : List _root_.F64),
      Good L q → (∀ x ∈ out, Good L x) → (∀ x ∈ rest, Good L x) →
      (rest.length : ℚ) * (8 * uq) ≤ 1 →
      (|q.val| + massL out + massL rest) * (1 + 4 * rest.length * uq) + 3 * rest.length * ηq ≤
        Kmax → (hL : -1074 ≤ L) → ChainOk q rest
  | [], q, _, hq, _, _, _, _, _ => hq.1
  | x :: xs, q, out, hq, ho, hr, hk, hB, hL => by
      have hx := hr x (by simp)
      have hxs : ∀ y ∈ xs, Good L y := fun y h => hr y (by simp [h])
      simp only [List.length_cons, Nat.cast_add, Nat.cast_one, massL_cons] at hk hB
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
      have hout' : ∀ y ∈ (growStep (q, out) x).2, Good L y := by
        intro y hy
        simp only [growStep, ← hp] at hy
        split at hy
        · rcases List.mem_append.mp hy with h | h
          · exact ho y h
          · simp at h; rw [h]; exact ⟨f2, g2⟩
        · exact ho y hy
      have msq : |(growStep (q, out) x).1.val| + massL (growStep (q, out) x).2 ≤
          (1 + 2 * uq) * (|q.val| + |x.val|) + 2 * ηq + massL out := by
        simp only [growStep, ← hp]
        split
        · simp only [massL_append, massL_cons, massL_nil]; linarith
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
      have rest := chain_ok L xs (growStep (q, out) x).1 (growStep (q, out) x).2
        (by simp only [growStep, ← hp]; exact ⟨f1, g1⟩) hout' hxs hk' hB' hL
      exact ⟨hq.1, hx.1, f1, f2, es, by simpa [growStep, ← hp] using rest⟩

/-! ## The sign scan is the sign of the sum -/

theorem sgnM_val {x : _root_.F64} (hx : x.isFinite) : sgnM x = sgnQ x.val :=
  sgnM_of_sign hx Iff.rfl Iff.rfl

theorem sgnQ_congr {p q : ℚ} (h1 : 0 < p ↔ 0 < q) (h2 : p < 0 ↔ q < 0) : sgnQ p = sgnQ q := by
  unfold sgnQ; simp only [h1, h2]

theorem sgnQ_ne {q : ℚ} (h : q ≠ 0) : F64.Ops.eq (sgnQ q) Z = false := by
  unfold sgnQ
  rcases lt_or_gt_of_ne h with hn | hp
  · rw [if_neg (by linarith), if_pos hn]; decide +kernel
  · rw [if_pos hp]; decide +kernel

theorem signL_append (l : List _root_.F64) (x : _root_.F64) :
    signL (l ++ [x]) = l.reverse.foldl signF (sgnM x) := by
  unfold signL
  rw [List.reverse_append, List.reverse_singleton, List.singleton_append, List.foldl_cons]
  congr 1

theorem signL_exp : ∀ (e : List _root_.F64), ExpP e → signL e = sgnQ (esumL e) := by
  intro e
  induction e using List.reverseRecOn with
  | nil => intro _; unfold signL sgnQ; simp [esumL]
  | append_singleton l x ih =>
    intro ⟨hf, k, K, hN⟩
    have hx : x.isFinite := hf x (by simp)
    rw [signL_append, sgnM_val hx]
    by_cases h0 : x.val = 0
    · have hz : sgnQ x.val = Z := by unfold sgnQ; simp [h0]
      rw [hz]
      have hl : ExpP l := ⟨fun y hy => hf y (by simp [hy]),
        Nonoverlapping.sublist ⟨k, K, hN⟩ (by simp)⟩
      have := ih hl
      unfold signL at this
      rw [this]; congr 1; simp [esumL, h0]
    · rw [foldl_signF_stop _ _ (sgnQ_ne h0)]
      have hN' : NO k K (l.map FP.val ++ x.val :: []) := by simpa using hN
      obtain ⟨s1, s2⟩ := NO_sign hN' h0 (by simp)
      apply sgnQ_congr
      · have e : esumL (l ++ [x]) = (l.map FP.val ++ x.val :: []).sum := by simp [esumL]
        rw [e, s1]
      · have e : esumL (l ++ [x]) = (l.map FP.val ++ x.val :: []).sum := by simp [esumL]
        rw [e, s2]

/-! ## The instance, and `disc` with no assumption -/

theorem grow_exp (L : ℤ) (hL : -1074 ≤ L) : GrowInv L ExpP := by
  intro e b hP he hb hn hB
  have hok := chain_ok L e b [] hb (by simp) he hn (by simpa using hB) hL
  obtain ⟨c1, c2⟩ := grow_chain_finite b e hok
  have hno := grow_nonoverlapping hok hP.2
  rw [growL_eq]
  refine ⟨fun x hx => ?_, Nonoverlapping.sublist hno ?_⟩
  · have := (List.mem_filter.mp hx).1
    rcases List.mem_append.mp this with h | h
    · exact c1 x h
    · simp at h; rw [h]; exact c2
  · have h := (List.filter_sublist (p := fun t => F64.Ops.ne t Z)
      (l := (growChain b e).1 ++ [(growChain b e).2])).map FP.val
    rw [List.map_append, List.map_singleton] at h; exact h

/-- **The expansion theory**, proven (verify/ieee754's Expansion.lean). -/
def expansionTheory : ExpansionTheory where
  P := ExpP
  nil := ⟨by simp, 0, 0, le_refl _⟩
  grow := grow_exp
  sign := fun e hP _ => signL_exp e hP

/-- **`exact::disc` is exact**: on the domain (inputs multiples of `2⁻²⁰⁰`, at most `2¹⁰⁰`), the
shipped function returns `+1`, `−1` or `+0` as `|p − c|² − r²` of the input doubles is positive,
negative or zero. No hypothesis beyond the domain. -/
theorem disc_exact (px py cx cy r : _root_.F64)
    (hpx : Good (-200) px ∧ |px.val| ≤ C0) (hpy : Good (-200) py ∧ |py.val| ≤ C0)
    (hcx : Good (-200) cx ∧ |cx.val| ≤ C0) (hcy : Good (-200) cy ∧ |cy.val| ≤ C0)
    (hr : Good (-200) r ∧ |r.val| ≤ C0) :
    exact.disc px py cx cy r ⦃ s => s = sgnQ (discQ px py cx cy r) ⦄ :=
  disc_spec expansionTheory px py cx cy r hpx hpy hcx hcy hr

end forge_sheet_core
