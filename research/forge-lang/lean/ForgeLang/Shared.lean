import ForgeLang.Subst

/-!
# InlineExpr and MakeShared

Both act on one top-level `let` and are stated like Add, Delete and Move: complete (the original
evaluates, so the edited one does, to the expected model) and sound (the edited one evaluates, so
the original does, and the edited model is the expected one).

* **InlineExpr** freezes a driven param: `let x = e` becomes `let x = r`, with `r` a fresh literal
  set to the value `e` had. Every number stays; `x` gains a handle. The edited model is the old
  one with `T` (the trace `e` had) written as `r`.
* **MakeShared** links a param to another by design: `let b = p` becomes `let b = a`, where `a`
  holds a trace `Ta` with the same value as `p`. Every number stays; `b` and everything built
  from it are now driven by what drives `a`. The edited model is the old one with `p` written as `Ta`.

Both rest on `substEq`: the rest of the document sees an environment that differs only by the
substitution, and evaluation commutes with it.
-/

set_option linter.unusedSimpArgs false

namespace ForgeLang

@[simp] theorem Model.subst_nil (θ : Site → Tr) : Model.subst θ [] = [] := rfl

theorem fS_eq {θ : Site → Tr} {r : Env × Model × Log} {Δ m l} (h : fS θ r = (Δ, m, l)) :
    r.1.subst θ = Δ ∧ r.2.1.subst θ = m ∧ substLog θ r.2.2 = l := by
  obtain ⟨_, _, _⟩ := r; simp only [fS, Prod.mk.injEq] at h; exact h

theorem map_ok_iff {α β : Type} {f : α → β} {x : R α} {b : β} :
    x.map f = .ok b ↔ ∃ a, x = .ok a ∧ f a = b := by
  cases x <;> simp

theorem let_var {f D σ ρ x a} {t : Tr} (h : ρ.lookup a = some (.num t)) :
    execOne (f + 2) D σ ρ (.let_ x (.var a)) = .ok ([(x, .num t)], [], []) := by
  simp [execOne, evalE, lookupNum, h]

theorem let_lit' {f D σ ρ x p} : execOne (f + 2) D σ ρ (.let_ x (.lit p)) = .ok ([(x, .num (.site p))], [], []) := by
  simp [execOne, evalE]

/-! ## MakeShared -/

/-- `p` written as `Ta`. -/
def linkTheta (p : Site) (Ta : Tr) : Site → Tr := fun s => if s = p then Ta else .site s

theorem link_hσ {τ : Site → Int} {p : Site} {Ta : Tr} (hval : Ta.den τ = τ p) :
    ∀ s, τ s = (linkTheta p Ta s).den τ := by
  intro s; unfold linkTheta; split
  · subst_vars; exact hval.symm
  · rfl

/-- The part before the edit runs the same either way, and the substitution leaves it alone. -/
theorem xs_fixed {θ : Site → Tr} {τ : Site → Int} (hσ : ∀ s, τ s = (θ s).den τ) {n D xs Δ1 m1 l1}
    (fix : FixT θ (codeB xs ++ D.code)) (h1 : execS n D τ [] xs = .ok (Δ1, m1, l1)) :
    Δ1.subst θ = Δ1 ∧ m1.subst θ = m1 ∧ substLog θ l1 = l1 := by
  have e := (substEq hσ n).s (D := D) (ρ := []) fix
  simp only [show Env.subst θ [] = [] from rfl, h1, map_ok, Except.ok.injEq] at e
  exact (fS_eq e.symm)

def MSFix (θ : Site → Tr) (D : Defs) (xs ys : List Stmt) : Prop := FixT θ (codeB xs ++ codeB ys ++ D.code)

theorem MSFix.xs {θ : Site → Tr} {D : Defs} {xs ys : List Stmt} (h : MSFix θ D xs ys) :
    FixT θ (codeB xs ++ D.code) := fun s hs => h s (by
  rcases List.mem_append.1 hs with hs | hs <;> simp [hs])

theorem MSFix.ys {θ : Site → Tr} {D : Defs} {xs ys : List Stmt} (h : MSFix θ D xs ys) :
    FixT θ (codeB ys ++ D.code) := fun s hs => h s (by
  rcases List.mem_append.1 hs with hs | hs <;> simp [hs])

