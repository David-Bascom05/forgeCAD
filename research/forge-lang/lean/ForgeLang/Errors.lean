import ForgeLang.Edits

/-!
# P2, the other half: a typed error is not an artefact of the budget

A run that stopped with any error other than `fuel` stops with the same error under every larger
budget. With `run_mono`: the budget decides only "finished or `fuel`"; what a document means
(a model, or a typed error) does not depend on it.
-/

namespace ForgeLang

theorem bind_err {α β : Type} {x : R α} {f : α → R β} {e : Err} :
    x >>= f = .error e ↔ x = .error e ∨ ∃ a, x = .ok a ∧ f a = .error e := by
  cases x <;> simp [bind, Except.bind]

@[simp] theorem err_bind {α β : Type} {e : Err} {f : α → R β} : (Except.error e : R α) >>= f = .error e := rfl

theorem evalE_mono {n m D σ ρ x r} (h : evalE n D σ ρ x = .ok r) (hm : n ≤ m) :
    evalE m D σ ρ x = .ok r := (sim n).e (.inr fun _ => rfl) h (fun _ _ => rfl) hm

theorem evalA_mono {n m D σ ρ x r} (h : evalA n D σ ρ x = .ok r) (hm : n ≤ m) :
    evalA m D σ ρ x = .ok r := (sim n).a (.inr fun _ => rfl) h (fun _ _ => rfl) hm

structure ErrStable (n : Nat) : Prop where
  e : ∀ {D σ ρ x er m}, evalE n D σ ρ x = .error er → er ≠ .fuel → n ≤ m → evalE m D σ ρ x = .error er
  one : ∀ {D σ ρ x er m}, execOne n D σ ρ x = .error er → er ≠ .fuel → n ≤ m →
    execOne m D σ ρ x = .error er
  s : ∀ {D σ ρ x er m}, execS n D σ ρ x = .error er → er ≠ .fuel → n ≤ m → execS m D σ ρ x = .error er
  ln : ∀ {D σ ρ i b ks er m}, loopN n D σ ρ i b ks = .error er → er ≠ .fuel → n ≤ m →
    loopN m D σ ρ i b ks = .error er
  lk : ∀ {D σ ρ i b kvs er m}, loopK n D σ ρ i b kvs = .error er → er ≠ .fuel → n ≤ m →
    loopK m D σ ρ i b kvs = .error er
  a : ∀ {D σ ρ as er m}, evalA n D σ ρ as = .error er → er ≠ .fuel → n ≤ m → evalA m D σ ρ as = .error er

