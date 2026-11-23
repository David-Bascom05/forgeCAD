import ForgeBoolCore.RefineTouch

/-!
# `edges_between`: the edge cells between a vertex and a facet round it

`between` enumerates the cells strictly between `v` and `φ`: `φ` with some of `v`'s other zeros
added (`encB`, `between_iff`). `edges_between` (either branch) returns exactly the edge cells `e`
with `v ≤ e ≤ φ` (`edges_between_spec`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

/-- `pc` with zero `o[j]` added where bit `j` of `m` is set. -/
def encB (pc : List Sgn) (o : List Nat) (m : Nat) : List Sgn :=
  (List.range pc.length).map fun q => if q ∈ o ∧ m.testBit (o.idxOf q) = true then .Zero else pc.getD q .Pos

theorem encB_length (pc : List Sgn) (o : List Nat) (m : Nat) : (encB pc o m).length = pc.length := by
  simp [encB]

theorem encB_getD (pc : List Sgn) (o : List Nat) (m q : Nat) (hq : q < pc.length) :
    (encB pc o m).getD q .Pos = if q ∈ o ∧ m.testBit (o.idxOf q) = true then .Zero else pc.getD q .Pos := by
  simp [encB, List.getD_eq_getElem?_getD, List.getElem?_map, hq]

/-- `le` in getD form. -/
def LeS (x y : List Sgn) : Prop :=
  y.length = x.length ∧ ∀ p, p < x.length → x.getD p .Pos = .Zero ∨ y.getD p .Pos = x.getD p .Pos

/-- **The cells strictly between `vc` and `pc` are the `encB pc o m`, `1 ≤ m < 2^|o|`.** Here `pc` is
a facet above `vc` with its zero at `zp`, and `o` is `vc`'s other zeros. -/
theorem between_iff (vc pc y : List Sgn) (zp : Nat) (hzp : zp < pc.length) (hpz : pc.getD zp .Pos = .Zero)
    (hone : ∀ q, q < pc.length → pc.getD q .Pos = .Zero → q = zp) (hvp : LeS vc pc) :
    (LeS vc y ∧ LeS y pc ∧ y ≠ pc) ↔
      ∃ m, 1 ≤ m ∧ m < 2 ^ ((zerosL vc).filter (· ≠ zp)).length ∧ encB pc ((zerosL vc).filter (· ≠ zp)) m = y := by
  set o := (zerosL vc).filter (· ≠ zp) with ho
  have hod : o.Nodup := (zerosL_nodup vc).filter _
  have hmo : ∀ q, q ∈ o ↔ q < vc.length ∧ vc.getD q .Pos = .Zero ∧ q ≠ zp := by
    intro q; simp [ho, mem_zerosL, and_assoc]
  have hvl : pc.length = vc.length := hvp.1
  constructor
  · rintro ⟨⟨hl1, h1⟩, ⟨hl2, h2⟩, hne⟩
    let bs : List Bool := o.map fun q => y.getD q .Pos == .Zero
    have hbl : bs.length = o.length := by simp [bs]
    have heq : encB pc o (ofBits bs) = y := by
      apply list_ext_getD (by rw [encB_length, ← hl2])
      intro p hp; rw [encB_length] at hp
      rw [encB_getD _ _ _ _ hp]
      by_cases hpo : p ∈ o
      · have hj : o.idxOf p < o.length := List.idxOf_lt_length_iff.mpr hpo
        have htb : (ofBits bs).testBit (o.idxOf p) = (y.getD p .Pos == .Zero) := by
          rw [testBit_ofBits]; simp [bs, List.getD_eq_getElem?_getD, List.getElem?_map, hj, List.getElem_idxOf]
        rw [htb]
        have := h2 p (by omega)
        by_cases hy : y.getD p .Pos = .Zero
        · rw [if_pos ⟨hpo, by rw [hy]; decide⟩, hy]
        · simp only [hpo, true_and, hy, beq_iff_eq, ite_false]
          rcases this with e | e
          · exact absurd e hy
          · exact e
      · simp only [hpo, false_and, ite_false]
        by_cases hpz : p = zp
        · subst hpz
          rw [hpz]
          rcases h2 p (by omega) with e | e
          · exact e.symm
          · rw [hpz] at e; exact e
        · have hvz : vc.getD p .Pos ≠ .Zero := fun e => hpo ((hmo p).mpr ⟨by omega, e, hpz⟩)
          rcases h1 p (by omega) with e | e
          · exact absurd e hvz
          · rcases hvp.2 p (by omega) with e2 | e2
            · exact absurd e2 hvz
            · rw [e, e2]
    refine ⟨ofBits bs, ?_, by rw [← hbl]; exact ofBits_lt bs, heq⟩
    by_contra h0
    have : ofBits bs = 0 := by omega
    apply hne; rw [← heq, this]
    apply list_ext_getD (by rw [encB_length])
    intro p hp; rw [encB_length] at hp; rw [encB_getD _ _ _ _ hp]; simp
  · rintro ⟨m, hm1, hm2, rfl⟩
    refine ⟨⟨by rw [encB_length, hvl], fun p hp => ?_⟩, ⟨by rw [encB_length], fun p hp => ?_⟩, fun e => ?_⟩
    · rw [encB_getD _ _ _ _ (by omega)]
      by_cases hz : vc.getD p .Pos = .Zero
      · exact Or.inl hz
      · right
        have : ¬ (p ∈ o ∧ m.testBit (o.idxOf p) = true) := fun h => hz ((hmo p).mp h.1).2.1
        rw [if_neg this]
        rcases hvp.2 p hp with e | e
        · exact absurd e hz
        · exact e
    · rw [encB_length] at hp
      rw [encB_getD _ _ _ _ hp]
      split
      · exact Or.inl rfl
      · exact Or.inr rfl
    · obtain ⟨b, hb, hbt⟩ : ∃ b, b < o.length ∧ m.testBit b = true := by
        by_contra hn; push_neg at hn
        have : m = 0 := Nat.eq_of_testBit_eq fun b => by
          by_cases hb : b < o.length
          · simpa using hn b hb
          · rw [Nat.testBit_lt_two_pow (Nat.lt_of_lt_of_le hm2 (Nat.pow_le_pow_right (by omega) (by omega)))]; simp
        omega
      have hq : o[b] ∈ o := List.getElem_mem _
      have hqi := (hmo _).mp hq
      have := congrArg (fun l => l.getD o[b] Sgn.Pos) e
      rw [encB_getD _ _ _ _ (by omega), if_pos ⟨hq, by rw [List.Nodup.idxOf_getElem hod]; exact hbt⟩] at this
      exact hqi.2.2 (hone _ (by omega) this.symm)

theorem others_spec (z : Slice Usize) (zp : Usize) (o : alloc.vec.Vec Usize) (j : Usize)
    (hj : j.val ≤ z.val.length)
    (ho : o.val.map (·.val) = ((z.val.map (·.val)).take j.val).filter (· ≠ zp.val)) :
    between_loop0 z zp o j ⦃ r => r.val.map (·.val) = (z.val.map (·.val)).filter (· ≠ zp.val) ⦄ := by
  unfold between_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec Usize × Usize) => z.val.length - q.2.val)
    (fun q => q.2.val ≤ z.val.length ∧ q.1.val.map (·.val) = ((z.val.map (·.val)).take q.2.val).filter (· ≠ zp.val))
  · rintro ⟨o0, i⟩ ⟨hi, ho0⟩
    unfold between_loop0.body
    simp only at hi ho0 ⊢
    split
    · rename_i hlt
      have hil : i.val < (z.val.map (·.val)).length := by simp; scalar_tac
      have ht : (z.val.map (·.val)).take (i.val + 1) = (z.val.map (·.val)).take i.val ++ [(z.val.map (·.val))[i.val]'hil] :=
        List.take_succ_eq_append_getElem hil
      have hol : o0.val.length ≤ i.val := by
        have := congrArg List.length ho0; simp only [List.length_map] at this; rw [this]
        exact (List.length_filter_le _ _).trans (by simp)
      step as ⟨x, hx⟩
      have hxv : x.val = (z.val.map (·.val))[i.val]'hil := by rw [hx]; simp
      split
      · rename_i hne
        step as ⟨o1, ho1⟩
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [ho1, hi1, ht, List.filter_append, ← ho0, ← hxv]
        have : x.val ≠ zp.val := fun e => (by simpa using hne : x ≠ zp) (UScalar.eq_of_val_eq e)
        simp [this]
      · rename_i heq
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hi1, ht, List.filter_append, ← ho0, ← hxv]
        have : x = zp := UScalar.eq_of_val_eq (by simpa using heq)
        simp [this]
    · rename_i hge
      simp only [WP.spec_ok]
      have : i.val = z.val.length := by scalar_tac
      rw [ho0, this, ← List.length_map (f := fun (u : Usize) => u.val), List.take_length]
  · exact ⟨hj, ho⟩

