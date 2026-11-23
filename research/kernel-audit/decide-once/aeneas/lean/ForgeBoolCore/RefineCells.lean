import ForgeBoolCore.Refine
open Aeneas Aeneas.Std Result

/-!
# Step 1: covectors as Rust slices

`with_set` is `List.set`, `same_cov` is equality, and `cmp_cov` is the lexicographic order
`cmpL` on covectors (by rank: `Neg < Zero < Pos`, a proper prefix first).
-/

namespace forge_bool_core

theorem slice_val {α : Type} (v : alloc.vec.Vec α) : v.slice.val = v.val := rfl

/-- `X.set i s`. -/
theorem with_set_spec (x : Slice Sgn) (i : Usize) (s : Sgn) :
    with_set x i s ⦃ v => v.val = x.val.set i.val s ⦄ := by
  unfold with_set
  have hclone : ∀ y ∈ x.val, Sgn.Insts.CoreCloneClone.clone y = ok y := by
    intro y _; cases y <;> rfl
  step as ⟨v, hv⟩
  subst hv
  split
  · rename_i h
    step as ⟨_, back, _⟩
    rename_i hb
    rw [hb]; simp only [alloc.vec.Vec.set_val_eq]; rfl
  · rename_i h
    simp only [WP.spec_ok]
    have : v.val.length ≤ i.val := by scalar_tac
    show v.val = v.val.set i.val s
    rw [List.set_eq_of_length_le this]

theorem same_cov_loop_spec (a b : Slice Sgn) (same : Bool) (i : Usize) (hi : i.val ≤ a.val.length)
    (hl : same = true → a.val.length = b.val.length) :
    same_cov_loop a b same i ⦃ r => r = (same && a.val.drop i.val == b.val.drop i.val) ⦄ := by
  unfold same_cov_loop
  apply loop.spec_decr_nat (fun (q : Bool × Usize) => a.val.length - q.2.val)
    (fun q => q.2.val ≤ a.val.length ∧ (q.1 = true → a.val.length = b.val.length) ∧
      (q.1 && a.val.drop q.2.val == b.val.drop q.2.val) = (same && a.val.drop i.val == b.val.drop i.val))
  · rintro ⟨o, j⟩ ⟨hj, hlj, heq⟩
    unfold same_cov_loop.body
    simp only at hj hlj heq ⊢
    cases o with
    | false => simp only [Bool.false_eq_true, ite_false, WP.spec_ok]; rw [← heq]; rfl
    | true =>
      simp only [ite_true]
      have hab := hlj rfl
      split
      · rename_i h
        have hja : j.val < a.val.length := by scalar_tac
        step as ⟨s, hs⟩
        step as ⟨t, ht⟩
        rw [sgn_eq]; simp only [Std.bind_ok]
        step as ⟨j1, hj1⟩
        refine ⟨by scalar_tac, ?_, ?_, by scalar_tac⟩
        · intro _; exact hab
        · rw [← heq, hj1, drop_cons a.val j.val hja, drop_cons b.val j.val (by omega)]
          have hs' : s = a.val[j.val]! := by rw [hs, getElem!_pos a.val j.val hja]
          have ht' : t = b.val[j.val]! := by rw [ht, getElem!_pos b.val j.val (by omega)]
          rw [← hs', ← ht']
          by_cases hst : s = t
          · subst hst; simp
          · simp [hst]
      · rename_i h
        simp only [WP.spec_ok]
        have : j.val = a.val.length := by scalar_tac
        rw [← heq, this]; simp [List.drop_of_length_le (by omega : b.val.length ≤ a.val.length)]
  · exact ⟨hi, hl, rfl⟩

/-- **`same_cov` is equality.** -/
theorem same_cov_spec (a b : Slice Sgn) : same_cov a b ⦃ r => r = decide (a.val = b.val) ⦄ := by
  unfold same_cov
  have hl : decide (Slice.len a = Slice.len b) = true → a.val.length = b.val.length := by
    intro h; simp at h; scalar_tac
  have := same_cov_loop_spec a b (decide (Slice.len a = Slice.len b)) 0#usize (by simp) hl
  apply WP.spec_mono this
  intro r hr
  rw [hr]
  by_cases h : a.val = b.val
  · have : Slice.len a = Slice.len b := by apply UScalar.eq_of_val_eq; simp [h]
    simp [h, this]
  · by_cases h2 : a.val.length = b.val.length
    · have : Slice.len a = Slice.len b := by apply UScalar.eq_of_val_eq; simp [h2]
      simp [h, this]
    · have : Slice.len a ≠ Slice.len b := by intro e; apply h2; have := congrArg (·.val) e; simpa using this
      simp [h, this]

