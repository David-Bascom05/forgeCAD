import ForgeBoolCore.EnumModel
import ForgeBoolCore.RefineExt
import ForgeBoolCore.RefineZeros
import ForgeBoolCore.RefineSortU32
import ForgeBoolCore.RefineKeep

/-!
# `around`: the result facets above a covector

The enumerating branch (at most `ENUM_ZEROS` = 12 zeros) builds each `encL x z keep m`, looks it up,
and keeps its result position; the scanning branch tests every result facet. Both give the sorted
positions `p` with `x ≤ result[p]` (`around_spec`).
-/

open Aeneas Aeneas.Std Result

namespace forge_bool_core

/-- The sign `around_enum` writes at zero `p`. -/
def encV (z : List Nat) (keep m p : Nat) : Sgn :=
  if z.idxOf p = keep then .Zero else if m.testBit (bitOf (z.idxOf p) keep) then .Pos else .Neg

/-- How many bits the first `j` zeros (bar `keep`) used. -/
def bitsUsed (j keep : Nat) : Nat := if j ≤ keep then j else j - 1

theorem and_one_testBit (m b : Nat) : ((m >>> b) &&& 1 = 1) ↔ m.testBit b = true := by
  rw [Nat.and_one_is_mod, Nat.shiftRight_eq_div_pow, Nat.testBit, Nat.shiftRight_eq_div_pow]
  simp [Nat.and_one_is_mod]

theorem buf_set_step (zn : List Nat) (hz : zn.Nodup) (keep m : Nat) (B0 b : List Sgn) (i : Nat)
    (hil : i < zn.length) (hzi : zn[i] < B0.length) (hbl : b.length = B0.length)
    (hb : ∀ p, p < B0.length → b.getD p .Pos =
      if p ∈ zn.take i then encV zn keep m p else B0.getD p .Pos) :
    (b.set zn[i] (encV zn keep m zn[i])).length = B0.length ∧
    ∀ p, p < B0.length → (b.set zn[i] (encV zn keep m zn[i])).getD p .Pos =
      if p ∈ zn.take (i + 1) then encV zn keep m p else B0.getD p .Pos := by
  refine ⟨by simp [hbl], fun p hp => ?_⟩
  have htake : zn.take (i + 1) = zn.take i ++ [zn[i]] := List.take_succ_eq_append_getElem hil
  by_cases hpe : p = zn[i]
  · subst hpe
    rw [htake]
    simp only [List.getD_eq_getElem?_getD, List.getElem?_set_self (by omega : zn[i] < b.length),
      Option.getD_some, List.mem_append, List.mem_singleton, or_true, ite_true]
  · rw [List.getD_eq_getElem?_getD, List.getElem?_set_ne (Ne.symm hpe), ← List.getD_eq_getElem?_getD, hb p hp]
    have hiff : p ∈ zn.take (i + 1) ↔ p ∈ zn.take i := by simp only [htake, List.mem_append, List.mem_singleton, hpe, or_false]
    by_cases hm : p ∈ zn.take i
    · rw [if_pos hm, if_pos (hiff.mpr hm)]
    · rw [if_neg hm, if_neg (fun h => hm (hiff.mp h))]