/-- The sign `between` writes at `o[j]`. -/
def bVal (pc : List Sgn) (o : List Nat) (m p : Nat) : Sgn :=
  if m.testBit (o.idxOf p) = true then .Zero else pc.getD p .Pos

theorem set2_step (o : List Nat) (ho : o.Nodup) (f : Nat → Sgn) (pc b : List Sgn) (i : Nat)
    (hil : i < o.length) (hoi : o[i] < pc.length) (hbl : b.length = pc.length)
    (hb1 : ∀ p, p < pc.length → p ∈ o.take i → b.getD p .Pos = f p)
    (hb2 : ∀ p, p < pc.length → p ∉ o → b.getD p .Pos = pc.getD p .Pos) :
    (b.set o[i] (f o[i])).length = pc.length ∧
    (∀ p, p < pc.length → p ∈ o.take (i + 1) → (b.set o[i] (f o[i])).getD p .Pos = f p) ∧
    (∀ p, p < pc.length → p ∉ o → (b.set o[i] (f o[i])).getD p .Pos = pc.getD p .Pos) := by
  have htake : o.take (i + 1) = o.take i ++ [o[i]] := List.take_succ_eq_append_getElem hil
  refine ⟨by simp [hbl], fun p hp hpt => ?_, fun p hp hpo => ?_⟩
  · by_cases hpe : p = o[i]
    · subst hpe
      simp only [List.getD_eq_getElem?_getD, List.getElem?_set_self (by omega : o[i] < b.length), Option.getD_some]
    · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm hpe), ← List.getD_eq_getElem?_getD]
      apply hb1 p hp
      rw [htake] at hpt
      simp only [List.mem_append, List.mem_singleton, hpe, or_false] at hpt
      exact hpt
  · have hpe : p ≠ o[i] := fun e => hpo (e ▸ List.getElem_mem hil)
    rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm hpe), ← List.getD_eq_getElem?_getD]
    exact hb2 p hp hpo

