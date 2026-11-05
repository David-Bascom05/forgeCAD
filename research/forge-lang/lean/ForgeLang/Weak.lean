import ForgeLang.Sim

/-!
# Weakening: more bindings never change a run that succeeded

`Sub ρ' ρ`: every name `ρ'` binds, `ρ` binds the same way. A run that succeeded in `ρ'` gives the same
result in `ρ`. This is what makes structural edits safe in a single-assignment document: deleting or
moving a statement only *removes* bindings from what later statements see, so a later statement
either fails (a typed `unbound`) or does exactly what it did before. With rebinding (`set`, or a
second `let` of a name) removing a binding can uncover an older one, `Sub` fails, and the meaning
changes silently (`Counter/Rebind.lean`).
-/

namespace ForgeLang

def Sub (ρ' ρ : Env) : Prop := ∀ x b, ρ'.lookup x = some b → ρ.lookup x = some b

theorem Sub.refl (ρ : Env) : Sub ρ ρ := fun _ _ h => h

theorem Sub.append (Δ : Env) {ρ' ρ : Env} (h : Sub ρ' ρ) : Sub (Δ ++ ρ') (Δ ++ ρ) := by
  induction Δ with
  | nil => simpa using h
  | cons a Δ ih =>
    intro x b hx
    obtain ⟨y, c⟩ := a
    simp only [List.cons_append, List.lookup] at hx ⊢
    split at hx
    · exact hx
    · exact ih x b hx

theorem Sub.cons (a : Name × Bind) {ρ' ρ : Env} (h : Sub ρ' ρ) : Sub (a :: ρ') (a :: ρ) := h.append [a]

theorem Sub.lookupNum {ρ' ρ : Env} (h : Sub ρ' ρ) {x : Name} {t : Tr} (hx : lookupNum ρ' x = .ok t) :
    lookupNum ρ x = .ok t := by
  unfold ForgeLang.lookupNum at hx ⊢
  split at hx
  · rename_i t' ht; rw [h _ _ ht]; exact hx
  · cases hx
  · cases hx

theorem Sub.lookupRef {ρ' ρ : Env} (h : Sub ρ' ρ) {x : Name} {rel : FId} {t : Tr}
    (hx : lookupRef ρ' x rel = .ok t) : lookupRef ρ x rel = .ok t := by
  unfold ForgeLang.lookupRef at hx ⊢
  split at hx
  · rename_i m hm; rw [h _ _ hm]; exact hx
  · cases hx
  · cases hx

theorem lookup_only (N : List Name) (x : Name) :
    ∀ ρ : Env, (only N ρ).lookup x = if x ∈ N then ρ.lookup x else none
  | [] => by simp [only, List.lookup]
  | (y, b) :: ρ => by
    have ih := lookup_only N x ρ
    unfold only at ih ⊢
    rw [List.filter_cons]
    by_cases hxy : x = y
    · subst hxy
      by_cases hx : x ∈ N <;> simp [hx, List.lookup] at ih ⊢
      exact ih
    · have e : (x == y) = false := by simp [hxy]
      split <;> simp only [List.lookup, e, ih]

theorem Sub.only (N : List Name) {ρ' ρ : Env} (h : Sub ρ' ρ) : Sub (only N ρ') (only N ρ) := by
  intro x b hx
  rw [lookup_only] at hx ⊢
  split at hx
  · rw [if_pos ‹_›]; exact h x b hx
  · cases hx

structure Weak (n : Nat) : Prop where
  e : ∀ {D σ ρ' ρ x r}, evalE n D σ ρ' x = .ok r → Sub ρ' ρ → evalE n D σ ρ x = .ok r
  one : ∀ {D σ ρ' ρ x r}, execOne n D σ ρ' x = .ok r → Sub ρ' ρ → execOne n D σ ρ x = .ok r
  s : ∀ {D σ ρ' ρ x r}, execS n D σ ρ' x = .ok r → Sub ρ' ρ → execS n D σ ρ x = .ok r
  ln : ∀ {D σ ρ' ρ i b ks r}, loopN n D σ ρ' i b ks = .ok r → Sub ρ' ρ → loopN n D σ ρ i b ks = .ok r
  lk : ∀ {D σ ρ' ρ i b kvs r}, loopK n D σ ρ' i b kvs = .ok r → Sub ρ' ρ → loopK n D σ ρ i b kvs = .ok r
  a : ∀ {D σ ρ' ρ as r}, evalA n D σ ρ' as = .ok r → Sub ρ' ρ → evalA n D σ ρ as = .ok r

theorem weak : ∀ n, Weak n := by
  intro n
  induction n with
  | zero => constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK, evalA]
  | succ n ih =>
    constructor
    · intro D σ ρ' ρ x r h hs
      cases x with
      | lit s => simpa [evalE] using h
      | cst c => simpa [evalE] using h
      | var x =>
        simp only [evalE, bind_ok] at h
        obtain ⟨t, ht, h⟩ := h
        cases h
        simp [evalE, hs.lookupNum ht]
      | ref x rel =>
        simp only [evalE, bind_ok] at h
        obtain ⟨t, ht, h⟩ := h
        cases h
        simp [evalE, hs.lookupRef ht]
      | add a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        simp [evalE, ih.e ha hs, ih.e hb hs]
      | mul a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        simp [evalE, ih.e ha hs, ih.e hb hs]
      | eq a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        simp [evalE, ih.e ha hs, ih.e hb hs]
      | ite c a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨t, l⟩, hb, h⟩ := h
        cases h
        simp only [evalE, ih.e hc hs, ok_bind, ih.e hb hs]
      | sel es k =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, h⟩ := h
        simp only at h
        split at h
        · rename_i e' he
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, he', h⟩ := h
          cases h
          simp [evalE, ih.e hk hs, hi, he, ih.e he' hs]
        · cases h
      | call f a =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        simp only at h
        split at h
        · rename_i x body hf
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, hb, h⟩ := h
          cases h
          simp [evalE, ih.e ha hs, hf, ih.e hb (hs.cons _)]
        · cases h
      | refIx x k rel =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, t, ht, h⟩ := h
        cases h
        simp [evalE, ih.e hk hs, hi, hs.lookupRef ht]
    · intro D σ ρ' ρ x r h hs
      cases x with
      | let_ x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, ih.e he hs]
      | set x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, hs.lookupNum hu, ih.e he hs]
      | feat x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, ih.e he hs]
      | forN x i c body =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, ih.e hc hs, hk, ih.ln hl hs]
      | forK x i kvs body =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, ih.lk hl hs]
      | ifs c a b =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l⟩, hb, h⟩ := h
        cases h
        simp only [execOne, ih.e hc hs, ok_bind, ih.s hb hs]
      | use x p a =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        simp only at h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l⟩, hb, h⟩ := h
          cases h
          simp [execOne, ih.e ha hs, hp, ih.s hb (hs.cons _)]
        · cases h
      | occ x c args =>
        simp only [execOne] at h
        split at h
        · rename_i C hC
          split at h
          · rename_i hok
            simp only [bind_ok] at h
            obtain ⟨⟨Δg, lg⟩, hg, ⟨Δd, ld⟩, hd, ⟨ρ1, mm, l⟩, hb, h⟩ := h
            cases h
            simp only [List.append_assoc] at hb
            have so := hs.only C.vis
            have hb' := ih.s hb ((so.append Δd).append Δg)
            simp [execOne, hC, hok, ih.a hg hs, ih.a hd so, hb']
          · cases h
        · cases h
    · intro D σ ρ' ρ x r h hs
      cases x with
      | nil => simpa [execS] using h
      | cons s ss =>
        simp only [execS, bind_ok] at h
        obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
        cases h
        simp [execS, ih.one h1 hs, ih.s h2 (hs.append Δ1)]
    · intro D σ ρ' ρ i b ks r h hs
      cases ks with
      | nil => simpa [loopN] using h
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopN, ih.s h1 (hs.cons _), ih.ln h2 hs]
    · intro D σ ρ' ρ i b kvs r h hs
      cases kvs with
      | nil => simpa [loopK] using h
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp [loopK, ih.e h0 hs, ih.s h1 (hs.cons _), ih.lk h2 hs]
    · intro D σ ρ' ρ as r h hs
      cases as with
      | nil => simpa [evalA] using h
      | cons a as =>
        obtain ⟨y, e⟩ := a
        simp only [evalA, bind_ok] at h
        obtain ⟨⟨t, l⟩, h0, ⟨Δ, l'⟩, h1, h⟩ := h
        cases h
        simp [evalA, ih.e h0 hs, ih.a h1 hs]

end ForgeLang