theorem enum_buf_spec (z : Slice Usize) (k keep : Usize) (m : U32) (B0 : List Sgn)
    (hk : k.val = z.val.length) (hkk : keep.val < k.val) (hk12 : k.val ≤ 13)
    (hz : (z.val.map (·.val)).Nodup) (hzl : ∀ q ∈ z.val, q.val < B0.length)
    (buf : alloc.vec.Vec Sgn) (bit : I32) (j : Usize)
    (hj : j.val ≤ k.val) (hbit : bit.val = (bitsUsed j.val keep.val : Int))
    (hbl : buf.val.length = B0.length)
    (hb : ∀ p, p < B0.length → buf.val.getD p .Pos =
      if p ∈ (z.val.map (·.val)).take j.val then encV (z.val.map (·.val)) keep.val m.val p else B0.getD p .Pos) :
    around_enum_loop0_loop0_loop0 z buf k keep m bit j ⦃ r =>
      r.val.length = B0.length ∧ ∀ p, p < B0.length → r.val.getD p .Pos =
        if p ∈ z.val.map (·.val) then encV (z.val.map (·.val)) keep.val m.val p else B0.getD p .Pos ⦄ := by
  unfold around_enum_loop0_loop0_loop0
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec Sgn × I32 × Usize) => k.val - q.2.2.val)
    (fun q => q.2.2.val ≤ k.val ∧ q.2.1.val = (bitsUsed q.2.2.val keep.val : Int) ∧ q.1.val.length = B0.length ∧
      ∀ p, p < B0.length → q.1.val.getD p .Pos =
        if p ∈ (z.val.map (·.val)).take q.2.2.val then encV (z.val.map (·.val)) keep.val m.val p
        else B0.getD p .Pos)
  · rintro ⟨b0, bt, i⟩ ⟨hi, hbt, hbl0, hb0⟩
    unfold around_enum_loop0_loop0_loop0.body
    simp only at hi hbt hbl0 hb0 ⊢
    split
    · rename_i hlt
      have hil : i.val < z.val.length := by scalar_tac
      have hzi := hzl _ (List.getElem_mem hil)
      split
      · rename_i heq
        step as ⟨ix, hix⟩
        step as ⟨pr, h1, h2⟩
        obtain ⟨v, back⟩ := pr
        simp only at h1 h2 ⊢
        subst h2
        step as ⟨j1, hj1⟩
        have hil' : i.val < (z.val.map (·.val)).length := by simp; omega
        have hixv : ix.val = (z.val.map (·.val))[i.val]'hil' := by rw [hix]; simp
        have hv : encV (z.val.map (·.val)) keep.val m.val ((z.val.map (·.val))[i.val]'hil') = .Zero := by
          unfold encV; rw [List.Nodup.idxOf_getElem hz]; simp [heq]
        have := buf_set_step _ hz keep.val m.val B0 b0.val i.val (by simp; omega) (by simp; omega) hbl0 hb0
        rw [hv, ← hixv] at this
        refine ⟨by scalar_tac, ?_, by simpa using this.1, ?_, by scalar_tac⟩
        · rw [hj1, hbt]; unfold bitsUsed; subst heq; split <;> split <;> scalar_tac
        · intro p hp; rw [hj1]; simpa using this.2 p hp
      · rename_i hne
        have hbt0 : 0 ≤ bt.val := by rw [hbt]; omega
        have hbt32 : bt.val < 32 := by rw [hbt]; unfold bitsUsed; split <;> omega
        step as ⟨sh, hsh⟩
        step as ⟨an, han⟩
        have hil' : i.val < (z.val.map (·.val)).length := by simp; omega
        have hbo : (bt.val).toNat = bitOf i.val keep.val := by
          rw [hbt]; unfold bitsUsed bitOf
          have : i.val ≠ keep.val := fun e => hne (by scalar_tac)
          split <;> split <;> omega
        have hanv : an.val = (m.val >>> bitOf i.val keep.val) &&& 1 := by
          rw [han, UScalar.val_and, hsh, hbo]; simp
        have hs : (if an = 1#u32 then ok Sgn.Pos else ok Sgn.Neg : Result Sgn) =
            ok (encV (z.val.map (·.val)) keep.val m.val ((z.val.map (·.val))[i.val]'hil')) := by
          unfold encV
          rw [List.Nodup.idxOf_getElem hz]
          have : i.val ≠ keep.val := fun e => hne (by scalar_tac)
          rw [if_neg this]
          have e : (an = 1#u32) ↔ m.val.testBit (bitOf i.val keep.val) = true := by
            rw [← and_one_testBit, ← hanv]; constructor
            · intro h; rw [h]; rfl
            · intro h; scalar_tac
          by_cases h1 : an = 1#u32
          · rw [if_pos h1, if_pos (e.mp h1)]
          · rw [if_neg h1, if_neg (fun h => h1 (e.mpr h))]
        rw [hs]
        step as ⟨ix, hix⟩
        step as ⟨pr, h1, h2⟩
        obtain ⟨v, back⟩ := pr
        simp only at h1 h2 ⊢
        subst h2
        step as ⟨bt2, hbt2⟩
        step as ⟨j1, hj1⟩
        have hixv : ix.val = (z.val.map (·.val))[i.val]'hil' := by rw [hix]; simp
        have := buf_set_step _ hz keep.val m.val B0 b0.val i.val (by simp; omega) (by simp; omega) hbl0 hb0
        refine ⟨by scalar_tac, ?_, by simpa using this.1, ?_, by scalar_tac⟩
        · rw [hj1, hbt2, hbt]; unfold bitsUsed
          have : i.val ≠ keep.val := fun e => hne (by scalar_tac)
          split <;> split <;> omega
        · intro p hp; rw [hj1]
          have hix2 : ix.val = (z.val[i.val]).val := by rw [hix]
          simp only [alloc.vec.Vec.set_val_eq, hix2]; simpa using this.2 p hp
    · rename_i hge
      have hik : i.val = k.val := by scalar_tac
      simp only [WP.spec_ok]
      refine ⟨hbl0, fun p hp => ?_⟩
      rw [hb0 p hp, hik, hk, List.take_of_length_le (by simp)]
  · exact ⟨hj, hbit, hbl, hb⟩

/-- What `around` reads from the assembled result: distinct cell ids, their positions in `rpos`, and
facets (one zero each). `W` is unused (kept so signatures stay put). -/
structure ResOK (l : Ledger) (ext : Ext) (a : Assembled) (W : Nat) : Prop where
  nodup : (a.result.val.map (·.val)).Nodup
  lt : ∀ r ∈ a.result.val, r.val < (allCells l ext).length
  small : a.result.val.length < U32.max
  rlen : a.rpos.val.length = (allCells l ext).length
  rpos : ∀ c, c < (allCells l ext).length → (a.rpos.val.getD c NONE).val =
    if c ∈ a.result.val.map (·.val) then (a.result.val.map (·.val)).idxOf c else U32.max
  facet : ∀ r ∈ a.result.val, (covAt l ext r.val).count .Zero = 1

/-- The result cell at position `p`. -/
def resAt (a : Assembled) (p : Nat) : Nat := (a.result.val.map (·.val)).getD p 0

/-- The result positions whose covector is `e`. -/
def hitsAt (l : Ledger) (ext : Ext) (a : Assembled) (e : List Sgn) : List Nat :=
  (List.range a.result.val.length).filter fun p => covAt l ext (resAt a p) = e

theorem resAt_mem (a : Assembled) {p : Nat} (hp : p < a.result.val.length) :
    resAt a p ∈ a.result.val.map (·.val) := by
  unfold resAt; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hp)]
  exact List.getElem_mem _

theorem resAt_lt {l : Ledger} {ext : Ext} {a : Assembled} {W : Nat} (h : ResOK l ext a W) {p : Nat}
    (hp : p < a.result.val.length) : resAt a p < (allCells l ext).length := by
  have := resAt_mem a hp; simp only [List.mem_map] at this
  obtain ⟨r, hr, e⟩ := this; rw [← e]; exact h.lt r hr

theorem eq_single_of_nodup {l : List Nat} {q : Nat} (hn : l.Nodup) (hq : q ∈ l) (hu : ∀ p ∈ l, p = q) :
    l = [q] := by
  match l, hn with
  | [], _ => simp at hq
  | a :: [], _ => rw [hu a (by simp)]
  | a :: b :: t, hn =>
    have ha := hu a (by simp); have hb := hu b (by simp)
    simp only [List.nodup_cons, List.mem_cons, not_or] at hn
    exact absurd (ha.trans hb.symm) hn.1.1

