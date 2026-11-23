import ForgeMachineCore.Funs
import ForgeMachine.Interp
open Aeneas Aeneas.Std Result

/-!
# The shipped interpreter core refines the spec

kernel/forge-machine-core's `step`, as Aeneas translates it (Funs.lean), against
verify/forge-machine's `ForgeMachine.step` (Interp.lean). The translation's values are read into
the spec's: an `i64` as its integer, a position vector as a function on `Fin n`.
-/

namespace forge_machine_core

/-- An optional `i64` as an optional integer. -/
def iv (o : Option Std.I64) : Option ℤ := o.map (·.val)

/-- A position vector as the spec's position. -/
def P (n : ℕ) (v : List (Option Std.I64)) : ForgeMachine.Pos n := fun k => iv (v.getD k.val none)

/-! ## copy -/

theorem copy_spec (p : Slice (Option Std.I64)) (hp : p.val.length < Usize.max) :
    copy p ⦃ r => r.val = p.val ⦄ := by
  unfold copy
  simp only
  unfold copy_loop
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec (Option Std.I64) × Usize) => p.val.length - q.2.val)
    (fun q => q.2.val ≤ p.val.length ∧ q.1.val = p.val.take q.2.val)
  · rintro ⟨out, k⟩ ⟨hk, ho⟩
    unfold copy_loop.body
    simp only at hk ho ⊢
    split
    · rename_i h
      step with Slice.index_usize_spec p k (by scalar_tac) as ⟨v, hv⟩
      step with alloc.vec.Vec.push_spec out v (by simp [ho]; scalar_tac) as ⟨o1, ho1⟩
      step as ⟨k1, hk1⟩
      have : k.val < p.val.length := by scalar_tac
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [ho1, ho, hv, hk1]
      rw [List.take_succ, List.getElem?_eq_getElem this]; rfl
    · rename_i h
      simp only [WP.spec_ok]
      have : k.val = p.val.length := by scalar_tac
      rw [ho, this, List.take_length]
  · simp [alloc.vec.Vec.with_capacity]

theorem one_spec (e : Ev) : one e ⦃ r => r.val = [e] ⦄ := by
  unfold one
  step with alloc.vec.Vec.push_spec (alloc.vec.Vec.new Ev) e (by simp; scalar_tac) as ⟨r, hr⟩
  simpa using hr

/-! ## target -/

/-- The bound every integer word and position is held to: |x| < 2^29 (536 m in µm). Sums of two
stay far inside `i64`, and a cycle's pecks inside a vector's length. -/
def Bd (x : ℤ) : Prop := -2 ^ 29 < x ∧ x < 2 ^ 29

def BdO (o : Option Std.I64) : Prop := ∀ v, o = some v → Bd v.val

/-- One axis's target over the integers: the value and whether it was an incremental word from an
unknown position. -/
def tOne (absolute : Bool) (p w : Option ℤ) : Option ℤ × Bool :=
  match w with
  | none => (p, false)
  | some d => if absolute then (some d, false) else
    match p with
    | some v => (some (v + d), false)
    | none => (none, true)

theorem target_one_spec (absolute : Bool) (p w : Option Std.I64) (hp : BdO p) (hw : BdO w) :
    target_one absolute p w ⦃ r => iv r.1 = (tOne absolute (iv p) (iv w)).1 ∧
      r.2 = (tOne absolute (iv p) (iv w)).2 ⦄ := by
  unfold target_one
  cases w with
  | none => simp [tOne, iv]
  | some d =>
    by_cases ha : absolute
    · simp [tOne, iv, ha]
    · cases p with
      | none => simp [tOne, iv, ha]
      | some v =>
        have h1 := hp v rfl; have h2 := hw d rfl
        simp only [Bd] at h1 h2
        simp only [ha, Bool.false_eq_true, ite_false]
        step as ⟨i, hi⟩
        simp [tOne, iv, hi]

/-- The values of a vector of optional `i64`. -/
def lv (v : List (Option Std.I64)) : List (Option ℤ) := v.map iv

/-- Every axis's target, in order. -/
def tList (absolute : Bool) (pos ax : List (Option ℤ)) : List (Option ℤ × Bool) :=
  (List.range pos.length).map fun k => tOne absolute (pos.getD k none) (ax.getD k none)

theorem lv_getD (v : List (Option Std.I64)) (k : ℕ) : (lv v).getD k none = iv (v.getD k none) := by
  simp only [lv, List.getD_eq_getElem?_getD, List.getElem?_map]
  cases v[k]? <;> rfl

