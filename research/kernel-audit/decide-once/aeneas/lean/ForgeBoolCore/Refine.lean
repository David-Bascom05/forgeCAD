import ForgeBoolCore.Funs
import ForgeExact.DecideOnce.Core
open Aeneas Aeneas.Std Result

/-!
# The translated core refines `ForgeExact.DecideOnce.core`: the first lemmas

`ForgeBoolCore/Funs.lean` is Aeneas's translation of kernel/forge-bool-core (charon 9e193f0,
aeneas 2e3d4488). The obligation (decide-once-spec.md §4): under the map `id ↦ cells[id]`, the
translated `core` returns `ForgeExact.DecideOnce.core` of the ledger the cells describe. This file
proves it leaf by leaf, from the covector primitives up; each lemma also says the Rust returns `ok`
(no panic, no overflow, no divergence).

Proven here: `zero_idx` is `Cov.zeroIdx`, `is_facet` is `Cov.isFacet`, `le` is `Cov.le`.
-/

namespace forge_bool_core

deriving instance DecidableEq for Sgn
instance : Inhabited Sgn := ⟨.Zero⟩

/-- A Rust sign as the Lean core's. -/
def toS : Sgn → ForgeExact.DecideOnce.Sgn
  | .Neg => .neg
  | .Zero => .zero
  | .Pos => .pos

theorem toS_inj {a b : Sgn} (h : toS a = toS b) : a = b := by
  cases a <;> cases b <;> simp_all [toS]

theorem toS_zero {a : Sgn} : toS a = .zero ↔ a = .Zero := by
  cases a <;> simp [toS]

/-- A slice of signs as a covector. -/
def covOf (x : Slice Sgn) : ForgeExact.DecideOnce.Cov := x.val.map toS

theorem sgn_eq (a b : Sgn) : Sgn.Insts.CoreCmpPartialEqSgn.eq a b = ok (decide (a = b)) := by
  cases a <;> cases b <;> simp [Sgn.Insts.CoreCmpPartialEqSgn.eq, Sgn.read_discriminant]

theorem sgn_ne (a b : Sgn) :
    core.cmp.PartialEq.ne.trait_default Sgn.Insts.CoreCmpPartialEqSgn a b = ok (decide (a ≠ b)) := by
  cases a <;> cases b <;> simp [core.cmp.PartialEq.ne.trait_default, core.cmp.PartialEq.ne.default,
    Sgn.Insts.CoreCmpPartialEqSgn.eq, Sgn.read_discriminant]

/-- The first zero at or after `i` (the length if none): what the loop computes. -/
def zFrom (l : List Sgn) (i : Nat) : Nat := i + ((l.drop i).idxOf .Zero)

theorem idxOf_map_toS (l : List Sgn) : (l.map toS).idxOf .zero = l.idxOf .Zero := by
  induction l with
  | nil => rfl
  | cons a t ih =>
    cases a <;> simp [toS, ih]

theorem drop_cons (l : List Sgn) (j : Nat) (h : j < l.length) :
    l.drop j = l[j]! :: l.drop (j + 1) := by
  rw [getElem!_pos l j h]; exact List.drop_eq_getElem_cons h

