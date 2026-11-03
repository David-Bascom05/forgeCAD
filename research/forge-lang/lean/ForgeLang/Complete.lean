import ForgeLang.Fv
import ForgeLang.Struct

/-!
# The complete half of Add, Delete and Move

`Struct.lean` proved soundness: *if* the edited block evaluates, it evaluates to the expected model.
Here: it *does* evaluate, given that nothing reads the name the edit takes away. "Reads" is
syntactic (`fvB`, `fvS`, plus `Defs.reads` for function and procedure bodies), so the check an
editor runs before the op is a name lookup in the source. With `Struct.lean`, each op is: refused
with a typed reason, or applied with exactly the expected model.
-/

namespace ForgeLang

/-- `s` binds no name in `N`. -/
def NotRead (s : Stmt) (N : List Name) : Prop := ∀ x, s.binder = some x → x ∉ N

theorem AgreeOn.symm {N : List Name} {ρ ρ' : Env} (h : AgreeOn N ρ ρ') : AgreeOn N ρ' ρ :=
  fun y hy => (h y hy).symm

/-- Bindings for names nobody reads are invisible. -/
theorem agree_drop {k D σ ρ0 s Δs ms ls} {N : List Name} (hs : execOne k D σ ρ0 s = .ok (Δs, ms, ls))
    (h : NotRead s N) (ρ : Env) : AgreeOn N (Δs ++ ρ) ρ := by
  intro y hy
  have : y ∉ keys Δs := fun hk => h y (execOne_keys hs y hk) hy
  rw [lookup_append, lookup_none this]; rfl

