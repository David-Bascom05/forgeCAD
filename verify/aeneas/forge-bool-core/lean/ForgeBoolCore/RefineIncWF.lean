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

/-! ### Loop 2: the verts -/

def rowU (ids : List U32) (lo hi : Nat) : List U32 := (ids.drop lo).take (hi - lo)

/-- The checks on one sector, as the Rust makes them. -/
def secP (l : Ledger) (lo hi slo shi : Nat) (x : Sector) : Prop :=
  (rowU' l.vert_sectors.sec.val slo shi).countP (secEq · x) = 1 ∧ x.facet.val < nfacets l ∧
  (rowU l.vert_edges.ids.val lo hi).count x.a = 1 ∧ (rowU l.vert_edges.ids.val lo hi).count x.b = 1 ∧
  1 ≤ (rowU l.edge_facets.ids.val (stAt l.edge_facets x.a.val) (stAt l.edge_facets (x.a.val + 1))).count x.facet ∧
  1 ≤ (rowU l.edge_facets.ids.val (stAt l.edge_facets x.b.val) (stAt l.edge_facets (x.b.val + 1))).count x.facet
where rowU' (secs : List Sector) (lo hi : Nat) : List Sector := (secs.drop lo).take (hi - lo)

theorem row_bounds (c : Csr) (n : Nat) (hc : CsrOK c n) (k : Usize) (hk : k.val + 1 < c.start.val.length) :
    row c k ⦃ r => r.1.val = stAt c k.val ∧ r.2.val = stAt c (k.val + 1) ∧
      stAt c k.val ≤ stAt c (k.val + 1) ∧ stAt c (k.val + 1) ≤ c.ids.val.length ⦄ := by
  apply WP.spec_mono (row_spec c k hk)
  rintro ⟨a, b⟩ ⟨ha, hb⟩
  exact ⟨ha, hb, csr_mono hc (k := k.val) 1 hk, csr_le_len hc hk⟩

theorem mem_of_count_one {β : Type} [BEq β] [LawfulBEq β] {L : List β} {x : β} (h : L.count x = 1) : x ∈ L := by
  apply List.count_pos_iff.mp; omega

set_option maxHeartbeats 2000000 in
theorem wf_loop2_sec_spec (l : Ledger) (hef : CsrOK l.edge_facets (nfacets l))
    (hve : CsrOK l.vert_edges (nrowsL l.edge_facets))
    (nf : Usize) (hnf : nf.val = nfacets l) (lo hi slo shi : Usize) (hlo : lo.val ≤ hi.val)
    (hhi : hi.val ≤ l.vert_edges.ids.val.length) (hslo : slo.val ≤ shi.val) (hshi : shi.val ≤ l.vert_sectors.sec.val.length)
    (ok0 ok : Bool) (j : Usize) (hj : slo.val ≤ j.val ∧ j.val ≤ shi.val)
    (h : ok = true ↔ ok0 = true ∧ ∀ t, slo.val ≤ t → t < j.val →
      secP l lo.val hi.val slo.val shi.val (l.vert_sectors.sec.val.getD t default)) :
    ledger_wf_loop2_loop1 l.edge_facets.start l.edge_facets.ids l.vert_edges.ids l.vert_sectors.sec nf ok lo hi slo shi j
      ⦃ r => r = true ↔ ok0 = true ∧ ∀ t, slo.val ≤ t → t < shi.val →
        secP l lo.val hi.val slo.val shi.val (l.vert_sectors.sec.val.getD t default) ⦄ := by
  unfold ledger_wf_loop2_loop1
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => shi.val + 1 - q.2.val)
    (fun q => slo.val ≤ q.2.val ∧ q.2.val ≤ shi.val ∧ (q.1 = true ↔ ok0 = true ∧ ∀ t, slo.val ≤ t → t < q.2.val →
      secP l lo.val hi.val slo.val shi.val (l.vert_sectors.sec.val.getD t default)))
  · rintro ⟨o, k⟩ ⟨hka, hkb, ho⟩
    unfold ledger_wf_loop2_loop1.body
    simp only at hka hkb ho ⊢
    split
    · rename_i hot
      have hok := ho.mp hot
      split
      · rename_i hlt
        have hkl : k.val < l.vert_sectors.sec.val.length := by scalar_tac
        step as ⟨sc, hsc⟩
        have hg : l.vert_sectors.sec.val.getD k.val default = sc := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hkl, Option.getD_some, hsc]
        -- the step: o' = true ↔ secP sc
        have hfin : ∀ (o' : Bool), (o' = true ↔ secP l lo.val hi.val slo.val shi.val sc) →
            ∀ (k1 : Usize), k1.val = k.val + 1 →
            (o' = true ↔ ok0 = true ∧ ∀ t, slo.val ≤ t → t < k1.val →
              secP l lo.val hi.val slo.val shi.val (l.vert_sectors.sec.val.getD t default)) := by
          intro o' ho' k1 hk1
          rw [hk1, ho']
          constructor
          · intro hp
            refine ⟨hok.1, fun t h1 h2 => ?_⟩
            rcases Nat.lt_or_ge t k.val with h3 | h3
            · exact hok.2 t h1 h3
            · have : t = k.val := by omega
              subst this; rw [hg]; exact hp
          · rintro ⟨_, hp⟩
            have := hp k.val hka (by omega); rw [hg] at this; exact this
        step with count_sec_spec (alloc.vec.Vec.deref l.vert_sectors.sec) slo shi sc hslo
          (by simp [alloc.vec.Vec.deref]; exact hshi) as ⟨c0, hc0⟩
        simp only [alloc.vec.Vec.deref, Slice.from_val] at hc0
        have hc0v : c0.val = (secP.rowU' l.vert_sectors.sec.val slo.val shi.val).countP (secEq · sc) := hc0
        have hvd : (alloc.vec.Vec.deref l.vert_edges.ids).val.length = l.vert_edges.ids.val.length := by
          simp [alloc.vec.Vec.deref]
        have hed : (alloc.vec.Vec.deref l.edge_facets.ids).val.length = l.edge_facets.ids.val.length := by
          simp [alloc.vec.Vec.deref]
        have hnefl : nrowsL l.edge_facets + 1 = l.edge_facets.start.val.length := by
          have := hef.nonempty; unfold nrowsL
          have : 0 < l.edge_facets.start.val.length := List.length_pos_iff.mpr this
          omega
        have hrowmem : ∀ x : U32, (rowU l.vert_edges.ids.val lo.val hi.val).count x = 1 →
            x.val + 1 < l.edge_facets.start.val.length := by
          intro x hx
          have hm := mem_of_count_one hx
          have := hve.ids x (List.mem_of_mem_drop (List.mem_of_mem_take hm))
          omega
        -- each failing branch: `o' = false` and `secP` fails
        have hfalse : ∀ (k1 : Usize), k1.val = k.val + 1 → ¬ secP l lo.val hi.val slo.val shi.val sc →
            (false = true ↔ ok0 = true ∧ ∀ t, slo.val ≤ t → t < k1.val →
              secP l lo.val hi.val slo.val shi.val (l.vert_sectors.sec.val.getD t default)) := by
          intro k1 hk1 hn
          exact hfin false ⟨(fun h => by cases h), fun h => absurd h hn⟩ k1 hk1
        split
        · rename_i hc1
          have hc1v : c0.val = 1 := by rw [hc1]; rfl
          step as ⟨i1, hi1⟩
          have hi1v : i1.val = sc.facet.val := by scalar_tac
          split
          · rename_i hfl
            have hflv : sc.facet.val < nfacets l := by rw [← hnf, ← hi1v]; scalar_tac
            step with count_in_spec (alloc.vec.Vec.deref l.vert_edges.ids) lo hi sc.a hlo (by rw [hvd]; exact hhi)
              as ⟨ca, hca⟩
            simp only [alloc.vec.Vec.deref, Slice.from_val] at hca
            split
            · rename_i hca1
              have hcav : (rowU l.vert_edges.ids.val lo.val hi.val).count sc.a = 1 := by
                rw [rowU, ← hca, hca1]; rfl
              step with count_in_spec (alloc.vec.Vec.deref l.vert_edges.ids) lo hi sc.b hlo (by rw [hvd]; exact hhi)
                as ⟨cb, hcb⟩
              simp only [alloc.vec.Vec.deref, Slice.from_val] at hcb
              split
              · rename_i hcb1
                have hcbv : (rowU l.vert_edges.ids.val lo.val hi.val).count sc.b = 1 := by
                  have : cb = 1#usize := by simpa using hcb1
                  rw [rowU, ← hcb, this]; rfl
                have hra := hrowmem sc.a hcav
                have hrb := hrowmem sc.b hcbv
                step as ⟨ia, hia⟩
                have hiav : ia.val = sc.a.val := by scalar_tac
                step with row_bounds l.edge_facets _ hef ia (by rw [hiav]; exact hra) as ⟨ar, hal, hah, hal1, hal2⟩
                obtain ⟨alo, ahi⟩ := ar
                simp only at hal hah ⊢
                step as ⟨ib, hib⟩
                have hibv : ib.val = sc.b.val := by scalar_tac
                step with row_bounds l.edge_facets _ hef ib (by rw [hibv]; exact hrb) as ⟨br, hbl, hbh, hbl1, hbl2⟩
                obtain ⟨blo, bhi⟩ := br
                simp only at hbl hbh ⊢
                step with count_in_spec (alloc.vec.Vec.deref l.edge_facets.ids) alo ahi sc.facet
                  (by rw [hal, hah]; exact hal1) (by rw [hed, hah]; exact hal2) as ⟨cfa, hcfa⟩
                simp only [alloc.vec.Vec.deref, Slice.from_val] at hcfa
                have hcfav : cfa.val = (rowU l.edge_facets.ids.val (stAt l.edge_facets sc.a.val)
                    (stAt l.edge_facets (sc.a.val + 1))).count sc.facet := by
                  rw [hcfa, rowU, hal, hah, hiav]
                split
                · rename_i hge1
                  step with count_in_spec (alloc.vec.Vec.deref l.edge_facets.ids) blo bhi sc.facet
                    (by rw [hbl, hbh]; exact hbl1) (by rw [hed, hbh]; exact hbl2) as ⟨cfb, hcfb⟩
                  simp only [alloc.vec.Vec.deref, Slice.from_val] at hcfb
                  have hcfbv : cfb.val = (rowU l.edge_facets.ids.val (stAt l.edge_facets sc.b.val)
                      (stAt l.edge_facets (sc.b.val + 1))).count sc.facet := by
                    rw [hcfb, rowU, hbl, hbh, hibv]
                  step as ⟨k1, hk1⟩
                  refine ⟨by scalar_tac, by scalar_tac, hfin _ ?_ k1 (by scalar_tac), by scalar_tac⟩
                  simp only [decide_eq_true_eq]
                  constructor
                  · intro hge2
                    exact ⟨by rw [← hc0v]; exact hc1v, hflv, hcav, hcbv, by rw [← hcfav]; scalar_tac,
                      by rw [← hcfbv]; scalar_tac⟩
                  · rintro ⟨_, _, _, _, _, h6⟩
                    rw [← hcfbv] at h6; scalar_tac
                · rename_i hge1
                  step as ⟨k1, hk1⟩
                  refine ⟨by scalar_tac, by scalar_tac, hfalse k1 (by scalar_tac) ?_, by scalar_tac⟩
                  rintro ⟨_, _, _, _, h5, _⟩
                  rw [← hcfav] at h5; exact hge1 (by scalar_tac)
              · rename_i hcb1
                step as ⟨k1, hk1⟩
                refine ⟨by scalar_tac, by scalar_tac, ?_, by scalar_tac⟩
                have hcbf : ¬ cb = 1#usize := by simpa using hcb1
                refine hfin _ ⟨fun h => absurd h hcb1, fun hp => ?_⟩ k1 (by scalar_tac)
                exfalso
                apply hcbf; have := hp.2.2.2.1; rw [rowU, ← hcb] at this
                apply UScalar.eq_of_val_eq; rw [this]; rfl
            · rename_i hca1
              try simp only [Std.bind_ok]
              try dsimp only
              try simp only [Std.bind_ok, Bool.false_eq_true, ite_false]
              step as ⟨k1, hk1⟩
              refine ⟨by scalar_tac, by scalar_tac, hfalse k1 (by scalar_tac) ?_, by scalar_tac⟩
              rintro ⟨_, _, h3, _⟩
              apply hca1; rw [rowU, ← hca] at h3; apply UScalar.eq_of_val_eq; rw [h3]; rfl
          · rename_i hfl
            try simp only [Std.bind_ok]
            try dsimp only
            try simp only [Std.bind_ok, Bool.false_eq_true, ite_false]
            step as ⟨k1, hk1⟩
            refine ⟨by scalar_tac, by scalar_tac, hfalse k1 (by scalar_tac) ?_, by scalar_tac⟩
            rintro ⟨_, h2, _⟩
            apply hfl; rw [← hnf] at h2; scalar_tac
        · rename_i hc1
          simp only [Std.bind_ok, uncurry, Bool.false_eq_true, ite_false]
          step as ⟨k1, hk1⟩
          refine ⟨by scalar_tac, by scalar_tac, hfalse k1 (by scalar_tac) ?_, by scalar_tac⟩
          rintro ⟨h1, _⟩
          apply hc1; apply UScalar.eq_of_val_eq; rw [hc0v, h1]; rfl
      · rename_i hge
        simp only [WP.spec_ok]
        have : k.val = shi.val := by scalar_tac
        simp only [this] at ho; exact ho
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      rw [this] at ho ⊢
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a h1 ha => h2.2 a h1 (by omega)⟩
        cases this
  · exact ⟨hj.1, hj.2, h⟩

theorem wf_loop2_edge_spec (v : alloc.vec.Vec U32) (lo hi : Usize) (hlo : lo.val ≤ hi.val)
    (hhi : hi.val ≤ v.val.length) (ok0 ok : Bool) (i : Usize) (hi1 : lo.val ≤ i.val ∧ i.val ≤ hi.val)
    (h : ok = true ↔ ok0 = true ∧ ∀ j, lo.val ≤ j → j < i.val →
      ((v.val.drop lo.val).take (hi.val - lo.val)).count (v.val.getD j 0#u32) = 1) :
    ledger_wf_loop2_loop0 v ok lo hi i ⦃ r => r = true ↔ ok0 = true ∧
      ∀ x ∈ (v.val.drop lo.val).take (hi.val - lo.val), ((v.val.drop lo.val).take (hi.val - lo.val)).count x = 1 ⦄ := by
  set sub := (v.val.drop lo.val).take (hi.val - lo.val) with hsub
  have hfin : (ok0 = true ∧ ∀ j, lo.val ≤ j → j < hi.val → sub.count (v.val.getD j 0#u32) = 1) ↔
      (ok0 = true ∧ ∀ x ∈ sub, sub.count x = 1) := by
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
  unfold ledger_wf_loop2_loop0
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => hi.val + 1 - q.2.val)
    (fun q => lo.val ≤ q.2.val ∧ q.2.val ≤ hi.val ∧ (q.1 = true ↔ ok0 = true ∧
      ∀ j, lo.val ≤ j → j < q.2.val → sub.count (v.val.getD j 0#u32) = 1))
  · rintro ⟨o, j⟩ ⟨hja, hj2, ho⟩
    unfold ledger_wf_loop2_loop0.body
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



theorem sec_mono' (l : Ledger) (hs : SecCsrOK l.vert_sectors) {k : Nat} :
    ∀ d, k + d < l.vert_sectors.start.val.length → sstAt l k ≤ sstAt l (k + d)
  | 0, _ => Nat.le_refl _
  | d + 1, hd => by
    have ih := sec_mono' l hs (k := k) d (by omega)
    have := hs.mono (k + d) (by omega)
    have e : k + (d + 1) = k + d + 1 := by omega
    rw [e]; exact Nat.le_trans ih this

theorem sec_le_len' (l : Ledger) (hs : SecCsrOK l.vert_sectors) {k : Nat} (hk : k < l.vert_sectors.start.val.length) :
    sstAt l k ≤ l.vert_sectors.sec.val.length := by
  have := sec_mono' l hs (k := k) (l.vert_sectors.start.val.length - 1 - k) (by omega)
  rw [show k + (l.vert_sectors.start.val.length - 1 - k) = l.vert_sectors.start.val.length - 1 by omega] at this
  have hl := hs.last
  unfold sstAt at *; omega

/-- The checks at vert `v`. -/
def VertOK (l : Ledger) (v : Nat) : Prop :=
  (∀ x ∈ rowU l.vert_edges.ids.val (stAt l.vert_edges v) (stAt l.vert_edges (v + 1)),
    (rowU l.vert_edges.ids.val (stAt l.vert_edges v) (stAt l.vert_edges (v + 1))).count x = 1) ∧
  ∀ t, sstAt l v ≤ t → t < sstAt l (v + 1) →
    secP l (stAt l.vert_edges v) (stAt l.vert_edges (v + 1)) (sstAt l v) (sstAt l (v + 1))
      (l.vert_sectors.sec.val.getD t default)

/-- The tables loop 2 reads are well formed (what the earlier checks give). -/
structure Pre2 (l : Ledger) : Prop where
  ef : CsrOK l.edge_facets (nfacets l)
  ve : CsrOK l.vert_edges (nrowsL l.edge_facets)
  sc : SecCsrOK l.vert_sectors
  rows : l.vert_sectors.start.val.length = l.vert_edges.start.val.length

set_option maxHeartbeats 1000000 in
theorem wf_loop2_spec (l : Ledger) (nf nv : Usize) (hnf : nf.val = nfacets l) (hnv : nv.val = nrowsL l.vert_edges)
    (ok0 ok : Bool) (v : Usize) (hpre : ok0 = true → Pre2 l) (hv : v.val ≤ nv.val)
    (h : ok = true ↔ ok0 = true ∧ ∀ w, w < v.val → VertOK l w) :
    ledger_wf_loop2 l.edge_facets.start l.edge_facets.ids l.vert_edges.start l.vert_edges.ids
      l.vert_sectors.start l.vert_sectors.sec nf nv ok v ⦃ r => r = true ↔ ok0 = true ∧ ∀ w, w < nv.val → VertOK l w ⦄ := by
  unfold ledger_wf_loop2
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => nv.val + 1 - q.2.val)
    (fun q => q.2.val ≤ nv.val ∧ (q.1 = true ↔ ok0 = true ∧ ∀ w, w < q.2.val → VertOK l w))
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold ledger_wf_loop2.body
    simp only at hj ho ⊢
    split
    · rename_i hot
      have hok := ho.mp hot
      have hp := hpre hok.1
      split
      · rename_i hlt
        have hjl : j.val < nrowsL l.vert_edges := by scalar_tac
        have hk : j.val + 1 < l.vert_edges.start.val.length := by unfold nrowsL at hjl; omega
        have hsk : j.val + 1 < l.vert_sectors.start.val.length := by rw [hp.rows]; exact hk
        have hm := csr_mono hp.ve (k := j.val) 1 hk
        have hl := csr_le_len hp.ve hk
        have hsm := sec_mono' l hp.sc (k := j.val) 1 hsk
        have hsl := sec_le_len' l hp.sc hsk
        step as ⟨i, hi⟩
        step as ⟨lo, hlo⟩
        have hlov : lo.val = stAt l.vert_edges j.val := by
          rw [hlo, stAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, ← hi]
          scalar_tac
        step as ⟨i1, hi1⟩
        step as ⟨i2, hi2⟩
        step as ⟨hiu, hhiu⟩
        have hhv : hiu.val = stAt l.vert_edges (j.val + 1) := by
          have : i1.val = j.val + 1 := by scalar_tac
          rw [hhiu, stAt, ← this, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega),
            Option.getD_some, ← hi2]
          scalar_tac
        step with wf_loop2_edge_spec l.vert_edges.ids lo hiu (by rw [hlov, hhv]; exact hm)
          (by rw [hhv]; exact hl) true o lo ⟨Nat.le_refl _, by rw [hlov, hhv]; exact hm⟩
          ⟨fun _ => ⟨rfl, fun a h1 h2 => by omega⟩, fun _ => hot⟩ as ⟨ok2, hok2⟩
        step as ⟨i3, hi3⟩
        step as ⟨slo, hslo⟩
        have hslov : slo.val = sstAt l j.val := by
          rw [hslo, sstAt, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some, ← hi3]
          scalar_tac
        step as ⟨i4, hi4⟩
        step as ⟨i5, hi5⟩
        step as ⟨shi, hshi⟩
        have hshv : shi.val = sstAt l (j.val + 1) := by
          have : i4.val = j.val + 1 := by scalar_tac
          rw [hshi, sstAt, ← this, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega),
            Option.getD_some, ← hi5]
          scalar_tac
        step with wf_loop2_sec_spec l hp.ef hp.ve nf hnf lo hiu slo shi (by rw [hlov, hhv]; exact hm)
          (by rw [hhv]; exact hl) (by rw [hslov, hshv]; exact hsm) (by rw [hshv]; exact hsl)
          (ok0 && ok2) ok2 slo ⟨Nat.le_refl _, by rw [hslov, hshv]; exact hsm⟩
          ⟨fun h2 => ⟨by simp [hok.1, h2], fun t h1 h3 => by omega⟩, fun h2 => by simp at h2; exact h2.2.1⟩
          as ⟨ok3, hok3⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        refine ⟨by omega, ?_, by omega⟩
        rw [hj1v, hok3]
        simp only [Bool.and_eq_true, hok.1, true_and, hok2]
        rw [hlov, hhv, hslov, hshv]
        constructor
        · rintro ⟨hE, hS⟩ w hw
          rcases Nat.lt_or_ge w j.val with h3 | h3
          · exact hok.2 w h3
          · have : w = j.val := by omega
            subst this; exact ⟨hE, hS⟩
        · intro hw
          exact hw j.val (by omega)
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = nv.val := by scalar_tac
        simp only [this] at ho; exact ho
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      rw [this] at ho ⊢
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a ha => h2.2 a (by omega)⟩
        cases this
  · exact ⟨hv, h⟩


/-! ### Loops 3–6: rooms, pieces, witnesses, every facet a piece -/

def RoomsTo (l : Ledger) (k : Nat) : Prop := ∀ f, f < k →
  (l.facet_rooms.val.getD f (0#u32, 0#u32)).1 ≠ (l.facet_rooms.val.getD f (0#u32, 0#u32)).2 ∧
  (l.facet_rooms.val.getD f (0#u32, 0#u32)).1.val < l.n_rooms.val ∧
  (l.facet_rooms.val.getD f (0#u32, 0#u32)).2.val < l.n_rooms.val

theorem wf_loop3_spec (l : Ledger) (nf : Usize) (ok0 ok : Bool) (f : Usize)
    (hpre : ok0 = true → l.facet_rooms.val.length = nf.val) (hf : f.val ≤ nf.val)
    (h : ok = true ↔ ok0 = true ∧ RoomsTo l f.val) :
    ledger_wf_loop3 l.n_rooms l.facet_rooms nf ok f ⦃ r => r = true ↔ ok0 = true ∧ RoomsTo l nf.val ⦄ := by
  unfold ledger_wf_loop3
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => nf.val + 1 - q.2.val)
    (fun q => q.2.val ≤ nf.val ∧ (q.1 = true ↔ ok0 = true ∧ RoomsTo l q.2.val))
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold ledger_wf_loop3.body
    simp only at hj ho ⊢
    split
    · rename_i hot
      have hok := ho.mp hot
      split
      · rename_i hlt
        have hjl : j.val < l.facet_rooms.val.length := by rw [hpre hok.1]; scalar_tac
        step as ⟨p, m, hpm⟩
        have hg : l.facet_rooms.val.getD j.val (0#u32, 0#u32) = (p, m) := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]; simp [hpm]
        have key : ∀ (o' : Bool), (o' = true ↔ (p ≠ m ∧ p.val < l.n_rooms.val ∧ m.val < l.n_rooms.val)) →
            ∀ (j1 : Usize), j1.val = j.val + 1 →
            (o' = true ↔ ok0 = true ∧ RoomsTo l j1.val) := by
          intro o' ho' j1 hj1
          rw [hj1, ho']
          constructor
          · intro hp
            refine ⟨hok.1, fun a ha => ?_⟩
            rcases Nat.lt_or_ge a j.val with h3 | h3
            · exact hok.2 a h3
            · have : a = j.val := by omega
              subst this; rw [hg]; exact hp
          · rintro ⟨_, hp⟩
            have := hp j.val (by omega); rw [hg] at this; exact this
        split
        · rename_i hne
          have hne' : p ≠ m := by simpa using hne
          split
          · rename_i hpl
            step as ⟨j1, hj1⟩
            refine ⟨by scalar_tac, key _ ⟨fun h => ⟨hne', by scalar_tac, by simp at h; scalar_tac⟩,
              fun h => by simp; scalar_tac⟩ j1 (by scalar_tac), by scalar_tac⟩
          · rename_i hpl
            step as ⟨j1, hj1⟩
            refine ⟨by scalar_tac, key _ ⟨(fun h => by cases h), fun h => absurd (by scalar_tac) hpl⟩ j1
              (by scalar_tac), by scalar_tac⟩
        · rename_i hne
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, key _ ⟨(fun h => by cases h), fun h => absurd (UScalar.eq_of_val_eq (by simpa using hne)) h.1⟩ j1
            (by scalar_tac), by scalar_tac⟩
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = nf.val := by scalar_tac
        simp only [this] at ho; exact ho
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      rw [this] at ho ⊢
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a ha => h2.2 a (by omega)⟩
        cases this
  · exact ⟨hf, h⟩

def InsideTo (l : Ledger) (k : Nat) : Prop := ∀ j, j < k →
  ((l.inside.val.getD j default).1 : U32).val < l.n_rooms.val ∧ ((l.inside.val.getD j default).2 : U32).val < l.nb.val

theorem wf_loop5_spec (l : Ledger) (ok0 ok : Bool) (k : Usize) (hk : k.val ≤ l.inside.val.length)
    (h : ok = true ↔ ok0 = true ∧ InsideTo l k.val) :
    ledger_wf_loop5 l.nb l.n_rooms l.inside ok k ⦃ r => r = true ↔ ok0 = true ∧ InsideTo l l.inside.val.length ⦄ := by
  unfold ledger_wf_loop5
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => l.inside.val.length + 1 - q.2.val)
    (fun q => q.2.val ≤ l.inside.val.length ∧ (q.1 = true ↔ ok0 = true ∧ InsideTo l q.2.val))
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold ledger_wf_loop5.body
    simp only at hj ho ⊢
    split
    · rename_i hot
      have hok := ho.mp hot
      split
      · rename_i hlt
        have hjl : j.val < l.inside.val.length := by scalar_tac
        step as ⟨r, b, hrb⟩
        have hg : l.inside.val.getD j.val default = (r, b) := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]; simp [hrb]
        have key : ∀ (o' : Bool), (o' = true ↔ ((r : U32).val < l.n_rooms.val ∧ (b : U32).val < l.nb.val)) →
            ∀ (j1 : Usize), j1.val = j.val + 1 →
            (o' = true ↔ ok0 = true ∧ InsideTo l j1.val) := by
          intro o' ho' j1 hj1
          rw [hj1, ho']
          constructor
          · intro hp
            refine ⟨hok.1, fun a ha => ?_⟩
            rcases Nat.lt_or_ge a j.val with h3 | h3
            · exact hok.2 a h3
            · have : a = j.val := by omega
              subst this; rw [hg]; exact hp
          · rintro ⟨_, hp⟩
            have := hp j.val (by omega); rw [hg] at this; exact this
        split
        · rename_i hrl
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, key _ ⟨fun h => ⟨by scalar_tac, by simp at h; scalar_tac⟩,
            fun h => by simp; scalar_tac⟩ j1 (by scalar_tac), by scalar_tac⟩
        · rename_i hrl
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, key _ ⟨(fun h => by cases h), fun h => absurd (by scalar_tac) hrl⟩ j1
            (by scalar_tac), by scalar_tac⟩
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = l.inside.val.length := by scalar_tac
        simp only [this] at ho; exact ho
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      rw [this] at ho ⊢
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a ha => h2.2 a (by omega)⟩
        cases this
  · exact ⟨hk, h⟩


def PiecesTo (l : Ledger) (k : Nat) : Prop := ∀ j, j < k →
  pcF l j < nfacets l ∧ pfF l j < l.face_body.val.length ∧
  l.face_class.val.getD (pfF l j) 0#u32 = l.facet_class.val.getD (pcF l j) 0#u32

def MarkedTo (l : Ledger) (ip : List Bool) (k : Nat) : Prop :=
  ∀ f, f < nfacets l → (ip.getD f false = true ↔ ∃ j, j < k ∧ pcF l j = f)

theorem wf_loop4_spec (l : Ledger) (ok0 ok : Bool) (ip : alloc.vec.Vec Bool) (k : Usize)
    (hpre : ok0 = true → l.face_class.val.length = l.face_body.val.length)
    (hk : k.val ≤ l.pieces.val.length) (hipl : ip.val.length = nfacets l)
    (h : ok = true ↔ ok0 = true ∧ PiecesTo l k.val) (hm : ok = true → MarkedTo l ip.val k.val) :
    ledger_wf_loop4 l.face_body l.face_class l.facet_class l.pieces (alloc.vec.Vec.len l.facet_class) ok ip k
      ⦃ r => (r.1 = true ↔ ok0 = true ∧ PiecesTo l l.pieces.val.length) ∧ r.2.val.length = nfacets l ∧
        (r.1 = true → MarkedTo l r.2.val l.pieces.val.length) ⦄ := by
  have hsm : True := trivial
  unfold ledger_wf_loop4
  apply loop.spec_decr_nat (fun (q : Bool × alloc.vec.Vec Bool × Usize) => l.pieces.val.length + 1 - q.2.2.val)
    (fun q => q.2.2.val ≤ l.pieces.val.length ∧ q.2.1.val.length = nfacets l ∧
      (q.1 = true ↔ ok0 = true ∧ PiecesTo l q.2.2.val) ∧ (q.1 = true → MarkedTo l q.2.1.val q.2.2.val))
  · rintro ⟨o, ipc, j⟩ ⟨hj, hipc, ho, hmo⟩
    unfold ledger_wf_loop4.body
    simp only at hj hipc ho hmo ⊢
    split
    · rename_i hot
      have hok := ho.mp hot
      have hmk := hmo hot
      split
      · rename_i hlt
        have hjl : j.val < l.pieces.val.length := by scalar_tac
        step as ⟨c, g, hcg⟩
        have hg : l.pieces.val.getD j.val default = (c, g) := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]; simp [hcg]
        have hpc : pcF l j.val = (c : U32).val := by unfold pcF; rw [hg]
        have hpf : pfF l j.val = (g : U32).val := by unfold pfF; rw [hg]
        -- the result: ok2 decides the piece's checks
        have key : ∀ (o' : Bool), (o' = true ↔ (pcF l j.val < nfacets l ∧ pfF l j.val < l.face_body.val.length ∧
              l.face_class.val.getD (pfF l j.val) 0#u32 = l.facet_class.val.getD (pcF l j.val) 0#u32)) →
            ∀ (ip1 : alloc.vec.Vec Bool), ip1.val.length = nfacets l →
            (o' = true → MarkedTo l ip1.val (j.val + 1)) →
            ∀ (j1 : Usize), j1.val = j.val + 1 →
            j1.val ≤ l.pieces.val.length ∧ ip1.val.length = nfacets l ∧
            (o' = true ↔ ok0 = true ∧ PiecesTo l j1.val) ∧ (o' = true → MarkedTo l ip1.val j1.val) := by
          intro o' ho' ip1 hl1 hm1 j1 hj1
          refine ⟨by omega, hl1, ?_, by rw [hj1]; exact hm1⟩
          rw [hj1, ho']
          constructor
          · intro hp
            refine ⟨hok.1, fun a ha => ?_⟩
            rcases Nat.lt_or_ge a j.val with h3 | h3
            · exact hok.2 a h3
            · have : a = j.val := by omega
              subst this; exact hp
          · rintro ⟨_, hp⟩; exact hp j.val (by omega)
        have hfl := hpre hok.1
        -- marking `c` keeps MarkedTo, one more piece
        have hmark : ∀ (ip1 : alloc.vec.Vec Bool), (c : U32).val < nfacets l →
            ip1.val = ipc.val.set (c : U32).val true → MarkedTo l ip1.val (j.val + 1) := by
          intro ip1 hc hip1 f hf
          rw [hip1]
          by_cases e : f = (c : U32).val
          · subst e
            rw [List.getD_eq_getElem?_getD, List.getElem?_set_self (by omega)]
            simp only [Option.getD_some, true_iff]
            exact ⟨j.val, by omega, hpc⟩
          · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm e), ← List.getD_eq_getElem?_getD,
              hmk f hf]
            constructor
            · rintro ⟨a, ha, h1⟩; exact ⟨a, by omega, h1⟩
            · rintro ⟨a, ha, h1⟩
              refine ⟨a, ?_, h1⟩
              rcases Nat.lt_succ_iff_lt_or_eq.mp ha with h | h
              · exact h
              · exfalso; apply e; rw [← h1, h, hpc]
        step as ⟨i1, hi1⟩
        have hi1v : i1.val = (c : U32).val := by scalar_tac
        split
        · rename_i hc1
          have hcl : (c : U32).val < nfacets l := by rw [← hi1v]; simpa [nfacets] using hc1
          step as ⟨i2, hi2⟩
          have hi2v : i2.val = (g : U32).val := by scalar_tac
          split
          · rename_i hg1
            have hgl : (g : U32).val < l.face_body.val.length := by rw [← hi2v]; scalar_tac
            step as ⟨i4, hi4⟩
            have hi4v : i4.val = (g : U32).val := by scalar_tac
            step as ⟨i5, hi5⟩
            step as ⟨i6, hi6⟩
            have hi6v : i6.val = (c : U32).val := by scalar_tac
            step as ⟨i7, hi7⟩
            have heq : (i5 = i7) ↔ l.face_class.val.getD (pfF l j.val) 0#u32 = l.facet_class.val.getD (pcF l j.val) 0#u32 := by
              rw [hpf, hpc, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega), Option.getD_some,
                List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simp [nfacets] at hcl; omega), Option.getD_some,
                hi5, hi7]
              simp only [hi4v, hi6v]
              exact Iff.rfl
            have ho' : (decide (i5 = i7) = true) ↔ (pcF l j.val < nfacets l ∧ pfF l j.val < l.face_body.val.length ∧
                l.face_class.val.getD (pfF l j.val) 0#u32 = l.facet_class.val.getD (pcF l j.val) 0#u32) := by
              rw [decide_eq_true_eq, heq, hpc, hpf]; simp [hcl, hgl]
            split
            · rename_i hd
              step as ⟨i2', hi2'⟩
              have hi2'v : i2'.val = (c : U32).val := by scalar_tac
              step as ⟨x0, _, hbk⟩
              obtain ⟨x1, bk⟩ := x0
              simp only at hbk ⊢
              step as ⟨j1, hj1⟩
              have := key _ ho' (bk true) (by rw [hbk]; simp [hipc]) (fun _ => hmark _ hcl (by rw [hbk, alloc.vec.Vec.set_val_eq, hi2'v]))
                j1 (by scalar_tac)
              obtain ⟨a1, a2, a3, a4⟩ := this
              exact ⟨a1, a2, a3, a4, by scalar_tac⟩
            · rename_i hd
              step as ⟨j1, hj1⟩
              have := key _ ho' ipc hipc (fun h => absurd h hd) j1 (by scalar_tac)
              obtain ⟨a1, a2, a3, a4⟩ := this
              exact ⟨a1, a2, a3, a4, by scalar_tac⟩
          · rename_i hg1
            simp only [Std.bind_ok, Bool.false_eq_true, ite_false]
            step as ⟨j1, hj1⟩
            have := key false ⟨(fun h => by cases h), fun h => absurd (by have := h.2.1; rw [hpf, ← hi2v] at this; scalar_tac) hg1⟩
              ipc hipc (fun h => by cases h) j1 (by scalar_tac)
            obtain ⟨a1, a2, a3, a4⟩ := this
            exact ⟨a1, a2, a3, a4, by scalar_tac⟩
        · rename_i hc1
          simp only [Std.bind_ok, Bool.false_eq_true, ite_false]
          step as ⟨j1, hj1⟩
          have := key false ⟨(fun h => by cases h), fun h => absurd (by have := h.1; rw [hpc, ← hi1v] at this; simp [nfacets] at this; scalar_tac) hc1⟩
            ipc hipc (fun h => by cases h) j1 (by scalar_tac)
          obtain ⟨a1, a2, a3, a4⟩ := this
          exact ⟨a1, a2, a3, a4, by scalar_tac⟩
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = l.pieces.val.length := by scalar_tac
        simp only [this] at ho hmo; exact ⟨ho, hipc, hmo⟩
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      subst this
      refine ⟨⟨(fun h => by cases h), fun h2 => ?_⟩, hipc, (fun h => by cases h)⟩
      exfalso
      have := ho.mpr ⟨h2.1, fun a ha => h2.2 a (by omega)⟩
      cases this
  · exact ⟨hk, hipl, h, hm⟩


theorem wf_loop6_spec (nf : Usize) (ip : alloc.vec.Vec Bool) (ok0 ok : Bool) (f : Usize)
    (hpre : ok0 = true → ip.val.length = nf.val) (hf : f.val ≤ nf.val)
    (h : ok = true ↔ ok0 = true ∧ ∀ g, g < f.val → ip.val.getD g false = true) :
    ledger_wf_loop6 nf ok ip f ⦃ r => r = true ↔ ok0 = true ∧ ∀ g, g < nf.val → ip.val.getD g false = true ⦄ := by
  unfold ledger_wf_loop6
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => nf.val + 1 - q.2.val)
    (fun q => q.2.val ≤ nf.val ∧ (q.1 = true ↔ ok0 = true ∧ ∀ g, g < q.2.val → ip.val.getD g false = true))
  · rintro ⟨o, j⟩ ⟨hj, ho⟩
    unfold ledger_wf_loop6.body
    simp only at hj ho ⊢
    split
    · rename_i hot
      have hok := ho.mp hot
      split
      · rename_i hlt
        have hjl : j.val < ip.val.length := by rw [hpre hok.1]; scalar_tac
        step as ⟨b, hb⟩
        step as ⟨j1, hj1⟩
        have hj1v : j1.val = j.val + 1 := by scalar_tac
        have hjn : j.val < nf.val := by scalar_tac
        refine ⟨by omega, ?_, by omega⟩
        have hg : ip.val.getD j.val false = b := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl, Option.getD_some, hb]
        rw [hj1v]
        constructor
        · intro hbt
          refine ⟨hok.1, fun a ha => ?_⟩
          rcases Nat.lt_or_ge a j.val with h3 | h3
          · exact hok.2 a h3
          · have : a = j.val := by omega
            subst this; rw [hg]; exact hbt
        · rintro ⟨_, hp⟩
          have := hp j.val (by omega); rw [hg] at this; exact this
      · rename_i hge
        simp only [WP.spec_ok]
        have : j.val = nf.val := by scalar_tac
        simp only [this] at ho; exact ho
    · rename_i hof
      simp only [WP.spec_ok]
      have : o = false := by simpa using hof
      subst this
      constructor
      · intro h; cases h
      · intro h2; exfalso
        have := ho.mpr ⟨h2.1, fun a ha => h2.2 a (by omega)⟩
        cases this
  · exact ⟨hf, h⟩

end inc

end forge_bool_core
