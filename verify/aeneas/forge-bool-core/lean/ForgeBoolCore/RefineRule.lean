import ForgeBoolCore.Funs
open Aeneas Aeneas.Std Result

/-!
# `apply_rule`: the Rust rule, as a Lean rule

`applyL r ms` is what a Rust `Rule` means on a membership vector `ms` (body `i` in iff `ms[i]`):
union, intersection, difference, or a table row `Σ_{ms[i]} 2^i`, out when a member is past the
machine word (`fits`). `apply_rule_spec`: the translation returns `ok (applyL r ms)`, for every
input.
-/

namespace forge_bool_core

/-- The machine word's bits (`usize::BITS`). -/
def nbits : Nat := UScalarTy.Usize.numBits

/-- The table row and whether it fits, over the first `k` bodies. -/
def rowUpTo (ms : List Bool) : Nat → Nat × Bool
  | 0 => (0, true)
  | k + 1 =>
    let acc := rowUpTo ms k
    if ms.getD k false then
      (if nbits ≤ k then (acc.1, false) else (acc.1 ||| (1 <<< k), acc.2))
    else acc

/-- **A Rust rule as a Lean rule** on the membership vector. -/
def applyL : Rule → List Bool → Bool
  | .Union, ms => decide (0 < ms.count true)
  | .Inter, ms => !ms.isEmpty && decide (ms.count true = ms.length)
  | .Diff, ms => !ms.isEmpty && ms.getD 0 false && decide (ms.count true = 1)
  | .Table t, ms => (rowUpTo ms ms.length).2 && decide ((rowUpTo ms ms.length).1 < t.val.length) &&
      t.val.getD (rowUpTo ms ms.length).1 false

theorem count_take_succ (ms : List Bool) (j : Nat) (h : j < ms.length) :
    (ms.take (j + 1)).count true = (ms.take j).count true + (if ms[j] then 1 else 0) := by
  rw [List.take_succ, List.count_append]
  simp [List.getElem?_eq_getElem h]
  cases ms[j] <;> simp

