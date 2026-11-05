import ForgeLang.Core

/-!
# The simulation theorem

One induction carries everything after it: a run that succeeded is repeated *exactly* (same traces,
same ids, same log) by a run with more fuel and with any literal table that agrees on the sites the
first run observed. Two corollaries:

* `σ' = σ`: more fuel never changes a result (P2: the budget is the only budget-dependent outcome).
* fuel unchanged: editing a literal that no branch, count or index looked at changes no id and no
  trace, so the new numbers are the old traces re-read (P3, P4 SetParam, P5).

Hypothesis `D.track`: the run records what it observes. Without it the theorem is false
(`Counter/ControlUntracked.lean`).
-/
namespace ForgeLang

def Agree (σ σ' : Site → Int) (L : Log) : Prop := ∀ s ∈ L, σ s = σ' s

theorem agree_append {σ σ' : Site → Int} {a b : Log} : Agree σ σ' (a ++ b) ↔ Agree σ σ' a ∧ Agree σ σ' b := by
  simp only [Agree, List.mem_append]
  exact ⟨fun h => ⟨fun s hs => h s (.inl hs), fun s hs => h s (.inr hs)⟩,
    fun h s hs => hs.elim (h.1 s) (h.2 s)⟩

theorem Tr.den_congr {σ σ' : Site → Int} : ∀ t : Tr, Agree σ σ' t.sites → t.den σ = t.den σ'
  | .site s, h => h s (by simp [Tr.sites])
  | .const _, _ => rfl
  | .add a b, h => by
    simp only [Tr.sites, agree_append] at h
    simp only [Tr.den, Tr.den_congr a h.1, Tr.den_congr b h.2]
  | .mul a b, h => by
    simp only [Tr.sites, agree_append] at h
    simp only [Tr.den, Tr.den_congr a h.1, Tr.den_congr b h.2]

/-- What the hypothesis of `Sim` buys at an observation point: the observed number is the same. -/
theorem den_obs {D : Defs} {σ σ' : Site → Int} {t : Tr} (h : D.track = true ∨ ∀ s, σ s = σ' s)
    (ag : Agree σ σ' (obs D t)) : t.den σ = t.den σ' := by
  rcases h with h | h
  · simp only [obs, h, ite_true] at ag
    exact Tr.den_congr t ag
  · exact Tr.den_congr t fun s _ => h s

theorem bind_ok {α β : Type} {x : R α} {f : α → R β} {b : β} :
    x >>= f = .ok b ↔ ∃ a, x = .ok a ∧ f a = .ok b := by
  cases x <;> simp [bind, Except.bind]

@[simp] theorem ok_bind {α β : Type} {a : α} {f : α → R β} : (Except.ok a : R α) >>= f = f a := rfl

structure Sim (n : Nat) : Prop where
  e : ∀ {D σ σ' ρ x r m}, (D.track = true ∨ ∀ s, σ s = σ' s) → evalE n D σ ρ x = .ok r → Agree σ σ' r.2 → n ≤ m →
    evalE m D σ' ρ x = .ok r
  one : ∀ {D σ σ' ρ x r m}, (D.track = true ∨ ∀ s, σ s = σ' s) → execOne n D σ ρ x = .ok r → Agree σ σ' r.2.2 → n ≤ m →
    execOne m D σ' ρ x = .ok r
  s : ∀ {D σ σ' ρ x r m}, (D.track = true ∨ ∀ s, σ s = σ' s) → execS n D σ ρ x = .ok r → Agree σ σ' r.2.2 → n ≤ m →
    execS m D σ' ρ x = .ok r
  ln : ∀ {D σ σ' ρ i b ks r m}, (D.track = true ∨ ∀ s, σ s = σ' s) → loopN n D σ ρ i b ks = .ok r → Agree σ σ' r.2 → n ≤ m →
    loopN m D σ' ρ i b ks = .ok r
  lk : ∀ {D σ σ' ρ i b kvs r m}, (D.track = true ∨ ∀ s, σ s = σ' s) → loopK n D σ ρ i b kvs = .ok r → Agree σ σ' r.2 →
    n ≤ m → loopK m D σ' ρ i b kvs = .ok r
  a : ∀ {D σ σ' ρ as r m}, (D.track = true ∨ ∀ s, σ s = σ' s) → evalA n D σ ρ as = .ok r → Agree σ σ' r.2 →
    n ≤ m → evalA m D σ' ρ as = .ok r

theorem sim : ∀ n, Sim n := by
  intro n
  induction n with
  | zero =>
    constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK, evalA]
  | succ n ih =>
    constructor
    · intro D σ σ' ρ x r m htr h ag hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases x with
      | lit s => simpa [evalE] using h
      | cst c => simpa [evalE] using h
      | var x => simpa [evalE] using h
      | ref x rel => simpa [evalE] using h
      | add a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        simp only [agree_append] at ag
        simp [evalE, ih.e htr ha ag.1 hm, ih.e htr hb ag.2 hm]
      | mul a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        simp only [agree_append] at ag
        simp [evalE, ih.e htr ha ag.1 hm, ih.e htr hb ag.2 hm]
      | eq a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, ⟨tb, lb⟩, hb, h⟩ := h
        cases h
        simp only [agree_append] at ag
        simp [evalE, ih.e htr ha ag.1.1.1 hm, ih.e htr hb ag.1.1.2 hm, den_obs htr ag.1.2,
          den_obs (t := tb) htr ag.2]
      | ite c a b =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨t, l⟩, hb, h⟩ := h
        cases h
        simp only [agree_append] at ag
        have hd := den_obs htr ag.1.2
        rw [hd] at hb
        simp only [evalE, ih.e htr hc ag.1.1 hm, ok_bind, ih.e htr hb ag.2 hm]
      | sel es k =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, h⟩ := h
        simp only at h
        split at h
        · rename_i e' he
          rw [bind_ok] at h
          obtain ⟨⟨t, l⟩, he', h⟩ := h
          cases h
          simp only [agree_append] at ag
          have hd := den_obs htr ag.1.2
          rw [hd] at hi
          simp [evalE, ih.e htr hk ag.1.1 hm, hi, he, ih.e htr he' ag.2 hm]
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
          simp only [agree_append] at ag
          simp [evalE, ih.e htr ha ag.1 hm, hf, ih.e htr hb ag.2 hm]
        · cases h
      | refIx x k rel =>
        simp only [evalE, bind_ok] at h
        obtain ⟨⟨tk, lk⟩, hk, i, hi, t, ht, h⟩ := h
        cases h
        simp only [agree_append] at ag
        have hd := den_obs htr ag.2
        rw [hd] at hi
        simp [evalE, ih.e htr hk ag.1 hm, hi, ht]
    · intro D σ σ' ρ x r m htr h ag hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases x with
      | let_ x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, ih.e htr he ag hm]
      | set x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨u, hu, ⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, hu, ih.e htr he ag hm]
      | feat x e =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨t, l⟩, he, h⟩ := h
        cases h
        simp [execOne, ih.e htr he ag hm]
      | forN x i c body =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, k, hk, ⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp only [agree_append] at ag
        rw [den_obs htr ag.1.2] at hk
        simp [execOne, ih.e htr hc ag.1.1 hm, hk, ih.ln htr hl ag.2 hm]
      | forK x i kvs body =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨mm, l⟩, hl, h⟩ := h
        cases h
        simp [execOne, ih.lk htr hl ag hm]
      | ifs c a b =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨tc, lc⟩, hc, ⟨ρ1, mm, l⟩, hb, h⟩ := h
        cases h
        simp only [agree_append] at ag
        rw [den_obs htr ag.1.2] at hb
        simp only [execOne, ih.e htr hc ag.1.1 hm, ok_bind, ih.s htr hb ag.2 hm]
      | use x p a =>
        simp only [execOne, bind_ok] at h
        obtain ⟨⟨ta, la⟩, ha, h⟩ := h
        simp only at h
        split at h
        · rename_i y body hp
          rw [bind_ok] at h
          obtain ⟨⟨ρ1, mm, l⟩, hb, h⟩ := h
          cases h
          simp only [agree_append] at ag
          simp [execOne, ih.e htr ha ag.1 hm, hp, ih.s htr hb ag.2 hm]
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
            simp only [agree_append] at ag
            simp [execOne, hC, hok, ih.a htr hg ag.1.1 hm, ih.a htr hd ag.1.2 hm, ih.s htr hb ag.2 hm]
          · cases h
        · cases h
    · intro D σ σ' ρ x r m htr h ag hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases x with
      | nil => simpa [execS] using h
      | cons s ss =>
        simp only [execS, bind_ok] at h
        obtain ⟨⟨ρ1, m1, l1⟩, h1, ⟨ρ2, m2, l2⟩, h2, h⟩ := h
        cases h
        simp only [agree_append] at ag
        simp [execS, ih.one htr h1 ag.1 hm, ih.s htr h2 ag.2 hm]
    · intro D σ σ' ρ i b ks r m htr h ag hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases ks with
      | nil => simpa [loopN] using h
      | cons k ks =>
        simp only [loopN, bind_ok] at h
        obtain ⟨⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp only [agree_append] at ag
        simp [loopN, ih.s htr h1 ag.1 hm, ih.ln htr h2 ag.2 hm]
    · intro D σ σ' ρ i b kvs r m htr h ag hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases kvs with
      | nil => simpa [loopK] using h
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [loopK, bind_ok] at h
        obtain ⟨⟨t, l0⟩, h0, ⟨ρ1, m1, l1⟩, h1, ⟨m2, l2⟩, h2, h⟩ := h
        cases h
        simp only [agree_append] at ag
        simp [loopK, ih.e htr h0 ag.1.1 hm, ih.s htr h1 ag.1.2 hm, ih.lk htr h2 ag.2 hm]
    · intro D σ σ' ρ as r m htr h ag hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases as with
      | nil => simpa [evalA] using h
      | cons a as =>
        obtain ⟨y, e⟩ := a
        simp only [evalA, bind_ok] at h
        obtain ⟨⟨t, l⟩, h0, ⟨Δ, l'⟩, h1, h⟩ := h
        cases h
        simp only [agree_append] at ag
        simp [evalA, ih.e htr h0 ag.1 hm, ih.a htr h1 ag.2 hm]

end ForgeLang