theorem fill_spec (pc : Slice Sgn) (o : alloc.vec.Vec Usize) (k : Usize) (m : U32)
    (hk : k.val = o.val.length) (hk12 : k.val ≤ 12)
    (hod : (o.val.map (·.val)).Nodup) (hol : ∀ q ∈ o.val, q.val < pc.val.length)
    (buf : alloc.vec.Vec Sgn) (j : Usize) (hj : j.val ≤ k.val) (hbl : buf.val.length = pc.val.length)
    (hb1 : ∀ p, p < pc.val.length → p ∈ (o.val.map (·.val)).take j.val →
      buf.val.getD p .Pos = bVal pc.val (o.val.map (·.val)) m.val p)
    (hb2 : ∀ p, p < pc.val.length → p ∉ o.val.map (·.val) → buf.val.getD p .Pos = pc.val.getD p .Pos) :
    between_loop1_loop0 pc o k buf m j ⦃ r =>
      r.val = encB pc.val (o.val.map (·.val)) m.val ⦄ := by
  unfold between_loop1_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec Sgn × Usize) => k.val - q.2.val)
    (fun q => q.2.val ≤ k.val ∧ q.1.val.length = pc.val.length ∧
      (∀ p, p < pc.val.length → p ∈ (o.val.map (·.val)).take q.2.val →
        q.1.val.getD p .Pos = bVal pc.val (o.val.map (·.val)) m.val p) ∧
      (∀ p, p < pc.val.length → p ∉ o.val.map (·.val) → q.1.val.getD p .Pos = pc.val.getD p .Pos))
  · rintro ⟨b0, i⟩ ⟨hi, hbl0, hb0, hb0'⟩
    unfold between_loop1_loop0.body
    simp only at hi hbl0 hb0 hb0' ⊢
    split
    · rename_i hlt
      have hil : i.val < o.val.length := by scalar_tac
      have hil' : i.val < (o.val.map (·.val)).length := by simp; omega
      have hoi := hol _ (List.getElem_mem hil)
      step as ⟨sh, hsh⟩
      step as ⟨an, han⟩
      have hidx : (o.val.map (·.val)).idxOf ((o.val.map (·.val))[i.val]'hil') = i.val := List.Nodup.idxOf_getElem hod _ _
      have hanv : an.val = (m.val >>> i.val) &&& 1 := by rw [han, UScalar.val_and, hsh]; simp
      have e : (an = 1#u32) ↔ m.val.testBit i.val = true := by
        rw [← and_one_testBit, ← hanv]; constructor
        · intro h; rw [h]; rfl
        · intro h; scalar_tac
      have hgs := set2_step _ hod (bVal pc.val (o.val.map (·.val)) m.val) pc.val b0.val i.val hil'
        (by simp; exact hoi) hbl0 hb0 hb0'
      have hoiv : ((o.val.map (·.val))[i.val]'hil') = (o.val[i.val]'hil).val := by simp
      by_cases h1 : an = 1#u32
      · simp only [h1, ite_true, Std.bind_ok]
        step as ⟨i2, hi2⟩
        step as ⟨w, back, hw, hback⟩
        subst hback
        step as ⟨j1, hj1⟩
        have hv : bVal pc.val (o.val.map (·.val)) m.val ((o.val.map (·.val))[i.val]'hil') = .Zero := by
          unfold bVal; rw [hidx, if_pos (e.mp h1)]
        rw [hv, hoiv] at hgs
        refine ⟨by scalar_tac, by simpa [hi2] using hgs.1, ?_, ?_, by scalar_tac⟩
        · intro p hp hpt; rw [alloc.vec.Vec.set_val_eq, hi2]; exact hgs.2.1 p hp (by rw [← hj1]; exact hpt)
        · intro p hp hpo; rw [alloc.vec.Vec.set_val_eq, hi2]; exact hgs.2.2 p hp hpo
      · simp only [h1, ite_false]
        step as ⟨i2, hi2⟩
        step as ⟨sv, hsv⟩
        step as ⟨i3, hi3⟩
        step as ⟨w, back, hw, hback⟩
        subst hback
        step as ⟨j1, hj1⟩
        have hv : bVal pc.val (o.val.map (·.val)) m.val ((o.val.map (·.val))[i.val]'hil') = sv := by
          unfold bVal; rw [hidx, if_neg (fun h => h1 (e.mpr h)), hoiv, hsv]
          subst hi2
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hoi]; rfl
        rw [hv, hoiv] at hgs
        refine ⟨by scalar_tac, by simpa [hi3] using hgs.1, ?_, ?_, by scalar_tac⟩
        · intro p hp hpt; rw [alloc.vec.Vec.set_val_eq, hi3]; exact hgs.2.1 p hp (by rw [← hj1]; exact hpt)
        · intro p hp hpo; rw [alloc.vec.Vec.set_val_eq, hi3]; exact hgs.2.2 p hp hpo
    · rename_i hge
      simp only [WP.spec_ok]
      have hik : i.val = k.val := by scalar_tac
      apply list_ext_getD (by rw [encB_length, hbl0])
      intro p hp; rw [hbl0] at hp
      rw [encB_getD _ _ _ _ hp]
      by_cases hpo : p ∈ o.val.map (·.val)
      · have ht : p ∈ (o.val.map (·.val)).take i.val := by
          rw [hik, hk, ← List.length_map (f := fun (u : Usize) => u.val), List.take_length]; exact hpo
        rw [hb0 p hp ht]; unfold bVal; split <;> simp_all
      · rw [hb0' p hp hpo]; simp [hpo]
  · exact ⟨hj, hbl, hb1, hb2⟩

/-- The cell ids whose covector is `e`. -/
def foundIds (l : Ledger) (ext : Ext) (e : List Sgn) : List Nat :=
  (List.range (allCells l ext).length).filter fun id => covAt l ext id = e

theorem foundIds_found {l : Ledger} {ext : Ext} (hok : ExtOK l ext) {e : List Sgn} {id : Nat}
    (hid : id < (allCells l ext).length) (he : covAt l ext id = e) : foundIds l ext e = [id] := by
  unfold foundIds
  apply eq_single_of_nodup (List.Nodup.filter _ List.nodup_range)
  · simp [hid, he]
  · intro p hp; simp only [List.mem_filter, List.mem_range, decide_eq_true_eq] at hp
    exact covAt_inj l ext hok hp.1 hid (hp.2.trans he.symm)

theorem foundIds_none {l : Ledger} {ext : Ext} {e : List Sgn}
    (hn : ∀ id, id < (allCells l ext).length → covAt l ext id ≠ e) : foundIds l ext e = [] := by
  unfold foundIds; apply List.filter_eq_nil_iff.mpr; intro p hp; simp at hp; simpa using hn p hp

/-- What `between`'s enumeration finds for the first `M` choices (choice 0, `φ` itself, skipped). -/
def foundB (l : Ledger) (ext : Ext) (pc : List Sgn) (o : List Nat) (M : Nat) : List Nat :=
  ((List.range M).drop 1).flatMap fun m => foundIds l ext (encB pc o m)

theorem foundB_succ (l : Ledger) (ext : Ext) (pc : List Sgn) (o : List Nat) (M : Nat) (hM : 1 ≤ M) :
    foundB l ext pc o (M + 1) = foundB l ext pc o M ++ foundIds l ext (encB pc o M) := by
  unfold foundB; rw [List.range_succ, List.drop_append_of_le_length (by simp; omega), List.flatMap_append]; simp

theorem foundIds_length {l : Ledger} {ext : Ext} (hok : ExtOK l ext) (e : List Sgn) : (foundIds l ext e).length ≤ 1 := by
  by_cases hx : ∃ id, id < (allCells l ext).length ∧ covAt l ext id = e
  · obtain ⟨id, hid, he⟩ := hx; rw [foundIds_found hok hid he]; simp
  · rw [foundIds_none (fun id hid he => hx ⟨id, hid, he⟩)]; simp

theorem foundB_length {l : Ledger} {ext : Ext} (hok : ExtOK l ext) (pc : List Sgn) (o : List Nat) :
    ∀ M, (foundB l ext pc o M).length ≤ M
  | 0 => by simp [foundB]
  | 1 => by simp [foundB]
  | M + 2 => by
    rw [foundB_succ _ _ _ _ _ (by omega), List.length_append]
    have := foundB_length hok pc o (M + 1); have := foundIds_length hok (encB pc o (M + 1)); omega

theorem between_m_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (pc : Slice Sgn) (o : alloc.vec.Vec Usize)
    (k : Usize) (combos : U32) (hk : k.val = o.val.length) (hk12 : k.val ≤ 12)
    (hod : (o.val.map (·.val)).Nodup) (hol : ∀ q ∈ o.val, q.val < pc.val.length) (hc : combos.val = 2 ^ k.val)
    (out : alloc.vec.Vec U32) (buf : alloc.vec.Vec Sgn) (m : U32) (hm1 : 1 ≤ m.val) (hm : m.val ≤ combos.val)
    (hbl : buf.val.length = pc.val.length)
    (hb : ∀ p, p < pc.val.length → p ∉ o.val.map (·.val) → buf.val.getD p .Pos = pc.val.getD p .Pos)
    (ho : out.val.map (·.val) = foundB l ext pc.val (o.val.map (·.val)) m.val) :
    between_loop1 l ext pc o out k combos buf m ⦃ r =>
      r.val.map (·.val) = foundB l ext pc.val (o.val.map (·.val)) combos.val ⦄ := by
  unfold between_loop1
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × alloc.vec.Vec Sgn × U32) => combos.val - q.2.2.val)
    (fun q => 1 ≤ q.2.2.val ∧ q.2.2.val ≤ combos.val ∧ q.2.1.val.length = pc.val.length ∧
      (∀ p, p < pc.val.length → p ∉ o.val.map (·.val) → q.2.1.val.getD p .Pos = pc.val.getD p .Pos) ∧
      q.1.val.map (·.val) = foundB l ext pc.val (o.val.map (·.val)) q.2.2.val)
  · rintro ⟨o0, b0, mi⟩ ⟨hmi1, hmi, hbl0, hb0, ho0⟩
    unfold between_loop1.body
    simp only at hmi1 hmi hbl0 hb0 ho0 ⊢
    split
    · rename_i hlt
      step with fill_spec pc o k mi hk hk12 hod hol b0 0#usize (by simp) hbl0 (by simp) hb0 as ⟨b1, hb1⟩
      have hkey : (alloc.vec.Vec.deref b1).val = encB pc.val (o.val.map (·.val)) mi.val := by
        simp [alloc.vec.Vec.deref, hb1]
      step with cells_find_spec l ext hok (alloc.vec.Vec.deref b1) as ⟨id, hid⟩
      rw [hkey] at hid
      obtain ⟨hid1, hid2⟩ := hid
      have hsucc := foundB_succ l ext pc.val (o.val.map (·.val)) mi.val hmi1
      have hol0 : o0.val.length ≤ mi.val := by
        have := congrArg List.length ho0; simp only [List.length_map] at this; rw [this]
        exact foundB_length hok _ _ _
      have hcb : combos.val ≤ 2 ^ 12 := by rw [hc]; exact Nat.pow_le_pow_right (by omega) hk12
      have hb1l : b1.val.length = pc.val.length := by rw [hb1, encB_length]
      have hb1o : ∀ p, p < pc.val.length → p ∉ o.val.map (·.val) → b1.val.getD p .Pos = pc.val.getD p .Pos := by
        intro p hp hpo; rw [hb1, encB_getD _ _ _ _ hp]; simp [hpo]
      split
      · rename_i hne
        have hne' : id ≠ NONE := by simpa using hne
        obtain ⟨hidl, hidc⟩ := hid2 hne'
        step as ⟨o1, ho1⟩
        step as ⟨m1, hm1'⟩
        refine ⟨by scalar_tac, by scalar_tac, hb1l, hb1o, ?_, by scalar_tac⟩
        rw [ho1, hm1', hsucc, foundIds_found hok hidl hidc]; simp [ho0]
      · rename_i hn
        have hn' : id = NONE := UScalar.eq_of_val_eq (by simpa using hn)
        step as ⟨m1, hm1'⟩
        refine ⟨by scalar_tac, by scalar_tac, hb1l, hb1o, ?_, by scalar_tac⟩
        rw [hm1', hsucc, foundIds_none (hid1 hn')]; simp [ho0]
    · rename_i hge
      simp only [WP.spec_ok]
      have : mi.val = combos.val := by scalar_tac
      rw [← this]; exact ho0
  · exact ⟨hm1, hm, hbl, hb, ho⟩