theorem loop0_spec (m : Slice Bool) (ins j : Usize) (hj : j.val ≤ m.val.length)
    (hins : ins.val = (m.val.take j.val).count true) :
    apply_rule_loop0 m ins j ⦃ r => r.val = m.val.count true ⦄ := by
  unfold apply_rule_loop0
  apply loop.spec_decr_nat (fun (q : Usize × Usize) => m.val.length - q.2.val)
    (fun q => q.2.val ≤ m.val.length ∧ q.1.val = (m.val.take q.2.val).count true)
  · rintro ⟨c, i⟩ ⟨hi, hc⟩
    unfold apply_rule_loop0.body
    simp only at hi hc ⊢
    split
    · rename_i h
      have hil : i.val < m.val.length := by scalar_tac
      have hcl : c.val ≤ i.val := by
        rw [hc]; have := List.count_le_length (a := true) (l := m.val.take i.val); simp at this; omega
      have hml : m.val.length ≤ Usize.max := by scalar_tac
      step as ⟨b, hb⟩
      have hb' : m.val[i.val] = b := by rw [hb]
      split
      · rename_i hbt
        step as ⟨c1, hc1⟩
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hi1, hc1, count_take_succ _ _ hil, hb', hbt, hc]; simp
      · rename_i hbt
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, by scalar_tac⟩
        rw [hi1, count_take_succ _ _ hil, hb']; simp [hbt, hc]
    · rename_i h
      have : i.val = m.val.length := by scalar_tac
      simp only [WP.spec_ok]
      rw [hc, this]; simp
  · exact ⟨hj, hins⟩

theorem one_shift_lt {i : Nat} (h : i < nbits) : 1 <<< i < Usize.size := by
  rw [Nat.shiftLeft_eq, Nat.one_mul]
  have : Usize.size = 2 ^ nbits := by rw [Usize.size, Usize.numBits]; rfl
  rw [this]
  exact Nat.pow_lt_pow_right (by omega) h

theorem loop1_spec (m : Slice Bool) (row : Usize) (fits : Bool) (j : Usize) (hj : j.val ≤ m.val.length)
    (hr : row.val = (rowUpTo m.val j.val).1) (hf : fits = (rowUpTo m.val j.val).2) :
    apply_rule_loop1 m row fits j ⦃ r => r.1.val = (rowUpTo m.val m.val.length).1 ∧
      r.2 = (rowUpTo m.val m.val.length).2 ⦄ := by
  unfold apply_rule_loop1
  apply loop.spec_decr_nat (fun (q : Usize × Bool × Usize) => m.val.length - q.2.2.val)
    (fun q => q.2.2.val ≤ m.val.length ∧ q.1.val = (rowUpTo m.val q.2.2.val).1 ∧
      q.2.1 = (rowUpTo m.val q.2.2.val).2)
  · rintro ⟨r0, f0, i⟩ ⟨hi, hr0, hf0⟩
    unfold apply_rule_loop1.body
    simp only at hi hr0 hf0 ⊢
    split
    · rename_i h
      have hil : i.val < m.val.length := by scalar_tac
      step as ⟨b, hb⟩
      have hgd : m.val.getD i.val false = b := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hil, hb]; rfl
      split
      · rename_i hbt
        step as ⟨i2, hi2⟩
        have hi2v : i2.val = nbits := by
          rw [hi2]; simp [core.num.Usize.BITS, nbits]
        split
        · rename_i hge
          step as ⟨i1, hi1⟩
          refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
          · rw [hi1]; simp only [rowUpTo, hgd, hbt, ite_true]
            rw [if_pos (by scalar_tac)]; exact hr0
          · rw [hi1]; simp only [rowUpTo, hgd, hbt, ite_true]
            rw [if_pos (by scalar_tac)]
        · rename_i hlt
          have hlt' : i.val < nbits := by scalar_tac
          step as ⟨i3, hi3⟩
          step as ⟨row2, hrow2⟩
          step as ⟨i1, hi1⟩
          refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
          · rw [hi1]; simp only [rowUpTo, hgd, hbt, ite_true]
            rw [if_neg (by omega)]
            simp only
            rw [hrow2, UScalar.val_or, hi3, Nat.mod_eq_of_lt (one_shift_lt hlt'), hr0]
          · rw [hi1]; simp only [rowUpTo, hgd, hbt, ite_true]
            rw [if_neg (by omega)]; exact hf0
      · rename_i hbt
        step as ⟨i1, hi1⟩
        refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
        · rw [hi1]; simp only [rowUpTo, hgd, hbt]; exact hr0
        · rw [hi1]; simp only [rowUpTo, hgd, hbt]; exact hf0
    · rename_i h
      have : i.val = m.val.length := by scalar_tac
      simp only [WP.spec_ok]
      rw [← this]; exact ⟨hr0, hf0⟩
  · exact ⟨hj, hr, hf⟩

/-- **`apply_rule` is `applyL`**, for every rule and membership vector: no panic, no overflow. -/
theorem apply_rule_spec (r : Rule) (m : Slice Bool) :
    apply_rule r m ⦃ b => b = applyL r m.val ⦄ := by
  unfold apply_rule
  step with loop0_spec m 0#usize 0#usize (by simp) (by simp) as ⟨ins, hins⟩
  cases r with
  | Union =>
    simp only [WP.spec_ok, applyL]
    by_cases h : 0 < m.val.count true
    · simp [h]; scalar_tac
    · simp [h]; scalar_tac
  | Inter =>
    step as ⟨e, he⟩
    split
    · rename_i h
      simp only [WP.spec_ok, applyL]
      have : m.val = [] := by simpa [he] using h
      simp [this]
    · rename_i h
      simp only [WP.spec_ok, applyL]
      have hne : m.val ≠ [] := by simpa [he] using h
      have : (ins = Slice.len m) = (m.val.count true = m.val.length) := by
        apply propext; constructor
        · intro e; rw [← hins]; rw [e]; simp
        · intro e; apply UScalar.eq_of_val_eq; rw [hins, e]; simp
      simp [hne, this]
  | Diff =>
    step as ⟨e, he⟩
    split
    · rename_i h
      simp only [WP.spec_ok, applyL]
      have : m.val = [] := by simpa [he] using h
      simp [this]
    · rename_i h
      have hne : m.val ≠ [] := by simpa [he] using h
      have hl : 0 < m.val.length := List.length_pos_iff.mpr hne
      step as ⟨b0, hb0⟩
      have hg : m.val.getD 0 false = b0 := by
        rw [List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hl, hb0]; rfl
      split
      · rename_i hb
        simp only [WP.spec_ok, applyL, hg, hb]
        have : (ins = 1#usize) = (m.val.count true = 1) := by
          apply propext; constructor
          · intro e; rw [← hins, e]; rfl
          · intro e; apply UScalar.eq_of_val_eq; rw [hins, e]; rfl
        simp [hne, this]
      · rename_i hb
        simp only [WP.spec_ok, applyL, hg]
        simp [hb]
  | Table t =>
    step with loop1_spec m 0#usize true 0#usize (by simp) (by simp [rowUpTo]) (by simp [rowUpTo])
      as ⟨row, fits, hrow, hfits⟩
    split
    · rename_i hf
      split
      · rename_i hlt
        have hlt' : row.val < t.val.length := by scalar_tac
        step as ⟨b, hb⟩
        simp only [applyL, ← hfits, hf, ← hrow, hlt', decide_true, Bool.true_and]
        rw [hb, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem hlt']; simp
      · rename_i hlt
        simp only [WP.spec_ok, applyL, ← hfits, hf, ← hrow]
        have : ¬row.val < t.val.length := by scalar_tac
        simp [this]
    · rename_i hf
      simp only [WP.spec_ok, applyL, ← hfits]
      simp [hf]

end forge_bool_core