theorem tList_take_succ (absolute : Bool) (pos ax : List (Option ℤ)) (k : ℕ) (hk : k < pos.length) :
    (tList absolute pos ax).take (k + 1) =
      (tList absolute pos ax).take k ++ [tOne absolute (pos.getD k none) (ax.getD k none)] := by
  simp only [tList]
  rw [← List.map_take, ← List.map_take, List.take_range, List.take_range,
    Nat.min_eq_left (by omega), Nat.min_eq_left (by omega), List.range_succ, List.map_append]
  rfl

theorem tList_length (absolute : Bool) (pos ax : List (Option ℤ)) :
    (tList absolute pos ax).length = pos.length := by simp [tList]

theorem target_spec (absolute : Bool) (pos ax : Slice (Option Std.I64))
    (hl : pos.val.length < Usize.max) (hp : ∀ o ∈ pos.val, BdO o) (hx : ∀ o ∈ ax.val, BdO o) :
    target absolute pos ax ⦃ r =>
      match r with
      | .Ok out => lv out.val = (tList absolute (lv pos.val) (lv ax.val)).map (·.1) ∧
          (tList absolute (lv pos.val) (lv ax.val)).all (fun x => !x.2)
      | .Err _ => (tList absolute (lv pos.val) (lv ax.val)).any (·.2) ⦄ := by
  unfold target
  simp only
  have hlen : (tList absolute (lv pos.val) (lv ax.val)).length = pos.val.length := by
    simp [tList_length, lv]
  step with (show target_loop absolute pos ax (alloc.vec.Vec.with_capacity (Option Std.I64) pos.len) false 0#usize
      ⦃ r => lv r.1.val = (tList absolute (lv pos.val) (lv ax.val)).map (·.1) ∧
        r.2 = (tList absolute (lv pos.val) (lv ax.val)).any (·.2) ⦄ from by
    unfold target_loop
    apply loop.spec_decr_nat
      (fun (q : alloc.vec.Vec (Option Std.I64) × Bool × Usize) => pos.val.length - q.2.2.val)
      (fun q => q.2.2.val ≤ pos.val.length ∧
        lv q.1.val = ((tList absolute (lv pos.val) (lv ax.val)).take q.2.2.val).map (·.1) ∧
        q.2.1 = ((tList absolute (lv pos.val) (lv ax.val)).take q.2.2.val).any (·.2))
    · rintro ⟨out, bad, k⟩ ⟨hk, ho, hb⟩
      unfold target_loop.body
      simp only at hk ho hb ⊢
      split
      · rename_i h
        have hkl : k.val < pos.val.length := by scalar_tac
        have hw : (if k < ax.len then Slice.index_usize ax k else ok none) ⦃ w =>
            w = ax.val.getD k.val none ⦄ := by
          split
          · rename_i h2
            step with Slice.index_usize_spec ax k (by scalar_tac) as ⟨w, hw⟩
            rw [hw]; simp [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (show k.val < ax.val.length by scalar_tac)]
          · rename_i h2
            simp only [WP.spec_ok]
            rw [List.getD_eq_getElem?_getD, List.getElem?_eq_none (by scalar_tac)]; rfl
        step with hw as ⟨w, hw⟩
        step with Slice.index_usize_spec pos k (by scalar_tac) as ⟨pk, hpk⟩
        have hpkb : BdO pk := hp pk (by rw [hpk]; exact List.getElem_mem _)
        have hwb : BdO w := by
          intro v hv
          rw [hw] at hv
          rw [List.getD_eq_getElem?_getD] at hv
          cases hq : ax.val[k.val]? with
          | none => simp [hq] at hv
          | some o => simp [hq] at hv; subst hv; exact hx _ (List.mem_of_getElem? hq) v rfl
        step with target_one_spec absolute pk w hpkb hwb as ⟨v, unknown, hv1, hv2⟩
        have hpkg : pk = pos.val.getD k.val none := by
          rw [hpk, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hkl]; rfl
        have hbad : (if unknown = true then ok true else ok bad) = ok (unknown || bad) := by
          cases unknown <;> rfl
        rw [hbad, bind_ok]
        step with alloc.vec.Vec.push_spec out v (by
            have := congrArg List.length ho
            simp [lv, List.length_take] at this; scalar_tac) as ⟨o1, ho1⟩
        step as ⟨k1, hk1⟩
        have ht := tList_take_succ absolute (lv pos.val) (lv ax.val) k.val (by simp [lv]; omega)
        rw [lv_getD, lv_getD, ← hpkg, ← hw] at ht
        refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
        · rw [ho1, hk1, ht]; simp [lv, ho] at *; simp_all [lv]
        · rw [hk1, ht, List.any_append, hb, hv2]; simp [Bool.or_comm]
      · rename_i h
        simp only [WP.spec_ok]
        have : k.val = pos.val.length := by scalar_tac
        rw [this, ← hlen, List.take_length] at ho hb
        exact ⟨ho, hb⟩
    · simp [alloc.vec.Vec.with_capacity, lv]) as ⟨out, bad, ho, hb⟩
  split
  · rename_i h; simp only [WP.spec_ok]; rw [← hb, h]
  · rename_i h; simp only [WP.spec_ok]
    refine ⟨ho, ?_⟩
    simp only [h] at hb
    rw [List.all_eq_true]; intro x hx
    have := List.any_eq_false.mp hb.symm x hx
    simpa using this

