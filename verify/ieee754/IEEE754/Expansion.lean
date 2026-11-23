/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import IEEE754.TwoSum
import IEEE754.TwoProduct

/-!
# Floating-point expansions (Shewchuk 1997): nonoverlapping, the sign, Grow-Expansion

An expansion is a list of floats, least first, whose exact sum is the value it stands for.
Shewchuk's invariant is *nonoverlapping*: each nonzero component's lowest set bit is above every
set bit of the components below it. Stated with `Mult` (a multiple of `2^k`), bottom up:

  `NO k K l`: `l`'s first component is a multiple of `2^k` and below some `2^k'` (`k ≤ k'`),
  the rest is `NO k' K`, and the last such bound is at most `2^K`.

Zeros fit anywhere (a zero is a multiple of every `2^k`), so zero elimination (dropping
components) keeps the invariant: `NO_sublist`, for any sublist.

What is proven here, for every format `f` (round to nearest; ties play no part):
* `NO_sum`, `NO_abs_sum`: a nonoverlapping list starting at `2^k` sums below `2^K`; so
  `NO_top_dominates`: everything below a nonzero component sums to less than it in magnitude, and
  `NO_sign`: **the sign of an expansion is the sign of its topmost nonzero component** (any zeros
  above it allowed), the fact `estimate`-style sign scans rely on.
* `two_sum_facts`: TwoSum's error is at most each operand in magnitude, the pair keeps any grid
  both operands are on, and the error is below a power of two the sum is a multiple of.
* `grow_step`, `grow_chain`: **Grow-Expansion keeps nonoverlapping** (Shewchuk's Theorem 10):
  adding a float `q` to a nonoverlapping `e` by a chain of TwoSums (`growChain`) gives errors and
  a final sum that are, in order, nonoverlapping, with the same exact sum (`grow_chain_sum`).

Hypotheses: every operation's result is finite (`ChainOk`: each step's sum and error finite,
and TwoSum exact there, which is `two_sum_exact` for `3 ≤ f.prec`, stated per step so a caller
can discharge it from its own bounds).
-/

set_option linter.unusedSectionVars false

namespace IEEE754

open FP

variable {f : Format}

/-! ## Nonoverlapping, bottom up -/

/-- `NO k K l`: the list `l` (least first) is nonoverlapping from `2^k` up to `2^K`. -/
def NO : ℤ → ℤ → List ℚ → Prop
  | k, K, [] => k ≤ K
  | k, K, x :: l => Mult x k ∧ ∃ k', k ≤ k' ∧ |x| < 2 ^ k' ∧ NO k' K l

/-- An expansion's values are nonoverlapping between some bounds. -/
def Nonoverlapping (l : List ℚ) : Prop := ∃ k K, NO k K l

@[simp] lemma NO_nil {k K : ℤ} : NO k K [] ↔ k ≤ K := Iff.rfl

@[simp] lemma NO_cons {k K : ℤ} {x : ℚ} {l : List ℚ} :
    NO k K (x :: l) ↔ Mult x k ∧ ∃ k', k ≤ k' ∧ |x| < 2 ^ k' ∧ NO k' K l := Iff.rfl

lemma NO_le : ∀ {k K : ℤ} {l : List ℚ}, NO k K l → k ≤ K
  | _, _, [], h => h
  | _, _, _ :: _, ⟨_, _, hk, _, h⟩ => le_trans hk (NO_le h)