/-! ## The order on covectors -/

def rk : Sgn → Nat
  | .Neg => 0
  | .Zero => 1
  | .Pos => 2

/-- Lexicographic comparison by rank; a proper prefix comes first. -/
def cmpL : List Sgn → List Sgn → Int
  | [], [] => 0
  | [], _ :: _ => -1
  | _ :: _, [] => 1
  | x :: xs, y :: ys => if rk x < rk y then -1 else if rk y < rk x then 1 else cmpL xs ys

theorem rank_spec (s : Sgn) : sort.rank s ⦃ r => r.val = rk s ⦄ := by
  cases s <;> simp [sort.rank, rk]

theorem rk_inj {a b : Sgn} (h : rk a = rk b) : a = b := by
  cases a <;> cases b <;> simp_all [rk]

theorem cmpL_drop_cons (a b : List Sgn) (i : Nat) (ha : i < a.length) (hb : i < b.length) :
    cmpL (a.drop i) (b.drop i) =
      if rk a[i]! < rk b[i]! then -1 else if rk b[i]! < rk a[i]! then 1
      else cmpL (a.drop (i + 1)) (b.drop (i + 1)) := by
  rw [drop_cons a i ha, drop_cons b i hb]; rfl

theorem cmpL_vals (a b : List Sgn) : cmpL a b = -1 ∨ cmpL a b = 0 ∨ cmpL a b = 1 := by
  induction a generalizing b with
  | nil => cases b <;> simp [cmpL]
  | cons x xs ih =>
    cases b with
    | nil => simp [cmpL]
    | cons y ys =>
      simp only [cmpL]
      split
      · simp
      · split
        · simp
        · exact ih ys

