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

/-! ### `count_in`, `count_sec` -/

theorem drop_take_succ {β : Type} (L : List β) {lo i : Nat} (hlo : lo ≤ i) (hi : i < L.length) :
    (L.drop lo).take (i + 1 - lo) = (L.drop lo).take (i - lo) ++ [L[i]] := by
  rw [show i + 1 - lo = (i - lo) + 1 by omega, List.take_add_one, List.getElem?_drop,
    show lo + (i - lo) = i by omega, List.getElem?_eq_getElem hi]
  rfl

theorem count_in_spec (ids : Slice U32) (lo hi : Usize) (x : U32) (hlo : lo.val ≤ hi.val)
    (hhi : hi.val ≤ ids.val.length) :
    count_in ids lo hi x ⦃ n => n.val = ((ids.val.drop lo.val).take (hi.val - lo.val)).count x ⦄ := by
  unfold count_in count_in_loop
  apply loop.spec_decr_nat (fun (q : Usize × Usize) => hi.val - q.2.val)
    (fun q => lo.val ≤ q.2.val ∧ q.2.val ≤ hi.val ∧
      q.1.val = ((ids.val.drop lo.val).take (q.2.val - lo.val)).count x)
  · rintro ⟨n, i⟩ ⟨h1, h2, hn⟩
    unfold count_in_loop.body
    simp only at h1 h2 hn ⊢
    split
    · rename_i hlt
      have hil : i.val < ids.val.length := by scalar_tac
      have hcl : ((ids.val.drop lo.val).take (i.val - lo.val)).count x ≤ i.val := by
        have := List.count_le_length (a := x) (l := (ids.val.drop lo.val).take (i.val - lo.val))
        simp at this; omega
      step as ⟨y, hy⟩
      have hstep := drop_take_succ ids.val h1 hil
      split
      · rename_i hyx
        step as ⟨n1, hn1⟩
        step as ⟨i1, hi1⟩
        have : i1.val = i.val + 1 := by scalar_tac
        refine ⟨by omega, by scalar_tac, ?_, by scalar_tac⟩
        rw [this, hstep, List.count_append, ← hn, hn1, ← hy, hyx]; simp
      · rename_i hyx
        step as ⟨i1, hi1⟩
        have : i1.val = i.val + 1 := by scalar_tac
        refine ⟨by omega, by scalar_tac, ?_, by scalar_tac⟩
        rw [this, hstep, List.count_append, ← hn, ← hy]
        simp [List.count_singleton, hyx]
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = hi.val := by scalar_tac
      rw [← this]; exact hn
  · exact ⟨Nat.le_refl _, hlo, by simp⟩

/-- Two sectors alike, field by field. -/
def secEq (y x : Sector) : Bool := decide (y.facet.val = x.facet.val ∧ y.a.val = x.a.val ∧ y.b.val = x.b.val)

theorem count_sec_spec (secs : Slice Sector) (lo hi : Usize) (x : Sector) (hlo : lo.val ≤ hi.val)
    (hhi : hi.val ≤ secs.val.length) :
    count_sec secs lo hi x ⦃ n => n.val = ((secs.val.drop lo.val).take (hi.val - lo.val)).countP (secEq · x) ⦄ := by
  unfold count_sec count_sec_loop
  apply loop.spec_decr_nat (fun (q : Usize × Usize) => hi.val - q.2.val)
    (fun q => lo.val ≤ q.2.val ∧ q.2.val ≤ hi.val ∧
      q.1.val = ((secs.val.drop lo.val).take (q.2.val - lo.val)).countP (secEq · x))
  · rintro ⟨n, i⟩ ⟨h1, h2, hn⟩
    unfold count_sec_loop.body
    simp only at h1 h2 hn ⊢
    split
    · rename_i hlt
      have hil : i.val < secs.val.length := by scalar_tac
      have hcl : ((secs.val.drop lo.val).take (i.val - lo.val)).countP (secEq · x) ≤ i.val := by
        have := List.countP_le_length (p := (secEq · x)) (l := (secs.val.drop lo.val).take (i.val - lo.val))
        simp at this; omega
      step as ⟨y, hy⟩
      have hstep := drop_take_succ secs.val h1 hil
      have key : ∀ (n1 : Usize), n1.val = n.val + (if secEq y x then 1 else 0) →
          ∀ (i1 : Usize), i1.val = i.val + 1 →
          n1.val = ((secs.val.drop lo.val).take (i1.val - lo.val)).countP (secEq · x) := by
        intro n1 hn1 i1 hi1
        rw [hi1, hstep, List.countP_append, ← hn, hn1, ← hy]; simp
      split
      · rename_i hf
        split
        · rename_i ha
          split
          · rename_i hb
            step as ⟨n1, hn1⟩
            step as ⟨i1, hi1⟩
            refine ⟨by scalar_tac, by scalar_tac, key n1 ?_ i1 (by scalar_tac), by scalar_tac⟩
            have : secEq y x = true := by simp [secEq, hf, ha, hb]
            rw [this]; scalar_tac
          · rename_i hb
            step as ⟨i1, hi1⟩
            refine ⟨by scalar_tac, by scalar_tac, key n ?_ i1 (by scalar_tac), by scalar_tac⟩
            have : secEq y x = false := by
              simp only [secEq, decide_eq_false_iff_not, not_and]
              intro _ _ e; exact hb (UScalar.eq_of_val_eq e)
            simp [this]
        · rename_i ha
          step as ⟨i1, hi1⟩
          refine ⟨by scalar_tac, by scalar_tac, key n ?_ i1 (by scalar_tac), by scalar_tac⟩
          have : secEq y x = false := by
            simp only [secEq, decide_eq_false_iff_not, not_and]
            intro _ e; exact absurd (UScalar.eq_of_val_eq e) ha
          simp [this]
      · rename_i hf
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, by scalar_tac, key n ?_ i1 (by scalar_tac), by scalar_tac⟩
        have : secEq y x = false := by
          simp only [secEq, decide_eq_false_iff_not, not_and]
          intro e; exact absurd (UScalar.eq_of_val_eq e) hf
        simp [this]
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = hi.val := by scalar_tac
      rw [← this]; exact hn
  · exact ⟨Nat.le_refl _, hlo, by simp⟩

