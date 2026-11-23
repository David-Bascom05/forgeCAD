import ForgeBoolCore.RefineIncCollapse

/-!
# Group T: `ledger_wf` is `LedgerWF2`

`inc::ledger_wf` decides `LedgerWF2`. With it the inc core's guard is exact: a ledger is refused
by the guard iff it is not well formed. This file proves the pieces bottom up: `csr_ok` is
`CsrOK`, `count_in` counts, and the main loops each decide their field.
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

namespace inc

open ForgeExact.DecideOnce

/-! ### `csr_ok` -/

/-- `start` rises over its first `k` entries. -/
def MonoTo (st : List U32) (k : Nat) : Prop :=
  ∀ j, j + 1 < k → j + 1 < st.length → (st.getD j 0#u32).val ≤ (st.getD (j + 1) 0#u32).val

theorem csr_ok_loop0_spec (v : alloc.vec.Vec U32) (ok0 : Bool) (ok : Bool) (k : Usize) (hk : 1 ≤ k.val)
    (h : ok = true ↔ ok0 = true ∧ MonoTo v.val k.val) :
    csr_ok_loop0 v ok k ⦃ r => r = true ↔ ok0 = true ∧ MonoTo v.val v.val.length ⦄ := by
  unfold csr_ok_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => v.val.length + 1 - q.2.val)
    (fun q => 1 ≤ q.2.val ∧ (q.1 = true ↔ ok0 = true ∧ MonoTo v.val q.2.val))
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold csr_ok_loop0.body
    simp only at hj ho ⊢
    split
    · rename_i hot
      split
      · rename_i hlt
        have hjl : j.val < v.val.length := by scalar_tac
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        step as ⟨i3, hi3⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, ?_, by omega⟩
        have hle : (i2 ≤ i3) ↔ (v.val.getD (j.val - 1) 0#u32).val ≤ (v.val.getD (j.val - 1 + 1) 0#u32).val := by
          have e1 : i1.val = j.val - 1 := by scalar_tac
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some,
            show j.val - 1 + 1 = j.val by omega, List.getD_eq_getElem?_getD,
            List.getElem?_eq_getElem hjl, Option.getD_some, hi2, hi3]
          simp only [e1]
          exact UScalar.le_equiv _ _
        rw [hj1v]
        have hok : ok0 = true ∧ MonoTo v.val j.val := ho.mp hot
        simp only [decide_eq_true_eq]
        constructor
        · intro h2
          refine ⟨hok.1, fun a ha hal => ?_⟩
          rcases Nat.lt_or_ge (a + 1) j.val with h3 | h3
          · exact hok.2 a h3 hal
          · have : a = j.val - 1 := by omega
            subst this; exact hle.mp h2
        · rintro ⟨_, hm⟩
          exact hle.mpr (hm (j.val - 1) (by omega) (by omega))
      · rename_i hge
        simp only [WP.spec_ok]
        rw [ho]
        constructor
        · rintro ⟨h1, h2⟩; exact ⟨h1, fun a ha hal => h2 a (by scalar_tac) hal⟩
        · rintro ⟨h1, h2⟩; exact ⟨h1, fun a ha hal => h2 a (by omega) hal⟩
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      rw [this] at ho ⊢
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a ha hal => h2.2 a (by omega) hal⟩
        cases this
  · exact ⟨hk, h⟩