theorem errStable : ∀ n, ErrStable n := by
  intro n
  induction n with
  | zero =>
    constructor <;> intros <;> simp_all [evalE, execOne, execS, loopN, loopK, evalA]
  | succ n ih =>
    constructor
    · intro D σ ρ x er m h hf hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases x with
      | lit s => simp [evalE] at h
      | cst c => simp [evalE] at h
      | var x => simpa [evalE] using h
      | ref x rel => simpa [evalE] using h
      | add a b | mul a b =>
        simp only [evalE, bind_err] at h
        rcases h with h | ⟨_, ha, h | ⟨_, hb, h⟩⟩
        · simp [evalE, ih.e h hf hm]
        · simp [evalE, evalE_mono ha hm, ih.e h hf hm]
        · cases h
      | eq a b =>
        simp only [evalE, bind_err] at h
        rcases h with h | ⟨_, ha, h | ⟨_, hb, h⟩⟩
        · simp [evalE, ih.e h hf hm]
        · simp [evalE, evalE_mono ha hm, ih.e h hf hm]
        · cases h
      | ite c a b =>
        simp only [evalE, bind_err] at h
        rcases h with h | ⟨_, hc, h | ⟨_, hb, h⟩⟩
        · simp [evalE, ih.e h hf hm]
        · simp only [evalE, evalE_mono hc hm, ok_bind, ih.e h hf hm]; rfl
        · cases h
      | sel es k =>
        simp only [evalE, bind_err] at h
        rcases h with h | ⟨_, hk, h | ⟨i, hi, h⟩⟩
        · simp [evalE, ih.e h hf hm]
        · simp [evalE, evalE_mono hk hm, h]
        · split at h
          · rename_i e' he
            rw [bind_err] at h
            rcases h with h | ⟨_, _, h⟩
            · simp [evalE, evalE_mono hk hm, hi, he, ih.e h hf hm]
            · cases h
          · cases h
            simp [evalE, evalE_mono hk hm, *]
      | call f a =>
        simp only [evalE, bind_err] at h
        rcases h with h | ⟨_, ha, h⟩
        · simp [evalE, ih.e h hf hm]
        · split at h
          · rename_i x body hfn
            rw [bind_err] at h
            rcases h with h | ⟨_, _, h⟩
            · simp [evalE, evalE_mono ha hm, hfn, ih.e h hf hm]
            · cases h
          · cases h
            simp [evalE, evalE_mono ha hm, *]
      | refIx x k rel =>
        simp only [evalE, bind_err] at h
        rcases h with h | ⟨_, hk, h | ⟨i, hi, h | ⟨_, ht, h⟩⟩⟩
        · simp [evalE, ih.e h hf hm]
        · simp [evalE, evalE_mono hk hm, h]
        · simp [evalE, evalE_mono hk hm, hi, h]
        · cases h
    · intro D σ ρ x er m h hf hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases x with
      | let_ x e | feat x e =>
        simp only [execOne, bind_err] at h
        rcases h with h | ⟨_, _, h⟩
        · simp [execOne, ih.e h hf hm]
        · cases h
      | set x e =>
        simp only [execOne, bind_err] at h
        rcases h with h | ⟨_, hu, h | ⟨_, _, h⟩⟩
        · simp [execOne, h]
        · simp [execOne, hu, ih.e h hf hm]
        · cases h
      | forN x i c body =>
        simp only [execOne, bind_err] at h
        rcases h with h | ⟨_, hc, h | ⟨k, hk, h | ⟨_, _, h⟩⟩⟩
        · simp [execOne, ih.e h hf hm]
        · simp [execOne, evalE_mono hc hm, h]
        · simp [execOne, evalE_mono hc hm, hk, ih.ln h hf hm]
        · cases h
      | forK x i kvs body =>
        simp only [execOne, bind_err] at h
        rcases h with h | ⟨_, _, h⟩
        · simp [execOne, ih.lk h hf hm]
        · cases h
      | ifs c a b =>
        simp only [execOne, bind_err] at h
        rcases h with h | ⟨_, hc, h | ⟨_, _, h⟩⟩
        · simp [execOne, ih.e h hf hm]
        · simp only [execOne, evalE_mono hc hm, ok_bind, ih.s h hf hm]; rfl
        · cases h
      | use x p a =>
        simp only [execOne, bind_err] at h
        rcases h with h | ⟨_, ha, h⟩
        · simp [execOne, ih.e h hf hm]
        · split at h
          · rename_i y body hp
            rw [bind_err] at h
            rcases h with h | ⟨_, _, h⟩
            · simp [execOne, evalE_mono ha hm, hp, ih.s h hf hm]
            · cases h
          · cases h
            simp [execOne, evalE_mono ha hm, *]
      | occ x c args =>
        simp only [execOne] at h
        split at h
        · rename_i C hC
          split at h
          · rename_i hok
            simp only [bind_err] at h
            rcases h with h | ⟨⟨Δg, lg⟩, hg, h | ⟨⟨Δd, ld⟩, hd, h | ⟨_, _, h⟩⟩⟩
            · simp [execOne, hC, hok, ih.a h hf hm]
            · simp [execOne, hC, hok, evalA_mono hg hm, ih.a h hf hm]
            · simp only [List.append_assoc] at h
              simp [execOne, hC, hok, evalA_mono hg hm, evalA_mono hd hm, ih.s h hf hm]
            · cases h
          · cases h; simp [execOne, hC, *]
        · cases h; simp [execOne, *]
    · intro D σ ρ x er m h hf hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases x with
      | nil => simp [execS] at h
      | cons s ss =>
        simp only [execS, bind_err] at h
        rcases h with h | ⟨_, h1, h | ⟨_, _, h⟩⟩
        · simp [execS, ih.one h hf hm]
        · simp [execS, execOne_mono h1 hm, ih.s h hf hm]
        · cases h
    · intro D σ ρ i b ks er m h hf hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases ks with
      | nil => simp [loopN] at h
      | cons k ks =>
        simp only [loopN, bind_err] at h
        rcases h with h | ⟨_, h1, h | ⟨_, _, h⟩⟩
        · simp [loopN, ih.s h hf hm]
        · simp [loopN, execS_mono h1 hm, ih.ln h hf hm]
        · cases h
    · intro D σ ρ i b kvs er m h hf hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases kvs with
      | nil => simp [loopK] at h
      | cons kv kvs =>
        obtain ⟨k, e⟩ := kv
        simp only [loopK, bind_err] at h
        rcases h with h | ⟨_, h0, h | ⟨_, h1, h | ⟨_, _, h⟩⟩⟩
        · simp [loopK, ih.e h hf hm]
        · simp [loopK, evalE_mono h0 hm, ih.s h hf hm]
        · simp [loopK, evalE_mono h0 hm, execS_mono h1 hm, ih.lk h hf hm]
        · cases h
    · intro D σ ρ as er m h hf hm
      obtain ⟨m, rfl⟩ : ∃ k, m = k + 1 := ⟨m - 1, by omega⟩
      have hm : n ≤ m := by omega
      cases as with
      | nil => simp [evalA] at h
      | cons a as =>
        obtain ⟨y, e⟩ := a
        simp only [evalA, bind_err] at h
        rcases h with h | ⟨_, h0, h | ⟨_, _, h⟩⟩
        · simp [evalA, ih.e h hf hm]
        · simp [evalA, evalE_mono h0 hm, ih.a h hf hm]
        · cases h

/-- P2: the meaning of a document is the same under every budget large enough to finish it, and
a typed error found under one budget is found under every larger one. -/
theorem run_error_stable {n m : Nat} {d : Doc} {er : Err} (h : run n d = .error er) (hf : er ≠ .fuel)
    (hm : n ≤ m) : run m d = .error er := by
  unfold run at h ⊢
  rw [bind_err] at h
  rcases h with h | ⟨⟨Δ, mm, l⟩, he, h⟩
  · rw [(errStable n).s h hf hm]; rfl
  · rw [execS_mono he hm]; exact h

end ForgeLang
