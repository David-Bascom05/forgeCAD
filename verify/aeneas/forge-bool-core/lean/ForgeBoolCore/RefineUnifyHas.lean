import ForgeBoolCore.Specs

/-!
# Group U, part 1: `has`, `same_set`, `faces_at`

The three helpers of `unify`, as set operations on lists of `u32`:

* `has xs x` is membership (`HasSpec`);
* `same_set a b` is `sameSet` (`SameSetSpec`): each list's members are in the other;
* `faces_at face_of ar` is `dedup (ar.map face_of)` (`FacesAtSpec`): the face numbers round a
  cell, first occurrences kept, in the order of `ar`.
-/

open Aeneas Aeneas.Std Result
open ForgeExact.DecideOnce

namespace forge_bool_core

/-- The values of a list of `u32`. -/
abbrev nv (xs : List U32) : List Nat := xs.map (·.val)

/-! ## The Props -/

def HasSpec : Prop :=
  ∀ (xs : Slice U32) (x : U32), has xs x ⦃ b => b = decide (x.val ∈ nv xs.val) ⦄

def SameSetSpec : Prop :=
  ∀ (a b : Slice U32), same_set a b ⦃ r =>
    r = ((nv a.val).all (fun x => decide (x ∈ nv b.val)) && (nv b.val).all (fun x => decide (x ∈ nv a.val))) ⦄

def FacesAtSpec : Prop :=
  ∀ (face_of ar : Slice U32), (∀ x ∈ ar.val, x.val < face_of.val.length) →
    faces_at face_of ar ⦃ r => nv r.val = dedup ((nv ar.val).map fun p => (nv face_of.val).getD p 0) ⦄

/-! ## `has` -/

theorem u_mem_nv_take_succ {xs : List U32} {i : Nat} (h : i < xs.length) (x : Nat) :
    x ∈ nv (xs.take (i + 1)) ↔ x ∈ nv (xs.take i) ∨ x = xs[i].val := by
  rw [List.take_succ_eq_append_getElem h]
  simp only [nv, List.map_append, List.mem_append, List.map_cons, List.map_nil, List.mem_singleton]

theorem u_has_loop_spec (xs : Slice U32) (x : U32) (found : Bool) (i : Usize) (hi : i.val ≤ xs.val.length)
    (hf : found = decide (x.val ∈ nv (xs.val.take i.val))) :
    has_loop xs x found i ⦃ b => b = decide (x.val ∈ nv xs.val) ⦄ := by
  unfold has_loop
  apply loop.spec_decr_nat (fun (st : Bool × Usize) => xs.val.length - st.2.val)
    (fun st => st.2.val ≤ xs.val.length ∧ st.1 = decide (x.val ∈ nv (xs.val.take st.2.val)))
  · rintro ⟨f, j⟩ ⟨hj, hfj⟩
    unfold has_loop.body
    simp only at hj hfj ⊢
    split
    · rename_i hft
      simp only [WP.spec_ok]
      rw [hft] at hfj
      have hm := of_decide_eq_true hfj.symm
      have : x.val ∈ nv xs.val := by
        simp only [nv, List.mem_map] at hm ⊢
        obtain ⟨y, hy, e⟩ := hm; exact ⟨y, List.mem_of_mem_take hy, e⟩
      simp only [hft]; exact (decide_eq_true this).symm
    · rename_i hff
      split
      · rename_i hlt
        have hjl : j.val < xs.val.length := by scalar_tac
        step as ⟨y, hy⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hj1]
        have hfj' : ¬ x.val ∈ nv (xs.val.take j.val) := by
          intro h; apply hff; rw [hfj]; exact decide_eq_true h
        apply Bool.eq_iff_iff.mpr
        rw [decide_eq_true_iff, decide_eq_true_iff, u_mem_nv_take_succ hjl, hy]
        constructor
        · intro e; right; rw [e]
        · rintro (h | h)
          · exact absurd h hfj'
          · exact UScalar.eq_of_val_eq h.symm
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = xs.val.length := by scalar_tac
        rw [hfj, this, List.take_length]
  · exact ⟨hi, hf⟩

/-- **`has` is membership.** -/
theorem has_spec : HasSpec := by
  intro xs x
  unfold has
  exact u_has_loop_spec xs x false 0#usize (by simp) (by simp [nv])

/-! ## `same_set` -/