/-- Every id below `n`, over the first `k`. -/
def IdsTo (ids : List U32) (n : Nat) (k : Nat) : Prop := ∀ i, i < k → i < ids.length → (ids.getD i 0#u32).val < n

theorem csr_ok_loop1_spec (v : alloc.vec.Vec U32) (n : U32) (ok0 : Bool) (ok : Bool) (k : Usize)
    (h : ok = true ↔ ok0 = true ∧ IdsTo v.val n.val k.val) :
    csr_ok_loop1 v n ok k ⦃ r => r = true ↔ ok0 = true ∧ IdsTo v.val n.val v.val.length ⦄ := by
  unfold csr_ok_loop1
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => v.val.length + 1 - q.2.val)
    (fun q => (q.1 = true ↔ ok0 = true ∧ IdsTo v.val n.val q.2.val))
  · rintro ⟨o, j⟩ ho
    unfold csr_ok_loop1.body
    simp only at ho ⊢
    split
    · rename_i hot
      split
      · rename_i hlt
        have hjl : j.val < v.val.length := by scalar_tac
        step as ⟨i2, hi2⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨?_, by omega⟩
        have hg : (v.val.getD j.val 0#u32) = i2 := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl, Option.getD_some, hi2]
        have hok := ho.mp hot
        rw [hj1v]
        simp only [decide_eq_true_eq]
        constructor
        · intro h2
          refine ⟨hok.1, fun a ha hal => ?_⟩
          rcases Nat.lt_or_ge a j.val with h3 | h3
          · exact hok.2 a h3 hal
          · have : a = j.val := by omega
            subst this; rw [hg]; exact h2
        · rintro ⟨_, hm⟩
          have := hm j.val (by omega) hjl
          rw [hg] at this; exact this
      · rename_i hge
        simp only [WP.spec_ok]
        rw [ho]
        constructor
        · rintro ⟨h1, h2⟩; exact ⟨h1, fun a ha hal => h2 a (by scalar_tac) hal⟩
        · rintro ⟨h1, h2⟩; exact ⟨h1, fun a ha hal => h2 a (by omega) hal⟩
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      rw [this] at ho ⊢
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a ha hal => h2.2 a (by omega) hal⟩
        cases this
  · exact h

theorem starts_at_zero_spec (st : Slice U32) :
    starts_at_zero st ⦃ b => b = true ↔ st.val ≠ [] ∧ (st.val.getD 0 0#u32).val = 0 ⦄ := by
  unfold starts_at_zero
  step as ⟨b, hb⟩
  split
  · rename_i h
    have : st.val = [] := by simpa using hb.mp h
    simp [this]
  · rename_i h
    have hne : st.val ≠ [] := by intro e; apply h; apply hb.mpr; simpa using e
    have h0 : 0 < st.val.length := List.length_pos_iff.mpr hne
    step as ⟨i, hi⟩
    simp only [WP.spec_ok, decide_eq_true_eq]
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h0, Option.getD_some, ← hi]
    constructor
    · intro e; exact ⟨hne, by rw [e]; rfl⟩
    · rintro ⟨_, e⟩; exact UScalar.eq_of_val_eq (by rw [e]; rfl)

theorem ids_iff (ids : List U32) (n : Nat) : IdsTo ids n ids.length ↔ ∀ x ∈ ids, x.val < n := by
  constructor
  · intro h x hx
    obtain ⟨i, hi, rfl⟩ := List.getElem_of_mem hx
    have := h i hi hi
    rwa [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi, Option.getD_some] at this
  · intro h i _ hi
    rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi, Option.getD_some]
    exact h _ (List.getElem_mem _)

/-- **`csr_ok` is `CsrOK`.** -/
theorem csr_ok_spec (c : Csr) (n : U32) : csr_ok c n ⦃ b => b = true ↔ CsrOK c n.val ⦄ := by
  unfold csr_ok
  step with starts_at_zero_spec _ as ⟨ok1, hok1⟩
  simp only [alloc.vec.Vec.deref, Slice.from_val] at hok1
  step with csr_ok_loop0_spec c.start ok1 ok1 1#usize (by simp)
    ⟨fun h => ⟨h, fun a ha _ => by simp at ha⟩, fun h => h.1⟩ as ⟨ok2, hok2⟩
  have hfinal : ∀ ok3 : Bool, (ok3 = true ↔ ok2 = true ∧
      (c.start.val.getD (c.start.val.length - 1) 0#u32).val = c.ids.val.length) →
      csr_ok_loop1 c.ids n ok3 0#usize ⦃ b => b = true ↔ CsrOK c n.val ⦄ := by
    intro ok3 h3
    apply WP.spec_mono (csr_ok_loop1_spec c.ids n ok3 ok3 0#usize
      ⟨fun h => ⟨h, fun a ha _ => by simp at ha⟩, fun h => h.1⟩)
    intro b hb
    rw [hb, h3, hok2, hok1, ids_iff]
    constructor
    · rintro ⟨⟨⟨⟨hne, hz⟩, hm⟩, hl⟩, hi⟩
      exact ⟨hne, hz, fun k hk => hm k hk hk, hl, hi⟩
    · intro h
      exact ⟨⟨⟨⟨h.nonempty, h.zero⟩, fun k hk _ => h.mono k hk⟩, h.last⟩, h.ids⟩
  split
  · rename_i h2
    have hne : c.start.val ≠ [] := ((hok2.mp h2).1 |> hok1.mp).1
    have h0 : 0 < c.start.val.length := List.length_pos_iff.mpr hne
    step as ⟨i1, hi1⟩
    step as ⟨i2, hi2⟩
    step as ⟨i3, hi3⟩
    apply hfinal
    simp only [decide_eq_true_eq, h2, true_and]
    have : i3.val = (c.start.val.getD (c.start.val.length - 1) 0#u32).val := by
      rw [hi3, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, hi2]
      have : i1.val = c.start.val.length - 1 := by scalar_tac
      simp only [this]; scalar_tac
    constructor
    · intro e; rw [← this, e]; simp
    · intro e; apply UScalar.eq_of_val_eq; rw [this, e]; simp
  · rename_i h2
    have : ok2 = false := by simpa using h2
    simp only [Std.bind_ok]
    exact hfinal false (by simp [this])

end inc

end forge_bool_core