theorem between_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (z : Slice Usize) (phi : U32)
    (hphi : phi.val < (allCells l ext).length) (hzd : (z.val.map (·.val)).Nodup)
    (hzl : ∀ q ∈ z.val, q.val < (covAt l ext phi.val).length) (hz12 : z.val.length ≤ 12) :
    between l ext z phi ⦃ r => r.val.map (·.val) =
      foundB l ext (covAt l ext phi.val) ((z.val.map (·.val)).filter (· ≠ (covAt l ext phi.val).idxOf .Zero))
        (2 ^ ((z.val.map (·.val)).filter (· ≠ (covAt l ext phi.val).idxOf .Zero)).length) ⦄ := by
  unfold between
  step with cov_spec l ext phi hphi as ⟨pc, hpc⟩
  step with zero_idx_spec pc as ⟨zp, hzp⟩
  have hzpv : zp.val = (covAt l ext phi.val).idxOf .Zero := by
    rw [hzp, ForgeExact.DecideOnce.Cov.zeroIdx, covOf, idxOf_map_toS, hpc]
  step with others_spec z zp (alloc.vec.Vec.new Usize) 0#usize (by simp) (by simp) as ⟨o, ho⟩
  rw [hzpv] at ho
  have hol : o.val.length ≤ 12 := by
    have := congrArg List.length ho; simp only [List.length_map] at this; rw [this]
    exact (List.length_filter_le _ _).trans (by simp; omega)
  have hod : (o.val.map (·.val)).Nodup := by rw [ho]; exact hzd.filter _
  have holt : ∀ q ∈ o.val, q.val < pc.val.length := by
    intro q hq
    have : q.val ∈ o.val.map (·.val) := List.mem_map_of_mem hq
    rw [ho] at this
    simp only [List.mem_filter, List.mem_map] at this
    obtain ⟨⟨u, hu, e⟩, _⟩ := this
    rw [hpc, ← e]; exact hzl u hu
  step as ⟨c, hc⟩
  have hcv : c.val = 2 ^ o.val.length := by
    have hsz : U32.size = 2 ^ 32 := by simp [U32.size, U32.numBits]
    have hlv : (alloc.vec.Vec.len o).val = o.val.length := by simp
    rw [hc, hlv, Nat.shiftLeft_eq, Nat.one_mul, hsz]
    apply Nat.mod_eq_of_lt
    have : 2 ^ o.val.length ≤ 2 ^ 12 := Nat.pow_le_pow_right (by omega) hol
    omega
  step with to_vec_sgn_spec pc as ⟨buf, hbuf⟩
  have hlen : (o.val.map (·.val)).length = o.val.length := by simp
  apply WP.spec_mono (between_m_spec l ext hok pc o (alloc.vec.Vec.len o) c (by simp) (by simp; omega) hod holt
    (by simpa using hcv) (alloc.vec.Vec.new U32) buf 1#u32 (by simp) (by rw [hcv]; exact Nat.one_le_two_pow)
    (by rw [hbuf]) (by intro p _ _; rw [hbuf]) (by simp [foundB]))
  intro r hr
  rw [hr, hcv, ← hlen, ho]; simp [hpc]