theorem P_eq (n : ℕ) (v : List (Option Std.I64)) (k : Fin n) : P n v k = (lv v).getD k.val none := by
  rw [lv_getD]; rfl

theorem tList_get (absolute : Bool) (pos ax : List (Option ℤ)) (k : ℕ) (hk : k < pos.length) :
    (tList absolute pos ax)[k]'(by simp [tList_length, hk]) = tOne absolute (pos.getD k none) (ax.getD k none) := by
  simp [tList]

/-- The translation's targets are the spec's. -/
theorem target_rel (n : ℕ) (absolute : Bool) (pos ax : List (Option Std.I64))
    (hp : pos.length = n) :
    ForgeMachine.target absolute (P n pos) (P n ax) =
      if (tList absolute (lv pos) (lv ax)).any (·.2) then none
      else some (fun k => ((tList absolute (lv pos) (lv ax)).map (·.1)).getD k.val none) := by
  have hl : (lv pos).length = n := by simp [lv, hp]
  have hget : ∀ k : Fin n, ((tList absolute (lv pos) (lv ax)).map (·.1)).getD k.val none =
      (tOne absolute (P n pos k) (P n ax k)).1 := by
    intro k
    rw [List.getD_eq_getElem?_getD, List.getElem?_map,
      List.getElem?_eq_getElem (by simp [tList_length, hl]), tList_get _ _ _ _ (by omega)]
    simp [P_eq]
  have hany : (tList absolute (lv pos) (lv ax)).any (·.2) = true ↔
      ∃ k : Fin n, (tOne absolute (P n pos k) (P n ax k)).2 = true := by
    rw [List.any_eq_true]
    constructor
    · rintro ⟨x, hx, h2⟩
      obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem hx
      have hi' : i < n := by simpa [tList_length, hl] using hi
      refine ⟨⟨i, hi'⟩, ?_⟩
      rw [tList_get _ _ _ _ (by omega)] at h2
      simpa [P_eq] using h2
    · rintro ⟨k, hk⟩
      refine ⟨_, List.getElem_mem (show k.val < (tList absolute (lv pos) (lv ax)).length by
        simp [tList_length, hl]), ?_⟩
      rw [tList_get _ _ _ _ (by omega)]
      simpa [P_eq] using hk
  unfold ForgeMachine.target
  by_cases ha : absolute
  · subst ha
    have : (tList true (lv pos) (lv ax)).any (·.2) = false := by
      rw [Bool.eq_false_iff]; intro h
      obtain ⟨k, hk⟩ := hany.mp h
      simp [tOne] at hk
      split at hk <;> simp_all
    simp only [ite_true, this, Bool.false_eq_true, ite_false, Option.some.injEq]
    funext k
    rw [hget k]; simp only [tOne]
    cases P n ax k <;> simp
  · simp only [Bool.not_eq_true] at ha; subst ha
    simp only [Bool.false_eq_true, ite_false]
    by_cases hall : ∀ (k : Fin n), (P n ax k).isSome = true → (P n pos k).isSome = true
    · have : (tList false (lv pos) (lv ax)).any (·.2) = false := by
        rw [Bool.eq_false_iff]; intro h
        obtain ⟨k, hk⟩ := hany.mp h
        have hk2 := hall k
        simp only [tOne, Bool.false_eq_true, ite_false] at hk
        cases hax : P n ax k <;> cases hpos : P n pos k <;> simp_all
      rw [if_pos hall]
      simp only [this, Bool.false_eq_true, ite_false, Option.some.injEq]
      funext k
      rw [hget k]; simp only [tOne, Bool.false_eq_true, ite_false]
      cases P n ax k <;> cases P n pos k <;> simp
    · have : (tList false (lv pos) (lv ax)).any (·.2) = true := by
        rw [hany]
        push_neg at hall
        obtain ⟨k, h1, h2⟩ := hall
        refine ⟨k, ?_⟩
        simp only [tOne]
        cases hx : P n ax k <;> simp_all
      rw [if_neg hall]
      simp only [this, ite_true]

end forge_machine_core
