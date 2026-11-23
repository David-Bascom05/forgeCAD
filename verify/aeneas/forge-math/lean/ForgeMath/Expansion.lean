/-
Copyright (c) 2026 ForgeCAD contributors. Released under the Mozilla Public License 2.0.
-/
import ForgeMath.Refine

/-!
# The shipped expansion arithmetic, as list functions

`sum_into` (Shewchuk's Fast-Expansion-Sum with zero elimination) and `scale_into`
(Scale-Expansion with zero elimination) write into a caller's buffer and return the length used.
Here they are list functions over the ieee models' `twoSum` and `twoProduct SPLITTER`
(`mergeL`, `growL`, `sumIntoL`, `scaleL`), and the translated Rust is shown to return `ok` with
the buffer's first `n` entries equal to the list (`sum_into_spec`, `scale_into_spec`). The
expansion theory (verify/ieee754's `Expansion.lean`) is stated against these lists.

"Nonzero" is the Rust's `x != 0.0`: `!FP.eq x (+0)`.
-/

open Aeneas Aeneas.Std Result ControlFlow
open forge_math

namespace ForgeMath.Expansion

open IEEE754 hiding F64

/-- `x != 0.0` -/
abbrev nz (x : F64) : Bool := !IEEE754.FP.eq x (.zero false)

/-- `next_component`'s merge by magnitude: `e`'s head when `f` is empty or `|e₀| < |f₀|`,
else `f`'s head. -/
def mergeL : List F64 → List F64 → List F64
  | [], f => f
  | e, [] => e
  | a :: e, b :: f =>
    if IEEE754.FP.lt (IEEE754.FP.abs a) (IEEE754.FP.abs b) then a :: mergeL e (b :: f)
    else b :: mergeL (a :: e) f
termination_by e f => e.length + f.length

lemma mergeL_nil_right (e : List F64) : mergeL e [] = e := by cases e <;> simp [mergeL]

lemma mergeL_length (e f : List F64) : (mergeL e f).length = e.length + f.length := by
  induction e generalizing f with
  | nil => simp [mergeL]
  | cons a e ih =>
    induction f with
    | nil => simp [mergeL]
    | cons b f ihf =>
      rw [mergeL]; split <;> simp_all <;> omega

/-- The twoSum chain: each nonzero error in order, and the last sum. -/
def growL (q : F64) : List F64 → List F64 × F64
  | [] => ([], q)
  | x :: xs =>
    let t := IEEE754.twoSum q x
    let r := growL t.1 xs
    ((if nz t.2 then [t.2] else []) ++ r.1, r.2)

/-- `sum_into` on lists. -/
def sumIntoL (e f : List F64) : List F64 :=
  match mergeL e f with
  | [] => []
  | x :: xs =>
    let r := growL x xs
    if nz r.2 || r.1.isEmpty then r.1 ++ [r.2] else r.1

lemma ofBits0 : F64.Ops.ofBits 0 = .zero false := by decide

/-- **`next_component`**: the head of the merge of what is left, and the cursors past it. -/
theorem next_spec (e f : Slice F64) (ei fi : Usize) (he : ei.val ≤ e.val.length) (hf : fi.val ≤ f.val.length)
    (ht : ei.val + fi.val < e.val.length + f.val.length) :
    predicates.next_component e f ei fi ⦃ r =>
      mergeL (e.val.drop ei.val) (f.val.drop fi.val) = r.1 :: mergeL (e.val.drop r.2.1.val) (f.val.drop r.2.2.val) ∧
      r.2.1.val + r.2.2.val = ei.val + fi.val + 1 ∧ r.2.1.val ≤ e.val.length ∧ r.2.2.val ≤ f.val.length ⦄ := by
  unfold predicates.next_component
  simp only [Slice.len, core.f64.F64.abs, bind_ok, F64.Ops.lt_def]
  split
  · have c1 : f.val.length ≤ fi.val := by scalar_tac
    have c2 : ei.val < e.val.length := by omega
    step*
    rw [List.drop_of_length_le c1, mergeL_nil_right, mergeL_nil_right, List.drop_eq_getElem_cons c2]
    refine ⟨by simp_all, by omega, by omega, hf⟩
  · have c1' : fi.val < f.val.length := by scalar_tac
    split
    · have c2 : ei.val < e.val.length := by scalar_tac
      rw [List.drop_eq_getElem_cons c2, List.drop_eq_getElem_cons c1']
      step*
      · rw [mergeL, if_pos (by simp_all), ← List.drop_eq_getElem_cons c1']
        refine ⟨by simp_all, by omega, by omega, hf⟩
      · rw [mergeL, if_neg (by simp_all), ← List.drop_eq_getElem_cons c2]
        refine ⟨by simp_all, by omega, he, by omega⟩
    · have c2 : ¬ ei.val < e.val.length := by scalar_tac
      step*
      rw [List.drop_of_length_le (by omega : e.val.length ≤ ei.val), List.drop_eq_getElem_cons c1']
      refine ⟨by simp_all [mergeL], by omega, he, by omega⟩

attribute [local step] next_spec

lemma take_set_succ {l : List F64} {n : ℕ} (x : F64) (h : n < l.length) :
    (l.set n x).take (n + 1) = l.take n ++ [x] := by
  rw [List.take_add_one, List.getElem?_set_self h, List.take_set_of_le (le_refl _)]; rfl

lemma take_one_set {l : List F64} (x : F64) (h : 0 < l.length) : (l.set 0 x).take 1 = [x] := by
  cases l with
  | nil => simp at h
  | cons a l => simp

lemma eq0_nz (x : F64) : F64.Ops.ne x (F64.Ops.ofBits 0) = nz x := by
  simp [F64.Ops.ne_def', ofBits0, nz]

lemma sum_loop_unfold (e f h : Slice F64) (n total : Usize) (q : F64) (ei fi k : Usize) :
    predicates.sum_into_loop e f h n total q ei fi k =
      Bind.bind (predicates.sum_into_loop.body e f total h n q ei fi k) (fun r =>
      match r with
      | cont (h1, n1, q1, ei1, fi1, k1) => predicates.sum_into_loop e f h1 n1 total q1 ei1 fi1 k1
      | done z => ok z) := by
  unfold predicates.sum_into_loop; rw [loop]; simp only []; congr 1; funext r; cases r <;> rfl

/-- **`sum_into`'s loop**: it appends the chain's nonzero errors to `h`'s first `n` entries and
returns the last sum. -/
theorem sum_loop_spec (e f : Slice F64) (total : Usize) (ht : total.val = e.val.length + f.val.length) :
    ∀ (m : ℕ) (h : Slice F64) (n : Usize) (q : F64) (ei fi k : Usize),
      total.val - k.val = m → ei.val + fi.val = k.val → ei.val ≤ e.val.length → fi.val ≤ f.val.length →
      k.val ≤ total.val → n.val < k.val → total.val ≤ h.val.length →
      predicates.sum_into_loop e f h n total q ei fi k ⦃ r =>
        r.1.val.take r.2.1.val = h.val.take n.val ++ (growL q (mergeL (e.val.drop ei.val) (f.val.drop fi.val))).1 ∧
        r.2.2 = (growL q (mergeL (e.val.drop ei.val) (f.val.drop fi.val))).2 ∧
        r.1.val.length = h.val.length ∧ r.2.1.val ≤ n.val + (total.val - k.val) ⦄ := by
  intro m
  induction m with
  | zero =>
    intro h n q ei fi k hm hk he hf hkt hn hh
    rw [sum_loop_unfold]; unfold predicates.sum_into_loop.body
    have hkt' : ¬ k < total := by scalar_tac
    simp only [hkt', ite_false]
    have e1 : e.val.drop ei.val = [] := List.drop_of_length_le (by omega)
    have e2 : f.val.drop fi.val = [] := List.drop_of_length_le (by omega)
    simp [e1, e2, mergeL, growL]
  | succ m ih =>
    intro h n q ei fi k hm hk he hf hkt hn hh
    rw [sum_loop_unfold]; unfold predicates.sum_into_loop.body
    have hkt' : k < total := by scalar_tac
    simp only [hkt', ite_true, two_sum_eq, bind_ok, eq0_nz, Std.bind_tc_eq, Std.bind_assoc]
    step*
    obtain ⟨x, ei2, fi2⟩ := x
    simp only at x_post x_post1 x_post2 x_post3 ⊢
    rw [x_post]; simp only [growL, Std.uncurry_apply_pair]
    generalize twoSum q x = t
    obtain ⟨s, err⟩ := t
    simp only [Std.uncurry_apply_pair]
    by_cases hnz : nz err
    · simp only [hnz, ite_true]
      have hb : n.val < h.val.length := by scalar_tac
      step*
      refine ⟨?_, by assumption, by simp_all, by scalar_tac⟩
      simp_all [take_set_succ _ hb]
    · simp only [hnz, Bool.false_eq_true, ite_false]
      step*

/-- **`sum_into`**: with room for `e.len() + f.len()` components, it does not panic, and the
buffer's first `n` entries are `sumIntoL e f`. -/
theorem sum_into_spec (e f h : Slice F64) (hh : e.val.length + f.val.length ≤ h.val.length) :
    predicates.sum_into e f h ⦃ r =>
      r.2.val.take r.1.val = sumIntoL e.val f.val ∧ r.2.val.length = h.val.length ⦄ := by
  unfold predicates.sum_into
  simp only [Slice.len, eq0_nz]
  step*
  · have e0 : e.val = [] := List.eq_nil_of_length_eq_zero (by scalar_tac)
    have f0 : f.val = [] := List.eq_nil_of_length_eq_zero (by scalar_tac)
    simp [e0, f0, sumIntoL, mergeL]
  · simp only [List.drop_zero] at q_post
    have ht : total.val = e.val.length + f.val.length := by simp_all
    apply WP.spec_bind (sum_loop_spec e f _ ht _ h 0#usize q ei fi 1#usize rfl
      (by simp_all) q_post2 q_post3 (by scalar_tac) (by simp) (by scalar_tac))
    rintro ⟨h1, n, q1⟩ ⟨a1, a2, a3, a4⟩
    simp only [Std.uncurry_apply_pair] at a1 a2 a3 a4 ⊢
    simp at a1
    have hlen : (growL q (mergeL (List.drop ↑ei e.val) (List.drop ↑fi f.val))).1.length = n.val := by
      rw [← a1, List.length_take]; scalar_tac
    unfold sumIntoL
    rw [q_post]
    simp only
    rw [← a2, ← a1]
    by_cases hq : nz q1
    · simp only [hq, ite_true, Bool.true_or]
      step*
      refine ⟨?_, by simp_all⟩
      simp_all [take_set_succ _ (by scalar_tac : n.val < h1.val.length)]
    · simp only [hq, Bool.false_eq_true, ite_false, Bool.false_or]
      split
      · step*
        have : (List.take n.val h1.val).isEmpty = true := by
          rw [List.isEmpty_iff, ← List.length_eq_zero_iff, List.length_take]; scalar_tac
        have hb0 : 0 < h1.val.length := by scalar_tac
        refine ⟨?_, by simp_all⟩
        simp_all [take_one_set _ hb0]
      · have : (List.take n.val h1.val).isEmpty = false := by
          rw [Bool.eq_false_iff, ne_eq, List.isEmpty_iff, ← List.length_eq_zero_iff, List.length_take]
          scalar_tac
        simp_all

/-! ## `scale_into` -/

/-- Scale-Expansion's steps after the first: for each component `x`, `x·b = p1 + p0`, then
`q + p0 = s + err`, then `p1 + s = s2 + err2`, keeping the nonzero errors in order. -/
def scaleStep (b q : F64) : List F64 → List F64 × F64
  | [] => ([], q)
  | x :: xs =>
    let p := IEEE754.twoProduct splitter x b
    let t := IEEE754.twoSum q p.2
    let u := IEEE754.twoSum p.1 t.1
    let r := scaleStep b u.1 xs
    ((if nz t.2 then [t.2] else []) ++ (if nz u.2 then [u.2] else []) ++ r.1, r.2)

/-- `scale_into` on lists. -/
def scaleL (e : List F64) (b : F64) : List F64 :=
  match e with
  | [] => []
  | x :: xs =>
    let p := IEEE754.twoProduct splitter x b
    let r := scaleStep b p.1 xs
    let hs := (if nz p.2 then [p.2] else []) ++ r.1
    if nz r.2 || hs.isEmpty then hs ++ [r.2] else hs

lemma scale_loop_unfold (e : Slice F64) (b : F64) (h : Slice F64) (n : Usize) (q : F64) (i : Usize) :
    predicates.scale_into_loop e b h n q i =
      Bind.bind (predicates.scale_into_loop.body e b h n q i) (fun r =>
      match r with
      | cont (h1, n1, q1, i1) => predicates.scale_into_loop e b h1 n1 q1 i1
      | done z => ok z) := by
  unfold predicates.scale_into_loop; rw [loop]; simp only []; congr 1; funext r; cases r <;> rfl

/-- **`scale_into`'s loop.** -/
theorem scale_loop_spec (e : Slice F64) (b : F64) :
    ∀ (m : ℕ) (h : Slice F64) (n : Usize) (q : F64) (i : Usize),
      e.val.length - i.val = m → 1 ≤ i.val → i.val ≤ e.val.length → n.val + 1 ≤ 2 * i.val →
      2 * e.val.length ≤ h.val.length →
      predicates.scale_into_loop e b h n q i ⦃ r =>
        r.1.val.take r.2.1.val = h.val.take n.val ++ (scaleStep b q (e.val.drop i.val)).1 ∧
        r.2.2 = (scaleStep b q (e.val.drop i.val)).2 ∧
        r.1.val.length = h.val.length ∧ r.2.1.val + 1 ≤ 2 * e.val.length ⦄ := by
  intro m
  induction m with
  | zero =>
    intro h n q i hm hi1 hi hn hh
    rw [scale_loop_unfold]; unfold predicates.scale_into_loop.body
    simp only [Slice.len]
    have c : ¬ i < Usize.ofNatCore e.val.length (by scalar_tac) := by scalar_tac
    simp only [c, ite_false]
    have e1 : e.val.drop i.val = [] := List.drop_of_length_le (by omega)
    simp [e1, scaleStep]; omega
  | succ m ih =>
    intro h n q i hm hi1 hi hn hh
    rw [scale_loop_unfold]; unfold predicates.scale_into_loop.body
    simp only [Slice.len]
    have c : i < Usize.ofNatCore e.val.length (by scalar_tac) := by scalar_tac
    have hil : i.val < e.val.length := by omega
    simp only [c, ite_true, two_sum_eq, two_product_eq, eq0_nz, Std.bind_tc_eq, Std.bind_assoc, bind_ok]
    rw [List.drop_eq_getElem_cons hil]
    simp only [scaleStep]
    step*
    subst x_post
    generalize twoProduct splitter (↑e)[↑i] b = p
    obtain ⟨p1, p0⟩ := p
    simp only [Std.uncurry_apply_pair]
    generalize twoSum q p0 = t
    obtain ⟨s, err⟩ := t
    simp only [Std.uncurry_apply_pair]
    generalize twoSum p1 s = u
    obtain ⟨s2, err2⟩ := u
    simp only [Std.uncurry_apply_pair]
    have hb1 : n.val < h.val.length := by omega
    have hb2 : n.val + 1 < h.val.length := by omega
    by_cases h1 : nz err <;> by_cases h2 : nz err2 <;>
      simp only [h1, h2, ite_true, Bool.false_eq_true, ite_false] <;> step* <;>
      refine ⟨?_, by simp_all, by simp_all, by scalar_tac⟩ <;> simp_all [take_set_succ]

/-- **`scale_into`**: with room for `2·e.len()` components, it does not panic, and the buffer's
first `n` entries are `scaleL e b`. -/
theorem scale_into_spec (e : Slice F64) (b : F64) (h : Slice F64) (hh : 2 * e.val.length ≤ h.val.length) :
    predicates.scale_into e b h ⦃ r =>
      r.2.val.take r.1.val = scaleL e.val b ∧ r.2.val.length = h.val.length ⦄ := by
  unfold predicates.scale_into
  simp only [core.slice.Slice.is_empty, two_product_eq, eq0_nz, bind_ok]
  by_cases he : e.val.length = 0
  · have e0 : e.val = [] := List.eq_nil_of_length_eq_zero he
    simp [Slice.length, e0, scaleL]
  · have he' : ¬ e.length = 0 := he
    simp only [he', decide_false, Bool.false_eq_true, ite_false]
    obtain ⟨x, xs, hx⟩ : ∃ x xs, e.val = x :: xs := by
      cases hc : e.val with
      | nil => exact absurd hc (by simpa using he)
      | cons x xs => exact ⟨x, xs, rfl⟩
    step*
    have hfx : fl = x := by simp [fl_post, hx]
    subst hfx
    clear fl_post
    unfold scaleL
    rw [hx]
    simp only
    generalize hp : twoProduct splitter fl b = p
    obtain ⟨q, lo⟩ := p
    simp only [Std.uncurry_apply_pair]
    have hb0 : 0 < h.val.length := by omega
    have loop : ∀ (h1 : Slice F64) (n : Usize), n.val ≤ 1 → h1.val.length = h.val.length →
        predicates.scale_into_loop e b h1 n q 1#usize ⦃ r =>
          r.1.val.take r.2.1.val = h1.val.take n.val ++ (scaleStep b q xs).1 ∧
          r.2.2 = (scaleStep b q xs).2 ∧ r.1.val.length = h.val.length ∧ r.2.1.val + 1 ≤ 2 * e.val.length ⦄ := by
      intro h1 n hn hl
      apply WP.spec_mono (scale_loop_spec e b _ h1 n q 1#usize rfl (by simp) (by simp; omega) (by simp; omega)
        (by omega))
      rintro r ⟨a1, a2, a3, a4⟩
      simp only [hx] at a1 a2
      exact ⟨by simpa using a1, a2, by omega, a4⟩
    by_cases hl : nz lo
    · simp only [hl, ite_true]
      step*
      all_goals
        have hb : n1.val < h2.val.length := by omega
        have hL : List.take n1.val h2.val = [lo] ++ (scaleStep b q xs).1 := by
          simp_all [take_one_set _ hb0]
      · refine ⟨?_, by simp_all⟩
        rw [h3_post, Slice.set_val_eq, n2_post, take_set_succ _ hb, hL]; simp_all
      · exfalso; simp_all
      · refine ⟨?_, by simp_all⟩
        rw [hL]; simp_all
        intro h0; simp [h0] at hb0
    · simp only [hl, Bool.false_eq_true, ite_false]
      step*
      all_goals
        have hb : n1.val < h2.val.length := by omega
        have hL : List.take n1.val h2.val = (scaleStep b q xs).1 := by simp_all
      · refine ⟨?_, by simp_all⟩
        rw [h3_post, Slice.set_val_eq, n2_post, take_set_succ _ hb, hL]; simp_all
      · have hS : (scaleStep b q xs).1 = [] := by rw [← hL]; simp_all
        have hb' : 0 < h2.val.length := by omega
        refine ⟨?_, by simp_all⟩
        rw [hS, h3_post, Slice.set_val_eq, n2_post, ← q1_post]
        simp [take_one_set _ hb']
      · have hn0 : n1.val ≠ 0 := fun h0 => ‹¬n1 = 0#usize› (by apply UScalar.eq_of_val_eq; simp [h0])
        have hS : (scaleStep b q xs).1.isEmpty = false := by
          rw [← hL, Bool.eq_false_iff, ne_eq, List.isEmpty_iff, ← List.length_eq_zero_iff, List.length_take]
          omega
        refine ⟨?_, by simp_all⟩
        rw [hL]; simp_all

end ForgeMath.Expansion