theorem leS_iff (x y : List Sgn) :
    ForgeExact.DecideOnce.Cov.le (x.map toS) (y.map toS) = true ↔ LeS x y := covle_iff x y

theorem leS_refl (x : List Sgn) : LeS x x := ⟨rfl, fun _ _ => Or.inr rfl⟩

theorem mem_foundB (l : Ledger) (ext : Ext) (pc : List Sgn) (o : List Nat) (M id : Nat) :
    id ∈ foundB l ext pc o M ↔ id < (allCells l ext).length ∧ ∃ m, 1 ≤ m ∧ m < M ∧ covAt l ext id = encB pc o m := by
  simp only [foundB, List.mem_flatMap, foundIds, List.mem_filter, List.mem_range, decide_eq_true_eq]
  constructor
  · rintro ⟨m, hm, hid, he⟩
    have := List.mem_iff_getElem.mp hm
    obtain ⟨i, hi, e⟩ := this
    simp only [List.length_drop, List.length_range] at hi
    simp only [List.getElem_drop, List.getElem_range] at e
    exact ⟨hid, m, by omega, by omega, he⟩
  · rintro ⟨hid, m, hm1, hm2, he⟩
    refine ⟨m, ?_, hid, he⟩
    rw [List.mem_iff_getElem]
    exact ⟨m - 1, by simp; omega, by simp; omega⟩