theorem execS_compose : ∀ (xs ys : List Stmt) {n D σ ρ Δ1 m1 l1 Δ2 m2 l2},
    execS n D σ ρ xs = .ok (Δ1, m1, l1) → execS n D σ (Δ1 ++ ρ) ys = .ok (Δ2, m2, l2) →
    execS (n + xs.length) D σ ρ (xs ++ ys) = .ok (Δ2 ++ Δ1, m1 ++ m2, l1 ++ l2)
  | [], ys, n, D, σ, ρ, Δ1, m1, l1, Δ2, m2, l2, h1, h2 => by
    cases n with
    | zero => simp [execS] at h1
    | succ n =>
      simp only [execS, Except.ok.injEq, Prod.mk.injEq] at h1
      obtain ⟨rfl, rfl, rfl⟩ := h1
      simpa using h2
  | x :: xs, ys, n, D, σ, ρ, Δ1, m1, l1, Δ2, m2, l2, h1, h2 => by
    cases n with
    | zero => simp [execS] at h1
    | succ k =>
      simp only [execS, bind_ok] at h1
      obtain ⟨⟨Δx, mx, lx⟩, hx, ⟨Δ', m', l'⟩, h', h1⟩ := h1
      cases h1
      have ih := execS_compose xs ys (execS_mono h' (Nat.le_succ k)) (by simpa using h2)
      have e : k + 1 + (x :: xs).length = (k + 1 + xs.length) + 1 := by simp; omega
      rw [e]
      simp [execS, execOne_mono hx (by omega : k ≤ k + 1 + xs.length), ih]

theorem execS_compose' {xs ys : List Stmt} {n n' D σ ρ Δ1 m1 l1 Δ2 m2 l2}
    (h1 : execS n D σ ρ xs = .ok (Δ1, m1, l1)) (h2 : execS n' D σ (Δ1 ++ ρ) ys = .ok (Δ2, m2, l2)) :
    ∃ N, execS N D σ ρ (xs ++ ys) = .ok (Δ2 ++ Δ1, m1 ++ m2, l1 ++ l2) :=
  ⟨_, execS_compose xs ys (execS_mono h1 (Nat.le_max_left n n')) (execS_mono h2 (Nat.le_max_right n n'))⟩

theorem execS_cons_compose {s : Stmt} {ys : List Stmt} {k k' D σ ρ Δs ms ls Δ2 m2 l2}
    (hs : execOne k D σ ρ s = .ok (Δs, ms, ls)) (h2 : execS k' D σ (Δs ++ ρ) ys = .ok (Δ2, m2, l2)) :
    ∃ N, execS N D σ ρ (s :: ys) = .ok (Δ2 ++ Δs, ms ++ m2, ls ++ l2) :=
  ⟨max k k' + 1, by
    simp [execS, execOne_mono hs (Nat.le_max_left k k'), execS_mono h2 (Nat.le_max_right k k')]⟩

/-- DeleteFeature, complete: if no later statement and no function body mentions what `s` binds,
the block without `s` evaluates, to the old model without `s`'s fragment. -/
theorem delete_complete {n D σ xs s ys Δ m l} (hr : NotRead s (fvB ys ++ D.reads))
    (h : execS n D σ [] (xs ++ s :: ys) = .ok (Δ, m, l)) :
    ∃ m1 ms m3 l1 ls l3, m = m1 ++ ms ++ m3 ∧ l = l1 ++ ls ++ l3 ∧
      ∃ N Δ', execS N D σ [] (xs ++ ys) = .ok (Δ', m1 ++ m3, l1 ++ l3) := by
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (s :: ys) h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hs, h3, e2⟩ := execS_cons h2
  have h3' := (indep n).s h3 (agree_drop hs hr _)
  obtain ⟨N, hN⟩ := execS_compose' h1 h3'
  simp only [Prod.mk.injEq] at e e2
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  exact ⟨m1, ms, m3, l1, ls, l3, by simp, by simp, N, _, hN⟩

/-- AddFeature, complete: if `s` evaluates where it is inserted and nothing after it (nor any
function body) mentions what it binds, the longer block evaluates, to the old model with `s`'s
fragment in place. -/
theorem add_complete {n D σ xs s ys Δ m l} (hr : NotRead s (fvB ys ++ D.reads))
    (h : execS n D σ [] (xs ++ ys) = .ok (Δ, m, l))
    (hs : ∀ Δ1 m1 l1, execS n D σ [] xs = .ok (Δ1, m1, l1) → ∃ k r, execOne k D σ (Δ1 ++ []) s = .ok r) :
    ∃ m1 ms m3 l1 ls l3, m = m1 ++ m3 ∧ l = l1 ++ l3 ∧
      ∃ N Δ', execS N D σ [] (xs ++ s :: ys) = .ok (Δ', m1 ++ ms ++ m3, l1 ++ ls ++ l3) := by
  obtain ⟨Δ1, m1, l1, Δ3, m3, l3, h1, h3, e⟩ := execS_append xs ys h
  obtain ⟨k, ⟨Δs, ms, ls⟩, hS⟩ := hs Δ1 m1 l1 h1
  have h3' := (indep n).s h3 (agree_drop hS hr _).symm
  obtain ⟨N1, hN1⟩ := execS_cons_compose hS h3'
  obtain ⟨N, hN⟩ := execS_compose' h1 hN1
  simp only [Prod.mk.injEq] at e
  obtain ⟨-, rfl, rfl⟩ := e
  exact ⟨m1, ms, m3, l1, ls, l3, rfl, rfl, N, _, by simpa [List.append_assoc] using hN⟩

/-- MoveFeature, complete: if neither of two neighbours (nor any function body) mentions what the
other binds, and they bind different names, the swapped block evaluates, to the old model with
the two fragments swapped. -/
theorem move_complete {n D σ xs s t ys Δ m l}
    (hst : ∀ x, s.binder = some x → t.binder ≠ some x)
    (hs : NotRead s (fvS t ++ D.reads)) (ht : NotRead t (fvS s ++ D.reads))
    (h : execS n D σ [] (xs ++ s :: t :: ys) = .ok (Δ, m, l)) :
    ∃ m1 ms mt m3 l1 ls lt l3, m = m1 ++ ms ++ mt ++ m3 ∧ l = l1 ++ ls ++ lt ++ l3 ∧
      ∃ N Δ', execS N D σ [] (xs ++ t :: s :: ys) = .ok (Δ', m1 ++ mt ++ ms ++ m3, l1 ++ lt ++ ls ++ l3) := by
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (s :: t :: ys) h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hS, h3, e2⟩ := execS_cons h2
  obtain ⟨Δt, mt, lt, Δ4, m4, l4, hT, h4, e3⟩ := execS_cons h3
  have hT' := (indep n).one hT (agree_drop hS hs _)
  have hS' := (indep n).one hS (agree_drop hT' ht _).symm
  have dis : ∀ x ∈ keys Δt, x ∉ keys Δs := by
    intro x hx hx'
    exact hst x (execOne_keys hS x hx') (execOne_keys hT x hx)
  have sw := sub_swap (ρ := Δ1 ++ []) dis
  rw [List.append_assoc, List.append_assoc] at sw
  have h4' := (weak n).s h4 sw
  obtain ⟨Na, ha⟩ := execS_cons_compose hS' h4'
  obtain ⟨Nb, hb⟩ := execS_cons_compose hT' ha
  obtain ⟨N, hN⟩ := execS_compose' h1 hb
  simp only [Prod.mk.injEq] at e e2 e3
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  obtain ⟨-, rfl, rfl⟩ := e3
  exact ⟨m1, ms, mt, m4, l1, ls, lt, l4, by simp, by simp, N, _, by simpa [List.append_assoc] using hN⟩

end ForgeLang
