import ForgeLang.Edits

/-!
# P4 and P5 for AddFeature, DeleteFeature, MoveFeature

Stated for a block of statements (the document's body is one). Each theorem is the *sound* half:
if the edited document evaluates, its model is the old model with the one fragment added, removed
or swapped, and every other feature keeps its id and its trace. If it does not evaluate, the edit
is refused with the typed error the run produced (a deleted feature still referenced is `unbound`).

The only assumption is single assignment for the names involved: the moved, added or deleted
statement does not bind a name an earlier statement of the block binds. No dependency analysis is
needed for soundness. `Counter/Rebind.lean` shows each theorem false without it.

Open: the complete half (the edit *does* evaluate when no later statement reads the binding) needs
a free-name lemma over the syntax; not mechanised.
-/

namespace ForgeLang

def Stmt.binder : Stmt → Option Name
  | .let_ x _ | .set x _ | .feat x _ | .forN x _ _ _ | .forK x _ _ _ | .use x _ _ => some x
  | .ifs _ _ _ => none

def binders (ss : List Stmt) : List Name := ss.filterMap Stmt.binder

def keys (ρ : Env) : List Name := ρ.map Prod.fst

theorem lookup_append (A ρ : Env) (x : Name) : (A ++ ρ).lookup x = (A.lookup x).or (ρ.lookup x) := by
  induction A with
  | nil => rfl
  | cons a A ih =>
    obtain ⟨y, b⟩ := a
    simp only [List.cons_append, List.lookup]
    split <;> simp_all

theorem lookup_none {A : Env} {x : Name} (h : x ∉ keys A) : A.lookup x = none := by
  induction A with
  | nil => rfl
  | cons a A ih =>
    obtain ⟨y, b⟩ := a
    simp only [keys, List.map_cons, List.mem_cons, not_or] at h
    simp only [List.lookup]
    have : (x == y) = false := by simp [h.1]
    rw [this]; exact ih h.2

theorem lookup_some {A : Env} {x : Name} {b : Bind} (h : A.lookup x = some b) : x ∈ keys A := by
  by_cases hx : x ∈ keys A
  · exact hx
  · rw [lookup_none hx] at h; cases h

/-- Adding bindings for names not yet bound is a weakening. -/
theorem sub_fresh {Δ ρ : Env} (h : ∀ x ∈ keys Δ, x ∉ keys ρ) : Sub ρ (Δ ++ ρ) := by
  intro x b hx
  rw [lookup_append, lookup_none (fun hd => h x hd (lookup_some hx)), hx]; rfl

/-- Two blocks of bindings with no name in common commute. -/
theorem sub_swap {A B ρ : Env} (h : ∀ x ∈ keys A, x ∉ keys B) : Sub (A ++ B ++ ρ) (B ++ A ++ ρ) := by
  intro x b hx
  rw [List.append_assoc, lookup_append, lookup_append] at hx
  rw [List.append_assoc, lookup_append, lookup_append]
  cases hA : A.lookup x with
  | none => cases hB : B.lookup x <;> simp_all [Option.or]
  | some a =>
    have hB : B.lookup x = none := lookup_none (h x (lookup_some hA))
    rw [hA, hB] at hx; rw [hB]; exact hx

theorem execOne_keys {n D σ ρ s Δ m l} (h : execOne n D σ ρ s = .ok (Δ, m, l)) :
    ∀ x ∈ keys Δ, s.binder = some x := by
  cases n with
  | zero => simp [execOne] at h
  | succ n =>
    cases s with
    | let_ _ _ | feat _ _ =>
      simp only [execOne, bind_ok] at h
      obtain ⟨_, _, h⟩ := h; cases h; simp [keys, Stmt.binder]
    | forK _ _ _ _ =>
      simp only [execOne, bind_ok] at h
      obtain ⟨_, _, h⟩ := h; cases h; simp [keys, Stmt.binder]
    | set _ _ =>
      simp only [execOne, bind_ok] at h
      obtain ⟨_, _, _, _, h⟩ := h; cases h; simp [keys, Stmt.binder]
    | ifs _ _ _ =>
      simp only [execOne, bind_ok] at h
      obtain ⟨_, _, _, _, h⟩ := h; cases h; simp [keys]
    | forN _ _ _ _ =>
      simp only [execOne, bind_ok] at h
      obtain ⟨_, _, _, _, _, _, h⟩ := h; cases h; simp [keys, Stmt.binder]
    | use _ _ _ =>
      simp only [execOne, bind_ok] at h
      obtain ⟨_, _, h⟩ := h
      split at h
      · rw [bind_ok] at h; obtain ⟨_, _, h⟩ := h; cases h; simp [keys, Stmt.binder]
      · cases h

theorem execS_keys : ∀ (ss : List Stmt) {n D σ ρ Δ m l}, execS n D σ ρ ss = .ok (Δ, m, l) →
    ∀ x ∈ keys Δ, x ∈ binders ss
  | [], n, _, _, _, _, _, _, h => by
    cases n <;> simp [execS] at h
    obtain ⟨rfl, -, -⟩ := h; simp [keys]
  | s :: ss, n, _, _, _, _, _, _, h => by
    cases n with
    | zero => simp [execS] at h
    | succ n =>
      simp only [execS, bind_ok] at h
      obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
      cases h
      intro x hx
      simp only [keys, List.map_append, List.mem_append] at hx
      rcases hx with hx | hx
      · have := execS_keys ss h2 x hx
        simp only [binders, List.filterMap_cons] at this ⊢
        split <;> simp_all
      · have := execOne_keys h1 x hx
        simp [binders, this]

/-- A block splits: what runs first, then the rest in its bindings. -/
theorem execS_append : ∀ (xs ys : List Stmt) {n D σ ρ r}, execS n D σ ρ (xs ++ ys) = .ok r →
    ∃ Δ1 m1 l1 Δ2 m2 l2, execS n D σ ρ xs = .ok (Δ1, m1, l1) ∧
      execS n D σ (Δ1 ++ ρ) ys = .ok (Δ2, m2, l2) ∧ r = (Δ2 ++ Δ1, m1 ++ m2, l1 ++ l2)
  | [], ys, n, D, σ, ρ, r, h => by
    cases n with
    | zero => simp [execS] at h
    | succ n =>
      obtain ⟨Δ2, m2, l2⟩ := r
      exact ⟨[], [], [], Δ2, m2, l2, by simp [execS], by simpa using h, by simp⟩
  | x :: xs, ys, n, D, σ, ρ, r, h => by
    cases n with
    | zero => simp [execS] at h
    | succ n =>
      simp only [List.cons_append, execS, bind_ok] at h
      obtain ⟨⟨Δx, mx, lx⟩, hx, ⟨Δ', m', l'⟩, h', h⟩ := h
      cases h
      obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs ys h'
      cases e
      refine ⟨Δ1 ++ Δx, mx ++ m1, lx ++ l1, Δ2, m2, l2, ?_, ?_, ?_⟩
      · simp [execS, hx, h1]
      · have := execS_mono h2 (Nat.le_succ n)
        simpa using this
      · simp

theorem execS_cons {s : Stmt} {ys : List Stmt} {n D σ ρ r} (h : execS n D σ ρ (s :: ys) = .ok r) :
    ∃ Δ1 m1 l1 Δ2 m2 l2, execOne n D σ ρ s = .ok (Δ1, m1, l1) ∧
      execS n D σ (Δ1 ++ ρ) ys = .ok (Δ2, m2, l2) ∧ r = (Δ2 ++ Δ1, m1 ++ m2, l1 ++ l2) := by
  cases n with
  | zero => simp [execS] at h
  | succ n =>
    simp only [execS, bind_ok] at h
    obtain ⟨⟨Δ1, m1, l1⟩, h1, ⟨Δ2, m2, l2⟩, h2, h⟩ := h
    cases h
    exact ⟨Δ1, m1, l1, Δ2, m2, l2, execOne_mono h1 (Nat.le_succ n), execS_mono h2 (Nat.le_succ n), rfl⟩

/-- `s` binds nothing that `xs` binds. -/
def FreshAfter (s : Stmt) (xs : List Stmt) : Prop := ∀ x, s.binder = some x → x ∉ binders xs

theorem fresh_keys {s : Stmt} {xs : List Stmt} {n D σ ρ Δs ms ls n' Δ1 m1 l1}
    (hf : FreshAfter s xs) (hs : execOne n D σ ρ s = .ok (Δs, ms, ls))
    (h1 : execS n' D σ [] xs = .ok (Δ1, m1, l1)) : ∀ x ∈ keys Δs, x ∉ keys (Δ1 ++ []) := by
  intro x hx hx1
  simp only [List.append_nil] at hx1
  exact hf x (execOne_keys hs x hx) (execS_keys xs h1 x hx1)

/-- DeleteFeature (read forwards) and AddFeature (read backwards). `s` is deleted from between
`xs` and `ys`; if the shorter block evaluates, its model is the longer one's without `s`'s fragment. -/
theorem delete_sound {n n' D σ xs s ys Δ m l Δ' m' l'} (hf : FreshAfter s xs)
    (h : execS n D σ [] (xs ++ s :: ys) = .ok (Δ, m, l))
    (h' : execS n' D σ [] (xs ++ ys) = .ok (Δ', m', l')) :
    ∃ m1 ms m3 l1 ls l3, (∃ k ρ Δs, execOne k D σ ρ s = .ok (Δs, ms, ls)) ∧
      m = m1 ++ ms ++ m3 ∧ l = l1 ++ ls ++ l3 ∧ m' = m1 ++ m3 ∧ l' = l1 ++ l3 := by
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (s :: ys) h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hs, h3, e2⟩ := execS_cons h2
  obtain ⟨Δ1', m1', l1', Δ3', m3', l3', h1', h3', e'⟩ := execS_append xs ys h'
  cases execS_unique h1 h1'
  have hw := (weak n').s h3' (sub_fresh (fresh_keys hf hs h1))
  cases execS_unique h3 hw
  simp only [Prod.mk.injEq] at e e2 e'
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  obtain ⟨-, rfl, rfl⟩ := e'
  exact ⟨m1, ms, m3, l1, ls, l3, ⟨n, _, _, hs⟩, by simp, by simp, rfl, rfl⟩

/-- MoveFeature: swap two neighbours. If the swapped block evaluates, its model is the old one
with the two fragments swapped. -/
theorem move_sound {n n' D σ xs s t ys Δ m l Δ' m' l'} (hs : FreshAfter s xs) (ht : FreshAfter t xs)
    (hst : ∀ x, s.binder = some x → t.binder ≠ some x)
    (h : execS n D σ [] (xs ++ s :: t :: ys) = .ok (Δ, m, l))
    (h' : execS n' D σ [] (xs ++ t :: s :: ys) = .ok (Δ', m', l')) :
    ∃ m1 ms mt m3 l1 ls lt l3, m = m1 ++ ms ++ mt ++ m3 ∧ l = l1 ++ ls ++ lt ++ l3 ∧
      m' = m1 ++ mt ++ ms ++ m3 ∧ l' = l1 ++ lt ++ ls ++ l3 := by
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append xs (s :: t :: ys) h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hS, h3, e2⟩ := execS_cons h2
  obtain ⟨Δt, mt, lt, Δ4, m4, l4, hT, h4, e3⟩ := execS_cons h3
  obtain ⟨Δ1', m1', l1', Δ2', m2', l2', h1', h2', e'⟩ := execS_append xs (t :: s :: ys) h'
  cases execS_unique h1 h1'
  obtain ⟨Δt', mt', lt', Δ3', m3', l3', hT', h3', e2'⟩ := execS_cons h2'
  obtain ⟨Δs', ms', ls', Δ4', m4', l4', hS', h4', e3'⟩ := execS_cons h3'
  -- t ran before s in the edit: fewer bindings, same result.
  have kS := fresh_keys hs hS h1
  have kT := fresh_keys ht hT' h1
  have wT := (weak n').one hT' (sub_fresh kS)
  cases execOne_unique hT wT
  -- s ran after t in the edit: more bindings, same result.
  have wS := (weak n).one hS (sub_fresh kT)
  cases execOne_unique hS' wS
  -- the rest sees the same names either way.
  have dis : ∀ x ∈ keys Δs, x ∉ keys Δt := by
    intro x hx hx'
    exact hst x (execOne_keys hS x hx) (execOne_keys hT x hx')
  have sw := sub_swap (ρ := Δ1 ++ []) dis
  rw [List.append_assoc, List.append_assoc] at sw
  have w4 := (weak n').s h4' sw
  cases execS_unique h4 w4
  simp only [Prod.mk.injEq] at e e2 e3 e' e2' e3'
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  obtain ⟨-, rfl, rfl⟩ := e3
  obtain ⟨-, rfl, rfl⟩ := e'
  obtain ⟨-, rfl, rfl⟩ := e2'
  obtain ⟨-, rfl, rfl⟩ := e3'
  exact ⟨m1, ms, mt, m4, l1, ls, lt, l4, by simp, by simp, by simp, by simp⟩

end ForgeLang