/-- The cells `edges_between` returns: edge cells between `vc` and `pc`. -/
def EdgeBetween (l : Ledger) (ext : Ext) (isE : List Bool) (vc pc : List Sgn) (e : Nat) : Prop :=
  e < isE.length ∧ isE.getD e false = true ∧ LeS vc (covAt l ext e) ∧ LeS (covAt l ext e) pc

theorem filter_snoc (f : Nat → Bool) (L : List Nat) (a : Nat) :
    (L ++ [a]).filter f = L.filter f ++ (if f a then [a] else []) := by
  rw [List.filter_append]; simp [List.filter_cons]

theorem eb_loop1_spec (isE : Slice Bool) (bs : alloc.vec.Vec U32) (out : alloc.vec.Vec U32) (i : Usize)
    (hbs : ∀ b ∈ bs.val, b.val < isE.val.length) (hbl : bs.val.length < Usize.max) (hi : i.val ≤ bs.val.length)
    (ho : out.val.map (·.val) = ((bs.val.map (·.val)).take i.val).filter (fun e => isE.val.getD e false)) :
    edges_between_loop1 isE out bs i ⦃ r => r.val.map (·.val) =
      (bs.val.map (·.val)).filter (fun e => isE.val.getD e false) ⦄ := by
  unfold edges_between_loop1
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × Usize) => bs.val.length - q.2.val)
    (fun q => q.2.val ≤ bs.val.length ∧
      q.1.val.map (·.val) = ((bs.val.map (·.val)).take q.2.val).filter (fun e => isE.val.getD e false))
  · rintro ⟨o0, j⟩ ⟨hj, ho0⟩
    unfold edges_between_loop1.body
    simp only at hj ho0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < bs.val.length := by scalar_tac
      have hjl' : j.val < (bs.val.map (·.val)).length := by simp; omega
      have ht : (bs.val.map (·.val)).take (j.val + 1) = (bs.val.map (·.val)).take j.val ++ [(bs.val.map (·.val))[j.val]'hjl'] :=
        List.take_succ_eq_append_getElem hjl'
      have hol : o0.val.length ≤ j.val := by
        have := congrArg List.length ho0; simp only [List.length_map] at this; rw [this]
        exact (List.length_filter_le _ _).trans (by simp)
      step as ⟨b, hb⟩
      have hbv : b.val = (bs.val.map (·.val))[j.val]'hjl' := by rw [hb]; simp
      have hbl' : b.val < isE.val.length := by rw [hb]; exact hbs _ (List.getElem_mem _)
      step as ⟨i3, hi3⟩
      have hi3v : i3.val = b.val := by rw [hi3]; scalar_tac
      step as ⟨t, ht'⟩
      have htv : t = isE.val.getD b.val false := by
        rw [ht', List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; simp [hi3v]
      split
      · rename_i htt
        step as ⟨o1, ho1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        have hp : isE.val.getD b.val false = true := by rw [← htv]; simpa using htt
        rw [ho1, hj1, ht, filter_snoc, ← ho0, ← hbv, if_pos hp]; simp
      · rename_i htf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        have hp : ¬ isE.val.getD b.val false = true := by rw [← htv]; simpa using htf
        rw [hj1, ht, filter_snoc, ← ho0, ← hbv, if_neg hp]; simp
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = bs.val.length := by scalar_tac
      rw [ho0, this, ← List.length_map (f := fun (u : U32) => u.val), List.take_length]
  · exact ⟨hi, ho⟩

theorem eb_loop0_spec (l : Ledger) (ext : Ext) (isE : Slice Bool) (vc pc : Slice Sgn)
    (hN : isE.val.length = (allCells l ext).length) (hsmall : (allCells l ext).length < U32.max)
    (out : alloc.vec.Vec U32) (e : Usize) (he : e.val ≤ isE.val.length)
    (ho : out.val.map (·.val) = (List.range e.val).filter fun c =>
      isE.val.getD c false && ForgeExact.DecideOnce.Cov.le (covOf vc) ((covAt l ext c).map toS) &&
        ForgeExact.DecideOnce.Cov.le ((covAt l ext c).map toS) (covOf pc)) :
    edges_between_loop0 l ext isE out vc pc e ⦃ r => r.val.map (·.val) =
      (List.range isE.val.length).filter fun c =>
      isE.val.getD c false && ForgeExact.DecideOnce.Cov.le (covOf vc) ((covAt l ext c).map toS) &&
        ForgeExact.DecideOnce.Cov.le ((covAt l ext c).map toS) (covOf pc) ⦄ := by
  unfold edges_between_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × Usize) => isE.val.length - q.2.val)
    (fun q => q.2.val ≤ isE.val.length ∧ q.1.val.map (·.val) = (List.range q.2.val).filter fun c =>
      isE.val.getD c false && ForgeExact.DecideOnce.Cov.le (covOf vc) ((covAt l ext c).map toS) &&
        ForgeExact.DecideOnce.Cov.le ((covAt l ext c).map toS) (covOf pc))
  · rintro ⟨o0, j⟩ ⟨hj, ho0⟩
    unfold edges_between_loop0.body
    simp only at hj ho0 ⊢
    split
    · rename_i hlt
      have hjl : j.val < isE.val.length := by scalar_tac
      have hol : o0.val.length ≤ j.val := by
        have := congrArg List.length ho0; simp only [List.length_map] at this; rw [this]
        exact (List.length_filter_le _ _).trans (by simp)
      have hsucc := List.range_succ (n := j.val)
      step as ⟨b, hb⟩
      have hbv : b = isE.val.getD j.val false := by
        rw [hb, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hjl]; rfl
      split
      · rename_i hbt
        step as ⟨c1, hc1⟩
        have hc1v : c1.val = j.val := by rw [hc1]; scalar_tac
        step with cov_spec l ext c1 (by omega) as ⟨s1, hs1⟩
        step with le_spec vc s1 as ⟨b1, hb1⟩
        have hb1v : b1 = ForgeExact.DecideOnce.Cov.le (covOf vc) ((covAt l ext j.val).map toS) := by
          simp only [hb1, covOf, hs1, hc1v]
        split
        · rename_i hb1t
          step as ⟨c2, hc2⟩
          have hc2v : c2.val = j.val := by rw [hc2]; scalar_tac
          step with cov_spec l ext c2 (by omega) as ⟨s2, hs2⟩
          step with le_spec s2 pc as ⟨b2, hb2⟩
          have hb2v : b2 = ForgeExact.DecideOnce.Cov.le ((covAt l ext j.val).map toS) (covOf pc) := by
            simp only [hb2, covOf, hs2, hc2v]
          split
          · rename_i hb2t
            step as ⟨c3, hc3⟩
            step as ⟨o1, ho1⟩
            step as ⟨j1, hj1⟩
            refine ⟨by scalar_tac, ?_, by scalar_tac⟩
            have hc3v : c3.val = j.val := by rw [hc3]; scalar_tac
            have hp : (isE.val.getD j.val false && ForgeExact.DecideOnce.Cov.le (covOf vc) ((covAt l ext j.val).map toS) &&
                ForgeExact.DecideOnce.Cov.le ((covAt l ext j.val).map toS) (covOf pc)) = true := by
              rw [← hbv, ← hb1v, ← hb2v]; simp_all
            rw [ho1, hj1, hsucc, filter_snoc, ← ho0, if_pos hp]; simp [hc3v]
          · rename_i hb2f
            step as ⟨j1, hj1⟩
            refine ⟨by scalar_tac, ?_, by scalar_tac⟩
            have hp : ¬ (isE.val.getD j.val false && ForgeExact.DecideOnce.Cov.le (covOf vc) ((covAt l ext j.val).map toS) &&
                ForgeExact.DecideOnce.Cov.le ((covAt l ext j.val).map toS) (covOf pc)) = true := by
              rw [← hbv, ← hb1v, ← hb2v]; simp_all
            rw [hj1, hsucc, filter_snoc, ← ho0, if_neg hp]; simp
        · rename_i hb1f
          step as ⟨j1, hj1⟩
          refine ⟨by scalar_tac, ?_, by scalar_tac⟩
          have hp : ¬ (isE.val.getD j.val false && ForgeExact.DecideOnce.Cov.le (covOf vc) ((covAt l ext j.val).map toS) &&
              ForgeExact.DecideOnce.Cov.le ((covAt l ext j.val).map toS) (covOf pc)) = true := by
            rw [← hbv, ← hb1v]; simp_all
          rw [hj1, hsucc, filter_snoc, ← ho0, if_neg hp]; simp
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        have hp : ¬ (isE.val.getD j.val false && ForgeExact.DecideOnce.Cov.le (covOf vc) ((covAt l ext j.val).map toS) &&
            ForgeExact.DecideOnce.Cov.le ((covAt l ext j.val).map toS) (covOf pc)) = true := by
          rw [← hbv]; simp_all
        rw [hj1, hsucc, filter_snoc, ← ho0, if_neg hp]; simp
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = isE.val.length := by scalar_tac
      rw [ho0, this]
  · exact ⟨he, ho⟩

