import ForgeBoolCore.Specs
open Aeneas Aeneas.Std Result

/-!
# Group U: `unify` and its helpers (`has`, `same_set`, `faces_at`)

First the helpers, each proven equal to its list reading and to return `ok`:
`U.has_spec` (`has xs x` is `x ∈ xs`), `U.same_set_spec` (`same_set a b` is the Lean `sameSet` on
the values: each list's members are all in the other).
-/

namespace forge_bool_core

theorem U.all_drop {α} (l : List α) (p : α → Bool) (j : Nat) (h : j < l.length) :
    (l.drop j).all p = (p l[j] && (l.drop (j + 1)).all p) := by
  rw [List.drop_eq_getElem_cons h, List.all_cons]

theorem U.mem_drop (l : List U32) (x : U32) (j : Nat) (h : j < l.length) :
    decide (x ∈ l.drop j) = (decide (l[j] = x) || decide (x ∈ l.drop (j + 1))) := by
  rw [List.drop_eq_getElem_cons h]
  simp only [List.mem_cons]
  by_cases e : l[j] = x
  · simp only [e, true_or, decide_true, Bool.true_or]
  · have : ¬ x = l[j] := fun h => e h.symm
    simp only [this, false_or, e, decide_false, Bool.false_or]

/-- `has_loop` from position `i` with flag `found`: found, or `x` among the rest. -/
theorem U.has_loop_spec (xs : Slice U32) (x : U32) (found : Bool) (i : Usize)
    (hi : i.val ≤ xs.val.length) :
    has_loop xs x found i ⦃ r => r = (found || decide (x ∈ xs.val.drop i.val)) ⦄ := by
  unfold has_loop
  apply loop.spec_decr_nat (fun (p : Bool × Usize) => xs.val.length - p.2.val)
    (fun p => p.2.val ≤ xs.val.length ∧
      (p.1 || decide (x ∈ xs.val.drop p.2.val)) = (found || decide (x ∈ xs.val.drop i.val)))
  · rintro ⟨f, j⟩ ⟨hj, heq⟩
    unfold has_loop.body
    simp only at hj heq ⊢
    cases f with
    | true =>
      simp only [ite_true, WP.spec_ok]
      rw [← heq]; rfl
    | false =>
      simp only [Bool.false_eq_true, ite_false]
      by_cases hlt : j.val < xs.val.length
      · have hlt' : j < Slice.len xs := by scalar_tac
        simp only [hlt', ite_true]
        step as ⟨y, hy⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [← heq, hj1, U.mem_drop _ _ _ hlt, hy]
        simp
      · have : ¬ j < Slice.len xs := by scalar_tac
        simp only [this, ite_false, WP.spec_ok]
        rw [← heq]
        have : j.val = xs.val.length := by omega
        rw [this]; simp
  · exact ⟨hi, rfl⟩

/-- **`has` is membership.** -/
theorem U.has_spec (xs : Slice U32) (x : U32) : has xs x ⦃ r => r = decide (x ∈ xs.val) ⦄ := by
  unfold has
  apply WP.spec_mono (U.has_loop_spec xs x false 0#usize (by simp))
  intro r hr; rw [hr]; simp

/-- `same_set`'s first loop: the flag and every member of the rest of `a` in `b`. -/
theorem U.same_set_loop0_spec (a b : Slice U32) (same : Bool) (i : Usize)
    (hi : i.val ≤ a.val.length) :
    same_set_loop0 a b same i ⦃ r => r = (same && (a.val.drop i.val).all fun y => decide (y ∈ b.val)) ⦄ := by
  unfold same_set_loop0
  apply loop.spec_decr_nat (fun (p : Bool × Usize) => a.val.length - p.2.val)
    (fun p => p.2.val ≤ a.val.length ∧
      (p.1 && (a.val.drop p.2.val).all fun y => decide (y ∈ b.val)) =
        (same && (a.val.drop i.val).all fun y => decide (y ∈ b.val)))
  · rintro ⟨f, j⟩ ⟨hj, heq⟩
    unfold same_set_loop0.body
    simp only at hj heq ⊢
    cases f with
    | false =>
      simp only [Bool.false_eq_true, ite_false, WP.spec_ok]
      rw [← heq]; rfl
    | true =>
      simp only [ite_true]
      by_cases hlt : j.val < a.val.length
      · have hlt' : j < Slice.len a := by scalar_tac
        simp only [hlt', ite_true]
        step as ⟨y, hy⟩
        step with U.has_spec as ⟨h, hh⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [← heq, hj1, hh, hy, U.all_drop _ _ _ hlt]
        simp
      · have : ¬ j < Slice.len a := by scalar_tac
        simp only [this, ite_false, WP.spec_ok]
        rw [← heq]
        have : j.val = a.val.length := by omega
        rw [this]; simp
  · exact ⟨hi, rfl⟩

/-- `same_set`'s second loop: likewise for `b` in `a`. -/
theorem U.same_set_loop1_spec (a b : Slice U32) (same : Bool) (i : Usize)
    (hi : i.val ≤ b.val.length) :
    same_set_loop1 a b same i ⦃ r => r = (same && (b.val.drop i.val).all fun y => decide (y ∈ a.val)) ⦄ := by
  unfold same_set_loop1
  apply loop.spec_decr_nat (fun (p : Bool × Usize) => b.val.length - p.2.val)
    (fun p => p.2.val ≤ b.val.length ∧
      (p.1 && (b.val.drop p.2.val).all fun y => decide (y ∈ a.val)) =
        (same && (b.val.drop i.val).all fun y => decide (y ∈ a.val)))
  · rintro ⟨f, j⟩ ⟨hj, heq⟩
    unfold same_set_loop1.body
    simp only at hj heq ⊢
    cases f with
    | false =>
      simp only [Bool.false_eq_true, ite_false, WP.spec_ok]
      rw [← heq]; rfl
    | true =>
      simp only [ite_true]
      by_cases hlt : j.val < b.val.length
      · have hlt' : j < Slice.len b := by scalar_tac
        simp only [hlt', ite_true]
        step as ⟨y, hy⟩
        step with U.has_spec as ⟨h, hh⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [← heq, hj1, hh, hy, U.all_drop _ _ _ hlt]
        simp
      · have : ¬ j < Slice.len b := by scalar_tac
        simp only [this, ite_false, WP.spec_ok]
        rw [← heq]
        have : j.val = b.val.length := by omega
        rw [this]; simp
  · exact ⟨hi, rfl⟩

/-- **`same_set` is `sameSet`**: each list's members all in the other. -/
theorem U.same_set_spec (a b : Slice U32) :
    same_set a b ⦃ r => r = ((a.val.all fun y => decide (y ∈ b.val)) &&
      (b.val.all fun y => decide (y ∈ a.val))) ⦄ := by
  unfold same_set
  step with U.same_set_loop0_spec a b true 0#usize (by simp) as ⟨s0, hs0⟩
  apply WP.spec_mono (U.same_set_loop1_spec a b s0 0#usize (by simp))
  intro r hr; rw [hr, hs0]; simp

/-! ## `faces_at`: the face numbers round a cell, each once, in order -/

/-- Append `f` unless it is there. -/
def U.pushNew (out : List U32) (f : U32) : List U32 := if f ∈ out then out else out ++ [f]

/-- `faces_at`'s reading: the faces of `ar`'s positions, first occurrences kept, after `out`. -/
def U.facesL (fo : List U32) (out : List U32) (ar : List U32) : List U32 :=
  (ar.map fun a => fo[a.val]!).foldl U.pushNew out

theorem U.pushNew_len (out : List U32) (f : U32) : (U.pushNew out f).length ≤ out.length + 1 := by
  unfold U.pushNew; split <;> simp

theorem U.facesL_len (fo out ar : List U32) : (U.facesL fo out ar).length ≤ out.length + ar.length := by
  induction ar generalizing out with
  | nil => simp [U.facesL]
  | cons a t ih =>
    simp only [U.facesL, List.map_cons, List.foldl_cons] at ih ⊢
    have := ih (U.pushNew out fo[a.val]!)
    have := U.pushNew_len out fo[a.val]!
    simp only [List.length_cons]; omega

theorem U.faces_at_loop_spec (fo ar : Slice U32) (out : alloc.vec.Vec U32) (j : Usize)
    (hj : j.val ≤ ar.val.length) (hin : ∀ a ∈ ar.val, a.val < fo.val.length)
    (hlen : out.val.length + ar.val.length ≤ Usize.max) :
    faces_at_loop fo ar out j ⦃ r => r.val = U.facesL fo.val out.val (ar.val.drop j.val) ⦄ := by
  unfold faces_at_loop
  apply loop.spec_decr_nat (fun (p : alloc.vec.Vec U32 × Usize) => ar.val.length - p.2.val)
    (fun p => p.2.val ≤ ar.val.length ∧ p.1.val.length + (ar.val.length - p.2.val) ≤ out.val.length + ar.val.length ∧
      U.facesL fo.val p.1.val (ar.val.drop p.2.val) = U.facesL fo.val out.val (ar.val.drop j.val))
  · rintro ⟨o, k⟩ ⟨hk, ho, heq⟩
    unfold faces_at_loop.body
    simp only at hk ho heq ⊢
    by_cases hlt : k.val < ar.val.length
    · have hlt' : k < Slice.len ar := by scalar_tac
      simp only [hlt', ite_true]
      step as ⟨a, ha⟩
      have hamem : a ∈ ar.val := by rw [ha]; exact List.getElem_mem hlt
      have hab := hin a hamem
      step as ⟨i2, hi2⟩
      have hi2' : i2.val < fo.val.length := by scalar_tac
      step as ⟨f, hf⟩
      step with U.has_spec as ⟨b, hb⟩
      have hd := List.drop_eq_getElem_cons (l := ar.val) hlt
      have key : U.facesL fo.val o.val (ar.val.drop k.val) =
          U.facesL fo.val (U.pushNew o.val f) (ar.val.drop (k.val + 1)) := by
        rw [hd]; simp only [U.facesL, List.map_cons, List.foldl_cons]
        congr 2
        have hia : i2.val = a.val := by scalar_tac
        rw [hf, ← ha, getElem!_pos fo.val a.val hab]
        simp only [hia]
      by_cases hm : f ∈ o.val
      · have : b = true := by rw [hb]; simpa [alloc.vec.Vec.deref] using hm
        simp only [this, ite_true]
        step as ⟨k1, hk1⟩
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        rw [← heq, key, hk1]; simp [U.pushNew, hm]
      · have : b = false := by rw [hb]; simpa [alloc.vec.Vec.deref] using hm
        simp only [this, Bool.false_eq_true, ite_false]
        have hroom : o.val.length < Usize.max := by scalar_tac
        step as ⟨o1, ho1⟩
        step as ⟨k1, hk1⟩
        refine ⟨by scalar_tac, by simp [ho1]; scalar_tac, ?_, by scalar_tac⟩
        rw [← heq, key, hk1, ho1]; simp [U.pushNew, hm]
    · have : ¬ k < Slice.len ar := by scalar_tac
      simp only [this, ite_false, WP.spec_ok]
      rw [← heq]
      have : k.val = ar.val.length := by omega
      rw [this]; simp [U.facesL]
  · exact ⟨hj, by simp only; omega, rfl⟩

/-- **`faces_at`**: the face numbers of `ar`'s positions, each once, in first-seen order. -/
theorem U.faces_at_spec (fo ar : Slice U32) (hin : ∀ a ∈ ar.val, a.val < fo.val.length)
    (hlen : ar.val.length ≤ Usize.max) :
    faces_at fo ar ⦃ r => r.val = U.facesL fo.val [] ar.val ⦄ := by
  unfold faces_at
  apply WP.spec_mono (U.faces_at_loop_spec fo ar (alloc.vec.Vec.new U32) 0#usize (by simp) hin (by simpa using hlen))
  intro r hr; rw [hr]; simp

end forge_bool_core