lemma NO_top_mono : ∀ {k K K' : ℤ} {l : List ℚ}, NO k K l → K ≤ K' → NO k K' l
  | _, _, _, [], h, hK => le_trans h hK
  | _, _, _, _ :: _, ⟨hx, k', hk, hlt, h⟩, hK => ⟨hx, k', hk, hlt, NO_top_mono h hK⟩

lemma NO_start_mono {k k' K : ℤ} {l : List ℚ} (h : NO k' K l) (hk : k ≤ k') : NO k K l := by
  cases l with
  | nil => exact le_trans hk h
  | cons x l =>
    obtain ⟨hx, k'', hk', hlt, h⟩ := h
    exact ⟨mult_of_le hx hk, k'', le_trans hk hk', hlt, h⟩

lemma NO_append : ∀ {k m K : ℤ} {l1 l2 : List ℚ}, NO k m l1 → NO m K l2 → NO k K (l1 ++ l2)
  | _, _, _, [], _, h1, h2 => NO_start_mono h2 h1
  | _, _, _, _ :: _, _, ⟨hx, k', hk, hlt, h⟩, h2 => ⟨hx, k', hk, hlt, NO_append h h2⟩

lemma NO_append_inv : ∀ {k K : ℤ} {l1 l2 : List ℚ}, NO k K (l1 ++ l2) →
    ∃ m, NO k m l1 ∧ NO m K l2
  | k, _, [], _, h => ⟨k, le_refl k, h⟩
  | _, _, _ :: _, _, ⟨hx, k', hk, hlt, h⟩ => by
    obtain ⟨m, h1, h2⟩ := NO_append_inv h
    exact ⟨m, ⟨hx, k', hk, hlt, h1⟩, h2⟩

/-- **Dropping components keeps nonoverlapping** (zero elimination is a case). -/
lemma NO_sublist {l l' : List ℚ} (hs : List.Sublist l' l) : ∀ {k K : ℤ}, NO k K l → NO k K l' := by
  induction hs with
  | slnil => exact fun h => h
  | cons a _ ih =>
    intro k K h
    obtain ⟨_, k', hk, _, h⟩ := h
    exact NO_start_mono (ih h) hk
  | cons_cons a _ ih =>
    intro k K h
    obtain ⟨hx, k', hk, hlt, h⟩ := h
    exact ⟨hx, k', hk, hlt, ih h⟩

lemma Nonoverlapping.sublist {l l' : List ℚ} (h : Nonoverlapping l) (hs : List.Sublist l' l) :
    Nonoverlapping l' := by
  obtain ⟨k, K, h⟩ := h; exact ⟨k, K, NO_sublist hs h⟩

lemma mult_abs {x : ℚ} {k : ℤ} (h : Mult x k) : Mult |x| k := by
  obtain ⟨n, rfl⟩ := h
  exact ⟨|n|, by rw [abs_mul, abs_of_pos (two_zpow_pos (K := ℚ) k)]; push_cast; rfl⟩

/-- A nonzero multiple of `2^k` is at least `2^k`. -/
lemma le_abs_of_mult {x : ℚ} {k : ℤ} (h : Mult x k) (hx : x ≠ 0) : 2 ^ k ≤ |x| := by
  by_contra hc; exact hx (mult_small h (lt_of_not_ge hc))

/-! ## The sum and the sign -/

/-- Anything below `2^k`, plus a list nonoverlapping from `2^k` to `2^K`, is below `2^K`. -/
lemma NO_sum : ∀ {k K : ℤ} {l : List ℚ} {P : ℚ}, NO k K l → |P| < 2 ^ k → |P + l.sum| < 2 ^ K
  | _, _, [], _, h, hP => by simpa using lt_of_lt_of_le hP (two_zpow_le h)
  | k, _, x :: l, P, ⟨hx, k', hk, hlt, h⟩, hP => by
    have hM : Mult ((2 : ℚ) ^ k' - 2 ^ k) k := mult_sub (mult_pow hk) (mult_pow (le_refl k))
    have hxle : |x| ≤ 2 ^ k' - 2 ^ k := mult_le_of_lt (mult_abs hx) hM (by linarith)
    have : |P + x| < 2 ^ k' := lt_of_le_of_lt (abs_add_le P x) (by linarith)
    have := NO_sum h this
    rw [List.sum_cons, ← add_assoc]; exact this

lemma NO_abs_sum {k K : ℤ} {l : List ℚ} (h : NO k K l) : |l.sum| < 2 ^ K := by
  simpa using NO_sum (P := 0) h (by simpa using two_zpow_pos (K := ℚ) k)

/-- **Below a nonzero component, the expansion sums to less than it** in magnitude. -/
lemma NO_top_dominates {k K : ℤ} {l z : List ℚ} {x : ℚ} (h : NO k K (l ++ x :: z))
    (hx : x ≠ 0) : |l.sum| < |x| := by
  obtain ⟨m, h1, h2⟩ := NO_append_inv h
  exact lt_of_lt_of_le (NO_abs_sum h1) (le_abs_of_mult h2.1 hx)

lemma sign_of_dominant {x s : ℚ} (hx : x ≠ 0) (h : |s| < |x|) :
    (0 < s + x ↔ 0 < x) ∧ (s + x < 0 ↔ x < 0) := by
  have hs := abs_lt.mp (lt_of_lt_of_le h (le_refl _))
  rcases lt_or_gt_of_ne hx with hn | hp
  · rw [abs_of_neg hn] at h; have := abs_lt.mp h
    constructor <;> constructor <;> intro <;> linarith
  · rw [abs_of_pos hp] at h; have := abs_lt.mp h
    constructor <;> constructor <;> intro <;> linarith

/-- **The sign of an expansion is its topmost nonzero component's** (zeros above it allowed). -/
theorem NO_sign {k K : ℤ} {l z : List ℚ} {x : ℚ} (h : NO k K (l ++ x :: z)) (hx : x ≠ 0)
    (hz : ∀ y ∈ z, y = 0) :
    (0 < (l ++ x :: z).sum ↔ 0 < x) ∧ ((l ++ x :: z).sum < 0 ↔ x < 0) := by
  have hz0 : z.sum = 0 := List.sum_eq_zero hz
  have e : (l ++ x :: z).sum = l.sum + x := by simp [List.sum_append, hz0]
  rw [e]; exact sign_of_dominant hx (NO_top_dominates h hx)

/-! ## TwoSum's error -/

/-- Rounding keeps any grid: a finite rounding of a multiple of `2^m` is one. -/
lemma round_mult_grid {zs : Bool} {r : ℚ} {m : ℤ} (hm : Mult r m)
    (hfin : (round f zs r).isFinite) : Mult (round f zs r).val m := by
  by_cases hr : r = 0
  · subst hr; simp [round, val]; exact mult_zero m
  by_cases hq : m ≤ qOf f r
  · exact mult_of_le (round_grid hr hfin).1 hq
  push Not at hq
  by_cases hk : f.qmin ≤ m
  · have hb : |r| < 2 ^ (m + f.prec - 1) := by
      by_contra hc; push Not at hc; have := qOf_ge (f := f) hc; omega
    have hrep := rep_of_mult (f := f) hm hk (by
      rw [← zpow_natCast, ← two_zpow_add]
      exact le_trans hb.le (two_zpow_le (by omega))) (abs_lt_of_round_finite hfin)
    rw [(hrep zs).2]; exact hm
  · push Not at hk
    obtain ⟨e, he, hme, _⟩ := mult_val hfin
    exact mult_of_le hme (by omega)

/-- **TwoSum's error** `t = a + b − s` (`s = a ⊕ b`): at most each operand in magnitude; on any
grid both operands are on, with the sum; below a power of two the sum is a multiple of. -/
theorem two_sum_facts {a b : FP f} (ha : a.isFinite) (hb : b.isFinite)
    (hs : (add a b).isFinite) (hsum : (twoSum a b).1.val + (twoSum a b).2.val = a.val + b.val) :
    |(twoSum a b).2.val| ≤ |b.val| ∧ |(twoSum a b).2.val| ≤ |a.val| ∧
      (∀ m : ℤ, Mult a.val m → Mult b.val m →
        Mult (twoSum a b).1.val m ∧ Mult (twoSum a b).2.val m) ∧
      ∃ u : ℤ, Mult (twoSum a b).1.val u ∧ |(twoSum a b).2.val| < 2 ^ u := by
  have h1 : (twoSum a b).1 = add a b := rfl
  have hr : add a b = round f (a.sign && b.sign) (a.val + b.val) := add_eq_round ha hb
  have ht : (twoSum a b).2.val = (a.val + b.val) - (add a b).val := by rw [← h1]; linarith
  rw [hr] at hs
  have hna := round_nearest hs ha
  have hnb := round_nearest hs hb
  rw [← hr] at hna hnb hs
  refine ⟨?_, ?_, ?_, ?_⟩
  · rw [ht, abs_sub_comm]; calc _ ≤ _ := hna
      _ = |b.val| := by rw [show a.val - (a.val + b.val) = -b.val by ring, abs_neg]
  · rw [ht, abs_sub_comm]; calc _ ≤ _ := hnb
      _ = |a.val| := by rw [show b.val - (a.val + b.val) = -a.val by ring, abs_neg]
  · intro m hma hmb
    have hab := mult_add hma hmb
    have hsm : Mult (add a b).val m := by rw [hr] at hs ⊢; exact round_mult_grid hab hs
    exact ⟨by rw [h1]; exact hsm, by rw [ht]; exact mult_sub hab hsm⟩
  · by_cases h0 : a.val + b.val = 0
    · have hs0 : (add a b).val = 0 := by rw [hr, h0]; simp [round, val]
      obtain ⟨e, _, hme, _⟩ := mult_val hs
      exact ⟨e, by rw [h1]; exact hme, by rw [ht, h0, hs0]; simpa using two_zpow_pos (K := ℚ) e⟩
    · rw [hr] at hs
      obtain ⟨g1, g2⟩ := round_grid h0 hs
      refine ⟨qOf f (a.val + b.val), by rw [h1, hr]; exact g1, ?_⟩
      rw [ht, abs_sub_comm, hr]
      exact lt_of_le_of_lt g2 (two_zpow_lt (by omega))

/-! ## Grow-Expansion -/

lemma zpow_min_gt {x : ℚ} {a b : ℤ} (ha : x < 2 ^ a) (hb : x < 2 ^ b) : x < 2 ^ min a b := by
  rcases le_total a b with h | h
  · rw [min_eq_left h]; exact ha
  · rw [min_eq_right h]; exact hb

/-- **One step of Grow-Expansion keeps nonoverlapping.** The state: the running sum `q`, a
multiple of `2^a`, and the errors so far `hs`, nonoverlapping up to `2^min(a, k)`; the next
component `y`, a multiple of `2^k` below `2^k'` (`k ≤ k'`). After `(s, t) = TwoSum(q, y)`, `s`
is a multiple of some `2^a'` and `hs ++ [t]` is nonoverlapping up to `2^min(a', k')`. -/
theorem grow_step {q y : FP f} (hq : q.isFinite) (hy : y.isFinite) (hs : (add q y).isFinite)
    (hsum : (twoSum q y).1.val + (twoSum q y).2.val = q.val + y.val)
    {a k k' k0 : ℤ} {hs' : List ℚ} (hQ : Mult q.val a) (hY : Mult y.val k) (hkk : k ≤ k')
    (hyl : |y.val| < 2 ^ k') (hH : NO k0 (min a k) hs') :
    ∃ a', Mult (twoSum q y).1.val a' ∧ NO k0 (min a' k') (hs' ++ [(twoSum q y).2.val]) := by
  obtain ⟨tb, -, tm, u, hsu, htu⟩ := two_sum_facts hq hy hs hsum
  set m := min a k
  obtain ⟨gs, gt⟩ := tm m (mult_of_le hQ (min_le_left a k)) (mult_of_le hY (min_le_right a k))
  refine ⟨max u m, mult_max hsu gs, NO_append hH ⟨gt, min (max u m) k', ?_, ?_, ?_⟩⟩
  · exact le_min (le_max_right u m) (le_trans (min_le_right a k) hkk)
  · exact zpow_min_gt (lt_of_lt_of_le htu (two_zpow_le (le_max_left u m)))
      (lt_of_le_of_lt tb hyl)
  · exact le_refl _

/-- Grow-Expansion without zero elimination: the errors, least first, then the final sum.
(With elimination the output is a sublist of this one, and keeps everything by `NO_sublist`.) -/
def growChain : FP f → List (FP f) → List (FP f) × FP f
  | q, [] => ([], q)
  | q, y :: ys =>
    let p := twoSum q y
    let r := growChain p.1 ys
    (p.2 :: r.1, r.2)

/-- Every step of the chain finite and exact. -/
def ChainOk : FP f → List (FP f) → Prop
  | q, [] => q.isFinite
  | q, y :: ys => q.isFinite ∧ y.isFinite ∧ (add q y).isFinite ∧ (twoSum q y).2.isFinite ∧
      (twoSum q y).1.val + (twoSum q y).2.val = q.val + y.val ∧ ChainOk (twoSum q y).1 ys

/-- The chain keeps the exact sum. -/
theorem grow_chain_sum : ∀ (q : FP f) (e : List (FP f)), ChainOk q e →
    ((growChain q e).1.map val).sum + (growChain q e).2.val = q.val + (e.map val).sum
  | q, [], _ => by simp [growChain]
  | q, y :: ys, ⟨_, _, _, _, hsum, h⟩ => by
    have := grow_chain_sum (twoSum q y).1 ys h
    simp only [growChain, List.map_cons, List.sum_cons] at this ⊢
    linarith

/-- The chain's components are finite. -/
theorem grow_chain_finite : ∀ (q : FP f) (e : List (FP f)), ChainOk q e →
    (∀ x ∈ (growChain q e).1, x.isFinite) ∧ (growChain q e).2.isFinite
  | q, [], h => by simp [growChain]; exact h
  | q, y :: ys, ⟨_, _, hs, ht, _, h⟩ => by
    obtain ⟨h1, h2⟩ := grow_chain_finite (twoSum q y).1 ys h
    simp only [growChain, List.mem_cons]
    exact ⟨fun x hx => by rcases hx with rfl | hx; exacts [ht, h1 x hx], h2⟩

lemma exists_top (x : ℚ) (m : ℤ) : ∃ K : ℤ, m ≤ K ∧ |x| < 2 ^ K := by
  obtain ⟨n, hn⟩ := pow_unbounded_of_one_lt |x| (by norm_num : (1 : ℚ) < 2)
  refine ⟨max m n, le_max_left _ _, lt_of_lt_of_le hn ?_⟩
  have : (2 : ℚ) ^ n = 2 ^ (n : ℤ) := (zpow_natCast 2 n).symm
  rw [this]; exact two_zpow_le (le_max_right _ _)

/-- **Grow-Expansion keeps nonoverlapping** (Shewchuk's Theorem 10), from any state of the
induction: errors `hs` so far, running sum `q`. -/
theorem grow_chain_NO : ∀ (e : List (FP f)) (q : FP f) (a k K k0 : ℤ) (hs : List ℚ),
    ChainOk q e → Mult q.val a → NO k K (e.map val) → NO k0 (min a k) hs →
    ∃ K', NO k0 K' (hs ++ ((growChain q e).1.map val ++ [(growChain q e).2.val]))
  | [], q, a, k, _, k0, hs, _, hQ, _, hH => by
    obtain ⟨K', hK, hlt⟩ := exists_top q.val (min a k)
    refine ⟨K', ?_⟩
    simp only [growChain, List.map_nil, List.nil_append]
    exact NO_append hH ⟨mult_of_le hQ (min_le_left a k), K', hK, hlt, le_refl K'⟩
  | y :: ys, q, a, k, K, k0, hs, ⟨hq, hy, hsf, _, hsum, h⟩, hQ, hE, hH => by
    obtain ⟨hY, k', hkk, hyl, hE'⟩ := hE
    obtain ⟨a', hQ', hH'⟩ := grow_step hq hy hsf hsum hQ hY hkk hyl hH
    obtain ⟨K', hK'⟩ := grow_chain_NO ys (twoSum q y).1 a' k' K k0 _ h hQ' hE' hH'
    refine ⟨K', ?_⟩
    simp only [growChain, List.map_cons, List.append_assoc, List.cons_append] at hK' ⊢
    simpa using hK'

/-- **Grow-Expansion keeps nonoverlapping**: `q` added to a nonoverlapping `e` gives a
nonoverlapping list of errors then the sum. -/
theorem grow_nonoverlapping {q : FP f} {e : List (FP f)} (h : ChainOk q e)
    (he : Nonoverlapping (e.map val)) :
    Nonoverlapping ((growChain q e).1.map val ++ [(growChain q e).2.val]) := by
  obtain ⟨k, K, hE⟩ := he
  have hq : q.isFinite := by cases e <;> [exact h; exact h.1]
  obtain ⟨a, _, hQ, _⟩ := mult_val hq
  obtain ⟨K', hK'⟩ := grow_chain_NO e q a k K (min a k) [] h hQ hE (le_refl _)
  exact ⟨min a k, K', by simpa using hK'⟩

end IEEE754