theorem hitsAt_found {l : Ledger} {ext : Ext} {a : Assembled} {W : Nat} (hok : ExtOK l ext) (h : ResOK l ext a W)
    {e : List Sgn} {id : Nat} (hid : id < (allCells l ext).length) (he : covAt l ext id = e) :
    hitsAt l ext a e = if id ∈ a.result.val.map (·.val) then [(a.result.val.map (·.val)).idxOf id] else [] := by
  unfold hitsAt
  have key : ∀ p, p < a.result.val.length → (covAt l ext (resAt a p) = e ↔ resAt a p = id) := by
    intro p hp; constructor
    · intro h1; exact covAt_inj l ext hok (resAt_lt h hp) hid (h1.trans he.symm)
    · intro h1; rw [h1, he]
  split
  · rename_i hm
    have hil := List.idxOf_lt_length_iff.mpr hm
    simp only [List.length_map] at hil
    apply eq_single_of_nodup (List.Nodup.filter _ List.nodup_range)
    · simp only [List.mem_filter, List.mem_range, decide_eq_true_eq]
      refine ⟨hil, (key _ hil).mpr ?_⟩
      unfold resAt; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hil)]
      simp [List.getElem_idxOf]
    · intro p hp
      simp only [List.mem_filter, List.mem_range, decide_eq_true_eq] at hp
      have e1 := (key p hp.1).mp hp.2
      unfold resAt at e1
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hp.1), Option.getD_some] at e1
      rw [← e1, List.Nodup.idxOf_getElem h.nodup]
  · rename_i hm
    apply List.filter_eq_nil_iff.mpr
    intro p hp
    simp only [List.mem_range] at hp
    simp only [decide_eq_true_eq]
    intro h1
    exact hm ((key p hp).mp h1 ▸ resAt_mem a hp)

theorem hitsAt_none {l : Ledger} {ext : Ext} {a : Assembled} {W : Nat} (h : ResOK l ext a W) {e : List Sgn}
    (hn : ∀ id, id < (allCells l ext).length → covAt l ext id ≠ e) : hitsAt l ext a e = [] := by
  unfold hitsAt
  apply List.filter_eq_nil_iff.mpr
  intro p hp
  simp only [List.mem_range] at hp
  simpa using hn _ (resAt_lt h hp)

theorem hitsAt_length {l : Ledger} {ext : Ext} {a : Assembled} {W : Nat} (hok : ExtOK l ext) (h : ResOK l ext a W)
    (e : List Sgn) : (hitsAt l ext a e).length ≤ 1 := by
  by_cases hx : ∃ id, id < (allCells l ext).length ∧ covAt l ext id = e
  · obtain ⟨id, hid, he⟩ := hx
    rw [hitsAt_found hok h hid he]; split <;> simp
  · rw [hitsAt_none h (fun id hid he => hx ⟨id, hid, he⟩)]; simp

/-- The hits of the first `M` sign choices with zero `keep` kept. -/
def hitsK (l : Ledger) (ext : Ext) (a : Assembled) (x : List Sgn) (z : List Nat) (keep M : Nat) : List Nat :=
  (List.range M).flatMap fun m => hitsAt l ext a (encL x z keep m)

theorem hitsK_succ (l : Ledger) (ext : Ext) (a : Assembled) (x : List Sgn) (z : List Nat) (keep M : Nat) :
    hitsK l ext a x z keep (M + 1) = hitsK l ext a x z keep M ++ hitsAt l ext a (encL x z keep M) := by
  unfold hitsK; rw [List.range_succ, List.flatMap_append]; simp

theorem hitsK_length {l : Ledger} {ext : Ext} {a : Assembled} {W : Nat} (hok : ExtOK l ext) (h : ResOK l ext a W)
    (x : List Sgn) (z : List Nat) (keep : Nat) : ∀ M, (hitsK l ext a x z keep M).length ≤ M
  | 0 => by simp [hitsK]
  | M + 1 => by
    rw [hitsK_succ, List.length_append]
    have := hitsK_length hok h x z keep M; have := hitsAt_length hok h (encL x z keep M); omega

theorem encL_getD (x : List Sgn) (z : List Nat) (keep m p : Nat) (hp : p < x.length) :
    (encL x z keep m).getD p .Pos = if p ∈ z then encV z keep m p else x.getD p .Pos := by
  rw [encL_get _ _ _ _ _ hp]; rfl

