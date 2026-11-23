import ForgeSheetCore.Funs
import AeneasF64.Lemmas
import IEEE754.TwoSum
import IEEE754.TwoProduct
import ForgeMath.Refine
open Aeneas Aeneas.Std Result

/-!
# `exact.rs`: the translated expansions are the list models

Each function of forge-sheet-core's exact.rs is shown equal to a pure function on lists of
doubles (`growL`, `addL`, `negL`, `mulL`, `signL`), built from verify/ieee754's `twoSum` and
`twoProduct` (`two_sum_eq`, `two_product_eq`), for lengths within `usize`. The float facts are
then stated on the models (ExactModel.lean).
-/

namespace forge_sheet_core

open IEEE754 hiding F64

/-- Zero, as the code writes it. -/
abbrev Z : _root_.F64 := F64.Ops.ofBits 0

theorem two_sum_eq (a b : _root_.F64) : exact.two_sum a b = ok (twoSum a b) := by
  simp only [exact.two_sum, lift, Std.bind_ok, F64.Ops.add_def, F64.Ops.sub_def]; rfl

theorem two_sum_spec (a b : _root_.F64) : exact.two_sum a b ⦃ p => p = twoSum a b ⦄ := by
  rw [two_sum_eq]; simp

/-- One step of Grow-Expansion: carry `q`, keep the error unless it is zero. -/
def growStep (st : _root_.F64 × List _root_.F64) (x : _root_.F64) : _root_.F64 × List _root_.F64 :=
  let p := twoSum st.1 x
  (p.1, if F64.Ops.ne p.2 Z then st.2 ++ [p.2] else st.2)

def growL (e : List _root_.F64) (b : _root_.F64) : List _root_.F64 :=
  let r := e.foldl growStep (b, [])
  if F64.Ops.ne r.1 Z then r.2 ++ [r.1] else r.2

theorem growStep_len (st : _root_.F64 × List _root_.F64) (x : _root_.F64) :
    (growStep st x).2.length ≤ st.2.length + 1 := by
  unfold growStep; dsimp only; split <;> simp

theorem foldl_growStep_len (l : List _root_.F64) (st : _root_.F64 × List _root_.F64) :
    (l.foldl growStep st).2.length ≤ st.2.length + l.length := by
  induction l generalizing st with
  | nil => simp
  | cons x xs ih =>
      simp only [List.foldl_cons, List.length_cons]
      have := ih (growStep st x); have := growStep_len st x; omega

theorem growL_len (e : List _root_.F64) (b : _root_.F64) : (growL e b).length ≤ e.length + 1 := by
  unfold growL
  have := foldl_growStep_len e (b, [])
  simp only [List.length_nil, zero_add] at this
  dsimp only; split <;> simp <;> omega