theorem zero_idx_loop_spec (x : Slice Sgn) (i : Usize) (hi : i.val ≤ x.val.length) :
    zero_idx_loop x i ⦃ r => r.val = zFrom x.val i.val ⦄ := by
  unfold zero_idx_loop
  apply loop.spec_decr_nat (fun (j : Usize) => x.val.length - j.val)
    (fun j => j.val ≤ x.val.length ∧ zFrom x.val j.val = zFrom x.val i.val)
  · rintro j ⟨hj, heq⟩
    unfold zero_idx_loop.body
    by_cases hlt : j.val < x.val.length
    · have hlt' : j < Slice.len x := by scalar_tac
      simp only [hlt', ite_true]
      step as ⟨s, hs⟩
      rw [sgn_ne]; simp only [Std.bind_ok]
      have hd := drop_cons x.val j.val hlt
      have hs' : s = x.val[j.val]! := by rw [hs, getElem!_pos x.val j.val hlt]
      by_cases hz : s = Sgn.Zero
      · rw [ite_eq_right (by simp [hz])]
        simp only [WP.spec_ok]
        rw [← heq]; simp only [zFrom]; rw [hd, ← hs', hz]; simp
      · rw [ite_eq_left (by simp [hz])]
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [← heq]; simp only [zFrom]; rw [hd, ← hs', hj1]
        simp [List.idxOf_cons, hz]; omega
    · have : ¬ j < Slice.len x := by scalar_tac
      simp only [this, ite_false, WP.spec_ok]
      rw [← heq, zFrom]
      have : j.val = x.val.length := by omega
      rw [this]; simp
  · exact ⟨hi, rfl⟩

/-- **`zero_idx` is `Cov.zeroIdx`.** -/
theorem zero_idx_spec (x : Slice Sgn) :
    zero_idx x ⦃ r => r.val = (covOf x).zeroIdx ⦄ := by
  unfold zero_idx
  have := zero_idx_loop_spec x 0#usize (by simp)
  apply WP.spec_mono this
  intro r hr
  rw [hr]
  simp [zFrom, covOf, ForgeExact.DecideOnce.Cov.zeroIdx, idxOf_map_toS]

/-! ## `is_facet` -/

theorem count_map_toS (l : List Sgn) : (l.map toS).count .zero = l.count .Zero := by
  induction l with
  | nil => rfl
  | cons a t ih => cases a <;> simp [toS, List.count_cons, ih]

theorem count_le_drop (l : List Sgn) (i : Nat) : (l.drop i).count .Zero ≤ l.length - i := by
  have := List.count_le_length (a := Sgn.Zero) (l := l.drop i)
  simp at this; omega

theorem is_facet_loop_spec (x : Slice Sgn) (n i : Usize) (hi : i.val ≤ x.val.length)
    (hn : n.val ≤ i.val) :
    is_facet_loop x n i ⦃ r => r.val = n.val + (x.val.drop i.val).count .Zero ⦄ := by
  unfold is_facet_loop
  apply loop.spec_decr_nat (fun (p : Usize × Usize) => x.val.length - p.2.val)
    (fun p => p.2.val ≤ x.val.length ∧ p.1.val ≤ p.2.val ∧
      p.1.val + (x.val.drop p.2.val).count .Zero = n.val + (x.val.drop i.val).count .Zero)
  · rintro ⟨m, j⟩ ⟨hj, hm, heq⟩
    unfold is_facet_loop.body
    simp only at hj hm heq ⊢
    by_cases hlt : j.val < x.val.length
    · have hlt' : j < Slice.len x := by scalar_tac
      simp only [hlt', ite_true]
      step as ⟨s, hs⟩
      rw [sgn_eq]; simp only [Std.bind_ok]
      have hd := drop_cons x.val j.val hlt
      have hs' : s = x.val[j.val]! := by rw [hs, getElem!_pos x.val j.val hlt]
      have hlen : x.val.length ≤ Usize.max := by scalar_tac
      by_cases hz : s = Sgn.Zero
      · simp only [hz, decide_true, ite_true]
        step as ⟨m1, hm1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        rw [← heq, hd, ← hs', hz, hj1, hm1]; simp [List.count_cons]; omega
      · simp only [hz, decide_false, Bool.false_eq_true, ite_false, Std.bind_ok]
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        rw [← heq, hd, ← hs', hj1]; simp [List.count_cons, hz]
    · have : ¬ j < Slice.len x := by scalar_tac
      simp only [this, ite_false, WP.spec_ok]
      rw [← heq]
      have : j.val = x.val.length := by omega
      rw [this]; simp
  · exact ⟨hi, hn, rfl⟩

/-- **`is_facet` is `Cov.isFacet`.** -/
theorem is_facet_spec (x : Slice Sgn) :
    is_facet x ⦃ r => r = (covOf x).isFacet ⦄ := by
  unfold is_facet
  step with is_facet_loop_spec x 0#usize 0#usize (by simp) (by simp) as ⟨n, hn⟩
  simp only [ForgeExact.DecideOnce.Cov.isFacet, covOf, count_map_toS]
  simp at hn
  by_cases h : x.val.count Sgn.Zero = 1
  · have : n = 1#usize := by scalar_tac
    simp [h, this]
  · have : n ≠ 1#usize := by intro e; apply h; rw [← hn, e]; rfl
    simp [h, this]

/-! ## `le` -/

/-- The condition `le` checks at each index. -/
def leAt (a b : Sgn) : Bool := a == .Zero || a == b

theorem zip_all_map (x y : List Sgn) :
    ((x.map toS).zip (y.map toS)).all (fun p => p.1 == .zero || p.1 == p.2) =
      (x.zip y).all (fun p => leAt p.1 p.2) := by
  induction x generalizing y with
  | nil => simp
  | cons a t ih =>
    cases y with
    | nil => simp
    | cons b u =>
      simp only [List.map_cons, List.zip_cons_cons, List.all_cons, ih]
      congr 1
      cases a <;> cases b <;> rfl

theorem le_loop_spec (x y : Slice Sgn) (ok0 : Bool) (i : Usize) (hi : i.val ≤ x.val.length)
    (hl : ok0 = true → x.val.length = y.val.length) :
    le_loop x y ok0 i ⦃ r =>
      r = (ok0 && ((x.val.drop i.val).zip (y.val.drop i.val)).all (fun p => leAt p.1 p.2)) ⦄ := by
  unfold le_loop
  apply loop.spec_decr_nat (fun (p : Bool × Usize) => x.val.length - p.2.val)
    (fun p => p.2.val ≤ x.val.length ∧ (p.1 = true → x.val.length = y.val.length) ∧
      (p.1 && ((x.val.drop p.2.val).zip (y.val.drop p.2.val)).all (fun q => leAt q.1 q.2)) =
        (ok0 && ((x.val.drop i.val).zip (y.val.drop i.val)).all (fun q => leAt q.1 q.2)))
  · rintro ⟨o, j⟩ ⟨hj, hlj, heq⟩
    unfold le_loop.body
    simp only at hj hlj heq ⊢
    cases o with
    | false =>
      simp only [Bool.false_eq_true, ite_false, WP.spec_ok]
      rw [← heq]; rfl
    | true =>
      simp only [ite_true]
      have hxy := hlj rfl
      by_cases hlt : j.val < x.val.length
      · have hlt' : j < Slice.len x := by scalar_tac
        simp only [hlt', ite_true]
        step as ⟨s, hs⟩
        rw [sgn_eq]; simp only [Std.bind_ok]
        have hdx := drop_cons x.val j.val hlt
        have hdy := drop_cons y.val j.val (by omega)
        have hs' : s = x.val[j.val]! := by rw [hs, getElem!_pos x.val j.val hlt]
        by_cases hz : s = Sgn.Zero
        · simp only [hz, decide_true, ite_true, Std.bind_ok]
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, fun _ => hxy, ?_, by scalar_tac⟩
          rw [← heq, hdx, hdy, ← hs', hz, hj1]
          simp [leAt]
        · simp only [hz, decide_false, Bool.false_eq_true, ite_false]
          have hjy : j < Slice.len y := by scalar_tac
          step as ⟨t, ht⟩
          rw [sgn_eq]; simp only [Std.bind_ok]
          step as ⟨j1, hj1⟩
          have ht' : t = y.val[j.val]! := by rw [ht, getElem!_pos y.val j.val (by omega)]
          refine ⟨by scalar_tac, fun _ => hxy, ?_, by scalar_tac⟩
          rw [← heq, hdx, hdy, ← hs', ← ht', hj1]
          have : (s == Sgn.Zero) = false := by simp [hz]
          simp only [leAt, this, Bool.false_or, List.zip_cons_cons, List.all_cons]
          have hst : decide (s = t) = (s == t) := by cases s <;> cases t <;> decide
          rw [hst, Bool.true_and]
      · have : ¬ j < Slice.len x := by scalar_tac
        simp only [this, ite_false, WP.spec_ok]
        rw [← heq]
        have : j.val = x.val.length := by omega
        rw [this]; simp
  · exact ⟨hi, hl, rfl⟩

/-- **`le` is `Cov.le`.** -/
theorem le_spec (x y : Slice Sgn) :
    le x y ⦃ r => r = ForgeExact.DecideOnce.Cov.le (covOf x) (covOf y) ⦄ := by
  unfold le
  have hl : (decide (Slice.len x = Slice.len y)) = true → x.val.length = y.val.length := by
    intro h; simp at h; scalar_tac
  have := le_loop_spec x y (decide (Slice.len x = Slice.len y)) 0#usize (by simp) hl
  apply WP.spec_mono this
  intro r hr
  rw [hr]
  simp only [ForgeExact.DecideOnce.Cov.le, covOf, List.length_map, zip_all_map]
  have : (decide (Slice.len x = Slice.len y)) = (x.val.length == y.val.length) := by
    by_cases h : x.val.length = y.val.length
    · simp [h]; scalar_tac
    · simp [h]
  rw [this]; simp

end forge_bool_core