/-- **`edges_between` returns exactly the edge cells between `v` and the facet `φ` above it.** -/
theorem edges_between_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (isE : Slice Bool) (v phi : U32)
    (z : Slice Usize) (hN : isE.val.length = (allCells l ext).length)
    (hv : v.val < (allCells l ext).length) (hphi : phi.val < (allCells l ext).length)
    (hz : z.val.map (·.val) = zerosL (covAt l ext v.val))
    (hfac : (covAt l ext phi.val).count .Zero = 1) (hvp : LeS (covAt l ext v.val) (covAt l ext phi.val)) :
    edges_between l ext isE v z phi ⦃ r => ∀ e, e ∈ r.val.map (·.val) ↔
      EdgeBetween l ext isE.val (covAt l ext v.val) (covAt l ext phi.val) e ⦄ := by
  have hsmall := hok.small
  unfold edges_between
  simp only
  split
  · step with cov_spec l ext v hv as ⟨vc, hvc⟩
    step with cov_spec l ext phi hphi as ⟨pc, hpc⟩
    apply WP.spec_mono (eb_loop0_spec l ext isE vc pc hN hsmall _ 0#usize (by simp) (by simp))
    intro r hr e
    rw [hr]
    simp only [List.mem_filter, List.mem_range, Bool.and_eq_true, covOf, hvc, hpc, leS_iff, EdgeBetween]
    tauto
  · rename_i hle
    have h12 : z.val.length ≤ 12 := by
      have : ¬ (12 < z.val.length) := by simpa [ENUM_ZEROS] using hle
      omega
    set vc := covAt l ext v.val
    set pc := covAt l ext phi.val
    have hlen : pc.length = vc.length := hvp.1
    have hzd : (z.val.map (·.val)).Nodup := by rw [hz]; exact zerosL_nodup _
    have hzl : ∀ q ∈ z.val, q.val < pc.length := by
      intro q hq
      have : q.val ∈ zerosL vc := by rw [← hz]; exact List.mem_map_of_mem hq
      rw [hlen]; exact ((mem_zerosL _ _).mp this).1
    step with between_spec l ext hok z phi hphi hzd hzl h12 as ⟨bs, hbs⟩
    -- the zero of `φ`
    obtain ⟨p0, hp0, hz0⟩ := count_pos_zero pc hfac
    have hone : ∀ q, q < pc.length → pc.getD q .Pos = .Zero → q = p0 :=
      fun q hq e => one_zero pc hfac q p0 hq hp0 e hz0
    have hzp : pc.idxOf .Zero = p0 := by
      have hm : Sgn.Zero ∈ pc := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hp0, Option.getD_some] at hz0
        rw [← hz0]; exact List.getElem_mem _
      have hi := List.idxOf_lt_length_iff.mpr hm
      apply hone _ hi
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi, Option.getD_some, List.getElem_idxOf]
    rw [hz, hzp] at hbs
    have hmem : ∀ e, e ∈ bs.val.map (·.val) ↔ e < (allCells l ext).length ∧ LeS vc (covAt l ext e) ∧
        LeS (covAt l ext e) pc ∧ covAt l ext e ≠ pc := by
      intro e
      rw [hbs, mem_foundB]
      constructor
      · rintro ⟨he, m, hm1, hm2, hc⟩
        exact ⟨he, (between_iff vc pc _ p0 hp0 hz0 hone hvp).mpr ⟨m, hm1, hm2, hc.symm⟩⟩
      · rintro ⟨he, hb⟩
        obtain ⟨m, hm1, hm2, hc⟩ := (between_iff vc pc _ p0 hp0 hz0 hone hvp).mp hb
        exact ⟨he, m, hm1, hm2, hc.symm⟩
    have hbsl : bs.val.length ≤ 2 ^ 12 := by
      have := congrArg List.length hbs; simp only [List.length_map] at this; rw [this]
      refine (foundB_length hok _ _ _).trans (Nat.pow_le_pow_right (by omega) ?_)
      have := List.length_filter_le (fun x => decide (x ≠ p0)) (zerosL vc)
      have : (zerosL vc).length ≤ 12 := by rw [← hz, List.length_map]; exact h12
      omega
    have hbsN : ∀ b ∈ bs.val, b.val < isE.val.length := by
      intro b hb; rw [hN]; exact ((hmem b.val).mp (List.mem_map_of_mem hb)).1
    step with eb_loop1_spec isE bs (alloc.vec.Vec.new U32) 0#usize hbsN (by scalar_tac) (by simp) (by simp)
      as ⟨out, hout⟩
    step as ⟨i1, hi1⟩
    have hi1v : i1.val = phi.val := by rw [hi1]; scalar_tac
    step as ⟨b, hb⟩
    have hbv : b = isE.val.getD phi.val false := by
      rw [hb, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; simp [hi1v]
    have hcore : ∀ e, (e ∈ bs.val.map (·.val) ∧ isE.val.getD e false = true) ∨ (b = true ∧ e = phi.val) ↔
        EdgeBetween l ext isE.val vc pc e := by
      intro e
      constructor
      · rintro (⟨h1, h2⟩ | ⟨h1, rfl⟩)
        · obtain ⟨he, hl1, hl2, _⟩ := (hmem e).mp h1
          exact ⟨by omega, h2, hl1, hl2⟩
        · exact ⟨by omega, by rw [← hbv]; exact h1, hvp, leS_refl _⟩
      · rintro ⟨he, hie, hl1, hl2⟩
        by_cases hc : covAt l ext e = pc
        · have he' := covAt_inj l ext hok (by omega) hphi hc
          right; refine ⟨?_, he'⟩
          rw [hbv, ← he']; exact hie
        · left; exact ⟨(hmem e).mpr ⟨by omega, hl1, hl2, hc⟩, hie⟩
    have houtl : out.val.length ≤ 2 ^ 12 := by
      have := congrArg List.length hout; simp only [List.length_map] at this; rw [this]
      exact (List.length_filter_le _ _).trans (by simp; omega)
    split
    · rename_i hbt
      step as ⟨o1, ho1⟩
      rename_i e
      rw [← hcore e, ho1, List.map_append, hout]
      simp only [List.mem_append, List.mem_filter, List.map_cons, List.map_nil, List.mem_singleton]
      simp [hbt]
    · rename_i hbf
      simp only [WP.spec_ok]
      intro e
      rw [← hcore e, hout]
      simp [hbf]

end forge_bool_core
