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

/-! ## put -/

/-- `put`'s value at index k. -/
def putF (a : Axes) (p : List (Option Std.I64)) (x y h : Std.I64) (k : ℕ) : Option Std.I64 :=
  if k = a.z.val then some h else if k = a.y.val then some y else if k = a.x.val then some x
  else p.getD k none

theorem put_spec (a : Axes) (p : Slice (Option Std.I64)) (x y h : Std.I64)
    (hl : p.val.length < Usize.max) :
    put a p x y h ⦃ r => r.val = (List.range p.val.length).map (putF a p.val x y h) ⦄ := by
  unfold put
  simp only
  unfold put_loop
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec (Option Std.I64) × Usize) => p.val.length - q.2.val)
    (fun q => q.2.val ≤ p.val.length ∧ q.1.val = (List.range q.2.val).map (putF a p.val x y h))
  · rintro ⟨out, k⟩ ⟨hk, ho⟩
    unfold put_loop.body
    simp only at hk ho ⊢
    split
    · rename_i hlt
      have hkl : k.val < p.val.length := by scalar_tac
      have hv : (if k = a.z then ok (some h) else if k = a.y then ok (some y)
          else if k = a.x then ok (some x) else Slice.index_usize p k) ⦃ v =>
          v = putF a p.val x y h k.val ⦄ := by
        unfold putF
        split
        · rename_i e; simp [e]
        · rename_i e1; split
          · rename_i e2
            have : k.val ≠ a.z.val := fun h' => e1 (by scalar_tac)
            have ey : k.val = a.y.val := by rw [e2]
            simp only [WP.spec_ok]; rw [if_neg this, if_pos ey]
          · rename_i e2; split
            · rename_i e3
              have hz' : k.val ≠ a.z.val := fun h' => e1 (by scalar_tac)
              have hy' : k.val ≠ a.y.val := fun h' => e2 (by scalar_tac)
              have ex : k.val = a.x.val := by rw [e3]
              simp only [WP.spec_ok]; rw [if_neg hz', if_neg hy', if_pos ex]
            · rename_i e3
              have : k.val ≠ a.z.val := fun h' => e1 (by scalar_tac)
              have : k.val ≠ a.y.val := fun h' => e2 (by scalar_tac)
              have : k.val ≠ a.x.val := fun h' => e3 (by scalar_tac)
              step with Slice.index_usize_spec p k (by scalar_tac) as ⟨w, hw⟩
              simp only [*, ite_false]
              rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hkl]; rfl
      step with hv as ⟨v, hv'⟩
      step with alloc.vec.Vec.push_spec out v (by rw [ho]; simp; scalar_tac) as ⟨o1, ho1⟩
      step as ⟨k1, hk1⟩
      refine ⟨by scalar_tac, ?_, by scalar_tac⟩
      rw [ho1, ho, hk1, List.range_succ, List.map_append, hv']; rfl
    · rename_i hge
      simp only [WP.spec_ok]
      have : k.val = p.val.length := by scalar_tac
      rw [ho, this]
  · simp [alloc.vec.Vec.with_capacity]

/-- The spec's axes for the translation's. -/
def AX (n : ℕ) (a : Axes) (hx : a.x.val < n) (hy : a.y.val < n) (hz : a.z.val < n) :
    ForgeMachine.Axes n := ⟨⟨a.x.val, hx⟩, ⟨a.y.val, hy⟩, ⟨a.z.val, hz⟩⟩

theorem put_rel (n : ℕ) (a : Axes) (hx : a.x.val < n) (hy : a.y.val < n) (hz : a.z.val < n)
    (p : List (Option Std.I64)) (hp : p.length = n) (x y h : Std.I64) :
    P n ((List.range p.length).map (putF a p x y h)) =
      ForgeMachine.put (AX n a hx hy hz) (P n p) x.val y.val h.val := by
  funext k
  have hk : k.val < p.length := by omega
  simp only [P, ForgeMachine.put, Function.update_apply, AX]
  rw [List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range hk]
  simp only [Option.map_some, Option.getD_some, putF]
  have ez : (k = ⟨a.z.val, hz⟩) ↔ k.val = a.z.val := Fin.ext_iff
  have ey : (k = ⟨a.y.val, hy⟩) ↔ k.val = a.y.val := Fin.ext_iff
  have ex : (k = ⟨a.x.val, hx⟩) ↔ k.val = a.x.val := Fin.ext_iff
  by_cases h1 : k.val = a.z.val
  · rw [if_pos h1, if_pos (ez.mpr h1)]; rfl
  · rw [if_neg h1, if_neg (fun e => h1 (ez.mp e))]
    by_cases h2 : k.val = a.y.val
    · rw [if_pos h2, if_pos (ey.mpr h2)]; rfl
    · rw [if_neg h2, if_neg (fun e => h2 (ey.mp e))]
      by_cases h3 : k.val = a.x.val
      · rw [if_pos h3, if_pos (ex.mpr h3)]; rfl
      · rw [if_neg h3, if_neg (fun e => h3 (ex.mp e))]

/-! ## The small tests -/

theorem is_arc_spec (g : Std.U32) : is_arc g ⦃ r => r = decide (g.val = 2 ∨ g.val = 3) ⦄ := by
  unfold is_arc
  split
  · rename_i h; simp only [WP.spec_ok]; have : g.val = 2 := by rw [h]; rfl
    simp [this]
  · rename_i h; simp only [WP.spec_ok]
    have : g.val ≠ 2 := fun e => h (by apply UScalar.eq_of_val_eq; simp [e])
    by_cases h3 : g.val = 3
    · have : g = 3#u32 := by apply UScalar.eq_of_val_eq; simp [h3]
      simp [this, h3]
    · have : g ≠ 3#u32 := fun e => h3 (by rw [e]; rfl)
      simp [this, h3, *]

theorem is_peck_spec (g : Std.U32) : is_peck g ⦃ r => r = decide (g.val = 83 ∨ g.val = 73) ⦄ := by
  unfold is_peck
  split
  · rename_i h; simp only [WP.spec_ok]; have : g.val = 83 := by rw [h]; rfl
    simp [this]
  · rename_i h; simp only [WP.spec_ok]
    have : g.val ≠ 83 := fun e => h (by apply UScalar.eq_of_val_eq; simp [e])
    by_cases h3 : g.val = 73
    · have : g = 73#u32 := by apply UScalar.eq_of_val_eq; simp [h3]
      simp [this, h3]
    · have : g ≠ 73#u32 := fun e => h3 (by rw [e]; rfl)
      simp [this, h3, *]

theorem feeds_out_spec (g : Std.U32) : feeds_out g ⦃ r => r = decide (g.val = 85 ∨ g.val = 84) ⦄ := by
  unfold feeds_out
  split
  · rename_i h; simp only [WP.spec_ok]; have : g.val = 85 := by rw [h]; rfl
    simp [this]
  · rename_i h; simp only [WP.spec_ok]
    have : g.val ≠ 85 := fun e => h (by apply UScalar.eq_of_val_eq; simp [e])
    by_cases h3 : g.val = 84
    · have : g = 84#u32 := by apply UScalar.eq_of_val_eq; simp [h3]
      simp [this, h3]
    · have : g ≠ 84#u32 := fun e => h3 (by rw [e]; rfl)
      simp [this, h3, *]

theorem is_cycle_spec (g : Std.U32) : is_cycle g ⦃ r => r = decide (g.val = 81 ∨ g.val = 82 ∨
    g.val = 83 ∨ g.val = 84 ∨ g.val = 85 ∨ g.val = 73) ⦄ := by
  unfold is_cycle
  have e81 : g = 81#u32 ↔ g.val = 81 := ⟨fun h => by rw [h]; rfl, fun h => UScalar.eq_of_val_eq (by simp [h])⟩
  have e82 : g = 82#u32 ↔ g.val = 82 := ⟨fun h => by rw [h]; rfl, fun h => UScalar.eq_of_val_eq (by simp [h])⟩
  have e83 : g = 83#u32 ↔ g.val = 83 := ⟨fun h => by rw [h]; rfl, fun h => UScalar.eq_of_val_eq (by simp [h])⟩
  have e84 : g = 84#u32 ↔ g.val = 84 := ⟨fun h => by rw [h]; rfl, fun h => UScalar.eq_of_val_eq (by simp [h])⟩
  have e85 : g = 85#u32 ↔ g.val = 85 := ⟨fun h => by rw [h]; rfl, fun h => UScalar.eq_of_val_eq (by simp [h])⟩
  have e73 : g = 73#u32 ↔ g.val = 73 := ⟨fun h => by rw [h]; rfl, fun h => UScalar.eq_of_val_eq (by simp [h])⟩
  simp only [e81, e82, e83, e84, e85, e73]
  split_ifs <;> simp only [WP.spec_ok] <;> simp_all

/-! ## Events and the hole -/

def C : Centre → ForgeMachine.Centre
  | .Abs a b => .abs a.val b.val
  | .R r => .r r.val

/-- The translation's event as the spec's. -/
def E (n : ℕ) : Ev → ForgeMachine.Ev n
  | .Rapid v => .rapid (P n v.val)
  | .Feed v f => .feed (P n v.val) f.val
  | .Arc v pl c cw full f => .arc (P n v.val) pl.val (C c) cw full f.val
  | .Home via axes => .home (P n via.val)
      (axes.val.filterMap fun u => if h : u.val < n then some ⟨u.val, h⟩ else none)
  | .Dwell ms => .dwell ms.val

def CY (c : Cyc) : ForgeMachine.Cyc := ⟨c.g.val, c.z.val, c.r.val, c.q.val, c.dwell.val, c.initial.val⟩

section hole
variable (n : ℕ) (a : Axes) (hx : a.x.val < n) (hy : a.y.val < n) (hz : a.z.val < n)
  (base : Slice (Option Std.I64)) (hb : base.val.length = n) (hbl : n < 2 ^ 16)

include hb hbl in
/-- A put, as the spec's position. -/
theorem put_P (x y h : Std.I64) :
    put a base x y h ⦃ r => P n r.val = ForgeMachine.put (AX n a hx hy hz) (P n base.val) x.val y.val h.val ⦄ := by
  have := put_spec a base x y h (by rw [hb]; scalar_tac)
  apply WP.spec_mono this
  intro r hr; rw [hr]; exact put_rel n a hx hy hz base.val hb x y h

/-- The events of one peck at depth d (the spec's `hole`'s inner list). -/
def pk (c : ForgeMachine.Cyc) (pc lift f x y : ℤ) (d : ℤ) : List (ForgeMachine.Ev n) :=
  let at_ := ForgeMachine.put (AX n a hx hy hz) (P n base.val) x y
  [.feed (at_ d) f] ++
    (if c.z < d then
      (if c.g = 83 then [.rapid (at_ c.r), .rapid (at_ (d + pc))] else [.rapid (at_ (d + lift))])
    else [])

theorem pecks_flat (c : ForgeMachine.Cyc) (pc lift f x y z s : ℤ) (hs : 0 < s) (d : ℤ) (hd : z < d) :
    (ForgeMachine.pecks z s hs d).flatMap (pk n a hx hy hz base c pc lift f x y) =
      pk n a hx hy hz base c pc lift f x y (max (d - s) z) ++
        (ForgeMachine.pecks z s hs (max (d - s) z)).flatMap (pk n a hx hy hz base c pc lift f x y) := by
  rw [ForgeMachine.pecks_eq, if_pos hd, List.flatMap_cons]

include hb hbl in
theorem hole_loop_spec (c : Cyc) (pc lift f x y s : Std.I64) (out0 : alloc.vec.Vec Ev)
    (hzr : c.z.val ≤ c.r.val) (hzb : Bd c.z.val) (hrb : Bd c.r.val) (hs : 0 < s.val)
    (hsb : s.val < 2 ^ 30) (hpc : Bd pc.val) (hlift : Bd lift.val) (hout : out0.val.length ≤ 2) :
    hole_loop a base c.g c.z c.r pc lift f x y out0 s c.r ⦃ r =>
      r.val.map (E n) = out0.val.map (E n) ++
        (ForgeMachine.pecks c.z.val s.val hs c.r.val).flatMap
          (pk n a hx hy hz base (CY c) pc.val lift.val f.val x.val y.val) ⦄ := by
  unfold hole_loop
  simp only [Bd] at hzb hrb hpc hlift
  let F := fun (d : ℤ) => (ForgeMachine.pecks c.z.val s.val hs d).flatMap
    (pk n a hx hy hz base (CY c) pc.val lift.val f.val x.val y.val)
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec Ev × Std.I64) => (q.2.val - c.z.val).toNat)
    (fun q => c.z.val ≤ q.2.val ∧ q.2.val ≤ c.r.val ∧
      q.1.val.map (E n) ++ F q.2.val = out0.val.map (E n) ++ F c.r.val ∧
      q.1.val.length + 3 * (q.2.val - c.z.val) ≤ 2 + 3 * (c.r.val - c.z.val))
  · rintro ⟨out, d⟩ ⟨hd1, hd2, hinv, hlen⟩
    unfold hole_loop.body
    simp only at hd1 hd2 hinv hlen ⊢
    split
    · rename_i hgt
      have hzd : c.z.val < d.val := by scalar_tac
      step as ⟨i1, hi1⟩
      have hnext : (if i1 > c.z then ok i1 else ok c.z) = ok (if i1 > c.z then i1 else c.z) := by
        split <;> rfl
      rw [hnext, bind_ok]
      have hnv : (if i1 > c.z then i1 else c.z).val = max (d.val - s.val) c.z.val := by
        split
        · rename_i h; have : i1.val > c.z.val := by scalar_tac
          rw [hi1]; omega
        · rename_i h; have : ¬ i1.val > c.z.val := by scalar_tac
          omega
      generalize hgn : (if i1 > c.z then i1 else c.z) = nx at hnv ⊢
      have hnxb : c.z.val ≤ nx.val ∧ nx.val ≤ c.r.val := by omega
      step with put_P n a hx hy hz base hb hbl x y nx as ⟨v, hv⟩
      have hl0 : out.val.length + 3 < Usize.max := by scalar_tac
      step with alloc.vec.Vec.push_spec out (Ev.Feed v f) (by scalar_tac) as ⟨o1, ho1⟩
      have hF := pecks_flat n a hx hy hz base (CY c) pc.val lift.val f.val x.val y.val c.z.val s.val hs d.val hzd
      have key : ∀ (o2 : alloc.vec.Vec Ev) (nx : Std.I64), nx.val = max (d.val - s.val) c.z.val →
          o2.val.map (E n) = out.val.map (E n) ++
            pk n a hx hy hz base (CY c) pc.val lift.val f.val x.val y.val nx.val →
          (o2.val.length : ℤ) + 3 * (nx.val - c.z.val) ≤ out.val.length + 3 * (d.val - c.z.val) →
          c.z.val ≤ nx.val ∧ nx.val ≤ c.r.val ∧
            o2.val.map (E n) ++ F nx.val = out0.val.map (E n) ++ F c.r.val ∧
            o2.val.length + 3 * (nx.val - c.z.val) ≤ 2 + 3 * (c.r.val - c.z.val) ∧
            (nx.val - c.z.val).toNat < (d.val - c.z.val).toNat := by
        intro o2 nx hnx ho2 hl2
        have h1 : max (d.val - s.val) c.z.val < d.val := by omega
        refine ⟨by omega, by omega, ?_, by omega, by omega⟩
        rw [ho2, List.append_assoc, hnx, ← hF]; exact hinv
      split
      · rename_i hgz
        split
        · rename_i h83
          have h83' : c.g.val = 83 := by rw [h83]; rfl
          step with put_P n a hx hy hz base hb hbl x y c.r as ⟨v1, hv1⟩
          step with alloc.vec.Vec.push_spec o1 (Ev.Rapid v1) (by rw [ho1, List.length_append, List.length_singleton]; scalar_tac) as ⟨o2, ho2⟩
          step as ⟨w2, hw2⟩
          step with put_P n a hx hy hz base hb hbl x y w2 as ⟨v2, hv2⟩
          step with alloc.vec.Vec.push_spec o2 (Ev.Rapid v2) (by rw [ho2, ho1, List.length_append, List.length_append, List.length_singleton, List.length_singleton]; scalar_tac) as ⟨o3, ho3⟩
          have hgz' : nx.val > c.z.val := by scalar_tac
          have hnz : nx.val ≤ d.val - 1 := by omega
          refine key o3 _ hnv ?_ ?_
          · rw [ho3, ho2, ho1]
            simp only [List.map_append, List.map_cons, List.map_nil, List.append_assoc, E, hv, hv1, hv2, hw2]
            simp [pk, CY, h83', hnv, show c.z.val < max (d.val - s.val) c.z.val by omega]
          · rw [ho3, ho2, ho1]; simp only [List.length_append, List.length_singleton]; push_cast; omega
        · rename_i h83
          have h83' : c.g.val ≠ 83 := fun e => h83 (UScalar.eq_of_val_eq (by simp [e]))
          step as ⟨w2, hw2⟩
          step with put_P n a hx hy hz base hb hbl x y w2 as ⟨v1, hv1⟩
          step with alloc.vec.Vec.push_spec o1 (Ev.Rapid v1) (by rw [ho1, List.length_append, List.length_singleton]; scalar_tac) as ⟨o2, ho2⟩
          have hgz' : nx.val > c.z.val := by scalar_tac
          have hnz : nx.val ≤ d.val - 1 := by omega
          refine key o2 _ hnv ?_ ?_
          · rw [ho2, ho1]
            simp only [List.map_append, List.map_cons, List.map_nil, List.append_assoc, E, hv, hv1, hw2]
            simp [pk, CY, h83', hnv, show c.z.val < max (d.val - s.val) c.z.val by omega]
          · rw [ho2, ho1]; simp only [List.length_append, List.length_singleton]; push_cast; omega
      · rename_i hgz
        have hnz : nx.val = c.z.val := by
          have : ¬ nx.val > c.z.val := by scalar_tac
          omega
        simp only [WP.spec_ok]
        refine key o1 _ hnv ?_ ?_
        · rw [ho1]
          simp only [List.map_append, List.map_cons, List.map_nil, E, hv]
          simp [pk, CY, hnv, show ¬ c.z.val < max (d.val - s.val) c.z.val by omega]
        · rw [ho1]; simp only [List.length_append, List.length_singleton]; push_cast; omega
    · rename_i hle
      simp only [WP.spec_ok]
      have : ¬ c.z.val < d.val := by scalar_tac
      have hF0 : F d.val = [] := by
        show (ForgeMachine.pecks c.z.val s.val hs d.val).flatMap _ = []
        rw [ForgeMachine.pecks_eq, if_neg this]; rfl
      rw [hF0, List.append_nil] at hinv
      exact hinv
  · refine ⟨hzr, le_rfl, rfl, ?_⟩
    simp only
    have : (out0.val.length : ℤ) ≤ 2 := by exact_mod_cast hout
    omega

end hole

end forge_machine_core