theorem enum_m_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (x : List Sgn) (z : Slice Usize) (k keep : Usize) (combos : U32)
    (hk : k.val = z.val.length) (hkk : keep.val < k.val) (hk12 : k.val ≤ 12)
    (hz : (z.val.map (·.val)).Nodup) (hzl : ∀ q ∈ z.val, q.val < x.length)
    (hc : combos.val = 2 ^ (k.val - 1))
    (out : alloc.vec.Vec U32) (buf : alloc.vec.Vec Sgn) (m : U32) (O0 : List Nat)
    (hm : m.val ≤ combos.val) (hbl : buf.val.length = x.length)
    (hb : ∀ p, p < x.length → p ∉ z.val.map (·.val) → buf.val.getD p .Pos = x.getD p .Pos)
    (hO : O0.length + combos.val < Usize.max)
    (ho : out.val.map (·.val) = O0 ++ hitsK l ext a x (z.val.map (·.val)) keep.val m.val) :
    around_enum_loop0_loop0 l ext a z out buf k keep combos m ⦃ r =>
      r.1 = a ∧ r.2.2.val.length = x.length ∧
      (∀ p, p < x.length → p ∉ z.val.map (·.val) → r.2.2.val.getD p .Pos = x.getD p .Pos) ∧
      r.2.1.val.map (·.val) = O0 ++ hitsK l ext a x (z.val.map (·.val)) keep.val combos.val ⦄ := by
  unfold around_enum_loop0_loop0
  apply loop.spec_decr_nat
    (fun (q : Assembled × alloc.vec.Vec U32 × alloc.vec.Vec Sgn × U32) => combos.val - q.2.2.2.val)
    (fun q => q.1 = a ∧ q.2.2.2.val ≤ combos.val ∧ q.2.2.1.val.length = x.length ∧
      (∀ p, p < x.length → p ∉ z.val.map (·.val) → q.2.2.1.val.getD p .Pos = x.getD p .Pos) ∧
      q.2.1.val.map (·.val) = O0 ++ hitsK l ext a x (z.val.map (·.val)) keep.val q.2.2.2.val)
  · rintro ⟨a0, o0, b0, mi⟩ ⟨ha, hmi, hbl0, hb0, ho0⟩
    unfold around_enum_loop0_loop0.body
    simp only at ha hmi hbl0 hb0 ho0 ⊢
    subst ha
    split
    · rename_i hlt
      step with enum_buf_spec z k keep mi b0.val hk hkk (by omega) hz (by simpa [hbl0] using hzl) b0 0#i32 0#usize
        (by simp) (by simp [bitsUsed]) rfl (by simp) as ⟨b1, hb1l, hb1⟩
      have hzl' : ∀ q ∈ z.val.map (·.val), q < x.length := by
        intro q hq; simp only [List.mem_map] at hq; obtain ⟨u, hu, e⟩ := hq; rw [← e]; exact hzl u hu
      have hb1e : b1.val = encL x (z.val.map (·.val)) keep.val mi.val := by
        apply list_ext_getD (by rw [encL_length, hb1l, hbl0])
        intro p hp
        rw [hb1l, hbl0] at hp
        rw [hb1 p (by omega), encL_getD _ _ _ _ _ hp]
        split
        · rfl
        · rename_i hpz; exact hb0 p hp hpz
      have hoL : o0.val.length ≤ O0.length + mi.val := by
        have := congrArg List.length ho0; simp only [List.length_map, List.length_append] at this
        have := hitsK_length hok h x (z.val.map (·.val)) keep.val mi.val; omega
      step with cells_find_spec l ext hok b1.deref as ⟨id, hid⟩
      have hkey : (b1.deref).val = encL x (z.val.map (·.val)) keep.val mi.val := by
        simp [alloc.vec.Vec.deref, hb1e]
      rw [hkey] at hid
      obtain ⟨hid1, hid2⟩ := hid
      have hsucc := hitsK_succ l ext a0 x (z.val.map (·.val)) keep.val mi.val
      split
      · rename_i hne
        have hne' : id ≠ NONE := by simpa using hne
        obtain ⟨hidl, hidc⟩ := hid2 hne'
        have hhits := hitsAt_found hok h hidl hidc
        step as ⟨i, hi⟩
        have hiv : i.val = id.val := by rw [hi]; scalar_tac
        have hrl := h.rlen
        step as ⟨i1, hi1⟩
        have hi1v : i1.val = if id.val ∈ a0.result.val.map (·.val) then (a0.result.val.map (·.val)).idxOf id.val
            else U32.max := by
          rw [← h.rpos id.val hidl]
          have : a0.rpos.val.getD id.val NONE = i1 := by
            rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by omega)]; simp [hi1, hiv]
          rw [this]
        split
        · rename_i hne1
          have hin : id.val ∈ a0.result.val.map (·.val) := by
            by_contra hn
            rw [if_neg hn] at hi1v
            exact (by simpa using hne1 : i1 ≠ NONE) (UScalar.eq_of_val_eq (by rw [hi1v, none_val]))
          rw [if_pos hin] at hi1v hhits
          step as ⟨i2, hi2⟩
          step as ⟨i3, hi3⟩
          have hi3v : i3 = i1 := by rw [hi3, hi1]; congr 1; scalar_tac
          step as ⟨o1, ho1⟩
          step as ⟨m1, hm1⟩
          refine ⟨by scalar_tac, by rw [hb1e, encL_length], ?_, ?_, by scalar_tac⟩
          · intro p hp hpz; rw [hb1e, encL_getD _ _ _ _ _ hp, if_neg hpz]
          · rw [ho1, hm1, hsucc, hhits, hi3v]; simp [ho0, hi1v]
        · rename_i hn1
          have hnin : id.val ∉ a0.result.val.map (·.val) := by
            intro hin; rw [if_pos hin] at hi1v
            have := List.idxOf_lt_length_iff.mpr hin
            have hs := h.small
            have : i1 = NONE := UScalar.eq_of_val_eq (by simpa using hn1)
            rw [this, none_val] at hi1v; simp only [List.length_map] at *; omega
          rw [if_neg hnin] at hhits
          step as ⟨m1, hm1⟩
          refine ⟨by scalar_tac, by rw [hb1e, encL_length], ?_, ?_, by scalar_tac⟩
          · intro p hp hpz; rw [hb1e, encL_getD _ _ _ _ _ hp, if_neg hpz]
          · rw [hm1, hsucc, hhits]; simp [ho0]
      · rename_i hn
        have hn' : id = NONE := UScalar.eq_of_val_eq (by simpa using hn)
        have hhits := hitsAt_none h (hid1 hn')
        step as ⟨m1, hm1⟩
        refine ⟨by scalar_tac, by rw [hb1e, encL_length], ?_, ?_, by scalar_tac⟩
        · intro p hp hpz; rw [hb1e, encL_getD _ _ _ _ _ hp, if_neg hpz]
        · rw [hm1, hsucc, hhits]; simp [ho0]
    · rename_i hge
      simp only [WP.spec_ok]
      have : mi.val = combos.val := by scalar_tac
      rw [← this]; exact ⟨trivial, hbl0, hb0, ho0⟩
  · exact ⟨rfl, hm, hbl, hb, ho⟩

/-- Every hit `around_enum` collects: each kept zero, each sign choice. -/
def hitsAll (l : Ledger) (ext : Ext) (a : Assembled) (x : List Sgn) (z : List Nat) (K : Nat) : List Nat :=
  (List.range K).flatMap fun kp => hitsK l ext a x z kp (2 ^ (z.length - 1))

theorem hitsAll_succ (l : Ledger) (ext : Ext) (a : Assembled) (x : List Sgn) (z : List Nat) (K : Nat) :
    hitsAll l ext a x z (K + 1) = hitsAll l ext a x z K ++ hitsK l ext a x z K (2 ^ (z.length - 1)) := by
  unfold hitsAll; rw [List.range_succ, List.flatMap_append]; simp

theorem hitsAll_length {l : Ledger} {ext : Ext} {a : Assembled} {W : Nat} (hok : ExtOK l ext) (h : ResOK l ext a W)
    (x : List Sgn) (z : List Nat) : ∀ K, (hitsAll l ext a x z K).length ≤ K * 2 ^ (z.length - 1)
  | 0 => by simp [hitsAll]
  | K + 1 => by
    rw [hitsAll_succ, List.length_append, Nat.succ_mul]
    have := hitsAll_length hok h x z K; have := hitsK_length hok h x z K (2 ^ (z.length - 1)); omega

theorem enum_keep_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (x : List Sgn) (z : Slice Usize) (k : Usize)
    (hk : k.val = z.val.length) (hk12 : k.val ≤ 12)
    (hz : (z.val.map (·.val)).Nodup) (hzl : ∀ q ∈ z.val, q.val < x.length)
    (out : alloc.vec.Vec U32) (buf : alloc.vec.Vec Sgn) (keep : Usize)
    (hkk : keep.val ≤ k.val) (hbl : buf.val.length = x.length)
    (hb : ∀ p, p < x.length → p ∉ z.val.map (·.val) → buf.val.getD p .Pos = x.getD p .Pos)
    (ho : out.val.map (·.val) = hitsAll l ext a x (z.val.map (·.val)) keep.val) :
    around_enum_loop0 l ext a z out buf k keep ⦃ r =>
      r.val.map (·.val) = hitsAll l ext a x (z.val.map (·.val)) k.val ⦄ := by
  unfold around_enum_loop0
  apply loop.spec_decr_nat
    (fun (q : Assembled × alloc.vec.Vec U32 × alloc.vec.Vec Sgn × Usize) => k.val - q.2.2.2.val)
    (fun q => q.1 = a ∧ q.2.2.2.val ≤ k.val ∧ q.2.2.1.val.length = x.length ∧
      (∀ p, p < x.length → p ∉ z.val.map (·.val) → q.2.2.1.val.getD p .Pos = x.getD p .Pos) ∧
      q.2.1.val.map (·.val) = hitsAll l ext a x (z.val.map (·.val)) q.2.2.2.val)
  · rintro ⟨a0, o0, b0, kp⟩ ⟨ha, hkp, hbl0, hb0, ho0⟩
    unfold around_enum_loop0.body
    simp only at ha hkp hbl0 hb0 ho0 ⊢
    subst ha
    split
    · rename_i hlt
      step as ⟨i, hi⟩
      step as ⟨c, hc⟩
      have hcv : c.val = 2 ^ (k.val - 1) := by
        have hiv : i.val = k.val - 1 := by scalar_tac
        have hsz : U32.size = 2 ^ 32 := by simp [U32.size, U32.numBits]
        rw [hc, hiv, Nat.shiftLeft_eq, Nat.one_mul, hsz]
        apply Nat.mod_eq_of_lt
        have : 2 ^ (k.val - 1) ≤ 2 ^ 11 := Nat.pow_le_pow_right (by omega) (by omega)
        omega
      have hzlen : (z.val.map (·.val)).length = k.val := by simp [hk]
      have hOl := hitsAll_length hok h x (z.val.map (·.val)) kp.val
      rw [hzlen] at hOl
      have hkpl : kp.val * 2 ^ (k.val - 1) + c.val < Usize.max := by
        have : 2 ^ (k.val - 1) ≤ 2 ^ 11 := Nat.pow_le_pow_right (by omega) (by omega)
        have : kp.val * 2 ^ (k.val - 1) ≤ 12 * 2 ^ 11 := Nat.mul_le_mul (by omega) this
        have := u32_le_usize; have := U32.max_eq; omega
      step with enum_m_spec l ext hok a0 W h x z k kp c hk hlt hk12 hz hzl hcv o0 b0 0#u32
        (hitsAll l ext a0 x (z.val.map (·.val)) kp.val) (by simp) hbl0 hb0 (by omega) (by simp [ho0, hitsK])
        as ⟨r, o1, b1, hr, hbl1, hb1, ho1⟩
      step as ⟨kp1, hkp1⟩
      refine ⟨hr, by scalar_tac, hbl1, hb1, ?_, by scalar_tac⟩
      rw [ho1, hkp1, hitsAll_succ, hzlen, ← hcv]
    · rename_i hge
      simp only [WP.spec_ok]
      have : kp.val = k.val := by scalar_tac
      rw [← this]; exact ho0
  · exact ⟨rfl, hkk, hbl, hb, ho⟩

theorem covle_cons (a b : Sgn) (x y : List Sgn) :
    ForgeExact.DecideOnce.Cov.le ((a :: x).map toS) ((b :: y).map toS) =
      ((a == .Zero || a == b) && ForgeExact.DecideOnce.Cov.le (x.map toS) (y.map toS)) := by
  simp only [ForgeExact.DecideOnce.Cov.le, List.map_cons, List.length_cons, List.zip_cons_cons, List.all_cons,
    List.length_map]
  have e : (x.length + 1 == y.length + 1) = (x.length == y.length) := by simp
  rw [e]
  generalize (x.length == y.length) = u
  generalize (((List.map toS x).zip (List.map toS y)).all fun p => p.1 == ForgeExact.DecideOnce.Sgn.zero || p.1 == p.2) = v
  cases a <;> cases b <;> cases u <;> cases v <;> decide

theorem covle_iff : ∀ (x y : List Sgn), ForgeExact.DecideOnce.Cov.le (x.map toS) (y.map toS) = true ↔
    y.length = x.length ∧ ∀ p, p < x.length → x.getD p .Pos = .Zero ∨ y.getD p .Pos = x.getD p .Pos
  | [], [] => by simp [ForgeExact.DecideOnce.Cov.le]
  | [], _ :: _ => by simp [ForgeExact.DecideOnce.Cov.le]
  | _ :: _, [] => by simp [ForgeExact.DecideOnce.Cov.le]
  | a :: x, b :: y => by
    rw [covle_cons, Bool.and_eq_true, covle_iff x y]
    constructor
    · rintro ⟨h1, h2, h3⟩
      refine ⟨by simp [h2], fun p hp => ?_⟩
      cases p with
      | zero => simp only [List.getD_cons_zero]; cases a <;> cases b <;> simp_all
      | succ p => simp only [List.getD_cons_succ]; exact h3 p (by simp at hp; omega)
    · rintro ⟨h1, h2⟩
      refine ⟨?_, by simpa using h1, fun p hp => by simpa using h2 (p + 1) (by simp; omega)⟩
      have := h2 0 (by simp); simp only [List.getD_cons_zero] at this
      cases a <;> cases b <;> simp_all

theorem mem_zerosL (x : List Sgn) (p : Nat) : p ∈ zerosL x ↔ p < x.length ∧ x.getD p .Pos = .Zero := by
  simp [zerosL]

theorem zerosL_nodup (x : List Sgn) : (zerosL x).Nodup := List.Nodup.filter _ List.nodup_range

theorem one_zero : ∀ (y : List Sgn), y.count .Zero = 1 → ∀ q r, q < y.length → r < y.length →
    y.getD q .Pos = .Zero → y.getD r .Pos = .Zero → q = r
  | [], _ => by simp
  | a :: t, h => by
    intro q r hq hr h1 h2
    by_cases ha : a = .Zero
    · subst ha
      have h0 : t.count .Zero = 0 := by simp [List.count_cons] at h; omega
      have nz : ∀ i, i < t.length → t.getD i .Pos ≠ .Zero := by
        intro i hi e
        have : Sgn.Zero ∈ t := by
          rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi, Option.getD_some] at e
          rw [← e]; exact List.getElem_mem _
        exact (List.count_eq_zero.mp h0) this
      cases q with
      | zero => cases r with
        | zero => rfl
        | succ r => exact absurd h2 (nz r (by simp at hr; omega))
      | succ q => exact absurd h1 (nz q (by simp at hq; omega))
    · have h1' : t.count .Zero = 1 := by
        simp [List.count_cons] at h
        simpa [ha] using h
      cases q with
      | zero => simp at h1; exact absurd h1 ha
      | succ q => cases r with
        | zero => simp at h2; exact absurd h2 ha
        | succ r =>
          simp only [List.getD_cons_succ] at h1 h2
          have := one_zero t h1' q r (by simp at hq; omega) (by simp at hr; omega) h1 h2
          omega

