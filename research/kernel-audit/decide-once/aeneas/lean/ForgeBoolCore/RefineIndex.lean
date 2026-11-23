import ForgeBoolCore.RefineCells
open Aeneas Aeneas.Std Result

/-!
# The sorted index: `Index::find` by binary search

`cmpL` is a total order (`cmpL_eq_zero`, `cmpL_swap`, `cmpL_lt_trans`). Over an index whose order
is sorted and holds every id (`IndexOK`), `Index::find` returns the id with the key's covector, or
`NONE` when there is none (`index_find_spec`).
-/

namespace forge_bool_core

theorem cmpL_eq_zero : ∀ (a b : List Sgn), cmpL a b = 0 ↔ a = b
  | [], [] => by simp [cmpL]
  | [], _ :: _ => by simp [cmpL]
  | _ :: _, [] => by simp [cmpL]
  | x :: xs, y :: ys => by
    simp only [cmpL, List.cons.injEq]
    by_cases h1 : rk x < rk y
    · simp only [h1, ite_true]
      constructor
      · intro h; cases h
      · rintro ⟨rfl, _⟩; omega
    · by_cases h2 : rk y < rk x
      · simp only [h1, h2, ite_true, ite_false]
        constructor
        · intro h; cases h
        · rintro ⟨rfl, _⟩; omega
      · simp only [h1, h2, ite_false]
        rw [cmpL_eq_zero xs ys]
        have : x = y := rk_inj (by omega)
        simp [this]

theorem cmpL_swap : ∀ (a b : List Sgn), cmpL b a = -cmpL a b
  | [], [] => by simp [cmpL]
  | [], _ :: _ => by simp [cmpL]
  | _ :: _, [] => by simp [cmpL]
  | x :: xs, y :: ys => by
    simp only [cmpL]
    by_cases h1 : rk x < rk y
    · simp [h1, show ¬rk y < rk x by omega]
    · by_cases h2 : rk y < rk x
      · simp [h1, h2]
      · simp only [h1, h2, ite_false]; exact cmpL_swap xs ys

theorem cmpL_lt_trans : ∀ (a b c : List Sgn), cmpL a b = -1 → cmpL b c = -1 → cmpL a c = -1
  | [], [], _, h, _ => by simp [cmpL] at h
  | [], _ :: _, [], _, h => by simp [cmpL] at h
  | [], _ :: _, _ :: _, _, _ => by simp [cmpL]
  | _ :: _, [], _, h, _ => by simp [cmpL] at h
  | _ :: _, _ :: _, [], _, h => by simp [cmpL] at h
  | x :: xs, y :: ys, z :: zs, h1, h2 => by
    simp only [cmpL] at h1 h2 ⊢
    by_cases a1 : rk x < rk y
    · by_cases b1 : rk y < rk z
      · simp [show rk x < rk z by omega]
      · by_cases b2 : rk z < rk y
        · simp [b1, b2] at h2
        · simp [show rk x < rk z by omega]
    · by_cases a2 : rk y < rk x
      · simp [a1, a2] at h1
      · simp only [a1, a2, ite_false] at h1
        have exy : rk x = rk y := by omega
        by_cases b1 : rk y < rk z
        · simp [show rk x < rk z by omega]
        · by_cases b2 : rk z < rk y
          · simp [b1, b2] at h2
          · simp only [b1, b2, ite_false] at h2
            simp only [show ¬rk x < rk z by omega, show ¬rk z < rk x by omega, ite_false]
            exact cmpL_lt_trans xs ys zs h1 h2

/-- `a ≤ b`: not after. -/
def leL (a b : List Sgn) : Prop := cmpL a b ≠ 1

theorem leL_cases {a b : List Sgn} (h : leL a b) : a = b ∨ cmpL a b = -1 := by
  rcases cmpL_vals a b with e | e | e
  · exact Or.inr e
  · exact Or.inl ((cmpL_eq_zero a b).mp e)
  · exact absurd e h

theorem le_lt_trans {a b c : List Sgn} (h1 : leL a b) (h2 : cmpL b c = -1) : cmpL a c = -1 := by
  rcases leL_cases h1 with rfl | h
  · exact h2
  · exact cmpL_lt_trans a b c h h2

theorem lt_le_trans {a b c : List Sgn} (h1 : cmpL a b = -1) (h2 : leL b c) : cmpL a c = -1 := by
  rcases leL_cases h2 with rfl | h
  · exact h1
  · exact cmpL_lt_trans a b c h1 h