/-! ### `ledger_wf`'s loops -/

theorem wf_loop0_eq (v : alloc.vec.Vec U32) (ok : Bool) (k : Usize) :
    ledger_wf_loop0 v ok k = csr_ok_loop0 v ok k := by
  unfold ledger_wf_loop0 csr_ok_loop0
  congr 1

/-- Row `[lo, hi)` of `ids`, as numbers. -/
def subN (ids : List U32) (lo hi : Nat) : List Nat := ((ids.drop lo).take (hi - lo)).map (·.val)

theorem count_val (L : List U32) (x : U32) : L.count x = (L.map (·.val)).count x.val := by
  rw [List.count_map_of_injective]
  intro a b h; exact UScalar.eq_of_val_eq h

theorem wf_loop1_inner_spec (v : alloc.vec.Vec U32) (lo hi : Usize) (hlo : lo.val ≤ hi.val)
    (hhi : hi.val ≤ v.val.length) (ok0 ok : Bool) (i : Usize) (hi1 : lo.val ≤ i.val ∧ i.val ≤ hi.val)
    (h : ok = true ↔ ok0 = true ∧ ∀ j, lo.val ≤ j → j < i.val →
      ((v.val.drop lo.val).take (hi.val - lo.val)).count (v.val.getD j 0#u32) ≤ 2) :
    ledger_wf_loop1_loop0 v ok lo hi i ⦃ r => r = true ↔ ok0 = true ∧
      ∀ x ∈ (v.val.drop lo.val).take (hi.val - lo.val), ((v.val.drop lo.val).take (hi.val - lo.val)).count x ≤ 2 ⦄ := by
  set sub := (v.val.drop lo.val).take (hi.val - lo.val) with hsub
  have hfin : (ok0 = true ∧ ∀ j, lo.val ≤ j → j < hi.val → sub.count (v.val.getD j 0#u32) ≤ 2) ↔
      (ok0 = true ∧ ∀ x ∈ sub, sub.count x ≤ 2) := by
    apply and_congr_right; intro _
    constructor
    · intro hj x hx
      obtain ⟨t, ht, rfl⟩ := List.getElem_of_mem hx
      have htl : t < hi.val - lo.val := by simp [hsub] at ht; omega
      have := hj (lo.val + t) (by omega) (by omega)
      have e : v.val.getD (lo.val + t) 0#u32 = sub[t] := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some]
        simp [hsub]
      rw [e] at this; exact this
    · intro hx j h1 h2
      apply hx
      have e : v.val.getD j 0#u32 = sub[j - lo.val]'(by simp [hsub]; omega) := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some]
        simp [hsub]; congr 1; omega
      rw [e]; exact List.getElem_mem _
  rw [← hfin]
  unfold ledger_wf_loop1_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => hi.val + 1 - q.2.val)
    (fun q => lo.val ≤ q.2.val ∧ q.2.val ≤ hi.val ∧ (q.1 = true ↔ ok0 = true ∧
      ∀ j, lo.val ≤ j → j < q.2.val → sub.count (v.val.getD j 0#u32) ≤ 2))
  · rintro ⟨o, j⟩ ⟨hja, hj2, ho⟩
    unfold ledger_wf_loop1_loop0.body
    simp only at hja hj2 ho ⊢
    split
    · rename_i hot
      split
      · rename_i hlt
        have hjl : j.val < v.val.length := by scalar_tac
        step as ⟨x, hx⟩
        step with count_in_spec (alloc.vec.Vec.deref v) lo hi x hlo (by simp [alloc.vec.Vec.deref]; exact hhi) as ⟨c, hc⟩
        simp only [alloc.vec.Vec.deref, Slice.from_val] at hc
        have hc' : c.val = sub.count x := hc
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
        have hg : v.val.getD j.val 0#u32 = x := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl, Option.getD_some, hx]
        have hok := ho.mp hot
        rw [hj1v]
        simp only [decide_eq_true_eq]
        constructor
        · intro h2
          refine ⟨hok.1, fun a h1 ha => ?_⟩
          rcases Nat.lt_or_ge a j.val with h3 | h3
          · exact hok.2 a h1 h3
          · have : a = j.val := by omega
            subst this; rw [hg, ← hc']; scalar_tac
        · rintro ⟨_, hm⟩
          have := hm j.val hja (by omega)
          rw [hg, ← hc'] at this; scalar_tac
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = hi.val := by scalar_tac
        rw [← this]; exact ho
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      rw [this] at ho ⊢
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a h1 ha => h2.2 a h1 (by omega)⟩
        cases this
  · exact ⟨hi1.1, hi1.2, h⟩

/-- The wings condition over the first `k` edges. -/
def WingsTo (c : Csr) (k : Nat) : Prop := ∀ e, e < k → ∀ φ, (rowL c e).count φ ≤ 2

theorem wings_row (c : Csr) (e : Nat) :
    (∀ x ∈ (c.ids.val.drop (stAt c e)).take (stAt c (e + 1) - stAt c e),
      ((c.ids.val.drop (stAt c e)).take (stAt c (e + 1) - stAt c e)).count x ≤ 2) ↔ ∀ φ, (rowL c e).count φ ≤ 2 := by
  rw [rowL_eq]
  unfold subL
  set sub := (c.ids.val.drop (stAt c e)).take (stAt c (e + 1) - stAt c e)
  constructor
  · intro h φ
    by_cases hm : φ ∈ sub.map (·.val)
    · obtain ⟨x, hx, rfl⟩ := List.mem_map.mp hm
      rw [← count_val]; exact h x hx
    · rw [List.count_eq_zero_of_not_mem hm]; omega
  · intro h x _
    rw [count_val]; exact h x.val

theorem wf_loop1_spec (c : Csr) (n : Nat) (ne : Usize) (hne : ne.val = nrowsL c) (ok0 ok : Bool) (e : Usize)
    (hc : ok0 = true → CsrOK c n) (he : e.val ≤ ne.val) (h : ok = true ↔ ok0 = true ∧ WingsTo c e.val) :
    ledger_wf_loop1 c.start c.ids ne ok e ⦃ r => r = true ↔ ok0 = true ∧ WingsTo c ne.val ⦄ := by
  unfold ledger_wf_loop1
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => ne.val + 1 - q.2.val)
    (fun q => q.2.val ≤ ne.val ∧ (q.1 = true ↔ ok0 = true ∧ WingsTo c q.2.val))
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold ledger_wf_loop1.body
    simp only at hj ho ⊢
    split
    · rename_i hot
      have hok := ho.mp hot
      have hcs := hc hok.1
      split
      · rename_i hlt
        have hjl : j.val < nrowsL c := by scalar_tac
        have hk : j.val + 1 < c.start.val.length := by unfold nrowsL at hjl; omega
        have hm := csr_mono hcs (k := j.val) 1 hk
        have hl := csr_le_len hcs hk
        step as ⟨i, hi⟩
        step as ⟨lo, hlo⟩
        have hlov : lo.val = stAt c j.val := by
          rw [hlo, stAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, ← hi]
          scalar_tac
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        step as ⟨hiu, hhiu⟩
        have hhv : hiu.val = stAt c (j.val + 1) := by
          have : i1.val = j.val + 1 := by scalar_tac
          rw [hhiu, stAt, ← this, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega),
            Option.getD_some, ← hi2]
          scalar_tac
        step with wf_loop1_inner_spec c.ids lo hiu (by rw [hlov, hhv]; exact hm)
          (by rw [hhv]; exact hl) true o lo ⟨Nat.le_refl _, by rw [hlov, hhv]; exact hm⟩
          ⟨fun _ => ⟨rfl, fun a h1 h2 => by omega⟩, fun _ => hot⟩ as ⟨ok2, hok2⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, ?_, by omega⟩
        rw [hj1v, hok2, hlov, hhv, wings_row]
        constructor
        · intro hw
          refine ⟨hok.1, fun a ha => ?_⟩
          rcases Nat.lt_or_ge a j.val with h3 | h3
          · exact hok.2 a h3
          · have : a = j.val := by omega
            subst this; exact hw
        · rintro ⟨_, hw⟩
          exact hw j.val (by omega)
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = ne.val := by scalar_tac
        rw [← this]; exact ho
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      rw [this] at ho ⊢
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a ha => h2.2 a (by omega)⟩
        cases this
  · exact ⟨he, h⟩

end inc

end forge_bool_core