theorem count_pos_zero (y : List Sgn) (h : y.count .Zero = 1) : ∃ p, p < y.length ∧ y.getD p .Pos = .Zero := by
  have : Sgn.Zero ∈ y := List.count_pos_iff.mp (by omega)
  obtain ⟨i, hi, e⟩ := List.getElem_of_mem this
  exact ⟨i, hi, by rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hi, Option.getD_some, e]⟩

/-- **The enumerated covectors are exactly the facets above `x`.** -/
theorem enc_iff (x y : List Sgn) (hy : y.count .Zero = 1) :
    ForgeExact.DecideOnce.Cov.le (x.map toS) (y.map toS) = true ↔
      ∃ kp m, kp < (zerosL x).length ∧ m < 2 ^ ((zerosL x).length - 1) ∧ encL x (zerosL x) kp m = y := by
  rw [covle_iff]
  constructor
  · rintro ⟨hl, hle⟩
    obtain ⟨p0, hp0, hz0⟩ := count_pos_zero y hy
    rw [hl] at hp0
    have hp0z : p0 ∈ zerosL x := by
      rw [mem_zerosL]; refine ⟨hp0, ?_⟩
      rcases hle p0 hp0 with e | e
      · exact e
      · rw [← e, hz0]
    apply enc_complete x y (zerosL x) (zerosL_nodup x) (fun q hq => ((mem_zerosL x q).mp hq).1) hl
    · intro p hp hpz
      rcases hle p hp with e | e
      · exact absurd ((mem_zerosL x p).mpr ⟨hp, e⟩) hpz
      · exact e
    · exact hp0z
    · exact hz0
    · intro q hq hne e
      have hql := ((mem_zerosL x q).mp hq).1
      exact hne (one_zero y hy q p0 (by omega) (by omega) e hz0)
  · rintro ⟨kp, m, _, _, rfl⟩
    refine ⟨encL_length _ _ _ _, fun p hp => ?_⟩
    rw [encL_getD _ _ _ _ _ hp]
    split
    · rename_i hpz; exact Or.inl ((mem_zerosL x p).mp hpz).2
    · exact Or.inr rfl