theorem grow_loop_spec (e : Slice _root_.F64) (q : _root_.F64) (out : alloc.vec.Vec _root_.F64)
    (i : Usize) (hi : i.val ≤ e.val.length) (hb : out.val.length + e.val.length < Usize.max) :
    exact.grow_loop e q out i ⦃ r =>
      (r.1, r.2.val) = (e.val.drop i.val).foldl growStep (q, out.val) ∧
      r.2.val.length ≤ out.val.length + (e.val.length - i.val) ⦄ := by
  unfold exact.grow_loop
  apply loop.spec_decr_nat
    (fun (st : _root_.F64 × alloc.vec.Vec _root_.F64 × Usize) => e.val.length - st.2.2.val)
    (fun st => i.val ≤ st.2.2.val ∧ st.2.2.val ≤ e.val.length ∧
      (e.val.drop st.2.2.val).foldl growStep (st.1, st.2.1.val) =
        (e.val.drop i.val).foldl growStep (q, out.val) ∧
      st.2.1.val.length ≤ out.val.length + (st.2.2.val - i.val))
  · rintro ⟨q1, o1, j⟩ ⟨hij, hj, hf, hl⟩
    unfold exact.grow_loop.body
    simp only at hij hj hf hl ⊢
    split
    · rename_i h
      have hjl : j.val < e.val.length := by scalar_tac
      step with Slice.index_usize_spec e j hjl as ⟨x, hx⟩
      have hdrop : e.val.drop j.val = x :: e.val.drop (j.val + 1) := by
        rw [hx]; exact List.drop_eq_getElem_cons hjl
      apply WP.spec_bind (two_sum_spec q1 x)
      rintro ⟨s, lo⟩ hts
      show WP.spec (do
        let out1 ← (if F64.Ops.ne lo (F64.Ops.ofBits 0) = true then o1.push lo else ok o1)
        let i2 ← j + 1#usize
        ok (ControlFlow.cont (s, out1, i2))) _
      by_cases hz : F64.Ops.ne lo Z = true
      · rw [if_pos hz]
        have hpl : o1.val.length < Usize.max := by omega
        step with alloc.vec.Vec.push_spec o1 _ hpl as ⟨o2, ho2⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨by omega, by omega, ?_, ?_, by omega⟩
        · have hz' : FP.eq lo Z = false := by simpa [F64.Ops.ne_def'] using hz
          rw [← hf, hdrop, List.foldl_cons, ho2, hj1v]; simp [growStep, ← hts, hz']
        · rw [ho2, List.length_append, List.length_singleton, hj1v]; omega
      · rw [if_neg hz]
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by omega
        refine ⟨by omega, by omega, ?_, by rw [hj1v]; omega, by omega⟩
        have hz' : FP.eq lo Z = true := by simpa [F64.Ops.ne_def'] using hz
        rw [← hf, hdrop, List.foldl_cons, hj1v]; simp [growStep, ← hts, hz']
    · rename_i h
      have : j.val = e.val.length := by scalar_tac
      simp only [WP.spec_ok]
      rw [this, List.drop_length] at hf
      simp only [List.foldl_nil] at hf
      refine ⟨hf, by omega⟩
  · exact ⟨le_refl _, hi, rfl, by simp⟩

/-- **`grow` is `growL`**, for lengths within `usize`. -/
theorem grow_spec (e : Slice _root_.F64) (b : _root_.F64) (hb : e.val.length + 1 < Usize.max) :
    exact.grow e b ⦃ r => r.val = growL e.val b ⦄ := by
  unfold exact.grow
  apply WP.spec_bind (grow_loop_spec e b (alloc.vec.Vec.new _root_.F64) 0#usize (by simp)
    (by simp; omega))
  rintro ⟨q, o⟩ ⟨hr, hl⟩
  have hr' : (q, o.val) = e.val.foldl growStep (b, []) := by simpa [alloc.vec.Vec.new] using hr
  have hl' : o.val.length ≤ e.val.length := by simpa [alloc.vec.Vec.new] using hl
  have hr1 : q = (e.val.foldl growStep (b, [])).1 := by rw [← hr']
  have hr2 : o.val = (e.val.foldl growStep (b, [])).2 := by rw [← hr']
  unfold growL
  show WP.spec (if F64.Ops.ne q (F64.Ops.ofBits 0) = true then o.push q else ok o) _
  by_cases hz : F64.Ops.ne q Z = true
  · rw [if_pos hz]
    step with alloc.vec.Vec.push_spec o q (by omega) as ⟨o2, ho2⟩
    rw [ho2, hr2, if_pos (by rw [← hr1]; exact hz), hr1]
  · rw [if_neg hz]
    simp only [WP.spec_ok]
    rw [hr2, if_neg (by rw [← hr1]; exact hz)]

/-! ## The other functions -/

theorem SPLITTER_eq : exact.SPLITTER = ForgeMath.splitter := by unfold exact.SPLITTER; rfl

theorem split_eq (a : _root_.F64) : exact.split a = ok (IEEE754.split ForgeMath.splitter a) := by
  simp [exact.split, IEEE754.split, lift, SPLITTER_eq]

theorem two_product_eq (a b : _root_.F64) :
    exact.two_product a b = ok (twoProduct ForgeMath.splitter a b) := by
  simp [exact.two_product, twoProduct, split_eq, IEEE754.split, lift]

@[simp] theorem deref_val {T : Type} (v : alloc.vec.Vec T) : (alloc.vec.Vec.deref v).val = v.val := by
  simp [alloc.vec.Vec.deref]

def ofL (x : _root_.F64) : List _root_.F64 := if F64.Ops.ne x Z then [x] else []

theorem exp_of_spec (x : _root_.F64) : exact.exp_of x ⦃ r => r.val = ofL x ⦄ := by
  unfold exact.exp_of ofL
  by_cases h : F64.Ops.ne x Z = true
  · rw [if_pos h, if_pos h]
    step with alloc.vec.Vec.push_spec (alloc.vec.Vec.new _root_.F64) x (by simp; scalar_tac) as ⟨v, hv⟩
    simpa [alloc.vec.Vec.new] using hv
  · rw [if_neg h, if_neg h]; simp [alloc.vec.Vec.new]

theorem ofL_len (x : _root_.F64) : (ofL x).length ≤ 1 := by unfold ofL; split <;> simp

def diffL (a b : _root_.F64) : List _root_.F64 := growL (ofL a) (FP.neg b)

theorem exp_diff_spec (a b : _root_.F64) : exact.exp_diff a b ⦃ r => r.val = diffL a b ⦄ := by
  unfold exact.exp_diff
  step with exp_of_spec a as ⟨e, he⟩
  simp only [lift, Std.bind_ok, F64.Ops.neg_def]
  have : (alloc.vec.Vec.deref e).val.length + 1 < Usize.max := by
    simp [alloc.vec.Vec.deref, he]; have := ofL_len a; scalar_tac
  step with grow_spec (alloc.vec.Vec.deref e) (FP.neg b) this as ⟨r, hr⟩
  rw [hr]; simp [diffL, alloc.vec.Vec.deref, he]

def negL (e : List _root_.F64) : List _root_.F64 := e.map FP.neg

theorem exp_neg_spec (e : Slice _root_.F64) (hb : e.val.length < Usize.max) :
    exact.exp_neg e ⦃ r => r.val = negL e.val ⦄ := by
  unfold exact.exp_neg exact.exp_neg_loop
  apply loop.spec_decr_nat
    (fun (st : alloc.vec.Vec _root_.F64 × Usize) => e.val.length - st.2.val)
    (fun st => st.2.val ≤ e.val.length ∧ st.1.val = negL (e.val.take st.2.val))
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold exact.exp_neg_loop.body
    simp only at hj ho ⊢
    split
    · rename_i h
      have hjl : j.val < e.val.length := by scalar_tac
      step with Slice.index_usize_spec e j hjl as ⟨x, hx⟩
      simp only [lift, Std.bind_ok, F64.Ops.neg_def]
      have hol : o.val.length < Usize.max := by rw [ho]; simp [negL]; omega
      step with alloc.vec.Vec.push_spec o (FP.neg x) hol as ⟨o2, ho2⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by omega
      refine ⟨by omega, ?_, by omega⟩
      subst hx
      rw [ho2, ho, hj1v, List.take_succ_eq_append_getElem hjl]
      simp only [negL, List.map_append, List.map_cons, List.map_nil]
    · rename_i h
      have : j.val = e.val.length := by scalar_tac
      simp only [WP.spec_ok]; rw [ho, this, List.take_length]
  · exact ⟨by simp, by simp [negL, alloc.vec.Vec.new]⟩

def addL (e o : List _root_.F64) : List _root_.F64 := o.foldl growL e

theorem addL_len (e o : List _root_.F64) : (addL e o).length ≤ e.length + o.length := by
  unfold addL
  induction o generalizing e with
  | nil => simp
  | cons x xs ih =>
      simp only [List.foldl_cons, List.length_cons]
      have := ih (growL e x); have := growL_len e x; omega

theorem exp_add_spec (e o : Slice _root_.F64) (hb : e.val.length + o.val.length + 1 < Usize.max) :
    exact.exp_add e o ⦃ r => r.val = addL e.val o.val ⦄ := by
  unfold exact.exp_add
  step as ⟨s, hs⟩
  all_goals try (intro x _; rfl)
  unfold exact.exp_add_loop
  apply loop.spec_decr_nat
    (fun (st : alloc.vec.Vec _root_.F64 × Usize) => o.val.length - st.2.val)
    (fun st => st.2.val ≤ o.val.length ∧ st.1.val = addL e.val (o.val.take st.2.val))
  · rintro ⟨w, j⟩ ⟨hj, hw⟩
    unfold exact.exp_add_loop.body
    simp only at hj hw ⊢
    split
    · rename_i h
      have hjl : j.val < o.val.length := by scalar_tac
      step with Slice.index_usize_spec o j hjl as ⟨x, hx⟩
      have hwl : (alloc.vec.Vec.deref w).val.length + 1 < Usize.max := by
        rw [deref_val, hw]
        have := addL_len e.val (o.val.take j.val)
        rw [List.length_take] at this
        have : min j.val o.val.length ≤ o.val.length := min_le_right _ _
        omega
      step with grow_spec (alloc.vec.Vec.deref w) x hwl as ⟨w2, hw2⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by omega
      refine ⟨by omega, ?_, by omega⟩
      rw [hw2, hj1v, List.take_succ_eq_append_getElem hjl, ← hx]
      simp [addL, alloc.vec.Vec.deref, hw, List.foldl_append]
    · rename_i h
      have : j.val = o.val.length := by scalar_tac
      simp only [WP.spec_ok]; rw [hw, this, List.take_length]
  · refine ⟨by simp, ?_⟩
    rw [hs]; simp [addL]; rfl

/-- One row of the product: `a` times each of `o`'s components, error then product grown in. -/
def mulStep (a : _root_.F64) (out : List _root_.F64) (b : _root_.F64) : List _root_.F64 :=
  growL (growL out (twoProduct ForgeMath.splitter a b).2) (twoProduct ForgeMath.splitter a b).1

def mulRow (a : _root_.F64) (o out : List _root_.F64) : List _root_.F64 := o.foldl (mulStep a) out

def mulL (e o : List _root_.F64) : List _root_.F64 := e.foldl (fun out a => mulRow a o out) []

theorem mulStep_len (a b : _root_.F64) (out : List _root_.F64) :
    (mulStep a out b).length ≤ out.length + 2 := by
  unfold mulStep
  have := growL_len out (twoProduct ForgeMath.splitter a b).2
  have := growL_len (growL out (twoProduct ForgeMath.splitter a b).2) (twoProduct ForgeMath.splitter a b).1
  omega

theorem mulRow_len (a : _root_.F64) (o out : List _root_.F64) :
    (mulRow a o out).length ≤ out.length + 2 * o.length := by
  unfold mulRow
  induction o generalizing out with
  | nil => simp
  | cons b bs ih =>
      simp only [List.foldl_cons, List.length_cons]
      have := ih (mulStep a out b); have := mulStep_len a b out; omega

theorem mulL_len_aux (e o out : List _root_.F64) :
    (e.foldl (fun out a => mulRow a o out) out).length ≤ out.length + 2 * e.length * o.length := by
  induction e generalizing out with
  | nil => simp
  | cons a rest ih =>
      simp only [List.foldl_cons, List.length_cons]
      have := ih (mulRow a o out); have := mulRow_len a o out
      have : 2 * (rest.length + 1) * o.length = 2 * rest.length * o.length + 2 * o.length := by ring
      omega

theorem row_spec (e o : Slice _root_.F64) (out : alloc.vec.Vec _root_.F64) (i : Usize)
    (hi : i.val < e.val.length) (hb : out.val.length + 2 * o.val.length + 2 < Usize.max) :
    exact.exp_mul_loop0_loop0 e o out i 0#usize ⦃ r =>
      r.val = mulRow e.val[i.val] o.val out.val ⦄ := by
  unfold exact.exp_mul_loop0_loop0
  apply loop.spec_decr_nat
    (fun (st : alloc.vec.Vec _root_.F64 × Usize) => o.val.length - st.2.val)
    (fun st => st.2.val ≤ o.val.length ∧ st.1.val = mulRow e.val[i.val] (o.val.take st.2.val) out.val)
  · rintro ⟨w, j⟩ ⟨hj, hw⟩
    unfold exact.exp_mul_loop0_loop0.body
    simp only at hj hw ⊢
    split
    · rename_i h
      have hjl : j.val < o.val.length := by scalar_tac
      step with Slice.index_usize_spec e i hi as ⟨a, ha⟩
      step with Slice.index_usize_spec o j hjl as ⟨b, hb'⟩
      rw [two_product_eq]
      simp only [Std.bind_ok]
      generalize hp : twoProduct ForgeMath.splitter a b = p
      obtain ⟨x, y⟩ := p
      show WP.spec (do
        let out1 ← exact.grow (alloc.vec.Vec.deref w) y
        let out2 ← exact.grow (alloc.vec.Vec.deref out1) x
        let j1 ← j + 1#usize
        ok (ControlFlow.cont (out2, j1))) _
      have hwlen : w.val.length ≤ out.val.length + 2 * j.val := by
        rw [hw]; have := mulRow_len e.val[i.val] (o.val.take j.val) out.val
        rw [List.length_take] at this; have : min j.val o.val.length ≤ j.val := min_le_left _ _; omega
      have g1 : (alloc.vec.Vec.deref w).val.length + 1 < Usize.max := by rw [deref_val]; omega
      step with grow_spec (alloc.vec.Vec.deref w) y g1 as ⟨w1, hw1⟩
      have g2 : (alloc.vec.Vec.deref w1).val.length + 1 < Usize.max := by
        rw [deref_val, hw1, deref_val]; have := growL_len w.val y
        omega
      step with grow_spec (alloc.vec.Vec.deref w1) x g2 as ⟨w2, hw2⟩
      step as ⟨j1, hj1⟩
      have hj1v : j1.val = j.val + 1 := by omega
      refine ⟨by omega, ?_, by omega⟩
      rw [hw2, hj1v, List.take_succ_eq_append_getElem hjl]
      simp only [deref_val, hw1, hw, mulRow, List.foldl_append, List.foldl_cons, List.foldl_nil]
      rw [← ha, ← hb']; simp only [mulStep, hp]
    · rename_i h
      have : j.val = o.val.length := by scalar_tac
      simp only [WP.spec_ok]; rw [hw, this, List.take_length]
  · exact ⟨by simp, by simp [mulRow]⟩

theorem exp_mul_spec (e o : Slice _root_.F64) (he : e.val.length ≤ 2 ^ 14) (ho : o.val.length ≤ 2 ^ 14) :
    exact.exp_mul e o ⦃ r => r.val = mulL e.val o.val ⦄ := by
  have hmax : 2 ^ 31 ≤ Usize.max := by
    have : U32.max ≤ Usize.max := by scalar_tac
    have : U32.max = 2 ^ 32 - 1 := by simp [U32.max_eq]
    omega
  unfold exact.exp_mul exact.exp_mul_loop0
  apply loop.spec_decr_nat
    (fun (st : alloc.vec.Vec _root_.F64 × Usize) => e.val.length - st.2.val)
    (fun st => st.2.val ≤ e.val.length ∧ st.1.val = mulL (e.val.take st.2.val) o.val)
  · rintro ⟨w, i⟩ ⟨hi, hw⟩
    unfold exact.exp_mul_loop0.body
    simp only at hi hw ⊢
    split
    · rename_i h
      have hil : i.val < e.val.length := by scalar_tac
      have hwlen : w.val.length ≤ 2 * i.val * o.val.length := by
        rw [hw]; have h1 := mulL_len_aux (e.val.take i.val) o.val []
        rw [List.length_take, min_eq_left hil.le] at h1
        simpa [mulL] using h1
      have hb : w.val.length + 2 * o.val.length + 2 < Usize.max := by
        have : 2 * i.val * o.val.length ≤ 2 * 2 ^ 14 * 2 ^ 14 := by
          apply Nat.mul_le_mul (Nat.mul_le_mul_left _ (by omega)) ho
        omega
      step with row_spec e o w i hil hb as ⟨w1, hw1⟩
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = i.val + 1 := by omega
      refine ⟨by omega, ?_, by omega⟩
      rw [hw1, hi1v, hw]
      simp only [mulL]
      rw [List.take_succ_eq_append_getElem hil, List.foldl_append]; rfl
    · rename_i h
      have : i.val = e.val.length := by scalar_tac
      simp only [WP.spec_ok]; rw [hw, this, List.take_length]
  · exact ⟨by simp, by simp [mulL, alloc.vec.Vec.new]⟩

/-! ## The sign -/

def sgnM (x : _root_.F64) : _root_.F64 :=
  if F64.Ops.gt x Z then F64.Ops.ofBits 4607182418800017408
  else if F64.Ops.lt x Z then F64.Ops.ofBits 13830554455654793216 else Z

theorem sgn_eq (x : _root_.F64) : exact.sgn x = ok (sgnM x) := by
  unfold exact.sgn sgnM; split_ifs <;> rfl

/-- The sign scan: the first nonzero sign from the top. -/
def signF (s x : _root_.F64) : _root_.F64 := if F64.Ops.eq s Z then sgnM x else s

def signL (e : List _root_.F64) : _root_.F64 := e.reverse.foldl signF Z

theorem foldl_signF_stop (l : List _root_.F64) (s : _root_.F64) (h : F64.Ops.eq s Z = false) :
    l.foldl signF s = s := by
  induction l with
  | nil => rfl
  | cons x xs ih => simp only [List.foldl_cons, signF, h]; exact ih

theorem exp_sign_spec (e : Slice _root_.F64) : exact.exp_sign e ⦃ r => r = signL e.val ⦄ := by
  unfold exact.exp_sign exact.exp_sign_loop
  apply loop.spec_decr_nat
    (fun (st : _root_.F64 × Usize) => st.2.val)
    (fun st => st.2.val ≤ e.val.length ∧
      (e.val.take st.2.val).reverse.foldl signF st.1 = signL e.val)
  · rintro ⟨s, i⟩ ⟨hi, hs⟩
    unfold exact.exp_sign_loop.body
    simp only at hi hs ⊢
    split
    · rename_i h
      have hi0 : 0 < i.val := by scalar_tac
      split
      · rename_i hz
        step as ⟨i1, hi1⟩
        have hi1l : i1.val < e.val.length := by omega
        step with Slice.index_usize_spec e i1 hi1l as ⟨x, hx⟩
        rw [sgn_eq]; simp only [Std.bind_ok, WP.spec_ok]
        refine ⟨by omega, ?_, by omega⟩
        rw [← hs]
        have ht : e.val.take i.val = e.val.take i1.val ++ [x] := by
          rw [hx, show i.val = i1.val + 1 by omega, List.take_succ_eq_append_getElem hi1l]
        rw [ht, List.reverse_append, List.reverse_singleton, List.singleton_append, List.foldl_cons]
        have hz' : FP.eq s Z = true := by simpa [F64.Ops.eq_def'] using hz
        simp [signF, hz']
      · rename_i hz
        simp only [WP.spec_ok]
        rw [← hs, foldl_signF_stop _ _ (by simpa using hz)]
    · rename_i h
      have hi0 : i.val = 0 := by scalar_tac
      simp only [WP.spec_ok]
      rw [← hs, hi0]; simp
  · refine ⟨by simp, ?_⟩
    simp [signL, List.take_length]

end forge_sheet_core
