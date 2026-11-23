import ForgeBoolCore.Refine
import Mathlib.Data.Nat.Bitwise

/-!
# The enumeration `around` makes

`x` with zeros `z`: for `keep < |z|` and `m < 2^(|z|-1)`, `encL x z keep m` keeps zero number `keep`
and signs the others by `m`'s bits. Every covector above `x` with exactly one zero is one of them
(`enc_complete`), and only one (`enc_unique`).
-/

namespace forge_bool_core

/-- The number whose bits are `bs`. -/
def ofBits : List Bool → Nat
  | [] => 0
  | b :: bs => (if b then 1 else 0) + 2 * ofBits bs

theorem testBit_ofBits : ∀ (bs : List Bool) (i : Nat), (ofBits bs).testBit i = bs.getD i false
  | [], i => by simp [ofBits]
  | b :: bs, 0 => by
    simp only [ofBits, Nat.testBit_zero, List.getD_cons_zero]
    cases b <;> simp <;> omega
  | b :: bs, i + 1 => by
    rw [Nat.testBit_succ, ofBits, List.getD_cons_succ]
    have : ((if b = true then 1 else 0) + 2 * ofBits bs) / 2 = ofBits bs := by cases b <;> simp <;> omega
    rw [this, testBit_ofBits bs i]

theorem ofBits_lt : ∀ (bs : List Bool), ofBits bs < 2 ^ bs.length
  | [] => by simp [ofBits]
  | b :: bs => by
    have := ofBits_lt bs
    simp only [ofBits, List.length_cons, Nat.pow_succ]
    cases b <;> simp <;> omega

/-- The bit sign `j` (≠ `keep`) reads. -/
def bitOf (j keep : Nat) : Nat := if j < keep then j else j - 1

/-- The zero a bit signs. -/
def jOf (b keep : Nat) : Nat := if b < keep then b else b + 1

theorem jOf_bitOf {j keep : Nat} (h : j ≠ keep) : jOf (bitOf j keep) keep = j := by
  unfold jOf bitOf; by_cases h1 : j < keep <;> simp only [h1, ite_true, ite_false] <;>
    (try split) <;> omega

theorem bitOf_jOf (b keep : Nat) : bitOf (jOf b keep) keep = b := by
  unfold jOf bitOf; by_cases h1 : b < keep <;> simp only [h1, ite_true, ite_false] <;>
    (try split) <;> omega

/-- The enumerated covector. -/
def encL (x : List Sgn) (z : List Nat) (keep m : Nat) : List Sgn :=
  (List.range x.length).map fun p =>
    if p ∈ z then
      (if z.idxOf p = keep then .Zero else if m.testBit (bitOf (z.idxOf p) keep) then .Pos else .Neg)
    else x.getD p .Pos

theorem encL_length (x : List Sgn) (z : List Nat) (keep m : Nat) : (encL x z keep m).length = x.length := by
  simp [encL]

theorem encL_get (x : List Sgn) (z : List Nat) (keep m p : Nat) (hp : p < x.length) :
    (encL x z keep m).getD p .Pos =
      if p ∈ z then
        (if z.idxOf p = keep then .Zero else if m.testBit (bitOf (z.idxOf p) keep) then .Pos else .Neg)
      else x.getD p .Pos := by
  simp [encL, List.getD_eq_getElem?_getD, List.getElem?_map, hp]

theorem list_ext_getD {a b : List Sgn} (hl : a.length = b.length) (h : ∀ p, p < a.length → a.getD p .Pos = b.getD p .Pos) :
    a = b := by
  apply List.ext_getElem hl
  intro p h1 h2
  have := h p h1
  simp only [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h1, List.getElem?_eq_getElem h2,
    Option.getD_some] at this
  exact this