theorem neg_iff {a b : List Sgn} : cmpL a b = 1 ↔ cmpL b a = -1 := by
  rw [cmpL_swap a b]; omega

/-- Past an element after the key, every later one is after it too. -/
theorem le_gt_trans {x y k : List Sgn} (h1 : leL x y) (h2 : cmpL x k = 1) : cmpL y k = 1 :=
  neg_iff.mpr (lt_le_trans (neg_iff.mp h2) h1)

/-! ## The binary search -/

/-- Cell `id`'s covector. -/
def cellAt (cells : Slice (alloc.vec.Vec Sgn)) (id : Nat) : List Sgn := (cells.val.getD id default).val

def ordAt (order : alloc.vec.Vec U32) (p : Nat) : Nat := (order.val.getD p 0#u32).val

/-- The index holds every id once in covector order, and ids stay below `NONE`. -/
structure IndexOK (order : alloc.vec.Vec U32) (cells : Slice (alloc.vec.Vec Sgn)) : Prop where
  ids_lt : ∀ p, p < order.val.length → ordAt order p < cells.val.length
  sorted : ∀ p q, p < q → q < order.val.length → leL (cellAt cells (ordAt order p)) (cellAt cells (ordAt order q))
  all : ∀ id, id < cells.val.length → ∃ p, p < order.val.length ∧ ordAt order p = id
  small : cells.val.length < U32.max

theorem ordAt_eq {order : alloc.vec.Vec U32} {p : Nat} (h : p < order.val.length) :
    ordAt order p = order.val[p].val := by
  simp [ordAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]

theorem cellAt_eq {cells : Slice (alloc.vec.Vec Sgn)} {id : Nat} (h : id < cells.val.length) :
    cellAt cells id = cells.val[id].val := by
  simp [cellAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]

theorem none_val : NONE.val = U32.max := by simp [NONE, U32.max_eq, U32.rMax]

/-- The result: the id with the key's covector, or `NONE` when no cell has it. -/
def FindOK (cells : Slice (alloc.vec.Vec Sgn)) (key : List Sgn) (r : U32) : Prop :=
  (r = NONE → ∀ id, id < cells.val.length → cellAt cells id ≠ key) ∧
  (r ≠ NONE → r.val < cells.val.length ∧ cellAt cells r.val = key)

theorem index_find_loop_spec (order : alloc.vec.Vec U32) (cells : Slice (alloc.vec.Vec Sgn))
    (key : Slice Sgn) (hok : IndexOK order cells) (lo hi : Usize) (at0 : U32)
    (hlh : lo.val ≤ hi.val ∧ hi.val ≤ order.val.length)
    (hnone : at0 = NONE → (∀ p, p < lo.val → cmpL (cellAt cells (ordAt order p)) key.val = -1) ∧
      (∀ p, hi.val ≤ p → p < order.val.length → cmpL (cellAt cells (ordAt order p)) key.val = 1))
    (hsome : at0 ≠ NONE → at0.val < cells.val.length ∧ cellAt cells at0.val = key.val) :
    index.Index.find_loop order cells key lo hi at0 ⦃ r => FindOK cells key.val r ⦄ := by
  unfold index.Index.find_loop
  apply loop.spec_decr_nat
    (fun (q : Usize × Usize × U32) => if q.2.2 = NONE then q.2.1.val - q.1.val + 1 else 0)
    (fun q => q.1.val ≤ q.2.1.val ∧ q.2.1.val ≤ order.val.length ∧
      (q.2.2 = NONE → (∀ p, p < q.1.val → cmpL (cellAt cells (ordAt order p)) key.val = -1) ∧
        (∀ p, q.2.1.val ≤ p → p < order.val.length → cmpL (cellAt cells (ordAt order p)) key.val = 1)) ∧
      (q.2.2 ≠ NONE → q.2.2.val < cells.val.length ∧ cellAt cells q.2.2.val = key.val))
  · rintro ⟨l, h, a⟩ ⟨hlh1, hlh2, hn, hs⟩
    unfold index.Index.find_loop.body
    simp only at hlh1 hlh2 hn hs ⊢
    split
    · rename_i ha
      obtain ⟨hbelow, habove⟩ := hn ha
      split
      · rename_i hlt
        have hlt' : l.val < h.val := by scalar_tac
        step as ⟨d, hd⟩
        step as ⟨d2, hd2⟩
        step as ⟨mid, hmid⟩
        have hml : mid.val < order.val.length := by scalar_tac
        have hmlo : l.val ≤ mid.val ∧ mid.val < h.val := by scalar_tac
        step as ⟨id, hid⟩
        step as ⟨i2, hi2⟩
        have hidv : ordAt order mid.val = id.val := by rw [ordAt_eq hml, hid]
        have hidl : id.val < cells.val.length := by rw [← hidv]; exact hok.ids_lt _ hml
        have hi2v : i2.val = id.val := by rw [hi2]; scalar_tac
        step as ⟨v1, hv1⟩
        step with cmp_cov_spec (alloc.vec.Vec.deref v1) key as ⟨o, ho⟩
        have hcell : (alloc.vec.Vec.deref v1).val = cellAt cells (ordAt order mid.val) := by
          rw [hidv, cellAt_eq hidl, hv1]; simp [alloc.vec.Vec.deref, hi2v]
        rw [hcell] at ho
        split
        · rename_i hneg
          have hc : cmpL (cellAt cells (ordAt order mid.val)) key.val = -1 := by
            have : o.val < 0 := by scalar_tac
            rcases cmpL_vals (cellAt cells (ordAt order mid.val)) key.val with e | e | e <;> omega
          step as ⟨l1, hl1⟩
          simp only [ha, ite_true, and_assoc]
          refine ⟨by scalar_tac, hlh2, fun _ => ⟨fun p hp => ?_, habove⟩, fun e => absurd rfl e, by scalar_tac⟩
          by_cases hpm : p = mid.val
          · rw [hpm]; exact hc
          · exact le_lt_trans (hok.sorted p mid.val (by scalar_tac) hml) hc
        · rename_i hneg
          split
          · rename_i hpos
            have hc : cmpL (cellAt cells (ordAt order mid.val)) key.val = 1 := by
              have : 0 < o.val := by scalar_tac
              rcases cmpL_vals (cellAt cells (ordAt order mid.val)) key.val with e | e | e <;> omega
            simp only [WP.spec_ok, ha, ite_true, and_assoc]
            refine ⟨by scalar_tac, by scalar_tac, fun _ => ⟨hbelow, fun p hp hpl => ?_⟩, fun e => absurd rfl e, by scalar_tac⟩
            by_cases hpm : p = mid.val
            · rw [hpm]; exact hc
            · exact le_gt_trans (hok.sorted mid.val p (by omega) hpl) hc
          · rename_i hpos
            have hc : cmpL (cellAt cells (ordAt order mid.val)) key.val = 0 := by
              rcases cmpL_vals (cellAt cells (ordAt order mid.val)) key.val with e | e | e
              · exfalso; apply hneg; rw [← ho] at e; scalar_tac
              · exact e
              · exfalso; apply hpos; rw [← ho] at e; scalar_tac
            have hidn : id ≠ NONE := by
              intro e; have := hok.small; rw [e, none_val] at hidl; omega
            simp only [WP.spec_ok, ha, ite_true, hidn, ite_false, and_assoc]
            refine ⟨hlh1, hlh2, fun e => e.elim, fun _ => ⟨hidl, ?_⟩, by omega⟩
            rw [← hidv]; exact (cmpL_eq_zero _ _).mp hc
      · rename_i hge
        simp only [WP.spec_ok]
        refine ⟨fun _ id hid heq => ?_, fun e => absurd ha e⟩
        obtain ⟨p, hp, hpe⟩ := hok.all id hid
        have hhl : h.val ≤ l.val := by scalar_tac
        by_cases hpl : p < l.val
        · have := hbelow p hpl; rw [hpe, heq] at this; rw [(cmpL_eq_zero _ _).mpr rfl] at this; cases this
        · have := habove p (by omega) hp; rw [hpe, heq] at this; rw [(cmpL_eq_zero _ _).mpr rfl] at this; cases this
    · rename_i ha
      simp only [WP.spec_ok]
      exact ⟨fun e => absurd e ha, fun _ => hs ha⟩
  · exact ⟨hlh.1, hlh.2, hnone, hsome⟩

/-- **`Index::find` is lookup by covector**, over an index that is sorted and holds every id. -/
theorem index_find_spec (ix : index.Index) (cells : Slice (alloc.vec.Vec Sgn)) (key : Slice Sgn)
    (hok : IndexOK ix.order cells) :
    index.Index.find ix cells key ⦃ r => FindOK cells key.val r ⦄ := by
  unfold index.Index.find
  apply index_find_loop_spec ix.order cells key hok
  · simp
  · intro _; exact ⟨fun p hp => by simp at hp, fun p hp hpl => by simp at hp; omega⟩
  · intro h; exact absurd rfl h

end forge_bool_core