theorem u_all_take_succ {l : List Nat} {i : Nat} (h : i < l.length) (p : Nat → Bool) :
    (l.take (i + 1)).all p = ((l.take i).all p && p l[i]) := by
  rw [List.take_succ_eq_append_getElem h, List.all_append]; simp

theorem u_ss_loop_spec (a b : Slice U32) (s0 same : Bool) (i : Usize) (hi : i.val ≤ a.val.length)
    (hs : same = (s0 && ((nv a.val).take i.val).all (fun x => decide (x ∈ nv b.val)))) :
    same_set_loop0 a b same i ⦃ r => r = (s0 && (nv a.val).all (fun x => decide (x ∈ nv b.val))) ⦄ := by
  unfold same_set_loop0
  apply loop.spec_decr_nat (fun (st : Bool × Usize) => a.val.length - st.2.val)
    (fun st => st.2.val ≤ a.val.length ∧
      st.1 = (s0 && ((nv a.val).take st.2.val).all (fun x => decide (x ∈ nv b.val))))
  · rintro ⟨f, j⟩ ⟨hj, hfj⟩
    unfold same_set_loop0.body
    simp only at hj hfj ⊢
    split
    · rename_i hft
      split
      · rename_i hlt
        have hjl : j.val < a.val.length := by scalar_tac
        have hjl' : j.val < (nv a.val).length := by simp [nv]; omega
        step as ⟨y, hy⟩
        step with has_spec b y as ⟨h1, hh1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        rw [hj1v, u_all_take_succ hjl', ← Bool.and_assoc, ← hfj, hft, hh1]
        simp [nv, hy]
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = a.val.length := by scalar_tac
        rw [hfj, this, ← List.length_map (f := fun (u : U32) => u.val), List.take_length]
    · rename_i hff
      simp only [WP.spec_ok]
      have : f = false := by simpa using hff
      rw [this] at hfj ⊢
      have h0 : (s0 && ((nv a.val).take j.val).all (fun x => decide (x ∈ nv b.val))) = false := hfj.symm
      by_cases hs0 : s0 = true
      · rw [hs0, Bool.true_and] at h0 ⊢
        rw [← List.take_append_drop j.val (nv a.val), List.all_append, h0, Bool.false_and]
      · simp [hs0]
  · exact ⟨hi, hs⟩

theorem u_ss_loop1_spec (b a : Slice U32) (s0 same : Bool) (i : Usize) (hi : i.val ≤ a.val.length)
    (hs : same = (s0 && ((nv a.val).take i.val).all (fun x => decide (x ∈ nv b.val)))) :
    same_set_loop1 b a same i ⦃ r => r = (s0 && (nv a.val).all (fun x => decide (x ∈ nv b.val))) ⦄ := by
  unfold same_set_loop1
  apply loop.spec_decr_nat (fun (st : Bool × Usize) => a.val.length - st.2.val)
    (fun st => st.2.val ≤ a.val.length ∧
      st.1 = (s0 && ((nv a.val).take st.2.val).all (fun x => decide (x ∈ nv b.val))))
  · rintro ⟨f, j⟩ ⟨hj, hfj⟩
    unfold same_set_loop1.body
    simp only at hj hfj ⊢
    split
    · rename_i hft
      split
      · rename_i hlt
        have hjl : j.val < a.val.length := by scalar_tac
        have hjl' : j.val < (nv a.val).length := by simp [nv]; omega
        step as ⟨y, hy⟩
        step with has_spec b y as ⟨h1, hh1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        rw [hj1v, u_all_take_succ hjl', ← Bool.and_assoc, ← hfj, hft, hh1]
        simp [nv, hy]
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = a.val.length := by scalar_tac
        rw [hfj, this, ← List.length_map (f := fun (u : U32) => u.val), List.take_length]
    · rename_i hff
      simp only [WP.spec_ok]
      have : f = false := by simpa using hff
      rw [this] at hfj ⊢
      have h0 : (s0 && ((nv a.val).take j.val).all (fun x => decide (x ∈ nv b.val))) = false := hfj.symm
      by_cases hs0 : s0 = true
      · rw [hs0, Bool.true_and] at h0 ⊢
        rw [← List.take_append_drop j.val (nv a.val), List.all_append, h0, Bool.false_and]
      · simp [hs0]
  · exact ⟨hi, hs⟩


/-- **`same_set` is `sameSet`.** -/
theorem same_set_spec : SameSetSpec := by
  intro a b
  unfold same_set
  step with u_ss_loop_spec a b true true 0#usize (by simp) (by simp) as ⟨s1, hs1⟩
  apply WP.spec_mono (u_ss_loop1_spec a b ((nv a.val).all fun x => decide (x ∈ nv b.val)) s1 0#usize
    (by simp) (by simp [hs1]))
  intro r hr; exact hr

/-! ## `faces_at` -/

theorem u_faces_at_loop_spec (face_of ar : Slice U32) (hin : ∀ x ∈ ar.val, x.val < face_of.val.length)
    (out : alloc.vec.Vec U32) (j : Usize) (hj : j.val ≤ ar.val.length)
    (ho : nv out.val = dedup (((nv ar.val).take j.val).map fun p => (nv face_of.val).getD p 0)) :
    faces_at_loop face_of ar out j ⦃ r => nv r.val = dedup ((nv ar.val).map fun p => (nv face_of.val).getD p 0) ⦄ := by
  unfold faces_at_loop
  apply loop.spec_decr_nat (fun (st : alloc.vec.Vec U32 × Usize) => ar.val.length - st.2.val)
    (fun st => st.2.val ≤ ar.val.length ∧
      nv st.1.val = dedup (((nv ar.val).take st.2.val).map fun p => (nv face_of.val).getD p 0))
  · rintro ⟨o, i⟩ ⟨hi, hoi⟩
    unfold faces_at_loop.body
    simp only at hi hoi ⊢
    split
    · rename_i hlt
      have hil : i.val < ar.val.length := by scalar_tac
      have hil' : i.val < (nv ar.val).length := by simp [nv]; omega
      have hol : o.val.length ≤ i.val := by
        have := congrArg List.length hoi
        simp only [nv, List.length_map] at this; rw [this]
        refine (ForgeBoolCore.Dedup.dedup_length_le _).trans ?_
        simp
      step as ⟨x, hx⟩
      step as ⟨x1, hx1⟩
      have hx1v : x1.val = x.val := by rw [hx1]; scalar_tac
      have hxl : x.val < face_of.val.length := by rw [hx]; exact hin _ (List.getElem_mem _)
      step as ⟨f, hf⟩
      have hfv : f.val = (nv face_of.val).getD (nv ar.val)[i.val] 0 := by
        have hxl' : (ar.val[i.val]).val < face_of.val.length := hx ▸ hxl
        simp [nv, hf, hx1v, hx, List.getD_eq_getElem?_getD, hxl']
      step with has_spec (alloc.vec.Vec.deref o) f as ⟨b, hb⟩
      have htk : ((nv ar.val).take (i.val + 1)).map (fun p => (nv face_of.val).getD p 0) =
          ((nv ar.val).take i.val).map (fun p => (nv face_of.val).getD p 0) ++ [f.val] := by
        rw [List.take_succ_eq_append_getElem hil', List.map_append, hfv]; simp
      have hmem : (f.val ∈ nv o.val) ↔ f.val ∈ ((nv ar.val).take i.val).map (fun p => (nv face_of.val).getD p 0) := by
        rw [hoi, mem_dedup]
      split
      · rename_i hbt
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        have hin' : f.val ∈ nv o.val := by
          have := hb ▸ hbt; simpa [alloc.vec.Vec.deref] using this
        rw [show i1.val = i.val + 1 by scalar_tac, htk, ForgeBoolCore.Dedup.dedup_snoc, if_pos (hmem.mp hin'), hoi]
        simp
      · rename_i hbf
        have hout : f.val ∉ nv o.val := by
          intro h; apply hbf; rw [hb]; simpa [alloc.vec.Vec.deref] using h
        step as ⟨o1, ho1⟩
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [show i1.val = i.val + 1 by scalar_tac, htk, ForgeBoolCore.Dedup.dedup_snoc,
          if_neg (fun h => hout (hmem.mpr h)), ← hoi, ho1]
        simp [nv]
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = ar.val.length := by scalar_tac
      rw [hoi, this, ← List.length_map (f := fun (u : U32) => u.val), List.take_length]
  · exact ⟨hj, ho⟩

/-- **`faces_at` is `dedup` of the face numbers.** -/
theorem faces_at_spec : FacesAtSpec := by
  intro face_of ar hin
  unfold faces_at
  exact u_faces_at_loop_spec face_of ar hin _ 0#usize (by simp) (by simp [nv, dedup, dedupAux])

end forge_bool_core