/-- MakeShared, complete. -/
theorem makeShared_complete {n D τ xs ys b a p Ta Δ m l}
    (fix : MSFix (linkTheta p Ta) D xs ys) (hval : Ta.den τ = τ p)
    (ha : ∀ k Δ1 m1 l1, execS k D τ [] xs = .ok (Δ1, m1, l1) → Δ1.lookup a = some (.num Ta))
    (h : execS n D τ [] (xs ++ .let_ b (.lit p) :: ys) = .ok (Δ, m, l)) :
    ∃ N Δ', execS N D τ [] (xs ++ .let_ b (.var a) :: ys) =
      .ok (Δ', m.subst (linkTheta p Ta), substLog (linkTheta p Ta) l) := by
  have hσ := link_hσ hval
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append _ _ h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hs, h3, e2⟩ := execS_cons h2
  cases let_lit hs
  obtain ⟨fΔ, fm, fl⟩ := xs_fixed hσ fix.xs h1
  have e3 := (substEq hσ n).s (D := D) (ρ := [(b, .num (.site p))] ++ (Δ1 ++ [])) (x := ys) fix.ys
  rw [h3] at e3
  simp only [Env.subst_append, fΔ, show Env.subst (linkTheta p Ta) [(b, Bind.num (Tr.site p))] =
    [(b, .num Ta)] by simp [Env.subst, Bind.subst, Tr.subst, linkTheta],
    show Env.subst (linkTheta p Ta) [] = [] from rfl, map_ok] at e3
  obtain ⟨N1, hN1⟩ := execS_cons_compose (let_var (f := 0) (D := D) (σ := τ) (x := b) (a := a)
    (ρ := Δ1 ++ []) (by simpa using ha n Δ1 m1 l1 h1)) e3
  obtain ⟨N, hN⟩ := execS_compose' h1 hN1
  simp only [Prod.mk.injEq] at e e2
  obtain ⟨rfl, rfl, rfl⟩ := e
  obtain ⟨rfl, rfl, rfl⟩ := e2
  refine ⟨N, Env.subst (linkTheta p Ta) Δ3 ++ [(b, .num Ta)] ++ Δ1, ?_⟩
  rw [hN]
  simp [fS, Model.subst_append, substLog_append, fm, fl, substLog_nil]

/-- MakeShared, sound: if the linked document evaluates, so does the original, and the linked
model is the original with `p` written as `Ta`. -/
theorem makeShared_sound {n D τ xs ys b a p Ta Δ' m' l'}
    (fix : MSFix (linkTheta p Ta) D xs ys) (hval : Ta.den τ = τ p)
    (ha : ∀ k Δ1 m1 l1, execS k D τ [] xs = .ok (Δ1, m1, l1) → Δ1.lookup a = some (.num Ta))
    (h' : execS n D τ [] (xs ++ .let_ b (.var a) :: ys) = .ok (Δ', m', l')) :
    ∃ N Δ m l, execS N D τ [] (xs ++ .let_ b (.lit p) :: ys) = .ok (Δ, m, l) ∧
      m' = m.subst (linkTheta p Ta) ∧ l' = substLog (linkTheta p Ta) l := by
  have hσ := link_hσ hval
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e⟩ := execS_append _ _ h'
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hs, h3, e2⟩ := execS_cons h2
  have hl := ha n Δ1 m1 l1 h1
  cases execOne_unique hs (let_var (f := 0) (D := D) (σ := τ) (x := b) (ρ := Δ1 ++ []) (by simpa using hl))
  obtain ⟨fΔ, fm, fl⟩ := xs_fixed hσ fix.xs h1
  have e3 := (substEq hσ n).s (D := D) (ρ := [(b, .num (.site p))] ++ (Δ1 ++ [])) (x := ys) fix.ys
  simp only [Env.subst_append, fΔ, show Env.subst (linkTheta p Ta) [(b, Bind.num (Tr.site p))] =
    [(b, .num Ta)] by simp [Env.subst, Bind.subst, Tr.subst, linkTheta],
    show Env.subst (linkTheta p Ta) [] = [] from rfl] at e3
  rw [h3, eq_comm, map_ok_iff] at e3
  obtain ⟨⟨Δ3o, m3o, l3o⟩, h3o, f3⟩ := e3
  obtain ⟨-, rfl, rfl⟩ := fS_eq f3
  obtain ⟨N1, hN1⟩ := execS_cons_compose (let_lit' (f := 0) (D := D) (σ := τ) (ρ := Δ1 ++ []) (x := b) (p := p)) h3o
  obtain ⟨N, hN⟩ := execS_compose' h1 hN1
  simp only [Prod.mk.injEq] at e e2
  obtain ⟨-, rfl, rfl⟩ := e
  obtain ⟨-, rfl, rfl⟩ := e2
  refine ⟨N, _, _, _, hN, ?_, ?_⟩
  · simp [Model.subst_append, fm]
  · simp [substLog_append, fl, substLog_nil]

/-- The numbers do not move. -/
theorem makeShared_nums {τ : Site → Int} {p : Site} {Ta : Tr} (hval : Ta.den τ = τ p) (m : Model) :
    (m.subst (linkTheta p Ta)).nums τ = m.nums τ := by
  simp [Model.nums, Model.subst, Tr.den_subst (link_hσ hval)]

/-! ## InlineExpr -/

/-- `r` written as `T`. -/
def inlineTheta (r : Site) (T : Tr) : Site → Tr := fun s => if s = r then T else .site s

theorem inline_hσ {τA : Site → Int} {r : Site} {T : Tr} :
    ∀ s, upd τA r (T.den τA) s = (inlineTheta r T s).den τA := by
  intro s; unfold inlineTheta upd; split <;> rfl

/-- InlineExpr, complete: if the original evaluates, the frozen document does, with the table
extended by `r := value of T`, and its model with `r` written as `T` is the original model. -/
theorem inlineExpr_complete {n D τA xs ys x e r T Δ m l}
    (fix : MSFix (inlineTheta r T) D xs ys)
    (he : ∀ k Δ1 m1 l1, execS k D τA [] xs = .ok (Δ1, m1, l1) → ∃ k' le, evalE k' D τA (Δ1 ++ []) e = .ok (T, le))
    (h : execS n D τA [] (xs ++ .let_ x e :: ys) = .ok (Δ, m, l)) :
    ∃ N Δ' m' l', execS N D (upd τA r (T.den τA)) [] (xs ++ .let_ x (.lit r) :: ys) = .ok (Δ', m', l') ∧
      m = m'.subst (inlineTheta r T) ∧
      ∃ l1 le l3, l = substLog (inlineTheta r T) l1 ++ le ++ substLog (inlineTheta r T) l3 ∧ l' = l1 ++ l3 := by
  have hσ := inline_hσ (τA := τA) (r := r) (T := T)
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e1⟩ := execS_append _ _ h
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hs, h3, e2⟩ := execS_cons h2
  have ex := (substEq hσ n).s (D := D) (ρ := []) (x := xs) fix.xs
  rw [show Env.subst (inlineTheta r T) [] = [] from rfl, h1, eq_comm, map_ok_iff] at ex
  obtain ⟨⟨Δ1b, m1b, l1b⟩, h1b, f1⟩ := ex
  obtain ⟨rfl, rfl, rfl⟩ := fS_eq f1
  obtain ⟨k', le, hev⟩ := he n _ _ _ h1
  have hs' : execOne (k' + 1) D τA (Env.subst (inlineTheta r T) Δ1b ++ []) (.let_ x e) =
      .ok ([(x, .num T)], [], le) := by simp only [List.append_nil] at hev; simp [execOne, hev]
  have hu := execOne_unique hs hs'
  simp only [Prod.mk.injEq] at hu
  obtain ⟨rfl, rfl, rfl⟩ := hu
  have e3 := (substEq hσ n).s (D := D) (ρ := [(x, .num (.site r))] ++ (Δ1b ++ [])) (x := ys) fix.ys
  simp only [Env.subst_append, show Env.subst (inlineTheta r T) [(x, Bind.num (Tr.site r))] =
    [(x, .num T)] by simp [Env.subst, Bind.subst, Tr.subst, inlineTheta],
    show Env.subst (inlineTheta r T) [] = [] from rfl] at e3
  rw [h3, eq_comm, map_ok_iff] at e3
  obtain ⟨⟨Δ3b, m3b, l3b⟩, h3b, f3⟩ := e3
  obtain ⟨-, rfl, rfl⟩ := fS_eq f3
  obtain ⟨N1, hN1⟩ := execS_cons_compose (let_lit' (f := 0) (D := D) (σ := upd τA r (T.den τA))
    (ρ := Δ1b ++ []) (x := x) (p := r)) h3b
  obtain ⟨N, hN⟩ := execS_compose' h1b hN1
  simp only [Prod.mk.injEq] at e1 e2
  obtain ⟨-, rfl, rfl⟩ := e1
  obtain ⟨-, rfl, rfl⟩ := e2
  refine ⟨N, _, _, _, hN, by simp [Model.subst_append], l1b, ls, l3b, by simp, by simp⟩