/-- **Every covector above `x` with its one zero among `x`'s zeros is enumerated.** -/
theorem enc_complete (x y : List Sgn) (z : List Nat) (hz : z.Nodup) (hzl : ∀ q ∈ z, q < x.length)
    (hyl : y.length = x.length) (hoff : ∀ p, p < x.length → p ∉ z → y.getD p .Pos = x.getD p .Pos)
    (p0 : Nat) (hp0 : p0 ∈ z) (hy0 : y.getD p0 .Pos = .Zero)
    (hon : ∀ q ∈ z, q ≠ p0 → y.getD q .Pos ≠ .Zero) :
    ∃ keep m, keep < z.length ∧ m < 2 ^ (z.length - 1) ∧ encL x z keep m = y := by
  let keep := z.idxOf p0
  have hkeep : keep < z.length := List.idxOf_lt_length_iff.mpr hp0
  let bs : List Bool := (List.range (z.length - 1)).map fun b => y.getD (z.getD (jOf b keep) 0) .Pos == .Pos
  refine ⟨keep, ofBits bs, hkeep, by have := ofBits_lt bs; simpa [bs] using this, ?_⟩
  apply list_ext_getD (by rw [encL_length, hyl])
  intro p hp
  rw [encL_length] at hp
  rw [encL_get _ _ _ _ _ hp]
  by_cases hpz : p ∈ z
  · rw [if_pos hpz]
    have hj : z.idxOf p < z.length := List.idxOf_lt_length_iff.mpr hpz
    have hzj : z.getD (z.idxOf p) 0 = p := by
      rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hj]; simp [List.getElem_idxOf]
    by_cases hk : z.idxOf p = keep
    · rw [if_pos hk]
      have : p = p0 := by
        have h1 := List.getElem_idxOf hj
        have h2 := List.getElem_idxOf hkeep
        simp only [hk] at h1; rw [← h1, h2]
      rw [this, hy0]
    · rw [if_neg hk, testBit_ofBits]
      have hb : bitOf (z.idxOf p) keep < z.length - 1 := by unfold bitOf; split <;> omega
      simp only [bs, List.getD_eq_getElem?_getD, List.getElem?_map, List.getElem?_range hb, Option.map_some,
        Option.getD_some, jOf_bitOf hk]
      have hzj' : z[z.idxOf p]?.getD 0 = p := by rw [← List.getD_eq_getElem?_getD, hzj]
      rw [hzj', ← List.getD_eq_getElem?_getD]
      have hne : y.getD p .Pos ≠ .Zero := hon p hpz (fun e => hk (by rw [e]))
      cases h : y.getD p .Pos <;> simp_all
  · rw [if_neg hpz, hoff p hp hpz]

/-- **Each enumerated covector comes from one `(keep, m)`.** -/
theorem enc_unique (x : List Sgn) (z : List Nat) (hz : z.Nodup) (hzl : ∀ q ∈ z, q < x.length)
    {keep keep' m m' : Nat} (hk : keep < z.length) (hk' : keep' < z.length)
    (hm : m < 2 ^ (z.length - 1)) (hm' : m' < 2 ^ (z.length - 1))
    (h : encL x z keep m = encL x z keep' m') : keep = keep' ∧ m = m' := by
  have hget : ∀ p (_ : p < x.length), (encL x z keep m).getD p Sgn.Pos = (encL x z keep' m').getD p Sgn.Pos :=
    fun p _ => by rw [h]
  have hzk : z[keep] ∈ z := List.getElem_mem _
  have hidx : ∀ j (hj : j < z.length), z.idxOf z[j] = j := fun j hj => List.Nodup.idxOf_getElem hz j hj
  have e1 := hget z[keep] (hzl _ hzk)
  simp only [encL_get _ _ _ _ _ (hzl _ hzk), if_pos hzk, hidx keep hk, ite_true] at e1
  have hkk : keep = keep' := by
    by_contra hne
    rw [if_neg hne] at e1
    split at e1 <;> cases e1
  subst hkk
  refine ⟨rfl, Nat.eq_of_testBit_eq fun b => ?_⟩
  by_cases hb : b < z.length - 1
  · have hj : jOf b keep < z.length := by unfold jOf; split <;> omega
    have hzj : z[jOf b keep] ∈ z := List.getElem_mem _
    have e2 := hget z[jOf b keep] (hzl _ hzj)
    have hne : jOf b keep ≠ keep := by unfold jOf; split <;> omega
    simp only [encL_get _ _ _ _ _ (hzl _ hzj), if_pos hzj, hidx _ hj, if_neg hne, bitOf_jOf] at e2
    cases h1 : m.testBit b <;> cases h2 : m'.testBit b <;> simp_all
  · rw [Nat.testBit_lt_two_pow (Nat.lt_of_lt_of_le hm (Nat.pow_le_pow_right (by omega) (by omega))),
      Nat.testBit_lt_two_pow (Nat.lt_of_lt_of_le hm' (Nat.pow_le_pow_right (by omega) (by omega)))]

end forge_bool_core