theorem resAt_facet {l : Ledger} {ext : Ext} {a : Assembled} {W : Nat} (h : ResOK l ext a W) {p : Nat}
    (hp : p < a.result.val.length) : (covAt l ext (resAt a p)).count .Zero = 1 := by
  have := resAt_mem a hp; simp only [List.mem_map] at this
  obtain ⟨r, hr, e⟩ := this; rw [← e]; exact h.facet r hr

theorem mem_hitsAt (l : Ledger) (ext : Ext) (a : Assembled) (e : List Sgn) (p : Nat) :
    p ∈ hitsAt l ext a e ↔ p < a.result.val.length ∧ covAt l ext (resAt a p) = e := by
  simp [hitsAt]

theorem mem_hitsAll {l : Ledger} {ext : Ext} {a : Assembled} {W : Nat} (h : ResOK l ext a W) (x : List Sgn) (p : Nat) :
    p ∈ hitsAll l ext a x (zerosL x) (zerosL x).length ↔
      p < a.result.val.length ∧
        ForgeExact.DecideOnce.Cov.le (x.map toS) ((covAt l ext (resAt a p)).map toS) = true := by
  simp only [hitsAll, hitsK, List.mem_flatMap, List.mem_range, mem_hitsAt]
  constructor
  · rintro ⟨kp, hkp, m, hm, hp, he⟩
    exact ⟨hp, (enc_iff x _ (resAt_facet h hp)).mpr ⟨kp, m, hkp, hm, he.symm⟩⟩
  · rintro ⟨hp, hle⟩
    obtain ⟨kp, m, hkp, hm, he⟩ := (enc_iff x _ (resAt_facet h hp)).mp hle
    exact ⟨kp, hkp, m, hm, hp, he.symm⟩