/-- InlineExpr, sound: if the frozen document evaluates (and `e` evaluates to `T` where it stood),
the original evaluates, and the frozen model is the original with `T` written as `r`. -/
theorem inlineExpr_sound {n D τA xs ys x e r T Δ' m' l'}
    (fix : MSFix (inlineTheta r T) D xs ys)
    (he : ∀ k Δ1 m1 l1, execS k D τA [] xs = .ok (Δ1, m1, l1) → ∃ k' le, evalE k' D τA (Δ1 ++ []) e = .ok (T, le))
    (h' : execS n D (upd τA r (T.den τA)) [] (xs ++ .let_ x (.lit r) :: ys) = .ok (Δ', m', l')) :
    ∃ N Δ m l, execS N D τA [] (xs ++ .let_ x e :: ys) = .ok (Δ, m, l) ∧ m = m'.subst (inlineTheta r T) := by
  have hσ := inline_hσ (τA := τA) (r := r) (T := T)
  obtain ⟨Δ1, m1, l1, Δ2, m2, l2, h1, h2, e1⟩ := execS_append _ _ h'
  obtain ⟨Δs, ms, ls, Δ3, m3, l3, hs, h3, e2⟩ := execS_cons h2
  cases let_lit hs
  have ex := (substEq hσ n).s (D := D) (ρ := []) (x := xs) fix.xs
  rw [h1] at ex
  simp only [show Env.subst (inlineTheta r T) [] = [] from rfl, map_ok] at ex
  obtain ⟨k', le, hev⟩ := he n _ _ _ ex
  have hs' : execOne (k' + 1) D τA (Env.subst (inlineTheta r T) Δ1 ++ []) (.let_ x e) =
      .ok ([(x, .num T)], [], le) := by simp only [List.append_nil] at hev; simp [execOne, hev]
  have e3 := (substEq hσ n).s (D := D) (ρ := [(x, .num (.site r))] ++ (Δ1 ++ [])) (x := ys) fix.ys
  simp only [Env.subst_append, show Env.subst (inlineTheta r T) [(x, Bind.num (Tr.site r))] =
    [(x, .num T)] by simp [Env.subst, Bind.subst, Tr.subst, inlineTheta],
    show Env.subst (inlineTheta r T) [] = [] from rfl] at e3
  rw [h3, map_ok] at e3
  obtain ⟨N1, hN1⟩ := execS_cons_compose hs' e3
  simp only [fS] at ex hN1
  obtain ⟨N, hN⟩ := execS_compose' (by simpa using ex) hN1
  simp only [Prod.mk.injEq] at e1 e2
  obtain ⟨-, rfl, rfl⟩ := e1
  obtain ⟨-, rfl, rfl⟩ := e2
  exact ⟨N, _, _, _, hN, by simp [Model.subst_append]⟩

/-- The numbers do not move. -/
theorem inlineExpr_nums {τA : Site → Int} {r : Site} {T : Tr} (m' : Model) :
    (m'.subst (inlineTheta r T)).nums τA = m'.nums (upd τA r (T.den τA)) := by
  simp [Model.nums, Model.subst, Tr.den_subst (inline_hσ (τA := τA) (r := r) (T := T))]

end ForgeLang