theorem cmp_cov_loop_spec (a b : Slice Sgn) (o : I8) (i : Usize)
    (hi : i.val ≤ a.val.length ∧ i.val ≤ b.val.length)
    (ho : (o.val = 0 ∧ cmpL a.val b.val = cmpL (a.val.drop i.val) (b.val.drop i.val)) ∨
      (o.val ≠ 0 ∧ o.val = cmpL a.val b.val)) :
    sort.cmp_cov_loop a b o i ⦃ r =>
      (r.val = 0 ∧ ∃ j, j ≤ a.val.length ∧ j ≤ b.val.length ∧ (j = a.val.length ∨ j = b.val.length) ∧
        cmpL a.val b.val = cmpL (a.val.drop j) (b.val.drop j)) ∨
      (r.val ≠ 0 ∧ r.val = cmpL a.val b.val) ⦄ := by
  unfold sort.cmp_cov_loop
  apply loop.spec_decr_nat (fun (q : I8 × Usize) => (if q.1.val = 0 then 1 else 0) + a.val.length - q.2.val)
    (fun q => q.2.val ≤ a.val.length ∧ q.2.val ≤ b.val.length ∧
      ((q.1.val = 0 ∧ cmpL a.val b.val = cmpL (a.val.drop q.2.val) (b.val.drop q.2.val)) ∨
        (q.1.val ≠ 0 ∧ q.1.val = cmpL a.val b.val)))
  · rintro ⟨o0, j⟩ ⟨hja, hjb, hinv⟩
    unfold sort.cmp_cov_loop.body
    simp only at hja hjb hinv ⊢
    split
    · rename_i h0
      have h0' : o0.val = 0 := by rw [h0]; rfl
      have hc : cmpL a.val b.val = cmpL (a.val.drop j.val) (b.val.drop j.val) := by
        rcases hinv with ⟨_, e⟩ | ⟨e, _⟩
        · exact e
        · exact absurd h0' e
      split
      · rename_i h1
        split
        · rename_i h2
          have hja' : j.val < a.val.length := by scalar_tac
          have hjb' : j.val < b.val.length := by scalar_tac
          step as ⟨s, hs⟩
          step with rank_spec s as ⟨x, hx⟩
          step as ⟨t, ht⟩
          step with rank_spec t as ⟨y, hy⟩
          have hs' : s = a.val[j.val]! := by rw [hs, getElem!_pos a.val j.val hja']
          have ht' : t = b.val[j.val]! := by rw [ht, getElem!_pos b.val j.val hjb']
          have hd := cmpL_drop_cons a.val b.val j.val hja' hjb'
          rw [← hs', ← ht'] at hd
          split
          · rename_i hxy
            have : rk s < rk t := by rw [← hx, ← hy]; scalar_tac
            step as ⟨j1, hj1⟩
            refine ⟨by scalar_tac, by scalar_tac, Or.inr ⟨by decide, ?_⟩, ?_⟩
            · rw [hc, hd, if_pos this]; rfl
            · have e : ((-1)#i8 : I8).val ≠ 0 := by decide
              simp only [e, h0', ite_false, ite_true]; omega
          · rename_i hxy
            split
            · rename_i hyx
              have h1' : ¬rk s < rk t := by rw [← hx, ← hy]; scalar_tac
              have : rk t < rk s := by rw [← hx, ← hy]; scalar_tac
              step as ⟨j1, hj1⟩
              refine ⟨by scalar_tac, by scalar_tac, Or.inr ⟨by decide, ?_⟩, ?_⟩
              · rw [hc, hd, if_neg h1', if_pos this]; rfl
              · have e : (1#i8 : I8).val ≠ 0 := by decide
                simp only [e, h0', ite_false, ite_true]; omega
            · rename_i hyx
              have h1' : ¬rk s < rk t := by rw [← hx, ← hy]; scalar_tac
              have h2' : ¬rk t < rk s := by rw [← hx, ← hy]; scalar_tac
              step as ⟨j1, hj1⟩
              refine ⟨by scalar_tac, by scalar_tac, Or.inl ⟨h0', ?_⟩, ?_⟩
              · rw [hc, hd, if_neg h1', if_neg h2', hj1]
              · simp [h0']; scalar_tac
        · rename_i h2
          simp only [WP.spec_ok]
          have : j.val = b.val.length := by scalar_tac
          exact Or.inl ⟨h0', j.val, hja, hjb, Or.inr this, hc⟩
      · rename_i h1
        simp only [WP.spec_ok]
        have : j.val = a.val.length := by scalar_tac
        exact Or.inl ⟨h0', j.val, hja, hjb, Or.inl this, hc⟩
    · rename_i h0
      simp only [WP.spec_ok]
      rcases hinv with ⟨e, _⟩ | ⟨e1, e2⟩
      · exact absurd (IScalar.eq_of_val_eq (by rw [e]; rfl)) h0
      · exact Or.inr ⟨e1, e2⟩
  · exact ⟨hi.1, hi.2, ho⟩

theorem cmpL_nil_left (l : List Sgn) : cmpL [] l = if l = [] then 0 else -1 := by
  cases l <;> simp [cmpL]

theorem cmpL_nil_right (l : List Sgn) : cmpL l [] = if l = [] then 0 else 1 := by
  cases l <;> simp [cmpL]

/-- **`cmp_cov` is `cmpL`.** -/
theorem cmp_cov_spec (a b : Slice Sgn) : sort.cmp_cov a b ⦃ r => r.val = cmpL a.val b.val ⦄ := by
  unfold sort.cmp_cov
  step with cmp_cov_loop_spec a b 0#i8 0#usize (by simp) (Or.inl ⟨rfl, by simp⟩) as ⟨o, ho⟩
  split
  · rename_i hne
    rcases ho with ⟨e, _⟩ | ⟨_, e⟩
    · exfalso; simp at hne; exact hne e
    · simp only [WP.spec_ok]; exact e
  · rename_i heq
    have ho0 : o.val = 0 := by simp at heq; exact heq
    obtain ⟨j, hja, hjb, hj, hc⟩ : ∃ j, j ≤ a.val.length ∧ j ≤ b.val.length ∧
        (j = a.val.length ∨ j = b.val.length) ∧ cmpL a.val b.val = cmpL (a.val.drop j) (b.val.drop j) := by
      rcases ho with ⟨_, h⟩ | ⟨e, _⟩
      · exact h
      · exact absurd ho0 e
    rw [hc]
    split
    · rename_i hlt
      have hlt' : a.val.length < b.val.length := by scalar_tac
      simp only [WP.spec_ok]
      have ej : j = a.val.length := by omega
      rw [ej, List.drop_of_length_le (Nat.le_refl _), cmpL_nil_left]
      have : b.val.drop a.val.length ≠ [] := by simp; omega
      simp [this]
    · rename_i hlt
      split
      · rename_i hgt
        have hgt' : b.val.length < a.val.length := by scalar_tac
        simp only [WP.spec_ok]
        have ej : j = b.val.length := by omega
        rw [ej, List.drop_of_length_le (Nat.le_refl b.val.length), cmpL_nil_right]
        have : a.val.drop b.val.length ≠ [] := by simp; omega
        simp [this]
      · rename_i hgt
        simp only [WP.spec_ok]
        have e1 : a.val.length = b.val.length := by scalar_tac
        have ea : a.val.drop j = [] := by simp; omega
        have eb : b.val.drop j = [] := by simp; omega
        rw [ea, eb]; rfl

end forge_bool_core