theorem hitsAll_nodup (l : Ledger) (ext : Ext) (a : Assembled) (x : List Sgn) :
    (hitsAll l ext a x (zerosL x) (zerosL x).length).Nodup := by
  have hz := zerosL_nodup x
  have hzl : ∀ q ∈ zerosL x, q < x.length := fun q hq => ((mem_zerosL x q).mp hq).1
  unfold hitsAll
  rw [List.nodup_flatMap]
  refine ⟨fun kp hkp => ?_, ?_⟩
  · unfold hitsK
    rw [List.nodup_flatMap]
    refine ⟨fun m _ => List.Nodup.filter _ List.nodup_range, ?_⟩
    apply List.Nodup.pairwise_of_forall_ne List.nodup_range
    intro m hm m' hm' hne
    simp only [List.mem_range] at hm hm' hkp
    intro p hp1 hp2
    rw [mem_hitsAt] at hp1 hp2
    exact hne (enc_unique x _ hz hzl hkp hkp hm hm' (hp1.2.symm.trans hp2.2)).2
  · apply List.Nodup.pairwise_of_forall_ne List.nodup_range
    intro kp hkp kp' hkp' hne
    simp only [List.mem_range] at hkp hkp'
    intro p hp1 hp2
    simp only [hitsK, List.mem_flatMap, List.mem_range, mem_hitsAt] at hp1 hp2
    obtain ⟨m, hm, _, e1⟩ := hp1
    obtain ⟨m', hm', _, e2⟩ := hp2
    exact hne (enc_unique x _ hz hzl hkp hkp' hm hm' (e1.symm.trans e2)).1

theorem sorted_eq_filter (r out : List Nat) (f : Nat → Bool) (n : Nat) (hperm : r.Perm out)
    (hsort : r.Pairwise (· ≤ ·)) (hnd : out.Nodup) (hmem : ∀ p, p ∈ out ↔ p < n ∧ f p = true) :
    r = (List.range n).filter f := by
  have hf : ((List.range n).filter f).Pairwise (· ≤ ·) :=
    (List.pairwise_lt_range.filter f).imp (fun h => Nat.le_of_lt h)
  apply List.Perm.eq_of_pairwise' hsort hf
  apply hperm.trans
  apply (List.perm_ext_iff_of_nodup hnd (List.Nodup.filter _ List.nodup_range)).mpr
  intro p; rw [hmem]; simp

/-- The positions `around` returns: the result facets above `x`. -/
def aroundL (l : Ledger) (ext : Ext) (a : Assembled) (x : List Sgn) : List Nat :=
  (List.range a.result.val.length).filter fun p =>
    ForgeExact.DecideOnce.Cov.le (x.map toS) ((covAt l ext (resAt a p)).map toS)

theorem around_enum_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (xc : Slice Sgn) (z : Slice Usize) (hzv : z.val.map (·.val) = zerosL xc.val) (hk12 : z.val.length ≤ 12) :
    around_enum l ext a xc z (alloc.vec.Vec.new U32) ⦃ r => r.val.map (·.val) = aroundL l ext a xc.val ⦄ := by
  unfold around_enum
  step with to_vec_sgn_spec xc as ⟨buf, hbuf⟩
  have hzl : ∀ q ∈ z.val, q.val < xc.val.length := by
    intro q hq
    have : q.val ∈ zerosL xc.val := by rw [← hzv]; exact List.mem_map_of_mem hq
    exact ((mem_zerosL _ _).mp this).1
  have hz : (z.val.map (·.val)).Nodup := by rw [hzv]; exact zerosL_nodup _
  step with enum_keep_spec l ext hok a W h xc.val z (Slice.len z) (by simp) (by simp; omega) hz hzl
    (alloc.vec.Vec.new U32) buf 0#usize (by simp) (by simp [hbuf]) (by intro p _ _; simp [hbuf])
    (by simp [hitsAll]) as ⟨o, ho⟩
  have hzlen : (Slice.len z).val = (zerosL xc.val).length := by
    simp only [Slice.len_val]; rw [← hzv, List.length_map]
  rw [hzv, hzlen] at ho
  have hol : o.val.length ≤ 12 * 2 ^ 11 := by
    have := congrArg List.length ho; simp only [List.length_map] at this; rw [this]
    have := hitsAll_length hok h xc.val (zerosL xc.val) (zerosL xc.val).length
    have h1 : (zerosL xc.val).length ≤ 12 := by rw [← hzv, List.length_map]; exact hk12
    have : 2 ^ ((zerosL xc.val).length - 1) ≤ 2 ^ 11 := Nat.pow_le_pow_right (by omega) (by omega)
    have : (zerosL xc.val).length * 2 ^ ((zerosL xc.val).length - 1) ≤ 12 * 2 ^ 11 := Nat.mul_le_mul h1 this
    omega
  step with sort_u32_spec o (by have := u32_le_usize; have := U32.max_eq; omega) as ⟨r, hperm, hsort⟩
  apply sorted_eq_filter _ (o.val.map (·.val)) _ _ (hperm.map _) _ (by rw [ho]; exact hitsAll_nodup l ext a xc.val)
  · intro p; rw [ho]; exact mem_hitsAll h xc.val p
  · exact List.pairwise_map.mpr (hsort.imp fun h => h)

theorem around_loop_spec (l : Ledger) (ext : Ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (xc : Slice Sgn) (out : alloc.vec.Vec U32) (p : Usize) (hp : p.val ≤ a.result.val.length)
    (ho : out.val.map (·.val) = (List.range p.val).filter fun q =>
      ForgeExact.DecideOnce.Cov.le (xc.val.map toS) ((covAt l ext (resAt a q)).map toS)) :
    around_loop l ext a xc out p ⦃ r => r.val.map (·.val) = aroundL l ext a xc.val ⦄ := by
  unfold around_loop
  apply loop.spec_decr_nat
    (fun (q : alloc.vec.Vec U32 × Usize) => a.result.val.length - q.2.val)
    (fun q => q.2.val ≤ a.result.val.length ∧ q.1.val.map (·.val) = (List.range q.2.val).filter fun q =>
      ForgeExact.DecideOnce.Cov.le (xc.val.map toS) ((covAt l ext (resAt a q)).map toS))
  · rintro ⟨o0, j⟩ ⟨hj, ho0⟩
    unfold around_loop.body
    simp only at hj ho0 ⊢
    have hs := h.small
    split
    · rename_i hlt
      have hjl : j.val < a.result.val.length := by scalar_tac
      step as ⟨i1, hi1⟩
      have hi1v : i1.val = resAt a j.val := by
        unfold resAt; rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem (by simpa using hjl)]; simp [hi1]
      have hi1l : i1.val < (allCells l ext).length := by rw [hi1v]; exact resAt_lt h hjl
      step with cov_spec l ext i1 hi1l as ⟨sv, hsv⟩
      step with le_spec xc sv as ⟨b, hb⟩
      have hsucc : (List.range (j.val + 1)).filter (fun q =>
          ForgeExact.DecideOnce.Cov.le (xc.val.map toS) ((covAt l ext (resAt a q)).map toS)) =
          (List.range j.val).filter (fun q =>
          ForgeExact.DecideOnce.Cov.le (xc.val.map toS) ((covAt l ext (resAt a q)).map toS)) ++
          (if ForgeExact.DecideOnce.Cov.le (xc.val.map toS) ((covAt l ext (resAt a j.val)).map toS) then [j.val] else []) := by
        rw [List.range_succ, List.filter_append]; simp [List.filter_cons]
      have hbv : b = ForgeExact.DecideOnce.Cov.le (xc.val.map toS) ((covAt l ext (resAt a j.val)).map toS) := by
        rw [hb, covOf, covOf, hsv, hi1v]
      have hol : o0.val.length ≤ j.val := by
        have := congrArg List.length ho0; simp only [List.length_map] at this; rw [this]
        exact (List.length_filter_le _ _).trans (by simp)
      split
      · rename_i hbt
        step as ⟨c, hc⟩
        step as ⟨o1, ho1⟩
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [ho1, hj1, hsucc, ← hbv, if_pos hbt]; simp [ho0, hc]; scalar_tac
      · rename_i hbf
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hj1, hsucc, ← hbv, if_neg hbf]; simp [ho0]
    · rename_i hge
      simp only [WP.spec_ok]
      have : j.val = a.result.val.length := by scalar_tac
      rw [ho0, this]; rfl
  · exact ⟨hp, ho⟩

/-- **`around` returns, in order, the positions of the result facets above `x`.** -/
theorem around_spec (l : Ledger) (ext : Ext) (hok : ExtOK l ext) (a : Assembled) (W : Nat) (h : ResOK l ext a W)
    (x : U32) (hx : x.val < (allCells l ext).length) :
    around l ext a x ⦃ r => r.val.map (·.val) = aroundL l ext a (covAt l ext x.val) ⦄ := by
  unfold around
  step with cov_spec l ext x hx as ⟨xc, hxc⟩
  step with zeros_spec xc as ⟨z, hz⟩
  split
  · apply WP.spec_mono (around_loop_spec l ext a W h xc _ 0#usize (by simp) (by simp))
    intro r hr; rw [hr, hxc]
  · rename_i hle
    have h12 : z.val.length ≤ 12 := by
      have : ¬ (12 < z.val.length) := by simpa [ENUM_ZEROS] using hle
      omega
    apply WP.spec_mono (around_enum_spec l ext hok a W h xc (alloc.vec.Vec.deref z) (by simpa [alloc.vec.Vec.deref] using hz)
      (by simpa [alloc.vec.Vec.deref] using h12))
    intro r hr; rw [hr, hxc]

end forge_bool_core
